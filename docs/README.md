# coven-threads documentation

`coven-threads` is OpenCoven's **authority-boundary gate layer**: the gate-shaped receiver that sits above the shipped `coven` Rust daemon and beneath every familiar's protected memory surface. It is a conforming implementation of RFC-0001 §5 (the Ward): the daemon stays authoritative for *who* may act and *what actions* exist; `coven-threads` adds the missing question — *does the authority state of the targeted surface permit this write?*

If you are new here, read [concepts.md](concepts.md) first. Everything else assumes its vocabulary.

## Current status (2026-07-21, honest version)

- **Phase 0 (design) — `[FROZEN]`.** The design doc `specs/PHASE-0-DESIGN.md` is frozen at v0.2 (2026-07-14, tag `v0.2-phase0-design`), with Nova sign-off and an RFC-0001 §5 round-trip verified.
- **Phases 1–2 — `[MERGED, NOT RELEASED]`; Phase 3 — `[ENGINEERING FROZEN]`.** Phase 1 crate FREEZE (`.18` closed), Phase 2 daemon-integration FREEZE (`.20` closed) + **merged into coven `main`** (PR https://github.com/OpenCoven/coven/pull/382 as commit `f745117`, after `.19` resolved by flipping this repo public for CI dep access), Phase 3 C7 portability FREEZE (`.21` closed) with envelope **decided: Shape B `.weave` canonical + lossy one-way `.af` exporter** (`.16` closed; the exporter itself is follow-up `threads-jq4`). Full workspace test suite is green (205 tests as of 2026-07-21: 174 unit + 17 C7 round-trip + 14 RFC-0001 §5 conformance, plus 1 ignored doc-test). **Release-cut pending:** the running coven daemon (release `v0.0.54`, 2026-07-14) predates PR #382; the next coven release will be the first shipped binary containing the gate. One human gate from this era remains — Nova's grimoire review (`threads-986.12`).
- **Phase 4 (cockpit UX) — `[FROZEN]` (2026-07-17).** Epic `threads-986.17` closed. All four Cave surfaces (weave rail with tension rollup, thread pane, strand inspector with tri-state diff + R7 lineage, proposal approval flow) merged via coven-cave PR #3223 per the `specs/PHASE-4-CAVE-SURFACES.md` contract; Charm (`.17.7`), Nova (`.17.8`), and Val (`.17.10` UX-accept, `.17.9` freeze) gates all passed. Follow-up: `threads-v3g` (daemon endpoints + adapter flip). Post-freeze: degraded-familiar surfacing landed (`threads-k9s` closed; coven PR #422 + coven-cave PR #3415).
- **Phase 5 (approval semantics) — `[ACTIVE]`.** Opened 2026-07-18 by Val+Nova decision; epic `threads-uqx`; spec `specs/PHASE-5-APPROVAL-SEMANTICS.md` (the normative source for this phase — start there, then see [phases.md](phases.md) for the bead-by-bead ledger). Core types, identity invariant predicates, surface regions + Gate-4 replay, and the daemon-side delayed-apply scheduler (coven PR #430) are closed; the Cave veto-window contract (`.7`) and RFC-0001 approval-tier alignment (`.12`) are in progress; the Nova (`.9`) and Val (`.10`) gates are open.

These docs describe the **frozen design**. Where implemented code goes beyond or refines the design, that is labeled explicitly. See [phases.md](phases.md) for the full breakdown.

## Who this is for

- **Operators / principals** who want to understand what protects a familiar's identity surface and what the failure modes look like.
- **Contributors** who need the vocabulary bound correctly before touching code (start with [concepts.md](concepts.md); the metaphor-referent binding rule is not optional).
- **Reviewers** checking conformance claims against RFC-0001 §5.

## Table of contents

| Doc | What it covers |
|---|---|
| [concepts.md](concepts.md) | The vocabulary (Thread, Weave, Strand, Channel), the two-compaction contract, the five channel-survival invariants, the descriptor-vs-predicate anti-pattern. **Read this first.** |
| [architecture.md](architecture.md) | Where coven-threads sits in the stack, the end-to-end enforcement flow, the `ward.audit` store, relationship to RFC-0001 and `coven/docs/SAFETY-MODEL.md`. |
| [authority-model.md](authority-model.md) | Gate 4 fail-closed as a conformance requirement, the three verdicts (Permit / DegradeToProposal / Reject), the thread tension state machine and repair path. |
| [channels-and-strands.md](channels-and-strands.md) | The four channels of load, the five strand types, which strands each channel structurally requires, WARD-C1–C7. |
| [phases.md](phases.md) | Phase 0 → 5 with honest status labels: what is frozen, what is implemented, what is active, what is blocked. |
| [faq.md](faq.md) | Anticipated questions, answered honestly with sources. |
| [glossary.md](glossary.md) | Every named term, one line each, alphabetical, with links to depth. |

## Normative sources

These docs are descriptive. When they disagree with a normative source, the source wins, in this order:

1. **RFC-0001 §5** (`familiar-contract/rfcs/RFC-0001-familiar-contract.md`) — the external correctness anchor. *RFC wins on any conflict.*
2. **`specs/PHASE-0-DESIGN.md`** — the frozen Phase 0 design (v0.2).
3. **`coven-grimoire` Ward Layer Spec Brief §9** — the canonical home of WARD-C1–C7.
4. **`coven/docs/SAFETY-MODEL.md`** — the daemon boundary this layer extends.

Diagrams in `diagrams/` are legibility aids derived from the design doc; they are not authoritative (see `diagrams/README.md`).
