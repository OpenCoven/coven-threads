//! Decisions (`validateDecision`, `verifyDecisionBundle` and `consumeDecision`
//! in the reference).

use serde_json::Value;

use super::canonical::{canonical_digest, canonicalize};
use super::evaluate::{evaluate_authorization, DECISION_VERSION};
use super::keys::{verify_signed_artifact, Keyring, SignatureVerifier};
use super::request::{
    capability_risk, validate_capabilities, validate_data, validate_evidence_scopes,
    validate_integrity_shape, validate_scope, validate_versions,
};
use super::shape::{self, exact_fields};
use super::{fail, AuthorityResult, Domain, ErrorCode};

const DECISION_FIELDS: [&str; 21] = [
    "schema_version",
    "decision_id",
    "request_id",
    "request_digest",
    "correlation",
    "bindings",
    "outcome",
    "granted_capabilities",
    "denied_capabilities",
    "degraded_capabilities",
    "scopes",
    "validity",
    "approval_requirement",
    "versions",
    "reason_codes",
    "producer",
    "issued_at",
    "recorded_at",
    "replay",
    "privacy",
    "integrity",
];

const BINDING_FIELDS: [&str; 13] = [
    "principal_id",
    "familiar_id",
    "familiar_embodiment_digest",
    "automation_id",
    "definition_revision",
    "definition_digest",
    "action_digest",
    "project_id",
    "workspace_id",
    "runtime_id",
    "runtime_descriptor_digest",
    "runtime_capabilities",
    "previous_approval_digest",
];

/// `validateDecision`: a well-formed decision signed by a `threads_authority`
/// key.
///
/// # Errors
///
/// The first failed check's code, in the reference's order.
pub fn validate_decision(
    value: &Value,
    keyring: &Keyring,
    verifier: &dyn SignatureVerifier,
) -> AuthorityResult<()> {
    let path = "$.decision";
    shape::object(Some(value), path)?;
    exact_fields(Some(value), &DECISION_FIELDS, path)?;
    let field = |name: &str| value.get(name);
    shape::exact(
        field("schema_version"),
        DECISION_VERSION,
        ErrorCode::SchemaUnknownVersion,
        "$.decision.schema_version",
    )?;
    shape::string(field("decision_id"), "$.decision.decision_id")?;
    shape::string(field("request_id"), "$.decision.request_id")?;
    shape::digest_hex(field("request_digest"), "$.decision.request_digest", true)?;

    let correlation = field("correlation");
    let correlated = |name: &str| correlation.and_then(|c| c.get(name));
    exact_fields(
        correlation,
        &["occurrence_id", "run_id", "attempt", "fence_generation"],
        "$.decision.correlation",
    )?;
    shape::string(
        correlated("occurrence_id"),
        "$.decision.correlation.occurrence_id",
    )?;
    shape::string(correlated("run_id"), "$.decision.correlation.run_id")?;
    shape::integer(correlated("attempt"), "$.decision.correlation.attempt", 1)?;
    shape::integer(
        correlated("fence_generation"),
        "$.decision.correlation.fence_generation",
        1,
    )?;

    let bindings = field("bindings");
    let bound = |name: &str| bindings.and_then(|b| b.get(name));
    exact_fields(bindings, &BINDING_FIELDS, "$.decision.bindings")?;
    for name in [
        "principal_id",
        "familiar_id",
        "automation_id",
        "project_id",
        "workspace_id",
        "runtime_id",
    ] {
        shape::string(bound(name), &format!("$.decision.bindings.{name}"))?;
    }
    for name in [
        "familiar_embodiment_digest",
        "definition_digest",
        "action_digest",
        "runtime_descriptor_digest",
    ] {
        shape::digest_hex(bound(name), &format!("$.decision.bindings.{name}"), true)?;
    }
    shape::integer(
        bound("definition_revision"),
        "$.decision.bindings.definition_revision",
        1,
    )?;
    validate_capabilities(
        bound("runtime_capabilities"),
        "$.decision.bindings.runtime_capabilities",
    )?;
    let previous = bound("previous_approval_digest");
    if !matches!(previous, Some(Value::Null)) {
        shape::digest_hex(
            previous,
            "$.decision.bindings.previous_approval_digest",
            true,
        )?;
    }
    let outcome = shape::enumeration(
        field("outcome"),
        &[
            "permit",
            "requires_approval",
            "degrade_to_proposal",
            "reject",
        ],
        "$.decision.outcome",
    )?;
    let granted = capabilities_or_empty(
        field("granted_capabilities"),
        "$.decision.granted_capabilities",
    )?;
    let bound_runtime = bound("runtime_capabilities")
        .and_then(Value::as_array)
        .expect("validated");
    if granted
        .iter()
        .any(|capability| !bound_runtime.contains(capability))
    {
        return fail(
            ErrorCode::DecisionRuntimeCapabilityMissing,
            "decision grants a capability absent from its bound runtime",
            "$",
        );
    }
    capabilities_or_empty(
        field("degraded_capabilities"),
        "$.decision.degraded_capabilities",
    )?;
    let denied = shape::array(
        field("denied_capabilities"),
        "$.decision.denied_capabilities",
        false,
        false,
    )?;
    for (index, denial) in denied.iter().enumerate() {
        let path = format!("$.decision.denied_capabilities[{index}]");
        exact_fields(Some(denial), &["capability", "reason_code"], &path)?;
        let capability = shape::string(denial.get("capability"), &format!("{path}.capability"))?;
        if capability_risk(capability).is_none() {
            return fail(
                ErrorCode::CapabilityUnknown,
                capability,
                format!("{path}.capability"),
            );
        }
        shape::string(denial.get("reason_code"), &format!("{path}.reason_code"))?;
    }
    let scopes = shape::array(field("scopes"), "$.decision.scopes", false, false)?;
    for (index, scope) in scopes.iter().enumerate() {
        validate_scope(scope, &format!("$.decision.scopes[{index}]"))?;
    }
    validate_evidence_scopes(granted, scopes, bound("principal_id"), "$.decision.scopes")?;

    let validity = field("validity");
    exact_fields(
        validity,
        &["not_before", "not_after"],
        "$.decision.validity",
    )?;
    let not_before = shape::timestamp(
        validity.and_then(|v| v.get("not_before")),
        "$.decision.validity.not_before",
    )?;
    let not_after = shape::timestamp(
        validity.and_then(|v| v.get("not_after")),
        "$.decision.validity.not_after",
    )?;
    if not_after <= not_before {
        return fail(
            ErrorCode::DecisionInvalidInterval,
            "decision expiry must follow its start",
            "$",
        );
    }
    match field("approval_requirement") {
        Some(Value::Null) => {
            if outcome == "requires_approval" {
                return fail(
                    ErrorCode::DecisionApprovalProfileMissing,
                    "approval outcome must name an approval profile",
                    "$",
                );
            }
        }
        requirement => {
            exact_fields(
                requirement,
                &["profile", "recurring_allowed"],
                "$.decision.approval_requirement",
            )?;
            let profile = shape::enumeration(
                requirement.and_then(|r| r.get("profile")),
                &["human_per_run", "protected_owner_per_run"],
                "$.decision.approval_requirement.profile",
            )?;
            let recurring = shape::boolean(
                requirement.and_then(|r| r.get("recurring_allowed")),
                "$.decision.approval_requirement.recurring_allowed",
            )?;
            if profile == "protected_owner_per_run" && recurring {
                return fail(
                    ErrorCode::DecisionRecurringApprovalForbidden,
                    "protected-owner approval is always per-run",
                    "$",
                );
            }
            if outcome != "requires_approval" {
                return fail(
                    ErrorCode::DecisionApprovalProfileUnexpected,
                    "non-approval outcome carries approval policy",
                    "$",
                );
            }
        }
    }
    validate_versions(field("versions"), "$.decision.versions")?;
    let reasons = shape::array(field("reason_codes"), "$.decision.reason_codes", true, true)?;
    for (index, reason) in reasons.iter().enumerate() {
        shape::string(Some(reason), &format!("$.decision.reason_codes[{index}]"))?;
    }
    let producer = field("producer");
    exact_fields(producer, &["id", "verifier_profile"], "$.decision.producer")?;
    shape::exact(
        producer.and_then(|p| p.get("id")),
        "coven-threads",
        ErrorCode::DecisionProducerForged,
        "$.decision.producer.id",
    )?;
    shape::exact(
        producer.and_then(|p| p.get("verifier_profile")),
        "automation-authority/1.0.0",
        ErrorCode::ProfileVersionUnknown,
        "$.decision.producer.verifier_profile",
    )?;
    shape::timestamp(field("issued_at"), "$.decision.issued_at")?;
    shape::timestamp(field("recorded_at"), "$.decision.recorded_at")?;
    let replay = field("replay");
    exact_fields(
        replay,
        &["dispatches", "consumption_required"],
        "$.decision.replay",
    )?;
    if replay
        .and_then(|r| r.get("dispatches"))
        .and_then(Value::as_f64)
        != Some(1.0)
    {
        return fail(
            ErrorCode::DecisionReplayPolicyInvalid,
            "expected 1",
            "$.decision.replay.dispatches",
        );
    }
    shape::boolean(
        replay.and_then(|r| r.get("consumption_required")),
        "$.decision.replay.consumption_required",
    )?;
    validate_data(field("privacy"), "$.decision.privacy")?;
    validate_integrity_shape(field("integrity"), "$.decision.integrity")?;
    verify_signed_artifact(
        value,
        Domain::Decision,
        keyring,
        Some("threads_authority"),
        verifier,
    )?;
    Ok(())
}

/// `validateCapabilitiesOrEmpty`: a unique list of known capabilities.
fn capabilities_or_empty<'a>(value: Option<&'a Value>, path: &str) -> AuthorityResult<&'a [Value]> {
    let items = shape::array(value, path, false, true)?;
    for (index, item) in items.iter().enumerate() {
        let capability = shape::string(Some(item), &format!("{path}[{index}]"))?;
        if capability_risk(capability).is_none() {
            return fail(ErrorCode::CapabilityUnknown, capability, path);
        }
    }
    Ok(items)
}

/// `verifyDecisionBundle`: `decision` is a valid signed decision, and exactly
/// the decision the profile computes for `request` under `snapshot`. Returns
/// its outcome.
///
/// # Errors
///
/// A decision or request validation code, an evaluation code, or
/// `decision_semantic_mismatch`.
pub fn verify_decision_bundle(
    request: &Value,
    decision: &Value,
    snapshot: &Value,
    keyring: &Keyring,
    verifier: &dyn SignatureVerifier,
) -> AuthorityResult<String> {
    validate_decision(decision, keyring, verifier)?;
    let expected = evaluate_authorization(request, snapshot, keyring, verifier)?;
    let mut unsigned = decision.clone();
    if let Some(object) = unsigned.as_object_mut() {
        object.remove("integrity");
    }
    if canonicalize(&unsigned)? != canonicalize(&expected)? {
        return fail(
            ErrorCode::DecisionSemanticMismatch,
            "signed decision does not match the reference policy evaluation",
            "$",
        );
    }
    Ok(decision["outcome"].as_str().unwrap_or_default().to_owned())
}

/// What a series of decision consumptions has seen.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConsumptionState {
    /// Consumed decision ids.
    pub decision_ids: Vec<String>,
    /// Consumed decision digests, `sha256:`-prefixed.
    pub decision_digests: Vec<String>,
}

/// `consumeDecision`: a valid `permit` or `requires_approval` decision, used
/// at most once.
///
/// # Errors
///
/// A decision validation code, `decision_not_consumable` or
/// `decision_replayed`.
pub fn consume_decision(
    decision: &Value,
    state: Option<&ConsumptionState>,
    keyring: &Keyring,
    verifier: &dyn SignatureVerifier,
) -> AuthorityResult<ConsumptionState> {
    validate_decision(decision, keyring, verifier)?;
    let outcome = decision["outcome"].as_str().unwrap_or_default();
    if !matches!(outcome, "permit" | "requires_approval") {
        return fail(
            ErrorCode::DecisionNotConsumable,
            format!("{outcome} decisions cannot authorize dispatch"),
            "$",
        );
    }
    let mut state = state.cloned().unwrap_or_default();
    let digest = format!(
        "sha256:{}",
        canonical_digest(decision, Domain::Decision.as_str())?
    );
    let decision_id = decision["decision_id"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    if state.decision_ids.contains(&decision_id) || state.decision_digests.contains(&digest) {
        return fail(
            ErrorCode::DecisionReplayed,
            "decision was already consumed",
            "$",
        );
    }
    state.decision_ids.push(decision_id);
    state.decision_digests.push(digest);
    Ok(state)
}
