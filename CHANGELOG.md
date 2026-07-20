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
- `WARD_AUDIT_SCHEMA_STATE_SQL` plus stable state tags (`missing`,
  `legacy_v013`, `current_v014`, `unknown`) — reusable table-local schema
  fingerprint contract for daemon callers, using exact stored
  `main.sqlite_master.sql` fingerprints for the durable `main.ward_audit`
  table, explicit main indexes, and append-only main triggers, plus ordered
  durable column metadata discovered through schema-qualified PRAGMAs.
  `missing` now means `main.ward_audit` is absent, the reserved main-schema
  `ward_audit` / `ward_audit_*` namespace is otherwise empty, **and** no temp
  shadow/reserved temp object exists; any temp-schema table/view/index/trigger
  named `ward_audit` or `ward_audit_*` fail-closes as `unknown`. Only the
  controlled fresh/migrated v0.1.4 table SQL variants and the shipped v0.1.3
  table SQL are accepted; no whitespace-destroying normalization is applied.
- `WARD_AUDIT_SCHEMA_SQL` — atomic, self-guarding schema initialization for the
  exact `current_v014` fingerprint. It begins a transaction, permits only
  `missing` or exact `current_v014` before any mutation, uses idempotent `IF NOT
  EXISTS` DDL for daemon compatibility, targets durable schema objects in
  `main` wherever SQLite syntax permits, then requires exact `current_v014`
  before `COMMIT`. Exact `legacy_v013`, every drifted `unknown` shape, and any
  temp shadow/reserved temp object fail closed; callers must `ROLLBACK` after
  any init error.
- `WARD_AUDIT_MIGRATION_V014_SQL` — transaction SQL for a detail-preserving
  rebuild of `main.ward_audit` guarded by the exact `legacy_v013` fingerprint
  plus temp-shadow rejection. The daemon should query
  `WARD_AUDIT_SCHEMA_STATE_SQL`, initialize when missing, migrate only
  `legacy_v013`, continue on `current_v014`, and fail closed on `unknown`. The
  migration independently re-checks `legacy_v013` before any `ALTER`, using the
  same exact stored-table + column/index/trigger predicate plus temp-shadow
  rejection, then copies rows into the replacement table without discarding
  evidence.
- Exhaustiveness test `schema_names_all_event_tags` extended to cover `ApplyAudit`.
- New tests: `for_apply_produces_correct_shape`,
  `for_apply_roundtrips_json`,
  `schema_state_query_returns_missing_on_empty_db`,
  `fresh_schema_sql_initializes_current_schema_atomically_and_enforces_append_only`,
  `exact_legacy_fixture_returns_legacy_v013`,
  `exact_current_schema_returns_current_v014`,
  `current_schema_sql_reruns_idempotently_and_preserves_rows_and_objects`,
  `fresh_and_migrated_current_schemas_use_controlled_exact_sql_variants`,
  `current_schema_with_spaced_event_type_literal_is_unknown`,
  `legacy_schema_with_spaced_event_type_literal_is_unknown_and_guard_preserves_state`,
  `legacy_schema_sql_rejects_and_rollback_preserves_state`,
  `fresh_schema_preserves_user_version_zero_and_creates_current_shape`,
  `fresh_schema_preserves_user_version_ninety_nine_and_creates_current_shape`,
  `legacy_plus_extra_column_and_data_is_unknown_and_guard_preserves_state`,
  `legacy_schema_with_extra_table_check_is_unknown_and_guard_preserves_constraint`,
  `legacy_schema_with_extra_unique_is_unknown_and_guard_preserves_constraint`,
  `current_schema_with_extra_table_check_is_unknown`,
  `current_schema_with_extra_unique_is_unknown`,
  `current_schema_missing_append_only_trigger_is_unknown_and_update_succeeds`,
  `current_schema_with_extra_index_is_unknown`,
  `current_schema_with_desc_index_is_unknown`,
  `current_schema_with_collated_index_is_unknown`,
  `current_schema_with_altered_trigger_error_literal_is_unknown`,
  `current_schema_with_altered_trigger_body_is_unknown`,
  `absent_ward_audit_with_reserved_index_collision_is_unknown_and_schema_sql_preserves_other_objects`,
  `absent_ward_audit_with_reserved_trigger_collision_is_unknown_and_schema_sql_preserves_other_objects`,
  `unknown_partial_current_schema_rejects_schema_sql_and_preserves_state`,
  `migration_rejects_current_schema_rows_with_detail_and_preserves_state`,
  `legacy_schema_upgrades_and_preserves_append_only_behavior`, and
  `rerunning_migration_after_legacy_upgrade_errors_and_preserves_rows`, and
  `post_alter_failure_rollback_restores_legacy_schema_after_create_conflict`, and
  `schema_qualified_table_valued_pragmas_resolve_main_and_temp_separately`,
  `current_main_with_temp_shadow_is_unknown_and_guards_preserve_main_and_temp_rows`,
  `missing_main_with_temp_ward_audit_shadow_is_unknown_and_schema_sql_rejects_without_creating_main`,
  `missing_main_with_reserved_temp_objects_is_unknown_and_schema_sql_preserves_temp_objects`,
  `legacy_main_with_temp_shadow_is_unknown_and_migration_rejects_before_mutating_either_schema`, and
  `unqualified_insert_targets_temp_shadow_while_schema_state_stays_unknown`.

### Design notes

- **Column approach (Option A):** `diff_hash` carries `next_sha256`; `prev_sha256` + `bytes_written` ride in the new `detail` TEXT column as JSON. This avoids a second table rebuild by not adding typed hash columns while keeping the data query-accessible via SQLite JSON functions.
- **Migration approach (Option B-lite):** exports
  `WARD_AUDIT_SCHEMA_STATE_SQL`, `WARD_AUDIT_SCHEMA_SQL`, and
  `WARD_AUDIT_MIGRATION_V014_SQL` so the daemon can classify `missing` /
  `legacy_v013` / `current_v014` / `unknown`, perform a quiet fail-closed init
  or upgrade, and recover cleanly with explicit rollback after any init or
  migration error. The durable contract is explicitly `main.ward_audit`, and
  any TEMP shadow/reserved temp object keeps the contract fail-closed as
  `unknown`. Production dependencies stay unchanged, while `coven-threads-core`
  now carries bundled `rusqlite` as a dev-dependency for executable migration
  tests.

## [0.1.3] — prior release

Initial audit module with seven `AuditEventType` variants (`proposal_submitted`, `proposal_approved`, `proposal_rejected`, `proposal_vetoed`, `ward_updated`, `validation_verdict`, `compaction_ledger`).
