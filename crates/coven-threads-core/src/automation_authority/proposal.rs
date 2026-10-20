//! Proposals (`validateProposal` in the reference): the Threads authority's
//! signed record of an action left unexecuted, which a later adoption must
//! authorize afresh.

use serde_json::Value;

use super::keys::{verify_signed_artifact, Keyring, SignatureVerifier};
use super::request::{validate_integrity_shape, SENSITIVITIES};
use super::shape::{self, exact_fields};
use super::{AuthorityResult, Domain, ErrorCode};

const PROPOSAL_VERSION: &str = "opencoven.automation-proposal/v1";

const PROPOSAL_FIELDS: [&str; 13] = [
    "schema_version",
    "proposal_id",
    "request_digest",
    "action_digest",
    "intended_target",
    "status",
    "protected_effects_performed",
    "result_claim",
    "requires_new_adoption",
    "content_digest",
    "created_at",
    "privacy",
    "integrity",
];

/// `validateProposal`: a well-formed proposal that claims no execution, no
/// protected effect and no result, requires a new adoption, and is signed by a
/// `threads_authority` key.
///
/// # Errors
///
/// The first failed check's code, in the reference's order.
pub fn validate_proposal(
    value: &Value,
    keyring: &Keyring,
    verifier: &dyn SignatureVerifier,
) -> AuthorityResult<()> {
    shape::object(Some(value), "$")?;
    exact_fields(Some(value), &PROPOSAL_FIELDS, "$")?;
    let field = |name: &str| value.get(name);
    shape::exact(
        field("schema_version"),
        PROPOSAL_VERSION,
        ErrorCode::SchemaUnknownVersion,
        "$.schema_version",
    )?;
    shape::string(field("proposal_id"), "$.proposal_id")?;
    shape::digest_hex(field("request_digest"), "$.request_digest", true)?;
    shape::digest_hex(field("action_digest"), "$.action_digest", true)?;
    shape::string(field("intended_target"), "$.intended_target")?;
    shape::exact(
        field("status"),
        "not_executed",
        ErrorCode::ProposalStatusForged,
        "$.status",
    )?;
    shape::exact_value(
        field("protected_effects_performed"),
        &Value::Bool(false),
        ErrorCode::ProposalEffectForbidden,
        "$.protected_effects_performed",
    )?;
    shape::exact(
        field("result_claim"),
        "proposal_only",
        ErrorCode::ProposalSuccessForged,
        "$.result_claim",
    )?;
    shape::exact_value(
        field("requires_new_adoption"),
        &Value::Bool(true),
        ErrorCode::ProposalAdoptionRequired,
        "$.requires_new_adoption",
    )?;
    shape::digest_hex(field("content_digest"), "$.content_digest", true)?;
    shape::timestamp(field("created_at"), "$.created_at")?;
    shape::enumeration(field("privacy"), &SENSITIVITIES, "$.privacy")?;
    validate_integrity_shape(field("integrity"), "$.integrity")?;
    verify_signed_artifact(
        value,
        Domain::Proposal,
        keyring,
        Some("threads_authority"),
        verifier,
    )?;
    Ok(())
}
