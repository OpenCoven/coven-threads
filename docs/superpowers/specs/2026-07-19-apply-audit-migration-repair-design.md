# ApplyAudit Migration Repair

## Goal

Repair PR #7 so upgrading a v0.1.3 `ward_audit` table to v0.1.4 cannot
silently erase immutable `detail` evidence, while keeping the change scoped to
the audit schema contract.

## Design

The v0.1.4 migration will first add the nullable `detail` column to the legacy
table, then rebuild the table and copy `detail` into the replacement table.
This gives genuine v0.1.3 rows a `NULL` detail value without hard-coding data
loss into the copy step.

Fresh v0.1.4 schema initialization will stamp `PRAGMA user_version = 14`.
Running the legacy migration against a current schema will fail when it tries
to add an already-present `detail` column, before dropping or rewriting any
table. Re-running the migration after a successful upgrade will likewise fail
closed.

## Tests

Executable SQLite tests will cover:

1. A representative v0.1.3 table upgrades successfully, preserves every
   existing field, gains `detail`, accepts `apply_audit`, retains append-only
   triggers, and ends at `user_version = 14`.
2. A fresh v0.1.4 schema starts at `user_version = 14` and preserves populated
   `ApplyAudit` detail.
3. Applying the legacy migration to a current schema returns an error and
   leaves the existing audit row unchanged.
4. Re-running the migration after a successful legacy upgrade returns an error
   and leaves migrated rows unchanged.

Tests will use an in-memory bundled SQLite dependency available only to the
crate's test target.

## Scope

This repair does not add daemon migration orchestration or change the
`WardAuditRecord` wire shape. The daemon remains responsible for choosing
schema initialization for new stores and the v0.1.4 migration for legacy
stores.
