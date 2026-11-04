# Native daemon evidence retention plan

**Goal:** Deliver #110's immutable native CI evidence archive and first-attempt/rerun accounting under #31. Keep #108's frozen matrix candidate untouched. This tooling prepares the actual 30-day ledger; it cannot establish elapsed days or authorize execution.

**Architecture:** A read-only collector uses authenticated `gh api` REST reads for one completed native workflow attempt. It retains original run/job/artifact metadata and artifact ZIP bytes, validates ZIP digest/CRC/paths and the matrix's file hash index, then checks source/attempt/OS/result bindings. Publication to an archive identity is atomic, immutable and idempotent for identical bytes; conflicts refuse. Failed, cancelled, skipped, missing and expired evidence remain explicit rows.

**Constraints:** The native workflow, three OS names, daemon pin, four targets and scenario inventory are explicit. No runtime audit store, workflow dispatch, new credentials, profile mutation, merge, promotion or acceptance is introduced. Existing core Nextest telemetry stays a separate profile and is not counted as a daemon result. A recovered rerun is an observation, not a causal flake diagnosis or first-attempt success. Archive output belongs outside tracked protocol source.

## Task 1: Source and attempt classification

- [ ] Write failing tests for exact run/job/OS/artifact identity, zero/failed/missing evidence and reruns.
- [ ] Implement pure native-attempt validation with three expected OS rows, explicit nonqualifying states and exact source/command/count/digest checks for passing rows.
- [ ] Verify valid synthetic controls and every refusal. These are tooling regressions, never native execution receipts.

## Task 2: Immutable collection

- [ ] Write failing tests for digest mismatch, ZIP traversal/symlinks/duplicates/case collisions/CRC, file-index mismatch, identical re-ingestion and conflicting archive identity.
- [ ] Implement the read-only REST collector with complete paginated metadata and original ZIP retention. Safely extract only validated regular files, qualify manifest/JUnit/capture evidence and retain canonical metadata on failures.
- [ ] Record command/source/tool identity and an artifact index for the archive. Validate source-scope changes and keep arrival times from authenticated run metadata.

## Task 3: Evidence and delivery

- [ ] Provide a deterministic summary of observed first attempts, later attempts and recovered reruns without inventing missing schedule slots or a 30-day pass rate. Document the separate #111 window contract.
- [ ] Run focused and repository tests, staged privacy/secrets, action/pin checks and required Rust gates; preserve first failures. Obtain the engineering review required by the review skill and address material findings with red-to-green regressions.
- [ ] Produce a separate concrete reviewed PR and exact receipts; keep actual native ingestion, retained-day coverage and activation limitations explicit. No closure follows from parser success alone.
