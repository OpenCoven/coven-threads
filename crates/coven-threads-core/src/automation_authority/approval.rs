//! Approvals (`validateApproval` in the reference): a principal's or
//! protected owner's signed consent to one operation, or to a bounded
//! recurring series of them.

use serde_json::Value;

use super::keys::{verify_signed_artifact, Keyring, SignatureVerifier, VerifiedKey};
use super::request::{
    validate_capabilities, validate_evidence_scopes, validate_integrity_shape, validate_scopes,
    validate_versions, SENSITIVITIES,
};
use super::shape::{self, exact_fields};
use super::{fail, AuthorityResult, Domain, ErrorCode};

const APPROVAL_VERSION: &str = "opencoven.automation-approval/v1";

const APPROVAL_FIELDS: [&str; 28] = [
    "schema_version",
    "approval_id",
    "approving_principal",
    "request_digest",
    "decision_digest",
    "familiar_id",
    "familiar_embodiment_digest",
    "automation",
    "authorized_principal_id",
    "occurrence_id",
    "run_id",
    "attempt",
    "fence_generation",
    "action_digest",
    "capabilities",
    "scopes",
    "project_id",
    "workspace_id",
    "runtime_id",
    "runtime_descriptor_digest",
    "runtime_capabilities",
    "versions",
    "use",
    "issued_at",
    "expires_at",
    "nonce",
    "rationale",
    "integrity",
];

/// `validateApproval`: a well-formed approval, within its window at `now` when
/// given, signed by the approving principal's own `principal` or
/// `protected_owner` key under the key it names.
///
/// # Errors
///
/// The first failed check's code, in the reference's order.
pub fn validate_approval(
    value: &Value,
    keyring: &Keyring,
    verifier: &dyn SignatureVerifier,
    now: Option<&Value>,
) -> AuthorityResult<()> {
    approval_key(value, keyring, verifier, now).map(|_| ())
}

/// `validateApproval`, returning the key that signed the approval, as the
/// reference's dispatch check looks it up again.
pub(crate) fn approval_key(
    value: &Value,
    keyring: &Keyring,
    verifier: &dyn SignatureVerifier,
    now: Option<&Value>,
) -> AuthorityResult<VerifiedKey> {
    shape::object(Some(value), "$")?;
    exact_fields(Some(value), &APPROVAL_FIELDS, "$")?;
    let field = |name: &str| value.get(name);
    shape::exact(
        field("schema_version"),
        APPROVAL_VERSION,
        ErrorCode::SchemaUnknownVersion,
        "$.schema_version",
    )?;
    shape::string(field("approval_id"), "$.approval_id")?;
    let approving = field("approving_principal");
    exact_fields(approving, &["id", "key_ref"], "$.approving_principal")?;
    let approving_id = shape::string(
        approving.and_then(|a| a.get("id")),
        "$.approving_principal.id",
    )?;
    let key_ref = shape::string(
        approving.and_then(|a| a.get("key_ref")),
        "$.approving_principal.key_ref",
    )?;
    for name in [
        "request_digest",
        "decision_digest",
        "familiar_embodiment_digest",
        "action_digest",
        "runtime_descriptor_digest",
    ] {
        shape::digest_hex(field(name), &format!("$.{name}"), true)?;
    }
    shape::string(field("familiar_id"), "$.familiar_id")?;
    let automation = field("automation");
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
    shape::string(
        field("authorized_principal_id"),
        "$.authorized_principal_id",
    )?;
    // `value.use?.kind`: a missing or non-object `use` has no kind.
    let use_ = field("use");
    let kind = use_.and_then(|u| u.get("kind"));
    if kind.and_then(Value::as_str) == Some("recurring") {
        for name in ["occurrence_id", "run_id", "attempt", "fence_generation"] {
            shape::exact_value(
                field(name),
                &Value::Null,
                ErrorCode::ApprovalRecurringShapeInvalid,
                &format!("$.{name}"),
            )?;
        }
    } else {
        shape::string(field("occurrence_id"), "$.occurrence_id")?;
        shape::string(field("run_id"), "$.run_id")?;
        shape::integer(field("attempt"), "$.attempt", 1)?;
        shape::integer(field("fence_generation"), "$.fence_generation", 1)?;
    }
    validate_capabilities(field("capabilities"), "$.capabilities")?;
    validate_scopes(field("scopes"), "$.scopes")?;
    let capabilities = field("capabilities")
        .and_then(Value::as_array)
        .expect("validated");
    validate_evidence_scopes(
        capabilities,
        field("scopes")
            .and_then(Value::as_array)
            .expect("validated"),
        field("authorized_principal_id"),
        "$.scopes",
    )?;
    shape::string(field("project_id"), "$.project_id")?;
    shape::string(field("workspace_id"), "$.workspace_id")?;
    shape::string(field("runtime_id"), "$.runtime_id")?;
    validate_capabilities(field("runtime_capabilities"), "$.runtime_capabilities")?;
    let runtime = field("runtime_capabilities")
        .and_then(Value::as_array)
        .expect("validated");
    if capabilities
        .iter()
        .any(|capability| !runtime.contains(capability))
    {
        return fail(
            ErrorCode::ApprovalRuntimeCapabilityMissing,
            "approval grants a capability absent from its bound runtime",
            "$",
        );
    }
    validate_versions(field("versions"), "$.versions")?;

    shape::object(use_, "$.use")?;
    let kind = shape::string(kind, "$.use.kind")?;
    match kind {
        "single_use" => shape::closed(use_, &["kind"], "$.use")?,
        "recurring" => {
            exact_fields(
                use_,
                &["kind", "grant_id", "max_uses", "occurrence_prefix"],
                "$.use",
            )?;
            let use_field = |name: &str| use_.and_then(|u| u.get(name));
            shape::string(use_field("grant_id"), "$.use.grant_id")?;
            let max_uses = shape::integer(use_field("max_uses"), "$.use.max_uses", 1)?;
            if max_uses > 366 {
                return fail(
                    ErrorCode::ApprovalUseTooBroad,
                    "recurring max_uses exceeds 366",
                    "$",
                );
            }
            let prefix = shape::string(use_field("occurrence_prefix"), "$.use.occurrence_prefix")?;
            if shape::utf16_len(prefix) < 8 || prefix.contains('*') {
                return fail(
                    ErrorCode::ApprovalUseTooBroad,
                    "recurring occurrence prefix is too broad",
                    "$",
                );
            }
        }
        other => {
            return fail(
                ErrorCode::ApprovalUseUnknown,
                format!("unknown approval use {other}"),
                "$.use.kind",
            )
        }
    }

    let issued = shape::timestamp(field("issued_at"), "$.issued_at")?;
    let expires = shape::timestamp(field("expires_at"), "$.expires_at")?;
    let now = match now {
        Some(_) => Some(shape::timestamp(now, "$.trusted_now")?),
        None => None,
    };
    if expires <= issued {
        return fail(
            ErrorCode::ApprovalInvalidInterval,
            "approval expiry must be after issue time",
            "$",
        );
    }
    if now.is_some_and(|now| now < issued) {
        return fail(
            ErrorCode::ApprovalNotYetValid,
            "approval has not reached its issue time",
            "$",
        );
    }
    if now.is_some_and(|now| now >= expires) {
        return fail(ErrorCode::ApprovalExpired, "approval has expired", "$");
    }
    shape::string(field("nonce"), "$.nonce")?;
    let rationale = field("rationale");
    exact_fields(rationale, &["text", "privacy"], "$.rationale")?;
    shape::string(rationale.and_then(|r| r.get("text")), "$.rationale.text")?;
    shape::enumeration(
        rationale.and_then(|r| r.get("privacy")),
        &SENSITIVITIES,
        "$.rationale.privacy",
    )?;
    validate_integrity_shape(field("integrity"), "$.integrity")?;
    let key = verify_signed_artifact(value, Domain::Approval, keyring, None, verifier)?;
    if !matches!(key.role.as_str(), "principal" | "protected_owner") {
        return fail(
            ErrorCode::IntegrityRoleMismatch,
            "approval must be signed by a principal authority key",
            "$",
        );
    }
    if key.principal_id.as_deref() != Some(approving_id) {
        return fail(
            ErrorCode::ApprovalPrincipalMismatch,
            "approval key belongs to another principal",
            "$",
        );
    }
    if value["integrity"]["key_id"].as_str() != Some(key_ref) {
        return fail(
            ErrorCode::ApprovalKeyMismatch,
            "approval key_ref does not match signing key",
            "$",
        );
    }
    Ok(key)
}
