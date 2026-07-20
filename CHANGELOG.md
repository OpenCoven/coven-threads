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
- `WARD_AUDIT_MIGRATION_V014_SQL` — transaction SQL for a detail-preserving
  rebuild of `ward_audit` after exact table-shape inspection. The daemon should
  run it only for the legacy v0.1.3 shape (`detail` absent and the table CHECK
  does not admit `apply_audit`); current schemas skip it, and partial/unknown
  shapes fail closed. The rebuild first adds `detail` to the legacy table, then
  copies rows into the replacement table without discarding evidence.
- Exhaustiveness test `schema_names_all_event_tags` extended to cover `ApplyAudit`.
- New tests: `for_apply_produces_correct_shape`,
  `for_apply_roundtrips_json`,
  `fresh_schema_preserves_user_version_zero_and_creates_current_shape`,
  `fresh_schema_preserves_user_version_ninety_nine_and_creates_current_shape`,
  `migration_rejects_current_schema_rows_with_detail_and_preserves_state`,
  `legacy_schema_upgrades_and_preserves_append_only_behavior`, and
  `rerunning_migration_after_legacy_upgrade_errors_and_preserves_rows`.

### Design notes

- **Column approach (Option A):** `diff_hash` carries `next_sha256`; `prev_sha256` + `bytes_written` ride in the new `detail` TEXT column as JSON. This avoids a second table rebuild by not adding typed hash columns while keeping the data query-accessible via SQLite JSON functions.
- **Migration approach (Option B-lite):** exports
  `WARD_AUDIT_MIGRATION_V014_SQL` so the daemon can perform a quiet guarded
  rebuild on upgrade after exact `ward_audit` shape inspection; production
  dependencies stay unchanged, while `coven-threads-core` now carries bundled
  `rusqlite` as a dev-dependency for executable migration tests.

## [0.1.3] — prior release

Initial audit module with seven `AuditEventType` variants (`proposal_submitted`, `proposal_approved`, `proposal_rejected`, `proposal_vetoed`, `ward_updated`, `validation_verdict`, `compaction_ledger`).
