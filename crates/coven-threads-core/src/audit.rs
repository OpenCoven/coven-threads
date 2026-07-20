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
//! `WARD_AUDIT_SCHEMA_SQL` creates the fresh v0.1.4 shape and stamps
//! `PRAGMA user_version = 14`, so new empty stores are versioned immediately.
//! `WARD_AUDIT_MIGRATION_V014_SQL` exists only for the exact v0.1.3 legacy
//! shape (no `detail` column, no `apply_audit` CHECK tag). It first adds
//! `detail`, then rebuilds `ward_audit` in one transaction so the current
//! CHECK set is installed and any existing `detail` values are preserved.
//! Running that migration against a current schema, or rerunning it after a
//! successful upgrade, fails before destructive work; callers must explicitly
//! roll back the failed transaction before continuing.
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

/// DDL for the `ward.audit` table inside `coven.sqlite3` (§3.4).
///
/// Append-only is enforced *in the store*: UPDATE and DELETE abort via
/// triggers (RFC-0001 §5.6: entries MUST NOT be deleted or modified).
/// DDL migration for v0.1.3 → v0.1.4: rebuilds `ward_audit` from the exact
/// legacy shape so its CHECK constraint includes `apply_audit` and its copy
/// path preserves `detail`.
///
/// SQLite cannot `ALTER` a CHECK constraint on an existing table. This SQL
/// performs a safe swap in one transaction:
/// 1. Adds the legacy `detail` column so the old table matches the copy shape.
/// 2. Creates `ward_audit_new` with the updated CHECK.
/// 3. Copies all existing rows, preserving `detail`.
/// 4. Drops the old table.
/// 5. Renames the new table into place.
/// 6. Re-creates indexes and append-only triggers.
///
/// **Run condition:** execute this only for the exact v0.1.3 legacy shape.
/// Fresh v0.1.4 schemas already stamp `PRAGMA user_version = 14`; legacy
/// v0.1.3 stores start at `0` and are stamped `14` only after this rebuild.
///
/// Current-schema invocation and post-upgrade reruns fail before any
/// destructive step: a current schema aborts on `ALTER TABLE ... ADD COLUMN
/// detail`, while a rerun after a successful upgrade aborts on
/// `CREATE TABLE ward_audit_new`. Callers must `ROLLBACK` failed migration
/// transactions before continuing.
pub const WARD_AUDIT_MIGRATION_V014_SQL: &str = r#"
BEGIN;

ALTER TABLE ward_audit ADD COLUMN detail TEXT;

CREATE TABLE ward_audit_new (
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

INSERT INTO ward_audit_new (
    id, event_type, proposal_id, familiar_id, ward_version, ward_hash,
    tier, decision, approver, diff_hash, detail, files_touched, channel,
    thread_id, submitted_at, decided_at, recorded_at
)
SELECT
    id, event_type, proposal_id, familiar_id, ward_version, ward_hash,
    tier, decision, approver, diff_hash, detail, files_touched, channel,
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

PRAGMA user_version = 14;

COMMIT;
"#;

/// DDL for the `ward.audit` table inside `coven.sqlite3` (§3.4).
///
/// See module docs for migration notes. Fresh schemas stamp
/// `PRAGMA user_version = 14`.
pub const WARD_AUDIT_SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS ward_audit (
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
    detail        TEXT,             -- event-type-specific JSON; see module docs
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

PRAGMA user_version = 14;
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{SurfaceId, WriterId};
    use rusqlite::{params, Connection};

    const FIXED_SUBMITTED_AT: &str = "2026-07-19T00:00:00.000Z";
    const FIXED_DECIDED_AT: &str = "2026-07-19T00:01:00.000Z";
    const FIXED_RECORDED_AT: &str = "2026-07-19T00:02:00.000Z";
    const FIXED_WARD_HASH: [u8; 32] = *b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const FIXED_DIFF_HASH: [u8; 32] = *b"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const FIXED_PREV_DETAIL: &str = r#"{"prev_sha256":"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc","bytes_written":42}"#;
    const FIXED_FILES_TOUCHED: &str = r#"["SOUL.md"]"#;

    /// Representative v0.1.3 `ward_audit` schema: no `detail` column and no
    /// `apply_audit` event tag, but still append-only.
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
    files_touched TEXT    NOT NULL,
    channel       TEXT,
    thread_id     TEXT,
    submitted_at  TEXT    NOT NULL,
    decided_at    TEXT    NOT NULL,
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

    fn open_conn(schema_sql: &str) -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(schema_sql).unwrap();
        conn
    }

    fn user_version(conn: &Connection) -> i64 {
        conn.query_row("PRAGMA user_version;", [], |row| row.get(0))
            .unwrap()
    }

    fn load_audit_row(conn: &Connection, id: i64) -> StoredAuditRow {
        conn.query_row(
            r#"
            SELECT id, event_type, proposal_id, familiar_id, ward_version, ward_hash,
                   tier, decision, approver, diff_hash, detail, files_touched,
                   channel, thread_id, submitted_at, decided_at, recorded_at
            FROM ward_audit
            WHERE id = ?1
            "#,
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

    fn load_legacy_audit_row(conn: &Connection, id: i64) -> StoredAuditRow {
        conn.query_row(
            r#"
            SELECT id, event_type, proposal_id, familiar_id, ward_version, ward_hash,
                   tier, decision, approver, diff_hash, files_touched, channel,
                   thread_id, submitted_at, decided_at, recorded_at
            FROM ward_audit
            WHERE id = ?1
            "#,
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

    fn insert_current_apply_audit_row(conn: &Connection) -> i64 {
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

    fn insert_legacy_ward_updated_row(conn: &Connection) -> i64 {
        conn.execute(
            r#"
            INSERT INTO ward_audit (
                event_type, proposal_id, familiar_id, ward_version, ward_hash,
                tier, decision, approver, diff_hash, files_touched, channel,
                thread_id, submitted_at, decided_at, recorded_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5,
                ?6, ?7, ?8, ?9, ?10, ?11,
                ?12, ?13, ?14, ?15
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
    fn fresh_schema_stamps_user_version_14() {
        let conn = open_conn(WARD_AUDIT_SCHEMA_SQL);
        assert_eq!(user_version(&conn), 14);
    }

    #[test]
    fn migration_rejects_current_schema_rows_with_detail() {
        let conn = open_conn(WARD_AUDIT_SCHEMA_SQL);
        let row_id = insert_current_apply_audit_row(&conn);
        let before = load_audit_row(&conn, row_id);
        let before_version = user_version(&conn);
        assert_eq!(before_version, 14);

        let err = conn
            .execute_batch(WARD_AUDIT_MIGRATION_V014_SQL)
            .expect_err("current-schema migration must fail on duplicate detail");
        assert!(
            err.to_string().contains("duplicate column name: detail"),
            "unexpected migration error: {err}"
        );
        let _ = conn.execute_batch("ROLLBACK;");

        let after = load_audit_row(&conn, row_id);
        assert_eq!(after, before);
        assert_eq!(user_version(&conn), before_version);
    }

    #[test]
    fn legacy_schema_upgrades_and_preserves_append_only_behavior() {
        let conn = open_conn(LEGACY_WARD_AUDIT_SCHEMA_SQL);
        let row_id = insert_legacy_ward_updated_row(&conn);
        let before = load_legacy_audit_row(&conn, row_id);
        assert_eq!(before.event_type, "ward_updated");
        assert_eq!(before.decision, "updated");
        assert_eq!(user_version(&conn), 0);

        conn.execute_batch(WARD_AUDIT_MIGRATION_V014_SQL).unwrap();
        assert_eq!(user_version(&conn), 14);

        let after = load_audit_row(&conn, row_id);
        assert_eq!(after.id, row_id);
        assert_eq!(after.event_type, "ward_updated");
        assert_eq!(after.proposal_id.as_deref(), Some("proposal-legacy"));
        assert_eq!(after.familiar_id, "familiar-legacy");
        assert_eq!(after.ward_version.as_deref(), Some("0.1.3"));
        assert_eq!(after.ward_hash, FIXED_WARD_HASH.to_vec());
        assert_eq!(after.tier.as_deref(), Some("tier_1"));
        assert_eq!(after.decision, "updated");
        assert_eq!(after.approver.as_deref(), Some("writer:legacy"));
        assert_eq!(after.diff_hash, Some(FIXED_DIFF_HASH.to_vec()));
        assert_eq!(after.detail, None);
        assert_eq!(after.files_touched, FIXED_FILES_TOUCHED);
        assert_eq!(after.channel.as_deref(), Some("mutation"));
        assert_eq!(after.thread_id.as_deref(), Some("thread-legacy"));
        assert_eq!(after.submitted_at, FIXED_SUBMITTED_AT);
        assert_eq!(after.decided_at, FIXED_DECIDED_AT);
        assert_eq!(after.recorded_at, FIXED_RECORDED_AT);

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
        let conn = open_conn(LEGACY_WARD_AUDIT_SCHEMA_SQL);
        let legacy_row_id = insert_legacy_ward_updated_row(&conn);

        conn.execute_batch(WARD_AUDIT_MIGRATION_V014_SQL).unwrap();
        assert_eq!(user_version(&conn), 14);

        let apply_row_id = insert_current_apply_audit_row(&conn);
        let legacy_before = load_audit_row(&conn, legacy_row_id);
        let apply_before = load_audit_row(&conn, apply_row_id);

        let err = conn
            .execute_batch(WARD_AUDIT_MIGRATION_V014_SQL)
            .expect_err("rerunning the migration must fail on duplicate detail");
        assert!(
            err.to_string().contains("duplicate column name: detail"),
            "unexpected migration error: {err}"
        );
        let _ = conn.execute_batch("ROLLBACK;");

        let legacy_after = load_audit_row(&conn, legacy_row_id);
        let apply_after = load_audit_row(&conn, apply_row_id);
        assert_eq!(legacy_after, legacy_before);
        assert_eq!(apply_after, apply_before);
        assert_eq!(user_version(&conn), 14);
    }
}
