# Architecture diagrams

Source-authoritative diagrams for `coven-threads`. Every rendered artifact
(`.svg`, `.png`) is generated from a committed source in [`src/`](src/) —
rendered files are never hand-edited. If a diagram disagrees with
`specs/PHASE-0-DESIGN.md` (FROZEN v0.2), `specs/PHASE-5-APPROVAL-SEMANTICS.md`
(decision record), or the shipped crate, the diagram is wrong; recreated
2026-07-15 against both (bead `threads-986.22`).

| Diagram | What it shows | Form (why) | Source of truth |
|---|---|---|---|
| `stack.{svg,png}` | Where the gate layer sits: familiar-controlled processes → daemon trust boundary (containing `coven-threads-core` as the imported validator crate) → protected surfaces, with the RFC-0001 §5.1 no-write-path cut drawn | flowchart with containment subgraph + forbidden edge — the containment *is* the Phase 2 architecture | PHASE-0-DESIGN §1, §3.1–3.2; RFC-0001 §5.1; `coven/docs/SAFETY-MODEL.md` |
| `enforcement.{svg,png}` | The shipped Phase 2 write path: `apply_familiar_edits` → `Ward::evaluate` → blocked-unit refusal → coven-threads gate → `Reject` / `DegradeToProposal` (staged) / `Permit` (`Ward::apply`), every verdict appended to `ward_audit` | decision flowchart — the verdict branching is the story | PHASE-0-DESIGN §5; coven PR #382 (`threads_gate.rs`, `api.rs`) |
| `weave-thread-strand.{svg,png}` | Vocabulary as the types relate: Weave carries an authoritative `PatternPredicate` (descriptor derived, **never enforced on** — §2.2 anti-pattern) and threads = `(surface → writer)` pairs, each with `holds_under` channels and a Vec of strands | containment subgraphs; dashed derive edge for the anti-pattern | PHASE-0-DESIGN §2.1–2.3; `src/{weave,thread,strand,pattern}.rs` |
| `thread-tension-state.{svg,png}` | The tension state machine (`Holds`/`Frayed`/`Snapped`) with the `holds_under(channel)` answer each state produces, incl. the fail-closed `NotCovered` answer | stateDiagram-v2 — a state machine is a state machine; verdicts are answers, not states, so they ride as notes | `src/thread.rs` (`TensionState`, `holds_under`, `fray`, `snap`); `src/fray.rs`; PHASE-0-DESIGN §2.3, §5 |
| `channel-strand-matrix.{svg,png}` | Exactly which strand kinds each channel's floor requires: Deliberate — none (consent is the gate); Forced — ContentHash + ManifestEntry (WARD-C1–C6); Serialization — SerializationMarker (WARD-C7); Mutation — ContentHash | per-channel grouping (box contents *are* the floor; no cross-arrow spaghetti) | `src/channel.rs` (`Channel::required_strand_kinds`); PHASE-0-DESIGN §2.4 |
| `delayed-apply-scheduler.{svg,png}` | The Phase 5 delayed-apply lifecycle: daemon classification assigns `ApprovalPath` (never derived from `Channel`), pending-visible veto window (`opened_at` · minimum-visible floor · deadline), veto/supersession closes, deadline-triggered evidence replay deciding apply vs fail-closed reject, the blocked `AwaitingHumanApproval` lane, and a reason-tagged audit row at every transition — no path reaches APPLY without live daemon re-materialization, and no provisional apply exists | decision flowchart with daemon containment + a forbidden provisional-apply edge — the single audited road to APPLY is the story | PHASE-5-APPROVAL-SEMANTICS §4 + decision 2 (also 1, 7, 8); coven PR #430 (daemon-owned scheduler); `src/approval.rs`, `src/audit.rs` |

## Regenerating

One command, from this directory:

```sh
./render.sh
```

It renders every `src/*.mmd` to `NAME.svg` and a 2x `NAME.png` through the
shared [`mermaid-config.json`](mermaid-config.json) theme. Regeneration is
byte-stable on a given toolchain (verified 2026-07-15: re-render diffed clean
against the committed artifacts).

**Toolchain (pinned):**

- `@mermaid-js/mermaid-cli` **11.16.0**, installed locally at
  `slides/community-explainer/node_modules` (`npm install` there first).
- A Chromium for puppeteer-core: `PUPPETEER_EXECUTABLE_PATH` if set, else
  Playwright's cached Chromium, else Google Chrome (see `resolve_chromium`
  in `render.sh`).

## Dark/light safety

The canvas is transparent, and the theme puts every glyph on a surface it
owns: nodes, clusters, edge labels, and notes all carry explicit fills with
explicit text colors, and free-floating text (titles, lines) uses mid-tone
`#9B8BB4`, legible on both white and near-black page backgrounds. Do not add
diagram text that sits directly on the transparent canvas in a theme-default
color.

**Embedding:** embed the `.png` (rendered at 2x), not the `.svg` — mermaid's
`htmlLabels` emit `<foreignObject>` nodes, which many `<img>`-context
renderers (including some GitHub paths) drop, losing every label. The SVGs
stay committed as the vector artifacts for direct viewing and print.

## Provenance

- Design doc: `specs/PHASE-0-DESIGN.md` (v0.2 frozen 2026-07-14, tag
  `v0.2-phase0-design`) — authoritative, with RFC-0001 upstream of it.
- Crate: `crates/coven-threads-core` (tags `v0.1.0`–`v0.1.2`).
- First diagram set rendered 2026-07-15 from the community-explainer deck;
  recreated the same day against the frozen design + shipped crate after an
  audit found content drift in all five (worst: the old matrix contradicted
  `Channel::required_strand_kinds()` on every channel). Audit record: bead
  `threads-986.22.1`.
