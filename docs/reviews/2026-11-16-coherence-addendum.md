# Coherence evidence addendum — 2026-11-16

This dated source observation supplements the [November 8 addendum](2026-11-08-coherence-addendum.md), the [November 6 addendum](2026-11-06-coherence-addendum.md) and the immutable [October 30 handoff](2026-10-30-coherence-decision-handoff.md). It records the Threads batch delivered by #123 and three Coven deliveries that bear on the Phase-5 criteria: #1652, #1665 and #1674. Observations were taken on 2026-11-16 at about 14:00 UTC. Earlier source tuples, execution receipts, original failures and human decisions retain their original scope. It selects no accepted source tuple or human criterion disposition.

## Threads: #123 batch delivery

Threads main advanced from `aba2e226c6809b823f39e0a838342e9ea330f59b` to `6d6f59abf2ed2a1c7d15dd4139c469fcaadeaf8d`, tree `5f7ef961d121027f670464ebac1c4ea80c8d3bb7`, through the normal merge of [PR #123](https://github.com/OpenCoven/coven-threads/pull/123) at 2026-11-13T04:37:09Z. Its parents are `aba2e226` and integration head `1a342d01c17739c3c7c8ce8b263063cecda210b8`, whose tree is the merged tree. The batch carries five reviewed heads with their original commits: #122 core Nextest retention (`99294ab6`), #121 approval-tier load supplement (`ae2968b8`), #103 authority-model pilot (`fc5b3fe9`), #86 corrected lifecycle/census wording (`5d0beb02`) and #100 Set C packet (`a1e9d8a6`). Val's [scoped source-hold exception](https://github.com/OpenCoven/coven-threads/issues/111#issuecomment-6843641054) authorized this exact batch after required checks and states that it does not accept Phase 5.

Post-merge main CI at `6d6f59ab` completed successfully: CI run [43366702421](https://github.com/OpenCoven/coven-threads/actions/runs/43366702421), daemon compatibility run [43366702418](https://github.com/OpenCoven/coven-threads/actions/runs/43366702418) and the dynamic CodeQL run [43366701119](https://github.com/OpenCoven/coven-threads/actions/runs/43366701119), with 13 of 13 check runs successful. The required compatibility pin is unchanged: Coven `226bfcc89ff6cad4bc9cc9618dad6fea970ecf58` with committed core `0021fd2662d0b82328371ee3a6957d645f776fda`. The integration's pinned-daemon artifact, recorded in the #123 description, verifies targets 104/44/38/33 and 112 journey/JUnit pairs at its tested merge; that result stays bound to its own tree.

The Git blobs of the October 30 handoff (`f4b45714`), the October 28 ledger (`36e67702`), the November 6 addendum (`c5eb2229`), the November 8 addendum (`58efab7c`), the adopted policy `specs/PHASE-5-SOLO-MAINTAINER-REVIEW.md` (`427a4ef4`) and `e2e/compatibility.toml` (`6b922510`) are identical between `aba2e226` and `6d6f59ab`. Under `docs/`, #123 adds four review documents (the Set C packet, the model pilot, the load supplement and the November 12 dispositions), extends the October 11 maintainer decisions with the #86 correction, and adds one dated observation to `docs/phases.md`. It deletes nothing.

What the batch changes for #13:

- **Criterion 4 wording.** The #86 correction removes the inference from an `OpenUnverified` census classification to a live, settled window. It keeps the governing obligation unchanged: every opened veto window requires exactly one typed terminal close. Merging the corrected text records it as reviewed engineering documentation. It is not Val's adoption of a narrower contract, and Decision 5 remains as historically recorded.
- **Set C.** The #100 packet is delivered with every original byte and a dated caveat that qualifies its historical census conclusions. Its observations keep their original date and source scope.
- **Criterion 7.** The #121 supplement records the 17 supplemental probes and the permanent Coven #1695 load-contract regressions, delivered as `43a523962874368e3d216c737361734f866e47e3`. Those regressions are in Coven main and in Coven v0.5.0. The supplement does not move the required pin.
- **Model pilot.** The #103 crate is an unpublished workspace member. Per the [#95 disposition](https://github.com/OpenCoven/coven-threads/issues/95#issuecomment-6906035858), its explorer and differential are adopted as ordinary tests and Kani is deferred ([#125](https://github.com/OpenCoven/coven-threads/issues/125)). The model is research evidence, not a canonical authority contract, and the Phase-5 veto-window state space is outside it. It neither advances nor blocks #13.
- **Retention.** #122 adds a separate core Nextest archive. It is CI evidence, not a second audit store, and supplies no native qualification.

## Coven source refresh, November 16

Coven main advanced from the last #13-observed `daace730cb1383c9af91ee9248317a02c5209073`, tree `ad4fab0e844e1bbff647088c69f70b0d928dd7cc`, to `3dc537fb0323fb291ca7fabfff3771bd6bf6c9bc`, tree `5a311461f9f2ffa13bff9786665623dcc084c16e`. The [complete comparison](https://github.com/OpenCoven/coven/compare/daace730cb1383c9af91ee9248317a02c5209073...3dc537fb0323fb291ca7fabfff3771bd6bf6c9bc) contains 46 commits across 164 paths, 24,714 additions and 1,141 deletions. Most of the delta is mobile v2 transport, relay and test hardening, CI routing, dependency bumps and engine pins; this addendum inspects only the three deliveries below and does not perform an exhaustive authority or recovery census of the rest. The [CLI manifest at the new head](https://github.com/OpenCoven/coven/blob/3dc537fb0323fb291ca7fabfff3771bd6bf6c9bc/crates/coven-cli/Cargo.toml) still requests Threads `7b5a9e688bff46bf9093e6065820921d119c4747`.

Latest published Coven is now [v0.5.0](https://github.com/OpenCoven/coven/releases/tag/v0.5.0), published 2026-11-10, at commit `d9b9d6a8b9f4775ad7fa0a2d95a9a78004c6d92c`, tree `11830408076c89205e6ed0ad85982abef07bf1ef`, also requesting Threads `7b5a9e68`. It includes #1643 (`db1e1084`) and #1695 (`43a52396`). It excludes all three deliveries below. At this observation 30 of the 65 reported check runs on `3dc537fb` had concluded: 24 succeeded and 6 were skipped. The other 35 had not concluded, so no current-head execution is claimed.

Among the paths sampled in earlier addenda, `crates/coven-cli/src/threads_gate.rs` is identical between `daace730` and `3dc537fb` (blob `6a8bbfe1`). `crates/coven-cli/src/api.rs`, `crates/coven-cli/src/api_power_loss_tests.rs` and `docs/ward-durability.md` changed. The only commits that change `api.rs` in this range are #1652 and #1665.

### Coven #1652: atomic direct Ward evidence

[PR #1652](https://github.com/OpenCoven/coven/pull/1652) merged at 2026-11-12T01:42:28Z as `757d13471a527e247d2cb1e0b8bac1689703d1ff`, tree `c17467b83634ed70d6a38f66b674c76d7b7edfd6`, from head `3bb116b58e12e2b8651f4bda5b0890755eb5e9e1`. It is an ancestor of observed main. It changes 10 paths with 1,547 additions and 56 deletions. Tracking issue [#1640](https://github.com/OpenCoven/coven/issues/1640) is closed as completed. This is the stronger candidate that the November 6 and 8 addenda recorded as open at `26865e39`; the delivered head differs from that observation.

From inspection of the delivered diff: direct (non-proposal) Ward writes now commit their Gate-4 audit rows and any applied protected-baseline advances in one FULL transaction, followed by the checked database-directory barrier, before reservation release and HTTP 200. This replaces the separate audit commit, the separate baseline commit and the trailing barrier from #1622 (`657c955f`), whose barrier-based repair is preserved as history. Failure remains typed: a failed audit insert, COMMIT, SQLite sync or directory barrier returns `audit_persist_failed`, and a failed baseline advance returns `ward_baseline_advance_failed`. Both report `writeApplied: true, retrySafe: false` and retain the reservation. An audit or baseline failure rolls back both database effects.

The delivered [`docs/ward-durability.md`](https://github.com/OpenCoven/coven/blob/757d13471a527e247d2cb1e0b8bac1689703d1ff/docs/ward-durability.md) states the remaining limits, recorded here unchanged:

- The pre-audit crash window is not closed. An unacknowledged direct write may have installed target bytes without audit evidence.
- A failed directory barrier follows COMMIT, so its evidence may already be durable.
- The audit reservation uses NORMAL commits. Power loss can drop a retained reservation or resurrect a released one as held capacity, and neither grants authority.
- Real-daemon journeys in `threads_e2e` need the `threads-test-clock` feature, which also compiles the synthetic power-loss fixture. Builds without that feature, including current release CLI artifact builds, contain neither.
- Only the database and WAL images undergo simulated loss. Target and pending-file assertions show consistency with the recovered database, not an independent filesystem power-cut proof.
- These are deterministic protocol tests, not physical power-cut certification.

For #13 this strengthens the acknowledgment evidence behind criterion 4's audit-completeness mapping at Coven main. It does not change the required pin, which predates it, and it does not close the criterion-4 gaps for quarantined, orphaned and unprovable histories. No daemon journey was executed for this addendum.

### Coven #1665: proposal decision origin isolated

[PR #1665](https://github.com/OpenCoven/coven/pull/1665) merged at 2026-11-14T11:58:40Z as `54670ed72fefea3084106b0a27dcc36c92223659`, tree `f865ca83e25f411e16f5c6a4889af5e6b55a3776`, from head `6e0206cf8c9ce6d9c23529a6a375597d27657dd5`. It is slice 6 of [#806](https://github.com/OpenCoven/coven/issues/806), which remains open. It changes 5 paths with 773 additions and 231 deletions.

The PR describes a move with no behavior change. The code that attributes a persisted Threads decision request to an actor moves from `api.rs` into a private `api::proposal_decision_origin` module: the trusted origin, its reservation lookup and purpose check, the request-digest binding, the historical apply-intent match and the invalid-origin classification. Visibility widens only to `pub(super)`. Claims, recovery and per-origin capabilities stay with the coordinator. A characterization commit lands before the move, a reported mutation check (dropping the expiry actor check) fails the suite, and a new `tests/proposal_decision_origin_boundary.rs` guard rejects wider visibility and store, Ward or coordinator dependencies.

This code is the decision-origin path whose #1618 residual #1643 repaired, and it is adjacent to the item-4 contract question. Its behavior-preservation claim rests on the PR's characterization and mutation evidence; this addendum did not execute those tests. A move does not change any criterion mapping or residual.

### Coven #1674: crash-left launch folders reaped

[PR #1674](https://github.com/OpenCoven/coven/pull/1674) merged at 2026-11-14T12:57:08Z as `b6d14869bbb53a7fc2179204e19c20f8102b7785`, tree `ac1cc390cb5c99edbdea1947a04296179347d01d`, from head `84ec5a1dda76d4f4e88a989add249f328f9d88a7`. It replaces #1624, which closed unmerged when its base branch was deleted, and [#1607](https://github.com/OpenCoven/coven/issues/1607) is closed as completed. It changes 8 paths with 1,151 additions and 5 deletions, all in `automations/scoped_read`, daemon startup, the startup diagnostics test and two architecture documents.

At startup the daemon now removes a scoped-launch folder only when it has this home's name prefix, opens as a real directory without following links, is owned by this user and closed to group and others, has a regular-file marker that names this home, and the marker's `flock` is free. Everything else is left and counted. Removal goes through file descriptors, the sweep is bounded (64 folders, 200,000 entries, 5 seconds), and the recovery log records counts without paths. This is cleanup of private harness transcripts in the scoped-read automation path. It is outside the Ward write, audit and veto-window paths, and it changes no #13 criterion mapping.

## Not refreshed

Cave was not refreshed for this addendum. The last #13-observed Cave head remains `a752cf0ea14ed23d112ddff14bb3119a730970f2`, with the criterion-3 authority-boundary note recorded on #121. The November 10 criterion-4 audit-lifecycle candidates, Coven `8edeb939` and Threads `0c491d1f`, remain unpublished. The native measurement, the source hold and the sole #111 collector are untouched. The `6d6f59ab` source handoff to that collector is recorded in its [own comment](https://github.com/OpenCoven/coven-threads/issues/111#issuecomment-6844164914).

## Unchanged evidence and human scope

All eight criterion mappings and the four accepted September dossiers remain as listed in the November 6 addendum. The July refusal, power-loss-in-scope ruling, historical freeze and October 30 unintended-close correction remain preserved.

Val's [November 8 approval instruction](https://github.com/OpenCoven/coven-threads/issues/13#issuecomment-6718700413) remains recorded. Its engineering qualification still applies: approval supplies no missing execution proof, and the criterion-4, 6 and 7 limits named there remain. The source-scoped coherence record under the [adopted policy template](../../specs/PHASE-5-SOLO-MAINTAINER-REVIEW.md#5-decision-records-you-can-use) is still outstanding. It needs exact Threads, Coven and applicable Cave SHAs, all eight criterion and four dossier dispositions, wording and residual treatment, rationale and the self-review acknowledgment. Val selects those; agents supply engineering evidence only. #14 receives no prerequisite from this observation, and no passing check, release or issue state supplies one.

This documentation changes no API, predicate, audit store, schema, migration or runtime behavior. Verification belongs in the accompanying PR receipt and must identify the exact candidate head. Rollback is to decline the additive PR or revert its documentation through review while preserving original packets, failures, receipts and foreign work.
