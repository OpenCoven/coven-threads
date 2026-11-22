# Retaining native daemon CI evidence

This collector implements the native evidence-retention tooling slice of [#110](https://github.com/OpenCoven/coven-threads/issues/110), using [#107 / PR #108](https://github.com/OpenCoven/coven-threads/pull/108)'s exact-attempt artifacts. It is a read-only GitHub Actions evidence consumer. Coven still owns runtime effects and audit persistence; this archive is not another runtime audit store, authority service or human acceptance record.

## Collect one completed attempt

Use an existing authenticated GitHub CLI with permission to read the repository's Actions metadata and artifacts, Node 24.22.0, and Python 3.11+. The CLI reads REST endpoints through `gh api`; it never dispatches, retries, merges, changes credentials or modifies GitHub state.

```bash
node scripts/retain-daemon-evidence.mjs RUN_ID ATTEMPT ARCHIVE_ROOT
node scripts/retain-daemon-evidence.mjs --summary ARCHIVE_ROOT
```

Use a dedicated archive directory outside tracked source. Both run ID and attempt are required; there is no implicit latest attempt or source pin. Collection accepts only a completed `.github/workflows/daemon-reliability.yml` run in `OpenCoven/coven-threads`, with the three explicit OS jobs and exact source/attempt identity. Metadata is fully paginated and rechecked after collection. Running, foreign, ambiguous, duplicate or changed identities refuse collection. A cancelled run that never started can retain a negative observation with no start time.

The archive identity is `RUN_ID/ATTEMPT`. Original run/job/artifact REST responses and downloaded ZIP bytes are retained, along with extracted evidence, a machine-readable `record.json`, a prose/table `packet.md` and a SHA-256 file index. Collector source-file hashes, actual Node and Python versions are recorded. Re-ingesting identical bytes is idempotent. Conflicting bytes never overwrite the original attempt; the separate collection is preserved for investigation. A failed metadata read leaves an indexed `.collect-*` snapshot outside canonical attempt identities. Its presence is not evidence of a running collector.

## Qualification and failure accounting

Job metadata remains authoritative for whether the job succeeded, failed, timed out, was cancelled, was skipped or is missing. A missing/expired artifact or absent digest cannot qualify. Original ZIP bytes must match GitHub's reported SHA-256; CRC, path, size, regular-file and collision checks run before extraction. Traversal, absolute/drive paths, backslash aliases, symlinks, duplicate/case-colliding members and file/directory collisions are refused. Extraction is bounded to 20,000 members, 512 MiB uncompressed total and 64 MiB per member. Original file-index hashes must match every extracted file, without missing or extra evidence.

A passing observation additionally requires matching run/attempt/source/platform/event, the reviewed Coven pin, the existing daemon Rust/Cargo 1.95.0 tuple, Node 24.22.0, all four actual commands and nonzero unfiltered passing counts, available executable identities, the guarded observation and matching original stdout/target receipts. All source-bound platform scenarios and manifest/JUnit/request/response/log captures are requalified. `macos`/`windows` fixture labels are compared with the producer's actual `darwin`/`win32` platform labels explicitly. Observation receipts cannot be replayed across attempts merely because the source is unchanged.

Failed jobs and corrupt, missing or contradictory evidence remain unqualified rows with their original metadata and available ZIP bytes. A later `retry-passed` report can record successful execution on that later attempt, but cannot replace or qualify attempt 1. The summary keeps scheduled and manual observations separate, identifies missing original attempts and reports later successful execution after an unqualified first observation. That is evidence of recovery on a rerun, not a causal diagnosis that a test was flaky.

Summary generation verifies each canonical archive's file index and metadata identity. Positive execution/qualification claims are replayed from original digest-checked ZIP bytes, rather than trusted from `record.json` alone. Unknown archive identities, duplicate attempts, duplicate OS rows, contradictory source scope or modified evidence refuse the summary. Incomplete/conflicting `.collect-*` snapshots are counted separately and cannot enter success denominators.

## Limits and continuation

The summary reports observed records only. It does not infer missing schedule slots, silently omit an expected OS/day, calculate a qualifying 30-day rate, shorten elapsed time or select a favorable denominator. [#111](https://github.com/OpenCoven/coven-threads/issues/111) owns the declared UTC window, expected opportunities, governing denominator and at-least-99.5-percent first-attempt target. Scope changes retain explicit source/tool/platform tuples rather than transferring acceptance between heads.

GitHub's native matrix artifact retention is configured for 45 days. That setting is not proof that bytes were downloaded or independently retained for an actual window. This collector provides a durable archive command; its invocation cadence and real native ingestion still need operational evidence. There are no real native observations until the matrix runs. Metadata snapshots rely on the authenticated REST collection path and later live provenance checks; offline file hashes alone do not authenticate a fabricated API response.

Core Nextest telemetry remains the distinct `Nextest telemetry (JUnit, fail on flaky)` job in `ci.yml`, with its existing retry/fail-on-flaky semantics and artifact identity. This native collector refuses the core CI workflow and does not count Nextest as daemon acceptance. The separate [core Nextest retention profile](nextest-evidence-retention.md) consumes existing original CI artifacts without broadening this native collector. The producer's generic Nextest artifact/14-day retention remains unchanged; durable collection cadence and actual retained-day coverage require operational evidence. Measured coverage remains #109. No workflow dispatch, merge, required-check promotion, deployment, release or Phase-5 human decision follows from these tooling results.

Rollback is a reviewed revert of this additive collector/tests/docs slice. Preserve original run histories, archive bytes, failure snapshots and the matrix pin/workflow. Do not delete or rewrite conflicting evidence to make a trend appear complete.
