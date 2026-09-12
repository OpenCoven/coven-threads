# Phases

> This page separates design approval, merged implementation, release provenance, and end-to-end proof. `[FROZEN]` means design complete and change-controlled; `[MERGED]` means present on downstream `main`; `[IN RELEASE TAG]` means included in a published release's source revision, not a claim about any running installation; `[ENGINEERING FROZEN]` means implementation complete at the recorded checkpoint. `[ACTIVE]`, `[BLOCKED]`, and `[NOT STARTED]` describe remaining work.
>
> **As of 2026-09-12 (UTC): phases 0–4 remain frozen, and Phase 5 remains active with four unresolved root remediation gates.** The Windows fixture, CLI deadline, and redundant store-checkpoint corrections now have native acceptance. Scheduled identity replay and both bounded auto routes have platform-scoped proof. The paired contributions still require owner integration; they do not close the root issues, install a required stable-pin compatibility lane, or supply either human coherence/freeze decision.

Read the [delivery strategy](strategy.md) for the order of work and the
[boundary remediation packet](reviews/2026-09-12-boundary-proof-remediation.md)
for the current candidate and evidence matrix. The earlier
[2026-09-12 handoff integration review](reviews/2026-09-12-handoff-integration-review.md) and
[2026-09-11 readiness review](reviews/2026-09-11-landscape-and-readiness.md)
retain the original findings. This page preserves historical evidence with its
original dates; a later observation does not transfer acceptance between heads.

## Current decision

Keep Phase 5 active. Use the
[boundary remediation packet](reviews/2026-09-12-boundary-proof-remediation.md)
for the current candidate and the [execution checkpoint](reviews/2026-09-12-execution-checkpoint.md)
for the earlier `65766b89` and `ea3f455a` observations. Their original failures,
results, and dates remain distinct.

OpenCoven/coven#969 merged at `edf3500883e72e4239ec7151904983f61a93b83f`.
The later daemon lineage binds supported scheduled identity evidence. Eight
valid-to-valid identity scenarios now prove stale rejection and fresh-authority
application across live deadlines and restart. The original Windows fixture
failure has native acceptance at daemon `63a49688`, with a separate
current-Threads observation of that identical daemon.

The remaining auto producer gap is implemented in paired #57 and
OpenCoven/coven#1029. The core code is `2c7a305e`; the combined daemon candidate
is `d01136f7`, based on the existing integration owner's `8d48bf79`. It requires
explicit tier-2 output-format policy, fresh deterministic regressions, staged
intake, and submission-bound replay. It does not make `MEMORY.md` auto-editable
or lower retired migration tiers. Earlier exact `2c7a305e` / `300d4068` Linux
evidence includes 71 auto scenarios; final combined-head native/current-core
receipts are tracked in the remediation packet. The separate CLI lifecycle
failure was repaired in `df0ccd91`: all six original native cases now pass,
with five diagnostic tests. Start/whole restart use one five-second deadline;
standalone stop/status remain two seconds. A different owned-serve fixture
then failed its 15-second readiness guard, with measured 13,436 ms store
initialization. OpenCoven/coven#1031 removed a reproduced redundant
checkpoint/reopen from that startup path without widening deadlines or
weakening durability. Final native run `34703393798` passes both workspace
and feature gates, including all six original CLI cases plus five diagnostics.
The physical cause of the earlier host timing outlier is not fully isolated;
the documentation-only passing rerun is not treated as its repair.

The owner merged OpenCoven/coven#1027 at its older `1294a893` head. Later
fixture finalization belongs to OpenCoven/coven#1029, not that merged PR's
native receipt. OpenCoven/coven#1022 and OpenCoven/coven#972 remain open.
The four root issues, full #13 acceptance, and human decisions remain open
even when a bounded contribution's tests pass.

Vocabulary (bound in [concepts.md](concepts.md)): **Thread** = authority relationship *surface → writer*; **Weave** = enforced pattern of threads; **Strand** = fiber inside a thread; **Channel** = axis of load.

## Release and verification evidence

[Coven `v0.4.3`](https://github.com/OpenCoven/coven/releases/tag/v0.4.3),
published 2026-09-02, resolves to
`8baa9c9b722a3a9553c6ed39b7e1ba2296ced95a`. Its
[`coven-cli` manifest](https://github.com/OpenCoven/coven/blob/8baa9c9b722a3a9553c6ed39b7e1ba2296ced95a/crates/coven-cli/Cargo.toml)
pins `coven-threads-core` to `c102844`. Its ancestry includes the scheduler
merge from OpenCoven/coven#430 and schema adoption from OpenCoven/coven#675.
[Cave `v0.4.1`](https://github.com/OpenCoven/coven-cave/releases/tag/v0.4.1),
published 2026-09-09, resolves to
`9366009e028ff10e7130c3d7824fe8d55e15323f`. Its ancestry includes the Phase-5 corrections from
OpenCoven/coven-cave#3628.

These are source-provenance facts, not certification of installed binaries,
deployment configuration, or all Phase-5 invariants. The historical claim that
no release contains Threads is obsolete, as is the `v0.1.2` dependency reference.

The readiness foundation merged in #34 (`db0049b`). The
[E2E contract](testing/e2e-contract.md) and
[compatibility manifest](../e2e/compatibility.toml) still distinguish core tests
from a required pinned daemon gate. OpenCoven/coven#931 is an **unmerged
integration checkpoint**, not full J1–J8 acceptance. Deterministic scheduler
control landed through OpenCoven/coven#968 at `066727f1`; supported scheduled
publication remains in unmerged OpenCoven/coven#972. Their earlier combined
real-daemon evidence is scoped to its recorded revisions. The stable
compatibility pin remains unchanged; local integration does not make an
unreviewed downstream revision a required gate.

**Bounded remediation landings, refreshed 2026-09-12:** protected intake
OpenCoven/coven#933 merged at `b7b3b4e17cfe21fd440e0f0429f3e38eae1785aa`;
typed-terminal recovery OpenCoven/coven#932 merged at
`aa527d2dbcd1edcda470483c1a91a519f24e8406`. Main now contains
`threads_protected_intake.rs` and `threads_terminal_recovery.rs`, but not the
full `threads_e2e.rs` target. The terminal target explicitly seeds legacy
opened-window history after public coherence intake; it does not establish
supported scheduled publication. The refreshed review records exact-head
receipts and remaining route/recovery coverage. Neither root issue is closed.

## Phase 0 — Design `[FROZEN]`

**Deliverable:** the design doc `specs/PHASE-0-DESIGN.md`, plus the repo scaffold and beads plan. No enforcement code was in scope, deliberately: *"If this doc is wrong, we would rather find out reading it than debugging it."*

**Status:** FROZEN v0.2, 2026-07-14, tag `v0.2-phase0-design`. The freeze gate was real: Sage's v0.1 draft, Echo's substrate-authority pass (which rebound the metaphor to correct referents and introduced the `Channel` enum), Nova's non-negotiables folded in, and a formal Nova sign-off including a verified round-trip against RFC-0001 §5.1 / §5.4 / §5.6. A post-freeze citation amendment (2026-07-15, bead `threads-986.12`) pointed C7's canonical home at the `coven-grimoire` Ward Layer Spec Brief §9 — a citation fix, not a design change.

The frozen doc is change-controlled. These docs describe it; they do not amend it.

## Phase 1 — Core crate `[FROZEN; IN RELEASE TAG]`

**Scope (beads `threads-986.6`–`.11`, Cody's Rust lane):** the `coven-threads-core` crate — `Strand`, `Thread`, `Weave`, `Channel`, `TensionState` types; the `PatternPredicate` trait with its derived `describe()` introspection; the hash-manifest layer (Merkle over strand hashes in canonical `(surface_path, writer_id)` order); and the RFC-0001 §5 conformance test suite mirrored into Rust.

**Status:** landed on `main` at `86550d8`; beads `986.6`–`.11` and the Phase 1 freeze (`threads-986.18`) are closed. The 2026-07-21 checkpoint recorded 205 tests (174 unit, 17 C7 round-trip, and 14 §5 conformance, plus one ignored doc-test). That historical count is not the current suite size. `unsafe_code = "forbid"` at the workspace level.

**What "implemented" does not mean:** the crate is a library. It has no side effects by design — no filesystem verification, no audit writes, no staging I/O. Until a daemon calls it (Phase 2), it enforces nothing.

## Phase 2 — Daemon integration `[FROZEN; IN RELEASE TAG]`

**Scope:** the validator call site inside the `coven` daemon's existing socket handling; the `DegradeToProposal` staging path at `~/.coven/pending/`; the `ward.audit` table live in `coven.sqlite3`; the notification protocol to the principal.

**Status, in two halves:**

- **Crate side — landed** (commit `5e68957`): `audit.rs` defines the `ward.audit` record shape and DDL (append-only via triggers, RFC-0001 §5.6 event vocabulary); `staging.rs` defines the pending-proposal record shape. The crate owns the *contracts*; the daemon owns the connection, the writes, and the directory.
- **Daemon side** `[IN RELEASE TAG]`: the validator call site, staging path, and audit table landed via OpenCoven/coven#382 and are present in the release ancestry above. The Phase 2 epic `threads-986.14`, freeze `threads-986.20`, and merge gate `threads-986.19` are closed. The merge gate was resolved by making this repository public so downstream CI could fetch the pinned dependency. This does not close the later protected-proposal route defect, `threads-dgg`.

The daemon calls the validator on protected edits. That integration does not
establish that every proposal, replay, or recovery route satisfies Phase 5.

## Phase 3 — Portability format `[ENGINEERING FROZEN; ENVELOPE DECIDED]`

**Scope:** the Coven Familiar Portability Format — the artifact a familiar exports to and imports from, with C7 enforced across the round-trip.

**Status:** the Phase 3 freeze (`threads-986.21`) and envelope decision (`threads-986.16`) are closed. `PortableWeave`, `SerializationContract`, and `export_weave`/`import_weave` implement the C7 round-trip and fail-visible behavior in `portability.rs` and `c7_roundtrip.rs`. The canonical format is `.weave`. The explicitly lossy, one-way `.af` exporter also landed: `threads-jq4` closed with merged #2 (`5b4a51a`). There is no `.af` import path. See `specs/PHASE-3-PORTABILITY.md` §6.

**Not `.af`-compatible: documented divergence.** The decided `.weave` format and lossy `.af` exporter do not provide a compatible `.af` round-trip surface. The reason is factual, source-verified 2026-07-14 against `letta-ai/letta/main/letta/serialize_schemas/pydantic_agent_schema.py`: Letta's `CoreMemoryBlockSchema` has no protection field, and the runtime `read_only` flag is stripped at export. An artifact format that cannot represent the protection contract cannot satisfy C7. Silent downgrade on import is precisely the failure mode C7 exists to refuse. This is a neutral engineering constraint, not a judgment of `.af` for its own goals; see the [FAQ](faq.md#why-isnt-it-af-compatible).

## Phase 4 — Coven Cave UX `[COMPLETE; FROZEN 2026-07-17]`

**Scope (epic `threads-986.17`, closed):** cockpit surfaces in Coven Cave — the weave rail view, a thread detail pane with tension state, strand inspection, and the proposal approval flow for staged writes from `~/.coven/pending/`. Charm owned the voice/copy pass on all four surfaces.

**Status:** **complete and frozen, 2026-07-17.** The surface contract is `specs/PHASE-4-CAVE-SURFACES.md` (#1). The weave rail, thread pane, strand inspector, and proposal approval flow merged in OpenCoven/coven-cave#3223. The recorded freeze gates were Charm's voice pass (`threads-986.17.7`), Nova's coherence sign-off (`.17.8`), and Val's UX acceptance (`.17.10`) and freeze (`.17.9`). The adapter follow-up `threads-v3g` is also closed, through OpenCoven/coven#408, OpenCoven/coven#409, and OpenCoven/coven-cave#3362. The earlier fixtures-first description is no longer current.

**Post-freeze addition:** degraded-familiar surfacing (`threads-k9s`, closed) — spec §2.7 `DegradedFamiliarView` + rendering rule §4.R12; daemon half merged as OpenCoven/coven#422, Cave half as OpenCoven/coven-cave#3415.

## Phase 5 — Approval semantics `[ACTIVE]`

**Scope (epic `threads-uqx`; spec `specs/PHASE-5-APPROVAL-SEMANTICS.md`):** the approval-ceremony layer over Gate 4 — typed approval paths, veto windows with delayed apply, semantic surface regions, and evidence replay at the deadline. Opened 2026-07-18 by Val+Nova decision.

**Design commitments** (stated here because they are easy to get subtly wrong):

- **`ApprovalPath` is orthogonal to `Channel`.** The typed `ApprovalPath` (auto / familiar-review / human / human-with-rationale) is the approval-ceremony axis. It is never derived from `Channel`, and `Channel` remains a first-class enforcement axis in its own right — neither is a descriptor of the other.
- **Delayed-apply only — no provisional apply.** A classified proposal is pending-visible for the whole veto window and is applied only after the deadline passes with no veto **and** the committed evidence replays to a match. Every window close carries an explicit reason.
- **Classification and scheduling are daemon-owned.** The crate defines the types and predicates; the daemon classifies, schedules, and applies.
- **Identity invariants are predicate-authoritative** (the descriptor-vs-predicate rule from [concepts.md](concepts.md) applies here too).

**Initial implementation ledger, reconciled 2026-09-11:**

- **Closed:** `.3` core approval types — `ApprovalPath`, `ApprovalPathKind`, `VetoWindow`, `ProposalClassification` (`approval.rs`); `.4` identity invariant predicates + advisory probes (`identity_invariants.rs`); `.5` `SurfaceRegionPredicate` + Gate-4 replay (`surface_regions.rs`); `.6` delayed-apply scheduler + audit — implemented **daemon-side in OpenCoven/coven#430** (daemon-owned classification and scheduler, deadline/minimum-visible revalidation, fail-closed committed-evidence replay, cross-platform conditional atomic writes, startup recovery); `.11` authority review findings resolved; `.2` RFC closure/provenance amendments; `.7` Cave veto-window contract; `.8` implementation and migration fidelity; `.12` RFC-0001 approval-tier alignment; `.13` authorized retired-Ward migration fixture. The proposal/decision-record PR #6 merged 2026-07-27 as `091607f`.
- **Open human gates:** `.9` Nova coherence sign-off and `.10` Val freeze. They are not the whole remaining implementation scope: the four remediation beads below still block sign-off. Agents must never simulate either decision.

**Nova's sign-off is BLOCKED, not pending (2026-07-29).** This is the single most important fact about Phase 5 and it is easy to miss from the bead counts alone. Nova ran independent core and integration reviews against coven `f3cd322`, coven-threads `091607f`, merged OpenCoven/coven#430 and OpenCoven/coven#464, merged OpenCoven/coven-cave#3581 and OpenCoven/coven-cave#3628, and OpenCoven/familiar-contract#3 and OpenCoven/familiar-contract#4, and **refused sign-off**. The design choices were explicitly affirmed as coherent — Channel and `ApprovalPath` remain separate axes, delayed apply is correct, Cave stays thin and fail-closed. What blocks is implementation, in five named beads:

| Bead | Blocking finding | Status |
|---|---|---|
| `threads-3jx` | `ward_audit` schema classification was substring-based | **closed** — PR #23, `8e2de93` |
| `threads-okc` | identity predicates must run at intake and delayed/restart replay | OpenCoven/coven#969 merged; bounded scheduled valid-identity proofs completed in `threads-vpp.2`; root integration/acceptance remains open in OpenCoven/coven#885 |
| `threads-980` | every opened window needs exactly one typed terminal close | bounded OpenCoven/coven#932 repair merged at `aa527d2d`; complete supported terminal/audit-chain acceptance remains blocked in OpenCoven/coven#886 |
| `threads-dgg` | protected `SOUL.md` must not stage/approve through a proposal route | bounded OpenCoven/coven#933 repair merged at `b7b3b4e1`; complete route/recovery acceptance remains blocked in OpenCoven/coven#887 |
| `threads-zav` | retired-Ward corpus must prove live schedulability and recovery | bounded Windows and auto work tracked in `threads-vpp`; broader publication/recovery remains with OpenCoven/coven#972 and OpenCoven/coven#888; new auto routes do not lower legacy migration tiers |

**Historical review:** Echo's 2026-08-09 static review at Coven `59c5be4`
confirmed the four findings then. It is not a current test result and cannot
waive independent review of subsequent changes.

**Draft prerequisite checkpoint, 2026-09-09:** the runtime prerequisites this section
previously described as missing are now implemented as unmerged drafts, not
absent: identity-predicate activation is draft OpenCoven/coven#969 (`0e94e9c`),
supported canonical scheduled publication with an explicit minimum-visibility
window is draft OpenCoven/coven#972 (`e139023f`), and an isolated deterministic
daemon-boundary test clock is draft OpenCoven/coven#968 (`8a4f2b7`). Two CI
prerequisites for exercising these against a real daemon are also drafted:
Windows daemon-lifecycle readiness (draft OpenCoven/coven#970, `cbd5626a`) and
authenticated Ubuntu-only apt source isolation (draft OpenCoven/coven#973,
`c731cada`, whose hosted run
[34388895012](https://github.com/OpenCoven/coven/actions/runs/34388895012)
passed in full). None of these five prerequisite drafts was merged at that
checkpoint. The September 12 continuation records later landings; this
paragraph does not describe current PR state.

**Hosted readiness evidence:** the initial Windows response-timeout repair
passed on OpenCoven/coven#970 at `9b591e4e`. A later pre-connect pipe timeout
on OpenCoven/coven#933 required another bounded retry fix, `cbd5626a`.
Its affected protected-route lineage at `3d2c06d0` now passes the full
[CI run 34394040609](https://github.com/OpenCoven/coven/actions/runs/34394040609),
including Windows and the PR gate. Scheduled publication at `e139023f` also
contains both Windows prerequisites. This is evidence for the named revisions,
not a blanket assertion about every subsequent draft head.

**Earlier published integrated evidence:** OpenCoven/coven#931 contained
`345d4cf0c2314ab70cbf843fee84e0db7776ec6d`, with this Threads checkout at
`c3bd46bc` proven by `cargo metadata`. The expanded authority/intake suite passed
218 tests. The complete feature-enabled Unix daemon target passed all 15 tests
on three consecutive runs. It covers canonical corpus migration and intake,
exact minimum visibility and deadline, restart, all five typed terminal
families, replacement-only apply after explicit supersession, no-window human
approval, contradictory human/opened-window history, invalid identity at
intake, unsupported duplicate-surface input, and reviewed drift. The bounded
write journey asserts a validation verdict and exact logged before/after hashes
and byte count.

The eight-case replay/restart loop covers veto, materialized drift, unavailable
Ward, changed/unavailable identity, rotated principal binding, and changed or
removed regional approval policy. Actual daemon failures exposed and drove
repairs for predicate failures without terminal closes, stale principal
binding, identity checks skipped at reviewed intake, stale approval policy,
and socket publication before private permissions were installed
(OpenCoven/coven#974). The socket repair retains client ownership checks and
passes deterministic no-clobber/private-publication regressions.

The bounded read-only engineering review of the integrated authority code found
no high-confidence introduced correctness defects. Hosted full-suite follow-up
also corrected a mismatched synthetic fixture and a prompt-failure test that
omitted a valid during-write termination outcome. These corrections preserve
the original refusal, failed-outcome, cleanup, and deadline assertions.

This is still bounded evidence: signed principal authorization, changed runtime
bindings, final authority-check/commit snapshot consistency, Windows daemon
journeys, Cave acceptance, and the reviewed required-pin gate are not certified. Linux
and the existing macOS push CI lane now invoke the feature-enabled daemon target.
Integration checkpoint `345d4cf0` passed the full
[hosted CI run 34396479737](https://github.com/OpenCoven/coven/actions/runs/34396479737),
including the Linux real-daemon step, Windows workspace suite, and PR gate.
Scheduled-publication checkpoint `e139023f` also passed its full
[hosted run 34396479783](https://github.com/OpenCoven/coven/actions/runs/34396479783).
The Windows workspace result is not Windows daemon-journey parity.
None of this closes the four root remediation gates or substitutes for Nova
or Val.

**Later integration evidence is not green:** the
[follow-up report on OpenCoven/coven#976](https://github.com/OpenCoven/coven/issues/976#issuecomment-5609267577)
records 11 passing and four failing default-parallel daemon tests at local
integration revision `299afacf`, including macOS transport error 35
(`Resource temporarily unavailable`). A
[serialized diagnostic rerun](https://github.com/OpenCoven/coven/issues/976#issuecomment-5609324318)
passed the four selected cases at the same revision, but it is not a production
fix or replacement acceptance result. The reports also identify a strict
home/socket-identity startup failure. These later results do not invalidate
the named hosted checkpoint above; they prevent extending its green status to
the continuing integration work.

**Current acceptance continuation:** OpenCoven/coven#978 (Windows owner-local
IPC) and OpenCoven/coven#979 (final validated-snapshot/commit binding) are merged
into draft OpenCoven/coven#931, not Coven main. The final-commit identity-drift
journey is wired into the shared harness. There are 15 daemon journeys on each
platform; the old standalone HTTP parser test moved to production-client
coverage.

At published daemon head `fd610f6`,
[run 34461979876](https://github.com/OpenCoven/coven/actions/runs/34461979876)
executed all 15 journeys on native Windows, with 23 passed scenario manifests,
including final-commit identity drift. Its actual synthetic merge was
`6daed594ffb36eea681adce0da090adaef48f812`; Windows used committed Threads
`c3bd46b`, not a current-checkout override. Overall CI remained red on separate
adoption-fixture timeouts.

The macOS socket-alias failure was reproduced and repaired at `42f5e71`:
discovery preserves the published socket leaf beneath the canonical home
instead of retaining a removed temporary hard-link alias. Ownership,
permissions, symlink rejection, and peer-identity checks remain intact.
Test-only cancellation diagnostics subsequently landed through
OpenCoven/coven#993 (`0a7040a7`), and fixture store initialization matching daemon
startup landed through OpenCoven/coven#996 (`98cfa9ce`). Neither change raises
deadlines or establishes a production cancellation/adoption defect.

The earlier combined head `d4653f14` passes 41 selected adoption/identity/commit
unit cases and all 15 default-parallel daemon journeys locally with current
Threads `2d21254` proven active by Cargo metadata.
[Run 34466364843](https://github.com/OpenCoven/coven/actions/runs/34466364843)
passes the Windows workspace and clock-feature unit steps, but fails four
daemon journeys on startup-health timeouts (11 passed, four failed). Available
setup-failure artifacts at that checkpoint did not establish the failing
startup phase. Subsequent retention repairs preserve failed startup events,
partial schema/status evidence, and fixed-category timing observations.

**Independently landed production repairs:** OpenCoven/coven#998
(`ce5e7b1367b2b15cc016b37a546f358685d05130`) makes runtime-evidence schema
initialization atomic, with caller-owned transaction and commit-failure
rollback coverage. Schema SQL and durability are unchanged.
OpenCoven/coven#1003 (`dabeab9556c2051890203620ac65685ce1d5156c`) lands the
macOS published-socket repair described above. These fixes are on Coven main,
not merely in the integration draft; neither establishes Windows startup
reliability or closes a Phase-5 remediation gate.

**Earlier hosted integration evidence:** draft OpenCoven/coven#931 at
`8576f41e6d622f63a3576b85bd2d3142776e59ca` passed
[CI run 34489425644](https://github.com/OpenCoven/coven/actions/runs/34489425644).
Native Windows executed 20 target cases: 15 daemon journeys and five
artifact regressions, plus 152 selected feature-unit cases. Artifact
`10157780426` contains 28 passed scenario manifests, all identifying the
synthetic merge `0d8ccfca1725475b5e45f2ec47d8c4c2c45bc89d`.
The Coven checkout was recorded clean, but the manifests record
`threads_dirty=true` and `local_threads_override_active=false`. The dirty flag
does not establish tracked source changes; its cause needs explanation.
Hosted Windows uses committed Threads `c3bd46b`, not a local-checkout override.
Earlier local override evidence remains separately scoped to its recorded head.
The manifests also omit `--features threads-test-clock` from their recorded
command, although the job log includes it. Preserve the actual invocation and
dependency state before using this as release-grade provenance.

OpenCoven/coven#1000 isolates evidence outside the build cache and qualifies
both storage roots and uploaded names by workflow run and attempt. Earlier
artifacts contained cached historical manifests; count only exact-provenance
evidence, and retrieve older commit-only artifact names by artifact ID.
The isolation implementation is verified in the draft, not yet on main.

Startup causation remains unresolved. In
[run 34482201141](https://github.com/OpenCoven/coven/actions/runs/34482201141),
two feature-off initial startups failed after pipe binding and before status
publication; the later feature-enabled target passed, without canceling those
failures. OpenCoven/coven#1001's bounded launch-budget and store-phase
measurement scope is now closed, with interpretation regressions merged
through OpenCoven/coven#1019 into draft OpenCoven/coven#931, not main.
Its two earlier successful diagnostic attempts did not reproduce
the failure. Diagnostic I/O can perturb timing; missing markers alone do not
prove blocking or process survival. No deadline increase, retry-to-green claim,
or first-attempt stability certification follows from these observations.
Broader reliability remains with OpenCoven/coven#884 and OpenCoven/coven#931; do not reopen the
completed measurement task merely to track that separate acceptance.

OpenCoven/coven#976 and OpenCoven/coven#977 remain open for reviewed integration
and acceptance. Independent engineering review of the bounded authority diff
is not Nova's coherence review. Earlier native green, local override evidence,
and component PR success cannot be transferred to a different combined head.
Browser acceptance still requires OpenCoven/coven-cave#5256's real-daemon
project, not mocked route composition or disappearance from a pending queue.

The final-commit binding obligation in OpenCoven/coven#977 is the
[E2E contract](testing/e2e-contract.md) section 7
snapshot requirement, not an additional requirement for globally simultaneous
multi-file visibility. J1 separately requires an atomic intended-file change.
Existing conditional-write regression tests do not, by themselves, prove that
unchanged identity sources and the committed approval binding share the final
validated snapshot.

Upstream Coven has merged the bounded consumer projection in
OpenCoven/coven#953 and verifiable receipt reads in OpenCoven/coven#975.
The production trusted runtime adapter remains owned by OpenCoven/coven#857.
The projection is evidence, not a new authorization path; its availability
does not certify changed-runtime or signed-authorization journeys. Engineering
approval does not replace the independent coherence review or freeze.

**Runtime prerequisite history and current continuation:** at Coven
`380e765e40e9f84771a805d51a64c06fe79c3110`, migration compiled retired identity
invariants but retained them only in the backup, with the active
[`WardConfig`](https://github.com/OpenCoven/coven/blob/380e765e40e9f84771a805d51a64c06fe79c3110/crates/coven-cli/src/ward.rs#L273-L293)
not activating them and
[`threads_gate.rs`](https://github.com/OpenCoven/coven/blob/380e765e40e9f84771a805d51a64c06fe79c3110/crates/coven-cli/src/threads_gate.rs#L188-L203)
supplying no candidate identity context; production staging at that checkpoint
still published legacy pending envelopes rather than a scheduled envelope
carrying classification and replay evidence. Draft OpenCoven/coven#969 targets
identity activation and draft OpenCoven/coven#972 targets scheduled publication;
both remain unmerged. Deterministic daemon-boundary time subsequently landed
through OpenCoven/coven#968 at `066727f1`. Windows/CI prerequisites landed through
OpenCoven/coven#970 at `4a04dc92`; OpenCoven/coven#973 was closed as superseded,
not independently merged. The integrated evidence above exercises
their supported runtime paths, but is not independent human closure. Do not
replace these contracts with test-only constructors or a new identity model.

**Draft checkpoint scope:** OpenCoven/coven#931 is now the combined integration
checkpoint, including cross-lane repairs not present on every standalone draft.
OpenCoven/coven#932 and OpenCoven/coven#933 have since landed bounded terminal and
protected-route repairs on main. Preserve those repairs when reconciling the
remaining overlapping integration commits; green individual branches are not
interchangeable with the integrated result.

**Related work outside the sign-off blocker set:** `threads-xpo` tracks the
missing daemon promotion-channel path; `threads-55s` depends on it for auditable
admission channels. `threads-ot6` is draft #25, with `threads-5mn` tracking its
upstream work reference and `threads-lm4` tracking runtime conformance.
`threads-76z` was reopened because its purported fix, #27, was closed **without
merge** and superseded by stricter terminal-close work. Do not resurrect its
null-close bypass. `threads-bnu` is closed via #28; `threads-t6t` records the
authorized privacy required-check activation after secret scanning landed in #41 and the
separate privacy job and source-reference correction landed in #40.
`threads-5rr` is an unratified design proposal, not
authorization to add an audit event.

### Repository governance: baseline active, boundary enforcement outstanding

#31 tracks required pinned daemon checks, deterministic time, OS/Cave acceptance,
Action SHA pins, and measured coverage/flake targets. The 2026-09-10 GitHub
snapshot reported `main` as unprotected and no repository rulesets.
On 2026-09-11, active [ruleset `22910327`](https://github.com/OpenCoven/coven-threads/rules/22910327)
("Threads main authority baseline")
enabled protection for `refs/heads/main`. Its original independent-review
requirement could not be fulfilled by the sole maintainer, who authored the
pending PRs. On 2026-09-12 (UTC), the maintainer explicitly authorized a
solo-maintainer policy, then separately authorized adding `Privacy policy guard`
as a required check. The active configuration has no bypass actors:

- Pull requests and resolved review threads remain required. Required approving
  reviews are zero; latest-push and extra unattributed-change approvals are
  disabled. Stale-review dismissal remains enabled for any reviews submitted.
- Five required checks are bound to GitHub Actions app `15368`: `Secret scanning`,
  `Rust quality and repository contract`, `Cargo test (compatibility baseline)`,
  `Nextest telemetry (JUnit, fail on flaky)`, and `Privacy policy guard`.
  Branches must be up to date.
- Branch deletion and force pushes are prohibited.

Explicit human approval must be recorded with its scope before an agent merges.
This is a maintainer process requirement, not a GitHub-enforced second-person
review. CI and agent reviews do not supply that approval. If another maintainer
joins, reassess independent review. To restore the original review controls,
set `required_approving_review_count` to `1`, `require_last_push_approval` to
`true`, and `require_extra_approval_for_unattributed_changes` to `true`,
preserving the later privacy required check.

The effective branch-rules API and `main.protected=true` confirm activation.
#46 merged this policy ledger at `91ff511609cfb717bf71c3cafe07c0cbb2a2a317`.
This is baseline governance, not completion of #31: the reviewed pinned daemon
E2E check is not yet available to require,
the scheduled latest-main canary is not wired, and coverage remains
informational. The solo-maintainer policy does not waive
the separate Nova coherence or Val freeze gates, or their engineering
prerequisites. [The authorization and exact policy change](https://github.com/OpenCoven/coven-threads/issues/31#issuecomment-5642501227)
are recorded on #31.

#38 merged as `af013612`; #37 is closed. It pins the existing
external CI actions and preserves explicit Rust 1.88.0 inputs in every
toolchain step. Its repository checker has regression coverage for mutable
refs, missing or divergent inputs, and unsupported declarations. It does not
add branch protection, a coverage ratchet, or a reviewed downstream gate.

#41 merged as `2d21254`, deploying checksum-pinned Gitleaks with index and
reachable `HEAD`-history scanning, sanitized diagnostics, and synthetic
regressions. Actual PR CI `34465028325` and merged-main CI `34465519559` passed.
The separate Copilot review workflow `34465035137` is not CI evidence.
PR #40 at `2a12c1ad6f813b636f49f1c17bbd3ff7cb65aaee` proposed a separate privacy
job and a maintainer-approved, provenance-preserving source-reference
correction. Its [CI run 34610860329](https://github.com/OpenCoven/coven-threads/actions/runs/34610860329)
passes both guards at that named head. Follow-up
`4fce2d9525cc966febb1a8db82587664a5a11c9a` corrects the stale `SECURITY.md`
rollout text and explicitly documents the checker-trust limitation.
The #40 follow-up passes [CI run 34647137078](https://github.com/OpenCoven/coven-threads/actions/runs/34647137078);
the #46 date correction passes [CI run 34647134180](https://github.com/OpenCoven/coven-threads/actions/runs/34647134180).
These results apply to their named heads. The combined privacy head
`2d17d0d4583801b1dfa831c7ca40a03930ceef80` subsequently passed
[CI run 34664678160](https://github.com/OpenCoven/coven-threads/actions/runs/34664678160).
#40 merged at `7168dae10f6b59bad8bc653b95f51911334ded74`; the job and
source-reference correction are now on `main`.

**Privacy required-check activation, 2026-09-12 UTC:** following
[explicit human approval](https://github.com/OpenCoven/coven-threads/issues/39#issuecomment-5643712162),
the existing `Privacy policy guard` context was added to ruleset `22910327`,
bound to GitHub Actions app `15368`. Pre-activation main
`3ff49c02abe5693a58529265fca5bf253f4f4844` already passed that exact check in
[CI run 34664925352](https://github.com/OpenCoven/coven-threads/actions/runs/34664925352).
Full ruleset read-back and the effective branch-rules API confirmed the single
addition; all prior settings were preserved. To roll back this activation,
remove only that context from the live required-check list, preserving every
other current setting. A documentation revert alone cannot change enforcement.

"Independent" means a separate job from secret scanning,
not an immutable checker that PR authors cannot modify. No guard exemption or
normative authority change is implied.

### Automation Authority Profile v1

The profile merged in #35 (`c3bd46b`). Profile version `1.0.0` is separate
from the Rust package's `0.2.0` version. Its schemas, Node-core reference
validator, exact conformance manifest, and vectors live under
`profiles/automation-authority/v1/`. It answers operation-specific automation
authority questions with four outcomes, separately from the Rust gate's three
verdicts. See the [profile guide](automation-authority-profile.md).

This is a shipped contract/reference implementation, not daemon scheduler,
credential-issuance, trusted-runtime, or consumer-adoption acceptance. Keep
those owner-specific integrations separate from Phase-5 closure.

## Summary table

| Phase | What it is | Status | Gate to next step |
|---|---|---|---|
| 0 | Design doc + scaffold | `[FROZEN]` v0.2, tag `v0.2-phase0-design` | — (done) |
| 1 | `coven-threads-core` crate | `[FROZEN; IN RELEASE TAG]`; Coven `v0.4.3` pins `c102844`; `.18` closed | Reviewed required stable-pin daemon gate remains open |
| 2 | Daemon integration | `[FROZEN; IN RELEASE TAG]`; `.14`, `.20`, and `.19` closed | Phase-5 route and replay defects remain separate |
| 3 | Portability format | `[ENGINEERING FROZEN]`; `.21`, `.16`, and exporter follow-up `threads-jq4` closed | No `.af` import or authority-preserving `.af` round-trip |
| 4 | Coven Cave UX | `[COMPLETE; FROZEN 2026-07-17]`; `threads-986.17`, adapter follow-up `threads-v3g`, and degraded-familiar follow-up `threads-k9s` closed | New Phase-5 live-daemon acceptance remains separate |
| 5 | Approval semantics | `[ACTIVE]`; sign-off refused 2026-07-29; four remediation beads remain unresolved, with draft harness and fix work underway | Real-daemon remediation evidence, then independent Nova sign-off (`.9`) and Val freeze (`.10`) |

## Known housekeeping discrepancies

Tracked in `docs/STATUS-2026-07-15.md` and worth knowing when reading the repo:

- **Historical license-plan drift:** `LICENSE` and Cargo package metadata
  specify MIT; the frozen design retains Apache-2.0 as its earlier plan.
  Reconcile that text through a maintainer decision before release. This audit
  changes neither the committed license nor the separate `PATENTS` file.
- **Historical Phase-0 planning:** `PHASE-0-DESIGN.md` Sections 7-9 retain the
  pre-freeze checklist and questions. Their historical-status annotation
  distinguishes them from current blockers without rewriting the decisions.
  Three questions were settled after the freeze: Section 9.2 through
  OpenCoven/coven#382, Section 9.3 by the Shape B decision in
  `specs/PHASE-3-PORTABILITY.md` Section 6, and Section 9.4 by the Phase-4
  contract and its recorded 2026-07-17 freeze. Section 9.1, whether federation
  needs a fourth `fabric` level, remains open and deferred.
- **`Channel::Deliberate` runtime reachability remains open.** The channel is
  specified in `PHASE-0-DESIGN.md` and implemented in the library, but
  `threads-xpo` still tracks the supported daemon promotion path. Draft #25
  explicitly labels `coven memory promote` as planned, not available. Do not
  turn the earlier daemon-wide symbol snapshot into an undated claim about
  every current code path.
