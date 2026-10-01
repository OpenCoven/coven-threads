# Status reconciliation — 2026-10-01

**Recorded by:** Echo (agent), at Val's request to bring current-status text
across repositories up to date.
**Scope:** reconcile this repository's current-status text with downstream
state that changed after the [2026-09-17 maintainer decisions](2026-09-17-maintainer-decisions.md).
This record accepts nothing and closes no gate. It changes no contract, pin,
schema, migration, or required check. Earlier dated reviews are not amended.

## Observations

Read on 2026-10-01 (UTC) from GitHub, git, and the local Beads board.

| Item | Observed state | Source |
| --- | --- | --- |
| OpenCoven/coven#1050 (`threads-xsd`): Linux owned-daemon HTTP-read fixture timeouts | Closed 2026-09-18 as fixed by OpenCoven/coven#1105 (`d415e7ed7bc4810ce3fed44c8dcb2fc2927db520`, merged 2026-09-15). The bead was closed 2026-09-20. | Closing comment on #1050 |
| OpenCoven/coven#1051 (`threads-4v8`): native Windows IPC and reservation latency | Closed 2026-09-20 on no-recurrence evidence. `Threads daemon tests (Windows)` passed on 3 of 3 completed `main` push runs after OpenCoven/coven#1132 (`0fab796c755221cb322c056d4ca05bd9b96b150a`, merged 2026-09-18). This is not a measured repair. The bead is still `blocked`. | Closing comment; runs 35393224472, 35493779048, 35500907951 |
| OpenCoven/coven#1047 (`threads-bp5.4`): cold Windows CLI startup outlier | Closed 2026-09-20 on the same no-recurrence evidence. The closing comment explicitly claims no measured improvement and leaves the five-second budget unchanged. It asks that a recurrence reopen the issue rather than file a new one. The bead is still `blocked`. | Closing comment on #1047 |
| Coven release provenance | The latest published release, `v0.4.4` (2026-09-14), pins Threads `0021fd2662d0b82328371ee3a6957d645f776fda` and contains accepted daemon `226bfcc8`. Tags `v0.4.5` and `v0.4.6` have no published release and carry the same pin. `v0.4.3` pinned `c102844`. | `crates/coven-cli/Cargo.toml` at each tag; GitHub releases |
| Downstream pin against Threads `main` | Coven `main` still pins `0021fd2`. Threads `main` (`7a94fe5`) is 65 commits later. The crate commits since then are `cecdb5b` (`threads-55s`), `07813e9`, `6a46f00`, and `99f1350` (`threads-vdv`; the constructor has no daemon emitter under Decision 2 of 2026-09-17), and `90a691b` (`threads-bv2`, test only). No decision to advance the pin is recorded. | `git rev-list`; `git log -- crates/` |
| Live Cave acceptance (OpenCoven/coven-cave#5256) | The `threads-live-daemon` Playwright project and its scheduled advisory workflow have landed (OpenCoven/coven-cave#5500, #5510). Nine scheduled days, Sep 22–30, are verified: 81 first-attempt journey passes across Chromium, Firefox, and WebKit (27 on Chromium), with zero retries or failures. The required 30-day Chromium window at or above 99.5% first-attempt is not yet satisfied. Promotion to a required check is a separate decision. | #5256 comment of 2026-10-01 |
| Human gates | #13 (`threads-uqx.9`) is open and has been dependency-clear since 2026-09-17. #14 (`threads-uqx.10`) is open and depends on #13. | GitHub; Beads |
| Deferred design | `threads-5rr` (familiar inbox and handoff ledger) is deferred until 2026-10-31. | Beads |

OpenCoven/coven#1067, intermittent macOS piped-lifecycle failures in the full
source suites, remains open. The #1047 closing comment calls it a different
failure shape. It has no Threads bead and is listed here only so it is not
mistaken for a closed follow-up.

## Effect on current-status text

- The native reliability follow-ups named in the 2026-09-17 record are closed
  on the Coven side. Two Threads bead mirrors still need reconciling, which is
  a maintainer action and is not taken here.
- `v0.4.4`, not `v0.4.3`, is the latest published Coven release carrying the
  Threads integration. That remains release provenance, not proof of full
  Phase-5 conformance.
- `threads-xpo` and `threads-lm4` were closed by Decision 2 of 2026-09-17, so
  the delivery strategy no longer names them as owners of open work.

## Remaining after this reconciliation

| Item | Bead | Waiting on |
| --- | --- | --- |
| Val coherence acceptance | `threads-uqx.9` (#13) | Val |
| Val scoped freeze or reaffirmation | `threads-uqx.10` (#14) | #13 |
| Live Cave acceptance window | none | OpenCoven/coven-cave#5256; 2026-10-21 at the earliest, if every scheduled day passes |
| Bead mirror reconciliation | `threads-4v8`, `threads-bp5.4` | maintainer |
| Whether Coven should advance its Threads pin | none | daemon lane |
| Familiar inbox and handoff ledger | `threads-5rr` | Val scoping after 2026-10-31 |
