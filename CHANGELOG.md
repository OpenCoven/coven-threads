# Changelog

All notable changes to `coven-threads-core` are documented here.

## [0.1.4] — unreleased

### Added

- `AuditEventType::ApplyAudit` variant + `"apply_audit"` tag in `WARD_AUDIT_SCHEMA_SQL`'s CHECK constraint (coven-threads#5).
  - Enables the coven daemon to persist Gate-4 Tier-2 applied-write records into `ward.audit` (downstream: coven#414).
- `WardAuditRecord::detail` field — nullable JSON column for event-type-specific payload.
  - For `ApplyAudit` events: `{"prev_sha256":"<hex>","bytes_written":N}`.
- `WardAuditRecord::for_apply(...)` constructor — builds the audit row for a Gate-4 applied write.
- `WardAuditRecord::apply_prev_sha256_hex()` / `apply_bytes_written()` — decode helpers for `ApplyAudit` detail.
- `APPLY_AUDIT_DETAIL_KEY_PREV` / `APPLY_AUDIT_DETAIL_KEY_BYTES` — stable JSON key constants.
- `WARD_AUDIT_MIGRATION_V014_SQL` — transaction SQL to rebuild `ward_audit` for stores created against v0.1.3 (which lacked the `apply_audit` CHECK entry and the `detail` column). Bumps `PRAGMA user_version = 14` inside the transaction so future migrations can gate on `user_version < N` instead of substring-sniffing `sqlite_master` DDL. The daemon should run this migration when `PRAGMA user_version < 14`.
- Exhaustiveness test `schema_names_all_event_tags` extended to cover `ApplyAudit`.
- New tests: `for_apply_produces_correct_shape`, `for_apply_roundtrips_json`, `migration_sql_contains_apply_audit_and_detail_column`.

### Design notes

- **Column approach (Option A):** `diff_hash` carries `next_sha256`; `prev_sha256` + `bytes_written` ride in the new `detail` TEXT column as JSON. This avoids a second table rebuild by not adding typed hash columns while keeping the data query-accessible via SQLite JSON functions.
- **Migration approach (Option B-lite):** exports `WARD_AUDIT_MIGRATION_V014_SQL` so the daemon can perform a quiet guarded rebuild on upgrade; does not add a rusqlite dependency to coven-threads-core.

## [0.1.3] — prior release

Initial audit module with seven `AuditEventType` variants (`proposal_submitted`, `proposal_approved`, `proposal_rejected`, `proposal_vetoed`, `ward_updated`, `validation_verdict`, `compaction_ledger`).
