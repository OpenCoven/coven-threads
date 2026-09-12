# Boundary proof remediation

The original Windows authority-fixture failure and scheduled-identity gaps
have exact-revision repair evidence. Both bounded auto variants and the CLI
deadline repair have native Windows proof. A newly measured owned-start
cold-store failure still blocks final combined acceptance. Phase 5 remains active.

Check the paired contributions before using a newer revision:

```sh
gh pr view 57 --repo OpenCoven/coven-threads
gh pr view 1029 --repo OpenCoven/coven
gh run view 34696507398 --repo OpenCoven/coven
```

This packet records evidence, not a release, installed-binary certification,
independent human coherence decision, or freeze. GitHub issues remain the
portable work record; you don't need the maintainer's Beads database.

## Candidate and ownership

| Item | Exact revision or disposition |
| --- | --- |
| Core code, #57 | `2c7a305e434d7ae7c0e628f992f271007a3ab06c` |
| Combined daemon, OpenCoven/coven#1029 | `b439b4d1286521d98e6ba1259cd0192112f62543` |
| Existing integration owner, OpenCoven/coven#1022 | `8d48bf79b8c00478f9d6f077e34a1a5bac460b77` at this checkpoint |
| Daemon's actual core Git dependency | `2c7a305e434d7ae7c0e628f992f271007a3ab06c`, checked through Cargo metadata |
| Stable compatibility pin | Unchanged `39feb6de98816d10b490091e918f62035e6ce0df`; `harness-required`, not an activated required daemon check |
| Work tracking | `threads-vpp`; bounded fixture, identity, auto, and CLI subtasks `.1` through `.4` closed; cold-store follow-up `.5` in progress |

OpenCoven/coven#1027 merged at `1294a893` through `cacb6470`. It did not merge
the branch's later `976218d5` fixture finalization. The combined contribution
preserves that later work and the integration owner's independent requirements
in one shared helper. No owner worktree was overwritten or replaced by a
second main integration.

OpenCoven/coven#969 merged at `edf3500883e72e4239ec7151904983f61a93b83f`.
OpenCoven/coven#972 remains open and owns separate scheduled-publication and
recovery work. These proofs do not silently close that broader scope.

## What changed

Windows authority fixtures retain an owned `daemon serve` child instead of
making every authority assertion depend on the short CLI launch budget.
Admission starts its fixed 15-second budget before spawn, authenticates the
child and endpoint, and never retries fatal health errors. Failure settlement
can observe natural exit within the remaining admission budget; required
termination/reaping shares one separate, persistent 15-second teardown budget.
Dedicated CLI start/stop/restart cases remain separate.

Failure artifacts finalize the owned child's exit before persistence. An
observed natural exit differs from a termination request. Cleanup-induced
status is not presented as a CLI command failure receipt. The original probe
error, stderr, request/response pairing, and truthful launch mode are retained.
An inherited macOS hard-linked socket failure was separately repaired by
canonicalizing the parent directory, not the socket leaf, while preserving
exact path, non-symlink socket, peer, device, and inode checks.

Scheduled identity cases change still-valid identity bytes, soul bytes,
roster metadata, or active predicates. Each runs at a live deadline and across
restart. The stale proposal receives one `EvidenceDiverged` close and no write;
a fresh proposal under the changed authority applies. Another restart must
preserve the submission/opening/terminal chain.

Auto now requires an explicit literal and effective tier-2 declaration for
root `output-format.json`. Both images must be at most 256 UTF-8 bytes and
match the closed object/string v1 schema. Configured parse/json and all
applicable deterministic probes must freshly pass. Both variants stage through
supported intake; explicit `veto: null` creates no fabricated window events.
A private commitment anchors images, regions, identity presence/value, policy,
and probe evidence in the existing submission audit. Replay and final writes
recheck authority; ambiguous applying state stays quarantined.

The implementation refuses invalid representations, mixed batches, outgoing
or composed symlinks, dangling destinations, and unsupported hardlink aliases.
Proven unrelated ordinary writes remain ordinary. Existing protected-memory
floors, stronger ceremonies, advisory probe consumers, and conservative retired
migration remain unchanged.

## Causal and platform evidence

The original Windows failure in
[run 34684041348](https://github.com/OpenCoven/coven/actions/runs/34684041348)
occurred before authority assertions: cold store initialization took 2,017 ms
while the launcher had 1,987 ms remaining after spawn. Its failed artifact is
not relabeled as successful.

Frozen published test sources were also run against production
`65766b89b4c945d325853cefdd05f364ee87cbac` with clean core `2c7a305e` actually
linked. Only test includes, helper visibility, and the temporary Cargo overlay
changed. All three expected reds reached the real daemon:

| Case | Observed pre-fix failure |
| --- | --- |
| Auto with veto | HTTP 200 and an immediate file write instead of staging |
| Auto without veto | HTTP 200 and an immediate file write instead of staging |
| Still-valid identity-byte drift | `proposal_approved` / `applied` with `replay_hash_matched=true` instead of `EvidenceDiverged` |

All setups completed. The overlay lock remained
`fd90e9f44d2b9db9593128c6374422366f948ee9574457e8d833602ccdebf6e8`.
The main auto support module from daemon `300d4068` has SHA-256
`0f78b45ab914091a1753592363c7bfff38e6753f4862cc367008941e865eb36d`;
the identity support module has SHA-256
`c7a65fb668e9fccf4cdebf1ba4d03aec6a02f31dc1328bf6ad86c429217c03f0`.
Review-discovered parser and alias defects received separate correct reds
before correction. A macOS filename-encoding setup failure was not counted
as an authority red; its Linux-only regression subsequently passed.

| Run | Exact scope | Verified result |
| --- | --- | --- |
| [34687832909](https://github.com/OpenCoven/coven/actions/runs/34687832909) | Native Windows, actual daemon merge `63a496886d1851a2518519692d55f2c59096d9b9`, committed core `c3bd46bc` | E2E 19 without clock, 33 with clock, dedicated CLI lifecycle 6; 36 manifests represent 5 smoke plus 31 feature journeys, with 8 identity packets |
| [34688911843](https://github.com/OpenCoven/coven-threads/actions/runs/34688911843) | Same daemon `63a49688`, clean current core `8a2f7fb4` override | 31 passing journeys and 8 identity packets |
| [34690932324](https://github.com/OpenCoven/coven-threads/actions/runs/34690932324) | Core `2c7a305e`, pre-consolidation daemon `300d4068` | 94 passing Linux journeys, including 71 auto cases |
| [34693826823](https://github.com/OpenCoven/coven-threads/actions/runs/34693826823) | Core `2c7a305e`, combined daemon `84b8dcc7` | 103 passing Linux journeys, including 71 auto cases and 8 identity packets |
| [34693820017](https://github.com/OpenCoven/coven/actions/runs/34693820017) | Native merge `2de44ef5b9e3b0f591b8286efc6d02e7f9d34572`, core `2c7a305e` | Authority E2E 70 passed; 83 passing manifests represent 6 smoke plus 77 feature journeys, including 45 auto cases and 8 identity packets. Dedicated CLI lifecycle was 5/6; the overall run failed |
| [34696513592](https://github.com/OpenCoven/coven-threads/actions/runs/34696513592) | Core `2c7a305e`, CLI-repaired daemon `df0ccd91` | 103 passing Linux journeys, including 71 auto cases and 8 identity packets |
| [34696507398](https://github.com/OpenCoven/coven/actions/runs/34696507398) | Native merge `a8486f407d518398f5e6ac1c4866101e0d0d86d1`, core `2c7a305e` | All 6 original CLI cases plus 5 diagnostic tests passed. Authority target 69/70; 82/83 manifests passed, including all 45 auto and 8 identity packets. Owned startup failed before the clock-restart scenario; overall acceptance failed |

The native merge and published daemon `84b8dcc7` have identical Git tree
`6713db280c487181404db04be8418fe6baa76757`. Unix-specific symlink and encoding
cases are not counted as Windows execution. The failed CLI case,
`abrupt_daemon_stop_kills_child_stalled_before_per_session_job_attachment`,
failed during initial `daemon start`, before its session/barrier assertions.
The child was still running and startup health timed out. No store-duration
claim is inferred for that failure because its fixture did not retain those
timings. OpenCoven/coven#1030 repaired that deadline path; a successful
authority fixture was not used as a replacement for the failed CLI gate.

The follow-up `df0ccd91` restores the earlier passing lineage's budget split.
The production-used `LifecycleOperation` selector gives start and the whole
restart one five-second deadline, established before profile resolution and
locking. Standalone stop/status remain two seconds. Restart does not renew
the budget between stopping and starting. Authentication, pending-error
classification, and original-deadline cleanup remain unchanged.

A deterministic three-second readiness regression failed under the old
selection and passed after repair. The actual Windows fixture now retains
allowlisted startup phases and numeric timing fields before temporary-home
cleanup, bounded to 16 KiB and a one-second caller deadline. Missing or delayed
diagnostics are explicit, not invented timings. Native merge `a8486f40`
and published `df0ccd91` have identical Git tree
`6c811d2bd5c12f75b66136d58bce98332c02e19e`.

The next native failure is different: `deterministic-clock-restart` failed
initial owned admission after 15.0066 seconds, without status publication or
authority assertions. Store initialization checkpoints reached 816 ms for
connection setup, 2,907 ms for Ward, 3,626 ms for runtime, 3,634 ms for main
schema, 8,136 ms for commit, and 13,436 ms for initialization end. No
`daemon-store-end` checkpoint was retained before cleanup. The child's exit
code 1 is explicitly marked `fixture_termination_requested`, not a spontaneous
daemon failure. OpenCoven/coven#1031 investigates the exact store path; the
unobserved remainder is not assigned an invented cause.

The same run's policy guard also rejected a public CLI guide edit. The
documentation-only `b439b4d1` moved the deadline contract into the existing
source-adjacent E2E reference without exempting that public page. It changes no
runtime source, fixture, or dependency. Its automatic CI run is not a causal
repair for the store failure.

The combined local targets passed E2E 91, identity invariants 41, protected
intake 35, and terminal recovery 30; no-clock E2E passed 36. Shared helper tests
occur in several targets, so these counts are not unique coverage totals.
Scoped macOS and Windows GNU cross-Clippy passed. Native execution is a
separate receipt, not inferred from cross compilation.

### Exact command scope

Native CI ran `cargo test --workspace --locked`, including the six original
Windows CLI cases and five diagnostics, successfully. It then ran the following
feature target, which was 69/70 on Windows and passed with 103 Linux journey
manifests in the current-core observation:

```sh
cargo test --locked -p coven-cli --features threads-test-clock \
  --test threads_e2e -- --nocapture
```

The parent also reran the production selector and portable diagnostic cases
locally against Rust 1.95.0. One selector test and five diagnostic tests passed:

```sh
cargo +1.95.0 test --offline --locked -p coven-cli \
  --bin coven --test daemon_startup_diagnostics -- \
  lifecycle_operation_budgets_separate_startup_from_stop_and_status \
  startup_failure_checkpoints --quiet
```

## Artifact receipts

Wrappers, digests, exact revisions, override mode, scenario uniqueness, and
identity audit packets were inspected independently of workflow success.

| Artifact | SHA-256 |
| --- | --- |
| Original native failure `10294773022` | `1d523c33e8c2690471c2cb0bcd6c73a3abcfa2220f9ea09532d16e0c7e4c8179` |
| Native repair `10297115458` | `f02f2c30968f159a9040c9beee7266478443fb518530ae08b029fff6bd14c9f4` |
| Same-daemon current-core proof `10297310050` | `318351a53346d99153cdd46dac2402dc1b559d22bcdd5514d427649114f63bb1` |
| First auto observation `10296383839` | `7b840e5ec12b3cf0552a69afbff29ecbdf06adcaec7c1af1d465347afd2a1063` |
| Combined observation `10298073019` | `d34c17c42d6fd4d81228334f2b73599916b3ef2ec2c4e0f14106196373a247d7` |
| Combined native authority evidence `10297893087` | `c36ec471a3035de28a4ec5f2f19a41fe442342abc1058895729d93e581be6e90` |
| CLI-repaired current-core observation `10297964782` | `9be24fdabd67b778a341036d9add93a5b73da12ae23191547e73eda3b18d0dc7` |
| CLI repair and new owned-start failure `10298638899` | `0596f02019af72dfb75bfd59aaeb9430e1372ef1a1ceb93a4e8fa95d18162164` |

Git dependency receipts can report `threads_dirty=true` because Cargo creates
an empty, untracked `.cargo-ok` marker. The inspected marker-only state has
fingerprint `e3f9ad050974054ecfbdbb5d8c67e4959c96a39b4e05e8458bf90ef53385c93b`,
with no tracked or staged changes. The flag was not hidden or rewritten.
Clean current-checkout overlays are recorded separately.

## Review, impact, and remaining gates

Consulted contracts include Familiar Contract RFC-0001 at
`13d150a32a817da19bb4e5053f2205b15db0bb0a`, the Phase-5 approval decision,
`specs/PHASE-5-APPROVAL-SEMANTICS.md`, public approval/identity/region/replay/audit
types, and the
[E2E contract](../testing/e2e-contract.md). Changes are in the core region,
exports, corpus, and architecture docs; daemon intake, probes, Ward resolution,
private replay evidence, fixtures, and their source-adjacent tests. The CLI
follow-up consulted `crates/coven-cli/src/daemon.rs`,
`crates/coven-cli/tests/windows_daemon_lifecycle.rs`, and the shared
`crates/coven-cli/tests/fixtures/threads_admission.rs`. Its new diagnostics
live in `crates/coven-cli/tests/fixtures/daemon_startup_diagnostics.rs` with a
portable target at `crates/coven-cli/tests/daemon_startup_diagnostics.rs`.
The paired pull requests retain the complete changed-file lists.

Separate automated reviews found and corrected the parser, alias, deadline,
and failure-evidence defects. Those reviews are not human coherence approval.
No SQL schema/migration, credential authority, second audit store, public
exhaustive Rust variant, real familiar fixture, or production deployment was
introduced. The core addition is an unreleased additive 0.2 Git candidate.

Before rollback, stop affected-familiar write admission and resolve or quarantine
auto claims using compatible code. Keep the daemon stopped during the paired
rollback. Before restarting older code, require a regular, non-symlink target
with protected tier 0 and no unresolved auto claims in its active queue.
Changing only the approval ceremony is insufficient on older daemons.
Preserve append-only audit and interrupted-apply evidence.

OpenCoven/coven#885, OpenCoven/coven#886, OpenCoven/coven#887, and
OpenCoven/coven#888 remain open. The owner must integrate the paired candidate
and reconcile its independent publication/recovery work. Preserve the pinned
core SHA or repeat affected proof after rewriting it. Cave live-daemon human
acceptance was not run because this work does not deploy a Cave/daemon pair.
No 30-day reliability target, required pinned-daemon check, stable-pin rollout,
Nova coherence decision, or Val freeze is claimed.
