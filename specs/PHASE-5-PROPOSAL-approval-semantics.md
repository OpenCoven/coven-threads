# PHASE-5 PROPOSAL — Approval semantics over the Phase-2 Ward

**Status:** PROPOSAL (draft — Phase 5 has not opened; phase gates per `AGENTS.md` still bind)
**Date:** 2026-07-18
**Bead:** `threads-uqx`
**Authors:** Sage lane (drafted by coordinator session agent)

---

## 0. Scope and sources

This is a proposal document only. It does not start Phase 5, amend frozen
Phase 0, or change the daemon. Val and Nova decide whether Phase 5 opens.

Sources read and cited:

- `specs/PHASE-0-DESIGN.md`: frozen v0.2; Thread (authority relationship:
  surface -> writer), Weave (enforced pattern of threads), Strand (fiber inside
  a thread), Channel (axis of load), predicate-authoritative / descriptor-
  derived rule, Gate-4 fail-closed posture, and WARD-C1..C7 lineage.
- `../familiar-contract/rfcs/RFC-0001-familiar-contract.md`: §4.2 protected
  invariants, §4.3 editable surface, §5.3 approval tiers, §5.4 gates, §5.5
  probes. RFC amendments for closure precondition and provenance predicate are
  pending on issues #3/#4 and are treated as upstream-in-flight.
- `../coven-grimoire/articles/drafts/2026-06-24-ward-layer-spec-brief.md` §9:
  canonical home of WARD-C1..C7.
- `specs/PHASE-4-CAVE-SURFACES.md`: approval flow §3.7, fail-closed rendering
  rules §4, and non-goals §6.
- `../coven/crates/coven-cli/src/ward.rs`, `threads_gate.rs`, `api.rs`: current
  daemon reality — path tiers, Gate 1-4 comments, protected-surface weave
  construction, and `decide_threads_proposal` as the only tier-0 commit point.
- `~/.coven/workspaces/familiars/nova/ward.toml.v01.bak`: read-only example of
  the retired dialect. This draft paraphrases its invariant shape and does not
  reproduce Val's exact invariant strings.

---

## 1. What v0.1 expressed that Phase 2 cannot

Phase 2 reduced Ward configuration to path-tier mapping: tier 0 protected,
tier 1 reviewed, tier 2 logged, tier 3 free. In the daemon, `WardConfig` stores
surface globs and `protected_surface`; `Ward::apply` refuses blocked proposals
as a unit, holds Tier 1 and Gate-1-authorized Tier 0 for Gate 3, and writes only
Tier 2/3. `apply_after_threads_approval` still reruns Gates 1 and 2, and
`api.rs::decide_threads_proposal` revalidates a pending proposal, calls the
threads gate, appends audit rows, advances baselines, removes the pending file,
and is the only current tier-0 approval commit point.

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

What stays dead is stringly coupling among `blocks = [...]`, `gate = "..."`,
and ad hoc veto fields. If Phase 5 opens, policy should be typed and
daemon-owned. The Cave may render it; it must not become a policy engine.

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

The v0.1 invariant syntax should stay dead. It was legible but underspecified:
parser semantics, evidence collection, normalization, provenance, and failure
handling were not defined. Pending RFC issues #3/#4 should land, or be waived
by Val/Nova, before Phase 5 freezes invariant interfaces.

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

Path tier remains a floor. A Tier 0 target still requires principal authority
and the threads gate. A Tier 2 path touching a high-risk semantic region may
still require human review. Highest ceremony wins for the proposal as a unit,
matching current all-or-nothing Ward behavior.

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

A synchronous gate answers before write: permit, stage, or reject. Phase 2's
protected-target flow stages on fray, waits for principal approval, revalidates,
and applies. Nothing writes before authority clears.

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
- final `proposal_approved`, `proposal_vetoed`, `proposal_expired`, or
  `proposal_rejected`: daemon revalidation result immediately before apply or
  refusal.

WARD-C6 matters by analogy: a time-separated, lossy or delayed boundary needs a
ledger. A proposal whose evidence cannot be replayed at deadline fails closed.
Future Cave surfaces should extend `ProposalView` only after daemon evidence
exists; until then unknown or stale veto state renders blocked under Phase 4 §4.

---

## 5. Explicit non-goals

- No general policy engine; README anti-goals and Phase 4 §6 bind.
- No client-side approval authority; Cave forwards, daemon decides.
- No revival of v0.1 TOML as normative format.
- No Phase-5 start and no changes to frozen Phase 0.
- No weakening of Gate 4; every path ends in live daemon re-materialization.
- No `.af` compatibility work; Phase 3 already decided `.weave` canonical plus
  lossy one-way `.af` exporter.
- No broad runtime capability system; tool grants may classify proposals, but
  runtime chamber authority remains separate.

---

## 6. Open questions for Val/Nova

1. Should approval semantics be `ApprovalPath`, separate from `Channel`?
   Recommended default: yes; channels remain load axes.
2. Are veto windows delayed-apply or provisional-apply?
   Recommended default: delayed apply only.
3. Do harness regions classify proposal ceremony only, or become threads?
   Recommended default: classify first; promote only with stable source
   projection.
4. Are invariants checked by deterministic extraction, identity probes, or both?
   Recommended default: deterministic where possible; probes are Gate-3
   evidence, not sole auto authority.
5. Must RFC issues #3/#4 land before freeze?
   Recommended default: yes for freeze, no for exploratory design.
6. Should Phase 5 extend Cave `ProposalView`, or wait for daemon policy?
   Recommended default: daemon contract first, Cave extension second.
7. Preserve labels `auto`, `familiar_review`, `human_review`, `human_required`?
   Recommended default: preserve display names, encode typed variants.
8. Add `proposal_window_opened` audit event?
   Recommended default: yes if delayed apply ships.

---

## 7. Rough phase shape, if Phase 5 opens

Skeleton only; do not create these beads until Val/Nova open Phase 5.

- `threads-uqx` — [EPIC] approval semantics over the Phase-2 Ward.
- `threads-uqx.1` — Sage/Nova gate: decide `ApprovalPath` vs `Channel` and veto
  semantics.
- `threads-uqx.2` — RFC dependency check: issues #3/#4; freeze blocked unless
  landed or waived.
- `threads-uqx.3` — Core type sketch: `ApprovalPath`, `VetoWindow`,
  `ProposalClassification`, region evidence, audit event shape.
- `threads-uqx.4` — Predicate design: identity invariant
  `PatternPredicate`; descriptor-derived proof.
- `threads-uqx.5` — Surface-region design: extractor predicates over
  materialized diffs and Gate-4 replay.
- `threads-uqx.6` — Daemon integration design: classification, delayed-apply
  scheduler, revalidation at deadline, append-only audit in `coven.sqlite3`.
- `threads-uqx.7` — Cave contract amendment after daemon read models exist;
  add blocked fixtures for veto states.
- `threads-uqx.8` — Cody implementation lane: core crate and daemon tests.
- `threads-uqx.9` — Nova sign-off: RFC round-trip, Gate-4 fail-closed proof,
  descriptor-not-authority review.
- `threads-uqx.10` — Val freeze: Phase-5 design frozen or rejected.

Suggested gates: proposal review; Nova design gate; Val authority gate; RFC
amendment gate; implementation gate proving no apply bypass; Cave fail-closed
rendering gate; final Val freeze after Nova sign-off.

---

## 8. Recommendation

Open Phase 5 only if Val/Nova want to recover semantic approval behavior, not
just document why it was lost.

Carry forward: approval paths as typed promotion ceremony over existing
channels; harness blocks as daemon-replayable surface-region classifiers;
protected invariants as predicate-authoritative identity checks.

Leave dead: v0.1 stringly gate/block coupling; invariant strings as enforcement
objects; client-side or descriptor-based authority; provisional apply during
veto windows unless explicitly accepted.

The conservative design is not to make Ward more general. It is to restore the
minimum semantic distinctions RFC-0001 still names while preserving the Phase-0
spine: source-authoritative surfaces, predicate-authoritative patterns,
daemon-owned audit, and fail-closed Gate 4.
