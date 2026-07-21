# PHASE-5 PROPOSAL — Approval semantics over the Phase-2 Ward

**Status:** PROVISIONAL DECISION RECORD (Sections 0–5 are the reviewed
proposal; Sections 6–8 record Val's implementation opening and the design
defaults attributed to Nova. Independent Nova attestation and freeze remain
gated.)
**Date:** 2026-07-18
**Decision date:** 2026-07-18
**Bead:** `threads-uqx`
**Authors:** Sage lane (drafted by coordinator session agent)
**Recorded by:** Val Alexander (decision commit `b085fc8`; its Nova co-author
trailer is attribution, not independent authority evidence)

---

## 0. Scope and sources

Sections 0–5 preserve the proposal as reviewed. They do not amend frozen Phase
0 or change the daemon. Sections 6–8 are Val's subsequent decision record that
opened Phase 5 implementation and the defaults attributed to Nova pending
independent attestation; they do not declare Phase 5 frozen.

Sources read and cited:

- `specs/PHASE-0-DESIGN.md`: frozen v0.2; Thread (authority relationship:
  surface -> writer), Weave (enforced pattern of threads), Strand (fiber inside
  a thread), Channel (axis of load), predicate-authoritative / descriptor-
  derived rule, Gate-4 fail-closed posture, and WARD-C1..C7 lineage.
- `../familiar-contract/rfcs/RFC-0001-familiar-contract.md`: §4.2 protected
  invariants, §4.3 editable surface, §5.3 approval tiers, §5.4 gates, §5.5
  probes. RFC amendments for closure precondition and provenance predicate are
  drafted in OpenCoven/familiar-contract PR #3. The approval-tier declaration
  and schema mapping remains a separate upstream gate (`threads-uqx.12`). Both
  remain freeze gates until Nova + Val independently approve and merge them.
- `../coven-grimoire/articles/drafts/2026-06-24-ward-layer-spec-brief.md` §9:
  canonical home of WARD-C1..C7.
- `specs/PHASE-4-CAVE-SURFACES.md`: approval flow §3.7, fail-closed rendering
  rules §4, and non-goals §6.
- `../coven/crates/coven-cli/src/ward.rs`, `threads_gate.rs`, `api.rs`: current
  daemon reality — path tiers, Gate 1-4 comments, protected-surface weave
  construction, and `decide_threads_proposal` as the current tier-0 proposal
  commit point. That protected-target proposal path diverges from RFC-0001
  §5.4 and is not precedent Phase 5 may preserve.
- `~/.coven/workspaces/familiars/nova/ward.toml.v01.bak`: read-only example of
  the retired dialect. This draft paraphrases its invariant shape and does not
  reproduce Val's exact invariant strings. It is discovery evidence, not an
  executable migration fixture; `threads-uqx.13` must produce an authorized,
  sanitized, repository-accessible replacement.

---

## 1. What v0.1 expressed that Phase 2 cannot

Phase 2 reduced Ward configuration to path-tier mapping: tier 0 protected,
tier 1 reviewed, tier 2 logged, tier 3 free. In the daemon, `WardConfig` stores
surface globs and `protected_surface`; `Ward::apply` refuses blocked proposals
as a unit, holds Tier 1 and Gate-1-authorized Tier 0 for Gate 3, and writes only
Tier 2/3. `apply_after_threads_approval` still reruns Gates 1 and 2, and
`api.rs::decide_threads_proposal` revalidates a pending proposal, calls the
threads gate, appends audit rows, advances baselines, removes the pending file,
and is the only current tier-0 approval commit point. RFC-0001 §5.4 instead
requires proposals touching the protected surface to be rejected and reserves
principal-authorized protected updates for a separate, audited path. Phase 5
must remove or isolate that divergence rather than encode it into
`ApprovalPath`.

The retired v0.1 dialect carried three semantic groups Phase 2 cannot express.

### 1.1 Approval tiers as named governance lanes

v0.1 distinguished four proposal lanes: automatic regression-gated promotion
with a human veto window; familiar-coherence review with a shorter veto window;
human pre-approval; and human approval with rationale plus audit. Each lane
bound named semantic blocks to a gate.

Phase 2 can say "logged" or "reviewed." It cannot say "this semantic region is
auto-promoted after regression but vetoable for 48 hours," nor distinguish
human approval from human approval with rationale. RFC-0001 §5.3 still names
these approval tiers and forbids auto-promotion of protected-surface proposals.

### 1.2 Editable harness blocks as semantic regions

v0.1 named editable blocks such as execution and recovery prompt regions, tool
defaults, skill configuration, subagent templates, output formats, heartbeat
behavior, tool grants, skill activations, memory conventions, session
introduction, and protected-surface adjacency. These are not file paths. One
block may cut across files; one file may contain several blocks.

Phase 2 only classifies materialized paths. It cannot answer which semantic
region a diff changed unless that region aligns with a glob. That loses the
RFC-0001 §4.3 editable-surface shape.

### 1.3 Protected invariants as cross-file identity predicates

v0.1 carried textual identity assertions over familiar metadata and purpose.
The real backup shows the shape: compact predicate-like statements about name,
person binding, pronouns, purpose, and Coven membership. This document does not
copy them.

Phase 2 protects files and globs. It does not protect an identity fact that can
be invalidated through an allowed path, prompt block, derived context, or future
portability envelope. RFC-0001 §4.2 says path-only protection is insufficient:
identity-probe inconsistency must reject a proposal regardless of target path.

---

## 2. Carry forward vs deliberately dead

### 2.1 Approval tiers: carry forward, old encoding dead

Carry forward the governance distinction; do not revive the v0.1 table shape.
The useful semantic is four approval paths: deterministic low-risk gate,
familiar-coherence gate, human approval, and human approval with rationale.
That matches RFC-0001 §5.3 and explains why Phase 4 already models
`proposal_approved`, `proposal_rejected`, and `proposal_vetoed` audit events.

The target design retires stringly enforcement coupling among `blocks = [...]`,
`gate = "..."`, and ad hoc veto fields, but the current RFC-0001 declaration
and `ward.schema.json` still define those fields normatively. Phase 5 therefore
must not declare that encoding dead or freeze a replacement until
`threads-uqx.12` amends the upstream RFC and schema with an explicit,
deterministic mapping to typed daemon-owned `ApprovalPath` and `SurfaceRegion`
semantics, or explicitly retires the fields. Unknown, ambiguous, or
uncompilable declarations fail closed. The Cave must not become a policy
engine.

Recommended name: `ApprovalPath`, not `Tier`. "Tier" is already the path-trust
axis in Phase 2. Reusing it would collapse where a write lands with which
ceremony promotes it.

### 2.2 Harness blocks: carry forward as regions, not authority

Carry forward the region abstraction; kill any claim that labels are
authoritative by themselves. A block label is a descriptor unless a
parser/extractor maps bytes to that region deterministically and the daemon can
replay that mapping at apply time. Phase 0 §2.2 binds the rule: predicate
authoritative, descriptor derived.

Recommended name: `SurfaceRegion` above `SurfaceId`. A `SurfaceRegion` is a
typed semantic region with an extractor predicate over materialized content.
Its descriptor may say "output formats" for the Cave, but the daemon gates on
extractor evidence and final materialized diff.

### 2.3 Protected invariants: carry forward as predicates, strings dead

Carry forward strongly. RFC-0001 §4.2 still requires semantic invariants, and
Phase 0 already gives them a home: `PatternPredicate::coherent` is the
authoritative gate; `describe()` yields a derived descriptor. Identity
invariants should become predicate implementations in the weave, not strings
interpreted by clients.

The v0.1 invariant interpretation should stay dead. RFC-0001 still requires a
TOML `[protected].invariants` declaration surface, so Phase 5 must define a
deterministic compiler from supported declarations into `PatternPredicate`
implementations. Parser semantics, normalization, evidence, and failure modes
belong to that compiler; unsupported declarations fail closed. The corrected
RFC PR #3 must land before Phase 5 freezes invariant interfaces.

---

## 3. Mapping sketches inside the frozen metaphor

### 3.1 Approval paths: gate policy over existing channels

Do not model approval paths as new `Channel` variants by default. Channel (axis
of load) says why a thread is stressed: deliberate, forced, serialization, or
mutation. Approval path says which promotion ceremony is required. A mutation
that needs human rationale is still `Channel::Mutation`.

Recommended default:

```rust
pub enum ApprovalPath {
    AutoRegression { veto: Option<VetoWindow> },
    FamiliarCoherence { veto: Option<VetoWindow> },
    HumanApproval,
    HumanApprovalWithRationale,
}

pub struct ProposalClassification {
    pub channel: Channel,
    pub affected_regions: Vec<SurfaceRegionId>,
    pub path_tier_floor: Tier,
    pub approval_path: ApprovalPath,
}
```

Path tier remains a floor only for proposal-eligible targets. RFC-0001 Gates 1,
2, and 4 reject every proposal whose declared or materialized diff touches the
protected surface (Phase-2 Tier 0). A principal-authorized Ward or protected
surface mutation is a separate authority path outside the proposal pipeline;
it is never promoted through `ApprovalPath`. A Tier 2 path touching a high-risk
semantic region may still require human review. Highest ceremony wins for each
proposal as a unit, matching current all-or-nothing Ward behavior.

Only add a new channel if Val/Nova decide approval-policy edits themselves need
a distinct load axis. That should not be the default.

### 3.2 Invariants: PatternPredicate implementations

Identity invariants should compile into `PatternPredicate` implementations.
The predicate takes relevant threads, strands, and verified proposal evidence;
its result is authoritative. Its descriptor is Cave-readable, labeled derived,
and never enforced on.

Concrete referent mapping:

- Thread (authority relationship: surface -> writer): protected identity
  surfaces keep principal-writer threads.
- Strand (fiber inside a thread): invariant evidence may use hashes, manifest
  entries, audit anchors, serialization markers, and only later a new strand if
  evidence forces one.
- Weave (enforced pattern of threads): identity predicates are part of the weave
  pattern, not side-table rows.
- Channel (axis of load): invariants must hold under mutation and serialization
  at minimum; forced compaction connects through WARD-C1..C6.

Recommended default: predicate-first; no new strand kind until implementation
evidence requires it.

### 3.3 Harness blocks: SurfaceRegion above SurfaceId

A `SurfaceRegion` should be daemon-replayable over a materialized diff:

```rust
pub trait SurfaceRegionPredicate {
    fn materialize(&self, proposal: &MaterializedDiff) -> RegionEvidence;
    fn describe(&self) -> SurfaceRegionDescriptor;
}
```

The descriptor is for humans. The predicate output feeds classification and
Gate 4 replay. It cannot depend on Cave state, agent self-report, or stale
metadata.

Thread options:

- one thread per block: inspectable, but risks threads detached from
  source-authoritative surfaces;
- one thread per source surface, with region evidence on the mutation request:
  closer to Phase 2 and safer by default.

Recommended default: keep threads source-bound; attach region evidence to
proposal classification. Promote a region to thread status only when it has a
stable source-authoritative projection.

---

## 4. Veto windows vs synchronous gates

A synchronous gate answers before write: permit, stage, or reject. For
proposal-eligible targets, Phase 2 may stage on fray, wait for the required
approval, revalidate, and apply. Protected targets are not proposal-eligible
under RFC-0001 §5.4; principal-authorized protected updates use a separate
audited authority path. Nothing writes before the applicable authority clears.

A veto window has two possible meanings:

1. delayed apply: gates pass, the proposal becomes pending-visible, and the
   daemon applies only after the window expires with no veto;
2. provisional apply: gates pass, the daemon applies immediately, and a later
   veto triggers rollback.

Recommended default: delayed apply. Provisional apply should be forbidden until
Val/Nova explicitly accept rollback semantics. Gate-4 fail-closed and Phase 4's
"no optimistic UI, no queued decisions" posture both prefer visible pending
state over speculative mutation.

Delayed apply needs audit moments:

- `proposal_submitted`: classification, diff hash, affected paths/regions,
  predicate evidence, deadline;
- `proposal_window_opened`: gate evidence and veto deadline recorded;
- one terminal event after deadline or explicit action:

| Terminal event | Close reason | Meaning |
| --- | --- | --- |
| `proposal_approved` | `applied` | Deadline and minimum visibility elapsed, no veto exists, replay matched, and the write committed. |
| `proposal_vetoed` | `vetoed` | A principal veto closed the window before apply. |
| `proposal_rejected` | `evidence_diverged` | Deadline triggered replay and the evidence hash changed. |
| `proposal_rejected` | `revalidation_failed` | Replay could not produce authoritative evidence. |
| `proposal_rejected` | `superseded` | A newer proposal replaced this pending proposal before apply. |

Deadline expiry is a trigger for revalidation, not a successful or failed
terminal state by itself. There is no `proposal_expired` terminal event.

WARD-C6 matters by analogy: a time-separated, lossy or delayed boundary needs a
ledger. A proposal whose evidence cannot be replayed at deadline fails closed.
Future Cave surfaces should extend `ProposalView` only after daemon evidence
exists; until then unknown or stale veto state renders blocked under Phase 4 §4.

---

## 5. Explicit non-goals

- No general policy engine; README anti-goals and Phase 4 §6 bind.
- No client-side approval authority; Cave forwards, daemon decides.
- No client-side enforcement of v0.1 stringly TOML semantics. RFC-0001's Ward
  TOML declaration and schema remain normative until `threads-uqx.12` defines
  their typed daemon mapping or explicit retirement.
- No Phase-5 freeze and no changes to frozen Phase 0. The later implementation
  opening is recorded separately in Sections 6–8.
- No weakening of Gate 4; every path ends in live daemon re-materialization.
- No `.af` compatibility work; Phase 3 already decided `.weave` canonical plus
  lossy one-way `.af` exporter.
- No broad runtime capability system; tool grants may classify proposals, but
  runtime chamber authority remains separate.

---

## 6. Recorded design defaults (Val, with Nova attribution pending)

Val's decision commit records all eight questions on Sage's recommended
defaults and opens implementation against them. The repository history
attributes Nova as a co-author, but a commit trailer is not independently
attributable authority evidence and cannot satisfy a Nova gate.

Record provenance: commit `b085fc8` (`spec(phase-5): record Val+Nova decisions,
open Phase 5`) is authored by Val Alexander and carries
`Co-authored-by: Nova <nova@opencoven.dev>`. This document treats that trailer
as attribution only. `threads-uqx.9` requires Nova to approve an identified
commit through an independently attributable channel before freeze; agents,
commit authors, and trailers cannot satisfy or simulate that gate.

1. **`ApprovalPath` separate from `Channel`? → YES.**
   Channel remains the load axis (why a thread is stressed). ApprovalPath is
   the promotion ceremony. Conflating them would make Phase 0 load-axis
   semantics ambiguous. `ApprovalPath` proceeds as a distinct type.
   *Both are authoritative in their own dimensions: Channel remains the
   first-class Phase-0 load/enforcement axis, while ApprovalPath selects the
   promotion ceremony. Never derive ApprovalPath from Channel. If this
   separation shifts, revisit all eight decisions.* (Echo, Sage 2026-07-18;
   corrected after independent coherence review 2026-07-19)

2. **Veto windows: delayed-apply or provisional-apply? → DELAYED APPLY ONLY.**
   Matches Gate-4 fail-closed posture and Phase 4 "no optimistic UI, no queued
   decisions." Provisional apply + rollback is a distinct threat model; not
   opening that door in Phase 5.
   *Audit implication: the veto window close event needs an explicit reason
   field — `applied | vetoed | evidence_diverged | revalidation_failed |
   superseded`. Without it,
   delayed-apply is a post-hoc audit black hole. Confirm `ApplyAudit` (issue
   #5) captures the close event, not just the apply event. (Echo 2026-07-18)*

3. **Harness regions: classify ceremony or become threads? → CLASSIFY FIRST.**
   Region → thread promotion only when a stable source-authoritative projection
   exists. Default: threads source-bound, region evidence on proposal
   classification. No premature thread proliferation.
   *Forward-only promotion: if a region reclassifies mid-session, promotion
   applies forward only. Retroactive projection would corrupt the authority
   trail with apparently-authored writes from before the promotion decision.
   (Echo 2026-07-18)*

4. **Invariants: deterministic extraction, identity probes, or both?
   → DETERMINISTIC WHERE POSSIBLE; probes as Gate-3 evidence, not sole authority.**
   Consistent with RFC-0001 rule that LLM-judge-only is forbidden for auto-tier.
   Same logic applies to invariant checking.
   *When deterministic extraction fails or is ambiguous, default is
   fail-closed (no promotion) — not silent fallback to LLM judgment.
   Ambiguity is an explicit ignored/blocked state. (Echo 2026-07-18)*

5. **Must the RFC #3/#4 amendments (familiar-contract PR #3) land before
   freeze? → YES.**
   The corrected amendments are drafted in OpenCoven/familiar-contract PR #3.
   Implementation may proceed against that reviewed shape, but Phase 5 cannot
   freeze until Nova + Val approve and merge the upstream normative text.

6. **Cave `ProposalView` extension: daemon contract first or simultaneous?
   → DAEMON CONTRACT FIRST.**
   Cave extension only after daemon read models exist. Consistent with Phase 4
   pattern. Cave ProposalView for Phase 5 starts as `[DESIGNED, NOT SHIPPED]`
   until release evidence exists. (Echo 2026-07-18)

7. **Preserve display labels `auto`, `familiar_review`, `human_review`,
   `human_required`? → YES — display names preserved, typed variants internal.**
   Daemon owns the typed `ApprovalPath` enum; Cave renders the human-readable
   labels. No mental-model breakage.
   *Label-variant round-trip must be a daemon wire contract, not Cave
   convention. Daemon emits `{variant, label, veto_deadline}`; Cave has zero
   policy freedom over label strings. Daemon should reject at load if a variant
   has no corresponding display label or vice versa. (Sage 2026-07-18)*

8. **Add `proposal_window_opened` audit event? → YES, when delayed apply ships.**
   Minimal ledger entry for auditable veto windows. Connects to issue #5
   (`ward_audit` / `ApplyAudit`); that lane (Cody) can consume both in the same
   schema pass.
   *Also requires a corresponding close event with reason field (see decision
   #2 audit note). The window is a first-class audit interval, not a gap.*

---

## 7. Phase shape — implementation is open

Val opened Phase 5 implementation on 2026-07-18. Beads are live. The phase is
not frozen and this record does not claim independent Nova approval.

- `threads-uqx` — [EPIC] approval semantics over the Phase-2 Ward.
- `threads-uqx.1` — ✅ RESOLVED: `ApprovalPath` separate from `Channel`; delayed
  apply; classify-first for regions; deterministic+probe for invariants.
- `threads-uqx.2` — RFC #3/#4 amendment gate: corrected DRAFT
  familiar-contract PR #3 must receive Nova + Val approval and merge before
  Phase 5 freeze.
- `threads-uqx.11` — Fresh authority-review remediation for this decision
  record; blocks Nova sign-off until the protected-target, RFC-schema,
  attestation, and corpus findings are resolved.
- `threads-uqx.12` — Upstream RFC/schema amendment defining the normative
  mapping or explicit retirement of `approval_tiers`, `blocks`, `gate`, and
  veto fields. Blocks implementation completion and Nova sign-off.
- `threads-uqx.13` — Authorized, sanitized, reproducible v0.1 migration fixture
  covering invariant declarations, approval lanes, veto settings, and harness
  regions. Blocks implementation completion.
- `threads-uqx.3` — Core type sketch: `ApprovalPath`, `VetoWindow`,
  `ProposalClassification`, region evidence, audit event shape.
  *Design constraints from Sage/Echo (2026-07-18) to incorporate:*
  *(a) `evidence_replay_hash` on `ProposalClassification` — delayed-apply*
  *revalidation at deadline must prove it replays the same evidence that*
  *opened the window (WARD-C7 generalizes: evidence must survive the time gap).*
  *(b) `VetoWindow.min_visible: Duration` — veto window is only fail-closed if*
  *pending state was actually visible long enough for a human to act on it*
  *(same shape as two-compaction contract).*
  *(c) Label mapping as daemon wire contract with load-time reject on drift.*
- `threads-uqx.4` — Predicate design: identity invariant
  `PatternPredicate`; descriptor-derived proof.
- `threads-uqx.5` — Surface-region design: extractor predicates over
  materialized diffs and Gate-4 replay.
- `threads-uqx.6` — Daemon integration design: classification, delayed-apply
  scheduler, revalidation at deadline, append-only audit in `coven.sqlite3`.
- `threads-uqx.7` — Cave contract amendment after daemon read models exist;
  add blocked fixtures for veto states.
- `threads-uqx.8` — Cody implementation lane: core crate and daemon tests,
  migration coverage, and fidelity coverage against `threads-uqx.13`, proving
  retired invariant declarations, approval lanes, veto settings, and harness
  regions are compiled deterministically or rejected explicitly. This lane
  also waits for the normative RFC/schema mapping in `threads-uqx.12`.
- `threads-uqx.9` — Nova sign-off: RFC round-trip, Gate-4 fail-closed proof,
  descriptor-not-authority review.
- `threads-uqx.10` — Val freeze: Phase-5 design frozen or rejected.

Required gates: proposal review; independently attributable Nova design gate;
direct Val authority gate; both RFC amendment gates (`threads-uqx.2` and
`threads-uqx.12`); authorized migration-fixture gate (`threads-uqx.13`);
implementation gate proving no apply bypass; Cave fail-closed rendering gate;
final direct Val freeze after Nova sign-off. Agent statements, authorship
claims, and commit trailers cannot satisfy human gates.

---

## 8. Recommendation — executed

Phase 5 implementation opened under Val's recorded decision. The design
defaults attributed to Nova still require independent attestation before
freeze. The intended direction is to recover semantic approval behavior, not
just document why it was lost.

Carry forward: approval paths as typed promotion ceremony over existing
channels; harness blocks as daemon-replayable surface-region classifiers;
protected invariants as predicate-authoritative identity checks.

Target for retirement after the upstream schema amendment: v0.1 stringly
gate/block coupling and invariant strings as enforcement objects. Already
forbidden: client-side or descriptor-based authority and provisional apply
during veto windows unless explicitly accepted.

The conservative design is not to make Ward more general. It is to restore the
minimum semantic distinctions RFC-0001 still names while preserving the Phase-0
spine: source-authoritative surfaces, predicate-authoritative patterns,
daemon-owned audit, and fail-closed Gate 4.
