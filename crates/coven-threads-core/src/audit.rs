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
//! Because [`AuditEventType`] and [`WardAuditRecord`] are public exhaustive
//! surfaces, that new variant plus [`WardAuditRecord::detail`] define the
//! v0.2.0 contract rather than a v0.1.x-compatible patch.
//!
//! ## Schema shape and migration gating
//!
//! `WARD_AUDIT_SCHEMA_SQL` initializes or verifies the exact v0.2.0
//! `main.ward_audit` shape in one `BEGIN IMMEDIATE` transaction, without
//! mutating database-wide `PRAGMA user_version`. It is safe for the daemon to
//! execute unconditionally on every store open: a pre-install guard permits
//! only `missing` or exact `current_v020`, the durable DDL runs explicitly in
//! `main` with `IF NOT EXISTS` compatibility, and a post-install guard requires
//! exact `current_v020` before `COMMIT`. The IMMEDIATE reservation means
//! concurrent initializers serialize before any guard/classification read, so a
//! second caller waits, re-runs the guard against the winner's committed
//! schema, and then idempotently sees `current_v020`. If either guard fails,
//! callers must explicitly `ROLLBACK` before continuing.
//! `WARD_AUDIT_SCHEMA_STATE_SQL` is the reusable, table-local fingerprint query
//! that returns one of four stable tags:
//! - `missing` — `main.ward_audit` does not exist, no unexpected durable
//!   main-schema object named `ward_audit` or `ward_audit_*` exists, and no
//!   temp-schema shadow/reserved object exists; initialize with
//!   `WARD_AUDIT_SCHEMA_SQL`;
//! - `legacy_v013` — `main.ward_audit` exactly matches the v0.1.3 legacy
//!   fingerprint, the durable reserved namespace contains only the expected
//!   table/index/trigger objects attached to `main.ward_audit`, and no temp
//!   shadow exists; run `WARD_AUDIT_MIGRATION_V020_SQL`;
//! - `current_v020` — `main.ward_audit` exactly matches the v0.2.0 current
//!   fingerprint, the durable reserved namespace contains only the expected
//!   table/index/trigger objects attached to `main.ward_audit`, and no temp
//!   shadow exists; continue without schema work;
//! - `unknown` — every other shape, including any extra or missing declared
//!   table constraint, column, index, or trigger, any unexpected durable
//!   main-schema object named `ward_audit` or `ward_audit_*`, and any
//!   temp-schema table/view/index/trigger named `ward_audit` or
//!   `ward_audit_*`; fail closed.
//!
//! The fingerprint uses exact `main.sqlite_master.sql` text for the durable
//! table, explicit durable indexes, and append-only durable triggers, plus
//! ordered `pragma_table_info('ward_audit', 'main')` metadata and
//! `pragma_index_list('ward_audit', 'main')` for explicit index discovery. It
//! does **not** normalize whitespace: the only accepted `current_v020`
//! table-SQL variants are the fresh `CREATE TABLE ward_audit` form and the
//! quoted `CREATE TABLE "ward_audit"` form SQLite stores after the exact legacy
//! migration path, and the `legacy_v013` fingerprint includes the inline
//! comments preserved from the shipped v0.1.3 DDL.
//! `WARD_AUDIT_MIGRATION_V020_SQL` exists only for the exact `legacy_v013`
//! fingerprint with no unexpected durable reserved-namespace object and no temp
//! shadow. It independently re-checks that fingerprint inside one
//! `BEGIN IMMEDIATE` transaction before any `ALTER`, then adds `detail` and
//! rebuilds `main.ward_audit` so the current CHECK set is installed and every
//! existing row is preserved. Before `COMMIT`, a distinct TEMP postcondition
//! guard reruns the shared schema-state CTE/predicates and requires exact
//! `current_v020`, so callers cannot durably commit a rebuilt-but-self-unknown
//! namespace if extra `ward_audit` / `ward_audit_*` objects appear mid-transaction.
//! The IMMEDIATE reservation means concurrent migrators serialize before any
//! legacy-guard read: the winner upgrades first, and a second caller waits,
//! reclassifies the durable table as current when the guard re-runs, then fails
//! closed at the legacy guard without a `sqlite_master` lock race. If the
//! postcondition guard or any later migration step fails after
//! `ALTER TABLE main.ward_audit ADD COLUMN detail`, callers must explicitly roll
//! back the failed transaction before continuing so SQLite restores the
//! untouched legacy table.
//!
//! ## Where content hashes ride for applied writes
//!
//! `WardAuditRecord::diff_hash` carries `next_sha256` (the post-write content
//! hash, matching the RFC-0001 §5.6 `diff_hash` semantic for verdict rows).
//! The complementary `prev_sha256` and `bytes_written` ride in `detail` as a
//! compact JSON object `{"prev_sha256":"<hex>","bytes_written":N}`. This
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

/// Stable JSON key for the `detail` field of an `apply_audit` row.
pub const APPLY_AUDIT_DETAIL_KEY_PREV: &str = "prev_sha256";
/// Stable JSON key for the bytes-written count in an `apply_audit` detail.
pub const APPLY_AUDIT_DETAIL_KEY_BYTES: &str = "bytes_written";

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

/// Stable tag returned by [`WARD_AUDIT_SCHEMA_STATE_SQL`] when `main.ward_audit`
/// is absent, no unexpected durable main-schema `ward_audit` /
/// `ward_audit_*` object exists, and no temp shadow/reserved object blocks the
/// durable contract.
pub const WARD_AUDIT_SCHEMA_STATE_MISSING: &str = "missing";
/// Stable tag returned by [`WARD_AUDIT_SCHEMA_STATE_SQL`] for the exact
/// `main.ward_audit` v0.1.3 legacy schema fingerprint, with no unexpected
/// durable reserved-namespace object and no temp shadow.
pub const WARD_AUDIT_SCHEMA_STATE_LEGACY_V013: &str = "legacy_v013";
/// Stable tag returned by [`WARD_AUDIT_SCHEMA_STATE_SQL`] for the exact
/// `main.ward_audit` v0.2.0 current schema fingerprint, with no unexpected
/// durable reserved-namespace object and no temp shadow.
pub const WARD_AUDIT_SCHEMA_STATE_CURRENT_V020: &str = "current_v020";
/// Stable tag returned by [`WARD_AUDIT_SCHEMA_STATE_SQL`] for every other
/// `main.ward_audit` shape, plus any unexpected durable reserved-namespace
/// object or any temp-schema `ward_audit` / `ward_audit_*` shadow object.
pub const WARD_AUDIT_SCHEMA_STATE_UNKNOWN: &str = "unknown";

macro_rules! ward_audit_reserved_name_predicate_sql {
    () => {
        r#"(lower(name) = 'ward_audit' OR lower(name) GLOB 'ward_audit_*')"#
    };
}

macro_rules! ward_audit_expected_durable_object_predicate_sql {
    () => {
        r#"(
            (type = 'table' AND name = 'ward_audit')
            OR (type = 'index' AND name = 'ward_audit_event_idx' AND tbl_name = 'ward_audit')
            OR (type = 'index' AND name = 'ward_audit_familiar_idx' AND tbl_name = 'ward_audit')
            OR (type = 'trigger' AND name = 'ward_audit_append_only_update' AND tbl_name = 'ward_audit')
            OR (type = 'trigger' AND name = 'ward_audit_append_only_delete' AND tbl_name = 'ward_audit')
            OR (type = 'trigger' AND name = 'ward_audit_require_single_terminal_insert' AND tbl_name = 'ward_audit')
            OR (type = 'trigger' AND name = 'ward_audit_require_authorization_insert' AND tbl_name = 'ward_audit')
            OR (type = 'trigger' AND name = 'ward_audit_require_proposal_approval_detail_insert' AND tbl_name = 'ward_audit')
            OR (type = 'trigger' AND name = 'ward_audit_require_window_close_detail_insert' AND tbl_name = 'ward_audit')
        )"#
    };
}

macro_rules! ward_audit_schema_state_ctes_sql {
    () => {
        concat!(
            r#"
WITH
    ward_audit_table AS (
        SELECT sql
        FROM main.sqlite_master
        WHERE type = 'table' AND name = 'ward_audit'
    ),
    ward_audit_exists AS (
        SELECT EXISTS(SELECT 1 FROM ward_audit_table) AS ok
    ),
    ward_audit_column_fingerprint AS (
        SELECT COALESCE(
            group_concat(
                printf(
                    '%d|%s|%s|%d|%s|%d',
                    cid,
                    name,
                    type,
                    "notnull",
                    COALESCE(dflt_value, '<null>'),
                    pk
                ),
                '||'
            ),
            ''
        ) AS fp
        FROM (
            SELECT cid, name, type, "notnull", dflt_value, pk
            FROM pragma_table_info('ward_audit', 'main')
            ORDER BY cid
        )
    ),
    ward_audit_index_fingerprint AS (
        SELECT COALESCE(group_concat(item, '||'), '') AS fp
        FROM (
            SELECT printf('%s|%s', il.name, sm.sql) AS item
            FROM pragma_index_list('ward_audit', 'main') AS il
            JOIN main.sqlite_master AS sm
              ON sm.type = 'index' AND sm.name = il.name
            WHERE il.origin = 'c' AND sm.sql IS NOT NULL
            ORDER BY il.name
        )
    ),
    ward_audit_trigger_fingerprint AS (
        SELECT COALESCE(group_concat(item, '||'), '') AS fp
        FROM (
            SELECT printf('%s|%s', name, COALESCE(sql, '<null>')) AS item
            FROM main.sqlite_master
            WHERE type = 'trigger' AND tbl_name = 'ward_audit'
            ORDER BY name
        )
    ),
    ward_audit_unexpected_durable_namespace_object_count AS (
        SELECT COUNT(*) AS count
        FROM main.sqlite_master
        WHERE type IN ('table', 'index', 'trigger', 'view')
          AND "#,
            ward_audit_reserved_name_predicate_sql!(),
            r#"
          AND NOT "#,
            ward_audit_expected_durable_object_predicate_sql!(),
            r#"
    ),
    ward_audit_temp_shadow_object_count AS (
        SELECT COUNT(*) AS count
        FROM temp.sqlite_master
        WHERE type IN ('table', 'index', 'trigger', 'view')
          AND "#,
            ward_audit_reserved_name_predicate_sql!(),
            r#"
    ),
    ward_audit_shape AS (
        SELECT
            (SELECT ok FROM ward_audit_exists) AS table_exists,
            COALESCE((SELECT sql FROM ward_audit_table), '') AS table_sql,
            (SELECT fp FROM ward_audit_column_fingerprint) AS column_fp,
            (SELECT fp FROM ward_audit_index_fingerprint) AS index_fp,
            (SELECT fp FROM ward_audit_trigger_fingerprint) AS trigger_fp,
            (SELECT count FROM ward_audit_unexpected_durable_namespace_object_count)
                AS unexpected_durable_namespace_object_count,
            (SELECT count FROM ward_audit_temp_shadow_object_count) AS temp_shadow_count
    )
"#
        )
    };
}

macro_rules! ward_audit_exact_legacy_table_sql_sql {
    () => {
        r#"'CREATE TABLE ward_audit (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type    TEXT    NOT NULL CHECK (event_type IN (
                      ''proposal_submitted'',''proposal_approved'',''proposal_rejected'',
                      ''proposal_vetoed'',''ward_updated'',''validation_verdict'',
                      ''compaction_ledger'')),
    proposal_id   TEXT,
    familiar_id   TEXT    NOT NULL,
    ward_version  TEXT,
    ward_hash     BLOB    NOT NULL,
    tier          TEXT,
    decision      TEXT    NOT NULL,
    approver      TEXT,
    diff_hash     BLOB,
    files_touched TEXT    NOT NULL, -- JSON array of surface ids
    channel       TEXT,
    thread_id     TEXT,
    submitted_at  TEXT    NOT NULL, -- RFC 3339
    decided_at    TEXT    NOT NULL, -- RFC 3339
    recorded_at   TEXT    NOT NULL DEFAULT (strftime(''%Y-%m-%dT%H:%M:%fZ'',''now''))
)'"#
    };
}

macro_rules! ward_audit_exact_current_fresh_table_sql_sql {
    () => {
        r#"'CREATE TABLE ward_audit (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type    TEXT    NOT NULL CHECK (event_type IN (
                      ''proposal_submitted'',''proposal_window_opened'',
                      ''proposal_approved'',''proposal_rejected'',
                      ''proposal_vetoed'',''ward_updated'',''memory_entry_admitted'',
                      ''principal_authorized_write'',''validation_verdict'',
                      ''compaction_ledger'',''apply_audit'')),
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
    recorded_at   TEXT    NOT NULL DEFAULT (strftime(''%Y-%m-%dT%H:%M:%fZ'',''now''))
)'"#
    };
}

macro_rules! ward_audit_exact_current_migrated_table_sql_sql {
    () => {
        r#"'CREATE TABLE "ward_audit" (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type    TEXT    NOT NULL CHECK (event_type IN (
                      ''proposal_submitted'',''proposal_window_opened'',
                      ''proposal_approved'',''proposal_rejected'',
                      ''proposal_vetoed'',''ward_updated'',''memory_entry_admitted'',
                      ''principal_authorized_write'',''validation_verdict'',
                      ''compaction_ledger'',''apply_audit'')),
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
    recorded_at   TEXT    NOT NULL DEFAULT (strftime(''%Y-%m-%dT%H:%M:%fZ'',''now''))
)'"#
    };
}

macro_rules! ward_audit_exact_explicit_index_fp_sql {
    () => {
        r#"'ward_audit_event_idx|CREATE INDEX ward_audit_event_idx    ON ward_audit (event_type, recorded_at)||ward_audit_familiar_idx|CREATE INDEX ward_audit_familiar_idx ON ward_audit (familiar_id, recorded_at)'"#
    };
}

macro_rules! ward_audit_exact_legacy_trigger_fp_sql {
    () => {
        r#"'ward_audit_append_only_delete|CREATE TRIGGER ward_audit_append_only_delete
BEFORE DELETE ON ward_audit
BEGIN
    SELECT RAISE(ABORT, ''ward_audit is append-only (RFC-0001 §5.6)'');
END||ward_audit_append_only_update|CREATE TRIGGER ward_audit_append_only_update
BEFORE UPDATE ON ward_audit
BEGIN
    SELECT RAISE(ABORT, ''ward_audit is append-only (RFC-0001 §5.6)'');
END'"#
    };
}

macro_rules! ward_audit_exact_trigger_fp_sql {
    () => {
        r#"'ward_audit_append_only_delete|CREATE TRIGGER ward_audit_append_only_delete
BEFORE DELETE ON ward_audit
BEGIN
    SELECT RAISE(ABORT, ''ward_audit is append-only (RFC-0001 §5.6)'');
END||ward_audit_append_only_update|CREATE TRIGGER ward_audit_append_only_update
BEFORE UPDATE ON ward_audit
BEGIN
    SELECT RAISE(ABORT, ''ward_audit is append-only (RFC-0001 §5.6)'');
END||ward_audit_require_authorization_insert|CREATE TRIGGER ward_audit_require_authorization_insert
BEFORE INSERT ON ward_audit
WHEN NEW.event_type IN (''ward_updated'', ''principal_authorized_write'')
    AND (
        NEW.detail IS NULL OR NOT json_valid(NEW.detail)
        OR json_type(NEW.detail, ''$.principal_authorization'') IS NOT ''text''
        OR COALESCE(length(trim(json_extract(NEW.detail, ''$.principal_authorization''))), 0) = 0
    )
BEGIN
    SELECT RAISE(ABORT, ''authorized Ward writes require principal_authorization'');
END||ward_audit_require_proposal_approval_detail_insert|CREATE TRIGGER ward_audit_require_proposal_approval_detail_insert
BEFORE INSERT ON ward_audit
WHEN NEW.event_type = ''proposal_approved''
    AND (
        NEW.detail IS NULL OR NOT json_valid(NEW.detail)
        OR json_type(NEW.detail, ''$.approval_path_label'') IS NOT ''text''
        OR json_extract(NEW.detail, ''$.approval_path_label'')
            NOT IN (''auto'',''familiar_review'',''human_review'',''human_required'')
        OR COALESCE(json_type(NEW.detail, ''$.rationale''), ''missing'')
            NOT IN (''null'',''text'')
        OR COALESCE(json_type(NEW.detail, ''$.window_close''), ''missing'')
            NOT IN (''null'',''object'')
        OR (
            json_extract(NEW.detail, ''$.approval_path_label'')
                IN (''human_review'',''human_required'')
            AND (
                NEW.approver IS NULL
                OR length(trim(NEW.approver)) = 0
            )
        )
        OR (
            json_extract(NEW.detail, ''$.approval_path_label'') = ''human_required''
            AND (
                json_type(NEW.detail, ''$.rationale'') IS NOT ''text''
                OR length(trim(json_extract(NEW.detail, ''$.rationale''))) = 0
            )
        )
        OR (
            json_extract(NEW.detail, ''$.approval_path_label'')
                IN (''human_review'',''human_required'')
            AND json_type(NEW.detail, ''$.window_close'') IS NOT ''null''
        )
        OR (
            json_extract(NEW.detail, ''$.approval_path_label'') = ''familiar_review''
            AND json_type(NEW.detail, ''$.window_close'') IS NOT ''object''
        )
        OR (
            EXISTS (
                SELECT 1 FROM ward_audit
                WHERE proposal_id = NEW.proposal_id
                  AND event_type = ''proposal_window_opened''
            )
            AND json_type(NEW.detail, ''$.window_close'') IS NOT ''object''
        )
        OR (
            json_type(NEW.detail, ''$.window_close'') IS ''object''
            AND (
                json_extract(NEW.detail, ''$.window_close.reason'') != ''applied''
                OR json_type(NEW.detail, ''$.window_close.replay_hash_matched'')
                    IS NOT ''true''
                OR COALESCE(
                    json_type(NEW.detail, ''$.window_close.rationale''),
                    ''missing''
                ) NOT IN (''null'',''text'')
            )
        )
    )
BEGIN
    SELECT RAISE(ABORT, ''proposal approval requires valid path-specific detail'');
END||ward_audit_require_single_terminal_insert|CREATE TRIGGER ward_audit_require_single_terminal_insert
BEFORE INSERT ON ward_audit
WHEN NEW.event_type IN (''proposal_approved'',''proposal_rejected'',''proposal_vetoed'')
    AND (
        NEW.proposal_id IS NULL
        OR EXISTS (
            SELECT 1 FROM ward_audit
            WHERE proposal_id = NEW.proposal_id
              AND event_type IN (
                  ''proposal_approved'',''proposal_rejected'',''proposal_vetoed''
              )
        )
    )
BEGIN
    SELECT RAISE(ABORT, ''proposal requires exactly one terminal event'');
END||ward_audit_require_window_close_detail_insert|CREATE TRIGGER ward_audit_require_window_close_detail_insert
BEFORE INSERT ON ward_audit
WHEN NEW.event_type IN (''proposal_rejected'', ''proposal_vetoed'')
    AND EXISTS (
        SELECT 1 FROM ward_audit
        WHERE proposal_id = NEW.proposal_id
          AND event_type = ''proposal_window_opened''
    )
    AND (
        NEW.detail IS NULL OR NOT json_valid(NEW.detail)
        OR json_type(NEW.detail, ''$.reason'') IS NOT ''text''
        OR json_extract(NEW.detail, ''$.reason'') NOT IN (
            ''applied'',''vetoed'',''evidence_diverged'',''revalidation_failed'',''superseded''
        )
        OR COALESCE(json_type(NEW.detail, ''$.replay_hash_matched''), ''missing'')
            NOT IN (''null'',''true'',''false'')
        OR COALESCE(json_type(NEW.detail, ''$.rationale''), ''missing'')
            NOT IN (''null'',''text'')
        OR (
            NEW.event_type = ''proposal_vetoed''
            AND json_extract(NEW.detail, ''$.reason'') != ''vetoed''
        )
        OR (
            NEW.event_type = ''proposal_rejected''
            AND json_extract(NEW.detail, ''$.reason'')
                NOT IN (''evidence_diverged'',''revalidation_failed'',''superseded'')
        )
        OR (
            json_extract(NEW.detail, ''$.reason'') = ''applied''
            AND json_type(NEW.detail, ''$.replay_hash_matched'') IS NOT ''true''
        )
        OR (
            json_extract(NEW.detail, ''$.reason'')
                IN (''evidence_diverged'',''revalidation_failed'')
            AND json_type(NEW.detail, ''$.replay_hash_matched'') IS NOT ''false''
        )
        OR (
            json_extract(NEW.detail, ''$.reason'') IN (''vetoed'',''superseded'')
            AND json_type(NEW.detail, ''$.replay_hash_matched'') IS NOT ''null''
        )
    )
BEGIN
    SELECT RAISE(ABORT, ''window terminal events require a valid close reason'');
END'"#
    };
}

macro_rules! ward_audit_exact_legacy_predicate_sql {
    () => {
        concat!(
            r#"
table_exists = 1
AND unexpected_durable_namespace_object_count = 0
AND temp_shadow_count = 0
AND table_sql = "#,
            ward_audit_exact_legacy_table_sql_sql!(),
            r#"
AND column_fp = '0|id|INTEGER|0|<null>|1||1|event_type|TEXT|1|<null>|0||2|proposal_id|TEXT|0|<null>|0||3|familiar_id|TEXT|1|<null>|0||4|ward_version|TEXT|0|<null>|0||5|ward_hash|BLOB|1|<null>|0||6|tier|TEXT|0|<null>|0||7|decision|TEXT|1|<null>|0||8|approver|TEXT|0|<null>|0||9|diff_hash|BLOB|0|<null>|0||10|files_touched|TEXT|1|<null>|0||11|channel|TEXT|0|<null>|0||12|thread_id|TEXT|0|<null>|0||13|submitted_at|TEXT|1|<null>|0||14|decided_at|TEXT|1|<null>|0||15|recorded_at|TEXT|1|strftime(''%Y-%m-%dT%H:%M:%fZ'',''now'')|0'
AND index_fp = "#,
            ward_audit_exact_explicit_index_fp_sql!(),
            r#"
AND trigger_fp = "#,
            ward_audit_exact_legacy_trigger_fp_sql!(),
            r#"
"#
        )
    };
}

macro_rules! ward_audit_exact_current_predicate_sql {
    () => {
        concat!(
            r#"
table_exists = 1
AND unexpected_durable_namespace_object_count = 0
AND temp_shadow_count = 0
AND table_sql IN ("#,
            ward_audit_exact_current_fresh_table_sql_sql!(),
            r#", "#,
            ward_audit_exact_current_migrated_table_sql_sql!(),
            r#")
AND column_fp = '0|id|INTEGER|0|<null>|1||1|event_type|TEXT|1|<null>|0||2|proposal_id|TEXT|0|<null>|0||3|familiar_id|TEXT|1|<null>|0||4|ward_version|TEXT|0|<null>|0||5|ward_hash|BLOB|1|<null>|0||6|tier|TEXT|0|<null>|0||7|decision|TEXT|1|<null>|0||8|approver|TEXT|0|<null>|0||9|diff_hash|BLOB|0|<null>|0||10|detail|TEXT|0|<null>|0||11|files_touched|TEXT|1|<null>|0||12|channel|TEXT|0|<null>|0||13|thread_id|TEXT|0|<null>|0||14|submitted_at|TEXT|1|<null>|0||15|decided_at|TEXT|1|<null>|0||16|recorded_at|TEXT|1|strftime(''%Y-%m-%dT%H:%M:%fZ'',''now'')|0'
AND index_fp = "#,
            ward_audit_exact_explicit_index_fp_sql!(),
            r#"
AND trigger_fp = "#,
            ward_audit_exact_trigger_fp_sql!(),
            r#"
"#
        )
    };
}

macro_rules! ward_audit_schema_state_case_sql {
    () => {
        concat!(
            r#"CASE
    WHEN temp_shadow_count > 0 THEN 'unknown'
    WHEN table_exists = 0 AND unexpected_durable_namespace_object_count = 0 THEN 'missing'
    WHEN "#,
            ward_audit_exact_legacy_predicate_sql!(),
            r#" THEN 'legacy_v013'
    WHEN "#,
            ward_audit_exact_current_predicate_sql!(),
            r#" THEN 'current_v020'
    ELSE 'unknown'
END"#
        )
    };
}

/// Table-local schema-state query for the durable `main.ward_audit` contract
/// inside `coven.sqlite3` (§3.4).
///
/// Callers run this exact query and branch on the stable text result:
/// - [`WARD_AUDIT_SCHEMA_STATE_MISSING`] — `main.ward_audit` is absent, no
///   unexpected durable main-schema object named `ward_audit` or
///   `ward_audit_*` exists, and no temp shadow/reserved object exists;
///   initialize with [`WARD_AUDIT_SCHEMA_SQL`];
/// - [`WARD_AUDIT_SCHEMA_STATE_LEGACY_V013`] — run
///   [`WARD_AUDIT_MIGRATION_V020_SQL`];
/// - [`WARD_AUDIT_SCHEMA_STATE_CURRENT_V020`] — continue without schema work;
/// - [`WARD_AUDIT_SCHEMA_STATE_UNKNOWN`] — fail closed and investigate the
///   table manually.
///
/// The fingerprint is strict: the exact `main.sqlite_master.sql` stored for
/// `main.ward_audit`, ordered column metadata from
/// `pragma_table_info('ward_audit', 'main')`, the exact explicit main-index SQL
/// set (discovered with `pragma_index_list('ward_audit', 'main')` and then read
/// from `main.sqlite_master`), and the exact append-only main-trigger SQL set
/// must all match. Full stored-table-SQL equality covers every declared
/// table-level constraint (`CHECK`, `UNIQUE`, foreign-key clauses, and the
/// `event_type` list), so any extra or missing column, constraint, index, or
/// trigger returns `unknown`. Across **all** durable states, the reserved
/// main-schema namespace is whitelisted to exactly these objects attached to
/// `main.ward_audit`: the `ward_audit` table, `ward_audit_event_idx`,
/// `ward_audit_familiar_idx`, `ward_audit_append_only_update`, and
/// `ward_audit_append_only_delete`. Any other main-schema table/view/index/
/// trigger whose name is exactly `ward_audit` or begins with `ward_audit_`
/// returns `unknown`, including `ward_audit_new`, backup/shadow tables, or
/// reserved-name indexes/triggers attached elsewhere. Any temp-schema
/// table/view/index/trigger whose name is exactly `ward_audit` or begins with
/// `ward_audit_` also returns `unknown`, even when `main.ward_audit` itself is
/// exact current or legacy, so callers cannot treat a shadowed namespace as
/// healthy durable state. No whitespace-destroying normalization is applied:
/// the only accepted `current_v020` table SQL variants are the fresh
/// `CREATE TABLE ward_audit (...)` form and SQLite's quoted
/// `CREATE TABLE "ward_audit" (...)` form produced by the exact legacy
/// migration path, while the `legacy_v013` fingerprint intentionally includes
/// the inline comments preserved from the shipped v0.1.3 DDL.
pub const WARD_AUDIT_SCHEMA_STATE_SQL: &str = concat!(
    ward_audit_schema_state_ctes_sql!(),
    r#"
SELECT "#,
    ward_audit_schema_state_case_sql!(),
    r#" AS schema_state
FROM ward_audit_shape;
"#,
);

/// DDL migration for the exact `main.ward_audit` `legacy_v013` fingerprint
/// inside `coven.sqlite3` (§3.4).
///
/// Append-only is enforced *in the store*: UPDATE and DELETE abort via
/// triggers (RFC-0001 §5.6: entries MUST NOT be deleted or modified).
/// SQLite cannot `ALTER` a CHECK constraint on an existing table, so this
/// transaction:
/// 1. reserves the main database up front with `BEGIN IMMEDIATE`, then
///    re-checks the exact legacy fingerprint in SQL before any mutation,
///    including exact stored `main.sqlite_master.sql` equality plus main
///    column/index/trigger fingerprints, the durable reserved-namespace
///    whitelist, and temp-shadow rejection;
/// 2. adds the legacy `detail` column on `main.ward_audit` so the old table
///    matches the copy shape;
/// 3. creates `main.ward_audit_new` with the updated CHECK;
/// 4. copies every existing row, preserving `detail`;
/// 5. swaps the tables; and
/// 6. re-creates the exact explicit main indexes and all 6 durable main triggers
///    (2 append-only: `ward_audit_append_only_update`, `ward_audit_append_only_delete`;
///    4 authority: `ward_audit_require_authorization_insert`,
///    `ward_audit_require_proposal_approval_detail_insert`,
///    `ward_audit_require_window_close_detail_insert`,
///    `ward_audit_require_single_terminal_insert`); and
/// 7. re-runs the shared schema-state expression in a distinct TEMP
///    postcondition guard and requires exact `current_v020` before `COMMIT`.
///
/// Callers should still branch on [`WARD_AUDIT_SCHEMA_STATE_SQL`] first:
/// initialize when the state is `missing`, migrate only `legacy_v013`,
/// continue on `current_v020`, and fail closed on `unknown`. This migration
/// independently guards the same `legacy_v013` fingerprint, durable
/// reserved-namespace whitelist, temp-shadow rejection, and exact-current
/// postcondition so callers cannot mutate a partial, already-current,
/// shadowed, or rebuilt-but-drifted schema by skipping classification. Because
/// the transaction begins IMMEDIATELY, concurrent migrators serialize before
/// the guard read; after the winner commits, a second caller re-runs the guard
/// against the now-current schema and fails closed there rather than racing
/// into a `sqlite_master` lock. If the postcondition guard or a later step
/// fails after `ALTER TABLE main.ward_audit ADD COLUMN detail`, callers must
/// `ROLLBACK` the failed transaction before continuing so SQLite restores the
/// untouched legacy table. This SQL does not read or write database-wide
/// `PRAGMA user_version`.
macro_rules! ward_audit_authority_triggers_sql {
    () => {
        r#"CREATE TRIGGER IF NOT EXISTS main.ward_audit_require_single_terminal_insert
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

CREATE TRIGGER IF NOT EXISTS main.ward_audit_require_authorization_insert
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

CREATE TRIGGER IF NOT EXISTS main.ward_audit_require_proposal_approval_detail_insert
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

CREATE TRIGGER IF NOT EXISTS main.ward_audit_require_window_close_detail_insert
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
"#
    };
}

pub const WARD_AUDIT_MIGRATION_V020_SQL: &str = concat!(
    r#"
BEGIN IMMEDIATE;

CREATE TEMP TABLE coven_threads_ward_audit_migration_guard (
    ok INTEGER NOT NULL CHECK (ok = 1)
);

INSERT INTO coven_threads_ward_audit_migration_guard (ok)
"#,
    ward_audit_schema_state_ctes_sql!(),
    r#"
SELECT CASE
    WHEN "#,
    ward_audit_exact_legacy_predicate_sql!(),
    r#" THEN 1
    ELSE 0
END
FROM ward_audit_shape;

DROP TABLE coven_threads_ward_audit_migration_guard;

ALTER TABLE main.ward_audit ADD COLUMN detail TEXT;

CREATE TABLE main.ward_audit_new (
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
    recorded_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

INSERT INTO main.ward_audit_new (
    id, event_type, proposal_id, familiar_id, ward_version, ward_hash,
    tier, decision, approver, diff_hash, detail, files_touched, channel,
    thread_id, submitted_at, decided_at, recorded_at
)
SELECT
    id, event_type, proposal_id, familiar_id, ward_version, ward_hash,
    tier, decision, approver, diff_hash, detail, files_touched, channel,
    thread_id, submitted_at, decided_at, recorded_at
FROM main.ward_audit;

DROP TABLE main.ward_audit;
ALTER TABLE main.ward_audit_new RENAME TO ward_audit;

CREATE INDEX IF NOT EXISTS main.ward_audit_familiar_idx ON ward_audit (familiar_id, recorded_at);
CREATE INDEX IF NOT EXISTS main.ward_audit_event_idx    ON ward_audit (event_type, recorded_at);

CREATE TRIGGER IF NOT EXISTS main.ward_audit_append_only_update
BEFORE UPDATE ON ward_audit
BEGIN
    SELECT RAISE(ABORT, 'ward_audit is append-only (RFC-0001 §5.6)');
END;

CREATE TRIGGER IF NOT EXISTS main.ward_audit_append_only_delete
BEFORE DELETE ON ward_audit
BEGIN
    SELECT RAISE(ABORT, 'ward_audit is append-only (RFC-0001 §5.6)');
END;

"#,
    ward_audit_authority_triggers_sql!(),
    r#"
CREATE TEMP TABLE coven_threads_ward_audit_migration_post_guard (
    ok INTEGER NOT NULL CHECK (ok = 1)
);

INSERT INTO coven_threads_ward_audit_migration_post_guard (ok)
"#,
    ward_audit_schema_state_ctes_sql!(),
    r#"
SELECT CASE
    WHEN ("#,
    ward_audit_schema_state_case_sql!(),
    r#") = 'current_v020' THEN 1
    ELSE 0
END
FROM ward_audit_shape;

DROP TABLE coven_threads_ward_audit_migration_post_guard;

COMMIT;
"#,
);

// Shared durable-main current-v0.2.0 DDL body used by the guarded init SQL and
// drift tests.

macro_rules! ward_audit_current_objects_sql {
    () => {
        r#"
CREATE TABLE IF NOT EXISTS main.ward_audit (
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
    recorded_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX IF NOT EXISTS main.ward_audit_familiar_idx ON ward_audit (familiar_id, recorded_at);
CREATE INDEX IF NOT EXISTS main.ward_audit_event_idx    ON ward_audit (event_type, recorded_at);

CREATE TRIGGER IF NOT EXISTS main.ward_audit_append_only_update
BEFORE UPDATE ON ward_audit
BEGIN
    SELECT RAISE(ABORT, 'ward_audit is append-only (RFC-0001 §5.6)');
END;

CREATE TRIGGER IF NOT EXISTS main.ward_audit_append_only_delete
BEFORE DELETE ON ward_audit
BEGIN
    SELECT RAISE(ABORT, 'ward_audit is append-only (RFC-0001 §5.6)');
END;

CREATE TRIGGER IF NOT EXISTS main.ward_audit_require_single_terminal_insert
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

CREATE TRIGGER IF NOT EXISTS main.ward_audit_require_authorization_insert
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

CREATE TRIGGER IF NOT EXISTS main.ward_audit_require_proposal_approval_detail_insert
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

CREATE TRIGGER IF NOT EXISTS main.ward_audit_require_window_close_detail_insert
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
"#
    };
}

/// DDL for the durable `main.ward_audit` table inside `coven.sqlite3` (§3.4).
///
/// See module docs for the full schema-state contract. This `BEGIN IMMEDIATE`
/// transaction is safe to run unconditionally on every store open: it permits
/// only exact `missing` or `current_v020` before any mutation, uses idempotent
/// `IF NOT EXISTS` DDL for daemon compatibility, then requires exact
/// `current_v020` before `COMMIT`. Exact `legacy_v013`, every drifted
/// `unknown` shape, every unexpected durable reserved-namespace object, and
/// every temp shadow/reserved temp object fail closed. The IMMEDIATE
/// reservation serializes concurrent initializers before any guard read so the
/// loser waits, re-runs the guard against committed state, and sees exact
/// `current_v020` rather than racing into `sqlite_master` lock errors. Durable
/// schema objects are explicitly created in `main`; the temp guard tables live
/// in `temp` under unique non-reserved names. If this SQL errors, callers must
/// explicitly `ROLLBACK` before continuing so SQLite discards any uncommitted
/// work. This DDL never mutates database-wide `PRAGMA user_version`.
pub const WARD_AUDIT_SCHEMA_SQL: &str = concat!(
    r#"
BEGIN IMMEDIATE;

CREATE TEMP TABLE coven_threads_ward_audit_schema_pre_guard (
    ok INTEGER NOT NULL CHECK (ok = 1)
);

INSERT INTO coven_threads_ward_audit_schema_pre_guard (ok)
"#,
    ward_audit_schema_state_ctes_sql!(),
    r#"
SELECT CASE
    WHEN ("#,
    ward_audit_schema_state_case_sql!(),
    r#") IN ('missing', 'current_v020') THEN 1
    ELSE 0
END
FROM ward_audit_shape;

DROP TABLE coven_threads_ward_audit_schema_pre_guard;
"#,
    ward_audit_current_objects_sql!(),
    r#"
CREATE TEMP TABLE coven_threads_ward_audit_schema_post_guard (
    ok INTEGER NOT NULL CHECK (ok = 1)
);

INSERT INTO coven_threads_ward_audit_schema_post_guard (ok)
"#,
    ward_audit_schema_state_ctes_sql!(),
    r#"
SELECT CASE
    WHEN ("#,
    ward_audit_schema_state_case_sql!(),
    r#") = 'current_v020' THEN 1
    ELSE 0
END
FROM ward_audit_shape;

DROP TABLE coven_threads_ward_audit_schema_post_guard;

COMMIT;
"#,
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{SurfaceId, WriterId};
    use rusqlite::{params, Connection};
    use std::{
        collections::BTreeSet,
        fs,
        io::ErrorKind,
        path::{Path, PathBuf},
        sync::{Arc, Barrier},
        thread,
        time::Duration,
    };
    use uuid::Uuid;

    const FIXED_SUBMITTED_AT: &str = "2026-07-19T00:00:00.000Z";
    const FIXED_DECIDED_AT: &str = "2026-07-19T00:01:00.000Z";
    const FIXED_RECORDED_AT: &str = "2026-07-19T00:02:00.000Z";
    const FIXED_WARD_HASH: [u8; 32] = *b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const FIXED_DIFF_HASH: [u8; 32] = *b"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const FIXED_PREV_DETAIL: &str = r#"{"prev_sha256":"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc","bytes_written":42}"#;
    const FIXED_FILES_TOUCHED: &str = r#"["SOUL.md"]"#;
    const CONCURRENT_DB_RUNS: usize = 3;


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
    fn schema_enforces_append_only() {
        assert!(WARD_AUDIT_SCHEMA_SQL.contains("ward_audit_append_only_update"));
        assert!(WARD_AUDIT_SCHEMA_SQL.contains("ward_audit_append_only_delete"));
        assert!(WARD_AUDIT_SCHEMA_SQL.contains("append-only (RFC-0001 §5.6)"));
        assert!(WARD_AUDIT_SCHEMA_SQL.contains("ward_audit_require_single_terminal_insert"));
    }

    #[test]
    fn schema_names_all_window_close_reason_tags() {
        // The trigger SQL literals and the enum must not drift.
        for sql in [WARD_AUDIT_SCHEMA_SQL, WARD_AUDIT_MIGRATION_V020_SQL] {
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
        for sql in [WARD_AUDIT_SCHEMA_SQL, WARD_AUDIT_MIGRATION_V020_SQL] {
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
                // display_label round-trips through the ApprovalPath parser.
                let parsed = ApprovalPath::from_display_label(kind.display_label());
                assert!(parsed.is_some(), "ApprovalPath::from_display_label({}) failed", kind.display_label());
            }
        }
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

    /// Exact shipped v0.1.3 `ward_audit` DDL from `origin/main` / the PR base.
    /// Keep the inline comments: SQLite preserves them in `sqlite_master.sql`,
    /// and the legacy fingerprint intentionally matches that stored text.
    const LEGACY_WARD_AUDIT_SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS ward_audit (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type    TEXT    NOT NULL CHECK (event_type IN (
                      'proposal_submitted','proposal_approved','proposal_rejected',
                      'proposal_vetoed','ward_updated','validation_verdict',
                      'compaction_ledger')),
    proposal_id   TEXT,
    familiar_id   TEXT    NOT NULL,
    ward_version  TEXT,
    ward_hash     BLOB    NOT NULL,
    tier          TEXT,
    decision      TEXT    NOT NULL,
    approver      TEXT,
    diff_hash     BLOB,
    files_touched TEXT    NOT NULL, -- JSON array of surface ids
    channel       TEXT,
    thread_id     TEXT,
    submitted_at  TEXT    NOT NULL, -- RFC 3339
    decided_at    TEXT    NOT NULL, -- RFC 3339
    recorded_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
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
"#;

    const EXPECTED_EXPLICIT_INDEX_NAMES: &[&str] =
        &["ward_audit_event_idx", "ward_audit_familiar_idx"];
    const EXPECTED_LEGACY_TRIGGER_NAMES: &[&str] = &[
        "ward_audit_append_only_delete",
        "ward_audit_append_only_update",
    ];
    const EXPECTED_TRIGGER_NAMES: &[&str] = &[
        "ward_audit_append_only_delete",
        "ward_audit_append_only_update",
        "ward_audit_require_authorization_insert",
        "ward_audit_require_proposal_approval_detail_insert",
        "ward_audit_require_single_terminal_insert",
        "ward_audit_require_window_close_detail_insert",
    ];

    #[derive(Debug, PartialEq, Eq)]
    struct StoredAuditRow {
        id: i64,
        event_type: String,
        proposal_id: Option<String>,
        familiar_id: String,
        ward_version: Option<String>,
        ward_hash: Vec<u8>,
        tier: Option<String>,
        decision: String,
        approver: Option<String>,
        diff_hash: Option<Vec<u8>>,
        detail: Option<String>,
        files_touched: String,
        channel: Option<String>,
        thread_id: Option<String>,
        submitted_at: String,
        decided_at: String,
        recorded_at: String,
    }

    fn user_version(conn: &Connection) -> i64 {
        conn.pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap()
    }

    fn set_user_version(conn: &Connection, version: i64) {
        conn.pragma_update(None, "user_version", version).unwrap();
    }

    fn concurrent_db_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/ward-audit-concurrency")
    }

    fn sqlite_artifact_paths(path: &Path) -> Vec<PathBuf> {
        ["", "-journal", "-wal", "-shm"]
            .into_iter()
            .map(|suffix| {
                let mut candidate = path.as_os_str().to_os_string();
                candidate.push(suffix);
                PathBuf::from(candidate)
            })
            .collect()
    }

    fn cleanup_sqlite_artifacts(path: &Path) {
        for candidate in sqlite_artifact_paths(path) {
            match fs::remove_file(&candidate) {
                Ok(()) => {}
                Err(err) if err.kind() == ErrorKind::NotFound => {}
                Err(err) => panic!("failed to remove {}: {err}", candidate.display()),
            }
        }
    }

    fn assert_sqlite_artifacts_absent(path: &Path) {
        for candidate in sqlite_artifact_paths(path) {
            assert!(
                !candidate.exists(),
                "expected {} to be cleaned up",
                candidate.display()
            );
        }
    }

    struct ScratchDbPath {
        path: PathBuf,
    }

    impl ScratchDbPath {
        fn new(prefix: &str) -> Self {
            let dir = concurrent_db_dir();
            fs::create_dir_all(&dir).unwrap();
            let path = dir.join(format!("{prefix}-{}.sqlite3", Uuid::new_v4()));
            cleanup_sqlite_artifacts(&path);
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for ScratchDbPath {
        fn drop(&mut self) {
            for candidate in sqlite_artifact_paths(&self.path) {
                let _ = fs::remove_file(candidate);
            }
        }
    }

    fn open_file_backed_connection(path: &Path) -> Connection {
        let conn = Connection::open(path).unwrap();
        conn.busy_timeout(Duration::from_secs(5)).unwrap();
        conn
    }

    fn run_concurrent_sql(path: &Path, sql: &'static str) -> Vec<Result<(), String>> {
        let barrier = Arc::new(Barrier::new(2));
        let handles: Vec<_> = (0..2)
            .map(|_| {
                let barrier = Arc::clone(&barrier);
                let path = path.to_path_buf();
                thread::spawn(move || {
                    let conn = open_file_backed_connection(&path);
                    conn.commit_hook(Some(|| {
                        thread::sleep(Duration::from_millis(150));
                        false
                    }));
                    barrier.wait();
                    conn.execute_batch(sql).map_err(|err| err.to_string())
                })
            })
            .collect();

        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect()
    }

    fn ward_audit_schema_state(conn: &Connection) -> String {
        conn.query_row(WARD_AUDIT_SCHEMA_STATE_SQL, [], |row| row.get(0))
            .unwrap()
    }

    fn assert_schema_state(conn: &Connection, expected: &str) {
        assert_eq!(ward_audit_schema_state(conn), expected);
    }

    fn sql_literal_value(conn: &Connection, literal_sql: &str) -> String {
        conn.query_row(&format!("SELECT {literal_sql};"), [], |row| row.get(0))
            .unwrap()
    }

    fn schema_master_table(schema: &str) -> &'static str {
        match schema {
            "main" => "main.sqlite_master",
            "temp" => "temp.sqlite_master",
            _ => panic!("unexpected schema: {schema}"),
        }
    }

    fn ward_audit_column_names(conn: &Connection, schema: &str) -> Vec<String> {
        let sql =
            format!("SELECT name FROM pragma_table_info('ward_audit', '{schema}') ORDER BY cid;");
        let mut stmt = conn.prepare(&sql).unwrap();
        stmt.query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    fn stored_table_sql_in_schema(conn: &Connection, schema: &str) -> String {
        conn.query_row(
            &format!(
                "SELECT sql FROM {} WHERE type = 'table' AND name = 'ward_audit';",
                schema_master_table(schema)
            ),
            [],
            |row| row.get(0),
        )
        .unwrap()
    }

    fn stored_table_sql(conn: &Connection) -> String {
        stored_table_sql_in_schema(conn, "main")
    }

    fn explicit_index_sql_fingerprint(conn: &Connection) -> String {
        conn.query_row(
            r#"
            SELECT COALESCE(group_concat(item, '||'), '')
            FROM (
                SELECT printf('%s|%s', il.name, sm.sql) AS item
                FROM pragma_index_list('ward_audit', 'main') AS il
                JOIN main.sqlite_master AS sm
                  ON sm.type = 'index' AND sm.name = il.name
                WHERE il.origin = 'c' AND sm.sql IS NOT NULL
                ORDER BY il.name
            );
            "#,
            [],
            |row| row.get(0),
        )
        .unwrap()
    }

    fn trigger_sql_fingerprint(conn: &Connection) -> String {
        conn.query_row(
            r#"
            SELECT COALESCE(group_concat(item, '||'), '')
            FROM (
                SELECT printf('%s|%s', name, sql) AS item
                FROM main.sqlite_master
                WHERE type = 'trigger' AND tbl_name = 'ward_audit'
                ORDER BY name
            );
            "#,
            [],
            |row| row.get(0),
        )
        .unwrap()
    }

    fn schema_sql_with_extra_table_constraint(
        base_schema_sql: &str,
        constraint_sql: &str,
    ) -> String {
        let marker = "\n);\n\nCREATE INDEX";
        let replacement = format!(",\n    {constraint_sql}{marker}");
        let schema_sql = base_schema_sql.replacen(marker, &replacement, 1);
        assert_ne!(
            schema_sql, base_schema_sql,
            "expected to inject {constraint_sql}"
        );
        schema_sql
    }

    fn table_only_sql(base_schema_sql: &str) -> String {
        let marker = "\n);\n\nCREATE INDEX";
        let (table_sql, _) = base_schema_sql
            .split_once(marker)
            .expect("ward_audit schema SQL must contain the table/index boundary marker");
        format!("{table_sql}\n);")
    }

    fn temp_shadow_table_sql(base_schema_sql: &str) -> String {
        let table_sql = table_only_sql(base_schema_sql);
        let shadow_sql = table_sql
            .replacen(
                "CREATE TABLE IF NOT EXISTS main.ward_audit (",
                "CREATE TEMP TABLE ward_audit (",
                1,
            )
            .replacen(
                "CREATE TABLE IF NOT EXISTS ward_audit (",
                "CREATE TEMP TABLE ward_audit (",
                1,
            );
        assert_ne!(
            shadow_sql, table_sql,
            "expected to rewrite ward_audit to TEMP"
        );
        shadow_sql
    }

    fn create_temp_shadow_table(conn: &Connection, base_schema_sql: &str) {
        conn.execute_batch(&temp_shadow_table_sql(base_schema_sql))
            .unwrap();
    }

    fn inject_sql_before_anchor(base_sql: &str, anchor: &str, injection_sql: &str) -> String {
        let replacement = format!("{injection_sql}\n\n{anchor}");
        let updated = base_sql.replacen(anchor, &replacement, 1);
        assert_ne!(
            updated, base_sql,
            "expected to inject before anchor {anchor}"
        );
        updated
    }

    fn migration_sql_with_durable_drift_before_post_guard(drift_sql: &str) -> String {
        inject_sql_before_anchor(
            WARD_AUDIT_MIGRATION_V020_SQL,
            "CREATE TEMP TABLE coven_threads_ward_audit_migration_post_guard (",
            drift_sql,
        )
    }

    fn legacy_schema_with_extra_table_constraint(constraint_sql: &str) -> String {
        schema_sql_with_extra_table_constraint(LEGACY_WARD_AUDIT_SCHEMA_SQL, constraint_sql)
    }

    fn current_schema_with_extra_table_constraint(constraint_sql: &str) -> String {
        schema_sql_with_extra_table_constraint(ward_audit_current_objects_sql!(), constraint_sql)
    }

    fn drift_event_type_literal(
        schema_sql: &str,
        exact_literal: &str,
        drifted_literal: &str,
    ) -> String {
        let exact = format!("'{exact_literal}'");
        let drifted = format!("'{drifted_literal}'");
        let drifted_schema_sql = schema_sql.replacen(&exact, &drifted, 1);
        assert_ne!(
            drifted_schema_sql, schema_sql,
            "expected to drift {exact_literal} to {drifted_literal}"
        );
        drifted_schema_sql
    }

    fn expected_explicit_index_names() -> BTreeSet<String> {
        EXPECTED_EXPLICIT_INDEX_NAMES
            .iter()
            .map(|name| (*name).to_string())
            .collect()
    }

    fn expected_legacy_trigger_names() -> BTreeSet<String> {
        EXPECTED_LEGACY_TRIGGER_NAMES
            .iter()
            .map(|name| (*name).to_string())
            .collect()
    }

    fn expected_trigger_names() -> BTreeSet<String> {
        EXPECTED_TRIGGER_NAMES
            .iter()
            .map(|name| (*name).to_string())
            .collect()
    }

    fn explicit_index_names_in_schema(conn: &Connection, schema: &str) -> BTreeSet<String> {
        let sql = format!(
            "SELECT name FROM pragma_index_list('ward_audit', '{schema}') WHERE origin = 'c' AND name NOT LIKE 'sqlite_autoindex_%' ORDER BY name;"
        );
        let mut stmt = conn.prepare(&sql).unwrap();
        stmt.query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    fn explicit_index_names(conn: &Connection) -> BTreeSet<String> {
        explicit_index_names_in_schema(conn, "main")
    }

    fn trigger_names_in_schema(conn: &Connection, schema: &str) -> BTreeSet<String> {
        let mut stmt = conn
            .prepare(&format!(
                "SELECT name FROM {} WHERE type = 'trigger' AND tbl_name = 'ward_audit' ORDER BY name;",
                schema_master_table(schema)
            ))
            .unwrap();
        stmt.query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    fn trigger_names(conn: &Connection) -> BTreeSet<String> {
        trigger_names_in_schema(conn, "main")
    }

    fn has_column_in_schema(conn: &Connection, schema: &str, name: &str) -> bool {
        conn.query_row(
            &format!(
                "SELECT EXISTS(SELECT 1 FROM pragma_table_info('ward_audit', '{schema}') WHERE name = ?1);"
            ),
            params![name],
            |row| row.get::<_, i64>(0),
        )
        .unwrap()
            == 1
    }

    fn has_column(conn: &Connection, name: &str) -> bool {
        has_column_in_schema(conn, "main", name)
    }

    fn schema_object_exists(
        conn: &Connection,
        schema: &str,
        object_type: &str,
        name: &str,
    ) -> bool {
        conn.query_row(
            &format!(
                "SELECT EXISTS(SELECT 1 FROM {} WHERE type = ?1 AND name = ?2);",
                schema_master_table(schema)
            ),
            params![object_type, name],
            |row| row.get::<_, i64>(0),
        )
        .unwrap()
            == 1
    }

    fn main_schema_object_exists(conn: &Connection, object_type: &str, name: &str) -> bool {
        schema_object_exists(conn, "main", object_type, name)
    }

    fn temp_schema_object_exists(conn: &Connection, object_type: &str, name: &str) -> bool {
        schema_object_exists(conn, "temp", object_type, name)
    }

    fn reserved_main_namespace_object_key(object_type: &str, name: &str, tbl_name: &str) -> String {
        format!("{object_type}|{name}|{tbl_name}")
    }

    fn reserved_main_namespace_objects(conn: &Connection) -> BTreeSet<String> {
        let mut stmt = conn
            .prepare(
                r#"
                SELECT printf('%s|%s|%s', type, name, COALESCE(tbl_name, '<null>'))
                FROM main.sqlite_master
                WHERE type IN ('table', 'index', 'trigger', 'view')
                  AND (lower(name) = 'ward_audit' OR lower(name) GLOB 'ward_audit_*')
                ORDER BY type, name;
                "#,
            )
            .unwrap();
        stmt.query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    fn expected_legacy_reserved_main_namespace_objects() -> BTreeSet<String> {
        BTreeSet::from([
            reserved_main_namespace_object_key("index", "ward_audit_event_idx", "ward_audit"),
            reserved_main_namespace_object_key("index", "ward_audit_familiar_idx", "ward_audit"),
            reserved_main_namespace_object_key("table", "ward_audit", "ward_audit"),
            reserved_main_namespace_object_key(
                "trigger",
                "ward_audit_append_only_delete",
                "ward_audit",
            ),
            reserved_main_namespace_object_key(
                "trigger",
                "ward_audit_append_only_update",
                "ward_audit",
            ),
        ])
    }

    fn expected_reserved_main_namespace_objects() -> BTreeSet<String> {
        BTreeSet::from([
            reserved_main_namespace_object_key("index", "ward_audit_event_idx", "ward_audit"),
            reserved_main_namespace_object_key("index", "ward_audit_familiar_idx", "ward_audit"),
            reserved_main_namespace_object_key("table", "ward_audit", "ward_audit"),
            reserved_main_namespace_object_key(
                "trigger",
                "ward_audit_append_only_delete",
                "ward_audit",
            ),
            reserved_main_namespace_object_key(
                "trigger",
                "ward_audit_append_only_update",
                "ward_audit",
            ),
            reserved_main_namespace_object_key(
                "trigger",
                "ward_audit_require_single_terminal_insert",
                "ward_audit",
            ),
            reserved_main_namespace_object_key(
                "trigger",
                "ward_audit_require_authorization_insert",
                "ward_audit",
            ),
            reserved_main_namespace_object_key(
                "trigger",
                "ward_audit_require_proposal_approval_detail_insert",
                "ward_audit",
            ),
            reserved_main_namespace_object_key(
                "trigger",
                "ward_audit_require_window_close_detail_insert",
                "ward_audit",
            ),
        ])
    }

    fn ward_audit_row_count(conn: &Connection, schema: &str) -> i64 {
        conn.query_row(
            &format!("SELECT COUNT(*) FROM {schema}.ward_audit;"),
            [],
            |row| row.get(0),
        )
        .unwrap()
    }

    fn assert_fresh_schema_preserves_user_version(initial_version: i64) {
        let conn = Connection::open_in_memory().unwrap();
        set_user_version(&conn, initial_version);
        conn.execute_batch(WARD_AUDIT_SCHEMA_SQL).unwrap();

        assert_eq!(user_version(&conn), initial_version);
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V020);

        let row_id = insert_current_apply_audit_row(&conn);
        let row = load_audit_row(&conn, row_id);
        assert_eq!(row.event_type, "apply_audit");
        assert_eq!(row.detail.as_deref(), Some(FIXED_PREV_DETAIL));
    }

    fn assert_legacy_schema_guard_rollback_preserves_state(
        schema_sql: &str,
        after_rollback: impl FnOnce(&Connection),
    ) {
        let conn = Connection::open_in_memory().unwrap();
        set_user_version(&conn, 37);
        conn.execute_batch(schema_sql).unwrap();
        let row_id = insert_legacy_ward_updated_row(&conn);
        let before = load_legacy_audit_row(&conn, row_id);
        let before_version = user_version(&conn);

        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);

        let err = conn
            .execute_batch(WARD_AUDIT_MIGRATION_V020_SQL)
            .expect_err("drifted legacy schema must fail at the migration guard");
        assert!(
            err.to_string().contains("CHECK constraint failed"),
            "unexpected migration error: {err}"
        );
        conn.execute_batch("ROLLBACK;").unwrap();

        assert_eq!(load_legacy_audit_row(&conn, row_id), before);
        assert_eq!(user_version(&conn), before_version);
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);
        assert!(!has_column(&conn, "detail"));
        assert_eq!(explicit_index_names(&conn), expected_explicit_index_names());
        assert_eq!(trigger_names(&conn), expected_legacy_trigger_names());

        after_rollback(&conn);
    }

    fn assert_legacy_drift_guard_rollback_preserves_state(
        constraint_sql: &str,
        after_rollback: impl FnOnce(&Connection),
    ) {
        let schema_sql = legacy_schema_with_extra_table_constraint(constraint_sql);
        assert_legacy_schema_guard_rollback_preserves_state(&schema_sql, after_rollback);
    }

    fn load_audit_row_from_schema(conn: &Connection, schema: &str, id: i64) -> StoredAuditRow {
        conn.query_row(
            &format!(
                r#"
            SELECT id, event_type, proposal_id, familiar_id, ward_version, ward_hash,
                   tier, decision, approver, diff_hash, detail, files_touched,
                   channel, thread_id, submitted_at, decided_at, recorded_at
            FROM {schema}.ward_audit
            WHERE id = ?1
            "#
            ),
            params![id],
            |row| {
                Ok(StoredAuditRow {
                    id: row.get(0)?,
                    event_type: row.get(1)?,
                    proposal_id: row.get(2)?,
                    familiar_id: row.get(3)?,
                    ward_version: row.get(4)?,
                    ward_hash: row.get(5)?,
                    tier: row.get(6)?,
                    decision: row.get(7)?,
                    approver: row.get(8)?,
                    diff_hash: row.get(9)?,
                    detail: row.get(10)?,
                    files_touched: row.get(11)?,
                    channel: row.get(12)?,
                    thread_id: row.get(13)?,
                    submitted_at: row.get(14)?,
                    decided_at: row.get(15)?,
                    recorded_at: row.get(16)?,
                })
            },
        )
        .unwrap()
    }

    fn load_audit_row(conn: &Connection, id: i64) -> StoredAuditRow {
        load_audit_row_from_schema(conn, "main", id)
    }

    fn load_legacy_audit_row_from_schema(
        conn: &Connection,
        schema: &str,
        id: i64,
    ) -> StoredAuditRow {
        conn.query_row(
            &format!(
                r#"
            SELECT id, event_type, proposal_id, familiar_id, ward_version, ward_hash,
                   tier, decision, approver, diff_hash, files_touched, channel,
                   thread_id, submitted_at, decided_at, recorded_at
            FROM {schema}.ward_audit
            WHERE id = ?1
            "#
            ),
            params![id],
            |row| {
                Ok(StoredAuditRow {
                    id: row.get(0)?,
                    event_type: row.get(1)?,
                    proposal_id: row.get(2)?,
                    familiar_id: row.get(3)?,
                    ward_version: row.get(4)?,
                    ward_hash: row.get(5)?,
                    tier: row.get(6)?,
                    decision: row.get(7)?,
                    approver: row.get(8)?,
                    diff_hash: row.get(9)?,
                    detail: None,
                    files_touched: row.get(10)?,
                    channel: row.get(11)?,
                    thread_id: row.get(12)?,
                    submitted_at: row.get(13)?,
                    decided_at: row.get(14)?,
                    recorded_at: row.get(15)?,
                })
            },
        )
        .unwrap()
    }

    fn load_legacy_audit_row(conn: &Connection, id: i64) -> StoredAuditRow {
        load_legacy_audit_row_from_schema(conn, "main", id)
    }

    fn load_legacy_extra_value(conn: &Connection, id: i64) -> Option<String> {
        conn.query_row(
            "SELECT legacy_extra FROM main.ward_audit WHERE id = ?1;",
            params![id],
            |row| row.get(0),
        )
        .unwrap()
    }

    fn ward_audit_new_conflict_value(conn: &Connection) -> String {
        conn.query_row(
            "SELECT conflict FROM ward_audit_new ORDER BY rowid LIMIT 1;",
            [],
            |row| row.get(0),
        )
        .unwrap()
    }

    fn insert_current_apply_audit_row_into_schema(conn: &Connection, schema: &str) -> i64 {
        conn.execute(
            &format!(
                r#"
            INSERT INTO {schema}.ward_audit (
                event_type, proposal_id, familiar_id, ward_version, ward_hash,
                tier, decision, approver, diff_hash, detail, files_touched,
                channel, thread_id, submitted_at, decided_at, recorded_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5,
                ?6, ?7, ?8, ?9, ?10, ?11,
                ?12, ?13, ?14, ?15, ?16
            )
            "#
            ),
            params![
                "apply_audit",
                Some("proposal-current"),
                "familiar-current",
                Some("0.2.0"),
                FIXED_WARD_HASH.as_ref(),
                Some("tier_2"),
                "applied",
                Option::<String>::None,
                Some(FIXED_DIFF_HASH.as_ref()),
                FIXED_PREV_DETAIL,
                FIXED_FILES_TOUCHED,
                Some("mutation"),
                Option::<String>::None,
                FIXED_SUBMITTED_AT,
                FIXED_DECIDED_AT,
                FIXED_RECORDED_AT,
            ],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn insert_current_apply_audit_row(conn: &Connection) -> i64 {
        insert_current_apply_audit_row_into_schema(conn, "main")
    }

    fn insert_current_apply_audit_row_unqualified(conn: &Connection) -> i64 {
        conn.execute(
            r#"
            INSERT INTO ward_audit (
                event_type, proposal_id, familiar_id, ward_version, ward_hash,
                tier, decision, approver, diff_hash, detail, files_touched,
                channel, thread_id, submitted_at, decided_at, recorded_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5,
                ?6, ?7, ?8, ?9, ?10, ?11,
                ?12, ?13, ?14, ?15, ?16
            )
            "#,
            params![
                "apply_audit",
                Some("proposal-current"),
                "familiar-current",
                Some("0.2.0"),
                FIXED_WARD_HASH.as_ref(),
                Some("tier_2"),
                "applied",
                Option::<String>::None,
                Some(FIXED_DIFF_HASH.as_ref()),
                FIXED_PREV_DETAIL,
                FIXED_FILES_TOUCHED,
                Some("mutation"),
                Option::<String>::None,
                FIXED_SUBMITTED_AT,
                FIXED_DECIDED_AT,
                FIXED_RECORDED_AT,
            ],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn try_insert_legacy_ward_updated_row_into_schema(
        conn: &Connection,
        schema: &str,
        decision: &str,
        recorded_at: &str,
    ) -> rusqlite::Result<i64> {
        conn.execute(
            &format!(
                r#"
            INSERT INTO {schema}.ward_audit (
                event_type, proposal_id, familiar_id, ward_version, ward_hash,
                tier, decision, approver, diff_hash, files_touched, channel,
                thread_id, submitted_at, decided_at, recorded_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5,
                ?6, ?7, ?8, ?9, ?10, ?11,
                ?12, ?13, ?14, ?15
            )
            "#
            ),
            params![
                "ward_updated",
                Some("proposal-legacy"),
                "familiar-legacy",
                Some("0.1.3"),
                FIXED_WARD_HASH.as_ref(),
                Some("tier_1"),
                decision,
                Some("writer:legacy"),
                Some(FIXED_DIFF_HASH.as_ref()),
                FIXED_FILES_TOUCHED,
                Some("mutation"),
                Some("thread-legacy"),
                FIXED_SUBMITTED_AT,
                FIXED_DECIDED_AT,
                recorded_at,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    fn try_insert_legacy_ward_updated_row(
        conn: &Connection,
        decision: &str,
        recorded_at: &str,
    ) -> rusqlite::Result<i64> {
        try_insert_legacy_ward_updated_row_into_schema(conn, "main", decision, recorded_at)
    }

    fn insert_legacy_ward_updated_row(conn: &Connection) -> i64 {
        try_insert_legacy_ward_updated_row(conn, "updated", FIXED_RECORDED_AT).unwrap()
    }

    fn insert_legacy_ward_updated_row_with_extra(conn: &Connection, extra: &str) -> i64 {
        conn.execute(
            r#"
            INSERT INTO main.ward_audit (
                event_type, proposal_id, familiar_id, ward_version, ward_hash,
                tier, decision, approver, diff_hash, files_touched, channel,
                thread_id, submitted_at, decided_at, recorded_at, legacy_extra
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5,
                ?6, ?7, ?8, ?9, ?10, ?11,
                ?12, ?13, ?14, ?15, ?16
            )
            "#,
            params![
                "ward_updated",
                Some("proposal-legacy"),
                "familiar-legacy",
                Some("0.1.3"),
                FIXED_WARD_HASH.as_ref(),
                Some("tier_1"),
                "updated",
                Some("writer:legacy"),
                Some(FIXED_DIFF_HASH.as_ref()),
                FIXED_FILES_TOUCHED,
                Some("mutation"),
                Some("thread-legacy"),
                FIXED_SUBMITTED_AT,
                FIXED_DECIDED_AT,
                FIXED_RECORDED_AT,
                extra,
            ],
        )
        .unwrap();
        conn.last_insert_rowid()
    }
    #[test]
    fn schema_and_migration_sql_use_begin_immediate() {
        for (label, sql) in [
            ("schema", WARD_AUDIT_SCHEMA_SQL),
            ("migration", WARD_AUDIT_MIGRATION_V020_SQL),
        ] {
            assert!(
                sql.trim_start().starts_with("BEGIN IMMEDIATE;"),
                "{label} SQL must reserve the main database before guard reads"
            );
        }
    }

    #[test]
    fn migration_sql_uses_distinct_pre_and_post_guards_before_commit() {
        let pre_guard_offset = WARD_AUDIT_MIGRATION_V020_SQL
            .find("CREATE TEMP TABLE coven_threads_ward_audit_migration_guard (")
            .expect("migration SQL must define a precondition guard");
        let post_guard_offset = WARD_AUDIT_MIGRATION_V020_SQL
            .find("CREATE TEMP TABLE coven_threads_ward_audit_migration_post_guard (")
            .expect("migration SQL must define a postcondition guard");
        let post_guard_drop_offset = WARD_AUDIT_MIGRATION_V020_SQL
            .find("DROP TABLE coven_threads_ward_audit_migration_post_guard;")
            .expect("migration SQL must drop the postcondition guard before commit");
        let commit_offset = WARD_AUDIT_MIGRATION_V020_SQL
            .rfind("COMMIT;")
            .expect("migration SQL must commit on success");

        assert!(
            pre_guard_offset < post_guard_offset,
            "postcondition guard must run after the legacy precondition guard"
        );
        assert!(
            post_guard_offset < post_guard_drop_offset,
            "postcondition guard must be dropped on the success path"
        );
        assert!(
            post_guard_drop_offset < commit_offset,
            "postcondition guard must run before COMMIT"
        );
    }
    #[test]
    fn schema_state_query_returns_missing_on_empty_db() {
        let conn = Connection::open_in_memory().unwrap();
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_MISSING);
    }

    #[test]
    fn schema_qualified_table_valued_pragmas_resolve_main_and_temp_separately() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(WARD_AUDIT_SCHEMA_SQL).unwrap();
        conn.execute_batch(
            "CREATE TEMP TABLE ward_audit (
                 id INTEGER PRIMARY KEY,
                 shadow TEXT NOT NULL
             );
             CREATE INDEX temp.temp_shadow_idx ON ward_audit (shadow);",
        )
        .unwrap();

        assert_eq!(
            ward_audit_column_names(&conn, "main"),
            vec![
                "id".to_string(),
                "event_type".to_string(),
                "proposal_id".to_string(),
                "familiar_id".to_string(),
                "ward_version".to_string(),
                "ward_hash".to_string(),
                "tier".to_string(),
                "decision".to_string(),
                "approver".to_string(),
                "diff_hash".to_string(),
                "detail".to_string(),
                "files_touched".to_string(),
                "channel".to_string(),
                "thread_id".to_string(),
                "submitted_at".to_string(),
                "decided_at".to_string(),
                "recorded_at".to_string(),
            ]
        );
        assert_eq!(
            ward_audit_column_names(&conn, "temp"),
            vec!["id".to_string(), "shadow".to_string()]
        );
        assert_eq!(
            explicit_index_names_in_schema(&conn, "main"),
            expected_explicit_index_names()
        );
        assert_eq!(
            explicit_index_names_in_schema(&conn, "temp"),
            BTreeSet::from(["temp_shadow_idx".to_string()])
        );
    }

    #[test]
    fn current_main_with_temp_shadow_is_unknown_and_guards_preserve_main_and_temp_rows() {
        let conn = Connection::open_in_memory().unwrap();
        set_user_version(&conn, 37);
        conn.execute_batch(WARD_AUDIT_SCHEMA_SQL).unwrap();
        let main_row_id = insert_current_apply_audit_row(&conn);
        let main_before = load_audit_row(&conn, main_row_id);

        create_temp_shadow_table(&conn, ward_audit_current_objects_sql!());
        let temp_row_id = insert_current_apply_audit_row_into_schema(&conn, "temp");
        let temp_before = load_audit_row_from_schema(&conn, "temp", temp_row_id);

        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);

        let init_err = conn
            .execute_batch(WARD_AUDIT_SCHEMA_SQL)
            .expect_err("temp shadow must make schema init fail closed");
        assert!(
            init_err.to_string().contains("CHECK constraint failed"),
            "unexpected schema init error: {init_err}"
        );
        conn.execute_batch("ROLLBACK;").unwrap();

        assert_eq!(load_audit_row(&conn, main_row_id), main_before);
        assert_eq!(
            load_audit_row_from_schema(&conn, "temp", temp_row_id),
            temp_before
        );
        assert_eq!(ward_audit_row_count(&conn, "main"), 1);
        assert_eq!(ward_audit_row_count(&conn, "temp"), 1);
        assert!(temp_schema_object_exists(&conn, "table", "ward_audit"));
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);

        let migration_err = conn
            .execute_batch(WARD_AUDIT_MIGRATION_V020_SQL)
            .expect_err("temp shadow must make migration fail closed");
        assert!(
            migration_err
                .to_string()
                .contains("CHECK constraint failed"),
            "unexpected migration error: {migration_err}"
        );
        conn.execute_batch("ROLLBACK;").unwrap();

        assert_eq!(load_audit_row(&conn, main_row_id), main_before);
        assert_eq!(
            load_audit_row_from_schema(&conn, "temp", temp_row_id),
            temp_before
        );
        assert_eq!(ward_audit_row_count(&conn, "main"), 1);
        assert_eq!(ward_audit_row_count(&conn, "temp"), 1);
        assert!(temp_schema_object_exists(&conn, "table", "ward_audit"));
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);

        conn.execute_batch("DROP TABLE temp.ward_audit;").unwrap();
        assert!(!temp_schema_object_exists(&conn, "table", "ward_audit"));
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V020);
        assert_eq!(load_audit_row(&conn, main_row_id), main_before);
    }

    #[test]
    fn missing_main_with_temp_ward_audit_shadow_is_unknown_and_schema_sql_rejects_without_creating_main(
    ) {
        let conn = Connection::open_in_memory().unwrap();
        set_user_version(&conn, 23);
        create_temp_shadow_table(&conn, ward_audit_current_objects_sql!());
        let temp_row_id = insert_current_apply_audit_row_into_schema(&conn, "temp");
        let temp_before = load_audit_row_from_schema(&conn, "temp", temp_row_id);

        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);

        let err = conn
            .execute_batch(WARD_AUDIT_SCHEMA_SQL)
            .expect_err("temp ward_audit shadow must block schema init before main creation");
        assert!(
            err.to_string().contains("CHECK constraint failed"),
            "unexpected schema init error: {err}"
        );
        assert!(!main_schema_object_exists(&conn, "table", "ward_audit"));
        conn.execute_batch("ROLLBACK;").unwrap();

        assert!(!main_schema_object_exists(&conn, "table", "ward_audit"));
        assert_eq!(
            load_audit_row_from_schema(&conn, "temp", temp_row_id),
            temp_before
        );
        assert_eq!(ward_audit_row_count(&conn, "temp"), 1);
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);
        assert_eq!(user_version(&conn), 23);

        conn.execute_batch("DROP TABLE temp.ward_audit;").unwrap();
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_MISSING);
    }

    #[test]
    fn missing_main_with_reserved_temp_objects_is_unknown_and_schema_sql_preserves_temp_objects() {
        for (label, setup_sql, object_type, object_name) in [
            (
                "view",
                "CREATE TEMP VIEW ward_audit_shadow_view AS SELECT 1 AS sentinel;",
                "view",
                "ward_audit_shadow_view",
            ),
            (
                "index",
                "CREATE TEMP TABLE other (id INTEGER PRIMARY KEY, note TEXT NOT NULL);
                 CREATE INDEX temp.ward_audit_shadow_idx ON other (note);",
                "index",
                "ward_audit_shadow_idx",
            ),
            (
                "trigger",
                "CREATE TEMP TABLE other (id INTEGER PRIMARY KEY, note TEXT NOT NULL);
                 CREATE TRIGGER temp.ward_audit_shadow_trigger
                 BEFORE UPDATE ON other
                 BEGIN
                     SELECT RAISE(ABORT, 'other is append-only');
                 END;",
                "trigger",
                "ward_audit_shadow_trigger",
            ),
        ] {
            let conn = Connection::open_in_memory().unwrap();
            conn.execute_batch(setup_sql).unwrap();

            assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);

            let err = conn
                .execute_batch(WARD_AUDIT_SCHEMA_SQL)
                .expect_err("reserved temp object must block schema init");
            assert!(
                err.to_string().contains("CHECK constraint failed"),
                "unexpected schema init error for {label}: {err}"
            );
            assert!(
                !main_schema_object_exists(&conn, "table", "ward_audit"),
                "schema init for {label} must not create main.ward_audit before rollback"
            );
            conn.execute_batch("ROLLBACK;").unwrap();

            assert!(
                temp_schema_object_exists(&conn, object_type, object_name),
                "{label} temp object should survive rollback"
            );
            assert!(!main_schema_object_exists(&conn, "table", "ward_audit"));
            assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);
        }
    }

    #[test]
    fn fresh_schema_sql_initializes_current_schema_atomically_and_enforces_append_only() {
        let conn = Connection::open_in_memory().unwrap();
        set_user_version(&conn, 11);
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_MISSING);

        conn.execute_batch(WARD_AUDIT_SCHEMA_SQL).unwrap();

        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V020);
        assert_eq!(user_version(&conn), 11);
        assert_eq!(explicit_index_names(&conn), expected_explicit_index_names());
        assert_eq!(trigger_names(&conn), expected_trigger_names());

        let row_id = insert_current_apply_audit_row(&conn);
        let update_err = conn
            .execute(
                "UPDATE ward_audit SET decision = 'changed' WHERE id = ?1;",
                params![row_id],
            )
            .unwrap_err();
        assert!(update_err.to_string().contains("append-only"));

        let delete_err = conn
            .execute("DELETE FROM ward_audit WHERE id = ?1;", params![row_id])
            .unwrap_err();
        assert!(delete_err.to_string().contains("append-only"));
    }

    #[test]
    fn exact_legacy_fixture_returns_legacy_v013() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(LEGACY_WARD_AUDIT_SCHEMA_SQL).unwrap();
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_LEGACY_V013);
        assert_eq!(
            reserved_main_namespace_objects(&conn),
            expected_legacy_reserved_main_namespace_objects()
        );
        assert_eq!(
            stored_table_sql(&conn),
            sql_literal_value(&conn, ward_audit_exact_legacy_table_sql_sql!())
        );
        assert_eq!(
            explicit_index_sql_fingerprint(&conn),
            sql_literal_value(&conn, ward_audit_exact_explicit_index_fp_sql!())
        );
        assert_eq!(
            trigger_sql_fingerprint(&conn),
            sql_literal_value(&conn, ward_audit_exact_legacy_trigger_fp_sql!())
        );
    }

    #[test]
    fn exact_current_schema_returns_current_v020() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(WARD_AUDIT_SCHEMA_SQL).unwrap();
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V020);
        assert_eq!(
            reserved_main_namespace_objects(&conn),
            expected_reserved_main_namespace_objects()
        );
        assert_eq!(
            stored_table_sql(&conn),
            sql_literal_value(&conn, ward_audit_exact_current_fresh_table_sql_sql!())
        );
        assert_eq!(
            explicit_index_sql_fingerprint(&conn),
            sql_literal_value(&conn, ward_audit_exact_explicit_index_fp_sql!())
        );
        assert_eq!(
            trigger_sql_fingerprint(&conn),
            sql_literal_value(&conn, ward_audit_exact_trigger_fp_sql!())
        );
    }

    #[test]
    fn current_schema_sql_reruns_idempotently_and_preserves_rows_and_objects() {
        let conn = Connection::open_in_memory().unwrap();
        set_user_version(&conn, 29);
        conn.execute_batch(WARD_AUDIT_SCHEMA_SQL).unwrap();
        let row_id = insert_current_apply_audit_row(&conn);
        let before = load_audit_row(&conn, row_id);
        let before_table_sql = stored_table_sql(&conn);
        let before_indexes = explicit_index_names(&conn);
        let before_triggers = trigger_names(&conn);

        conn.execute_batch(WARD_AUDIT_SCHEMA_SQL).unwrap();

        assert_eq!(load_audit_row(&conn, row_id), before);
        assert_eq!(stored_table_sql(&conn), before_table_sql);
        assert_eq!(explicit_index_names(&conn), before_indexes);
        assert_eq!(trigger_names(&conn), before_triggers);
        assert_eq!(user_version(&conn), 29);
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V020);
    }

    #[test]
    fn fresh_and_migrated_current_schemas_use_controlled_exact_sql_variants() {
        let fresh = Connection::open_in_memory().unwrap();
        fresh.execute_batch(WARD_AUDIT_SCHEMA_SQL).unwrap();

        let migrated = Connection::open_in_memory().unwrap();
        migrated
            .execute_batch(LEGACY_WARD_AUDIT_SCHEMA_SQL)
            .unwrap();
        migrated
            .execute_batch(WARD_AUDIT_MIGRATION_V020_SQL)
            .unwrap();

        assert_schema_state(&fresh, WARD_AUDIT_SCHEMA_STATE_CURRENT_V020);
        assert_schema_state(&migrated, WARD_AUDIT_SCHEMA_STATE_CURRENT_V020);
        assert_eq!(
            stored_table_sql(&fresh),
            sql_literal_value(&fresh, ward_audit_exact_current_fresh_table_sql_sql!())
        );
        assert_eq!(
            stored_table_sql(&migrated),
            sql_literal_value(
                &migrated,
                ward_audit_exact_current_migrated_table_sql_sql!()
            )
        );
        assert_ne!(stored_table_sql(&fresh), stored_table_sql(&migrated));
        assert_eq!(
            explicit_index_sql_fingerprint(&fresh),
            sql_literal_value(&fresh, ward_audit_exact_explicit_index_fp_sql!())
        );
        assert_eq!(
            explicit_index_sql_fingerprint(&migrated),
            sql_literal_value(&migrated, ward_audit_exact_explicit_index_fp_sql!())
        );
        assert_eq!(
            trigger_sql_fingerprint(&fresh),
            sql_literal_value(&fresh, ward_audit_exact_trigger_fp_sql!())
        );
        assert_eq!(
            trigger_sql_fingerprint(&migrated),
            sql_literal_value(&migrated, ward_audit_exact_trigger_fp_sql!())
        );
    }

    #[test]
    fn current_schema_with_spaced_event_type_literal_is_unknown() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(&drift_event_type_literal(
            ward_audit_current_objects_sql!(),
            "apply_audit",
            "apply_ audit",
        ))
        .unwrap();

        assert_eq!(explicit_index_names(&conn), expected_explicit_index_names());
        assert_eq!(trigger_names(&conn), expected_trigger_names());
        assert!(stored_table_sql(&conn).contains("'apply_ audit'"));
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);
    }

    #[test]
    fn legacy_main_with_temp_shadow_is_unknown_and_migration_rejects_before_mutating_either_schema()
    {
        let conn = Connection::open_in_memory().unwrap();
        set_user_version(&conn, 37);
        conn.execute_batch(LEGACY_WARD_AUDIT_SCHEMA_SQL).unwrap();
        let main_row_id = insert_legacy_ward_updated_row(&conn);
        let main_before = load_legacy_audit_row(&conn, main_row_id);

        create_temp_shadow_table(&conn, LEGACY_WARD_AUDIT_SCHEMA_SQL);
        let temp_row_id = try_insert_legacy_ward_updated_row_into_schema(
            &conn,
            "temp",
            "updated",
            FIXED_RECORDED_AT,
        )
        .unwrap();
        let temp_before = load_legacy_audit_row_from_schema(&conn, "temp", temp_row_id);

        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);

        let err = conn
            .execute_batch(WARD_AUDIT_MIGRATION_V020_SQL)
            .expect_err("temp shadow must make legacy migration fail before mutation");
        assert!(
            err.to_string().contains("CHECK constraint failed"),
            "unexpected migration error: {err}"
        );
        assert!(!has_column_in_schema(&conn, "main", "detail"));
        assert!(!has_column_in_schema(&conn, "temp", "detail"));
        assert!(!main_schema_object_exists(&conn, "table", "ward_audit_new"));
        assert!(temp_schema_object_exists(&conn, "table", "ward_audit"));

        conn.execute_batch("ROLLBACK;").unwrap();

        assert_eq!(load_legacy_audit_row(&conn, main_row_id), main_before);
        assert_eq!(
            load_legacy_audit_row_from_schema(&conn, "temp", temp_row_id),
            temp_before
        );
        assert_eq!(user_version(&conn), 37);
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);

        conn.execute_batch("DROP TABLE temp.ward_audit;").unwrap();
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_LEGACY_V013);
    }

    #[test]
    fn legacy_schema_with_spaced_event_type_literal_is_unknown_and_guard_preserves_state() {
        let schema_sql = drift_event_type_literal(
            LEGACY_WARD_AUDIT_SCHEMA_SQL,
            "compaction_ledger",
            "compaction_ ledger",
        );
        assert_legacy_schema_guard_rollback_preserves_state(&schema_sql, |conn| {
            assert!(stored_table_sql(conn).contains("'compaction_ ledger'"));
            assert!(!stored_table_sql(conn).contains("'compaction_ledger'"));
        });
    }

    #[test]
    fn legacy_schema_sql_rejects_and_rollback_preserves_state() {
        let conn = Connection::open_in_memory().unwrap();
        set_user_version(&conn, 37);
        conn.execute_batch(LEGACY_WARD_AUDIT_SCHEMA_SQL).unwrap();
        let row_id = insert_legacy_ward_updated_row(&conn);
        let before = load_legacy_audit_row(&conn, row_id);
        let before_version = user_version(&conn);

        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_LEGACY_V013);

        let err = conn
            .execute_batch(WARD_AUDIT_SCHEMA_SQL)
            .expect_err("exact legacy schema must fail closed at the schema-init guard");
        assert!(
            err.to_string().contains("CHECK constraint failed"),
            "unexpected schema init error: {err}"
        );
        conn.execute_batch("ROLLBACK;").unwrap();

        assert_eq!(load_legacy_audit_row(&conn, row_id), before);
        assert_eq!(user_version(&conn), before_version);
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_LEGACY_V013);
        assert!(!has_column(&conn, "detail"));
        assert_eq!(explicit_index_names(&conn), expected_explicit_index_names());
        assert_eq!(trigger_names(&conn), expected_legacy_trigger_names());
    }

    #[test]
    fn fresh_schema_preserves_user_version_zero_and_creates_current_shape() {
        assert_fresh_schema_preserves_user_version(0);
    }

    #[test]
    fn fresh_schema_preserves_user_version_ninety_nine_and_creates_current_shape() {
        assert_fresh_schema_preserves_user_version(99);
    }

    #[test]
    fn legacy_plus_extra_column_and_data_is_unknown_and_guard_preserves_state() {
        let conn = Connection::open_in_memory().unwrap();
        set_user_version(&conn, 37);
        conn.execute_batch(LEGACY_WARD_AUDIT_SCHEMA_SQL).unwrap();
        conn.execute_batch("ALTER TABLE ward_audit ADD COLUMN legacy_extra TEXT;")
            .unwrap();
        let row_id = insert_legacy_ward_updated_row_with_extra(&conn, "survivor");
        let before = load_legacy_audit_row(&conn, row_id);
        let before_extra = load_legacy_extra_value(&conn, row_id);
        let before_version = user_version(&conn);

        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);

        let err = conn
            .execute_batch(WARD_AUDIT_MIGRATION_V020_SQL)
            .expect_err("partial legacy schema must fail at the migration guard");
        assert!(
            err.to_string().contains("CHECK constraint failed"),
            "unexpected migration error: {err}"
        );
        conn.execute_batch("ROLLBACK;").unwrap();

        assert_eq!(load_legacy_audit_row(&conn, row_id), before);
        assert_eq!(load_legacy_extra_value(&conn, row_id), before_extra);
        assert_eq!(user_version(&conn), before_version);
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);
        assert!(has_column(&conn, "legacy_extra"));
        assert!(!has_column(&conn, "detail"));
        assert_eq!(explicit_index_names(&conn), expected_explicit_index_names());
        assert_eq!(trigger_names(&conn), expected_legacy_trigger_names());
    }

    #[test]
    fn legacy_schema_with_extra_table_check_is_unknown_and_guard_preserves_constraint() {
        assert_legacy_drift_guard_rollback_preserves_state(
            "CHECK (length(decision) > 0)",
            |conn| {
                let err = try_insert_legacy_ward_updated_row(conn, "", FIXED_RECORDED_AT)
                    .expect_err(
                        "legacy CHECK drift must still reject empty decisions after rollback",
                    );
                assert!(
                    err.to_string().contains("CHECK constraint failed"),
                    "unexpected post-rollback CHECK error: {err}"
                );
            },
        );
    }

    #[test]
    fn legacy_schema_with_extra_unique_is_unknown_and_guard_preserves_constraint() {
        assert_legacy_drift_guard_rollback_preserves_state(
            "UNIQUE (decision, recorded_at)",
            |conn| {
                let err = try_insert_legacy_ward_updated_row(conn, "updated", FIXED_RECORDED_AT)
                    .expect_err(
                        "legacy UNIQUE drift must still reject duplicate decision/recorded_at rows after rollback",
                    );
                assert!(
                    err.to_string().contains("UNIQUE constraint failed"),
                    "unexpected post-rollback UNIQUE error: {err}"
                );
            },
        );
    }

    #[test]
    fn current_schema_with_extra_table_check_is_unknown() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(&current_schema_with_extra_table_constraint(
            "CHECK (length(decision) > 0)",
        ))
        .unwrap();

        assert_eq!(explicit_index_names(&conn), expected_explicit_index_names());
        assert_eq!(trigger_names(&conn), expected_trigger_names());
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);
    }

    #[test]
    fn current_schema_with_extra_unique_is_unknown() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(&current_schema_with_extra_table_constraint(
            "UNIQUE (decision, recorded_at)",
        ))
        .unwrap();

        assert_eq!(explicit_index_names(&conn), expected_explicit_index_names());
        assert_eq!(trigger_names(&conn), expected_trigger_names());
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);
    }

    #[test]
    fn current_schema_missing_append_only_trigger_is_unknown_and_update_succeeds() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(WARD_AUDIT_SCHEMA_SQL).unwrap();
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V020);
        let row_id = insert_current_apply_audit_row(&conn);

        conn.execute_batch("DROP TRIGGER ward_audit_append_only_update;")
            .unwrap();

        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);
        conn.execute(
            "UPDATE ward_audit SET decision = 'mutated' WHERE id = ?1;",
            params![row_id],
        )
        .unwrap();
        assert_eq!(load_audit_row(&conn, row_id).decision, "mutated");
    }

    #[test]
    fn current_schema_with_extra_index_is_unknown() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(WARD_AUDIT_SCHEMA_SQL).unwrap();
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V020);

        conn.execute_batch("CREATE INDEX ward_audit_decision_idx ON ward_audit (decision);")
            .unwrap();

        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);
    }

    #[test]
    fn current_schema_with_desc_index_is_unknown() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(WARD_AUDIT_SCHEMA_SQL).unwrap();
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V020);

        conn.execute_batch(
            "DROP INDEX ward_audit_event_idx;
             CREATE INDEX ward_audit_event_idx ON ward_audit (event_type, recorded_at DESC);",
        )
        .unwrap();

        assert_eq!(explicit_index_names(&conn), expected_explicit_index_names());
        assert!(
            explicit_index_sql_fingerprint(&conn).contains("recorded_at DESC"),
            "expected DESC drift in index SQL"
        );
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);
    }

    #[test]
    fn current_schema_with_collated_index_is_unknown() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(WARD_AUDIT_SCHEMA_SQL).unwrap();
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V020);

        conn.execute_batch(
            "DROP INDEX ward_audit_event_idx;
             CREATE INDEX ward_audit_event_idx ON ward_audit (event_type COLLATE NOCASE, recorded_at);",
        )
        .unwrap();

        assert_eq!(explicit_index_names(&conn), expected_explicit_index_names());
        assert!(
            explicit_index_sql_fingerprint(&conn).contains("COLLATE NOCASE"),
            "expected collation drift in index SQL"
        );
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);
    }

    #[test]
    fn current_schema_with_extra_reserved_main_view_or_table_is_unknown_and_schema_sql_preserves_state(
    ) {
        for (label, setup_sql, object_type, object_name, object_row_value) in [
            (
                "view",
                "CREATE VIEW ward_audit_history AS SELECT id, decision FROM main.ward_audit;",
                "view",
                "ward_audit_history",
                None,
            ),
            (
                "table",
                "CREATE TABLE ward_audit_backup (id INTEGER PRIMARY KEY, note TEXT NOT NULL);
                 INSERT INTO ward_audit_backup (id, note) VALUES (1, 'sentinel');",
                "table",
                "ward_audit_backup",
                Some("sentinel"),
            ),
        ] {
            let conn = Connection::open_in_memory().unwrap();
            set_user_version(&conn, 41);
            conn.execute_batch(WARD_AUDIT_SCHEMA_SQL).unwrap();
            let row_id = insert_current_apply_audit_row(&conn);
            let before = load_audit_row(&conn, row_id);

            conn.execute_batch(setup_sql).unwrap();

            let reserved_objects = reserved_main_namespace_objects(&conn);
            assert!(
                expected_reserved_main_namespace_objects().is_subset(&reserved_objects),
                "{label} drift must preserve the expected durable whitelist objects"
            );
            assert!(
                reserved_objects.contains(&reserved_main_namespace_object_key(
                    object_type,
                    object_name,
                    object_name
                )),
                "{label} drift should register the extra durable namespace object"
            );
            assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);

            let err = conn
                .execute_batch(WARD_AUDIT_SCHEMA_SQL)
                .expect_err("extra reserved durable object must make schema init fail closed");
            assert!(
                err.to_string().contains("CHECK constraint failed"),
                "unexpected schema init error for {label}: {err}"
            );
            conn.execute_batch("ROLLBACK;").unwrap();

            assert_eq!(load_audit_row(&conn, row_id), before);
            assert_eq!(user_version(&conn), 41);
            assert_eq!(explicit_index_names(&conn), expected_explicit_index_names());
            assert_eq!(trigger_names(&conn), expected_trigger_names());
            assert!(main_schema_object_exists(&conn, object_type, object_name));
            if let Some(expected_note) = object_row_value {
                assert_eq!(
                    conn.query_row(
                        "SELECT note FROM ward_audit_backup WHERE id = 1;",
                        [],
                        |row| row.get::<_, String>(0)
                    )
                    .unwrap(),
                    expected_note
                );
            }
            assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);
        }
    }

    #[test]
    fn current_schema_with_altered_trigger_error_literal_is_unknown() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(WARD_AUDIT_SCHEMA_SQL).unwrap();
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V020);

        conn.execute_batch(
            "DROP TRIGGER ward_audit_append_only_update;
             CREATE TRIGGER ward_audit_append_only_update
             BEFORE UPDATE ON ward_audit
             BEGIN
                 SELECT RAISE(ABORT, 'ward_audit is append-only (drifted)');
             END;",
        )
        .unwrap();

        assert_eq!(trigger_names(&conn), expected_trigger_names());
        assert!(
            trigger_sql_fingerprint(&conn).contains("drifted"),
            "expected trigger-literal drift in trigger SQL"
        );
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);
    }

    #[test]
    fn current_schema_with_altered_trigger_body_is_unknown() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(WARD_AUDIT_SCHEMA_SQL).unwrap();
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V020);

        conn.execute_batch(
            "DROP TRIGGER ward_audit_append_only_delete;
             CREATE TRIGGER ward_audit_append_only_delete
             BEFORE DELETE ON ward_audit
             BEGIN
                 SELECT 1;
                 SELECT RAISE(ABORT, 'ward_audit is append-only (RFC-0001 §5.6)');
             END;",
        )
        .unwrap();

        assert_eq!(trigger_names(&conn), expected_trigger_names());
        assert!(
            trigger_sql_fingerprint(&conn).contains("SELECT 1;"),
            "expected trigger-body drift in trigger SQL"
        );
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);
    }

    #[test]
    fn absent_ward_audit_with_reserved_index_collision_is_unknown_and_schema_sql_preserves_other_objects(
    ) {
        let conn = Connection::open_in_memory().unwrap();
        set_user_version(&conn, 23);
        conn.execute_batch(
            "CREATE TABLE other (id INTEGER PRIMARY KEY, note TEXT NOT NULL);
             INSERT INTO other (id, note) VALUES (1, 'sentinel');
             CREATE INDEX ward_audit_event_idx ON other (note);",
        )
        .unwrap();

        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);

        let err = conn
            .execute_batch(WARD_AUDIT_SCHEMA_SQL)
            .expect_err("reserved index collision must fail closed before creating ward_audit");
        assert!(
            err.to_string().contains("CHECK constraint failed"),
            "unexpected schema init error: {err}"
        );
        conn.execute_batch("ROLLBACK;").unwrap();

        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);
        assert!(!main_schema_object_exists(&conn, "table", "ward_audit"));
        assert!(main_schema_object_exists(&conn, "table", "other"));
        assert!(main_schema_object_exists(
            &conn,
            "index",
            "ward_audit_event_idx"
        ));
        assert_eq!(
            conn.query_row("SELECT note FROM other WHERE id = 1;", [], |row| row
                .get::<_, String>(0))
                .unwrap(),
            "sentinel"
        );
        assert_eq!(user_version(&conn), 23);
    }

    #[test]
    fn absent_ward_audit_with_reserved_trigger_collision_is_unknown_and_schema_sql_preserves_other_objects(
    ) {
        let conn = Connection::open_in_memory().unwrap();
        set_user_version(&conn, 31);
        conn.execute_batch(
            "CREATE TABLE other (id INTEGER PRIMARY KEY, note TEXT NOT NULL);
             INSERT INTO other (id, note) VALUES (1, 'sentinel');
             CREATE TRIGGER ward_audit_append_only_update
             BEFORE UPDATE ON other
             BEGIN
                 SELECT RAISE(ABORT, 'other is append-only');
             END;",
        )
        .unwrap();

        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);

        let err = conn
            .execute_batch(WARD_AUDIT_SCHEMA_SQL)
            .expect_err("reserved trigger collision must fail closed before creating ward_audit");
        assert!(
            err.to_string().contains("CHECK constraint failed"),
            "unexpected schema init error: {err}"
        );
        conn.execute_batch("ROLLBACK;").unwrap();

        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);
        assert!(!main_schema_object_exists(&conn, "table", "ward_audit"));
        assert!(main_schema_object_exists(&conn, "table", "other"));
        assert!(main_schema_object_exists(
            &conn,
            "trigger",
            "ward_audit_append_only_update"
        ));
        let trigger_err = conn
            .execute("UPDATE other SET note = 'changed' WHERE id = 1;", [])
            .unwrap_err();
        assert!(trigger_err.to_string().contains("other is append-only"));
        assert_eq!(
            conn.query_row("SELECT note FROM other WHERE id = 1;", [], |row| row
                .get::<_, String>(0))
                .unwrap(),
            "sentinel"
        );
        assert_eq!(user_version(&conn), 31);
    }

    #[test]
    fn unknown_partial_current_schema_rejects_schema_sql_and_preserves_state() {
        let conn = Connection::open_in_memory().unwrap();
        set_user_version(&conn, 37);
        conn.execute_batch(WARD_AUDIT_SCHEMA_SQL).unwrap();
        let row_id = insert_current_apply_audit_row(&conn);
        let before = load_audit_row(&conn, row_id);

        conn.execute_batch("DROP TRIGGER ward_audit_append_only_update;")
            .unwrap();
        let before_triggers = trigger_names(&conn);

        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);

        let err = conn
            .execute_batch(WARD_AUDIT_SCHEMA_SQL)
            .expect_err("partial current schema must fail closed at the schema-init guard");
        assert!(
            err.to_string().contains("CHECK constraint failed"),
            "unexpected schema init error: {err}"
        );
        conn.execute_batch("ROLLBACK;").unwrap();

        assert_eq!(load_audit_row(&conn, row_id), before);
        assert_eq!(trigger_names(&conn), before_triggers);
        assert_eq!(explicit_index_names(&conn), expected_explicit_index_names());
        assert_eq!(user_version(&conn), 37);
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);
    }

    #[test]
    fn migration_rejects_current_schema_rows_with_detail_and_preserves_state() {
        let conn = Connection::open_in_memory().unwrap();
        set_user_version(&conn, 37);
        conn.execute_batch(WARD_AUDIT_SCHEMA_SQL).unwrap();
        let row_id = insert_current_apply_audit_row(&conn);
        let before = load_audit_row(&conn, row_id);
        let before_version = user_version(&conn);

        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V020);

        let err = conn
            .execute_batch(WARD_AUDIT_MIGRATION_V020_SQL)
            .expect_err("current schema migration must fail at the legacy guard");
        assert!(
            err.to_string().contains("CHECK constraint failed"),
            "unexpected migration error: {err}"
        );
        conn.execute_batch("ROLLBACK;").unwrap();

        assert_eq!(load_audit_row(&conn, row_id), before);
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V020);
        assert_eq!(user_version(&conn), before_version);
    }

    #[test]
    fn legacy_schema_with_preexisting_main_ward_audit_new_is_unknown_and_guard_rejects_before_alter(
    ) {
        let conn = Connection::open_in_memory().unwrap();
        set_user_version(&conn, 37);
        conn.execute_batch(LEGACY_WARD_AUDIT_SCHEMA_SQL).unwrap();
        let row_id = insert_legacy_ward_updated_row(&conn);
        let before = load_legacy_audit_row(&conn, row_id);
        conn.execute_batch(
            "CREATE TABLE main.ward_audit_new (conflict TEXT NOT NULL);
             INSERT INTO main.ward_audit_new (conflict) VALUES ('sentinel');",
        )
        .unwrap();

        let reserved_objects = reserved_main_namespace_objects(&conn);
        assert!(
            expected_legacy_reserved_main_namespace_objects().is_subset(&reserved_objects),
            "legacy fixture should still include the expected durable whitelist objects"
        );
        assert!(
            reserved_objects.contains(&reserved_main_namespace_object_key(
                "table",
                "ward_audit_new",
                "ward_audit_new"
            )),
            "preexisting ward_audit_new should register as an unexpected durable object"
        );
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);

        let err = conn
            .execute_batch(WARD_AUDIT_MIGRATION_V020_SQL)
            .expect_err("preexisting ward_audit_new must make migration fail before ALTER");
        assert!(
            err.to_string().contains("CHECK constraint failed"),
            "unexpected migration error: {err}"
        );
        assert!(!has_column(&conn, "detail"));
        conn.execute_batch("ROLLBACK;").unwrap();

        assert_eq!(load_legacy_audit_row(&conn, row_id), before);
        assert_eq!(user_version(&conn), 37);
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);
        assert!(!has_column(&conn, "detail"));
        assert_eq!(ward_audit_new_conflict_value(&conn), "sentinel");
        assert_eq!(explicit_index_names(&conn), expected_explicit_index_names());
        assert_eq!(trigger_names(&conn), expected_legacy_trigger_names());
    }

    #[test]
    fn legacy_schema_upgrade_passes_post_guard_and_preserves_append_only_behavior() {
        let conn = Connection::open_in_memory().unwrap();
        set_user_version(&conn, 37);
        conn.execute_batch(LEGACY_WARD_AUDIT_SCHEMA_SQL).unwrap();
        let row_id = insert_legacy_ward_updated_row(&conn);
        let before = load_legacy_audit_row(&conn, row_id);
        assert_eq!(user_version(&conn), 37);
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_LEGACY_V013);

        conn.execute_batch(WARD_AUDIT_MIGRATION_V020_SQL).unwrap();

        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V020);
        assert_eq!(user_version(&conn), 37);
        assert_eq!(explicit_index_names(&conn), expected_explicit_index_names());
        assert_eq!(trigger_names(&conn), expected_trigger_names());

        let after = load_audit_row(&conn, row_id);
        assert_eq!(after.id, row_id);
        assert_eq!(after.event_type, before.event_type);
        assert_eq!(after.proposal_id, before.proposal_id);
        assert_eq!(after.familiar_id, before.familiar_id);
        assert_eq!(after.ward_version, before.ward_version);
        assert_eq!(after.ward_hash, before.ward_hash);
        assert_eq!(after.tier, before.tier);
        assert_eq!(after.decision, before.decision);
        assert_eq!(after.approver, before.approver);
        assert_eq!(after.diff_hash, before.diff_hash);
        assert_eq!(after.detail, None);
        assert_eq!(after.files_touched, before.files_touched);
        assert_eq!(after.channel, before.channel);
        assert_eq!(after.thread_id, before.thread_id);
        assert_eq!(after.submitted_at, before.submitted_at);
        assert_eq!(after.decided_at, before.decided_at);
        assert_eq!(after.recorded_at, before.recorded_at);

        let apply_row_id = insert_current_apply_audit_row(&conn);
        let apply_row = load_audit_row(&conn, apply_row_id);
        assert_eq!(apply_row.event_type, "apply_audit");
        assert_eq!(apply_row.detail.as_deref(), Some(FIXED_PREV_DETAIL));

        let update_err = conn
            .execute(
                "UPDATE ward_audit SET decision = 'changed' WHERE id = ?1",
                params![row_id],
            )
            .unwrap_err();
        assert!(update_err.to_string().contains("append-only"));

        let delete_err = conn
            .execute("DELETE FROM ward_audit WHERE id = ?1", params![row_id])
            .unwrap_err();
        assert!(delete_err.to_string().contains("append-only"));
    }

    #[test]
    fn post_guard_rejects_durable_drift_and_rollback_restores_exact_legacy_state() {
        let conn = Connection::open_in_memory().unwrap();
        set_user_version(&conn, 37);
        conn.execute_batch(LEGACY_WARD_AUDIT_SCHEMA_SQL).unwrap();
        let row_id = insert_legacy_ward_updated_row(&conn);
        let before = load_legacy_audit_row(&conn, row_id);

        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_LEGACY_V013);

        let drift_sql = migration_sql_with_durable_drift_before_post_guard(
            "CREATE VIEW main.ward_audit_history AS SELECT id, decision FROM main.ward_audit;",
        );
        let err = conn
            .execute_batch(&drift_sql)
            .expect_err("postcondition guard must reject durable drift before commit");
        assert!(
            err.to_string().contains("CHECK constraint failed"),
            "unexpected migration error: {err}"
        );
        assert!(has_column(&conn, "detail"));
        assert!(main_schema_object_exists(
            &conn,
            "view",
            "ward_audit_history"
        ));
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);

        conn.execute_batch("ROLLBACK;").unwrap();

        assert_eq!(load_legacy_audit_row(&conn, row_id), before);
        assert_eq!(user_version(&conn), 37);
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_LEGACY_V013);
        assert!(!has_column(&conn, "detail"));
        assert!(!main_schema_object_exists(&conn, "table", "ward_audit_new"));
        assert!(!main_schema_object_exists(
            &conn,
            "view",
            "ward_audit_history"
        ));
        assert_eq!(
            stored_table_sql(&conn),
            sql_literal_value(&conn, ward_audit_exact_legacy_table_sql_sql!())
        );
        assert_eq!(explicit_index_names(&conn), expected_explicit_index_names());
        assert_eq!(trigger_names(&conn), expected_legacy_trigger_names());
    }

    #[test]
    fn rerunning_migration_after_legacy_upgrade_errors_and_preserves_rows() {
        let conn = Connection::open_in_memory().unwrap();
        set_user_version(&conn, 37);
        conn.execute_batch(LEGACY_WARD_AUDIT_SCHEMA_SQL).unwrap();
        let legacy_row_id = insert_legacy_ward_updated_row(&conn);

        conn.execute_batch(WARD_AUDIT_MIGRATION_V020_SQL).unwrap();
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V020);
        assert_eq!(user_version(&conn), 37);

        let apply_row_id = insert_current_apply_audit_row(&conn);
        let legacy_before = load_audit_row(&conn, legacy_row_id);
        let apply_before = load_audit_row(&conn, apply_row_id);

        let err = conn
            .execute_batch(WARD_AUDIT_MIGRATION_V020_SQL)
            .expect_err("rerunning the migration must fail at the legacy guard");
        assert!(
            err.to_string().contains("CHECK constraint failed"),
            "unexpected migration error: {err}"
        );
        conn.execute_batch("ROLLBACK;").unwrap();

        assert_eq!(load_audit_row(&conn, legacy_row_id), legacy_before);
        assert_eq!(load_audit_row(&conn, apply_row_id), apply_before);
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V020);
        assert_eq!(user_version(&conn), 37);
    }

    #[test]
    fn concurrent_schema_initialization_serializes_without_locked_errors() {
        for attempt in 0..CONCURRENT_DB_RUNS {
            let path = {
                let db = ScratchDbPath::new("concurrent-schema-init");
                let path = db.path().to_path_buf();

                {
                    let setup = open_file_backed_connection(db.path());
                    set_user_version(&setup, 41);
                }

                let outcomes = run_concurrent_sql(db.path(), WARD_AUDIT_SCHEMA_SQL);
                assert!(
                    outcomes.iter().all(|result| result.is_ok()),
                    "attempt {attempt} concurrent init outcomes: {outcomes:?}"
                );

                {
                    let final_conn = open_file_backed_connection(db.path());
                    assert_eq!(user_version(&final_conn), 41);
                    assert_schema_state(&final_conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V020);
                    assert_eq!(
                        explicit_index_names(&final_conn),
                        expected_explicit_index_names()
                    );
                    assert_eq!(trigger_names(&final_conn), expected_trigger_names());
                }

                path
            };

            assert_sqlite_artifacts_absent(&path);
        }
    }

    #[test]
    fn concurrent_legacy_migration_waits_then_rejects_current_at_guard() {
        for attempt in 0..CONCURRENT_DB_RUNS {
            let path = {
                let db = ScratchDbPath::new("concurrent-legacy-migration");
                let path = db.path().to_path_buf();

                let (row_id, before) = {
                    let setup = open_file_backed_connection(db.path());
                    set_user_version(&setup, 37);
                    setup.execute_batch(LEGACY_WARD_AUDIT_SCHEMA_SQL).unwrap();
                    let row_id = insert_legacy_ward_updated_row(&setup);
                    let before = load_legacy_audit_row(&setup, row_id);
                    (row_id, before)
                };

                let outcomes = run_concurrent_sql(db.path(), WARD_AUDIT_MIGRATION_V020_SQL);
                let success_count = outcomes.iter().filter(|result| result.is_ok()).count();
                let errors: Vec<_> = outcomes
                    .iter()
                    .filter_map(|result| result.as_ref().err())
                    .collect();
                assert_eq!(
                    success_count, 1,
                    "attempt {attempt} concurrent migration outcomes: {outcomes:?}"
                );
                assert_eq!(
                    errors.len(),
                    1,
                    "attempt {attempt} concurrent migration outcomes: {outcomes:?}"
                );
                let error = errors[0].to_lowercase();
                assert!(
                    error.contains("check constraint failed"),
                    "attempt {attempt} unexpected migration error: {}",
                    errors[0]
                );
                assert!(
                    !error.contains("locked"),
                    "attempt {attempt} migration must wait then fail at the guard, not lock: {}",
                    errors[0]
                );

                {
                    let final_conn = open_file_backed_connection(db.path());
                    assert_eq!(user_version(&final_conn), 37);
                    assert_schema_state(&final_conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V020);
                    assert_eq!(ward_audit_row_count(&final_conn, "main"), 1);
                    assert_eq!(load_audit_row(&final_conn, row_id), before);
                    assert_eq!(
                        explicit_index_names(&final_conn),
                        expected_explicit_index_names()
                    );
                    assert_eq!(trigger_names(&final_conn), expected_trigger_names());
                }

                path
            };

            assert_sqlite_artifacts_absent(&path);
        }
    }

    #[test]
    fn unqualified_insert_targets_temp_shadow_while_schema_state_stays_unknown() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(WARD_AUDIT_SCHEMA_SQL).unwrap();
        let main_row_id = insert_current_apply_audit_row(&conn);
        let main_before = load_audit_row(&conn, main_row_id);

        create_temp_shadow_table(&conn, ward_audit_current_objects_sql!());

        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);
        assert_eq!(ward_audit_row_count(&conn, "main"), 1);
        assert_eq!(ward_audit_row_count(&conn, "temp"), 0);

        let temp_row_id = insert_current_apply_audit_row_unqualified(&conn);

        assert_eq!(load_audit_row(&conn, main_row_id), main_before);
        assert_eq!(ward_audit_row_count(&conn, "main"), 1);
        assert_eq!(ward_audit_row_count(&conn, "temp"), 1);
        assert_eq!(
            load_audit_row_from_schema(&conn, "temp", temp_row_id).event_type,
            "apply_audit"
        );
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);
    }

    #[test]
    fn sqlite_post_alter_failure_rollback_restores_legacy_schema_for_production_migration_contract()
    {
        let conn = Connection::open_in_memory().unwrap();
        set_user_version(&conn, 37);
        conn.execute_batch(LEGACY_WARD_AUDIT_SCHEMA_SQL).unwrap();
        let row_id = insert_legacy_ward_updated_row(&conn);
        let before = load_legacy_audit_row(&conn, row_id);

        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_LEGACY_V013);

        // Validate SQLite rollback semantics for the production migration's
        // post-ALTER failure contract without relying on a preexisting reserved
        // durable namespace object.
        let err = conn
            .execute_batch(
                r#"
                BEGIN;
                ALTER TABLE main.ward_audit ADD COLUMN detail TEXT;
                CREATE TABLE main.ward_audit_new (conflict TEXT NOT NULL);
                CREATE TABLE main.ward_audit_new (conflict TEXT NOT NULL);
                "#,
            )
            .expect_err("controlled post-ALTER SQL failure must abort the transaction");
        assert!(
            err.to_string()
                .contains("table ward_audit_new already exists"),
            "unexpected migration error: {err}"
        );
        conn.execute_batch("ROLLBACK;").unwrap();

        assert_eq!(load_legacy_audit_row(&conn, row_id), before);
        assert_eq!(user_version(&conn), 37);
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_LEGACY_V013);
        assert!(!has_column(&conn, "detail"));
        assert!(!main_schema_object_exists(&conn, "table", "ward_audit_new"));
        assert_eq!(
            stored_table_sql(&conn),
            sql_literal_value(&conn, ward_audit_exact_legacy_table_sql_sql!())
        );
        assert_eq!(explicit_index_names(&conn), expected_explicit_index_names());
        assert_eq!(trigger_names(&conn), expected_legacy_trigger_names());
    }
}

