# Common-tuple status for the eight coherence criteria

**Engineering evidence, 2026-11-28.** This packet supplies no human
disposition. Val's [coherence decision](https://github.com/OpenCoven/coven-threads/issues/13#issuecomment-7207324591)
accepts seven criteria, each at its own tuple, and asks for one common tuple
that carries evidence for all eight. This packet takes the newest tuple with
retained daemon-boundary execution and records, criterion by criterion, what
is shown there, what is not, and why. Earlier acceptances do not transfer to
this tuple, and evidence for this tuple does not transfer back.

## The candidate tuple, T-NOW

| Element | Value |
| --- | --- |
| Coven | `9415c3098f9635963db593970b8943370e90bf81`, Coven main from 2026-11-25 through this packet |
| Threads core compiled | `4955cf8dc97e7fde07adf4e233fd6e060ee584e4`, Threads main, through the local Cargo override (`local_threads_override_active: true`, `threads_dirty: false`). Coven main's own pin is Threads `7b5a9e68`; the core diff from there to `4955cf8d` is confined to `automation_authority` and the unpublished model crate. |
| Cave / producer | **None at this producer.** Cave main `d67f1559` pins its live-daemon harness to producer Coven `b07935f0` (`tests/threads-live-daemon/compatibility.json`), which is T-CAVE2. |

**Execution:** hosted scheduled run [`46036830391`](https://github.com/OpenCoven/coven-threads/actions/runs/46036830391), "Daemon observation (advisory)", 2026-11-27 16:49–16:54 UTC.

| Run detail | Value |
| --- | --- |
| Platform | Linux, rustc 1.99.1 as recorded |
| Target | `threads_e2e` only |
| Artifact | `threads-daemon-observation-46036830391-1`, SHA-256 `866f76175ef593124b5a7cb30681aaa2e7491f077bde11c242a8ff90cd5a6f11` as downloaded; retention ends 14 days after the run |
| Coven tree | Dirty only through the recorded overlay: config overlay `3546b5a6…`, lock overlay `67dc2a9b…` |

The run is advisory and not required. I replayed its retained bytes offline:

- **Journeys:** all 134 have JUnit `failures="0"` and a manifest result of `passed`. Every manifest names the same Coven/Threads pair with the override active.
- **Windows:** 51 opened, and all 51 are `typed_terminal_recorded` in their window census, with 0 unresolved histories.
- **Closes:** 13 `applied`, 2 `vetoed`, 20 `evidence_diverged`, 15 `revalidation_failed`, 1 `superseded`.
- **Paths:** 20 windows on `auto`, 31 on `familiar_review`.

**Not executed at T-NOW:** the other three required daemon targets (`threads_identity_invariants`, `threads_protected_intake`, `threads_terminal_recovery`), macOS and Windows, and any Cave journey. The required pinned check executes those four targets at Coven `226bfcc8`, a different tuple. For Threads main that is run [`45976553628`](https://github.com/OpenCoven/coven-threads/actions/runs/45976553628).

## Per-criterion status at T-NOW

| # | Criterion | Evidence at T-NOW | Not shown at T-NOW | Status |
| --- | --- | --- | --- | --- |
| 1 | RFC round-trip | Threads core at `4955cf8d`: `rfc0001_s5_conformance` 14/14, `c7_roundtrip` 17/17, `phase5_retired_ward_corpus` 4/4. Local rustc 1.99.1, and in hosted CI run [`45976553539`](https://github.com/OpenCoven/coven-threads/actions/runs/45976553539). Related daemon identity journeys passed: `identity-intake-changed`, `identity-intake-missing`, the identity-replay cases `scheduled-valid-identity-bytes-live/restart`, and `scheduled-window-identity-changed/unavailable`. | `threads_identity_invariants` not executed at this Coven; no macOS run. | Partly shown |
| 2 | Gate-4 revalidation by every reachable ApprovalPath | Every window on `auto` (20) and `familiar_review` (31) reached a typed close, including 15 `revalidation_failed` and 20 `evidence_diverged`. Human-path journeys passed: `reviewed-human-no-window`, `changed-human-path-opened-window`, and the five `final-commit-*` release and identity-drift journeys. | Same bound as T-SEP: the journeys exercise the paths `threads_e2e` reaches, not every route. Protected Tier-0 stays unschedulable. `threads_protected_intake` not executed. | Partly shown |
| 3 | Descriptor is not authority | `protected-proposal-route-prohibition` (403), `protected-rejection-audit` and `smoke-unsigned-protected-rejection` passed. | The accepted Set C route census (84 routes) ends at `c658603d`. Coven has 234 commits since, `crates/coven-cli/src/api.rs` changed by −2,432/+858 lines, and distinct `/api/v1` route literals grew from 259 to 280 (approximate; includes test strings). The census must be rerun at this Coven. | **Not shown** |
| 4 | Audit completeness and typed reason | Supported intake: 51 of 51 opened windows have exactly one typed close, covering all five reasons, with 0 unresolved histories. | Damaged histories (quarantined, orphaned, untrusted, unprovable apply) have no close path at this Coven. The [options memo](2026-11-28-criterion-4-options.md) on this branch records why and proposes option A, which awaits Val's decision. `threads_terminal_recovery` was not executed here. The six #1716 journey families were executed only at Coven `346b7717`/`8edeb939` with core `aba2e226`. | **Not met** |
| 5 | Replay hash present and enforced | 20 `evidence_diverged` closes. `scheduled-window-surface-diverged`, `out-of-band-reviewed-drift` and the `output-auto-drift-*` families passed. | No byte-level binding of stored hash, re-derived hash, decision and final bytes per reviewed path was extracted from the retained state in this packet. | Partly shown |
| 6 | Minimum visibility and reachable pending state | Daemon side: `retired-corpus-scheduled-restart` passed. It asserts that the durable pending record and the submission audit bind the same classification, that `earliest_close` and the deadline equal the corpus policy, and that the target bytes are unchanged one second before minimum visibility, across a restart (Coven `tests/threads_e2e.rs:405-460`). | No Cave journey exists at this producer. The Cave harness pins `b07935f0`. Showing it needs a reviewed Cave harness repin to `9415c309` and a dispatch, and that dispatch was not authorized tonight. | **Not shown** |
| 7 | Load-time label/variant mapping | All 17 `approval_tier_load_*` journeys: the 4 valid configurations returned 200; the unknown tier and 12 mismatches returned 500 `ward_config_invalid`. The pattern matches T-LOAD. | Linux only; not every load-error class, the same bound as T-LOAD. | Shown, within that bound |
| 8 | Advisory probes separated, Gate-3 only | `output-auto-advisory-isolation` and the `output-auto-regression-gates` and `-floors` journeys passed. | No model probe is wired into the daemon, so separation is vacuous at the daemon, as at T-SEP. Kani is unrun (#125). | Shown as separation only |

## What this means for #13

T-NOW cannot yet carry all eight criteria.

- **Criterion 4** is not met at any tuple. This is now a design question; see the options memo.
- **Criteria 3 and 6** need new execution at this Coven:
  - a fresh route census;
  - a Cave harness repin and live dispatch.
- **Criteria 1, 2 and 5** need the three remaining required daemon targets run at this Coven to match the scope of their earlier acceptances.

To finish a common-tuple packet, in order:

1. Val decides the criterion-4 options.
2. The Coven owner implements the chosen recovery semantics, with daemon-boundary regressions.
3. Run all four required targets at the resulting Coven with the current Threads override active, on Linux and macOS.
4. Rerun the Set C route census there.
5. Repin the Cave harness to that producer and dispatch the live journeys.

Every later Coven or Threads commit makes a new tuple.

## Residuals

- The run is advisory and scheduled, not the required check.
- The artifact expires after 14 days. Archive it before then if it will be relied on.
- The route-literal count is a coarse indicator of drift, not a census.
- This packet reads retained evidence; it reran nothing at the daemon boundary.
