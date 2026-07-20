# ApplyAudit Migration Repair

## Goal

Repair PR #7 so `ward_audit` upgrades are gated by a full table-local schema
fingerprint/state contract, not a two-boolean proxy. The upgrade must preserve
immutable evidence, preserve unrelated shared-store `user_version`, reject
partial or drifted schemas before mutation, and remain scoped to the audit
schema contract.

## Schema-state contract

`crates/coven-threads-core::WARD_AUDIT_SCHEMA_STATE_SQL` is the reusable query
callers run before touching the durable `main.ward_audit` table. It returns one
stable tag:

- `missing` — `main.ward_audit` does not exist, no unexpected durable
  main-schema object named `ward_audit` or `ward_audit_*` exists, and no temp
  shadow/reserved temp object exists; initialize with `WARD_AUDIT_SCHEMA_SQL`.
- `legacy_v013` — exact v0.1.3 legacy fingerprint plus the durable namespace
  whitelist; run
  `WARD_AUDIT_MIGRATION_V014_SQL`.
- `current_v014` — exact v0.1.4 current fingerprint plus the durable namespace
  whitelist; continue without schema work.
- `unknown` — every other shape; fail closed and investigate manually.

The fingerprint is exact and table-local to `main.ward_audit`:

- exact stored `main.sqlite_master.sql` for the `ward_audit` table;
- ordered durable-column metadata from `pragma_table_info('ward_audit',
  'main')`, including name, type, `NOT NULL`, `DEFAULT`, and primary-key
  metadata;
- durable explicit index discovery from `pragma_index_list('ward_audit',
  'main')`, with exact index SQL then read from `main.sqlite_master` and
  fingerprinted as `name|sql` (excluding SQLite internal autoindexes); and
- exact stored `main.sqlite_master.sql` for each append-only trigger (ordered
  by name and fingerprinted as `name|sql`).

The full stored table definition covers every declared table-level constraint —
the `event_type` CHECK list, extra `CHECK` clauses, `UNIQUE` clauses, and
foreign-key clauses — so any extra or missing constraint, column, index, or
trigger classifies as `unknown`. Across **all** durable states, the reserved
main-schema namespace is whitelisted to exactly these `main.ward_audit`
objects: the table itself, indexes `ward_audit_event_idx` and
`ward_audit_familiar_idx`, and triggers
`ward_audit_append_only_update` / `ward_audit_append_only_delete`, each attached
to `main.ward_audit`. Every other main-schema table/view/index/trigger whose
name is exactly `ward_audit` or begins with `ward_audit_` also classifies as
`unknown`, including `ward_audit_new`, backup/shadow tables, and reserved-name
indexes/triggers attached elsewhere. Any temp-schema table/view/index/trigger
whose name is exactly `ward_audit` or begins with `ward_audit_` likewise
classifies as `unknown`, even when `main.ward_audit` is otherwise exact current
or legacy; that fail-closed rule stops TEMP shadows from being mistaken for
healthy durable state and blocks unqualified daemon writes from proceeding under
a false `current_v014`. No whitespace-destroying normalization is applied: only
the exact stored SQL variants SQLite emits for the shipped schemas are
accepted. For `current_v014`, that means the fresh `CREATE TABLE ward_audit
(...)` form and the quoted `CREATE TABLE "ward_audit" (...)` form produced by
the legacy rename path. For `legacy_v013`, the fingerprint intentionally
includes the inline comments preserved from the shipped v0.1.3 DDL.

## Initialization design

`WARD_AUDIT_SCHEMA_SQL` must stay compatible with the current daemon behavior:
the daemon executes it unconditionally on every store open. The safe contract is
therefore:

1. `BEGIN IMMEDIATE;` to reserve the main-database write slot before any guard
   read/classification;
2. run a uniquely named TEMP pre-install guard that reuses the exact
   schema-state CTEs and permits only `missing` or exact `current_v014`;
3. create the durable `main.ward_audit` table, explicit main indexes, and
   append-only main triggers with `IF NOT EXISTS`;
4. run a uniquely named TEMP post-install guard that reuses the same
   schema-state expression and requires exact `current_v014`; and
5. `COMMIT;`

This makes fresh installs atomic, makes exact current reruns idempotent, and
serializes concurrent initializers before they read `main.sqlite_master`: the
winner reserves the main-database write slot first, and the second caller waits
until commit, re-runs the guard against the committed schema, and then
idempotently sees exact `current_v014`. The path still fails closed for
`legacy_v013`, `unknown`, any unexpected durable reserved-name object, any temp
shadow/reserved temp object, or any malformed/silently skipped install result.
If either guard errors, callers must explicitly `ROLLBACK` before continuing so
SQLite removes any uncommitted `main.ward_audit` table created on the failed
path while preserving unrelated durable/temp objects that caused the collision.

## Migration design

The v0.1.4 repair remains a table-local migration. The SQL still:

1. `ALTER TABLE main.ward_audit ADD COLUMN detail TEXT`;
2. creates a strict replacement `main.ward_audit_new`;
3. copies every row, preserving `detail`;
4. swaps tables; and
5. re-creates the required explicit main indexes and append-only main triggers.

The change is the guardrail: immediately after `BEGIN IMMEDIATE;`, the
migration creates a uniquely named TEMP guard table with `CHECK (ok = 1)` and
inserts `1` only if the exact `legacy_v013` durable fingerprint holds **and**
no unexpected durable reserved-name object or temp shadow/reserved temp object
exists. Any missing/current/partial/shadowed schema inserts `0` instead,
aborting before `ALTER TABLE`. That IMMEDIATE reservation also serializes
concurrent migrators before the guard read: the winner upgrades first, and a
second caller waits, re-runs the guard against the now-current durable table,
and fails closed there instead of racing into a `sqlite_master` lock. This
makes the migration fail closed even if a caller skips the classification query,
and keeps initialization and migration aligned on the same exact state
contract.

If a later step fails after `ALTER TABLE`, callers must explicitly `ROLLBACK`
the failed transaction before continuing so SQLite restores the untouched
legacy table. A preexisting `main.ward_audit_new` is now itself durable drift
and must fail at the guard before `ALTER TABLE`; rollback semantics for the
production migration's post-`ALTER` failure contract are instead validated with
a controlled duplicate `CREATE TABLE main.ward_audit_new` later in the same
transaction. Ward SQL never reads or writes database-wide `PRAGMA user_version`,
so unrelated shared-store version state is preserved.

## Tests

Executable rusqlite tests cover:

1. schema-state classification for `missing`, exact `legacy_v013`, and exact
   `current_v014`, including absent-table reserved durable-namespace collisions,
   exact current/legacy shapes with extra durable reserved-name view/table
   objects, reserved temp objects, and TEMP `ward_audit` shadows classifying as
   `unknown`;
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
11. a controlled post-`ALTER` SQL failure inside the transaction validating
    SQLite rollback semantics for the production migration contract and
    restoring the full legacy table state; and
12. schema-qualified PRAGMA syntax on bundled SQLite plus the reason for the
   fail-closed contract: unqualified inserts hit TEMP first while the contract
   remains `unknown`.
13. file-backed multi-connection initialization with two simultaneous callers,
   `busy_timeout`, and repeated runs proving both `WARD_AUDIT_SCHEMA_SQL`
   executions complete without locked/schema-locked errors because
   `BEGIN IMMEDIATE` serializes them before guard reads; and
14. file-backed multi-connection legacy migration with two simultaneous callers,
   `busy_timeout`, and repeated runs proving one migration succeeds while the
   second waits, re-runs the guard against exact `current_v014`, and fails only
   at the legacy guard (never with a lock error), while preserving the legacy
   row data.

## Scope

This repair does not add daemon migration orchestration or change the
`WardAuditRecord` wire shape. The daemon remains responsible for calling
`WARD_AUDIT_SCHEMA_STATE_SQL`, choosing fresh initialization vs legacy
migration, treating `main.ward_audit` as the only durable audit contract,
accepting only the exact durable whitelist of `main.ward_audit` plus its two
indexes and two append-only triggers, running `WARD_AUDIT_SCHEMA_SQL` only
through the allowed `missing`/`current_v014` contract, and failing closed on
`unknown`. Concurrent callers must also treat a migration guard rejection after
waiting as a signal to reclassify: another writer may already have serialized
the schema to exact `current_v014`.
