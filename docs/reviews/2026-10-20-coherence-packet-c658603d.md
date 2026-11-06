# EVIDENCE PACKET — NOT AN ACCEPTANCE

OpenCoven/coven-threads#13 item 4 (audit completeness) re-checked at a Coven revision that contains coven#1305 (bead `threads-uqx.21`). Assembled by Echo from two read-only parts plus Echo's own CI and state checks. No tests were run locally; test results come only from the CI runs cited. This packet does not accept, reject or defer #13.

## Revisions (Set C)

| Repo | SHA | How chosen |
|---|---|---|
| Coven | `c658603d22b880fd680529d3552dae45fede633b` | `main` on 2026-10-20 (committed 05:51:30Z, #1415). GitHub reports it verified. It contains #1305's merge `c33da615` (`git merge-base --is-ancestor`). It is 100 commits after Set B `f772b513`. |
| Threads core | `78e47c9d2430582128679fd4a54f9966f2f4f976` | `Cargo.lock` pin at `c658603d`. Unchanged from Set B. |
| Cave | `681130f3` | Last `main` commit at or before 05:51:30Z (committed 2026-10-19T22:30:15Z, #6102). `c60bbef9` (Set B) is an ancestor, 88 commits earlier. Threads-facing files are byte-identical apart from `turbopackIgnore` build hints in `coven-daemon.ts` (part 2 §3). |

Val's scope ruling (2026-10-14) applies: **OS crash and power loss are in scope for item 4.**

## Rulings and follow-ups recorded for this packet

- **2026-10-28, in an Echo session, Val accepted the decision-origin quarantine residual** (below, "The two residuals under Decision 5") as a **named residual, not a gap** under Decision 5. Nothing is applied in that case. This records how one residual is classified. It is not an acceptance, rejection or deferral of item 4 or #13, which remain Val's own decision on GitHub. Decision 5 itself is still unmerged in #86.
- The residual is filed as [OpenCoven/coven#1618](https://github.com/OpenCoven/coven/issues/1618). Fixing it removes the residual; the residual does not block item 4.
- The direct-write and legacy-lane findings (outside item 4) are filed as [OpenCoven/coven#1619](https://github.com/OpenCoven/coven/issues/1619).
- **Both findings were re-checked on Coven `main` `1552ab62` on 2026-10-28.** `threads_gate.rs` and `store.rs` are byte-identical to `c658603d`. `api.rs` was refactored by #1456, but the decision reservation (`api.rs:9251`, purpose `L9270`) still commits at NORMAL before the claim is published (`L9272`), and the `decision-origin` quarantine is at `L9561-9570`. The direct-write Gate-4 persistence is at `L6620`, before the 200 at `L6698`. Note that `1552ab62` pins Threads `7b5a9e68`, not `78e47c9d`. This packet's verdicts are for Set C only.

## Relationship to the 2026-10-28 current-coherence packet

[coven-threads#96](https://github.com/OpenCoven/coven-threads/pull/96) landed `docs/reviews/2026-10-28-current-coherence.md` for #91, at Coven `1552ab62`. That packet is the current eight-item ledger. This packet is the line-level invariant review of item 4 at an earlier, CI-complete pin (`c658603d`, where macOS Rust tests ran and passed), with a delta check of the other seven items. The two agree on #1305 and its platform limits. This packet adds:

- the crash-point table and every `ward_audit` writer, classified durable or NORMAL;
- the decision-origin residual (coven#1618) and the direct-write and legacy-lane findings (coven#1619), which #96 does not name;
- Val's 2026-10-28 classification of the first.

## Summary of proposed verdicts (Set B → Set C)

| # | Item | Set B (`f772b513`) | Set C (`c658603d`) | Part |
|---|---|---|---|---|
| 1 | RFC round-trip | supported | supported. Cited sources are byte-identical. | 2 |
| 2 | Gate-4 fail-closed | supported | supported. #1305 only adds syncs and error returns on the approval path. The four in-apply authority checks are intact (`ward.rs:3323`, `3339`, `3437`, `3491`). Exact-SHA CI now includes macOS. | 2 |
| 3 | Descriptor-not-authority | supported | supported. Route count unchanged (84). The CX-1e refusal is byte-identical (Echo diffed it: 23 lines, `api.rs:11430` at B, `11441` at C). Two reviewer notes, neither a counterexample: `coven familiar create` (#1288) is a new owner CLI writer outside the Threads gate, create-only, the same class as `ward migrate`; `GET /main-session/events` (#1307) takes no transport authority, which is outside Threads scope. | 2 |
| 4 | Audit completeness | partial under both wordings (power-loss gap, coven#1300) | **Original wording: partial**, for the unchanged Set A reasons only. **Decision 5 with power loss in scope: supported, with a no-effect residual**, which Val accepted as a named residual on 2026-10-28 (coven#1618). | 1 |
| 5 | evidence_replay_hash | supported | supported. Byte-identical. | 2 |
| 6 | min_visible | supported (daemon) | supported (daemon). Cave visibility is still not found (coven-cave#5256). | 2 |
| 7 | Label-variant mapping | supported | supported. Both #1276 tests are present (`ward.rs:6923`, `6957`). | 2 |
| 8 | Advisory probes | supported (vacuous at the daemon) | supported (vacuous). #1306 only swapped the YAML library behind one parse probe. | 2 |

**Item 4 in one paragraph.** #1305 closes the Set B power-loss gap for every applied change. The apply intent is committed with `synchronous=FULL` (`api.rs:16048`) before the claim's applying state is synced and before any target swap. The terminal is durable before the 200. The claim's rename and removal are ordered after the database. The filesystem-dependent false `proposal_rejected / evidence_diverged` close can no longer happen. A NORMAL opening written earlier in the same request becomes durable with the FULL apply-intent commit, because SQLite recovers a WAL prefix (part 1 §4.2, §5). Lane 3 of `protected_approval_survives_loss_of_unsynced_wal_suffix` tests that same-request veto case.

**The two residuals under Decision 5. Neither involves an applied change:**

1. **Decision-origin quarantine.** The decision reservation (`api.rs:10454-10474`) commits at NORMAL, but the claim holding the decision request is fully synced. Power loss between them leaves a durable claim with no reservation. The next attempt quarantines it as `decision-origin` (`api.rs:10764-10774`). The census reports `QuarantinedOpening`. If the NORMAL opening was lost in the same cut, the result is instead a durable `proposal_submitted` with no opening, which the census does not list. Proposed fix: commit the reservation through `store::durable_transaction` before publishing the claim, as the scheduled lane already does (`threads_gate.rs:1602`).
2. **The same case, read against Decision 5's wording.** Decision 5 calls a deployed state-(3) window "a defect to investigate". Here power loss alone can reach state (3), with no daemon defect beyond the NORMAL reservation.

**Outside item 4, but found on the way:**

- **Direct (non-proposal) Ward writes** return 200 after a NORMAL `apply_audit` commit (`threads_gate.rs:1040-1077`, called from `api.rs:6629-6706`). So power loss can keep a direct Ward effect without its Gate-4 audit row. That contradicts the intent stated at `api.rs:6615-6618`.
- **The legacy lane** (no approval tiers, `stage_legacy_pending_proposal`, `threads_gate.rs:1348`) writes `proposal_submitted` at NORMAL and then returns 202. The scheduled lane is fixed.

**Test gaps (part 1 §7.2).** The approval matrix covers approve only: there is no reject or veto terminal under power loss. The harness never drops an unsynced file or directory change, so the closure of the false-reject case rests on reading the code, not on a test. No test asserts `fullfsync=ON` on ordinary connections.

## Echo spot-checks (2026-10-22)

All at `c658603d` unless noted:

- `append_proposal_apply_intent` wraps its INSERT in `store::durable_transaction` (`api.rs:16048`ff). On the decision path, the intent (`L12579`) comes before `persist_proposal_applying_state` (`L12591`), which writes the claim through the synced writer, then `apply_after_review_approval` (`L12653`), then `finalize_approved_proposal` (`L12764`).
- `WardAuditReservation::acquire` (`store.rs:787`) uses a plain `BEGIN IMMEDIATE` (`L873`), not the durable path, so it commits at NORMAL. The `decision-origin` quarantine exists at `api.rs:10764-10774`: "persisted decision request had no pre-existing durable authority".
- `stage_legacy_pending_proposal` (`threads_gate.rs:1348`) inserts `proposal_submitted` with a plain `conn.execute` (`L1391`).
- `persist_apply_audit_records_on_connection` (`threads_gate.rs:1040`) uses plain `BEGIN IMMEDIATE` / `COMMIT` when it owns the transaction. The Gate-4 comment is at `api.rs:6615-6618`.
- `api_power_loss_tests.rs` decides only `"approve"` (`L75`, `L77`, `L207`, `L224`, `L328`).
- The CX-1e refusal block is identical at B `api.rs:11430` and C `api.rs:11441` (23 lines, `diff` empty). The #1276 tests are at `ward.rs:6923` and `6957`. `proposal_scheduler.rs` and `ward_decision.rs` have identical blob ids at both SHAs. `ward_audit_census.rs` changed (`0d6b6f83` → `3d9d8dda`, the #1305 registry bookkeeping).
- Cave `681130f3` is the newest `main` commit at or before the Coven pin (`gh api …/commits?until=…`), and `c60bbef9` is its ancestor.

## Exact-SHA CI at `c658603d` (Echo)

Two runs exist for this exact SHA, and both succeeded:

| Run | Event | Conclusion |
|---|---|---|
| 39377159532 | `merge_group` (`gh-readonly-queue/main/pr-1415-…`) | success |
| 39379174287 | `push` (`main`) | success |

The push run includes `Rust tests (macOS)` (success). At Set B the only macOS job was cancelled, so this is the first macOS result for this evidence chain. In the merge-group run, the macOS jobs were skipped by change classification.

**Power-loss and durability tests, read from the job logs** (`gh run view --log`). Each test below printed `... ok` in every listed job:

| Test | `path:line` at `c658603d` |
|---|---|
| `protected_approval_survives_loss_of_unsynced_wal_suffix` | `crates/coven-cli/src/api_power_loss_tests.rs:2` |
| `scheduled_submission_survives_loss_of_unsynced_wal_suffix` | `api_power_loss_tests.rs:162` |
| `failed_authority_sync_never_acknowledges_an_unrecorded_apply` | `api_power_loss_tests.rs:214` |
| `finalization_keeps_baseline_and_terminal_atomic_and_durable` | `api_power_loss_tests.rs:257` |
| `terminal_retry_does_not_acknowledge_a_failed_directory_barrier` | `api_power_loss_tests.rs:324` |
| `failed_post_commit_directory_barrier_is_retried` | `store/durability.rs:118` |
| `authority_transaction_restores_normal_on_success_error_and_unwind` | `store/durability.rs:152` |
| `directory_barrier_retains_identity_after_path_replacement` | `durable_fs.rs:431` |
| `failed_publication_barrier_leaves_recoverable_file_and_returns_error` | `proposal_store.rs:454` |

Jobs where all nine passed: push `Rust tests (Linux)` 118484108480, `Rust tests (Windows)` 118484108688, `Rust tests (macOS)` 118484108703; merge-group `Rust tests (Linux)` 118477819242, `Rust tests (Windows)` 118477819336.

The `api_power_loss_tests.rs` cases are compiled into `api.rs`'s `#[cfg(test)] mod tests` (`api.rs:17402-17404`); `power_loss_tests` is `#[cfg(test)]` in `main.rs:80-81`.

**Threads suites** (same command lines as Set B):

| Job | `--bin coven threads_` | four real-daemon suites | Set B (for comparison) |
|---|---|---|---|
| push `Threads tests (Linux)` 118484108453 | 230 passed | 108 / 45 / 39 / 34 | 105 / 45 / 39 / 34 |
| merge-group `Threads tests (Linux)` 118477819310 | 230 passed | 108 / 45 / 39 / 34 | |
| push `Threads daemon tests (Windows)` 118484108630 | 221 passed | 85 / 44 / 35 / 33 | 82 / 44 / 35 / 33 |
| merge-group `Threads daemon tests (Windows)` 118477819191 | 221 passed | 85 / 44 / 35 / 33 | |

0 failed and 0 ignored in every suite. Threads-side CI is unchanged from Set B because the pin is unchanged (Threads CI 35110172749; its daemon-compatibility job 35110172753 still exercises the Set A daemon).

## State of the item-4 wording (Echo, 2026-10-20)

- coven-threads#86 is **open and unmerged** (`mergeable_state=clean`). Its only content commit is `fb0a2627` (2026-10-11). Today's head `67c65373` (05:30:28Z) is a merge of `main` into the branch, which brought in only the Set B packet file. Decision 5's text is unchanged. BunsDev was assigned at 05:30:20Z.
- coven-threads#13 and #14 are open.

## Item-4 file delta since Set B (Echo)

Commits in `f772b513..c658603d` that touch the item-4 files (`ward_audit_census.rs`, `proposal_scheduler.rs`, `threads_gate.rs`, `proposal_store.rs`, `ward_decision.rs`, `ward_identity.rs`, `threads_clock.rs`, `store.rs`, `store/schema.rs`, `store/durability.rs`, `durable_fs.rs`, `ward.rs`):

- `c33da615` #1305: protected Ward writes made durable across power loss
- `bf3d9885` #1311: tolerate concurrent directory publication marker opens
- `305f7573` #1320: trace durable protected-refusal operations
- `67464b89` #1321, `c1c35239` #1324, `fc50f2bd` #1374, `699f6039` #1390: main-session and automation features

`api.rs` commits in range: #1305, #1307 (main-session SSE), #1324, #1381 (automation diagnostics).
---

<!-- ===== PART 1: item 4 crash-point re-walk and every ward_audit writer ===== -->

# EVIDENCE — NOT AN ACCEPTANCE

OpenCoven/coven-threads#13 item 4 ("audit completeness") at Set C.

- Set C: coven `c658603d22b880fd680529d3552dae45fede633b` (main, 2026-10-20T05:51:30Z). Threads core pin unchanged: `78e47c9d`.
- Set B: coven `f772b51377f88d97c9d44d87093e5b641231972e`.
- Fix under review: coven#1305, merge `c33da61549d880acab45aaaeb45e945cf686ec95` (ancestor of `c658603d`).
- Prepared by Echo for Val, 2026-10-20. Read-only review; nothing built or run.

All `path:line` references are at `c658603d` unless marked otherwise.

---

## 0. Durability primitives at Set C

`git merge-base --is-ancestor` confirms Set B `f772b513` → `c33da615` (#1305) → Set C `c658603d`. #1305 is a single-parent (squash) merge on `b4b37b98`.

| Primitive | Where | What it does |
| --- | --- | --- |
| Ordinary writable connection | `store/schema.rs:626-631` (`WRITABLE_CONNECTION_PRAGMAS`), applied at `L602` and `L662` | `synchronous = NORMAL`, **`fullfsync = ON`** (new in #1305). A NORMAL commit writes the WAL with no fsync. |
| `store::durable_transaction` | `store/durability.rs:9-80`, re-exported `store.rs:26` | If the connection is already inside a transaction, it requires `synchronous >= 2 && fullfsync` and joins it (`L13-19`); a NORMAL outer transaction is refused. Otherwise: set `synchronous=FULL; fullfsync=ON` (`L26`), open the db directory and its parent (`L32-48`), `BEGIN IMMEDIATE` (`L52`), write, `COMMIT` (`L58`), then checked `sync_directory` on the db directory (`L61-65`) and the home's parent (`L66-70`), then restore NORMAL (`L74-77`). A failed directory barrier returns `Err` **after** COMMIT succeeded (test `L118-149`). |
| `store::durable_barrier` | `store/durability.rs:85-91` | A `durable_transaction` that upserts `store_meta.ward_durability_barrier` with fresh random bytes, so a real FULL commit (and WAL fsync) happens even when the evidence row already exists. |
| `durable_fs::sync_file` | `durable_fs.rs:166-196` | `sync_all` on a retained handle (Windows: `ReOpenFile` for write first). `F_FULLFSYNC` on macOS per the comment at `L35-36` (Rust std behaviour, not re-verified). |
| `durable_fs::sync_directory` | `durable_fs.rs:353-369` → native `L12-41` (Unix: `openat(fd, ".")` then `sync_all`), `L43-157` (Windows: refuses non-NTFS at `L89-92`, `NtCreateFile` relative reopen, `FlushFileBuffers`) | Checked directory barrier. Any failure is an error. |
| `durable_fs::sync_parent` | `durable_fs.rs:198-210` | `sync_directory(open_directory(path.parent()))`. It reopens the parent **by path**, not a retained handle. |
| `durable_fs::create_dir_all` | `durable_fs.rs:216-311` | Marker in `.coven-directory-publications` before `mkdir`; child and parent `sync_directory` (`L290-293`); marker removed only after both. |
| `proposal_store::sync_existing` | `proposal_store.rs:415-438` | Opens the claim no-follow, `sync_file`, then `sync_directory` of its parent. |

SQLite semantics relied on below (SQLite documentation, not re-verified against the bundled build): in WAL mode `synchronous=FULL` fsyncs the WAL at every commit; `NORMAL` fsyncs it only before a checkpoint; WAL recovery after power loss replays the longest valid prefix of committed frames. `docs/ward-durability.md:49` claims the locked bundled build does not disable sync or directory sync; that was **not verified** here.

## 1. Post-#1305 drift (`c33da615..c658603d`)

82 commits. `git diff --stat c33da615 c658603d` over the twelve listed paths:

| File | Δ | Content | Touches durability or `ward_audit`? |
| --- | --- | --- | --- |
| `api.rs` | +104/−2 | `/api/v1/actions` short-circuit for `coven.automations.doctor.v1` (`api.rs:925-931`); `hub::complete_job` gains a `runtime` argument (`api.rs:1362`); `serialize_event_with_limit` becomes `pub(crate)` (`api.rs:5302`); three new doctor tests in `mod tests` (`api.rs:17417-17507`). | **No.** No line touches `ward_audit`, the decision route, the scheduler or a durable primitive. |
| `durable_fs.rs` | +84/−7 | `checkpoint(Phase::DirectoryNative*)` diagnostics in both native barriers (`L15-39`, `L64-149`); Windows close error captured before diagnostics (`L146`); `create_dir_all` retries `ENOENT` from the marker open up to 15 times (`L260-273`); two tests (`L384-428`). | **Durability, behaviour-preserving.** The barriers still sync the same objects and still fail closed. The retry only changes when a publication *succeeds* under concurrent creators; persistent `ENOENT` still errors. |
| `store/durability.rs` | +24/−1 | `checkpoint(Phase::Durable*)` diagnostics throughout `transaction` (`L10-78`). Restore order: `restore.restore()` result is captured, the guard is dropped (which re-runs restore), then the error is returned (`L73-77`). | **Durability, behaviour-preserving.** Same pragmas, same BEGIN IMMEDIATE / COMMIT / two directory barriers, same order. |
| `store/schema.rs` | +57/−1 | `hub_jobs` delivery columns and an immutability trigger (`schema.rs:445-455`, `L1341-1383`); `main_session_compactions` schema (`L541-543`); `automations::runs::ensure_delivery_columns` (`L565`). | **No.** No `ward_audit`, reservation or pragma change. |
| `ward.rs`, `proposal_store.rs`, `threads_gate.rs`, `store.rs`, `ward_audit_census.rs`, `proposal_scheduler.rs`, `power_loss_tests.rs`, `api_power_loss_tests.rs` | 0 | — | Unchanged since #1305. |

### 1.1 Commits in range that touch these paths

`git log c33da615..c658603d -- <the twelve paths>` lists eight commits:

| Commit | Subject | Assessment |
| --- | --- | --- |
| `bf3d9885` (#1311, parent `7d7a0411`) | fix: tolerate concurrent directory publication marker opens | One file, `durable_fs.rs` +60/−4. Replaces a single `registry.open_with(name, …)` with a loop that retries only `ErrorKind::NotFound`, at most 15 times, through the same retained registry and the same no-follow options (`durable_fs.rs:260-273`). Adds `concurrent_directory_publication_succeeds_for_every_creator` (`L384-412`: 32 rounds × 8 threads; every creator succeeds, directory exists, marker gone) and `publication_marker_symlink_is_refused_without_touching_target` (`L414-428`). The mkdir, the child and parent `sync_directory` (`L290-293`) and the marker removal after them (`L306-310`) are unchanged. **No `ward_audit` write. Durability is not weakened:** the retry happens before the lock and mkdir, and a persistent `ENOENT` still fails closed. |
| `305f7573` (#1320, parent `60ed9cf2`) | chore: trace durable protected-refusal operations | `durable_fs.rs` +27, `store/durability.rs` +25, `threads_request_diagnostics.rs` +60 (new `Phase::Durable*` and `Phase::DirectoryNative*` variants), `tests/support/threads_output_auto_cases.rs` +80. **It adds or changes no `ward_audit` or refusal write.** Every source change is a `checkpoint(Phase::…)` call, plus two error-handling refinements: the Windows `CloseHandle` error is captured before diagnostics can clobber `GetLastError` (`durable_fs.rs:144-155`), and `RestoreMode` is explicitly dropped before the restore error is returned (`store/durability.rs:73-77`). Its new test `output_auto_protected_refusal_records_durable_audit_phases` asserts that the **existing** protected-target refusal writes exactly one `ward_audit` row with `decision='protected-target-not-proposable'`, and that the request log shows one full durable sequence (mode → dir open → parent open → BEGIN → write → COMMIT → dir sync → parent sync → restore) and two native directory barriers. That writer is `api.rs:6196-6212`: `append_proposal_decision_audit` with `AuditEventType::ProposalRejected`, which is a terminal and so takes the `durable_transaction` branch (§4, W7). **That refusal row is durable.** Its reservation is acquired at `api.rs:6181-6195` and finished at `L6213`, both at NORMAL. |
| `3fddd860` (#1307) | stream main-session events with bounded SSE | Only `api.rs` hunk is `serialize_event_with_limit` visibility (`@@ -5292`). No `ward_audit`/reservation/durable/pragma line in the commit. |
| `67464b89` (#1321) | deliver automation run results into the main session | No hunk in `api.rs` or `store.rs`; schema columns for automations only. No `ward_audit`/reservation/durable/pragma line. |
| `c1c35239` (#1324) | post hub job result summaries into the main session | `api.rs` hunk `@@ -1352` only (`complete_job` gains `runtime`); `hub_jobs` columns and trigger in `schema.rs`. No `ward_audit` line. |
| `fc50f2bd` (#1374) | compact Claude's Home natively at 60% fill | `main_session_compactions` schema. No `ward_audit`/durable line. |
| `e0c2f199` (#1381) | owner-local automation diagnostic observations | `api.rs` hunks `@@ -922` (doctor short-circuit) and `@@ -17407` (tests). The doctor opens the store `SQLITE_OPEN_READ_ONLY` (`automations/doctor_read.rs:54-56`). The single keyword hit in the commit is the word "asynchronous" in a plan document. No `ward_audit` write. |
| `699f6039` (#1390) | flush compaction facts to the daily note | `schema.rs` compaction columns. No `ward_audit`/durable line. |

Method for the "no" rows: `git show <c>` filtered to added/removed lines matching `ward_audit|WardAuditReservation|coven_ward_audit|durable_transaction|durable_barrier|synchronous|fullfsync|decide_threads_proposal|finalize_approved|append_proposal|ensure_proposal_window|apply_intent`, plus the `api.rs`/`store.rs` hunk headers. None of #1307, #1321, #1324, #1374, #1381 or #1390 changes the approve-and-apply path.

Conclusion: the item-4 code at Set C is the #1305 code. Everything below about the approve path, writers, census and tests is #1305 as merged, plus #1311's retry and #1320's diagnostics.

## 2. Order of operations at Set C (`ApprovedApplyMode::Initial`)

All `api.rs` lines are at `c658603d`. "NORMAL" means the COMMIT returns after `write()` to the WAL with no fsync. "FULL" means a `store::durable_transaction` (§0): WAL fsync at COMMIT with `fullfsync=ON`, then checked syncs of the db directory and its parent. Function `decide_threads_proposal_inner` spans `api.rs:10203-12808`.

| # | Step | Code | What is made durable, and how |
| --- | --- | --- | --- |
| 1 | In-process Ward write/audit lock; open store | `api.rs:10213-10215`, `10301` | — |
| 2 | Scheduled-submission reconciliation for this proposal | `api.rs:10302-10307` → `13481-13691` | Reservation `DELETE`s: NORMAL. A missing receipt is re-appended through `append_scheduled_submission_audit`: FULL (`api.rs:13656` → `threads_gate.rs:1679`). |
| 3 | Terminal-first check. If a terminal exists: `durable_barrier`, release reservations, clean artifacts, idempotent 200 | `api.rs:10309-10339` (barrier `L10310`, release `L10311` → `14853-14870`, 200 `L10329`) | **FULL barrier before the idempotent 200.** The barrier is a fresh FULL commit after the terminal, so the terminal is durable by WAL prefix even if its own post-COMMIT directory barrier had failed. Reservation release: NORMAL. |
| 4 | Expiry supersession of reservation purposes (expiry only) | `api.rs:10434-10439` → `14829-14851` | NORMAL |
| 5 | Capacity reservation row `proposal:{id}:{decision}`; purpose bound to the planned request | `WardAuditReservation::acquire` `api.rs:10454` → `store.rs:787-1047` (`BEGIN IMMEDIATE` `L873`, `INSERT` `L956-962`, `COMMIT` `L969`); `replace_purpose` `api.rs:10473` → `store.rs:1053-1072` | **NORMAL.** This row is also the decision-origin authority read by `trusted_decision_approver` (`api.rs:15165-15188`). |
| 6 | Decision claim: pending file rewritten with `decision_request`, then renamed to `….{decision}.deciding` | `PendingDecisionClaim::acquire` `api.rs:10475` → `14973-15046`; `replace_existing{,_for_terminal_decision}` (`api.rs:15024-15030`) → `write_atomic` (`proposal_store.rs:301-328`: stage `sync_all` `L318`, rename `L320`, `sync_parent` `L322`); `rename_existing` (`api.rs:15031` → `proposal_store.rs:385-402`: rename `L394`, `sync_parent` `L401`) | **File and directory synced** (checked). Not ordered after any WAL sync: the reservation from step 5 is still NORMAL here. |
| 7 | Submission-receipt validation; decision-origin checks | `api.rs:10620-10689`; `L10764-10774` (persisted request with a non-reused reservation → quarantine `decision-origin`); `L10829-10876` (`trusted_decision_approver`) | read-only, or quarantine (`proposal_store::quarantine`, `proposal_store.rs:148-183`, renames synced at `L171`, `L180-181`). No audit row. |
| 8 | Decision-time `proposal_window_opened`, veto paths only, if no scheduler pass wrote it | `api.rs:11114-11127` → `ensure_proposal_window_opened_audit_with_conn` `api.rs:13750-13830` (`INSERT … WHERE NOT EXISTS` `L13805-13828`) | **NORMAL** (autocommit `conn.execute`). |
| 9 | Revalidation: identity, auto-regression evidence, live tier, before-images, approval policy; second terminal check | `api.rs:11128-12166`; terminal re-check with `durable_barrier` `L11539-11570` | read-only; a typed rejection terminal is FULL (§4, W7); a `validation_verdict` refusal is NORMAL (§4, W6) and is followed by `restore_pending`, not by an effect. |
| 10 | **Apply-intent row** (`event_type='validation_verdict'`, `decision='proposal-apply-intent'`) | `api.rs:12579-12588` → `append_proposal_apply_intent` `api.rs:16035-16063`, `store::durable_transaction` `L16048` | **FULL.** Because the WAL is one file and recovery replays a prefix, this fsync also makes durable every earlier NORMAL commit on this database: the step-5 reservation and purpose, the step-8 opening, and any scheduler-written opening (§4.2). |
| 11 | Claim rewritten with `ProposalApplyingState` | `api.rs:12590-12599` → `persist_proposal_applying_state` `api.rs:16023-16031` → `replace_existing` → `write_atomic` | **File and directory synced**, strictly after step 10 returned. |
| 12 | `ApplyIntentBeforeWrite` failpoint; `ward_config_is_unchanged`; test-only pause | `api.rs:12601-12604`, `12605-12624`, `12625-12629` | — (`pause_threads_final_commit_fixture` is a no-op outside the test clock, `api.rs:15793-15795`) |
| 13 | **Target write**, `ApprovedApplyMode::Initial` | `apply_after_review_approval` `api.rs:12653-12665` → `write_atomically_if_unchanged` `ward.rs:3228-3607` | New parent components: each child and parent directory synced (`prepare_staging_parent`, `ward.rs:5894-5900`). Each target staged and `sync_all`ed (`stage_contents`, `ward.rs:4747-4759`). Swap: `renameat(…, EXCHANGE)` on Linux and macOS (`ward.rs:5676-5696`), two moves on Windows (`L5627-5645`), or `hard_link_without_replace` for a created target (`L3345`). **Final verification syncs each target's retained file identity and its retained parent directory** (`ward.rs:3467-3468`) before the last authority check (`L3491`). Authority checks also run before the first swap (`L3323`), before each swap (`L3339`) and after each swap (`L3437`). A rollback syncs the restored file and its directory before reporting success (`ward.rs:3625-3630`). |
| 14 | `ApplyBeforeAudit` failpoint; store identity check | `api.rs:12759`, `12760-12763` (`verify_store_path`, `store.rs:1090-1111`, read-only) | — |
| 15 | **`finalize_approved_proposal`**: apply audit rows, baseline advance, `proposal_approved` terminal carrying typed `window_close` | `api.rs:12764-12786` → `api.rs:16804-16851`, `store::durable_transaction` `L16808`; rows `threads_gate.rs:1085-1154` (`INSERT` `L1100`); baseline `threads_gate.rs:942-957` → `store_baseline` `L907-930` (`ward_manifest` upsert); terminal `api.rs:16831-16848` → `16861-16952` (its own `durable_transaction` at `L16948` joins the outer FULL transaction, `store/durability.rs:13-19`) | **FULL**, one atomic transaction. A failure, including a failed post-COMMIT directory barrier, returns `proposal_audit_persist_failed_response` (`api.rs:12784-12785`), never 200. |
| 16 | Reservation release (`DELETE` + COMMIT) | `api.rs:12787` → `store.rs:1113-1143` | NORMAL |
| 17 | `AuditBeforeCleanup` failpoint; claim removal | `api.rs:12788`; `claim.consume()` `api.rs:12789` → `api.rs:15053-15058` → `remove_existing` (`proposal_store.rs:404-412`: unlink `L409`, `sync_parent` `L411`) | **Directory synced.** A failed barrier returns `Err`, not 200. |
| 18 | HTTP 200 | `api.rs:12790-12807` | — |

**Recovery mode** (`ApprovedApplyMode::Recovery`, `api.rs:12179-12428`) differs as follows:

- The claim already carries `ProposalApplyingState`. Before anything else the recorded intent must exist and equal it, with a matching actor, or the claim is quarantined with `proposal-recovery-intent-unverifiable` / `proposal-recovery-decision-actor-invalid` (`api.rs:11571-11622`). No audit row is written by that quarantine.
- After `verify_recoverable_apply_state` (`L12180-12198`) and `ward_config_is_unchanged` (`L12204-12213`), it runs **`store::durable_barrier` (`L12223`) and `proposal_store::sync_existing` (`L12224`, `proposal_store.rs:415-438`) before any target is touched.** This makes an intent written by an older NORMAL-mode build durable, and re-syncs the claim file and its directory. The comment at `L12219-12220` states this intent.
- Recovery accepts already-applied bytes (`ward.rs:3259-3260`, `L3270`) and, at final verification, syncs the retained identity of an already-applied target and its parent directory (`ward.rs:3444-3468`) before finalizing.
- Then the same FULL `finalize_approved_proposal` (`api.rs:12384-12406`), NORMAL reservation release (`L12407`), synced claim removal (`L12409`), and 200 (`L12427`).

Non-approve 200s in the same function: `api.rs:10329` and `L11560` (idempotent, each preceded by `durable_barrier`), `L11419` (protected-target rejection, FULL terminal at `L11398-11414`) and `L12018` (rejection or veto terminal, FULL at `L11969-11985`). No 200 in `decide_threads_proposal_inner` is returned before a FULL commit that follows the terminal.

## 3. Crash-point table at Set C

Definitions are the same as Set B §2.4. "Process crash": the daemon dies, the OS keeps every completed `write()`, COMMIT and rename. "Power loss": OS crash or power cut; SQLite recovers the longest valid WAL prefix; a file or directory change survives for certain only once its checked sync has returned. All power-loss statements assume storage that honours successful flushes (`docs/ward-durability.md:76-78`). Step numbers are §2's. Census classification is `ward_audit_census.rs:312-394`; it lists only histories with an opening (`L232-235`) and ignores `validation_verdict` rows except to collect apply intents (`L191-196`).

**Process-crash column: unchanged from Set B.** #1305 did not reorder any step. It added barriers (FULL commits, file and directory syncs) between existing steps, a FULL barrier before each idempotent terminal reply (`api.rs:10310`, `11540`), and a barrier pair before Recovery applies (`api.rs:12223-12224`). Under a process crash those only change whether a request can fail at a barrier. A barrier failure returns `Err` and is retried; `terminal_retry_does_not_acknowledge_a_failed_directory_barrier` (`api_power_loss_tests.rs:324-334`) shows the retry. Census logic is unchanged apart from skipping the publication registry (`ward_audit_census.rs:420-425`).

| Crash after step | Process crash (Set B = Set C) | Power loss, Set B (for reference) | Power loss, Set C |
| --- | --- | --- | --- |
| 1-5 (reservation only, no claim) | Nothing to resume; the next request starts over. | Reservation may be lost. | Same as Set B. Nothing protected exists yet. |
| 6-9 (claim with `decision_request` published, no intent) | Claim recovered; the decision resumes. Census `OpenUnverified` if an opening exists. | Claim rename not directory-synced; reservation and a same-request opening may be lost. | The claim and its request are durable (step 6). **The step-5 reservation row and a NORMAL opening may still be lost**, unless some later FULL commit or a checkpoint on the same database happened first. On retry the claim is found with `request_preexisting` true and a freshly inserted, non-reused reservation, so the decision quarantines it: `proposal-decision-origin-invalid` (`api.rs:10764-10774`; or `api.rs:15184-15188` → `L10860-10869`). **No effect has been written.** The ledger keeps the FULL `proposal_submitted` with no close. Census: `QuarantinedOpening` if an opening is durable, otherwise nothing. New finding; see §8 residuals. |
| 10 (intent committed, claim has no applying state) | Intent present, target untouched. Census `UnprovableApply` until a decision completes. | Intent may be lost. | **Same as process crash.** The intent is FULL; its WAL fsync also carried the reservation and any opening (§4.2). |
| 11-12 (applying state persisted, no swap) | Recovery verifies the intent (`api.rs:11571-11622`) and applies once. Census `UnprovableApply` until then. | Intent may be lost while the applying-state claim survives → quarantine `proposal-recovery-intent-unverifiable`; the census lists nothing if there is no opening. | **Same as process crash.** The intent was durable before step 11 started, and step 11 is synced before step 13 starts. The Set B quarantine-by-power-loss case is closed. |
| **13-14 (targets swapped, finalize not committed)** | Recovery accepts already-applied bytes (`ward.rs:3259-3277`) and finalizes. Census `UnprovableApply` until then. Designed recovery window. | **Hazard:** target bytes survive, intent, baseline and terminal all lost; quarantine or a false `evidence_diverged` close. | **Same as process crash.** The intent and the applying-state claim are both durable before the first swap. A swap that had not yet reached its final-verification sync (`ward.rs:3467-3468`) may itself be lost; Recovery then sees the before-image and applies it again (Recovery accepts either the before-image or the approved bytes per target). One sub-case was **not traced**: a Windows two-step move (`ward.rs:5627-5645`) cut between its two renames. It is the same under a process crash. |
| 15 (finalize committed) | Terminal found first (`api.rs:10309-10339`); idempotent; artifacts cleaned. `TypedTerminalRecorded`. | Finalize may be lost as a WAL suffix; same as rows 13-14. Also, the terminal could survive while an un-synced target rename was lost. | **Durable.** Finalize returns only after the FULL COMMIT and both directory barriers. The targets were file- and directory-synced at step 13, before finalize began, so the reverse case (terminal present, target holding old bytes) is also closed. `TypedTerminalRecorded`. |
| 16-18 (reservation released, claim removed, 200 returned) | Complete. | **Client saw 200** while intent, apply rows, baseline and terminal could all be lost. | **Terminal durable before 200.** The NORMAL reservation release may be lost, and the row then reappears. Only a later decision request for the same proposal reclaims it (`api.rs:10311`, `11548`); no other reclaim path was found (see §9). That is capacity bookkeeping, not audit. The claim removal is synced before 200. If it is lost (cut between unlink and directory sync), the claim reappears beside a durable terminal; the terminal-first path cleans it up, and the census says `TypedTerminalRecorded` (terminal outranks artifacts, `ward_audit_census.rs:369-372`). |
| Rejection or veto terminal written (`api.rs:11969-11985`, `L11010-11032`, `L11398-11414`, `L12135-12153`, and `append_open_window_revalidation_failure` `L16570-16607`) | `TypedTerminalRecorded`; claim consumed after. | Terminal may be lost after the client saw 200/409-terminal. | **Durable before the response.** All of these go through W7's terminal branch (§4). |
| Refusal after intent with rollback (`api.rs:12681-12709`, `12740-12757`) | Intent plus refusal row; claim restored to pending; census `UnprovableApply` with a pending artifact. | Intent may be lost. | The intent is durable. The rollback is synced (`ward.rs:3625-3630`) before the refusal row. The refusal row (`validation_verdict`, NORMAL) and the reservation release may be lost; `restore_pending` is synced. The census result is identical, because it ignores `validation_verdict` rows. No effect survives. |

### 3.1 The four specific questions

**(a) Can the target bytes survive while the apply-intent row is lost? No**, given flush-honouring storage. `append_proposal_apply_intent` commits FULL (`api.rs:16048`): WAL fsync with `F_FULLFSYNC` on macOS, then the db-directory and parent barriers. It returns before `persist_proposal_applying_state` (`api.rs:12590`), and that returns before the first target rename (`ward.rs:3343`/`L3345`). A failure at any barrier returns `Err` before any swap. In Recovery mode, `durable_barrier` (`api.rs:12223`) re-establishes the same ordering for an intent written by an older build. That intent could already have been lost under Set B. If it was, Recovery quarantines rather than applies (`api.rs:11574-11584`).

**(b) Can the client see 200 while finalize or the terminal is lost? No.** The approve 200s (`api.rs:12807`, `12427`) follow a successful FULL `finalize_approved_proposal`. The idempotent 200s (`L10329`, `L11560`) follow a fresh `durable_barrier`, which covers a terminal whose own directory barrier failed after COMMIT. The rejection 200s (`L11419`, `L12018`) follow a FULL terminal. Test: `failed_authority_sync_never_acknowledges_an_unrecorded_apply` (`api_power_loss_tests.rs:214-254`).

**(c) Can the claim rename or claim removal be lost or reordered relative to the DB?**

- **Claim creation (step 6): yes, one reorder remains.** The claim file and its rename are synced, but the reservation row that authorizes its `decision_request` is NORMAL and is not synced first. Power loss in steps 6-9 can keep the claim and lose the reservation. The result is a fail-closed quarantine (`decision-origin`), not an effect. The same applies to the expiry claim path (`api.rs:14939-14951`) and its NORMAL purpose supersession (`api.rs:14829-14851`).
- **Applying-state rewrite (step 11):** strictly after a FULL intent, so it cannot get ahead of the DB.
- **Claim removal (step 17):** strictly after the FULL terminal. If the removal is lost, the result is harmless (row 16-18).
- **`restore_pending` after a synced rollback:** may get ahead of its NORMAL refusal row and reservation release. No effect, and the census result is the same.

**(d) The filesystem-dependent false `proposal_rejected / evidence_diverged` case: closed.** That rejection runs only when the claim carries no applying state (`api.rs:12108-12165`, guard `L12108`). `proposal_recovery_is_proven_unapplied` returns true only in that case too (`api.rs:10062-10064`). At Set C, the applying state is on stable storage before the first swap (step 11), and a claim is rewritten without it only by `restore_pending`, which runs after a synced rollback. So, with flushes honoured, a claim without applying state cannot coexist with surviving approved bytes. This depends on the same storage assumption as everything else here. It is **not tested** (§7).

## 4. Every `ward_audit` writer

Method: `git grep` at `c658603d` for `INSERT INTO ward_audit`, `INSERT INTO coven_ward_audit_reservations`, `UPDATE coven_ward_audit_reservations`, `DELETE FROM coven_ward_audit_reservations`, and the callers of every enclosing function. Test-only regions were excluded: `api.rs` `mod tests` from `L17402`, `store.rs` `mod tests` from `L5139`, `threads_gate.rs` `mod tests` from `L1753`, `ward_audit_census.rs` `mod tests` from `L467`, and `*_tests.rs` files. Production `INSERT INTO ward_audit` sites are exactly eight: `api.rs:13806`, `16050`, `16926`; `threads_gate.rs:985`, `1100`, `1392`, `1681`; `store.rs:3602`. No other production file writes `ward_audit`. `daemon.rs:16577` is a test fixture; `observe.rs` only reads.

### 4.1 Writer table

| ID | Function and `path:line` | Event type(s) | Callers | Commit mode |
| --- | --- | --- | --- | --- |
| W1 | `append_scheduled_submission_audit`, `threads_gate.rs:1647-1702` (INSERT `L1681`) | `proposal_submitted` (`decision='staged:scheduled'`) | `stage_scheduled_coherence_proposal` `threads_gate.rs:1614`; receipt recovery in `reconcile_scheduled_submission_reservations_with_conn` `api.rs:13656` | **FULL** (`durable_transaction`, `threads_gate.rs:1679`) |
| W2 | `stage_legacy_pending_proposal`, `threads_gate.rs:1348-1412` (INSERT `L1391-1410`) | `proposal_submitted` (`staged:coherence` / `staged:authority`) | `stage_coherence_proposal` when Ward has no compiled approval tiers (`threads_gate.rs:1236-1256`), reached from `api.rs:6365` and `6545`; `gate_protected_edits` degrade path `threads_gate.rs:450`, unreachable in production because its only caller passes `gated_targets: &[]` (`api.rs:6285`) | **NORMAL** (autocommit). The request then runs the NORMAL reservation `finish` and returns **202** (`api.rs:6583-6600`, `6396-6407`). |
| W3 | `ensure_proposal_window_opened_audit_with_conn`, `api.rs:13750-13830` (INSERT `L13805-13828`); wrapper `ensure_proposal_window_opened_audit` `api.rs:13732-13748` opens its own connection | `proposal_window_opened` (`decision='window-opened'`). Only when the proposal has a veto deadline (`L13758-13762`) and exactly one receipt (`scheduled_submission_authority`, `L13767-13772`). | Scheduler `api.rs:13196` (veto path, not yet opened), `13264` (protected or revalidation-required), `13366` (every scheduled proposal before the due check; a no-op without a veto deadline); decision time `api.rs:11118` (veto path, not yet opened) | **NORMAL** (autocommit) at every call site |
| W4 | `append_audit_row`, `threads_gate.rs:959-1011` (INSERT `L985`) | `validation_verdict` per target (reject / degrade / permit) | `gate_protected_edits` `threads_gate.rs:385` (identity rejection) and `L415` (gated targets; unreachable as above); `api.rs:12439` (authority-lane revalidation per gated target at decision time, before the apply intent; a later FULL intent or terminal in the same request carries it, `L12448-12460`) | **NORMAL** |
| W5 | `append_proposal_apply_intent`, `api.rs:16035-16063` (INSERT `L16050`) | `validation_verdict` with `decision='proposal-apply-intent'` | `api.rs:12579` (Initial) | **FULL** (`api.rs:16048`) |
| W6 | `append_proposal_refusal_audit`, `api.rs:16743-16770` → W7 | `validation_verdict`, `decision='proposal-revalidation-failed'` | `api.rs:11722`, `11823`, `11945`, `12080`, `12358`, `12473`, `12697`, `12741` | **NORMAL** (W7's non-terminal branch) |
| W7 | `append_proposal_decision_audit_with_probe_summary`, `api.rs:16861-16952` (INSERT `L16926`); wrapper `append_proposal_decision_audit` `L16853-16859` | Terminal: `proposal_approved`, `proposal_rejected`, `proposal_vetoed` (`L16867-16872`). Non-terminal: anything else. | Terminal callers: `api.rs:6196` (protected-target refusal, `ProposalRejected`); `11010` (AUTO regression, `ProposalRejected`); `11398` (protected target at decision, `ProposalRejected`); `11702`, `11780`, `11900`, `12048`, `12135` (`ProposalRejected`); `11969` (`decision_semantics.rejection_event`: `ProposalRejected` or `ProposalVetoed`, `api.rs:7860-7968`); `16590` (`append_open_window_revalidation_failure`, `ProposalRejected`, called from `api.rs:10709`, `10975`, `11149`, `11217`, `11279`, `11325`, `12289`, `12451`, `12684`); `16831` (`ProposalApproved`, inside finalize). **Only non-terminal caller:** W6 (`ValidationVerdict`). | Terminal: **FULL** (`L16947-16948`); inside finalize it joins the outer FULL transaction (`store/durability.rs:13-19`). Non-terminal: **NORMAL** (`L16950`). |
| W8 | `append_apply_audit_records_at`, `threads_gate.rs:1085-1154` (INSERT `L1100`) | `apply_audit` per logged write (`WardAuditRecord::for_apply`) | `finalize_approved_proposal` `api.rs:16809`; direct path via W10 | Proposal path: **FULL** (outer, `api.rs:16808`). Direct path: see W10. |
| W9 | `advance_surface_baseline_from_bytes`, `threads_gate.rs:942-957` → `store_baseline` `L907-930` | Not `ward_audit`: `ward_manifest` upsert (baseline) | `finalize_approved_proposal` `api.rs:16823`; `advance_surface_baseline` `threads_gate.rs:932-940` ← `api.rs:6735` | Proposal path: **FULL** (outer). Direct path: NORMAL. |
| W10 | `persist_apply_audit_records_on_connection`, `threads_gate.rs:1040-1077` | `apply_audit` (via W8) | Direct (non-proposal) apply, `api.rs:6629-6641` | **NORMAL** (`BEGIN IMMEDIATE … COMMIT`, `L1051-1070`). Followed by NORMAL baseline advance (`api.rs:6643`, `6717-6740`), NORMAL reservation finish (`L6668`), then 200 (`L6706`). |
| W11 | `append_compaction_ledger_events`, `store.rs:3583-3626` (INSERT `L3602`) | `compaction_ledger` (`proposal_id` NULL) | `vacuum_store_path` `store.rs:3553` | **NORMAL**; preceded by `VACUUM` (`L3533`) and a `wal_checkpoint(TRUNCATE)` (`L3551`) |

Reservation writers (`coven_ward_audit_reservations`):

| ID | Function and `path:line` | Commit mode |
| --- | --- | --- |
| R1 | `WardAuditReservation::acquire` INSERT/resize, `store.rs:873-974`; activation-failure restore `L991-1028` | NORMAL |
| R2 | `replace_purpose`, `store.rs:1053-1072` | NORMAL at the decision path (`api.rs:10473`). **FULL** only at the scheduled-submission binding (`threads_gate.rs:1602-1606`). |
| R3 | `finish`, `store.rs:1113-1143`; `release_best_effort` (Drop), `L1162-1180` | NORMAL |
| R4 | `release_superseded_proposal_reservations` `api.rs:14718-14737`; `mark_proposal_reservations_superseded_by_expiry` `L14829-14851`; `release_terminal_proposal_reservations` `L14853-14871` | NORMAL |
| R5 | Scheduled-submission reconciliation `DELETE`s, `api.rs:13514`, `13532`, `13551`, `13576`, `13601`, `13683` | NORMAL |
| R6 | Capacity trigger `coven_ward_audit_capacity_insert` (`store/schema.rs:797-880`): on every `ward_audit` INSERT it decrements the active reservation (`L869`) and updates capacity (`L876`) | Same transaction as the triggering INSERT, so it inherits that INSERT's mode |

### 4.2 The WAL-prefix argument, made explicit

Every writer above uses the same SQLite file, `store_path(coven_home)`; `ward_audit`, the reservations and `ward_manifest` are tables in it. In WAL mode all connections append commit frames to one `-wal` file, in commit order, under the single writer lock. A FULL commit fsyncs that file (with `F_FULLFSYNC` on macOS because `fullfsync=ON`), and `fsync` covers the whole file, not only the new frames. After power loss, SQLite replays the longest prefix of frames with valid chained checksums.

So once a FULL commit has returned, **every earlier commit on the database is durable, including NORMAL commits from any connection.** If a checkpoint has run since, those earlier frames are already in the main database file. Under NORMAL, SQLite syncs the WAL before a checkpoint and the database after it (SQLite documentation; not re-verified against the bundled build). The argument needs two conditions: the NORMAL commit precedes the FULL commit in WAL order, and the FULL commit returned successfully.

What this gives at Set C:

- Opening (W3) written at decision time (step 8) or by any earlier scheduler pass → durable no later than the apply intent (W5, step 10) or the rejection/veto terminal (W7) of that request.
- Decision reservation and purpose (R1/R2, step 5) → durable no later than the apply intent or terminal.
- Scheduled reservation INSERT (R1 at `api.rs:6261`) → durable at the FULL binding (R2 at `threads_gate.rs:1602`).

It does **not** help a NORMAL row that is followed by an acknowledgement or an effect *before* any FULL commit.

### 4.3 NORMAL writers, tested against (i)-(iii)

| Writer | (i) A protected effect it justifies can survive its loss? | (ii) A response acknowledging it can already have been returned? | (iii) A later durable row that depends on it can survive its loss? |
| --- | --- | --- | --- |
| W2 legacy `proposal_submitted` | No. The proposal stays pending. A later decision with no receipt quarantines it (`legacy_submission_authority` `api.rs:13853-13919`, `legacy_submission_preflight_response` `L13947-13984`) and applies nothing. | **Yes.** 202 at `api.rs:6600`/`6407` follows only NORMAL commits. The pending file is synced (`threads_gate.rs:1319` → `publish_new`). After power loss the client's staged proposal can lose its receipt and later be quarantined. Legacy proposals never open a window, so this is outside the census. | No. Any later decision row is FULL and would follow it in WAL order. A decision on a receipt-less legacy proposal quarantines first. |
| W3 opening | No. Every effect is preceded in the same request by a FULL commit (intent, or the Recovery `durable_barrier`). | Not as an audit acknowledgement. Scheduler passes return no response. The decision-time opening is followed by a FULL terminal or intent, or else by a non-terminal 409 (`restore_pending` paths) with no effect. Whether any API surface displays an opening before it is durable was **not traced**. | **No**, by §4.2. Intent and terminal follow it in WAL order. A lost opening with no later FULL commit is rewritten by the next scheduler pass (`api.rs:13194-13219`) with the same `deadline`/`earliest_close` (they come from the classification, `L13758-13762`) but a new `decided_at`. |
| W4 gate verdicts | No. At intake it is a refusal (403, `api.rs:6309-6317`). At decision time (`api.rs:12439`) the apply intent that precedes any effect is FULL and carries it. | Yes, at intake and at the non-terminal decision 409 (`api.rs:12483-12492`). The verdict is informational. | No. |
| W6 refusal `validation_verdict` | No. It follows a synced rollback or no write at all. | Yes: non-terminal 409s. | No. The census ignores `validation_verdict` rows except the intent marker (`ward_audit_census.rs:191-196`). |
| W10 direct apply records (+ W9 direct baseline) | **Yes**, for direct (non-proposal) writes to non-protected Tier-1/2 surfaces. The staged file is synced (`ward.rs:4740-4741`), but the swap in `write_direct_batch` (`ward.rs:2620-2840`) has no directory sync after it, and the audit is NORMAL. Either can survive without the other. | **Yes.** 200 at `api.rs:6706` follows only NORMAL commits. | No. |
| W11 compaction ledger | No. | `vacuum_store_path` returns a report; caller not traced. | No. |
| R1-R5 reservations | No. | No audit acknowledgement depends on them. | **Yes, in one case.** The decision reservation (R1/R2, step 5) is the decision-origin authority for a claim's persisted `decision_request`. The claim is synced (step 6) but the reservation is not. Losing it makes a durable claim unusable: fail-closed quarantine (§3 row 6-9). The scheduled submission binding avoids this by being FULL before publication. |

Item-4 conclusion for this section: **every row in the proposal window chain that an effect or a 200 depends on is FULL, or is carried by a FULL commit as a WAL prefix, before the effect or the 200.** The NORMAL writers that remain either guard nothing durable (W3, W4, W6, R*), sit outside the window chain (W2 legacy lane, W10 direct writes, W11), or cause a fail-closed quarantine without an effect (R1/R2 vs. the claim).

## 5. Opening lost with the effect

**Set B finding.** A veto path whose opening was first written in the same request could lose the opening, intent, baseline and terminal together, while the effect survived. The census then listed nothing.

**At Set C the opening writer itself is still NORMAL.** W3 is a plain autocommit `conn.execute` at every call site (`api.rs:13805-13828`; call sites `11118`, `13196`, `13264`, `13366`). It is **not** wrapped in `durable_transaction`.

**But the FULL apply-intent commit carries it as a WAL prefix before any effect.**

- **Decision-time opening.** Written at `api.rs:11118`, on the same `conn` that later commits the intent FULL at `api.rs:12579` → `16048`. Any opening from an earlier scheduler pass is an even earlier commit on the same database file (the wrapper's separate connection, `api.rs:13739`, writes to the same WAL). By §4.2 the intent's WAL fsync makes the opening durable before `persist_proposal_applying_state` and before the first swap.
- **Recovery.** The same holds through `durable_barrier` (`api.rs:12223`).
- **Rejection and veto.** The FULL terminal carries the opening.

So **the opening can no longer be lost together with an effect or a terminal.**

**Direct test evidence.** Lane 3 of `protected_approval_survives_loss_of_unsynced_wal_suffix` exercises exactly the Set B case.

- Its staging helper writes a receipt but no opening (`api.rs:37167-37266`). No scheduler pass runs before the decision. The WAL is checkpointed first (`api_power_loss_tests.rs:69`). So the opening is first written at `api.rs:11118` inside the decision request.
- After restoring only synced images, the census lists exactly one window at every boundary, including `ApplyIntentBeforeWrite`: `unprovable_apply` before finalize, `typed_terminal_recorded` after (`api_power_loss_tests.rs:122-131`). After recovery it is `typed_terminal_recorded` (`L149-155`).

**What remains.**

1. **The census still lists nothing for paths without an opening** (`ward_audit_census.rs:232-235`, unchanged by design). These are HumanApproval, HumanApprovalWithRationale, `AutoRegression{veto: None}` and legacy. At Set C an applied change on such a path cannot lose its intent or terminal (W5 and W7 are FULL), so nothing is hidden by the omission. The matrix asserts intent and terminal presence for those lanes directly (`api_power_loss_tests.rs:106-119`) and asserts an empty census (`L132-136`).
2. **An opening can still be lost when no effect follows.** Example: a scheduler-written opening followed by power loss before any FULL commit or checkpoint on the database. The next scheduler pass rewrites it (`api.rs:13194-13219`) with the same deadline and earliest close but a later `decided_at`. Not tested (§7).
3. **Opening and decision reservation lost together, claim durable** (§3 row 6-9). The claim is quarantined at `api.rs:10764-10774` before the decision-time opening at `L11118` is reached. The window then never becomes durable, and the ledger keeps a FULL `proposal_submitted` with no opening and no close. Nothing was applied; the census lists nothing; the quarantined file sits in `pending/quarantine`. This is a submitted-but-never-closed chain caused by power loss alone, without a daemon defect.

## 6. Submission ordering

Set B: `threads_gate.rs:1608-1612` published the pending file (synced) before a NORMAL `append_scheduled_submission_audit`, so a power loss could keep a pending proposal whose receipt was lost, and the opening writer would then refuse it.

**Scheduled lane at Set C** (route `api.rs:6251-6407`/`6545-6600` → `stage_coherence_proposal` `threads_gate.rs:1182-1260` → `stage_scheduled_coherence_proposal` `threads_gate.rs:1436-1628`):

| # | Step | Code | Durability | Power loss here leaves |
| --- | --- | --- | --- | --- |
| S1 | Reservation `direct:{uuid}` | `api.rs:6261-6275` | NORMAL | Nothing, or an orphan reservation if a later FULL commit carried it. No reclaim path was found for a non-recovery-purpose orphan (§9). |
| S2 | Identity gate (W4 rows on rejection only) | `api.rs:6277-6317` | NORMAL | No proposal. |
| S3 | Classification, probes, `pending/` directory publication | `threads_gate.rs:1446-1570` (`durable_fs::create_dir_all` `L1569`) | Directory checked-synced | No proposal. |
| S4 | **Recovery binding**: reservation purpose replaced with `ScheduledSubmissionRecovery` (proposal id, familiar, weave hash, body hash) | `threads_gate.rs:1595-1606` | **FULL** (`durable_transaction`, `L1602`). It also makes S1's INSERT durable by prefix. | Binding durable, no pending file. The next reconciliation deletes the reservation as orphaned (`api.rs:13550-13556`). |
| S5 | Pending file published | `publish_new` `threads_gate.rs:1610` → `proposal_store.rs:330-340` → `write_atomic` (`sync_all`, rename, `sync_parent`) | File and directory synced | Binding plus pending file, **no receipt**. Reconciliation verifies the body hash (`api.rs:13575-13582`) and appends the receipt through W1, FULL (`api.rs:13622-13681`). It runs at the start of every scheduler pass (`api.rs:13089`) and before every decision for that proposal (`api.rs:10302-10307`). So the scheduler's opening never meets a receipt-less pending file it would refuse. Covered by `scheduled_submission_survives_loss_of_unsynced_wal_suffix` with `before_receipt=true` (`api_power_loss_tests.rs:162-211`). |
| S6 | Test failpoint | `threads_gate.rs:1612` | — | — |
| S7 | **Receipt** `proposal_submitted` | `threads_gate.rs:1614-1621` → `L1647-1702` | **FULL** (`L1679`) | Receipt durable. The reservation still has its binding purpose. |
| S8 | Reservation finish | `api.rs:6397` / `6583` → `store.rs:1113-1143` | NORMAL | Receipt durable, binding reservation may reappear. Reconciliation sees `receipt_count > 0` and deletes it (`api.rs:13682-13687`). Covered by the same test with `before_receipt=false`. |
| S9 | HTTP 202 | `api.rs:6399-6407` / `6600` | — | **The 202 always follows a FULL receipt.** |

**Order verdict:** the pending file is still published before the receipt, but a FULL binding written before publication makes that window recoverable, and the receipt is FULL before the client is told. This matches `docs/ward-durability.md:14-15`.

**Legacy lane (no compiled approval tiers) at Set C:** pending published and synced (`threads_gate.rs:1374-1380` → `stage_pending_proposal` `L1285-1321`), then a **NORMAL** `proposal_submitted` (W2, `L1391-1410`), a NORMAL reservation finish and **202** (`api.rs:6583-6600`). There is no recovery binding. Power loss after the 202 can keep the pending file and lose its receipt. The next decision then quarantines it (`api.rs:13947-13984`, reason `submission-receipt`) without applying anything. Legacy proposals have no veto window, so this lies outside the census and the window chain, but it is an acknowledged submission whose `proposal_submitted` link can vanish. `docs/ward-durability.md:14` only claims durability for the *scheduled* lane.

## 7. Tests

Nothing was run; this lists source at `c658603d`. `api_power_loss_tests.rs` is `include!`d into `api.rs` `mod tests { mod power_loss_cases { … } }` (`api.rs:17402-17405`); its line numbers are its own. Whether CI at `c658603d` ran and passed these was **not checked**.

| Test | `path:line` | What it asserts |
| --- | --- | --- |
| Harness `PowerLoss` (test VFS) | `power_loss_tests.rs:1-206` | Wraps the default VFS. Captures the db and `-wal` bytes only at a successful `xSync` (`L71-107`). `restore` writes those images back and deletes `-shm` (`L162-185`). `fail_next_sync` injects `SQLITE_IOERR_FSYNC` (`L81-83`, `L156-158`). `sync_flags` records flags (`L159-161`). It touches no other file: targets and claims keep whatever the live filesystem has. |
| `protected_approval_survives_loss_of_unsynced_wal_suffix` | `api_power_loss_tests.rs:2-159` | 6 lanes × 4 boundaries: `ApplyIntentBeforeWrite`, `ApplyBeforeAudit`, `AuditBeforeCleanup`, none (= 200). A keeper connection stops close-time checkpoints (`L15-17`); a `TRUNCATE` checkpoint runs before the decision (`L69`); a NORMAL insert after the decision must vanish (`L87-105`), proving the suffix-loss model is live. Then: the intent survives (`L106-109`); the target is untouched by the restore (`L110`); a terminal exists iff the boundary is `AuditBeforeCleanup` or none (`L111-119`); census `typed_terminal_recorded`/`unprovable_apply` for lane 3 and empty for the rest (`L120-136`); recovery completes exactly one proposal (`L138-144`); final bytes are approved, no claim remains, exactly one `proposal_approved` (`L145-148`); lane 3 census `typed_terminal_recorded` after recovery (`L149-155`). |
| `scheduled_submission_survives_loss_of_unsynced_wal_suffix` | `api_power_loss_tests.rs:162-211` | Power loss after publication without a receipt, and after a 202. After restore: the pending file exists, exactly one receipt, and approval returns 200. |
| `failed_authority_sync_never_acknowledges_an_unrecorded_apply` | `api_power_loss_tests.rs:214-254` | An injected SQLite sync failure, before and after the apply, yields no 200 and no terminal. After the apply, the claim and intent remain. A retry returns 200. A `SQLITE_SYNC_FULL` flag was observed. |
| `finalization_keeps_baseline_and_terminal_atomic_and_durable` | `api_power_loss_tests.rs:257-321` | An aborted terminal insert leaves the baseline unchanged. A successful finalize leaves terminal and baseline present after an image restore. |
| `terminal_retry_does_not_acknowledge_a_failed_directory_barrier` | `api_power_loss_tests.rs:324-334` | 200, then a retry with a failed directory barrier errors (the `durable_barrier` at `api.rs:10310`), then 200. |
| `failed_post_commit_directory_barrier_is_retried` | `store/durability.rs:117-149` | The commit is visible even though the directory barrier failed; `barrier` repeats the failed barrier; the mode returns to NORMAL (1). |
| `authority_transaction_restores_normal_on_success_error_and_unwind` | `store/durability.rs:151-189` | NORMAL is restored after success, error and panic; failed transactions roll back; refuses to join a NORMAL outer transaction (`L182-187`). |
| `durable_fs` units | `durable_fs.rs:384-514` | Concurrent publication (`L384-411`, #1311); symlink marker refused (`L413-428`); barrier keeps directory identity after a path swap (`L430-458`); a trusted ancestor needs no parent flush (`L460-469`); an interrupted publication repeats its barrier (`L471-484`); case-insensitive marker equivalence (`L486-502`); a failed creation barrier publishes no children (`L504-514`). |
| `failed_publication_barrier_leaves_recoverable_file_and_returns_error` | `proposal_store.rs:453-462` | `write_atomic` returns `Err` on a failed directory barrier although the bytes landed; `sync_existing` then succeeds. |
| `directory_publication_registry_is_bookkeeping_only_when_a_real_directory` | `ward_audit_census.rs:865-914` | A real `.coven-directory-publications` directory in `pending/` is not an unattributed artifact; a file, symlink or quarantined copy of that name is. `QuarantinedOpening` is unaffected. |
| `output_auto_protected_refusal_records_durable_audit_phases` (#1320) | `tests/support/threads_output_auto_cases.rs:168-239` | A protected refusal writes exactly one `protected-target-not-proposable` row through one complete durable sequence and two native directory barriers. Phase order only; no image restore. |
| `open_store_retains_initialization_wal_without_close_and_reopen` | `store.rs:6560`ff | Asserts `synchronous == 1` (`L6581-6584`). **No assertion of `fullfsync = ON` on ordinary connections was found.** |

### 7.1 Approval-matrix coverage

| Required case | Covered? | Evidence |
| --- | --- | --- |
| No-opening path: HumanApproval | Yes (approve only) | lane 1, `api_power_loss_tests.rs:25-26` |
| No-opening path: HumanApprovalWithRationale | Yes (approve only) | lane 2, `L27` |
| No-opening path: `AutoRegression{veto: None}` | Yes (approve only) | Lane 4, output-auto, decided by `decide_threads_proposal_automatic` (`L74-75`). The fixture's `[approval_tiers.auto]` has no veto key (`api_output_auto_tests.rs:19`), and the test asserts an empty census for it (`L132-136`). That it is exactly `veto: None` is inferred from these two facts, not read from a type. |
| Legacy and created-target lanes | Yes (approve only) | lanes 0 and 5 (`L20-23`, `L51-61`) |
| Same-request opening on a veto path | **Yes** | Lane 3: the helper writes no opening (`api.rs:37167-37266`), and no scheduler pass runs before the decision. |
| Census classification after loss | Partly | Lane 3 only, before and after recovery. Other lanes assert an empty census by design. |

### 7.2 Gaps

1. **No power-loss test for rejection or veto terminals.** The matrix is approve-only. The #1320 test checks the durable phase order for one refusal terminal, but restores no image.
2. **No test of power loss between claim publication and the first FULL commit** (steps 6-9). This is the reservation-lost, claim-kept quarantine of §3.
3. **The filesystem side of power loss is not modelled.** The harness drops only an unsynced SQLite suffix. Every file keeps its live state (`power_loss_tests.rs:162-185`; `api_power_loss_tests.rs:94-110`), so no test loses an unsynced target rename, claim rename or claim unlink. The file barriers (`ward.rs:3467-3468`, `3625-3630`, `5896-5900`; `proposal_store.rs:322`, `401`, `411`) are exercised only through injected directory-sync failures in unit tests. The §3.1(d) closure is argued from the code, not tested.
4. **No test of a NORMAL scheduler opening lost and rewritten** (new `decided_at`).
5. **No test of legacy receipt loss after a 202 (W2), or of direct-apply audit loss after a 200 (W10).**
6. **Single-target lanes only.** Multi-target batches cut between swaps, and the Windows two-step move cut mid-way, are not covered.
7. **No test recovers an intent written by a pre-#1305 NORMAL build.** `failed_authority_sync_never_acknowledges_an_unrecorded_apply` (`after_apply=true`) covers a failed barrier in Recovery, but not the upgrade case.
8. **No assertion of `fullfsync=ON`.** No platform evidence of `F_FULLFSYNC` on macOS. No physical power-cut certification; `docs/ward-durability.md:94` says so.

## 8. Proposed item-4 verdict at Set C

Scope: OS crash and power loss are in scope (Val, 2026-10-14). This is a proposal for Val, not an acceptance.

### 8.0 What changed from Set B, and what did not

- `git diff --stat f772b513 c658603d` on census, scheduler, `threads_gate.rs`, `ward_decision.rs` and `ward_identity.rs` touches only `ward_audit_census.rs` (+57: the registry skip at `L420-425` and its test) and `threads_gate.rs` (+18/−7: the #1305 durable binding, durable receipt and directory publication). The scheduler, `ward_decision.rs` and `ward_identity.rs` are unchanged.
- The census classification (`ward_audit_census.rs:312-394`) is unchanged.
- The opening writer is unchanged apart from line numbers (`api.rs:13750-13830`).
- The Threads core pin is unchanged: `78e47c9d` in `Cargo.lock:924-926` and `crates/coven-cli/Cargo.toml:72` at both SHAs. So the Set B trigger inventory (`audit.rs:1011-1029`, `L1329`, `L1358`, `L1419` at `78e47c9d`) still holds, and **there is still no SQL ordering constraint**. "Receipt before opening" remains a runtime precondition only (`scheduled_submission_authority`, message at `api.rs:14523`).

### 8.1 (a) Original wording

> "`proposal_submitted → window_opened → close(reason)` chain is present and no gap exists between open and close. Close event has reason field."

The power-loss residual that Set B added (Set B §2.4 rows 8-13) is **closed at Set C** for every window whose effect or terminal happened:

- No applied change can lose its apply intent, baseline advance or terminal (§3.1 a, b).
- The opening can no longer be lost together with an effect (§5).
- The false `proposal_rejected / evidence_diverged` close is closed (§3.1 d), so a recorded reason no longer misstates the outcome.
- Scheduled `proposal_submitted` is FULL before the 202, and is recoverable from the FULL binding before that (§6).

What still stands:

- The Set A limiters, unchanged: no SQL ordering constraint; a lazy opening (first scheduler pass or decision time), so there is an interval with no opening row; quarantined or unprovable windows are reported but not closed.
- **New, narrower power-loss residual:** a claim kept with its decision reservation lost leads to a fail-closed quarantine (§3 row 6-9). That leaves `proposal_submitted` with no close, and with no opening if the NORMAL opening was lost too. No effect is involved.

**Proposed: partial.** It is still partial for the Set A reasons, and is no longer weakened by power loss on applied changes.

### 8.2 (b) Decision 5 (coven-threads#86), power loss in scope

D5's three states: (1) `TypedTerminalRecorded`; (2) `OpenUnverified`; (3) `UnprovableApply`, `QuarantinedOpening`, `UntrustedArtifacts`, `OrphanedOpening` or `InconsistentHistory`, reported and never repaired.

- **Every opening that survives power loss lands in a D5 state.**
  - An applied change reaches (1) after recovery. The matrix shows this for the veto lane (`api_power_loss_tests.rs:122-131`, `149-155`).
  - An interrupted apply is (3) `UnprovableApply` exactly as after a process crash.
  - A claim that lost its reservation is (3) `QuarantinedOpening`.
  - `OrphanedOpening` from loss after a 200 (Set B row 11-13) is no longer reachable, because the terminal is durable before the 200.
- **Lost openings are no longer hidden behind effects.** Every effect is preceded by a FULL commit that carries the opening (§4.2, §5). The Set B "census cannot see an applied window" case is closed.
- **No false-reason closes.** §3.1(d).
- **What D5 does not name.**
  - Power loss alone, with no daemon defect, can still produce a state-(3) `QuarantinedOpening`. D5 calls a deployed state-(3) window "a defect to investigate"; here the defect is the NORMAL decision reservation, not a census defect.
  - When the scheduler's NORMAL opening is lost in the same cut, the quarantined proposal has a durable receipt and no durable opening. By D5's literal text that is not an opened window, so it is not a gap. It is still a submission the census cannot see.
  - In both cases nothing was applied.

**Proposed under D5 with power loss in scope: supported, with the two no-effect residuals above for Val to accept or route.** If Val reads "a power-loss-reachable state-(3) window" as a gap, the verdict becomes **partial**, and the fix is a small Coven change: commit the decision reservation and purpose (R1/R2 at `api.rs:10454-10474`) through `durable_transaction` before the claim is published, as the scheduled lane already does with its binding (`threads_gate.rs:1602`).

### 8.3 Residual risks

1. **Storage must honour flushes.** Lying write caches, firmware, filesystem corruption and external deletion are out of scope (`docs/ward-durability.md:76-78`). macOS relies on `F_FULLFSYNC` through Rust's `sync_all` and SQLite `fullfsync=ON`; neither was verified here against the toolchain or the bundled SQLite.
2. **Windows: directory durability is NTFS-only.** Non-NTFS volumes fail closed with "durable Ward directory writes require NTFS on Windows" (`durable_fs.rs:85-92`). That is safe, but approvals on ReFS, FAT, exFAT or network volumes become unavailable. Linux directory-fsync semantics on network or FUSE filesystems were not assessed.
3. **Deterministic tests, not physical power-cut certification** (`docs/ward-durability.md:80-94`). The harness drops only an unsynced SQLite suffix, never an unsynced file or directory change (§7.2 #3).
4. **NORMAL writers that matter:**
   - **Decision reservation vs. claim** (R1/R2, §3 row 6-9). Window-chain relevant; fail-closed.
   - **Legacy `proposal_submitted` (W2) after a 202** (§6). Outside the window chain; the proposal is later quarantined.
   - **Direct-apply `apply_audit` (W10) and direct baseline advance after a 200** (`api.rs:6629-6706`). A non-proposal Ward effect can survive without its Gate-4 audit row. This contradicts the intent stated at `api.rs:6615-6618`, though it is outside item 4's proposal chain.
   - **NORMAL openings (W3).** Safe by prefix before any effect. A lost opening is rewritten with a different `decided_at`.
5. **Lazy opening**, unchanged.
6. **No SQL ordering constraint**, unchanged since Set A/B (§8.0).
7. **Reported but never closed:** `UnprovableApply` and `QuarantinedOpening` still need manual handling.
8. **Capacity leak after power loss.** A decision reservation released at NORMAL after a 200 can reappear, and only a repeat decision request for that proposal reclaims it (`api.rs:10311`, `11548`).
9. **Set B-era history is not repaired.** Windows damaged by power loss while Set B code ran stay as recorded. Set C Recovery quarantines an applying claim whose intent is missing (`api.rs:11574-11584`) rather than inventing one.
10. **Cost of `fullfsync=ON` on every ordinary connection's checkpoints, and of FULL authority commits, was not measured.** #1320 adds tracing for #1227 latency and claims no closure.

## 9. Not found / not traced

**Not verified (external semantics relied on):**

1. SQLite WAL semantics: FULL fsyncs the WAL at every commit; NORMAL syncs the WAL before a checkpoint and the database after it; recovery replays a valid prefix. Taken from SQLite documentation and not re-verified against the bundled `rusqlite 0.40` / `libsqlite3-sys` build (`crates/coven-cli/Cargo.toml:97`). The §4.2 prefix argument depends on these.
2. The bundled SQLite build flags. `git grep` at `c658603d` for `SQLITE_DEFAULT_SYNCHRONOUS`, `SQLITE_DEFAULT_WAL_SYNCHRONOUS`, `SQLITE_DISABLE_DIRSYNC` and `SQLITE_NO_SYNC` found nothing in the coven tree. The libsqlite3-sys build script was not inspected, so `docs/ward-durability.md:49` ("does not disable sync or directory sync") is **not verified**.
3. That Rust's `File::sync_all` uses `F_FULLFSYNC` on macOS (`durable_fs.rs:35-36` comment): not verified in the toolchain.
4. Linux directory-fsync behaviour on network or FUSE filesystems: not assessed.

**Not traced in code:**

5. A Windows two-step target move (`ward.rs:5627-5645`) cut between its two renames: the Recovery outcome. It is the same under a process crash.
6. Whether any API surface (for example the proposal list behind `scheduled_decision_body`, `api.rs:37285-37307`) displays an opening, or a staged 202, before it is durable.
7. The follow-up after an apply-intent COMMIT whose post-COMMIT directory barrier failed. `append_proposal_apply_intent` returns `Err` at `api.rs:12588`, the claim guard's `Drop` renames the claim back (`api.rs:15077-15083`), and the reservation is preserved (`api.rs:10775-10777`). The next decision's handling of a pending file that already has a durable intent was not traced end to end.
8. The caller and response of `vacuum_store_path` (W11).
9. Set B carry-over: "a pinned WAL still fails closed with 507". Not re-checked.
10. Set B carry-over: the effect of an un-advanced baseline on later intake, for Set B-era damage. Not traced; at Set C the baseline advance is inside the FULL finalize, so it is moot for new approvals.

**Not found:**

11. A reclaim path for a decision reservation whose NORMAL release was lost after a 200, other than a repeat decision request for that proposal (`api.rs:10311`, `11548`). Also not found: a reclaim path for an orphaned `direct:{uuid}` intake reservation.
12. A test asserting `fullfsync = ON` on ordinary writable connections. The store test asserts only `synchronous == 1` (`store.rs:6581-6584`).
13. Power-loss tests for rejection or veto terminals; for the claim-kept, reservation-lost window; for lost file or directory changes; for a lost scheduler opening; for legacy-receipt or direct-apply audit loss; for multi-target batches; and for recovery of a pre-#1305 NORMAL intent (§7.2).
14. Exact-SHA CI results for `c658603d`: not checked (no `gh` call made for this pass).

**Inferred rather than read:**

15. That matrix lane 4 is exactly `AutoRegression{veto: None}`. This is inferred from the fixture (`api_output_auto_tests.rs:19`) and the test's empty-census assertion (`api_power_loss_tests.rs:132-136`).

**Not measured:**

16. The latency cost of FULL authority commits and of `fullfsync=ON` checkpoints.
17. How long a NORMAL decision reservation stays exposed: from claim publication until the next FULL commit or checkpoint on the database. No Coven code bounds that interval by time.

---

<!-- ===== PART 2: items 1, 2, 3, 5, 6, 7, 8 and Cave at Set C ===== -->

# EVIDENCE — NOT AN ACCEPTANCE

OpenCoven/coven-threads#13 delta check at Set C, part 2: items 1, 2, 3, 5, 6, 7, 8 (item 4 is checked separately). Prepared for Echo on behalf of Val. Read-only static reading of git objects plus read-only `gh api` GETs. No tests, builds, checkouts or fetches.

## 0. Revisions

| Repo | Set B | Set C | How fixed |
|---|---|---|---|
| Coven | `f772b51377f88d97c9d44d87093e5b641231972e` | `c658603d22b880fd680529d3552dae45fede633b` | Given. Committed 2026-10-20T05:51:30Z, "test(automations): audit SQLite corruption refusal and fixture recovery (#1415)". `git merge-base --is-ancestor f772b513 c658603d` succeeds. There are 100 first-parent commits in `f772b513..c658603d`. |
| Threads core | `78e47c9d2430582128679fd4a54f9966f2f4f976` | `78e47c9d2430582128679fd4a54f9966f2f4f976` (**unchanged**) | `Cargo.lock` L924-926 is identical at both Coven SHAs: `name = "coven-threads-core"`, `version = "0.2.0"`, `source = "git+https://github.com/OpenCoven/coven-threads?rev=78e47c9d…#78e47c9d…"`. The `Cargo.lock` blob changed (`b132278087da…` → `4f64b911121f…`), but not this entry. Every Threads-core citation therefore carries by construction. No Threads file was re-read. |
| Cave | `c60bbef90bdb859f4cb4f2fb44685eb02d939b71` | `681130f34e1f4bf2fe1736c6b23ee2613f9ce14b` | This is the last first-parent `main` commit at or before 2026-10-20T05:51:30Z. See §3. |

**Range facts used below:**

- **#1305 (`c33da61549d880acab45aaaeb45e945cf686ec95`)** is a **single-parent squash commit** (parent `b4b37b98`), not a merge commit. It was committed 2026-10-15T08:16:49Z. It touches `api.rs`, `ward.rs`, `threads_gate.rs`, `proposal_store.rs`, `store.rs`, `store/schema.rs`, `ward_audit_census.rs`, `eval_loop.rs`, `daemon.rs` and `main.rs`. It adds `durable_fs.rs`, `store/durability.rs`, `api_power_loss_tests.rs`, `power_loss_tests.rs` and `docs/ward-durability.md`.
- **The only `api.rs` commits in range** are #1305 `c33da615`, #1307 `3fddd860`, #1324 `c1c35239` and #1381 `e0c2f199` (`git log f772b513..c658603d -- crates/coven-cli/src/api.rs`).

**Method.**

- Read-only `git show/diff/grep/log/rev-parse/ls-tree/merge-base` on the local Coven and Cave clones, with no fetch or checkout. The Cave objects for both SHAs were already present locally.
- Read-only `gh api` GETs.
- Cited Set B ranges were mapped to Set C by an `awk` pass over `git diff -U0` hunk headers, then byte-compared with `diff <(git show B:… | sed -n a,bp) <(git show C:… | sed -n c,dp)`. A range is called "identical" only when that diff is empty.
- **No tests, builds or CI were run.**
- One read-only `gh api …/actions/runs/<id>/jobs` call was **declined by the operator** and was not retried, so CI is reported at the run level only (§2, item 2).

Refs below are `coven@c658603d:crates/coven-cli/src/<file>` ("C") or `coven@f772b513:…` ("B") unless marked.

## 1. File-level delta (Set B → Set C)

`git diff --stat f772b513 c658603d -- crates/ tests/ specs/`: 138 files, +50800/−1180, all under `crates/`. No top-level `tests/` or `specs/` path changed.

The table below lists every Coven file the Set B packet cites for items 1, 2, 3, 5, 6, 7 and 8. Paths are under `crates/coven-cli/` unless shown otherwise. Blob ids are full; "Δ" is `git diff --shortstat`.

**Changed files:**

| File | Blob @ f772b513 | Blob @ c658603d | Δ / commits in range | Items citing |
|---|---|---|---|---|
| `src/api.rs` | `44107c309f8345d87738f1dede9a75ce7d472a1b` | `232c93e653eb6770cfd9fba136902f01ae7ff2b4` | +160/−44; #1305, #1307, #1324, #1381 | 1, 2, 3, 5, 6, 7, 8 |
| `src/ward.rs` | `d6daa42627749a4d464c1f868ee9692c0bb1ee0c` | `79969744415bef886b3923c6892fc45d5fbc0954` | +17/−1; #1305 | 1, 2, 3, 6, 7, 8 |
| `src/threads_gate.rs` | `91d90f540cda84387d2698944ddd3b8f6b133f23` | `304e95a366416c448e5c5e4ca1c45d4cabdb8cf0` | +11/−7; #1305 | 3 (and 2 via apply audit) |
| `src/proposal_store.rs` | `8f68a942d8ae14fbedaaefcebcbdac0254b4b297` | `18d34390f970681823cddca2b771de9247effbf2` | +67/−6; #1305 | 2, 3 |
| `src/ward_probes.rs` | `2e586483ade72287d5da7ff7c719dff2537d7a44` | `8c0c49e7de0a43d7ae3b6fff130bff25013ccc8c` | +66/−2; #1306 `921b44f5` | 3, 8 |
| `src/api_routes.rs` | `eefb3d8dfb1201916f2ed8645e4de8d92ab22c65` | `2378c04f34d33bc0bd82b20717d075da7e22d7b7` | +4/−2 (doc comment only); #1307 | 3 |
| `src/ward_audit_census.rs` | `0d6b6f835d1c850741f60b1464a3ae9ae839ffdc` | `3d9d8dda474906aa71aa0ec5a849b2c914578495` | +57; #1305 | 3 (consumer); item 4 |
| `src/memory_import.rs` | `387ff707fec2bf1f92a344970123b1a508e2f7b5` | `a16a1445c396a0ac39d1900c935b06fcda9f1d72` | +151/−14, tests only; #1337 | 3 (S4) |
| `src/daemon.rs` | `a4d8752eb056b8a4bae1248f0a2c43f77182da27` | `1ae4ef97da62af5cca5c45e66ce03af506394769` | +733/−109; 8 commits incl. #1305, #1307, #1356 | 3 |
| `src/store.rs` | `1ca2aba723cd16b57f2f32f05881747c5caae4f9` | `d2f9986153ecf7754dd192e77c201ec8627b0b5c` | +8/−2; #1305 | 2, 3 (EP-18, EP-44) |
| `src/store/schema.rs` | `d9439e50de513e133c7d4482cbae07185d4316fb` | `0b39516e95da08633e2da2faeb727bd53324a8c7` | +61/−1; #1305, #1321, #1324, #1374, #1390 | 2 (#1132 context); item 4 |
| `src/cockpit_sources.rs` | `84459d8b2f4dee2a7570d6c0920cb9309b59ae92` | `5af56b99aa4bd8cb9c4c1fca51cf4bc5a5eaa4dc` | +10/−4; #1288 `b14f8228` | 3 (EP-30, NEP-06) |
| `src/eval_loop.rs` | `fdbfae74fadc897e002bedb904a50cfb40aab91b` | `5b9655bea767e20fb79647516d00e0589da9d930` | +3/−1; #1305 | 3 (EP-91) |
| `src/control_plane.rs` | `09ef8db888900abf2c680f4f9b044080e9c015f1` | `0e5b91070b84aa5900fa439802c279e47065cb7d` | +385/−24; 7 automations/missions commits | 3 (NEP-04) |
| `src/main.rs` | `fe0f36f1e2b832eef4f0521e6a64bd453451abde` | `f92c7fef2a06cded4bdfd87b93f11790acdeb43d` | +248/−12; 12 commits incl. #1288 | 3 (S2–S4), 1 (help text) |
| `src/familiar_ledger.rs` | `53fedbfa249daf743256355346c06e2c1999d7a6` | `b66ecb5440d8d1c1be314f4fa1b0ce7d65ffceba` | +2/−2; #1348 | 3 (NEP-06) |
| `src/main_session_routes.rs` | `7338a8b5bafa19fc4af9010f881dfcf66f336901` | `ef3b6f9b6c218648c005c1320e3288405456a467` | +76/−3; #1307, #1365, #1374 | 3 (NEP-07…10) |
| `src/threads_request_diagnostics.rs` | `84190d08e36dbf26036baa544fffff558e198d02` | `3a4c5cf323c53d6945a751a7a54c2f74bf0311cd` | +251/−5; #1320, #1350 | 1, 2 (`checkpoint`) |
| `Cargo.toml` (crate) | `9befa8ce9f63fce8cc3370bb0621e20440ac652b` | `96f9b3fd2f1dcbbfca1e85835967c842c18c3d1b` | +3/−3: `serde_yaml_ng` → `serde-saphyr`, rusqlite features, windows-sys features | 3 (EP-90) |
| `tests/threads_e2e.rs` | `fa979548d086723d2aab39108f59b7c657df115d` | `b2771f4f765404c4453638de2b2165f6bf9913a2` | +9/−3; #1303 `e0708b19` | 2, 5, 6, 7 |
| `tests/support/threads_output_auto_cases.rs` | `a993319578fa2b6607b6bf890a5a067df308ed43` | `b403094afba0231eb438c225e946957ee5d0faac` | +84/−2; #1320, #1350 | 2, 5, 7, 8 |

**Unchanged files (blob identical at both SHAs):**

| File | Blob (both SHAs) | Items citing |
|---|---|---|
| `src/proposal_scheduler.rs` | `01019c21164ad126347b7e25a4bc605eca53655f` | 1, 3, 5, 6, 7 |
| `src/ward_identity.rs` | `c0a04711762f31934609def928f2a38a728ca29a` | 3 |
| `src/threads_clock.rs` | `6d585f75c2729ea5ebe4150d93a6c52a0374325b` | 3 |
| `src/ward_decision.rs` | `9c5e0a9a992a585f966b8d1c97769223dad7666e` | 3 (S2) |
| `src/output_format_auto.rs` | `5a1a2b91941e05102b00f189e5e7b8c8e51cb542` | 1, 3, 5, 8 |
| `src/familiar_identity.rs` | `db4264999c0bdb8dcf0991fee77082bcafddc39d` | 3 |
| `src/ward_migrate.rs` | `341b8471a0f4ab714c9c246bf3d4f2d6f1e5c208` | 3 (S3) |
| `src/request_authority.rs` | `84725cf4d6136f71aec50a3365352625e642cc63` | 3 (EP-03/04, NEP-01/04) |
| `src/mobile_memory/gateway.rs` | `59f512e4d033366598eb2675406580cd6a116d27` | 3 (NEP-02) |
| `src/chat_context_admission.rs` | `69aad00565136504aa93079a23897e64fcea5ebd` | 3 (NEP-01/02) |
| `src/familiar_issuer.rs` | `1485159dd38db68df8b407a80c4049bbdc8d62d0` | 3 |
| `src/api_legacy_submission_tests.rs` | `aa6eee8820c3537f57cb9c769d051ff5ea29c66d` | 2, 3 (CX-1e test) |
| `src/api_response.rs` | `0a16b21b4da7702db15def39c80da772a318bcd1` | 3 |
| `src/automations/authority_keys.rs` | `bbfa45ff895842a7a89534a677e6ec2365ed559a` | 3 (NEP-06) |
| `tests/threads_identity_invariants.rs` | `5da3323aea93ffcd907d00c203c260bf09579511` | 2, 7 |
| `tests/threads_protected_intake.rs` | `9b83558dedbdb9e71e5d6dedc0979bacc997ddae` | 3 |
| `tests/threads_terminal_recovery.rs` | `bc394df377930fd165de117caa3c6a511a2d0bd7` | (4) |
| `tests/support/threads_final_commit_cases.rs` | `99d853d0d1db4b6ac370d455cce28e29a4c70db0` | 2 |
| `tests/support/threads_identity_replay_cases.rs` | `a9bc617f807f8275414db141fb4b0f9b97ce88d6` | 2 |
| `tests/support/threads_corpus_closure_cases.rs` | `8a989a6204370b55a17ee6096a6bf74cab5f532e` | 1 |
| `tests/support/threads_output_auto_review_cases.rs` | `a9ff272e66ebc7fcdedec11f9415adfbfdbf0f66` | 7 |
| `docs/design/ward-gate3-coherence.md` (repo root) | `aaef44a323bfe9a429ebcc5a80e87e3fd7566f35` | 8 |

**New files in range under `crates/coven-cli/`.** There are 44 (`git diff --name-status … | grep -v '^M'`). The ones bearing on these items are:

- `src/durable_fs.rs`, `src/store/durability.rs`, `src/api_power_loss_tests.rs` and `src/power_loss_tests.rs` (#1305);
- `src/familiar_create.rs` (#1288);
- `src/main_session_events.rs` (#1307).

The rest are `automations/*`, `missions_cli.rs`, `main_session_{compaction,transcript}.rs` and tests.

**Module census.** Files under `src/` matching the Set B Ward/Threads reference regex (`crate::ward::|crate::ward\b|\bward::[A-Za-z_]|threads_gate::|coven_threads_core::|proposal_store::|proposal_scheduler::|ward_identity::|ward_probes::|decide_threads_proposal`): the Set B and Set C lists differ by exactly **one added file, `api_power_loss_tests.rs`**, which is test-only (included under `#[cfg(test)]` at api.rs C L17402-17405).

## 2. Per-item delta

### Item 1 — RFC round-trip

**Status: carried.** Some containing files changed, but every cited range is byte-identical.

- **Threads side.** The pin is unchanged (§0), so Threads `approval.rs`, `surface_regions.rs`, `identity_invariants.rs`, `audit.rs` and the §6 decision text are the Set B bytes.
- **Coven side.** Each Set B citation maps as follows:

| Set B citation | Set C | Status |
|---|---|---|
| `ward.rs` `compiled_approval_tiers` L589-727 | L589-727 | identical (`ward.rs` changed only at B L2558 and later) |
| `ward.rs` `enum ProbeId` L254-259 | L254-259 | identical |
| `api.rs` `revalidate_scheduled_approval_policy` B L8011-8042 | C L8021-8052 | identical |
| `api.rs` `proposal_decision_semantics` B L7830-7963 | C L7840-7973 | identical |
| `api.rs` `process_due_threads_proposals` B L13051-13418 | C L13069-13436 | identical (Set B's diagnostic `checkpoint` lines included) |
| `due` block B L13355-13366 | C L13373-13384 | identical |
| `api.rs` `threads_proposals_response` B L7452-7757 (`from_classification` B L7691-7708) | C L7462-7767 (`from_classification` C L7701-7718) | identical |
| `api.rs` `ensure_proposal_window_opened_audit_with_conn` B L13732-13812 | C L13750-13830 | identical |
| `proposal_scheduler.rs`, `output_format_auto.rs`, `tests/support/threads_corpus_closure_cases.rs` | — | blobs unchanged (§1) |
| `checkpoint()` no-op claim (B `threads_request_diagnostics.rs:212-223`) | C `threads_request_diagnostics.rs:272-288` | Still returns `()`. The logging body is `#[cfg(feature = "threads-test-clock")]`, followed by `let _ = phase;`. It cannot alter control flow. |

**`memory_entry_admitted` (threads-55s).** There is still no Coven producer at C. `git grep -E 'for_memory_entry_admitted|MemoryEntryAdmitted|MemoryEntryAdmission'` returns 0 hits. The string appears only at:

- `api.rs:6973`, the filter list;
- `api.rs:35197`, a test;
- `main.rs:1375`, help text;
- three SQL fixtures under `tests/fixtures/automations-release-stores/`.

**Covering tests.** The Coven `api.rs` test `threads_scheduled_replay_rejects_changed_live_approval_policy` moved from B L44836 to C L44952. Its body is identical: the whole `api.rs` region from B L17393 to EOF is byte-identical to C L17509 to EOF (+116). The Threads tests are unchanged by pin.

**Not re-checked:** familiar-contract PRs #3 and #4 (decision 5) and region promotion code (decision 3). Both were not found at Set B, and neither was searched again.

### Item 2 — Gate-4 fail-closed

**Status: changed source.** Every change on the approval path comes from #1305 (`c33da615`, durability), and every cited enforcement point still holds.

**Unchanged (byte-identical after offset):**

- `decide_threads_proposal` and `_automatic`: B L9945-9974 → C L9955-9984.
- `proposal_decision_semantics` through `revalidate_scheduled_approval_policy`: B L7830-8044 → C L7840-8054.
- `revalidate_auto_regression` through `ensure_proposal_final_authority_unchanged`: B L15646-15769 → C L15664-15787.
- `apply_after_review_approval`: B L15918-15970 → C L15936-15988.
- `ensure_proposal_window_opened_audit{,_with_conn}`: B L13714-13824 → C L13732-13842.
- `process_due_threads_proposals`: B L13051-13418 → C L13069-13436.
- In `ward.rs`, all at the same line numbers: `apply_after_scheduled_approval_with_commit_check` L2089-2155, config-open refinement L2102-2114, coherence apply L2024-2082 and legacy apply L1923-1991.
- Every EP-40 … EP-78 range inside `decide_threads_proposal_inner` is identical except the six listed below.

**Changed inside `decide_threads_proposal_inner`** (B L10193-12791 → C L10203-12809). The diff is exactly three insertions:

1. **C L10310** `store::durable_barrier(&conn)?;`. It sits on the early terminal-found path (EP-42, B L10299-10328), before `release_terminal_proposal_reservations`.
2. **C L11540**, the same call on the second terminal-found check (EP-62, B L11528-11558).

   Both run only when a terminal row already exists. `durable_barrier` (`store/durability.rs:85-91`) upserts `store_meta.ward_durability_barrier = hex(randomblob(16))` inside a FULL plus `fullfsync` transaction. That key is never read: `git grep ward_durability_barrier` finds only `durability.rs:87`. Its only possible effect on the outcome is an error return.
3. **C L12219-12224**, in the recovery branch (EP-72, B L12167-12410): `claim.preserve(); audit_reservation.preserve_if_unfinished(); store::durable_barrier(&conn)?; proposal_store::sync_existing(&claim.path)?;`.
   - **Position.** It runs after `verify_recoverable_apply_state` (C L12180-12198), `recovery_authorization` (C L12203), `ward_config_is_unchanged` (C L12204-12213) and the fixture pause (C L12214-12218). It runs before the `final_authority_check` closure (C L12226-12243) and `apply_after_review_approval` (C L12244).
   - **On a barrier error.** The claim stays at its claim path, because `Drop` does not rename a preserved claim back (C L15077-15083), and the reservation is kept active. The proposal stays in recoverable applying state.
   - **Effect.** It removes no check and opens no apply path.

**Changed audit writers.** `git diff -w` shows no change to SQL text or bound parameters.

- **`append_proposal_apply_intent`** (EP-75b, B L16017-16052 → C L16035-16071): the same INSERT, now inside `store::durable_transaction`.
- **`finalize_approved_proposal`** (EP-78b, B L16784-16838 → C L16804-16851): its hand-written `BEGIN IMMEDIATE` / `COMMIT` / `ROLLBACK` is replaced by `durable_transaction`. That is an Immediate rusqlite `Transaction`, which rolls back on drop (`durability.rs:52-59`). The same three writes stay atomic.
- **`append_proposal_decision_audit_with_probe_summary`** (EP-68b, B L16848-16932 → C L16861-16952): terminal rows only (`ProposalApproved/Rejected/Vetoed`, C L16867-16872) are written in `durable_transaction`. The preconditions are unchanged (C L16873-16887): no prior terminal, and typed `window_close` exactly when a window opened.

**What `durable_transaction` does** (`store/durability.rs:9-80`):

- Inside a non-durable outer transaction it **refuses**: "authority write requires a durable outer transaction" (L13-17).
- Otherwise it sets `synchronous=FULL` and `fullfsync=ON`, commits, syncs the store directory and its parent, and restores the previous mode (L21-79).
- Each new branch is an `Err` return.

**`ward.rs` changes inside `write_atomically_if_unchanged`:**

- The four in-apply authority checks are at **C L3323, L3339, L3437 and L3491** (Set B L3319, L3335, L3433, L3485). B L3224-3440 → C L3228-3444 and B L3480-3490 → C L3486-3496 are identical.
- New C L3467-3468 adds `durable_fs::sync_file(identity)` and `sync_anchored_directory(parent)` in the batch-finalization loop, before the C L3491 check. Failure rolls back (C L3494-3496).
- `rollback_conditional_writes` gains post-rollback syncs (C L3625-3630), and `prepare_staging_parent` gains directory syncs (C L5894-5900).
- None of these changes which bytes are approved or which checks run.

**Covering tests:**

- **`api.rs` unit tests**, all byte-identical (B L17393–EOF ≡ C L17509–EOF):
  - `threads_scheduled_human_required_enforces_rationale_and_applies` C L37381
  - `…_deadline_replay_refuses_diverged_before_image` C L38545
  - `…_deadline_replay_failure_closes_window` C L38656
  - `threads_scheduler_revalidates_old_window_instead_of_expiring_it` C L40360
- **`tests/threads_e2e.rs`:** the cited bodies hash identical at the same lines: L394, L606, L661, L867, L960. The shared fixture helper `restart_daemon` was split by #1303 (`e0708b19`) into `restart_daemon` and `restart_daemon_via_cli` (C L2635-2650). For these tests it behaves the same: a foreground daemon is stopped and started, and otherwise the CLI restart runs as at B.
- **`tests/support/threads_output_auto_cases.rs`:** the bodies are identical at shifted lines:
  - `…_deadline_and_restart_reject_…` B L462 → C L544
  - `…_final_commit_rechecks_policy_…` B L592 → C L674
  - `…_no_veto_survives_restart_…` B L197 → C L279
- **Unchanged blobs:** `threads_final_commit_cases.rs`, `threads_identity_replay_cases.rs` and `threads_identity_invariants.rs`.
- The #1278 test is unchanged (§2 item 3).
- The new #1305 tests (`api_power_loss_tests.rs`, `power_loss_tests.rs`) were **not reviewed**; they belong to item 4.

**Exact-SHA CI** (read-only `gh api …/actions/runs?head_sha=c658603d…`):

- 39379174287, CI / `push` / `main`: completed, **success**, attempt 1.
- 39377159532, CI / `merge_group` (`gh-readonly-queue/main/pr-1415-…`): completed, **success**, attempt 1.

Per-job results (Linux, Windows and macOS Threads suites) were **not read**: the jobs call was declined. So "macOS has a Set C Threads-suite result" is **not re-checked**.

**Overlap with item 4.** Every hunk above is #1305 durability. It was read here only to judge whether revalidation or the in-apply authority check is bypassed, and it is not. Whether it closes the coven#1300 power-loss gap is item 4's question and was not assessed here.

### Item 3 — Descriptor-not-authority

**Status: changed source.** No authority-bearing change was found. One new out-of-daemon writer of identity inputs (S5) and one new TCP-readable streaming route are noted below.

#### 3a. Enforcement-point carry (Set B → Set C)

All 126 Set B `api.rs` ranges cited in Part A §1b and §3 were mapped and byte-compared.

- **Identical:** all EP-01 … EP-92 ranges and NEP-03, except the six below. Examples:
  - EP-12 intake `signed_by`: B L6071-6082 → C L6081-6092.
  - EP-15: B L6147-6210 → C L6157-6220.
  - `protected_proposal_targets`: B L5913-5953 → C L5923-5963.
  - EP-19: B L6267-6322 → C L6277-6332.
  - EP-31 fingerprint disclosure: B L6737-6792 → C L6747-6802.
  - EP-59: B L11290-11429 → C L11301-11440.
  - EP-66: B L11828-11918 → C L11840-11930.
  - EP-73: B L12412-12477 → C L12430-12495.
  - EP-77: B L12613-12739 → C L12631-12757.
  - `authorization_from_writer`: B L16654-16660 → C L16674-16680.
  - Legacy receipt: B L13835-13901 → C L13853-13919.
  - Scheduled receipt: B L14557-14625 → C L14575-14643.
  - EP-80 scheduler: B L13051-13411 → C L13069-13429.
  - EP-90: B L17206-17302 → C L17226-17322.
  - EP-91 arm: B L1122-1183 → C L1129-1190.
  - Actor constant L55.
- **Touched, all #1305 durability (detail in item 2):**
  - EP-42a (C L10310 barrier);
  - EP-62 (C L11540 barrier);
  - EP-68b, EP-75b and EP-78b (writes wrapped in `durable_transaction`, with the same SQL);
  - EP-72a (C L12219-12224 barrier and claim sync).

  None reads a client value, and none adds a permit.
- **`ward.rs`.** All 21 cited Gate-1, apply and approval-tier ranges are identical at the same line numbers or with a fixed offset: L387-412, L425-588, L589-727, L889-902, L1561-1620, L1622-1627, L1639-1704, L1683-1692, L1719-1724, L1804-1896, L1883-1895, L1923-1991, L2024-2082, L2089-2155 and L2195-2200.
- **`threads_gate.rs`.** These are identical at the same lines: L169-171, L327-480 (with L360-363 and L376-401), L529-664, L932-957, L959-1011, L1040-1077, L1085-1154, L1182-1260 (staging writer L1192-1195), L1318, L1323-1346 and L1348-1412. `#[cfg(test)]` moved from L1700 to L1704.
  - **EP-20** (B L1436-1626, L1645-1698) is touched by #1305 only:
    - `create_dir_all` → `durable_fs::create_dir_all` (C L1293, C L1569);
    - `replace_purpose` wrapped in `durable_transaction` (C L1602-1606);
    - the `append_scheduled_submission_audit` INSERT wrapped in `durable_transaction` (C L1679-1702), with the same SQL.
  - The pending publish sync moved from B L1608-1612 to C L1610-1614, identical.
- **`proposal_store.rs`.**
  - EP-33/86: B L211-219 → C L214-222, identical.
  - EP-85: B L261-282 → C L264-285 and B L326-336 → C L330-340, identical.
  - Added: `sync_parent` after renames and removals, and the new `sync_existing` (C L415-438). These are fsyncs only.
- **Other files:**
  - `ward_probes.rs` EP-34 L306-313 and EP-61 L129-174, L315-371: identical.
  - `cockpit_sources.rs` EP-30 `write_familiar_icon`: B L248-322 → C L254-328, identical.
  - `roster_identity` L148-199: identical.
  - `eval_loop.rs` `enqueue_run`, `clear_eval_loop_lock` and `familiar_workspace` (C L267-299, L305-325, L329-331): identical. `mutation_lock` now creates its directory through `durable_fs` only when absent (C L188-190).
  - `store.rs` `WardAuditReservation` ranges B L780-968 → C L786-974 and B L1107-1137 → C L1113-1143: identical.
  - `familiar_ledger.rs` NEP-06 ranges (L85-171, L230-278, L573-625, L644-737, L739-765, L1024-1031): identical. Only the `REGISTER` and `ADOPT` constants became `pub(crate)` (C L63-64).
  - Unchanged blobs: `ward_identity.rs`, `ward_decision.rs`, `threads_clock.rs`, `request_authority.rs`, `chat_context_admission.rs`, `mobile_memory/gateway.rs`, `familiar_issuer.rs`, `ward_migrate.rs`, `familiar_identity.rs` and `api_response.rs`.
  - `api_routes.rs`: only a module doc comment changed (C L25-28). EP-01 code B L30-74 is identical at C L32-76, and the constant is at C L35.

**Grep census, `api.rs` production portion.** The ranges are B L1-17380 and C L1-17400; `pub(crate) mod tests` is at B L17381 and C L17401. Set B counts reproduce the Set B packet's figures.

| pattern | B | C |
|---|---|---|
| `validate_fail_closed` | 1 | 1 |
| `\.evaluate\(` | 3 | 3 |
| `proposal_decision_semantics\(` | 2 | 2 |
| `gate_protected_edits\(` | 1 | 1 |
| `ensure_proposal_window_opened_audit` | 7 | 7 |
| `append_[a-z_]*audit[a-z_]*\(` | 27 | 27 |
| `INSERT INTO ward_audit` | 4 | 4 |
| `principalKeyFingerprint\|principal_key_fingerprint\|signed_by\|authorization_from_writer` | 13 | 13 |
| `trusted_decision_approver` | 7 | 7 |
| `protected_proposal_targets` | 4 | 4 |
| `"blocked": true` | 69 | 69 |
| `api_error\(` | 219 | 219 |
| `quarantine` | 55 | 55 |
| `bail!\|ensure!` | 74 | 74 |
| `allows_[a-z_]+\(` | 4 | 4 |
| `RequestAuthority::` | 4 | 4 |
| `durable_transaction\|durable_barrier` | 0 | 6 (#1305) |

#### 3b. HTTP routes and non-HTTP entries added, removed or changed since f772b513

**API dispatch (`api.rs`).**

- **No arm was added or removed.** There are 84 arm headers at both SHAs, and a text diff of the headers is empty. The match runs C L885-1478.
- **Two arm bodies changed:**
  - **`POST /actions`** (C L910-…): a new `coven.automations.doctor.v1` short-circuit at **C L925-931** (#1381).
    - **Placement.** It runs *after* the NEP-04 transport refusal (`automation_transport_rejection`, C L920-924) and before the store opens.
    - **TCP.** The action is in `OWNER_AUTOMATION_READS` (`control_plane.rs` C L206, in the list starting at C L204), not `TCP_AUTOMATION_READS` (starting at C L193, whose list is unchanged from B L178-184). So a TCP caller gets 403. The test `doctor_api_boundary_open_failure_is_sanitized_and_tcp_rejects_first` asserts this (`api.rs` C L17444).
    - **Handler.** `control_plane::doctor_existing_store` (C L268-275) calls `automations::doctor::inspect_owner_store_path`. `automations/doctor*.rs` has 0 Ward/Threads references (grep `ward_audit|ward::|threads`). The tests assert that it creates no store and leaves permissions and bytes unchanged (`api.rs` C L17417-17508: `doctor_boundary_request` L17417, tests at L17430, L17444 and L17463).
    - **Assessment.** It is not a Ward/Threads-governed surface, and it trusts no client value as authority.
  - **`POST /hub/jobs/{id}/complete`** (C L1362): passes `runtime` to `hub::complete_job` (#1324; posts a result notice to the main session). `hub.rs` is not in the Ward/Threads census.
- **NEP-04 itself** (`control_plane.rs` B L207-245 → C L224-266) gained two more owner-only refusal reasons, mission GET and lease inspection (C L250-253). Deny-by-default is unchanged.
- **NEP-07 `GET /main-session`** now also returns read-only `transcript` and `lastCompaction` fields (`main_session_routes.rs` C L264-273). NEP-08, NEP-09 and NEP-10 ranges are identical (C L278-360, L692-749, L751-806). The Set B observation that `/reset` and `/rollover` take no transport authority still stands.

**Transport (`daemon.rs`).**

- The Host/Origin guard (EP-02) moved from B L6119-6134 to C L6233-6248, identical.
- The restricted-session preflight moved from B L6135-6161 to C L6249-6275, identical.
- The **authority mapping** moved from B L6180-6183 to **C L6294-6297, identical**: `HostGuard::Disabled` → `OwnerLocalIpc`, `Loopback` → `Tcp`.
- **New early dispatch: `GET /main-session/events`, SSE** (#1307, `3fddd860`).
  - **Classification.** It is classified at C L6316-6317 through `main_session_events::claims`. That function is `GET` only, and matches after `api_routes` normalization (`main_session_events.rs:46-52`).
  - **Gate.** It passes the same refusal-only chat-context gate as the other early dispatches (C L6325-6341).
  - **Handler.** It is served at C L6344-6355. The handler takes **no `RequestAuthority`**, so a loopback TCP caller that passes the Host/Origin guard can stream main-session events.
  - **Assessment.** `main_session_events.rs` has 0 Ward/Threads references and reads the store; its only lock is the main-session scope lock, taken with `try_acquire`. This is outside Threads scope and in the same class as the NEP-09/NEP-10 observation. It is not a counterexample.
- **Parsed headers** are now 5: `last-event-id` was added at C L6813-6821. The `if-none-match` handling (NEP-11) moved from B L6677-6682 to C L6822-6827, identical.
- **Listeners**, re-checked for EP-03:
  - **TCP** `serve_accepted_tcp_connection` (C L4911-4950; the policy at C L4936-4949) now calls `handle_http_stream_with_lifecycle` directly, with `HostGuard::Loopback` and `LifecycleControl::Disabled`. That is the same policy Set B's `handle_http_stream` wrapper applied; the wrapper is now `#[cfg(test)]`.
  - **Unix socket**: `HostGuard::Disabled` and `LifecycleControl::OwnerLocal`, unchanged, plus a stream configurator (C L6744).
  - **Windows named pipe**: now `serve_named_pipe_connection` (C L7095-7134), still `HostGuard::Disabled` → `OwnerLocalIpc`. The owner-only SDDL `owner_only_pipe_sddl` moved from B L6710-6729 to C L6855-6874, identical. A connection that arrives at the in-flight cap is now dropped instead of served on the accept thread (`drop(stream)`, C L7065).
  - The EP-03 mapping is unchanged.

**Non-HTTP entries:**

- **S1** `start_threads_proposal_scheduler`: B L5049-5073 → C L5142-5166, identical. Call sites are C L5864 and C L7043.
- **S2** (`main.rs` B L4637-4652 → C L4783-4798), **S3** (B L4667-4687 → C L4813-4833) and **S4** (B L1854-1871 → C L1947-1964): identical. The `memory_import.rs` S4 ranges L563-581 and L760 are identical; its changes are tests only.
- **New S5: `coven familiar create`** (#1288, `b14f8228`).
  - **Entry.** `main.rs:1884` → `familiar_create::run` → `create` (`familiar_create.rs:191-308`).
  - **Writes:**
    - `familiars.toml`: it appends one `[[familiar]]` under `familiars.toml.lock` (L247). It refuses an existing id (L265-270), requires exactly one appended entry with the old entries intact (L280-295), and commits atomically (L298).
    - It seeds `SOUL.md` and `IDENTITY.md` under `workspaces/familiars/<id>/` **only when absent**, keeping existing files (`seed_workspace` L424-468).
    - It writes no `ward.toml`, `pending/`, `ward_audit` or `ward_manifest`.
  - **Reach.** It is not reachable through the daemon: references are `main.rs:46`, `752`, `1734`, `1884` and tests only.
  - **It does not pass through the Threads gate.** It is an owner-local operator CLI outside the daemon, the same class as S3 (`ward migrate`, which rewrites `ward.toml` with no gate) and S4. It decides nothing and is not an enforcement point.
  - **Effect on existing familiars.** Identity predicates read the roster entry by exact id (`ward_identity.rs:232-246`, which filters `entry.id == familiar_id`). Appending a new id therefore cannot change any existing familiar's identity evidence.
  - **What it adds to the Set A gap.** Set A recorded that out-of-band writers of `familiars.toml`, `ward.toml` and `pending/` were never audited. S5 is one more known writer, and it is create-only.
- **Not production:** the automations conformance target `mission_commands.rs` writes `familiars.toml`, `SOUL.md`, `IDENTITY.md` and `ward.toml` (L419-436), but only into a fresh `scheduler_conformance_tempdir()` (L237-242; `conformance_target.rs:4020`), never the real home. The other `familiars.toml` string hits added in range are in `#[cfg(test)]` modules: `automations/delivery.rs:521` (tests from L494), `hub.rs:2856` (tests from L1574) and `main_session_compaction.rs:855` (tests from L835).

#### 3c. New code paths that write Threads/Ward authority state

| Path | Where | Same Threads gate and authority checks? |
|---|---|---|
| Apply intent, finalize (apply audit, baseline, terminal), terminal decision rows, scheduled submission receipt, reservation purpose | `api.rs` C L16035-16071, L16804-16851, L16861-16952; `threads_gate.rs` C L1602-1606, L1679-1702 (inside `append_scheduled_submission_audit` C L1647-1702) | **Yes.** These are the existing writers at their existing positions, with the same SQL. Only the commit mode changed. |
| `store_meta.ward_durability_barrier` | `store/durability.rs:85-91`, called at `api.rs` C L10310, L11540, L12223 | Not authority state. It is a sync marker, never read. It runs inside the existing decision path. |
| Directory publication registry `.coven-directory-publications` | `durable_fs.rs:236-244`, `346-350`; it appears in `pending/` | Bookkeeping only. The census skips it (`ward_audit_census.rs` C L420-425). The scheduler ignores it, because only names ending `.json` or `.json.{approve,reject}.deciding` are candidates (`proposal_store.rs` C L115-119, used at `api.rs` C L12934). |
| Pending, claim and quarantine fsyncs; `sync_existing` | `proposal_store.rs` `sync_parent` at C L171, L180-181, L322, L401, L411; `sync_existing` C L415-438 | fsync only. No new bytes. |
| `coven familiar create` (S5) | `familiar_create.rs:191-308` | **No.** It is outside the daemon, as above. |

#### 3d. CX-1 and CX-1e at Set C

- **The CX-1e refusal (#1278, `21988a00`) is present and byte-identical:** `api.rs` **C L11441-11463** (B L11430-11452).
  - Condition at C L11448-11451: `decision == "approve" && applying_state.is_none() && review_kind == PendingReviewKind::Authority && scheduled.is_none()`.
  - On a match: `restore_pending` (C L11453), `audit_reservation.finish()` (C L11454), and 409 `legacy-authority-approval-unsupported` (C L11459).
  - **Ordering.** It runs after EP-59 (C L11301-11440) and before EP-60 (C L11464) and EP-73 (C L12430-12495).
- **The regression test is unchanged.** The file `api_legacy_submission_tests.rs` has the same blob (`aa6eee88…`). The test `legacy_authority_tier_one_approval_is_refused_even_with_the_principal_label` is at L178, and the file is included at `api.rs` C L36122-36125 (B L36006-36008).
- **CX-1a to CX-1d refs at Set C:**
  - Gate 1: `ward.rs` L1719-1724 (and L1683-1692), unchanged.
  - Writer label: `threads_gate.rs` L360-363, L1192-1195 and L1323-1346, unchanged.
  - The only production `gate_protected_edits(` call is `api.rs` C L6277, with `gated_targets: &[]` at C L6285. The other hits are the definition at `threads_gate.rs:327` and test calls in `api_legacy_submission_tests.rs`.
  - EP-73: C L12430-12495.
  - Writer-bound weave in EP-66: C L11864-11873.
  - **Classes are unchanged:** 0 class-(a) sites, and CX-1a…d remain class (b).
- **Recovery residual.** The recovery branch changed only by the #1305 barrier insertion (C L12219-12224). `recovery_authorization` (C L12203) still reaches Ward Gate 1 only, after EP-59. A test resuming a pre-`21988a00` legacy apply intent is still **not found** (not re-searched beyond `api_legacy_submission_tests.rs`, whose blob is unchanged).

**Cave side:** see §3. The Threads-facing Cave bytes are unchanged, and no request sends an authority field.

### Item 5 — evidence_replay_hash

**Status: carried, unchanged source.** Every cited range is byte-identical, though `api.rs` as a file changed.

| Set B citation | Set C | Status |
|---|---|---|
| Threads `approval.rs` L405-431, `surface_regions.rs` L192-227 | pin unchanged | carried |
| `proposal_scheduler.rs` L82-90, L137-161, L190-208 | blob `01019c21…` at both | unchanged |
| `output_format_auto.rs` `regression_evidence` L72-115 | blob `5a1a2b91…` at both | unchanged |
| `api.rs` `revalidate_scheduled_materialized_before` B L7980-8009 | C L7990-8019 | identical |
| `api.rs` `revalidate_auto_regression_evidence` B L15696-15729 (hash sub-range B L15708-15713) | C L15714-15747 (C L15726-15731) | identical |
| `api.rs` `revalidate_auto_regression` B L15646-15694 | C L15664-15712 | identical |
| `api.rs` `ensure_proposal_window_opened_audit_with_conn` B L13732-13812 (`evidence_replay_hash_hex` B L13768-13774) | C L13750-13830 (C L13786-13792) | identical |
| Identity block inside `decide_threads_proposal_inner` B L12009-12024 | C L12021-12036 | identical. The only changes in that function are the three #1305 insertions (item 2); none is in this block. |

**Covering tests:**

- **`api.rs`:** `threads_scheduled_deadline_replay_refuses_diverged_before_image` C L38545, `…_rejects_deleted_empty_before_image` C L38621 and `…_replay_failure_closes_window` C L38656. All identical (+116).
- **`tests/threads_e2e.rs`:** `scheduled_window_replay_fails_closed_across_real_daemon_restart` at L661, with an identical body.
- **`threads_output_auto_cases.rs`:** `output_auto_deadline_and_restart_reject_regression_and_valid_evidence_drift`, B L462 → C L544, identical.
- **`proposal_scheduler.rs` tests:** unchanged blob.

The Set A/B limiters carry: enforcement is composite, not a literal live re-hash, and identity is committed separately.

### Item 6 — min_visible (daemon side)

**Status: carried, unchanged source.** Every cited range is byte-identical.

| Set B citation | Set C | Status |
|---|---|---|
| Threads `VetoWindow` (`approval.rs`) | pin unchanged | carried |
| `ward.rs` `declared_veto_window` L730-745, `veto_window_from_hours` L747-758, `compiled_approval_tiers` L589-727 | same lines | identical |
| `proposal_scheduler.rs` `ScheduledProposal::try_new` L92-134, L137-161 | unchanged blob | unchanged |
| `api.rs` `proposal_decision_semantics` B L7830-7963; refusal `proposal-minimum-visibility-open` B L7891 | C L7840-7973; refusal at **C L7901** | identical |
| `api.rs` scheduler `due` block B L13355-13366 | C L13373-13384 | identical |
| `api.rs` `threads_proposals_response` B L7452-7757; list visibility B L7691-7723 | C L7462-7767; C L7701-7733 | identical |
| Route `("GET", "/threads/proposals") => threads_proposals_response(…)` B L1073 | C L1080 | identical line |

**Covering tests**, all with identical bodies:

- **`api.rs`:**
  - `threads_scheduler_tick_observes_fixture_before_min_visible_at_earliest_close_and_beyond_deadline` C L40239.
  - `threads_scheduled_veto_window_delays_apply_and_records_veto` C L37835, with its assertion at C L37863.
  - `post_familiar_edits_stages_scheduled_publication_from_retired_ward_intake` C L45005, with the premature-approve assertion at C L45106.
  - `threads_scheduled_applies_only_after_veto_deadline` C L38005.
  - `threads_scheduler_opens_window_once_and_waits_until_deadline` C L39889.
- **`tests/threads_e2e.rs`:** `retired_corpus_scheduled_intake_survives_restart_and_applies_once` at L394.
- **`proposal_scheduler.rs`:** L613, unchanged blob.
- **`ward.rs`:** `validation_rejects_invalid_retired_ward_approval_metadata`, B L6755 → C L6771.

**Overlap with #1305.** It does not touch any function above. Window timing still derives from `staged_at` and the scheduled `VetoWindow`, not from store rows.

**Cave side:** see §3. Every Cave function cited for item 6 is in a Threads-facing file whose blob is unchanged, so the Set B Cave reading carries. Browser reachability was **not re-checked** (coven-cave#5256).

### Item 7 — Label-variant mapping

**Status: carried, unchanged source.** The #1276 tests are still present, with identical bodies.

| Set B citation | Set C | Status |
|---|---|---|
| Threads `approval.rs` label functions; `audit.rs` label CHECK (B L1706-1765) | pin unchanged | carried |
| `ward.rs` `WardConfig::from_toml_str` L415-419, `validate` L425-497 | same lines | identical |
| `ward.rs` `compiled_approval_tiers` L589-727 (unknown tier L604-606, gate L652-656, map L684-696) | same lines | identical. The rejection strings are at **C L605** and **C L654**. |
| `ward.rs` gate constants L372-375 | same lines | identical |
| `proposal_scheduler.rs` `validate_scheduled_json_shape` L318 and following | unchanged blob | unchanged |
| `api.rs` `from_classification` emit B L7691-7696 | C L7701-7706 | identical |
| Cave `proposal-authority.ts` `APPROVAL_PATH_LABELS` and the mismatch block | blob `08d696bb…` at both Cave SHAs | unchanged |

**The #1276 tests at Set C:**

- `validation_rejects_unknown_approval_tier_at_load`: `ward.rs` **C L6923-6954** (B L6907-6938). It is byte-identical, and asserts at C L6951.
- `validation_rejects_approval_tier_gate_drift_at_load`: `ward.rs` **C L6957-6994** (B L6941-6978). It is byte-identical, and asserts at C L6988.

**Other covering tests:**

- `ward.rs`: `parses_retired_ward_approval_metadata` C L6655 and `validation_rejects_invalid_retired_ward_approval_metadata` C L6771, both identical.
- `proposal_scheduler.rs` tests: unchanged blob.
- Cave `threads-adapters.test.ts`: unchanged blob (§3).

**Real-daemon malformed-load journey: still not found.** At C:

- `git grep -nE 'unknown tier|gate must be' -- crates/` hits only `ward.rs:605`, `654`, `6951` and `6988`.
- Every `approval_tiers` hit under `crates/coven-cli/tests/` writes or derives a valid pair, or removes the table. The hits are:
  - `support/threads_output_auto_cases.rs:71`, `501`, `527`;
  - `support/threads_output_auto_review_cases.rs:153`;
  - `threads_e2e.rs:710`, `719`, `1848`, `1884`;
  - `threads_identity_invariants.rs:49`, `63`, `172`.

### Item 8 — Advisory probes

**Status: changed source.** One probe's implementation changed, and the conclusion is unchanged.

| Set B citation | Set C | Status |
|---|---|---|
| Threads `identity_invariants.rs` `AdvisoryProbeResult` and the `AdvisoryProbes` block | pin unchanged | carried |
| No daemon `advisory_probes` integration | `git grep -nE 'advisory_probes\|AdvisoryProbes\|AdvisoryProbeResult'` over the whole tree at C: only `docs/design/ward-gate3-coherence.md:247`, blob `aaef44a3…` unchanged | carried |
| `ward.rs` `enum ProbeId` L254-259 | same lines | identical |
| `ward_probes.rs` module docs L1-6 ("a probe alone never approves a write") | same lines | identical |
| `output_format_auto.rs` `authoritative_projection` L117-136 | unchanged blob | unchanged |

**The change: #1306 (`921b44f5`).** `ward_probes.rs` `parse_markdown_front_matter` (C L459-…) now validates YAML front matter with `serde_saphyr::from_str_with_options::<IgnoredAny>` (C L480) instead of `serde_yaml_ng`. The options are chosen to keep the previous acceptance (merge keys as ordinary keys, non-finite floats passed through), and duplicate keys still fail closed. The crate `Cargo.toml` swaps the dependency. New tests:

- `markdown_front_matter_preserves_yaml_syntax_support`, C L985;
- `markdown_front_matter_rejects_invalid_yaml_without_exposing_contents`, C L1013.

**Why the conclusion holds:**

- This changes how a deterministic Parse probe judges Markdown front matter. It does not change whether probes are advisory.
- The only authoritative probe use is unchanged:
  - `output_format_auto.rs:79-106` requires a `ProbeId::Parse` probe with `ProbeFormat::Json` on the output-format surface, plus every applicable result `Passed`;
  - and the evidence commits to that projection (L107-114).
- A probe can therefore refuse, or feed a regression commitment. It still cannot approve alone.
- Whether `serde_saphyr` accepts exactly the same inputs as `serde_yaml_ng` was **not verified** beyond the new tests' vectors.

**Other new code checked for probe authority.** `crates/coven-agents` changed heavily (`agent_loop.rs`, `delegation.rs`, `runner.rs`). At C, `git grep -lE 'coven_threads_core|ApprovalPath|ward::|WardConfig|threads_gate|ward_probes' -- crates/coven-agents` returns **0 files**.

**Covering tests**, all with identical bodies:

- `ward_probes.rs`: `deterministic_failures_and_probe_errors_remain_advisory_evidence` B L809 → C L816, and `absent_probe_config_is_explicitly_unscored` B L1054 → C L1118.
- `api.rs`: `threads_coherence_approve_keeps_failed_probes_advisory` C L36707.
- `threads_output_auto_cases.rs`: `output_auto_keeps_stronger_ceremonies_and_other_probes_advisory` B L411 → C L493, and `output_auto_requires_applicable_json_and_all_probes_passed` B L310 → C L392.
- Threads tests: unchanged by pin.

## 3. Cave

### Pick

**Set C Cave: `681130f34e1f4bf2fe1736c6b23ee2613f9ce14b`.** It was committed 2026-10-19T22:30:15Z (17:30:15-05:00): "fix(build): stop every route trace walking the checkout, and gate it in CI (#6101) (#6102)".

- **API listing.** `gh api 'repos/OpenCoven/coven-cave/commits?sha=main&until=2026-10-20T05:51:30Z&per_page=5'` returns it first. Its single parent is `e32da9840`, followed by `4969b8f9c`, `2bc7929b0` and `e7559d76f`. Each has one parent, so the chain is linear.
- **Nothing later.** `…&since=2026-10-19T22:30:16Z` returned **no commits** at query time. So nothing on `main` falls between this commit and the cutoff.
- **First-parent check, local read-only git** (objects already present, no fetch):
  - `git rev-list --first-parent --count c60bbef9..681130f3` = 88, the same as the all-parents count. The history is linear.
  - `c60bbef9` appears once in `git rev-list --first-parent 681130f3`.
  - `git merge-base --is-ancestor c60bbef9 681130f3` succeeds.
- **`gh api repos/OpenCoven/coven-cave/compare/c60bbef9...681130f3`:** `status: ahead`, `ahead_by: 88`, `behind_by: 0`. Its file list is **capped at 300**, and the true total is 2196 files (local `git diff --name-only`), so the compare list was not used to clear any file. Local `git ls-tree` and `git diff` on the same two SHAs were used instead.

### Threads-facing files (Set B part A §4 and part C §1/§3)

| Cave file | Blob @ c60bbef9 | Blob @ 681130f3 | |
|---|---|---|---|
| `src/lib/threads-adapters.ts` | `83decb708b937513d371ddf31d48033473351213` | same | unchanged |
| `src/lib/proposal-authority.ts` | `08d696bb5b26e7a8039495d7f00245c18ac7d2ba` | same | unchanged |
| `src/lib/proposal-flow.ts` | `5fbeaaaab11f4ca7c3a0b3a18bfac7088387e54a` | same | unchanged |
| `src/lib/weave-rail.ts` | `771bb64d7351a4d3afac9297ee3f7d759a5c5866` | same | unchanged |
| `src/lib/proposal-submission.ts` | `41805fb42b2ad358c6cab3e9b7ee2b8e8126d408` | same | unchanged |
| `src/lib/server/proposal-submission.ts` | `ded99f2cf65a81565ca1ded56ca897eaef41e8c3` | same | unchanged |
| `src/lib/server/api-security.ts` (`rejectNonLocalRequest`) | `7e93f7bdfec6ea9fd89c950a55e8bca19f743e17` | same | unchanged |
| `src/lib/threads-read.ts` | `eab3b056034c3dcb95c41839aeed4552e4adb7b8` | same | unchanged |
| `src/lib/proposal-normalize.ts` | `a211ba3aaf0c38b11ab112fb220853db6140c3d3` | same | unchanged |
| `src/lib/weave-map.ts` | `d799e69a5c05466e9ff560f7c0e7903702378c14` | same | unchanged |
| `src/components/proposal-approval.tsx` | `1c502ec25c67521d8074daad3bd3e964e5e7be15` | same | unchanged |
| `src/components/weave-rail.tsx` | `cef429c1f1bc4bfdd47c6be1552f70cd09c40cc9` | same | unchanged |
| `src/app/api/proposals/route.ts` | `df7aaadacfa30b1ab433a9210bcbf0c73464154c` | same | unchanged |
| `src/app/api/proposals/submit/route.ts` | `a817563544d54df5309c5dfc5ad37d098c24634c` | same | unchanged |
| `src/app/api/proposals/[id]/approve/route.ts` | `1b239cf03cb4453f36329b3dc5f527f1cd5e5199` | same | unchanged |
| `src/app/api/proposals/[id]/reject/route.ts` | `c0bb3ffec8e74401ba864205e97600e99ce39951` | same | unchanged |
| `src/app/api/threads/[id]/route.ts` | `dfbd8caaeb6f08bb2c3ea6f9e3f83de54da53cc6` | same | unchanged |
| `src/app/api/threads/[id]/audit/route.ts` | `568eca2ccd2242bb02a5e59d12b598937ee1c278` | same | unchanged |
| `src/app/api/threads/[id]/strands/route.ts` | `5b47a2207e0a50c721ca144f7bf99e21aff7d00a` | same | unchanged |
| Tests: `threads-adapters.test.ts` `0b7347fd…`, `proposal-flow.test.ts` `5ac4405c…`, `proposal-authority-daemon.test.ts` `b9633753…`, `threads-read.test.ts` `eb76c3b0…`, `app/api/proposals-flow-e2e.test.ts` `1a883b15…` | — | same | unchanged |
| **`src/lib/coven-daemon.ts`** | `4b9b357c79a1293ba50845d5b0083e4ba46e372c` | `47c40bbf4904549fdb389a2156e395c44d7bc298` | **changed** (+4/−4) |

**The `coven-daemon.ts` change** comes from #6097 (`4969b8f9c`). It is four `path.join(covenHome, …)` calls rewritten as `path.join(/* turbopackIgnore: true */ covenHome, …)`, at L278 (`daemon.json`) and L393, L433 and L440 (`coven.sock`). This is a bundler hint. The joined path values are the same, and the Set B socket and pipe constants (L51-55) are not in the diff.

**Effect on Set B's owner-local-proxy judgement.** Cave still reaches the daemon over the owner socket or pipe, so its requests arrive as `OwnerLocalIpc`. That reviewer judgement carries unchanged.

### Authority fields in requests (items 3 and 6)

These greps cover non-test `src` (excluding `*.test.ts` and `*.test.tsx`) at both Cave SHAs:

| Pattern | c60bbef9 (files / hits) | 681130f3 (files / hits) |
|---|---|---|
| `principalKeyFingerprint`, `principal_key_fingerprint`, `principal:`, `signed_by`, `approverId`, `decisionActor`, `owner:local-ipc`, `familiars\.ledger` | 0 / 0 each | 0 / 0 each |
| `"writer"\|writer:` | 11 / 25 | 11 / 25 (same file list) |
| `ward_audit` | 5 / 16 | 5 / 16 (same file list) |
| `threads/proposals` | 1 / 2 | 1 / 2 |
| `/edits` | 1 / 1 | 1 / 1 |
| `expectedRevision` | 13 / 64 | 22 / 100 |

- **The `expectedRevision` increase** is entirely automations code: `app/api/codex-automations/[id]/route.ts`, `app/api/coven-automations/[id]/lifecycle/route.ts`, `components/automations-view.tsx`, `components/automations/cron-detail-panel.tsx`, `lib/automations/{lifecycle,revision-rebase,use-automation-lifecycle}.ts` and `lib/server/{automation-lifecycle,coven-automations-client}.ts`. None of it targets a Threads or Ward route.
- **Threads call sites at 681130f3** are the Set B ones: `src/lib/server/proposal-submission.ts:15` (`/api/v1/familiars/{id}/edits`) and `src/lib/threads-adapters.ts:451-453` (`/api/v1/threads/proposals`, `/{id}/{decision}`).
- **Daemon familiar routes.** The set of daemon `/api/v1/familiars…` paths Cave calls is the same at both SHAs: list, `{id}/icon` and `{id}/edits`.

**Conclusion.** No Threads-facing Cave file changed apart from the build-hint-only edit to `coven-daemon.ts`. No request now sends `principalKeyFingerprint`, an actor, a writer, or any other authority field. The Set B Cave findings for items 3 and 6 carry unchanged.

## 4. Proposed verdicts at Set C

These are proposals for Val's review, not acceptances. No tests were run. Item 4 is checked separately and is not proposed here.

| # | Item | Set B | Proposed at Set C | One-line justification |
|---|---|---|---|---|
| 1 | RFC round-trip | supported | **supported** | The Threads pin is unchanged, and every Coven mapping site (`compiled_approval_tiers`, `proposal_decision_semantics`, the `due` block, `threads_proposals_response`, the opening writer) is byte-identical at C. |
| 2 | Gate-4 fail-closed | supported | **supported** | The only approval-path changes are #1305 durability wrappers and barriers. Each adds a sync or an `Err` return, and none removes or reorders a revalidation. The four in-apply authority checks are intact at `ward.rs` C L3323, L3339, L3437 and L3491, and two exact-SHA CI runs concluded success. Job-level results, including macOS, were not read. |
| 3 | Descriptor-not-authority | supported | **supported** | There are no API arm additions or removals. The EP-03 transport mapping is identical (`daemon.rs` C L6294-6297), and the CX-1e refusal is byte-identical at `api.rs` C L11441-11463 with its test unchanged. Every new Ward/Threads write is an existing writer at its existing gate position. Two notes for the reviewer, neither a counterexample: the new owner CLI `coven familiar create` (S5) writes identity inputs outside the daemon (create-only, same class as S3), and `GET /main-session/events` (SSE) is TCP-readable with no transport authority (outside Threads scope). The Set B Cave owner-local-proxy judgement carries. |
| 5 | evidence_replay_hash | supported | **supported** | Every cited source and covering test is byte-identical at C. Only line offsets moved. |
| 6 | min_visible | supported (daemon scope) | **supported (daemon scope)** | Daemon enforcement and tests are byte-identical (refusal at `api.rs` C L7901, `due` block at C L13373-13384), and every Threads-facing Cave file is byte-identical at `681130f3`. Browser reachability is still not found (coven-cave#5256). |
| 7 | Label-variant mapping | supported | **supported** | The #1276 tests are present and byte-identical (`ward.rs` C L6923-6954, C L6957-6994), and the rejections at C L605 and L654 are unchanged. The real-daemon malformed-load journey is still not found, and is optional. |
| 8 | Advisory probes | supported (vacuous at daemon) | **supported (vacuous at daemon)** | There is still no daemon `AdvisoryProbes` integration. #1306 changed only the Parse probe's YAML library. The sole authoritative probe use (`output_format_auto.rs:79-106`, parse/json on the output-format surface) is unchanged. |

No item is proposed as excluded-by-prior-decision.

## 5. Every "not found" / "not re-checked"

**Not found at Set C** (searched in this pass):

1. **Real-daemon malformed approval-tier load journey** (item 7). Grep results are in §2 item 7.
2. **Coven producer of `memory_entry_admitted`**, or any caller of `for_memory_entry_admitted` (item 1): 0 hits.
3. **Daemon `advisory_probes` / `AdvisoryProbes` integration** (item 8). The only tree hit is the design doc.
4. **New API dispatch arm**: none. There are 84 arm headers at both SHAs, with identical text.
5. **New production module referencing Ward/Threads code**: none. The census differs only by the test-only `api_power_loss_tests.rs`.
6. **Production caller of `gate_protected_edits` with non-empty `gated_targets`**: none. The only one is `api.rs` C L6277, with `&[]` at C L6285.
7. **Reader of `store_meta.ward_durability_barrier`**: none (`durability.rs:87` only).
8. **Cave non-test use of `principalKeyFingerprint`, `principal_key_fingerprint`, `principal:`, `signed_by`, `approverId`, `decisionActor`, `owner:local-ipc` or `familiars.ledger`** at `681130f3`: 0 hits.
9. **Cave request body setting a writer, actor or approver for a daemon Threads route**: none found.

**Not re-checked in this pass** (carried from Set B, or out of scope):

10. **Per-job CI results at `c658603d`.** That includes whether the Linux, Windows and macOS Threads suites ran and passed. Only run-level conclusions were read: 39379174287 push success, 39377159532 merge_group success. The jobs GET was declined by the operator and not retried.
11. **Threads daemon-compatibility run against `c658603d`.**
12. **A test that resumes a pre-`21988a00` legacy authority-lane apply intent** (CX-1e residual). It was not re-searched beyond the unchanged `api_legacy_submission_tests.rs`.
13. **A test staging the CX-1e shape with `client:unsigned`**, and **a test approving a scheduled Tier-1 proposal to success with writer `client:unsigned`** (CX-1d). Neither was re-searched. The relevant test files are unchanged blobs (`threads_protected_intake.rs`, `api_legacy_submission_tests.rs`), and `threads_e2e.rs` changed only in a fixture helper.
14. **Region-to-thread promotion code** (item 1, decision 3) and **familiar-contract PRs #3 and #4** (decision 5).
15. **Cave browser reachability of a pending proposal during `min_visible`** (item 6, coven-cave#5256).
16. **Transport-authority checks on `POST /main-session/reset` and `/rollover`.** These were not re-read; the ranges are identical, so the Set B observation stands. **New and similar:** `GET /main-session/events` takes no `RequestAuthority` (`daemon.rs` C L6344-6355).
17. **Eval-loop `{id}` validation** (EP-91). The arm and `familiar_workspace` are identical; the validation itself was not re-searched.
18. **Whether `serde_saphyr` accepts exactly the inputs `serde_yaml_ng` did** (item 8, #1306). Only the new test vectors were read.
19. **Out-of-band writers of `familiars.toml`, `ward.toml` and `pending/`.**
    - In Coven, the production writers found in the range diff are the new owner CLI `coven familiar create` (S5) and the existing `ward migrate` (S3).
    - Cave-side writers (for example the summoning circle) were **not re-audited**.
    - Whether S5's `familiars.toml.lock` coordinates with the daemon's own roster writer (`write_familiar_icon`, EP-30) was **not checked**.
20. **#1305 durability semantics** (item 4 scope). This covers whether the barriers, `fullfsync` and directory syncs close coven#1300; the new `api_power_loss_tests.rs` and `power_loss_tests.rs`; the `durable_fs` publication registry beyond the census and scheduler skip; and `store/schema.rs` pragma changes (`PRAGMA fullfsync = ON` added to `WRITABLE_CONNECTION_PRAGMAS`, C L630). It was read only to the extent needed for items 2 and 3.
21. **`control_plane.rs` internals beyond NEP-04 and `doctor_existing_store`**, `automations/doctor*.rs` beyond a Ward/Threads grep, and `main_session_events.rs` beyond `claims`, imports and a write grep. All were census-checked only.
22. **Cave files outside the Threads-facing set.** Of 2196 changed files in `c60bbef9..681130f3`, only the Threads-facing set in §3 and the authority-claim greps were read.
