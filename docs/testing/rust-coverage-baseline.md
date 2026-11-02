# Source-bound Rust coverage baseline

Issue [#109](https://github.com/OpenCoven/coven-threads/issues/109) owns measured coverage under #31. The existing `Coverage baseline (informational)` job stays informational and uses the same workspace/default-feature/default-profile `cargo llvm-cov --locked --workspace --lcov` measurement. Node is pinned to 24.22.0, Rust to 1.88.0 and cargo-llvm-cov to 0.9.1. The action remains SHA-pinned with its checksum verification enabled. No regression floor or required check is activated.

## Measurement contract

Run from a clean committed checkout with the pinned tools and `llvm-tools-preview` installed:

```bash
node scripts/measure-rust-coverage.mjs /absolute/new/output-directory
```

The output directory must be new and either outside the checkout or git-ignored. Repeated runs cannot replace an earlier directory. The runner records commit/tree, tracked Rust source hashes and lengths, configuration/lock hashes, actual Rust/Cargo/LLVM/coverage-tool/Node versions, LLVM executable hashes, platform, command and resolved dependency feature sets. It rejects inherited instrumentation/target/profile/tool overrides and external Cargo configuration. It checks source cleanliness and identity before and after execution and qualification. These checks identify observed source; they are not a security boundary against an actor concurrently changing and restoring the filesystem.

`measurement.json` starts unqualified. A command failure, signal, missing tool, incomplete export or source mismatch leaves it unqualified with the original exit status where available. Original `measurement.log`, `export.log`, `probes.json`, any partial `lcov.info` and `llvm-summary.json` remain intact and receive byte lengths and SHA-256 entries. Probe output/status/signal/spawn errors and earlier successful tool identities survive a later probe failure. A runner/setup failure before initialization may produce no receipt; the GitHub job remains the authoritative negative observation. There is no retry in this wrapper.

Qualification requires detailed source-bound records, positive aggregate covered/total line and function counts, consistent summaries and complete terminal records. The parser supports the detailed [LLVM LCOV exporter](https://github.com/llvm/llvm-project/blob/llvmorg-20.1.5/llvm/tools/llvm-cov/CoverageExporterLcov.cpp) format: SF, FN, FNDA, DA, BRDA, FNF/FNH, LF/LH, BRF/BRH and end_of_record. It recomputes summaries, preserves commas in function names and refuses unknown tags, duplicate/conflicting entries, unsafe counters, foreign/untracked sources and source-line overflow. It deliberately refuses other LCOV dialects, including summary-only exports; it does not silently approximate them.

## Denominators and exclusions

Line coverage is the number of DA records with a positive execution count divided by the number of DA records. LLVM's FNF/FNH count function groups; FN/FNDA enumerate instantiations, so generics may have more FN entries than FNF. The runner additionally exports `cargo llvm-cov report --locked --workspace --json --summary-only --output-path OUTPUT/llvm-summary.json` from the same measured profile without rerunning tests. It requires matching LCOV/JSON source sets and line, branch, function-group and instantiation summaries. The parser recomputes line/branch/instantiation counts and authenticates the tool's function-group summary against that paired export; it never guesses grouping from start lines. Known JSON summary versions 2.0.1 and 3.0.1 are supported. Functions and instantiations remain separate metrics.

Branch counts are recorded only where complete branch records and matching summaries exist; they are not a branch-coverage promise for Rust code. Zero-denominator files have a null percentage. A missing branch scope makes the aggregate branch metric null. Global zero line/function denominators or zero covered counts refuse qualification. The feature query uses `cargo metadata --locked --offline --format-version 1 --filter-platform HOST` and records only resolved nodes; it cannot require downloading dependencies for untested platforms.

The [pinned cargo-llvm-cov default filename exclusions](https://github.com/taiki-e/cargo-llvm-cov/blob/v0.9.1/README.md#exclude-file-from-coverage) remain enabled. These exclude dependency/toolchain/build sources and paths such as tests/examples/benches and test-named files. Doctests are not requested. Inline tests in ordinary source files may remain in the reported denominator. All tracked Rust files absent from LCOV are listed as `unreported`; absence is not classified as fully covered, zero covered or proof of exclusion. Only explicitly reported source is measured. This baseline does not measure Node tooling, native daemon journeys, physical durability or human acceptance.

`scopeSha256` binds command, default-feature/profile/exclusion settings, tool/platform identity, source and reported path sets, configuration/lock hashes and resolved dependency features. Compare percentages only when both measurements qualify and their scope objects and hashes match. Changes to Rust file contents may then be compared across identified commits; changing source path sets, lock/configuration, tools, platform/image or reported sets creates a new baseline series. Reviewers must also examine changes in total denominators; a matching scope does not prevent source edits from deleting executable lines.

## Hosted evidence and floor decision

The job prints qualified numeric totals, exact commit/tree, scope hash and original LCOV/log hashes to its logs. It uploads the original output directory under `rust-coverage-RUN_ID-ATTEMPT` for 45 days and prints the resulting artifact ID and GitHub-reported ZIP digest after upload. The pre-upload receipt cannot contain the digest of its own future ZIP. Retain the original ZIP and authenticate its bytes against GitHub metadata before treating a downloaded copy as archived proof; retain receipt/source/run/job/attempt identities together. A successful upload or informational job conclusion alone does not establish measurement qualification. Inspect `measurement.json.qualified` and the actual command/receipt.

A qualified hosted measurement supplies the initial numeric baseline. Propose a concrete line/function floor only from those observed values, with exclusions, denominator, scope and expected regression/cost tradeoff recorded in the issue or reviewed evidence packet. There is no fabricated default floor. Enforcement and required-check promotion require their separate maintainer decision. Until a real measurement and original artifact authentication exist, numeric coverage and archival qualification remain unknown.

Rollback is a reviewed revert of this additive job wrapper, parser and documentation while preserving original measurements and failures. The stable daemon compatibility pin and separate latest-main canary are untouched.
