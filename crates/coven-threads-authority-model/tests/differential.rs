//! Model-to-implementation differential (coven-threads#95).
//!
//! Every reachable model chain is rebuilt in the implementation by replaying
//! its witness path as real, Ed25519-signed lifecycle events. Each model
//! event in the differential space is then concretized against that chain
//! and applied through `apply_lifecycle_event`, and the two must agree:
//!
//! - when the model accepts, the implementation accepts with the same
//!   state, sequence, consumption count and consumed occurrences;
//! - when the model refuses, the implementation refuses with a first error
//!   code that names one of the model's reasons.
//!
//! Keys are generated per run; no private key is stored or printed.

use std::collections::HashSet;
use std::path::Path;
use std::time::Instant;

use coven_threads_authority_model::explore::{
    authority_core_space, consistent_disposition, describe, explore,
};
use coven_threads_authority_model::lifecycle::{
    apply, Approval, Binding, Chain, DigestClaim, Disposition, Event, Kind, NoLaunch,
    OccurrenceDisposition, Phase, PreviousClaim, Reason, Reasons, SequenceClaim, State,
    SubjectClaim, Use, Variant, MAX_USES_BOUND,
};
use coven_threads_core::automation_authority::{
    apply_lifecycle_event, canonical_digest, canonicalize, sign_artifact, strict_parse_json,
    ArtifactSigner, AuthorityError, Domain, ErrorCode, Keyring, LifecycleState, SignatureVerifier,
};
use ring::rand::SystemRandom;
use ring::signature::{Ed25519KeyPair, KeyPair, UnparsedPublicKey, ED25519};
use serde_json::{json, Value};

const AUTHORITY_KEY: &str = "key:threads:test-authority";
const PRINCIPAL_KEY: &str = "key:principal:alice";
const ED25519_SPKI_PREFIX: [u8; 12] = [
    0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
];

struct Ring;

impl SignatureVerifier for Ring {
    fn verify_ed25519(&self, key: &[u8; 32], message: &[u8; 32], signature: &[u8; 64]) -> bool {
        UnparsedPublicKey::new(&ED25519, key)
            .verify(message, signature)
            .is_ok()
    }
}

struct Signer {
    id: &'static str,
    pair: Ed25519KeyPair,
}

impl Signer {
    fn generate(id: &'static str) -> Self {
        let document = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new()).unwrap();
        Self {
            id,
            pair: Ed25519KeyPair::from_pkcs8(document.as_ref()).unwrap(),
        }
    }

    fn pem(&self) -> String {
        let mut der = ED25519_SPKI_PREFIX.to_vec();
        der.extend_from_slice(self.pair.public_key().as_ref());
        format!(
            "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----\n",
            base64(&der)
        )
    }
}

impl ArtifactSigner for Signer {
    fn key_id(&self) -> &str {
        self.id
    }

    fn sign_ed25519(&self, message: &[u8; 32]) -> [u8; 64] {
        self.pair.sign(message).as_ref().try_into().unwrap()
    }
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | (u32::from(*b) << (16 - 8 * i)));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// The keys, keyring and signed approvals one run uses.
struct Fixture {
    authority: Signer,
    keyring: Keyring,
    single_use: Value,
    recurring: Vec<Value>,
}

fn vector_approval(file: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../profiles/automation-authority/v1/vectors")
        .join(file);
    let mut approval =
        strict_parse_json(&std::fs::read_to_string(path).unwrap()).unwrap()["approval"].clone();
    approval.as_object_mut().unwrap().remove("integrity");
    approval
}

impl Fixture {
    fn new() -> Self {
        let authority = Signer::generate(AUTHORITY_KEY);
        let principal = Signer::generate(PRINCIPAL_KEY);
        let keyring = Keyring::new(vec![
            (
                AUTHORITY_KEY.into(),
                json!({"role": "threads_authority", "public_key_pem": authority.pem()}),
            ),
            (
                PRINCIPAL_KEY.into(),
                json!({"role": "principal", "principal_id": "principal:alice", "public_key_pem": principal.pem()}),
            ),
        ]);
        keyring.validate().unwrap();

        let mut single = vector_approval("10-single-use-approval-consumed.json");
        single["approval_id"] = json!("approval:pilot");
        single["occurrence_id"] = json!(occurrence_id(Binding { occurrence: 0 }));
        single["run_id"] = json!(run_id(Binding { occurrence: 0 }));
        let single_use = sign_artifact(&single, Domain::Approval, &principal).unwrap();

        let recurring = (1..=MAX_USES_BOUND)
            .map(|max_uses| {
                let mut approval =
                    vector_approval("10-recurring-approval-reuses-bounded-authority.json");
                approval["approval_id"] = json!("approval:pilot");
                approval["use"]["max_uses"] = json!(max_uses);
                approval["use"]["occurrence_prefix"] = json!("occ:daily:");
                sign_artifact(&approval, Domain::Approval, &principal).unwrap()
            })
            .collect();
        Self {
            authority,
            keyring,
            single_use,
            recurring,
        }
    }

    fn approval(&self, approval: Approval) -> &Value {
        match approval.use_ {
            Use::SingleUse => &self.single_use,
            Use::Recurring { max_uses } => &self.recurring[usize::from(max_uses) - 1],
        }
    }
}

fn occurrence_id(binding: Binding) -> String {
    if binding.in_prefix() {
        format!("occ:daily:000{}", binding.occurrence)
    } else {
        format!("occ:outside:000{}", binding.occurrence)
    }
}

fn run_id(binding: Binding) -> String {
    format!("run:daily:000{}", binding.occurrence)
}

fn other_digest(c: char) -> String {
    format!("sha256:{}", c.to_string().repeat(64))
}

/// The concrete signed event for model `event` appended to concrete `head`.
fn concretize(
    fixture: &Fixture,
    approval: &Value,
    head: Option<&LifecycleState>,
    event: Event,
    serial: u64,
) -> Value {
    let sequence = head.map_or(0, |h| h.sequence);
    let request_digest = approval["request_digest"].clone();
    let decision_digest = approval["decision_digest"].clone();
    let approval_digest = format!(
        "sha256:{}",
        canonical_digest(approval, Domain::Approval.as_str()).unwrap()
    );
    let binding_json = |binding: Binding| {
        json!({
            "request_digest": request_digest,
            "decision_digest": decision_digest,
            "occurrence_id": occurrence_id(binding),
            "run_id": run_id(binding),
            "attempt": 1,
            "fence_generation": 4,
        })
    };
    let (own_occurrence, own_run) = if approval["use"]["kind"] == "single_use" {
        (
            approval["occurrence_id"].clone(),
            approval["run_id"].clone(),
        )
    } else {
        (
            json!(occurrence_id(Binding { occurrence: 0 })),
            json!(run_id(Binding { occurrence: 0 })),
        )
    };
    let unsigned = json!({
        "schema_version": "opencoven.automation-approval-event/v1",
        "event_id": format!("event:pilot:{serial}"),
        "approval_id": match event.subject {
            SubjectClaim::Same => "approval:pilot",
            SubjectClaim::Different => "approval:other",
        },
        "request_digest": request_digest,
        "decision_digest": decision_digest,
        "approval_digest": match event.digest {
            DigestClaim::Null => Value::Null,
            DigestClaim::Matching => json!(approval_digest),
            DigestClaim::Other => json!(other_digest('d')),
        },
        "sequence": match event.sequence {
            SequenceClaim::Expected => sequence + 1,
            SequenceClaim::Stale => sequence,
            SequenceClaim::Skipped => sequence + 2,
        },
        "previous_event_digest": match (event.previous, head) {
            (PreviousClaim::Head, None) => Value::Null,
            (PreviousClaim::Head, Some(h)) => json!(h.last_event_digest),
            (PreviousClaim::Other, _) => json!(other_digest('e')),
        },
        "from_state": event.from.as_str(),
        "to_state": event.to.as_str(),
        "event": event.kind.as_str(),
        "occurred_at": "2026-09-03T13:03:00Z",
        "actor": "threads_authority",
        "execution_phase": event.phase.as_str(),
        "dispatch_disposition": event.disposition.as_str(),
        "consumption": event.consumption.map_or(Value::Null, binding_json),
        "occurrence_disposition": event.occurrence_disposition.map_or(Value::Null, |d| {
            let (occurrence, run) = if d.matches_approval {
                (own_occurrence.clone(), own_run.clone())
            } else {
                (json!("occ:elsewhere"), json!("run:elsewhere"))
            };
            json!({
                "occurrence_id": occurrence,
                "run_id": run,
                "disposition": match d.kind {
                    NoLaunch::Rejected => "rejected_no_launch",
                    NoLaunch::Expired => "expired_no_launch",
                },
            })
        }),
    });
    sign_artifact(&unsigned, Domain::ApprovalEvent, &fixture.authority).unwrap()
}

fn run(
    fixture: &Fixture,
    approval: &Value,
    head: Option<&LifecycleState>,
    event: Event,
    serial: u64,
) -> Result<LifecycleState, AuthorityError> {
    let concrete = concretize(fixture, approval, head, event, serial);
    let evidence = event.approval_supplied.then_some(approval);
    apply_lifecycle_event(head, &concrete, evidence, &fixture.keyring, &Ring)
}

/// The implementation's first codes each model reason may surface as.
fn codes(reason: Reason) -> &'static [ErrorCode] {
    match reason {
        Reason::Shape => &[
            ErrorCode::SchemaInteger,
            ErrorCode::SchemaEnum,
            ErrorCode::LifecycleConsumptionUnexpected,
            ErrorCode::LifecycleOccurrenceDispositionUnexpected,
            ErrorCode::RejectionDispositionInvalid,
            ErrorCode::RejectionOccurrenceMissing,
            ErrorCode::ExpirationDispositionInvalid,
            ErrorCode::ExpirationOccurrenceMissing,
            ErrorCode::RevocationDispositionInvalid,
            ErrorCode::ConsumptionBindingMissing,
            ErrorCode::ConsumptionDispositionInvalid,
            ErrorCode::LifecycleDispositionInvalid,
        ],
        Reason::ApprovalMissing => &[ErrorCode::LifecycleApprovalMissing],
        Reason::ApprovalMismatch => &[ErrorCode::LifecycleApprovalMismatch],
        Reason::ApprovalPremature => &[ErrorCode::LifecycleApprovalPremature],
        Reason::OccurrenceMismatch => &[ErrorCode::LifecycleOccurrenceMismatch],
        Reason::Replay => &[ErrorCode::LifecycleReplay],
        Reason::SubjectMismatch => &[ErrorCode::LifecycleSubjectMismatch],
        Reason::StateForged => &[ErrorCode::LifecycleStateForged],
        Reason::ChainMismatch => &[ErrorCode::LifecycleChainMismatch],
        Reason::TransitionInvalid => &[ErrorCode::LifecycleTransitionInvalid],
        Reason::ConsumptionReplayed => &[ErrorCode::ApprovalReplayed],
        Reason::ConsumptionMismatch => &[ErrorCode::ApprovalConsumptionMismatch],
        Reason::OccurrenceOutOfScope => &[ErrorCode::ApprovalOccurrenceOutOfScope],
        Reason::UsageExhausted => &[ErrorCode::ApprovalUsageExhausted],
    }
}

fn consumed_bits(fixture_approval: &Value, state: &LifecycleState) -> u8 {
    let mut bits = 0;
    for binding in Binding::ALL {
        let key = canonicalize(&json!({
            "request_digest": fixture_approval["request_digest"],
            "decision_digest": fixture_approval["decision_digest"],
            "occurrence_id": occurrence_id(binding),
            "run_id": run_id(binding),
            "attempt": 1,
            "fence_generation": 4,
        }))
        .unwrap();
        if state.consumed_occurrences.contains(&key) {
            bits |= binding.bit();
        }
    }
    bits
}

/// Compares one step; returns a description of any disagreement.
fn compare(
    fixture_approval: &Value,
    chain: Chain,
    event: Event,
    model: Result<Chain, Reasons>,
    implementation: &Result<LifecycleState, AuthorityError>,
) -> Option<String> {
    let context = || format!("at {chain:?}\n  event: {}", describe(event));
    match (model, implementation) {
        (Ok(next), Ok(state)) => {
            let same = state.state == next.state.as_str()
                && state.sequence == i64::from(next.sequence)
                && state.consumption_count == u64::from(next.count)
                && consumed_bits(fixture_approval, state) == next.consumed;
            (!same).then(|| {
                format!(
                    "{}\n  model {next:?}\n  implementation {state:?}",
                    context()
                )
            })
        }
        (Ok(next), Err(error)) => Some(format!(
            "{}\n  model accepts {next:?}\n  implementation refuses {error}",
            context()
        )),
        (Err(reasons), Ok(state)) => Some(format!(
            "{}\n  model refuses {:?}\n  implementation accepts {}",
            context(),
            reasons.iter().collect::<Vec<_>>(),
            state.state
        )),
        (Err(reasons), Err(error)) => {
            let named = reasons
                .iter()
                .any(|reason| codes(reason).contains(&error.code));
            (!named).then(|| {
                format!(
                    "{}\n  model reasons {:?}\n  implementation first code {}",
                    context(),
                    reasons.iter().collect::<Vec<_>>(),
                    error.code
                )
            })
        }
    }
}

/// Every single-claim perturbation of `event`, including the full
/// `(phase, disposition)` product.
fn perturbations(event: Event) -> Vec<Event> {
    let mut out = Vec::new();
    for kind in Kind::ALL {
        out.push(Event { kind, ..event });
    }
    for from in State::ALL {
        out.push(Event { from, ..event });
    }
    for to in State::ALL {
        out.push(Event { to, ..event });
    }
    for sequence in [
        SequenceClaim::Expected,
        SequenceClaim::Stale,
        SequenceClaim::Skipped,
    ] {
        out.push(Event { sequence, ..event });
    }
    for previous in [PreviousClaim::Head, PreviousClaim::Other] {
        out.push(Event { previous, ..event });
    }
    for subject in [SubjectClaim::Same, SubjectClaim::Different] {
        out.push(Event { subject, ..event });
    }
    for digest in [DigestClaim::Null, DigestClaim::Matching, DigestClaim::Other] {
        out.push(Event { digest, ..event });
    }
    out.push(Event {
        approval_supplied: !event.approval_supplied,
        ..event
    });
    for consumption in std::iter::once(None).chain(Binding::ALL.map(Some)) {
        out.push(Event {
            consumption,
            ..event
        });
    }
    for kind in [NoLaunch::Rejected, NoLaunch::Expired] {
        for matches_approval in [true, false] {
            out.push(Event {
                occurrence_disposition: Some(OccurrenceDisposition {
                    kind,
                    matches_approval,
                }),
                ..event
            });
        }
    }
    out.push(Event {
        occurrence_disposition: None,
        ..event
    });
    for phase in Phase::ALL {
        for disposition in Disposition::ALL {
            out.push(Event {
                phase,
                disposition,
                ..event
            });
        }
    }
    out
}

/// The well-formed event of `kind` from `chain`'s state: the expected
/// sequence, head and subject, the approval digest exactly when the chain
/// is approved or the event approves, the kind's own dispositions, and the
/// lowest occurrence not yet consumed.
fn well_formed(kind: Kind, chain: Chain, approval: Approval) -> Event {
    let (phase, disposition) = consistent_disposition(kind);
    let to = match kind {
        Kind::Request => State::Requested,
        Kind::Approve => State::Approved,
        Kind::Reject => State::Rejected,
        Kind::Expire => State::Expired,
        Kind::Revoke => State::Revoked,
        Kind::Consume => coven_threads_authority_model::lifecycle::consume_target(approval.use_),
    };
    let fresh = Binding::ALL
        .into_iter()
        .find(|b| chain.consumed & b.bit() == 0)
        .unwrap_or(Binding { occurrence: 0 });
    Event {
        kind,
        from: chain.state,
        to,
        sequence: SequenceClaim::Expected,
        previous: PreviousClaim::Head,
        subject: SubjectClaim::Same,
        digest: if chain.approved || kind == Kind::Approve {
            DigestClaim::Matching
        } else {
            DigestClaim::Null
        },
        approval_supplied: true,
        phase,
        disposition,
        consumption: (kind == Kind::Consume).then_some(fresh),
        occurrence_disposition: match kind {
            Kind::Reject => Some(OccurrenceDisposition {
                kind: NoLaunch::Rejected,
                matches_approval: true,
            }),
            Kind::Expire => Some(OccurrenceDisposition {
                kind: NoLaunch::Expired,
                matches_approval: true,
            }),
            _ => None,
        },
    }
}

/// A fixed-seed xorshift, so the sampled cases are the same every run.
struct XorShift(u64);

impl XorShift {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

struct Report {
    cases: u64,
    accepted: u64,
    mismatches: Vec<String>,
    codes: HashSet<&'static str>,
}

/// Runs the differential with `variant` standing in for the model.
fn differential(variant: Variant, samples_per_chain: usize) -> Report {
    let fixture = Fixture::new();
    let space = authority_core_space();
    let mut report = Report {
        cases: 0,
        accepted: 0,
        mismatches: Vec::new(),
        codes: HashSet::new(),
    };
    let mut serial = 0;
    let mut rng = XorShift(0x9e37_79b9_7f4a_7c15);
    let approvals = std::iter::once(Approval {
        use_: Use::SingleUse,
    })
    .chain((1..=MAX_USES_BOUND).map(|max_uses| Approval {
        use_: Use::Recurring { max_uses },
    }));
    for approval in approvals {
        let concrete_approval = fixture.approval(approval);
        // The faithful exploration supplies the reachable chains and their
        // witnesses; a broken variant is then compared from each of them.
        let exploration = explore(Variant::Faithful, approval, &space);
        for (chain, witness) in &exploration.reachable {
            // Rebuild the chain in the implementation.
            let mut head: Option<LifecycleState> = None;
            let mut model_chain = Chain::EMPTY;
            for event in witness {
                serial += 1;
                let state = run(&fixture, concrete_approval, head.as_ref(), *event, serial)
                    .unwrap_or_else(|error| {
                        panic!("witness step refused: {error}\n  {}", describe(*event))
                    });
                model_chain = apply(Variant::Faithful, model_chain, approval, *event).unwrap();
                head = Some(state);
            }
            assert_eq!(model_chain, *chain);

            let accepted_here: Vec<Event> = space
                .iter()
                .copied()
                .filter(|event| apply(Variant::Faithful, *chain, approval, *event).is_ok())
                .collect();
            let mut cases: Vec<Event> = accepted_here
                .iter()
                .flat_map(|e| perturbations(*e))
                .collect();
            // Also a well-formed event of every kind from the current state,
            // so chains that accept nothing of a kind (terminal, exhausted,
            // not yet approved) are probed where the guards live.
            for kind in Kind::ALL {
                cases.extend(perturbations(well_formed(kind, *chain, approval)));
            }
            for _ in 0..samples_per_chain {
                cases.push(space[(rng.next() % space.len() as u64) as usize]);
            }
            for event in cases {
                serial += 1;
                let model = apply(variant, *chain, approval, event);
                let implementation = run(&fixture, concrete_approval, head.as_ref(), event, serial);
                report.cases += 1;
                match &implementation {
                    Ok(_) => report.accepted += 1,
                    Err(error) => {
                        report.codes.insert(error.code.as_str());
                    }
                }
                if let Some(mismatch) =
                    compare(concrete_approval, *chain, event, model, &implementation)
                {
                    report
                        .mismatches
                        .push(format!("{:?}: {mismatch}", approval.use_));
                }
            }
        }
    }
    report
}

fn samples() -> usize {
    std::env::var("AUTHORITY_MODEL_DIFF_SAMPLES")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(300)
}

#[test]
fn model_and_implementation_agree() {
    let started = Instant::now();
    let report = differential(Variant::Faithful, samples());
    let mut seen: Vec<_> = report.codes.iter().collect();
    seen.sort();
    eprintln!(
        "cases={} accepted={} mismatches={} distinct_codes={} elapsed={:?}\ncodes: {seen:?}",
        report.cases,
        report.accepted,
        report.mismatches.len(),
        report.codes.len(),
        started.elapsed()
    );
    assert!(report.cases > 10_000 && report.accepted > 0);
    // Every lifecycle chain code the model names is exercised.
    for reason in Reason::ALL {
        assert!(
            codes(reason)
                .iter()
                .any(|code| report.codes.contains(code.as_str())),
            "no case surfaced {reason:?}"
        );
    }
    assert!(
        report.mismatches.is_empty(),
        "{} mismatches; first five:\n{}",
        report.mismatches.len(),
        report
            .mismatches
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn differential_detects_a_broken_model() {
    // Negative control for the differential itself: substituting erratum 1's
    // relation for the model must produce mismatches, because the
    // implementation refuses the consumes that relation accepts.
    let report = differential(Variant::Erratum1ConsumeChecksTargetOnly, 0);
    assert!(
        report
            .mismatches
            .iter()
            .any(|m| m.contains("model accepts") && m.contains("lifecycle_transition_invalid")),
        "no mismatch detected among {} cases",
        report.cases
    );
    eprintln!(
        "broken-model mismatches={} of cases={}\nfirst:\n{}",
        report.mismatches.len(),
        report.cases,
        report.mismatches[0]
    );
}

#[test]
fn consume_on_an_empty_chain_is_refused() {
    // Owner-local regression for the model's shortest erratum-1
    // counterexample: a recurring consume as the first event, with no
    // request or approve. The pre-erratum reference (f5dc609^) accepted it;
    // erratum 1's vector covers only a consume after revocation.
    let fixture = Fixture::new();
    let approval = &fixture.recurring[1];
    let event = Event {
        kind: Kind::Consume,
        from: State::Required,
        to: State::Approved,
        sequence: SequenceClaim::Expected,
        previous: PreviousClaim::Head,
        subject: SubjectClaim::Same,
        digest: DigestClaim::Null,
        approval_supplied: true,
        phase: Phase::Dispatching,
        disposition: Disposition::LaunchAuthorized,
        consumption: Some(Binding { occurrence: 0 }),
        occurrence_disposition: None,
    };
    let error = run(&fixture, approval, None, event, 1).unwrap_err();
    assert_eq!(error.code, ErrorCode::LifecycleTransitionInvalid);
    // The same event with the approval digest claimed is premature.
    let error = run(
        &fixture,
        approval,
        None,
        Event {
            digest: DigestClaim::Matching,
            ..event
        },
        2,
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::LifecycleApprovalPremature);
}
