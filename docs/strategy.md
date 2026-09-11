# Delivery strategy

Finish the real-daemon authority boundary before declaring Phase 5 complete
or expanding its authority claims.

**Planning baseline: 2026-09-11.** This is an explanatory delivery strategy,
not a new normative contract, task tracker, or approval. [GitHub #31](https://github.com/OpenCoven/coven-threads/issues/31)
and its linked issues own cross-repository work; Beads owns local task status.
The [delivery ledger](phases.md) records implementation evidence, and the
[readiness review](reviews/2026-09-11-landscape-and-readiness.md) records the
findings behind this sequence.

## Starting position

| Surface | Present in the repository | Not established by that fact |
| --- | --- | --- |
| Rust core | Typed validation, approvals, identity predicates, audit schema/migrations, `.weave` portability, and conformance suites | Authentication, filesystem effects, or every daemon route |
| Automation Authority Profile v1 | Schemas, reference validator, signed evidence contracts, and exact vectors | Production scheduler, credential issuance, runtime-adapter adoption, or Rust API integration |
| Repository quality | Pinned toolchain/actions, Cargo baseline, Nextest, informational coverage, and secret scanning | Required daemon compatibility, privacy enforcement on main, or a coverage ratchet |
| Governance | Active main-branch ruleset with independent review and four required checks | A required pinned-daemon check or either Phase-5 human decision |
| Downstream integration | Earlier release-source integration and substantial draft process-boundary evidence | Reviewed landing of the four remediations, full acceptance, or installed-binary conformance |

The core's three verdicts and the automation profile's four outcomes answer
different questions. Do not add an automation approval outcome to the Rust
gate or infer daemon adoption merely because a reference vector passes.

## Sequence and exit evidence

### 1. Finish the bounded documentation and privacy landing

**Owner:** authenticated maintainer and an eligible independent reviewer.
**Tracking:** #46, #40, #39; `threads-chk`, `threads-t6t`.

#46 documents server-side protection that is already active. The engineering
review found no blocker in that delta; follow-up `d51d40a` aligns the page date.
#40's source-reference correction is published, and follow-up `4fce2d9`
corrects its stale `SECURITY.md` rollout text and clarifies checker trust.
Obtain an eligible independent review of each new exact head.

Both PRs touch `docs/phases.md`. Reconcile their final combined wording rather
than treating either candidate's older ledger as authoritative. The audit
branch carries broader explanatory changes, not another copy of their
workflow or source-reference amendments. Coordinate overlapping documentation
at landing without changing the remaining acceptance gates.

Exit evidence is reviewed landing and the required checks at the accepted
head. Adding `Privacy policy guard` to the ruleset is a separate authorized
administrative change after rollout. A separate scanner job is not tamper-proof
enforcement against changes to the checker or calling workflow.

### 2. Close the implementation and acceptance gaps in Coven

**Owners:** daemon integration lane and Cody for implementation; Echo for
predicate/audit/replay review; Sage for evidence mapping.
**Tracking:** OpenCoven/coven#931, OpenCoven/coven#976,
OpenCoven/coven#977; `threads-8pz`.

Resolve the shared draft's merge conflicts before collecting final evidence.
Reconcile overlapping component commits; do not merge every historical branch
or restore #27's rejected null-close approach.

| Obligation | Existing work owner | Required closure evidence |
| --- | --- | --- |
| Identity binding from classification through replay | OpenCoven/coven#885; `threads-okc` | Bind supported predicate/candidate evidence at intake; prove changed-but-still-valid evidence cannot reuse stale authority, including restart |
| Every approval ceremony reaches supported intake | #13 and OpenCoven/coven#888; `threads-zav` | Prove both auto variants and the human paths through a supported production route; do not lower protected region floors or insert test-only envelopes |
| Exactly one typed close per opened window | OpenCoven/coven#886; `threads-980` | Complete terminal/recovery matrix, duplicate prevention, and explicit resolution of ambiguous applying state without fabricated receipts |
| Protected proposals never become writes | OpenCoven/coven#887; `threads-dgg` | Full known-route, retry, cross-familiar, stale-claim, and recovery refusal matrix with persisted effects |
| Retired-Ward schedulability | OpenCoven/coven#888; `threads-zav` | Supported migration/intake, observable minimum visibility, exact time boundaries, restart, and unsupported-input refusal |
| Final authority snapshot matches committed bytes | OpenCoven/coven#977; `threads-8pz.13` | Named authority-change interleavings, intended-file atomicity, audit agreement, and recovery/rollback evidence |
| Native startup reliability and artifact provenance | OpenCoven/coven#1001, OpenCoven/coven#1000 | Explain retained startup failures and dependency state; record actual command, exact merge, run/attempt, and override mode |

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
