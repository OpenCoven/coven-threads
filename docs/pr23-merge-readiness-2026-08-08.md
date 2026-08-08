# Merge Readiness — PR #23 (audit schema fingerprint gating)

**Date:** 2026-08-08
**Prepared by:** Echo
**Bead:** `threads-3jx`
**Status:** RECOMMENDATION — not a merge, not an approval

---

## What this is

A verification packet for `threads-3jx`, whose acceptance requires PR #23 merged and adopted. I verified what I could verify myself and marked what I could not. **Merge authority is not mine**; this is evidence for whoever holds it.

## Verified this session

Head `87e944ba2c9e40d316307386d2d167bcad18f342`, branch `merge/pr18-onto-main` → `main`.

| check | result | how |
|---|---|---|
| mergeable | `true` / `clean` | GitHub REST |
| current main contained | yes — `e7610a0` is an ancestor of the head | `git merge-base --is-ancestor` |
| CI | `cargo test (workspace)` success, 2026-08-03, **at this exact head** | REST check-runs |
| local tests | **211 + 17 + 4 + 14 passed, 0 failed**, 1 declared doc-test ignore | `cargo test --workspace` in `.worktrees/pr-23-audit` |
| `cargo fmt --all --check` | clean | local |
| draft | no | REST |
| scope | 9 files, +3590/−359, 4 commits | REST |

### The three criteria I previously could not confirm

In earlier notes I flagged four-state fingerprint classification, TEMP shadowing, and concurrent-startup fail-closed as PR-body claims I had not proved. **All three now have named passing tests**, observed directly in local output:

- **Four states** — `exact_current_schema_returns_current_v020`, `exact_legacy_fixture_returns_legacy_v013`, absent-path cases, and a large family of `*_is_unknown` tests (extra index, extra unique, DESC index, collated index, altered trigger body, altered trigger error literal, spaced event-type literal, extra table check, missing append-only trigger).
- **TEMP shadowing** — `legacy_main_with_temp_shadow_is_unknown_and_migration_rejects_before_mutating_either_schema`; `current_main_with_temp_shadow_is_unknown_and_guards_preserve_main_and_temp_rows`.
- **Concurrent startup** — `concurrent_schema_initialization_serializes_without_locked_errors`, alongside `schema_and_migration_sql_use_begin_immediate`.

Also observed: `legacy_schema_sql_rejects_and_rollback_preserves_state`, `rerunning_migration_after_legacy_upgrade_errors_and_preserves_rows`, and `legacy_plus_extra_column_and_data_is_unknown_and_guard_preserves_state` — these speak to "unknown schemas are never rebuilt or stamped."

I read test *names and pass status*, not test *bodies*. A named passing test is strong evidence, not proof the assertion is the right one.

## Two findings

### 1. No APPROVED review exists — and none is possible

All three reviews on PR #23 are `COMMENTED`, never `APPROVED`:

- `copilot-pull-request-reviewer[bot]` COMMENTED 2026-07-22
- `BunsDev` COMMENTED 2026-08-03 (×2)

I had been describing this PR as "review threads resolved," which is true and *not the same thing* as approved.

**CORRECTED 2026-08-08 (same day, after posting the request):** I first recommended obtaining an APPROVED GitHub review. That is not achievable in this repository and I should have checked before recommending it. `BunsDev` is the sole collaborator and the author of this PR; GitHub permits neither self-approval nor self-review-request. Verified across every recent PR — #6, #18, #23, #24, #25 — **approved count is 0 in all cases.** No PR in this repo has ever been APPROVED. Familiars act through Val's account; there is no second GitHub identity.

The repo's real mechanism is the **attestation comment**: named reviewer, exact commit, what was checked, what was found, what was explicitly not covered. Nova's 2026-07-29 coherence review used it; so did the PR #6 verification packet. That is the gate to ask for, and it is a stronger record than a green checkmark.

### 2. `cargo clippy -D warnings` fails — but not because of this PR

```
crates/coven-threads-core/src/staging.rs:127
  if raw.len() % 4 != 0
  → clippy::manual_is_multiple_of
```

**This is toolchain drift, not a PR defect.** Evidence:

- The line was introduced 2026-07-15 in `5e68957` (Phase 2 crate-side ward.audit contract) — before PR #23 existed.
- PR #23 does not touch `staging.rs`.
- The workspace sets `rust-version = "1.88"` with **no `rust-toolchain.toml`**; local is 1.95.0. `manual_is_multiple_of` post-dates 1.88.

So the epic's recorded "clippy -D warnings clean on Rust 1.88" was true when recorded. Merging PR #23 neither causes nor fixes this. Worth its own bead — an unpinned toolchain means the gate result depends on whoever runs it.

## What still remains on `threads-3jx` after a merge

Merging is necessary but **not sufficient**. The bead also requires:

- release / pin adoption
- daemon startup integration — `coven` still pins `6fa360b`
- integration tests proving adoption on current `coven` main

`threads-3jx` should **not** be closed at merge.

## Recommendation

**Merge is technically justified.** Green at the exact head, clean, contains current main, and the specific correctness claims now have passing tests. The freeze does not block it — Val ruled 2026-08-08 that freeze means phase closure, not merge, with PR #6 (`091607f`, 2026-07-27) as precedent for merging with both human gates open.

**Before merging, I would want:** a written **attestation comment** from someone who reads the audit code — Cody or Nova — naming the exact commit, what they checked, and what they did not cover. Not a GitHub APPROVED review; that is structurally unavailable here (see finding 1). Not ceremony either. This PR rewrites schema classification on the authority boundary, and the durable record should show a human affirmed the assertions, not just that CI went green. Requested on the PR 2026-08-08.

**Merging does not:** grant `threads-uqx.9`, constitute a freeze decision on `threads-uqx.10`, or close `threads-3jx`.

## Bead state at time of writing

`threads-3jx` moved `open` → `in_progress` at **2026-08-08T08:36:05Z**.

**Attribution — corrected twice, final reading last.**

*First reading:* "cannot attribute; possibly something else moved it."

*Second reading (wrong):* I found the `.beads/interactions.jsonl` field change at `08:36:05Z` sitting one second from my own merge-readiness `bd comment` at `08:36`, and concluded the likeliest cause was me.

*Final reading (evidence-backed):* **Cody claimed it and is actively working.** A worktree `.worktrees/threads-3jx` on branch `fix/threads-3jx-audit-schema` was created at `03:36–03:38` local — the same minutes as the status flip — and carries live uncommitted changes:

```
CHANGELOG.md                                   |  24 +-
crates/coven-threads-core/src/audit.rs         | 201 ++++++++++++--
docs/superpowers/plans/...migration-repair.md  |  14 +-
docs/superpowers/specs/...repair-design.md     |  46 +--
4 files changed, 226 insertions(+), 59 deletions(-)
```

The `audit.rs` work adds table-level `CHECK` constraints for `proposal_window_opened` (requiring non-null `proposal_id` and a valid `approval_path_label` in JSON `detail`) and revises the fingerprint module docs to state that both accepted `current_v020` table-SQL variants retain the Phase-5 proposal-window and memory-admission constraints, so deployed exact-current stores stay classified `current_v020`.

The timestamp coincidence with my comment was exactly that — a coincidence, within the same minute. My second reading over-corrected: having been wrong once by pointing away from myself, I swung to assuming I was the cause, and that was also wrong. **The worktree is the load-bearing evidence; the `interactions.jsonl` timestamp alone could not distinguish the two explanations.**

Nothing on the board was changed by this note. No bead was claimed or closed by me.

## Consequence for this packet

The verified results above describe head `87e944b`. **Cody's in-progress work is not in that head.** Once it lands, this packet's test evidence and the requested attestation both apply to a superseded revision and must be re-run against the new head.

— Echo 🪞
