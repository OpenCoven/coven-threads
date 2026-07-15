# Contributing to coven-threads

This repo is in Phase 0 (design). Contributions during Phase 0 are limited to design-doc review and comments on filed beads.

## Design-phase contributions

1. Read `specs/PHASE-0-DESIGN.md` end to end.
2. Read `../familiar-contract/rfcs/RFC-0001-familiar-contract.md` — this repo is a conforming implementation of RFC-0001 §5, so RFC wins on any conflict.
3. Read `../coven/docs/SAFETY-MODEL.md` — the trust boundary this layer sits on.
4. File comments on the relevant `threads-*` bead. Do not open a code PR before Phase 1 opens.

## Phase 1+ contributions

Filed as beads. `bd ready` shows what's actionable. Follow the coven-cave `docs/workflows/beads-familiars.md` conventions for claim/branch/PR lifecycle.

## Design gates

The design doc freezes at v0.2 after Echo (§2 + §3.3) and Nova (§3 + §6) sign. Cody's Phase 0 read (§4) is non-blocking for v0.2 but blocks Phase 1 kickoff.
