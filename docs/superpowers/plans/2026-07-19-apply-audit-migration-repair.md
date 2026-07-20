# ApplyAudit Migration Repair Implementation Plan

**Goal:** Replace the old two-boolean `ward_audit` migration gate with a full
schema fingerprint/state contract built around exact durable
`main.sqlite_master.sql` fingerprints for the table/indexes/triggers plus
ordered column metadata, reserve the durable `ward_audit` /
`ward_audit_*` namespace to an exact five-object whitelist in every state, fail
closed on any TEMP shadow/reserved temp object, make schema initialization fail
closed and atomic, preserve evidence and unrelated `user_version`, and prove
rollback behavior with executable rusqlite tests.

**Architecture:** Keep ownership in `crates/coven-threads-core/src/audit.rs`.
Expose `WARD_AUDIT_SCHEMA_STATE_SQL` plus stable tags so callers can branch on
`missing` / `legacy_v013` / `current_v014` / `unknown`, reserve `missing` for
an absent `main.ward_audit` with no unexpected durable reserved-name object and
no temp shadow/reserved temp object, require the same exact durable whitelist
for `legacy_v013` and `current_v014`, wrap `WARD_AUDIT_SCHEMA_SQL` in atomic
pre/post guards that allow only `missing`/`current_v014`, and embed the same
exact `legacy_v013` + no-unexpected-durable-object + no-temp-shadow predicate
inside `WARD_AUDIT_MIGRATION_V014_SQL` before any `ALTER TABLE`. Durable
DDL/DML stays explicitly qualified to `main` wherever SQLite permits it, and
the init/migration paths never write database-wide `user_version`.

**Tech Stack:** Rust 2021, SQLite, Cargo, bundled `rusqlite` for in-memory
tests.

---

## Final File Map

For this follow-up fingerprint repair, the diff stays scoped to these four
files:

1. `CHANGELOG.md`
2. `crates/coven-threads-core/src/audit.rs`
3. `docs/superpowers/plans/2026-07-19-apply-audit-migration-repair.md`
4. `docs/superpowers/specs/2026-07-19-apply-audit-migration-repair-design.md`

---

## Task 1: Lock the behavior down with tests

- Add schema-state tests for:
  - empty DB → `missing`;
  - exact legacy fixture → `legacy_v013`;
  - exact current schema → `current_v014`;
  - exact legacy/current with only the durable whitelist objects still
    classifying correctly;
  - absent-table reserved-name collisions (`ward_audit_event_idx`,
    `ward_audit_append_only_update`) → `unknown`;
  - exact current + extra durable reserved-name view/table → `unknown`;
  - exact legacy + preexisting `main.ward_audit_new` → `unknown`;
  - current/legacy/missing durable states plus a TEMP `ward_audit` shadow or
    reserved temp object → `unknown`.
- Add syntax-verification tests for bundled SQLite:
  - `pragma_table_info('ward_audit', 'main')` sees durable columns even when a
    TEMP `ward_audit` exists; and
  - `pragma_index_list('ward_audit', 'main')` sees durable explicit indexes.
- Add init-safety tests for:
  - clean empty DB → `WARD_AUDIT_SCHEMA_SQL` succeeds atomically, lands on
    `current_v014`, and append-only UPDATE/DELETE still abort;
  - exact current schema → rerunning `WARD_AUDIT_SCHEMA_SQL` is idempotent and
    preserves rows/objects;
  - exact legacy schema → `WARD_AUDIT_SCHEMA_SQL` rejects, requires explicit
    `ROLLBACK`, and preserves state/data;
  - exact current `main.ward_audit` + TEMP shadow clone → init rejects,
    explicit rollback preserves both main and temp rows, and dropping the TEMP
    shadow restores `current_v014`;
  - missing main + TEMP `ward_audit` / reserved `ward_audit_*` temp object →
    init rejects and never creates `main.ward_audit`;
  - unknown partial current schema → `WARD_AUDIT_SCHEMA_SQL` rejects and
    preserves the drifted state.
- Add drift tests for:
  - legacy + extra column/data → `unknown`, migration guard failure, explicit
    `ROLLBACK`, and preservation of row data, extra column, indexes, triggers,
    and unrelated `user_version`;
  - legacy/current `event_type` CHECK drift where a quoted literal gains an
    internal space (for example `apply_ audit`) → `unknown`, with the legacy
    guard failing before mutation and rollback preserving data;
  - legacy/current with an extra table-level `CHECK (length(decision) > 0)` →
    `unknown`, with the legacy guard failing before mutation and rollback
    preserving the original CHECK behavior;
  - legacy/current with an extra table-level `UNIQUE (decision, recorded_at)`
    → `unknown`, with the legacy guard failing before mutation and rollback
    preserving the original UNIQUE behavior;
  - current missing one append-only trigger → `unknown`, plus an `UPDATE`
    succeeding to prove why it is unknown;
  - current with an extra explicit object (index), `recorded_at DESC`, or
    `COLLATE NOCASE` on the explicit index SQL → `unknown`;
  - current with altered append-only trigger error literal/body → `unknown`.
- Update existing migration failure tests so exact current/rerun cases fail at
  the new guard and always `ROLLBACK` with `unwrap()`.
- Add legacy TEMP-shadow migration coverage proving the guard rejects before any
  main/temp mutation and rollback preserves the exact legacy durable table.
- Add reason-demo coverage proving an unqualified `INSERT INTO ward_audit ...`
  lands in TEMP while the durable contract remains `unknown`.
- Add a controlled post-`ALTER` rollback coverage path that starts from exact
  legacy, performs the same `ALTER TABLE main.ward_audit ADD COLUMN detail
  TEXT`, then forces a later SQL error by creating `main.ward_audit_new` twice
  inside the same transaction; require the error, explicit rollback success,
  full row/schema/user_version restoration, and `legacy_v013` classification.
- Add exact-stored-SQL tests proving fresh current and successfully migrated
  current both classify `current_v014` while matching only their controlled
  SQLite-emitted table-SQL variants, and that the shipped legacy fixture
  matches `legacy_v013`.

## Task 2: Implement the schema fingerprint/state contract

- Add stable state tag constants.
- Add public `WARD_AUDIT_SCHEMA_STATE_SQL` that fingerprints:
  - exact stored `main.sqlite_master.sql` for `ward_audit`, covering all
    table-level constraints;
  - ordered durable column metadata from `pragma_table_info('ward_audit',
    'main')` (including `recorded_at` default and PK metadata);
  - explicit durable index discovery from `pragma_index_list('ward_audit',
    'main')`, with exact index SQL then read from `main.sqlite_master`;
  - exact append-only trigger SQL from `main.sqlite_master`;
  - at every durable state, an exact reserved main-schema whitelist consisting
    only of table `main.ward_audit`, indexes `ward_audit_event_idx` and
    `ward_audit_familiar_idx` attached to it, and triggers
    `ward_audit_append_only_update` / `ward_audit_append_only_delete` attached
    to it, with every other `ward_audit` / `ward_audit_*` main object rejected;
  - at every durable state, rejection when any temp-schema table/view/index/
    trigger is named `ward_audit` or begins with `ward_audit_`; and
  - only the controlled fresh/migrated current table-SQL variants plus the
    shipped legacy table-SQL variant, including the presence of `apply_audit`
    and any preserved inline comments.
- Re-export the state query and tags from `crates/coven-threads-core/src/lib.rs`
  for root-API callers.
- Update `WARD_AUDIT_SCHEMA_SQL` so it:
  - starts a transaction;
  - creates a uniquely named TEMP pre-install guard with `CHECK (ok = 1)`;
  - inserts `1` only when the shared schema-state expression returns
    `missing` or `current_v014`, otherwise inserts `0` and aborts before any
    mutation;
  - keeps `CREATE TABLE/INDEX/TRIGGER IF NOT EXISTS` for current daemon
    compatibility while targeting `main` unambiguously wherever SQLite syntax
    allows it;
  - creates a uniquely named TEMP post-install guard that requires exact
    `current_v014`, aborting on any silent no-op/collision or malformed result;
  - commits on success; and
  - requires callers to `ROLLBACK` after any init error.
- Update `WARD_AUDIT_MIGRATION_V014_SQL` so it:
  - starts a transaction;
  - creates a uniquely named TEMP guard table with `CHECK (ok = 1)`;
  - inserts `1` only when the full exact legacy durable predicate holds and no
    unexpected durable reserved-name object or temp shadow/reserved temp object
    exists, otherwise inserts `0` and aborts;
  - drops the guard on the success path;
  - keeps `ALTER ADD detail`, strict replacement-table creation, detail copy,
    and explicit main index/trigger recreation, all qualified to `main`
    wherever SQLite syntax permits;
  - never writes `PRAGMA user_version`.

## Task 3: Update the written contract and validate

- Update `audit.rs` docs, the design doc, the plan doc, and `CHANGELOG.md` to
  say that the durable audit contract is `main.ward_audit`, exact stored
  `main.sqlite_master.sql` equality covers every declared table-level
  constraint, schema-qualified PRAGMAs are required for durable column/index
  inspection, the exact durable namespace whitelist applies in every state,
  temp shadows/reserved temp objects fail closed, and no whitespace-destroying
  normalization is allowed.
- Document the caller contract explicitly:
  - query `WARD_AUDIT_SCHEMA_STATE_SQL`;
  - initialize on `missing` only;
  - migrate only `legacy_v013`;
  - continue on `current_v014`;
  - fail closed on `unknown`;
  - `ROLLBACK` after any init or migration error; and
  - rely on the init/migration independent guards as second lines of defense.
- Run:
  - `cargo fmt`
  - focused audit tests
  - `cargo test --workspace` if focused tests pass
  - `git diff --check`
  - stale-wording searches for the old partial-gate terminology.

## Commit

Create a new commit (no amend) with:

```text
fix(audit): reject durable namespace drift

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
```
