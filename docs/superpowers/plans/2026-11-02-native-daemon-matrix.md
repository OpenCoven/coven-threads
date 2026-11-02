# Native daemon matrix implementation plan

**Goal:** Implement the native three-OS matrix acceptance contract in [#107](https://github.com/OpenCoven/coven-threads/issues/107), the first slice of the native-reliability goal under #31.

**Architecture:** Keep the existing required compatibility and advisory latest-main workflows unchanged. An additive scheduled workflow selects the reviewed manifest pin on three native runners and invokes a reliability wrapper around the existing guarded Cargo overlay. The wrapper retains per-target process results, actual counts, executable hashes and source-bound journey evidence; invalid or absent evidence fails qualification.

**Tech stack:** Node standard library, Python standard-library XML parsing, Cargo/libtest, SHA-pinned GitHub Actions.

**Specification:** #107, `docs/testing/e2e-contract.md`, `e2e/compatibility.toml`, and the unchanged Phase-5 semantics.

## Constraints and review focus

- Keep Coven `226bfcc89ff6cad4bc9cc9618dad6fea970ecf58` and its committed Threads pin unchanged; verify both before the disposable overlay.
- Run all four existing targets with controlled time, no filtering, no retry, positive counts and zero failed/ignored/filtered cases on each OS. Each OS completes independently.
- The current Threads dependency, immutable source, overlay config and resolved lock are guarded by the existing canary before and after successful execution. An unsuccessful run cannot qualify.
- Do not conflate artifact-bearing journeys with companion test counts. Authenticate manifest/JUnit pairs separately and retain contradictory or incomplete originals as failed evidence.
- Record actual platform/tool/run/attempt/source/executable identities and hash retained artifacts; reject symlinks and missing files. Rerun success cannot qualify as first-attempt success.
- Raw private homes, credentials and real declarations are outside scope. Sanitize runner/checkout/home paths in process output; synthetic daemon fixture evidence retains its existing sanitation.
- Workflow dispatch, merge, required-check promotion, release and human acceptance are separate gates. No change activates one through an issue-closing keyword.

## Task 1: Evidence qualification

- [ ] Add failing tests for libtest counts and source-bound manifest/JUnit qualification, including empty, partial, failed, skipped, duplicate and mismatched evidence.
- [ ] Implement `scripts/lib/daemon-reliability-evidence.mjs` with strict count parsing, Python XML verification and a regular-file SHA-256 index.
- [ ] Verify focused negative and positive controls. Synthetic parser controls are tooling tests, never daemon execution receipts.

## Task 2: Native runner

- [ ] Add failing tests for four-target process execution, failure retention, no retry, per-target commands/counts and binary identities.
- [ ] Implement `scripts/run-daemon-reliability.mjs`, reusing `readCompatibility` and `runCanary`; record outputs for each Cargo test target and continue remaining targets after a test failure.
- [ ] Validate manifests against actual current Threads/Coven commits, resolved lock, active override and native platform. Retain a failed receipt on setup/process/evidence failure.
- [ ] Run the focused suites and complete repository Node suite.

## Task 3: Workflow and operator contract

- [ ] Add the nightly Linux/macOS/Windows workflow with fixed runner families, explicit shell, no matrix fail-fast, exact attempt/OS artifact names and unconditional evidence upload.
- [ ] Include the workflow in pinned-action/toolchain validation and document qualification, failure accounting, activation and rollback.
- [ ] Run staged privacy/secrets guards, bootstrap and required fast/full checks on the pinned toolchain. Preserve any unavailable-tool or infrastructure failures honestly.
- [ ] Obtain a fresh whole-branch engineering review, address material findings, and deliver a concrete PR and checkpoint. Hosted native execution remains outstanding until actual run artifacts prove it.

## Later goal slices

After matrix delivery, open separately scoped children for measured coverage, flake retention and the 30-day first-attempt ledger. Retention must support that window; missing days/OSes and reruns remain visible. None of these future outcomes is completed by landing this first workflow.
