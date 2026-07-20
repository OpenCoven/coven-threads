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
- `WARD_AUDIT_MIGRATION_V014_SQL` — transaction SQL to rebuild `ward_audit` for stores created against v0.1.3 (which lacked the `apply_audit` CHECK entry and the `detail` column). Future daemon integration must gate this SQL by exact `ward_audit` table shape: legacy means the table exists, `detail` is absent, and the CHECK does not contain `apply_audit`; current means `detail` exists and `apply_audit` is accepted. The SQL preserves database-wide `PRAGMA user_version`.
- Exhaustiveness test `schema_names_all_event_tags` extended to cover `ApplyAudit`.
- New tests: `for_apply_produces_correct_shape`, `for_apply_roundtrips_json`, `fresh_schema_preserves_user_version_zero_and_creates_current_shape`, `fresh_schema_preserves_user_version_ninety_nine_and_creates_current_shape`, `migration_rejects_current_schema_rows_with_detail_and_preserves_state`, `legacy_schema_upgrades_and_preserves_append_only_behavior`, and `rerunning_migration_after_legacy_upgrade_errors_and_preserves_rows`.

### Design notes

- **Column approach (Option A):** `diff_hash` carries `next_sha256`; `prev_sha256` + `bytes_written` ride in the new `detail` TEXT column as JSON. This avoids a second table rebuild by not adding typed hash columns while keeping the data query-accessible via SQLite JSON functions.
- **Migration approach (Option B-lite):** exports `WARD_AUDIT_MIGRATION_V014_SQL` so the daemon can perform a quiet guarded rebuild on upgrade; does not add a rusqlite dependency to coven-threads-core.

## [0.1.3] — prior release

Initial audit module with seven `AuditEventType` variants (`proposal_submitted`, `proposal_approved`, `proposal_rejected`, `proposal_vetoed`, `ward_updated`, `validation_verdict`, `compaction_ledger`).
