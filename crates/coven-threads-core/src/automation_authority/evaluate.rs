//! Authorization evaluation (`evaluateAuthorization` and `decisionBase` in the
//! reference).

use serde_json::{json, Value};

use super::canonical::{canonical_digest, canonicalize};
use super::keys::{Keyring, SignatureVerifier};
use super::request::{
    capability_risk, risk_number, validate_authorization_request, validate_capabilities,
    validate_evidence_scopes, validate_scopes, RISK_CLASSES,
};
use super::shape::{self, exact_fields};
use super::{fail, AuthorityResult, Domain, ErrorCode};

pub(crate) const DECISION_VERSION: &str = "opencoven.automation-authorization-decision/v1";

const SNAPSHOT_REQUIRED: [&str; 8] = [
    "now",
    "policy",
    "policy_digest",
    "manifest",
    "manifest_digest",
    "recurring_grants",
    "protected_owner_approval",
    "recurring_approval_allowed",
];

const GRANT_FIELDS: [&str; 20] = [
    "grant_id",
    "principal_id",
    "familiar_id",
    "familiar_embodiment_digest",
    "automation_id",
    "definition_revision",
    "definition_digest",
    "action_type",
    "action_digest",
    "project_id",
    "workspace_id",
    "runtime_id",
    "runtime_descriptor_digest",
    "runtime_capabilities",
    "risk_classes",
    "capabilities",
    "scopes",
    "expires_at",
    "max_uses",
    "uses",
];

/// `evaluateAuthorization`: the unsigned decision the profile computes for a
/// valid, signed `request` under `snapshot`. The Threads authority signs it
/// with [`super::sign_artifact`] under [`Domain::Decision`].
///
/// # Errors
///
/// A request validation code, then the first failed snapshot check, in the
/// reference's order.
pub fn evaluate_authorization(
    request: &Value,
    snapshot: &Value,
    keyring: &Keyring,
    verifier: &dyn SignatureVerifier,
) -> AuthorityResult<Value> {
    validate_authorization_request(request, keyring, verifier)?;
    let path = "$.policy_snapshot";
    shape::object(Some(snapshot), path)?;
    shape::required(Some(snapshot), &SNAPSHOT_REQUIRED, path)?;
    let mut allowed = SNAPSHOT_REQUIRED.to_vec();
    allowed.push("previous_approval_digest");
    shape::closed(Some(snapshot), &allowed, path)?;
    let field = |name: &str| snapshot.get(name);
    let now = shape::timestamp(field("now"), "$.policy_snapshot.now")?;
    shape::string(field("policy"), "$.policy_snapshot.policy")?;
    shape::digest_hex(
        field("policy_digest"),
        "$.policy_snapshot.policy_digest",
        true,
    )?;
    shape::string(field("manifest"), "$.policy_snapshot.manifest")?;
    shape::digest_hex(
        field("manifest_digest"),
        "$.policy_snapshot.manifest_digest",
        true,
    )?;
    let owner_approval = shape::boolean(
        field("protected_owner_approval"),
        "$.policy_snapshot.protected_owner_approval",
    )?;
    let recurring_allowed = shape::boolean(
        field("recurring_approval_allowed"),
        "$.policy_snapshot.recurring_approval_allowed",
    )?;
    if field("previous_approval_digest").is_some() {
        shape::digest_hex(
            field("previous_approval_digest"),
            "$.policy_snapshot.previous_approval_digest",
            true,
        )?;
    }

    let replay = &request["replay"];
    if now < shape::millis(replay.get("issued_at")) {
        return fail(
            ErrorCode::RequestNotYetValid,
            "request issue time is in the future",
            "$",
        );
    }
    if now >= shape::millis(replay.get("expires_at")) {
        return fail(ErrorCode::RequestExpired, "request has expired", "$");
    }
    let versions = &request["versions"];
    if !shape::strict_equal(field("policy"), versions.get("policy"))
        || !shape::strict_equal(field("policy_digest"), versions.get("policy_digest"))
    {
        return fail(
            ErrorCode::PolicyStale,
            "request policy snapshot is stale",
            "$",
        );
    }
    if !shape::strict_equal(field("manifest"), versions.get("manifest"))
        || !shape::strict_equal(field("manifest_digest"), versions.get("manifest_digest"))
    {
        return fail(
            ErrorCode::ManifestStale,
            "request manifest snapshot is stale",
            "$",
        );
    }
    let previous = request.get("previous_approval_digest");
    if !matches!(previous, Some(Value::Null))
        && !shape::strict_equal(field("previous_approval_digest"), previous)
    {
        return fail(
            ErrorCode::PreviousApprovalStale,
            "previous approval evidence is absent or changed",
            "$",
        );
    }
    let runtime = &request["context"]["runtime"];
    let runtime_capabilities: Vec<&str> = runtime["capabilities"]
        .as_array()
        .expect("validated")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    if let Some(unknown) = runtime_capabilities
        .iter()
        .find(|capability| capability_risk(capability).is_none())
    {
        return fail(ErrorCode::RuntimeCapabilityUnknown, *unknown, "$");
    }
    let grants = shape::array(
        field("recurring_grants"),
        "$.policy_snapshot.recurring_grants",
        false,
        false,
    )?;
    for (index, grant) in grants.iter().enumerate() {
        validate_recurring_grant(grant, index)?;
    }

    let action = &request["action"];
    let risk = risk_number(action["risk_class"].as_str().unwrap_or_default());
    let proposal_safe = action["proposal_safe"] == Value::Bool(true);
    let conditions = request["conditions"].as_array().expect("validated");
    let has_condition = |name: &str| conditions.iter().any(|condition| condition == name);
    let requested: Vec<&str> = request["requested_capabilities"]
        .as_array()
        .expect("validated")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    let approval_candidate = has_condition("automation_imported")
        || has_condition("automation_new")
        || risk == 2
        || (risk == 3 && !proposal_safe)
        || (risk == 4 && owner_approval);
    if approval_candidate
        && requested
            .iter()
            .any(|capability| !runtime_capabilities.contains(capability))
    {
        return fail(
            ErrorCode::RuntimeCapabilityMissing,
            "approval cannot authorize a capability absent from the bound runtime",
            "$",
        );
    }

    // The first grant that matches, as `Array.prototype.find` takes it.
    let mut matching = None;
    for grant in grants {
        if grant_matches(grant, request, now)? {
            matching = Some(grant);
            break;
        }
    }
    let allowed_capabilities: Vec<&str> = matching
        .and_then(|grant| grant["capabilities"].as_array())
        .map(|items| items.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let mut granted: Vec<&str> = requested
        .iter()
        .copied()
        .filter(|capability| {
            allowed_capabilities.contains(capability) && runtime_capabilities.contains(capability)
        })
        .collect();
    let mut denied: Vec<&str> = requested
        .iter()
        .copied()
        .filter(|capability| !granted.contains(capability))
        .collect();
    let grant_scopes = match matching.and_then(|grant| grant["scopes"].as_array()) {
        Some(scopes) => scopes
            .iter()
            .map(canonicalize)
            .collect::<AuthorityResult<Vec<_>>>()?,
        None => Vec::new(),
    };
    let mut scopes = Vec::new();
    for scope in request["scopes"].as_array().expect("validated") {
        if grant_scopes.contains(&canonicalize(scope)?) {
            scopes.push(scope.clone());
        }
    }

    let mut degraded: Vec<&str> = Vec::new();
    let (outcome, reason) =
        if has_condition("automation_imported") || has_condition("automation_new") {
            (
                "requires_approval",
                "new_or_imported_automation_review_required",
            )
        } else if risk <= 1 && !granted.is_empty() && !scopes.is_empty() {
            ("permit", "bounded_recurring_grant")
        } else if risk == 2 {
            ("requires_approval", "risk_r2_per_run_approval")
        } else if risk == 3 && proposal_safe {
            degraded = requested.clone();
            ("degrade_to_proposal", "risk_r3_proposal_only")
        } else if risk == 3 {
            ("requires_approval", "risk_r3_per_run_approval")
        } else if risk == 4 && owner_approval {
            ("requires_approval", "risk_r4_protected_owner_per_run")
        } else if risk == 4 {
            ("reject", "risk_r4_protected_owner_missing")
        } else {
            degraded = requested.clone();
            (
                if proposal_safe {
                    "degrade_to_proposal"
                } else {
                    "reject"
                },
                "no_matching_authority",
            )
        };

    // Default JavaScript sorts compare UTF-16 code units; the closed
    // capability vocabulary is ASCII, where that is byte order, and
    // `localeCompare` agrees with it.
    granted.sort_unstable();
    denied.sort_unstable();
    degraded.sort_unstable();
    let mut bound_runtime = runtime_capabilities.clone();
    bound_runtime.sort_unstable();
    let request_digest = canonical_digest(request, Domain::Request.as_str())?;
    let execution = &request["execution"];
    let risk_class = action["risk_class"].as_str().unwrap_or_default();
    let snapshot_now = field("now").cloned().unwrap_or(Value::Null);
    Ok(json!({
        "schema_version": DECISION_VERSION,
        "decision_id": format!(
            "decision:{}:{}",
            request["request_id"].as_str().unwrap_or_default(),
            &request_digest[..16]
        ),
        "request_id": request["request_id"],
        "request_digest": format!("sha256:{request_digest}"),
        "correlation": {
            "occurrence_id": execution["occurrence_id"],
            "run_id": execution["run_id"],
            "attempt": execution["attempt"],
            "fence_generation": execution["fence_generation"],
        },
        "bindings": {
            "principal_id": request["principal"]["id"],
            "familiar_id": request["familiar"]["id"],
            "familiar_embodiment_digest": request["familiar"]["embodiment_digest"],
            "automation_id": request["automation"]["id"],
            "definition_revision": request["automation"]["definition_revision"],
            "definition_digest": request["automation"]["definition_digest"],
            "action_digest": action["digest"],
            "project_id": request["context"]["project_id"],
            "workspace_id": request["context"]["workspace_id"],
            "runtime_id": runtime["id"],
            "runtime_descriptor_digest": runtime["descriptor_digest"],
            "runtime_capabilities": bound_runtime,
            "previous_approval_digest": request["previous_approval_digest"],
        },
        "outcome": outcome,
        "granted_capabilities": granted,
        "denied_capabilities": denied
            .iter()
            .map(|capability| json!({"capability": capability, "reason_code": "capability_not_granted"}))
            .collect::<Vec<_>>(),
        "degraded_capabilities": degraded,
        "scopes": scopes,
        "validity": { "not_before": snapshot_now, "not_after": replay["expires_at"] },
        "approval_requirement": if outcome == "requires_approval" {
            json!({
                "profile": if risk_class == "R4" { "protected_owner_per_run" } else { "human_per_run" },
                "recurring_allowed": risk_class == "R2" && recurring_allowed,
            })
        } else {
            Value::Null
        },
        "versions": request["versions"],
        "reason_codes": [reason],
        "producer": { "id": "coven-threads", "verifier_profile": "automation-authority/1.0.0" },
        "issued_at": snapshot_now,
        "recorded_at": snapshot_now,
        "replay": {
            "dispatches": 1,
            "consumption_required": outcome == "permit" || outcome == "requires_approval",
        },
        "privacy": request["data"],
    }))
}

/// `grantMatchesRequest`: every identity and runtime field equal, the same
/// runtime capability set, the risk class allowed, unexpired, and uses left.
fn grant_matches(grant: &Value, request: &Value, now: i64) -> AuthorityResult<bool> {
    let pairs = [
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
        ("action_type", &request["action"]["type"]),
        ("action_digest", &request["action"]["digest"]),
        ("project_id", &request["context"]["project_id"]),
        ("workspace_id", &request["context"]["workspace_id"]),
        ("runtime_id", &request["context"]["runtime"]["id"]),
        (
            "runtime_descriptor_digest",
            &request["context"]["runtime"]["descriptor_digest"],
        ),
    ];
    if !pairs
        .iter()
        .all(|(name, expected)| shape::strict_equal(grant.get(*name), Some(expected)))
    {
        return Ok(false);
    }
    let sorted = |value: &Value| -> AuthorityResult<String> {
        let mut items: Vec<Value> = value.as_array().cloned().unwrap_or_default();
        items.sort_by(|left, right| {
            left.as_str()
                .unwrap_or_default()
                .encode_utf16()
                .cmp(right.as_str().unwrap_or_default().encode_utf16())
        });
        canonicalize(&Value::Array(items))
    };
    if sorted(&grant["runtime_capabilities"])?
        != sorted(&request["context"]["runtime"]["capabilities"])?
    {
        return Ok(false);
    }
    let risk_allowed = grant["risk_classes"].as_array().is_some_and(|classes| {
        classes
            .iter()
            .any(|class| *class == request["action"]["risk_class"])
    });
    let uses_left = match (grant["uses"].as_f64(), grant["max_uses"].as_f64()) {
        (Some(uses), Some(max)) => uses < max,
        _ => false,
    };
    Ok(risk_allowed && shape::millis(grant.get("expires_at")) > now && uses_left)
}

/// `validateRecurringGrant`.
fn validate_recurring_grant(grant: &Value, index: usize) -> AuthorityResult<()> {
    let path = format!("$.policy_snapshot.recurring_grants[{index}]");
    let at = |field: &str| format!("{path}.{field}");
    let field = |name: &str| grant.get(name);
    exact_fields(Some(grant), &GRANT_FIELDS, &path)?;
    for name in [
        "grant_id",
        "principal_id",
        "familiar_id",
        "automation_id",
        "action_type",
        "project_id",
        "workspace_id",
        "runtime_id",
    ] {
        shape::string(field(name), &at(name))?;
    }
    shape::integer(field("definition_revision"), &at("definition_revision"), 1)?;
    for name in [
        "familiar_embodiment_digest",
        "definition_digest",
        "action_digest",
        "runtime_descriptor_digest",
    ] {
        shape::digest_hex(field(name), &at(name), true)?;
    }
    validate_capabilities(field("runtime_capabilities"), &at("runtime_capabilities"))?;
    let risks = shape::array(field("risk_classes"), &at("risk_classes"), true, true)?;
    for (risk_index, risk) in risks.iter().enumerate() {
        shape::enumeration(
            Some(risk),
            &RISK_CLASSES,
            &format!("{path}.risk_classes[{risk_index}]"),
        )?;
    }
    validate_capabilities(field("capabilities"), &at("capabilities"))?;
    validate_scopes(field("scopes"), &at("scopes"))?;
    validate_evidence_scopes(
        field("capabilities")
            .and_then(Value::as_array)
            .expect("validated"),
        field("scopes")
            .and_then(Value::as_array)
            .expect("validated"),
        field("principal_id"),
        &at("scopes"),
    )?;
    shape::timestamp(field("expires_at"), &at("expires_at"))?;
    let max = shape::integer(field("max_uses"), &at("max_uses"), 1)?;
    let uses = shape::integer(field("uses"), &at("uses"), 0)?;
    if uses > max {
        return fail(
            ErrorCode::RecurringGrantUsageInvalid,
            "recurring grant usage exceeds its bound",
            path,
        );
    }
    Ok(())
}
