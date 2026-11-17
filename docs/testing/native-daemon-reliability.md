# Native daemon reliability

The scheduled matrix implements [#107](https://github.com/OpenCoven/coven-threads/issues/107), the first native-reliability slice under [#31](https://github.com/OpenCoven/coven-threads/issues/31). It exercises the real Coven daemon on Linux, macOS and Windows using the reviewed full-SHA pin from `e2e/compatibility.toml` and the actual current Threads checkout. It leaves the required pull-request compatibility check and scheduled latest-main canary separate. Workflow-file delivery is not evidence that the native runs have passed.

## Qualification

Each OS runs all four manifest targets once with `threads-test-clock`, locked Cargo resolution and no test filter. Targets run separately so a failure does not suppress another target's first result. Matrix fail-fast is disabled. A target qualifies only if its process succeeds, exactly one complete libtest summary reports positive passed counts and zero failed/ignored/measured/filtered cases, and Cargo identifies its executable. The wrapper also requires the actual daemon executable identity. Counts are per-target test counts; they are not counts of artifact-bearing journeys.

The existing canary proves that the selected daemon uses the exact current Threads package before and after execution and checks committed sources, the recorded Cargo config overlay and resolved lockfile for drift. The wrapper verifies the daemon's original committed Threads pin before creating that disposable overlay. The matrix keeps the existing daemon Rust/Cargo 1.95.0 tuple, separately from Threads' Rust 1.88.0 minimum. Node is 24.22.0; Python 3.13 supplies standard-library TOML/XML parsing. Actual tool versions, OS, architecture and kernel are recorded.

Every captured journey must have a passed, setup-complete manifest with captured fixture evidence, matching Coven/Threads commits, the resolved lock digest, native platform, active local override and a clean Threads source. Its matching JUnit must parse as one actual passing test with matching scenario identity and no failure/error/skip element. Missing, empty, partial, contradictory or mismatched evidence fails qualification. The downstream checkout is intentionally modified only by its recorded Cargo overlay; its dirty flag is not interpreted as unmodified committed bytes. Existing source/config/lock guards define that exception.

`e2e/native-daemon-scenarios.json` enumerates the pinned harness's exact scenario inventory: 86 common scenarios, 25 additional Unix scenarios and one additional Linux scenario. It binds the scenario-generating source files by SHA-256, including each loop and helper expansion. Linux therefore requires 112 distinct pairs, macOS 111 and Windows 86, with no missing, duplicate or extra scenario. Linux's 112 agrees with the retained required-pin receipt; the other counts are source-derived expectations pending native execution, not claimed passing results. A new daemon pin or changed harness source requires a reviewed inventory update. Each pair also requires valid captured request/response JSON and a nonempty daemon log. Nested JUnit suites or additional cases are refused.

## macOS runner candidate

The candidate in [the #116 decision packet](../reviews/2026-11-05-macos-runner-viability.md) selects the standard `macos-15-intel` physical runner while preserving `macos-15` as the logical macOS evidence slot in job names, artifact names and the ledger. It still contributes exactly one of three daily OS opportunities. Retained GitHub job labels identify the physical runner, and the qualifier requires `x64` for `macos-15-intel` or `arm64` for historical `macos-15` jobs; missing, contradictory or ambiguous labels refuse qualification. The exact report architecture remains part of the accepted source profile. An Intel pass establishes no ARM execution coverage. Original ARM failure archives remain replayable without rewriting their bytes or source profiles.

## Retained evidence

Each OS/run/attempt receives a new artifact directory and unique uploaded artifact name. Evidence includes `observation.json`, `reliability.json`, `targets.json`, each target's separate stdout/stderr, Cargo preflight logs and the existing sanitized journey manifests/JUnit/request/response/failure bundles. Cargo compiler-artifact messages supply the actual daemon/test executable identities, which are SHA-256 hashed without uploading the executables. `artifact-index.json` hashes every other retained regular file; it excludes itself. Symlink or special-file evidence is refused. Checkout, runner evidence and home paths in process logs are replaced with placeholders; synthetic fixture evidence retains the daemon's own sanitation.

The runner never retries. A GitHub rerun retains a new attempt identity: even if execution passes, it records `retry-passed` and returns failure so the original failing first attempt cannot be turned into a qualifying green. Local execution without a GitHub attempt can supply local engineering results but cannot qualify as a hosted first attempt. A cancelled or setup-failed job with no evidence is not a pass; artifact upload is unconditional and missing files cause an upload failure. GitHub run/job metadata must also be retained by the later ledger, because a runner terminated before the wrapper starts cannot write a receipt.

Artifacts are retained for 45 days so a subsequent 30-day ledger can still authenticate the full window. This does not establish a coverage floor, flake rate, 30-day reliability rate, physical power-loss guarantee, current-main daemon acceptance, installed-binary certification or Phase-5 human acceptance. The reviewed pin predates later durability repairs. Coverage measurement, flake-trend retention and the first-attempt ledger remain separately scoped outcomes of the active goal; no historical window is transferred to this new matrix.

## Execution and activation

For an authorized local experiment, use clean disposable Threads and Coven worktrees and a new evidence directory:

```bash
node scripts/run-daemon-reliability.mjs COVEN_CHECKOUT NEW_ARTIFACT_DIRECTORY
```

The Coven checkout must match the manifest's reviewed pin. The command appends a Cargo path override and resolves its lockfile in that disposable checkout; it does not restore the checkout for reuse. Retain the evidence, then discard only the experiment's own checkout through the applicable cleanup process.

The scheduled workflow becomes eligible after normal landing. Manual dispatch, merge authorization, required-check promotion, protected settings, release and deployment retain their separate gates. No dispatch or promotion is implied by this document or by a successful local test. Rollback is a reviewed revert of this additive workflow/wrapper/evidence tooling, preserving the existing required workflow, compatibility manifest, latest-main canary and original failure artifacts.
