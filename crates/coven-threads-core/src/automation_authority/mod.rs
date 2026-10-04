//! The automation-authority profile (`profiles/automation-authority/v1`):
//! request validation, authorization evaluation, approvals, the approval
//! lifecycle, and final dispatch verification.
//!
//! This is a Rust port of the reference validator,
//! `profiles/automation-authority/v1/validator.mjs`, for the trusted daemon to
//! call in process. For the same inputs it reaches the same result and raises
//! the same first error code, because the profile's conformance vectors judge
//! a refusal by its first code alone. The JavaScript validator stays the
//! reference: where the two disagree, this port is wrong.
//!
//! Ported here, and checked against all 130 conformance vectors:
//!
//! - [`strict_parse_json`]: the profile's strict I-JSON parser;
//! - [`validate_authorization_request`] and [`adopt_authorization_request`];
//! - [`evaluate_authorization`], which computes the unsigned decision;
//! - [`validate_decision`], [`verify_decision_bundle`] and [`consume_decision`];
//! - [`validate_consumption_snapshot`] and [`validate_approval`];
//! - [`apply_lifecycle_event`] and [`verify_lifecycle_chain`];
//! - [`verify_dispatch`], the check before a launch;
//! - [`validate_proposal`] and [`authorize_evidence_read`];
//! - [`canonical_digest`] and [`sign_artifact`], the integrity primitives.
//!
//! **Crypto stays outside the core.** Signatures are checked through
//! [`SignatureVerifier`] and made through [`ArtifactSigner`], so the caller
//! supplies the Ed25519 implementation and this crate keeps no crypto backend.
//!
//! **Where the reference has no code.** In a few places the reference reads a
//! member of `undefined` or `null` and throws a `TypeError` rather than a
//! profile error, such as a lifecycle transition that reads the use of an
//! approval it was not given. The port refuses those inputs too, with
//! `schema_type` or `schema_required`.
//!
//! **One deliberate divergence.** The JavaScript parser silently drops an
//! object key named `__proto__`, a language quirk. This port refuses it as
//! `json_invalid` rather than accept a document whose bytes and parsed value
//! disagree.

mod approval;
mod canonical;
mod decision;
mod dispatch;
mod evaluate;
mod evidence;
mod json;
mod keys;
mod lifecycle;
mod proposal;
mod request;
mod shape;
mod snapshot;

use std::fmt;

pub use approval::validate_approval;
pub use canonical::{canonical_digest, canonicalize, Domain};
pub use decision::{consume_decision, validate_decision, verify_decision_bundle, ConsumptionState};
pub use dispatch::{verify_dispatch, DispatchAuthorization, RequiredConsumption};
pub use evaluate::evaluate_authorization;
pub use evidence::authorize_evidence_read;
pub use json::{strict_parse_json, MAX_DEPTH};
pub use keys::{sign_artifact, ArtifactSigner, Keyring, SignatureVerifier};
pub use lifecycle::{apply_lifecycle_event, verify_lifecycle_chain, LifecycleState};
pub use proposal::validate_proposal;
pub use request::{adopt_authorization_request, validate_authorization_request, AdoptionState};
pub use snapshot::validate_consumption_snapshot;

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
    ConsumptionSnapshotFromFuture => "consumption_snapshot_from_future",
    ConsumptionSnapshotDuplicate => "consumption_snapshot_duplicate",
    ApprovalRecurringShapeInvalid => "approval_recurring_shape_invalid",
    ApprovalRuntimeCapabilityMissing => "approval_runtime_capability_missing",
    ApprovalUseTooBroad => "approval_use_too_broad",
    ApprovalUseUnknown => "approval_use_unknown",
    ApprovalInvalidInterval => "approval_invalid_interval",
    ApprovalNotYetValid => "approval_not_yet_valid",
    ApprovalExpired => "approval_expired",
    ApprovalPrincipalMismatch => "approval_principal_mismatch",
    ApprovalKeyMismatch => "approval_key_mismatch",
    LifecycleConsumptionUnexpected => "lifecycle_consumption_unexpected",
    LifecycleOccurrenceDispositionUnexpected => "lifecycle_occurrence_disposition_unexpected",
    LifecycleActorForged => "lifecycle_actor_forged",
    RejectionDispositionInvalid => "rejection_disposition_invalid",
    RejectionOccurrenceMissing => "rejection_occurrence_missing",
    ExpirationDispositionInvalid => "expiration_disposition_invalid",
    ExpirationOccurrenceMissing => "expiration_occurrence_missing",
    RevocationDispositionInvalid => "revocation_disposition_invalid",
    ConsumptionBindingMissing => "consumption_binding_missing",
    ConsumptionDispositionInvalid => "consumption_disposition_invalid",
    LifecycleDispositionInvalid => "lifecycle_disposition_invalid",
    LifecycleApprovalMissing => "lifecycle_approval_missing",
    LifecycleApprovalMismatch => "lifecycle_approval_mismatch",
    LifecycleApprovalPremature => "lifecycle_approval_premature",
    LifecycleOccurrenceMismatch => "lifecycle_occurrence_mismatch",
    LifecycleReplay => "lifecycle_replay",
    LifecycleSubjectMismatch => "lifecycle_subject_mismatch",
    LifecycleStateForged => "lifecycle_state_forged",
    LifecycleChainMismatch => "lifecycle_chain_mismatch",
    LifecycleTransitionInvalid => "lifecycle_transition_invalid",
    ApprovalReplayed => "approval_replayed",
    ApprovalConsumptionMismatch => "approval_consumption_mismatch",
    ApprovalOccurrenceOutOfScope => "approval_occurrence_out_of_scope",
    ApprovalUsageExhausted => "approval_usage_exhausted",
    DecisionRuntimeCapabilitiesChanged => "decision_runtime_capabilities_changed",
    DispatchRuntimeCapabilitiesChanged => "dispatch_runtime_capabilities_changed",
    ConsumptionSnapshotStale => "consumption_snapshot_stale",
    RequestNotAdopted => "request_not_adopted",
    DecisionRequestMismatch => "decision_request_mismatch",
    DecisionBindingMismatch => "decision_binding_mismatch",
    DispatchPrincipalMismatch => "dispatch_principal_mismatch",
    DispatchFamiliarMismatch => "dispatch_familiar_mismatch",
    DispatchFamiliarChanged => "dispatch_familiar_changed",
    DispatchAutomationMismatch => "dispatch_automation_mismatch",
    DispatchDefinitionChanged => "dispatch_definition_changed",
    DispatchOccurrenceMismatch => "dispatch_occurrence_mismatch",
    DispatchRunMismatch => "dispatch_run_mismatch",
    DispatchAttemptMismatch => "dispatch_attempt_mismatch",
    DispatchStaleFence => "dispatch_stale_fence",
    DispatchActionChanged => "dispatch_action_changed",
    DispatchProjectMismatch => "dispatch_project_mismatch",
    DispatchWorkspaceMismatch => "dispatch_workspace_mismatch",
    DispatchRuntimeChanged => "dispatch_runtime_changed",
    DispatchPolicyStale => "dispatch_policy_stale",
    DispatchManifestStale => "dispatch_manifest_stale",
    DispatchRuntimeDowngrade => "dispatch_runtime_downgrade",
    DecisionNotYetValid => "decision_not_yet_valid",
    DecisionExpired => "decision_expired",
    DispatchRejected => "dispatch_rejected",
    ProposalDispatchForbidden => "proposal_dispatch_forbidden",
    ApprovalRequired => "approval_required",
    ApprovalRecurringNotAllowed => "approval_recurring_not_allowed",
    ApprovalRecurringAuthorizationMissing => "approval_recurring_authorization_missing",
    ApprovalRecurringAuthorizationMismatch => "approval_recurring_authorization_mismatch",
    ApprovalRecurringAuthorizationUnexpected => "approval_recurring_authorization_unexpected",
    ApprovalRoleMismatch => "approval_role_mismatch",
    ApprovalBindingMismatch => "approval_binding_mismatch",
    ApprovalDefinitionChanged => "approval_definition_changed",
    ApprovalCapabilitiesChanged => "approval_capabilities_changed",
    ApprovalScopesChanged => "approval_scopes_changed",
    ApprovalRuntimeCapabilitiesChanged => "approval_runtime_capabilities_changed",
    ApprovalPolicyChanged => "approval_policy_changed",
    DispatchRuntimeCapabilityMissing => "dispatch_runtime_capability_missing",
    ApprovalLifecycleRequired => "approval_lifecycle_required",
    ApprovalLifecycleHeadMismatch => "approval_lifecycle_head_mismatch",
    ApprovalStateInvalid => "approval_state_invalid",
    ApprovalUnexpected => "approval_unexpected",
    ProposalStatusForged => "proposal_status_forged",
    ProposalEffectForbidden => "proposal_effect_forbidden",
    ProposalSuccessForged => "proposal_success_forged",
    ProposalAdoptionRequired => "proposal_adoption_required",
    EvidenceReadInvalidInterval => "evidence_read_invalid_interval",
    EvidenceReadTimeRequired => "evidence_read_time_required",
    EvidenceReadNotYetValid => "evidence_read_not_yet_valid",
    EvidenceReadExpired => "evidence_read_expired",
    EvidenceReaderMismatch => "evidence_reader_mismatch",
    EvidenceReadRoleMismatch => "evidence_read_role_mismatch",
    EvidenceReadUnauthorized => "evidence_read_unauthorized",
    EvidenceSensitivityUnknown => "evidence_sensitivity_unknown",
    EvidenceRetentionUnknown => "evidence_retention_unknown",
    EvidenceSubjectMismatch => "evidence_subject_mismatch",
    EvidenceSensitivityDenied => "evidence_sensitivity_denied",
    EvidenceRetentionDenied => "evidence_retention_denied",
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
