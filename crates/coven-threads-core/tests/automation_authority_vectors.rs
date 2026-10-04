//! Runs all 130 automation-authority conformance vectors against the Rust
//! port, invoked and judged as `profiles/automation-authority/v1/
//! run-vectors.mjs` invokes and judges them: a positive vector must succeed
//! with its expected outcome, and a negative one must fail with exactly its
//! expected first error code.
//!
//! The vectors are read from this repository through `CARGO_MANIFEST_DIR`.

use std::fs;
use std::path::{Path, PathBuf};

use coven_threads_core::automation_authority::{
    adopt_authorization_request, apply_lifecycle_event, authorize_evidence_read, consume_decision,
    evaluate_authorization, strict_parse_json, validate_approval, validate_authorization_request,
    validate_consumption_snapshot, validate_proposal, verify_decision_bundle, verify_dispatch,
    AdoptionState, AuthorityResult, ConsumptionState, Keyring, LifecycleState, SignatureVerifier,
};
use ring::signature::{UnparsedPublicKey, ED25519};
use serde_json::{json, Value};

/// Every operation the manifest names.
const OPERATIONS: [&str; 12] = [
    "strict_parse",
    "validate_request",
    "request_adoption",
    "evaluate_request",
    "verify_decision",
    "decision_consumption",
    "validate_consumption_snapshot",
    "validate_approval",
    "lifecycle",
    "verify_dispatch",
    "validate_proposal",
    "evidence_read",
];

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

fn profile() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../profiles/automation-authority/v1")
}

fn read(path: &Path) -> Value {
    strict_parse_json(&fs::read_to_string(path).unwrap())
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn keyring() -> Keyring {
    let document = read(&profile().join("keyring.json"));
    assert_eq!(
        document["schema_version"],
        "opencoven.automation-authority-test-keyring/v1"
    );
    let keyring = Keyring::from_object(document["keys"].as_object().unwrap());
    keyring.validate().unwrap();
    keyring
}

/// Runs one vector body, returning the outcome when the operation yields one.
fn execute(operation: &str, body: &Value, keyring: &Keyring) -> AuthorityResult<Option<String>> {
    let mut keyring = keyring.clone();
    if let Some(mutation) = body.get("keyring_mutation") {
        let record = keyring
            .record_mut(mutation["key_id"].as_str().unwrap())
            .expect("mutated key exists");
        if let Some(field) = mutation.get("remove").and_then(Value::as_str) {
            record.as_object_mut().unwrap().remove(field);
        }
        if let Some(set) = mutation.get("set").and_then(Value::as_object) {
            for (key, value) in set {
                record[key] = value.clone();
            }
        }
    }
    let keyring = &keyring;
    let repetitions = || body["repetitions"].as_u64().unwrap();
    match operation {
        "strict_parse" => strict_parse_json(body["raw_json"].as_str().unwrap()).map(|_| None),
        "validate_request" => {
            validate_authorization_request(&body["request"], keyring, &Ring).map(|()| None)
        }
        "request_adoption" => {
            let mut state: Option<AdoptionState> = None;
            for _ in 0..repetitions() {
                state = Some(adopt_authorization_request(
                    &body["request"],
                    state.as_ref(),
                    keyring,
                    &Ring,
                    body.get("now"),
                )?);
            }
            Ok(None)
        }
        "evaluate_request" => {
            evaluate_authorization(&body["request"], &body["policy"], keyring, &Ring)
                .map(|decision| decision["outcome"].as_str().map(ToOwned::to_owned))
        }
        "verify_decision" => verify_decision_bundle(
            &body["request"],
            &body["decision"],
            &body["policy"],
            keyring,
            &Ring,
        )
        .map(Some),
        "decision_consumption" => {
            let mut state: Option<ConsumptionState> = None;
            for _ in 0..repetitions() {
                state = Some(consume_decision(
                    &body["decision"],
                    state.as_ref(),
                    keyring,
                    &Ring,
                )?);
            }
            Ok(None)
        }
        "validate_approval" => {
            validate_approval(&body["approval"], keyring, &Ring, body.get("now")).map(|()| None)
        }
        "validate_consumption_snapshot" => validate_consumption_snapshot(
            &body["consumption_snapshot"],
            keyring,
            &Ring,
            body.get("now"),
        )
        .map(|()| None),
        "lifecycle" => {
            // `lifecycleState`, then the last event once more when
            // `replay_last` asks for it.
            let events = body["events"].as_array().unwrap();
            let mut state: Option<LifecycleState> = None;
            for event in events {
                state = Some(apply_lifecycle_event(
                    state.as_ref(),
                    event,
                    body.get("approval"),
                    keyring,
                    &Ring,
                )?);
            }
            if body["replay_last"] == true {
                apply_lifecycle_event(
                    state.as_ref(),
                    events.last().unwrap_or(&Value::Null),
                    body.get("approval"),
                    keyring,
                    &Ring,
                )?;
            }
            Ok(None)
        }
        "verify_dispatch" => {
            let or_null = |name: &str| body.get(name).cloned().unwrap_or(Value::Null);
            let bundle = json!({
                "request": or_null("request"),
                "decision": or_null("decision"),
                "approval": or_null("approval"),
                "approval_authorization_request": or_null("approval_authorization_request"),
                "approval_authorization_decision": or_null("approval_authorization_decision"),
                "lifecycle_events": body.get("events").filter(|events| !events.is_null()).cloned().unwrap_or(json!([])),
                "consumption_snapshot": or_null("consumption_snapshot"),
                "snapshot": or_null("snapshot"),
            });
            verify_dispatch(&bundle, keyring, &Ring).map(|_| None)
        }
        "validate_proposal" => validate_proposal(&body["proposal"], keyring, &Ring).map(|()| None),
        "evidence_read" => authorize_evidence_read(
            &body["read"],
            &body["evidence"],
            keyring,
            &Ring,
            body.get("now"),
        )
        .map(|()| None),
        other => unreachable!("unknown manifest operation {other}"),
    }
}

#[test]
fn every_vector_matches_the_reference() {
    let manifest = read(&profile().join("manifest.json"));
    let keyring = keyring();
    let vectors = manifest["vectors"].as_array().unwrap();
    assert_eq!(vectors.len(), 130);
    assert!(
        vectors
            .iter()
            .all(|vector| OPERATIONS.contains(&vector["operation"].as_str().unwrap())),
        "every manifest operation is ported"
    );

    let mut failures = Vec::new();
    for vector in vectors {
        let id = vector["id"].as_str().unwrap();
        let body = read(
            &profile()
                .join("vectors")
                .join(vector["file"].as_str().unwrap()),
        );
        let expected = &vector["expected"];
        match (
            execute(vector["operation"].as_str().unwrap(), &body, &keyring),
            expected["ok"].as_bool().unwrap(),
        ) {
            (Ok(outcome), true) => {
                if let Some(want) = expected["outcome"].as_str() {
                    if outcome.as_deref() != Some(want) {
                        failures.push(format!("{id}: expected outcome {want}, got {outcome:?}"));
                    }
                }
            }
            (Ok(_), false) => failures.push(format!(
                "{id}: expected {}, got success",
                expected["error_code"]
            )),
            (Err(error), true) => failures.push(format!("{id}: expected success, got {error}")),
            (Err(error), false) => {
                if Some(error.code.as_str()) != expected["error_code"].as_str() {
                    failures.push(format!(
                        "{id}: expected {}, got {error}",
                        expected["error_code"]
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} of 130 vectors failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
