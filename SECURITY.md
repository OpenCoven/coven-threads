# Security policy — coven-threads

## Reporting vulnerabilities

Report security issues by direct message to Val (@BunsDev on GitHub) — do not open a public issue.

## What this repo enforces

`coven-threads` is a validator library. Its correctness is a security property of the whole OpenCoven stack:

- If `coven-threads` incorrectly returns `Permit` for a mutation that violates a thread, the daemon has been fooled and a familiar's identity surface can be silently mutated. This is the failure mode the whole layer exists to prevent.
- If `coven-threads` incorrectly returns `Reject` for a legitimate mutation, the familiar's surface is falsely locked and requires manual repair. This is a UX failure, not a security failure, but it must be surfaced clearly.
- Panics inside the validator MUST be caught by the daemon and treated as `Reject` (fail-closed) with a diagnostic.

## Trust model

Read `../coven/docs/SAFETY-MODEL.md`. `coven-threads` inherits the daemon's trust model and adds only *typed protected surface* validation on top. It does not open new network surfaces, does not accept remote configuration, does not hold secrets.

## Reproducibility

Every accepted `Permit` result MUST be reproducible from the inputs: the request, the weave state at request time, and the strand-verification results. Non-reproducible acceptances are bugs.
