# ApplyAudit Migration Repair Implementation Plan

**Goal:** Replace the old two-boolean `ward_audit` migration gate with a full
schema fingerprint/state contract built around normalized `CREATE TABLE`
equality plus column/index/trigger fingerprints, add an in-migration legacy
guard, preserve evidence and unrelated `user_version`, and prove rollback
behavior with executable rusqlite tests.

**Architecture:** Keep ownership in `crates/coven-threads-core/src/audit.rs`.
Expose `WARD_AUDIT_SCHEMA_STATE_SQL` plus stable tags so callers can branch on
`missing` / `legacy_v013` / `current_v014` / `unknown`, and embed the same
exact `legacy_v013` predicate inside `WARD_AUDIT_MIGRATION_V014_SQL` before any
`ALTER TABLE`. The migration remains table-local and never writes
database-wide `user_version`.

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
  - exact current schema → `current_v014`.
- Add drift tests for:
  - legacy + extra column/data → `unknown`, migration guard failure, explicit
    `ROLLBACK`, and preservation of row data, extra column, indexes, triggers,
    and unrelated `user_version`;
  - legacy/current with an extra table-level `CHECK (length(decision) > 0)` →
    `unknown`, with the legacy guard failing before mutation and rollback
    preserving the original CHECK behavior;
  - legacy/current with an extra table-level `UNIQUE (decision, recorded_at)`
    → `unknown`, with the legacy guard failing before mutation and rollback
    preserving the original UNIQUE behavior;
  - current missing one append-only trigger → `unknown`, plus an `UPDATE`
    succeeding to prove why it is unknown;
  - current with an extra explicit object (index) → `unknown`.
- Update existing migration failure tests so exact current/rerun cases fail at
  the new guard and always `ROLLBACK` with `unwrap()`.
- Add post-`ALTER` rollback coverage by precreating conflicting
  `ward_audit_new`, forcing `CREATE TABLE ward_audit_new` to fail after the
  guard and `ALTER TABLE` succeed, then rolling back and asserting the legacy
  row/schema are restored.
- Add a normalization-equivalence test proving fresh current and successfully
  migrated current both classify `current_v014` under the same normalized table
  definition.

## Task 2: Implement the schema fingerprint/state contract

- Add stable state tag constants.
- Add public `WARD_AUDIT_SCHEMA_STATE_SQL` that fingerprints:
  - full normalized `CREATE TABLE ward_audit (...)` equality, covering all
    table-level constraints;
  - ordered column metadata (including `recorded_at` default and PK metadata);
  - exact explicit index set, excluding SQLite autoindexes;
  - exact append-only trigger set; and
  - legacy/current event-list differences inside the normalized table
    definition, including the presence of `apply_audit`.
- Re-export the state query and tags from `crates/coven-threads-core/src/lib.rs`
  for root-API callers.
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
  say that full normalized `CREATE TABLE` equality covers every declared
  table-level constraint, alongside column/index/trigger fingerprints, instead
  of the old partial exact-check wording.
- Document the caller contract explicitly:
  - query `WARD_AUDIT_SCHEMA_STATE_SQL`;
  - initialize on `missing`;
  - migrate only `legacy_v013`;
  - continue on `current_v014`;
  - fail closed on `unknown`;
  - rely on the migration’s independent legacy guard as a second line of
    defense.
- Run:
  - `cargo fmt`
  - focused audit tests
  - `cargo test --workspace` if focused tests pass
  - `git diff --check`
  - stale-wording searches for the old partial-gate terminology.

## Commit

Create a new commit (no amend) with:

```text
fix(audit): fingerprint full table definition

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
```
