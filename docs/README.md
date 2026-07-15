# coven-threads documentation

`coven-threads` is OpenCoven's **authority-boundary gate layer**: the gate-shaped receiver that sits above the shipped `coven` Rust daemon and beneath every familiar's protected memory surface. It is a conforming implementation of RFC-0001 §5 (the Ward): the daemon stays authoritative for *who* may act and *what actions* exist; `coven-threads` adds the missing question — *does the authority state of the targeted surface permit this write?*

If you are new here, read [concepts.md](concepts.md) first. Everything else assumes its vocabulary.

## Current status (2026-07-15, honest version)

- **Phase 0 (design) — `[FROZEN]`.** The design doc `specs/PHASE-0-DESIGN.md` is frozen at v0.2 (2026-07-14, tag `v0.2-phase0-design`), with Nova sign-off and an RFC-0001 §5 round-trip verified.
- **Phases 1–3 (code) — `[ENGINEERING FROZEN]`.** All three engineering phases are frozen with evidence — bead `threads-986.18` (Phase 1 crate FREEZE), `threads-986.20` (Phase 2 daemon-integration FREEZE), `threads-986.21` (Phase 3 C7 portability FREEZE) — all closed. Full workspace test suite is green (98 tests: 72 unit + 12 C7 round-trip + 14 §5 conformance). **The gate is on coven `main`**: PR https://github.com/OpenCoven/coven/pull/382 merged 2026-07-15 after `threads-986.19` resolved (repo flipped public for CI dep access), and the portability envelope is decided (`threads-986.16` closed: Shape B `.weave` + lossy one-way `.af` exporter). One human gate remains — Nova's grimoire review (`threads-986.12`).
- **Phase 4 (cockpit UX) — `[NOT STARTED]`.**

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
| [phases.md](phases.md) | Phase 0 → 4 with honest status labels: what is frozen, what is implemented, what is blocked, what is not started. |
| [faq.md](faq.md) | Anticipated questions, answered honestly with sources. |
| [glossary.md](glossary.md) | Every named term, one line each, alphabetical, with links to depth. |

## Normative sources

These docs are descriptive. When they disagree with a normative source, the source wins, in this order:

1. **RFC-0001 §5** (`familiar-contract/rfcs/RFC-0001-familiar-contract.md`) — the external correctness anchor. *RFC wins on any conflict.*
2. **`specs/PHASE-0-DESIGN.md`** — the frozen Phase 0 design (v0.2).
3. **`coven-grimoire` Ward Layer Spec Brief §9** — the canonical home of WARD-C1–C7.
4. **`coven/docs/SAFETY-MODEL.md`** — the daemon boundary this layer extends.

Diagrams in `diagrams/` are legibility aids derived from the design doc; they are not authoritative (see `diagrams/README.md`).
