# Repository landscape and readiness review

The repository has substantial contract implementation, but Phase 5 is not
ready to freeze. Baseline branch protection is active; reviewed daemon
integration and required current-checkout acceptance remain missing.

**Date:** 2026-09-11. **Tracking:** #31, #13, `threads-9nf`; publication
follow-through `threads-ufe`.
This is Nova-lane engineering evidence, not the independently attributable
Nova coherence decision, Val's freeze, or permission to bypass review.

## Decisions and publication at the original review

| Review target | Engineering recommendation | Published evidence |
| --- | --- | --- |
| #46, governance documentation | No blocking finding in the delta; eligible independent review still required | [Exact-head review](https://github.com/OpenCoven/coven-threads/pull/46#pullrequestreview-5182694841) |
| #40, privacy rollout | Stale candidate text corrected in follow-up `4fce2d9`; eligible independent review and required-check activation remain separate | [Original exact-head finding and replacement text](https://github.com/OpenCoven/coven-threads/pull/40#pullrequestreview-5182694836) |
| #13, Phase-5 coherence | Keep blocked while the source and acceptance gaps below remain open | [Published engineering blocker list](https://github.com/OpenCoven/coven-threads/issues/13#issuecomment-5639655226), not a checked human checklist |
| #31, boundary strategy | Finish the reviewed daemon boundary before broadening acceptance claims | [Published strategy update](https://github.com/OpenCoven/coven-threads/issues/31#issuecomment-5639663138) and [delivery strategy](../strategy.md) |

Both PR reviews are `COMMENTED`, not `APPROVED`. They were published through
the authenticated PR-author account at the user's request. That account cannot
supply the independent approval required by the original policy. Neither review
edited the PR branch or unlocked a protected merge.

**Publication follow-through:** at the user's explicit request, subsequent
documentation-only commits corrected #40 at
`4fce2d9525cc966febb1a8db82587664a5a11c9a` and aligned #46's date at
`d51d40a72a2bda92827fdfe00cdab02408e95e1f`. The original review citations below
remain bound to their original heads. No independent approval is inferred
for either follow-up, and this audit's broader documentation is proposed on
`docs/repository-landscape-strategy`.

Both follow-ups passed exact-head hosted CI: [#40 run 34647137078](https://github.com/OpenCoven/coven-threads/actions/runs/34647137078)
and [#46 run 34647134180](https://github.com/OpenCoven/coven-threads/actions/runs/34647134180).
These are repository checks, not new daemon acceptance or independent reviews.

### Authorized landing follow-through, 2026-09-12 UTC

The maintainer approved the documentation work, confirmed the solo-maintainer
setup, and explicitly authorized a [review-policy change](https://github.com/OpenCoven/coven-threads/issues/31#issuecomment-5642501227).
Ruleset `22910327` now requires zero approving reviews and disables latest-push
and extra unattributed-change approvals. Required PRs, resolved conversations,
stale-review dismissal, all four strict required checks, branch safeguards, and
the absence of bypass actors are unchanged. This is a real policy revision,
not an independent approval supplied by an agent.

#46 merged at `91ff511609cfb717bf71c3cafe07c0cbb2a2a317`, following
[CI at `caf0683`](https://github.com/OpenCoven/coven-threads/actions/runs/34664611356).
#40 merged at `7168dae10f6b59bad8bc653b95f51911334ded74`, following
[CI at `2d17d0d`](https://github.com/OpenCoven/coven-threads/actions/runs/34664678160).
The checker-trust conversation was resolved by narrowing the enforcement claim,
not making the checker immutable. Privacy required-check activation was still
separate at this checkpoint. #47 reconciled this audit with both landings.

This update supersedes the original pending-landing and repository-review
requirements below, not the source findings, daemon acceptance limits, or
Phase-5 coherence/freeze prerequisites. The [contributor policy](../../CONTRIBUTING.md#solo-maintainer-merge-policy)
and [ledger](../phases.md#repository-governance-baseline-active-boundary-enforcement-outstanding)
record the current process and rollback.

### Privacy required-check follow-through, 2026-09-12 UTC

At 05:16 UTC, the maintainer
[explicitly approved requiring the privacy check](https://github.com/OpenCoven/coven-threads/issues/39#issuecomment-5643712162).
The existing `Privacy policy guard` context is now the fifth required check in
ruleset `22910327`, bound to GitHub Actions app `15368`. The four earlier
contexts and every other rule field were preserved. The exact check had
already passed on main `3ff49c02abe5693a58529265fca5bf253f4f4844` in
[run 34664925352](https://github.com/OpenCoven/coven-threads/actions/runs/34664925352).
API read-back confirmed the active ruleset and effective branch requirement.
This supersedes the historical pending-activation statements below, not the
checker-trust limits or any daemon/coherence/freeze obligation.

## Identified baseline

| Surface | Revision and observed state |
| --- | --- |
| Threads audit baseline/main | `5be96b0ecb8e0b9a98d2cfd04fc12ce5c0fcd4fd`; the audit branch proposes documentation changes, not a merged-main claim |
| #46 at original review | `6265ca718dec658b03d614ecca4c048b1c15c205`; follow-up head recorded above |
| #40 at original review | `2a12c1ad6f813b636f49f1c17bbd3ff7cb65aaee`; follow-up head recorded above |
| OpenCoven/coven#931 | `8576f41e6d622f63a3576b85bd2d3142776e59ca`, open draft; latest query reports merge conflicts and no submitted reviews |
| Coven main | `066727f1a34377f4dfff0f5a7f80898a3a1c1ea5`; CLI manifest requests Threads `c102844`; no `threads_e2e` target |
| Stable compatibility pin | `39feb6de98816d10b490091e918f62035e6ce0df`; no `threads_e2e` target; `harness-required` remains accurate |
| Latest recorded hosted daemon merge | `0d8ccfca1725475b5e45f2ec47d8c4c2c45bc89d`, not current Coven main |

The audit covered the root documentation, `docs/*.md`, E2E contract,
diagram documentation, historical ApplyAudit plans, repository manifest, and
relevant decision records. It also inspected current issue/PR/governance state,
the exact downstream scheduled producer and loader, the core replay hash and
region floors, and hosted artifact provenance. The earlier same-day detailed
coherence mapping is carried forward only at its unchanged immutable daemon
head.

This is a bounded documentation and engineering-readiness review, not an
exhaustive code/security audit. Frozen specifications, the PDF, rendered
diagrams, and slides were not rewritten or recertified. Existing historical
wording is not a current acceptance claim merely because it remains committed.

## Documentation audit

**Summary:** eight actionable findings, four high and four medium. The main
problem is mixing design intent, historical implementation, and present
acceptance. Style review is scoped to revised prose, not a wholesale rewrite
of frozen or archival text.

Line references below use the Threads baseline above, except where the #40
candidate is explicitly identified. Quotes preserve the original wording.

### High: authority or current-state clarity

1. `docs/architecture.md:23` [Terminology]: public-library access is confused
   with daemon authority. **Offending:** "reachable *only* by the privileged
   daemon process". **Rewrite:** "Any program can call the public library,
   but a client-side result grants no authority." Applied to the architecture
   explanation and the FAQ's corresponding isolation claim.

2. `docs/phases.md:312` [Structure]: an obsolete governance observation reads
   as current. **Offending:** "snapshot reports `main` as unprotected and no
   repository rulesets." **Rewrite:** "The September 10 snapshot reported
   `main` as unprotected. Active ruleset `22910327` enabled baseline protection
   on September 11; required pinned-daemon acceptance remains absent."
   Applied to the ledger, with the four required contexts and review rules.

3. `docs/glossary.md:18` [Terminology]: the replay mechanism is described more
   broadly than the inspected implementation proves. **Offending:** "the
   daemon re-materializes and re-derives it live at the delayed-apply deadline".
   **Rewrite:** "`evidence_replay_hash` commits materialized diff and region
   evidence. Matching it alone does not prove identity binding or final live
   authority." Applied to the glossary, with the FAQ distinguishing load-time
   reconstruction, live checks, and the open identity commitment.

4. `SECURITY.md:40-42` at #40's reviewed head [Structure]: rollout text
   contradicts the same candidate's source correction and hosted result.
   **Offending:** "Draft #40 adds a separate `Privacy policy guard` job that
   currently fails on that documented conflict." **Rewrite:** "This candidate
   adds the separate privacy job and approved source-reference correction;
   both guards pass at the recorded head. Reviewed landing and required-check
   activation remain separate." Published as a required candidate correction,
   then applied with explicit user authorization in follow-up `4fce2d9`,
   including the surrounding stale paragraphs and command comment. The audit
   branch separately distinguishes candidate evidence from the not-yet-landed
   job and source correction.

### Medium: reader guidance and historical scope

5. `docs/concepts.md:24` [Code]: a design sketch resembles an available
   executable. **Offending:** "The design requires that `coven-threads inspect
   <thread-id>` return the thread's current tension state".
   **Rewrite:** "The frozen design sketches this command, but this repository
   ships a library, not that executable." Applied with a pointer to supported
   daemon/Cave inspection and the tension contract.

6. `README.md:99` [Structure]: a Phase-0 anti-goal is presented as current
   repository scope after Phase 3. **Offending:** "**Not a runtime-portability
   format.** That's Phase 3's job. Phase 0 is enforcement design, not export."
   **Rewrite:** "Threads owns the `.weave` portability contract and lossy
   one-way `.af` export. Coven owns runtime adoption and effects." Applied
   without changing the frozen design.

7. `docs/diagrams/README.md:3` [Terminology]: the introduction suggests the
   diagrams are authority. **Offending:** "Source-authoritative diagrams for
   `coven-threads`." **Rewrite:** "These diagrams explain `coven-threads`;
   they are not authority or current acceptance evidence." Applied with
   explicit historical scope for the original staging diagram and windowed
   lifecycle illustration; root and architecture captions now carry the warning.

8. `docs/channels-and-strands.md:93` [Structure]: implemented portability is
   still future tense. **Offending:** "The portability format that will wrap
   `Serialization` in practice". **Rewrite:** "The implemented `.weave`
   portability contract." Applied; a separate callout keeps the still-planned
   deliberate-channel promotion command from appearing shipped.

**Patterns:** current claims need dated evidence and an owner, not stronger
adjectives. The new strategy separates work order from the historical ledger.
Completed ApplyAudit plans now link to that strategy and explicitly retire
their old commit instruction. License reconciliation remains a maintainer
decision, not a wording fix that can choose a license on its own.

## Implementation inventory

| Surface | Source-backed inventory | Limit |
| --- | --- | --- |
| Rust package | One workspace crate, `coven-threads-core` `0.2.0`, MIT metadata, 15 public modules; `Cargo.toml:4-10`, `crates/coven-threads-core/src/lib.rs:56-116` | `CHANGELOG.md:5` still labels `0.2.0` unreleased; the frozen Apache-2.0 plan does not change the current MIT files |
| Rust integration targets | `rfc0001_s5_conformance`, `c7_roundtrip`, and `phase5_retired_ward_corpus` under `crates/coven-threads-core/tests/` | In-process contracts, not a daemon process boundary |
| Automation profile | Profile `1.0.0`, 18 manifest categories, 130 vectors, and 11 schemas under `profiles/automation-authority/v1/` | Independent version line and reference bundle, not proof of production adoption |
| CI | Five jobs in `.github/workflows/ci.yml:15-112`: secret scanning, quality, Cargo baseline, Nextest, and informational coverage | Four are required by the active ruleset; no daemon checkout, required E2E, or scheduled canary |
| Bootstrap/checks | `scripts/agent-bootstrap.sh` and `scripts/agent-check.sh` supply the documented pinned bootstrap and local gates, including Node profile checks | They do not satisfy the independent human decisions |

The contributor and profile guides now make these limits explicit. No
executable implementation was changed or runtime result inferred from a file
count.

## Phase-5 engineering review

### Two source-confirmed acceptance gaps

**Supported auto-path reachability:** the [scheduled producer][producer]
requires region evidence and combines path and region floors. The
[built-in regions][regions] assign floors 0 or 1, while the
[scheduled loader][scheduler] rejects floor 0 and auto approval at floor 1.
Thus this producer cannot supply either `AutoRegression` variant with those
regions. Identify and prove a supported route; do not lower a protected floor
or use a test-only constructor to satisfy the checklist.

**Classification-time identity binding:** the producer sets its replay hash
using the [diff/region hash function][hash]. This does not separately commit
the identity predicate/configuration or unedited candidate identity sources
required by OpenCoven/coven#885. Later live predicate evaluation and
[final applying/recovery binding][binding] are useful but do not establish the
intake-to-deadline commitment. Prove changed-but-still-valid identity evidence
cannot reuse stale authority. This is a source/acceptance finding, not a newly
executed unauthorized-write counterexample.

### Eight-item coherence mapping

| #13 obligation | Evidence and remaining acceptance |
| --- | --- |
| RFC round-trip | [Recorded defaults][decisions] map substantially to [approval types][approval], predicates, and audit contracts. Source gaps above prevent a complete runtime claim. |
| Every approval path revalidates | Familiar-review and human-review process evidence exists; supported auto and full rationale-required process acceptance remain open. [Apply ceremony][ceremony] is not proof of every producer route. |
| Descriptor is not authority | Inspected [validator][validator] and materialization chain use predicates and authoritative bytes. No exhaustive Cave/client conformance is claimed. |
| Audit completeness: `proposal_submitted -> window_opened -> close(reason)` | The [retired-corpus journey][corpus] binds submission detail to the pending classification; its [terminal helper][terminal-helper] asserts one opening and one typed close for the same proposal. These cases do not establish gap-free ordering and correspondence across every route or crash boundary. Full-chain acceptance, failure/recovery coverage, and explicit ambiguous-applying resolution remain open. |
| Replay hash enforced | [Scheduled load][scheduler] reconstructs diff/region evidence; live apply checks follow. The complete classification-time identity commitment is still unresolved. |
| Minimum visibility | [Real retired-corpus journey][corpus] observes pending/restart and time advancement. Exact deadline, publication visibility, and browser reachability need distinct evidence. |
| Label/variant load contract | [Typed envelope][wire] and negative core cases exist. A real-daemon malformed-load case must substantiate the boundary claim. |
| Advisory separation | [Model-advisory types][advisory] remain separate; inspected daemon probes are deterministic. An unimplemented model-probe integration is not certified. |

For **audit completeness**, #13's `window_opened` shorthand denotes the
canonical `proposal_window_opened` event. Each link has its own evidence limit:

| Link | Source-backed observation | Remaining proof |
| --- | --- | --- |
| Submission to opening | The corpus journey reads `proposal_submitted` by proposal ID and compares its classification with the durable pending envelope. The [opening writer][opening-writer] derives interval and replay fields from the scheduled proposal. | The inspected journey does not independently assert event ordering, complete submission/opening correspondence, or absence of a gap at every publication/crash boundary. |
| Opening to typed close | The terminal helper requires one opening and one terminal record under the same proposal ID, then checks the terminal family, reason, and replay result. [SQL guards][audit] enforce typed terminal constraints. | Cover every failure/recovery route, preserve linkage after restart, and resolve ambiguous applying state without inventing a receipt. |

This mapping retains the full obligation rather than reducing it to terminal
multiplicity. These are source and previously hosted observations at the
recorded daemon revision, not new execution or a checked human decision.

All four remediation issues remain open: OpenCoven/coven#885 (`threads-okc`),
OpenCoven/coven#886 (`threads-980`), OpenCoven/coven#887 (`threads-dgg`), and
OpenCoven/coven#888 (`threads-zav`). The [E2E contract](../testing/e2e-contract.md)
requires attributable pre-fix red, production fix, exact-head green,
lower-level regression, and rollback evidence for each. Branch conflicts,
incomplete route/recovery matrices, signed principal/trusted-runtime proof,
and OpenCoven/coven-cave#5256's browser acceptance are not closed by this review.

## Hosted evidence and governance

[Coven run 34489425644][daemon-run] passed at the recorded draft head.
Its native Windows target has 15 daemon journeys plus five artifact
regressions. Artifact `10157780426` was re-read in memory: all 28 manifests
pass, with 23 distinct scenario names, the recorded synthetic merge above,
older Threads `c3bd46bcadb6396db8436c47411a4d0eac17192b`,
`coven_dirty=false`, `threads_dirty=true`, and override false. Repeated scenario
manifests are not independent acceptance journeys.

The dirty flag does not prove tracked source modification; explain it rather
than labeling the whole dependency tree clean. The artifact records a generic
command without `--features threads-test-clock`; the earlier job-log inspection
records the feature-enabled invocation. Preserve the actual command in future
evidence. Earlier startup failures in run `34482201141` remain unexplained.
No fresh daemon run or 30-day reliability measurement was performed here.

At the original review, [ruleset `22910327`][ruleset] enforced one approval, latest-push approval,
stale-approval dismissal, resolved review threads, up-to-date branches, four
GitHub Actions-bound checks, and deletion/non-fast-forward protection with no
bypass actors. The separate follow-through decisions above changed three
approval settings and later added the fifth privacy check. Daemon E2E remains
absent from the required contexts. Neither the current
workflow nor a TOML schedule declaration supplies the missing nightly canary.

The #40 checker-trust comment is a documented boundary limitation, not a newly
introduced functional blocker for separate-job rollout: PRs can modify both
checker and workflow. An immutable enforcement policy needs an explicit
governance design, not a privileged workflow shortcut.

## Impact, verification, and rollback

The audit changes are explanatory Markdown only. They do not modify Rust, SQL,
migrations, fixtures, dependency pins, workflows, frozen specifications,
identity definitions, or human-gate status. Historical source correction in
#40 was reviewed and subsequently merged separately, not introduced by this
audit. The later server-side policy change is recorded in the follow-through
above; it is not a claim that the original audit changed governance.

Read-only evidence commands included:

```bash
gh pr diff 46
gh pr view 40 --json headRefOid,files,reviews,statusCheckRollup
gh pr view 931 -R OpenCoven/coven --json headRefOid,isDraft,mergeStateStatus,reviews
gh api repos/OpenCoven/coven-threads/rulesets/22910327
gh api repos/OpenCoven/coven-threads/rules/branches/main
gh run view 34489425644 -R OpenCoven/coven --json headSha,event,status,conclusion
git diff --check
```

The API also supplied exact-revision source, target-directory listings, issue
acceptance text, and the hosted ZIP inspected in memory. PR reviews were
published as comments bound to their exact commit IDs, not approvals.
Whitespace and local-link checks cover this documentation change; no fresh
build, full Rust suite, or daemon result is claimed.

Rollback is a reviewed documentation correction. It cannot undo server-side
protection or establish runtime safety. Preserve recorded contradictory
evidence, Ward backups, and unresolved applying claims; do not downgrade into
the original authority defects. Remaining work follows [the strategy](../strategy.md).

[producer]: https://github.com/OpenCoven/coven/blob/8576f41e6d622f63a3576b85bd2d3142776e59ca/crates/coven-cli/src/threads_gate.rs#L1318-L1425
[scheduler]: https://github.com/OpenCoven/coven/blob/8576f41e6d622f63a3576b85bd2d3142776e59ca/crates/coven-cli/src/proposal_scheduler.rs#L51-L220
[regions]: https://github.com/OpenCoven/coven-threads/blob/5be96b0ecb8e0b9a98d2cfd04fc12ce5c0fcd4fd/crates/coven-threads-core/src/surface_regions.rs#L306-L545
[hash]: https://github.com/OpenCoven/coven-threads/blob/5be96b0ecb8e0b9a98d2cfd04fc12ce5c0fcd4fd/crates/coven-threads-core/src/surface_regions.rs#L185-L249
[binding]: https://github.com/OpenCoven/coven/blob/8576f41e6d622f63a3576b85bd2d3142776e59ca/crates/coven-cli/src/api.rs#L12667-L12844
[decisions]: https://github.com/OpenCoven/coven-threads/blob/5be96b0ecb8e0b9a98d2cfd04fc12ce5c0fcd4fd/specs/PHASE-5-APPROVAL-SEMANTICS.md#L311-L435
[approval]: https://github.com/OpenCoven/coven-threads/blob/5be96b0ecb8e0b9a98d2cfd04fc12ce5c0fcd4fd/crates/coven-threads-core/src/approval.rs#L82-L431
[validator]: https://github.com/OpenCoven/coven-threads/blob/5be96b0ecb8e0b9a98d2cfd04fc12ce5c0fcd4fd/crates/coven-threads-core/src/validate.rs#L130-L195
[audit]: https://github.com/OpenCoven/coven-threads/blob/5be96b0ecb8e0b9a98d2cfd04fc12ce5c0fcd4fd/crates/coven-threads-core/src/audit.rs#L1307-L1449
[ceremony]: https://github.com/OpenCoven/coven/blob/8576f41e6d622f63a3576b85bd2d3142776e59ca/crates/coven-cli/src/api.rs#L7468-L7600
[corpus]: https://github.com/OpenCoven/coven/blob/8576f41e6d622f63a3576b85bd2d3142776e59ca/crates/coven-cli/tests/threads_e2e.rs#L302-L402
[terminal-helper]: https://github.com/OpenCoven/coven/blob/8576f41e6d622f63a3576b85bd2d3142776e59ca/crates/coven-cli/tests/threads_e2e.rs#L1473-L1511
[opening-writer]: https://github.com/OpenCoven/coven/blob/8576f41e6d622f63a3576b85bd2d3142776e59ca/crates/coven-cli/src/api.rs#L12126-L12222
[wire]: https://github.com/OpenCoven/coven-threads/blob/5be96b0ecb8e0b9a98d2cfd04fc12ce5c0fcd4fd/crates/coven-threads-core/src/approval.rs#L571-L677
[advisory]: https://github.com/OpenCoven/coven-threads/blob/5be96b0ecb8e0b9a98d2cfd04fc12ce5c0fcd4fd/crates/coven-threads-core/src/identity_invariants.rs#L770-L915
[daemon-run]: https://github.com/OpenCoven/coven/actions/runs/34489425644
[ruleset]: https://github.com/OpenCoven/coven-threads/rules/22910327
