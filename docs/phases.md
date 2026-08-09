# Phases

> This page is the honest status ledger. Labels used throughout: `[FROZEN]` (design complete and change-controlled), `[MERGED]` (landed on the `main` branch of a downstream system that will call this crate at runtime once cut into a release; buildable and integration-tested but **not in a released binary** yet), `[RELEASED]` (in a tagged downstream release users can install), `[ENGINEERING FROZEN]` (code complete, tests green, awaiting one named decision to reach a deployed system), `[ACTIVE]` (open engineering phase with in-flight beads), `[BLOCKED]` (waiting on a named decision), `[NOT STARTED]`.
>
> The one-sentence truth: **phases 0–4 are frozen, Phase 5 (approval semantics) is active but its sign-off gate was refused on 2026-07-29 and the four named remediation beads are still open and unclaimed, everything downstream is merged but not in a released binary — no enforcement exists anywhere in production.** No released daemon in the wild calls this code. If a doc or deck implies otherwise, this page wins.

Vocabulary (bound in [concepts.md](concepts.md)): **Thread** = authority relationship *surface → writer*; **Weave** = enforced pattern of threads; **Strand** = fiber inside a thread; **Channel** = axis of load.

## Phase 0 — Design `[FROZEN]`

**Deliverable:** the design doc `specs/PHASE-0-DESIGN.md`, plus the repo scaffold and beads plan. No enforcement code was in scope, deliberately: *"If this doc is wrong, we would rather find out reading it than debugging it."*

**Status:** FROZEN v0.2, 2026-07-14, tag `v0.2-phase0-design`. The freeze gate was real: Sage's v0.1 draft, Echo's substrate-authority pass (which rebound the metaphor to correct referents and introduced the `Channel` enum), Nova's non-negotiables folded in, and a formal Nova sign-off including a verified round-trip against RFC-0001 §5.1 / §5.4 / §5.6. A post-freeze citation amendment (2026-07-15, bead `threads-986.12`) pointed C7's canonical home at the `coven-grimoire` Ward Layer Spec Brief §9 — a citation fix, not a design change.

The frozen doc is change-controlled. These docs describe it; they do not amend it.

## Phase 1 — Core crate `[MERGED, NOT RELEASED]`

**Scope (beads `threads-986.6`–`.11`, Cody's Rust lane):** the `coven-threads-core` crate — `Strand`, `Thread`, `Weave`, `Channel`, `TensionState` types; the `PatternPredicate` trait with its derived `describe()` introspection; the hash-manifest layer (Merkle over strand hashes in canonical `(surface_path, writer_id)` order); and the RFC-0001 §5 conformance test suite mirrored into Rust.

**Status:** landed on `main` (commit `86550d8`), beads `986.6`–`.11` closed with evidence; **Phase 1 FREEZE recorded in `threads-986.18` (closed).** The full workspace test suite is green — 205 tests as of 2026-07-21 (174 unit, 17 C7 round-trip, 14 §5 conformance; plus 1 ignored doc-test). The count has grown past the Phase-1 freeze baseline because the crate now also carries the Phase 5 approval-semantics modules (see Phase 5 below). `unsafe_code = "forbid"` at the workspace level.

**What "implemented" does not mean:** the crate is a library. It has no side effects by design — no filesystem verification, no audit writes, no staging I/O. Until a daemon calls it (Phase 2), it enforces nothing.

## Phase 2 — Daemon integration `[FROZEN; MERGED TO COVEN MAIN]`

**Scope:** the validator call site inside the `coven` daemon's existing socket handling; the `DegradeToProposal` staging path at `~/.coven/pending/`; the `ward.audit` table live in `coven.sqlite3`; the notification protocol to the principal.

**Status, in two halves:**

- **Crate side — landed** (commit `5e68957`): `audit.rs` defines the `ward.audit` record shape and DDL (append-only via triggers, RFC-0001 §5.6 event vocabulary); `staging.rs` defines the pending-proposal record shape. The crate owns the *contracts*; the daemon owns the connection, the writes, and the directory.
- **Daemon side — merged** `[MERGED, NOT RELEASED]`: the validator call site (on `POST /familiars/{id}/edits`), staging path, and the live audit table landed on coven `main` via **PR https://github.com/OpenCoven/coven/pull/382** (branch `feat/threads-gate-validator`, squash-merged 2026-07-15). The Phase 2 epic bead `threads-986.14` is **closed** (engineering complete), **`threads-986.20` (Phase 2 FREEZE) is closed**, and **`threads-986.19` (merge gate) is closed** — resolved by flipping `coven-threads` public so coven CI can fetch the pinned git dependency (deny.toml `[sources]` allow-lists it). Cross-repo write authority for the branch work itself was already gated and Val-granted (bead `threads-986.15`, closed).

Every `POST /familiars/{id}/edits` touching a tier-0 surface on a daemon built from coven `main` now flows through this gate.

## Phase 3 — Portability format `[ENGINEERING FROZEN; ENVELOPE DECIDED]`

**Scope:** the Coven Familiar Portability Format — the artifact a familiar exports to and imports from, with C7 enforced across the round-trip.

**Status:** **Phase 3 FREEZE recorded in `threads-986.21` (closed).** The *semantics* are implemented and tested (`portability.rs` + the `c7_roundtrip.rs` suite, 17 tests as of 2026-07-21): the `PortableWeave` envelope, the `SerializationContract` with its drift-visible contract hash, `export_weave`/`import_weave` with the full fail-visibly matrix (tamper → hash mismatch; version skew, contract skew, duplicate pairs → typed refusals; import never widens authority). The *interchange encoding* is **decided** (`threads-986.16` closed, 2026-07-15): **Shape B — the net-new `.weave` envelope — is canonical**, plus a clearly-marked lossy one-way `.af` exporter for Letta handoff (follow-up bead `threads-jq4`; no `.af` import path, ever). Decision record: `specs/PHASE-3-PORTABILITY.md` §6.

**Not `.af`-compatible — documented divergence.** Whatever shape wins, the format will not be a compatible `.af` round-trip surface. The reason is factual, source-verified 2026-07-14 against `letta-ai/letta/main/letta/serialize_schemas/pydantic_agent_schema.py`: Letta's `CoreMemoryBlockSchema` has no protection field, and the runtime `read_only` flag is stripped at export. An artifact format that cannot represent the protection contract cannot satisfy C7 — silent downgrade on import is precisely the failure mode C7 exists to refuse. This is a neutral engineering constraint, not a judgment of `.af` for its own goals; see the [FAQ](faq.md#why-isnt-it-af-compatible).

## Phase 4 — Coven Cave UX `[COMPLETE; FROZEN 2026-07-17]`

**Scope (epic `threads-986.17`, closed):** cockpit surfaces in Coven Cave — the weave rail view, a thread detail pane with tension state, strand inspection, and the proposal approval flow for staged writes from `~/.coven/pending/`. Charm owned the voice/copy pass on all four surfaces.

**Status:** **complete and FROZEN, 2026-07-17.** The surface contract is `specs/PHASE-4-CAVE-SURFACES.md` (this repo's PR #1). All four surfaces — the weave rail with tension rollup, the thread pane (Holds/Frayed/Snapped), the strand inspector with tri-state diff + R7 lineage, and the proposal approval flow — merged via **coven-cave PR #3223** (18 checks green, `test:app` 740/740). The freeze gates were real: Charm voice pass (`threads-986.17.7`), Nova coherence sign-off (`.17.8`), Val UX-accept (`.17.10`), and Val freeze approval (`.17.9`). Rendering rules R1–R11 are enforced fail-closed. Follow-up bead: `threads-v3g` (daemon endpoints + adapter flip — the surfaces render against an adapter until the daemon HTTP endpoints land).

**Post-freeze addition:** degraded-familiar surfacing (`threads-k9s`, closed) — spec §2.7 `DegradedFamiliarView` + rendering rule §4.R12; daemon half merged as coven PR #422, Cave half as coven-cave PR #3415.

## Phase 5 — Approval semantics `[ACTIVE]`

**Scope (epic `threads-uqx`; spec `specs/PHASE-5-APPROVAL-SEMANTICS.md`):** the approval-ceremony layer over Gate 4 — typed approval paths, veto windows with delayed apply, semantic surface regions, and evidence replay at the deadline. Opened 2026-07-18 by Val+Nova decision.

**Design commitments** (stated here because they are easy to get subtly wrong):

- **`ApprovalPath` is orthogonal to `Channel`.** The typed `ApprovalPath` (auto / familiar-review / human / human-with-rationale) is the approval-ceremony axis. It is never derived from `Channel`, and `Channel` remains a first-class enforcement axis in its own right — neither is a descriptor of the other.
- **Delayed-apply only — no provisional apply.** A classified proposal is pending-visible for the whole veto window and is applied only after the deadline passes with no veto **and** the committed evidence replays to a match. Every window close carries an explicit reason.
- **Classification and scheduling are daemon-owned.** The crate defines the types and predicates; the daemon classifies, schedules, and applies.
- **Identity invariants are predicate-authoritative** (the descriptor-vs-predicate rule from [concepts.md](concepts.md) applies here too).

**Ledger, as of 2026-08-09:**

- **Closed:** `.3` core approval types — `ApprovalPath`, `ApprovalPathKind`, `VetoWindow`, `ProposalClassification` (`approval.rs`); `.4` identity invariant predicates + advisory probes (`identity_invariants.rs`); `.5` `SurfaceRegionPredicate` + Gate-4 replay (`surface_regions.rs`); `.6` delayed-apply scheduler + audit — implemented **daemon-side in coven PR #430** (daemon-owned classification and scheduler, deadline/minimum-visible revalidation, fail-closed committed-evidence replay, cross-platform conditional atomic writes, startup recovery); `.11` authority review findings resolved; `.2` RFC closure/provenance amendments; `.7` Cave veto-window contract; `.8` implementation and migration fidelity; `.12` RFC-0001 approval-tier alignment; `.13` authorized retired-Ward migration fixture. The proposal/decision-record PR #6 merged 2026-07-27 as `091607f`.
- **Open — and this is the whole of Phase 5's remaining state:** `.9` Nova coherence sign-off gate and `.10` Val freeze gate. Both are human gates that agents must never simulate. Related: `threads-3xd` — RFC-0001 amendments (§5.5 closure precondition + §4.2 predicate (iv) provenance).

**Nova's sign-off is BLOCKED, not pending (2026-07-29).** This is the single most important fact about Phase 5 and it is easy to miss from the bead counts alone. Nova ran independent core and integration reviews against coven `f3cd322`, coven-threads `091607f`, merged coven PRs #430/#464, merged Cave PRs #3581/#3628, and familiar-contract PRs #3/#4, and **refused sign-off**. The design choices were explicitly affirmed as coherent — Channel and `ApprovalPath` remain separate axes, delayed apply is correct, Cave stays thin and fail-closed. What blocks is implementation, in five named beads:

| Bead | Blocking finding | Status |
|---|---|---|
| `threads-3jx` | `ward_audit` schema classification was substring-based | **closed** — PR #23, `8e2de93` |
| `threads-okc` | identity predicates not wired into daemon mutation/replay; accepted migrated invariants remain backup-only | open, unclaimed |
| `threads-980` | opened veto windows can reach rejection branches with no typed close detail | open, unclaimed |
| `threads-dgg` | protected `SOUL.md` changes still stage/approve through a proposal route, contrary to RFC-0001 §5.4 | open, unclaimed |
| `threads-zav` | retired-Ward corpus does not prove end-to-end schedulability | open, unclaimed |

**Re-verified 2026-08-09 (Echo):** all four open blockers are **still true** against current coven `main` `59c5be4` — 129 commits after the commit Nova reviewed. None of the four has been claimed or started, and none of the relevant code paths changed in that range. Nova's findings transfer verbatim to current code; she does not need to re-review. Evidence with file:line is recorded on `threads-uqx.9`. Caveat stated there and repeated here: that was static verification (code read against the recorded claims), not a test run.

So Phase 5 is `[ACTIVE]` in name but **stalled on unclaimed remediation work**, not on either human gate. `.10` cannot ripen until `.9` clears, and `.9` cannot clear until the four beads close.

**Related open beads not in the gate chain** (none block `.9`): `threads-xpo` `Channel::Deliberate` is unreachable from the daemon; `threads-55s` record channel on `memory_entry_admitted` rows (blocked by `xpo`); `threads-ot6` promotion-write ↔ weave seam contract (draft PR #25, awaiting Cody); `threads-76z` schema permits an unapprovable proposal the Rust types forbid (awaiting a Nova ruling); `threads-bnu` unpinned local Rust toolchain; `threads-t6t` no secret-scanning CI; `threads-5rr` familiar inbox + handoff ledger.

## Summary table

| Phase | What it is | Status | Gate to next step |
|---|---|---|---|
| 0 | Design doc + scaffold | `[FROZEN]` v0.2, tag `v0.2-phase0-design` | — (done) |
| 1 | `coven-threads-core` crate | `[MERGED, NOT RELEASED]` — imported by coven `main` (Cargo.toml git dep at tag `v0.1.2`); the running coven daemon binary (release `v0.0.54`, 2026-07-14) predates PR #382 and does not yet call it; 205 tests green (2026-07-21); `.18` closed | — (frozen) |
| 2 | Daemon integration | `[MERGED, NOT RELEASED]`, `.14` + `.20` + `.19` closed; PR #382 merged 2026-07-15 as commit `f745117`; next coven release will include it (current release `v0.0.54` predates the merge) | — (release-cut) |
| 3 | Portability format | `[ENGINEERING FROZEN]`, `.21` + `.16` closed; envelope `[DECIDED: Shape B + lossy .af export]` | follow-up `threads-jq4` (exporter) |
| 4 | Coven Cave UX | `[COMPLETE; FROZEN 2026-07-17]`, epic `threads-986.17` closed; coven-cave PR #3223 merged (18 checks green, `test:app` 740/740); Charm `.17.7`, Nova `.17.8`, Val `.17.10` + `.17.9` gates passed; post-freeze `threads-k9s` closed (coven PR #422 + coven-cave PR #3415) | follow-up `threads-v3g` (daemon endpoints + adapter flip) |
| 5 | Approval semantics | `[ACTIVE]` since 2026-07-18 — but **sign-off refused 2026-07-29** and no remediation work is claimed. Epic `threads-uqx`; `.2`–`.8`/`.11`/`.12`/`.13` closed; **only `.9` + `.10` remain**. Nova named five implementation blockers; `threads-3jx` closed, and `threads-okc`/`980`/`dgg`/`zav` are open and unclaimed (all four re-verified still true 2026-08-09 vs coven `59c5be4`) | close the four remediation beads → Nova sign-off (`.9`) → Val freeze (`.10`) |

## Known housekeeping discrepancies

Tracked in `docs/STATUS-2026-07-15.md` and worth knowing when reading the repo:

- **License mismatch:** the design doc and README say *Apache-2.0 (planned)*; the committed `LICENSE` file is MIT (with a separate `PATENTS` file). Needs a deliberate reconciliation; until then, treat the license as unsettled.
- **`.bak` files in `specs/`:** three pre-freeze backups sit beside the frozen doc; git history already preserves them.
- **`PHASE-0-DESIGN.md` §9 is headed "Open questions (need resolution before v0.2 freeze)" — but v0.2 froze on 2026-07-14 with all four still listed.** Recorded here rather than fixed, because the design doc is frozen and change-controlled; this page describes it and does not amend it. Three of the four were settled by events after the freeze: §9.2 (Phase 2 daemon-integration ownership) resolved when Phase 2 merged as coven PR #382; §9.3 (portability format Shape A vs B) resolved 2026-07-15 as **Shape B**, recorded in `specs/PHASE-3-PORTABILITY.md` §6; §9.4 (Phase 4 UI/UX) resolved by `specs/PHASE-4-CAVE-SURFACES.md` and the 2026-07-17 Phase 4 freeze. §9.1 (whether federation forces a fourth `fabric` level) is genuinely still open and still correctly deferred. The heading is what is stale, not the content — a reader who trusts it will think four live blockers stand in front of a freeze that already happened.
- **`Channel::Deliberate` is specified but unreachable.** `PHASE-0-DESIGN.md` §2.4 and non-negotiable #4 (the two-compaction contract) treat `Deliberate` and `Forced` as distinct channels with distinct survival requirements. `Forced` is woven into the daemon's `PROTECTED_CHANNELS`; `Deliberate` is referenced nowhere in coven, and every channel value the daemon constructs is a hardcoded `Channel::Mutation`. Tracked as `threads-xpo`. Relevant when reading §2.4 or the promotion-write seam contract, both of which describe intent that no code path can currently express.
