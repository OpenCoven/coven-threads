//! `ward.audit` — the daemon-owned audit store contract (§3.4, RFC-0001 §5.6).
//!
//! **One store.** `ward.audit` is a table inside the daemon's existing
//! `coven.sqlite3`, reachable through the existing socket, daemon-owned (Nova
//! non-negotiable #3). Any alternative — sidecar file, separate DB — creates two
//! sources of audit truth. The table is spelled `ward_audit` in SQL because a
//! literal dot would collide with SQLite's attached-database syntax, and an
//! attached `ward.*` database would *be* the forbidden sidecar.
//!
//! This module owns the record shape and the DDL; the daemon owns the
//! connection and the writes (Phase 2). RFC-0001 §5.6: entries MUST NOT be
//! deleted or modified — enforced in the store itself via append-only triggers,
//! not just by convention.
//!
//! WARD-C6 (compaction ledger appends to `ward.audit`) rides the same table:
//! `AuditEventType::CompactionLedger`.
//!
//! **Gate-4 applied writes** are also auditable here via
//! `AuditEventType::ApplyAudit` (see §3.4, coven-threads#5).
//!
//! ## Schema versioning and migration
//!
//! `WARD_AUDIT_SCHEMA_SQL` uses `CREATE TABLE IF NOT EXISTS`, so new empty
//! stores automatically receive the current DDL. Legacy stores must be rebuilt
//! with [`ward_audit_migration_sql`], choosing whether the source schema has a
//! `detail` column from schema reality. The migration rebuilds `ward_audit` in
//! a single transaction (create, copy, drop, rename) because SQLite cannot
//! `ALTER` a CHECK constraint.
//!
//! ## Where content hashes ride for applied writes
//!
//! `WardAuditRecord::diff_hash` carries `next_sha256` (the post-write content
//! hash, matching the RFC-0001 §5.6 `diff_hash` semantic for verdict rows).
//! The complementary `prev_sha256` and `bytes_written` ride in `detail` as a
//! compact JSON object `{"prev_sha256":"<hex-or-null>","bytes_written":N}`. This
//! choice:
//! - avoids new columns that would require another SQLite table rebuild;
//! - keeps `diff_hash` as the canonical single-hash field (consistent with
//!   verdict rows where it holds the proposal diff hash);
//! - makes `prev_sha256` query-accessible via SQLite JSON functions when
//!   needed (`json_extract(detail, '$.prev_sha256')`).

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::approval::{
    ApprovalPath, ApprovalPathKind, ProposalApprovalAuditDetail, ProposalWindowAuditDetail,
    ProposalWindowCloseAuditDetail, WindowCloseReason,
};
use crate::channel::Channel;
use crate::ids::{FamiliarId, ProposalId, SurfaceId, ThreadId, WriterId};
use crate::validate::{MutationRequest, RejectReason, Verdict};

/// Stable JSON key for the `detail` field of an `apply_audit` row;
pub const APPLY_AUDIT_DETAIL_KEY_PREV: &str = "prev_sha256";
/// Stable JSON key for the bytes-written count in an `apply_audit` detail.
pub const APPLY_AUDIT_DETAIL_KEY_BYTES: &str = "bytes_written";
/// Current `ward_audit` component schema version.
pub const WARD_AUDIT_SCHEMA_VERSION: i64 = 20;
/// SQL used after a fresh/current schema is verified without a rebuild.
pub const WARD_AUDIT_STAMP_V020_SQL: &str = "
CREATE TABLE IF NOT EXISTS ward_schema_meta (
    component TEXT PRIMARY KEY NOT NULL,
    version   INTEGER NOT NULL
);
INSERT INTO ward_schema_meta (component, version)
VALUES ('ward_audit', 20)
ON CONFLICT(component) DO UPDATE SET version = excluded.version;
";

/// Safe startup action for the daemon-owned `ward_audit` schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WardAuditSchemaAction {
    /// No table exists: create the current schema, then stamp it.
    InitializeFresh,
    /// A legacy table without `detail` exists.
    MigrateLegacyWithoutDetail,
    /// A legacy table with `detail` exists; preserve every detail payload.
    MigrateLegacyWithDetail,
    /// The table is current but predates component-version tracking.
    StampCurrent,
    /// Component metadata is newer than this library; do not downgrade.
    UnsupportedNewerVersion,
    /// Schema and version are already current.
    None,
}

/// Decide the safe audit-schema startup action from schema reality and version.
///
/// `schema_sql` must contain the SQL for the table and its Ward-owned triggers.
/// Schema reality is authoritative. Component metadata alone must never cause
/// a rebuild or suppress one because a buggy controller could stamp a legacy
/// schema.
pub fn ward_audit_schema_action(
    schema_sql: Option<&str>,
    component_version: Option<i64>,
) -> WardAuditSchemaAction {
    if component_version.is_some_and(|version| version > WARD_AUDIT_SCHEMA_VERSION) {
        return WardAuditSchemaAction::UnsupportedNewerVersion;
    }
    let Some(schema_sql) = schema_sql else {
        return WardAuditSchemaAction::InitializeFresh;
    };
    let required_fragments = [
        "detail",
        "proposal_window_opened",
        "memory_entry_admitted",
        "principal_authorized_write",
        "apply_audit",
        "json_array_length(detail, '$.entry_hash') = 32",
        "ward_audit_require_authorization_insert",
        "ward_audit_require_proposal_approval_detail_insert",
        "ward_audit_require_window_close_detail_insert",
        "ward_audit_require_single_terminal_insert",
        "julianday(json_extract(detail, '$.deadline')) IS NOT NULL",
        "NOT GLOB '*[^0-9A-Fa-f]*'",
    ];
    if required_fragments
        .iter()
        .any(|fragment| !schema_sql.contains(fragment))
    {
        return if schema_sql.contains("detail") {
            WardAuditSchemaAction::MigrateLegacyWithDetail
        } else {
            WardAuditSchemaAction::MigrateLegacyWithoutDetail
        };
    }
    if component_version.unwrap_or_default() < WARD_AUDIT_SCHEMA_VERSION {
        WardAuditSchemaAction::StampCurrent
    } else {
        WardAuditSchemaAction::None
    }
}

/// RFC-0001 §5.6 detail for a standard memory admission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryEntryAdmissionAuditDetail {
    /// Hash of the admitted memory entry.
    pub entry_hash: Vec<u8>,
    /// Prior committed Ward state or principal-authorized write event.
    pub source_attestation: String,
}

/// RFC-0001 §5.6 detail for a principal-authorized memory write.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrincipalAuthorizedWriteAuditDetail {
    /// Principal authorization evidence for the write.
    pub principal_authorization: String,
}

/// Event types recorded in `ward.audit`.
///
/// The first five are RFC-0001 §5.6's named set, verbatim. The last two are the
/// coven-threads extensions: every gate verdict is auditable (§5), and WARD-C6's
/// compaction ledger lands here rather than in a second store (§3.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    /// A proposal was submitted for review (RFC-0001 §5.6).
    ProposalSubmitted,
    /// A delayed-apply veto window became visible and auditable.
    ProposalWindowOpened,
    /// A proposal was approved (RFC-0001 §5.6).
    ProposalApproved,
    /// A proposal was rejected (RFC-0001 §5.6).
    ProposalRejected,
    /// A proposal was vetoed inside its veto window (RFC-0001 §5.6).
    ProposalVetoed,
    /// The Ward itself was updated (RFC-0001 §5.6).
    WardUpdated,
    /// A continuity-bearing memory entry was admitted with provenance.
    MemoryEntryAdmitted,
    /// A principal authorized a memory write outside proposal admission.
    PrincipalAuthorizedWrite,
    /// A gate verdict was issued by `validate` (§5 — every verdict is audited).
    ValidationVerdict,
    /// WARD-C6 compaction ledger entry (§3.3, inherited by reference).
    CompactionLedger,
    /// A Gate-4 Tier-2 write was applied and its content snapshot recorded
    /// (coven-threads#5, coven#414).
    ///
    /// Field mapping for this event type:
    /// - `diff_hash` — `next_sha256` (post-write content hash of the surface).
    /// - `detail`    — JSON `{"prev_sha256":"<hex>","bytes_written":N}`;
    ///   `prev_sha256` is the pre-write content hash, `bytes_written` is the
    ///   number of bytes written to the surface.
    /// - `tier`      — typically `"tier_2"` (logged-write tier).
    /// - `decision`  — `"applied"` (the write completed successfully).
    /// - `files_touched` — the resolved surface ids that were written.
    ApplyAudit,
}

impl AuditEventType {
    /// Stable string tag (matches the serde snake_case encoding).
    pub fn tag(&self) -> &'static str {
        match self {
            AuditEventType::ProposalSubmitted => "proposal_submitted",
            AuditEventType::ProposalWindowOpened => "proposal_window_opened",
            AuditEventType::ProposalApproved => "proposal_approved",
            AuditEventType::ProposalRejected => "proposal_rejected",
            AuditEventType::ProposalVetoed => "proposal_vetoed",
            AuditEventType::WardUpdated => "ward_updated",
            AuditEventType::MemoryEntryAdmitted => "memory_entry_admitted",
            AuditEventType::PrincipalAuthorizedWrite => "principal_authorized_write",
            AuditEventType::ValidationVerdict => "validation_verdict",
            AuditEventType::CompactionLedger => "compaction_ledger",
            AuditEventType::ApplyAudit => "apply_audit",
        }
    }
}

/// One row of `ward.audit` (RFC-0001 §5.6 field set).
///
/// For `ApplyAudit` events, the extra per-apply fields (`prev_sha256`,
/// `bytes_written`) are encoded in the `detail` field as a JSON object (see
/// module-level docs and [`APPLY_AUDIT_DETAIL_KEY_PREV`]).
///
/// `ward_hash` is RFC-0001 §5.6's audit-log field; at this layer it is the
/// `weave_hash` of the weave the verdict was issued against — the commitment
/// binding "which authority state decided this."
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WardAuditRecord {
    /// What happened.
    pub event_type: AuditEventType,
    /// The proposal this event concerns, if any.
    pub proposal_id: Option<ProposalId>,
    /// The familiar whose weave was consulted.
    pub familiar_id: FamiliarId,
    /// Ward/weave version tag (semver string of the ward document, if known).
    pub ward_version: Option<String>,
    /// The `weave_hash` at decision time (RFC-0001 §5.6 `ward_hash`).
    pub ward_hash: Vec<u8>,
    /// Approval tier, when the event is tier-mediated (RFC-0001 §5.3).
    pub tier: Option<String>,
    /// The decision, as a stable tag (e.g. `permit`, `degrade_to_proposal`,
    /// `reject:unknown_surface`).
    pub decision: String,
    /// Who approved/decided, when a principal or tier actor is involved.
    pub approver: Option<WriterId>,
    /// Hash of the proposed diff, when the event carries one.
    ///
    /// For `ApplyAudit` events this is the `next_sha256` — the post-write
    /// content hash of the surface.
    pub diff_hash: Option<Vec<u8>>,
    /// Opaque JSON detail payload, event-type–specific.
    ///
    /// **Invariant for `ApplyAudit` rows:** `detail` MUST be a JSON object
    /// containing exactly:
    /// - `"prev_sha256"` — hex string (64 lower-case ASCII chars), the
    ///   pre-write SHA-256 of the surface content, or `null` when unknown.
    /// - `"bytes_written"` — u64, number of bytes written to the surface.
    ///
    /// Other event types MAY leave `detail` as `None`, or use event-type-
    /// specific shapes documented at their construction site. Callers MUST
    /// NOT assume `detail` is non-null for non-`ApplyAudit` rows.
    pub detail: Option<String>,
    /// The surfaces this event touches.
    pub files_touched: Vec<SurfaceId>,
    /// The channel the triggering request arrived on, if any (§2.4).
    pub channel: Option<Channel>,
    /// The thread that carried (or refused) the authority, if any.
    pub thread_id: Option<ThreadId>,
    /// When the triggering request was submitted.
    pub submitted_at: OffsetDateTime,
    /// When the decision was made.
    pub decided_at: OffsetDateTime,
}

impl WardAuditRecord {
    /// Build the audit row for a gate verdict (§5: the daemon appends one of
    /// these for every `validate` call it acts on).
    pub fn for_verdict(
        familiar_id: FamiliarId,
        weave_hash: &[u8],
        request: &MutationRequest,
        verdict: &Verdict,
        submitted_at: OffsetDateTime,
        decided_at: OffsetDateTime,
    ) -> Self {
        let (decision, thread_id, event_type) = match verdict {
            Verdict::Permit { thread } => (
                "permit".to_string(),
                Some(*thread),
                AuditEventType::ValidationVerdict,
            ),
            Verdict::DegradeToProposal { thread, .. } => (
                "degrade_to_proposal".to_string(),
                Some(*thread),
                AuditEventType::ValidationVerdict,
            ),
            Verdict::Reject { reason } => (
                format!("reject:{}", reject_tag(reason)),
                None,
                AuditEventType::ValidationVerdict,
            ),
        };
        Self {
            event_type,
            proposal_id: None,
            familiar_id,
            ward_version: None,
            ward_hash: weave_hash.to_vec(),
            tier: None,
            decision,
            approver: None,
            diff_hash: None,
            detail: None,
            files_touched: vec![request.surface.clone()],
            channel: Some(request.channel),
            thread_id,
            submitted_at,
            decided_at,
        }
    }

    /// Build the audit row for a Gate-4 Tier-2 applied write (coven-threads#5).
    ///
    /// `next_hash` is the post-write content hash (SHA-256) of the surface;
    /// it rides in `diff_hash` matching the single-hash pattern used for
    /// verdict rows.
    ///
    /// `prev_hash` (pre-write content hash) and `bytes_written` are encoded in
    /// `detail` as `{"prev_sha256":"<hex>","bytes_written":N}` because adding
    /// typed columns would require an immediate SQLite table rebuild in all
    /// consumers.
    ///
    /// `weave_hash` is the Ward/weave version the Gate-4 decision was made
    /// against (RFC-0001 §5.6 `ward_hash`).
    #[allow(clippy::too_many_arguments)]
    pub fn for_apply(
        familiar_id: FamiliarId,
        weave_hash: &[u8],
        surface: SurfaceId,
        tier: &str,
        prev_hash: Option<&[u8]>,
        next_hash: Option<&[u8]>,
        bytes_written: u64,
        channel: Option<Channel>,
        submitted_at: OffsetDateTime,
        decided_at: OffsetDateTime,
    ) -> Self {
        let prev_hex = prev_hash.map(bytes_to_hex);
        let detail = serde_json::json!({
            APPLY_AUDIT_DETAIL_KEY_PREV: prev_hex,
            APPLY_AUDIT_DETAIL_KEY_BYTES: bytes_written,
        })
        .to_string();
        Self {
            event_type: AuditEventType::ApplyAudit,
            proposal_id: None,
            familiar_id,
            ward_version: None,
            ward_hash: weave_hash.to_vec(),
            tier: Some(tier.to_string()),
            decision: "applied".to_string(),
            approver: None,
            diff_hash: next_hash.map(|h| h.to_vec()),
            detail: Some(detail),
            files_touched: vec![surface],
            channel,
            thread_id: None,
            submitted_at,
            decided_at,
        }
    }

    /// Build an RFC-0001 §5.6 standard memory-admission audit row.
    pub fn for_memory_entry_admitted(
        familiar_id: FamiliarId,
        weave_hash: &[u8],
        entry_hash: &[u8],
        source_attestation: impl Into<String>,
        decided_at: OffsetDateTime,
    ) -> Self {
        let detail = MemoryEntryAdmissionAuditDetail {
            entry_hash: entry_hash.to_vec(),
            source_attestation: source_attestation.into(),
        };
        Self {
            event_type: AuditEventType::MemoryEntryAdmitted,
            proposal_id: None,
            familiar_id,
            ward_version: None,
            ward_hash: weave_hash.to_vec(),
            tier: None,
            decision: "admitted".into(),
            approver: None,
            diff_hash: None,
            detail: Some(serde_json::to_string(&detail).expect("serializing typed audit detail")),
            files_touched: Vec::new(),
            channel: None,
            thread_id: None,
            submitted_at: decided_at,
            decided_at,
        }
    }

    /// Build an RFC-0001 §5.6 principal-authorized memory-write audit row.
    pub fn for_principal_authorized_write(
        familiar_id: FamiliarId,
        weave_hash: &[u8],
        approver: WriterId,
        principal_authorization: impl Into<String>,
        files_touched: Vec<SurfaceId>,
        decided_at: OffsetDateTime,
    ) -> Self {
        let detail = PrincipalAuthorizedWriteAuditDetail {
            principal_authorization: principal_authorization.into(),
        };
        Self {
            event_type: AuditEventType::PrincipalAuthorizedWrite,
            proposal_id: None,
            familiar_id,
            ward_version: None,
            ward_hash: weave_hash.to_vec(),
            tier: None,
            decision: "authorized".into(),
            approver: Some(approver),
            diff_hash: None,
            detail: Some(serde_json::to_string(&detail).expect("serializing typed audit detail")),
            files_touched,
            channel: Some(Channel::Mutation),
            thread_id: None,
            submitted_at: decided_at,
            decided_at,
        }
    }

    /// Validate event-specific detail requirements before persistence.
    pub fn validate_event_detail(&self) -> Result<(), String> {
        if matches!(
            self.event_type,
            AuditEventType::ProposalApproved
                | AuditEventType::ProposalRejected
                | AuditEventType::ProposalVetoed
        ) && self.proposal_id.is_none()
        {
            return Err("proposal terminal events require proposal_id".into());
        }
        match self.event_type {
            AuditEventType::ProposalWindowOpened => {
                let detail: ProposalWindowAuditDetail = serde_json::from_str(
                    self.detail
                        .as_deref()
                        .ok_or("proposal_window_opened requires detail")?,
                )
                .map_err(|error| format!("invalid proposal window detail: {error}"))?;
                if self.proposal_id.is_none()
                    || detail.approval_path_label.trim().is_empty()
                    || detail.evidence_replay_hash_hex.len() != 64
                    || !detail
                        .evidence_replay_hash_hex
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit())
                    || detail.earliest_close > detail.deadline
                {
                    return Err(
                        "proposal_window_opened requires a valid path, replay hash, and interval"
                            .into(),
                    );
                }
            }
            AuditEventType::ProposalApproved => {
                let detail: ProposalApprovalAuditDetail = serde_json::from_str(
                    self.detail
                        .as_deref()
                        .ok_or("proposal_approved requires approval detail")?,
                )
                .map_err(|error| format!("invalid proposal approval detail: {error}"))?;
                validate_proposal_approval_detail(self.approver.as_ref(), &detail)?;
            }
            AuditEventType::ProposalRejected | AuditEventType::ProposalVetoed
                if self.detail.is_some() =>
            {
                let detail: ProposalWindowCloseAuditDetail =
                    serde_json::from_str(self.detail.as_deref().expect("detail checked above"))
                        .map_err(|error| {
                            format!("invalid proposal window close detail: {error}")
                        })?;
                validate_window_close_detail(&self.event_type, &detail)?;
            }
            AuditEventType::MemoryEntryAdmitted => {
                let detail: MemoryEntryAdmissionAuditDetail = serde_json::from_str(
                    self.detail
                        .as_deref()
                        .ok_or("memory_entry_admitted requires detail")?,
                )
                .map_err(|error| format!("invalid memory admission detail: {error}"))?;
                if detail.entry_hash.len() != 32 || detail.source_attestation.trim().is_empty() {
                    return Err(
                        "memory_entry_admitted requires entry_hash and source_attestation".into(),
                    );
                }
            }
            AuditEventType::WardUpdated | AuditEventType::PrincipalAuthorizedWrite => {
                let detail: PrincipalAuthorizedWriteAuditDetail = serde_json::from_str(
                    self.detail
                        .as_deref()
                        .ok_or("authorized Ward writes require detail")?,
                )
                .map_err(|error| format!("invalid principal authorization detail: {error}"))?;
                if detail.principal_authorization.trim().is_empty() {
                    return Err("authorized Ward writes require principal_authorization".into());
                }
            }
            AuditEventType::ApplyAudit => {
                let detail: serde_json::Value = serde_json::from_str(
                    self.detail
                        .as_deref()
                        .ok_or("apply_audit requires detail")?,
                )
                .map_err(|error| format!("invalid apply_audit detail: {error}"))?;
                let object = detail
                    .as_object()
                    .ok_or("apply_audit detail must be a JSON object")?;
                if object.len() != 2
                    || !object.contains_key(APPLY_AUDIT_DETAIL_KEY_PREV)
                    || !object.contains_key(APPLY_AUDIT_DETAIL_KEY_BYTES)
                {
                    return Err(format!(
                        "apply_audit detail must contain exactly \
                         {APPLY_AUDIT_DETAIL_KEY_PREV} and {APPLY_AUDIT_DETAIL_KEY_BYTES}"
                    ));
                }
                let prev_is_valid = match &object[APPLY_AUDIT_DETAIL_KEY_PREV] {
                    serde_json::Value::Null => true,
                    serde_json::Value::String(hex) => {
                        hex.len() == 64
                            && hex
                                .bytes()
                                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
                    }
                    _ => false,
                };
                if !prev_is_valid {
                    return Err(format!(
                        "apply_audit {APPLY_AUDIT_DETAIL_KEY_PREV} must be null or \
                         64 lower-case hex chars"
                    ));
                }
                if !object[APPLY_AUDIT_DETAIL_KEY_BYTES].is_u64() {
                    return Err(format!(
                        "apply_audit {APPLY_AUDIT_DETAIL_KEY_BYTES} must be a u64"
                    ));
                }
            }
            // No detail contract for these event types: `detail` is optional
            // freeform. Terminal events reaching here (rejected/vetoed without
            // detail) already passed the proposal_id check above; when they do
            // carry detail, the guarded arm above validates it.
            AuditEventType::ProposalSubmitted
            | AuditEventType::ProposalRejected
            | AuditEventType::ProposalVetoed
            | AuditEventType::ValidationVerdict
            | AuditEventType::CompactionLedger => {}
        }
        Ok(())
    }

    /// Decode `prev_sha256` from the `detail` JSON for `ApplyAudit` rows.
    ///
    /// Returns `None` for non-`ApplyAudit` events or if the field is missing.
    pub fn apply_prev_sha256_hex(&self) -> Option<String> {
        if self.event_type != AuditEventType::ApplyAudit {
            return None;
        }
        let detail = self.detail.as_deref()?;
        let v: serde_json::Value = serde_json::from_str(detail).ok()?;
        v.get(APPLY_AUDIT_DETAIL_KEY_PREV)
            .and_then(|x| x.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    }

    /// Decode `bytes_written` from the `detail` JSON for `ApplyAudit` rows.
    ///
    /// Returns `None` for non-`ApplyAudit` events or if the field is missing.
    pub fn apply_bytes_written(&self) -> Option<u64> {
        if self.event_type != AuditEventType::ApplyAudit {
            return None;
        }
        let detail = self.detail.as_deref()?;
        let v: serde_json::Value = serde_json::from_str(detail).ok()?;
        v.get(APPLY_AUDIT_DETAIL_KEY_BYTES).and_then(|x| x.as_u64())
    }
}

fn validate_window_close_detail(
    event_type: &AuditEventType,
    detail: &ProposalWindowCloseAuditDetail,
) -> Result<(), String> {
    let event_matches_reason = match event_type {
        AuditEventType::ProposalApproved => detail.reason == WindowCloseReason::Applied,
        AuditEventType::ProposalVetoed => detail.reason == WindowCloseReason::Vetoed,
        AuditEventType::ProposalRejected => matches!(
            detail.reason,
            WindowCloseReason::EvidenceDiverged
                | WindowCloseReason::RevalidationFailed
                | WindowCloseReason::Superseded
        ),
        _ => false,
    };
    if !event_matches_reason {
        return Err("proposal terminal event does not match its window close reason".into());
    }

    let replay_result_matches = match detail.reason {
        WindowCloseReason::Applied => detail.replay_hash_matched == Some(true),
        WindowCloseReason::EvidenceDiverged | WindowCloseReason::RevalidationFailed => {
            detail.replay_hash_matched == Some(false)
        }
        WindowCloseReason::Vetoed | WindowCloseReason::Superseded => {
            detail.replay_hash_matched.is_none()
        }
    };
    if !replay_result_matches {
        return Err("window close replay result does not match the close reason".into());
    }
    Ok(())
}

fn validate_proposal_approval_detail(
    approver: Option<&WriterId>,
    detail: &ProposalApprovalAuditDetail,
) -> Result<(), String> {
    let has_approver = approver
        .map(WriterId::as_str)
        .is_some_and(|value| !value.trim().is_empty());
    let path = ApprovalPath::from_display_label(&detail.approval_path_label)
        .ok_or("proposal approval carries an unknown approval path")?;
    match path {
        ApprovalPathKind::HumanApprovalWithRationale => {
            if !has_approver
                || detail
                    .rationale
                    .as_deref()
                    .map_or(true, |rationale| rationale.trim().is_empty())
            {
                return Err("human_required approval requires an approver and rationale".into());
            }
            if detail.window_close.is_some() {
                return Err("human approval paths must not carry window close evidence".into());
            }
        }
        ApprovalPathKind::HumanApproval => {
            if !has_approver {
                return Err("human_review approval requires an approver".into());
            }
            if detail.window_close.is_some() {
                return Err("human approval paths must not carry window close evidence".into());
            }
        }
        ApprovalPathKind::FamiliarCoherence => {
            let close = detail
                .window_close
                .as_ref()
                .ok_or("familiar_review approval requires window close evidence")?;
            validate_window_close_detail(&AuditEventType::ProposalApproved, close)?;
        }
        ApprovalPathKind::AutoRegression => {
            if let Some(close) = &detail.window_close {
                validate_window_close_detail(&AuditEventType::ProposalApproved, close)?;
            }
        }
    }
    Ok(())
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn reject_tag(reason: &RejectReason) -> &'static str {
    match reason {
        RejectReason::UnknownSurface { .. } => "unknown_surface",
        RejectReason::WriterNotBound { .. } => "writer_not_bound",
        RejectReason::ChannelNotCovered { .. } => "channel_not_covered",
        RejectReason::ThreadSnapped { .. } => "thread_snapped",
        RejectReason::WeaveBroken { .. } => "weave_broken",
        RejectReason::SurfaceDegraded { .. } => "surface_degraded",
        RejectReason::ValidatorPanic { .. } => "validator_panic",
    }
}

/// DDL for the `ward.audit` table inside `coven.sqlite3` (§3.4).
///
/// Idempotent (`IF NOT EXISTS` throughout) so the daemon can apply it at
/// startup. Append-only is enforced *in the store*: UPDATE and DELETE abort via
/// triggers (RFC-0001 §5.6: entries MUST NOT be deleted or modified).
/// Template for rebuilding a legacy `ward_audit` table to schema v20.
///
/// SQLite cannot `ALTER` a CHECK constraint on an existing table. This SQL
/// performs a safe swap in one transaction:
/// 1. Creates `ward_audit_new` with the updated CHECK (plus `detail` column).
/// 2. Copies all existing rows (NULLing `detail` for old rows).
/// 3. Drops the old table.
/// 4. Renames the new table into place.
/// 5. Re-creates indexes and append-only triggers.
///
/// Callers must inspect `sqlite_master` and use [`ward_audit_schema_action`].
/// [`ward_audit_migration_sql`] substitutes the correct source expression so
/// stores with an existing `detail` column preserve every payload while older
/// stores use SQL `NULL`.
///
/// The `ward_audit_new` table name is not guarded with `IF NOT EXISTS`, so a
/// double-run fails on the `CREATE` step and cannot silently lose data.
const WARD_AUDIT_MIGRATION_TEMPLATE_SQL: &str = r#"
BEGIN;

CREATE TABLE ward_audit_new (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type    TEXT    NOT NULL CHECK (event_type IN (
                      'proposal_submitted','proposal_window_opened',
                      'proposal_approved','proposal_rejected',
                      'proposal_vetoed','ward_updated','memory_entry_admitted',
                      'principal_authorized_write','validation_verdict',
                      'compaction_ledger','apply_audit')),
    proposal_id   TEXT,
    familiar_id   TEXT    NOT NULL,
    ward_version  TEXT,
    ward_hash     BLOB    NOT NULL,
    tier          TEXT,
    decision      TEXT    NOT NULL,
    approver      TEXT,
    diff_hash     BLOB,
    detail        TEXT,
    files_touched TEXT    NOT NULL,
    channel       TEXT,
    thread_id     TEXT,
    submitted_at  TEXT    NOT NULL,
    decided_at    TEXT    NOT NULL,
    recorded_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    CHECK (
        event_type != 'proposal_window_opened' OR (
            proposal_id IS NOT NULL
            AND detail IS NOT NULL AND json_valid(detail)
            AND json_type(detail, '$.approval_path_label') IS 'text'
            AND length(trim(json_extract(detail, '$.approval_path_label'))) > 0
            AND json_type(detail, '$.deadline') IS 'text'
            AND julianday(json_extract(detail, '$.deadline')) IS NOT NULL
            AND json_type(detail, '$.earliest_close') IS 'text'
            AND julianday(json_extract(detail, '$.earliest_close')) IS NOT NULL
            AND julianday(json_extract(detail, '$.earliest_close'))
                <= julianday(json_extract(detail, '$.deadline'))
            AND json_type(detail, '$.evidence_replay_hash_hex') IS 'text'
            AND length(json_extract(detail, '$.evidence_replay_hash_hex')) = 64
            AND json_extract(detail, '$.evidence_replay_hash_hex')
                NOT GLOB '*[^0-9A-Fa-f]*'
            AND json_type(detail, '$.affected_regions') IS 'array'
        )
    ),
    CHECK (
        event_type != 'memory_entry_admitted' OR (
            detail IS NOT NULL AND json_valid(detail)
            AND json_type(detail, '$.entry_hash') IS 'array'
            AND json_array_length(detail, '$.entry_hash') = 32
            AND json_type(detail, '$.source_attestation') IS 'text'
            AND length(trim(json_extract(detail, '$.source_attestation'))) > 0
        )
    )
);

INSERT INTO ward_audit_new (
    id, event_type, proposal_id, familiar_id, ward_version, ward_hash,
    tier, decision, approver, diff_hash, detail, files_touched, channel,
    thread_id, submitted_at, decided_at, recorded_at
)
SELECT
    id, event_type, proposal_id, familiar_id, ward_version, ward_hash,
    tier, decision, approver, diff_hash, __SOURCE_DETAIL__, files_touched, channel,
    thread_id, submitted_at, decided_at, recorded_at
FROM ward_audit;

DROP TABLE ward_audit;
ALTER TABLE ward_audit_new RENAME TO ward_audit;

CREATE INDEX IF NOT EXISTS ward_audit_familiar_idx ON ward_audit (familiar_id, recorded_at);
CREATE INDEX IF NOT EXISTS ward_audit_event_idx    ON ward_audit (event_type, recorded_at);
CREATE TRIGGER IF NOT EXISTS ward_audit_append_only_update
BEFORE UPDATE ON ward_audit
BEGIN
    SELECT RAISE(ABORT, 'ward_audit is append-only (RFC-0001 §5.6)');
END;

CREATE TRIGGER IF NOT EXISTS ward_audit_append_only_delete
BEFORE DELETE ON ward_audit
BEGIN
    SELECT RAISE(ABORT, 'ward_audit is append-only (RFC-0001 §5.6)');
END;

CREATE TRIGGER IF NOT EXISTS ward_audit_require_single_terminal_insert
BEFORE INSERT ON ward_audit
WHEN NEW.event_type IN ('proposal_approved','proposal_rejected','proposal_vetoed')
    AND (
        NEW.proposal_id IS NULL
        OR EXISTS (
            SELECT 1 FROM ward_audit
            WHERE proposal_id = NEW.proposal_id
              AND event_type IN (
                  'proposal_approved','proposal_rejected','proposal_vetoed'
              )
        )
    )
BEGIN
    SELECT RAISE(ABORT, 'proposal requires exactly one terminal event');
END;

CREATE TRIGGER IF NOT EXISTS ward_audit_require_authorization_insert
BEFORE INSERT ON ward_audit
WHEN NEW.event_type IN ('ward_updated', 'principal_authorized_write')
    AND (
        NEW.detail IS NULL OR NOT json_valid(NEW.detail)
        OR json_type(NEW.detail, '$.principal_authorization') IS NOT 'text'
        OR COALESCE(length(trim(json_extract(NEW.detail, '$.principal_authorization'))), 0) = 0
    )
BEGIN
    SELECT RAISE(ABORT, 'authorized Ward writes require principal_authorization');
END;

CREATE TRIGGER IF NOT EXISTS ward_audit_require_proposal_approval_detail_insert
BEFORE INSERT ON ward_audit
WHEN NEW.event_type = 'proposal_approved'
    AND (
        NEW.detail IS NULL OR NOT json_valid(NEW.detail)
        OR json_type(NEW.detail, '$.approval_path_label') IS NOT 'text'
        OR json_extract(NEW.detail, '$.approval_path_label')
            NOT IN ('auto','familiar_review','human_review','human_required')
        OR COALESCE(json_type(NEW.detail, '$.rationale'), 'missing')
            NOT IN ('null','text')
        OR COALESCE(json_type(NEW.detail, '$.window_close'), 'missing')
            NOT IN ('null','object')
        OR (
            json_extract(NEW.detail, '$.approval_path_label')
                IN ('human_review','human_required')
            AND (
                NEW.approver IS NULL
                OR length(trim(NEW.approver)) = 0
            )
        )
        OR (
            json_extract(NEW.detail, '$.approval_path_label') = 'human_required'
            AND (
                json_type(NEW.detail, '$.rationale') IS NOT 'text'
                OR length(trim(json_extract(NEW.detail, '$.rationale'))) = 0
            )
        )
        OR (
            json_extract(NEW.detail, '$.approval_path_label')
                IN ('human_review','human_required')
            AND json_type(NEW.detail, '$.window_close') IS NOT 'null'
        )
        OR (
            json_extract(NEW.detail, '$.approval_path_label') = 'familiar_review'
            AND json_type(NEW.detail, '$.window_close') IS NOT 'object'
        )
        OR (
            EXISTS (
                SELECT 1 FROM ward_audit
                WHERE proposal_id = NEW.proposal_id
                  AND event_type = 'proposal_window_opened'
            )
            AND json_type(NEW.detail, '$.window_close') IS NOT 'object'
        )
        OR (
            json_type(NEW.detail, '$.window_close') IS 'object'
            AND (
                json_extract(NEW.detail, '$.window_close.reason') != 'applied'
                OR json_type(NEW.detail, '$.window_close.replay_hash_matched')
                    IS NOT 'true'
                OR COALESCE(
                    json_type(NEW.detail, '$.window_close.rationale'),
                    'missing'
                ) NOT IN ('null','text')
            )
        )
    )
BEGIN
    SELECT RAISE(ABORT, 'proposal approval requires valid path-specific detail');
END;

CREATE TRIGGER IF NOT EXISTS ward_audit_require_window_close_detail_insert
BEFORE INSERT ON ward_audit
WHEN NEW.event_type IN ('proposal_rejected', 'proposal_vetoed')
    AND EXISTS (
        SELECT 1 FROM ward_audit
        WHERE proposal_id = NEW.proposal_id
          AND event_type = 'proposal_window_opened'
    )
    AND (
        NEW.detail IS NULL OR NOT json_valid(NEW.detail)
        OR json_type(NEW.detail, '$.reason') IS NOT 'text'
        OR json_extract(NEW.detail, '$.reason') NOT IN (
            'applied','vetoed','evidence_diverged','revalidation_failed','superseded'
        )
        OR COALESCE(json_type(NEW.detail, '$.replay_hash_matched'), 'missing')
            NOT IN ('null','true','false')
        OR COALESCE(json_type(NEW.detail, '$.rationale'), 'missing')
            NOT IN ('null','text')
        OR (
            NEW.event_type = 'proposal_vetoed'
            AND json_extract(NEW.detail, '$.reason') != 'vetoed'
        )
        OR (
            NEW.event_type = 'proposal_rejected'
            AND json_extract(NEW.detail, '$.reason')
                NOT IN ('evidence_diverged','revalidation_failed','superseded')
        )
        OR (
            json_extract(NEW.detail, '$.reason') = 'applied'
            AND json_type(NEW.detail, '$.replay_hash_matched') IS NOT 'true'
        )
        OR (
            json_extract(NEW.detail, '$.reason')
                IN ('evidence_diverged','revalidation_failed')
            AND json_type(NEW.detail, '$.replay_hash_matched') IS NOT 'false'
        )
        OR (
            json_extract(NEW.detail, '$.reason') IN ('vetoed','superseded')
            AND json_type(NEW.detail, '$.replay_hash_matched') IS NOT 'null'
        )
    )
BEGIN
    SELECT RAISE(ABORT, 'window terminal events require a valid close reason');
END;

CREATE TABLE IF NOT EXISTS ward_schema_meta (
    component TEXT PRIMARY KEY NOT NULL,
    version   INTEGER NOT NULL
);
INSERT INTO ward_schema_meta (component, version)
VALUES ('ward_audit', 20)
ON CONFLICT(component) DO UPDATE SET version = excluded.version;

COMMIT;
"#;

/// Build the transactional legacy migration selected from schema reality.
///
/// Set `source_has_detail` only when the source table actually contains that
/// column. The resulting SQL either copies `detail` byte-for-byte or supplies
/// `NULL` for pre-detail schemas.
pub fn ward_audit_migration_sql(source_has_detail: bool) -> String {
    WARD_AUDIT_MIGRATION_TEMPLATE_SQL.replace(
        "__SOURCE_DETAIL__",
        if source_has_detail { "detail" } else { "NULL" },
    )
}

/// DDL for the `ward.audit` table inside `coven.sqlite3` (§3.4).
///
/// See module docs for migration notes.
pub const WARD_AUDIT_SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS ward_audit (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type    TEXT    NOT NULL CHECK (event_type IN (
                      'proposal_submitted','proposal_window_opened',
                      'proposal_approved','proposal_rejected',
                      'proposal_vetoed','ward_updated','memory_entry_admitted',
                      'principal_authorized_write','validation_verdict',
                      'compaction_ledger','apply_audit')),
    proposal_id   TEXT,
    familiar_id   TEXT    NOT NULL,
    ward_version  TEXT,
    ward_hash     BLOB    NOT NULL,
    tier          TEXT,
    decision      TEXT    NOT NULL,
    approver      TEXT,
    diff_hash     BLOB,
    detail        TEXT,             -- event-type-specific JSON; see module docs
    files_touched TEXT    NOT NULL, -- JSON array of surface ids
    channel       TEXT,
    thread_id     TEXT,
    submitted_at  TEXT    NOT NULL, -- RFC 3339
    decided_at    TEXT    NOT NULL, -- RFC 3339
    recorded_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    CHECK (
        event_type != 'proposal_window_opened' OR (
            proposal_id IS NOT NULL
            AND detail IS NOT NULL AND json_valid(detail)
            AND json_type(detail, '$.approval_path_label') IS 'text'
            AND length(trim(json_extract(detail, '$.approval_path_label'))) > 0
            AND json_type(detail, '$.deadline') IS 'text'
            AND julianday(json_extract(detail, '$.deadline')) IS NOT NULL
            AND json_type(detail, '$.earliest_close') IS 'text'
            AND julianday(json_extract(detail, '$.earliest_close')) IS NOT NULL
            AND julianday(json_extract(detail, '$.earliest_close'))
                <= julianday(json_extract(detail, '$.deadline'))
            AND json_type(detail, '$.evidence_replay_hash_hex') IS 'text'
            AND length(json_extract(detail, '$.evidence_replay_hash_hex')) = 64
            AND json_extract(detail, '$.evidence_replay_hash_hex')
                NOT GLOB '*[^0-9A-Fa-f]*'
            AND json_type(detail, '$.affected_regions') IS 'array'
        )
    ),
    CHECK (
        event_type != 'memory_entry_admitted' OR (
            detail IS NOT NULL AND json_valid(detail)
            AND json_type(detail, '$.entry_hash') IS 'array'
            AND json_array_length(detail, '$.entry_hash') = 32
            AND json_type(detail, '$.source_attestation') IS 'text'
            AND length(trim(json_extract(detail, '$.source_attestation'))) > 0
        )
    )
);

CREATE INDEX IF NOT EXISTS ward_audit_familiar_idx ON ward_audit (familiar_id, recorded_at);
CREATE INDEX IF NOT EXISTS ward_audit_event_idx    ON ward_audit (event_type, recorded_at);
CREATE TRIGGER IF NOT EXISTS ward_audit_append_only_update
BEFORE UPDATE ON ward_audit
BEGIN
    SELECT RAISE(ABORT, 'ward_audit is append-only (RFC-0001 §5.6)');
END;

CREATE TRIGGER IF NOT EXISTS ward_audit_append_only_delete
BEFORE DELETE ON ward_audit
BEGIN
    SELECT RAISE(ABORT, 'ward_audit is append-only (RFC-0001 §5.6)');
END;

CREATE TRIGGER IF NOT EXISTS ward_audit_require_single_terminal_insert
BEFORE INSERT ON ward_audit
WHEN NEW.event_type IN ('proposal_approved','proposal_rejected','proposal_vetoed')
    AND (
        NEW.proposal_id IS NULL
        OR EXISTS (
            SELECT 1 FROM ward_audit
            WHERE proposal_id = NEW.proposal_id
              AND event_type IN (
                  'proposal_approved','proposal_rejected','proposal_vetoed'
              )
        )
    )
BEGIN
    SELECT RAISE(ABORT, 'proposal requires exactly one terminal event');
END;

CREATE TRIGGER IF NOT EXISTS ward_audit_require_authorization_insert
BEFORE INSERT ON ward_audit
WHEN NEW.event_type IN ('ward_updated', 'principal_authorized_write')
    AND (
        NEW.detail IS NULL OR NOT json_valid(NEW.detail)
        OR json_type(NEW.detail, '$.principal_authorization') IS NOT 'text'
        OR COALESCE(length(trim(json_extract(NEW.detail, '$.principal_authorization'))), 0) = 0
    )
BEGIN
    SELECT RAISE(ABORT, 'authorized Ward writes require principal_authorization');
END;

CREATE TRIGGER IF NOT EXISTS ward_audit_require_proposal_approval_detail_insert
BEFORE INSERT ON ward_audit
WHEN NEW.event_type = 'proposal_approved'
    AND (
        NEW.detail IS NULL OR NOT json_valid(NEW.detail)
        OR json_type(NEW.detail, '$.approval_path_label') IS NOT 'text'
        OR json_extract(NEW.detail, '$.approval_path_label')
            NOT IN ('auto','familiar_review','human_review','human_required')
        OR COALESCE(json_type(NEW.detail, '$.rationale'), 'missing')
            NOT IN ('null','text')
        OR COALESCE(json_type(NEW.detail, '$.window_close'), 'missing')
            NOT IN ('null','object')
        OR (
            json_extract(NEW.detail, '$.approval_path_label')
                IN ('human_review','human_required')
            AND (
                NEW.approver IS NULL
                OR length(trim(NEW.approver)) = 0
            )
        )
        OR (
            json_extract(NEW.detail, '$.approval_path_label') = 'human_required'
            AND (
                json_type(NEW.detail, '$.rationale') IS NOT 'text'
                OR length(trim(json_extract(NEW.detail, '$.rationale'))) = 0
            )
        )
        OR (
            json_extract(NEW.detail, '$.approval_path_label')
                IN ('human_review','human_required')
            AND json_type(NEW.detail, '$.window_close') IS NOT 'null'
        )
        OR (
            json_extract(NEW.detail, '$.approval_path_label') = 'familiar_review'
            AND json_type(NEW.detail, '$.window_close') IS NOT 'object'
        )
        OR (
            EXISTS (
                SELECT 1 FROM ward_audit
                WHERE proposal_id = NEW.proposal_id
                  AND event_type = 'proposal_window_opened'
            )
            AND json_type(NEW.detail, '$.window_close') IS NOT 'object'
        )
        OR (
            json_type(NEW.detail, '$.window_close') IS 'object'
            AND (
                json_extract(NEW.detail, '$.window_close.reason') != 'applied'
                OR json_type(NEW.detail, '$.window_close.replay_hash_matched')
                    IS NOT 'true'
                OR COALESCE(
                    json_type(NEW.detail, '$.window_close.rationale'),
                    'missing'
                ) NOT IN ('null','text')
            )
        )
    )
BEGIN
    SELECT RAISE(ABORT, 'proposal approval requires valid path-specific detail');
END;

CREATE TRIGGER IF NOT EXISTS ward_audit_require_window_close_detail_insert
BEFORE INSERT ON ward_audit
WHEN NEW.event_type IN ('proposal_rejected', 'proposal_vetoed')
    AND EXISTS (
        SELECT 1 FROM ward_audit
        WHERE proposal_id = NEW.proposal_id
          AND event_type = 'proposal_window_opened'
    )
    AND (
        NEW.detail IS NULL OR NOT json_valid(NEW.detail)
        OR json_type(NEW.detail, '$.reason') IS NOT 'text'
        OR json_extract(NEW.detail, '$.reason') NOT IN (
            'applied','vetoed','evidence_diverged','revalidation_failed','superseded'
        )
        OR COALESCE(json_type(NEW.detail, '$.replay_hash_matched'), 'missing')
            NOT IN ('null','true','false')
        OR COALESCE(json_type(NEW.detail, '$.rationale'), 'missing')
            NOT IN ('null','text')
        OR (
            NEW.event_type = 'proposal_vetoed'
            AND json_extract(NEW.detail, '$.reason') != 'vetoed'
        )
        OR (
            NEW.event_type = 'proposal_rejected'
            AND json_extract(NEW.detail, '$.reason')
                NOT IN ('evidence_diverged','revalidation_failed','superseded')
        )
        OR (
            json_extract(NEW.detail, '$.reason') = 'applied'
            AND json_type(NEW.detail, '$.replay_hash_matched') IS NOT 'true'
        )
        OR (
            json_extract(NEW.detail, '$.reason')
                IN ('evidence_diverged','revalidation_failed')
            AND json_type(NEW.detail, '$.replay_hash_matched') IS NOT 'false'
        )
        OR (
            json_extract(NEW.detail, '$.reason') IN ('vetoed','superseded')
            AND json_type(NEW.detail, '$.replay_hash_matched') IS NOT 'null'
        )
    )
BEGIN
    SELECT RAISE(ABORT, 'window terminal events require a valid close reason');
END;
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{SurfaceId, WriterId};

    fn request() -> MutationRequest {
        MutationRequest {
            surface: SurfaceId::new("SOUL.md"),
            writer: WriterId::new("principal:val"),
            channel: Channel::Mutation,
            identity_context: None,
        }
    }

    #[test]
    fn verdict_records_carry_rfc_5_6_fields() {
        let now = OffsetDateTime::now_utc();
        let record = WardAuditRecord::for_verdict(
            FamiliarId::new(),
            &[0xab; 32],
            &request(),
            &Verdict::Reject {
                reason: RejectReason::UnknownSurface {
                    surface: SurfaceId::new("SOUL.md"),
                },
            },
            now,
            now,
        );
        assert_eq!(record.event_type, AuditEventType::ValidationVerdict);
        assert_eq!(record.decision, "reject:unknown_surface");
        assert_eq!(record.ward_hash, vec![0xab; 32]);
        assert_eq!(record.files_touched, vec![SurfaceId::new("SOUL.md")]);
        assert_eq!(record.channel, Some(Channel::Mutation));
    }

    #[test]
    fn audit_records_roundtrip_json() {
        let now = OffsetDateTime::now_utc();
        let record = WardAuditRecord::for_verdict(
            FamiliarId::new(),
            &[1; 32],
            &request(),
            &Verdict::Permit {
                thread: ThreadId::new(),
            },
            now,
            now,
        );
        let json = serde_json::to_string(&record).unwrap();
        let back: WardAuditRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(record, back);
    }

    #[test]
    fn schema_names_all_event_tags() {
        // The CHECK constraint and the enum must not drift.
        for et in [
            AuditEventType::ProposalSubmitted,
            AuditEventType::ProposalWindowOpened,
            AuditEventType::ProposalApproved,
            AuditEventType::ProposalRejected,
            AuditEventType::ProposalVetoed,
            AuditEventType::WardUpdated,
            AuditEventType::MemoryEntryAdmitted,
            AuditEventType::PrincipalAuthorizedWrite,
            AuditEventType::ValidationVerdict,
            AuditEventType::CompactionLedger,
            AuditEventType::ApplyAudit,
        ] {
            assert!(
                WARD_AUDIT_SCHEMA_SQL.contains(&format!("'{}'", et.tag())),
                "schema CHECK is missing event tag {}",
                et.tag()
            );
            // serde encoding matches the tag.
            let json = serde_json::to_string(&et).unwrap();
            assert_eq!(json, format!("\"{}\"", et.tag()));
        }
    }

    #[test]
    fn schema_names_all_window_close_reason_tags() {
        // The trigger SQL literals and the enum must not drift.
        let migration = ward_audit_migration_sql(true);
        for sql in [WARD_AUDIT_SCHEMA_SQL, migration.as_str()] {
            for reason in [
                WindowCloseReason::Applied,
                WindowCloseReason::Vetoed,
                WindowCloseReason::EvidenceDiverged,
                WindowCloseReason::RevalidationFailed,
                WindowCloseReason::Superseded,
            ] {
                assert!(
                    sql.contains(&format!("'{}'", reason.tag())),
                    "trigger SQL is missing window close reason tag {}",
                    reason.tag()
                );
                // serde encoding matches the tag.
                let json = serde_json::to_string(&reason).unwrap();
                assert_eq!(json, format!("\"{}\"", reason.tag()));
            }
        }
    }

    #[test]
    fn schema_names_all_approval_path_labels() {
        // The trigger SQL literals and the display-label contract must not drift.
        let migration = ward_audit_migration_sql(true);
        for sql in [WARD_AUDIT_SCHEMA_SQL, migration.as_str()] {
            for kind in [
                ApprovalPathKind::AutoRegression,
                ApprovalPathKind::FamiliarCoherence,
                ApprovalPathKind::HumanApproval,
                ApprovalPathKind::HumanApprovalWithRationale,
            ] {
                assert!(
                    sql.contains(&format!("'{}'", kind.display_label())),
                    "trigger SQL is missing approval path label {}",
                    kind.display_label()
                );
                // Label round-trips through the wire contract.
                assert_eq!(
                    ApprovalPath::from_display_label(kind.display_label()),
                    Some(kind)
                );
            }
        }
    }

    #[test]
    fn schema_enforces_append_only() {
        assert!(WARD_AUDIT_SCHEMA_SQL.contains("ward_audit_append_only_update"));
        assert!(WARD_AUDIT_SCHEMA_SQL.contains("ward_audit_append_only_delete"));
        assert!(WARD_AUDIT_SCHEMA_SQL.contains("append-only (RFC-0001 §5.6)"));
        assert!(WARD_AUDIT_SCHEMA_SQL.contains("ward_audit_require_single_terminal_insert"));
    }

    #[test]
    fn for_apply_produces_correct_shape() {
        let now = OffsetDateTime::now_utc();
        let familiar_id = FamiliarId::new();
        let surface = SurfaceId::new("SOUL.md");
        let prev = [0xaa_u8; 32];
        let next = [0xbb_u8; 32];

        let record = WardAuditRecord::for_apply(
            familiar_id,
            &[0xcc; 32],
            surface.clone(),
            "tier_2",
            Some(&prev),
            Some(&next),
            42,
            Some(Channel::Mutation),
            now,
            now,
        );
        assert_eq!(record.event_type, AuditEventType::ApplyAudit);
        assert_eq!(record.decision, "applied");
        assert_eq!(record.tier.as_deref(), Some("tier_2"));
        assert_eq!(record.diff_hash, Some(next.to_vec()));
        assert_eq!(record.files_touched, vec![surface]);

        // detail decoding helpers
        let prev_hex = record.apply_prev_sha256_hex().unwrap();
        assert_eq!(prev_hex.len(), 64);
        assert!(prev_hex.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(record.apply_bytes_written(), Some(42));
    }

    #[test]
    fn for_apply_roundtrips_json() {
        let now = OffsetDateTime::now_utc();
        let record = WardAuditRecord::for_apply(
            FamiliarId::new(),
            &[1; 32],
            SurfaceId::new("SOUL.md"),
            "tier_2",
            None,
            None,
            0,
            None,
            now,
            now,
        );
        let json = serde_json::to_string(&record).unwrap();
        let back: WardAuditRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(record, back);
        let detail: serde_json::Value =
            serde_json::from_str(record.detail.as_deref().unwrap()).unwrap();
        assert!(detail[APPLY_AUDIT_DETAIL_KEY_PREV].is_null());
        assert_eq!(record.apply_prev_sha256_hex(), None);
    }

    #[test]
    fn apply_audit_constructor_detail_passes_validation() {
        let now = OffsetDateTime::UNIX_EPOCH;
        let with_prev = WardAuditRecord::for_apply(
            FamiliarId::new(),
            &[1; 32],
            SurfaceId::new("SOUL.md"),
            "tier_2",
            Some(&[0xaa; 32]),
            Some(&[0xbb; 32]),
            42,
            Some(Channel::Mutation),
            now,
            now,
        );
        assert!(with_prev.validate_event_detail().is_ok());

        let without_prev = WardAuditRecord::for_apply(
            FamiliarId::new(),
            &[1; 32],
            SurfaceId::new("SOUL.md"),
            "tier_2",
            None,
            None,
            0,
            None,
            now,
            now,
        );
        assert!(without_prev.validate_event_detail().is_ok());
    }

    #[test]
    fn apply_audit_rejects_missing_or_malformed_detail() {
        let now = OffsetDateTime::UNIX_EPOCH;
        let mut record = WardAuditRecord::for_apply(
            FamiliarId::new(),
            &[1; 32],
            SurfaceId::new("SOUL.md"),
            "tier_2",
            Some(&[0xaa; 32]),
            Some(&[0xbb; 32]),
            42,
            None,
            now,
            now,
        );
        assert!(record.validate_event_detail().is_ok());

        // Missing detail entirely.
        record.detail = None;
        assert!(record.validate_event_detail().is_err());

        // Not a JSON object.
        record.detail = Some("[]".into());
        assert!(record.validate_event_detail().is_err());

        // Missing bytes_written.
        record.detail = Some(r#"{"prev_sha256":null}"#.into());
        assert!(record.validate_event_detail().is_err());

        // Missing prev_sha256.
        record.detail = Some(r#"{"bytes_written":1}"#.into());
        assert!(record.validate_event_detail().is_err());

        // Extra key.
        record.detail = Some(r#"{"prev_sha256":null,"bytes_written":1,"extra":true}"#.into());
        assert!(record.validate_event_detail().is_err());

        // prev_sha256 too short.
        record.detail = Some(r#"{"prev_sha256":"abcd","bytes_written":1}"#.into());
        assert!(record.validate_event_detail().is_err());

        // prev_sha256 upper-case hex.
        record.detail = Some(format!(
            r#"{{"prev_sha256":"{}","bytes_written":1}}"#,
            "AB".repeat(32)
        ));
        assert!(record.validate_event_detail().is_err());

        // prev_sha256 mistyped.
        record.detail = Some(r#"{"prev_sha256":42,"bytes_written":1}"#.into());
        assert!(record.validate_event_detail().is_err());

        // bytes_written negative.
        record.detail = Some(r#"{"prev_sha256":null,"bytes_written":-1}"#.into());
        assert!(record.validate_event_detail().is_err());

        // bytes_written mistyped.
        record.detail = Some(r#"{"prev_sha256":null,"bytes_written":"1"}"#.into());
        assert!(record.validate_event_detail().is_err());

        // Valid lower-case hex prev restores acceptance.
        record.detail = Some(format!(
            r#"{{"prev_sha256":"{}","bytes_written":1}}"#,
            "ab".repeat(32)
        ));
        assert!(record.validate_event_detail().is_ok());
    }

    #[test]
    fn apply_detail_helpers_return_none_for_non_apply_events() {
        let now = OffsetDateTime::UNIX_EPOCH;
        let mut record = WardAuditRecord::for_apply(
            FamiliarId::new(),
            &[1; 32],
            SurfaceId::new("SOUL.md"),
            "tier_2",
            Some(&[0xaa; 32]),
            Some(&[0xbb; 32]),
            42,
            None,
            now,
            now,
        );
        assert!(record.apply_prev_sha256_hex().is_some());
        assert_eq!(record.apply_bytes_written(), Some(42));

        // A non-ApplyAudit record carrying the same detail keys decodes nothing.
        record.event_type = AuditEventType::ValidationVerdict;
        assert_eq!(record.apply_prev_sha256_hex(), None);
        assert_eq!(record.apply_bytes_written(), None);
    }

    #[test]
    fn migration_sql_contains_apply_audit_and_detail_column() {
        let migration = ward_audit_migration_sql(true);
        assert!(
            migration.contains("'apply_audit'"),
            "migration CHECK must contain apply_audit tag"
        );
        assert!(
            migration.contains("detail"),
            "migration must add detail column"
        );
        assert!(
            migration.contains("BEGIN;"),
            "migration must be a transaction"
        );
        assert!(
            migration.contains("COMMIT;"),
            "migration must be a transaction"
        );
        assert!(
            !migration.contains("CREATE TABLE IF NOT EXISTS ward_audit_new"),
            "migration must fail fast if a stale ward_audit_new table exists"
        );
        assert_eq!(WARD_AUDIT_SCHEMA_VERSION, 20);
        assert!(!migration.contains("PRAGMA user_version"));
        assert!(!WARD_AUDIT_STAMP_V020_SQL.contains("PRAGMA user_version"));
        assert!(WARD_AUDIT_STAMP_V020_SQL.contains("ward_schema_meta"));
    }

    #[test]
    fn migration_preserves_detail_when_the_source_has_it() {
        let preserving = ward_audit_migration_sql(true);
        assert!(preserving.contains("diff_hash, detail, files_touched"));
        assert!(!preserving.contains("__SOURCE_DETAIL__"));

        let pre_detail = ward_audit_migration_sql(false);
        assert!(pre_detail.contains("diff_hash, NULL, files_touched"));
        assert!(!pre_detail.contains("__SOURCE_DETAIL__"));
    }

    #[test]
    fn provenance_detail_shapes_match_rfc_fields() {
        let admitted = MemoryEntryAdmissionAuditDetail {
            entry_hash: vec![0xab; 32],
            source_attestation: "ward:abc123".into(),
        };
        let admitted_json = serde_json::to_value(admitted).unwrap();
        assert!(admitted_json.get("entry_hash").is_some());
        assert!(admitted_json.get("source_attestation").is_some());

        let write = PrincipalAuthorizedWriteAuditDetail {
            principal_authorization: "principal:val-ed25519".into(),
        };
        let write_json = serde_json::to_value(write).unwrap();
        assert!(write_json.get("principal_authorization").is_some());
    }

    #[test]
    fn provenance_events_reject_missing_required_detail() {
        let now = OffsetDateTime::UNIX_EPOCH;
        let record = WardAuditRecord {
            event_type: AuditEventType::MemoryEntryAdmitted,
            proposal_id: None,
            familiar_id: FamiliarId::new(),
            ward_version: None,
            ward_hash: vec![0xaa; 32],
            tier: None,
            decision: "admitted".into(),
            approver: None,
            diff_hash: None,
            detail: None,
            files_touched: vec![],
            channel: None,
            thread_id: None,
            submitted_at: now,
            decided_at: now,
        };

        assert!(record.validate_event_detail().is_err());

        let mut ward_update = record;
        ward_update.event_type = AuditEventType::WardUpdated;
        assert!(ward_update.validate_event_detail().is_err());
    }

    #[test]
    fn proposal_window_open_rejects_missing_required_detail() {
        let now = OffsetDateTime::UNIX_EPOCH;
        let record = WardAuditRecord {
            event_type: AuditEventType::ProposalWindowOpened,
            proposal_id: Some(ProposalId::new()),
            familiar_id: FamiliarId::new(),
            ward_version: None,
            ward_hash: vec![0xaa; 32],
            tier: None,
            decision: "pending".into(),
            approver: None,
            diff_hash: None,
            detail: None,
            files_touched: vec![],
            channel: Some(Channel::Mutation),
            thread_id: None,
            submitted_at: now,
            decided_at: now,
        };

        assert!(record.validate_event_detail().is_err());

        let mut malformed = record;
        malformed.detail = Some(
            serde_json::to_string(&ProposalWindowAuditDetail {
                approval_path_label: "familiar_review".into(),
                deadline: now,
                earliest_close: now + time::Duration::seconds(1),
                evidence_replay_hash_hex: "g".repeat(64),
                affected_regions: vec![],
            })
            .unwrap(),
        );
        assert!(malformed.validate_event_detail().is_err());
    }

    #[test]
    fn proposal_window_close_detail_matches_terminal_event() {
        let now = OffsetDateTime::UNIX_EPOCH;
        let mut record = WardAuditRecord {
            event_type: AuditEventType::ProposalApproved,
            proposal_id: Some(ProposalId::new()),
            familiar_id: FamiliarId::new(),
            ward_version: None,
            ward_hash: vec![0xaa; 32],
            tier: None,
            decision: "applied".into(),
            approver: None,
            diff_hash: None,
            detail: Some(
                serde_json::to_string(&ProposalApprovalAuditDetail {
                    approval_path_label: "familiar_review".into(),
                    rationale: None,
                    window_close: Some(ProposalWindowCloseAuditDetail {
                        reason: WindowCloseReason::Applied,
                        replay_hash_matched: Some(true),
                        rationale: None,
                    }),
                })
                .unwrap(),
            ),
            files_touched: vec![],
            channel: Some(Channel::Mutation),
            thread_id: None,
            submitted_at: now,
            decided_at: now,
        };
        assert!(record.validate_event_detail().is_ok());

        record.detail = Some(
            serde_json::to_string(&ProposalApprovalAuditDetail {
                approval_path_label: "familiar_review".into(),
                rationale: None,
                window_close: Some(ProposalWindowCloseAuditDetail {
                    reason: WindowCloseReason::Vetoed,
                    replay_hash_matched: None,
                    rationale: None,
                }),
            })
            .unwrap(),
        );
        assert!(record.validate_event_detail().is_err());
    }

    #[test]
    fn human_required_approval_requires_nonempty_rationale() {
        let now = OffsetDateTime::UNIX_EPOCH;
        let mut record = WardAuditRecord {
            event_type: AuditEventType::ProposalApproved,
            proposal_id: Some(ProposalId::new()),
            familiar_id: FamiliarId::new(),
            ward_version: None,
            ward_hash: vec![0xaa; 32],
            tier: None,
            decision: "applied".into(),
            approver: Some(WriterId::new("principal:val")),
            diff_hash: None,
            detail: Some(
                serde_json::to_string(&ProposalApprovalAuditDetail {
                    approval_path_label: "human_required".into(),
                    rationale: None,
                    window_close: None,
                })
                .unwrap(),
            ),
            files_touched: vec![],
            channel: Some(Channel::Mutation),
            thread_id: None,
            submitted_at: now,
            decided_at: now,
        };
        assert!(record.validate_event_detail().is_err());

        record.detail = Some(
            serde_json::to_string(&ProposalApprovalAuditDetail {
                approval_path_label: "human_required".into(),
                rationale: Some("principal reviewed identity impact".into()),
                window_close: None,
            })
            .unwrap(),
        );
        assert!(record.validate_event_detail().is_ok());

        record.approver = Some(WriterId::new("   "));
        assert!(record.validate_event_detail().is_err());

        record.approver = None;
        assert!(record.validate_event_detail().is_err());

        record.approver = Some(WriterId::new("principal:val"));
        record.proposal_id = None;
        assert!(record.validate_event_detail().is_err());
    }

    #[test]
    fn provenance_constructors_produce_valid_required_detail() {
        let now = OffsetDateTime::UNIX_EPOCH;
        let admitted = WardAuditRecord::for_memory_entry_admitted(
            FamiliarId::new(),
            &[0xaa; 32],
            &[0xbb; 32],
            "ward:event-1",
            now,
        );
        assert!(admitted.validate_event_detail().is_ok());

        let write = WardAuditRecord::for_principal_authorized_write(
            FamiliarId::new(),
            &[0xaa; 32],
            WriterId::new("principal:val"),
            "signature:abc",
            vec![SurfaceId::new("MEMORY.md")],
            now,
        );
        assert!(write.validate_event_detail().is_ok());
    }

    #[test]
    fn schema_action_initializes_missing_table() {
        assert_eq!(
            ward_audit_schema_action(None, None),
            WardAuditSchemaAction::InitializeFresh
        );
    }

    #[test]
    fn schema_action_migrates_legacy_table_even_with_forged_version() {
        let legacy = "CREATE TABLE ward_audit (event_type TEXT CHECK \
            (event_type IN ('proposal_submitted','ward_updated')))";
        assert_eq!(
            ward_audit_schema_action(Some(legacy), Some(WARD_AUDIT_SCHEMA_VERSION)),
            WardAuditSchemaAction::MigrateLegacyWithoutDetail
        );
    }

    #[test]
    fn schema_action_migrates_table_missing_persistence_constraints() {
        let unconstrained = "CREATE TABLE ward_audit (
            detail TEXT,
            event_type TEXT CHECK (event_type IN (
                'proposal_window_opened','memory_entry_admitted',
                'principal_authorized_write','apply_audit'
            ))
        )";
        assert_eq!(
            ward_audit_schema_action(Some(unconstrained), Some(WARD_AUDIT_SCHEMA_VERSION)),
            WardAuditSchemaAction::MigrateLegacyWithDetail
        );
    }

    #[test]
    fn schema_action_stamps_current_unversioned_table_without_rebuild() {
        assert_eq!(
            ward_audit_schema_action(Some(WARD_AUDIT_SCHEMA_SQL), None),
            WardAuditSchemaAction::StampCurrent
        );
    }

    #[test]
    fn schema_action_leaves_current_versioned_table_unchanged() {
        assert_eq!(
            ward_audit_schema_action(Some(WARD_AUDIT_SCHEMA_SQL), Some(WARD_AUDIT_SCHEMA_VERSION)),
            WardAuditSchemaAction::None
        );
    }

    #[test]
    fn schema_action_refuses_to_downgrade_a_newer_component() {
        assert_eq!(
            ward_audit_schema_action(
                Some(WARD_AUDIT_SCHEMA_SQL),
                Some(WARD_AUDIT_SCHEMA_VERSION + 1)
            ),
            WardAuditSchemaAction::UnsupportedNewerVersion
        );
        assert_eq!(
            ward_audit_schema_action(None, Some(WARD_AUDIT_SCHEMA_VERSION + 1)),
            WardAuditSchemaAction::UnsupportedNewerVersion
        );
    }
}
