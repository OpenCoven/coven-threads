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
| 3 | Descriptor is not authority | `protected-proposal-route-prohibition` (403), `protected-rejection-audit` and `smoke-unsigned-protected-rejection` passed. A static dispatch census ran at this Coven with the Set C method (see [Criterion 3 census](#criterion-3-census-at-9415c309)). All 84 Set C arm headers are unchanged. Three arms were added. Two are `/internal/ward/power-loss`, compiled only under the `threads-test-clock` feature, and refuse any transport other than owner IPC. | One new arm, `/host/*` (Castle folder sync, #1809 and #1830), lets the daemon write peer-synchronized bytes under a folder the owner chooses, with no Ward gate. It refuses only overlap with the daemon home. A familiar workspace configured outside the home is not refused, and `SOUL.md`, `IDENTITY.md` and `ward.toml` are not in the sync exclusions. Val must classify this path as outside or inside criterion 3. Ten new Ward- or Threads-referencing files, including the #857 Runtime Authority decision modules, are not classified here. Static reading only. | **Not shown**: one unclassified write path |
| 4 | Audit completeness and typed reason | Supported intake: 51 of 51 opened windows have exactly one typed close, covering all five reasons, with 0 unresolved histories. | Damaged histories (quarantined, orphaned, untrusted, unprovable apply) have no close path at this Coven. The [options memo](2026-11-29-criterion-4-options.md) on this branch records why and proposes option A, which awaits Val's decision. `threads_terminal_recovery` was not executed here. The six #1716 journey families were executed only at Coven `346b7717`/`8edeb939` with core `aba2e226`. | **Not met** |
| 5 | Replay hash present and enforced | 20 `evidence_diverged` closes. `scheduled-window-surface-diverged`, `out-of-band-reviewed-drift` and the `output-auto-drift-*` families passed. From the retained audit rows, for all 51 windows (see [Criterion 5 binding](#criterion-5-binding-from-retained-rows)): the opening's `evidence_replay_hash_hex` equals the 32-byte submission `classification.evidence_replay_hash`; `replay_hash_matched` follows the Threads rule for every reason; and every one of the 13 `applied` closes is preceded by a `proposal-apply-intent` verdict carrying before-images and a 32-byte recovery commitment. | The retained rows do not carry the hash re-derived at close or the final target bytes, so the stored-to-re-derived-to-final-bytes chain cannot be checked from this artifact. `apply_audit` rows exist only for `Tier::Logged` writes (`ward.rs:2879`), which explains why 1 of 13 applied windows has one. `threads_identity_invariants` and `threads_protected_intake` not executed. | Partly shown |
| 6 | Minimum visibility and reachable pending state | Daemon side: `retired-corpus-scheduled-restart` passed. It asserts that the durable pending record and the submission audit bind the same classification, that `earliest_close` and the deadline equal the corpus policy, and that the target bytes are unchanged one second before minimum visibility, across a restart (Coven `tests/threads_e2e.rs:405-460`). | No Cave journey exists at this producer. The Cave harness pins `b07935f0`. Showing it needs a reviewed Cave harness repin to `9415c309` and a dispatch, and that dispatch was not authorized tonight. | **Not shown** |
| 7 | Load-time label/variant mapping | All 17 `approval_tier_load_*` journeys: the 4 valid configurations returned 200; the unknown tier and 12 mismatches returned 500 `ward_config_invalid`. The pattern matches T-LOAD. | Linux only; not every load-error class, the same bound as T-LOAD. | Shown, within that bound |
| 8 | Advisory probes separated, Gate-3 only | `output-auto-advisory-isolation` and the `output-auto-regression-gates` and `-floors` journeys passed. | No model probe is wired into the daemon, so separation is vacuous at the daemon, as at T-SEP. Kani is unrun (#125). | Shown as separation only |

## What this means for #13

T-NOW cannot yet carry all eight criteria.

- **Criterion 4** is not met at any tuple. This is now a design question; see the options memo.
- **Criterion 3** has a rerun dispatch census at this Coven. It needs a classification of the `/host/*` Castle sync path, and a classification of the ten new Ward- or Threads-referencing files.
- **Criterion 6** needs a Cave harness repin and a live dispatch.
- **Criteria 1, 2 and 5** need the three remaining required daemon targets run at this Coven to match the scope of their earlier acceptances. Criterion 5 also needs the re-derived hash and final bytes retained, or asserted, per reviewed path.

To finish a common-tuple packet, in order:

1. Val decides the criterion-4 options and classifies the Castle sync path for criterion 3.
2. The Coven owner implements the chosen recovery semantics, with daemon-boundary regressions. If Val places Castle sync inside criterion 3, the same owner adds a refusal of Castle roots that overlap a familiar workspace.
3. Run all four required targets at the resulting Coven with the current Threads override active, on Linux and macOS.
4. Rerun the dispatch census there with the scripts below, and classify every new Ward- or Threads-referencing file.
5. Repin the Cave harness to that producer and dispatch the live journeys.

## Criterion 3 census at `9415c309`

This census uses the method of the Set B packet (`2026-10-13-coherence-packet-f772b513.md`, "Dispatch census"). It counts the arm headers of `match (method, route.as_ref())` in `crates/coven-cli/src/api.rs`, meaning lines starting with eight spaces and `(`, and then diffs the header text. I read the source with `git show <rev>:<path>` and changed no Coven checkout.

| Revision | Match span | Arms | Default arm |
| --- | --- | --- | --- |
| `c658603d` (Set C) | `L885-1478` | 84 | yes |
| `9415c309` (T-NOW) | `L938-1557` | 87 | yes |

The header diff contains exactly three additions and no removals or edits.

| New arm | Location | Reading |
| --- | --- | --- |
| `(_, route) if route.starts_with("/host/")` | `api.rs:950` → `host_sync.rs` (`Workers::route` `L44`, `dispatch` `L426`) | Refuses anything other than `OwnerLocalIpc` (403 `owner_local_required`). `POST /host/init` binds a Castle root and refuses overlap with the daemon home (`L431-435`). Sync and resolve then write peer-committed bytes under that root through `coven-host` `d031b333`. Its name filter (`lib.rs:18-33`) excludes dotfiles, credentials and similar names, not `SOUL.md`, `IDENTITY.md` or `ward.toml`. Familiar workspaces can be configured outside the daemon home (`cockpit_sources.rs:96-118`), and no check compares the root with them. There is no Ward or Threads call on this path. |
| `(_, "/internal/ward/power-loss") if !authority.allows_ward_proposal_access()` | `api.rs:1132` | `#[cfg(feature = "threads-test-clock")]`; refusal-only 403 `transport_forbidden`, `writeApplied: false`. The feature is not in `default` (`crates/coven-cli/Cargo.toml`). |
| `("POST", "/internal/ward/power-loss")` | `api.rs:1192` → `ward_power_loss_fixture_response` (`L15308`) | The same feature gate. It controls the synthetic power-loss fixture (#1640). It is absent in a default build, which a Coven test asserts (`api.rs:38343`). |

**Whole-tree census.** I grepped `crates/coven-cli/src` with the Set B pattern set. The set of files that match grew from 17 to 29:

- `api.rs` split: `api/proposal_{apply_intent,decision_origin,document,preflight}.rs` and their tests.
- #857 Runtime Authority: `automations/threads_decisions.rs` and `automations/trusted_authority.rs`. These decide automation dispatch through `coven_threads_core::automation_authority`, not protected-edit proposals. They still need classifying.
- `proposal_store.rs` and its scheduler tests.

**Grep census.** Each pattern was counted over `api.rs` plus `api/*.rs`, test files excluded, with the same method at all three revisions. These whole-file counts are not comparable to the production-range table in the Set B packet.

- `f772b513` and `c658603d` give identical counts for all 15 patterns.
- From `c658603d` to `9415c309`, only four counts change:
  - `trusted_decision_approver` +2: a visibility and import change from the module split.
  - `allows_*(` +1: the power-loss guard.
  - `api_error(` +8: the power-loss fixture (5) and host sync (3).
  - `bail!|ensure!` +3: `writeScope` launch checks (#1651, owner-IPC only, `api.rs:4162-4172`) and a runtime-envelope check.
- The count of `INSERT INTO ward_audit` is unchanged. The new `ward_manifest` upsert at `api.rs:469` is inside `#[cfg(test)]`.

## Criterion 5 binding from retained rows

I checked run `46036830391` (artifact SHA-256 above) offline with `python3 -I`, keeping the scripts outside the artifact directory.

- **Windows:** 51 opened, with exactly one submission, one opening and one terminal each, and no terminal recorded before its opening.
- **Hashes:** 51/51 submission hashes are 32 bytes, and 51/51 openings carry the same hash as hex.
- **Close detail:** 51/51 follow `validate_window_close_detail`: `applied` with `Some(true)`, `evidence_diverged` and `revalidation_failed` with `Some(false)`, and `vetoed` and `superseded` with `None`.
- **Apply intent:**
  - All 13 `applied` windows have a `proposal-apply-intent` verdict before the terminal, with before-images and a 32-byte recovery commitment.
  - 3 rejected windows also have an apply intent before a typed close: `final-commit-identity-drift` and `output-auto-final-policy` (`revalidation_failed`), and `output-auto-final-surface` (`evidence_diverged`). In each, drift found at final commit, after the intent was written, closed the window live. If that live close is missing, option A in the memo holds this history shape instead of closing it.
  - The other 35 non-applied windows have none.
- **Not retained:** the re-derived hash at close, the final target bytes, and the pending record (except where a journey asserts it in Coven's own test).

Every later Coven or Threads commit makes a new tuple.

## Residuals

- The run is advisory and scheduled, not the required check.
- The artifact expires after 14 days. Archive it before then if it will be relied on.
- The criterion-3 census is static. It inspects only the three new router arms in depth. It does not reread the bodies of the 84 unchanged arms, although a body can change without its header changing. The `/host/*` reading was not exercised at runtime.
- The `coven-host` source was read from the local Cargo checkout at the pinned rev `d031b33`, not from a fresh clone.
- This packet reads retained evidence; it reran nothing at the daemon boundary.
