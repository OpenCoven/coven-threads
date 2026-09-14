# Native reliability handoff (2026-09-14)

The reliability continuation fixes fixture defects and adds accepted native
diagnostics. It does not establish a Windows startup repair or finish Phase 5.
Use this checkpoint alongside the
[September 13 engineering acceptance](2026-09-13-engineering-acceptance.md),
which remains the source-specific record for the three closed root issues.

You can inspect the remaining root obligation without touching a deployed
profile:

```sh
gh issue view 886 --repo OpenCoven/coven
```

GitHub issues retain live work status. This document records a dated engineering
checkpoint, not a new policy, release, deployment, or human approval.

## Accepted contributions

| Scope | Accepted source | What changed |
| --- | --- | --- |
| Setup fixture reliability, OpenCoven/coven#1053 | OpenCoven/coven#1057, `c56c2e2f10329c05df9273ac48914cf6b7ebb410` | Positive-process fixture guards and an execution marker. Production setup behavior is unchanged |
| Unix HTTP-read diagnostics, OpenCoven/coven#1050 | OpenCoven/coven#1058, `9b7d07aa30915f5d6eafd2cfb1bca8ae3bd59ce2` | Fixed request kinds, response progress, and original-deadline observations. The runtime cause remains open |
| Windows pipe, reservation, and tick diagnostics, OpenCoven/coven#1051 | OpenCoven/coven#1059, `b3b2d043a4ee586ccbf25ef6aad21db8a1171a54` | Fixture phase evidence and finer accounting, checkpoint, lock, COMMIT, release, and tick boundaries. The runtime causes remain open |
| Relative-socket fixture isolation, OpenCoven/coven#1062 | OpenCoven/coven#1063, `ebb5ae68ec2e8380979b7906f2710e556c550ea3` | Both cwd-sensitive lifecycle readers share the existing mutex. Production discovery and authentication are unchanged |
| Startup phase diagnostics, OpenCoven/coven#1047 | OpenCoven/coven#1061, `03c8ae9bb398af756b6472ea2f417c607760d8b7` | Fixed startup checkpoints and observer-excluding wall intervals compose with the accepted request diagnostics. The runtime cause remains open |

These contributions landed through ordinary protected PRs. Independent agent
review found no high-confidence introduced bugs in the diagnostic changes;
that review is engineering evidence, not independent human acceptance.

## Demonstrated fixture causes

The setup fixtures used a two-second positive-process hang guard. Under
parallel fresh-helper startup, actions took 4,103 ms and 4,973 ms, timing out
before version probing. Production correctly refused publication without a
verified version. OpenCoven/coven#1057 changes only positive fixture guards
to 30 seconds, retaining the explicit timeout cases and production version
deadline. Its hostile-version marker also exposes an earlier negative that
could pass without executing the hostile input.

The cwd investigation in OpenCoven/coven#1062 found two lifecycle readers that
did not participate in the existing process-global cwd mutex. A controlled
interleaving reproduces the actual wrong-home `Discovery` refusal using a deep
working directory and a shallower sibling socket allocator. Both readers now
hold `PROCESS_CWD_LOCK` from relative-path construction through the request.
The original canonical identity assertions remain intact.

The regression uses an isolated subprocess, not global suite serialization.
An earlier `/`-cwd control detected the missing lock but did not reproduce the
refusal because the endpoint-ancestor fallback legitimately matched that path.
That control result is retained separately from the actual discovery failure.
Neither fixture finding explains the native timing observations below.

## Source-specific evidence

| Contribution | Native run, attempt 1 | Actual execution source |
| --- | --- | --- |
| Setup correction | [34813100261](https://github.com/OpenCoven/coven/actions/runs/34813100261) | `d522c1383704f9282b4c8a072723f78f242bcd79` |
| Unix diagnostics | [34816038807](https://github.com/OpenCoven/coven/actions/runs/34816038807) | `2dec42cb6f0e7220899f2673c8f38914fe205094` |
| Reconciled Windows diagnostics | [34829074051](https://github.com/OpenCoven/coven/actions/runs/34829074051) | `000b2bdd8665504c513b2423a2b49fbc489d0707` |
| Reconciled cwd-fixture correction | [34832209134](https://github.com/OpenCoven/coven/actions/runs/34832209134) | `01081201d4bf4f9af0c3dc001ed20d1b73d79cb5` |
| Final startup diagnostic composition | [34835358925](https://github.com/OpenCoven/coven/actions/runs/34835358925) | `f85f5bcdc103b58d3dd65b634f73d23d84883550` |

The Windows diagnostic run executed all four targets with 81 / 43 / 34 / 32
tests. Its 93 complete native manifest/JUnit pairs are artifact counts, not
test counts. The actual execution, reviewed head, and accepted main share tree
`fc1567eba1211e10751f626dc0f6fafaac2a6695`. Original failures remain in the
evidence packet; the later passing composition does not repair them by
implication.

Initial cwd-fix run
[34827463823](https://github.com/OpenCoven/coven/actions/runs/34827463823)
passed at `a6829a8a`, actual source
`41ef28d07a4cde8b3a5b564ad9c0d86e39f55840`. The Linux log records all 50 health
tests, including the new interleaving case. The watcher stopped on a GitHub
GraphQL quota error, not a failed native job. That initial result is not
substituted for current-base acceptance. The later reconciled run passed at
`ebedf8bbe4fdf56a44e374ab316ef03ef302e558`, preserving the original fixture
patch byte-for-byte on accepted main. Its actual source and reviewed head
share tree `233bc3e1a703d911dabae0a7bb9fb064aeffebd8`; its Linux log again
records all 50 health tests. The Unix-only fixture is not a Windows execution
claim. OpenCoven/coven#1062 is closed; the native runtime issues are not.

The final startup composition at `eb05a810eb7d899da8d7a66e16b9a71e4e84594c`
preserves the reviewed startup patch without further implementation changes.
Its single fresh native run passed, with 93 complete manifest/JUnit pairs and
the lifetime-job target at 12/12. Actual execution, reviewed composition, and
accepted main share tree `c1c33feb6ad380fa2c15ad352277d9740452bcc9`.
One owned-foreground packet demonstrates combined startup and finer request
coverage. That is not standalone CLI proof or a causal improvement over E.

## Native causes still unresolved

OpenCoven/coven#1047, OpenCoven/coven#1050, and OpenCoven/coven#1051 remain open.
Keep their observations distinct:

| Observation | Evidence boundary |
| --- | --- |
| Windows startup | The composition at `9d6f73771f47db41ad2535e82731992a950341f7` failed both Windows jobs in run `34823642408`. One startup recorded a 2.271 s COMMIT interval and `store-close-begin` at 3.276 s, with no close-end marker |
| Linux response deadlines | The original failures do not identify empty response, partial header, body, or framing-boundary expiry. Accepted diagnostics distinguish these in future observations |
| Legacy Windows pipe, A | Original OS 232 output lacked the failing call phase. An assumed fixture handle sequence is not an observed sequence |
| Reservation latency, B | The old 5.187 s interval combined accounting, checkpoints, locking, mutation, COMMIT, and activation. It is not a pure lock or fsync measurement |
| Tick after rejection, C | The audit already contains the `evidence_diverged` close. An HTTP timeout is not evidence of a missing typed close |
| Later clock-control tick, D | A distinct request reached response start after 5.445 s. It is not the original rejection packet, and the later pass is not a repair |
| Post-admission identity request, E | Startup completed, then store opening took 1.474 s and the composite reservation interval 6.469 s. The captured audit is empty and the restart subcase was unreached |

A visible last log line does not prove its own append returned. Missing
`store-close-end` cannot distinguish the current append, close, descheduling,
or subsequent termination. Numeric intervals include scheduling and operation
time; they do not establish kernel, device, or fsync causation. Enabled
diagnostics add observer I/O and can perturb timing.

OpenCoven/coven#1061 subsequently landed on the accepted diagnostic and fixture
composition. Its earlier `6098a2ec` pass and `9d6f7377` failures remain separate
source-specific outcomes. The failed source was not retried or relabeled as
repaired; the passing composition supplies no retrospective timing explanation.

Further engineering must use bounded operation-specific evidence at an exact
source. Preserve original failures, deadlines, and cleanup. A same-head retry
or a passing later source does not demonstrate a causal repair or the 30-day
reliability target.

## Phase-5 and operator boundaries

The required compatibility pin remains Coven
`226bfcc89ff6cad4bc9cc9618dad6fea970ecf58`, with committed Threads core
`0021fd2662d0b82328371ee3a6957d645f776fda`. The newer native runs use that
committed core, not a current-local-Threads override. They do not repin or
replace the required four-target lane or change the six strict `main`
protection contexts.

OpenCoven/coven#885, OpenCoven/coven#887, and OpenCoven/coven#888 retain their
finite accepted criteria. OpenCoven/coven#886 still needs an explicitly selected
deployed-history inventory and evidence-backed disposition. No real profile
was inventoried during this continuation. Use the
[read-only operator procedure](https://github.com/OpenCoven/coven/blob/226bfcc89ff6cad4bc9cc9618dad6fea970ecf58/docs/design/threads-terminal-recovery.md#read-only-operator-census);
inventory completion is not healthy history or permission to fabricate a close.

Val's coherence decision in [#13](https://github.com/OpenCoven/coven-threads/issues/13)
and subsequent scoped freeze in [#14](https://github.com/OpenCoven/coven-threads/issues/14)
remain separate under the
[adopted policy](../../specs/PHASE-5-SOLO-MAINTAINER-REVIEW.md).
Neither agent persona nor general permission to continue supplies those decisions.

## Portable evidence and rollback

The retained operator archives are separate from protocol source:

| Archive | SHA-256 |
| --- | --- |
| `1053-accepted-evidence.tar.gz` | `578eb36d888fd96b35976bcc26946a3f35fa6c3c24febbd888dda4472c2f9344` |
| `1062-accepted-evidence.tar.gz` | `700801b6d8f42dc062d371e159f729a51e7a8e11d76a96c3bc0227d6759da515` |
| `native-reliability-final-composition.tar.gz` | `b0fa14248dc0e2af1821826678bce70f4a70636d4b04ad4ec42b53ce0d690876` |

The final native archive embeds the unchanged post-1059 evidence chain and
adds the final startup composition, source bundles, artifact digests, and
cleanup receipts. Public PRs and linked run artifacts identify the contributions;
the portable archives must be shared separately when handing off their retained
local copies.

Rollback is an ordinary reviewed revert of the relevant contribution, using
the first-parent mainline for a merge commit. There is no schema or data
migration, second audit store, deadline increase, durability reduction, or
authority change to reverse. Preserve the accepted setup and transport work
when reverting a later diagnostic contribution.

CLI start and whole restart remain five seconds; owned-authority admission
remains fifteen seconds; stop and status remain two seconds. The initializing
runtime-guarded store is still explicitly closed before readiness.
