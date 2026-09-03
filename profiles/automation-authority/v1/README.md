# Automation Authority Profile v1.0.0

Portable artifacts for `OpenCoven/coven-threads#29`.

- `schemas/` — closed JSON Schema 2020-12 wire contracts.
- `validator.mjs` — Node-core strict parser, canonical digest/signature
  verifier, reference policy, lifecycle, replay, proposal, dispatch, and
  evidence-read validator.
- `manifest.json` — exact positive/negative vector inventory and expectations.
- `vectors/` — 97 signed conformance vectors across all 18 issue categories.
- `keyring.json` — synthetic principal, protected-owner, auditor, and Threads
  authority public keys used only by the vectors.
- `run-vectors.mjs` — read-only dependency-free runner.
- `tools/generate-vectors.mjs` — maintainer tool that replaces the synthetic
  vector keys/signatures; it is not invoked by conformance runs.

Run:

```sh
node --test profiles/automation-authority/v1/tests/*.test.mjs
node profiles/automation-authority/v1/run-vectors.mjs
```

Normative semantics, preimages, ownership, and integration obligations are in
[`specs/AUTOMATION-AUTHORITY-PROFILE-v1.md`](../../../specs/AUTOMATION-AUTHORITY-PROFILE-v1.md).
