//! Answers automation-authority cases in one process, for
//! `scripts/automation-authority-differential.mjs`.
//!
//! Reads JSON Lines from stdin, one case per line: `{"operation", "body",
//! "keys"}`, where `keys` is a keyring's records by key id. Writes one line
//! per case: `{"code": <first error code or null>, "result": <string or
//! null>}`. `evaluate_request` returns the canonical decision text,
//! `verify_decision` the outcome, `lifecycle` the canonical lifecycle state
//! (`"null"` before any event), `verify_dispatch` the canonical dispatch
//! result, and every other operation `"ok"`. Each operation is invoked as
//! `profiles/automation-authority/v1/run-vectors.mjs` invokes it.

use std::io::{self, BufRead, Write};

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

fn run(operation: &str, body: &Value, keyring: &Keyring) -> AuthorityResult<String> {
    let repetitions = body["repetitions"].as_u64().unwrap_or(1);
    match operation {
        "strict_parse" => {
            strict_parse_json(body["raw_json"].as_str().unwrap_or_default()).map(|_| "ok".into())
        }
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
            Ok("ok".into())
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
            Ok("ok".into())
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

fn main() -> io::Result<()> {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for line in io::stdin().lock().lines() {
        let case: Value = serde_json::from_str(&line?)?;
        let keys = case["keys"].as_object().cloned().unwrap_or_default();
        let keyring = Keyring::from_object(&keys);
        let answer = match run(
            case["operation"].as_str().unwrap_or_default(),
            &case["body"],
            &keyring,
        ) {
            Ok(result) => json!({ "code": null, "result": result }),
            Err(error) => json!({ "code": error.code.as_str(), "result": null }),
        };
        writeln!(out, "{answer}")?;
    }
    Ok(())
}
