# Delivery strategy

Finish the real-daemon authority boundary before declaring Phase 5 complete
or expanding its authority claims.

**Planning baseline: 2026-09-11; repository rollout updated 2026-09-12 (UTC).**
This is an explanatory delivery strategy,
not a new normative contract, task tracker, or approval. [GitHub #31](https://github.com/OpenCoven/coven-threads/issues/31)
and its linked issues own cross-repository work; Beads owns local task status.
The [delivery ledger](phases.md) records implementation evidence, and the
[readiness review](reviews/2026-09-11-landscape-and-readiness.md) records the
findings behind this sequence.
The [September 12 continuation](reviews/2026-09-12-handoff-integration-review.md)
refreshes the daemon head, landed checkpoints, and remaining evidence owners.
The [execution checkpoint](reviews/2026-09-12-execution-checkpoint.md) records
the subsequent advisory-lane landing and exact-current-checkout daemon result.
The [boundary remediation packet](reviews/2026-09-12-boundary-proof-remediation.md)
records the later Windows, identity-replay, and bounded-auto contributions.

## Starting position

| Surface | Present in the repository | Not established by that fact |
| --- | --- | --- |
| Rust core | Typed validation, approvals, identity predicates, audit schema/migrations, `.weave` portability, and conformance suites | Authentication, filesystem effects, or every daemon route |
| Automation Authority Profile v1 | Schemas, reference validator, signed evidence contracts, and exact vectors | Production scheduler, credential issuance, runtime-adapter adoption, or Rust API integration |
| Repository quality | Pinned toolchain/actions, Cargo baseline, Nextest, informational coverage, required secret scanning, and a required separate privacy job | Required daemon compatibility, immutable checker authority, or a coverage ratchet |
| Governance | Authorized solo-maintainer policy, required PRs, resolved conversations, and five strict required checks | GitHub-enforced independent review, a required pinned-daemon check, or either Phase-5 human decision |
| Downstream integration | Earlier release-source integration and substantial draft process-boundary evidence | Reviewed landing of the four remediations, full acceptance, or installed-binary conformance |

The core's three verdicts and the automation profile's four outcomes answer
different questions. Do not add an automation approval outcome to the Rust
gate or infer daemon adoption merely because a reference vector passes.

## Sequence and exit evidence

### 1. Finish the bounded documentation and privacy landing

**Owner:** authenticated maintainer under the [solo-maintainer merge policy](../CONTRIBUTING.md#solo-maintainer-merge-policy).
**Public tracking:** [governance ledger #46](https://github.com/OpenCoven/coven-threads/pull/46),
[privacy rollout #40](https://github.com/OpenCoven/coven-threads/pull/40), and
[rollout issue #39](https://github.com/OpenCoven/coven-threads/issues/39).
**Beads:** `threads-6qw` records initial governance; `threads-1e9` records the
authorized solo-maintainer revision; `threads-chk` records the landed ledger;
`threads-t6t` records the separately authorized privacy required-check activation. GitHub
records remain usable without access to a local Beads database.

#46 merged at `91ff511609cfb717bf71c3cafe07c0cbb2a2a317`, recording active
protection and its authorized solo-maintainer revision. #40 merged at
`7168dae10f6b59bad8bc653b95f51911334ded74`, adding the separate privacy job and
approved historical source correction. Their final heads passed required
checks before normal merges. No independent GitHub review is claimed.

#47 merged at `3ff49c02abe5693a58529265fca5bf253f4f4844`, reconciling their
overlapping `SECURITY.md` and `docs/phases.md` wording with the broader audit.
Keep the original dated evidence distinct from later
landing and policy changes, and preserve the remaining acceptance gates.

Exit evidence is recorded human authorization, resolved conversations, and
normal landing after the required checks at the accepted head.
`Privacy policy guard` was subsequently added to the ruleset on 2026-09-12
under [separate human authorization](https://github.com/OpenCoven/coven-threads/issues/39#issuecomment-5643712162).
The original four required checks and all other rule settings were preserved.
This completes the bounded rollout, not the daemon acceptance work below.
A required scanner job is not tamper-proof enforcement against
changes to the checker or calling workflow.

### 2. Close the implementation and acceptance gaps in Coven

**Owners:** daemon integration lane and Cody for implementation; Echo for
predicate/audit/replay review; Sage for evidence mapping.
**Tracking:** OpenCoven/coven#1022, OpenCoven/coven#931, OpenCoven/coven#976,
OpenCoven/coven#977; `threads-8pz`.

The original September 12 target was
`ea3f455aa66bb92880d95f57eeb7ca8d845b7fe6`, against main
`aa527d2dbcd1edcda470483c1a91a519f24e8406`. Its shared branch remains
untouched. The separate draft OpenCoven/coven#1022 initially composed both parents
at `65766b89b4c945d325853cefdd05f364ee87cbac`; that head's exact-current-Threads
Linux observation and native macOS boundary targets pass, with retained
failure history in the execution checkpoint. Those receipts remain historical;
review the current paired candidate below rather than landing an obsolete
head. Preserve the clock
(OpenCoven/coven#968), protected-intake (OpenCoven/coven#933), and typed-terminal
(OpenCoven/coven#932) checkpoints already landed on main. Reconcile overlapping
component commits; do not merge every historical branch or restore #27's
rejected null-close approach.

| Obligation | Existing work owner | Required closure evidence |
| --- | --- | --- |
| Identity binding from classification through replay | OpenCoven/coven#885; `threads-okc` | Bind supported predicate/candidate evidence at intake; prove changed-but-still-valid evidence cannot reuse stale authority, including restart |
| Every approval ceremony reaches supported intake | Existing integration OpenCoven/coven#1022; bounded auto OpenCoven/coven#1029 and #57; separate publication owner OpenCoven/coven#972 | Accept exact-head native/current-core proof of both auto variants and the human paths; preserve protected floors and reject injected positive envelopes. #13 is the downstream acceptance gate, not the implementation owner |
| Complete submission/opening/typed-close chain | OpenCoven/coven#886; `threads-980`; bounded OpenCoven/coven#932 fix is on main | Matching and ordered submission/opening/close evidence across crashes, complete terminal/recovery matrix, duplicate prevention, and explicit resolution of ambiguous applying state without fabricated receipts |
| Protected proposals never become writes | OpenCoven/coven#887; `threads-dgg`; bounded OpenCoven/coven#933 fix is on main | Full known-route, retry, cross-familiar, stale-claim, and recovery refusal matrix with persisted effects |
| Retired-Ward schedulability | OpenCoven/coven#888; `threads-zav` | Supported migration/intake, observable minimum visibility, exact time boundaries, restart, and unsupported-input refusal |
| Final authority snapshot matches committed bytes | OpenCoven/coven#977; `threads-8pz.13` | Named authority-change interleavings, intended-file atomicity, audit agreement, and recovery/rollback evidence |
| Native startup reliability and artifact provenance | Named repair OpenCoven/coven#1027; final fixture consolidation OpenCoven/coven#1029; broader OpenCoven/coven#884 and OpenCoven/coven#1000 | Preserve the original failure and exact native repair receipt; verify later heads separately, including launch/exit origin and dependency state. OpenCoven/coven#1001 is closed for measurement, not reliability |

The landed terminal checkpoint's legacy-window recovery target is not
supported scheduled publication. OpenCoven/coven#969 has since merged; the
supported scheduled producer now commits identity evidence, and
`threads-vpp.2` supplies the missing valid-to-valid replay proof.
OpenCoven/coven#972 subsequently landed at `8bab50f6`, after the publication
foundation OpenCoven/coven#1022 at `2de5aaa8`; broader acceptance still belongs
to OpenCoven/coven#888. OpenCoven/coven#1019 supplied synthetic budget interpretation, not the
later native repair. Keep those earlier receipts dated rather than calling
them current blockers.

The next integration step remains the paired #57 / OpenCoven/coven#1029
contribution. Core `2c7a305e`, daemon `d01136f7`, and owner base `8d48bf79`
identify its earlier checkpoint, not the later composed source. Use the paired
pull requests for current revisions and fresh native/current-core acceptance,
including the registry's corrected mixed-batch refusal. Land through the existing
owner, rather than reopening parallel fixture implementations. Preserve the
exact pinned core commit, or update the pin and repeat the affected proof if
history is rewritten. The new region is strictly bounded output formatting,
not a reason to downgrade protected memory or migration policy.
The earlier `84b8dcc7` authority target passed natively, but its dedicated CLI
startup gate failed. The OpenCoven/coven#1030 deadline repair now passes all six
original native CLI cases. The subsequent OpenCoven/coven#1031 correction
removes a reproduced redundant store checkpoint/reopen without widening the
fixture or production deadlines. Final native run `34703393798` passes the
whole workspace and authority target. Preserve both failed runs, the
same-runtime documentation-only comparison, and the corrected-source proof;
none establishes the longer-term reliability target by itself.

The all-ceremony proof above is broader than OpenCoven/coven#888's requirement
for a minimal valid retired-Ward case. Keep the corpus remediation in
`threads-zav`; do not silently expand its acceptance criteria or assign
implementation work to the human gate. The shared daemon lane owns the
broader route coverage needed for #13.

For each of the four root blockers, retain a pre-fix red, the production fix,
an exact-head green, a lower-level regression, and migration/rollback notes.
Test existence, a serialized diagnostic retry, or an engineering recommendation
does not close the blocker. If supporting every ceremony requires changing a
canonical contract, obtain that decision rather than silently narrowing #13.

### 3. Establish current-checkout compatibility

**Owners:** Sage and the release/governance maintainer.
**Tracking:** #31 and [`e2e/compatibility.toml`](../e2e/compatibility.toml).

After the daemon target and fixes land through review, propose a full-SHA pin
to that accepted downstream revision. Keep the current pin unchanged until
then; `status = "harness-required"` is honest today.

The required topology is:

```text
current Threads checkout -> reviewed Coven SHA -> supported daemon request
  -> ephemeral files / SQLite / pending state -> scheduler / replay / restart
```

Prove the Cargo override is active before running J1-J8, and fail the job when
it is not. Retain exact invocation, dependency state, sanitized artifacts, and
run/attempt identity. A committed historical Threads dependency cannot stand
in for the pull request's checkout.

Follow the [E2E rollout contract](testing/e2e-contract.md#10-ci-rollout):
observe the non-required lane, prove intentional red cases and stable
diagnostics, then require pinned Linux daemon acceptance. Implement the
non-blocking scheduled `coven/main` canary separately. The TOML declaration
alone does not schedule it.

The `daemon-observation.yml` workflow now supplies that advisory schedule and
manual full-SHA candidate runs. It records exact revisions and overlay
resolution, rejects any dependency edge not pointing to the current Threads
checkout, and fails visibly when the selected daemon lacks `threads_e2e`.
It landed through #49, #50, and #51 and is active on the default branch.
The execution checkpoint records the first bounded successful observation and
the retained setup failures. Neither activation nor that exact-head result
authorizes a new required check, replacement stable pin, or Phase-5 freeze.

### 4. Complete resilience and human acceptance

**Owners:** daemon lane for native processes; Charm for Cave; maintainer for
measurement and enforcement.
**Tracking:** #31 and OpenCoven/coven-cave#5256.

Run the scheduled native OS matrix and real-daemon Cave project. Browser
acceptance must cover stale/disconnected state, reconnect, and daemon-owned
approval outcomes without optimistic client authority. A disappearing pending
item is not proof of a committed write.

Use the existing targets: 8/8 critical journeys, 4/4 remediation dossiers,
99.5% first-attempt success over 30 days, and the documented latency budgets.
These are acceptance targets, not measured accomplishments. Retry-success
must not erase a failed first attempt. Introduce a coverage floor only from
observed data.

### 5. Request the reserved decisions and reconcile release metadata

**Owners:** Nova's independently attributable coherence gate, then Val's
freeze decision; maintainer for release metadata.
**Tracking:** #13; `threads-uqx.9`, `threads-uqx.10`.

Present the eight-item coherence mapping, accepted commits, remaining
exceptions, and rollback evidence. Nova's decision must identify its accepted
scope; Val decides after that gate. An agent persona, author-side comment,
commit trailer, or green check cannot supply either decision.

Before publishing a Threads release, reconcile the license texts and package
metadata through an explicit maintainer decision, and distinguish the
`0.2.0` unreleased changelog from downstream release ancestry. Do not rewrite
frozen decisions or promise Cargo `0.1.x` compatibility for exhaustive public
changes.

## Keep adjacent work separate

Draft #25 is language-only promotion-seam work. `threads-xpo` owns deliberate
channel reachability, `threads-55s` owns admission-channel audit follow-through,
and `threads-lm4` owns runtime conformance. `threads-5mn` retains the unresolved
upstream mirror reference; do not invent an ID or a promotion command.

Automation-profile consumers need their own owner-specific adoption evidence.
The trusted runtime adapter remains Coven-owned. Neither this work nor
`threads-5rr` authorizes a new identity model, audit store, or audit event.
Independent planning can proceed, but it cannot bypass an unresolved phase
freeze or expand protected-write authority.

## Documentation and rollback discipline

Keep source precedence unchanged: RFC-0001, decision records, public contracts
and vectors, then explanatory docs. Update current status at the top of the
ledger; retain historical results with dates and exact revisions instead of
relabeling them as current. Link completed plans to this strategy and leave
them historical.

Document each boundary-changing PR's objective, non-goals, canonical sources,
authority/privacy/compatibility impact, exact commands/results, downstream
status, and rollback. Preserve Ward backups and ambiguous pending/applying
evidence. A downgrade must not restore unvalidated writes, discard unresolved
claims, or weaken typed terminal evidence.
