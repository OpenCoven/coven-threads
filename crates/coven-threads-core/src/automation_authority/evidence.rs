//! Evidence reads (`authorizeEvidenceRead` in the reference): a signed,
//! time-bounded token that lets a principal read its own evidence, or an
//! auditor read another principal's, within a sensitivity and retention
//! bound.

use serde_json::Value;

use super::keys::{verify_signed_artifact, Keyring, SignatureVerifier};
use super::request::{validate_integrity_shape, RETENTIONS, SENSITIVITIES};
use super::shape::{self, exact_fields};
use super::{fail, AuthorityResult, Domain, ErrorCode};

const EVIDENCE_READ_VERSION: &str = "opencoven.automation-evidence-read/v1";

const READ_FIELDS: [&str; 12] = [
    "schema_version",
    "read_id",
    "requesting_principal_id",
    "authorization_proof_ref",
    "subject_principal_id",
    "automation_id",
    "maximum_sensitivity",
    "retention_classes",
    "issued_at",
    "expires_at",
    "nonce",
    "integrity",
];

/// The rank of a sensitivity, `SENSITIVITY[name]` in the reference.
fn sensitivity_rank(name: &str) -> Option<usize> {
    SENSITIVITIES
        .iter()
        .position(|candidate| *candidate == name)
}

/// `authorizeEvidenceRead`: `value` is a valid evidence-read token at `now`,
/// signed by the requesting principal's own key (a `principal` key for a
/// self-read, an `auditor` key for anyone else's), and `evidence` falls
/// within its subject, sensitivity and retention bounds.
///
/// # Errors
///
/// The first failed check's code, in the reference's order. A missing or
/// falsy `now` is `evidence_read_time_required`.
pub fn authorize_evidence_read(
    value: &Value,
    evidence: &Value,
    keyring: &Keyring,
    verifier: &dyn SignatureVerifier,
    now: Option<&Value>,
) -> AuthorityResult<()> {
    shape::object(Some(value), "$")?;
    exact_fields(Some(value), &READ_FIELDS, "$")?;
    let field = |name: &str| value.get(name);
    shape::exact(
        field("schema_version"),
        EVIDENCE_READ_VERSION,
        ErrorCode::SchemaUnknownVersion,
        "$.schema_version",
    )?;
    shape::string(field("read_id"), "$.read_id")?;
    let requesting = shape::string(
        field("requesting_principal_id"),
        "$.requesting_principal_id",
    )?;
    shape::string(
        field("authorization_proof_ref"),
        "$.authorization_proof_ref",
    )?;
    let subject = shape::string(field("subject_principal_id"), "$.subject_principal_id")?;
    shape::string(field("automation_id"), "$.automation_id")?;
    let maximum = shape::enumeration(
        field("maximum_sensitivity"),
        &SENSITIVITIES,
        "$.maximum_sensitivity",
    )?;
    let retention_classes = shape::array(
        field("retention_classes"),
        "$.retention_classes",
        true,
        true,
    )?;
    for (index, class) in retention_classes.iter().enumerate() {
        shape::enumeration(
            Some(class),
            &RETENTIONS,
            &format!("$.retention_classes[{index}]"),
        )?;
    }
    let issued = shape::timestamp(field("issued_at"), "$.issued_at")?;
    let expires = shape::timestamp(field("expires_at"), "$.expires_at")?;
    if expires <= issued {
        return fail(
            ErrorCode::EvidenceReadInvalidInterval,
            "evidence-read expiry must be after issue time",
            "$",
        );
    }
    if !shape::truthy(now) {
        return fail(
            ErrorCode::EvidenceReadTimeRequired,
            "trusted current time is required",
            "$",
        );
    }
    let now = shape::timestamp(now, "$.trusted_now")?;
    if now < issued {
        return fail(
            ErrorCode::EvidenceReadNotYetValid,
            "evidence-read token is not yet valid",
            "$",
        );
    }
    if now >= expires {
        return fail(
            ErrorCode::EvidenceReadExpired,
            "evidence-read token has expired",
            "$",
        );
    }
    shape::string(field("nonce"), "$.nonce")?;
    validate_integrity_shape(field("integrity"), "$.integrity")?;
    let key = verify_signed_artifact(value, Domain::EvidenceRead, keyring, None, verifier)?;
    if key.principal_id.as_deref() != Some(requesting) {
        return fail(
            ErrorCode::EvidenceReaderMismatch,
            "read request signing principal mismatch",
            "$",
        );
    }
    if requesting == subject && key.role != "principal" {
        return fail(
            ErrorCode::EvidenceReadRoleMismatch,
            "self-read tokens require a principal signing key",
            "$",
        );
    }
    if requesting != subject && key.role != "auditor" {
        return fail(
            ErrorCode::EvidenceReadUnauthorized,
            "cross-principal evidence read requires auditor authority",
            "$",
        );
    }

    let evidence = shape::object(Some(evidence), "$.evidence")?;
    // `Object.hasOwn(SENSITIVITY, evidence.sensitivity)` converts the value to
    // a property key, so `["internal"]` names `internal` too.
    let sensitivity = evidence
        .get("sensitivity")
        .map(|sensitivity| shape::js_string(Some(sensitivity)))
        .and_then(|name| sensitivity_rank(&name));
    let Some(sensitivity) = sensitivity else {
        return fail(
            ErrorCode::EvidenceSensitivityUnknown,
            "evidence sensitivity is missing or unknown",
            "$.evidence.sensitivity",
        );
    };
    let retention = evidence.get("retention").and_then(Value::as_str);
    if !retention.is_some_and(|retention| RETENTIONS.contains(&retention)) {
        return fail(
            ErrorCode::EvidenceRetentionUnknown,
            "evidence retention is missing or unknown",
            "$.evidence.retention",
        );
    }
    if evidence.get("principal_id").and_then(Value::as_str) != Some(subject)
        || !shape::strict_equal(evidence.get("automation_id"), field("automation_id"))
    {
        return fail(
            ErrorCode::EvidenceSubjectMismatch,
            "evidence does not match the authorized subject",
            "$",
        );
    }
    if Some(sensitivity) > sensitivity_rank(maximum) {
        return fail(
            ErrorCode::EvidenceSensitivityDenied,
            "evidence exceeds reader sensitivity clearance",
            "$",
        );
    }
    if !retention_classes
        .iter()
        .any(|class| class.as_str() == retention)
    {
        return fail(
            ErrorCode::EvidenceRetentionDenied,
            "evidence retention class is outside read scope",
            "$",
        );
    }
    Ok(())
}
