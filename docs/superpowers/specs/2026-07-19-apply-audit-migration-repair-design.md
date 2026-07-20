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

- `missing` — `ward_audit` does not exist and no reserved main-schema
  `ward_audit*` table/index/trigger/view names already exist; initialize with
  `WARD_AUDIT_SCHEMA_SQL`.
- `legacy_v013` — exact v0.1.3 legacy fingerprint; run
  `WARD_AUDIT_MIGRATION_V014_SQL`.
- `current_v014` — exact v0.1.4 current fingerprint; continue without schema
  work.
- `unknown` — every other shape; fail closed and investigate manually.

The fingerprint is exact and table-local:

- exact stored `sqlite_master.sql` for the `ward_audit` table;
- ordered column metadata from `pragma_table_info('ward_audit')`, including
  name, type, `NOT NULL`, `DEFAULT`, and primary-key metadata;
- exact stored `sqlite_master.sql` for each explicit index (ordered by name and
  fingerprinted as `name|sql`, excluding SQLite internal autoindexes); and
- exact stored `sqlite_master.sql` for each append-only trigger (ordered by
  name and fingerprinted as `name|sql`).

The full stored table definition covers every declared table-level constraint —
the `event_type` CHECK list, extra `CHECK` clauses, `UNIQUE` clauses, and
foreign-key clauses — so any extra or missing constraint, column, index, or
trigger classifies as `unknown`. When `ward_audit` itself is absent, any
preexisting main-schema object whose name is reserved under `ward_audit*` also
classifies as `unknown`; that prevents schema init from silently skipping
append-only objects already attached to other tables. No whitespace-destroying
normalization is applied: only the exact stored SQL variants SQLite emits for
the shipped schemas are accepted. For `current_v014`, that means the fresh
`CREATE TABLE ward_audit (...)` form and the quoted
`CREATE TABLE "ward_audit" (...)` form produced by the legacy rename path. For
`legacy_v013`, the fingerprint intentionally includes the inline comments
preserved from the shipped v0.1.3 DDL.

## Initialization design

`WARD_AUDIT_SCHEMA_SQL` must stay compatible with the current daemon behavior:
the daemon executes it unconditionally on every store open. The safe contract is
therefore:

1. `BEGIN;`
2. run a TEMP pre-install guard that reuses the exact schema-state CTEs and
   permits only `missing` or exact `current_v014`;
3. create the `ward_audit` table, explicit indexes, and append-only triggers
   with `IF NOT EXISTS`;
4. run a TEMP post-install guard that reuses the same schema-state expression
   and requires exact `current_v014`; and
5. `COMMIT;`

This makes fresh installs atomic, makes exact current reruns idempotent, and
fails closed for `legacy_v013`, `unknown`, or any malformed/silently skipped
install result. If either guard errors, callers must explicitly `ROLLBACK`
before continuing so SQLite removes any uncommitted `ward_audit` table created
on the failed path while preserving unrelated objects that caused the
collision.

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
even if a caller skips the classification query, and keeps initialization and
migration aligned on the same exact state contract.

If a later step fails after `ALTER TABLE` (for example, `ward_audit_new`
already exists), callers must explicitly `ROLLBACK` the failed transaction
before continuing so SQLite restores the untouched legacy table. Ward SQL never
reads or writes database-wide `PRAGMA user_version`, so unrelated shared-store
version state is preserved.

## Tests

Executable rusqlite tests cover:

1. schema-state classification for `missing`, exact `legacy_v013`, and exact
   `current_v014`, including absent-table reserved-namespace collisions
   classifying as `unknown`;
2. fresh empty-store initialization classifying `missing`, running
   `WARD_AUDIT_SCHEMA_SQL` atomically to `current_v014`, and enforcing
   append-only UPDATE/DELETE rejection;
3. exact current reruns of `WARD_AUDIT_SCHEMA_SQL` staying idempotent while
   preserving rows, indexes, triggers, and exact stored SQL fingerprints;
4. exact legacy/current/unknown init-guard failures requiring explicit
   `ROLLBACK` and preserving preexisting data/object state;
5. reserved-name collision repros (`ward_audit_event_idx` and
   `ward_audit_append_only_update` on another table) failing closed before any
   `ward_audit` table is created; and
6. partial legacy drift (`legacy` + extra column/data) classifying as
   `unknown`, rejecting at the guard, and rolling back cleanly without losing
   row data, extra columns, indexes, triggers, or unrelated `user_version`;
7. legacy/current drift caused by extra table-level `CHECK` or `UNIQUE`
   constraints classifying as `unknown`, with legacy guard failures requiring
   explicit `ROLLBACK` and preserving the original constraint behavior;
8. current/legacy `event_type` CHECK drift where a quoted literal gains an
   internal space (for example `apply_ audit`) classifying as `unknown`, with
   the legacy guard rejecting before mutation and rollback preserving data;
9. current drift cases such as a missing append-only trigger, an extra index, a
   `recorded_at DESC` index, a `COLLATE NOCASE` index, or altered append-only
   trigger SQL classifying as `unknown`;
10. successful exact-legacy upgrade landing in `current_v014`, with fresh and
   migrated current schemas both classifying `current_v014` while matching only
   their controlled exact stored SQL variants; and
11. post-`ALTER` failure rollback restoring the full legacy table state.

## Scope

This repair does not add daemon migration orchestration or change the
`WardAuditRecord` wire shape. The daemon remains responsible for calling
`WARD_AUDIT_SCHEMA_STATE_SQL`, choosing fresh initialization vs legacy
migration, running `WARD_AUDIT_SCHEMA_SQL` only through the allowed
`missing`/`current_v014` contract, and failing closed on `unknown`.
