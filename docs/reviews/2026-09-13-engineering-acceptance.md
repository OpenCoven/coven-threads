# Phase-5 engineering acceptance and operator handoff

Three root engineering blockers are closed. Global historical audit closure
and Val's two human decisions remain open.

This September 13 checkpoint records accepted source and evidence, not a
deployment, release, or replacement normative contract. GitHub #31 and its
linked issues remain the work record; this document is the shareable handoff.

## Accepted scope

| Gate | Disposition at accepted daemon `226bfcc8` |
| --- | --- |
| OpenCoven/coven#885 / `threads-okc` | Closed: current intake/adapter/replay inventory, ordinary final-write identity binding, and required current-Threads compatibility |
| OpenCoven/coven#887 / `threads-dgg` | Closed: nine-route protected proposal matrix, real receipt reuse, startup refusal, and exact-source client boundary |
| OpenCoven/coven#888 / `threads-zav` | Closed: all seven actual retired-corpus scenarios, original red/fix/green, true recovery-worker contention, and typed closes |
| OpenCoven/coven#886 / `threads-980` | Open: supported recovery matrix and read-only census landed, but real deployed/unprovable history has not been resolved |
| #13 and #14 | Open: Val's human coherence acceptance, then current-scope freeze/reaffirmation |

The [adopted solo-maintainer policy](../../specs/PHASE-5-SOLO-MAINTAINER-REVIEW.md)
lets Val author and self-review. Its adoption is separate from accepting these
engineering results and from the subsequent freeze. The July 29 statement on
#14 remains historical evidence, not an automatic extension to later revisions.
Agent reviews are engineering evidence, not independent human approval.

## Immutable sources and normal landings

Selected daemon: `226bfcc89ff6cad4bc9cc9618dad6fea970ecf58`.
Its committed core is `0021fd2662d0b82328371ee3a6957d645f776fda`.

| Contribution | Normal accepted-main merge |
| --- | --- |
| OpenCoven/coven#1045, ordinary-write identity binding | `28cbd41c82d53e3856e0073974808d6dd624ebac` |
| OpenCoven/coven#1044, complete protected-route matrix | `e6fa686fbd083d69e2473b3c51775879b8baeefa` |
| OpenCoven/coven#1046, seven corpus/recovery scenarios | `ca95804bb40e04fca240a80691e47f0a99acb2e2` |
| OpenCoven/coven#1049, read-only global history census | `226bfcc89ff6cad4bc9cc9618dad6fea970ecf58` |
| #62, tracked/untracked/submodule source-drift guard | `2a7c1779a3a0e1653c1f674ac789b3c21f275a4a` |
| #65, four-target current-Threads required lane | `7f98ae83eb556fd5f11b34474947fc27a4b36566` |

Each contribution followed normal PR protection, current-base reconciliation,
actual review-conversation disposition, and exact-source acceptance. No main
push, force push, bypass, or old integration-branch shortcut was used.

The [landing evidence matrix](https://github.com/OpenCoven/coven-threads/issues/31#issuecomment-5656531969)
links the exact files, lines, original failures, and root dossiers. The
identity test-only red `4993521d` and production repair `4b6acb73`, protected
route red/fix lineage in OpenCoven/coven#933, and actual corpus test-only red
`d15d4939` are retained. Passing new coverage is not invented production-red
evidence.

## Required current-Threads evidence

[Linux run `34786471550`](https://github.com/OpenCoven/coven-threads/actions/runs/34786471550)
passed on attempt 1. Its actual Threads PR merge execution is
`56cd22ba3c0caba5c25051cad6ed324104edf676`, not merely the PR head.
Tree `cb2402ee004f0483bb570f5c989b905012864c59` equals reviewed head
`a1545cf191a62411684f9ddbda56b9d1e5975f1b`. It exercised accepted daemon
`226bfcc8` using the actual clean local Threads dependency.

```sh
cargo test --locked -p coven-cli --features threads-test-clock \
  --test threads_e2e --test threads_identity_invariants \
  --test threads_protected_intake --test threads_terminal_recovery -- --nocapture
```

This is the actual wrapper command, run after source/config/lock checks and
dependency-edge proof, not a direct validator call.

| Evidence | Result |
| --- | --- |
| Actual target counts, in command order | 104 / 44 / 38 / 33 passed; no failures, ignores, or filtered cases |
| Complete, unique E2E manifest/JUnit pairs | 112, all exact-source Linux/local-override receipts |
| Valid-to-valid identity packets | Eight, covering four classes live and after restart |
| Required corpus/history subset | 13 openings and 13 single ordered normative closes |
| Actual synthetic census reports | 42 reports covering 51 distinct typed histories |
| Existing focused Node regressions | 132 passed, including intentional missing-target and provenance failures |

The 13 required closes are seven `revalidation_failed`, one `superseded`,
three `applied`, one `evidence_diverged`, and one `vetoed`. Original audit rows
and actual census reports agree on submission/opening/close order and normative
replay-hash semantics. Unsupported intake and human approval without a window
do not acquire fabricated opening/close rows.

Artifact `10326542438` has independently recomputed SHA256
`17ffa42f5298ebf056e65f8d8a0d291b8f9c56883395131b2b339196eefe375c`.
The archive, raw logs, per-file index, and inspection source are retained in the
session handoff. Hosted retention is 14 days; preserve the packet separately.
Manifest count is not test count. Companion targets repeat some helpers and
are evidenced by the wrapper/logs; each fixture command is its documented
single-target reproducer.

The first draft run `34777258225` intentionally rejected the old
`harness-required` manifest before checkout. Earlier candidate
`f306e5c` / `e4c5b6fb` macOS results remain candidate-only evidence.
Neither is relabelled as this accepted-main Linux observation.

## Native evidence and retained uncertainty

[Accepted native CI `34784497791`](https://github.com/OpenCoven/coven/actions/runs/34784497791)
passed on attempt 1. Its actual execution `64b6a937` has the same tree as
accepted daemon `226bfcc8`. Windows ran 81 / 43 / 34 / 32 feature-target tests,
with 93 complete manifest/JUnit pairs across feature and workspace artifacts.
This uses committed Git core, not a current-Threads override.

The complete local locked serial workspace passed 4,082 tests with five
existing ignores. The unrelated default-parallel privacy-report limitation
remains in OpenCoven/coven#1053. Startup and IPC observations remain in
OpenCoven/coven#1047, OpenCoven/coven#1050, and OpenCoven/coven#1051.
The corpus component's first failed native attempt and one controlled
same-head repeat are retained. Later green does not prove a causal repair,
erase the failed attempt, or establish 99.5% first-attempt reliability over
30 days. No product deadline was widened.

Cave `a0f43d3cd141ad4f3e391991dd4ceb4544a71226` has 101 fixture/source tests
for narrow forwarding and daemon-derived outcomes. SDK
`952b51b6889c7397e188a0cda3e09ff53a542e14` has 21 tests for its health-only
transport, not a nonexistent Threads approval API. These satisfy the inspected
client-authority clause, not OpenCoven/coven-cave#5256's live-daemon acceptance.
The distinct operation-bound protected-write profile remains disabled.
J1 retains the E2E contract's explicit strongest-current-authorization limitation;
no signed-principal profile is claimed.

## Enforcement and rollback

After normal #65 landing, `Pinned daemon compatibility` became the sixth
strict required Actions-app `15368` context in ruleset `22910327`.
The [activation record](https://github.com/OpenCoven/coven-threads/issues/64#issuecomment-5656613112)
preserves the exact scope and before/after comparison. All five previous
contexts, current-base strictness, PR/resolved-conversation requirements,
stale-review dismissal, no-bypass policy, and deletion/force-push protections
remain unchanged. The latest-main schedule stays advisory.

The manifest's `ready` field records eligibility, not approval. No runtime
authority, audit schema, migration, identity model, credential profile,
release, or deployment changed in the Threads lane. Its guards check command
boundaries, not hermeticity or transient changes restored between commands.

If compatibility needs rollback, record the exact failure and deliberately
reconcile the pin, workflow, and live rule while retaining prior checks and
failure evidence. Do not substitute mutable main or skip failing companions.
Reverting the identity fix must never restore an unvalidated write: affected
routes would need to stay disabled. The census is additive and read-only, so
stopping its use needs no schema rollback.

## Remaining operator and human work

Use a binary built from the accepted daemon revision, not an older installed
release, and explicitly select the existing profile you intend to inspect:

```sh
COVEN_HOME=/path/to/existing/profile coven ward audit-census --json
```

The census does not start a daemon, initialize a profile, migrate a schema,
repair history, or authorize work. Preserve the store and related proposal
evidence first. Review `unresolvedHistories`, `classification`, and `issues`;
exit zero and `complete: true` mean the inventory completed, not that history
is healthy. Artifact names are non-atomic, unverified observations.

No real deployed profile was inspected in this engineering task. Missing
proposal bytes cannot be reconstructed from a digest. An uncertain interrupted
apply cannot safely be declared an unapplied rejection. Keep unprovable work
non-executable and document the evidence-backed operator disposition on
OpenCoven/coven#886; do not balance counts or manufacture terminal receipts.
Review real output locally and share only appropriately redacted findings.
Never turn real familiar declarations or secrets into fixtures.

After that obligation and broader live-Cave/reliability acceptance are
resolved, Val can record the exact accepted scope on #13, then the subsequent
freeze/reaffirmation on #14. Engineering or policy adoption alone cannot close
either gate.
