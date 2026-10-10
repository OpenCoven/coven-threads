# Automation Authority Profile v1.0.0

Portable artifacts for `OpenCoven/coven-threads#29`.

- `schemas/` — closed JSON Schema 2020-12 wire contracts.
- `validator.mjs` — Node-core strict parser, canonical digest/signature
  verifier, reference policy, lifecycle, replay, proposal, dispatch, and
  evidence-read validator.
- `manifest.json` — exact positive/negative vector inventory and expectations.
- `vectors/` — 130 signed conformance vectors across all 18 issue categories.
- `keyring.json` — synthetic principal, protected-owner, auditor, and Threads
  authority public keys used only by the vectors.
- `schemas/keyring.schema.json` — closed role/identity contract for the
  portable public keyring.
- `run-vectors.mjs` — read-only dependency-free runner.
- `tools/generate-vectors.mjs` — maintainer tool that replaces the synthetic
  vector keys/signatures; it is not invoked by conformance runs.

Run:

```sh
node --test profiles/automation-authority/v1/tests/*.test.mjs
node profiles/automation-authority/v1/run-vectors.mjs
```

## Rust port

`coven_threads_core::automation_authority` ports the evaluation half of
`validator.mjs` for the trusted daemon to call in process:
- the strict parser and canonical digests;
- request validation and adoption;
- `evaluateAuthorization`;
- decision validation, bundle verification and consumption.

For the same inputs it reaches the same result and the same first error code.
Signatures go through a caller-supplied `SignatureVerifier`, so the core keeps
no crypto backend.

- `cargo test -p coven-threads-core --test automation_authority_vectors` runs
  the 44 vectors those operations cover: `strict_parse`, `validate_request`,
  `request_adoption`, `evaluate_request`, `verify_decision` and
  `decision_consumption`. The approval, lifecycle, consumption-snapshot,
  proposal, evidence-read and dispatch operations are not ported yet.
- `scripts/automation-authority-differential.mjs` runs both validators on
  about 39,000 mutants of those vectors and requires the same first code, or
  the same canonical decision. Mutants are re-signed with fresh keys, and
  signature tampering is included. It needs the batch example:

  ```sh
  cargo build -p coven-threads-core --example automation_authority_batch
  node scripts/automation-authority-differential.mjs \
    target/debug/examples/automation_authority_batch
  ```

`validator.mjs` stays the reference. The port deliberately differs in two
places:
- it refuses a `__proto__` object key, which JavaScript drops silently;
- it refuses nesting beyond 2,048 levels as `json_invalid`, where the
  reference's recursive parser throws a non-profile `RangeError` near 1,750.

Normative semantics, preimages, ownership, and integration obligations are in
[`specs/AUTOMATION-AUTHORITY-PROFILE-v1.md`](../../../specs/AUTOMATION-AUTHORITY-PROFILE-v1.md).
