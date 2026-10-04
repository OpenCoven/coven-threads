//! Final dispatch verification (`validateDispatchSnapshot` and
//! `verifyDispatch` in the reference): everything a launch rests on, checked
//! together against the trusted dispatch-time snapshot.

use serde_json::{json, Value};

use super::approval::approval_key;
use super::canonical::{canonical_digest, canonicalize};
use super::decision::{validate_decision, verify_decision_bundle};
use super::keys::{Keyring, SignatureVerifier};
use super::lifecycle::verify_lifecycle_chain;
use super::request::{validate_authorization_request, validate_capabilities};
use super::shape::{self, exact_fields};
use super::snapshot::validate_consumption_snapshot;
use super::{fail, AuthorityResult, Domain, ErrorCode};

/// The domain of the digest that binds a dispatch to its snapshot.
const DISPATCH_BINDING_DOMAIN: &str = "opencoven:automation-dispatch-binding:v1";

const BUNDLE_FIELDS: [&str; 8] = [
    "request",
    "decision",
    "approval",
    "approval_authorization_request",
    "approval_authorization_decision",
    "lifecycle_events",
    "consumption_snapshot",
    "snapshot",
];

const SNAPSHOT_FIELDS: [&str; 24] = [
    "now",
    "principal_id",
    "familiar_id",
    "familiar_embodiment_digest",
    "automation_id",
    "definition_revision",
    "definition_digest",
    "occurrence_id",
    "run_id",
    "attempt",
    "fence_generation",
    "action_digest",
    "runtime_id",
    "runtime_descriptor_digest",
    "runtime_capabilities",
    "project_id",
    "workspace_id",
    "policy",
    "policy_digest",
    "manifest",
    "manifest_digest",
    "consumption_revision",
    "policy_snapshot",
    "approval_authorization_policy_snapshot",
];

/// What a verified dispatch must record when it launches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchAuthorization {
    /// The digest binding the request, the decision and the dispatch
    /// snapshot, `sha256:`-prefixed.
    pub dispatch_binding_digest: String,
    /// The consumption the launch must commit atomically.
    pub required_consumption: RequiredConsumption,
}

/// The consumption a verified dispatch must commit atomically with its launch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequiredConsumption {
    /// The decision to mark consumed, `sha256:`-prefixed.
    pub decision_digest: String,
    /// The approval whose usage advances, when the decision required one.
    pub approval_id: Option<String>,
    /// The approval's committed usage count before this dispatch.
    pub approval_usage_before: Option<u64>,
}

/// `validateDispatchSnapshot`.
fn validate_dispatch_snapshot(snapshot: &Value) -> AuthorityResult<()> {
    let path = "$.dispatch_bundle.snapshot";
    shape::object(Some(snapshot), path)?;
    exact_fields(Some(snapshot), &SNAPSHOT_FIELDS, path)?;
    let field = |name: &str| snapshot.get(name);
    let at = |name: &str| format!("{path}.{name}");
    shape::timestamp(field("now"), &at("now"))?;
    for name in [
        "principal_id",
        "familiar_id",
        "automation_id",
        "occurrence_id",
        "run_id",
        "runtime_id",
        "project_id",
        "workspace_id",
        "policy",
        "manifest",
    ] {
        shape::string(field(name), &at(name))?;
    }
    for name in [
        "familiar_embodiment_digest",
        "definition_digest",
        "action_digest",
        "runtime_descriptor_digest",
        "policy_digest",
        "manifest_digest",
    ] {
        shape::digest_hex(field(name), &at(name), true)?;
    }
    for name in [
        "definition_revision",
        "attempt",
        "fence_generation",
        "consumption_revision",
    ] {
        shape::integer(field(name), &at(name), 1)?;
    }
    validate_capabilities(field("runtime_capabilities"), &at("runtime_capabilities"))?;
    shape::object(field("policy_snapshot"), &at("policy_snapshot"))?;
    let approval_policy = field("approval_authorization_policy_snapshot");
    if approval_policy != Some(&Value::Null) {
        shape::object(
            approval_policy,
            &at("approval_authorization_policy_snapshot"),
        )?;
    }
    Ok(())
}

/// `assertBinding`: `actual === expected`, or `code`.
fn assert_binding(
    name: &str,
    actual: &Value,
    expected: &Value,
    code: ErrorCode,
) -> AuthorityResult<()> {
    if shape::strict_equal(Some(actual), Some(expected)) {
        Ok(())
    } else {
        fail(code, format!("{name} changed after authorization"), "$")
    }
}

/// `sameStringSet`: the two capability lists, each sorted, are equal.
fn same_string_set(left: &Value, right: &Value) -> AuthorityResult<bool> {
    let sorted = |value: &Value| -> AuthorityResult<String> {
        let mut items: Vec<Value> = value.as_array().cloned().unwrap_or_default();
        items.sort_by(|left, right| {
            shape::js_string(Some(left))
                .encode_utf16()
                .cmp(shape::js_string(Some(right)).encode_utf16())
        });
        canonicalize(&Value::Array(items))
    };
    Ok(sorted(left)? == sorted(right)?)
}

/// `verifyDispatch`: the final check before a launch. `bundle` is the
/// reference's closed object of `request`, `decision`, `approval`,
/// `approval_authorization_request`, `approval_authorization_decision`,
/// `lifecycle_events`, `consumption_snapshot` and `snapshot`, with `null` for
/// an absent approval artifact and `[]` for no lifecycle events.
///
/// The request and its decision must verify, be adopted and unconsumed in the
/// signed consumption snapshot, and bind exactly the dispatch-time snapshot.
/// A `requires_approval` decision also needs a valid approval of the right
/// role bound to the same operation, with an authenticated lifecycle chain in
/// the `approved` state whose head the consumption snapshot commits; a
/// recurring approval needs the grant request, decision and policy snapshot
/// that authorized it.
///
/// # Errors
///
/// The first failed check's code, in the reference's order.
pub fn verify_dispatch(
    bundle: &Value,
    keyring: &Keyring,
    verifier: &dyn SignatureVerifier,
) -> AuthorityResult<DispatchAuthorization> {
    shape::object(Some(bundle), "$.dispatch_bundle")?;
    exact_fields(Some(bundle), &BUNDLE_FIELDS, "$.dispatch_bundle")?;
    let request = &bundle["request"];
    let decision = &bundle["decision"];
    let approval = &bundle["approval"];
    let authorization_request = &bundle["approval_authorization_request"];
    let authorization_decision = &bundle["approval_authorization_decision"];
    let lifecycle_events = &bundle["lifecycle_events"];
    let consumption_snapshot = &bundle["consumption_snapshot"];
    let snapshot = &bundle["snapshot"];
    shape::array(
        Some(lifecycle_events),
        "$.dispatch_bundle.lifecycle_events",
        false,
        false,
    )?;
    validate_dispatch_snapshot(snapshot)?;
    validate_authorization_request(request, keyring, verifier)?;
    validate_decision(decision, keyring, verifier)?;
    let runtime = &request["context"]["runtime"];
    if !same_string_set(
        &decision["bindings"]["runtime_capabilities"],
        &runtime["capabilities"],
    )? {
        return fail(
            ErrorCode::DecisionRuntimeCapabilitiesChanged,
            "decision runtime capabilities do not exactly match the request",
            "$",
        );
    }
    if !same_string_set(&snapshot["runtime_capabilities"], &runtime["capabilities"])? {
        return fail(
            ErrorCode::DispatchRuntimeCapabilitiesChanged,
            "dispatch runtime capabilities do not exactly match the request",
            "$",
        );
    }
    verify_decision_bundle(
        request,
        decision,
        &snapshot["policy_snapshot"],
        keyring,
        verifier,
    )?;
    let now = shape::millis(snapshot.get("now"));
    let replay = &request["replay"];
    if now < shape::millis(replay.get("issued_at")) {
        return fail(
            ErrorCode::RequestNotYetValid,
            "dispatch request has not reached its issue time",
            "$",
        );
    }
    if now >= shape::millis(replay.get("expires_at")) {
        return fail(
            ErrorCode::RequestExpired,
            "dispatch request has expired",
            "$",
        );
    }
    validate_consumption_snapshot(consumption_snapshot, keyring, verifier, snapshot.get("now"))?;
    assert_binding(
        "consumption store revision",
        &consumption_snapshot["store_revision"],
        &snapshot["consumption_revision"],
        ErrorCode::ConsumptionSnapshotStale,
    )?;

    let request_digest = format!(
        "sha256:{}",
        canonical_digest(request, Domain::Request.as_str())?
    );
    let adoptions = consumption_snapshot["request_adoptions"]
        .as_array()
        .expect("validated");
    if adoptions.iter().any(|adoption| {
        (adoption["nonce"] == replay["nonce"] || adoption["adoption_key"] == replay["adoption_key"])
            && adoption["request_digest"].as_str() != Some(request_digest.as_str())
    }) {
        return fail(
            ErrorCode::RequestReplayed,
            "nonce or adoption key was previously used by another request",
            "$",
        );
    }
    let matching = adoptions
        .iter()
        .filter(|adoption| {
            adoption["request_digest"].as_str() == Some(request_digest.as_str())
                && adoption["nonce"] == replay["nonce"]
                && adoption["adoption_key"] == replay["adoption_key"]
        })
        .count();
    if matching == 0 {
        return fail(
            ErrorCode::RequestNotAdopted,
            "dispatch request has no authenticated adoption evidence",
            "$",
        );
    }
    if matching != 1 {
        return fail(
            ErrorCode::RequestReplayed,
            "dispatch request has duplicate adoption evidence",
            "$",
        );
    }
    let decision_digest = format!(
        "sha256:{}",
        canonical_digest(decision, Domain::Decision.as_str())?
    );
    if consumption_snapshot["decision_consumptions"]
        .as_array()
        .expect("validated")
        .iter()
        .any(|digest| digest.as_str() == Some(decision_digest.as_str()))
    {
        return fail(
            ErrorCode::DecisionReplayed,
            "dispatch decision was already consumed",
            "$",
        );
    }
    let request_digest_value = Value::String(request_digest.clone());
    assert_binding(
        "request digest",
        &decision["request_digest"],
        &request_digest_value,
        ErrorCode::DecisionRequestMismatch,
    )?;
    assert_binding(
        "request id",
        &decision["request_id"],
        &request["request_id"],
        ErrorCode::DecisionRequestMismatch,
    )?;
    let execution = &request["execution"];
    let correlation = &decision["correlation"];
    for (name, member) in [
        ("decision occurrence", "occurrence_id"),
        ("decision run", "run_id"),
        ("decision attempt", "attempt"),
        ("decision fence", "fence_generation"),
    ] {
        assert_binding(
            name,
            &correlation[member],
            &execution[member],
            ErrorCode::DecisionBindingMismatch,
        )?;
    }
    let bindings = &decision["bindings"];
    let expected = [
        ("principal_id", &request["principal"]["id"]),
        ("familiar_id", &request["familiar"]["id"]),
        (
            "familiar_embodiment_digest",
            &request["familiar"]["embodiment_digest"],
        ),
        ("automation_id", &request["automation"]["id"]),
        (
            "definition_revision",
            &request["automation"]["definition_revision"],
        ),
        (
            "definition_digest",
            &request["automation"]["definition_digest"],
        ),
        ("action_digest", &request["action"]["digest"]),
        ("project_id", &request["context"]["project_id"]),
        ("workspace_id", &request["context"]["workspace_id"]),
        ("runtime_id", &runtime["id"]),
        ("runtime_descriptor_digest", &runtime["descriptor_digest"]),
        (
            "previous_approval_digest",
            &request["previous_approval_digest"],
        ),
    ];
    for (name, value) in expected {
        assert_binding(
            name,
            &bindings[name],
            value,
            ErrorCode::DecisionBindingMismatch,
        )?;
    }

    let checks = [
        (
            "principal",
            "principal_id",
            &request["principal"]["id"],
            ErrorCode::DispatchPrincipalMismatch,
        ),
        (
            "familiar",
            "familiar_id",
            &request["familiar"]["id"],
            ErrorCode::DispatchFamiliarMismatch,
        ),
        (
            "familiar embodiment",
            "familiar_embodiment_digest",
            &request["familiar"]["embodiment_digest"],
            ErrorCode::DispatchFamiliarChanged,
        ),
        (
            "automation",
            "automation_id",
            &request["automation"]["id"],
            ErrorCode::DispatchAutomationMismatch,
        ),
        (
            "definition revision",
            "definition_revision",
            &request["automation"]["definition_revision"],
            ErrorCode::DispatchDefinitionChanged,
        ),
        (
            "definition digest",
            "definition_digest",
            &request["automation"]["definition_digest"],
            ErrorCode::DispatchDefinitionChanged,
        ),
        (
            "occurrence",
            "occurrence_id",
            &execution["occurrence_id"],
            ErrorCode::DispatchOccurrenceMismatch,
        ),
        (
            "run",
            "run_id",
            &execution["run_id"],
            ErrorCode::DispatchRunMismatch,
        ),
        (
            "attempt",
            "attempt",
            &execution["attempt"],
            ErrorCode::DispatchAttemptMismatch,
        ),
        (
            "fence",
            "fence_generation",
            &execution["fence_generation"],
            ErrorCode::DispatchStaleFence,
        ),
        (
            "action",
            "action_digest",
            &request["action"]["digest"],
            ErrorCode::DispatchActionChanged,
        ),
        (
            "project",
            "project_id",
            &request["context"]["project_id"],
            ErrorCode::DispatchProjectMismatch,
        ),
        (
            "workspace",
            "workspace_id",
            &request["context"]["workspace_id"],
            ErrorCode::DispatchWorkspaceMismatch,
        ),
        (
            "runtime id",
            "runtime_id",
            &runtime["id"],
            ErrorCode::DispatchRuntimeChanged,
        ),
        (
            "runtime descriptor",
            "runtime_descriptor_digest",
            &runtime["descriptor_digest"],
            ErrorCode::DispatchRuntimeChanged,
        ),
        (
            "policy",
            "policy",
            &request["versions"]["policy"],
            ErrorCode::DispatchPolicyStale,
        ),
        (
            "policy digest",
            "policy_digest",
            &request["versions"]["policy_digest"],
            ErrorCode::DispatchPolicyStale,
        ),
        (
            "manifest",
            "manifest",
            &request["versions"]["manifest"],
            ErrorCode::DispatchManifestStale,
        ),
        (
            "manifest digest",
            "manifest_digest",
            &request["versions"]["manifest_digest"],
            ErrorCode::DispatchManifestStale,
        ),
    ];
    for (name, member, expected, code) in checks {
        assert_binding(name, &snapshot[member], expected, code)?;
    }
    let runtime_capabilities = snapshot["runtime_capabilities"]
        .as_array()
        .expect("validated");
    for capability in decision["granted_capabilities"]
        .as_array()
        .expect("validated")
    {
        if !runtime_capabilities.contains(capability) {
            return fail(
                ErrorCode::DispatchRuntimeDowngrade,
                format!(
                    "runtime no longer exposes {}",
                    capability.as_str().unwrap_or_default()
                ),
                "$",
            );
        }
    }
    let validity = &decision["validity"];
    if now < shape::millis(validity.get("not_before")) {
        return fail(
            ErrorCode::DecisionNotYetValid,
            "decision validity interval has not started",
            "$",
        );
    }
    if now >= shape::millis(validity.get("not_after")) {
        return fail(
            ErrorCode::DecisionExpired,
            "decision validity interval has ended",
            "$",
        );
    }
    match decision["outcome"].as_str() {
        Some("reject") => {
            return fail(
                ErrorCode::DispatchRejected,
                "rejected decision cannot dispatch",
                "$",
            )
        }
        Some("degrade_to_proposal") => {
            return fail(
                ErrorCode::ProposalDispatchForbidden,
                "proposal-only decision cannot dispatch protected effects",
                "$",
            )
        }
        _ => {}
    }
    let approval_policy = &snapshot["approval_authorization_policy_snapshot"];
    let mut approval_usage_before = None;
    if decision["outcome"] == "requires_approval" {
        if !shape::truthy(Some(approval)) {
            return fail(
                ErrorCode::ApprovalRequired,
                "dispatch requires operation-specific approval",
                "$",
            );
        }
        let key = approval_key(approval, keyring, verifier, snapshot.get("now"))?;
        let requirement = &decision["approval_requirement"];
        let recurring = approval["use"]["kind"] == "recurring";
        if recurring && requirement["recurring_allowed"] != true {
            return fail(
                ErrorCode::ApprovalRecurringNotAllowed,
                "decision requires a single per-run approval",
                "$",
            );
        }
        if recurring {
            verify_recurring_authorization(bundle, keyring, verifier)?;
        } else if !authorization_request.is_null()
            || !authorization_decision.is_null()
            || !approval_policy.is_null()
        {
            return fail(
                ErrorCode::ApprovalRecurringAuthorizationUnexpected,
                "single-use approval cannot carry a recurring grant decision",
                "$",
            );
        }
        match requirement["profile"].as_str() {
            Some("protected_owner_per_run") if key.role != "protected_owner" => {
                return fail(
                    ErrorCode::ApprovalRoleMismatch,
                    "protected_owner_per_run requires a protected_owner signing key",
                    "$",
                );
            }
            Some("human_per_run")
                if key.role != "principal"
                    || approval["approving_principal"]["id"] != request["principal"]["id"] =>
            {
                return fail(
                    ErrorCode::ApprovalRoleMismatch,
                    "human_per_run requires the authorized principal's signing key",
                    "$",
                );
            }
            _ => {}
        }
        let approval_expected = [
            ("familiar_id", &request["familiar"]["id"]),
            (
                "familiar_embodiment_digest",
                &request["familiar"]["embodiment_digest"],
            ),
            ("authorized_principal_id", &request["principal"]["id"]),
            ("action_digest", &request["action"]["digest"]),
            ("project_id", &request["context"]["project_id"]),
            ("workspace_id", &request["context"]["workspace_id"]),
            ("runtime_id", &runtime["id"]),
            ("runtime_descriptor_digest", &runtime["descriptor_digest"]),
        ];
        for (name, value) in approval_expected {
            assert_binding(
                name,
                &approval[name],
                value,
                ErrorCode::ApprovalBindingMismatch,
            )?;
        }
        let automation = &request["automation"];
        if approval["automation"]["id"] != automation["id"] {
            return fail(
                ErrorCode::ApprovalBindingMismatch,
                "approval automation changed",
                "$",
            );
        }
        if !shape::strict_equal(
            approval["automation"].get("definition_revision"),
            automation.get("definition_revision"),
        ) || approval["automation"]["definition_digest"] != automation["definition_digest"]
        {
            return fail(
                ErrorCode::ApprovalDefinitionChanged,
                "approval definition binding changed",
                "$",
            );
        }
        if canonicalize(&approval["capabilities"])?
            != canonicalize(&request["requested_capabilities"])?
        {
            return fail(
                ErrorCode::ApprovalCapabilitiesChanged,
                "approval capability binding changed",
                "$",
            );
        }
        if canonicalize(&approval["scopes"])? != canonicalize(&request["scopes"])? {
            return fail(
                ErrorCode::ApprovalScopesChanged,
                "approval scope binding changed",
                "$",
            );
        }
        if canonicalize(&approval["runtime_capabilities"])?
            != canonicalize(&runtime["capabilities"])?
        {
            return fail(
                ErrorCode::ApprovalRuntimeCapabilitiesChanged,
                "approval runtime capabilities changed",
                "$",
            );
        }
        if canonicalize(&approval["versions"])? != canonicalize(&request["versions"])? {
            return fail(
                ErrorCode::ApprovalPolicyChanged,
                "approval policy or manifest binding changed",
                "$",
            );
        }
        for capability in request["requested_capabilities"]
            .as_array()
            .expect("validated")
        {
            if !runtime_capabilities.contains(capability) {
                return fail(
                    ErrorCode::DispatchRuntimeCapabilityMissing,
                    format!(
                        "approval-required dispatch runtime lacks {}",
                        capability.as_str().unwrap_or_default()
                    ),
                    "$",
                );
            }
        }
        let decision_digest_value = Value::String(decision_digest.clone());
        if recurring {
            let prefix = approval["use"]["occurrence_prefix"]
                .as_str()
                .unwrap_or_default();
            if !execution["occurrence_id"]
                .as_str()
                .unwrap_or_default()
                .starts_with(prefix)
            {
                return fail(
                    ErrorCode::ApprovalOccurrenceOutOfScope,
                    "occurrence is outside recurring approval pattern",
                    "$",
                );
            }
        } else {
            let single_use_expected = [
                ("request_digest", &request_digest_value),
                ("decision_digest", &decision_digest_value),
                ("occurrence_id", &execution["occurrence_id"]),
                ("run_id", &execution["run_id"]),
                ("attempt", &execution["attempt"]),
                ("fence_generation", &execution["fence_generation"]),
            ];
            for (name, value) in single_use_expected {
                assert_binding(
                    name,
                    &approval[name],
                    value,
                    ErrorCode::ApprovalBindingMismatch,
                )?;
            }
        }
        if lifecycle_events.as_array().is_none_or(Vec::is_empty) {
            return fail(
                ErrorCode::ApprovalLifecycleRequired,
                "authenticated append-only lifecycle evidence is required",
                "$",
            );
        }
        let lifecycle = verify_lifecycle_chain(
            lifecycle_events,
            Some(approval),
            keyring,
            verifier,
            snapshot.get("now"),
        )?;
        let committed = consumption_snapshot["approval_heads"]
            .as_array()
            .expect("validated")
            .iter()
            .find(|head| head["approval_id"] == approval["approval_id"]);
        let usage = committed.and_then(|head| head["usage_count"].as_f64());
        if committed.is_none_or(|head| {
            head["head_event_digest"].as_str() != Some(lifecycle.last_event_digest.as_str())
        }) || usage != Some(lifecycle.consumption_count as f64)
        {
            return fail(
                ErrorCode::ApprovalLifecycleHeadMismatch,
                "lifecycle chain does not match authenticated consumption state",
                "$",
            );
        }
        if lifecycle.state != "approved" {
            return fail(
                ErrorCode::ApprovalStateInvalid,
                "approval is not in dispatchable approved state",
                "$",
            );
        }
        if recurring
            && lifecycle.consumption_count as f64
                >= approval["use"]["max_uses"].as_f64().unwrap_or(f64::NAN)
        {
            return fail(
                ErrorCode::ApprovalUsageExhausted,
                "recurring approval usage bound is exhausted",
                "$",
            );
        }
        let current = canonicalize(&json!({
            "request_digest": request_digest,
            "decision_digest": decision_digest,
            "occurrence_id": execution["occurrence_id"],
            "run_id": execution["run_id"],
            "attempt": execution["attempt"],
            "fence_generation": execution["fence_generation"],
        }))?;
        if lifecycle.consumed_occurrences.contains(&current) {
            return fail(
                ErrorCode::ApprovalReplayed,
                "approval already consumed for this exact dispatch",
                "$",
            );
        }
        approval_usage_before = Some(lifecycle.consumption_count);
    } else if !approval.is_null()
        || !authorization_request.is_null()
        || !authorization_decision.is_null()
        || !approval_policy.is_null()
        || lifecycle_events
            .as_array()
            .is_some_and(|events| !events.is_empty())
    {
        return fail(
            ErrorCode::ApprovalUnexpected,
            "non-approval decision cannot carry approval authority",
            "$",
        );
    }
    let binding = canonical_digest(
        &json!({
            "request_digest": request_digest,
            "decision_digest": decision_digest,
            "snapshot": snapshot,
        }),
        DISPATCH_BINDING_DOMAIN,
    )?;
    Ok(DispatchAuthorization {
        dispatch_binding_digest: format!("sha256:{binding}"),
        required_consumption: RequiredConsumption {
            decision_digest,
            approval_id: approval["approval_id"].as_str().map(ToOwned::to_owned),
            approval_usage_before,
        },
    })
}

/// The recurring-approval branch of `verifyDispatch`: the immutable grant
/// request, decision and policy snapshot that authorized recurring use, bound
/// exactly to the approval.
fn verify_recurring_authorization(
    bundle: &Value,
    keyring: &Keyring,
    verifier: &dyn SignatureVerifier,
) -> AuthorityResult<()> {
    let approval = &bundle["approval"];
    let request = &bundle["approval_authorization_request"];
    let decision = &bundle["approval_authorization_decision"];
    let policy = &bundle["snapshot"]["approval_authorization_policy_snapshot"];
    if !shape::truthy(Some(request))
        || !shape::truthy(Some(decision))
        || !shape::truthy(Some(policy))
    {
        return fail(
            ErrorCode::ApprovalRecurringAuthorizationMissing,
            "recurring approval requires its immutable grant request, decision, and policy snapshot",
            "$",
        );
    }
    validate_authorization_request(request, keyring, verifier)?;
    verify_decision_bundle(request, decision, policy, keyring, verifier)?;
    let mismatch = |message: &str| {
        fail(
            ErrorCode::ApprovalRecurringAuthorizationMismatch,
            message.to_owned(),
            "$",
        )
    };
    let request_digest = format!(
        "sha256:{}",
        canonical_digest(request, Domain::Request.as_str())?
    );
    let decision_digest = format!(
        "sha256:{}",
        canonical_digest(decision, Domain::Decision.as_str())?
    );
    if approval["request_digest"].as_str() != Some(request_digest.as_str())
        || approval["decision_digest"].as_str() != Some(decision_digest.as_str())
        || decision["request_digest"] != approval["request_digest"]
        || decision["request_id"] != request["request_id"]
        || decision["outcome"] != "requires_approval"
        || decision["approval_requirement"]["recurring_allowed"] != true
    {
        return mismatch("recurring approval grant decision does not authorize recurring use");
    }
    let execution = &request["execution"];
    let correlation = json!({
        "occurrence_id": execution["occurrence_id"],
        "run_id": execution["run_id"],
        "attempt": execution["attempt"],
        "fence_generation": execution["fence_generation"],
    });
    if canonicalize(&decision["correlation"])? != canonicalize(&correlation)? {
        return mismatch("recurring grant decision correlation does not match its request");
    }
    let runtime = &request["context"]["runtime"];
    let grant_bindings = &decision["bindings"];
    let bindings = [
        (
            "principal_id",
            &request["principal"]["id"],
            &approval["authorized_principal_id"],
        ),
        (
            "familiar_id",
            &request["familiar"]["id"],
            &approval["familiar_id"],
        ),
        (
            "familiar_embodiment_digest",
            &request["familiar"]["embodiment_digest"],
            &approval["familiar_embodiment_digest"],
        ),
        (
            "automation_id",
            &request["automation"]["id"],
            &approval["automation"]["id"],
        ),
        (
            "definition_revision",
            &request["automation"]["definition_revision"],
            &approval["automation"]["definition_revision"],
        ),
        (
            "definition_digest",
            &request["automation"]["definition_digest"],
            &approval["automation"]["definition_digest"],
        ),
        (
            "action_digest",
            &request["action"]["digest"],
            &approval["action_digest"],
        ),
        (
            "project_id",
            &request["context"]["project_id"],
            &approval["project_id"],
        ),
        (
            "workspace_id",
            &request["context"]["workspace_id"],
            &approval["workspace_id"],
        ),
        ("runtime_id", &runtime["id"], &approval["runtime_id"]),
        (
            "runtime_descriptor_digest",
            &runtime["descriptor_digest"],
            &approval["runtime_descriptor_digest"],
        ),
    ];
    for (name, requested, approved) in bindings {
        assert_binding(
            name,
            requested,
            approved,
            ErrorCode::ApprovalRecurringAuthorizationMismatch,
        )?;
        assert_binding(
            name,
            &grant_bindings[name],
            approved,
            ErrorCode::ApprovalRecurringAuthorizationMismatch,
        )?;
    }
    if canonicalize(&request["requested_capabilities"])? != canonicalize(&approval["capabilities"])?
        || canonicalize(&request["scopes"])? != canonicalize(&approval["scopes"])?
        || request["action"]["risk_class"] != "R2"
        || canonicalize(&runtime["capabilities"])?
            != canonicalize(&approval["runtime_capabilities"])?
        || canonicalize(&grant_bindings["runtime_capabilities"])?
            != canonicalize(&approval["runtime_capabilities"])?
        || canonicalize(&request["versions"])? != canonicalize(&approval["versions"])?
        || canonicalize(&decision["versions"])? != canonicalize(&approval["versions"])?
    {
        return mismatch("recurring approval exceeds or changes its grant decision");
    }
    Ok(())
}
