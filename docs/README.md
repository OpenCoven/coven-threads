# coven-threads documentation

`coven-threads` is OpenCoven's **protected-authority validator**, imported by the
trusted `coven` daemon. The daemon owns authentication and effects; Threads
defines whether protected-surface state permits a write. Its contracts target
RFC-0001 §5, while the delivery ledger records unresolved daemon conformance
findings.

If you are new here, read [concepts.md](concepts.md) first. Everything else assumes its vocabulary.

## Current status (2026-09-12 UTC)

- **Phase 0 (design) — `[FROZEN]`.** The design doc `specs/PHASE-0-DESIGN.md` is frozen at v0.2 (2026-07-14, tag `v0.2-phase0-design`), with Nova sign-off and an RFC-0001 §5 round-trip verified.
- **Phases 1–2: `[FROZEN; IN RELEASE TAG]`; Phase 3: `[ENGINEERING FROZEN]`.** Coven `v0.4.3` includes the daemon integration and pins Threads to `c102844`. This is release-source provenance, not verification of deployed configuration or current-checkout compatibility. Phase 3 retains the decided `.weave` envelope and lossy one-way `.af` export semantics.
- **Phase 4: `[FROZEN]` (2026-07-17).** The four Cave surfaces and their recorded human gates are complete under `threads-986.17`. The daemon-adapter follow-up `threads-v3g` and degraded-familiar follow-up `threads-k9s` are closed. See the delivery ledger for merge evidence.
- **Phase 5: `[ACTIVE]`.** The earlier core, scheduler, Cave contract (`.7`), and RFC alignment (`.12`) implementation beads are closed. Four daemon-boundary blockers still prevent coherence sign-off. The shared harness and protected/terminal fixes are draft checkpoints, and neither Nova's review nor Val's freeze can be replaced by agent evidence. See [phases.md](phases.md) for the work graph and [the E2E contract](testing/e2e-contract.md) for closure requirements.

Baseline `main` protection is active under the
[authorized solo-maintainer policy](../CONTRIBUTING.md#solo-maintainer-merge-policy).
The separate privacy job has landed, but is not a required check. The required
pinned-daemon check and scheduled canary are not wired. The Automation Authority Profile v1 is a
separate shipped schema/reference-validator contract, not evidence of daemon
adoption or a replacement for the Rust gate.

These docs describe frozen and active contracts without amending them. Use the
[delivery strategy](strategy.md) for sequencing, the [ledger](phases.md) for
delivery evidence, and the [dated readiness review](reviews/2026-09-11-landscape-and-readiness.md)
for findings and remaining acceptance obligations.

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
| [automation-authority-profile.md](automation-authority-profile.md) | Operation-specific automation authority, approvals, proposal-only downgrade, replay/TOCTOU, privacy, and portable vectors. |
| [channels-and-strands.md](channels-and-strands.md) | The four channels of load, the five strand types, which strands each channel structurally requires, WARD-C1–C7. |
| [phases.md](phases.md) | Phase 0 → 5 with honest status labels: what is frozen, what is implemented, what is active, what is blocked. |
| [strategy.md](strategy.md) | Workstream order, canonical owners, closure evidence, and release prerequisites. GitHub issues and Beads retain task status. |
| [Readiness review](reviews/2026-09-11-landscape-and-readiness.md) | Dated landscape, documentation corrections, and engineering recommendations for the open review gates. |
| [E2E contract](testing/e2e-contract.md) | Real-daemon topology, eight required journeys, deterministic time, and blocker-closure evidence. |
| [faq.md](faq.md) | Anticipated questions, answered honestly with sources. |
| [glossary.md](glossary.md) | Every named term, one line each, alphabetical, with links to depth. |

## Normative sources

These docs are descriptive. When they disagree with a normative source, the source wins, in this order:

1. **RFC-0001** (`familiar-contract/rfcs/RFC-0001-familiar-contract.md`) governs familiar identity and Ward requirements.
2. **Frozen or active decision records under `specs/`** govern Threads semantics, including the Phase-5 and versioned automation authority contracts.
3. **Public Rust contracts and conformance vectors** in `crates/coven-threads-core` define the implementation surface.
4. **Explanatory material under `docs/`** describes those contracts without amending them.

The `coven-grimoire` Ward Layer Spec Brief §9 is the cited canonical home of
WARD-C1–C7. `coven/docs/SAFETY-MODEL.md` describes the daemon boundary; it does
not replace the source precedence above.

Diagrams in `diagrams/` are legibility aids derived from the design doc; they are not authoritative (see `diagrams/README.md`).

## Historical plans and reports

The [July status report](STATUS-2026-07-15.md), [ApplyAudit design](superpowers/specs/2026-07-19-apply-audit-migration-repair-design.md),
and [ApplyAudit implementation plan](superpowers/plans/2026-07-19-apply-audit-migration-repair.md)
preserve earlier checkpoints. Do not execute their completed tasks or treat
their old counts as current acceptance. Rendered diagrams, the PDF, and slides
are explanatory snapshots, not release evidence.
