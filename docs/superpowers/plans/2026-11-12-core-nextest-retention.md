# Core Nextest Retention Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver #110's missing core Nextest retention profile and an evidence-based issue disposition packet.

**Architecture:** Separate core-CI collector and pure qualification/summary module reuse the existing ZIP/index/publication safeguards. A bounded Python XML parser preserves actual Nextest retry markers. Positive claims replay from original evidence; metadata-only negatives retain uncertainty.

**Tech Stack:** Node 24.22.0, Python 3.11+ standard library, authenticated GitHub REST, existing archive helpers.

**Spec:** `docs/superpowers/specs/2026-11-12-core-nextest-retention-design.md`

## Global Constraints

- Preserve held main aba2e226c6809b823f39e0a838342e9ea330f59b and all existing worktree/collector ownership.
- Use a separate core archive outside tracked protocol source; native archive remains read-only to this work.
- Keep first workflow attempts, test retries and later workflow attempts separate.
- No merge, dispatch/rerun, collector/pin/check/threshold/window change, release/deployment, issue closure or human acceptance.
- Preserve original failures; qualify no missing, ambiguous, contradictory or zero-test observation.

## Review Focus

- Legacy generic artifact names must not attach an older report to a later attempt; test upload-time identity refusal.
- A flaky-fail report can contain both failure and flaky markers; preserve both without counting one test twice.
- A failed setup may have no checkout/log/XML; preserve a negative row with unknown bindings.
- An offline record/index can be coherently edited; replay claim semantics and require separate authenticated metadata provenance.
- Artifact expiry and incomplete historical coverage remain explicit gaps; no empty archive or recovered retry is a passing denominator.

### Task 1: Strict core report interpretation

**Files:** Create `scripts/lib/nextest-junit.py`, `scripts/lib/nextest-retention.mjs`, and `scripts/tests/nextest-retention.test.mjs`.

**Interfaces:** `parseNextestJunit(bytes)` returns exact counts/cases/UUID/timestamp or refuses. `classifyNextestMetadata({run,jobs,artifacts})` returns a core-only attempt observation. `qualifyNextest(record, xml, log, source)` binds positive evidence. `summarizeNextest(records)` reports deterministic scoped observed trends.

- [ ] Write literal mixed pass/failure/error/skipped/flaky/rerun cases; assert independently derived counts and first-attempt refusal. Add duplicate, malformed, XML entity, zero-count, count-mismatch and ambiguous artifact controls.
- [ ] Run `node --test scripts/tests/nextest-retention.test.mjs`; expected refusal because implementation is missing.
- [ ] Implement the separate parser and classification/qualification/summary module, then rerun the same command; expected all controls pass.
- [ ] Commit the independently testable interpretation unit with its tests.

### Task 2: Immutable collection and replay

**Files:** Create `scripts/retain-nextest-evidence.mjs` and `scripts/tests/nextest-retention-collector.test.mjs`; document in `docs/testing/nextest-evidence-retention.md` and link from `docs/testing/daemon-evidence-retention.md`.

**Interfaces:** `collectNextestAttempt(runId, attempt, archiveRoot, read)` consumes authenticated REST bytes; `summarizeNextestArchive(archiveRoot)` verifies/replays every canonical observation before summarizing. Reuse `extractArtifact`, `publishArchive`, `verifyArtifactIndex` and `writeArtifactIndex` unchanged.

- [ ] Write integration controls for original ZIP validation, exact attempt/source/log/UUID binding, metadata changes, idempotent re-ingestion, conflict refusal, expired/missing/setup-failure observations and replay tampering. Run them first; expected missing collector assertion.
- [ ] Implement fully paginated before/after snapshots, original evidence retention, configuration source binding, separate core archive and deterministic summary/packet. Failures retain indexed staging snapshots.
- [ ] Run both new suites plus the existing archive/native retention suites; expected all pass and native behavior unchanged.
- [ ] Commit the collector/tests/docs unit.

### Task 3: Real evidence, review and disposition

**Files:** Add a dated review packet under `docs/reviews/`; retain mutable operational evidence outside tracked source.

**Interfaces:** Consume collector/replay results and exact current issue criteria. Produce an indexed original-byte evidence package, retention-gap report, reviewed PR and issue checkpoints.

- [ ] Authenticate and ingest existing completed run 42458806610/1 and its original Nextest artifact, plus further existing distinct scope/negative evidence where available. No workflow execution is triggered for sampling.
- [ ] Reconcile #107, #109 and #115 against exact accepted source, original failures, repaired native execution and every stated acceptance criterion; preserve #116 and #111 limits.
- [ ] Stage changes and run appropriate repository fast/full gates on pinned Rust/Node. Preserve exact commands, first failures and source identity.
- [ ] Obtain one fresh whole-branch engineering review; verify material corrections with failing-first controls and the full relevant suite.
- [ ] Prepare an unmerged PR, publish concise preflighted issue checkpoints and verify exact readbacks. Seal and verify the evidence index.
- [ ] Audit every objective requirement against current evidence; complete only when the candidate and disposition package are ready for maintainer review.
