# AGENTS.md — coven-threads

This repo implements OpenCoven's authority-boundary gate layer. It is the gate-shaped receiver Ward v0.2 sits on and it sits above the `coven` Rust daemon.

## Working here

- **Design doc is authoritative**: `specs/PHASE-0-DESIGN.md`. If code disagrees with the design doc, the design doc wins until v0.2 freezes.
- **Ward RFC-0001 is upstream of everything here.** If this repo disagrees with `familiar-contract/rfcs/RFC-0001-familiar-contract.md`, RFC wins and this repo is wrong.
- **`coven` daemon is the trust boundary.** This repo does not replace it; it becomes a validator library the daemon imports.
- Every code change touches the beads DB. Prefix: `threads-*`. Use `bd ready --json` to pick up work; use `bd close` with an evidence-bearing reason.
- Phase gates matter. Phase N+1 does not start until Phase N's freeze bead closes.

## Familiars owning surfaces here

- 🌿 **Sage** — design doc co-driver; Phase 0 authoring; research and synthesis.
- 🔮 **Echo** — substrate-authority co-driver on design doc; metaphor mapping (§2); four-invariant co-design (§3.3).
- 👑 **Nova** — coven-daemon integration lane (§3); compatibility contract (§6); lane assignments.
- ⚡ **Cody** — Rust type sketch and Phase 1+ crate work.
- ✨ **Charm** — UX voice and Phase 4 Cave surface language (deferred).

## What this repo is not

See README.md § "Anti-goals".

## Related

- `../coven/` — Rust daemon (authority boundary this layer sits on)
- `../familiar-contract/` — RFC-0001 upstream normative reference
- `../coven-cave/` — cockpit that will surface weave/thread/strand state (Phase 4)
- `../coven-grimoire/` — Ward Layer Spec Brief

_First commit 2026-07-14._
