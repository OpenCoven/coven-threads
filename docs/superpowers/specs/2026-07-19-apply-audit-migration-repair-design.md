# ApplyAudit Migration Repair

## Goal

Repair PR #7 so upgrading a v0.1.3 `ward_audit` table to v0.1.4 cannot
silently erase immutable `detail` evidence, while keeping the change scoped to
the audit schema contract.

## Design

The v0.1.4 migration remains a table-local repair. Future daemon integration
must inspect `ward_audit` itself before execution:

- exact legacy v0.1.3: the table exists, `detail` is absent, and the table
  CHECK does not contain `apply_audit`;
- current v0.1.4: `detail` exists and the table accepts `apply_audit`;
- partial/unknown shapes: fail closed and do not run the migration.

For the exact legacy shape, the migration first adds the nullable `detail`
column to the legacy table, then rebuilds the table and copies `detail` into
the replacement table. This gives genuine v0.1.3 rows a `NULL` detail value
without hard-coding data loss into the copy step. The SQL never reads or
writes database-wide `PRAGMA user_version`, so unrelated shared-store version
state is preserved.

Running the legacy migration against a current schema will fail when it tries
to add an already-present `detail` column, before dropping or rewriting any
table. Re-running the migration after a successful upgrade will likewise fail
closed. Callers remain responsible for rolling back a failed migration
transaction before continuing.

## Tests

Executable SQLite tests will cover:

1. A representative v0.1.3 table upgrades successfully, preserves every
   existing field, gains `detail`, accepts `apply_audit`, retains append-only
   triggers, and preserves a pre-set unrelated `user_version` (for example
   `37`).
2. A fresh v0.1.4 schema preserves pre-set database-wide `user_version` values
   `0` and `99` while creating the current table shape and preserving populated
   `ApplyAudit` detail.
3. Applying the legacy migration to a current schema returns an error and
   leaves the existing audit row and database-wide `user_version` unchanged.
4. Re-running the migration after a successful legacy upgrade returns an error
   and leaves migrated rows and database-wide `user_version` unchanged.

Tests will use an in-memory bundled SQLite dependency available only to the
crate's test target.

## Scope

This repair does not add daemon migration orchestration or change the
`WardAuditRecord` wire shape. The daemon remains responsible for choosing
schema initialization for new stores and the exact-shape-gated v0.1.4
migration for legacy stores.
