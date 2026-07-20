# ApplyAudit Migration Repair Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the v0.1.4 `ward_audit` migration preserve immutable evidence and fail closed when invoked against an already-current schema.

**Architecture:** Keep migration ownership in `audit.rs`. The legacy migration adds `detail` before rebuilding and copies that column verbatim; the current schema stamps version 14. In-memory SQLite tests execute both schema paths and verify data preservation, append-only enforcement, versioning, and safe failure.

**Tech Stack:** Rust 2021, SQLite, `rusqlite` test dependency, Cargo test.

---

## File Map

- Modify `crates/coven-threads-core/Cargo.toml` to add bundled SQLite for tests only.
- Modify `crates/coven-threads-core/src/audit.rs` to repair the migration SQL, stamp fresh schemas, and add executable migration tests.
- Modify `Cargo.lock` through Cargo after adding the test dependency.

### Task 1: Add executable migration regressions

**Files:**
- Modify: `crates/coven-threads-core/Cargo.toml`
- Modify: `crates/coven-threads-core/src/audit.rs:435-590`
- Modify: `Cargo.lock`

- [ ] **Step 1: Add the test-only SQLite dependency**

Append this section to `crates/coven-threads-core/Cargo.toml`:

```toml
[dev-dependencies]
rusqlite = { version = "0.31", features = ["bundled"] }
```

- [ ] **Step 2: Add a representative legacy schema fixture**

Inside `audit.rs`'s `tests` module, import `rusqlite::{params, Connection}` and add:

```rust
const LEGACY_WARD_AUDIT_SCHEMA_SQL: &str = r#"
CREATE TABLE ward_audit (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type    TEXT    NOT NULL CHECK (event_type IN (
                      'proposal_submitted','proposal_approved','proposal_rejected',
                      'proposal_vetoed','ward_updated','validation_verdict',
                      'compaction_ledger')),
    proposal_id   TEXT,
    familiar_id   TEXT    NOT NULL,
    ward_version  TEXT,
    ward_hash     BLOB    NOT NULL,
    tier          TEXT,
    decision      TEXT    NOT NULL,
    approver      TEXT,
    diff_hash     BLOB,
    files_touched TEXT    NOT NULL,
    channel       TEXT,
    thread_id     TEXT,
    submitted_at  TEXT    NOT NULL,
    decided_at    TEXT    NOT NULL,
    recorded_at   TEXT    NOT NULL
);

CREATE TRIGGER ward_audit_append_only_update
BEFORE UPDATE ON ward_audit
BEGIN
    SELECT RAISE(ABORT, 'ward_audit is append-only (RFC-0001 §5.6)');
END;

CREATE TRIGGER ward_audit_append_only_delete
BEFORE DELETE ON ward_audit
BEGIN
    SELECT RAISE(ABORT, 'ward_audit is append-only (RFC-0001 §5.6)');
END;
"#;

fn insert_audit_row(connection: &Connection, event_type: &str, detail: Option<&str>) -> i64 {
    connection
        .execute(
            "INSERT INTO ward_audit (
                event_type, proposal_id, familiar_id, ward_version, ward_hash,
                tier, decision, approver, diff_hash, detail, files_touched,
                channel, thread_id, submitted_at, decided_at, recorded_at
             ) VALUES (
                ?1, 'proposal-1', 'familiar-1', '0.1.4', X'0102',
                'tier_2', 'applied', 'principal:val', X'0304', ?2, '[\"SOUL.md\"]',
                'mutation', 'thread-1', '2026-07-19T00:00:00Z',
                '2026-07-19T00:00:01Z', '2026-07-19T00:00:02Z'
             )",
            params![event_type, detail],
        )
        .unwrap();
    connection.last_insert_rowid()
}
```

For the legacy test, insert its pre-migration row with a dedicated statement
that omits `detail`, because the v0.1.3 table does not have that column.

- [ ] **Step 3: Add the failing fresh-schema version test**

```rust
#[test]
fn fresh_schema_is_stamped_v014() {
    let connection = Connection::open_in_memory().unwrap();
    connection.execute_batch(WARD_AUDIT_SCHEMA_SQL).unwrap();

    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, 14);
}
```

- [ ] **Step 4: Add the failing current-schema preservation test**

```rust
#[test]
fn legacy_migration_rejects_current_schema_without_erasing_detail() {
    let connection = Connection::open_in_memory().unwrap();
    connection.execute_batch(WARD_AUDIT_SCHEMA_SQL).unwrap();
    let detail = r#"{"prev_sha256":"abcd","bytes_written":42}"#;
    let id = insert_audit_row(&connection, "apply_audit", Some(detail));

    let error = connection
        .execute_batch(WARD_AUDIT_MIGRATION_V014_SQL)
        .expect_err("current schema must reject the legacy migration");
    assert!(error.to_string().contains("duplicate column name: detail"));
    let _ = connection.execute_batch("ROLLBACK;");

    let preserved: String = connection
        .query_row(
            "SELECT detail FROM ward_audit WHERE id = ?1",
            [id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(preserved, detail);
}
```

- [ ] **Step 5: Add the legacy-upgrade and rerun tests**

The legacy-upgrade test must:

1. Execute `LEGACY_WARD_AUDIT_SCHEMA_SQL`.
2. Insert a fully populated legacy row without `detail`.
3. Execute `WARD_AUDIT_MIGRATION_V014_SQL`.
4. Assert the row's original values remain unchanged and `detail IS NULL`.
5. Assert `PRAGMA user_version = 14`.
6. Insert an `apply_audit` row with populated `detail`.
7. Assert both append-only triggers reject `UPDATE` and `DELETE`.

The rerun test must execute the migration successfully once, invoke it again,
assert the duplicate-`detail` error, roll back the failed transaction, and
confirm the original migrated row still exists unchanged.

- [ ] **Step 6: Run the focused tests and confirm the regression**

Run:

```bash
cargo test -p coven-threads-core audit::tests -- --nocapture
```

Expected before the SQL repair: `fresh_schema_is_stamped_v014` fails with
`left: 0, right: 14`, and
`legacy_migration_rejects_current_schema_without_erasing_detail` fails because
the migration succeeds and replaces `detail` with `NULL`.

### Task 2: Make the migration preserve evidence and fail closed

**Files:**
- Modify: `crates/coven-threads-core/src/audit.rs:302-433`

- [ ] **Step 1: Guard the legacy migration before destructive work**

Immediately after `BEGIN;` in `WARD_AUDIT_MIGRATION_V014_SQL`, add:

```sql
ALTER TABLE ward_audit ADD COLUMN detail TEXT;
```

This succeeds only for the actual v0.1.3 shape. A current schema or a repeated
upgrade fails before `DROP TABLE ward_audit`.

- [ ] **Step 2: Make replacement-table creation strict**

Change:

```sql
CREATE TABLE IF NOT EXISTS ward_audit_new (
```

to:

```sql
CREATE TABLE ward_audit_new (
```

An abandoned or conflicting migration table must abort instead of being reused.

- [ ] **Step 3: Copy immutable detail instead of nulling it**

Change the migration's `SELECT` projection from:

```sql
tier, decision, approver, diff_hash, NULL, files_touched, channel,
```

to:

```sql
tier, decision, approver, diff_hash, detail, files_touched, channel,
```

- [ ] **Step 4: Stamp fresh v0.1.4 schemas**

Append this statement to `WARD_AUDIT_SCHEMA_SQL`, after trigger creation:

```sql
PRAGMA user_version = 14;
```

- [ ] **Step 5: Correct the migration documentation**

Update the module and constant comments to state:

- New stores receive current DDL and `user_version = 14`.
- The legacy migration is only for the exact v0.1.3 shape.
- It adds `detail`, copies every row including `detail`, and stamps version 14.
- Current-schema invocation or rerun fails before destructive work.
- Callers must roll back a failed migration transaction.

- [ ] **Step 6: Run focused tests**

Run:

```bash
cargo test -p coven-threads-core audit::tests -- --nocapture
```

Expected: all audit tests pass.

- [ ] **Step 7: Commit the repair**

```bash
git add crates/coven-threads-core/Cargo.toml \
  crates/coven-threads-core/src/audit.rs Cargo.lock
git commit -m "fix(audit): preserve detail during v0.1.4 migration" \
  -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

### Task 3: Validate and prepare PR #7 update

**Files:**
- Verify: all changed files
- Update through CLI: Bead `threads-44o`, PR #7

- [ ] **Step 1: Run repository quality gates**

Run:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Expected: all commands exit successfully with no warnings.

- [ ] **Step 2: Review the final diff**

Run:

```bash
git diff origin/cody/apply-audit-v014...HEAD --check
git diff origin/cody/apply-audit-v014...HEAD --stat
git status --short --branch
```

Expected: only the design, plan, manifest/lockfile, and audit migration files
are changed; the worktree is clean after commits.

- [ ] **Step 3: Record verification in Beads**

Update `threads-44o` with the branch, worktree, commit SHA, focused test result,
and full quality-gate result. Do not close the Bead until PR #7 is updated and
the fix is accepted or merged.

- [ ] **Step 4: Re-check remote branch drift**

Run:

```bash
git fetch origin cody/apply-audit-v014
git log --oneline --left-right HEAD...origin/cody/apply-audit-v014
```

Expected: no remote-only commits. If remote-only commits exist, stop and
reconcile before pushing.

- [ ] **Step 5: Update PR #7 and publish review evidence**

Push the worktree tip to the existing PR branch only if it is a fast-forward:

```bash
git push origin HEAD:cody/apply-audit-v014
```

Then comment on PR #7 with the root cause, the fail-closed migration design,
and the exact quality gates that passed.
