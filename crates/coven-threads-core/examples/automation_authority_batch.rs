//! Answers automation-authority cases in one process, for
//! `scripts/automation-authority-differential.mjs`.
//!
//! Reads JSON Lines from stdin, one case per line: `{"operation", "body",
//! "keys"}`, where `keys` is a keyring's records by key id. Writes one line
//! per case: `{"code": <first error code or null>, "result": <string or
//! null>}`. `evaluate_request` returns the canonical decision text,
//! `verify_decision` the outcome, `request_adoption` and
//! `decision_consumption` the canonical final replay state (`"null"` when no
//! repetition ran), and every other operation `"ok"`.

use std::io::{self, BufRead, Write};

use coven_threads_core::automation_authority::{
    adopt_authorization_request, canonicalize, consume_decision, evaluate_authorization,
    strict_parse_json, validate_authorization_request, verify_decision_bundle, AdoptionState,
    AuthorityResult, ConsumptionState, Keyring, SignatureVerifier,
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
        other => panic!("unsupported operation {other}"),
    }
}

/// The final replay state as the differential's `canonical` prints it.
fn replay_state(state: Option<Value>) -> AuthorityResult<String> {
    state.map_or_else(|| Ok("null".to_owned()), |state| canonicalize(&state))
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
