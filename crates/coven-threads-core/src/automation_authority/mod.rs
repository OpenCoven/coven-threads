//! The automation-authority profile (`profiles/automation-authority/v1`):
//! request validation, authorization evaluation and decision verification.
//!
//! This is a Rust port of the evaluation half of the reference validator,
//! `profiles/automation-authority/v1/validator.mjs`, for the trusted daemon to
//! call in process. For the same inputs it reaches the same result and raises
//! the same first error code, because the profile's conformance vectors judge
//! a refusal by its first code alone. The JavaScript validator stays the
//! reference: where the two disagree, this port is wrong.
//!
//! Ported here, and checked against the 44 vectors that exercise them:
//!
//! - [`strict_parse_json`]: the profile's strict I-JSON parser;
//! - [`validate_authorization_request`] and [`adopt_authorization_request`];
//! - [`evaluate_authorization`], which computes the unsigned decision;
//! - [`validate_decision`], [`verify_decision_bundle`] and [`consume_decision`];
//! - [`canonical_digest`] and [`sign_artifact`], the integrity primitives.
//!
//! Approvals, lifecycle events, consumption snapshots, proposals, evidence
//! reads and dispatch verification are not ported yet.
//!
//! **Crypto stays outside the core.** Signatures are checked through
//! [`SignatureVerifier`] and made through [`ArtifactSigner`], so the caller
//! supplies the Ed25519 implementation and this crate keeps no crypto backend.
//!
//! **One deliberate divergence.** The JavaScript parser silently drops an
//! object key named `__proto__`, a language quirk. This port refuses it as
//! `json_invalid` rather than accept a document whose bytes and parsed value
//! disagree.

mod canonical;
mod decision;
mod evaluate;
mod json;
mod keys;
mod request;
mod shape;

use std::fmt;

pub use canonical::{canonical_digest, canonicalize, Domain};
pub use decision::{consume_decision, validate_decision, verify_decision_bundle, ConsumptionState};
pub use evaluate::evaluate_authorization;
pub use json::{strict_parse_json, MAX_DEPTH};
pub use keys::{sign_artifact, ArtifactSigner, Keyring, SignatureVerifier};
pub use request::{adopt_authorization_request, validate_authorization_request, AdoptionState};

macro_rules! codes {
    ($($variant:ident => $text:literal,)*) => {
        /// The profile's error codes, as the reference validator raises them.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[non_exhaustive]
        pub enum ErrorCode {
            $(
                #[doc = concat!("`", $text, "`")]
                $variant,
            )*
        }

        impl ErrorCode {
            /// The code as the reference validator spells it.
            #[must_use]
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text,)*
                }
            }
        }
    };
}

codes! {
    JsonInvalid => "json_invalid",
    JsonNonIjson => "json_non_ijson",
    JsonDuplicateKey => "json_duplicate_key",
    JsonUnsafeInteger => "json_unsafe_integer",
    SchemaType => "schema_type",
    SchemaMinItems => "schema_min_items",
    SchemaDuplicateItem => "schema_duplicate_item",
    SchemaInteger => "schema_integer",
    SchemaEnum => "schema_enum",
    SchemaTimestamp => "schema_timestamp",
    SchemaDigest => "schema_digest",
    SchemaUnknownField => "schema_unknown_field",
    SchemaRequired => "schema_required",
    SchemaUnknownVersion => "schema_unknown_version",
    IntegrityDomainInvalid => "integrity_domain_invalid",
    IntegrityRoleUnknown => "integrity_role_unknown",
    IntegrityPrincipalIdMissing => "integrity_principal_id_missing",
    IntegrityPrincipalIdUnexpected => "integrity_principal_id_unexpected",
    IntegrityKeyUnknown => "integrity_key_unknown",
    IntegrityAlgorithmUnknown => "integrity_algorithm_unknown",
    IntegritySignatureNoncanonical => "integrity_signature_noncanonical",
    IntegrityRoleMismatch => "integrity_role_mismatch",
    IntegrityDigestMismatch => "integrity_digest_mismatch",
    IntegritySignatureInvalid => "integrity_signature_invalid",
    CapabilityUnknown => "capability_unknown",
    ScopeTooBroad => "scope_too_broad",
    ScopeInsecureNetwork => "scope_insecure_network",
    ScopeKindUnknown => "scope_kind_unknown",
    ScopeDuplicate => "scope_duplicate",
    CapabilityScopeMismatch => "capability_scope_mismatch",
    EvidenceScopeCrossPrincipalForbidden => "evidence_scope_cross_principal_forbidden",
    RequestInvalidInterval => "request_invalid_interval",
    ActionUnknown => "action_unknown",
    ActionRiskUnderclassified => "action_risk_underclassified",
    CapabilityRiskUnderclassified => "capability_risk_underclassified",
    ProfileVersionUnknown => "profile_version_unknown",
    R2SafeguardsMissing => "r2_safeguards_missing",
    PrincipalKeyMismatch => "principal_key_mismatch",
    RequestNotYetValid => "request_not_yet_valid",
    RequestExpired => "request_expired",
    RequestReplayed => "request_replayed",
    PolicyStale => "policy_stale",
    ManifestStale => "manifest_stale",
    PreviousApprovalStale => "previous_approval_stale",
    RuntimeCapabilityUnknown => "runtime_capability_unknown",
    RuntimeCapabilityMissing => "runtime_capability_missing",
    RecurringGrantUsageInvalid => "recurring_grant_usage_invalid",
    DecisionRuntimeCapabilityMissing => "decision_runtime_capability_missing",
    DecisionInvalidInterval => "decision_invalid_interval",
    DecisionApprovalProfileMissing => "decision_approval_profile_missing",
    DecisionRecurringApprovalForbidden => "decision_recurring_approval_forbidden",
    DecisionApprovalProfileUnexpected => "decision_approval_profile_unexpected",
    DecisionProducerForged => "decision_producer_forged",
    DecisionReplayPolicyInvalid => "decision_replay_policy_invalid",
    DecisionSemanticMismatch => "decision_semantic_mismatch",
    DecisionNotConsumable => "decision_not_consumable",
    DecisionReplayed => "decision_replayed",
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A refusal, with the profile's code, a message, and the JSON path it
/// concerns.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{code} at {path}: {message}")]
pub struct AuthorityError {
    /// The profile's error code. Conformance judges a refusal by this alone.
    pub code: ErrorCode,
    /// What failed.
    pub message: String,
    /// The JSON path of the offending value, such as `$.action.type`.
    pub path: String,
}

/// The result of a profile operation.
pub type AuthorityResult<T> = Result<T, AuthorityError>;

pub(crate) fn fail<T>(
    code: ErrorCode,
    message: impl Into<String>,
    path: impl Into<String>,
) -> AuthorityResult<T> {
    Err(AuthorityError {
        code,
        message: message.into(),
        path: path.into(),
    })
}
