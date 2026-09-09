# Phases

> This page separates design approval, merged implementation, release provenance, and end-to-end proof. `[FROZEN]` means design complete and change-controlled; `[MERGED]` means present on downstream `main`; `[IN RELEASE TAG]` means included in a published release's source revision, not a claim about any running installation; `[ENGINEERING FROZEN]` means implementation complete at the recorded checkpoint. `[ACTIVE]`, `[BLOCKED]`, and `[NOT STARTED]` describe remaining work.
>
> **As of 2026-09-09: phases 0–4 remain frozen, and Phase 5 remains active with four unresolved remediation gates.** Daemon and Cave release tags include the earlier integration. Published draft fixes now have bounded real-daemon evidence, but full boundary acceptance and the independent coherence/freeze decisions remain outstanding.

Vocabulary (bound in [concepts.md](concepts.md)): **Thread** = authority relationship *surface → writer*; **Weave** = enforced pattern of threads; **Strand** = fiber inside a thread; **Channel** = axis of load.

## Release and verification evidence

[Coven `v0.4.3`](https://github.com/OpenCoven/coven/releases/tag/v0.4.3),
published 2026-09-02, resolves to
`8baa9c9b722a3a9553c6ed39b7e1ba2296ced95a`. Its
[`coven-cli` manifest](https://github.com/OpenCoven/coven/blob/8baa9c9b722a3a9553c6ed39b7e1ba2296ced95a/crates/coven-cli/Cargo.toml)
pins `coven-threads-core` to `c102844`. Its ancestry includes the scheduler
merge from OpenCoven/coven#430 and schema adoption from OpenCoven/coven#675.
[Cave `v0.4.1`](https://github.com/OpenCoven/coven-cave/releases/tag/v0.4.1),
published 2026-09-09, includes the Phase-5 corrections from
OpenCoven/coven-cave#3628.

These are source-provenance facts, not certification of installed binaries,
deployment configuration, or all Phase-5 invariants. The historical claim that
no release contains Threads is obsolete, as is the `v0.1.2` dependency reference.

The readiness foundation merged in #34 (`db0049b`). The
[E2E contract](testing/e2e-contract.md) and
[compatibility manifest](../e2e/compatibility.toml) still distinguish core tests
from a required pinned daemon gate. OpenCoven/coven#931 is an **unmerged
integration checkpoint**, not full J1–J8 acceptance. Deterministic scheduler
control and supported scheduled publication are implemented in drafts
OpenCoven/coven#968 and OpenCoven/coven#972 and exercised together through the
real daemon. The stable compatibility pin remains unchanged; local integration
does not make an unreviewed downstream revision a required gate.

## Phase 0 — Design `[FROZEN]`

**Deliverable:** the design doc `specs/PHASE-0-DESIGN.md`, plus the repo scaffold and beads plan. No enforcement code was in scope, deliberately: *"If this doc is wrong, we would rather find out reading it than debugging it."*

**Status:** FROZEN v0.2, 2026-07-14, tag `v0.2-phase0-design`. The freeze gate was real: Sage's v0.1 draft, Echo's substrate-authority pass (which rebound the metaphor to correct referents and introduced the `Channel` enum), Nova's non-negotiables folded in, and a formal Nova sign-off including a verified round-trip against RFC-0001 §5.1 / §5.4 / §5.6. A post-freeze citation amendment (2026-07-15, bead `threads-986.12`) pointed C7's canonical home at the `coven-grimoire` Ward Layer Spec Brief §9 — a citation fix, not a design change.

The frozen doc is change-controlled. These docs describe it; they do not amend it.

## Phase 1 — Core crate `[FROZEN; IN RELEASE TAG]`

**Scope (beads `threads-986.6`–`.11`, Cody's Rust lane):** the `coven-threads-core` crate — `Strand`, `Thread`, `Weave`, `Channel`, `TensionState` types; the `PatternPredicate` trait with its derived `describe()` introspection; the hash-manifest layer (Merkle over strand hashes in canonical `(surface_path, writer_id)` order); and the RFC-0001 §5 conformance test suite mirrored into Rust.

**Status:** landed on `main` at `86550d8`; beads `986.6`–`.11` and the Phase 1 freeze (`threads-986.18`) are closed. The 2026-07-21 checkpoint recorded 205 tests (174 unit, 17 C7 round-trip, and 14 §5 conformance, plus one ignored doc-test). That historical count is not the current suite size. `unsafe_code = "forbid"` at the workspace level.

**What "implemented" does not mean:** the crate is a library. It has no side effects by design — no filesystem verification, no audit writes, no staging I/O. Until a daemon calls it (Phase 2), it enforces nothing.

## Phase 2 — Daemon integration `[FROZEN; IN RELEASE TAG]`

**Scope:** the validator call site inside the `coven` daemon's existing socket handling; the `DegradeToProposal` staging path at `~/.coven/pending/`; the `ward.audit` table live in `coven.sqlite3`; the notification protocol to the principal.

**Status, in two halves:**

- **Crate side — landed** (commit `5e68957`): `audit.rs` defines the `ward.audit` record shape and DDL (append-only via triggers, RFC-0001 §5.6 event vocabulary); `staging.rs` defines the pending-proposal record shape. The crate owns the *contracts*; the daemon owns the connection, the writes, and the directory.
- **Daemon side** `[IN RELEASE TAG]`: the validator call site, staging path, and audit table landed via OpenCoven/coven#382 and are present in the release ancestry above. The Phase 2 epic `threads-986.14`, freeze `threads-986.20`, and merge gate `threads-986.19` are closed. The merge gate was resolved by making this repository public so downstream CI could fetch the pinned dependency. This does not close the later protected-proposal route defect, `threads-dgg`.

The daemon calls the validator on protected edits. That integration does not
establish that every proposal, replay, or recovery route satisfies Phase 5.

## Phase 3 — Portability format `[ENGINEERING FROZEN; ENVELOPE DECIDED]`

**Scope:** the Coven Familiar Portability Format — the artifact a familiar exports to and imports from, with C7 enforced across the round-trip.

**Status:** the Phase 3 freeze (`threads-986.21`) and envelope decision (`threads-986.16`) are closed. `PortableWeave`, `SerializationContract`, and `export_weave`/`import_weave` implement the C7 round-trip and fail-visible behavior in `portability.rs` and `c7_roundtrip.rs`. The canonical format is `.weave`. The explicitly lossy, one-way `.af` exporter also landed: `threads-jq4` closed with merged #2 (`5b4a51a`). There is no `.af` import path. See `specs/PHASE-3-PORTABILITY.md` §6.

**Not `.af`-compatible — documented divergence.** Whatever shape wins, the format will not be a compatible `.af` round-trip surface. The reason is factual, source-verified 2026-07-14 against `letta-ai/letta/main/letta/serialize_schemas/pydantic_agent_schema.py`: Letta's `CoreMemoryBlockSchema` has no protection field, and the runtime `read_only` flag is stripped at export. An artifact format that cannot represent the protection contract cannot satisfy C7 — silent downgrade on import is precisely the failure mode C7 exists to refuse. This is a neutral engineering constraint, not a judgment of `.af` for its own goals; see the [FAQ](faq.md#why-isnt-it-af-compatible).

## Phase 4 — Coven Cave UX `[COMPLETE; FROZEN 2026-07-17]`

**Scope (epic `threads-986.17`, closed):** cockpit surfaces in Coven Cave — the weave rail view, a thread detail pane with tension state, strand inspection, and the proposal approval flow for staged writes from `~/.coven/pending/`. Charm owned the voice/copy pass on all four surfaces.

**Status:** **complete and frozen, 2026-07-17.** The surface contract is `specs/PHASE-4-CAVE-SURFACES.md` (#1). The weave rail, thread pane, strand inspector, and proposal approval flow merged in OpenCoven/coven-cave#3223. The recorded freeze gates were Charm's voice pass (`threads-986.17.7`), Nova's coherence sign-off (`.17.8`), and Val's UX acceptance (`.17.10`) and freeze (`.17.9`). The adapter follow-up `threads-v3g` is also closed, through OpenCoven/coven#408, OpenCoven/coven#409, and OpenCoven/coven-cave#3362. The earlier fixtures-first description is no longer current.

**Post-freeze addition:** degraded-familiar surfacing (`threads-k9s`, closed) — spec §2.7 `DegradedFamiliarView` + rendering rule §4.R12; daemon half merged as coven PR #422, Cave half as coven-cave PR #3415.

## Phase 5 — Approval semantics `[ACTIVE]`

**Scope (epic `threads-uqx`; spec `specs/PHASE-5-APPROVAL-SEMANTICS.md`):** the approval-ceremony layer over Gate 4 — typed approval paths, veto windows with delayed apply, semantic surface regions, and evidence replay at the deadline. Opened 2026-07-18 by Val+Nova decision.

**Design commitments** (stated here because they are easy to get subtly wrong):

- **`ApprovalPath` is orthogonal to `Channel`.** The typed `ApprovalPath` (auto / familiar-review / human / human-with-rationale) is the approval-ceremony axis. It is never derived from `Channel`, and `Channel` remains a first-class enforcement axis in its own right — neither is a descriptor of the other.
- **Delayed-apply only — no provisional apply.** A classified proposal is pending-visible for the whole veto window and is applied only after the deadline passes with no veto **and** the committed evidence replays to a match. Every window close carries an explicit reason.
- **Classification and scheduling are daemon-owned.** The crate defines the types and predicates; the daemon classifies, schedules, and applies.
- **Identity invariants are predicate-authoritative** (the descriptor-vs-predicate rule from [concepts.md](concepts.md) applies here too).

**Ledger, as of 2026-09-09:**

- **Closed:** `.3` core approval types — `ApprovalPath`, `ApprovalPathKind`, `VetoWindow`, `ProposalClassification` (`approval.rs`); `.4` identity invariant predicates + advisory probes (`identity_invariants.rs`); `.5` `SurfaceRegionPredicate` + Gate-4 replay (`surface_regions.rs`); `.6` delayed-apply scheduler + audit — implemented **daemon-side in coven PR #430** (daemon-owned classification and scheduler, deadline/minimum-visible revalidation, fail-closed committed-evidence replay, cross-platform conditional atomic writes, startup recovery); `.11` authority review findings resolved; `.2` RFC closure/provenance amendments; `.7` Cave veto-window contract; `.8` implementation and migration fidelity; `.12` RFC-0001 approval-tier alignment; `.13` authorized retired-Ward migration fixture. The proposal/decision-record PR #6 merged 2026-07-27 as `091607f`.
- **Open human gates:** `.9` Nova coherence sign-off and `.10` Val freeze. They are not the whole remaining implementation scope: the four remediation beads below still block sign-off. Agents must never simulate either decision.

**Nova's sign-off is BLOCKED, not pending (2026-07-29).** This is the single most important fact about Phase 5 and it is easy to miss from the bead counts alone. Nova ran independent core and integration reviews against coven `f3cd322`, coven-threads `091607f`, merged coven PRs #430/#464, merged Cave PRs #3581/#3628, and familiar-contract PRs #3/#4, and **refused sign-off**. The design choices were explicitly affirmed as coherent — Channel and `ApprovalPath` remain separate axes, delayed apply is correct, Cave stays thin and fail-closed. What blocks is implementation, in five named beads:

| Bead | Blocking finding | Status |
|---|---|---|
| `threads-3jx` | `ward_audit` schema classification was substring-based | **closed** — PR #23, `8e2de93` |
| `threads-okc` | identity predicates must run at intake and delayed/restart replay | in progress; draft OpenCoven/coven#969 (`0e94e9c`), issue OpenCoven/coven#885 |
| `threads-980` | every opened window needs exactly one typed terminal close | in progress; draft OpenCoven/coven#932 / issue OpenCoven/coven#886 |
| `threads-dgg` | protected `SOUL.md` must not stage/approve through a proposal route | open; draft OpenCoven/coven#933 / issue OpenCoven/coven#887 |
| `threads-zav` | retired-Ward corpus must prove live schedulability and recovery | open; OpenCoven/coven#888 |

**Historical review:** Echo's 2026-08-09 static review at Coven `59c5be4`
confirmed the four findings then. It is not a current test result and cannot
waive independent review of subsequent changes.

**Current delivery, as of 2026-09-09:** the runtime prerequisites this section
previously described as missing are now implemented as unmerged drafts, not
absent: identity-predicate activation is draft OpenCoven/coven#969 (`0e94e9c`),
supported canonical scheduled publication with an explicit minimum-visibility
window is draft OpenCoven/coven#972 (`7a5f4244`), and an isolated deterministic
daemon-boundary test clock is draft OpenCoven/coven#968 (`8a4f2b7`). Two CI
prerequisites for exercising these against a real daemon are also drafted:
Windows daemon-lifecycle readiness (draft OpenCoven/coven#970, `cbd5626a`) and
authenticated Ubuntu-only apt source isolation (draft OpenCoven/coven#973,
`c731cada`, whose hosted run
[34388895012](https://github.com/OpenCoven/coven/actions/runs/34388895012)
passed in full). None of these five prerequisite drafts is merged; treat them as
drafted fixes, not closed prerequisites.

**Hosted readiness evidence:** the initial Windows response-timeout repair
passed on OpenCoven/coven#970 at `9b591e4e`. A later pre-connect pipe timeout
on OpenCoven/coven#933 required another bounded retry fix, `cbd5626a`.
Its affected protected-route lineage at `3d2c06d0` now passes the full
[CI run 34394040609](https://github.com/OpenCoven/coven/actions/runs/34394040609),
including Windows and the PR gate. Scheduled publication at `cf8df046` also
contains both Windows prerequisites. This is evidence for the named revisions,
not a blanket assertion about every subsequent draft head.

**Published integrated evidence:** OpenCoven/coven#931 now contains
`c226656dbb2b5b344eab1625cc9dfe31fe1fe9e6`, with this Threads checkout at
`c3bd46bc` proven by `cargo metadata`. The selected authority suite passed
189 tests. The complete feature-enabled Unix daemon target passed all 15 tests
on three consecutive runs. It covers canonical corpus migration and intake,
exact minimum visibility and deadline, restart, all five typed terminal
families, replacement-only apply after explicit supersession, no-window human
approval, contradictory human/opened-window history, invalid identity at
intake, unsupported duplicate-surface input, and reviewed drift. The bounded
write journey asserts a validation verdict and exact logged before/after hashes
and byte count.

The eight-case replay/restart loop covers veto, materialized drift, unavailable
Ward, changed/unavailable identity, rotated principal binding, and changed or
removed regional approval policy. Actual daemon failures exposed and drove
repairs for predicate failures without terminal closes, stale principal
binding, identity checks skipped at reviewed intake, stale approval policy,
and socket publication before private permissions were installed
(OpenCoven/coven#974). The socket repair retains client ownership checks and
passes deterministic no-clobber/private-publication regressions.

This is still bounded evidence: signed principal authorization, changed runtime
bindings, aggregate multi-file atomic visibility, Windows daemon journeys,
Cave acceptance, and the reviewed required-pin gate are not certified. Linux
and the existing macOS push CI lane now invoke the feature-enabled daemon target;
the new integration head's hosted result is separate from its local evidence.
None of this closes the four root remediation gates or substitutes for Nova
or Val.

**Runtime prerequisites, now drafted rather than missing:** at Coven
`380e765e40e9f84771a805d51a64c06fe79c3110`, migration compiled retired identity
invariants but retained them only in the backup, with the active
[`WardConfig`](https://github.com/OpenCoven/coven/blob/380e765e40e9f84771a805d51a64c06fe79c3110/crates/coven-cli/src/ward.rs#L273-L293)
not activating them and
[`threads_gate.rs`](https://github.com/OpenCoven/coven/blob/380e765e40e9f84771a805d51a64c06fe79c3110/crates/coven-cli/src/threads_gate.rs#L188-L203)
supplying no candidate identity context; production staging at that checkpoint
still published legacy pending envelopes rather than a scheduled envelope
carrying classification and replay evidence. Draft OpenCoven/coven#969 targets
identity activation, draft OpenCoven/coven#972 targets scheduled publication,
and draft OpenCoven/coven#968 targets deterministic daemon-boundary time —
each remains an open, unmerged draft. The integrated evidence above exercises
their supported runtime paths, but is not independent human closure. Do not
replace these contracts with test-only constructors or a new identity model.

**Draft checkpoint scope:** OpenCoven/coven#931 is now the combined integration
checkpoint, including cross-lane repairs not present on every standalone draft.
OpenCoven/coven#932 and OpenCoven/coven#933 retain their scoped terminal and
protected-route history. Reconcile overlapping commits before merging; green
individual branches are not interchangeable with the integrated result.

**Related work outside the sign-off blocker set:** `threads-xpo` tracks the
missing daemon promotion-channel path; `threads-55s` depends on it for auditable
admission channels. `threads-ot6` is draft #25, with `threads-5mn` tracking its
upstream work reference and `threads-lm4` tracking runtime conformance.
`threads-76z` was reopened because its purported fix, #27, was closed **without
merge** and superseded by stricter terminal-close work. Do not resurrect its
null-close bypass. `threads-bnu` is closed via #28; `threads-t6t` still tracks
absent secret/privacy CI. `threads-5rr` is an unratified design proposal, not
authorization to add an audit event.

### Repository governance still outstanding

#31 tracks required pinned daemon checks, deterministic time, OS/Cave acceptance,
Action SHA pins, and measured coverage/flake targets. The 2026-09-09 GitHub
snapshot reports `main` as unprotected and no repository rulesets. CI and
CODEOWNERS files do not themselves enforce branch protection. The readiness
foundation is merged, but these governance items remain open.

## Summary table

| Phase | What it is | Status | Gate to next step |
|---|---|---|---|
| 0 | Design doc + scaffold | `[FROZEN]` v0.2, tag `v0.2-phase0-design` | — (done) |
| 1 | `coven-threads-core` crate | `[FROZEN; IN RELEASE TAG]`; Coven `v0.4.3` pins `c102844`; `.18` closed | Current-checkout daemon compatibility still needs proof |
| 2 | Daemon integration | `[FROZEN; IN RELEASE TAG]`; `.14`, `.20`, and `.19` closed | Phase-5 route and replay defects remain separate |
| 3 | Portability format | `[ENGINEERING FROZEN]`; `.21`, `.16`, and exporter follow-up `threads-jq4` closed | No `.af` import or authority-preserving `.af` round-trip |
| 4 | Coven Cave UX | `[COMPLETE; FROZEN 2026-07-17]`; `threads-986.17`, adapter follow-up `threads-v3g`, and degraded-familiar follow-up `threads-k9s` closed | New Phase-5 live-daemon acceptance remains separate |
| 5 | Approval semantics | `[ACTIVE]`; sign-off refused 2026-07-29; four remediation beads remain unresolved, with draft harness and fix work underway | Real-daemon remediation evidence, then independent Nova sign-off (`.9`) and Val freeze (`.10`) |

## Known housekeeping discrepancies

Tracked in `docs/STATUS-2026-07-15.md` and worth knowing when reading the repo:

- **License mismatch:** the design doc and README say *Apache-2.0 (planned)*; the committed `LICENSE` file is MIT (with a separate `PATENTS` file). Needs a deliberate reconciliation; until then, treat the license as unsettled.
- **`.bak` files in `specs/`:** three pre-freeze backups sit beside the frozen doc; git history already preserves them.
- **`PHASE-0-DESIGN.md` §9 is headed "Open questions (need resolution before v0.2 freeze)" — but v0.2 froze on 2026-07-14 with all four still listed.** Recorded here rather than fixed, because the design doc is frozen and change-controlled; this page describes it and does not amend it. Three of the four were settled by events after the freeze: §9.2 (Phase 2 daemon-integration ownership) resolved when Phase 2 merged as coven PR #382; §9.3 (portability format Shape A vs B) resolved 2026-07-15 as **Shape B**, recorded in `specs/PHASE-3-PORTABILITY.md` §6; §9.4 (Phase 4 UI/UX) resolved by `specs/PHASE-4-CAVE-SURFACES.md` and the 2026-07-17 Phase 4 freeze. §9.1 (whether federation forces a fourth `fabric` level) is genuinely still open and still correctly deferred. The heading is what is stale, not the content — a reader who trusts it will think four live blockers stand in front of a freeze that already happened.
- **`Channel::Deliberate` is specified but unreachable.** `PHASE-0-DESIGN.md` §2.4 and non-negotiable #4 (the two-compaction contract) treat `Deliberate` and `Forced` as distinct channels with distinct survival requirements. `Forced` is woven into the daemon's `PROTECTED_CHANNELS`; `Deliberate` is referenced nowhere in coven, and every channel value the daemon constructs is a hardcoded `Channel::Mutation`. Tracked as `threads-xpo`. Relevant when reading §2.4 or the promotion-write seam contract, both of which describe intent that no code path can currently express.
