# Native daemon 30-day ledger implementation plan

> **For agentic workers:** Use superpowers:executing-plans inline with one fresh whole-branch review.

**Goal:** Implement #111's replayable ledger over 30 actual UTC days without inventing missing native executions, changing the required daemon pin or selecting favorable denominators after observing results.

**Architecture:** Consume #110's original ZIP/metadata archives through a reusable verified reader. Validate retained terminal-close captures, obtain a complete read-only GitHub workflow-run census with original server dates, then build 90 explicit scheduled OS rows and the full per-journey table from an explicit dated declaration and accepted source/profile series. Save an immutable indexed packet; no operational window is selected by this change.

**Tech Stack:** Node 24.22.0, existing Python ZIP/XML validators, authenticated GitHub REST through `gh api` only.

**Spec:** https://github.com/OpenCoven/coven-threads/issues/111 and `docs/testing/e2e-contract.md` success criteria; native workflow/scenario inventory and retention contracts in this branch.

## Global constraints

- Thirty complete UTC days, one 07:47 schedule opportunity per day and Linux/macOS/Windows rows: exactly 90 governing first-attempt OS opportunities. At least 99.5% must qualify; with 90 observations this requires all 90.
- Keep per-journey expected and qualified counts alongside OS rates. Reruns, manual runs, source changes, missing/skipped/cancelled jobs, late/ambiguous arrivals and missing bytes never replace an original scheduled opportunity.
- A caller supplies the window date, declaration time, inventory digest and explicit accepted source/tool/profile series. No implicit latest source or accepted series. Declare these for review before selecting an operational window; this implementation selects none.
- Match schedule runs to their UTC creation date only when creation is at or after 07:47. Cross-midnight delays and duplicate runs in a slot remain unsupported/ambiguous negatives. Record actual creation/start delays.
- Obtain all workflow runs for the declared date range, fully paginate and recheck complete snapshots. Refuse the API's 1,000-result search bound rather than claiming a truncated census.
- Actual GitHub response dates must prove the entire window elapsed; pure test clocks/fixtures never establish a real window. Record caller declaration provenance separately from engineering qualification; never infer human acceptance.
- Preserve original artifacts/metadata and first failures. No dispatch, retry, merge, deployment, threshold/required-check activation, runtime audit writer or public authority contract change.

## Review focus

- A positive archive without its first attempt, or a failed attempt followed by a successful retry, cannot qualify a day.
- A deliberately omitted failed run in a slot must be exposed by the complete census, including duplicate-slot ambiguity.
- Missing or duplicate terminal rows, unsupported census classifications and absent mandatory terminal captures must fail qualification.
- Saved journey counts must be rederived from original ZIPs; copied record fields and rewritten indexes cannot fabricate qualification.
- Unelapsed windows, future/invalid dates, pagination changes, clock/header failures and source/profile mismatches must remain explicit negatives or refuse collection.

### Task 1: Verified archive records and terminal captures

**Files:** Modify `scripts/retain-daemon-evidence.mjs`; create `scripts/lib/daemon-window-terminals.mjs` and `scripts/tests/daemon-window-terminals.test.mjs`.

**Interfaces:** Export `readRetainedArchive(root, {terminalEvidence=false}) -> {records, separately_retained_collections}` while preserving `summarizeArchive(root)`. Recompare original journey proof during replay. `qualifyTerminalCaptures(journeysRoot)` returns scoped nonzero capture/window counts and zero missing/duplicate anomalies, or throws. Inspect original `state/ward-window-census.json` plus `ward-audit.jsonl`; require the pinned common `explicit-supersession` capture. Count only retained census-capture scope, not global deployment history.

- [ ] Write failing controls for valid typed captures, missing/duplicate/open/terminal rows, unsupported/incomplete census and missing required scenario.
- [ ] Run `node --test scripts/tests/daemon-window-terminals.test.mjs`; expected missing implementation failure.
- [ ] Implement capture reader and reusable original-ZIP replay; negative rows retain missing terminal evidence explicitly. Existing summary behavior stays compatible.
- [ ] Run terminal and retention controls; expected all pass, then commit.

### Task 2: Deterministic window classification

**Files:** Create `scripts/lib/daemon-window-ledger.mjs` and `scripts/tests/daemon-window-ledger.test.mjs`.

**Interfaces:** `validateWindowDeclaration(value)` validates exact 30-day declaration, repository/workflow/07:47 schedule, inventory SHA-256, fixed governing metric and explicit per-OS source series. `buildWindowLedger(declaration, censusRuns, verifiedRecords, serverObservedAt)` returns fixed 90 OS rows, per-journey rows, series/event/attempt/missing/arrival counts and engineering qualification. Source inventory is the committed pinned file; declaration must bind its digest.

- [ ] Write failing tests for all-positive synthetic control, one failure/missing day/OS, retries, manual substitution, duplicate schedule slots, late arrivals, omitted census runs, changed source/profile/lock, invalid dates/declarations and unelapsed server time.
- [ ] Run `node --test scripts/tests/daemon-window-ledger.test.mjs`; expected missing implementation failure.
- [ ] Implement deterministic classification with no inferred success or filtered denominator. A changed accepted series requires an explicit matching declaration entry; publish separate series metrics.
- [ ] Run the same controls; expected all pass, then commit.

### Task 3: Read-only census collector and evidence packet

**Files:** Create `scripts/qualify-daemon-window.mjs`, `scripts/tests/daemon-window-collector.test.mjs` and `docs/testing/native-daemon-window.md`.

**Interfaces:** CLI takes explicit declaration JSON, native archive root and new output directory. Fetch `repos/OpenCoven/coven-threads/actions/workflows/daemon-reliability.yml/runs` for the date range without filtering conclusions; save every HTTP response including Date, paginate/recheck, replay archive bytes, build ledger and write prose/table packet plus file index. No CLI clock override. Original provider fixtures remain test-only.

- [ ] Write failing controls for paginated stable census, changed pages, 1,000-result bound, missing/contradictory HTTP Date, unindexed/mutated archives and existing-output refusal.
- [ ] Run `node --test scripts/tests/daemon-window-collector.test.mjs`; expected missing implementation failure.
- [ ] Implement collector with indexed failure output and exclusive publication, and document declaration/source acceptance, denominator, terminal scope, actual-time limits and rollback.
- [ ] Run focused/all Node suites, staged privacy/secrets, action/pin/diff checks and required pinned repository gates. Obtain one fresh whole-branch review; fix material findings with failing controls and rerun the suite. Publish a draft stacked PR with exact tree and retained first-failure receipts.
- [ ] Keep actual collection, accepted declaration, three-OS execution and 30 elapsed days explicitly unproven until their real receipts exist. No tooling success closes #111 or any human gate.
