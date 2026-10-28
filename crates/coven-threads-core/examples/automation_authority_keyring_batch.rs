//! Ordered-keyring companion to the general authority differential batch.
//! Transport uses an array of `(id, record)` pairs so object parsing cannot
//! change JavaScript's Map iteration order or discard duplicate constructor ids.
use std::io::{self, BufRead, Write};

use coven_threads_core::automation_authority::{
    apply_lifecycle_event, authorize_evidence_read, canonicalize, strict_parse_json,
    validate_approval, validate_authorization_request, validate_consumption_snapshot,
    validate_decision, validate_proposal, AuthorityResult, Keyring, SignatureVerifier,
};
use ring::signature::{UnparsedPublicKey, ED25519};
use serde::Deserialize;
use serde_json::{json, Value};

struct Ring;

impl SignatureVerifier for Ring {
    fn verify_ed25519(&self, key: &[u8; 32], message: &[u8; 32], signature: &[u8; 64]) -> bool {
        UnparsedPublicKey::new(&ED25519, key)
            .verify(message, signature)
            .is_ok()
    }
}

#[derive(Deserialize)]
struct Case {
    operation: String,
    body_text: String,
    key_records: Vec<(String, Value)>,
}

fn run(case: Case) -> AuthorityResult<String> {
    let body = strict_parse_json(&case.body_text)?;
    let keys = Keyring::new(case.key_records);
    let result = match case.operation.as_str() {
        "validate_request" => validate_authorization_request(&body["request"], &keys, &Ring),
        "validate_decision" => validate_decision(&body["decision"], &keys, &Ring),
        "validate_approval" => validate_approval(&body["approval"], &keys, &Ring, body.get("now")),
        "validate_consumption_snapshot" => validate_consumption_snapshot(
            &body["consumption_snapshot"],
            &keys,
            &Ring,
            body.get("now"),
        ),
        "validate_proposal" => validate_proposal(&body["proposal"], &keys, &Ring),
        "evidence_read" => authorize_evidence_read(
            &body["read"],
            &body["evidence"],
            &keys,
            &Ring,
            body.get("now"),
        ),
        "validate_keyring" => keys.validate(),
        "lifecycle" => {
            let mut state = None;
            for event in body["events"].as_array().expect("corpus event array") {
                state = Some(apply_lifecycle_event(
                    state.as_ref(),
                    event,
                    body.get("approval"),
                    &keys,
                    &Ring,
                )?);
            }
            return canonicalize(&state.map_or(Value::Null, |state| {
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
            }));
        }
        other => panic!("unsupported operation {other}"),
    };
    result.map(|()| "ok".to_owned())
}

fn main() -> io::Result<()> {
    if std::env::args().nth(1).as_deref() == Some("--describe") {
        let arbitrary_precision = serde_json::from_str::<Value>("1.00")
            .map(|number| number.to_string())
            .is_ok_and(|text| text == "1.00");
        println!(
            "{}",
            json!({"serde_json_arbitrary_precision": arbitrary_precision})
        );
        return Ok(());
    }
    let mut out = io::stdout().lock();
    for line in io::stdin().lock().lines() {
        let case: Case = serde_json::from_str(&line?)?;
        let answer = match run(case) {
            Ok(result) => json!({"code": null, "result": result}),
            Err(error) => json!({"code": error.code.as_str(), "result": null}),
        };
        writeln!(out, "{answer}")?;
    }
    Ok(())
}
