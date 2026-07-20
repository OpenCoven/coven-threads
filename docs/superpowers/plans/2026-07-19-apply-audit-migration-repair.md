# ApplyAudit Migration Repair Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the v0.1.4 `ward_audit` migration preserve immutable evidence, preserve unrelated database-wide version state, and fail closed when invoked outside the exact legacy schema shape.

**Architecture:** Keep migration ownership in `audit.rs`. The legacy repair adds `detail` before rebuilding and copies that column verbatim; callers decide between fresh schema creation and legacy repair by inspecting the exact `ward_audit` table shape (`detail` column presence plus whether the table CHECK admits `apply_audit`). Ward SQL never writes database-wide `user_version`; the tests preserve and inspect unrelated store-wide values only through rusqlite pragma helpers.

**Tech Stack:** Rust 2021, SQLite, Cargo, bundled `rusqlite` dev-dependency for in-memory tests.

---

## File Map

- Modify `docs/superpowers/specs/2026-07-19-apply-audit-migration-repair-design.md` to document exact `ward_audit` shape gating and explicit preservation of unrelated database-wide `user_version`.
- Modify `docs/superpowers/plans/2026-07-19-apply-audit-migration-repair.md` to mirror the landed migration design, exact test names, and final diff expectations.
- Modify `CHANGELOG.md` to align the v0.1.4 migration notes with the repaired implementation and executable migration regressions.
- Modify `crates/coven-threads-core/Cargo.toml` to add bundled `rusqlite` under `[dev-dependencies]` for executable SQLite migration tests.
- Modify `Cargo.lock` to resolve that test-only dependency.
- Modify `crates/coven-threads-core/src/audit.rs` to repair the migration SQL, document fail-closed table-shape gating, and add executable migration tests.

### Task 1: Add executable migration regressions

**Files:**
- Modify: `crates/coven-threads-core/Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/coven-threads-core/src/audit.rs:450-980`

- [ ] **Step 1: Add the bundled SQLite test dependency**

Add the test-only dependency that backs the in-memory migration coverage:

```toml
[dev-dependencies]
rusqlite = { version = "0.31", features = ["bundled"] }
```

Then refresh `Cargo.lock` so the final changed-file set includes both
`crates/coven-threads-core/Cargo.toml` and `Cargo.lock`.

- [ ] **Step 2: Add representative legacy fixtures and pragma helpers**

Inside `audit.rs`'s `tests` module, add the representative v0.1.3 fixture,
shared constants, and helper functions that match the final tests:

```rust
const LEGACY_WARD_AUDIT_SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS ward_audit (
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
    recorded_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX IF NOT EXISTS ward_audit_familiar_idx ON ward_audit (familiar_id, recorded_at);
CREATE INDEX IF NOT EXISTS ward_audit_event_idx    ON ward_audit (event_type, recorded_at);

CREATE TRIGGER IF NOT EXISTS ward_audit_append_only_update
BEFORE UPDATE ON ward_audit
BEGIN
    SELECT RAISE(ABORT, 'ward_audit is append-only (RFC-0001 §5.6)');
END;

CREATE TRIGGER IF NOT EXISTS ward_audit_append_only_delete
BEFORE DELETE ON ward_audit
BEGIN
    SELECT RAISE(ABORT, 'ward_audit is append-only (RFC-0001 §5.6)');
END;
"#;

fn user_version(conn: &Connection) -> i64 {
    conn.pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap()
}

fn set_user_version(conn: &Connection, version: i64) {
    conn.pragma_update(None, "user_version", version).unwrap();
}
```

Keep the legacy insert path separate from the current-schema insert helper: the
legacy row must omit `detail`, while the current helper inserts an
`apply_audit` row with populated `detail`.

- [ ] **Step 3: Add the fresh-schema preservation tests**

Match the final test names exactly by routing both cases through the shared
helper:

```rust
fn assert_fresh_schema_preserves_user_version(initial_version: i64) {
    let conn = Connection::open_in_memory().unwrap();
    set_user_version(&conn, initial_version);
    conn.execute_batch(WARD_AUDIT_SCHEMA_SQL).unwrap();

    assert_eq!(user_version(&conn), initial_version);
    assert_current_ward_audit_shape(&conn);

    let row_id = insert_current_apply_audit_row(&conn);
    let row = load_audit_row(&conn, row_id);
    assert_eq!(row.event_type, "apply_audit");
    assert_eq!(row.detail.as_deref(), Some(FIXED_PREV_DETAIL));
}

#[test]
fn fresh_schema_preserves_user_version_zero_and_creates_current_shape() {
    assert_fresh_schema_preserves_user_version(0);
}

#[test]
fn fresh_schema_preserves_user_version_ninety_nine_and_creates_current_shape() {
    assert_fresh_schema_preserves_user_version(99);
}
```

These tests must verify both outcomes: the schema keeps the unrelated
store-wide `user_version` unchanged, and the resulting table accepts
`apply_audit` with populated `detail`.

- [ ] **Step 4: Add the current-schema rejection test**

Use the current test name and assertions:

```rust
#[test]
fn migration_rejects_current_schema_rows_with_detail_and_preserves_state() {
    let conn = Connection::open_in_memory().unwrap();
    set_user_version(&conn, 37);
    conn.execute_batch(WARD_AUDIT_SCHEMA_SQL).unwrap();
    assert_current_ward_audit_shape(&conn);
    let row_id = insert_current_apply_audit_row(&conn);
    let before = load_audit_row(&conn, row_id);
    let before_version = user_version(&conn);

    let err = conn
        .execute_batch(WARD_AUDIT_MIGRATION_V014_SQL)
        .expect_err("current-schema migration must fail on duplicate detail");
    assert!(err.to_string().contains("duplicate column name: detail"));
    let _ = conn.execute_batch("ROLLBACK;");

    assert_eq!(load_audit_row(&conn, row_id), before);
    assert_current_ward_audit_shape(&conn);
    assert_eq!(user_version(&conn), before_version);
}
```

- [ ] **Step 5: Add the legacy-upgrade and rerun tests**

Add the two remaining migration tests with their final names:

1. `legacy_schema_upgrades_and_preserves_append_only_behavior`
   - seed the exact legacy table shape;
   - preset an unrelated database-wide `user_version` to `37` via
     `set_user_version(&conn, 37)`;
   - insert one legacy `ward_updated` row without `detail`;
   - run `WARD_AUDIT_MIGRATION_V014_SQL`;
   - assert the migrated row preserves every original field, now reports
     `detail == None`, the rebuilt table accepts `apply_audit`, and the
     unrelated `user_version` is still `37`;
   - insert a current `apply_audit` row and assert its populated `detail`
     survives;
   - assert both append-only triggers reject `UPDATE` and `DELETE`.
2. `rerunning_migration_after_legacy_upgrade_errors_and_preserves_rows`
   - migrate the legacy fixture once successfully;
   - insert a current `apply_audit` row after the successful upgrade;
   - rerun `WARD_AUDIT_MIGRATION_V014_SQL`, assert the duplicate-`detail`
     failure, roll back the failed transaction, and confirm both rows and the
     unrelated `user_version` remain unchanged.

- [ ] **Step 6: Run the focused tests**

Run:

```bash
cargo test -p coven-threads-core audit::tests -- --nocapture
```

Expected after Tasks 1 and 2: all audit tests pass, including the five migration
coverage tests above.

### Task 2: Make the migration preserve evidence and fail closed

**Files:**
- Modify: `CHANGELOG.md`
- Modify: `crates/coven-threads-core/src/audit.rs:309-430`
- Modify: `docs/superpowers/specs/2026-07-19-apply-audit-migration-repair-design.md`
- Modify: `docs/superpowers/plans/2026-07-19-apply-audit-migration-repair.md`

- [ ] **Step 1: Guard the legacy migration before destructive work**

Immediately after `BEGIN;` in `WARD_AUDIT_MIGRATION_V014_SQL`, add:

```sql
ALTER TABLE ward_audit ADD COLUMN detail TEXT;
```

This succeeds only for the exact v0.1.3 shape. A current schema or repeated
upgrade fails before `DROP TABLE ward_audit` can run.

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

- [ ] **Step 4: Preserve database-wide version state**

Remove the database-wide version writes from `WARD_AUDIT_SCHEMA_SQL` and
`WARD_AUDIT_MIGRATION_V014_SQL`. Fresh DDL and the legacy repair must leave
unrelated `user_version` state untouched; migration choice belongs solely to the
observed `ward_audit` table shape.

- [ ] **Step 5: Correct the migration documentation**

Update the module docs, constant comments, and design doc to state:

- new stores receive the current DDL without mutating database-wide
  `user_version`;
- the legacy repair is only for the exact v0.1.3 shape: `ward_audit` exists,
  `detail` is absent, and the table CHECK does not contain `apply_audit`;
- current v0.1.4 or partial/unknown shapes fail closed rather than running the
  repair;
- the migration adds `detail`, copies every row including `detail`, and
  preserves unrelated database-wide `user_version`;
- current-schema invocation or rerun fails before destructive work;
- callers must roll back a failed migration transaction before continuing.

- [ ] **Step 6: Run focused tests**

Run:

```bash
cargo test -p coven-threads-core audit::tests -- --nocapture
```

Expected: all audit tests pass.

- [ ] **Step 7: Commit the repair**

```bash
git add CHANGELOG.md \
  docs/superpowers/specs/2026-07-19-apply-audit-migration-repair-design.md \
  docs/superpowers/plans/2026-07-19-apply-audit-migration-repair.md \
  crates/coven-threads-core/Cargo.toml \
  Cargo.lock \
  crates/coven-threads-core/src/audit.rs
git commit -m "fix(audit): keep migration versioning table-local" \
  -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

### Task 3: Validate and prepare PR #7 update

**Files:**
- Verify: `CHANGELOG.md`
- Verify: `docs/superpowers/specs/2026-07-19-apply-audit-migration-repair-design.md`
- Verify: `docs/superpowers/plans/2026-07-19-apply-audit-migration-repair.md`
- Verify: `crates/coven-threads-core/Cargo.toml`
- Verify: `Cargo.lock`
- Verify: `crates/coven-threads-core/src/audit.rs`

- [ ] **Step 1: Run repository quality gates**

Run:

```bash
cargo fmt --all -- --check
cargo test -p coven-threads-core audit::tests -- --nocapture
```

Expected: both commands exit successfully.

- [ ] **Step 2: Review the final diff**

Run:

```bash
git diff origin/cody/apply-audit-v014...HEAD --check
git diff origin/cody/apply-audit-v014...HEAD --stat
git status --short --branch
```

Expected: only `CHANGELOG.md`,
`docs/superpowers/specs/2026-07-19-apply-audit-migration-repair-design.md`,
`docs/superpowers/plans/2026-07-19-apply-audit-migration-repair.md`,
`crates/coven-threads-core/Cargo.toml`, `Cargo.lock`, and
`crates/coven-threads-core/src/audit.rs` are changed; the worktree is clean
after commits.

- [ ] **Step 3: Re-check remote branch drift**

Run:

```bash
git fetch origin cody/apply-audit-v014
git log --oneline --left-right HEAD...origin/cody/apply-audit-v014
```

Expected: no remote-only commits. If remote-only commits exist, stop and
reconcile before pushing.

- [ ] **Step 4: Update PR #7 and publish review evidence**

Push the worktree tip to the existing PR branch only if it is a fast-forward:

```bash
git push origin HEAD:cody/apply-audit-v014
```

Then update PR #7 with the root cause, the fail-closed migration design, and
the exact quality gates that passed.
