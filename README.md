# coven-threads

**Status (2026-09-09):** Engineering phases 0–4 are **frozen**. Phase 5 is **active**, with four unresolved implementation blockers and independent Nova/Val gates still outstanding. Coven `v0.4.3` includes the earlier daemon integration and pins Threads to `c102844`; release ancestry does not prove full Phase-5 conformance or current-checkout compatibility. See [the delivery ledger](docs/phases.md) for evidence, draft fixes, and the real-daemon work graph.
**License:** stated as Apache-2.0 (planned) in the design doc; the committed `LICENSE` file is currently MIT — a known discrepancy pending reconciliation (see `docs/STATUS-2026-07-15.md` §6).
**Owners (design phase):** Sage 🌿 + Echo 🔮 co-drive; Nova 👑 + Sage on lane assignments; Cody ⚡ Phase 1+ crate lane

---

## What this is

`coven-threads` is OpenCoven's **authority-boundary gate layer**: the external, structural enforcement contract that sits *above* the `coven` Rust daemon's untrusted-client boundary, and *underneath* every familiar's protected memory surface.

In the vocabulary of the Familiar Contract (RFC-0001) and the Ward v0.2 spec: this is the *gate-shaped receiver* on which Ward's four validation gates sit. Ward specifies **what** the gates check; `coven-threads` specifies **how** they are enforced, by an authority outside familiar cooperation.

![Where coven-threads sits: familiar-controlled processes reach the daemon over a unix socket; the daemon trust boundary contains coven-threads-core as an imported validator crate and is the sole write authority over protected surfaces; RFC-0001 §5.1 forbids any familiar write path](docs/diagrams/stack.png)

It defines validator contracts for RFC-0001 §5. **RFC wins on any conflict**
with this repository. Full daemon conformance remains subject to the unresolved
boundary findings in [the delivery ledger](docs/phases.md).

Gate 4 fail-closed is a line-one conformance property, not a hardening milestone: an implementation that allows Gate 4 to be bypassed does not conform to RFC-0001 (§5.4).

## Documentation

User-facing docs live in [`docs/`](docs/README.md):

- [Concepts](docs/concepts.md) — the vocabulary (Thread, Weave, Strand, Channel), the two-compaction contract, the descriptor-vs-predicate anti-pattern. **Read this first.**
- [Architecture](docs/architecture.md) — where this layer sits, the enforcement flow, the `ward.audit` store.
- [Authority model](docs/authority-model.md) — fail-closed, the three verdicts, the tension state machine.
- [Automation Authority Profile](docs/automation-authority-profile.md) — operation-specific permit / approval / proposal / reject decisions, replay-safe approval evidence, and portable vectors.
- [Channels and strands](docs/channels-and-strands.md) — the four channels, the five strand kinds, WARD-C1–C7.
- [Phases](docs/phases.md) — what is frozen, what is implemented, what is active, what is blocked.
- [FAQ](docs/faq.md) · [Glossary](docs/glossary.md)

The frozen design doc is [`specs/PHASE-0-DESIGN.md`](specs/PHASE-0-DESIGN.md); the docs describe it and never amend it.

The versioned Automation Authority Profile is
[`specs/AUTOMATION-AUTHORITY-PROFILE-v1.md`](specs/AUTOMATION-AUTHORITY-PROFILE-v1.md),
with closed schemas, a Node-core reference validator, and its mandatory exact
conformance manifest under
[`profiles/automation-authority/v1/`](profiles/automation-authority/v1/).
Threads defines the authority decision; Coven still owns scheduling,
persistence, final dispatch, and effects.

## Why this is not just "OpenTrust"

The `coven` daemon already ships an authority boundary — untrusted clients speak over a unix socket to a trusted Rust daemon that revalidates every sensitive request. That boundary is real, documented in `coven/docs/SAFETY-MODEL.md`, and works.

Threads supplies the typed protected-surface question alongside **who** may act
and **what action** exists. The daemon imports this validator; Threads does not
replace its trust boundary or own filesystem and persistence effects. The
remaining work is complete route, predicate, and replay enforcement, not the
absence of any integration.

![The shipped Phase 2 enforcement flow: client request through Ward::evaluate, blocked proposals refused as a unit, the coven-threads gate validating each protected target fail-closed, and the three verdicts — Reject (403), DegradeToProposal (staged at ~/.coven/pending/), Permit (Ward::apply) — with every verdict appended to the append-only ward_audit table](docs/diagrams/enforcement.png)

## The weaving metaphor

The architecture is named around the metaphor of weaving because the metaphor *does load-bearing work*, not because it sounds pretty. Every term is bound to a concrete referent at first use (design doc §2.5); the referents below are the frozen v0.2 bindings:

- **Thread** (*authority relationship: surface → writer*) — a directional line from a protected surface (SOUL.md, MEMORY.md, an identity field) to the authority that gates writes to it. One thread per `(surface, writer)` pair. Threads have **tension**: they hold, fray, or snap under load.
- **Weave** (*enforced pattern of threads across a familiar or Coven*) — the invariant that these specific threads must all hold *together* for the identity to be coherent. Ward's four gates are the **loom** the weave is made on — the fixed structure threads run through — not threads themselves.
- **Strand** (*fiber inside a thread: hash | signature | manifest entry | audit trail | serialization marker*) — the fibers that make a thread survive stress. A thread survives a channel iff its strands survive that channel; a thread **frays** when a strand fails, which is what makes failure legible.
- **Channel** (*axis of load a thread must hold under*) — `Deliberate`, `Forced`, `Serialization`, `Mutation`. Every gate check is one question: *does thread T hold under channel C?*

**Design intent of the metaphor:** a familiar's identity is not a single object protected by a single gate. It is a *woven* structure of typed protected surfaces with distinct authority relationships. The metaphor makes the multi-surface, multi-authority reality of the architecture visible instead of collapsing it into "protect SOUL.md."

![A weave contains an authoritative PatternPredicate (whose descriptor is derived and never enforced on) and threads — one per surface→writer pair — each carrying holds_under channels and a vector of strands](docs/diagrams/weave-thread-strand.png)

![The thread tension state machine: Holds, Frayed (repairable), Snapped (terminal), with the holds_under answer each state produces — including the fail-closed NotCovered answer for uncovered channels](docs/diagrams/thread-tension-state.png)

## The four invariants this layer must preserve

Co-designed as *channels a weave must survive*, not stacked as features (design doc §3.3):

1. **Identity-as-memory-property** — the identity surface is a typed layer of memory, not a runtime configuration; threads bind to typed surfaces at construction time.
2. **Structural mutation authority** — the gate is external to the familiar; the familiar cannot cooperate its way past it. Enforcement is Rust-side, daemon-called.
3. **Two-compaction contract** — deliberate memory compaction (dreaming) and forced context compaction are distinct channels with distinct survival requirements; WARD-C1–C6 govern the forced channel.
4. **Survives serialization (WARD-C7)** — the authority contract must round-trip across export/import, or fail visibly. Numbered seventh so its lineage from C1–C6 stays legible; canonical home for C1–C7 jointly is the `coven-grimoire` Ward Layer Spec Brief §9.

These are non-negotiable and must be *co-designed*, not stacked — the `Channel` enum is where the type system holds them together.

![Per-channel strand floors from Channel::required_strand_kinds — Deliberate has no structural floor (consent is the gate); Forced requires ContentHash and ManifestEntry (WARD-C1–C6); Serialization requires SerializationMarker (WARD-C7); Mutation requires ContentHash](docs/diagrams/channel-strand-matrix.png)

## Phase plan

Honest labels; the detailed ledger is [docs/phases.md](docs/phases.md).

- **Phase 0 — design doc + beads scaffolding + repo skeleton.** ✅ **FROZEN v0.2** (2026-07-14, tag `v0.2-phase0-design`). Nova sign-off; RFC-0001 §5 round-trip verified. No enforcement code, by design.
- **Phase 1: core crate.** `[FROZEN; IN RELEASE TAG]`. `coven-threads-core` provides typed validation and conformance vectors. Coven `v0.4.3` imports revision `c102844`; it does not automatically test this checkout. `threads-986.18` is closed.
- **Phase 2: daemon integration.** `[FROZEN; IN RELEASE TAG]`. Contracts in `audit.rs` and `staging.rs` are consumed by the integration from OpenCoven/coven#382. Beads `.14`, `.20`, and `.19` are closed; later proposal-route defects remain Phase-5 blockers.
- **Phase 3 — portability contract.** C7 round-trip semantics implemented and tested (`portability` module + 17-test round-trip suite); the interchange envelope is **decided** (`threads-986.16` closed): **Shape B (`.weave`) canonical, plus a lossy one-way `.af` exporter** — see the §6 decision record in `specs/PHASE-3-PORTABILITY.md`.
- **Phase 4: cockpit integration.** `[COMPLETE; FROZEN 2026-07-17]`. The weave rail, thread pane, strand inspector, and proposal flow landed in OpenCoven/coven-cave#3223 with the recorded human gates. The daemon-adapter follow-up (`threads-v3g`) and degraded-familiar follow-up (`threads-k9s`) are also closed. New Phase-5 live-daemon acceptance remains separate.
- **Phase 5: approval semantics.** `[ACTIVE]`, not frozen. `ApprovalPath` and `Channel` remain independent; delayed apply requires live evidence replay and exactly one typed terminal close. The earlier core, scheduler, Cave contract, RFC alignment, and corpus implementation beads are closed, but `threads-okc`, `threads-980`, `threads-dgg`, and `threads-zav` remain unresolved. OpenCoven/coven#931, OpenCoven/coven#932, and OpenCoven/coven#933 are draft checkpoints, not closure proof. Nova coherence sign-off (`threads-uqx.9`) and Val freeze (`threads-uqx.10`) remain human gates.

## Anti-goals

- **Not a general-purpose policy engine.** This is a *typed* authority layer for OpenCoven familiar surfaces. Reusability is a nice-to-have; typed correctness is the goal.
- **Not a runtime-portability format.** That's Phase 3's job. Phase 0 is enforcement design, not export.
- **Not `.af`-compatible.** Documented divergence, source-verified 2026-07-14: Letta's `CoreMemoryBlockSchema` has no protection field and runtime `read_only` is stripped at export — silent downgrade on import is exactly what WARD-C7 refuses. See [docs/faq.md](docs/faq.md#why-isnt-it-af-compatible).

## Related

- `coven/docs/SAFETY-MODEL.md` — authority boundary this layer sits on
- `familiar-contract/rfcs/RFC-0001-familiar-contract.md` — the contract this layer enforces (RFC wins on conflict)
- `coven-grimoire` Ward Layer Spec Brief §9 — canonical home of WARD-C1–C7
- `research/synthesis/memory-layer-comparison-opencoven-openclaw-hermes-2026-07-13.md` — comparative context

---

_First commit: 2026-07-14. Repo scaffolded in Phase 0 evening session with Val's greenlight._
