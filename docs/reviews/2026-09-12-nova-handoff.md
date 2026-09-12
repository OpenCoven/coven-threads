# Nova handoff: Threads delivery and Phase-5 readiness

The documentation, repository governance, and privacy rollout are complete.
Your next priority is the real Coven daemon integration, not reopening the
finished repository rollout. **Phase 5 remains active and is not ready to
freeze.**

| Field | Value |
| --- | --- |
| Prepared for | Nova |
| Snapshot | 2026-09-12 UTC |
| Threads main | `ce7349c97d9ff08d70fc54472afe9fd3c52935ad` |
| Scope | Engineering handoff, not independent coherence sign-off or Val's freeze |

**Continuation, 2026-09-12:** the [handoff integration review](2026-09-12-handoff-integration-review.md)
re-evaluates the new daemon head and records bounded clock, protected-intake,
and typed-terminal checkpoints already landed on Coven main. It also records
the closed measurement scope of OpenCoven/coven#1001, without claiming startup
reliability. Use that review for the refreshed disposition, evidence matrix,
owners, and landing sequence; the snapshot below remains historical.

## Start here

Read the [delivery strategy][strategy], [phase ledger][phases], and
[source-cited readiness review][review]. Those links are pinned to the
Threads revision above, so you can share this document without a local checkout.

Refresh live state before acting. From the Threads checkout:

```bash
git status --short
gh pr view 931 -R OpenCoven/coven \
  --json headRefOid,isDraft,mergeStateStatus,statusCheckRollup
gh api repos/OpenCoven/coven-threads/rules/branches/main
```

**Important change since the audit:** OpenCoven/coven#931 has a new head.
The old source findings and hosted results must not be transferred to it
without fresh review.

## What landed

| PR | Delivered | Merge commit | Exact PR-head CI |
| --- | --- | --- | --- |
| [#46][pr46] | Governance ledger and authorized solo-maintainer policy | `91ff511609cfb717bf71c3cafe07c0cbb2a2a317` | [34664611356][ci46], passed |
| [#40][pr40] | Separate privacy job, approved historical source-reference correction, and explicit checker-trust limits | `7168dae10f6b59bad8bc653b95f51911334ded74` | [34664678160][ci40], passed |
| [#47][pr47] | Landscape audit, delivery strategy, and reconciled documentation | `3ff49c02abe5693a58529265fca5bf253f4f4844` | [34664819612][ci47], passed |
| [#48][pr48] | Documentation of the separately authorized privacy required-check activation and rollback | `ce7349c97d9ff08d70fc54472afe9fd3c52935ad` | [34675262894][ci48], passed |

Final Threads [main CI 34675300710][main-ci] also passed at `ce7349c`.
On #48, GitHub returned exactly five required checks, all successful, before
normal merge. These are repository results, not daemon-boundary acceptance.

The audit corrected library-versus-daemon authority claims, clarified which
commands ship, and distinguished package/profile versions, historical plans, replay
evidence limits, and current-versus-historical delivery status. The full
`proposal_submitted -> proposal_window_opened -> close(reason)` obligation
is retained, rather than reduced to terminal-event uniqueness.

No Rust, audit-schema, migration, dependency, or compatibility-pin change was
made in this work. The approved #40 historical citation correction changed no
normative authority decision. No private interaction history was committed.

## Current human approval and repository policy

The maintainer confirmed that this is a solo-maintainer repository and
explicitly approved the documentation work, eligible merges, the
[solo-maintainer policy][solo-approval], and later the
[privacy required-check activation][privacy-approval].

Active ruleset `22910327` protects `main` with:

- Pull requests, resolved review conversations, and up-to-date branches.
- Five required checks bound to GitHub Actions app `15368`: `Secret scanning`,
  `Rust quality and repository contract`, `Cargo test (compatibility baseline)`,
  `Nextest telemetry (JUnit, fail on flaky)`, and `Privacy policy guard`.
- Branch-deletion and non-fast-forward protection, with no bypass actors.
- Zero required approving reviews; latest-push and extra unattributed-change
  approvals disabled. Stale-review dismissal remains enabled.

The [contributor policy][contributing] requires recorded, scoped human
authorization before agent merges. GitHub does not enforce that conversational
approval process or provide a second-person review under this setup.
Do not request an imaginary teammate, submit author self-approval, or treat
an agent persona as independently authenticated authority.

Privacy is now required, not merely proposed. The checker and calling workflow
remain PR-controlled. This is not immutable enforcement against PR authors
or proof of complete PII coverage. See [the security policy][security].

Neither repository policy decision closes the Phase-5 coherence or freeze
gates. The maintainer's approvals here were not a blanket acceptance of
unreviewed daemon changes.

## Current daemon state and evidence limits

| Surface | Refreshed observation |
| --- | --- |
| [OpenCoven/coven#931][daemon-pr] | Open draft at `ea3f455aa66bb92880d95f57eeb7ca8d845b7fe6`; merge state `DIRTY` |
| Checks visible on that draft head | Only a skipped `auto-merge` check was returned; no passing daemon CI result was established for this head |
| Coven main | `aa527d2dbcd1edcda470483c1a91a519f24e8406`; `coven-cli` still requests Threads `c102844`; `crates/coven-cli/tests/threads_e2e.rs` is absent |
| Threads compatibility pin | `39feb6de98816d10b490091e918f62035e6ce0df`, still `harness-required` |
| Required daemon lane and latest-main canary | Not wired in Threads CI; the TOML nightly declaration does not create a workflow |

The earlier source review examined daemon head
`8576f41e6d622f63a3576b85bd2d3142776e59ca`, **not** the new draft head above.
This handoff refreshed GitHub state and the current daemon-main manifest/test
listing; it did not re-review the new daemon implementation or run it.

Earlier hosted integration [run 34489425644][daemon-ci] tested synthetic merge
`0d8ccfca1725475b5e45f2ec47d8c4c2c45bc89d`. Its inspected artifact
`10157780426` contained 28 passing manifests covering 23 distinct scenario
names, using older Threads `c3bd46bcadb6396db8436c47411a4d0eac17192b` with
`local_threads_override_active=false`.

That artifact is progress, not current-checkout acceptance. Preserve the
recorded `threads_dirty=true` limitation and the omitted feature flag in the
generic command provenance. Earlier startup failures remain unexplained by
that later green. Do not count retries as first-attempt reliability or assume
green at one head applies to another.

## The four unresolved remediation gates

All four downstream issues were still open at this handoff.

| Gate | Canonical implementation tracking | Required closure |
| --- | --- | --- |
| `threads-okc` | [OpenCoven/coven#885][identity] | Authoritative identity predicates and evidence bound at intake, deadline replay, and restart |
| `threads-980` | [OpenCoven/coven#886][terminal] | Full submission/opening/typed-close linkage, exactly one close per opened window, and complete failure/recovery handling |
| `threads-dgg` | [OpenCoven/coven#887][protected] | No protected `SOUL.md` write authority through proposal/stage/approve routes, including retry and recovery |
| `threads-zav` | [OpenCoven/coven#888][corpus] | A valid retired-Ward case through supported intake, visible scheduling, restart, and one terminal outcome without loss or double apply |

The earlier review identified two specific acceptance gaps to **re-evaluate
against the new daemon head**, not assume still present:

1. **Supported auto-path reachability.** The inspected producer combined
   built-in region floors with scheduler restrictions that prevented a
   positive `AutoRegression` route. Prove both auto variants and the human
   paths through supported production intake. Do not lower protected floors
   or use test-only envelope construction to manufacture acceptance.
2. **Classification-time identity commitment.** The inspected replay hash
   covered diff/region evidence rather than the separately required identity
   predicate/candidate evidence. Later live revalidation and final-commit
   binding did not prove intake-to-deadline commitment. Include identity
   evidence that changes but remains valid, not only invalid or unavailable cases.

Also preserve the complete audit obligation: matching submission and opening
records, their ordering/correspondence across crash boundaries, and one
correctly typed terminal close. A count assertion alone does not prove it.

The broader all-ceremony work belongs to the daemon producer/harness lane:
[OpenCoven/coven#972][producer] and [OpenCoven/coven#884][harness], integrated
through OpenCoven/coven#931. OpenCoven/coven#888 is the narrower retired-corpus
remediation. Threads [#13][coherence] is an acceptance gate, not an
implementation owner.

## Recommended next sequence

1. Refresh OpenCoven/coven#931, inspect changes since the reviewed head, and
   reconcile its merge conflicts with current Coven main. Review the combined
   revision; do not merge every overlapping historical component branch.
2. Re-evaluate the two findings above and all four open issues. Separate
   implemented repairs from still-missing process-boundary evidence.
3. Prove the current Threads checkout is active with `cargo metadata` or
   `cargo tree` before any downstream result is counted. Use the real daemon,
   supported transport, synthetic credentials, disposable filesystem/SQLite
   state, and deterministic time.
4. Assemble attributable pre-fix red, production fix, exact-head green,
   lower-level regression, sanitized artifacts, and migration/rollback notes
   for each blocker. Include audit-chain continuity and final Gate-4
   snapshot-to-committed-bytes interleavings.
5. After reviewed daemon landing, propose the stable full-SHA compatibility
   pin and advisory-to-required Linux rollout. Keep the non-blocking
   latest-main canary separate. Follow with native OS and live-daemon Cave
   acceptance; do not invent a 30-day reliability measurement.
6. Present a fresh eight-obligation coherence packet for an independently
   attributable decision, then the separate Val freeze decision. Do not
   check either gate merely because repository CI passes.

Use the [E2E contract][e2e] for J1-J8: bounded authorized write, unsigned/unbound
protected write, out-of-band drift, identity mutation/replay, protected-route
prohibition, terminal matrix, retired-Ward schedulability, and committed-evidence
drift/restart.

For #13, preserve all eight review obligations: RFC round-trip, revalidation on
every approval path, descriptor-not-authority, audit completeness, replay
binding, minimum visibility, label/variant load correctness, and advisory
separation.

**Requested return from you:** a new exact-head review disposition, evidence
matrix, remaining blocker list with owners, and recommended landing sequence.
If a finding is fixed, identify the production commit and the proof. If it is
not, state the next concrete implementation or evidence task.

## Ownership, tracking, and rollback

Threads owns validator, approval/replay, audit-schema, and portability
contracts. Coven owns authentication, filesystem/SQLite effects, staging,
scheduling, apply, and recovery. Familiar Contract owns identity. Cave renders
and forwards; Psyche owns orchestration lifecycle. Do not create a second
canonical writer, identity model, or audit store.

Preserve source precedence: Familiar Contract RFC-0001, frozen/active Threads
decisions, public Rust contracts/vectors, then explanatory docs. Read the
[Phase-5 decision record][phase5] before changing an approval contract.

Completed rollout tasks: `threads-1e9`, `threads-chk`, `threads-xgc`, and
`threads-t6t`. [#39][privacy-issue] is closed.
[Boundary epic #31][boundary] and [coherence gate #13][coherence] remain open.
Beads is local/shared work tracking, not a clean-clone prerequisite. Public
GitHub links carry the shareable evidence.

To roll back privacy enforcement, remove only `Privacy policy guard` from the
live required-check list before removing its workflow job, preserving other
current settings. Reverting documentation cannot change GitHub policy.
Restoring independent-review requirements is a separate maintainer decision,
not part of this handoff. Keep the existing compatibility pin until reviewed
replacement evidence is ready.

Adjacent promotion-channel, profile-adoption, and release/license work remains
separate. Do not reopen the completed docs rollout or broaden authority to
avoid the unfinished daemon proof.

[strategy]: https://github.com/OpenCoven/coven-threads/blob/ce7349c97d9ff08d70fc54472afe9fd3c52935ad/docs/strategy.md
[phases]: https://github.com/OpenCoven/coven-threads/blob/ce7349c97d9ff08d70fc54472afe9fd3c52935ad/docs/phases.md
[review]: https://github.com/OpenCoven/coven-threads/blob/ce7349c97d9ff08d70fc54472afe9fd3c52935ad/docs/reviews/2026-09-11-landscape-and-readiness.md
[contributing]: https://github.com/OpenCoven/coven-threads/blob/ce7349c97d9ff08d70fc54472afe9fd3c52935ad/CONTRIBUTING.md
[security]: https://github.com/OpenCoven/coven-threads/blob/ce7349c97d9ff08d70fc54472afe9fd3c52935ad/SECURITY.md
[e2e]: https://github.com/OpenCoven/coven-threads/blob/ce7349c97d9ff08d70fc54472afe9fd3c52935ad/docs/testing/e2e-contract.md
[phase5]: https://github.com/OpenCoven/coven-threads/blob/ce7349c97d9ff08d70fc54472afe9fd3c52935ad/specs/PHASE-5-APPROVAL-SEMANTICS.md
[pr46]: https://github.com/OpenCoven/coven-threads/pull/46
[pr40]: https://github.com/OpenCoven/coven-threads/pull/40
[pr47]: https://github.com/OpenCoven/coven-threads/pull/47
[pr48]: https://github.com/OpenCoven/coven-threads/pull/48
[ci46]: https://github.com/OpenCoven/coven-threads/actions/runs/34664611356
[ci40]: https://github.com/OpenCoven/coven-threads/actions/runs/34664678160
[ci47]: https://github.com/OpenCoven/coven-threads/actions/runs/34664819612
[ci48]: https://github.com/OpenCoven/coven-threads/actions/runs/34675262894
[main-ci]: https://github.com/OpenCoven/coven-threads/actions/runs/34675300710
[solo-approval]: https://github.com/OpenCoven/coven-threads/issues/31#issuecomment-5642501227
[privacy-approval]: https://github.com/OpenCoven/coven-threads/issues/39#issuecomment-5643712162
[daemon-pr]: https://github.com/OpenCoven/coven/pull/931
[daemon-ci]: https://github.com/OpenCoven/coven/actions/runs/34489425644
[identity]: https://github.com/OpenCoven/coven/issues/885
[terminal]: https://github.com/OpenCoven/coven/issues/886
[protected]: https://github.com/OpenCoven/coven/issues/887
[corpus]: https://github.com/OpenCoven/coven/issues/888
[producer]: https://github.com/OpenCoven/coven/pull/972
[harness]: https://github.com/OpenCoven/coven/issues/884
[coherence]: https://github.com/OpenCoven/coven-threads/issues/13
[privacy-issue]: https://github.com/OpenCoven/coven-threads/issues/39
[boundary]: https://github.com/OpenCoven/coven-threads/issues/31
