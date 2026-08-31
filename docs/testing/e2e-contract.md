# Coven Threads end-to-end verification contract

**Status:** P0 implementation contract  
**Risk class:** R4 — identity/authorization/persistence boundary  
**Canonical compatibility pin:** [`e2e/compatibility.toml`](../../e2e/compatibility.toml)

## 1. Why this layer exists

`coven-threads-core` is intentionally a side-effect-free validator library. Its
unit and conformance suites prove typed gate, audit-schema, portability, and
fail-closed semantics in process. They do not prove that a real client request
crosses the Coven daemon boundary, invokes the current Threads checkout, and
produces the expected filesystem, SQLite, pending-proposal, scheduler, and
restart outcomes.

The missing proof is therefore a boundary-spanning journey:

```text
current coven-threads change
    -> real coven daemon process
    -> real supported request boundary
    -> coven_threads_core gate from this checkout
    -> ephemeral familiar workspace
    -> ephemeral coven.sqlite3 + pending state
    -> scheduler/replay/restart
    -> optional real Cave acceptance journey
```

This contract does not turn Threads into a daemon or UI repository. It defines
what downstream implementations must prove before a Threads change is treated
as compatible.

## 2. Canonical ownership

| Concern | Canonical owner | E2E responsibility |
| --- | --- | --- |
| Familiar identity and protected invariants | `familiar-contract` | Supply pinned normative fixtures; Threads must not redefine identity. |
| Protected mutation verdict and audit schema | `coven-threads` | Define typed inputs, verdicts, reason codes, audit constraints, and vectors. |
| Principal authentication, storage, staging, scheduling, apply, recovery | `coven` | Launch the process and own every effect. |
| Orchestration context and attempt lifecycle | `psyche` | Consume authority evidence; never manufacture a verdict. |
| Human rendering and approval forwarding | `coven-cave` | Render daemon truth; never become an approval authority. |

There is one audit store: `ward_audit` inside the daemon-owned
`coven.sqlite3`. The harness must not create a second audit truth.

## 3. Required topology

The required daemon suite lives in `OpenCoven/coven`, because that repository
owns the executable, transport, persistence, staging, and scheduler. A Threads
PR checks out both repositories and overlays the current Threads crate into the
pinned Coven checkout:

```toml
[patch."https://github.com/OpenCoven/coven-threads"]
coven-threads-core = { path = "../coven-threads/crates/coven-threads-core" }
```

Before executing any journey, the job must prove the override is active through
`cargo metadata` or `cargo tree`. Testing Coven's historical Git revision while
claiming to test the pull request is a hard failure.

Two downstream references are maintained:

1. **Required compatibility pin** — a reviewed full commit SHA from
   `e2e/compatibility.toml`. This is the stable pull-request gate.
2. **Latest-main canary** — a scheduled non-blocking run against `coven/main`.
   This detects ecosystem drift without making unrelated downstream movement
   nondeterministically break Threads pull requests.

## 4. Isolation and safety

Every run must allocate unique, disposable state:

- temporary `COVEN_HOME`;
- temporary familiar workspace;
- temporary daemon socket/port;
- temporary SQLite database;
- temporary pending/staging directory;
- synthetic principal, familiar, Ward, and content fixtures;
- deterministic run identifier included in every artifact.

The suite must never:

- read or modify a developer's real `~/.coven`;
- use production credentials or real familiar declarations;
- call `gate_protected_edits` directly in place of the daemon transport;
- bypass authentication, request adoption, staging, scheduler, or replay code;
- infer success solely from an HTTP status while ignoring disk and audit state;
- sleep on wall-clock time to establish correctness;
- upload secrets, private prompts, or personal identity content as artifacts.

Unknown state, unavailable evidence, an inactive override, or an ambiguous
fixture must fail the suite closed.

## 5. Required black-box journeys

The first required suite contains eight journeys. Each journey asserts the
public response and all applicable persisted effects.

### J1 — bounded authorized write

A correctly authenticated and bound principal submits a permitted write.

Required assertions:

- daemon returns the typed permit/applied disposition;
- the intended file changes exactly once and atomically;
- an authoritative validation row is appended;
- any applied-write row carries the expected previous/next hashes and byte count;
- no pending proposal remains.

Until the signed principal-authorization profile lands, the fixture must use the
strongest current daemon-owned authorization path and state that limitation in
its evidence. A client-supplied identity string is never sufficient proof.

### J2 — unsigned or unbound protected write

An unbound requester attempts a protected mutation.

Required assertions:

- request is rejected;
- protected bytes are unchanged;
- no proposal or approval route can later apply the write;
- one typed rejection audit row exists;
- protected content is not echoed in the error.

### J3 — out-of-band drift

After establishing a baseline, mutate a governed surface outside the daemon and
submit a conflicting edit.

Required assertions:

- the drift is detected from materialized bytes;
- the protected surface is not overwritten;
- any degrade-to-proposal result is explicitly non-executed;
- pending artifact, response, and audit row bind the same proposal/diff digest.

### J4 — identity invariant mutation and replay

Submit a candidate that violates a declared identity predicate, then exercise
both intake and delayed replay.

Required assertions:

- intake fails closed or stages only where the governing contract permits;
- deadline replay evaluates the same authoritative predicate set;
- changed or unavailable evidence cannot apply;
- descriptor text is never treated as authority.

This journey is the closure evidence for `threads-okc`.

### J5 — protected `SOUL.md` route prohibition

Attempt every known proposal/stage/approve route for a Tier-0 protected
`SOUL.md` change.

Required assertions:

- the proposal pipeline cannot apply the protected change;
- a principal-authorized protected update, where supported, uses a distinct
  audited authority path;
- supplying a fingerprint or approval identifier cannot convert the proposal
  route into protected-write authority.

This journey is the closure evidence for `threads-dgg`.

### J6 — complete veto-window terminal matrix

Open a real delayed-apply window and force every terminal family:

- applied;
- vetoed;
- evidence diverged;
- revalidation failed;
- superseded.

Required assertions:

- every opened window receives exactly one terminal event;
- every terminal event contains the normative typed close reason;
- no duplicate terminal event can be appended;
- a human approval path without a veto window remains valid;
- an inconsistent human-path plus opened-window history is rejected, not
  normalized by dropping close detail.

This journey is the closure evidence for `threads-980` and the review gate for
PR #27.

### J7 — retired-Ward schedulability

Pass a valid repository-owned retired-Ward corpus case through the live daemon.

Required assertions:

- daemon classifies and persists the proposal;
- scheduler makes the pending interval observable for `min_visible`;
- restart recovery re-adopts the pending work;
- the proposal reaches one valid terminal decision without loss or double apply;
- unsupported corpus input fails closed.

This journey is the closure evidence for `threads-zav`.

### J8 — committed-evidence drift and restart

Open a delayed window, stop the daemon, alter committed evidence or authority,
and restart.

Required assertions:

- replay detects the mismatch;
- stale policy, revoked approval, changed familiar binding, changed runtime, or
  changed proposal bytes cannot be reused;
- restart neither loses nor duplicates the terminal disposition;
- file mutation and terminal audit evidence agree after recovery.

## 6. Deterministic time

Deadline behavior must use an injected clock or scheduler seam. Real sleeps are
not acceptable correctness evidence. The test interface must support:

- advancing to just before `min_visible`;
- advancing to the exact earliest close;
- advancing beyond deadline;
- restart with persisted logical time inputs;
- revocation or evidence change between check and commit.

If the production scheduler cannot currently accept deterministic time, the
clock seam is a production prerequisite—not permission to weaken assertions.

## 7. Atomicity and TOCTOU proof

For any journey that applies bytes, final Gate-4 revalidation and the committed
binding must refer to the same immutable snapshot or transaction boundary.
Evidence must include:

- request/proposal digest;
- exact Threads and Coven commits;
- familiar/root/revision or current equivalent binding;
- policy/Ward/weave digest;
- principal authorization reference;
- pre-write and post-write content digests;
- audit terminal event identifier;
- committed file-state digest.

A relevant change after decision but before commit must refuse dispatch or
commit. The daemon must not reconstruct authority from incomplete client fields.

## 8. Failure evidence bundle

On failure, write a sanitized bundle under
`target/e2e-artifacts/<run-id>/` containing:

```text
manifest.json              exact commits, command, platform, scenario
request.json               synthetic redacted request
response.json              daemon response
logs/daemon.log            sanitized process log
state/ward-audit.jsonl      ordered relevant rows
state/pending-tree.txt      pending artifact inventory
state/workspace-tree.txt    governed workspace inventory and hashes
state/sqlite-schema.txt     relevant schema fingerprint, never real user data
junit.xml                  machine-readable result
```

`manifest.json` must record whether the local Cargo override was proven active.
Artifacts use synthetic data only and default to the retention in
`e2e/compatibility.toml`.

## 9. Red -> fix -> green closure rule

A Phase-5 blocker is not closed by prose or another in-process fixture alone.
Its implementation pull request must contain:

1. a failing black-box test against the pre-fix behavior;
2. the production fix in the canonical owning repository;
3. a passing black-box test after the fix;
4. a lower-level regression test for fast diagnosis;
5. the named CI check and failure artifact;
6. migration and rollback notes where persisted state changes.

Human sign-off gates remain human. Agents may assemble evidence and recommend a
decision; they must never simulate Nova's independent review or Val's freeze.

## 10. CI rollout

### Observation

- keep `cargo test --locked --workspace` as the compatibility baseline;
- add Nextest with JUnit and fail-on-flaky semantics;
- collect coverage without enforcing a threshold;
- measure discovered-test parity and runtime.

### Boundary

- land the daemon fixture downstream;
- run the eight journeys as non-required checks;
- verify diagnostic bundles and deliberate red cases.

### Enforcement

- require the pinned Linux daemon suite after all eight journeys are stable;
- protect `main` with quality, Cargo compatibility, and daemon E2E checks;
- fail if the Cargo override is inactive.

### Expansion

- run Linux/macOS/Windows on schedule;
- run a three-journey live-daemon Cave project nightly;
- promote one Chromium acceptance journey only after the flake budget is met;
- set a coverage regression floor from measured baseline, not an invented target.

## 11. Workstream assignment

| Lane | Familiar/role | Owns | Must not own |
| --- | --- | --- | --- |
| T0 — contract and evidence | Sage | compatibility ledger, artifact schema, conformance mapping | runtime implementation |
| T1 — core and regression | Cody | Rust lower-level tests, typed contracts, deterministic seams shared with daemon | human sign-off |
| T2 — daemon boundary | Nova integration lane | process fixture, request client, SQLite/pending/filesystem assertions, restart | redefining Threads semantics |
| T3 — audit/identity review | Echo | terminal-close, predicate-authority, replay and store invariants | daemon lifecycle ownership |
| T4 — human acceptance | Charm | Cave live-daemon rendering/approval journeys | client-side policy or approval authority |
| T5 — release/governance | maintainer | CI enforcement, protected branch, release evidence, rollback | weakening gates for velocity |

GitHub assignments remain on the accountable maintainer until bot/familiar
identities have independently authenticated repository authority.

## 12. Success metrics

- 8/8 critical authority journeys pass through the real daemon boundary;
- 4/4 named Phase-5 implementation blockers carry red-to-green E2E evidence;
- first-attempt E2E pass rate is at least 99.5% over 30 days;
- flaky retry-successes still fail the required check;
- pull-request critical-path p50 is under 8 minutes and p95 under 15 minutes;
- duplicate or missing terminal close records remain zero;
- correctness tests using real wall-clock sleeps remain zero;
- every required run proves the current Threads checkout was linked;
- escaped authority-boundary regressions remain zero.
