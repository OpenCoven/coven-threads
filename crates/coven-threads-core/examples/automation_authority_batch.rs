//! Answers automation-authority cases in one process, for
//! `scripts/automation-authority-differential.mjs`.
//!
//! Reads frames from stdin, one case each: an envelope line, `{"operation",
//! "body_bytes", "keys"?}`, then exactly `body_bytes` bytes of the vector body
//! as serialized text, then a newline. The envelope is transport, parsed with
//! `serde_json`. The body is the artifact under test: it crosses the
//! production [`strict_parse_json`], and nothing else, before any operation
//! runs, so the parser is compared on the same bytes the reference reads, and
//! a body it refuses answers with that `json_*` code. `keys`, a keyring's
//! records by key id, is given when the keyring changes and holds for the
//! frames that follow. Writes one line per case: `{"code": <first error code
//! or null>, "result": <string or null>}`. `strict_parse` returns the
//! canonical text of the parsed
//! `raw_json`, `evaluate_request` the canonical decision text,
//! `verify_decision` the outcome, `request_adoption` and
//! `decision_consumption` the canonical final replay state (`"null"` when no
//! repetition ran), `lifecycle` the canonical lifecycle state (`"null"` before
//! any event), `verify_dispatch` the canonical dispatch result, and every
//! other operation `"ok"`. Each operation is invoked as
//! `profiles/automation-authority/v1/run-vectors.mjs` invokes it.
//!
//! `--describe` prints the build's `serde_json` configuration instead, so a
//! run records which one it compared.

use std::io::{self, BufRead, Read, Write};

use coven_threads_core::automation_authority::{
    adopt_authorization_request, apply_lifecycle_event, authorize_evidence_read, canonicalize,
    consume_decision, evaluate_authorization, strict_parse_json, validate_approval,
    validate_authorization_request, validate_consumption_snapshot, validate_proposal,
    verify_decision_bundle, verify_dispatch, AdoptionState, AuthorityError, AuthorityResult,
    ConsumptionState, ErrorCode, Keyring, LifecycleState, SignatureVerifier,
};
use ring::signature::{UnparsedPublicKey, ED25519};
use serde_json::{json, Value};

struct Ring;

impl SignatureVerifier for Ring {
    fn verify_ed25519(
        &self,
        public_key: &[u8; 32],
        message: &[u8; 32],
        signature: &[u8; 64],
    ) -> bool {
        UnparsedPublicKey::new(&ED25519, public_key)
            .verify(message, signature)
            .is_ok()
    }
}

/// How many times `for (let index = 0; index < (body.repetitions ?? 1);
/// index += 1)` runs: once when the count is missing or null, never when it
/// is negative or not a number, and up to the next integer for a fraction.
fn repetitions(body: &Value) -> u64 {
    let count = match body.get("repetitions") {
        None | Some(Value::Null) => return 1,
        Some(Value::Number(number)) => number.as_f64().unwrap_or(f64::NAN),
        Some(Value::Bool(flag)) => f64::from(u8::from(*flag)),
        // `Number(text)`: blank is zero, and anything that is not a number is
        // NaN, which no index is less than.
        Some(Value::String(text)) if text.trim().is_empty() => 0.0,
        Some(Value::String(text)) => text.trim().parse().unwrap_or(f64::NAN),
        Some(_) => f64::NAN,
    };
    if count.is_nan() || count <= 0.0 {
        0
    } else {
        // Saturates at u64::MAX for an infinite count, as the reference's loop
        // would run until an operation refuses.
        count.ceil() as u64
    }
}

fn run(operation: &str, body: &Value, keyring: &Keyring) -> AuthorityResult<String> {
    let repetitions = repetitions(body);
    match operation {
        "strict_parse" => strict_parse_json(body["raw_json"].as_str().unwrap_or_default())
            .and_then(|parsed| canonicalize(&parsed)),
        "validate_request" => {
            validate_authorization_request(&body["request"], keyring, &Ring).map(|()| "ok".into())
        }
        "request_adoption" => {
            let mut state: Option<AdoptionState> = None;
            for _ in 0..repetitions {
                state = Some(adopt_authorization_request(
                    &body["request"],
                    state.as_ref(),
                    keyring,
                    &Ring,
                    body.get("now"),
                )?);
            }
            replay_state(state.map(|state| {
                json!({
                    "nonces": state.nonces,
                    "adoption_keys": state.adoption_keys,
                    "request_digests": state.request_digests,
                })
            }))
        }
        "evaluate_request" => {
            let decision =
                evaluate_authorization(&body["request"], &body["policy"], keyring, &Ring)?;
            canonicalize(&decision)
        }
        "verify_decision" => verify_decision_bundle(
            &body["request"],
            &body["decision"],
            &body["policy"],
            keyring,
            &Ring,
        ),
        "decision_consumption" => {
            let mut state: Option<ConsumptionState> = None;
            for _ in 0..repetitions {
                state = Some(consume_decision(
                    &body["decision"],
                    state.as_ref(),
                    keyring,
                    &Ring,
                )?);
            }
            replay_state(state.map(|state| {
                json!({
                    "decision_ids": state.decision_ids,
                    "decision_digests": state.decision_digests,
                })
            }))
        }
        "validate_approval" => {
            validate_approval(&body["approval"], keyring, &Ring, body.get("now"))
                .map(|()| "ok".into())
        }
        "validate_consumption_snapshot" => validate_consumption_snapshot(
            &body["consumption_snapshot"],
            keyring,
            &Ring,
            body.get("now"),
        )
        .map(|()| "ok".into()),
        "lifecycle" => lifecycle(body, keyring),
        "verify_dispatch" => {
            // `body.x ?? null`, and `body.events ?? []`; a missing artifact is
            // `undefined` in the reference, which it refuses as `null`.
            let or_null = |name: &str| body.get(name).cloned().unwrap_or(Value::Null);
            let events = match body.get("events") {
                None | Some(Value::Null) => json!([]),
                Some(events) => events.clone(),
            };
            let bundle = json!({
                "request": or_null("request"),
                "decision": or_null("decision"),
                "approval": or_null("approval"),
                "approval_authorization_request": or_null("approval_authorization_request"),
                "approval_authorization_decision": or_null("approval_authorization_decision"),
                "lifecycle_events": events,
                "consumption_snapshot": or_null("consumption_snapshot"),
                "snapshot": or_null("snapshot"),
            });
            let verified = verify_dispatch(&bundle, keyring, &Ring)?;
            let consumption = verified.required_consumption;
            canonicalize(&json!({
                "ok": true,
                "dispatch_binding_digest": verified.dispatch_binding_digest,
                "required_consumption": {
                    "decision_digest": consumption.decision_digest,
                    "approval_id": consumption.approval_id,
                    "approval_usage_before": consumption.approval_usage_before,
                },
            }))
        }
        "validate_proposal" => {
            validate_proposal(&body["proposal"], keyring, &Ring).map(|()| "ok".into())
        }
        "evidence_read" => authorize_evidence_read(
            &body["read"],
            &body["evidence"],
            keyring,
            &Ring,
            body.get("now"),
        )
        .map(|()| "ok".into()),
        other => panic!("unsupported operation {other}"),
    }
}

/// The final replay state as the differential's `canonical` prints it.
fn replay_state(state: Option<Value>) -> AuthorityResult<String> {
    state.map_or_else(|| Ok("null".to_owned()), |state| canonicalize(&state))
}

/// `lifecycleState`, then the last event once more when `replay_last` is
/// truthy, as `run-vectors.mjs` runs a lifecycle vector.
fn lifecycle(body: &Value, keyring: &Keyring) -> AuthorityResult<String> {
    // `for (const event of body.events)`: an array yields its items and a
    // string its characters; anything else is not iterable, a `TypeError` the
    // port refuses.
    let events: Vec<Value> = match body.get("events") {
        Some(Value::Array(items)) => items.clone(),
        Some(Value::String(text)) => text.chars().map(|ch| json!(ch.to_string())).collect(),
        _ => {
            return Err(AuthorityError {
                code: ErrorCode::SchemaType,
                message: "lifecycle events are not iterable".into(),
                path: "$.events".into(),
            })
        }
    };
    let approval = body.get("approval");
    let mut state: Option<LifecycleState> = None;
    for event in &events {
        state = Some(apply_lifecycle_event(
            state.as_ref(),
            event,
            approval,
            keyring,
            &Ring,
        )?);
    }
    let replay_last = match body.get("replay_last") {
        None | Some(Value::Null) => false,
        Some(Value::Bool(flag)) => *flag,
        Some(Value::Number(number)) => number.as_f64().is_some_and(|number| number != 0.0),
        Some(Value::String(text)) => !text.is_empty(),
        Some(_) => true,
    };
    if replay_last {
        apply_lifecycle_event(
            state.as_ref(),
            events.last().unwrap_or(&Value::Null),
            approval,
            keyring,
            &Ring,
        )?;
    }
    canonicalize(&state.map_or(Value::Null, |state| {
        json!({
            "approval_id": state.approval_id,
            "request_digest": state.request_digest,
            "decision_digest": state.decision_digest,
            "state": state.state,
            "sequence": state.sequence,
            "last_event_digest": state.last_event_digest,
            "consumption_count": state.consumption_count,
            "consumed_occurrences": state.consumed_occurrences,
            "event_ids": state.event_ids,
        })
    }))
}

/// The build's `serde_json` configuration, probed rather than declared: with
/// `arbitrary_precision` a number keeps its literal text.
fn describe() -> Value {
    let arbitrary_precision = serde_json::from_str::<Value>("1.00")
        .map(|number| number.to_string())
        .is_ok_and(|text| text == "1.00");
    json!({ "serde_json_arbitrary_precision": arbitrary_precision })
}

/// The artifact body, from its serialized text, through the production
/// parser: never `serde_json`, which would collapse a duplicate key to its
/// last value and give every refusal one code.
fn parse_body(text: &str) -> AuthorityResult<Value> {
    strict_parse_json(text)
}

fn main() -> io::Result<()> {
    if std::env::args().nth(1).as_deref() == Some("--describe") {
        println!("{}", describe());
        return Ok(());
    }
    let malformed = |message: &str| io::Error::new(io::ErrorKind::InvalidData, message.to_owned());
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut keyring: Option<Keyring> = None;
    let mut envelope = String::new();
    loop {
        // The envelope line is transport; the body bytes after it are the
        // artifact, and cross the production parser as text.
        envelope.clear();
        if input.read_line(&mut envelope)? == 0 {
            return Ok(());
        }
        let frame: Value = serde_json::from_str(&envelope)?;
        if let Some(keys) = frame.get("keys").and_then(Value::as_object) {
            keyring = Some(Keyring::from_object(keys));
        }
        let Some(keyring) = keyring.as_ref() else {
            return Err(malformed("a case needs a keyring before it"));
        };
        let Some(length) = frame["body_bytes"].as_u64() else {
            return Err(malformed("a frame must give the body's length in bytes"));
        };
        let mut body = vec![0_u8; usize::try_from(length).map_err(|_| malformed("body too long"))?];
        input.read_exact(&mut body)?;
        let mut newline = [0_u8; 1];
        input.read_exact(&mut newline)?;
        if newline != *b"\n" {
            return Err(malformed("a body must end its frame with a newline"));
        }
        let text = std::str::from_utf8(&body).map_err(|_| malformed("a body must be UTF-8"))?;
        let operation = frame["operation"].as_str().unwrap_or_default();
        let answer = match parse_body(text).and_then(|body| run(operation, &body, keyring)) {
            Ok(result) => json!({ "code": null, "result": result }),
            Err(error) => json!({ "code": error.code.as_str(), "result": null }),
        };
        writeln!(out, "{answer}")?;
    }
}
