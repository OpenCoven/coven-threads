# ApplyAudit Migration Repair

## Goal

Repair PR #7 so `ward_audit` upgrades are gated by a full table-local schema
fingerprint/state contract, not a two-boolean proxy. The upgrade must preserve
immutable evidence, preserve unrelated shared-store `user_version`, reject
partial or drifted schemas before mutation, and remain scoped to the audit
schema contract.

## Schema-state contract

`crates/coven-threads-core::WARD_AUDIT_SCHEMA_STATE_SQL` is the reusable query
callers run before touching `ward_audit`. It returns one stable tag:

- `missing` — `ward_audit` does not exist; initialize with
  `WARD_AUDIT_SCHEMA_SQL`.
- `legacy_v013` — exact v0.1.3 legacy fingerprint; run
  `WARD_AUDIT_MIGRATION_V014_SQL`.
- `current_v014` — exact v0.1.4 current fingerprint; continue without schema
  work.
- `unknown` — every other shape; fail closed and investigate manually.

The fingerprint is exact and table-local:

- ordered column metadata from `pragma_table_info('ward_audit')`, including
  name, type, `NOT NULL`, `DEFAULT`, and primary-key metadata;
- exact explicit index set (excluding SQLite internal autoindexes);
- exact append-only trigger set;
- exact `event_type` CHECK signature, distinguishing legacy absence vs current
  presence of `apply_audit`.

Extra or missing columns, indexes, triggers, or CHECK drift all classify as
`unknown`.

## Migration design

The v0.1.4 repair remains a table-local migration. The SQL still:

1. `ALTER TABLE ward_audit ADD COLUMN detail TEXT`;
2. creates a strict replacement `ward_audit_new`;
3. copies every row, preserving `detail`;
4. swaps tables; and
5. re-creates the required explicit indexes and append-only triggers.

The change is the guardrail: immediately after `BEGIN;`, the migration creates
a TEMP guard table with `CHECK (ok = 1)` and inserts `1` only if the exact
`legacy_v013` fingerprint holds. Any missing/current/partial schema inserts `0`
instead, aborting before `ALTER TABLE`. This makes the migration fail closed
even if a caller skips the classification query.

If a later step fails after `ALTER TABLE` (for example, `ward_audit_new`
already exists), callers must explicitly `ROLLBACK` the failed transaction
before continuing so SQLite restores the untouched legacy table. Ward SQL never
reads or writes database-wide `PRAGMA user_version`, so unrelated shared-store
version state is preserved.

## Tests

Executable rusqlite tests cover:

1. schema-state classification for `missing`, exact `legacy_v013`, and exact
   `current_v014`;
2. partial legacy drift (`legacy` + extra column/data) classifying as
   `unknown`, rejecting at the guard, and rolling back cleanly without losing
   row data, extra columns, indexes, triggers, or unrelated `user_version`;
3. current drift cases such as a missing append-only trigger or an extra index
   classifying as `unknown`;
4. exact current/rerun guard failures requiring explicit `ROLLBACK`;
5. successful exact-legacy upgrade landing in `current_v014`; and
6. post-`ALTER` failure rollback restoring the full legacy table state.

## Scope

This repair does not add daemon migration orchestration or change the
`WardAuditRecord` wire shape. The daemon remains responsible for calling
`WARD_AUDIT_SCHEMA_STATE_SQL`, choosing fresh initialization vs legacy
migration, and failing closed on `unknown`.
