# ApplyAudit Migration Repair Implementation Plan

**Goal:** Replace the old two-boolean `ward_audit` migration gate with a full
schema fingerprint/state contract built around exact stored
`sqlite_master.sql` fingerprints for the table/indexes/triggers plus ordered
column metadata, reserve the `ward_audit*` namespace on fresh installs, make
schema initialization fail closed and atomic, preserve evidence and unrelated
`user_version`, and prove rollback behavior with executable rusqlite tests.

**Architecture:** Keep ownership in `crates/coven-threads-core/src/audit.rs`.
Expose `WARD_AUDIT_SCHEMA_STATE_SQL` plus stable tags so callers can branch on
`missing` / `legacy_v013` / `current_v014` / `unknown`, reserve `missing` for
an absent table with no preexisting main-schema `ward_audit*` objects, wrap
`WARD_AUDIT_SCHEMA_SQL` in atomic pre/post guards that allow only
`missing`/`current_v014`, and embed the same exact `legacy_v013` predicate
inside `WARD_AUDIT_MIGRATION_V014_SQL` before any `ALTER TABLE`. The init and
migration paths remain table-local and never write database-wide `user_version`.

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
  - absent-table reserved-name collisions (`ward_audit_event_idx`,
    `ward_audit_append_only_update`) → `unknown`.
- Add init-safety tests for:
  - clean empty DB → `WARD_AUDIT_SCHEMA_SQL` succeeds atomically, lands on
    `current_v014`, and append-only UPDATE/DELETE still abort;
  - exact current schema → rerunning `WARD_AUDIT_SCHEMA_SQL` is idempotent and
    preserves rows/objects;
  - exact legacy schema → `WARD_AUDIT_SCHEMA_SQL` rejects, requires explicit
    `ROLLBACK`, and preserves state/data;
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
- Add post-`ALTER` rollback coverage by precreating conflicting
  `ward_audit_new`, forcing `CREATE TABLE ward_audit_new` to fail after the
  guard and `ALTER TABLE` succeed, then rolling back and asserting the legacy
  row/schema are restored.
- Add exact-stored-SQL tests proving fresh current and successfully migrated
  current both classify `current_v014` while matching only their controlled
  SQLite-emitted table-SQL variants, and that the shipped legacy fixture
  matches `legacy_v013`.

## Task 2: Implement the schema fingerprint/state contract

- Add stable state tag constants.
- Add public `WARD_AUDIT_SCHEMA_STATE_SQL` that fingerprints:
  - exact stored `sqlite_master.sql` for `ward_audit`, covering all table-level
    constraints;
  - ordered column metadata (including `recorded_at` default and PK metadata);
  - exact explicit index SQL (ordered by name, excluding SQLite autoindexes);
  - exact append-only trigger SQL (ordered by name);
  - when `ward_audit` is absent, an empty reserved main-schema
    `ward_audit*` namespace across tables/indexes/triggers/views; and
  - only the controlled fresh/migrated current table-SQL variants plus the
    shipped legacy table-SQL variant, including the presence of `apply_audit`
    and any preserved inline comments.
- Re-export the state query and tags from `crates/coven-threads-core/src/lib.rs`
  for root-API callers.
- Update `WARD_AUDIT_SCHEMA_SQL` so it:
  - starts a transaction;
  - creates a TEMP pre-install guard with `CHECK (ok = 1)`;
  - inserts `1` only when the shared schema-state expression returns
    `missing` or `current_v014`, otherwise inserts `0` and aborts before any
    mutation;
  - keeps `CREATE TABLE/INDEX/TRIGGER IF NOT EXISTS` for current daemon
    compatibility;
  - creates a TEMP post-install guard that requires exact `current_v014`,
    aborting on any silent no-op/collision or malformed result;
  - commits on success; and
  - requires callers to `ROLLBACK` after any init error.
- Update `WARD_AUDIT_MIGRATION_V014_SQL` so it:
  - starts a transaction;
  - creates a TEMP guard table with `CHECK (ok = 1)`;
  - inserts `1` only when the full exact legacy predicate holds, otherwise
    inserts `0` and aborts;
  - drops the guard on the success path;
  - keeps `ALTER ADD detail`, strict replacement-table creation, detail copy,
    and explicit index/trigger recreation;
  - never writes `PRAGMA user_version`.

## Task 3: Update the written contract and validate

- Update `audit.rs` docs, the design doc, the plan doc, and `CHANGELOG.md` to
  say that exact stored `sqlite_master.sql` equality covers every declared
  table-level constraint, alongside column/index/trigger fingerprints, that the
  main-schema `ward_audit*` namespace is reserved on fresh installs, and that
  no whitespace-destroying normalization is allowed.
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
fix(audit): make schema initialization fail closed

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
```
