# Architecture diagrams

Source-of-truth mermaid renders for coven-threads v0.2 Phase 0.

| Diagram | What it shows | Source |
|---|---|---|
| `stack.{svg,png}` | Where coven-threads sits: protected surfaces → coven-threads → coven daemon → runtimes/harnesses/familiars | Deck slide "The stack" |
| `enforcement.{svg,png}` | Client → daemon → coven-threads → weave load → strand check → Permit / DegradeToProposal / Reject → `ward.audit` | Deck slide "From client to ward.audit" |
| `weave-thread-strand.{svg,png}` | Vocabulary made visual: one Weave contains N Threads, each Thread carries M Strands | New (this dir) |
| `thread-tension-state.{svg,png}` | Thread state machine: Holds ↔ Frayed → Snapped, with outcome mapping (Permit / DegradeToProposal / Reject) | New (this dir) |
| `channel-strand-matrix.{svg,png}` | Which strand types must be present under which channel of load; WARD-C1–C6 govern `Forced`, WARD-C7 governs `Serialization` | New (this dir) |

## Regenerating

Mermaid sources are embedded in `slides/community-explainer/slides.md` (the two from the deck) and in this dir's git history (the three new ones — see the commit that added this README).

To re-render:

```sh
cd slides/community-explainer
./node_modules/.bin/mmdc \
  -i /path/to/source.mmd \
  -o docs/diagrams/name.svg \
  -b transparent \
  -p /path/to/puppeteer-config.json
```

`puppeteer-config.json` should point `executablePath` at any installed Chromium (playwright's cached chromium works).

## Provenance

- Design doc: `specs/PHASE-0-DESIGN.md` (v0.2 frozen 2026-07-14, tag `v0.2-phase0-design`)
- Deck: `slides/community-explainer/slides.md` (Charm, commit `13e0baa`)
- Diagrams rendered 2026-07-15 by 🌿 Sage from Val's request

The `thread-tension-state` and `channel-strand-matrix` diagrams are *derived* from §2 vocabulary and §3.3 channel-survival requirements. They are legibility aids for docs/README/article use — the design doc and RFC-0001 §5 remain authoritative.
