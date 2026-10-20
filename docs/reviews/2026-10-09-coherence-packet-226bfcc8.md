# EVIDENCE PACKET — NOT AN ACCEPTANCE

OpenCoven/coven-threads #13 (`threads-uqx.9`, bead `threads-uqx.14` refresh): the eight-item Phase-5 coherence mapping at pinned versions.

This packet is engineering evidence only. It does not accept, reject, or defer anything. It does not check a #13 box, and it does not stand in for Val's human self-review under `specs/PHASE-5-SOLO-MAINTAINER-REVIEW.md`. Each verdict says only whether the sources and tests found at the pinned SHAs support the checklist text.

| Field | Value |
| --- | --- |
| Date | 2026-10-09 |
| Coven daemon | `226bfcc89ff6cad4bc9cc9618dad6fea970ecf58` (OpenCoven/coven#1049, landed on main 2026-09-13T17:03:23-05:00) |
| Threads core | `0021fd2662d0b82328371ee3a6957d645f776fda`. The daemon's `Cargo.lock` pins exactly this revision (`coven-threads-core 0.2.0`, `git+…?rev=0021fd26…`). |
| Cave | `258e587f9edce9cdfe200d6f5aff6076fdb2c645` ("Persist desktop-managed tailnet device approvals", committer date 2026-09-13T11:05:39-05:00) |
| Tests re-run | **None.** No cargo, pnpm, or other build or test was run. Test names come from source at the pinned SHAs. Results come only from the CI runs cited. |
| Method | Read-only `git show/log/grep/ls-tree/merge-base/cat-file/diff` and read-only `gh run/issue/api` |

## Why this Cave SHA

The rule is the last commit on Cave `origin/main` (first-parent) whose committer date is on or before the Coven `226bfcc8` committer date (epoch 1789337003, 2026-09-13T22:03:23Z). Committer dates along Cave's first-parent main are monotonic: no step goes backwards. The selected commit is `258e587f` at 16:05:39Z. The next main commit is `8a06421a705c2d7891c3f44cc580c569f6cbe2c1` (#5388, 17:06:14-05:00), which landed about 3 minutes after the cutoff.

The 2026-09-13 engineering-acceptance doc cites Cave `a0f43d3cd141ad4f3e391991dd4ceb4544a71226`. That commit is not on Cave's first-parent main and is not an ancestor of `258e587f`. However, `git diff a0f43d3c 258e587f` over every Threads-facing Cave file listed below is empty, so both SHAs carry the same Threads surfaces.

**Current Cave main, note only.** `6d988ec277bfb623667f511214f8fcfb2c76d1d8` (2026-10-09) descends from `258e587f`. Between the two, the Threads surfaces changed by +420/−59 lines across 8 files. The changes touch `threads-adapters.ts`, `proposal-authority.ts`, `proposal-flow.ts`, `proposal-approval.tsx`, `weave-rail.ts`, and `api/proposals/*`, including a new `api/proposals/submit`. Relevant commits: `adba56809`, `25740a969` ("Add real-daemon proposal journeys and verified terminal receipts"), `8388ce751`, `f1648e72a`, `7e1f1efff`, `c9388e64c`. Nothing in this packet was mapped at `6d988ec2`.

## CI on the exact target SHAs

These runs apply to every item below. Only runs whose `headSha` is exactly a target SHA are listed as exact-SHA evidence.

| Repo / SHA | Run | Workflow | Conclusion |
| --- | --- | --- | --- |
| coven `226bfcc8` | [34785628407](https://github.com/OpenCoven/coven/actions/runs/34785628407) (push, attempt 1) | CI | **failure** |
| coven-threads `0021fd26` | [34720725886](https://github.com/OpenCoven/coven-threads/actions/runs/34720725886) (pull_request) | CI | success (all 6 jobs) |
| coven-threads `0021fd26` | [34720762231](https://github.com/OpenCoven/coven-threads/actions/runs/34720762231) (workflow_dispatch) | Daemon observation (advisory) | success |
| coven-cave `258e587f` | [34767545639](https://github.com/OpenCoven/coven-cave/actions/runs/34767545639) (push) | CI | success (all 21 jobs) |
| coven-cave `258e587f` | [34767545572](https://github.com/OpenCoven/coven-cave/actions/runs/34767545572) | CodeQL | success |
| coven-cave `258e587f` | Main health 34768571753, 34768674692, 34778037998 | Main health | success |
| coven-cave `258e587f` | Main health 34767545982, 34777478125 | Main health | action_required |
| coven-cave `258e587f` | Main health 34785740857 | Main health | skipped |
| coven-cave `258e587f` | Recover missing PR CI 34773656879, 34781733099 | Recover missing PR CI | failure |

**Coven exact-SHA run 34785628407, job detail.** Rust tests (Linux), Rust tests (macOS), and Rust lint (Linux) passed. Three jobs failed:

- **Threads daemon tests (Windows):** 80 passed, 1 failed. The failure is `threads_e2e::output_auto_cases::output_auto_deadline_and_restart_reject_regression_and_valid_evidence_drift`, with "fixture daemon request POST /api/v1/familiars/sage/edits … no response bytes arrived before the deadline".
- **Rust tests (Windows):** 35 passed, 1 failed. The failure is `failure_artifacts_retain_failed_restart_cli_output`, with "timed out waiting for Coven daemon startup health".
- **CLI performance baseline:** `coven-perf.json` was missing or empty.

The PR gate failed as a result. No Threads doc on `origin/main` cites this run; a grep for `34785628407` found nothing. These failures were not re-diagnosed here. They resemble the native reliability findings tracked in OpenCoven/coven#1047, #1050, and #1051, but that link is an observation, not a finding.

**Runs that are not on the exact SHA.** These are listed for context and are not counted as exact-SHA evidence:

- Coven [34784497791](https://github.com/OpenCoven/coven/actions/runs/34784497791) passed on PR head `98e89c4f8b3975a0feb939dccb7afb4dcb480eff`. That head's tree is `db70ae955bc71bdf444889e5ffbe1723708f5203`, which I verified locally is identical to `226bfcc8`'s tree. The acceptance doc cites this run as the accepted native CI.
- Threads [34786471550](https://github.com/OpenCoven/coven-threads/actions/runs/34786471550) ("Daemon compatibility") passed on Threads head `a1545cf191a62411684f9ddbda56b9d1e5975f1b` against daemon `226bfcc8`. `git diff 0021fd26 a1545cf1 -- crates/ Cargo.toml Cargo.lock` is empty.

---

## Item 1 — RFC round-trip

> "every design decision in `specs/PHASE-5-APPROVAL-SEMANTICS.md` §6 maps cleanly to a type or predicate in the implementation beads. No decision is orphaned or contradicted."

The §6 decisions are taken from `coven-threads@0021fd26:specs/PHASE-5-APPROVAL-SEMANTICS.md:L318-395`.

| §6 decision | Type / predicate / call site |
| --- | --- |
| 1. `ApprovalPath` separate from `Channel` | `coven-threads@0021fd26:crates/coven-threads-core/src/approval.rs:L82-114` (`enum ApprovalPath`). `ProposalClassification` (`approval.rs:L405-431`) carries both `channel` and `approval_path`. The daemon derives the path from region bindings, never from the channel: `coven@226bfcc8:crates/coven-cli/src/ward.rs:L589-727` (`WardConfig::compiled_approval_tiers`) and `coven@226bfcc8:crates/coven-cli/src/api.rs:L7906-7937` (`revalidate_scheduled_approval_policy`, which uses `ApprovalPath::highest`, `approval.rs:L158-196`). |
| 2. Delayed apply only, with a close reason field | `WindowCloseReason` (`approval.rs:L471-485`) and `tag()` (`L489-497`). `ProposalWindowCloseAuditDetail` (`L537-546`). `ProposalApprovalAuditDetail` (`L553-560`). The daemon never applies before the deadline. The scheduler sets `due` only when `now >= veto_deadline` (`api.rs:L13203-13214` inside `process_due_threads_proposals`, `L12918-13261`). `proposal_decision_semantics` (`api.rs:L7725-7858`) refuses `approve` with `proposal-minimum-visibility-open` or `proposal-veto-window-open`. No provisional-apply or rollback-on-veto path was found. |
| 3. Classify regions first; promotion is forward-only | `ProposalClassification.affected_regions` (`approval.rs:L405-431`). `SurfaceRegionPredicate` (`surface_regions.rs:L318-326`). `SurfaceRegionRegistry::classify_all` and `path_tier_floor` (`L595-647`). Daemon cross-checks: `proposal_scheduler.rs:L270-316` (`validate_region_evidence`). Threads stay source-bound. **Region-to-thread promotion code: not found.** No promotion exists to inspect, so the forward-only rule is vacuous at these SHAs. |
| 4. Deterministic where possible; ambiguity fails closed | `IdentityInvariantSet::compile` and `evaluate` (`identity_invariants.rs:L211-307`). `IdentityAwarePattern` (`L343-374`). `AdvisoryProbes` is a separate type (`L885-915`). Daemon probes are the four deterministic `ProbeId`s (`coven@226bfcc8:crates/coven-cli/src/ward.rs`, `enum ProbeId`). The tests below cover the fail-closed behaviour. |
| 5. Upstream RFC amendments land before freeze | A process decision with no implementation type. The cited familiar-contract PRs #3 and #4 are **not found in the inspected repositories** (familiar-contract was not inspected). |
| 6. Daemon contract before Cave | The daemon emits `ApprovalPathWireEnvelope::from_classification` in `threads_proposals_response` (`api.rs:L7586-7603`, function `L7347-7652`). Cave consumes it and checks it against staged authority: `coven-cave@258e587f:src/lib/proposal-authority.ts:L78-83,L801-822,L975-1015`. |
| 7. Display labels preserved as a daemon wire contract | See Item 7. |
| 8. `proposal_window_opened` audit event | `AuditEventType::ProposalWindowOpened` (`coven-threads@0021fd26:…/src/audit.rs:L141-174`). Detail is validated in `WardAuditRecord::validate_event_detail` (`audit.rs:L413-541`). The daemon writer is `ensure_proposal_window_opened_audit_with_conn` (`api.rs:L13582-13662`). |

**Covering tests at these SHAs:**

- **Threads `approval.rs` tests:** `highest_ceremony_wins`, `cross_variant_highest_merges_the_losing_paths_veto_window`, `human_paths_drop_lower_windows_by_design`, `proposal_classification_roundtrips_json`, `window_close_reason_tags_are_stable`.
- **Threads `identity_invariants.rs` tests:** `missing_candidate_extraction_fails_closed`, `stale_candidate_fact_extraction_fails_closed`, `compiler_requires_name_and_person`, `retired_invariant_declarations_compile_with_full_fidelity`.
- **Threads `tests/phase5_retired_ward_corpus.rs`:** `valid_cases_cover_identity_approval_veto_and_region_fidelity`, `unsupported_cases_fail_closed_for_each_authority_input`.
- **Coven `proposal_scheduler.rs` unit tests:** `protected_path_floor_is_not_schedulable`, `reviewed_tier_cannot_use_auto_approval`, `region_tier_zero_cannot_be_hidden_by_classification`.
- **Coven `tests/support/threads_corpus_closure_cases.rs`:** `unsupported_retired_identity_declaration_never_migrates_or_stages`.
- **Coven `api.rs` unit test:** `threads_scheduled_replay_rejects_changed_live_approval_policy`.

**CI:** see the exact-SHA table. The coven run failed overall; the Threads and Cave CI runs passed.

**Verdict: supported.** Decisions 1, 2, 4, 6, 7, and 8 each map to named types or predicates, and no contradiction was found.

**Gaps:**

- Decision 3's forward-only promotion has no implementation to inspect.
- Decision 5 is a process decision and was not verified in its external repository.
- The 2026-09-11 "supported auto-path reachability" gap is now addressed by the bounded output-format route: `coven@226bfcc8:crates/coven-cli/src/output_format_auto.rs:L12-136` and Threads `OutputFormatRegion` (`surface_regions.rs:L510-594`). This is the only supported `AutoRegression` producer found.

---

## Item 2 — Gate-4 fail-closed proof

> "every `ApprovalPath` ends in daemon re-materialization. No path allows apply without live revalidation at deadline."

**Source.** All four variants are applied only through `decide_threads_proposal_inner` (`coven@226bfcc8:crates/coven-cli/src/api.rs:L10083-12657`):

- `AutoRegression{veto:None}` and the veto-bearing paths enter through the scheduler's `decide_threads_proposal_automatic(…,"approve")` (`api.rs:L13203-13221`).
- The human paths enter through `POST /api/v1/threads/proposals/{id}/approve`.

Inside that function, before any write:

1. **Live identity:** `ward_identity::candidate_rejection` and `candidate_binding` must equal the stored `identity_evidence` (`api.rs:L11876-11891`).
2. **Auto-regression evidence:** `revalidate_auto_regression` (`L15496-15544`) and `revalidate_auto_regression_evidence` (`L15546-15579`) recompute `output_format_auto::regression_evidence` from the live config (`output_format_auto.rs:L72-115`). They compare it with the submission receipt and the authoritative probe projection.
3. **Live tier escalation**, then `revalidate_scheduled_materialized_before` (`api.rs:L7875-7904`). This re-reads every live surface and requires exact before-image bytes.
4. **Live approval policy:** `revalidate_scheduled_approval_policy` (`L7906-7937`).
5. **Final commit:** `ensure_proposal_final_authority_unchanged` (`L15581-15617`) runs as the `final_authority_check` closure inside `apply_after_review_approval` for both the Initial apply (`L12480-12510`) and the Recovery apply (`L12074-12110`).

Each refusal path writes a typed terminal (`EvidenceDiverged` or `RevalidationFailed`). The persisted envelope is re-derived on every load: `ScheduledProposal::try_new` (`proposal_scheduler.rs:L52-134`) and the `Deserialize` impl.

**Covering tests:**

- **Coven `tests/threads_e2e.rs` (real daemon):**
  - `scheduled_window_replay_fails_closed_across_real_daemon_restart`, with 8 scenarios: vetoed, surface-diverged, ward-unavailable, identity-changed, identity-unavailable, binding-revoked, approval-policy-changed, approval-policy-removed.
  - `out_of_band_reviewed_drift_is_refused_without_execution`
  - `reviewed_human_approval_validates_and_audits_without_veto_window`
  - `changed_human_path_cannot_erase_a_real_opened_window`
  - `retired_corpus_scheduled_intake_survives_restart_and_applies_once`
- **Included support cases:**
  - `output_auto_deadline_and_restart_reject_regression_and_valid_evidence_drift`
  - `output_auto_final_commit_rechecks_policy_and_expected_image`
  - `output_auto_no_veto_survives_restart_without_window_events`
  - `final_commit_identity_drift_closes_opened_window_without_applying`
  - `scheduled_valid_{identity,soul,roster_metadata,active_policy}_*_reject_at_deadline_and_after_restart`
- **Coven `tests/threads_identity_invariants.rs`:** `scheduled_identity_replay_covers_manual_and_immediate_routes` (lanes human_review, human_required, auto), `direct_identity_drift_at_commit_refuses_real_daemon_write`, `changed_valid_identity_evidence_refuses_approval_after_restart`.
- **Coven `api.rs` unit tests:** `threads_scheduled_deadline_replay_refuses_diverged_before_image`, `threads_scheduled_deadline_replay_failure_closes_window`, `threads_scheduled_human_required_enforces_rationale_and_applies`, `threads_scheduler_revalidates_old_window_instead_of_expiring_it`.

**CI:** coven exact-SHA run 34785628407 **failed** in the Windows Threads daemon job. The failing test is one of this item's covering tests (`output_auto_deadline_and_restart_reject_regression_and_valid_evidence_drift`). The same-tree, non-exact-SHA run 34784497791 passed.

**Verdict: supported** (source and tests), with the CI caveat above.

**Gaps:**

- The only exact-SHA hosted run failed on a covering auto-path test (an HTTP timeout on Windows). The root cause was not determined here.
- "Re-materialization" is implemented as live before-image byte equality plus re-derivation of the replay hash from the stored diff, not as a fresh diff hash of live state. See Item 5.
- The non-scheduled legacy review kinds (`PendingReviewKind`) follow a separate `validate_fail_closed` route (`api.rs:L12279-12300`). That route was not exhaustively mapped.

---

## Item 3 — Descriptor-not-authority check

> "no Cave or client-side surface is enforced on. Every enforcement point traces to a daemon-owned predicate or audit row."

**Source, daemon side:**

- Descriptors are labelled derived and non-authoritative: `SurfaceRegionDescriptor` (`coven-threads@0021fd26:…/surface_regions.rs:L290-301`) and `PatternDescriptor` (`pattern.rs:L89`).
- Decisions use predicate output: `classify_all`, `IdentityInvariantSet::evaluate`, `validate_fail_closed`.
- The daemon uses `SurfaceRegionRegistry::descriptors()` only to enumerate the daemon-owned region-id set during config compilation (`ward.rs:L612-616`).
- The daemon re-derives approval semantics itself (`proposal_decision_semantics`, `api.rs:L7725-7858`) and ignores client claims.

**Source, Cave side (`coven-cave@258e587f`):**

- `src/lib/threads-adapters.ts`: `DaemonThreadsAdapter.decide` forwards only. Its comment reads "This adapter never mutates anything itself". It POSTs `/api/v1/threads/proposals/{id}/{approve|reject}` with `expectedRevision` and `note` (about L583-617), and `blockedFromDecision` (about L625-634) surfaces daemon refusals.
- `src/lib/proposal-authority.ts`:
  - `APPROVAL_PATH_LABELS` (L78-83) is a mirror table used only to block on mismatch (L801-810).
  - `normalizeAvailableDecisions` (L877-896) derives which buttons appear from the daemon lifecycle and variant.
- The decision routes `src/app/api/proposals/[id]/{approve,reject}/route.ts` are 20-21 lines each and forward-only.

**Observations, not enforcement:**

- Cave refuses to forward when the pending file is missing or corrupt locally. This only narrows what Cave will forward; it grants nothing.
- Cave reads `coven.sqlite3` `ward_audit` read-only and lists `pending/` directly (`threads-adapters.ts` `audit()` about L517-553, `proposals()` about L556-580), bypassing the daemon API for reads.

**Covering tests:**

- **Coven `tests/threads_protected_intake.rs`:** `explicit_proposal_and_stage_routes_are_not_alternate_write_endpoints`, `fingerprints_and_invented_approvals_never_gain_protected_authority`, `protected_refusal_audit_is_durable_but_cannot_be_replayed_as_approval`, `genuine_prior_approvals_cannot_authorize_other_familiar_project_or_target`.
- **Coven `threads_e2e.rs`:** `protected_proposal_routes_never_gain_write_authority`.
- **Coven `api.rs` unit test:** `threads_scheduler_rejects_forged_automatic_human_approval_origin`.
- **Threads unit tests:** `execution_prompt_region_describe_is_derived_and_non_authoritative`, `familiar_name_invariant_describe_is_derived`.
- **Cave `src/lib/threads-adapters.test.ts`:** "approve forwards the exact expected revision and note without touching the pending file", "blocks a joined proposal when daemon metadata mismatches staged authority", "R5: approve and reject refuse in fixtures mode — no daemon, no decision".
- **Cave `src/app/api/proposals-flow-e2e.test.ts`:** "decision routes preserve their local guard and forward the shared parser result exactly", "R6: a corrupt staged file answers 409 and the daemon is never asked".
- **Cave `src/lib/proposal-flow.test.ts`:** "renders verified reject-only veto authority as one Veto action", "409 proposal-refused warns that daemon revalidation may consume the pending proposal".

**CI:** Cave CI 34767545639 passed. The coven exact-SHA run failed (see the table).

**Verdict: partial.**

**Gaps:**

- The inspected enforcement points all trace to the daemon, but no exhaustive enumeration of every daemon enforcement point in the 49,903-line `api.rs` was done here.
- Cave carries client-side policy derivations: its mirror label table and available-decisions derivation. These control display only; the daemon re-decides.
- Cave reads daemon-owned state directly from disk rather than through the daemon API.
- **Live Cave acceptance at `258e587f`: not found.** It is tracked separately in OpenCoven/coven-cave#5256.

---

## Item 4 — Audit completeness

> "`proposal_submitted → window_opened → close(reason)` chain is present and no gap exists between open and close. Close event has reason field."

**Source:**

- **`proposal_submitted`:** written at staging by `threads_gate::append_scheduled_submission_audit` (`coven@226bfcc8:crates/coven-cli/src/threads_gate.rs:L1645`ff; `scheduled_submission_detail` at `L1628`).
- **`proposal_window_opened`:** written lazily, not at submission, by `ensure_proposal_window_opened_audit_with_conn` (`api.rs:L13582-13662`), with `INSERT … WHERE NOT EXISTS` (idempotent). It is called:
  - on the first scheduler pass (`api.rs:L13031`, `L13099`, `L13201`);
  - at decision time, before any terminal on a veto path (`api.rs:L10991-11003`).
  Detail fields come from `ProposalWindowAuditDetail` (`approval.rs:L517-530`).
- **Close:** `ProposalWindowCloseAuditDetail.reason: WindowCloseReason` (`approval.rs:L537-546`).
- **SQL enforcement in the current schema** (`coven-threads@0021fd26:…/audit.rs`):
  - `ward_audit_require_single_terminal_insert` (`L1669-1684`)
  - `ward_audit_require_proposal_approval_detail_insert` (`L1698-1757`), which requires `window_close` to be an object when an opening exists
  - `ward_audit_require_window_close_detail_insert` (`L1759-1802`), which requires a valid reason, a matching event family, and replay-match semantics
  - append-only triggers (`L1657-1668`)
- **Census:** `ward_audit_census.rs` `inspect` (`L141-256`) and `classify` (`L312-394`) report `TypedTerminalRecorded`, `InconsistentHistory`, `UnprovableApply`, `OpenUnverified`, `QuarantinedOpening`, `UntrustedArtifacts`, and `OrphanedOpening` without repair.

**Covering tests:**

- **Coven `threads_e2e.rs`:** `assert_window_terminal` (helper `L2022-2098`) asserts exactly one opening, one typed terminal with a normative reason, no expiry terminal, and census agreement. It is used by `retired_corpus_scheduled_intake_survives_restart_and_applies_once`, `explicit_supersession_closes_only_the_replaced_window`, `scheduled_window_replay_fails_closed_across_real_daemon_restart`, and `changed_human_path_cannot_erase_a_real_opened_window`.
- **Ordered chain:** `support/threads_identity_replay_cases.rs::assert_scheduled_valid_identity_drift` (submitted then opened, `ORDER BY rowid`). The `api.rs` unit test `threads_scheduler_terminalizes_unknown_review_kind_after_opening_window` checks submitted → opened → rejected(revalidation_failed) in order.
- **Coven `tests/threads_terminal_recovery.rs`:** `census_reports_orphaned_opening_without_familiar_or_pending_and_never_repairs`, `census_reports_retained_claim_and_unprovable_apply_without_inventing_terminal`, `public_coherence_approval_has_no_fabricated_window_close`, `synthetic_legacy_opened_history_is_rejected_once_across_restart`.
- **Threads `audit.rs` unit tests:** `proposal_window_close_detail_matches_terminal_event`, `proposal_window_open_rejects_missing_required_detail`, `schema_names_all_window_close_reason_tags`.
- **Coven `ward_audit_census.rs` unit tests:** `all_normative_closes_are_typed_and_human_no_window_is_excluded`, `apply_intents_must_follow_the_opening_and_precede_any_terminal`.

**CI:** the coven exact-SHA run failed (see the table). The acceptance doc reports that run 34786471550, which is not an exact-SHA run, found 13 openings and 13 ordered normative closes.

**Verdict: partial.**

**Gaps:**

- **A constraint that `proposal_submitted` must precede `proposal_window_opened`: not found.** No SQL or runtime check enforces it; only tests assert the order.
- The opening row is written at the first scheduler tick or decision, not at submission. Between submission and opening there is an interval with no opening row (the deadline is still derived from `staged_at`).
- By design, an opened window can stay without a typed close when its claim is quarantined or the apply is unprovable. The census reports such windows rather than closing them.
- Under 2026-09-17 Decision 1, the deployed-history disposition is scoped to the workstation profile.

---

## Item 5 — evidence_replay_hash

> "**evidence_replay_hash** is present on `ProposalClassification` and enforced at deadline revalidation."

**Source:**

- **The field:** `ProposalClassification.evidence_replay_hash: [u8; 32]` (`coven-threads@0021fd26:…/approval.rs:L405-431`).
- **The hash function:** `evidence_replay_hash(diff, evidence)` (`surface_regions.rs:L192-227`).
- **Enforcement:**
  - **On every load of the scheduled envelope**, including at the deadline and in recovery: `ScheduledProposal::try_new` re-runs `SurfaceRegionRegistry::default_registry().classify_all` over the persisted diff, recomputes the hash, and refuses on mismatch (`coven@226bfcc8:crates/coven-cli/src/proposal_scheduler.rs:L82-90`). `from_persisted_parts` and `TryFrom<ScheduledProposalWire>` then reject stored region evidence that differs from the replay (`L137-161`, `L190-208`).
  - **At the deadline decision:** `revalidate_scheduled_materialized_before` (`api.rs:L7875-7904`) requires live surface bytes to equal the diff's before-images. Divergence gives `proposal-evidence-diverged` and `WindowCloseReason::EvidenceDiverged`; a read failure gives `RevalidationFailed`.
  - **For `AutoRegression`:** `revalidate_auto_regression_evidence` passes the classification's `evidence_replay_hash` into `output_format_auto::regression_evidence` (`api.rs:L15558-15563`).
  - **In the audit trail:** the hash is copied into `proposal_window_opened` detail as `evidence_replay_hash_hex` (`api.rs:L13618-13624`).

**Covering tests:**

- **Coven `proposal_scheduler.rs` unit tests:** `replay_hash_mismatch_fails_closed` (L644), `deserialization_replays_region_predicates_instead_of_trusting_evidence` (L666).
- **Coven `api.rs` unit tests:** `threads_scheduled_deadline_replay_refuses_diverged_before_image`, `threads_scheduled_deadline_rejects_deleted_empty_before_image`, `threads_scheduled_deadline_replay_failure_closes_window`.
- **Coven `threads_e2e.rs`:** `scheduled_window_replay_fails_closed_across_real_daemon_restart` (the "surface-diverged" scenario) and `output_auto_deadline_and_restart_reject_regression_and_valid_evidence_drift`.
- **Threads `surface_regions.rs` unit tests:** `evidence_replay_hash_is_independent_of_evidence_order`, `evidence_replay_hash_commits_field_boundaries`, `evidence_replay_hash_commits_unclassified_surfaces`, `execution_prompt_replay_commits_the_before_state`.

**CI:** the coven exact-SHA run failed. Its failing Windows test is one of this item's covering tests. Threads CI passed.

**Verdict: supported.**

**Gaps:**

- Deadline enforcement is a composite of three checks: the hash re-derived over the stored diff, live before-image byte equality, and auto-regression rebinding. It is not a literal live re-diff followed by a hash comparison.
- The hash does not commit identity evidence. Identity is a parallel commitment (`identity_evidence`, checked at `api.rs:L11876-11891`) that closed under coven#885. The 2026-09-11 "classification-time identity binding" gap was resolved that way, not by widening the hash.

---

## Item 6 — min_visible

> "**min_visible** enforced on `VetoWindow` — pending state was actually reachable before deadline could close."

**Source:**

- **Threads `VetoWindow`:** construction requires `min_visible ≤ duration` (`VetoWindow::try_new`, `coven-threads@0021fd26:…/approval.rs:L281-291,L300-314`), and `TryFrom<VetoWindowWire>` (`L372-378`) enforces the same on deserialization. Window timing comes from `earliest_close` (`L343-349`) and `is_min_visible_elapsed` (`L357-363`).
- **Coven config:** an explicit `min_visible_seconds` is required whenever `human_veto_window_hours` is set, and is rejected on human paths. See `declared_veto_window` and `veto_window_from_hours` (`coven@226bfcc8:crates/coven-cli/src/ward.rs:L729-758`) and the checks in `compiled_approval_tiers` (`L661-682`).
- **Coven scheduler:** `ScheduledProposal::try_new` derives `earliest_close` and `veto_deadline` from `staged_at` and rejects inconsistent persisted values (`proposal_scheduler.rs:L92-134,L137-161`).
- **Coven decision gate:** `proposal_decision_semantics` returns `proposal-minimum-visibility-open` when `now < earliest_close` (`api.rs:L7785-7787`). The scheduler's `due` check uses `veto_deadline`, which is at or after `earliest_close` by the `VetoWindow` invariant (`api.rs:L13203-13212`).
- **Visibility:** the daemon proposals view emits `earliestClose` and `approvalPath.veto_deadline` (`api.rs:L7586-7618`).

**Covering tests:**

- **Coven `threads_e2e.rs`:** `retired_corpus_scheduled_intake_survives_restart_and_applies_once`. It asserts that `earliest_close` and `veto_deadline` equal corpus policy, ticks at `minimum-1` and `minimum` across a restart with bytes unchanged, and requires `GET /api/v1/threads/proposals` to list the proposal at the minimum ("restart lost the visible pending interval"). The proposal is applied only after `duration+1`.
- **Coven `api.rs` unit tests:**
  - `threads_scheduler_tick_observes_fixture_before_min_visible_at_earliest_close_and_beyond_deadline` (L39424)
  - `threads_scheduled_veto_window_delays_apply_and_records_veto` (asserts `proposal-minimum-visibility-open`)
  - `post_familiar_edits_stages_scheduled_publication_from_retired_ward_intake` (premature approve gives `proposal-minimum-visibility-open`)
  - `threads_scheduled_applies_only_after_veto_deadline`
  - `threads_scheduler_opens_window_once_and_waits_until_deadline`
- **Coven `proposal_scheduler.rs`:** `veto_window_derives_deadline_and_minimum_visibility`.
- **Coven `ward.rs`:** `validation_rejects_invalid_retired_ward_approval_metadata` (missing, invalid, or misplaced `min_visible_seconds`).
- **Threads `approval.rs`:** `veto_window_min_visible_gate`, `veto_window_earliest_close_is_staged_at_plus_min_visible`, `veto_window_deserialization_rejects_invalid_visibility`, `veto_window_timestamp_overflow_fails_closed`.

**CI:** the coven exact-SHA run failed (not on these named tests; see the table). Threads CI passed.

**Verdict: supported** (daemon scope).

**Gaps:**

- Reachability is shown through the daemon list API only. **Browser or Cave reachability at `258e587f`: not found.** It is tracked in coven-cave#5256.
- The scheduler does not check `earliest_close` directly; it relies on the `min_visible ≤ duration` invariant.
- A principal veto (reject) is accepted before `earliest_close`, by design.
- `AutoRegression{veto:None}` has no visibility interval, by design.

---

## Item 7 — Label-variant mapping

> "**Label-variant mapping** is a daemon load-time contract, not a Cave convention. Drift fails at load."

**Source:**

- **Threads:**
  - `ApprovalPath::display_label` is an exhaustive match, so a variant with no label fails at compile time (`approval.rs:L122-129`).
  - `from_display_label` returns `None` for any unknown label (`L136-144`).
  - `ApprovalPathKind::display_label` (`L231-260`).
  - `ApprovalPathWireEnvelope` uses `deny_unknown_fields`. Its `Deserialize` impl calls `validate_label_round_trip` (`L572-612`), as does `from_classification` (`L616-646`, round trip at `L650-675`).
  - In SQL, `approval_path_label` must be one of `auto`, `familiar_review`, `human_review`, `human_required` (`audit.rs:L1698-1757`).
- **Coven load time:** `WardConfig::from_toml_str` (`ward.rs:L415-419`) calls `validate()`, which calls `compiled_approval_tiers()` (`L450`, function `L589-727`). That function:
  - rejects unknown tier keys (`L604-606`: "approval_tiers contains unknown tier");
  - requires each tier's `gate` string to match its tier (`L652-656`);
  - maps the four fixed tier names to typed variants (`L684-696`).
- **Coven scheduled envelope:** `ApprovalPath` uses a closed serde `kind` tag. The strict JSON shape check `validate_scheduled_json_shape` (`proposal_scheduler.rs:L318`ff) rejects unknown and wrong-variant fields.
- **Coven emit:** the daemon builds the wire envelope through `ApprovalPathWireEnvelope::from_classification` (`api.rs:L7586-7591`).
- **Cave:** the mirror table `APPROVAL_PATH_LABELS` (`proposal-authority.ts:L78-83`) is used only to block on any daemon label/variant mismatch (L801-810).

**Covering tests:**

- **Threads `approval.rs`:** `all_variants_have_display_labels`, `display_labels_round_trip_to_kind`, `wire_envelope_label_round_trip_validates`, `wire_envelope_rejects_unknown_label`, `wire_envelope_rejects_mismatched_label_and_variant`, `wire_envelope_deserialization_rejects_invalid_contract`, `auto_regression_requires_the_veto_key_on_the_wire`.
- **Threads `audit.rs`:** `schema_names_all_approval_path_labels`.
- **Coven `proposal_scheduler.rs`:** `deserialization_rejects_unknown_fields`, `deserialization_rejects_unknown_nested_policy_fields`, `deserialization_rejects_fields_from_the_wrong_policy_variant`.
- **Coven `ward.rs`:** `parses_retired_ward_approval_metadata`, `validation_rejects_invalid_retired_ward_approval_metadata` (covers an unknown block, an unassigned block, and `min_visible` misuse).
- **Cave `threads-adapters.test.ts`:** "blocks a joined proposal when daemon metadata mismatches staged authority".

**CI:** the coven exact-SHA run failed (see the table). Threads and Cave CI passed.

**Verdict: partial.** The load-time contract exists in source and has unit coverage.

**Gaps:**

- **A test exercising the "unknown tier" rejection (`ward.rs:L605`): not found.** A grep across `crates/` matched only the source line.
- **A test exercising the wrong-`gate` rejection (`ward.rs:L654`): not found.**
- **A real-daemon journey that loads a malformed tier or label and refuses: not found.** The 2026-09-11 review asked for one.
- Cave keeps its own copy of the label table. It is used only to block on mismatch, not as policy.

---

## Item 8 — Advisory probes

> "**Advisory probes** (if present) are in a separate `advisory_probes` block — never mixed with deterministic evidence in `probes`. They feed Gate-3 only and are not sole authority for any gated path."

**Source:**

- **Threads:**
  - `AdvisoryProbeResult` (`identity_invariants.rs:L791-807`) is never authoritative: the authoritative flag is rejected and confidence is range-checked (`L837-883`).
  - `AdvisoryProbes` is a separate block (`L885-915`).
- **Coven `226bfcc8`:**
  - **A daemon `advisory_probes` block or model/LLM probe integration: not found.** `git grep -E 'advisory_probes|AdvisoryProbes'` over `crates/` returned nothing.
  - Daemon `probes` hold only the four deterministic `ProbeId`s (Parse, SizeDelta, ProtectedRegion, PatternLint; `ward.rs` `enum ProbeId`) via `ward_probes.rs`.
  - The `ward_probes.rs` module docs (L1-6) say the probes are advisory except for opted-in output-format `AutoRegression`, and "a probe alone never approves a write". `output_format_auto::authoritative_projection` (`output_format_auto.rs:L117-136`) is the only authoritative use, and it is deterministic.

**Covering tests:**

- **Threads `identity_invariants.rs`:** `advisory_probe_result_is_never_authoritative`, `advisory_probe_rejects_authoritative_flag`, `advisory_probe_deserialization_rejects_authoritative_flag`, `advisory_probes_block_deserialization_rejects_ill_formed_result`, `advisory_probes_block_empty_means_no_signals_ran`.
- **Coven `ward_probes.rs`:** `deterministic_failures_and_probe_errors_remain_advisory_evidence`, `absent_probe_config_is_explicitly_unscored`.
- **Coven `api.rs`:** `threads_coherence_approve_keeps_failed_probes_advisory`.
- **Coven `threads_e2e.rs` support:** `output_auto_keeps_stronger_ceremonies_and_other_probes_advisory`, `output_auto_requires_applicable_json_and_all_probes_passed`.

**CI:** the coven exact-SHA run failed (see the table). Threads CI passed.

**Verdict: supported.** The condition is vacuous at the daemon because no model advisory probes exist there; the separation is enforced by type in Threads.

**Gaps:**

- The daemon uses the word "advisory" for its deterministic `probes` field. That is a different meaning from the checklist's `advisory_probes` block, and a reviewer should not read one as the other.
- No daemon integration of `AdvisoryProbes` exists to certify.

---

## Summary of verdicts

| # | Item | Verdict | Main limiter |
| --- | --- | --- | --- |
| 1 | RFC round-trip | supported | Decision 3 promotion not implemented; decision 5 is external and was not inspected |
| 2 | Gate-4 fail-closed | supported | The exact-SHA coven CI run failed on a covering Windows auto-path test |
| 3 | Descriptor-not-authority | partial | No exhaustive enforcement-point census; Cave reads state directly; live Cave evidence not found |
| 4 | Audit completeness | partial | No submitted-before-opened constraint; lazy opening; quarantined or unprovable windows are reported, not closed |
| 5 | evidence_replay_hash | supported | Composite enforcement, not a literal live re-hash; identity bound separately |
| 6 | min_visible | supported | Browser/Cave reachability not found |
| 7 | Label-variant mapping | partial | Unknown-tier and wrong-gate branches untested; no real-daemon malformed-load journey |
| 8 | Advisory probes | supported | Vacuous at the daemon; terminology overlap |

No item was marked excluded-by-prior-decision. None of the 2026-09-17 decisions removes a #13 item. Decision 1 only scopes the #886 deployed-history disposition, which affects Item 4.

## Root-issue evidence pointers (not re-derived)

All four are CLOSED as of a read-only query on 2026-10-09.

- **OpenCoven/coven#885 (`threads-okc`), closed 2026-09-13:**
  - [acceptance comment](https://github.com/OpenCoven/coven/issues/885#issuecomment-5656608531)
  - [identity closure packet](https://github.com/OpenCoven/coven/issues/885#issuecomment-5654726468)
- **OpenCoven/coven#886 (`threads-980`), closed 2026-09-17:**
  - [disposition](https://github.com/OpenCoven/coven/issues/886#issuecomment-5719691181)
  - [deployed census comment](https://github.com/OpenCoven/coven/issues/886#issuecomment-5676712866)
  - `docs/reviews/2026-09-15-deployed-audit-census.md`
  - `docs/reviews/2026-09-17-maintainer-decisions.md`, Decision 1
- **OpenCoven/coven#887 (`threads-dgg`), closed 2026-09-13:**
  - [acceptance](https://github.com/OpenCoven/coven/issues/887#issuecomment-5656608527)
  - [route evidence](https://github.com/OpenCoven/coven/issues/887#issuecomment-5654515217)
- **OpenCoven/coven#888 (`threads-zav`), closed 2026-09-13:**
  - [acceptance](https://github.com/OpenCoven/coven/issues/888#issuecomment-5656608528)
  - [corpus packet](https://github.com/OpenCoven/coven/issues/888#issuecomment-5654515214)
- **Cross-root:**
  - [landing evidence matrix (#31)](https://github.com/OpenCoven/coven-threads/issues/31#issuecomment-5656531969)
  - `docs/reviews/2026-09-13-engineering-acceptance.md`

## Out of scope (Set B delta)

These changes come after the pinned SHAs. They are listed without judgment.

**Coven, first-parent `origin/main` after `226bfcc8`, touching Threads-related files or the pin:**

- `8ecbf9a99ec38717b0f682bb5008c175d86dfe14` (2026-10-07): fix: adopt Threads memory admission channel support (#1260). Bumps `coven-threads-core` from `0021fd26` to `78e47c9d2430582128679fd4a54f9966f2f4f976` (`Cargo.lock`, `crates/coven-cli/Cargo.toml`).
- `c72bddb71adf00f9f7e2d3f51a534bd3d6e5d69a` (2026-10-07): docs: reconcile dated Threads status and promotion wording (#1188)
- `a7ea8f518a60e8e53968658858fe193451958178` (2026-10-05): feat(coven-agents): ProposalReview seam — review tool calls before dispatch, fail closed (#1152)
- `fbffc1a1bf66516e2d8676c5e94e19bd2e233ad7` (2026-10-05): test(cli): verify three-harness parity on Windows (#1252)
- `291c153891569145e1929538f6ad22c925b3395c` (2026-10-05): feat(mobile): authorize local trusted-device introductions (#1258)
- `ad6c80bc260b4417974cc3af6ca4779902b807e8` (2026-10-05): feat: land familiar ledger and issuer slices (#1209)
- `18d278e9ece577556d97e922104da60f137b7bfa` (2026-10-02): fix(automations): prove upgrades from released stores … (#1190)
- `68da978c7e2308a8a90e692a0f21772f575f6986` (2026-09-21): feat(client): package the owner-adjacent opencoven-coven-client crate (#1141)
- `0fab796c755221cb322c056d4ca05bd9b96b150a` (2026-09-18): fix(store): stop fsyncing every commit and checkpointing on every request (#1132)
- `adf88403e7b7614958b1b9f7f16669d7ce36dddc` (2026-09-18): chore(cli): delete dead code … (#1131), touches proposal-related lines in `api.rs`/`ward.rs`
- `bcd686ef53722c38c7b28e13f2dff83c6a5202d5` (2026-09-16): fix(ci): run the real-daemon Threads suites once per platform (#1118)
- `3b8fe9bc010c623ee35f0088f30f91ce1e18ab44` (2026-09-16): fix(tests): pin the clock in the stale pre-apply recovery test (#1111)
- `e3c875adacb196844fb625984e95a9a8a1b1821e` (2026-09-16): fix(tests): prove the daemon serves requests before restarting tests use it (#1107)
- `d415e7ed7bc4810ce3fed44c8dcb2fc2927db520` (2026-09-15): fix(tests): stop treating an expired probe slice as a fatal daemon error (#1105)
- `03c8ae9bb398af756b6472ea2f417c607760d8b7` (2026-09-14): Merge #1061 investigate/1047-native-startup
- `b3b2d043a4ee586ccbf25ef6aad21db8a1171a54` (2026-09-14): Merge #1059 investigate/1051-native-pipe

**Threads crate code, `0021fd26..78e47c9d`.** Only `crates/coven-threads-core/src/audit.rs` changed (+49/−1):

- `cecdb5b8a60994b0f22e55010d44271b998fc19a` (2026-09-16): fix(threads-55s): record the channel a memory admission arrived on. Landed via merge `78e47c9d` (#72).

**Threads crate code, `78e47c9d..origin/main` (`7a94fe55`):**

- `07813e9ddc8b914b818f944125527536bb08caee`: feat(threads-vdv): add the committed-Ward-state audit constructor
- `6a46f003098f90cff4001155ca6a6cfa80e63a83`: fix(threads-vdv): reject a ward_updated row without a usable ward_hash
- `99f1350246cb39f3dd5b9c4c569747561a07f319`: docs(threads-vdv): settle ward_version and principal_authorization
- `90a691ba396f97900c335e91c6cd727ed12336fd`: test(threads-bv2): add crate-local promotion seam conformance

Docs-only Threads merges in the same range: #58, #60, #63, #66, #67, #68, #25, #69, #70, #71, #73, #74, #76, #77, #79.

**Cave:** `258e587f..6d988ec2`, see the header note.

## Diff vs the 2026-09-11 mapping

Source: `docs/reviews/2026-09-11-landscape-and-readiness.md` L215-247. That review recorded no formal verdicts, only evidence and remaining acceptance.

| Item | 2026-09-11 status | Now | Why it changed |
| --- | --- | --- | --- |
| 1 RFC round-trip | Substantial, blocked by two source gaps | supported | The auto-path gap is addressed by the output-format `AutoRegression` route (`output_format_auto.rs`, Threads #57 and `OutputFormatRegion`). The identity-binding gap closed under coven#885 via a separate `identity_evidence` commitment. |
| 2 Gate-4 | Familiar and human evidence present; auto and rationale paths open | supported (CI caveat) | Auto and rationale lanes now have real-daemon coverage (`output_auto_*`, `scheduled_identity_replay_covers_manual_and_immediate_routes` with human_required/auto, `threads_scheduled_human_required_enforces_rationale_and_applies`). The final-commit authority check is in both apply modes. |
| 3 Descriptor-not-authority | No exhaustive Cave conformance claimed | partial (unchanged in substance) | Cave forwarding tests exist at `258e587f`, but live Cave acceptance (#5256) and an exhaustive census are still not found. |
| 4 Audit completeness | Ordering, correspondence, and recovery open | partial (narrower) | Ordered submitted → opened → close is now asserted in tests. The recovery matrix and read-only census landed. #886 closed on a profile-scoped census (Decision 1). The runtime ordering constraint and the lazy opening remain. |
| 5 Replay hash | Identity commitment unresolved | supported | The identity commitment is separate and enforced at decision and final commit (#885 closed). Live before-image and auto-evidence rebinding are evidenced. |
| 6 min_visible | Exact deadline, publication visibility, and browser reachability needed separately | supported (daemon) | Exact `earliest_close`/deadline equality and listing at the minimum across restart are now asserted in E2E. Browser reachability is still not found. |
| 7 Label/variant | Needed a real-daemon malformed-load case | partial (unchanged) | No real-daemon malformed-load journey found. The unknown-tier and wrong-gate branches are untested. |
| 8 Advisory separation | Types separate; probes deterministic; no model probe | supported (unchanged) | Still no daemon model-probe integration. Deterministic output-format authority was added and is explicitly scoped. |

## Addendum (Echo, 2026-10-09): exact-SHA Windows failure triage

- Run 34785628407 (push, main, `226bfcc8`, attempt 1), job 103800507099 "Threads daemon tests (Windows)". `threads_e2e` reported 80 passed and 1 failed. The failing test was `output_auto_cases::output_auto_deadline_and_restart_reject_regression_and_valid_evidence_drift`. The error was `fixture daemon request POST /api/v1/familiars/sage/edits` followed by `invalid Coven daemon HTTP response: no response bytes arrived before the deadline`. The test had been running for more than 60 s before it failed.
- The failure was a timeout in the test's own request to the daemon. No assertion about an approval decision failed. In the same job, the other targets passed (213 lib tests and the 43/34/32 identity, intake and recovery targets). The Linux and macOS Rust test jobs in the same run passed.
- The same tree passed earlier. PR head `98e89c4f` has tree `db70ae95`, identical to `226bfcc8`. Its Windows Threads job, 103797410597 in run 34784497791, passed.
- The signature matches coven#1051 item 2: the same route `POST /api/v1/familiars/sage/edits` gets no response on Windows. That issue measured a slow reservation phase taking about 5.2 s. coven#1132 (`0fab796c`, merged 2026-09-18) stopped the fsync on every commit and the checkpoint on every request. Three later Windows runs on `main` were green, and #1051 was closed. #1132 is NOT an ancestor of `226bfcc8`. It IS an ancestor of `8ecbf9a9` (the Oct 7 Threads bump).
- The last 60 CI runs on `main` (2026-09-28 to 2026-10-09) show the Windows Threads job as 46 success, 8 skipped and 6 not run. None failed.
- This run is not cited in coven#1051.
- Assessment: this is a known Windows daemon-latency defect that is present at `226bfcc8` and fixed after it. It is not a Gate-4 decision defect. The attribution to #1051 is inferred from the matching route and symptom. This run's log contains no reservation checkpoint data that would prove the phase.
