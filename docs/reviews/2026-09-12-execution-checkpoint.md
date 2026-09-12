# Phase-5 execution checkpoint

**The current-checkout daemon observation lane is unblocked. Phase 5 is not
fully unblocked or ready to freeze.**

This records execution of the [handoff recommendations](2026-09-12-handoff-integration-review.md),
not another design proposal or an independent human decision.
Tracking: [Threads #31][boundary], `threads-cxv`; completed observation work:
`threads-cxv.1`.

## Completed and attributable

| Deliverable | Evidence | Scope |
| --- | --- | --- |
| Integrated handoff and advisory workflow | [Threads #49][pr49], main merge `237031a3e7acc1ba834cdc6aa9d1d5125ea68ca7` | Scheduled latest-main and full-SHA manual observations; no new required check |
| Existing Cargo configuration preserved | [Threads #50][pr50], main merge `df696c3fe972d670104cdabac5ce4ea7b24b8ec3` | Existing daemon network/build settings retained; explicit config and lock overlay hashes |
| Separate downstream compiler pin | [Threads #51][pr51], main merge `22187284425a2c70798bf117c899af1837fb32cb` | Advisory Rust 1.95.0 for daemon `sysinfo 0.39.6`; core baseline remains Rust 1.88.0 |
| Exact-current-checkout daemon observation | [Run 34678114852][green], [artifact 10292074982][green-artifact] | Threads `ac7884a2a51b36839f1c2168c5b90aea6397985b`, Coven `ea3f455aa66bb92880d95f57eeb7ca8d845b7fe6`, attempt 1 of this run |
| Shareable owner coordination and result | [Threads continuation][published], [daemon integration coordination][coordination] | Existing identity and producer claims respected; no competing edits to their worktrees |

Each Threads PR landed normally after its five required checks passed.
The workflow is active on the default branch. API read-back still reports the
original five required checks, and `e2e/compatibility.toml` still pins
`39feb6de98816d10b490091e918f62035e6ce0df` with `harness-required`.

### What the passing daemon run proves

The runner resolved `coven-cli`'s actual Threads dependency edge to the exact
current checkout, not merely a local package with the right name. Its
`observation.json` records the exact commit pair, toolchain, invocation,
run/attempt, and original/overlay Cargo config and lockfile hashes.

The actual command was:

```bash
cargo test --locked -p coven-cli --features threads-test-clock \
  --test threads_e2e -- --nocapture
```

All 20 target tests passed, with zero filtered or ignored tests. The downloaded
bundle contains 23 distinct scenario manifests; all report `passed`,
`local_threads_override_active=true`, the exact commit pair above, and
`threads_dirty=false`. They include the retired-corpus restart, five terminal
families, changed/unavailable identity, protected refusal, human no-window
approval, final-commit identity drift, and lifecycle scenarios.

`coven_dirty=true` is expected here: the clean daemon checkout receives the
explicit configuration and lock overlay. The wrapper records both hashes.
The older daemon's per-scenario command text still omits the feature flag;
the wrapper receipt and actual workflow invocation record it correctly. Do not
silently rewrite the older artifact's provenance.

This is an observation of the named **older integration revision**, not
acceptance of a newly reconciled daemon head, proof of missing scenarios, or
closure of any root blocker. Twenty test functions and 23 scenarios are not a
substitute for the eight-obligation coherence matrix.

### Preserved failures and production corrections

| Attempt | Actual result | Correction / retained evidence |
| --- | --- | --- |
| Clean local main preflight | Refused missing full `threads_e2e` at Coven `aa527d2d`; neither checkout changed | Local receipt `target/daemon-observation-main-20260912/observation.json`; intentional refusal, not root-defect red |
| [Run 34677886218][config-red] | Preflight refused the actual downstream `.cargo/config.toml` | [Artifact 10292714338][config-artifact]; #50 preserves existing settings |
| [Run 34678004690][compiler-red] | Exact local override proven, but daemon compilation refused Rust 1.88 because `sysinfo 0.39.6` requires 1.95 | [Artifact 10292743997][compiler-artifact]; #51 pins the downstream compiler separately |
| [Run 34678114852][green] | Real-daemon target passed with the current Threads checkout | New run, not an overwritten retry or a claim of first-attempt reliability |

The first two hosted failures are observation-runner setup failures, not
pre-fix reproductions of the four authority defects. They remain in the
published history and must remain in reliability accounting.

## Canonical implementation continuation

The current-main reconciliation uses an isolated Coven worktree and the
existing `issue-884` claim. The original integration branch is preserved.
Active `issue-885` and `issue-888` claims remain with their existing owners;
[identity coordination][identity-coordination] and
[producer coordination][producer-coordination] state the source gaps and
boundary obligations without overwriting those branches.

This checkpoint does not yet certify a replacement integration commit.
The accepted observation above cannot be transferred to it. Its own exact-head
review, preserved-main regression evidence, and daemon run are required.

## Remaining blockers, owners, and closure conditions

| Work | Status and next concrete requirement | Canonical owner |
| --- | --- | --- |
| Classification-time identity binding | Still open: commit predicate/configuration and candidate evidence at intake; prove valid-to-valid changes refuse deadline/restart reuse | [OpenCoven/coven#885][identity], active identity lane; Echo review |
| Supported auto ceremonies | Still blocked by built-in floor-0/1 regions and scheduled-loader constraints; obtain any necessary scoped region/approval decision before implementing a low-risk route | [OpenCoven/coven#972][producer], [OpenCoven/coven#884][harness]; Threads owns any public region contract change |
| Complete audit chain / terminal matrix | Preserve landed typed-close repairs; prove submission/opening/close correspondence and crash ordering, every family, duplicate prevention, and ambiguous applying-state disposition | [OpenCoven/coven#886][terminal], daemon lane; Echo review |
| Protected route matrix | Preserve landed refusal/control-alias repairs; finish all known-route, retry, cross-binding, drift, and recovery cases | [OpenCoven/coven#887][protected], daemon lane |
| Retired corpus | Preserve the landed clock; prove the final supported producer's synthetic migration/intake/visibility/restart/terminal path | [OpenCoven/coven#888][corpus], active producer/corpus lane; Sage mapping |
| Final authority-to-bytes binding | Prove the final daemon-owned authority snapshot remains bound to committed bytes across drift, interleavings, and restart on the accepted combined head | [OpenCoven/coven#977][binding], daemon lane |
| Stable required compatibility | Await reviewed daemon landing and complete attributable dossiers before proposing a new full-SHA pin or enforcement | Threads #31 / maintainer |
| Native OS and Cave | Linux observation does not certify native Windows/macOS or live-daemon Cave acceptance | Daemon lane; Charm / OpenCoven/coven-cave#5256 |
| Reliability measurement | No 30-day window or 99.5% first-attempt result is established by these runs | Maintainer / harness owner |
| Coherence and freeze | Still reserved, independently attributable decisions; no checklist item checked by this execution | [Threads #13][coherence], then Val |

## Verification, authority impact, and rollback

The final runner/pin regression invocation passed 40 tests:

```bash
node --test scripts/tests/daemon-canary.test.mjs scripts/tests/check-ci-pins.test.mjs
node scripts/privacy-guard.mjs
git diff --cached --check
```

The indexed privacy guard and staged whitespace checks passed. Actual hosted
execution, rather than these lower-level tests, exposed and drove both setup
corrections. The passing daemon run's receipt and all 23 manifests were
downloaded and inspected, including exact commit, override, dirty-state, and
scenario provenance.

No identity definition, Rust contract, audit schema, migration, stable pin,
required rule, protected floor, or human gate was changed by the Threads
observation work. No real familiar declarations or private interaction history
were published.

To roll back observation, revert the advisory workflow and runner changes;
there is no required-check rule to remove. Preserve the dated receipts and
root-blocker evidence. A daemon rollback must remain fail-closed and preserve
ambiguous pending/applying state rather than restoring an unvalidated write or
inventing a terminal receipt.

[boundary]: https://github.com/OpenCoven/coven-threads/issues/31
[pr49]: https://github.com/OpenCoven/coven-threads/pull/49
[pr50]: https://github.com/OpenCoven/coven-threads/pull/50
[pr51]: https://github.com/OpenCoven/coven-threads/pull/51
[green]: https://github.com/OpenCoven/coven-threads/actions/runs/34678114852
[green-artifact]: https://github.com/OpenCoven/coven-threads/actions/runs/34678114852/artifacts/10292074982
[config-red]: https://github.com/OpenCoven/coven-threads/actions/runs/34677886218
[config-artifact]: https://github.com/OpenCoven/coven-threads/actions/runs/34677886218/artifacts/10292714338
[compiler-red]: https://github.com/OpenCoven/coven-threads/actions/runs/34678004690
[compiler-artifact]: https://github.com/OpenCoven/coven-threads/actions/runs/34678004690/artifacts/10292743997
[published]: https://github.com/OpenCoven/coven-threads/issues/31#issuecomment-5644202431
[coordination]: https://github.com/OpenCoven/coven/issues/884#issuecomment-5644128132
[identity-coordination]: https://github.com/OpenCoven/coven/issues/885#issuecomment-5644130346
[producer-coordination]: https://github.com/OpenCoven/coven/issues/888#issuecomment-5644130475
[identity]: https://github.com/OpenCoven/coven/issues/885
[producer]: https://github.com/OpenCoven/coven/pull/972
[harness]: https://github.com/OpenCoven/coven/issues/884
[terminal]: https://github.com/OpenCoven/coven/issues/886
[protected]: https://github.com/OpenCoven/coven/issues/887
[corpus]: https://github.com/OpenCoven/coven/issues/888
[binding]: https://github.com/OpenCoven/coven/issues/977
[coherence]: https://github.com/OpenCoven/coven-threads/issues/13
