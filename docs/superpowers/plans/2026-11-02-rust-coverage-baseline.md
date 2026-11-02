# Rust coverage baseline implementation plan

> **For agentic workers:** Use superpowers:executing-plans for inline execution and one fresh whole-branch review.

**Goal:** Deliver #109's source-bound measurement in the existing informational coverage job, preserving original LCOV and failed measurements before proposing a floor from actual hosted results.

**Architecture:** A strict parser supports the pinned LLVM LCOV exporter subset and recomputes line/function/branch summaries. A runner binds measurement to a clean committed source, exact tools, command, features and file hashes, retains original output, and emits numeric JSON to hosted logs. No threshold is enforced.

**Tech Stack:** Node 24.22.0 standard library, Rust 1.88.0 with llvm-tools-preview, cargo-llvm-cov 0.9.1, existing SHA-pinned Actions.

**Spec:** https://github.com/OpenCoven/coven-threads/issues/109; existing `.github/workflows/ci.yml` coverage command; pinned LLVM exporter and cargo-llvm-cov documentation linked by the operator contract.

## Global constraints

- Preserve `cargo llvm-cov --locked --workspace --lcov` semantics, default features/profile/exclusions and informational status.
- Retain original LCOV, command log and unqualified receipt on failure; never overwrite an existing output directory.
- Bind every source record to a tracked regular Rust file inside the exact clean checkout. List tracked files absent from LCOV explicitly.
- Require positive global line/function denominators and covered counts; reject malformed, duplicate, inconsistent, incomplete or unsupported records.
- No daemon pin, runtime authority, audit schema, required-check, workflow-trigger, release or human-gate changes.

## Review focus

- Commas in LLVM function names must not truncate identities.
- Zero-denominator source files remain explicit with null percentages, not invented complete coverage.
- Dirty or changed source and inherited instrumentation/feature overrides must refuse qualification.
- Tool/setup failures and partially written LCOV remain unqualified with original bytes preserved.
- A change in file set, configuration, features, tools or platform must change the comparison scope.

### Task 1: Strict source-bound LCOV qualification

**Files:** Create `scripts/lib/rust-coverage.mjs` and `scripts/tests/rust-coverage.test.mjs`.

**Interfaces:** `parseLcov(text, {root, sources})` consumes source entries `{path, sha256, lines}` and returns sorted per-file and aggregate `{lines, functions, branches}` covered/total/percent values plus unreported source paths. Export `sha256(bytes)` for the runner.

- [ ] Write tests for valid multi-source LLVM output, comma names, zero-denominator files, missing/duplicate/conflicting records, unsafe counters, foreign paths and source-line bounds.
- [ ] Run `node --test scripts/tests/rust-coverage.test.mjs`; expected first failure is the missing module.
- [ ] Implement strict parser and recomputed summaries without silently discarding records.
- [ ] Run the same command; expected all controls pass, then commit parser and tests.

### Task 2: Source/tool measurement and informational CI receipt

**Files:** Create `scripts/measure-rust-coverage.mjs`, `scripts/tests/rust-coverage-runner.test.mjs`, `docs/testing/rust-coverage-baseline.md`; modify `.github/workflows/ci.yml`.

**Interfaces:** Runner CLI takes one new output directory. Export `measureCoverage({root, output, run, env})` with an injectable subprocess provider for deterministic tooling controls; production uses real child processes. Consume Task 1's parser and hash helper. Return qualified receipt only after command success, valid LCOV and identical before/after source snapshots. Capture actual rustc/Cargo/LLVM/cargo-llvm-cov versions, resolved dependency features and source/config digests. Emit a stable comparison-scope hash excluding changed source content but including the exact file set/config/tool/feature/platform identities.

- [ ] Write failing tests using disposable committed synthetic repositories: successful measurement, wrong tools, failing command/partial output, changed source, inherited overrides and repeated-output refusal. Add workflow assertions for pinned coverage tool/Node, informational status and exact-attempt retention.
- [ ] Run `node --test scripts/tests/rust-coverage*.test.mjs`; expected new tests fail before runner/workflow implementation.
- [ ] Implement runner, preserve original bytes and failure receipt, update existing job with pinned Node/tool and exact-attempt 45-day artifact. Record post-upload artifact ID/digest in hosted logs separately from the pre-upload receipt.
- [ ] Document denominators, default exclusions, unreported sources, compatible-scope comparison, original-byte authentication, missing local tools and floor-decision boundary.
- [ ] Run focused/all Node tests, staged privacy/secrets, pin/lint checks and full pinned repository gate. Expected all pass; actual coverage requires hosted execution because local coverage tools are unavailable.
- [ ] Commit, obtain fresh whole-branch engineering review and address material findings in one failing-then-passing fix pass. Publish a draft PR with exact tree receipts, inspect hosted numeric measurement and retain every first failure. Propose a floor only after a qualified measurement; no merge or enforcement activation is included.
