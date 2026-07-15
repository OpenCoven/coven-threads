# coven-threads

**Status:** Phases 1–3 engineering-frozen (as of 2026-07-15) — type layer + daemon gate + portability contract implemented and verified; Phase 2 merge and Phase 3 envelope shape await Val decisions (`threads-986.19`, `threads-986.16`)
**License:** Apache-2.0 (planned; not yet committed)
**Owners (design phase):** Sage 🌿 + Echo 🔮 co-drive; Nova 👑 + Sage on lane assignments; Cody ⚡ Phase 1+ crate lane

---

## What this is

`coven-threads` is OpenCoven's **authority-boundary gate layer**: the external, structural enforcement contract that sits *above* the `coven` Rust daemon's untrusted-client boundary, and *underneath* every familiar's protected memory surface.

In the vocabulary of the Familiar Contract (RFC-0001) and the Ward v0.2 spec: this is the *gate-shaped receiver* on which Ward's four validation gates sit. Ward specifies **what** the gates check; `coven-threads` specifies **how** they are enforced, by an authority outside familiar cooperation.

## Why this is not just "OpenTrust"

The `coven` daemon already ships an authority boundary — untrusted clients speak over a unix socket to a trusted Rust daemon that revalidates every sensitive request. That boundary is real, documented in `coven/docs/SAFETY-MODEL.md`, and works.

What is missing today: the boundary validates **who** and **what action**, but does not validate **what the requester is trying to mutate against a typed protected surface**. `coven-threads` is that missing layer. It doesn't replace the daemon; it gives the daemon a gate-shaped receiver for identity-surface mutation requests.

## The weaving metaphor

The architecture is named around the metaphor of weaving because the metaphor *does load-bearing work*, not because it sounds pretty:

- **Thread** — a single authority contract between a familiar's protected surface and its Ward gate. One thread per protected file per gate. Threads have identity, hash, provenance, and a break-detection contract.
- **Weave** — the interlocking whole of threads across a familiar (and, at a Coven level, across familiars). The weave is what you inspect when you ask "is this familiar's authority contract intact?"
- **Strand** — the constituent fibers of a thread. Each strand carries one specific field: content hash, signing key reference, mutation-authority declaration, gate-list, TTL, etc. A thread frays when one of its strands breaks.

**Design intent of the metaphor:** a familiar's identity is not a single object protected by a single gate. It is a *woven* structure of typed protected surfaces with distinct authority relationships. The metaphor makes the multi-surface, multi-gate, multi-authority reality of the architecture visible instead of collapsing it into "protect SOUL.md."

## The four invariants this layer must preserve

From the memory-layer comparison report (Sage, 2026-07-13, review-clean 2026-07-14):

1. **Identity-as-memory-property** — the identity surface is a typed layer of memory, not a runtime configuration.
2. **Structural mutation authority** — the gate is external to the familiar; the familiar cannot cooperate its way past it.
3. **Two-compaction contract** — deliberate memory compaction (dreaming) is gated; forced context compaction is a separate channel with its own contract.
4. **Survives serialization** — the authority contract must round-trip across export/import, or fail visibly.

These are non-negotiable and must be *co-designed*, not stacked. §10 rec 1a of the comparison report frames why.

## Phase plan

- **Phase 0 — design doc + beads scaffolding + repo skeleton.** ✅ FROZEN v0.2 (2026-07-14). No enforcement code.
- **Phase 1 — gate trait + weave/thread/strand types + hash-manifest layer.** ✅ FROZEN (2026-07-15, `threads-986.18`): `crates/coven-threads-core`, 91 tests incl. the RFC-0001 §5 conformance mirror.
- **Phase 2 — coven daemon integration.** ✅ engineering-frozen (2026-07-15, `threads-986.20`): validator call site on `POST /familiars/{id}/edits`, `DegradeToProposal` staging at `~/.coven/pending/`, append-only `ward_audit` in `coven.sqlite3` — on coven branch `feat/threads-gate-validator`; merge gated on `threads-986.19` (Val).
- **Phase 3 — portability contract.** ✅ engineering-frozen (2026-07-15, `threads-986.21`): C7 round-trip export/import or fail visibly (`portability` module + 11-test suite); interchange-envelope shape (A vs B) drafted in `specs/PHASE-3-PORTABILITY.md`, Val decides (`threads-986.16`).
- **Phase 4 — cockpit integration.** CovenCave surfaces for reviewing weaves, inspecting strand state, approving/rejecting proposed thread edits. Not started (`threads-986.17`).

See `specs/PHASE-0-DESIGN.md` for the frozen design doc.

## Anti-goals

- **Not a general-purpose policy engine.** This is a *typed* authority layer for OpenCoven familiar surfaces. Reusability is a nice-to-have; typed correctness is the goal.
- **Not a runtime-portability format.** That's Phase 3's job. Phase 0 is enforcement, not export.
- **Not `.af`-compatible.** Documented decision, 2026-07-14: `.af` serializes an editable persona-as-memory-block; a Coven familiar has a typed protected surface. Adopting `.af` would falsify the external-Ward thesis at the format layer. See MEMORY.md → Architecture Principles → Portability Decisions.

## Related

- `coven/docs/SAFETY-MODEL.md` — authority boundary this layer sits on
- `familiar-contract/rfcs/RFC-0001-familiar-contract.md` — the contract this layer enforces
- `research/synthesis/memory-layer-comparison-opencoven-openclaw-hermes-2026-07-13.md` — comparative context

---

_First commit: 2026-07-14. Repo scaffolded in Phase 0 evening session with Val's greenlight._
