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
//! ## Schema shape and migration gating
//!
//! `WARD_AUDIT_SCHEMA_SQL` initializes or verifies the exact v0.1.4
//! `main.ward_audit` shape in one transaction, without mutating database-wide
//! `PRAGMA user_version`. It is safe for the daemon to execute unconditionally
//! on every store open: a pre-install guard permits only `missing` or exact
//! `current_v014`, the durable DDL runs explicitly in `main` with `IF NOT
//! EXISTS` compatibility, and a post-install guard requires exact
//! `current_v014` before `COMMIT`. If either guard fails, callers must
//! explicitly `ROLLBACK` before continuing.
//! `WARD_AUDIT_SCHEMA_STATE_SQL` is the reusable, table-local fingerprint query
//! that returns one of four stable tags:
//! - `missing` — `main.ward_audit` does not exist, the reserved durable
//!   `ward_audit` / `ward_audit_*` namespace is otherwise empty in `main`, and
//!   no temp-schema shadow/reserved object exists; initialize with
//!   `WARD_AUDIT_SCHEMA_SQL`;
//! - `legacy_v013` — `main.ward_audit` exactly matches the v0.1.3 legacy
//!   fingerprint and no temp shadow exists; run `WARD_AUDIT_MIGRATION_V014_SQL`;
//! - `current_v014` — `main.ward_audit` exactly matches the v0.1.4 current
//!   fingerprint and no temp shadow exists; continue without schema work;
//! - `unknown` — every other shape, including any extra or missing declared
//!   table constraint, column, index, or trigger, any reserved durable-name
//!   collision, and any temp-schema table/view/index/trigger named
//!   `ward_audit` or `ward_audit_*`; fail closed.
//!
//! The fingerprint uses exact `main.sqlite_master.sql` text for the durable
//! table, explicit durable indexes, and append-only durable triggers, plus
//! ordered `pragma_table_info('ward_audit', 'main')` metadata and
//! `pragma_index_list('ward_audit', 'main')` for explicit index discovery. It
//! does **not** normalize whitespace: the only accepted `current_v014`
//! table-SQL variants are the fresh `CREATE TABLE ward_audit` form and the
//! quoted `CREATE TABLE "ward_audit"` form SQLite stores after the exact legacy
//! migration path, and the `legacy_v013` fingerprint includes the inline
//! comments preserved from the shipped v0.1.3 DDL.
//! `WARD_AUDIT_MIGRATION_V014_SQL` exists only for the exact `legacy_v013`
//! fingerprint with no temp shadow. It independently re-checks that fingerprint
//! inside the transaction before any `ALTER`, then adds `detail` and rebuilds
//! `main.ward_audit` so the current CHECK set is installed and every existing
//! row is preserved. If a later migration step fails after
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

use crate::channel::Channel;
use crate::ids::{FamiliarId, ProposalId, SurfaceId, ThreadId, WriterId};
use crate::validate::{MutationRequest, RejectReason, Verdict};

/// Stable JSON key for the `detail` field of an `apply_audit` row;
pub const APPLY_AUDIT_DETAIL_KEY_PREV: &str = "prev_sha256";
/// Stable JSON key for the bytes-written count in an `apply_audit` detail.
pub const APPLY_AUDIT_DETAIL_KEY_BYTES: &str = "bytes_written";

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
    /// A proposal was approved (RFC-0001 §5.6).
    ProposalApproved,
    /// A proposal was rejected (RFC-0001 §5.6).
    ProposalRejected,
    /// A proposal was vetoed inside its veto window (RFC-0001 §5.6).
    ProposalVetoed,
    /// The Ward itself was updated (RFC-0001 §5.6).
    WardUpdated,
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
            AuditEventType::ProposalApproved => "proposal_approved",
            AuditEventType::ProposalRejected => "proposal_rejected",
            AuditEventType::ProposalVetoed => "proposal_vetoed",
            AuditEventType::WardUpdated => "ward_updated",
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
    ///   pre-write SHA-256 of the surface content, or `""` when unknown.
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
            APPLY_AUDIT_DETAIL_KEY_PREV: prev_hex.unwrap_or_default(),
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

    /// Decode `prev_sha256` from the `detail` JSON for `ApplyAudit` rows.
    ///
    /// Returns `None` for non-`ApplyAudit` events or if the field is missing.
    pub fn apply_prev_sha256_hex(&self) -> Option<String> {
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
        let detail = self.detail.as_deref()?;
        let v: serde_json::Value = serde_json::from_str(detail).ok()?;
        v.get(APPLY_AUDIT_DETAIL_KEY_BYTES).and_then(|x| x.as_u64())
    }
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
/// is absent, the reserved main-schema `ward_audit` / `ward_audit_*` namespace
/// is otherwise empty, and no temp shadow/reserved object blocks the durable
/// contract.
pub const WARD_AUDIT_SCHEMA_STATE_MISSING: &str = "missing";
/// Stable tag returned by [`WARD_AUDIT_SCHEMA_STATE_SQL`] for the exact
/// `main.ward_audit` v0.1.3 legacy schema fingerprint, with no temp shadow.
pub const WARD_AUDIT_SCHEMA_STATE_LEGACY_V013: &str = "legacy_v013";
/// Stable tag returned by [`WARD_AUDIT_SCHEMA_STATE_SQL`] for the exact
/// `main.ward_audit` v0.1.4 current schema fingerprint, with no temp shadow.
pub const WARD_AUDIT_SCHEMA_STATE_CURRENT_V014: &str = "current_v014";
/// Stable tag returned by [`WARD_AUDIT_SCHEMA_STATE_SQL`] for every other
/// `main.ward_audit` shape, plus reserved-name collisions in the durable
/// namespace or any temp-schema `ward_audit` / `ward_audit_*` shadow object.
pub const WARD_AUDIT_SCHEMA_STATE_UNKNOWN: &str = "unknown";

macro_rules! ward_audit_reserved_name_predicate_sql {
    () => {
        r#"(lower(name) = 'ward_audit' OR lower(name) GLOB 'ward_audit_*')"#
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
    ward_audit_reserved_namespace_object_count AS (
        SELECT COUNT(*) AS count
        FROM main.sqlite_master
        WHERE type IN ('table', 'index', 'trigger', 'view')
          AND "#,
            ward_audit_reserved_name_predicate_sql!(),
            r#"
          AND NOT (type = 'table' AND name = 'ward_audit')
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
            (SELECT count FROM ward_audit_reserved_namespace_object_count) AS reserved_namespace_count,
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
                      ''proposal_submitted'',''proposal_approved'',''proposal_rejected'',
                      ''proposal_vetoed'',''ward_updated'',''validation_verdict'',
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
                      ''proposal_submitted'',''proposal_approved'',''proposal_rejected'',
                      ''proposal_vetoed'',''ward_updated'',''validation_verdict'',
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
END'"#
    };
}

macro_rules! ward_audit_exact_legacy_predicate_sql {
    () => {
        concat!(
            r#"
table_exists = 1
AND temp_shadow_count = 0
AND table_sql = "#,
            ward_audit_exact_legacy_table_sql_sql!(),
            r#"
AND column_fp = '0|id|INTEGER|0|<null>|1||1|event_type|TEXT|1|<null>|0||2|proposal_id|TEXT|0|<null>|0||3|familiar_id|TEXT|1|<null>|0||4|ward_version|TEXT|0|<null>|0||5|ward_hash|BLOB|1|<null>|0||6|tier|TEXT|0|<null>|0||7|decision|TEXT|1|<null>|0||8|approver|TEXT|0|<null>|0||9|diff_hash|BLOB|0|<null>|0||10|files_touched|TEXT|1|<null>|0||11|channel|TEXT|0|<null>|0||12|thread_id|TEXT|0|<null>|0||13|submitted_at|TEXT|1|<null>|0||14|decided_at|TEXT|1|<null>|0||15|recorded_at|TEXT|1|strftime(''%Y-%m-%dT%H:%M:%fZ'',''now'')|0'
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

macro_rules! ward_audit_exact_current_predicate_sql {
    () => {
        concat!(
            r#"
table_exists = 1
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
    WHEN table_exists = 0 AND reserved_namespace_count = 0 THEN 'missing'
    WHEN "#,
            ward_audit_exact_legacy_predicate_sql!(),
            r#" THEN 'legacy_v013'
    WHEN "#,
            ward_audit_exact_current_predicate_sql!(),
            r#" THEN 'current_v014'
    ELSE 'unknown'
END"#
        )
    };
}

/// Table-local schema-state query for the durable `main.ward_audit` contract
/// inside `coven.sqlite3` (§3.4).
///
/// Callers run this exact query and branch on the stable text result:
/// - [`WARD_AUDIT_SCHEMA_STATE_MISSING`] — `main.ward_audit` is absent, the
///   reserved main-schema `ward_audit` / `ward_audit_*` namespace is otherwise
///   empty, and no temp shadow/reserved object exists; initialize with
///   [`WARD_AUDIT_SCHEMA_SQL`];
/// - [`WARD_AUDIT_SCHEMA_STATE_LEGACY_V013`] — run
///   [`WARD_AUDIT_MIGRATION_V014_SQL`];
/// - [`WARD_AUDIT_SCHEMA_STATE_CURRENT_V014`] — continue without schema work;
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
/// trigger returns `unknown`. Any temp-schema table/view/index/trigger whose
/// name is exactly `ward_audit` or begins with `ward_audit_` also returns
/// `unknown`, even when `main.ward_audit` itself is exact current or legacy, so
/// callers cannot treat a temp shadow as healthy durable state. When
/// `main.ward_audit` itself is absent, any preexisting main-schema
/// table/index/trigger/view whose name is reserved under the same
/// `ward_audit` / `ward_audit_*` namespace likewise returns `unknown`, so
/// initialization cannot silently skip append-only objects on another durable
/// table. No whitespace-destroying normalization is applied: the only accepted
/// `current_v014` table SQL variants are the fresh `CREATE TABLE ward_audit
/// (...)` form and SQLite's quoted `CREATE TABLE "ward_audit" (...)` form
/// produced by the exact legacy migration path, while the `legacy_v013`
/// fingerprint intentionally includes the inline comments preserved from the
/// shipped v0.1.3 DDL.
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
/// 1. re-checks the exact legacy fingerprint in SQL before any mutation,
///    including exact stored `main.sqlite_master.sql` equality plus main
///    column/index/trigger fingerprints and temp-shadow rejection;
/// 2. adds the legacy `detail` column on `main.ward_audit` so the old table
///    matches the copy shape;
/// 3. creates `main.ward_audit_new` with the updated CHECK;
/// 4. copies every existing row, preserving `detail`;
/// 5. swaps the tables; and
/// 6. re-creates the exact explicit main indexes and append-only main triggers.
///
/// Callers should still branch on [`WARD_AUDIT_SCHEMA_STATE_SQL`] first:
/// initialize when the state is `missing`, migrate only `legacy_v013`,
/// continue on `current_v014`, and fail closed on `unknown`. This migration
/// independently guards the same `legacy_v013` fingerprint and temp-shadow
/// rejection so callers cannot mutate a partial, already-current, or
/// temp-shadowed schema by skipping classification. If a later step fails after
/// `ALTER TABLE main.ward_audit ADD COLUMN detail`, callers must `ROLLBACK` the
/// failed transaction before continuing so SQLite restores the untouched legacy
/// table. This SQL does not read or write database-wide `PRAGMA user_version`.
pub const WARD_AUDIT_MIGRATION_V014_SQL: &str = concat!(
    r#"
BEGIN;

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
                      'proposal_submitted','proposal_approved','proposal_rejected',
                      'proposal_vetoed','ward_updated','validation_verdict',
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

COMMIT;
"#,
);

// Shared durable-main current-v0.1.4 DDL body used by the guarded init SQL and
// drift tests.
macro_rules! ward_audit_current_objects_sql {
    () => {
        r#"
CREATE TABLE IF NOT EXISTS main.ward_audit (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type    TEXT    NOT NULL CHECK (event_type IN (
                      'proposal_submitted','proposal_approved','proposal_rejected',
                      'proposal_vetoed','ward_updated','validation_verdict',
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
"#
    };
}

/// DDL for the durable `main.ward_audit` table inside `coven.sqlite3` (§3.4).
///
/// See module docs for the full schema-state contract. This transaction is
/// safe to run unconditionally on every store open: it permits only exact
/// `missing` or `current_v014` before any mutation, uses idempotent `IF NOT
/// EXISTS` DDL for daemon compatibility, then requires exact `current_v014`
/// before `COMMIT`. Exact `legacy_v013`, every drifted `unknown` shape, and
/// every temp shadow/reserved temp object fail closed. Durable schema objects
/// are explicitly created in `main`; the temp guard tables live in `temp`
/// under unique non-reserved names. If this SQL errors, callers must
/// explicitly `ROLLBACK` before continuing so SQLite discards any uncommitted
/// work. This DDL never mutates database-wide `PRAGMA user_version`.
pub const WARD_AUDIT_SCHEMA_SQL: &str = concat!(
    r#"
BEGIN;

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
    r#") IN ('missing', 'current_v014') THEN 1
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
    r#") = 'current_v014' THEN 1
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
    use std::collections::BTreeSet;

    const FIXED_SUBMITTED_AT: &str = "2026-07-19T00:00:00.000Z";
    const FIXED_DECIDED_AT: &str = "2026-07-19T00:01:00.000Z";
    const FIXED_RECORDED_AT: &str = "2026-07-19T00:02:00.000Z";
    const FIXED_WARD_HASH: [u8; 32] = *b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const FIXED_DIFF_HASH: [u8; 32] = *b"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const FIXED_PREV_DETAIL: &str = r#"{"prev_sha256":"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc","bytes_written":42}"#;
    const FIXED_FILES_TOUCHED: &str = r#"["SOUL.md"]"#;

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
    const EXPECTED_TRIGGER_NAMES: &[&str] = &[
        "ward_audit_append_only_delete",
        "ward_audit_append_only_update",
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
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V014);

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
            .execute_batch(WARD_AUDIT_MIGRATION_V014_SQL)
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
        assert_eq!(trigger_names(&conn), expected_trigger_names());

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
                Some("0.1.4"),
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
                Some("0.1.4"),
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

    fn request() -> MutationRequest {
        MutationRequest {
            surface: SurfaceId::new("SOUL.md"),
            writer: WriterId::new("principal:val"),
            channel: Channel::Mutation,
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
            AuditEventType::ProposalApproved,
            AuditEventType::ProposalRejected,
            AuditEventType::ProposalVetoed,
            AuditEventType::WardUpdated,
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
    }

    #[test]
    fn for_apply_produces_correct_shape() {
        let now = OffsetDateTime::now_utc();
        let prev = [0xaa_u8; 32];
        let next = [0xbb_u8; 32];
        let record = WardAuditRecord::for_apply(
            FamiliarId::new(),
            &[0xcc; 32],
            SurfaceId::new("SOUL.md"),
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
        assert_eq!(record.diff_hash, Some(vec![0xbb; 32]));
        assert_eq!(record.files_touched, vec![SurfaceId::new("SOUL.md")]);

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
            .execute_batch(WARD_AUDIT_MIGRATION_V014_SQL)
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
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V014);
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

        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V014);
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
            stored_table_sql(&conn),
            sql_literal_value(&conn, ward_audit_exact_legacy_table_sql_sql!())
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
    fn exact_current_schema_returns_current_v014() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(WARD_AUDIT_SCHEMA_SQL).unwrap();
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V014);
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
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V014);
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
            .execute_batch(WARD_AUDIT_MIGRATION_V014_SQL)
            .unwrap();

        assert_schema_state(&fresh, WARD_AUDIT_SCHEMA_STATE_CURRENT_V014);
        assert_schema_state(&migrated, WARD_AUDIT_SCHEMA_STATE_CURRENT_V014);
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
            .execute_batch(WARD_AUDIT_MIGRATION_V014_SQL)
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
        assert_eq!(trigger_names(&conn), expected_trigger_names());
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
            .execute_batch(WARD_AUDIT_MIGRATION_V014_SQL)
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
        assert_eq!(trigger_names(&conn), expected_trigger_names());
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
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V014);
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
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V014);

        conn.execute_batch("CREATE INDEX ward_audit_decision_idx ON ward_audit (decision);")
            .unwrap();

        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_UNKNOWN);
    }

    #[test]
    fn current_schema_with_desc_index_is_unknown() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(WARD_AUDIT_SCHEMA_SQL).unwrap();
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V014);

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
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V014);

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
    fn current_schema_with_altered_trigger_error_literal_is_unknown() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(WARD_AUDIT_SCHEMA_SQL).unwrap();
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V014);

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
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V014);

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

        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V014);

        let err = conn
            .execute_batch(WARD_AUDIT_MIGRATION_V014_SQL)
            .expect_err("current schema migration must fail at the legacy guard");
        assert!(
            err.to_string().contains("CHECK constraint failed"),
            "unexpected migration error: {err}"
        );
        conn.execute_batch("ROLLBACK;").unwrap();

        assert_eq!(load_audit_row(&conn, row_id), before);
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V014);
        assert_eq!(user_version(&conn), before_version);
    }

    #[test]
    fn legacy_schema_upgrades_and_preserves_append_only_behavior() {
        let conn = Connection::open_in_memory().unwrap();
        set_user_version(&conn, 37);
        conn.execute_batch(LEGACY_WARD_AUDIT_SCHEMA_SQL).unwrap();
        let row_id = insert_legacy_ward_updated_row(&conn);
        let before = load_legacy_audit_row(&conn, row_id);
        assert_eq!(user_version(&conn), 37);
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_LEGACY_V013);

        conn.execute_batch(WARD_AUDIT_MIGRATION_V014_SQL).unwrap();

        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V014);
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
    fn rerunning_migration_after_legacy_upgrade_errors_and_preserves_rows() {
        let conn = Connection::open_in_memory().unwrap();
        set_user_version(&conn, 37);
        conn.execute_batch(LEGACY_WARD_AUDIT_SCHEMA_SQL).unwrap();
        let legacy_row_id = insert_legacy_ward_updated_row(&conn);

        conn.execute_batch(WARD_AUDIT_MIGRATION_V014_SQL).unwrap();
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V014);
        assert_eq!(user_version(&conn), 37);

        let apply_row_id = insert_current_apply_audit_row(&conn);
        let legacy_before = load_audit_row(&conn, legacy_row_id);
        let apply_before = load_audit_row(&conn, apply_row_id);

        let err = conn
            .execute_batch(WARD_AUDIT_MIGRATION_V014_SQL)
            .expect_err("rerunning the migration must fail at the legacy guard");
        assert!(
            err.to_string().contains("CHECK constraint failed"),
            "unexpected migration error: {err}"
        );
        conn.execute_batch("ROLLBACK;").unwrap();

        assert_eq!(load_audit_row(&conn, legacy_row_id), legacy_before);
        assert_eq!(load_audit_row(&conn, apply_row_id), apply_before);
        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_CURRENT_V014);
        assert_eq!(user_version(&conn), 37);
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
    fn post_alter_failure_rollback_restores_legacy_schema_after_create_conflict() {
        let conn = Connection::open_in_memory().unwrap();
        set_user_version(&conn, 37);
        conn.execute_batch(LEGACY_WARD_AUDIT_SCHEMA_SQL).unwrap();
        let row_id = insert_legacy_ward_updated_row(&conn);
        let before = load_legacy_audit_row(&conn, row_id);
        conn.execute_batch(
            "CREATE TABLE ward_audit_new (conflict TEXT NOT NULL);\nINSERT INTO ward_audit_new (conflict) VALUES ('sentinel');",
        )
        .unwrap();

        assert_schema_state(&conn, WARD_AUDIT_SCHEMA_STATE_LEGACY_V013);

        let err = conn
            .execute_batch(WARD_AUDIT_MIGRATION_V014_SQL)
            .expect_err("conflicting replacement table must fail after ALTER TABLE succeeds");
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
        assert_eq!(ward_audit_new_conflict_value(&conn), "sentinel");
    }
}
