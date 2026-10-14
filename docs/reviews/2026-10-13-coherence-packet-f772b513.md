# EVIDENCE PACKET — NOT AN ACCEPTANCE

OpenCoven/coven-threads #13 follow-up coherence packet at Set B (bead `threads-uqx.19`). It is assembled by Echo from three read-only parts (A, B and C below). No tests were run; results come only from the CI runs cited. This packet does not accept, reject or defer #13.

| Repo | Set B SHA |
|---|---|
| Coven | `f772b51377f88d97c9d44d87093e5b641231972e` (pinned by Val, 2026-10-13) |
| Threads core | `78e47c9d2430582128679fd4a54f9966f2f4f976` (Cargo.lock pin) |
| Cave | `c60bbef90bdb859f4cb4f2fb44685eb02d939b71`: the last first-parent `main` commit at or before the Coven target landed (05:47:11Z). This corrects the scope's `de266c11`, which landed at 06:04Z. The Threads-facing files are identical at both. |

## Scope ruling recorded for this packet

On 2026-10-14, in an Echo session, Val ruled that **OS crash and power loss are in scope** for #13 item 4. Item 4 therefore cannot be supported at Set B until the durability gap in part B §2 is fixed. That gap is filed as [OpenCoven/coven#1300](https://github.com/OpenCoven/coven/issues/1300). This note records a scope instruction only. It is not an acceptance, deferral or rejection of #13, which remains Val's own decision on GitHub.

## Summary of proposed verdicts (Set A → Set B)

| # | Item | Set A (`226bfcc8`) | Set B (`f772b513`) | Part |
|---|---|---|---|---|
| 1 | RFC round-trip | supported | supported | C |
| 2 | Gate-4 fail-closed | supported, with an exact-SHA Windows CI caveat | supported. The Windows caveat is cleared: the Windows Threads jobs passed in both exact-SHA runs. | B |
| 3 | Descriptor-not-authority | partial (CX-1e) | supported. CX-1e is closed by `21988a00`; CX-1a to CX-1d are class (b) and named. There is one reviewer judgement on Cave's local-only proxy gate. | A |
| 4 | Audit completeness | partial | **partial**, under both the original wording and Decision 5 (#86, unmerged). OS crash and power loss are in scope (Val, 2026-10-14), and the coven#1132 durability gap is open as OpenCoven/coven#1300. Under Decision 5 the item is supported for a daemon process crash only. | B |
| 5 | evidence_replay_hash | supported | supported | C |
| 6 | min_visible | supported (daemon) | supported (daemon scope); Cave visibility is still not found (coven-cave#5256) | C |
| 7 | Label-variant mapping | partial | supported (#1276); the real-daemon malformed-load test is still not found | C |
| 8 | Advisory probes | supported | supported (vacuous at the daemon) | C |

**Key finding (part B §2).** coven#1132 (`0fab796c`) set `synchronous = NORMAL` on every writable store connection. At Set A, `synchronous` was never set, and the store test asserted `>= 2` (FULL or stricter). Under OS crash or power loss at Set B, a protected file can therefore keep the approved bytes while the apply-intent row, baseline advance and terminal are lost:

- The target file is `sync_all`ed and swapped in between two audit commits that are no longer synced.
- The census then shows `QuarantinedOpening` or `OrphanedOpening`, or nothing at all on paths with no opening.
- In one filesystem-dependent case, the change is recorded as `proposal_rejected / evidence_diverged` even though it was applied.

A process crash is unaffected at both SHAs. The proposed severity is medium.

**Exact-SHA CI:**

- Coven merge-group run 38209489872 succeeded. The four Threads suites passed on Linux (105/45/39/34) and on Windows (82/44/35/33).
- Coven push run 38211237006 was cancelled. Its macOS Rust tests job hit the 20-minute limit before any suite reported, so **no macOS Set B result exists**.
- Threads CI 35110172749 succeeded. Daemon compatibility 35110172753 succeeded, but against the Set A daemon.
- Cave CI 38207714524 succeeded.

## Echo spot-checks (2026-10-13)

- **Set B pragmas:** `coven@f772b513:crates/coven-cli/src/store/schema.rs` sets `PRAGMA synchronous = NORMAL` in `WRITABLE_CONNECTION_PRAGMAS` (L611-615), applied at L590 and L646.
- **Set A pragmas:** `@226bfcc8` has no `synchronous` in `schema.rs`. Its test `store.rs:6230` asserts `>= 2`; at Set B, `store.rs:6576` asserts `== 1`.
- **Approve-path order at Set B:** `append_proposal_apply_intent` (api.rs ~L12561), then `apply_after_review_approval` (~L12635), then `finalize_approved_proposal` (~L12746).
- **Cave pick:** `c60bbef9` was committed at 2026-10-12T05:23:16Z, and it is the first parent of `de266c11`.
- **Threads CI:** 35110172749 (CI) and 35110172753 (daemon compatibility) succeeded on `78e47c9d`.
- **macOS:** in push run 38211237006, `Rust tests (macOS)` was cancelled.
- **Local paths:** the three parts contain no local absolute paths.

---


<!-- ===== PART A ===== -->

# OpenCoven/coven-threads#13 item 3 at Set B: Part A (delta enumeration)

**EVIDENCE — NOT AN ACCEPTANCE**

- Bead: threads-uqx.19 (issue #13, checklist item 3, "Descriptor-not-authority check")
- Criterion: "no Cave or client-side surface is enforced on. Every enforcement point traces to a daemon-owned predicate or audit row."
- Set B Coven: `OpenCoven/coven@f772b51377f88d97c9d44d87093e5b641231972e`
- Set B Threads core (pinned there): `OpenCoven/coven-threads@78e47c9d2430582128679fd4a54f9966f2f4f976`
- Set A Coven: `OpenCoven/coven@226bfcc89ff6cad4bc9cc9618dad6fea970ecf58` (Threads `0021fd2662d0b82328371ee3a6957d645f776fda`)
- Set A Cave: `OpenCoven/coven-cave@258e587f9edce9cdfe200d6f5aff6076fdb2c645`
- Set B Cave: `OpenCoven/coven-cave@c60bbef90bdb859f4cb4f2fb44685eb02d939b71`. This is the last first-parent `origin/main` commit at or before 2026-10-12T05:47:11Z. It is not `de266c11`, which is 17 minutes later; see section 4.
- Date: 2026-10-13
- **No tests were run.** No cargo, pnpm, build or CI query. Static reading of git objects only.
- Set A input: the Set A item-3 enumeration (Echo workspace handoff `2026-10-11-threads-13-item3-enforcement-enumeration-226bfcc8.md`, not in this repository) (77 enforcement points, CX-1).

## 1. Set A point-by-point carry/changed table

**Status counts over the 77 Set A points:**

- **Unchanged: 73.**
- **Changed: 4.** EP-03, EP-80, EP-90 and EP-91. None of the four changes is authority-bearing.
- **Removed: 0.**

Two unchanged points have a narrowed reach, because their own code is identical but a new refusal now runs before them (EP-73 and the EP-77 legacy-authority branch). Two unchanged points depend on a `store.rs` function that changed in diagnostics and WAL policy only (EP-18 and EP-44). Both pairs are noted below.

"Unchanged" means the cited code range is byte-identical after the line offset (section 5 lists every `diff` range). `<file>` refs are `coven@f772b513:crates/coven-cli/src/<file>`.

### 1a. Points whose code lives only in unchanged files

Proof for each file: `git diff --shortstat 226bfcc8 f772b513 -- crates/coven-cli/src/<file>` is empty. Set A line refs therefore carry unchanged.

| File (empty diff) | Set A points / refs carried as-is |
|---|---|
| `threads_gate.rs` | EP-19 body `L327-480` (`L360-363`, `L376-401`, `L403-470`, `L959-1011`); EP-20 `L1182-1260`, `L1436-1626`, `L1463`, `L1470-1497`, `L1595-1606`, `L1645-1698`; EP-23 `L1236-1256`, `L1348-1412`, `L1391-1410`; EP-24 `L1040-1077`, `L1085-1154`; EP-25 `L932-957`; EP-66 `L529-664` (`L542-545`, `L595-601`); EP-78 `L1085-1154`, `L942-957`; EP-80 body digest `L169-171`; EP-92 call sites `L1318`, `L1594`. The production/test boundary is `#[cfg(test)]` at `L1700` (an inner item at `L1022`), and the `gate_protected_edits` definition is at `L327`. |
| `proposal_scheduler.rs` | EP-87 `L52-134` (`L66-68`, `L72-79`, `L81-84`, `L86-89`, `L91-121`); EP-88 `L137-163`, `L193-207` |
| `proposal_store.rs` | EP-33 / EP-86 `L211-219`; EP-85 `L326-336`, `L261-282` |
| `ward_identity.rs` | EP-19 / EP-69 `L11-49`, `L29-49`, `L51-129`; EP-30 note `L213-263` |
| `ward_probes.rs` | EP-34 `L306-313`; EP-61 `L315-371`, `L129-174` |
| `threads_clock.rs` | EP-90 `L170-203`, `L308-344`, `L334-337`, `L366-372` |
| `ward_decision.rs` | EP-06 / S2 `L44-93` |
| `api_routes.rs` | EP-01 `L52-74`, constant `L33` |
| `output_format_auto.rs`, `ward_audit_census.rs`, `familiar_identity.rs`, `memory_import.rs`, `ward_migrate.rs` | EP-20, EP-69 and EP-10 consumers; S3 `ward_migrate.rs:L105-560`; S4 `memory_import.rs:L563-581`, `L760` |
| `coven-threads-core` `validate.rs`, `weave.rs` (`0021fd26..78e47c9d` empty) | EP-73 `validate.rs:L142-234` (writer binding `L152-161`); `weave.rs:L76-121` |

### 1b. `api.rs` points (file changed; every range below byte-compared)

**Offsets** (old → new):

| Set A range | Offset |
|---|---|
| `L1-592` | +0 |
| `L657-856` | +23 |
| `L926-1301` | +71 |
| `L1302-2348` | +81 |
| `L4652-9862` | +105 |
| `L9864-11319` | +110 |
| `L11320-12918` | +133 |
| `L13260-17089` | +150 |

**Unchanged (byte-identical ranges):**

| ID | Set B location(s) | Status |
|---|---|---|
| EP-01 | applied `api.rs:L862-879` | unchanged. It gains a new caller in the transport for NEP-02 (`daemon.rs:L6210-6211`), same function. |
| EP-04 | `api.rs:L1043-1056` → `request_authority.rs:L22-24` (Set A `L18-20`, same body) | unchanged |
| EP-05 | `api.rs:L1057-1072` | unchanged |
| EP-06 | `api.rs:L761-819` (`L780-800`); `ward_decision.rs:L44-93` | unchanged |
| EP-10 | `L5988-6003` | unchanged |
| EP-11 | `L6011-6057` | unchanged |
| EP-12 | `L6071-6082` | unchanged (CX-1 intake; section 3) |
| EP-13 | `L6090-6114` → `ward.rs:L387-412`, `L425-588` | unchanged |
| EP-14 | `L6119-6145` → `ward.rs:L1561-1620`, `L1639-1704`, `L1719-1724` | unchanged |
| EP-15 | `L6147-6210`; `protected_proposal_targets` `L5913-5953`; `ward.rs:L1622-1627` | unchanged |
| EP-16 | `L6213-6222` | unchanged |
| EP-17 | `L6225-6240` | unchanged |
| EP-18 | `L6246-6265`, `L6166-6185`, `L5699-5719`; `store.rs` `WardAuditReservation::acquire` `L780-...` | api.rs unchanged. The `store.rs` dependency changed in diagnostics and WAL-checkpoint policy only (section 5); admission and refusal logic are unchanged. |
| EP-19 | `L6267-6322` (`gated_targets: &[]` `L6275`) | unchanged |
| EP-20 | `L6324-6398`; `ward.rs:L589-727` | unchanged |
| EP-21 | `L6402-6436`; `ward.rs:L1883-1895` | unchanged |
| EP-22 | `L6437-6501`; `ward.rs:L1804-1896`, `L2195-2200`, `L889-902` | unchanged |
| EP-23 | `L6513-6602` | unchanged |
| EP-24 | `L6612-6632` | unchanged |
| EP-25 | `L6633-6657`, `L6707-6728` | unchanged |
| EP-30 | `api.rs:L5528-5589` → `cockpit_sources.rs:L248-322` (moved +52; body identical to Set A `L196-270`) | unchanged |
| EP-31 | `L6737-6792` (fingerprint disclosure `L6782`) | unchanged |
| EP-32 | `L6816-6954`; event-tag validation `L6970-...`; tag list `L6956-6968` | unchanged |
| EP-33 | `L7596-7613`, `L7413-7450`, `L9490-9574`, UUID check `L1076-1085` | unchanged |
| EP-34 | `L7637-7656`, descriptors `L7667-7730` | unchanged |
| EP-35 | `L7759-7801` | unchanged |
| EP-40 | `L10206-10214` | unchanged |
| EP-41 | `L10215-10289` | unchanged |
| EP-42 | `L10299-10328`; `L16247-16286` | unchanged |
| EP-43 | `L10331-10336`; `L10164-10187`; `L9991-10036`; `L10046-10162` | unchanged |
| EP-44 | `L10337-10553`; `L15319-15331`; `L14862-15029` (`L14995-14998`); `L10536-10542`; `L9633-9639` | unchanged (same `store.rs` note as EP-18) |
| EP-45 | `L10562-10605` | unchanged |
| EP-46 | `L10609-10678`; `L14034-14052`; `L14484-14627`; `L13835-13901` | unchanged |
| EP-47 | `L10679-10723`; `L16481-16504` | unchanged |
| EP-48 | `L10724-10811` | unchanged |
| EP-49 | `L10818-10865`; `L15147-15179`; `L15181-15317` (`L15244-15273`); actor constant `L55` | unchanged. The HTTP wrappers are unchanged. The automatic wrapper (`L9962-9973`) gained two diagnostic checkpoints only; see EP-80. |
| EP-50 | `L10867-10918` | unchanged |
| EP-51 | `L10921-10950`; `L16384-16465`; `L16300-16322` | unchanged |
| EP-52 | `L10962-11031`; `L14123-14202`; `L14204-14309`; `L14311-14407` | unchanged |
| EP-53 | `L11032-11051` | unchanged |
| EP-54 | `L11053-11072` | unchanged |
| EP-55 | `L11073-11099` | unchanged |
| EP-56 | `L11103-11116`; `L13732-13812` (`L13749-13754`) | unchanged |
| EP-57 | `L11117-11251`; `L16642-16652` | unchanged |
| EP-58 | `L11252-11289` | unchanged |
| EP-59 | `L11290-11429`; `authorization_from_writer` `L16654-16660` | unchanged. **NEP-03 is inserted immediately after it, at `L11430-11452`.** |
| EP-60 | `L11453-11481`; `L7830-7963` | unchanged |
| EP-61 | `L11482-11527`; `L8090-8140` | unchanged |
| EP-62 | `L11528-11558` | unchanged |
| EP-63 | `L11559-11651`; `L16075-16119`; `L15492-15562` | unchanged |
| EP-64 | `L11652-11739` | unchanged |
| EP-65 | `L11740-11827` | unchanged |
| EP-66 | `L11828-11918` (writer-bound weave `L11852-11861`) | unchanged |
| EP-67 | `L11919-11949` | unchanged |
| EP-68 | `L11951-12007`; `L16848-16932` | unchanged |
| EP-69 | `L12009-12094`; `L15646-15694` | unchanged |
| EP-70 | `L12096-12155`; `L7980-8009`; `L8011-8042` | unchanged |
| EP-71 | `L12157-12166` | unchanged |
| EP-72 | `L12167-12410`; `L16159-16219`; `L15731-15767`; `recovery_authorization` `L466-471` (identical) | unchanged |
| EP-73 | `L12412-12477` | **body unchanged, reach narrowed.** Legacy unscheduled authority-lane approvals no longer reach it (refused at NEP-03, `L11437-11452`). At Set B it evaluates scheduled authority-lane proposals only (CX-1d). |
| EP-74 | `L12479-12534`; `L15779-15841`; `L15843-15883` | unchanged |
| EP-75 | `L12536-12581`; `L16017-16052` | unchanged |
| EP-76 | `L12587-12606`; `L16698-16701` | unchanged |
| EP-77 | `L12613-12739`; `L15918-15969` → `ward.rs:L2089-2155` (scheduled), `L2024-2082` (coherence), `L1923-1991` (legacy authority, `Blocked`-only check `L1958`) | **body unchanged, reach narrowed.** The legacy-authority branch is now reached only from recovery (EP-72) of a pre-fix apply intent. |
| EP-78 | `L12746-12768`; `L16784-16838` | unchanged |
| EP-81 | `L13682-13712` | unchanged |
| EP-92 | `L9173-9834` (`L9691-9795`, `L9801-9828`); `L12815-12844`; limits `L8148-8151` | unchanged |

**Changed:**

| ID | Set B location(s) | What changed | Authority impact |
|---|---|---|---|
| EP-80 | scheduler `api.rs:L13051-13411`. Sub-refs: reconcile `L13463-13673`; quarantine `L12986-13004`, `L13115-13121`; receipts `L13129-13164`; windows `L13173-13202`; protected targets `L13030-13047` → reject `L13244-13300`; durable resume `L13312-13324`; expiry `L13325-13344`; auto-approve `L13355-13373`. Automatic wrapper `L9962-9973`; daemon start `daemon.rs:L5049-5073` (sites `L5766`, `L6898`). | 17 `checkpoint(Phase::Scheduler…)` lines and one `use` line were added. The function is byte-identical to Set A after stripping them. | none (diagnostics only) |
| EP-90 | `api.rs:L17206-17302` → `threads_clock.rs` (unchanged); feature `Cargo.toml:L13-15` (`default = []`, unchanged) | 5 `checkpoint(Phase::Tick…)` lines and one `use` line; identical after stripping. `Cargo.toml` adds `required-features = ["threads-test-clock"]` to the four Threads integration test targets (test wiring only). | none |
| EP-91 | `api.rs:L1122-1183` (arm unchanged) → `eval_loop.rs:L265-297` (`enqueue_run`), `L303-323` (`clear_eval_loop_lock`), `L176-192` (`MutationLock` and `mutation_lock`), `L327-329` (`familiar_workspace` → `cockpit_sources.rs:L88`) | adds an exclusive `eval-loop/mutation.lock` in the familiar workspace around both mutations. `{id}` is still not validated, and `force` still overrides staleness. | none for Ward/Threads (not a governed surface). The Set A out-of-scope observation stands. |

### 1c. Transport points in `daemon.rs` / `request_authority.rs` (files changed)

| ID | Set B location(s) | Status |
|---|---|---|
| EP-02 | `daemon.rs:L6119-6134` (Set A `L5482-5497`, untouched) | unchanged |
| EP-03 | Restricted-session preflight `daemon.rs:L6135-6139`. API dispatch: `RequestAuthority` is now computed once at `L6180-6183`, before early dispatch; at Set A it was inside the API closure at `L5563-5566`. The mapping is identical: `HostGuard::Disabled` → `OwnerLocalIpc`, `Loopback` → `Tcp`. Listeners: Unix socket `L6601-6611` (`HostGuard::Disabled`, untouched); TCP `L4849-4857` (`HostGuard::Loopback`, untouched); **Windows named pipe `L6881-6968`** (`HostGuard::Disabled` at `L6931`, `L6956`; owner-only SDDL `L6710-6729`, identical to Set A `L5949-5968`). `request_authority.rs:L1-31`: enum unchanged, predicates added (section 2). | **changed (refactor, same mapping).** The Windows pipe was present at Set A (`L6120-6194`) but missing from the Set A EP-03 row. It is added here; nothing changed in substance. |

**Set A pre-dispatch interceptions, at Set B:**

| Interception | Set B location | Status |
|---|---|---|
| Restricted-session preflight | `daemon.rs:L6135-6161` | unchanged |
| Lifecycle shutdown | size limit `L6168-6176`; dispatch through `EarlyDispatch::LifecycleShutdown` `L6193-6197`, `L6228-6229` | refactored |
| Mobile pairing local control | `L6198-6201`, `L6231-6236`, behind `claims_local_control` `mobile_memory/gateway.rs:L868-873` | refactored |

Both refactored early dispatches now pass through the refusal-only chat-context gate (NEP-02).

### 1d. Route inventory and non-HTTP entries at Set B

All arm bodies are byte-identical to Set A:

| Route | Set B arm | Body |
|---|---|---|
| R1 | `L1009-1011` | |
| R2 | `L1012-1017` | |
| R3 | `L1020-1025` | |
| R4 | `L1028-1033` | |
| R5 | `L1036-1041` | |
| R6 | `L1042` | |
| R7 | `L1043-1056` | |
| R8 | `L1057-1072` | |
| R9 | `L1073` | |
| R10 | `L1074-1086` | |
| R11 | `L1087-1092` | |
| R12 | `L1093-1098` | |
| R13 | `L1099-1102` | |
| R14 | `L1103-1106` | |
| R15–R17 | `L1223-1280` | |
| R18 | `L1286-1289` | `L1663-1828` |
| R19 | `L1122-1183` | |

Non-HTTP entries:

- S1: `daemon.rs:L5049-5073`.
- S2: `main.rs:L4637-4652`.
- S3: `main.rs:L4667-4687`.
- S4: `main.rs:L1854-1871`.

S2, S3 and S4 are byte-identical to Set A.

## 2. New enforcement points at Set B

Refs are `coven@f772b513:crates/coven-cli/src/<file>`. Client-input labels are as defined in Set A: ID, REV, NOTE, DATA, PAGE, CRED, LABEL, TRUSTED AS AUTHORITY.

**Dispatch delta.**

- The `match (method, route)` is now `api.rs:L885-1471`. At Set A it was `L857-1390`.
- It has **84 arms plus the default 404**, against 80 at Set A. A text diff of the arm headers shows exactly 4 new arms, all `/main-session*`, at `L1373-1382`.
- Every Set A arm header is unchanged. Old `L926-1301` and `L1302-1390` are byte-identical to new `L997-1372` and `L1383-1471`.
- The only Set A arm whose body changed is `POST /actions` (`L910-1005`).

**Whole-tree census.** These patterns were grepped over `crates/coven-cli/src`: `crate::ward::`, `ward::<ident>`, `threads_gate::`, `coven_threads_core::`, `proposal_store::`, `proposal_scheduler::`, `ward_identity::`, `ward_probes::` and `decide_threads_proposal`.

- The set of matching files differs between Set A and Set B by exactly one file, `familiar_ledger.rs`.
- No other new module references Ward or Threads code.
- The 33 new source files include `automations/*` (17), `chat_context_admission.rs`, `main_session*.rs` (4), `familiar_issuer.rs`, `familiar_cli.rs`, `eval_loop_cli.rs`, `mobile_memory/introduction.rs` and `registry/introduction.rs`, `stream_turn_state.rs`, and two test-only files.

| ID | Function | Location | Decision | Authority source | Client inputs → treatment | Ward/Threads-governed mutation decided, permitted, or fed? |
|---|---|---|---|---|---|---|
| NEP-01 | Chat-context intent gate (pre-dispatch, API) (#1134) | `api.rs:L880-884` → `chat_context_admission.rs:L435-504` | It applies only to POST/PUT/PATCH/DELETE bodies that are valid JSON and contain `contextAdmission` (`L441-451`). The inner closure has type `Result<Infallible, Rejection>` (`L452`, `L500-503`), so it **can only refuse**: 403 on a non-owner transport (`L453-455`), 400 for a wrong route or verb (`L459-464`, `L495-497`), 409 on a digest or launch mismatch (`L465-494`), and otherwise 503 `chat_context_admission_unavailable` (`L498`, `L163-174`). A request without the field passes through unchanged (`L449-451`). | transport `allows_chat_context_admission` (`request_authority.rs:L14-16`, OwnerLocalIpc only); daemon-side SHA-256 comparisons (`L332-346`) | `contextAdmission.*` → DATA (compared; refuse-only); `prompt` / `data` / `launchPolicy` / `harness` / `model` / `familiarId` / `projectRoot` → DATA (compared only). `Request::parse` (`L212-...`) is in-memory; `Request::read` (file) is used only by the CLI (`main.rs:L4903`). | **No.** It is refusal-only and writes nothing. It does sit in front of R4, R7, R11 and R12: a body carrying `contextAdmission` is refused before Ward or Threads handling. That adds refusals and grants nothing. |
| NEP-02 | Chat-context gate on early dispatch (transport) | `daemon.rs:L6193-6225`. The early-dispatch classes are lifecycle shutdown (`L6193-6197`) and mobile local control on the owner socket (`L6198-6201`, `mobile_memory/gateway.rs:L859-873` `claims_local_control`, enforced again at `L881-883`). | Same refusal-only `reject_api_intent`, applied only to requests that bypass the API handler, after the path is normalized as the handler would (`L6209-6225`). | as NEP-01 | as NEP-01 | **No.** Lifecycle and mobile pairing only. |
| NEP-03 (EP-59a) | Legacy authority-lane approval refusal (#1278) | `api.rs:L11430-11452` | 409 `legacy-authority-approval-unsupported` for an approve of an unscheduled authority-lane proposal with no applying state. It restores the claim and finishes the reservation, and writes no terminal or apply-intent row. | daemon-parsed envelope (`review_kind` `L10679`, `scheduled` `L10919`) and daemon decision state (`applying_state` `L10812`) | verb (route) → routing. The writer label is **not** an input (section 3). | **Refuses** a Threads-governed mutation. It closes CX-1e (section 3). |
| NEP-04 | Automation / ledger transport gate on `/actions` (#1162, #1164; replaces `automation_receipt_transport_rejection`) | `api.rs:L920-924` → `control_plane.rs:L207-245` | Runs before the store opens. Familiar-ledger actions over TCP → `AuthorityRequired` (`L215-224`). `coven.automations.*` actions not in `TCP_AUTOMATION_READS` → refused over TCP, deny-by-default (`L225-244`). | transport `allows_owner_automation_access` (`request_authority.rs:L26-30`, OwnerLocalIpc only) | `action` → routing | **No.** It is refusal-only. `control_plane.rs` has 0 Ward/Threads references (census above). |
| NEP-05 | `CommandAuthority` mapping | `api.rs:L937-945` | OwnerLocalIpc → `OwnerLocal`; Tcp → `Unauthenticated`. The value is passed to `familiar_ledger::route` and to `control_plane::route_action_at` / `route_action_with_authority` (`L955-978`). | EP-03 (transport only) | none | **No.** |
| NEP-06 | Familiar ledger actions (#1209): `coven.familiars.ledger.{register,adopt,retire,revoke,restore,get}.v1` | `api.rs:L946-963` → `familiar_ledger.rs:L573-625` (`route`), `L644-737` (`execute`), `L230-278` (`read_declarations`) | Refuses any caller that is not `OwnerLocal` (`L586-594`; the second layer after NEP-04). Mutating actions run in one immediate transaction, with an idempotent replay keyed by adoption key and request digest (`L739-765`). They write **only** SQLite tables `familiar_ledger_{commands,roots,revisions,events}` (`L716-728`, `L798-...`, `L882-883`, `L931-...`, `L1003-1004`, `L1039-...`, `L1096-...`, `L1123-1124`, `L1144-...`), and those tables are protected by append-only and immutability triggers (`L85-171`). `adopt` and `restore` read the daemon `familiar-binding` signing key (`automations/authority_keys.rs:L214`). | `familiars.toml` lookup by exact id (`cockpit_sources::roster_identity`, a new read-only function inserted at `cockpit_sources.rs:L148-199`); `SOUL.md`, `IDENTITY.md` and `ward.toml` read through `threads_gate::read_surface_if_exists` (`L241`; `threads_gate.rs` is unchanged); `WardConfig::from_toml_str` used **refuse-only** (`L264-268`, `InvalidWard`); head revision comparison (`L1024-1031`) | `familiarId` → ID (re-looked-up in the roster); `rootId` / `revisionId` → ID (looked up in the ledger, `L960-966`); `expectedRevisionId` → REV (compared to the daemon head, 409 `RevisionConflict`); `reason` → NOTE (≤ 500 bytes); `adoptionKey` → ID (idempotency) | **No.** It writes no familiar-home file, no `familiars.toml`, no `~/.coven/pending/`, no `ward_audit` and no `ward_manifest`. It does not feed any Ward/Threads EP. The only consumers of ledger state are this route, the NEP-04 prefix check (`control_plane.rs:L216`), and `familiar_issuer.rs`, whose only reference outside its own file is `mod familiar_issuer;` (`main.rs:L50`), so it has **no production caller**. No `api.rs`, `threads_gate.rs`, `ward*.rs`, `proposal_*.rs` file references `familiar_ledger` or `familiar_contract` except the `/actions` arm. The `familiar-contract` crate (new dependency, `Cargo.toml`, rev `93f486cf`) is used only by `familiar_ledger.rs` and `familiar_issuer.rs`. |
| NEP-07 | `GET /main-session` (#1186, #1269) | `api.rs:L1373` → `main_session_routes.rs:L237-256` | read | main-session store | `scope` query → PAGE-like selector | **No.** |
| NEP-08 | `POST /main-session/turn` | `api.rs:L1374-1376` → `main_session_routes.rs:L258-...` | Pipes input into a live bound session (`api::record_input`, `L402`) or launches one through `api::launch_session_with_executor` with the **transport authority** (`L464-467`). It refuses a harness, familiar or root that does not match the bound session (`L330-340`). | existing session-launch checks (unknown familiar → `unknown_familiar`, launch policy `api.rs:L2508`, maintenance writer), per-scope OS file lock at `~/.coven/main-session-locks/` (`L62-90`) | `prompt` / attachments / `projectRoot` → DATA; `familiarId` → ID (re-looked-up by `launch_session`); `harness` → DATA | **No.** Session launch is not Ward- or Threads-governed (the Set A scope note on `api.rs` session code holds). The main_session modules have 0 Ward/Threads references and no production file writes other than the lock directory. |
| NEP-09 | `POST /main-session/reset` | `api.rs:L1377-1379` → `main_session_routes.rs:L672-729` | rotates the main-session record and may relaunch | main-session store; scope lock | `familiarId` / harness / root settings → ID / DATA | **No.** The handler takes no `authority` argument, so a TCP caller can rotate the main session. That is an observation outside Threads scope. |
| NEP-10 | `POST /main-session/rollover` | `api.rs:L1380-1382` → `main_session_routes.rs:L731-786` | archives and rotates the record in one transaction | main-session store | DATA | **No.** As NEP-09, no `authority` argument. |
| NEP-11 | `If-None-Match` header | parsed at `daemon.rs:L6677-6682`; used at `L6262-6267` (`write_api_response_conditional`, `L6296-...`) | conditional response (304) only | — | header → cache validator | **No.** The daemon now parses **4** headers (`content-length`, `host`, `origin`, `if-none-match`), against 3 at Set A. |

**Changed, not new:**

- The `/actions` scheduler-wake list (`api.rs:L979-1003`) gained 7 automation action names. That is a wake hint only.
- The `launch_session` refactor (`api.rs:L2429-2446`, `launch_session_with_executor`) moved the launch into an executor closure. Its authority parameter and launch-policy checks (`L2508`, `L3001`) are unchanged in kind. It is outside Ward/Threads.

**Result.** At Set B there are 11 new enforcement points (NEP-01 to NEP-11):

- **One decides a Threads-governed outcome.** NEP-03 does, and only by refusing.
- **None permits a Ward- or Threads-governed mutation, or feeds an input into one.**
- **None treats a client value as authority.**

Every new transport check (NEP-01, NEP-02, NEP-04, NEP-05, and NEP-06's owner check) derives from EP-03's listener-based `RequestAuthority`.

## 3. CX-1 at Set B

Line references in this section are `coven@f772b513:crates/coven-cli/src/<file>` unless marked. The threads_gate.rs refs are unchanged from Set A, because that file is byte-identical (section 5).

### What CX-1 is at Set B (unchanged mechanics)

- **Intake.** `POST /familiars/{id}/edits` still wraps the client string as `ward::Authorization::signed_by(<text>)` at `api.rs:L6071-6082` (`signed_by` at `L6073`). The handler doc comment at `api.rs:L5973-5974` says the field "is retained for wire compatibility but never upgrades this proposal endpoint into Tier-0 write authority". The comment was also present at Set A, because the whole familiar block `api.rs:L5423-8046` (Set B `L5528-8151`) is byte-identical.
- **Gate 1.** It is still string equality, with no signature check: `ward.rs:L1719-1724` (`is_authorized`). It is applied to Tier 0 at `ward.rs:L1683-1692`.
- **Writer label.** `WriterId("principal:{text}")` is still produced at `threads_gate.rs:L360-363` (gate) and `L1192-1195` (staging), and persisted at `L1323-1346`.
- **Back to authorization.** `authorization_from_writer` is at `api.rs:L16654-16660`.
- **Receipts.** Neither receipt binds `pending.writer`:
  - legacy: `legacy_submission_authority`, `api.rs:L13835-13901`, receipt fields at `L13868-13889`;
  - scheduled: `api.rs:L14557-14625`.
- **Disclosure.** The expected value is still disclosed to any caller by `GET /familiars/{id}/ward`, at `api.rs:L6782` (EP-31).

### CX-1e: closed by coven#1278 (`21988a00`)

`21988a00` touches only `api.rs` (+23) and `api_legacy_submission_tests.rs` (+86). It is in `f772b513`'s history (`226bfcc8..f772b513`, 165 commits). The production change is one inserted block, between EP-59 and EP-60, in `decide_threads_proposal_inner`:

- **Refusal: `api.rs:L11430-11452`.**
  - The condition at `L11437-11440` is `decision == "approve" && applying_state.is_none() && review_kind == PendingReviewKind::Authority && scheduled.is_none()`.
  - On a match it restores the claim (`L11442`), finishes the audit reservation (`L11443`), and returns 409 `legacy-authority-approval-unsupported` (`L11444-11451`).
  - It writes no terminal row and no apply intent.
- **Bindings it relies on:**
  - `review_kind` is bound at `api.rs:L10679` from `PendingReviewKind::from_stored_value`, and `None | Some("authority")` maps to `Authority` (`api.rs:L8067-8073`). A missing `reviewKind` is therefore caught.
  - `scheduled` is bound at `L10919`.
  - `applying_state` is bound at `L10812`.
- **Ordering.** The block runs after EP-59 (Tier-0 / Ward-control refusal, `L11290-11429`) and before:
  - EP-60 semantics (`L11453`);
  - the recovery branch (`L12167-12410`);
  - EP-73, the `validate_fail_closed` writer binding (`L12412-12477`).
- **Reachability of EP-73 for legacy unscheduled authority-lane proposals at Set B:**
  - `approve` with no applying state is refused at `L11437-11452`.
  - `reject`, veto and supersede terminalize at EP-68 (`L11951-12007`), before EP-73.
  - An applying state (recovery) returns at `L12409`, before EP-73 at `L12412`.
  - So **no path at Set B evaluates `validate_fail_closed` with a legacy unscheduled writer label.**
- **The writer label is not an input to the new refusal.** The condition reads only the verb, `applying_state`, `review_kind` and `scheduled`. A `principal:{live fingerprint}` label and `client:unsigned` are refused alike.
- **Regression test: `crates/coven-cli/src/api_legacy_submission_tests.rs:L177-261`, `legacy_authority_tier_one_approval_is_refused_even_with_the_principal_label`.** It is included into the api test module at `api.rs:L36006-36008` (`include!`). The test:
  1. stages a real legacy authority-lane proposal through `gate_protected_edits` with `signed_by("fpr-val")` and non-empty `gated_targets` on a Tier-1 surface (`L189-216`);
  2. asserts `scheduled().is_none()`, `review_kind.is_none()` and writer `principal:fpr-val` (`L218-220`);
  3. mends the fray so the writer label would be consulted (`L224-230`);
  4. asserts that approve returns 409 `legacy-authority-approval-unsupported` (`L232-243`), with the file untouched, no terminal row, no apply intent, and the proposal still pending (`L244-250`);
  5. asserts that a later reject terminalizes with 200 (`L253-259`).

  It does not separately stage a `client:unsigned` label. Static reading shows the refusal does not read the label (above).
- **Not run.** This packet ran no tests. Exact-SHA CI is part B's concern.

**Residual (stated, not a Set B permit).** The guard exempts `applying_state.is_some()`: "Recovery of an apply that already began is not affected" (comment `L11435-11436`, commit message).

- Recovery (`L12167-12410`, byte-identical to Set A EP-72) completes an apply whose `proposal-apply-intent` row (EP-75) already exists. It requires that row through EP-49 / EP-63 (`load_proposal_apply_intent` `L16075-16119`; recovery commitment `L15492-15562`).
- At Set B a legacy authority-lane approval cannot write a new apply intent. EP-75 (`L12536-12581`) lies after both the guard and EP-73.
- So the residual covers only an apply intent written by a pre-`21988a00` daemon, which had made the CX-1e permit decision itself.
- Inside recovery, the writer-derived `Authorization` reaches only Ward Gate 1: `recovery_authorization` at `L12191`, `apply_after_review_approval` at `L12226-12230`, and rollback at `L12322-12326`. Gate 1 changes only Tier-0 verdicts, and EP-59 (`L11290-11429`) has already refused Tier 0 on that path. That is class (b), the same as CX-1b.
- Not found: a test that resumes a pre-fix legacy authority-lane apply intent at Set B.

### CX-1a to CX-1d at Set B

Every cited range is byte-identical to Set A after the line offset (section 1). The classes are unchanged.

| Site | Set B refs | Class at Set B |
|---|---|---|
| **CX-1a** Ward Gate 1, direct intake (EP-14 → EP-15 / EP-22) | Gate-1 call `api.rs:L6119-6145` → `ward.rs:L1561-1620`, `L1683-1692`, `L1719-1724`. EP-15 refuses any Tier-0 or Ward-control target irrespective of the verdict: `api.rs:L6147-6210`, target set `protected_proposal_targets` `api.rs:L5913-5953`. Direct apply holds `AuthorizedProtectedChange` (`requires_coherence` `ward.rs:L2195-2200`; `apply_inner` `ward.rs:L1804-1896`). | **(b)** |
| **CX-1b** Ward Gate 1 at decision (EP-59 → EP-64 / EP-77) | `authorization_from_writer(&pending.writer)` at `api.rs:L11290`; Tier-0 / control refusal `L11343-11429`; coherence approve refuses `AuthorizedProtectedChange` at `api.rs:L11652-11663`; `ward.rs` coherence apply `L2024-2082` and scheduled apply `L2089-2155` refuse it; legacy apply `ward.rs:L1923-1991` refuses only `Blocked` (`L1958`), but EP-59 runs first. | **(b)** |
| **CX-1c** Threads gate on direct intake (EP-19) | The only production caller still passes `gated_targets: &[]` (`api.rs:L6275`, call `L6267-6277`). `threads_gate.rs:L376-401` (identity rows with the approver label) and `L403-470` (dormant writer-bound branch) are unchanged. The `gate_protected_edits(` call-site grep at Set B finds only `api.rs:L6267` plus the definition at `threads_gate.rs:L327` (tests excluded). | **(b)**, dormant |
| **CX-1d** EP-73 writer binding, scheduled proposals | EP-73 `api.rs:L12412-12477` (refusal branch `L12430-12475`); weave built with `Some(&pending.writer)` in EP-66 (`api.rs:L11852-11861`, inside `L11828-11918`). The other two `Some(&pending.writer)` weave builds (`L11356-11365`, `L11745-11754`) feed only `revalidation_failure_weave_hash`, which is recorded on refusal rows; `threads_gate.rs:L542-545` replaces any `principal:*` label with the live principal, and `L595-601` self-binds a non-principal label; `coven-threads@78e47c9d:crates/coven-threads-core/src/validate.rs:L152-161`, which is unchanged since `0021fd26` (empty `git diff 0021fd26 78e47c9d -- validate.rs weave.rs`). | **(b)**, refuse-only |
| **CX-1e** EP-73, legacy unscheduled authority lane | Refused before EP-73 by `api.rs:L11437-11452` (above). | **Closed.** No permit at Set B. Residual: recovery of a pre-fix apply intent, class (b) at Set B. |

**Count:** at Set B, CX-1 has **0 class-(a) sites**. CX-1a, CX-1b, CX-1c and CX-1d remain class (b), and are named here as required. The fingerprint is still a client-asserted value that reaches daemon predicates (Gate 1, the Threads writer label), but no remaining site lets it permit anything the unsigned value would not.

Test coverage carried from Set A: `tests/threads_protected_intake.rs` and `tests/threads_e2e.rs` are unchanged (`git diff --stat 226bfcc8 f772b513` is empty for both), so the Set A coverage notes for CX-1a and CX-1d apply as written. Still not found: a test that approves a scheduled Tier-1 proposal to success with writer `client:unsigned`. The CX-1d "unsigned ≡ matching" reading remains static.

## 4. Cave delta

**Set B Cave SHA: `c60bbef90bdb859f4cb4f2fb44685eb02d939b71`** ("Polish Research Desk navigation and Studio workflows (#5937)"). It was committed 2026-10-12T00:23:16-05:00, which is 05:23:16Z.

- **Rule.** Take the last first-parent commit on `origin/main` whose committer date is at or before `f772b513`'s committer date, 2026-10-12T05:47:11Z. The ref read was local `origin/main` at `284282a7`, without a fetch.
- **Discrepancy with the scope document.** The scope document proposed `de266c11`, but that commit's committer date is 2026-10-12T01:04:27-05:00, which is 06:04:27Z, after the Coven target. The rule therefore selects `c60bbef9`. `258e587f` is an ancestor of `c60bbef9`.
- **The choice does not change this section.** `git diff --stat c60bbef9 de266c11` over the Threads-facing paths below is empty.

**Threads-facing diff `258e587f..c60bbef9`.** Paths: `src/lib/threads-adapters.ts`, `src/lib/proposal-authority.ts`, `src/lib/proposal-flow.ts`, `src/app/api/proposals/**` and `src/app/api/threads/**`. 6 files changed, +290/−42, matching the scope document's count. The commits are `c9388e64c`, `7e1f1efff`, `f1648e72a`, `8388ce751`, `25740a969` and `adba56809`.

| File | Change | Enforces policy? Forwards an authority claim? |
|---|---|---|
| `src/app/api/proposals/submit/route.ts` (new, `f1648e72a`) | `POST` behind `rejectNonLocalRequest` (`L10-11`), body ≤ 512 KiB, `parseProposalSubmission`, then `forwardProposalSubmission`. Supporting files outside the listed set, also new in this range: `src/lib/proposal-submission.ts` (+51), `src/lib/server/proposal-submission.ts` (+26). | **No authority claim forwarded.** The parser accepts **exactly 3 keys**: `familiarId`, `target`, `contents` (`proposal-submission.ts:L19-29`; `Object.keys(body).length !== 3` → reject). The forwarder builds a fresh body, `{ edits: [{ target, contents }] }`, for `POST /api/v1/familiars/{id}/edits` (`server/proposal-submission.ts:L13-21`). `principalKeyFingerprint` cannot pass through. Cave-side input checks (id regex, relative path with no `..`, ≤ 64 KiB) are **pre-filters**. The daemon re-adjudicates everything: EP-10 id resolution, EP-11 budget, Gate 2 path materialization, EP-15, EP-19 and EP-20. The outcome mapper exposes only confirmed dispositions (`L31-51`). |
| `src/app/api/proposals/route.ts` | `GET ?view=outcomes` → `adapter.proposalOutcomes()` | Read only. |
| `src/lib/threads-adapters.ts` (+158) | Paginated reads of daemon `GET /threads/proposals` with duplicate, cursor and contract checks. `proposalOutcomes()` reads `ward_audit` terminal rows **read-only** (`DatabaseSync(..., { readOnly: true })`) and refuses on any non-unique terminal. Stricter pending-empty verification. Header doc now names `daemon` as the default adapter (`adba56809`). | **No.** Read-side validation only. The decision forwarder (`decide`, around `L705-755`) is unchanged: its request body is `{ expectedRevision?, note? }` (`L725`) to `/api/v1/threads/proposals/{id}/{decision}` (`L452-453`, `L730`). Those are REV and NOTE, the two fields Set A EP-41/EP-48 classify. |
| `src/lib/proposal-authority.ts` (+50) | It accepts the validated daemon evidence extensions `identityEvidence`, `autoRegressionEvidence`, `probes` and `probeSummary`, with strict shape checks. Its comment reads: "These diagnostics remain part of the canonical revision; none grants authority." | **No.** It validates daemon output for display, and refuses to display malformed envelopes. Nothing it computes is sent to the daemon. |
| `src/lib/proposal-flow.ts` (+65) | The UI decision outcome now requires a daemon-confirmed terminal (`decision ∈ {approved,…}`, `blocked === false`, `meta.adapter === "daemon"`, matching `proposalId`); otherwise it reports `unconfirmed`. Refusal copy was reworded. | **No.** It interprets responses for display. It does not gate a daemon decision. |

**Authority-claim grep over Cave `src` at `c60bbef9` (non-test files):**

- 0 hits each: `principalKeyFingerprint`, `principal_key_fingerprint`, `principal:`, `signed_by`, `approverId`, `decisionActor` and `owner:local-ipc`.
- `approvalId`: 2 hits, both in `src/lib/tweet-thread-protocol.ts` (social thread publishing, unrelated).
- `writer:` / `"writer"`: 25 hits across 11 files. In Threads code the hits are daemon-output normalizers and display (`proposal-authority.ts`, `threads-read.ts`, `proposal-normalize.ts`, `weave-map.ts`) and one fixture. No request body sets a writer.
- Every Cave call site of the daemon's `/edits` and `/threads/proposals*` routes: `server/proposal-submission.ts:L15` and `threads-adapters.ts:L451-453`, `L730`.

**Unchanged since Set A:** `src/app/api/proposals/[id]/approve/route.ts`, `.../reject/route.ts`, `src/app/api/threads/**`, and `rejectNonLocalRequest` (`api-security.ts:L36-...`). The only `api-security.ts` change adds `rejectNonLocalOrMobileRequest` and `isMobileAccessRequest` (+29). **No proposal or threads route uses that mobile variant.** All three mutating proposal routes call `rejectNonLocalRequest`: approve `L14`, reject `L13`, submit `L10`.

**Conclusion for section 4.**

- At `c60bbef9`, no Cave code forwards `principalKeyFingerprint` or any other authority claim. Decision bodies carry REV and NOTE only, and submission bodies carry DATA only.
- No Cave code makes a policy decision the daemon relies on. Cave's checks are UI gating, display validation, or pre-filters, and the daemon re-adjudicates each of them.

**Observation for the reviewer: pre-existing, not new at Set B, and not assessed at Set A.**

- Cave reaches the daemon over the owner socket or pipe (`src/lib/coven-daemon.ts`, `socketPath` / `WINDOWS_PIPE_PREFIX` `L51-55`), so every Cave-forwarded request arrives as `RequestAuthority::OwnerLocalIpc` (EP-03).
- For approve and reject (R11/R12, transport-gated by EP-04), which HTTP callers can reach that owner-local client is therefore decided by Cave's `rejectNonLocalRequest` (loopback Host, sidecar token, Origin), a Cave-side check.
- The daemon itself consults nothing Cave asserts. `request_authority.rs:L1-5` documents transport ownership as the trust root.
- Whether "Cave as an owner-local proxy" falls under "no Cave surface is enforced on" is a reviewer judgement. Read literally, the daemon's predicate is daemon-owned (the listener), so this packet does not count it as a counterexample.
- The new submit proxy adds no authority. R4 (`/familiars/{id}/edits`) is not transport-gated at the daemon, and TCP callers can reach it directly.

## 5. Coverage statement

**Method.**

- All reads were `git -C <repo> show <sha>:<path>`, `git diff [-U0|--stat|--shortstat] <a> <b> -- <path>`, `git grep <pattern> <sha> -- <path>`, `git log` and `git ls-tree`. Output was piped through `sed`, `grep`, `awk`, `nl` and `diff <(…) <(…)`.
- No checkout, worktree, fetch, scratch file, test, build, CI query or GitHub write. Nothing under `~/.coven` was touched except this output file.
- **Line mapping.** Set A `api.rs`, `ward.rs` and `daemon.rs` ranges were mapped to Set B with an `awk` pass over `git diff -U0` hunk headers. The pass computes the line offset and flags any range a hunk modifies, or inserts into strictly inside.
- **Range check.** Every untouched range was then confirmed by `diff`ing the extracted Set A and Set B line ranges. The results are below.

**Files with empty `git diff --shortstat 226bfcc8 f772b513`** (Set A line refs carry unchanged):

`threads_gate.rs`, `proposal_scheduler.rs`, `proposal_store.rs`, `ward_audit_census.rs`, `ward_decision.rs`, `ward_identity.rs`, `ward_probes.rs`, `threads_clock.rs`, `output_format_auto.rs`, `api_routes.rs`, `familiar_identity.rs`, `memory_import.rs`, `ward_migrate.rs`, `api_response.rs`, `tests/threads_protected_intake.rs` and `tests/threads_e2e.rs`. Threads core: `git diff 0021fd26 78e47c9d -- crates/coven-threads-core/src/{validate,weave}.rs` is empty. The only crate change is `audit.rs` (+49/−1, `cecdb5b8`, memory-admission channel), and Coven has no constructor caller of it. `memory_entry_admitted` appears only in the audit filter list `api.rs:L6963`, a test at `L35081`, and CLI help at `main.rs:L1307`.

**Changed files and how they were checked:**

- **`api.rs` (+785/−58).** All production hunks were read. Byte-identical Set A → Set B ranges:
  - `L738-796` → `L761-819`;
  - `L839-856` → `L862-879`;
  - `L926-1301` → `L997-1372`;
  - `L1302-1390` → `L1383-1471`;
  - `L5423-8046` → `L5528-8151`;
  - `L9068-9862` → `L9173-9967`;
  - `L9869-11319` → `L9979-11429`;
  - `L11320-12917` → `L11453-13050`;
  - `L13260-16782` → `L13410-16932`.

  The scheduler `L12918-13261` and test clock `L17056-17147` are identical after removing `checkpoint(Phase::…)` and its `use` line. `checkpoint` is a no-op without `threads-test-clock`, and with the feature it only appends a log line (`threads_request_diagnostics.rs:L212-223`).
- **`ward.rs` (+92/−19).** Production hunks: four `#[cfg(test)]` gates on non-production helpers, two on no-check approval wrappers, and removal of the unused `apply_after_scheduled_approval` no-check wrapper. The rest is tests. All 13 cited Set A ranges are untouched.
- **`request_authority.rs` (+11/−3).** Full diff read: it adds `allows_chat_context_admission`, and renames `allows_automation_receipt_access` → `allows_owner_automation_access`. `allows_ward_proposal_access` is unchanged at `L22-24`.
- **`daemon.rs` (+2214/−139).** The transport function `handle_http_stream_with_lifecycle` (`L6087-6274`) was read in full against Set A `L5450-5598`. Other EP ranges were mapped and untouched. Production references to Ward/Threads are the scheduler start sites only (`L5049-5073`, `L5766`, `L6898`), which match Set A (`L4514`, `L5138`, `L6137`).
- **`store.rs` (+408/−21).** `WardAuditReservation` hunks: diagnostics traces, removal of two unconditional `PRAGMA wal_checkpoint(PASSIVE)` calls (admission and release; the on-demand TRUNCATE remains, `L853-863`), and a no-op refactor of `ward_audit_reservation_bytes`. Admission, refusal, token and purpose logic are unchanged. **Durability effects (#1132) were not assessed here.** They belong to items 2 and 4 in part B.
- **`store/schema.rs` (+55/−17).** Connection-pragma changes (#1132). Hunk headers only; not read for item 3.
- **`cockpit_sources.rs` (+52).** A read-only `roster_identity` was inserted. `write_familiar_icon` moved from `L196-270` to `L248-322` with an identical body.
- **`eval_loop.rs`.** Production diff read (mutation lock, v2 read-side fields).
- **`mobile_memory/gateway.rs`.** Full diff read.
- **`control_plane.rs` (+1856/−162).** `automation_transport_rejection` (`L207-245`) read. The rest was census-checked only: 0 Ward/Threads references.
- **`main.rs`.** S2, S3 and S4 ranges are byte-identical (`L4637-4652`, `L4667-4687`, `L1854-1871`).

**Dispatch census.** The `match` arm headers (lines starting `        (` inside the match) number 80 at Set A (`L858-1389`) and 84 at Set B (`L886-1470`). A text diff shows exactly 4 additions, all `/main-session*`.

**Grep census, `api.rs` production** (Set A `L1-17225`, Set B `L1-17380`; same regexes as the Set A packet):

| pattern | Set A | Set B | delta attributed to |
|---|---|---|---|
| `validate_fail_closed` | 1 | 1 | — |
| `\.evaluate\(` | 3 | 3 | — |
| `proposal_decision_semantics\(` | 2 | 2 | — |
| `gate_protected_edits\(` | 1 | 1 | — |
| `ensure_proposal_window_opened_audit` | 7 | 7 | — |
| `append_[a-z_]*audit[a-z_]*\(` | 27 | 27 | — |
| `INSERT INTO ward_audit` | 4 | 4 | — |
| `principalKeyFingerprint\|principal_key_fingerprint\|signed_by\|authorization_from_writer` | 12 | 13 | the #1278 comment at `L11432` |
| `trusted_decision_approver` | 7 | 7 | — |
| `protected_proposal_targets` | 4 | 4 | — |
| `"blocked": true` | 68 | 69 | the #1278 refusal at `L11447` |
| `api_error\(` | 219 | 219 | — |
| `quarantine` | 55 | 55 | — |
| `bail!\|ensure!` | 73 | 74 | the `SessionRuntime::launch_main_session` default `bail!` at `L599` |
| `allows_[a-z_]+\(` | 4 | 4 | — |
| `RequestAuthority::` | 2 | 4 | the `CommandAuthority` mapping at `L939`, `L942` |

**Module census.** Files under `crates/coven-cli/src` matching `crate::ward::|crate::ward\b|\bward::[A-Za-z_]|threads_gate::|coven_threads_core::|proposal_store::|proposal_scheduler::|ward_identity::|ward_probes::|decide_threads_proposal`: the Set A and Set B lists differ only by the addition of `familiar_ledger.rs`. The Set B list has 16 files, including test-only files.

**Ledger and issuer consumers.** `familiar_ledger::` / `familiar_ledger_*` are referenced only from `api.rs:L952-955`, `control_plane.rs:L216` and `familiar_issuer.rs`. `familiar_issuer` is referenced outside its own file only by `mod familiar_issuer;` at `main.rs:L50`.

**New-module file-write census** (production portion, `fs::write|fs::rename|create_dir|File::create|OpenOptions|remove_file|write_atomically`):

- `main_session*.rs`: 1 hit, the lock directory `~/.coven/main-session-locks`.
- `chat_context_admission.rs`, `familiar_ledger.rs`, `familiar_issuer.rs`: 0.

**Cave** (`258e587f..c60bbef9`):

- The diff over the 5 listed path groups was read in full. Production files were read in full; test files were not read.
- Also read: `src/lib/proposal-submission.ts`, `src/lib/server/proposal-submission.ts`, and the `api-security.ts` diff.
- A route-gate grep covered `src/app/api/{proposals,threads}/**/route.ts`.
- Authority-claim greps over non-test `src` are listed in section 4.

**A Set A gap found and recorded.** The Set A EP-03 row named only the Unix socket and TCP listeners. The Windows named-pipe listener, which maps to `OwnerLocalIpc` through `HostGuard::Disabled` behind an owner-only SDDL, existed at Set A (`daemon.rs:L6120-6194`, SDDL `L5949-5968`) and is unchanged at Set B (`L6881-6968`, SDDL `L6710-6729`, byte-identical). Section 1 adds it to EP-03.

**Not covered:**

- No tests or CI were run or read. Exact-SHA CI is part B's concern.
- `store.rs` / `store/schema.rs` durability under #1132 is part B's concern, for items 2 and 4.
- The internals of `control_plane.rs`, the 17 new `automations/*` files, `main_session*.rs`, `familiar_ledger.rs` (beyond `route`, `execute`, `replay`, `read_declarations`, and the `register` / `revoke` / `restore` / `adopt` input reads) and `chat_context_admission.rs` (beyond `reject_api_intent` and the `check_*` functions) were checked by census and targeted reads, not line by line. None references Ward/Threads code except `familiar_ledger.rs`'s two read and validation calls.
- `coven-threads-core` internals were treated as daemon-owned predicates, as at Set A.
- Cave proxy middleware (`proxy.ts`, the mobile marker) and `rejectNonLocalRequest`'s body were not read. That body is unchanged since Set A.
- No live Cave or live daemon run.

## 6. Proposed item-3 verdict

**Proposed: supported.** This is evidence, not an acceptance; a human decides. One reviewer judgement is named below: Cave acts as an owner-local proxy (section 4).

Half (ii), "every enforcement point traces to a daemon-owned predicate or audit row", holds at `f772b513`:

- **CX-1e is closed.** The only class-(a) site at Set A was CX-1e. At Set B, `api.rs:L11430-11452` (coven#1278, `21988a00`) refuses every approval of an unscheduled authority-lane proposal before the writer-bound `validate_fail_closed` at EP-73 (`L12412-12477`). It does so without reading the writer label. No path at Set B evaluates EP-73 with a legacy unscheduled writer label (section 3), and a regression test exists: `api_legacy_submission_tests.rs:L177-261`.
- **CX-1a to CX-1d remain class (b) and are named here.** The client-asserted `principalKeyFingerprint` still reaches Ward Gate 1 and the Threads writer label as an *input*. At every remaining site it is compared, recorded, or overridden; it never permits anything the unsigned value would not.
  - CX-1a and CX-1b: Tier 0 is refused irrespective of the verdict (EP-15 `api.rs:L6147-6210`; EP-59 `L11290-11429`).
  - CX-1c: the writer-bound gate branch is dormant (`gated_targets: &[]`, `L6275`).
  - CX-1d: scheduled proposals are refuse-only.
- **The residual is historical.** Recovery of an apply intent written by a pre-fix daemon is exempt from the new refusal. At Set B that path rests on the daemon's own `proposal-apply-intent` audit row, and its writer-derived `Authorization` reaches only Gate 1 after EP-59 (class b).
- **Every Set A enforcement point carries forward.** All 77 have byte-identical bodies, except four whose changes are not authority-bearing (section 1): EP-03 transport refactor with the same mapping, EP-80 and EP-90 diagnostics checkpoints, and EP-91 eval-loop mutation lock.
- **No new enforcement point trusts a client value.** Of the 11 new points (section 2), one decides a Threads outcome, and only by refusing (NEP-03). None permits a Ward- or Threads-governed mutation or feeds one. The new familiar ledger (NEP-06) writes only its own SQLite tables, uses `WardConfig` refuse-only, and no Ward or Threads point consumes its state. Every new transport check derives from listener-based `RequestAuthority`.

Half (i), "no Cave or client-side surface is enforced on":

- **Daemon side: supported.** No daemon decision reads Cave state. `RequestAuthority` is transport-derived (EP-03, including the Windows owner-only pipe). Persisted descriptors are still re-derived (EP-46, EP-49, EP-61, EP-88), and the decision actor is still the daemon constant (`api.rs:L55`).
- **Cave side, at `c60bbef9`: supported** (section 4).
  - No Cave code forwards `principalKeyFingerprint` or any other authority claim. The new submit route builds `{ edits: [{ target, contents }] }` from a body of exactly 3 keys, and decision bodies carry only `expectedRevision` and `note`.
  - Cave's own checks are UI gating, display validation, or pre-filters. The daemon re-adjudicates each of them, so no daemon decision relies on them.
- **Reviewer judgement (pre-existing since before Set A, not a Set B change).**
  - Cave calls the daemon over the owner socket or pipe, so its forwarded approve and reject requests carry `OwnerLocalIpc`.
  - Which HTTP callers can reach Cave's approve and reject routes is therefore decided by Cave's `rejectNonLocalRequest`.
  - The daemon's own predicate is still daemon-owned (the listener), so this packet does not count it as a counterexample.
  - A reviewer who reads "no Cave surface is enforced on" as also excluding Cave's proxy gate would mark half (i) partial on this point. That reading would apply equally at Set A.

**Non-blocking follow-ups** (none is a counterexample to item 3):

1. a test that approves a scheduled Tier-1 proposal to success with writer `client:unsigned`, to pin the static CX-1d reading;
2. a test that the new refusal ignores a `client:unsigned` label, or a note that the guard reads no label (static: it reads no label);
3. dropping `principalKeyFingerprint` at intake entirely, which would retire CX-1a to CX-1d as inputs;
4. outside Threads scope: `POST /main-session/reset` and `/rollover` take no transport authority (NEP-09, NEP-10), and the eval-loop `{id}` and `force` observation from Set A still stands (EP-91).

## 7. Every "not found"

- A test that resumes, at Set B, a legacy authority-lane apply intent written before `21988a00` (the recovery residual in section 3).
- A test that stages the CX-1e shape with `client:unsigned` and asserts the new refusal. The regression test uses `principal:fpr-val` only.
- A test that approves a scheduled Tier-1 proposal to success with writer `client:unsigned`. This was not found at Set A, and the test files are unchanged.
- A production caller of `gate_protected_edits` with non-empty `gated_targets`. The only production caller is `api.rs:L6267-6277` with `&[]`.
- A production caller of `familiar_issuer` (the issue path). Its only reference outside its own file is `mod familiar_issuer;` at `main.rs:L50`.
- Any Ward or Threads enforcement point that reads `familiar_ledger_*` tables.
- A daemon producer of `memory_entry_admitted`, `principal_authorized_write` or `ward_updated`. This was not found at Set A, and no new module references those constructors (section 2 census).
- Transport-authority checks on `POST /main-session/reset` and `/main-session/rollover` (NEP-09, NEP-10). This is a Threads-irrelevant observation.
- Validation of the eval-loop `{id}` path segment at Set B (EP-91). The dispatch arm and `familiar_workspace` delegation are unchanged.
- Cave: any use of `principalKeyFingerprint`, `principal_key_fingerprint`, `principal:`, `signed_by`, `approverId`, `decisionActor` or `owner:local-ipc` in non-test `src` at `c60bbef9`.
- Cave: a request body that sets a writer or approver for a daemon route.
- Cave: a proposal or threads route that uses the new `rejectNonLocalOrMobileRequest` mobile gate.
- Cave: the scope document's `de266c11` does **not** satisfy the committer-date rule. It lands 17 minutes after `f772b513`, and `c60bbef9` was used instead. The Threads-facing files are identical between the two.
- Cave: live Cave evidence, which this static pass did not run.

---

<!-- ===== PART B ===== -->

# EVIDENCE — NOT AN ACCEPTANCE

OpenCoven/coven-threads #13, Part B of the Set B follow-up packet (bead `threads-uqx.19`): checklist items 2 and 4, plus a crash-durability review of coven#1132.

This is engineering evidence only. It does not accept, reject or defer anything, does not check a #13 box, and does not stand in for Val's human self-review under `specs/PHASE-5-SOLO-MAINTAINER-REVIEW.md`.

| Field | Value |
| --- | --- |
| Date | 2026-10-13 |
| Set B Coven | `f772b51377f88d97c9d44d87093e5b641231972e` |
| Set B Threads core | `78e47c9d2430582128679fd4a54f9966f2f4f976` (pinned by that Coven commit) |
| Set A Coven | `226bfcc89ff6cad4bc9cc9618dad6fea970ecf58` |
| Set A Threads core | `0021fd2662d0b82328371ee3a6957d645f776fda` |
| Tests re-run | **None. No tests run.** No cargo, build or sqlite3 was run. Results come only from the CI runs cited. |
| Method | Read-only `git show/grep/diff/log/ls-tree` and read-only `gh run view` / `gh pr view` |

---

## 1. Item 2 — Gate-4 fail-closed: Set B trace and diff vs Set A

> "every `ApprovalPath` ends in daemon re-materialization. No path allows apply without live revalidation at deadline."

All lines below are `crates/coven-cli/src/api.rs` at `f772b513` unless another file is named.

### 1.1 Entry points

- **Human route.** `POST /api/v1/threads/proposals/{id}/approve` goes to `decide_threads_proposal` (`L1091` → `L9945-9960`). That calls the inner function with `revision_required=true` and actor `OWNER_LOCAL_DECISION_ACTOR`.
- **Scheduler.** `process_due_threads_proposals` (`L13051-13418`) calls `decide_threads_proposal_automatic` (`L9962-9974`) in three places:
  - for a fresh due proposal (`L13368`), after `ensure_proposal_window_opened_audit` (`L13348`) and the due computation (`L13355-13363`);
  - to resume a durable request (`L13317`);
  - to resume a claim (`L13273`).
- **Common path.** Every approval goes through `decide_threads_proposal_inner` (`L10193-12791`).

### 1.2 Per-variant trace

| `ApprovalPath` | Who can approve | Semantic gate (`proposal_decision_semantics`, `L7830-7963`) | Window and close |
| --- | --- | --- | --- |
| `AutoRegression{veto:None}` | Scheduler only. It is due at once (`L13356`); a human reject returns `proposal-not-human-decidable` (`L7943-7944`). | approve allowed (`L7900`) | No window (`window_close` is `None`, `L7949-7956`) |
| `AutoRegression{veto:Some}` | Scheduler when `now >= veto_deadline` (`L13357-13360`); a human approve before then is refused | `proposal-minimum-visibility-open` / `proposal-veto-window-open` (`L7885-7899`) | Opening ensured (`L13348`; `L11104-11116`). Close `Applied`, `replay_hash_matched: true` (`L7949-7956`) |
| `FamiliarCoherence` | same as above | same | same |
| `HumanApproval` | Human route only; the scheduler never approves it (`L13361-13362`) | approve allowed (`L7879`) | No window |
| `HumanApprovalWithRationale` | Human route only | non-empty note required, else `proposal-rationale-required` (`L7880-7884`) | No window |

**Revalidation, common to all five, before any write.** These checks are skipped only in Recovery mode, which is gated separately on a matching durable apply intent (`L11559-11610`):

1. **Identity:**
   - `ward_identity::candidate_rejection`: a failure is `RevalidationFailed`;
   - `candidate_binding` compared with the stored `identity_evidence`: a mismatch is `EvidenceDiverged` (`L12009-12025`).
2. **Auto-regression evidence:** `revalidate_auto_regression` (`L12026` → `L15646-15694`).
   - On `AutoRegression` it recomputes `output_format_auto::regression_evidence` from live config and compares it with the stored evidence and the probe projection (`L15696-15729`). It then requires exactly one `proposal_submitted` anchor with matching `autoRegressionEvidence` (`L15676-15692`).
   - On any other path, stored auto evidence fails closed (`L15662-15673`).
3. **Scheduled paths only:**
   - live tier escalation;
   - `revalidate_scheduled_materialized_before`, which requires exact live before-image bytes or absence (`L12096-12109` → `L7980-8009`);
   - `revalidate_scheduled_approval_policy`, which recompiles the live `approval_tiers` and requires the same highest path (`L12108` → `L8011-8042`).
   - Each refusal writes a typed `proposal_rejected` terminal with `EvidenceDiverged` or `RevalidationFailed`, plus a window close when one exists, and consumes the claim (`L12028-12067`, `L12110-12153`).
4. **Pre-apply gates:**
   - `ward::supports_atomic_approved_writes()` refusal (`L12157-12166`);
   - apply intent (`L12561`), then applying state (`L12573`);
   - `ward_config_is_unchanged` (`L12587-12606`).

**Final authority check at commit.** This still exists.

- **Where it runs.** `ensure_proposal_final_authority_unchanged` (`L15731-15767`) is built as the `final_authority_check` closure for both the Initial apply (`L12613-12647`, `ApprovedApplyMode::Initial`) and the Recovery apply (`L12207-12238`). Both go through `apply_after_review_approval` (`L15918-15969`) to `Ward::apply_after_scheduled_approval_with_commit_check` (`ward.rs:2089-2155`).
- **What it checks:**
  - the Ward config bytes are unchanged, which carries the live approval policy;
  - the live baselines are reloaded;
  - the recovery commitment is recomputed and compared, which covers authority, identity and baselines;
  - `revalidate_auto_regression` is re-run.
- **When it runs** inside `write_atomically_if_unchanged`, after staging:
  - before the first swap (`ward.rs:3319`);
  - immediately before each target swap (`L3335`);
  - after each swap's verification (`L3433`);
  - at batch final verification (`L3485`);
  - and to refine a failed config open (`ward.rs:2102-2114`).
- **What a failure does.** A failure rolls back (`fail_after_conditional_rollback`). In the decision, final-authority drift with an opened window writes a typed revalidation-failure close (`L12659-12677`); otherwise it writes a refusal audit and restores the pending file (`L12679-12691`).
- **Scope of "in-transaction".** The check runs inside the apply, between staging and swaps. It is not inside a SQLite transaction.

### 1.3 Diff against Set A

Ranges are function bodies, compared with `diff` of `git show` output.

| Function | Set A `226bfcc8` | Set B `f772b513` | Diff |
| --- | --- | --- | --- |
| `decide_threads_proposal_inner` | `L10083-12658` | `L10193-12791` | **One insertion, #1278 (`21988a00`): `L11430-11452`.** Nothing else. |
| `decide_threads_proposal` + `_automatic` | `L9840-9865` | `L9945-9974` | `_automatic` wraps the call in `checkpoint(Phase::SchedulerDecisionBegin/Returned)`, added by `979e704d` |
| `proposal_decision_semantics` … `revalidate_scheduled_approval_policy` | `L7725-7939` | `L7830-8044` | identical |
| `revalidate_auto_regression` … `ensure_proposal_final_authority_unchanged` | `L15496-15619` | `L15646-15769` | identical |
| `apply_after_review_approval` | `L15768-15820` | `L15918-15970` | identical |
| `process_due_threads_proposals` | `L12918-13268` | `L13051-13418` | only `checkpoint(Phase::Scheduler*)` lines (`979e704d`) |
| `ensure_proposal_window_opened_audit{,_with_conn}` | `L13564-13674` | `L13714-13824` | identical |
| `ward.rs` | — | — | #1131 (`adf88403`) adds `#[cfg(test)]` to test-only wrappers and deletes the unused no-check `apply_after_scheduled_approval`. #1276 (`f772b513`) adds tests. The production `_with_commit_check` path and `write_atomically_if_unchanged` are unchanged. |

**Explanation of every change:**

- **#1278 (`21988a00`, "refuse approval of unscheduled authority-lane proposals").**
  - **Condition:** `decision == "approve" && applying_state.is_none() && review_kind == PendingReviewKind::Authority && scheduled.is_none()`.
  - **Effect:** it restores the pending file, finishes the reservation, and returns 409 `legacy-authority-approval-unsupported`. It writes no audit row.
  - **Placement:** after the protected-target refusal and before `proposal_decision_semantics`.
  - **What it does not touch:** reject, expiry and Recovery (`applying_state` present).
  - **Effect on item 2:** every `ApprovalPath` proposal is scheduled, so this branch is unreachable for all five variants and their traces are unchanged. It only adds a refusal.
  - **Effect on a Set A gap:** the gap was "non-scheduled legacy review kinds use `validate_fail_closed` (now `L12412`ff) and were not exhaustively mapped". A fresh approval can no longer enter the unscheduled **authority** lane. The unscheduled **coherence** lane still follows its probe-evidence route and was not re-mapped here.
  - **Test:** `legacy_authority_tier_one_approval_is_refused_even_with_the_principal_label` (`src/api_legacy_submission_tests.rs:178`). It asserts the 409 reason, no terminal, no apply intent, a retained pending file, and that a later reject still terminalizes.
- **Diagnostics in `decide_threads_proposal_automatic` and the scheduler (`979e704d`; extended by #1132, which removed the passive-checkpoint phases).**
  - `checkpoint()` returns `()` and cannot alter control flow (`threads_request_diagnostics.rs:212-223`).
  - Without the `threads-test-clock` feature its body is `let _ = phase;`.
  - With the feature it appends to `daemon-recovery.log` only while a fixture-mode request trace is active.
- **#1274 (`171876e3`).** Its `api.rs` change is in `kill_session` (error formatted with `{:#}`) and its test. It is **not** on the decision path.
- **#1132 (`0fab796c`).** It does not touch the decision logic. It changes store durability (§2). It creates no path to apply without revalidation: every effect is still preceded by the checks above. What it changes is whether the evidence of an applied decision survives power loss, which bears on item 4.

### 1.4 Covering tests at Set B

All of these exist at the cited locations.

- **`tests/threads_e2e.rs`:**
  - `scheduled_window_replay_fails_closed_across_real_daemon_restart` (`:661`)
  - `out_of_band_reviewed_drift_is_refused_without_execution` (`:960`)
  - `reviewed_human_approval_validates_and_audits_without_veto_window` (`:867`)
  - `changed_human_path_cannot_erase_a_real_opened_window` (`:606`)
  - `retired_corpus_scheduled_intake_survives_restart_and_applies_once` (`:394`)
- **`tests/support/threads_output_auto_cases.rs`:**
  - `output_auto_deadline_and_restart_reject_regression_and_valid_evidence_drift` (`:462`)
  - `output_auto_final_commit_rechecks_policy_and_expected_image` (`:592`)
  - `output_auto_no_veto_survives_restart_without_window_events` (`:197`)
- **`tests/support/threads_final_commit_cases.rs`:** `final_commit_identity_drift_closes_opened_window_without_applying` (`:230`)
- **`tests/support/threads_identity_replay_cases.rs`:**
  - `scheduled_valid_identity_bytes_reject_at_deadline_and_after_restart` (`:84`)
  - `scheduled_valid_soul_bytes_reject_at_deadline_and_after_restart` (`:89`)
  - `scheduled_valid_roster_metadata_rejects_at_deadline_and_after_restart` (`:94`)
  - `scheduled_valid_active_policy_rejects_at_deadline_and_after_restart` (`:99`)
- **`tests/threads_identity_invariants.rs`:**
  - `scheduled_identity_replay_covers_manual_and_immediate_routes` (`:136`)
  - `direct_identity_drift_at_commit_refuses_real_daemon_write` (`:266`)
  - `changed_valid_identity_evidence_refuses_approval_after_restart` (`:572`)
- **`src/api.rs` unit tests:**
  - `threads_scheduled_human_required_enforces_rationale_and_applies` (`:37265`)
  - `threads_scheduled_deadline_replay_refuses_diverged_before_image` (`:38429`)
  - `threads_scheduled_deadline_replay_failure_closes_window` (`:38540`)
  - `threads_scheduler_revalidates_old_window_instead_of_expiring_it` (`:40244`)
- **New since Set A:** `src/api_legacy_submission_tests.rs::legacy_authority_tier_one_approval_is_refused_even_with_the_principal_label` (`:178`), from #1278.

**CI.** The exact-SHA runs are in §4:

- the four real-daemon targets passed on Linux (105/45/39/34) and Windows (82/44/35/33) in both runs;
- macOS has no exact-SHA Threads-suite result.

The Set A caveat was a failing exact-SHA Windows run. At Set B both exact-SHA Windows Threads jobs are green.

**Remaining gaps:**

- **macOS:** no exact-SHA Threads-suite result.
- **Unscheduled coherence lane:** not re-mapped.
- **Composite re-materialization:** it is live before-image equality plus a stored-diff replay, not a fresh live diff hash. This is unchanged from Set A, item 5.

---

## 2. coven#1132 crash-durability review

Commit `0fab796c755221cb322c056d4ca05bd9b96b150a` (merged 2026-09-18T20:45:59Z). It is an ancestor of Set B `f772b513` and is **not** an ancestor of Set A `226bfcc8` (`git merge-base --is-ancestor`). Files: `crates/coven-cli/src/store.rs`, `crates/coven-cli/src/store/schema.rs`, `crates/coven-cli/src/threads_request_diagnostics.rs`, `crates/coven-cli/tests/support/threads_output_auto_cases.rs`.

### 2.1 SQLite settings before and after

| Setting | Set A `226bfcc8` | Set B `f772b513` |
| --- | --- | --- |
| `journal_mode` | WAL (`store/schema.rs:580`, `enable_wal_with_retry`) | WAL, unchanged (`store/schema.rs:622`) |
| `synchronous` | **Never set.** Writable connections run only `busy_timeout`, `foreign_keys` and `recursive_triggers` (`store/schema.rs:565-567`, `605-607`), so SQLite's compiled default applies. The Set A unit test `open_store_retains_initialization_wal_without_close_and_reopen` asserts `synchronous >= 2` (`store.rs:6230`), i.e. FULL or EXTRA. | **`NORMAL` (1)** on every writable connection: `WRITABLE_CONNECTION_PRAGMAS` (`store/schema.rs:611-615`), applied in `configure_initializing_connection` (`L590`) and `configure_runtime_writable_connection` (`L646`). `open_initialized_store` (the per-request open) goes through the latter (`schema.rs:134`ff). The same test now asserts `== 1` (`store.rs:6576`). |
| `temp_store` | default | `MEMORY` (`schema.rs:615`) |
| `wal_autocheckpoint` | 4 MiB worth of pages (`configure_ward_audit_wal`) | unchanged: `WARD_AUDIT_WAL_AUTOCHECKPOINT_BYTES = 4 MiB` (`schema.rs:15`, applied `L660-671`) |
| `journal_size_limit` | 16 MiB | unchanged (`schema.rs:16`, `L672-675`) |
| `fullfsync` / `checkpoint_fullfsync` | not set (grep found no use) | not set |

The libsqlite3-sys build flags for the pinned version (0.38.2, `bundled`) were not inspected; that crate version is **not found** in the local cargo cache. The Set A test assertion is the evidence for FULL-or-higher at Set A.

### 2.2 Checkpoint behaviour before and after

| Site | Set A | Set B |
| --- | --- | --- |
| `WardAuditReservation::acquire` | unconditional `wal_checkpoint(PASSIVE)` (`store.rs:688`), then `TRUNCATE` only if the WAL would exceed its budget (`L699-706`) | PASSIVE removed. `TRUNCATE` only when WAL size plus the admission would exceed `wal_limit_bytes` (`store.rs:853-863`; default limit 128 MiB, `schema.rs:12`) |
| `WardAuditReservation::finish` | `wal_checkpoint(PASSIVE)` after the release COMMIT (`store.rs:967-975`) | **removed** (`store.rs:1107-1137`) |
| Scheduled maintenance | — | PASSIVE only when the WAL is at least 16 MiB (`store.rs:4646-4657`, `MAINTENANCE_CHECKPOINT_WAL_BYTES` `L37`) |
| Compaction | `TRUNCATE` (`store.rs:3352`) | `TRUNCATE` (`store.rs:3545`), unchanged |
| SQLite autocheckpoint | at about 4 MiB of WAL | unchanged |

At Set B no code path on the approve-and-apply route syncs the WAL. Under `synchronous=NORMAL` the WAL is synced only when a checkpoint backfills it (SQLite documentation, not re-verified here). A checkpoint happens at about 4 MiB of new WAL, when maintenance sees at least 16 MiB, when an admission nears the 128 MiB budget, or at compaction. Otherwise the WAL reaches disk only through the operating system's own write-back, which Coven does not control. The other `wal_checkpoint` hits at Set B are in test code (`api.rs:7015`, `7317`; `event_writer.rs:1924` is inside `mod tests` at `L993`).

### 2.3 Order of operations on the approve-and-apply path (Set B, `ApprovedApplyMode::Initial`)

All `api.rs` lines are at `f772b513`. "WAL only" means the COMMIT returns after `write()` to the WAL, with no fsync, at Set B. At Set A the same COMMIT fsynced the WAL.

| # | Step | Code | What is made durable, and how |
| --- | --- | --- | --- |
| 1 | In-process Ward write/audit lock; open store | `api.rs:10203`, `10291` | — |
| 2 | Capacity reservation row (`coven_ward_audit_reservations`) and decision claim (pending file rewritten with `decision_request`, then renamed to the claim path) | loop `api.rs:10337-10552`; `store.rs:865-968`; claim `api.rs:14993-15021` | Reservation: WAL only. Claim: `proposal_store::write_atomic` stages, `sync_all`s, then renames (`proposal_store.rs:308-317`); the rename to the claim path is a plain `fs::rename` (`L390`). No parent-directory fsync anywhere. |
| 3 | `proposal_window_opened` row, veto paths only, if not already written by a scheduler pass | `api.rs:11104-11116` → `13732`ff | WAL only |
| 4 | Revalidation: identity, auto-regression evidence, live tier, before-images, approval policy | `api.rs:12009-12155` | read-only, or a typed rejection terminal (WAL only) |
| 5 | **Apply-intent row** (`ward_audit`, `event_type='validation_verdict'`, `decision='proposal-apply-intent'`), autocommit | `api.rs:12561` → `16017-16052` | **WAL only** |
| 6 | Claim rewritten with `ProposalApplyingState` | `api.rs:12573` → `16005-16013` → `write_atomic` | staged file `sync_all`, then rename; no directory fsync |
| 7 | `ward_config_is_unchanged`; test-only pause | `api.rs:12587`, `12607` | — |
| 8 | **Target write** with in-apply authority checks | `api.rs:12635` → `ward.rs:2089-2155` → `write_atomically_if_unchanged` `ward.rs:3224`ff | Each target is staged and **`sync_all`ed** (`ward.rs:4735-4746`). On Linux and macOS it is then swapped in with `renameat(…, EXCHANGE)` (`ward.rs:5664-5684`); on Windows it is moved in two steps (`L5615-5633`). There is no directory fsync. The authority check runs before the first swap (`L3319`), before each swap (`L3335`), after each swap (`L3433`) and at final verification (`L3485`). |
| 9 | Store-identity check | `api.rs:12742` | read-only |
| 10 | **`finalize_approved_proposal`**: one `BEGIN IMMEDIATE … COMMIT` containing the apply audit rows (`threads_gate::append_apply_audit_records_at`), the **baseline advance** (`advance_surface_baseline_from_bytes`) and the **`proposal_approved` terminal** carrying typed `window_close` | `api.rs:12746` → `16784-16838` | **WAL only**. The three writes are atomic with each other. |
| 11 | Reservation release (`DELETE` + COMMIT) | `api.rs:12769` → `store.rs:1107-1137` | WAL only. At Set A this step also ran a PASSIVE checkpoint. |
| 12 | Claim removal | `api.rs:12771` → `15035-15040` → `proposal_store::remove_existing` | no fsync |
| 13 | HTTP 200 | `api.rs:12772-12789` | — |

The Recovery mode (`api.rs:12167-12389`) re-applies the same steps 8 to 12. It first requires the recorded apply-intent row to match the claim's applying state (`api.rs:11559-11610`).

### 2.4 Crash-point table

"Process crash" means the daemon dies and the OS keeps running. Every completed `write()`, COMMIT and rename is then in the OS page cache and survives. That is true at both SHAs, and SQLite WAL mode is durable across an application crash at both FULL and NORMAL.

"Power loss" means an OS crash or power cut. SQLite then recovers a **prefix** of committed transactions: WAL frames carry chained checksums, and recovery stops at the first invalid frame (SQLite's WAL design, not re-verified here). Files survive independently of SQLite. The census `classify` row is from `ward_audit_census.rs:312-394`, unchanged between the sets. The census lists only histories that have an opening row (`L232-235`).

| Crash after step | Process crash (Set A = Set B) | Power loss, Set A (FULL) | Power loss, Set B (NORMAL) |
| --- | --- | --- | --- |
| 2-4 (claimed, no intent) | No target written. The durable request is resumed or rejected later. Census: `OpenUnverified` (opening plus claim). | same | same. The reservation and a same-request opening row may also be lost; the scheduler rewrites the opening. No effect has been written. |
| 5 (intent committed, claim has no applying state) | Intent row present, target untouched. Census: `UnprovableApply` until a decision completes. | same. The intent was fsynced at COMMIT. | Intent may be lost: same as rows 2-4. No effect has been written. |
| 6-7 (applying state persisted, target not yet swapped) | Recovery mode verifies the intent (`api.rs:11559-11610`) and applies once. Census `UnprovableApply` until then. | same | Intent may be lost while the claim with applying state survives. Recovery then **quarantines** the claim (`api.rs:11562-11571`, `proposal-recovery-intent-unverifiable`). That is safe, because the targets are unchanged, but the proposal is stranded. Census: `QuarantinedOpening` if an opening is durable; **not listed** if there is none. |
| **8-9 (target swapped, finalize not committed)** | Recovery treats the bytes as already applied (`ward.rs:3255-3266`) and finalizes. Census `UnprovableApply` until then. This is the designed recovery window. | The intent row is durable, because its COMMIT fsynced before step 6. The terminal is absent. Same outcome as a process crash: `UnprovableApply`, then recovery completes. | **Hazard.** The new target bytes can survive (their data is `sync_all`ed and the rename is journaled by the filesystem), while the recovered SQLite prefix ends before step 5. Then **the apply-intent row, the baseline advance and the terminal are all gone.** (a) If the claim with applying state survives, recovery quarantines it (`api.rs:11562`). The census reports `QuarantinedOpening` when an opening is durable. It lists **nothing** when there is no opening: the HumanApproval, HumanApprovalWithRationale and `AutoRegression{veto:None}` paths, or a veto path whose opening was first written in this same request (step 3). (b) The claim could revert to a state without applying state; whether it can depends on the filesystem, since the claim rename is not directory-fsynced. In that case `proposal_recovery_is_proven_unapplied` returns true (`api.rs:10052-10054`). Re-deciding then finds the live bytes differ from the before-image (`api.rs:12103-12104`) and writes a **`proposal_rejected` terminal with `evidence_diverged`** for a change that is on disk (`api.rs:12110-12143`). The census reports `TypedTerminalRecorded`. |
| 10 (finalize committed) | Terminal found first (`api.rs:10299-10327`); the request is idempotent and artifacts are cleaned up. `TypedTerminalRecorded`. | durable | Finalize is not synced, so it may be lost as a suffix: same as rows 8-9. |
| 11-13 (reservation released, claim removed, 200 returned) | complete | durable | **The client has seen 200**, yet the intent, apply rows, baseline and terminal can all still be lost. The claim removal may also persist. The proposal then has no claim, no pending file and no audit. The census reports `OrphanedOpening` if an opening is durable, otherwise **nothing**. The target holds the approved bytes with no ledger entry and no baseline advance. |

How long the Set B hazard window lasts: from step 5 until the WAL is next synced (§2.2), which no Coven code bounds by time. On a quiet workstation the 4 MiB autocheckpoint may not fire for a long time, so in practice the bound is the operating system's write-back. This was **not measured**.

Not traced: the consequence of an un-advanced baseline after case (a) or (b). The next intake would see the live surface differ from the stored baseline. Whether that is reported as out-of-band drift was **not traced** in this pass.

### 2.5 Direct answers

**Can a file write persist while the audit rows that justify it are lost?**

- **Process crash: no, at either SHA.** The apply-intent row is committed before the claim's applying state and before any target swap. Recovery or the census always finds it.
- **OS crash or power loss at Set B: yes.** The apply-intent INSERT (`api.rs:16030`) and the finalize transaction (`api.rs:16788-16832`) are WAL-only. The target bytes are `sync_all`ed (`ward.rs:4744`) and swapped in (`ward.rs:5675`) between them, and nothing orders WAL durability before the swap. After reboot the protected file can hold the approved bytes with no apply intent, no baseline advance and no terminal. The census reports this as `QuarantinedOpening` or `OrphanedOpening`, or does not report it at all when the path has no opening. In a filesystem-dependent sub-case it records a false `proposal_rejected / evidence_diverged`.

**Did #1132 widen that window? Yes.**

- At Set A, the default FULL mode fsynced the WAL at the apply-intent COMMIT. So under power loss the intent row was durable before any target swap. Power loss between the swap and finalize left `UnprovableApply` plus a recoverable claim, which is the designed case.
- #1132 removed both that ordering guarantee and the release-side PASSIVE checkpoint that would have synced the WAL shortly after finalize.
- Two caveats apply to Set A:
  1. The rename of the target and of the claim was never directory-fsynced, at either SHA.
  2. Neither SHA sets SQLite `fullfsync`. On macOS, SQLite's default sync is plain `fsync()`, while Rust's `File::sync_all` uses `F_FULLFSYNC` there. Both statements come from general platform knowledge and were **not verified** in these checkouts. So even Set A's intent-before-effect ordering was not strict under power loss on macOS.

**Tests.** None found that simulate a lost WAL suffix while files persist. The decision failpoints (`ApplyIntentBeforeWrite` `api.rs:12583`, `ApplyBeforeAudit` `L12741`, `AuditBeforeCleanup` `L12770`) simulate a process crash only. A wider grep for power-loss or fsync tests was started and stopped before it returned, so this is **not found**, not proven absent.

### 2.6 #1132's own durability claims, checked against the code

| PR claim | Check |
| --- | --- |
| "`synchronous = NORMAL` … durable across a daemon crash, WAL synced before each checkpoint, no fsync per commit; only an OS crash or power loss can lose the final commits" | Consistent with the code (`schema.rs:597-615`) and SQLite's documented WAL semantics. It is accurate for SQLite in isolation. It does not consider that Ward target writes are fsynced independently and land *between* two WAL-only commits. So the "final commits" it may lose include the evidence for a durable protected write. |
| "the store is per-user local session state and the daemon is the only writer" (Risk) | **Incomplete.** The same store holds `ward_audit`: the append-only apply-intent, terminal and baseline ledger that census and recovery treat as authority evidence. |
| "no documented durability contract references fsync-per-commit (grepped `docs/`, `specs/`, `SECURITY.md`)" | A grep of Coven `docs/ specs/ SECURITY.md` at `f772b513`, and of Threads `specs/ docs/` at `78e47c9d`, found no fsync-per-commit contract. No intent-before-effect *durability* requirement was found in the Threads specs either. The design relies on ordering, apply intent before effect, which FULL had made power-loss durable as a side effect. |
| "The 30 s proposal scheduler, 60 s maintenance pass, and 60 s automations tick checkpoint on their own cadence" | Describes the pre-change state. At Set B no unconditional periodic checkpoint remains: the reservation checkpoints are gone or conditional, and maintenance requires 16 MiB (§2.2). |
| "`wal_autocheckpoint` (4 MiB) and the maintenance pass bound the WAL" | Consistent: `schema.rs:15`, `store.rs:37`, `L4646`. It bounds WAL *size*, not time to durability. |
| "a pinned WAL still fails closed with 507" | **Not verified**; the 507 mapping was not traced. |
| Risk level "medium"; rollback is "revert the commit" | No schema change, which is consistent. Per-connection pragmas only. |

**Related, out of scope:** `threads_gate.rs:1608-1612` publishes the pending file (`sync_all`) before `append_scheduled_submission_audit`. At Set B a power loss can therefore keep a pending proposal whose `proposal_submitted` row was lost. The opening writer then refuses to open a window without exactly one receipt (`api.rs:14497-14507`). The recovery outcome was not traced.

---

## 3. Item 4: audit completeness under both readings

> "`proposal_submitted → window_opened → close(reason)` chain is present and no gap exists between open and close. Close event has reason field."

### 3.1 What is unchanged (proof)

- `git diff 226bfcc8 f772b513` is empty for `ward_audit_census.rs`, `proposal_scheduler.rs`, `threads_gate.rs`, `proposal_store.rs`, `ward_decision.rs`, `ward_identity.rs`, `threads_clock.rs`, `output_format_auto.rs` and `ward_probes.rs`. Blob ids are identical at both SHAs: census `0d6b6f835d1c850741f60b1464a3ae9ae839ffdc`, scheduler `01019c21164ad126347b7e25a4bc605eca53655f`.
- The opening writer `ensure_proposal_window_opened_audit{,_with_conn}` is byte-identical: Set A `api.rs:13564-13674` diffs empty against Set B `api.rs:13714-13824`.
- In `process_due_threads_proposals`, Set A `L12918-13268` against Set B `L13051-13418`, the only additions are `checkpoint(Phase::Scheduler*)` diagnostics (19 lines). Its opening call sites are now `api.rs:13178`, `13246` and `13348`; the decision-time call site is `api.rs:11104-11116`.
- In Threads core, `0021fd26..78e47c9d` changes only `crates/coven-threads-core/src/audit.rs` (+49/−1, `cecdb5b8`). No changed line contains `TRIGGER`, `CREATE`, `RAISE`, `SELECT` or `INSERT`. At `78e47c9d` the trigger definitions are `ward_audit_require_single_terminal_insert` (`audit.rs:1329`), `ward_audit_require_proposal_approval_detail_insert` (`L1358`) and `ward_audit_require_window_close_detail_insert` (`L1419`). The append-only triggers are named at `L1011-1029`.
- Covering tests are present at Set B:
  - `assert_window_terminal` (`tests/threads_e2e.rs:2022`), used by `retired_corpus_scheduled_intake_survives_restart_and_applies_once` (`:394`), `explicit_supersession_closes_only_the_replaced_window` (`:783`), `scheduled_window_replay_fails_closed_across_real_daemon_restart` (`:661`) and `changed_human_path_cannot_erase_a_real_opened_window` (`:606`).
  - `tests/threads_terminal_recovery.rs`: `census_reports_orphaned_opening_without_familiar_or_pending_and_never_repairs` (`:34`), `census_reports_retained_claim_and_unprovable_apply_without_inventing_terminal` (`:55`), `public_coherence_approval_has_no_fabricated_window_close` (`:131`), `synthetic_legacy_opened_history_is_rejected_once_across_restart` (`:179`).
  - `api.rs` `threads_scheduler_terminalizes_unknown_review_kind_after_opening_window` (`:40285`).
  - Census units `all_normative_closes_are_typed_and_human_no_window_is_excluded` (`ward_audit_census.rs:534`) and `apply_intents_must_follow_the_opening_and_precede_any_terminal` (`:942`).
  - `support/threads_identity_replay_cases.rs::assert_scheduled_valid_identity_drift` (`:103`).
- **Changed:** `store.rs` (#1132 durability, §2; #1086 permissions; #1197 orphaned sessions, which does not touch `ward_audit`). This is the only item-4-relevant delta.

### 3.2 Correction to a Set A "not found"

The Set A packet said a submitted-before-opened check was "not found. No SQL or runtime check enforces it". That is incomplete:

- **There is a runtime precondition.** The daemon's only opening writer calls `scheduled_submission_authority` (`api.rs:13749`). That function refuses unless exactly one `proposal_submitted` row exists for the proposal (`scheduled_submission_authority_inner`, `api.rs:14497-14507`, message "scheduled proposal must have exactly one submission receipt"). The same check is present at Set A (`api.rs:14334`, message at `L14355`).
- **There is still no SQL constraint.** A direct insert, or a history whose receipt is later lost, is not prevented by the schema.
- **The opening is still lazy:** first scheduler pass, or decision time.

### 3.3 Assessment

**Original wording.** Every Set A limiter stands, apart from the correction above:

- no SQL ordering constraint;
- a lazy opening, so there is an interval with no opening row;
- quarantined or unprovable windows are reported but not closed.

#1132 adds an **OS crash or power-loss residual** (§2.4, rows 8-13):

- An applied change can lose its apply intent, its baseline advance and its terminal. The census then reports `QuarantinedOpening` or `OrphanedOpening`; these are "open without close".
- If the opening was written in the same request, the opening is lost too. The **whole chain** for an effect that landed is then absent, and the census lists nothing.
- In a filesystem-dependent sub-case, the chain is closed with a typed `proposal_rejected / evidence_diverged` although the change was applied. The "reason field" is present, but it misstates the outcome.
- At Set A the intent row was fsynced before the effect. Power loss could still leave `UnprovableApply`, the designed recovery case, but not a missing intent.

**Proposed: partial.**

**Decision 5 (#86, not merged).** For daemon crash and restart, every opened window ends in one of D5's three states:

- (1) `TypedTerminalRecorded`;
- (2) `OpenUnverified`;
- (3) `UnprovableApply`, `QuarantinedOpening`, `UntrustedArtifacts`, `OrphanedOpening` or `InconsistentHistory`, reported and never repaired.

The triggers still reject a terminal without typed close detail. With census and scheduler unchanged, this holds at Set B as at Set A, so **for process crash it is supported.**

Under power loss at Set B, three things happen:

- **Surviving openings stay classified.** Every opening that survives still falls into state (3) (`QuarantinedOpening` or `OrphanedOpening`), or into (1) in the false-reject sub-case. By D5's literal definition, a gap is an opened window in none of the states or one the census cannot see, so none of these is a gap.
- **Lost openings are invisible.** An opening lost together with the effect's audit leaves no opened window for the census to see (`ward_audit_census.rs:232-235` lists only histories with an opening). D5 does not name this case. It is the "census cannot see" situation, but caused by lost history rather than a census defect.
- **False-reject closes.** A false `evidence_diverged` close satisfies D5's state (1) while recording the wrong outcome.

D5 says a state-(3) window on a deployed profile "is a defect to investigate". #1132 makes such windows reachable without any daemon defect, through power loss alone.

**Proposed under D5: supported for daemon crash and restart; partial if OS crash or power loss is in scope.** Choosing the scope is a contract decision for Val. The alternative is to fix the Coven issue proposed in §5.

**Related.** At Set B, power loss can also drop a `proposal_submitted` row while its pending file (published first and `sync_all`ed, `threads_gate.rs:1608-1612`) survives. The opening writer then refuses (§3.2) rather than writing an opening without a receipt. What happens to that proposal next was **not traced**.

---

## 4. Exact-SHA CI at `f772b513`

Both runs have `headSha` exactly `f772b51377f88d97c9d44d87093e5b641231972e` and are attempt 1. Results come from read-only `gh run view` and job logs. The workflow source is `.github/workflows/ci.yml` at `f772b513`.

**Which jobs run the four Threads suites.** Three jobs run the isolated Threads-clock unit filter (`cargo test --locked -p coven-cli --bin coven --features threads-test-clock threads_`) and then `cargo test --locked -p coven-cli --test threads_e2e --features threads-test-clock --test threads_identity_invariants --test threads_protected_intake --test threads_terminal_recovery --no-fail-fast`:

- Rust tests (Linux) (`ci.yml:289-292`)
- Threads daemon tests (Windows) (`ci.yml:347-352`)
- Rust tests (macOS) (`ci.yml:392-395`), which runs only on `push` (`ci.yml:371`) and has `timeout-minutes: 20`

Rust tests (Windows) does not run them (`ci.yml:305-321`).

### Run 38209489872: `merge_group`, `gh-readonly-queue/main/pr-1276-…`, conclusion **success**

| Job (id) | Conclusion | Threads-clock unit filter | `threads_e2e` | `threads_identity_invariants` | `threads_protected_intake` | `threads_terminal_recovery` |
| --- | --- | --- | --- | --- | --- | --- |
| Rust tests (Linux) (114680814637) | success | 228 passed, 0 failed | 105 passed, 0 failed | 45 passed | 39 passed | 34 passed |
| Threads daemon tests (Windows) (114680814673) | success | 218 passed, 0 failed | 82 passed, 0 failed (259.94 s) | 44 passed | 35 passed | 33 passed |
| Rust tests (macOS) | **skipped** (push-only) | — | — | — | — | — |

The run's other jobs were Rust tests (Windows), Rust lint (Linux), Policy guard, Dependency audit, the npm smoke jobs (linux-x64 and windows), the two Automations bundles, and PR gate (114684907659). All of them succeeded. CLI performance baseline, both AFS mount backends, engine contract, Channels, OpenClaw bridge and Restricted runtime were skipped. All test results have 0 ignored.

### Run 38211237006: `push`, `main`, conclusion **cancelled**

| Job (id) | Conclusion | Threads-clock unit filter | `threads_e2e` | `threads_identity_invariants` | `threads_protected_intake` | `threads_terminal_recovery` |
| --- | --- | --- | --- | --- | --- | --- |
| Rust tests (Linux) (114686063664) | success | 228 passed | 105 passed | 45 passed | 39 passed | 34 passed |
| Threads daemon tests (Windows) (114686063761) | success | 218 passed | 82 passed (242.81 s) | 44 passed | 35 passed | 33 passed |
| Rust tests (macOS) (114686063678) | **cancelled** | 228 passed | **no result**: the target started at 06:32:04Z and the log ends `##[error]The operation was canceled.` at 06:34:14Z | not run | not run | not run |
| PR gate (114691743071) | **failure** | — | — | — | — | — |

**Why it was cancelled:** the macOS job started at 06:14:02Z and was cancelled at 06:34:14Z. That is 20 min 12 s against its `timeout-minutes: 20`. Its workspace test step alone took 13 min 22 s. CI concurrency cancels in-progress runs only for `pull_request` (`ci.yml:12-14`), so this was a **job timeout**, not a superseding push and not a test failure. No `threads_e2e` test outcome was printed on macOS. The PR gate failed as a consequence. All other jobs in the run succeeded, apart from the skipped OpenClaw bridge, Channels, Restricted runtime, AFS (Linux) and generic npm matrix jobs.

**Notes.**

- **Per-platform counts differ.** Linux runs 105, 45, 39 and 34 tests in the four targets; Windows runs 82, 44, 35 and 33. The difference is presumably platform-gated tests; which tests are gated was **not enumerated**.
- **No exact-SHA macOS Threads-suite result exists** for `f772b513`. The macOS isolated unit filter passed (228).
- **The earlier `main` push run on `21988a00` (38176762125) was also `cancelled`.** Its cause was not inspected.
- **Policy section 3:** these results are for `f772b513` only. They are not merged with Set A's run 34785628407, which failed on Windows. The Set A Windows caveat on item 2 is answered at Set B by two green exact-SHA Windows Threads jobs, provided the merge-queue run is accepted as exact-SHA evidence. Its `headSha` is exact; its ref is the merge-queue branch, not `main`.
- **None of these suites exercises power loss** (§2.5), so a green run says nothing about the #1132 finding.

---

## 5. Proposed verdicts

These are engineering proposals, not acceptances. Val decides under the policy's section 4.

| Item | Set A verdict | Proposed Set B verdict | Basis |
| --- | --- | --- | --- |
| 2 Gate-4 fail-closed | supported, with a CI caveat | **supported** | Inside `decide_threads_proposal_inner` the only diff between the sets is the #1278 refusal. That refusal applies only to unscheduled authority-lane proposals, which carry no `ApprovalPath`. The `decide_threads_proposal_automatic` change consists only of diagnostics calls; `checkpoint()` returns `()` and is a no-op without `threads-test-clock`. `proposal_decision_semantics`, the revalidation helpers, `ensure_proposal_final_authority_unchanged`, `apply_after_review_approval` and the ward commit-check path are byte-identical apart from #1131's `#[cfg(test)]` and dead-wrapper removals. The in-apply authority check is present in both Initial and Recovery modes. Exact-SHA Linux and Windows Threads suites are green in merge-queue run 38209489872 and push run 38211237006. Residuals: no exact-SHA macOS Threads-suite result (§4); #1132 does not create an apply-without-revalidation path (§2.5). |
| 4 Audit completeness, original wording | partial | **partial** | The Set A limiters stand, except one Set A "not found" that is corrected in §3.2. #1132 adds a power-loss residual: an applied change can lose its apply intent and terminal, and also its opening when that row was written in the same request. |
| 4 under Decision 5 (#86, not merged) | n/a | **supported for daemon crash and restart; partial at Set B if OS crash or power loss is in scope** | For process crash every opened window ends in one of D5's three states, and census and scheduler are byte-identical to Set A. Under power loss at Set B, D5's states still technically cover every surviving opening. But (a) a change can land with *no* chain at all, and (b) in a filesystem-dependent case it gets a typed `evidence_diverged` rejection although it was applied. D5's definition does not call either of these a "gap". Both defeat the purpose of the criterion. See §3.3. |

**#1132 finding (proposed): severity medium; should become a Coven issue.**

- **What:** since `0fab796c` (#1132), Ward-audited protected writes are not power-loss-ordered behind their audit evidence. The apply-intent row (`api.rs:16030`) and the finalize transaction (`api.rs:16788-16832`) are WAL-only under `synchronous=NORMAL` (`store/schema.rs:614`). The target bytes are `sync_all`ed (`ward.rs:4744`) and swapped in between them. After an OS crash or power loss, a protected surface can carry approved bytes with no apply intent, no baseline advance and no terminal. A 200 response does not imply a durable ledger entry.
- **Why medium, not high:** it needs an OS crash or power loss inside the unsynced window, and no process-crash path is affected. The effect was still fully revalidated before it was written, so no unauthorized content is written. What is lost is the audit, the baseline, and the census visibility of an authorized write.
- **Why not low:** the ledger is what census and recovery treat as authority evidence. #886's root criterion is the typed-terminal matrix with explicit handling of inconsistent history. The PR's risk note described the store as "session state".
- **Possible remedies, for the issue rather than this packet:**
  - restore WAL sync for the decision path's apply-intent COMMIT (and optionally the finalize COMMIT) only, for example a per-connection `PRAGMA synchronous=FULL` on the decision connection or an explicit WAL sync before the first target swap. This keeps #1132's latency win for submissions;
  - separately consider directory fsync after target and claim renames, which is pre-existing at both SHAs;
  - add a power-loss model test, for example a WAL-suffix truncation fixture with the target already swapped.

---

## 6. Every "not found"

Each entry is something not found, not verified, or not traced in this pass.

1. **libsqlite3-sys 0.38.2 build flags.** These are the flags behind `SQLITE_DEFAULT_SYNCHRONOUS` and `SQLITE_DEFAULT_WAL_SYNCHRONOUS` (pinned with `rusqlite` 0.40.2, `bundled`). **Not found:** that version is not in the local cargo cache. Set A's FULL-or-higher setting is inferred from the Set A test assertion `synchronous >= 2` (`store.rs:6230`).
2. **A test that simulates OS crash or power loss** (a lost WAL suffix while target or claim files persist). **Not found.** The existing decision failpoints model a process crash only. A broader grep for power-loss and fsync tests was interrupted before it returned, so this is not proven absent.
3. **A documented durability contract** requiring fsync per commit, or power-loss-durable apply intent before effect. **Not found** in Coven `docs/ specs/ SECURITY.md` at `f772b513`, nor in Threads `specs/ docs/` at `78e47c9d`.
4. **#1132's claim that "a pinned WAL still fails closed with 507".** **Not verified**; the HTTP 507 mapping was not traced.
5. **Recovery for a pending proposal whose `proposal_submitted` row was lost to power loss.** **Not traced** beyond the opening writer's refusal (`api.rs:14497-14507`).
6. **What happens downstream when a baseline is not advanced** after a power-loss apply (whether it is reported as out-of-band drift). **Not traced.**
7. **An exact-SHA macOS Threads-suite result for `f772b513`.** **Not found.** macOS was skipped in `merge_group`, and push run 38211237006 timed out during `threads_e2e`.
8. **Which tests are platform-gated**, explaining the Linux and Windows count differences (105/45/39/34 against 82/44/35/33). **Not enumerated.**
9. **The cause of the cancelled `main` push run on `21988a00`** (38176762125). **Not inspected.**
10. **An exhaustive re-map of the unscheduled coherence-lane decision route.** **Not done.** #1278 closes only the unscheduled authority lane.
11. **An SQL constraint that `proposal_submitted` precedes `proposal_window_opened`.** **Not found.** There is a runtime precondition in the opening writer (§3.2), which corrects the Set A packet.
12. **macOS fsync semantics.** That SQLite uses plain `fsync()` on macOS without `PRAGMA fullfsync`, while Rust `File::sync_all` uses `F_FULLFSYNC`, is general platform knowledge. It was **not verified** in these checkouts.
13. **SQLite WAL semantics relied on in §2:** with `synchronous=NORMAL` the WAL is synced only at checkpoint, and recovery keeps only a checksum-valid commit prefix. These come from SQLite documentation and were **not re-verified** against the bundled source.
14. **The duration of the Set B unsynced window** on a real workstation. **Not measured.**
15. **Whether a claim rename can be lost while a later target rename persists** (§2.4 case b). This depends on the filesystem and was **not verified** for APFS, ext4 or NTFS.
16. **Interrupted grep.** In the first session, one read-only grep (power-loss/fsync tests plus the `ProposalDecisionFailpoint` enum listing) was stopped by the operator before it returned. Its results were not used.

---

<!-- ===== PART C ===== -->

# EVIDENCE — NOT AN ACCEPTANCE

OpenCoven/coven-threads #13, follow-up packet at Set B, **Part C**: checklist items 1, 5, 6, 7 and 8, plus revision and CI bookkeeping. Bead `threads-uqx.19`.

This is engineering evidence only. It does not accept, reject or defer anything, does not check a #13 box, and does not stand in for Val's human self-review under `specs/PHASE-5-SOLO-MAINTAINER-REVIEW.md`.

| Field | Value |
| --- | --- |
| Date | 2026-10-13 |
| Set B Coven | `f772b51377f88d97c9d44d87093e5b641231972e` |
| Set B Threads core | `78e47c9d2430582128679fd4a54f9966f2f4f976` |
| Set B Cave | `c60bbef90bdb859f4cb4f2fb44685eb02d939b71` ("Polish Research Desk navigation and Studio workflows (#5937)", committer date 2026-10-12T05:23:16Z). See section 1. |
| Set A Coven | `226bfcc89ff6cad4bc9cc9618dad6fea970ecf58` |
| Set A Threads core | `0021fd2662d0b82328371ee3a6957d645f776fda` |
| Set A Cave | `258e587f9edce9cdfe200d6f5aff6076fdb2c645` |
| Tests re-run | **None. No tests run.** No cargo, pnpm, build or test was run. |
| Method | Read-only `git show/log/grep/diff/ls-tree/rev-list` and read-only `gh` (view/list/api GET). Two other read-only git commands were also used: `git rev-parse` (to resolve tips) and one `git merge-base --is-ancestor f772b513 origin/main` on the Coven clone, before I switched ancestry checks to `rev-list`. No fetch, so the remote tips are as of the local clones. |

---

## 1. Set B revisions

| Repo | Set B SHA | How it was fixed |
| --- | --- | --- |
| Coven | `f772b51377f88d97c9d44d87093e5b641231972e` | Given. Committer date 2026-10-12T05:47:11Z (epoch 1791784031). Subject: "test(ward): cover the approval_tiers unknown-tier and wrong-gate load rejections (#1276)". It equals Coven `origin/main` in the local clone at the time of reading. |
| Threads core | `78e47c9d2430582128679fd4a54f9966f2f4f976` | `coven@f772b513:Cargo.lock` pins `coven-threads-core 0.2.0`, `source = "git+https://github.com/OpenCoven/coven-threads?rev=78e47c9d2430582128679fd4a54f9966f2f4f976#78e47c9d…"`. |
| Cave | `c60bbef90bdb859f4cb4f2fb44685eb02d939b71` | The rule below. |

### Why this Cave SHA

The rule is the same as Set A: the last commit on Cave `origin/main`, first-parent, whose committer date is at or before the Coven target's committer date (2026-10-12T05:47:11Z, epoch 1791784031).

- **Pick:** `c60bbef90bdb859f4cb4f2fb44685eb02d939b71`, committer date 2026-10-12T05:23:16Z (epoch 1791782596), about 24 minutes before the cutoff.
- **Not the pick:** `de266c1149332f2cb686410066c631fcc1870b2d` (#5943, "Make X publishing failures recoverable without losing drafts"), committer date 2026-10-12T06:04:27Z (epoch 1791785067). It landed about 17 minutes *after* the cutoff. The scope document proposed it as "`main` on 2026-10-12" and asked for confirmation during the packet run; under the rule it does not qualify. Its first parent is `c60bbef9`.
- **Monotonic dates:** along Cave first-parent `origin/main` from `258e587f` to the current tip (`0fb839b4a5e006e725fefdbf82ebed66b928e1f3`), 318 steps, no committer date goes backwards. So "last at or before the cutoff" is well defined.
- **Ancestry:** `258e587f` (Set A Cave) is reachable from `c60bbef9` (`git rev-list c60bbef9 | grep 258e587f…` matched once). There are 310 first-parent commits in `258e587f..c60bbef9`.

**The pick does not change the Threads-facing Cave surface.** The 18 Threads-facing Cave files (`src/lib/{threads-adapters,proposal-authority,proposal-flow,weave-rail}.ts` and their tests, `src/lib/proposal-authority-daemon.test.ts`, `src/components/{proposal-approval,weave-rail}.tsx` and tests, `src/app/api/proposals/**`, `src/app/api/proposals-flow-e2e.test.ts`) have an **empty** `git diff` for `c60bbef9..de266c11`, and also for `c60bbef9..origin/main` (`0fb839b4`). Whichever of the three is used, the Threads-facing bytes are the same. The rule still picks `c60bbef9`.

**Cave Threads-facing delta `258e587f..c60bbef9`:** 13 files, +746/−116 including tests. Without the `.test.ts` files it is 7 files, +386/−59: `api/proposals/route.ts` (+7/−), `api/proposals/submit/route.ts` (+18, new), `components/proposal-approval.tsx` (+143/−), `lib/proposal-authority.ts` (+50/−), `lib/proposal-flow.ts` (+65/−), `lib/threads-adapters.ts` (+158/−), `lib/weave-rail.ts` (+4). The commits are the six the scope names: `adba56809`, `25740a969`, `8388ce751`, `f1648e72a`, `7e1f1efff`, `c9388e64c`. (The scope document's "6 changed, +290/−42" uses a file set I could not reproduce; the figures above come from the file list given here.)

---

## 2. Items 1, 5, 7, 8 (unchanged-source proof and tests)

**Method for this section.** For a whole file, "unchanged" means `git diff <A> <B> -- <file>` printed nothing. For a function inside a changed file, I extracted the item from its `fn`/`function`/`const` line to the closing brace at the same indentation, at both revisions, and compared SHA-256. Line numbers are given as A → B. Inside a byte-identical function, Set A's sub-ranges move by the same offset.

**Whole-file status, Coven `226bfcc8..f772b513`:**

- **Unchanged (empty diff):** `src/proposal_scheduler.rs`, `src/output_format_auto.rs`, `src/ward_probes.rs`, `src/threads_gate.rs`, `src/ward_audit_census.rs`, `src/ward_identity.rs`, `src/threads_clock.rs`, `tests/threads_e2e.rs`, `tests/threads_identity_invariants.rs`, `tests/threads_protected_intake.rs`, `tests/threads_terminal_recovery.rs`, `tests/support/threads_corpus_closure_cases.rs`.
- **Changed:**
  - `src/api.rs`: +785/−58.
  - `src/ward.rs`: +92/−19. The commits are #1131 `adf88403` (`#[cfg(test)]` gates and removal of a test-only wrapper, all at L934 or later) and #1276 `f772b513` (+74, tests only).
  - `tests/support/threads_output_auto_cases.rs`: +16/−4. The commits are `392af173` and #1132 `0fab796c`. The only change is the expected diagnostic-phase list inside `output_auto_submission_records_bounded_request_phases`, which this part does not cite.

**Whole-file status, Threads `0021fd26..78e47c9d`:** under `crates/`, only `crates/coven-threads-core/src/audit.rs` changed (+49/−1, commit `cecdb5b8`). The following are unchanged: `approval.rs`, `surface_regions.rs`, `identity_invariants.rs`, `pattern.rs`, `lib.rs` and `tests/phase5_retired_ward_corpus.rs`. `Cargo.toml` and `Cargo.lock` are also unchanged.

### Item 1 — RFC round-trip

**The §6 decision text is unchanged.** `specs/PHASE-5-APPROVAL-SEMANTICS.md` changed between `0021fd26` and `78e47c9d` in one commit, `bd358af` (2026-09-13, "docs: activate Val-approved Phase-5 self-review policy"). It changed:

- the status line;
- an added review-ownership amendment;
- the §6 heading, from "(Val; Nova attestation pending)" to "(Val; human coherence acceptance pending)";
- the §6 provenance paragraph (Nova sign-off replaced by Val's human acceptance);
- §7 and §8 wording.

No hunk falls inside the eight numbered decisions. I extracted them, from "1. **" through the line before "## 7.", at both revisions: 67 lines each, SHA-256 `ab6cce06…c70ae33a` at both. §6 is L318-395 at A and L326-405 at B; the decisions are L330-396 at A and L339-405 at B.

| §6 decision | Set A citation | Set B status |
| --- | --- | --- |
| 1. `ApprovalPath` separate from `Channel` | Threads `approval.rs` L82-114, L158-196, L405-431. Coven `ward.rs` `compiled_approval_tiers` L589-727. `api.rs` `revalidate_scheduled_approval_policy` L7906-7937 | `approval.rs`: file unchanged. `compiled_approval_tiers`: A L589-727 → B L589-727, **identical**. `revalidate_scheduled_approval_policy`: A L7906-7937 → B L8011-8042, **identical**. |
| 2. Delayed apply only, with a close reason | `approval.rs` L471-497, L537-560. `api.rs` scheduler `due` check L13203-13214 in `process_due_threads_proposals` L12918-13261. `proposal_decision_semantics` L7725-7858 | `approval.rs`: unchanged. `proposal_decision_semantics`: A L7725-7858 → B L7830-7963, **identical**. **`process_due_threads_proposals` changed:** A L12918-13261 → B L13051-13411. The whole diff is one added `use crate::threads_clock::request_diagnostics::{checkpoint, Phase};` and 18 added `checkpoint(Phase::Scheduler…);` lines. Remove those 19 lines and the body hashes the same as A (`ce349e53…`). `checkpoint` (`threads_request_diagnostics.rs:L212`) returns `()`. Without feature `threads-test-clock` it is `let _ = phase;`; with the feature it appends one log line. It cannot change control flow. The `due` block (`let due = match …approval_path` through `};`) is A L13208-13219 → B L13355-13366, **byte-identical**. |
| 3. Classify regions first; promotion forward-only | `approval.rs` L405-431. `surface_regions.rs` L318-326, L595-647. `proposal_scheduler.rs` L270-316 | All three files are unchanged. **Region-to-thread promotion code is still not found.** |
| 4. Deterministic where possible; ambiguity fails closed | `identity_invariants.rs` L211-307, L343-374, L885-915. `ward.rs` `enum ProbeId` | `identity_invariants.rs`: unchanged. `enum ProbeId`: A L254-259 → B L254-259, **identical**. |
| 5. Upstream RFC amendments before freeze | Process decision; familiar-contract PRs #3/#4 not found | Decision text unchanged. familiar-contract was again not inspected, so those PRs are still **not found**. Threads-55s note below. |
| 6. Daemon contract before Cave | `api.rs` `threads_proposals_response` L7347-7652 (`from_classification` at L7586-7603). Cave `proposal-authority.ts` L78-83, L801-822, L975-1015 | `threads_proposals_response`: A L7347-7652 → B L7452-7757, **identical**, so `from_classification` is at B L7691-7708. Cave `c60bbef9`: `APPROVAL_PATH_LABELS` A L78-83 → B L78-83, **identical**. `normalizeApprovalPath` (holds Set A's L801-822) A L797-826 → B L844-873, **identical**. `normalizeProposalAuthority` A L975-1015 → B L1023-1063, **byte-identical**. One Cave change nearby: `isCompleteScheduledEnvelope` now also requires `hasValidProducerEvidence(raw)` (from `8388ce751`). This *narrows* which staged envelopes Cave accepts, because probe reports and evidence byte arrays must be well-formed. It grants nothing, and the comment reads "none grants authority". |
| 7. Display labels as a daemon wire contract | See item 7 | See item 7. |
| 8. `proposal_window_opened` audit event | Threads `audit.rs` `AuditEventType` L141-174 and `validate_event_detail` L413-541. `api.rs` `ensure_proposal_window_opened_audit_with_conn` L13582-13662 | `audit.rs` changed, but `AuditEventType` A L141-174 → B L141-174 is **identical** and `validate_event_detail` A L413-541 → B L421-549 is **identical**. `ensure_proposal_window_opened_audit_with_conn`: A L13582-13662 → B L13732-13812, **identical**. |

The Set A gap citations are also unchanged. `output_format_auto.rs` (L12-136) is unchanged as a file. `surface_regions.rs` (`OutputFormatRegion` L510-594) is unchanged as a file.

**Threads crate delta `0021fd26..78e47c9d` (`cecdb5b8`, threads-55s, landed via merge `78e47c9d`, #72):**

- `WardAuditRecord::for_memory_entry_admitted` (A L351-379 → B L358-387, **changed**) gains a parameter, `channel: Option<Channel>`, between `source_attestation` and `decided_at`. The body now sets `channel,` instead of `channel: None`. A doc comment is added.
- The existing unit test's call now passes `Some(Channel::Deliberate)`. A new test, `memory_admission_records_the_channel_it_arrived_on` (`audit.rs:L2424`), checks every `Channel::ALL` value, checks that `Deliberate` differs from `Mutation`, and checks that `None` still validates.
- There is no SQL, trigger or schema change. The `ward_audit_require_proposal_approval_detail_insert` trigger range is A L1698-1757 → B L1706-1765, byte-identical. The commit message states that the `channel` column is nullable and that no `memory_entry_admitted` constraint reads it.
- **Bearing on §6.** The change touches only the `Channel` axis of a memory-admission row. It does not touch `ApprovalPath` or any proposal type. Decision 1 (separating `ApprovalPath` from `Channel`) is unaffected. Under decision 5 / threads-55s it records an audit-legibility fix in the Threads crate, with no daemon caller yet. The commit message itself defers the end-to-end round-trip to `threads-xpo`.
- **Coven has no producer of `memory_entry_admitted`.** At `f772b513`:
  - `git grep for_memory_entry_admitted` over the whole tree: **0 hits**.
  - `git grep -E 'MemoryEntryAdmitted|MemoryEntryAdmission'`: **0 hits**.
  - The string `memory_entry_admitted` appears only in:
    - `api.rs:L6963`: `WARD_AUDIT_EVENT_TAGS`, an audit filter allow-list;
    - `api.rs:L35081`: the test `familiar_audit_event_filter_accepts_every_ledger_event_type`, filter only;
    - `main.rs:L1307`: CLI `--event` help text;
    - three SQL fixtures under `tests/fixtures/automations-release-stores/` (`v0.4.3.sql`, `v0.4.6.sql`, `v0.4.6-rollback.sql`), as schema DDL.
  - So the signature change breaks no Coven caller, and the Coven pin bump (#1260, `8ecbf9a9`) needed no call-site change.

**Covering tests at Set B** (all exist; bodies byte-identical to A where the file changed):

- **Threads `approval.rs`:** `highest_ceremony_wins` (L737), `cross_variant_highest_merges_the_losing_paths_veto_window` (L780), `human_paths_drop_lower_windows_by_design` (L805), `proposal_classification_roundtrips_json` (L959), `window_close_reason_tags_are_stable` (L928).
- **Threads `identity_invariants.rs`:** `missing_candidate_extraction_fails_closed` (L1116), `stale_candidate_fact_extraction_fails_closed` (L1124), `compiler_requires_name_and_person` (L1055), `retired_invariant_declarations_compile_with_full_fidelity` (L1020).
- **Threads `tests/phase5_retired_ward_corpus.rs`:** `valid_cases_cover_identity_approval_veto_and_region_fidelity` (L94), `unsupported_cases_fail_closed_for_each_authority_input` (L194).
- **Threads `audit.rs` (new at B):** `memory_admission_records_the_channel_it_arrived_on` (L2424).
- **Coven `proposal_scheduler.rs`** (file unchanged): `protected_path_floor_is_not_schedulable` (L633), `reviewed_tier_cannot_use_auto_approval` (L686), `region_tier_zero_cannot_be_hidden_by_classification` (L655).
- **Coven `tests/support/threads_corpus_closure_cases.rs`** (file unchanged): `unsupported_retired_identity_declaration_never_migrates_or_stages` (L108).
- **Coven `api.rs`:** `threads_scheduled_replay_rejects_changed_live_approval_policy`, A L44109 → B L44836, identical.

**Set B finding:** every §6 mapping cited at Set A resolves to byte-identical source at Set B. The two exceptions change nothing in the mapping: the scheduler function gained diagnostic-only checkpoints, with the `due` block identical; and the Threads memory-admission constructor gained a parameter with no daemon caller. No orphaned or contradicted decision was found.

### Item 5 — evidence_replay_hash

| Set A citation | Set B status |
| --- | --- |
| Threads `approval.rs` L405-431 (`ProposalClassification.evidence_replay_hash`) | File unchanged. |
| Threads `surface_regions.rs` L192-227 (`evidence_replay_hash`) | File unchanged. |
| Coven `proposal_scheduler.rs` L82-90 (`ScheduledProposal::try_new` re-classify and re-hash), L137-161 (`from_persisted_parts`), L190-208 (`TryFrom<ScheduledProposalWire>`) | File unchanged. |
| Coven `api.rs` `revalidate_scheduled_materialized_before` L7875-7904 | A L7875-7904 → B L7980-8009, **identical**. |
| Coven `api.rs` `revalidate_auto_regression_evidence` L15546-15579 (hash passed into `regression_evidence` at L15558-15563) | A L15546-15579 → B L15696-15729, **identical**. The sub-range is at B L15708-15713. |
| Coven `api.rs` `revalidate_auto_regression` L15496-15544 (cited in item 2, feeds the same path) | A L15496-15544 → B L15646-15694, **identical**. |
| Coven `api.rs` `ensure_proposal_window_opened_audit_with_conn`, `evidence_replay_hash_hex` at L13618-13624 | Function A L13582-13662 → B L13732-13812, **identical**. The sub-range is at B L13768-13774. |
| Coven `output_format_auto.rs` `regression_evidence` L72-115 | File unchanged. |
| Gap note: identity check `api.rs` L11876-11891 (inside `decide_threads_proposal_inner`) | `decide_threads_proposal_inner` changed (A L10083-12657 → B L10193-12790). The diff deletes no lines. It adds one block, the #1278 refusal `legacy-authority-approval-unsupported` (B L11430-11452), which runs only for `decision == "approve"`, `PendingReviewKind::Authority`, no applying state and `scheduled.is_none()`. The identity block is A L11876-11891 → B L12009-12024, **byte-identical** (SHA-256 prefix `c7c46428ca99` at both). |

**Covering tests at Set B** (all exist; bodies byte-identical to A):

- **Coven `proposal_scheduler.rs`** (file unchanged): `replay_hash_mismatch_fails_closed` (L644), `deserialization_replays_region_predicates_instead_of_trusting_evidence` (L666).
- **Coven `api.rs`:**
  - `threads_scheduled_deadline_replay_refuses_diverged_before_image`: A L37730 → B L38429.
  - `threads_scheduled_deadline_rejects_deleted_empty_before_image`: A L37806 → B L38505.
  - `threads_scheduled_deadline_replay_failure_closes_window`: A L37841 → B L38540.
- **Coven `tests/threads_e2e.rs`** (file unchanged): `scheduled_window_replay_fails_closed_across_real_daemon_restart` (L661-779), the "surface-diverged" scenario.
- **Coven `tests/support/threads_output_auto_cases.rs`:** `output_auto_deadline_and_restart_reject_regression_and_valid_evidence_drift`, A L450-519 → B L462-531, identical. The file's only change is in an uncited test; see the method note above.
- **Threads `surface_regions.rs`** (file unchanged): `evidence_replay_hash_is_independent_of_evidence_order` (L925), `evidence_replay_hash_commits_field_boundaries` (L954), `evidence_replay_hash_commits_unclassified_surfaces` (L977), `execution_prompt_replay_commits_the_before_state` (L794).

**Set B finding:** every item-5 source is byte-identical at Set B. The Set A gaps carry over unchanged:

- enforcement is composite, not a literal live re-diff and re-hash;
- identity is committed separately.

Set A's note about a failing exact-SHA Windows run on `output_auto_deadline_and_restart_reject_regression_and_valid_evidence_drift` does not carry over; section 4 has the Set B CI.

### Item 7 — Label-variant mapping

| Set A citation | Set B status |
| --- | --- |
| Threads `approval.rs`: `display_label` L122-129, `from_display_label` L136-144, `ApprovalPathKind::display_label` L231-260, wire envelope `Deserialize` / `validate_label_round_trip` L572-612, `from_classification` L616-646 and round-trip L650-675 | File unchanged. |
| Threads `audit.rs` SQL: `approval_path_label` limited to `auto`, `familiar_review`, `human_review`, `human_required` (`ward_audit_require_proposal_approval_detail_insert`, L1698-1757) | `audit.rs` changed elsewhere (`cecdb5b8`). This range is A L1698-1757 → B L1706-1765, byte-identical. The `NOT IN ('auto','familiar_review','human_review','human_required')` line is B L1713. |
| Coven `ward.rs` `WardConfig::from_toml_str` L415-419 → `validate()` (calls `compiled_approval_tiers` at L450) | `from_toml_str` A L415-419 → B L415-419, **identical**. `validate` A L425-497 → B L425-497, **identical**. |
| Coven `ward.rs` `compiled_approval_tiers` L589-727: unknown-tier rejection L604-606, gate rejection L652-656, tier-name → variant map L684-696 | The function is A L589-727 → B L589-727, **identical**. The cited line ranges are byte-identical at the same line numbers: L604-606 (prefix `300fe439e8eb`), L652-656 (`a50040f7aab9`), L684-696 (`60fca98d5f57`). The rejection strings at B are L605 ``"ward config approval_tiers contains unknown tier `{tier}`"`` and L654 ``"approval_tiers.{tier_name}.gate must be `{expected_gate}`, found `{}`"``. |
| Coven `ward.rs` gate constants (used by the new test) | `APPROVAL_GATE_AUTO = "regression_suite"`, `APPROVAL_GATE_FAMILIAR_REVIEW = "familiar_coherence_check"`, `APPROVAL_GATE_HUMAN_REVIEW = "human_approval"`, `APPROVAL_GATE_HUMAN_REQUIRED = "human_approval_with_rationale"`, at L372-375 in both revisions. |
| Coven `proposal_scheduler.rs` `validate_scheduled_json_shape` L318ff | File unchanged. |
| Coven `api.rs` emit via `ApprovalPathWireEnvelope::from_classification` L7586-7591 | Inside `threads_proposals_response` (identical, A L7347 → B L7452). This range is B L7691-7696. |
| Cave `proposal-authority.ts` `APPROVAL_PATH_LABELS` L78-83; mismatch block L801-810 | At `c60bbef9`: table L78-83, **identical**. `normalizeApprovalPath` A L797-826 → B L844-873, **identical**, so the mismatch block is at B L848-857. |

**#1276 (`f772b513`) adds two tests and nothing else.** `git show --stat f772b513` reports `crates/coven-cli/src/ward.rs | 74 +` and no deletions. The parent is `21988a00` (#1278). Neither test exists at A.

- **`validation_rejects_unknown_approval_tier_at_load`** (`ward.rs:L6907-6938`). It loads, through `WardConfig::from_toml_str`, a config that has a valid `[approval_tiers.familiar_review]` table (gate, window, `min_visible_seconds = 900`) next to a well-formed `[approval_tiers.mystery_review]`. It asserts that the error contains ``approval_tiers contains unknown tier `mystery_review` ``. This covers the L605 branch.
- **`validation_rejects_approval_tier_gate_drift_at_load`** (`ward.rs:L6941-6978`). For each of the four tiers it tries the three other tiers' gate constants, 12 cases in all. Each case asserts that the error contains ``approval_tiers.{tier}.gate must be `{expected}`, found `{wrong}` ``. This covers the L654 branch. Because the source compares with `!=`, any non-matching string fails; the test samples only cross-tier gates.

**Other covering tests at Set B** (all exist; bodies byte-identical to A):

- **Threads `approval.rs`** (file unchanged): `all_variants_have_display_labels` (L691), `display_labels_round_trip_to_kind` (L707), `wire_envelope_label_round_trip_validates` (L983), `wire_envelope_rejects_unknown_label` (L1032), `wire_envelope_rejects_mismatched_label_and_variant` (L1045), `wire_envelope_deserialization_rejects_invalid_contract` (L1056), `auto_regression_requires_the_veto_key_on_the_wire` (L829).
- **Threads `audit.rs`:** `schema_names_all_approval_path_labels`, A L2001 → B L2009, identical.
- **Coven `proposal_scheduler.rs`** (file unchanged): `deserialization_rejects_unknown_fields` (L697), `deserialization_rejects_unknown_nested_policy_fields` (L713), `deserialization_rejects_fields_from_the_wrong_policy_variant` (L729).
- **Coven `ward.rs`:** `parses_retired_ward_approval_metadata` (A L6640 → B L6639), `validation_rejects_invalid_retired_ward_approval_metadata` (A L6756 → B L6755), both identical.
- **Cave `src/lib/threads-adapters.test.ts` at `c60bbef9`:** "blocks a joined proposal when daemon metadata mismatches staged authority" (L713).

**Real-daemon malformed-load journey: not found.** At `f772b513`:

- The strings `unknown tier` and `gate must be` appear only in `ward.rs`: the two source lines and the two new unit tests.
- Every `approval_tiers` occurrence under `crates/coven-cli/tests/` writes a **valid** tier/gate pair:
  - `threads_identity_invariants.rs` L49, L63, L172;
  - `threads_e2e.rs` L710, L719, L1847, L1883;
  - `support/threads_output_auto_cases.rs` L67, L419, L445;
  - `support/threads_output_auto_review_cases.rs` L153.
  
  Some of these change a valid policy (`approval-policy-changed`) or remove it (`approval-policy-removed`).
- The `ward-unavailable` scenario (`threads_e2e.rs:L687`) deletes `ward.toml`. It does not load a malformed tier or label.

The two new tests run at the config-parse layer the daemon uses at load (`WardConfig::from_toml_str`), not through a running daemon.

**Set B finding:** both Set A source gaps (no test for the unknown-tier branch, no test for the wrong-gate branch) are closed by #1276, and the production rejections are byte-identical to Set A. The one remaining gap is the optional real-daemon malformed-load journey, which is still not found.

### Item 8 — Advisory probes

| Set A citation | Set B status |
| --- | --- |
| Threads `identity_invariants.rs`: `AdvisoryProbeResult` L791-807, authoritative-flag rejection and confidence range L837-883, `AdvisoryProbes` block L885-915 | File unchanged. |
| Coven: no daemon `advisory_probes` block or model/LLM probe integration (`git grep -E 'advisory_probes\|AdvisoryProbes'` over `crates/` found nothing) | Same result at `f772b513`: `git grep -nE 'advisory_probes\|AdvisoryProbes\|AdvisoryProbeResult'` over `crates/` returns **0 hits**. Over the whole tree, the only hit is `docs/design/ward-gate3-coherence.md:L247`, a design note, and that file is unchanged since `226bfcc8`. |
| Coven `ward.rs` `enum ProbeId` (Parse, SizeDelta, ProtectedRegion, PatternLint) | A L254-259 → B L254-259, **identical**. |
| Coven `ward_probes.rs` module docs L1-6 ("a probe alone never approves a write") | File unchanged. The text is the same at L1-6. |
| Coven `output_format_auto.rs` `authoritative_projection` L117-136 (the only authoritative use, deterministic) | File unchanged. |

**New code in the delta, checked for probe authority:**

- **Coven #1152** (`a7ea8f51`, the `coven-agents` "ProposalReview seam"). Ten files mention `ProposalReview`, all under `crates/coven-agents/`. None of them references `coven_threads_core`, `ApprovalPath`, `ward::`, `WardConfig`, `threads_gate` or `ward_probes` (0 matches in each). It is not wired into Ward Gate-3 or any Threads approval path.
- **Cave `8388ce751`** ("Accept validated daemon proposal evidence extensions"). It adds `isProbeReport`, `hasValidProducerEvidence` and `isProbeSummary` to `proposal-authority.ts`, with `PROBE_IDS = {parse, size-delta, protected-region, pattern-lint}`, which matches the four daemon `ProbeId`s. Its comment reads "These diagnostics remain part of the canonical revision; none grants authority."
  - The new check is one more condition on `isCompleteScheduledEnvelope`, so it only narrows which envelopes Cave accepts.
  - `decisionAvailability` (`proposal-flow.ts` L141-227) is byte-identical to A and does not read probes.

**Covering tests at Set B** (all exist; bodies byte-identical to A):

- **Threads `identity_invariants.rs`** (file unchanged): `advisory_probe_result_is_never_authoritative` (L1432), `advisory_probe_rejects_authoritative_flag` (L1459), `advisory_probe_deserialization_rejects_authoritative_flag` (L1502), `advisory_probes_block_deserialization_rejects_ill_formed_result` (L1509), `advisory_probes_block_empty_means_no_signals_ran` (L1517).
- **Coven `ward_probes.rs`** (file unchanged): `deterministic_failures_and_probe_errors_remain_advisory_evidence` (L809), `absent_probe_config_is_explicitly_unscored` (L1054).
- **Coven `api.rs`:** `threads_coherence_approve_keeps_failed_probes_advisory`, A L35892 → B L36591.
- **Coven `tests/support/threads_output_auto_cases.rs`:** `output_auto_keeps_stronger_ceremonies_and_other_probes_advisory` (A L399 → B L411) and `output_auto_requires_applicable_json_and_all_probes_passed` (A L298 → B L310).

**Set B finding:** unchanged from Set A. The separation is enforced by type in Threads, and there is still no daemon integration of `AdvisoryProbes`, so the condition is vacuous at the daemon. The terminology caveat carries over: the daemon's deterministic `probes` field is called "advisory", but it is not the checklist's `advisory_probes` block.

---

## 3. Item 6 — min_visible (daemon and Cave)

> "**min_visible** enforced on `VetoWindow` — pending state was actually reachable before deadline could close."

### Daemon side: unchanged

| Set A citation | Set B status |
| --- | --- |
| Threads `VetoWindow::try_new` L281-291, L300-314; `earliest_close` L343-349; `is_min_visible_elapsed` L357-363; `TryFrom<VetoWindowWire>` L372-378 | `approval.rs` unchanged. |
| Coven `ward.rs` `declared_veto_window` / `veto_window_from_hours` L729-758; checks in `compiled_approval_tiers` L661-682 | `declared_veto_window` A L730-745 → B L730-745, **identical**. `veto_window_from_hours` A L747-758 → B L747-758, **identical**. `compiled_approval_tiers` A L589-727 → B L589-727, **identical**. |
| Coven `proposal_scheduler.rs` `ScheduledProposal::try_new` L92-134, L137-161 | File unchanged. |
| Coven `api.rs` `proposal_decision_semantics`: `proposal-minimum-visibility-open` when `now < earliest_close` (L7785-7787) | The function A L7725-7858 → B L7830-7963 is **identical**. The refusal `return Err("proposal-minimum-visibility-open");` moves from A L7786 to B L7891. |
| Coven `api.rs` scheduler `due` check uses `veto_deadline` (L13203-13212) | `process_due_threads_proposals` changed only by diagnostic `checkpoint` calls; see item 1, decision 2. The `due` block A L13208-13219 → B L13355-13366 is **byte-identical**. |
| Coven `api.rs` visibility: list view emits `earliestClose` and `approvalPath.veto_deadline` (L7586-7618) | `threads_proposals_response` A L7347-7652 → B L7452-7757, **identical**, so the range is B L7691-7723. The route `("GET", "/threads/proposals") => threads_proposals_response(coven_home, None, query)` moves from A L1002 to B L1073. |

**Covering tests at Set B** (all exist; bodies byte-identical to A):

- **Coven `tests/threads_e2e.rs`** (file unchanged): `retired_corpus_scheduled_intake_survives_restart_and_applies_once` (L394-495).
- **Coven `api.rs`:**
  - `threads_scheduler_tick_observes_fixture_before_min_visible_at_earliest_close_and_beyond_deadline`: A L39424 → B L40123.
  - `threads_scheduled_veto_window_delays_apply_and_records_veto`: A L37020 → B L37719. Its `proposal-minimum-visibility-open` assertion is at B L37747.
  - `post_familiar_edits_stages_scheduled_publication_from_retired_ward_intake`: A L44162 → B L44889. Its premature-approve assertion is at B L44990.
  - `threads_scheduled_applies_only_after_veto_deadline`: A L37190 → B L37889.
  - `threads_scheduler_opens_window_once_and_waits_until_deadline`: A L39074 → B L39773.
- **Coven `proposal_scheduler.rs`:** `veto_window_derives_deadline_and_minimum_visibility` (L613).
- **Coven `ward.rs`:** `validation_rejects_invalid_retired_ward_approval_metadata` (A L6756 → B L6755).
- **Threads `approval.rs`:** `veto_window_min_visible_gate` (L880), `veto_window_earliest_close_is_staged_at_plus_min_visible` (L872), `veto_window_deserialization_rejects_invalid_visibility` (L908), `veto_window_timestamp_overflow_fails_closed` (L892).

**Not assessed here:** #1132 (`store.rs`, fsync and checkpoint) and the #1278 decision-path change belong to items 2 and 4. Neither touches the functions above. The window timing is derived from `staged_at` and the `VetoWindow` in the scheduled envelope, not from store rows.

### Cave side at `c60bbef9`

Threads-facing Cave files changed `258e587f..c60bbef9`; see section 1. For item 6 I compared each function that touches `min_visible`, `earliestClose`, the veto deadline or list visibility.

| Cave function | A → B | Status | Role |
| --- | --- | --- | --- |
| `proposal-authority.ts` `isVetoWindowShape` | L404-414 → L408-418 | **identical** | Rejects a staged `VetoWindow` unless `min_visible ≤ duration` (`isDurationAtMost`). |
| `proposal-authority.ts` `hasConsistentStagedVetoMetadata` | L420-439 → L424-443 | **identical** | Requires `veto_deadline == staged_at + duration` and `earliest_close == staged_at + min_visible` to the nanosecond, and both null for a path with no veto. |
| `proposal-authority.ts` `normalizeDaemonSummary` | L828 → L875 | **changed by one line** | It adds `if ("probeSummary" in raw && !isProbeSummary(raw.probeSummary)) return null;`. The `earliestClose` parsing is untouched. |
| `proposal-authority.ts` `normalizeAvailableDecisions` | L877-895 → L925-943 | **identical** | `veto-window-open` gives `["reject"]` only. `awaiting-human-approval` gives approve and reject only for the two human variants. |
| `proposal-authority.ts` `normalizeVerifiedAuthority` | L913-964 → L961-1012 | **identical** | `!sameStagedInstant(stagedEnvelope.earliest_close, daemonSummary.earliestClose)` (B L996) makes the authority null. |
| `proposal-authority.ts` `normalizeProposalAuthority` | L975-1016 → L1023-1064 | **identical** | A null verified authority becomes `blocked("daemon-mismatch")`. |
| `proposal-flow.ts` `proposalListModel` | L22-28 → L22-28 | **identical** | Keeps every parse-ok proposal, sorted by `stagedAt`, and lists corrupt ones separately. **No time-based filter.** |
| `proposal-flow.ts` `proposalPill` | L38-94 → L38-94 | **identical** | Its comment says the pill is "derived from the daemon's own lifecycle — never from a local clock or a guess". |
| `proposal-flow.ts` `decisionAvailability` | L141-227 → L141-227 | **identical** | For `veto-window-open`, the only action is a single `reject` labelled "Veto". Approve is never offered inside a veto window. |
| `threads-adapters.ts` `joinProposalSummaries` / `proposals()` | B L211-224; `proposals()` B L668-701 | (file changed) | Lists every file in `pending/` and joins it to the paginated daemon summaries. Blocked envelopes are returned when the daemon is unavailable or unparseable, or when the pending directory changed during the read. **No clock comparison.** |
| `components/proposal-approval.tsx` | — | (file changed) | Displays `vetoDeadline` (L121-124, L151-157) and `earliestClose` (L159-160, "not reported" when null). No `Date.now`, `new Date`, timer, or comparison against `earliestClose` was found in this file or in `proposal-flow.ts`. |

**What changed in `proposal-flow.ts` (+65/−26).** The import now also takes `surfaceStateFromPayload`. `DecisionOutcome` changes from `applied`/`refused` to `confirmed`/`refused`/`unconfirmed`. A new `reconcileDecisionOutcomes` keeps uncertain outcomes until a fresh authoritative read. `decisionOutcomeFromResponse` now takes `proposalId` and reports `confirmed` only when the response has `status 200`, `blocked === false`, `meta.adapter === "daemon"`, fresh decision state, `data.ok === true`, a matching `proposalId`, and a terminal that matches the decision. The refusal copy is reworded. These changes affect what Cave says **after** a decision; they do not change when a pending proposal is listed or which actions are offered. They come from `7e1f1efff`.

**Does Cave still show the proposal for at least `min_visible`?**

- **What the source shows:** Cave neither hides nor shortens a pending proposal on time. A parse-ok staged file is listed as long as it is in `pending/` and the daemon summary read is available. No clock-based filter exists in the list path.
- **What makes the interval exist:** the daemon. It refuses `approve` before `earliest_close` and does not apply before `veto_deadline ≥ earliest_close`. So, on the daemon's terms, the file is still in `pending/` for at least `min_visible`, unless the principal vetoes early, which is allowed by design.
- **What I could not show:** that a Cave user actually sees it. Whenever the daemon summary is unavailable or unparseable, Cave renders the queue as blocked ("Decisions stay disabled while the queue is unverifiable. Anything already staged remains staged.") and does not list it. **Live browser reachability at `c60bbef9`: not found.** It is tracked separately in coven-cave#5256, as at Set A.

**Does Cave enforce anything?**

- **Not `min_visible`.** Cave has no clock gate. The `earliestClose` value is display-only.
- **What Cave does is consistency checking that blocks:**
  - `min_visible ≤ duration`;
  - `earliest_close` and `veto_deadline` derived exactly from `staged_at` and the window;
  - the daemon's `earliestClose` equal to the staged value.

  Any mismatch makes the authority `blocked("daemon-mismatch")` and disables decisions.
- **Lifecycle-derived action restriction:** inside the veto window only "Veto" (reject) is offered. These checks can only narrow what Cave forwards; they never grant a decision. The daemon re-decides every request (`proposal-minimum-visibility-open`, unchanged).

**Cave tests at `c60bbef9` touching this item** (names from source; not run):

- **`src/lib/threads-read.test.ts`:**
  - "requires veto deadlines to equal staged_at plus duration and min_visible exactly" (L1730)
  - "binds daemon earliestClose to the staged derived instant, including null presence" (L1897)
  - "requires root deadline presence to match whether the staged approval path has a veto" (L1656)
  - "maps veto-bearing auto and familiar paths to reject only" (L1933)
  - "maps veto-window-open proposals to reject only" (L2016)
  
  This file changed only by +3 lines, each adding `autoRegressionEvidence: Array(32).fill(7),` to a fixture.
- **`src/lib/proposal-flow.test.ts`:** "renders verified reject-only veto authority as one Veto action" (L151).
- **`src/lib/proposal-authority-daemon.test.ts`** (new since A): "real daemon evidence retains the confirmed veto-only authority path" (L11), "auto-regression requires its commitment with and without a veto window" (L76).

**Set B finding (item 6):**

- **Daemon:** enforcement and covering tests are byte-identical to Set A.
- **Cave:** at `c60bbef9` nothing hides a pending proposal early, and the veto-window UI offers only Veto. Cave enforces no `min_visible` itself; it only blocks on inconsistent window metadata.
- **Browser reachability:** not found.

---

## 4. CI and root-dossier bookkeeping

Sources: read-only `gh run list --commit`, `gh run view --json jobs`, `gh run view --job --log` (grep only) and `gh api repos/…/issues/N`, read on 2026-10-13. Only runs whose `headSha` is exactly a Set B SHA are listed as Set B evidence. Set A runs are in the last subsection and are not combined with these.

### 4.1 Set B exact-SHA runs

| Repo / SHA | Run | Workflow / event | Conclusion |
| --- | --- | --- | --- |
| coven `f772b513` | [38209489872](https://github.com/OpenCoven/coven/actions/runs/38209489872) | CI / `merge_group` (branch `gh-readonly-queue/main/pr-1276-21988a00…`) | **success** |
| coven `f772b513` | [38211237006](https://github.com/OpenCoven/coven/actions/runs/38211237006) | CI / `push` (main) | **cancelled** (see below) |
| coven `f772b513` | 38287686717, 38427066134 (Engine bump); 38292423048 (Branch Triage); 38476339653, 38476339913, 38476339914, 38476357099, 38476464448, 38476499016 (Dependabot Updates) | `schedule` / `dynamic` | success. These are maintenance workflows that run no tests, listed for completeness. |
| coven-threads `78e47c9d` | [35110172749](https://github.com/OpenCoven/coven-threads/actions/runs/35110172749) | CI / `push` (main) | **success**, 6 of 6 jobs: Nextest telemetry (JUnit, fail on flaky); Coverage baseline (informational); Secret scanning; Privacy policy guard; Cargo test (compatibility baseline); Rust quality and repository contract |
| coven-threads `78e47c9d` | [35110172753](https://github.com/OpenCoven/coven-threads/actions/runs/35110172753) | Daemon compatibility / `push` | **success** (job "Pinned daemon compatibility"). **Caveat:** its log shows `COVEN_REF: 226bfcc89ff6cad4bc9cc9618dad6fea970ecf58`. It checked Threads `78e47c9d` against the **Set A** daemon, not `f772b513`. |
| coven-cave `c60bbef9` | [38207714524](https://github.com/OpenCoven/coven-cave/actions/runs/38207714524) | CI / `push` (main) | **success**: 22 success, 1 skipped (`iOS build`), 23 jobs |
| coven-cave `c60bbef9` | 38207713953 | CodeQL / `dynamic` | success |
| coven-cave `c60bbef9` | 38208980939 | Main health / `workflow_run` | success |
| coven-cave `c60bbef9` | 38209794146, 38210733240 | Main health / `workflow_run` | skipped |

**Coven 38209489872 (`merge_group`, success), job detail:**

- **Rust tests (Linux)**, job 114680814637: success.
  - `cargo test --workspace --locked` passed. Inside it, the `coven` bin unit target reported `3880 passed; 0 failed; 4 ignored`, and the log shows `ward::tests::validation_rejects_approval_tier_gate_drift_at_load ... ok` and `ward::tests::validation_rejects_unknown_approval_tier_at_load ... ok`. These are the two #1276 tests.
  - Then the clock-feature steps ran: bin `threads_` filter, 228 passed (3675 filtered out); `threads_e2e` 105 passed; `threads_identity_invariants` 45; `threads_protected_intake` 39; `threads_terminal_recovery` 34. All had 0 failed. The target names come from the `Running tests/threads_*.rs` lines.
- **Rust tests (Windows)**, job 114680814643: success. This job runs `cargo test --workspace --locked` only. Since #1116 it does not run the four Threads targets (`ci.yml` L305-310).
- **Threads daemon tests (Windows)**, job 114680814673: success. The bin `threads_` filter passed 218 (3452 filtered out); `threads_e2e` 82; `threads_identity_invariants` 44; `threads_protected_intake` 35; `threads_terminal_recovery` 33. All had 0 failed; the target names come from the `Running tests\threads_*.rs` lines.
- **Rust lint (Linux), Policy guard, Dependency audit, the two Automations bundles, npm onboarding smoke (windows and linux-x64), and PR gate** (114684907659): success.
- **Skipped:** Rust tests (macOS), CLI performance baseline, AFS mount backend (Linux, macOS), engine contract, Channels package, Restricted runtime (macOS), OpenClaw bridge.
  - `rust-test-macos` has `if: github.event_name == 'push' && …` (`ci.yml` L371), so a `merge_group` run always skips it.

**Coven 38211237006 (`push`, cancelled), job detail:**

- **Rust tests (Linux)**, job 114686063664: success. Bin `threads_` 228; then 105 / 45 / 39 / 34, all 0 failed. These match the merge-group counts, in the command's target order.
- **Threads daemon tests (Windows)**, job 114686063761: success. Bin 218; then 82 / 44 / 35 / 33, all 0 failed.
- **Rust tests (Windows)** (114686063651), Policy guard, Rust lint, engine contract, CLI performance baseline, AFS mount backend (macOS), Dependency audit, both Automations bundles, and npm onboarding smoke (windows, linux-x64, macos-x64, macos): success.
- **Rust tests (macOS)**, job 114686063678: **cancelled**. The job has `timeout-minutes: 20` (`ci.yml` L373). It started at 06:14:02Z, and the log shows `##[error]The operation was canceled.` at 06:34:14Z.
  - That fell inside the "Exercise real-daemon Threads journeys" step (started 06:31:13Z).
  - The earlier `cargo test --workspace` step passed, and the clock-feature bin `threads_` step passed 228 (3674 filtered out).
  - No `Running tests/threads_*.rs` line and no result line for the four suites appear before the cancel. **No macOS result for the four suites exists at this SHA.**
- **PR gate**, job 114691743071: **failure**, with `PR gate rejected job results: rust-test-macos=cancelled`.
- The concurrency group is `ci-${{ github.ref }}`, with `cancel-in-progress` only for `pull_request` (`ci.yml` L12-14). The cancel therefore matches the job's own 20-minute timeout, not a superseding push. This is inferred from the timings and config; no log line names the timeout.

**Where the four Threads suites run with `--features threads-test-clock` at `f772b513`** (`.github/workflows/ci.yml`):

| Job | Lines | When it runs | Set B exact-SHA result |
| --- | --- | --- | --- |
| `rust-test-linux` "Rust tests (Linux)" | L289-292 | Rust changes | success in 38209489872 and in 38211237006 |
| `threads-test-windows` "Threads daemon tests (Windows)" | L347-352 | Rust changes | success in 38209489872 and in 38211237006 |
| `rust-test-macos` "Rust tests (macOS)" | L392-395 | `push` only | skipped in 38209489872; **cancelled before any suite reported** in 38211237006 |

`.github/workflows/release-npm.yml` L52 runs the same command at release time, outside these runs.

**Cave 38207714524:** the Threads-facing Cave tests ran in "Frontend validation (app tests)" (job 114674859018). The log shows the following tests passing:

- "requires veto deadlines to equal staged_at plus duration and min_visible exactly"
- "real daemon evidence retains the confirmed veto-only authority path"
- "blocks a joined proposal when daemon metadata mismatches staged authority"
- "renders verified reject-only veto authority as one Veto action"

I did not tally the job's per-file test totals; the conclusion is success.

### 4.2 Root issues coven#885-#888

All four are CLOSED. REST returned these `closed_at` values on 2026-10-13:

- **#885**, "P0 threads-okc: Wire identity predicates into mutation intake and deadline replay", closed 2026-09-13T22:30:00Z.
- **#886**, "P0 threads-980: Require one typed terminal close for every opened veto window", closed 2026-09-17T19:00:14Z.
- **#887**, "P0 threads-dgg: Remove protected SOUL.md from the proposal/approval pipeline", closed 2026-09-13T22:30:00Z.
- **#888**, "P0 threads-zav: Prove retired-Ward proposals are schedulable and recoverable end to end", closed 2026-09-13T22:30:00Z.

**Set A dossiers.** These were not re-derived; they are carried from the Set A packet's "Root-issue evidence pointers":

- **#885:** [acceptance comment](https://github.com/OpenCoven/coven/issues/885#issuecomment-5656608531); [identity closure packet](https://github.com/OpenCoven/coven/issues/885#issuecomment-5654726468).
- **#886:** [disposition](https://github.com/OpenCoven/coven/issues/886#issuecomment-5719691181); [deployed census comment](https://github.com/OpenCoven/coven/issues/886#issuecomment-5676712866); `docs/reviews/2026-09-15-deployed-audit-census.md`; `docs/reviews/2026-09-17-maintainer-decisions.md` Decision 1 (scoped to the workstation profile).
- **#887:** [acceptance](https://github.com/OpenCoven/coven/issues/887#issuecomment-5656608527); [route evidence](https://github.com/OpenCoven/coven/issues/887#issuecomment-5654515217).
- **#888:** [acceptance](https://github.com/OpenCoven/coven/issues/888#issuecomment-5656608528); [corpus packet](https://github.com/OpenCoven/coven/issues/888#issuecomment-5654515214).
- **Cross-root:** [landing evidence matrix (coven-threads#31)](https://github.com/OpenCoven/coven-threads/issues/31#issuecomment-5656531969); `docs/reviews/2026-09-13-engineering-acceptance.md`.

**Set B CI runs that exercise the same suites.** These are separate evidence and are not merged with the Set A dossiers. The suite-to-root pairing below is **inferred from issue titles**; it is not taken from the dossiers.

| Root | Suite most directly exercising it (inferred) | Set B exact-SHA runs (`f772b513`) |
| --- | --- | --- |
| #885 identity | `threads_identity_invariants` | 38209489872: Linux 45 passed, Windows 44 passed. 38211237006: Linux 45, Windows 44. macOS: no result. |
| #886 typed terminal close | `threads_terminal_recovery` (and `assert_window_terminal` in `threads_e2e`) | 38209489872: Linux 34, Windows 33. 38211237006: Linux 34, Windows 33. macOS: no result. |
| #887 protected SOUL.md | `threads_protected_intake` | 38209489872: Linux 39, Windows 35. 38211237006: Linux 39, Windows 35. macOS: no result. |
| #888 retired-Ward end to end | `threads_e2e` | 38209489872: Linux 105, Windows 82. 38211237006: Linux 105, Windows 82. macOS: no result. |

All of these Set B results report 0 failed.

The per-platform counts differ: Linux 105/45/39/34 against Windows 82/44/35/33. I did not determine why, and these counts are not compared with Set A's.

### 4.3 Set A CI, for separation only

These runs are recorded in the Set A packet and are **not** Set B evidence:

- coven `226bfcc8` push run 34785628407: **failure** (Windows Threads `output_auto_deadline_and_restart_reject_regression_and_valid_evidence_drift` timeout; Windows Rust failure; CLI perf baseline).
- coven-threads `0021fd26`: 34720725886 (CI, success) and 34720762231 (Daemon observation, success).
- coven-cave `258e587f`: 34767545639 (CI, success).

The Set A Windows failure does not recur in either Set B Coven run: the "Threads daemon tests (Windows)" job passed in both. The two results are kept apart. Set B passing does not retroactively change the Set A result.

---

## 5. Proposed verdicts

These are proposals for Val's self-review, not acceptances. Each verdict rests on sources and test names at Set B. No tests were run. The CI runs on these exact SHAs are in section 4 and are kept separate from Set A's.

| # | Item | Set A | Proposed at Set B | Basis | Main limiter |
| --- | --- | --- | --- | --- | --- |
| 1 | RFC round-trip | supported | **supported** | The §6 decision text (8 decisions, 67 lines) is byte-identical. Every Set A mapping site is byte-identical at Set B, except two changes that do not touch the mapping: `process_due_threads_proposals` gained diagnostic `checkpoint` calls (its `due` block is identical), and `for_memory_entry_admitted` gained a `channel` parameter (threads-55s), which Coven never calls. | Decision 3 promotion is not implemented, so it is vacuous. Decision 5 is external and was not inspected. Both are unchanged from Set A. |
| 5 | evidence_replay_hash | supported | **supported** | All cited sources are byte-identical: the Threads files, `proposal_scheduler.rs`, `output_format_auto.rs`, and four `api.rs` functions. The identity block inside the changed `decide_threads_proposal_inner` is byte-identical. The covering tests exist with identical bodies. | Enforcement is composite, not a literal live re-hash. Identity is committed separately. |
| 6 | min_visible | supported (daemon) | **supported (daemon scope)** | Daemon enforcement (`proposal_decision_semantics`, the `due` block, `ward.rs` window checks, the `VetoWindow` invariants) and its tests are byte-identical. At Cave `c60bbef9`, nothing hides a pending proposal on a clock, and only Veto is offered in the window. Cave enforces no `min_visible` itself; it blocks on inconsistent window metadata. | Browser/Cave reachability: not found (coven-cave#5256). If the daemon summary is unavailable, Cave blocks the whole queue. |
| 7 | Label-variant mapping | partial | **supported** | #1276 adds `validation_rejects_unknown_approval_tier_at_load` and `validation_rejects_approval_tier_gate_drift_at_load`, which cover the two branches Set A found untested (`ward.rs` L605 and L654). The production rejections and `compiled_approval_tiers` are byte-identical. | The real-daemon malformed-load journey is still not found; the scope treats it as optional. The new tests run at the `WardConfig::from_toml_str` layer, not through a running daemon. |
| 8 | Advisory probes | supported | **supported** | The Threads types are unchanged. `ward_probes.rs`, `output_format_auto.rs` and `ProbeId` are unchanged. There is still no daemon `AdvisoryProbes` integration. The new #1152 `ProposalReview` seam and Cave probe-report validation grant no authority. | Vacuous at the daemon; the terminology overlap remains. |

No item is proposed as excluded-by-prior-decision.

**Set B CI context (section 4).** In both Coven runs on `f772b513` (merge-group 38209489872 success; push 38211237006 cancelled), Linux and Windows ran the four real-daemon Threads suites with 0 failures, and Linux ran the two #1276 tests, which passed. Threads `78e47c9d` CI passed, and Cave `c60bbef9` CI passed, including the cited Cave tests. Two limits apply:

- **macOS:** no Set B result exists for the four suites. The merge-group run skips macOS by design, and the push run's macOS job was cancelled at its 20-minute limit before any suite reported.
- **Threads daemon compatibility:** the run on `78e47c9d` used the Set A daemon `226bfcc8`.

Neither limit is a failure of a covering test. Neither changes the proposed verdicts, but the macOS gap should be named in review.

---

## 6. Every "not found"

**Source and tests:**

1. **Region-to-thread promotion code** (item 1, decision 3): not found at Threads `78e47c9d` or Coven `f772b513`. The forward-only rule stays vacuous.
2. **familiar-contract PRs #3 and #4** (item 1, decision 5): not found. familiar-contract was not inspected.
3. **A Coven producer of `memory_entry_admitted`**, or any caller of `WardAuditRecord::for_memory_entry_admitted` (item 1, threads-55s): not found.
   - `git grep for_memory_entry_admitted` and `git grep -E 'MemoryEntryAdmitted|MemoryEntryAdmission'` at `f772b513` both return 0 hits.
   - The tag appears only in the filter allow-list, one filter test, CLI help text and SQL fixtures.
4. **A real-daemon journey that loads a malformed approval tier or label and refuses** (item 7): not found. Every `approval_tiers` fixture under `crates/coven-cli/tests/` is valid. `ward-unavailable` deletes `ward.toml`; it is not a malformed load.
5. **Live browser or Cave reachability of a pending proposal during `min_visible`, at Cave `c60bbef9`** (item 6): not found. It is tracked in coven-cave#5256.
6. **A Cave clock comparison against `earliestClose`** (item 6): not found. No `Date.now`, `new Date`, timer or comparison exists in `proposal-flow.ts` or `proposal-approval.tsx`. `earliestClose` is only displayed or compared with the staged value.
7. **A daemon `advisory_probes` / `AdvisoryProbes` integration** (item 8): not found. There are 0 hits under `crates/`; the only hit in the tree is the design doc `docs/design/ward-gate3-coherence.md:L247`, which is unchanged.

**Bookkeeping:**

8. **The file set behind the scope document's Cave count, "6 changed, +290/−42"**: not found. My Threads-facing set gives 7 non-test files, +386/−59 (13 files, +746/−116 with tests). See section 1.

**CI:**

9. **A macOS result for the four real-daemon Threads suites at `f772b513`**: not found.
   - In the merge-group run 38209489872, `Rust tests (macOS)` was skipped (`if: github.event_name == 'push'`).
   - In the push run 38211237006, it was cancelled at its 20-minute limit before any `Running tests/threads_*.rs` line or result appeared.
10. **A log line naming the timeout as the cause of the macOS cancel**: not found. The attribution to `timeout-minutes: 20` is inferred from the 06:14:02Z → 06:34:14Z timing, the job config, and the non-cancelling concurrency policy for `push`.
11. **A Threads "Daemon compatibility" run of `78e47c9d` against daemon `f772b513`**: not found among the exact-SHA runs on `78e47c9d`. Run 35110172753 used `COVEN_REF: 226bfcc8`. Runs on other Threads commits were not searched.
12. **The reason the Linux and Windows suite counts differ** (105/45/39/34 against 82/44/35/33): not determined. Platform-gated tests are a plausible cause, but I did not verify it.
13. **Per-file Cave test totals in job 114674859018**: not tallied. Only the four cited titles were confirmed passing.

---
