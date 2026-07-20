# Changelog

All notable changes to `coven-threads-core` are documented here.

## [0.2.0] — unreleased

### Added

- `AuditEventType::ApplyAudit` variant + `"apply_audit"` tag in `WARD_AUDIT_SCHEMA_SQL`'s CHECK constraint (coven-threads#5).
  - Enables the coven daemon to persist Gate-4 Tier-2 applied-write records into `ward.audit` (downstream: coven#414).
- `WardAuditRecord::detail` field — nullable JSON column for event-type-specific payload.
  - For `ApplyAudit` events: `{"prev_sha256":"<hex>","bytes_written":N}`.
- `WardAuditRecord::for_apply(...)` constructor — builds the audit row for a Gate-4 applied write.
- `WardAuditRecord::apply_prev_sha256_hex()` / `apply_bytes_written()` — decode helpers for `ApplyAudit` detail.
- `APPLY_AUDIT_DETAIL_KEY_PREV` / `APPLY_AUDIT_DETAIL_KEY_BYTES` — stable JSON key constants.
- Phase 5 approval paths, veto windows, semantic surface regions, full-diff replay commitments, and validated daemon-to-client wire envelopes.
- Deterministic compilation of the retired Ward identity declarations for name, person, pronouns, purpose, and Coven membership.
- RFC-0001 provenance events with persistence-level required-detail constraints.
- `ward_audit_migration_sql` — transaction SQL builder that preserves existing detail payloads while also supporting pre-detail legacy tables. Schema ownership is tracked in `ward_schema_meta`, not SQLite's database-global `PRAGMA user_version`.
- Exhaustiveness test `schema_names_all_event_tags` extended to cover `ApplyAudit`.
- New tests: `for_apply_produces_correct_shape`, `for_apply_roundtrips_json`, `migration_sql_contains_apply_audit_and_detail_column`.

### Design notes

- **Column approach (Option A):** `diff_hash` carries `next_sha256`; `prev_sha256` + `bytes_written` ride in the new `detail` TEXT column as JSON. This avoids a second table rebuild by not adding typed hash columns while keeping the data query-accessible via SQLite JSON functions.
- **Migration approach (Option B-lite):** exports a migration builder and schema-action classifier so the daemon can perform the correct guarded rebuild for the observed source schema; component metadata avoids colliding with unrelated tables in `coven.sqlite3`.

## [0.1.3] — prior release

Initial audit module with seven `AuditEventType` variants (`proposal_submitted`, `proposal_approved`, `proposal_rejected`, `proposal_vetoed`, `ward_updated`, `validation_verdict`, `compaction_ledger`).
