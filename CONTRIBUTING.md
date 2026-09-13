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
compatibility remains work under #31. The separate advisory daemon workflow
supports scheduled latest-main and full-SHA manual observations as described in
the [E2E contract](docs/testing/e2e-contract.md#10-ci-rollout); it is not a
required compatibility gate. Use the
[delivery strategy](docs/strategy.md) for sequencing and owner-specific
acceptance rather than repeating a completed historical plan.

## Solo-maintainer merge policy

You can land an approved change without inventing a second reviewer. The
maintainer [authorized this policy](https://github.com/OpenCoven/coven-threads/issues/31#issuecomment-5642501227)
for the solo-maintainer repository on 2026-09-12 (UTC).

Use a pull request, record the human approval and its scope on GitHub, resolve
review conversations with their actual disposition, and satisfy the required
checks against an up-to-date branch before merging. Agents need explicit human
authorization; a passing check, model review, or persona is not approval.
Material changes outside the approved scope need renewed authorization.

Ruleset `22910327` requires no approving review and disables latest-push and
extra unattributed-change approvals. It retains pull requests, resolved review
threads, stale-review dismissal, five strict required GitHub Actions checks,
and deletion/non-fast-forward protection, with no bypass actors. GitHub
enforces those controls, but does not enforce the conversational human-approval
process or provide independent review.

The fifth required check, `Privacy policy guard`, was
[separately authorized](https://github.com/OpenCoven/coven-threads/issues/39#issuecomment-5643712162)
on 2026-09-12. It retains the checker-trust limits in [the security policy](SECURITY.md).

See the [governance ledger](docs/phases.md#repository-governance-baseline-active-boundary-enforcement-outstanding)
for exact checks and rollback. Reassess independent review when another
maintainer joins. This repository merge policy does not change protected-write
authority or waive the design gates below.

## Design gates

The Phase-0 design freeze is complete. Phase-5 remediation, Nova's independent coherence review, and Val's freeze remain outstanding. Agents may prepare evidence; they cannot replace either human decision.
