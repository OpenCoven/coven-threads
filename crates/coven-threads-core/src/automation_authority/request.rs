//! Authorization requests (`validateAuthorizationRequest` and
//! `adoptAuthorizationRequest` in the reference), with the shared capability
//! and scope rules.

use serde_json::Value;

use super::canonical::{canonical_digest, canonicalize};
use super::keys::{verify_signed_artifact, Keyring, SignatureVerifier};
use super::shape::{self, exact_fields};
use super::{fail, AuthorityResult, Domain, ErrorCode};

pub(crate) const REQUEST_VERSION: &str = "opencoven.automation-authorization-request/v1";

/// Each capability with its risk floor.
pub(crate) const CAPABILITY_RISK: [(&str, u8); 11] = [
    ("analysis.read", 0),
    ("artifact.write", 1),
    ("state.mutate", 2),
    ("network.fetch", 3),
    ("network.publish", 3),
    ("credential.use", 3),
    ("evidence.read", 3),
    ("identity.mutate", 4),
    ("authority.admin", 4),
    ("release.publish", 4),
    ("resource.delete", 4),
];

/// Each action type with its risk floor.
const ACTIONS: [(&str, u8); 9] = [
    ("analysis.read", 0),
    ("artifact.create", 1),
    ("state.migrate", 2),
    ("network.fetch", 3),
    ("external.publish", 3),
    ("identity.mutate", 4),
    ("authority.change", 4),
    ("release.publish", 4),
    ("resource.delete", 4),
];

pub(crate) const RISK_CLASSES: [&str; 5] = ["R0", "R1", "R2", "R3", "R4"];
pub(crate) const SENSITIVITIES: [&str; 4] = ["public", "internal", "confidential", "restricted"];
pub(crate) const RETENTIONS: [&str; 3] = [
    "ephemeral_24h",
    "authority_evidence_90d",
    "authority_evidence_1y",
];

pub(crate) fn capability_risk(capability: &str) -> Option<u8> {
    CAPABILITY_RISK
        .iter()
        .find(|(name, _)| *name == capability)
        .map(|(_, risk)| *risk)
}

pub(crate) fn risk_number(class: &str) -> u8 {
    RISK_CLASSES
        .iter()
        .position(|candidate| *candidate == class)
        .map_or(u8::MAX, |index| index as u8)
}

/// `validateCapabilities`: a non-empty, unique list of known capabilities.
pub(crate) fn validate_capabilities(value: Option<&Value>, path: &str) -> AuthorityResult<()> {
    let items = shape::array(value, path, true, true)?;
    for (index, item) in items.iter().enumerate() {
        let capability = shape::string(Some(item), &format!("{path}[{index}]"))?;
        if capability_risk(capability).is_none() {
            return fail(
                ErrorCode::CapabilityUnknown,
                format!("unknown capability {capability}"),
                format!("{path}[{index}]"),
            );
        }
    }
    Ok(())
}

/// `validateScope`.
pub(crate) fn validate_scope(scope: &Value, path: &str) -> AuthorityResult<()> {
    let object = shape::object(Some(scope), path)?;
    let kind = shape::string(object.get("kind"), &format!("{path}.kind"))?;
    let at = |field: &str| format!("{path}.{field}");
    match kind {
        "filesystem" => {
            exact_fields(
                Some(scope),
                &["kind", "root", "path", "access", "recursive"],
                path,
            )?;
            shape::enumeration(object.get("root"), &["project", "workspace"], &at("root"))?;
            let candidate = shape::string(object.get("path"), &at("path"))?.replace('\\', "/");
            shape::enumeration(object.get("access"), &["read", "write"], &at("access"))?;
            let recursive = shape::boolean(object.get("recursive"), &at("recursive"))?;
            let drive = candidate.len() >= 3
                && candidate.as_bytes()[0].is_ascii_alphabetic()
                && &candidate[1..3] == ":/";
            if candidate == "."
                || candidate == "/"
                || candidate.contains('*')
                || candidate.contains("..")
                || candidate.starts_with('/')
                || drive
                || (recursive && candidate.split('/').filter(|part| !part.is_empty()).count() < 2)
            {
                return fail(
                    ErrorCode::ScopeTooBroad,
                    "filesystem scope must be a narrow relative path",
                    at("path"),
                );
            }
        }
        "network" => {
            exact_fields(
                Some(scope),
                &["kind", "scheme", "host", "port", "path_prefix", "methods"],
                path,
            )?;
            shape::exact(
                object.get("scheme"),
                "https",
                ErrorCode::ScopeInsecureNetwork,
                &at("scheme"),
            )?;
            let host = shape::string(object.get("host"), &at("host"))?;
            if host == "localhost"
                || host == "0.0.0.0"
                || host.contains('*')
                || !host.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || byte == b'.'
                        || byte == b'-'
                })
            {
                return fail(
                    ErrorCode::ScopeTooBroad,
                    "network host must be an exact non-local DNS name",
                    at("host"),
                );
            }
            let port = shape::integer(object.get("port"), &at("port"), 1)?;
            if port > 65_535 {
                return fail(
                    ErrorCode::SchemaInteger,
                    "port must be <= 65535",
                    at("port"),
                );
            }
            let prefix = shape::string(object.get("path_prefix"), &at("path_prefix"))?;
            if !prefix.starts_with('/') || prefix == "/" {
                return fail(
                    ErrorCode::ScopeTooBroad,
                    "network path_prefix must be narrower than /",
                    at("path_prefix"),
                );
            }
            for method in shape::array(object.get("methods"), &at("methods"), true, true)? {
                shape::enumeration(Some(method), &["GET", "POST", "PUT"], &at("methods"))?;
            }
        }
        "credential" => {
            exact_fields(
                Some(scope),
                &["kind", "credential_ref", "audience", "operations"],
                path,
            )?;
            shape::string(object.get("credential_ref"), &at("credential_ref"))?;
            if shape::string(object.get("audience"), &at("audience"))?.contains('*') {
                return fail(
                    ErrorCode::ScopeTooBroad,
                    "credential audience must be exact",
                    path,
                );
            }
            let operations = shape::array(object.get("operations"), &at("operations"), true, true)?;
            for (index, operation) in operations.iter().enumerate() {
                shape::string(Some(operation), &format!("{path}.operations[{index}]"))?;
            }
        }
        "evidence" => {
            exact_fields(
                Some(scope),
                &["kind", "principal_id", "automation_id", "retention_classes"],
                path,
            )?;
            shape::string(object.get("principal_id"), &at("principal_id"))?;
            shape::string(object.get("automation_id"), &at("automation_id"))?;
            let classes = shape::array(
                object.get("retention_classes"),
                &at("retention_classes"),
                true,
                true,
            )?;
            for (index, class) in classes.iter().enumerate() {
                shape::enumeration(
                    Some(class),
                    &RETENTIONS,
                    &format!("{path}.retention_classes[{index}]"),
                )?;
            }
        }
        _ => {
            return fail(
                ErrorCode::ScopeKindUnknown,
                format!("unknown scope kind {kind}"),
                at("kind"),
            )
        }
    }
    Ok(())
}

/// `validateScopes`: a non-empty list of valid, distinct scopes.
pub(crate) fn validate_scopes(value: Option<&Value>, path: &str) -> AuthorityResult<()> {
    let scopes = shape::array(value, path, true, false)?;
    let mut seen = Vec::with_capacity(scopes.len());
    for (index, scope) in scopes.iter().enumerate() {
        validate_scope(scope, &format!("{path}[{index}]"))?;
        let encoded = canonicalize(scope)?;
        if seen.contains(&encoded) {
            return fail(
                ErrorCode::ScopeDuplicate,
                "duplicate scope",
                format!("{path}[{index}]"),
            );
        }
        seen.push(encoded);
    }
    Ok(())
}

/// `validateGenericEvidenceScopes`: generic `evidence.read` is self-only.
pub(crate) fn validate_evidence_scopes(
    capabilities: &[Value],
    scopes: &[Value],
    principal_id: Option<&Value>,
    path: &str,
) -> AuthorityResult<()> {
    if !capabilities
        .iter()
        .any(|capability| capability == "evidence.read")
    {
        return Ok(());
    }
    for (index, scope) in scopes.iter().enumerate() {
        if scope.get("kind").and_then(Value::as_str) == Some("evidence")
            && !shape::strict_equal(scope.get("principal_id"), principal_id)
        {
            return fail(
                ErrorCode::EvidenceScopeCrossPrincipalForbidden,
                "generic evidence.read is self-only; cross-principal reads require AutomationEvidenceRead auditor authority",
                format!("{path}[{index}].principal_id"),
            );
        }
    }
    Ok(())
}

fn validate_capability_scope_pairing(
    capabilities: &[Value],
    scopes: &[Value],
) -> AuthorityResult<()> {
    let kinds: Vec<&str> = scopes
        .iter()
        .filter_map(|scope| scope.get("kind").and_then(Value::as_str))
        .collect();
    for capability in capabilities.iter().filter_map(Value::as_str) {
        let required = match capability {
            "credential.use" => "credential",
            "network.fetch" | "network.publish" => "network",
            "evidence.read" => "evidence",
            _ => continue,
        };
        if !kinds.contains(&required) {
            return fail(
                ErrorCode::CapabilityScopeMismatch,
                format!("{capability} requires an explicit {required} scope"),
                "$.scopes",
            );
        }
    }
    Ok(())
}

const REQUEST_FIELDS: [&str; 16] = [
    "schema_version",
    "request_id",
    "principal",
    "replay",
    "familiar",
    "automation",
    "execution",
    "action",
    "requested_capabilities",
    "scopes",
    "context",
    "versions",
    "previous_approval_digest",
    "conditions",
    "data",
    "integrity",
];

/// `validateAuthorizationRequest`: the request's shape, risk floors, scopes and
/// safeguards, then its signature by the requesting principal's own key.
///
/// # Errors
///
/// The first failed check's code, in the reference's order.
pub fn validate_authorization_request(
    value: &Value,
    keyring: &Keyring,
    verifier: &dyn SignatureVerifier,
) -> AuthorityResult<()> {
    shape::object(Some(value), "$")?;
    exact_fields(Some(value), &REQUEST_FIELDS, "$")?;
    shape::exact(
        value.get("schema_version"),
        REQUEST_VERSION,
        ErrorCode::SchemaUnknownVersion,
        "$.schema_version",
    )?;
    shape::string(value.get("request_id"), "$.request_id")?;

    let principal = value.get("principal");
    exact_fields(principal, &["id", "authorization_proof_ref"], "$.principal")?;
    shape::string(principal.and_then(|p| p.get("id")), "$.principal.id")?;
    shape::string(
        principal.and_then(|p| p.get("authorization_proof_ref")),
        "$.principal.authorization_proof_ref",
    )?;

    let replay = value.get("replay");
    let replay_field = |name: &str| replay.and_then(|r| r.get(name));
    exact_fields(
        replay,
        &["nonce", "adoption_key", "issued_at", "expires_at"],
        "$.replay",
    )?;
    shape::string(replay_field("nonce"), "$.replay.nonce")?;
    shape::string(replay_field("adoption_key"), "$.replay.adoption_key")?;
    let issued = shape::timestamp(replay_field("issued_at"), "$.replay.issued_at")?;
    let expires = shape::timestamp(replay_field("expires_at"), "$.replay.expires_at")?;
    if expires <= issued {
        return fail(
            ErrorCode::RequestInvalidInterval,
            "request expiry must be after issue time",
            "$.replay",
        );
    }

    let familiar = value.get("familiar");
    exact_fields(familiar, &["id", "embodiment_digest"], "$.familiar")?;
    shape::string(familiar.and_then(|f| f.get("id")), "$.familiar.id")?;
    shape::digest_hex(
        familiar.and_then(|f| f.get("embodiment_digest")),
        "$.familiar.embodiment_digest",
        true,
    )?;

    let automation = value.get("automation");
    exact_fields(
        automation,
        &["id", "definition_revision", "definition_digest"],
        "$.automation",
    )?;
    shape::string(automation.and_then(|a| a.get("id")), "$.automation.id")?;
    shape::integer(
        automation.and_then(|a| a.get("definition_revision")),
        "$.automation.definition_revision",
        1,
    )?;
    shape::digest_hex(
        automation.and_then(|a| a.get("definition_digest")),
        "$.automation.definition_digest",
        true,
    )?;

    let execution = value.get("execution");
    let execution_field = |name: &str| execution.and_then(|e| e.get(name));
    exact_fields(
        execution,
        &["occurrence_id", "run_id", "attempt", "fence_generation"],
        "$.execution",
    )?;
    shape::string(
        execution_field("occurrence_id"),
        "$.execution.occurrence_id",
    )?;
    shape::string(execution_field("run_id"), "$.execution.run_id")?;
    shape::integer(execution_field("attempt"), "$.execution.attempt", 1)?;
    shape::integer(
        execution_field("fence_generation"),
        "$.execution.fence_generation",
        1,
    )?;

    let action = value.get("action");
    let action_field = |name: &str| action.and_then(|a| a.get(name));
    exact_fields(
        action,
        &["type", "digest", "risk_class", "proposal_safe"],
        "$.action",
    )?;
    let action_type = shape::string(action_field("type"), "$.action.type")?;
    let Some(action_floor) = ACTIONS
        .iter()
        .find(|(name, _)| *name == action_type)
        .map(|(_, floor)| *floor)
    else {
        return fail(
            ErrorCode::ActionUnknown,
            format!("unknown action {action_type}"),
            "$.action.type",
        );
    };
    shape::digest_hex(action_field("digest"), "$.action.digest", true)?;
    let risk_class = shape::enumeration(
        action_field("risk_class"),
        &RISK_CLASSES,
        "$.action.risk_class",
    )?;
    shape::boolean(action_field("proposal_safe"), "$.action.proposal_safe")?;
    let risk = risk_number(risk_class);
    if risk < action_floor {
        return fail(
            ErrorCode::ActionRiskUnderclassified,
            "declared risk is lower than the action floor",
            "$.action",
        );
    }

    let capabilities_value = value.get("requested_capabilities");
    validate_capabilities(capabilities_value, "$.requested_capabilities")?;
    let capabilities = capabilities_value
        .and_then(Value::as_array)
        .expect("validated");
    let capability_floor = capabilities
        .iter()
        .filter_map(|capability| capability.as_str().and_then(capability_risk))
        .max()
        .unwrap_or(0);
    if risk < capability_floor {
        return fail(
            ErrorCode::CapabilityRiskUnderclassified,
            "declared risk is lower than a requested capability floor",
            "$.requested_capabilities",
        );
    }
    validate_scopes(value.get("scopes"), "$.scopes")?;
    let scopes = value["scopes"].as_array().expect("validated");
    validate_capability_scope_pairing(capabilities, scopes)?;
    validate_evidence_scopes(
        capabilities,
        scopes,
        principal.and_then(|p| p.get("id")),
        "$.scopes",
    )?;

    let context = value.get("context");
    exact_fields(
        context,
        &["project_id", "workspace_id", "runtime"],
        "$.context",
    )?;
    shape::string(
        context.and_then(|c| c.get("project_id")),
        "$.context.project_id",
    )?;
    shape::string(
        context.and_then(|c| c.get("workspace_id")),
        "$.context.workspace_id",
    )?;
    let runtime = context.and_then(|c| c.get("runtime"));
    shape::object(runtime, "$.context.runtime")?;
    exact_fields(
        runtime,
        &["id", "descriptor_digest", "capabilities"],
        "$.context.runtime",
    )?;
    shape::string(runtime.and_then(|r| r.get("id")), "$.context.runtime.id")?;
    shape::digest_hex(
        runtime.and_then(|r| r.get("descriptor_digest")),
        "$.context.runtime.descriptor_digest",
        true,
    )?;
    validate_capabilities(
        runtime.and_then(|r| r.get("capabilities")),
        "$.context.runtime.capabilities",
    )?;

    validate_versions(value.get("versions"), "$.versions")?;
    let previous = value.get("previous_approval_digest");
    if !matches!(previous, Some(Value::Null)) {
        shape::digest_hex(previous, "$.previous_approval_digest", true)?;
    }
    let conditions = shape::array(value.get("conditions"), "$.conditions", false, true)?;
    for (index, condition) in conditions.iter().enumerate() {
        shape::string(Some(condition), &format!("$.conditions[{index}]"))?;
    }
    if risk_class == "R2"
        && !(conditions.iter().any(|c| c == "deterministic_validation")
            && conditions.iter().any(|c| c == "rollback_plan"))
    {
        return fail(
            ErrorCode::R2SafeguardsMissing,
            "R2 operations require deterministic_validation and rollback_plan conditions",
            "$.conditions",
        );
    }
    validate_data(value.get("data"), "$.data")?;
    validate_integrity_shape(value.get("integrity"), "$.integrity")?;
    let key = verify_signed_artifact(value, Domain::Request, keyring, None, verifier)?;
    if key.principal_id.as_deref() != principal.and_then(|p| p.get("id")).and_then(Value::as_str) {
        return fail(
            ErrorCode::PrincipalKeyMismatch,
            "request signing key belongs to another principal",
            "$",
        );
    }
    if key.role != "principal" {
        return fail(
            ErrorCode::IntegrityRoleMismatch,
            "request must be authenticated by a principal key",
            "$",
        );
    }
    Ok(())
}

/// `validateIntegrityShape`: an object first, then exactly the four members.
pub(crate) fn validate_integrity_shape(
    integrity: Option<&Value>,
    path: &str,
) -> AuthorityResult<()> {
    shape::object(integrity, path)?;
    exact_fields(
        integrity,
        &["alg", "key_id", "signed_digest", "signature_b64"],
        path,
    )
}

/// The `versions` member requests and decisions share.
pub(crate) fn validate_versions(versions: Option<&Value>, path: &str) -> AuthorityResult<()> {
    let field = |name: &str| versions.and_then(|v| v.get(name));
    exact_fields(
        versions,
        &[
            "profile",
            "policy",
            "policy_digest",
            "manifest",
            "manifest_digest",
        ],
        path,
    )?;
    shape::exact(
        field("profile"),
        "1.0.0",
        ErrorCode::ProfileVersionUnknown,
        &format!("{path}.profile"),
    )?;
    shape::string(field("policy"), &format!("{path}.policy"))?;
    shape::digest_hex(
        field("policy_digest"),
        &format!("{path}.policy_digest"),
        true,
    )?;
    shape::string(field("manifest"), &format!("{path}.manifest"))?;
    shape::digest_hex(
        field("manifest_digest"),
        &format!("{path}.manifest_digest"),
        true,
    )?;
    Ok(())
}

/// The `data` (or a decision's `privacy`) member.
pub(crate) fn validate_data(data: Option<&Value>, path: &str) -> AuthorityResult<()> {
    exact_fields(data, &["sensitivity", "retention"], path)?;
    shape::enumeration(
        data.and_then(|d| d.get("sensitivity")),
        &SENSITIVITIES,
        &format!("{path}.sensitivity"),
    )?;
    shape::enumeration(
        data.and_then(|d| d.get("retention")),
        &RETENTIONS,
        &format!("{path}.retention"),
    )?;
    Ok(())
}

/// What a series of request adoptions has seen.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdoptionState {
    /// Adopted replay nonces.
    pub nonces: Vec<String>,
    /// Adopted adoption keys.
    pub adoption_keys: Vec<String>,
    /// Adopted request digests, `sha256:`-prefixed.
    pub request_digests: Vec<String>,
}

/// `adoptAuthorizationRequest`: a valid request, within its window at `now`
/// when given, whose nonce, adoption key and digest are all new.
///
/// # Errors
///
/// A validation code, `schema_timestamp` for a malformed `now`,
/// `request_not_yet_valid`, `request_expired` or `request_replayed`.
pub fn adopt_authorization_request(
    value: &Value,
    state: Option<&AdoptionState>,
    keyring: &Keyring,
    verifier: &dyn SignatureVerifier,
    now: Option<&Value>,
) -> AuthorityResult<AdoptionState> {
    validate_authorization_request(value, keyring, verifier)?;
    let replay = &value["replay"];
    if now.is_some() {
        let now = shape::timestamp(now, "$.trusted_now")?;
        if now < shape::millis(replay.get("issued_at")) {
            return fail(
                ErrorCode::RequestNotYetValid,
                "request has not reached its issue time",
                "$",
            );
        }
        if now >= shape::millis(replay.get("expires_at")) {
            return fail(ErrorCode::RequestExpired, "request has expired", "$");
        }
    }
    let mut state = state.cloned().unwrap_or_default();
    let digest = format!(
        "sha256:{}",
        canonical_digest(value, Domain::Request.as_str())?
    );
    let nonce = replay["nonce"].as_str().unwrap_or_default().to_owned();
    let adoption_key = replay["adoption_key"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    if state.nonces.contains(&nonce)
        || state.adoption_keys.contains(&adoption_key)
        || state.request_digests.contains(&digest)
    {
        return fail(
            ErrorCode::RequestReplayed,
            "request nonce, adoption key, or digest was already adopted",
            "$",
        );
    }
    state.nonces.push(nonce);
    state.adoption_keys.push(adoption_key);
    state.request_digests.push(digest);
    Ok(state)
}
