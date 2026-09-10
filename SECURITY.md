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

## Repository secret scanning and privacy rollout

The `Secret scanning` job in `.github/workflows/ci.yml` runs on
pull requests and pushes to `main`. It needs no private stores, Beads database,
credentials, or downstream checkout. Third-party Actions retain immutable
commit pins and the existing explicit Rust toolchain parity checks.

**Separate privacy rollout (#39, draft #40):** the proposed privacy policy rejects a
pre-existing runtime-path example in the frozen Phase-5 specification. This is
a policy/example conflict, not a claim of exposed personal data. Privacy
enforcement is not enabled in this secret-scanning job; its checker and
synthetic regressions are included for reproducible policy evaluation.
Draft #40 remains blocked pending a maintainer-approved policy clarification
or a change-controlled example correction. This independently deployable
secret-scanning slice neither waives that requirement nor claims privacy
acceptance. No exemption or frozen-spec edit is included.

From the repository root:

```bash
bash scripts/install-gitleaks.sh /tmp/threads-guard-bin
export PATH="/tmp/threads-guard-bin:$PATH"
node --test scripts/tests/*.test.mjs scripts/tests/secret-guard.integration.mjs
node scripts/secret-guard.mjs
# Manual proposed-policy evaluation; currently blocked as documented above.
node scripts/privacy-guard.mjs
```

The installer supports Linux x86-64 (CI) and macOS arm64. Gitleaks 8.30.1
archives are verified against SHA-256 digests committed in the installer
before extraction. Updating the version requires reviewing and updating both
the version file and digests; the guard rejects version skew.

**Scope:** the manual privacy checker scans every stage-zero Git index blob and filename, not
unstaged or untracked files. Clean checkout CI therefore covers the submitted
tree. Stage intended changes before local scanning. Secret scanning covers
the same index via a permission-restricted temporary snapshot plus every
commit reachable from `HEAD` using Gitleaks built-in rules. It rejects shallow
history; it does not inspect other refs, unreachable objects, remote stores,
live familiar workspaces, or a local Beads database. Already tracked exports
are public repository input and receive no exemption. Temporary snapshots are
removed on normal completion and handled failures.

The privacy categories are conversation/session keys, numeric messenger IDs,
literal Unix/macOS home directories (including bare home paths), internal
Coven/OpenClaw runtime paths, E.164-shaped phone numbers (8-15 digits), and
invite/handoff/tailnet URLs containing `token`. Use `FAMILIAR_ROOT` or angle
bracket placeholders instead of personal examples. Numeric phone heuristics
can produce false positives; they are not a full PII classifier.

No file/line exemption, baseline, or inline `gitleaks:allow` /
`guard-scan-allow` bypass is provided. Privacy scans bytes regardless of
extension or NULs. Symlinks, submodules, unresolved index stages, unreadable
input, oversized Git command output (64 MiB), and scanner failures fail
closed rather than silently skipping input. Run from the repository root.

**Diagnostics:** privacy emits only fixed category names and file counts;
secret scanning emits only clean/blocked status for index and history.
Scanner stdout/stderr is discarded even with redaction enabled, because raw
diagnostics may reveal filenames or surrounding text. No reports or scanned
content are uploaded. Exit 1 means findings; exit 2 means unsupported input
or a tooling error. Investigate privately; do not paste matching content into
public issues, logs, or fixtures.

**Limits:** a green result is pattern coverage, not proof that no secret or
personal data exists. Gitleaks retains its upstream default rules and default
false-positive exclusions; binary formats, archives (not expanded), unknown
credential formats, arbitrary prose, and encoded identifiers may evade these
guards. Privacy is index-only, not a historical privacy audit. The guards do
not certify daemon authority, deployed behavior, branch protection, or human
review. Pull-request code can modify CI, so maintainer review and required-check
policy remain separate controls.

Policy provenance: adapted the categories and separate default-secret scan
from `OpenCoven/coven-memory` at
`eb542a598c93775f9d411c03de22a894c17e8527`
(`scripts/privacy-patterns.sh`, `scripts/guard-scan.sh`, `.gitleaks.toml`,
`.gitleaks-default.toml`, `.gitleaks-version`, and `SECURITY.md`), and consulted
`OpenCoven/coven` at `efaf948a8a6f8b9abfe5bca7b1e661cb0cebdc01`
(`scripts/check-coven-privacy.py`, `scripts/check-secrets.py`). The small
Node implementation uses this repository's existing built-in test runner
instead of importing Coven's much larger repository-specific Python secret
heuristics. It intentionally does not copy upstream mutable Action refs,
unchecked downloads, match-printing, or broad exemptions.

Roll back this CI slice by reverting its workflow job, scripts, configuration,
and this section; no Rust API, audit schema, data migration, or downstream
runtime change is involved. Tracking and rollout decisions remain in #39
(`threads-t6t`), under #31; defining a job does not make it a required check.
