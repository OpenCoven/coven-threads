# Native daemon 30-day ledger

This is the engineering evidence consumer for [#111](https://github.com/OpenCoven/coven-threads/issues/111), stacked after [#108](https://github.com/OpenCoven/coven-threads/pull/108)'s matrix and [#112](https://github.com/OpenCoven/coven-threads/pull/112)'s immutable archive. Tooling and synthetic controls cannot supply native execution or elapsed days. This change selects no operational window and posts no declaration. The required Coven pin and separate latest-main canary stay unchanged.

## Declare the window before it begins

The governing metric is fixed in this implementation: 90 scheduled first-attempt OS opportunities across 30 complete UTC days, one 07:47 slot per day for each of `ubuntu-24.04`, `macos-15` and `windows-2022`. At least 99.5% must qualify. One unqualified observation gives 89/90 (98.88889%), so this cadence requires all 90. A retry, manual execution, missing job/artifact or unsupported source cannot reduce the denominator. Per-journey tables preserve the pinned full inventory alongside this governing rate; they do not silently substitute a more favorable denominator.

A maintainer must review and select the operational start and accepted source/profile scope. Before the first UTC day, publish an unedited comment on #111 from the authenticated `BunsDev` account containing this contract as JSON (raw JSON or one `json` code fence), with all placeholders replaced:

```json
{
  "schema_version": "opencoven.threads-native-window/v1",
  "repository": "OpenCoven/coven-threads",
  "workflow": ".github/workflows/daemon-reliability.yml",
  "window_start": "<YYYY-MM-DD>",
  "days": 30,
  "schedule_utc": "07:47",
  "governing_metric": "scheduled_os_first_attempts",
  "minimum_percent": 99.5,
  "scenario_inventory_sha256": "<SHA-256 of the committed native-daemon-scenarios.json>"
}
```

The collector verifies the comment's actual GitHub ID, issue, account, creation/update timestamps and exact contract. It refuses an edited comment or one created after the selected window begins. The account is provenance for the predeclaration; it does not prove a human authored or approved it. Human acceptance remains separate. This timing proof prevents a caller from backdating a local JSON file to select a favorable completed period.

The local declaration file contains those same contract fields plus `declared_at` (the exact GitHub comment creation timestamp), `declaration_comment_id` (the positive numeric ID), and an `accepted_series` array. Each series has a unique `id` and a `profiles` object with all three exact runner labels. Each profile must be the complete verified `source_series` object from #110's original receipts: `coven_sha`, `threads_sha`, `lock_sha256`, `platform`, `arch`, `rustc`, `cargo`, `node`. All three profiles in one series identify the same Threads SHA. No source/tool/lock/architecture default or implicit latest revision exists.

Obtain initial native pilot receipts before selecting the window so those profiles can be reviewed. Later source series require an explicit accepted entry and retain separate accounting; they never inherit an earlier head's acceptance. The predeclared date range, metric, target, schedule and inventory cannot change. Accepting a later series does not remove any failed or unsupported opportunity. Overlapping profile aliases are ambiguous and cannot qualify a row. The declaration file and its complete digest are retained with every ledger.

## Collect and replay

With Node 24.22.0, Python 3.11+ and an authenticated `gh` CLI, retain completed native attempts using the [existing collector](daemon-evidence-retention.md), then run:

```bash
node scripts/qualify-daemon-window.mjs DECLARATION_JSON ARCHIVE_ROOT NEW_OUTPUT_DIRECTORY
```

The output must be new and outside the immutable archive. The CLI only reads GitHub REST data through `gh api`; it never dispatches, reruns, merges, publishes declarations or changes credentials. It has no clock override. A valid but unqualified ledger exits 1 and retains its evidence. Setup/read/validation failures leave indexed failure output; an existing directory is never overwritten. Raw HTTP responses and original subprocess output/status/signal/spawn errors are retained where available.

Collection lists all runs of the native workflow over the full declared UTC date range, with no outcome or source filter. It completely paginates and rechecks the census; changed/incomplete pages, duplicate identities and the [GitHub 1,000-result search bound](https://docs.github.com/en/rest/actions/workflow-runs#list-workflow-runs-for-a-workflow) refuse collection. Manual runs are retained as excluded observations, never substituted for schedule slots. A scheduled run maps to its UTC creation date only when creation is at or after 07:47. Multiple runs in a date are ambiguous; arrivals before that time, including a possible cross-midnight delayed trigger, are unsupported. Creation and queue/start delays remain explicit. No favorable run is selected from duplicate slots.

Every retained attempt in the window is replayed from the original digest-checked ZIP, file index, reports, target output, complete scenario manifests and JUnit. Copied journey totals must match the rederived proof. The collector then reauthenticates original run identity/outcome, all native jobs and original artifact identity/digest against live REST responses, and records source commit/tree mappings. Artifact expiration is separate from retained-byte integrity: already-retained bytes still need the authoritative original digest. A forged offline index alone cannot authenticate a GitHub result. Original archives outside the window remain counted separately.

The minimum Date across authenticated GitHub responses must be at or after the exclusive end of all 30 UTC days. A local clock, test fixture, simulated timestamp or an earlier Cave window cannot satisfy that condition. The collector records its actual Node/Python versions, clean commit/tree and script hashes; a dirty checkout cannot produce a qualified window. Original HTTP snapshots, declaration, source identities, `ledger.json`, prose/table `packet.md` and file index remain in the output. A completed unqualified row stays in the opportunity count, and a later passing attempt remains recovery evidence rather than a first-attempt success or causal flake diagnosis.

## Terminal-close scope

Window qualification additionally replays retained `state/ward-window-census.json` and `state/ward-audit.jsonl` captures. Each captured opened window must have exactly one opened row before exactly one permitted typed terminal, with a matching census reason. Read-only evidence validation mirrors `ProposalWindowCloseAuditDetail` and `validate_window_close_detail` in the canonical Rust source: approved/applied requires replay true, vetoed/vetoed requires no replay result, and rejected evidence-diverged/revalidation-failed/superseded require their canonical false/false/null replay results. Optional rationale must be text or null. Duplicate audit IDs, omitted opened windows, incomplete/unsupported census classifications, reversed chains, invalid payload types, contradictory reasons and expiry terminals refuse that scoped proof. Every OS must include the pinned common `explicit-supersession` journey's two typed closes (`superseded` and `applied`); empty or missing captures cannot count as zero anomalies.

Counts cover retained terminal-census captures, including the actual pinned daemon's canonical terminal assertions. They do not claim global deployed-history closure or inspect production profiles. Successful journey manifests alone do not fabricate numeric terminal counts. A positive execution lacking this additional capture proof remains unqualified in the 30-day ledger. The current terminal interpretation is bound to the unchanged pinned daemon source; another profile needs explicit review.

## Delivery and limits

Review the ledger's expected/qualified OS and journey counts, missing/late/ambiguous states, source-series accounting, original failure/retry history, terminal scope and authenticated dates together. Engineering qualification does not imply human coherence/freeze, release, deployment or required-check promotion. The stable compatibility pin is not repinned by this tooling. Actual native collection cadence, independent archive retention and 30 elapsed days remain required operational evidence.

Rollback is a reviewed revert of this additive reader/ledger/collector slice. Preserve original ZIPs, declarations, HTTP metadata, failure snapshots and run histories. No runtime audit store or protected-write authority is introduced.
