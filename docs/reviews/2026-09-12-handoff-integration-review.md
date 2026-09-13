# Nova handoff integration review

**Disposition: integrate the delivery direction; keep Phase 5 active.**
The supplied [handoff](2026-09-12-nova-handoff.md) correctly prioritizes the
real daemon boundary over reopening the completed repository rollout. Its
current-state picture needs the independently landed daemon checkpoints below.
Both named source gaps remain at the refreshed integration head.

**Date:** 2026-09-12 UTC. **Tracking:** [#31][boundary], [#13][coherence],
`threads-qhu`. This is a bounded engineering review, not PR-wide approval,
Nova's independently attributable coherence decision, or Val's freeze.

## Scope and evidence standard

Review the new daemon delta, re-evaluate the two source findings, distinguish
landed repairs from open acceptance, and integrate those facts into the
delivery ledger and strategy. No daemon merge, runtime repair, compatibility
pin change, workflow activation, or human-gate decision is part of this change.

Source inspection establishes what the named files implement or assert.
GitHub run receipts establish the result and head of those runs, not a new
local reproduction. Issue comments supply attributed red-to-green reports;
their historical artifacts were not downloaded or rerun in this review.
No current-Threads-override daemon run was performed.

Canonical sources consulted, in precedence order:

- [Familiar Contract RFC-0001][rfc], sections 4.2 and 5.1-5.4, at
  `13d150a32a817da19bb4e5053f2205b15db0bb0a`.
- [Phase-5 decisions][phase5], especially sections 3, 6, and 7; no decision
  record is amended.
- Threads `approval.rs`, `identity_invariants.rs`, `surface_regions.rs`, and
  `validate.rs` at `ce7349c97d9ff08d70fc54472afe9fd3c52935ad`.
- The [E2E contract][e2e], compatibility manifest, delivery ledger, strategy,
  supplied handoff, and [previous readiness review](2026-09-11-landscape-and-readiness.md).

## Refreshed revision ledger

| Surface | Observation |
| --- | --- |
| Threads source baseline | `ce7349c97d9ff08d70fc54472afe9fd3c52935ad`; this review changes documentation only |
| [OpenCoven/coven#931][integration] | Open draft at `ea3f455aa66bb92880d95f57eeb7ca8d845b7fe6`, merge state `DIRTY` |
| Previously inspected daemon head | `8576f41e6d622f63a3576b85bd2d3142776e59ca` |
| Delta between those daemon heads | One commit, 127 additions to `crates/coven-cli/src/daemon.rs`, entirely two unit tests; no production authority or harness change |
| Checks attached to the new integration head | One completed, skipped `auto-merge` check; no passing daemon CI established at that head |
| [Coven main][main] | `aa527d2dbcd1edcda470483c1a91a519f24e8406`; still requests Threads `c102844` |
| Main daemon targets | `threads_protected_intake.rs` and `threads_terminal_recovery.rs` exist; `threads_e2e.rs` is absent |
| Integration dependency | Requests Threads `c3bd46bcadb6396db8436c47411a4d0eac17192b`; this is not the current Threads checkout |
| Stable compatibility pin | `39feb6de98816d10b490091e918f62035e6ce0df`, still `harness-required`; unchanged |
| Threads CI and effective main rules | Five required checks remain, including privacy; no pinned-daemon job or scheduled latest-main canary is wired |

### New-head disposition

The [new commit][delta] carries
`startup_budget_distinguishes_parent_cost_from_child_store_elapsed` and
`startup_budget_is_not_renewed_between_launch_and_readiness`. They use synthetic
timelines/controller observations, keep the original deadline, and explicitly
do not reproduce Windows startup failure. No blocking defect was found in
this bounded delta inspection.

[OpenCoven/coven#1019][diagnostic-pr] has a successful [child-head CI receipt][diagnostic-ci]
at `d0fab9edb4990b49ea093a80589eb5915e6b2b3b`; it merged into the
integration branch, not main. That result must not be relabeled as CI for
`ea3f455a`. [OpenCoven/coven#1001][diagnostic-issue] is closed for its
measurement/interpretation scope. Its [closure comment][diagnostic-close]
explicitly leaves startup causation and broader reliability with
OpenCoven/coven#884 and OpenCoven/coven#931.

## Landed repairs the continuation must preserve

| Checkpoint | Production landing and inspected evidence | Remaining limit |
| --- | --- | --- |
| Windows readiness / CI prerequisite, [OpenCoven/coven#970][windows-pr] | Main merge `4a04dc927f5ab1ba3d6a725373cf0e477eefd386`; [OpenCoven/coven#973][apt-pr] is closed as superseded, not separately merged | Does not explain the later intermittent startup failures |
| Deterministic clock, [OpenCoven/coven#968][clock-pr] | Main merge `066727f1a34377f4dfff0f5a7f80898a3a1c1ea5`; [issue continuation][corpus-comment] records the bounded clock prerequisite | Does not supply supported scheduled publication or close the corpus gate |
| Protected intake, [OpenCoven/coven#933][protected-pr] | Main merge `b7b3b4e17cfe21fd440e0f0429f3e38eae1785aa`; [source target][protected-tests], [PR CI][protected-ci], [merged-main CI][protected-main-ci] | Bounded refusal/recovery repair, not the full nine-case route matrix or separate protected-write authority |
| Typed terminal recovery, [OpenCoven/coven#932][terminal-pr] | Main merge `aa527d2dbcd1edcda470483c1a91a519f24e8406`; [source target][terminal-tests], [PR CI][terminal-ci], [merged-main CI][terminal-main-ci] | Synthetic legacy opened-window recovery, not supported scheduled intake or all terminal families |

The protected [implementation report][protected-comment] records identical
initial journeys at unchanged main (one pass, six failures) versus the repaired
checkpoint (seven passes). Its final PR run was **attempt 2**; preserve that
retry rather than count it as first-attempt reliability.

The terminal [implementation report][terminal-comment] records nine passes and
three failures with the new journey file on unchanged protected-checkpoint
production, followed by repaired checkpoint results. Its final PR and
merged-main runs both report success on attempt 1. Source inspection confirms
that the legacy tests begin with public coherence intake, then explicitly seed
stopped-daemon historical window state. That is useful recovery evidence, not
proof of the supported publication path.

These are attributable production fixes and bounded hosted results. They do
not close OpenCoven/coven#886 or OpenCoven/coven#887. In particular, quarantined
ambiguous applying state is not a fabricated rejection receipt or typed close.
Identity [OpenCoven/coven#969][identity-pr] and scheduled publication
[OpenCoven/coven#972][producer-pr] remain open, unmerged work.

## Re-evaluated source findings

### Supported auto-path reachability: still blocked

At `ea3f455a`, the [producer][producer] still requires non-empty default
region evidence and sets `path_tier_floor` to the minimum of path and region
floors. The [default registry][regions] supplies only floor 0 or floor 1.
The [scheduled loader][scheduler] rejects floor 0 entirely and rejects
`AutoRegression` at floor 1. Both auto variants therefore remain unreachable
through this producer with the built-in registry.

Next implementation/evidence task: the daemon producer owner must establish
a supported low-risk route and exercise auto with and without veto, familiar
review, human review, and rationale-required review through production intake.
Do not lower protected floors or inject scheduled envelopes to manufacture a
positive result. If this requires a canonical region/approval design change,
obtain that scoped decision first; this report does not choose one.

### Classification-time identity commitment: still missing

The [intake gate][identity-intake] evaluates configured predicates, and the
[applying/recovery commitment][binding] binds candidate identity later.
However, the producer still commits only the [diff/region hash][hash] into
classification; the scheduled envelope has no separate intake identity
commitment. Live-valid identity at apply is not proof that intake's
predicate/candidate evidence remained unchanged.

Next implementation task: OpenCoven/coven#885's daemon adapter must bind the
supported predicate/configuration and candidate evidence at classification,
then compare it at deadline and restart using the existing canonical identity
contracts. Add changed-but-still-valid evidence alongside invalid, missing,
ambiguous, and unsupported cases. Include persisted-envelope migration and
fail-closed downgrade handling. This is a source-confirmed missing obligation,
not a newly executed unauthorized-write counterexample.

The relevant core files are unchanged between the previous review's Threads
baseline and `ce7349c`; the daemon producer, loader, binding, and harness are
unchanged between the two daemon heads. Their current contents were inspected,
not accepted merely because the earlier review named them.

## Eight-obligation coherence evidence matrix

No row below checks the human checklist on [#13][coherence].

| Obligation | Evidence at the named revisions | Remaining proof / owner |
| --- | --- | --- |
| RFC round-trip | Recorded defaults and public typed contracts preserve separate Channel/ApprovalPath axes, delayed apply, and predicate authority | Complete runtime mapping after auto reachability and identity commitment repairs; Sage mapping, daemon implementation |
| Every approval path revalidates | Identity intake, scheduled reconstruction, and final applying commitment exist; familiar/human journey source is present | Supported auto variants and rationale-required path plus exact-head boundary results; Cody / daemon producer |
| Descriptor is not authority | Inspected intake evaluates predicates and materialized evidence rather than display labels | Cross-client/Cave conformance remains outside this source pass; Echo review, Charm live-daemon acceptance |
| Complete submission/opening/typed-close chain | [Corpus journey][corpus] compares submission classification with pending evidence; [terminal helper][terminal-helper] counts one opening and one typed close; main has bounded legacy recovery repairs | Prove correspondence and order for `proposal_submitted -> proposal_window_opened -> close(reason)` across publication/crash boundaries and all terminal branches; Echo / daemon |
| Replay commitment enforced | Loader reconstructs diff/region evidence; applying state adds a later identity commitment | Intake-to-deadline identity binding remains missing, including valid-to-valid changes and restart; Cody / OpenCoven/coven#885 |
| Minimum visibility | Corpus source checks before minimum visibility, at minimum visibility, pending inspection after restart, and apply after deadline | Exact deadline and full publication-to-human-reachability evidence at accepted head; daemon harness, Charm |
| Label/variant load correctness | `ApprovalPathEnvelope` validates label round-trip; scheduled wire loading rejects inconsistent derived fields | Selectable malformed-load daemon evidence, not only core construction tests; Cody / harness |
| Advisory separation | Core advisory types are distinct from deterministic identity evaluation | Preserve separation in any producer/client integration; no unimplemented model-probe path certified; Echo |

## Four root blockers and concrete ownership

All four GitHub issues remain open and their Threads beads remain blocked.
GitHub assigns them to `BunsDev`; familiar lanes below describe work ownership,
not independently authenticated repository authority.

| Blocker / journey | Current engineering disposition | Next concrete task / owner |
| --- | --- | --- |
| `threads-okc`, [OpenCoven/coven#885][identity-issue], J4/J8 | Predicate activation is drafted; classification-time identity binding is still missing | Reconcile OpenCoven/coven#969 with current main, add committed identity evidence and valid-to-valid deadline/restart regressions; Cody, Echo review |
| `threads-980`, [OpenCoven/coven#886][terminal-issue], J6 | Bounded typed-close/recovery fix is on main; full root acceptance remains incomplete | Preserve that repair in integration; prove all five close families, duplicate prevention, complete audit-chain crash ordering, and explicit ambiguous-applying disposition; daemon lane, Echo |
| `threads-dgg`, [OpenCoven/coven#887][protected-issue], J2/J5 | Bounded protected-intake/control-alias/recovery fix is on main | Preserve all landed refusals and finish the issue's nine-route/retry/cross-binding/recovery matrix; Cody / daemon, Echo |
| `threads-zav`, [OpenCoven/coven#888][corpus-issue], J7 | Clock prerequisite is on main; supported corpus journey remains in integration source | Land supported producer, run the synthetic retired fixture through migration/intake/visibility/restart/one terminal outcome, with unsupported negatives; daemon harness, Sage corpus mapping |

J1/J3 bounded write and out-of-band drift remain part of the full J1-J8 suite,
not substitutes for any root blocker. OpenCoven/coven#977 retains final
snapshot-to-committed-bytes interleavings; OpenCoven/coven#1000 retains artifact
provenance; OpenCoven/coven#884 and OpenCoven/coven#931 retain native reliability and integrated
coverage. All-ceremony reachability belongs to producer/harness work, not an
expanded definition of the minimal retired-corpus gate or implementation on #13.

## Recommended landing sequence

1. Reconcile OpenCoven/coven#931 with exact main `aa527d2d`, preserving the
   landed clock, protected-intake, typed-terminal, Windows, and CI repairs.
   Review the combined head. Do not re-merge OpenCoven/coven#932,
   OpenCoven/coven#933, or the
   superseded OpenCoven/coven#973 as if they were still pending.
2. Finish identity commitment and supported ceremony reachability in the
   existing daemon implementation lanes. Retain lower-level regressions and
   explicit migration/rollback treatment; do not change canonical floors or
   public contracts without a scoped decision.
3. Prove the current Threads Cargo override before feature-enabled real-daemon
   J1-J8. Assemble a separate pre-fix red, production commit, exact-head green,
   lower-level regression, sanitized artifact, and migration/rollback dossier
   for each root blocker. Include full audit-chain and final-commit evidence.
4. After reviewed daemon landing, propose its full-SHA compatibility pin.
   Observe the advisory lane before requiring Linux acceptance; add the
   non-blocking latest-main canary separately. Preserve run/attempt provenance
   and first-attempt failures, including the unresolved startup history.
5. Complete native OS and live-daemon Cave acceptance and measured reliability,
   then present the eight-obligation packet for the independently attributable
   coherence decision and the separate Val freeze decision.

## Integration, verification, and rollback

Changed documentation: this review, the supplied handoff's continuation link,
`docs/phases.md`, and `docs/strategy.md`. No Rust, dependency, schema, migration,
compatibility pin, workflow, or remote repository setting was changed. Private
interaction history is not part of the documentation change.

Evidence collection used:

```bash
git status --short
git -C ../coven/.worktrees/threads-production-integration diff --numstat \
  8576f41e6d622f63a3576b85bd2d3142776e59ca..ea3f455aa66bb92880d95f57eeb7ca8d845b7fe6
git -C ../coven/.worktrees/threads-production-integration diff \
  8576f41e6d622f63a3576b85bd2d3142776e59ca..ea3f455aa66bb92880d95f57eeb7ca8d845b7fe6 \
  -- crates/coven-cli/src/daemon.rs
gh pr view 931 -R OpenCoven/coven \
  --json headRefOid,isDraft,mergeStateStatus,statusCheckRollup
gh api repos/OpenCoven/coven/commits/ea3f455aa66bb92880d95f57eeb7ca8d845b7fe6/check-runs
gh api repos/OpenCoven/coven-threads/rules/branches/main
```

Those commands returned the revisions, 127-line test-only delta, skipped
head check, and five effective requirements recorded above. Main tree listing
and ancestry checks confirmed the two bounded test targets and prerequisite
landings. Run API queries confirmed the linked SHA/result/attempt receipts.
Source reads, rather than a downstream run, support the two renewed findings.
No new current-checkout acceptance or canary result is claimed.

Rollback is documentation-only: undo this review's prose and links without
changing live policy or discarding historical evidence. Leave the stable pin,
all four root blockers, and both human gates unchanged. Future daemon rollback
must preserve ambiguous pending/applying state and fail closed rather than
restore unvalidated writes or silently downgrade ceremonies.

[boundary]: https://github.com/OpenCoven/coven-threads/issues/31
[coherence]: https://github.com/OpenCoven/coven-threads/issues/13
[rfc]: https://github.com/OpenCoven/familiar-contract/blob/13d150a32a817da19bb4e5053f2205b15db0bb0a/rfcs/RFC-0001-familiar-contract.md#42-protected-invariants
[phase5]: https://github.com/OpenCoven/coven-threads/blob/ce7349c97d9ff08d70fc54472afe9fd3c52935ad/specs/PHASE-5-APPROVAL-SEMANTICS.md
[e2e]: https://github.com/OpenCoven/coven-threads/blob/ce7349c97d9ff08d70fc54472afe9fd3c52935ad/docs/testing/e2e-contract.md
[integration]: https://github.com/OpenCoven/coven/pull/931
[main]: https://github.com/OpenCoven/coven/commit/aa527d2dbcd1edcda470483c1a91a519f24e8406
[delta]: https://github.com/OpenCoven/coven/compare/8576f41e6d622f63a3576b85bd2d3142776e59ca...ea3f455aa66bb92880d95f57eeb7ca8d845b7fe6
[diagnostic-pr]: https://github.com/OpenCoven/coven/pull/1019
[diagnostic-ci]: https://github.com/OpenCoven/coven/actions/runs/34664640288
[diagnostic-issue]: https://github.com/OpenCoven/coven/issues/1001
[diagnostic-close]: https://github.com/OpenCoven/coven/issues/1001#issuecomment-5643714784
[windows-pr]: https://github.com/OpenCoven/coven/pull/970
[apt-pr]: https://github.com/OpenCoven/coven/pull/973#issuecomment-5610896170
[clock-pr]: https://github.com/OpenCoven/coven/pull/968
[corpus-comment]: https://github.com/OpenCoven/coven/issues/888#issuecomment-5639308800
[protected-pr]: https://github.com/OpenCoven/coven/pull/933
[protected-tests]: https://github.com/OpenCoven/coven/blob/b7b3b4e17cfe21fd440e0f0429f3e38eae1785aa/crates/coven-cli/tests/threads_protected_intake.rs
[protected-ci]: https://github.com/OpenCoven/coven/actions/runs/34654505395
[protected-main-ci]: https://github.com/OpenCoven/coven/actions/runs/34657221026
[protected-comment]: https://github.com/OpenCoven/coven/issues/887#issuecomment-5641819954
[terminal-pr]: https://github.com/OpenCoven/coven/pull/932
[terminal-tests]: https://github.com/OpenCoven/coven/blob/aa527d2dbcd1edcda470483c1a91a519f24e8406/crates/coven-cli/tests/threads_terminal_recovery.rs
[terminal-ci]: https://github.com/OpenCoven/coven/actions/runs/34670044061
[terminal-main-ci]: https://github.com/OpenCoven/coven/actions/runs/34671111724
[terminal-comment]: https://github.com/OpenCoven/coven/issues/886#issuecomment-5643352022
[identity-pr]: https://github.com/OpenCoven/coven/pull/969
[producer-pr]: https://github.com/OpenCoven/coven/pull/972
[producer]: https://github.com/OpenCoven/coven/blob/ea3f455aa66bb92880d95f57eeb7ca8d845b7fe6/crates/coven-cli/src/threads_gate.rs#L1318-L1425
[scheduler]: https://github.com/OpenCoven/coven/blob/ea3f455aa66bb92880d95f57eeb7ca8d845b7fe6/crates/coven-cli/src/proposal_scheduler.rs#L20-L158
[regions]: https://github.com/OpenCoven/coven-threads/blob/ce7349c97d9ff08d70fc54472afe9fd3c52935ad/crates/coven-threads-core/src/surface_regions.rs#L306-L545
[hash]: https://github.com/OpenCoven/coven-threads/blob/ce7349c97d9ff08d70fc54472afe9fd3c52935ad/crates/coven-threads-core/src/surface_regions.rs#L185-L249
[identity-intake]: https://github.com/OpenCoven/coven/blob/ea3f455aa66bb92880d95f57eeb7ca8d845b7fe6/crates/coven-cli/src/threads_gate.rs#L250-L335
[binding]: https://github.com/OpenCoven/coven/blob/ea3f455aa66bb92880d95f57eeb7ca8d845b7fe6/crates/coven-cli/src/api.rs#L12667-L12844
[corpus]: https://github.com/OpenCoven/coven/blob/ea3f455aa66bb92880d95f57eeb7ca8d845b7fe6/crates/coven-cli/tests/threads_e2e.rs#L302-L402
[terminal-helper]: https://github.com/OpenCoven/coven/blob/ea3f455aa66bb92880d95f57eeb7ca8d845b7fe6/crates/coven-cli/tests/threads_e2e.rs#L1473-L1511
[identity-issue]: https://github.com/OpenCoven/coven/issues/885
[terminal-issue]: https://github.com/OpenCoven/coven/issues/886
[protected-issue]: https://github.com/OpenCoven/coven/issues/887
[corpus-issue]: https://github.com/OpenCoven/coven/issues/888
