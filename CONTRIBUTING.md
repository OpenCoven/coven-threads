# Contributing to coven-threads

You can contribute implementation changes and documentation corrections within the current contracts. Phases 0–4 are frozen, and Phase 5 remains active. Read [the delivery ledger](docs/phases.md) for unresolved implementation and human gates.

## Before you contribute

1. Read [the contributor entry point](AGENTS.md) and the relevant decision records under `specs/`. Frozen contracts require change control.
2. Read `../familiar-contract/rfcs/RFC-0001-familiar-contract.md`. RFC-0001 governs familiar identity and Ward requirements and wins on conflicts.
3. Read `../coven/docs/SAFETY-MODEL.md` for the daemon trust boundary.
4. Use a GitHub issue for changes to public contracts or cross-repository behavior. Link the relevant `threads-*` bead when Beads is available; a clean-clone contribution must not require a personal Beads database.

## Local verification

From the repository root:

```bash
bash scripts/agent-bootstrap.sh
bash scripts/agent-check.sh fast
bash scripts/agent-check.sh full
```

Follow `AGENTS.md` for authority-impact evidence, downstream acceptance, and the applicable contribution workflow.

These commands validate local repository contracts and the portable automation
profile; they do not launch a Coven daemon. Required current-checkout
compatibility and the scheduled canary remain work under #31. Use the
[delivery strategy](docs/strategy.md) for sequencing and owner-specific
acceptance rather than repeating a completed historical plan.

## Design gates

The Phase-0 design freeze is complete. Phase-5 remediation, Nova's independent coherence review, and Val's freeze remain outstanding. Agents may prepare evidence; they cannot replace either human decision.
