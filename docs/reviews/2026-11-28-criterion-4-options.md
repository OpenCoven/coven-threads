# Criterion 4 options: one typed terminal close for every opened veto window

**Engineering research, 2026-11-28.** This memo selects nothing. Val's
[coherence decision](https://github.com/OpenCoven/coven-threads/issues/13#issuecomment-7207324591)
defers criterion 4 and keeps its contract as written: *every opened veto window
gets exactly one typed terminal close; a census classification is not a close.*
This memo asks how damaged histories can meet that contract. It compares
options, recommends one, and prototypes it in the Threads core. The criterion-4
disposition, #13 and #14 remain Val's.

| Source | Revision |
| --- | --- |
| Coven main (read, not executed) | `9415c3098f9635963db593970b8943370e90bf81` |
| Threads main | `4955cf8dc97e7fde07adf4e233fd6e060ee584e4` |
| Prototype branch | `research/criterion-4-terminal-disposition-20261128` |
| Retained journey evidence | Coven #1716 candidate `8edeb939`; [evidence on draft #128](https://github.com/OpenCoven/coven-threads/pull/128) |

The Coven source map was read at the exact commit with `git show`. File and line
references below are to that commit. No Coven checkout, branch, claim or daemon
run was made: task-worktree allocation was at 12 of 12 and this session had no
retirement authorization. Claims marked *(inference)* rest on reading code, not
on execution.

## 1. What is already guaranteed, and what is missing

**At most one terminal is enforced twice.**

- In the store, core trigger `ward_audit_require_single_terminal_insert`
  (`crates/coven-threads-core/src/audit.rs`) aborts a second terminal.
- In the daemon, every close goes through one writer,
  `append_proposal_decision_audit_with_probe_summary` (`api.rs:14912`). It
  refuses a second terminal and inserts inside `store::durable_transaction`.

**At least one terminal is not guaranteed.** That is the criterion-4 gap. No
daemon path writes a close for a quarantined, orphaned, untrusted or
inconsistent history, and no operator command can either:

- `coven ward approve|reject` needs a file in `pending/` and otherwise returns
  `proposal-not-found` (`api.rs:8992-9000`, `11317`);
- `ward_audit` is append-only.

Coven's design record says so explicitly
(`docs/design/threads-terminal-recovery.md`, "Quarantine is not a close"):

> Quarantine does **not** satisfy the typed terminal invariant and requires
> separate resolution. … Do not delete quarantined intent or classify it as a
> completed rejection to make the terminal count balance.

Two Threads contracts already expect the gap to be closed:

- E2E contract J8 requires that after committed evidence is altered and the
  daemon restarts, "replay detects the mismatch" and "restart neither loses nor
  duplicates the terminal disposition";
- §12 sets "duplicate or missing terminal close records remain zero" as a
  success metric.

The retained #1716 journeys show the opposite for tampered histories: zero
terminals across restarts.

## 2. Where each census class comes from at Coven `9415c309`

The census (`crates/coven-cli/src/ward_audit_census.rs`, `classify` at `312-385`)
picks the first class that applies, in this order:

1. any issue → InconsistentHistory;
2. a terminal → TypedTerminalRecorded;
3. an apply intent → UnprovableApply;
4. a pending item or claim → OpenUnverified;
5. a quarantine entry → QuarantinedOpening;
6. untrusted artifacts → UntrustedArtifacts;
7. otherwise OrphanedOpening.

| Class | Reachable without tampering? | Evidence |
| --- | --- | --- |
| OpenUnverified | Yes. The normal state before the terminal. | Supported intake: `api.rs:1102` → `apply_familiar_edits` (`6093`) → `stage_scheduled_coherence_proposal` (`threads_gate.rs:1439`). |
| UnprovableApply | Yes, transiently. It is the window between the FULL intent commit (`api.rs:11086`) and finalize. Power-loss lane 3 asserts it, then `typed_terminal_recorded` after recovery (`api_power_loss_tests.rs:122-156`). *(inference)* It can persist: recovery returns a non-terminal 409 for ever when the recovery commitment diverges or adjudication blocks (`api.rs:10148-10165`, read directly). It is quarantined when the familiar or Ward becomes unavailable (`9642-9718`). | Because the intent ranks above artifacts, a quarantined interrupted apply also shows as UnprovableApply. |
| QuarantinedOpening | The retained journeys use tampering. The crash-only decision-origin route (#1618) closed with #1643's FULL reservation barrier (`api.rs:8976-8981`). *(inference, untested)* A daemon upgrade can quarantine an older pending file that no longer parses or replays (`11553`, `11583`; `proposal_scheduler.rs:153`, `196`), leaving zero terminals. | Scheduler quarantine `11371`; decision quarantine `11387`, which returns `terminal:false, manualRecoveryRequired:true`. |
| OrphanedOpening | *(inference)* No route without external deletion. Artifacts are removed only after a durable terminal (`13470`, `14696`). Quarantine and restore only rename within `pending/`. | Retained test deletes the file itself (`tests/threads_terminal_recovery.rs:34-51`). |
| UntrustedArtifacts | *(inference)* Needs the same loss as orphaning, plus a leftover `.staged` file from an interrupted `write_atomic` (`proposal_store.rs:310`). No cleanup of those files was found. | Unknown suffixes are counted untrusted (`ward_audit_census.rs:396-465`). |
| InconsistentHistory | *(inference)* Blocked by the guards above for duplicate terminals, duplicate openings (`INSERT … WHERE NOT EXISTS`, `api.rs:12222-12245`) and untyped closes. Reachable through tampering or pre-trigger legacy rows, including any legacy `proposal_expired` terminal (counted as MissingTypedClose). | Not traced exhaustively: scope divergence between opening and terminal. |

## 3. What any answer must preserve

1. **Truthful close meanings** (`specs/PHASE-5-APPROVAL-SEMANTICS.md` §4).
   `revalidation_failed` means "replay could not produce authoritative
   evidence"; `evidence_diverged` means "replay … and the evidence hash
   changed". Both are `proposal_rejected`, which asserts that nothing was
   applied. `vetoed` asserts a principal veto before apply; `applied` asserts a
   committed write with a matched replay.
2. **No fabricated no-write assertion.** The Coven design record forbids it.
3. **Delayed apply only** (decision 2). Rolling back a landed write is
   provisional-apply semantics and is out.
4. **Minimum visibility.** An automatic close must not shorten a window
   (criterion 6).
5. **Fail closed.** A transient I/O or authority failure is never proof.
6. **One audit store; R4 change rules.** A new close reason changes a public
   exhaustive enum, the trigger's tag list and the `current_v020` fingerprint.

**When is "no write happened" provable?**

The apply intent carries the decision, recovery commitment, weave hash and
before-images (`api/proposal_apply_intent.rs:1-40`). Since #1305 (Coven
`c33da615`, 2026-10-15, first release v0.4.11) it has been committed with
`synchronous=FULL` before any target is swapped (`docs/ward-durability.md`,
step 4). So for a window opened by a daemon carrying #1305, **no intent row**
proves **no write started**. Two limits follow:

- **Earlier histories.** An intent written at NORMAL could be lost to power loss
  while target bytes survived, so its absence is not proof.
- **Intent present.** Matching every before-image proves the current bytes, not
  the history. A landed write later reverted by someone else looks the same. A
  rejection there would be an unproven assertion.

## 4. Options

| | A: proven-subset recovery close | B: new typed reason | C: leave unclosed, classify | D: owner-only explicit disposition |
| --- | --- | --- | --- | --- |
| What | Coven recovery appends `proposal_rejected` with an existing reason, only where §3 proves no write; holds the rest with a typed hold | Add a reason (for example `recovery_unresolved`) for histories whose outcome cannot be determined; owner-attested, never automatic | Keep census classes as the record of damaged windows | New owner IPC command that closes a damaged window |
| Contract fidelity | Exact for the subset it closes; leaves the residual open | Closes everything, but "closed with unknown outcome" is weaker evidence | **Conflicts** with Val's retained contract ("a census classification is not a close") | Depends on reason: `vetoed` is truthful only before any apply; for interrupted applies it needs B |
| Fail-closed | Yes; contradictory facts hold | Yes if consumers treat the new reason as "may have been written" | Neutral | Yes; human in the loop |
| Schema / migration | None | `current_v020` → new fingerprint; trigger tag list; census and store fingerprint checks; rollback migration | None | None with existing reasons |
| Semver | Additive (`0.2.x`); prototype types are `#[non_exhaustive]` | `WindowCloseReason` is exhaustive: `0.3.0` | None | None (Coven-only) |
| Cross-repo impact | Coven reconciler and a durability marker; Cave unchanged (existing reasons) | Coven, Cave rendering, census; `terminal_decision_label` (`api.rs:14303-14316`) would mislabel a new reason "rejected" | None | Coven command, Cave surface |
| What it leaves | Interrupted applies, histories from before #1305, inconsistent histories | Inconsistent histories (append cannot repair) | Everything | Interrupted applies without B |

## 5. Recommendation

**Adopt A now. Treat B as Val's decision for the residual.**

1. **A covers every no-intent damage class at durable revisions** with no
   schema, semver or Cave change. That is quarantined, orphaned and untrusted
   windows. Concretely, when the deadline has passed:
   - quarantined, replay diverged → `evidence_diverged`;
   - quarantined, evidence not producible → `revalidation_failed`;
   - orphaned or untrusted (staged bytes gone) → `revalidation_failed`;
   - quarantined, evidence still replays → **held**, so the owner can restore
     it.

   This matches the existing documented disposition "one opening, no apply
   intent, confirmed invalid authority → typed `revalidation_failed`". It
   changes one row of the Coven design record: "audit opening without any
   retained pending/claim item: not automatically repaired". That is a Coven
   owner decision.
2. **Interrupted applies stay held.** This keeps the Coven record's rule against
   fabricated no-write assertions. If Val keeps the contract literal for them,
   only B (owner-attested, never automatic) closes them truthfully.
3. **Histories from before #1305 stay held** unless the owner chooses B or a
   named residual. Fail closed: Coven should write a durable store marker when a
   daemon carrying #1305 first opens the store, and treat only openings after
   that marker as `AbsentDurable`. The row timestamp alone is not enough.
4. **Inconsistent histories are an integrity incident under every option.**
   Appending cannot repair them. The criterion-4 record should name them
   separately.

The September census of the maintainer workstation found zero opened windows. If
the profiles in scope still hold none, the legacy residual is empty there. That
is a fact to re-establish at the chosen tuple, not one to assume.

## 6. Prototype

On the prototype branch, `coven_threads_core::window_recovery::recovery_disposition`
encodes A as a pure function from `WindowRecoveryFacts` to `TerminalDisposition`:

- `AlreadyClosed`;
- `Ordinary`, where the existing paths own the close;
- `Close` with existing reasons only;
- `Hold`, with one of `InconsistentHistory`, `AwaitDeadline`,
  `TransientAuthority`, `NoWriteUnproven`, `InterruptedApply`,
  `RestorableEvidence` or `ContradictoryFacts`.

Nothing calls it. All public items are additive and the enums are
`#[non_exhaustive]`. The only change to existing code makes
`validate_window_close_detail` `pub(crate)` so the prototype's tests can call
it.

Failing-first:

- Commit `f3ca0f8` adds 12 tests against a stub that holds every window. 10
  failed; the 2 invariant tests passed vacuously, since they cannot fail on a
  stub that never closes.
- Commit `9d0d716` implements the rules. 12 of 12 pass.

The tests check every combination of facts (3 opening counts × 3 terminal counts
× 2 × 2 × 4 × 4 × 4 = 2,304). Across all of them, recovery produces exactly four
close shapes. Each one:

- is a `proposal_rejected`;
- passes the core's own window-close validation;
- appears only with one consistent opening, no terminal, a passed deadline,
  durable proof of no intent, no active item and a non-transient replay
  outcome.

Recovery never produces `applied` or `vetoed`.

**Encoding limit.** The core requires `replay_hash_matched = Some(false)` for
`revalidation_failed`. For a window whose evidence cannot be produced, that
records "no authoritative match", not "replayed and differed". The reason field
carries the distinction; the boolean does not.

Local verification uses Homebrew rustc 1.99.1. The pinned 1.88.0 gate runs only
in CI.

```text
cargo test --locked -p coven-threads-core --lib window_recovery   12 passed (red: 2 passed, 10 failed)
cargo clippy --locked --workspace --all-targets -- -D warnings    clean
cargo test --locked --workspace                                   321 passed, 0 failed
```

## 7. What Coven would need for A (not started)

1. A startup and periodic reconciliation pass, under `ward_write_audit_lock`.
   It would enumerate openings without terminals (reusing the census `load`),
   gather `WindowRecoveryFacts`, and append the close in one durable transaction
   before consuming any retained artifact.
   - The existing `append_open_window_revalidation_failure` (`api.rs:14621`)
     takes `files_touched` and `channel` from a `PendingProposal`.
   - For an orphaned window those must come from the durable `proposal_submitted`
     receipt instead. That is a real gap to design.
2. The durable post-#1305 marker that defines `AbsentDurable`.
3. Quarantined artifacts preserved after the close, for investigation.
4. Daemon-boundary regressions, red to green. The six #1716 journey families
   change expectation:

   | Journey | Expected after A |
   | --- | --- |
   | Orphaned opening | One `revalidation_failed` after the deadline |
   | Malformed pending (no intent) | One `revalidation_failed` or `evidence_diverged` |
   | Synthetic intent | Held, `InterruptedApply` |
   | Interrupted claim with Ward intact | Unchanged: applied once |
   | Interrupted claim with Ward removed | Held, `InterruptedApply` |
   | Live window | Unchanged: no terminal before the deadline |

   Restart must never add a second close.
5. Pinning: Coven imports Threads `7b5a9e68`. Adopting the function means moving
   that pin to a reviewed Threads revision, an R4 change with the usual
   evidence.

## 8. Decisions for Val

1. **Recovery semantics.** Adopt A (the prototype's rules) as the criterion-4
   recovery semantics, modify it, or reject it. If adopted, the Coven owner
   implements §7 and updates the "Quarantine is not a close" row for orphaned
   windows.
2. **Residuals.** For interrupted applies whose outcome cannot be determined,
   and for histories from before #1305, choose one:
   - B, a new owner-attested typed reason (`0.3.0` plus a schema migration);
   - a named residual outside criterion 4, which narrows the retained contract;
   - continued deferral.
3. **Durability boundary.** Use a store marker (fail closed), or daemon revision
   v0.4.11 or later, to decide when a missing intent counts as proof.
4. **Inconsistent histories.** Confirm they are recorded as integrity incidents,
   not as criterion-4 closes.

## 9. Residual risk and rollback

- The Coven map is static reading at one commit. Rows marked *(inference)*,
  especially upgrade-induced quarantine and leftover `.staged` files, need
  daemon-boundary tests before anyone relies on them.
- A's no-write proof assumes storage honors fsync and SQLite FULL. The Coven
  durability record already places storage that ignores fsync outside the
  protocol.
- Nothing here changes runtime behavior, the pin, the schema or any human gate.
  Rollback is closing the draft PR and deleting the branch.
