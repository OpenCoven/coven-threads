# PHASE-4 — Cave Surfaces: the data contract between daemon memory state and the Coven Cave UI

**Status:** DRAFT (bead `threads-986.17.1` — written first, before any UI code; blocks `.17.2`–`.17.9`)
**Date:** 2026-07-15
**Upstream:** RFC-0001 §5 (Ward: gates, tiers, audit); `PHASE-0-DESIGN.md` (FROZEN v0.2) §2 metaphor bindings, §2.5 binding rule, §3.4 single audit store, §5 verdicts
**Substrate:** `coven-threads-core` v0.1.2 (98 tests green) + coven daemon integration branch `feat/threads-gate-validator` (PR OpenCoven/coven#382, merge gated on `threads-986.19`)
**Shape reference:** OpenTrust `docs/MEMORY-API-CONTRACT.md` / `docs/MEMORY-LAYER-STANDARD.md` — mirrored in *shape* (contract-first, evidence-over-summaries, freshness surfaced everywhere), **never in stack** (no Tauri, no Convex, no plugin architecture, no vendored code)

**Phase 5 amendment (2026-07-20, bead `threads-uqx.7`):** scheduled
proposal lifecycle metadata comes from the daemon read model added by
OpenCoven/coven#430. Cave may combine that metadata with staged-file contents
for inspection, but it never infers lifecycle transitions from clocks or local
files.

---

## 0. Ground rules (normative, inherited — violating any of these is wrong by declaration)

1. **Predicate authoritative, descriptor derived.** The UI MUST NOT present a
   `PatternDescriptor` (or any `describe()`-derived summary) as if it were
   enforcement. Descriptor content is always labeled derived. Every displayed
   *status* traces to a predicate result delivered by the read adapter.
2. **The UI never mutates protected memory.** All writes flow through the
   daemon. The Cave reads state and approves/rejects **staged** proposals only,
   by forwarding the decision to the daemon. No route in this contract writes
   to a protected surface, to `~/.coven/pending/`, or to `coven.sqlite3`.
3. **Gate 4 fail-closed is a rendering rule here.** Unknown, unverifiable, or
   daemon-absent state renders as **blocked / fail-closed** — never
   healthy-by-default. §4 enumerates every such state. A UI that shows a green
   pill it cannot trace to evidence is non-conformant.
4. **Metaphor-referent binding (§2.5) applies one layer up.** *Thread* =
   authority relationship surface → writer; *Weave* = enforced pattern of
   threads; *Strand* = fiber inside a thread; *Channel* = axis of load. UI copy
   uses these words only with their referents attached (§5 glossary). Nova
   reviews for code-to-UI coherence (blocking, `threads-986.17.8`).
5. **Implementation home is `coven-cave`, following `coven-cave` conventions**:
   every route registered in `src/app/api/api-contracts.test.ts`; every test
   file registered in `scripts/run-tests.mjs` `SUITES`; `pnpm typecheck`,
   `pnpm test:app`, `pnpm test:api` green before ready-for-review.

## 1. Sources of truth the Cave reads

| Source | Owner | What it holds | Read by |
|---|---|---|---|
| Weave state (via daemon; fixture mirror until `.19` merges) | daemon (`threads_gate.rs`) | `Weave` / `Thread` / `Strand` / `TensionState` per familiar | weave rail, thread pane, strand inspection |
| `ward_audit` table in `coven.sqlite3` | daemon (append-only; UPDATE/DELETE abort triggers) | one row per audit event (RFC-0001 §5.6 field set + coven-threads extensions) | audit lineage view, thread pane history |
| `~/.coven/pending/*.json` | daemon (atomic sibling-rename writes) | `PendingProposal` staged by `DegradeToProposal` | proposal approval flow |
| Daemon socket `~/.coven/coven.sock` | daemon | live API incl. additive `threadsGate` payload; approve/reject forwarding target | adapter `daemon-present`; POST forwarders |

**Two adapters, one interface (`.17.2`):**

- `daemon-absent` — reads fixtures from `coven-cave/fixtures/phase-4/`.
  **Default** until `threads-986.19` merges PR #382. In this mode every
  response carries `meta.adapter = "fixtures"` and the approval POST routes
  refuse (§3.7) — there is no daemon to forward to, so the action fails
  closed.
- `daemon-present` — reads the coven socket and the sqlite audit table.
  Same read-model shapes, `meta.adapter = "daemon"`.

Fixture states (minimum set, all ten): `weave-holds`, `weave-frayed`,
`weave-snapped`, `weave-unknown`, `daemon-timeout`, `audit-empty`,
`audit-with-lineage`, `pending-empty`, `pending-with-proposals`,
`pending-corrupt`.

## 2. Read models

Read models are the UI-facing JSON the Cave API returns. Adapters normalize
raw source encodings (serde externally-tagged enums, sqlite rows, staged JSON)
into these shapes; the raw form stays available under `trace` so every status
pill can open its evidence (OpenTrust trace-detail shape).

Conventions: camelCase keys; timestamps RFC 3339 strings; hashes/binary
lowercase hex strings; ids are strings (UUID or human-readable, as the source
defines them).

### 2.1 `TensionView`

The crate's `TensionState` has exactly three states. The *view* adds the two
UI-only fail-closed states — `unknown` arises when the adapter cannot fetch or
verify, `stale` when `staleAfter` has passed (§3.9). Neither exists in the
crate and neither may be persisted back toward it.

```jsonc
// state: "holds" | "frayed" | "snapped" | "unknown" | "stale"
{ "state": "holds" }

{
  "state": "frayed",
  "strand": "9f0c…-uuid | null",      // null = a required strand is missing
  "channel": "deliberate|forced|serialization|mutation",
  "reason": {                          // FrayReason, normalized
    "kind": "content-hash-mismatch|signature-invalid|manifest-entry-mismatch|audit-trail-unverifiable|required-strand-missing|serialization-marker-mismatch|other",
    "missingKind": "ContentHash|Signature|ManifestEntry|AuditTrail|SerializationMarker", // only for required-strand-missing
    "detail": "string"                 // only for other
  },
  "detectedAt": "2026-07-15T09:00:00Z"
}

{
  "state": "snapped",
  "channel": "…",
  "reason": { "kind": "revoked|multiple-strand-fray|pattern-broken|other", "detail": "…" },
  "at": "2026-07-15T09:00:00Z"
}

{ "state": "unknown", "why": "daemon-unreachable|unparseable|no-fixture|meta-missing" }
{ "state": "stale",   "lastKnown": { "state": "holds" }, "observedAt": "…" }
```

### 2.2 `WeaveSummary` and `WeaveDetail`

```jsonc
// WeaveSummary — the rail row
{
  "id": "weave-uuid",
  "familiarId": "sage",                // human-readable id, as ward_audit stores it
  "threadCount": 4,
  "tensionRollup": { "state": "frayed", "…": "worst contained thread state; §4.R1" },
  "coherence": "coherent|degraded|broken|unknown",   // predicate result — never the descriptor
  "degradedSurfaces": ["MEMORY.md"],   // present iff degraded
  "weaveHash": "hex"
}

// WeaveDetail — adds:
{
  "…": "all WeaveSummary fields",
  "threads": [ /* ThreadView, §2.3 */ ],
  "patternDescriptor": {               // derived — render only with the derived label (§4.R2)
    "derived": true,
    "summary": "…verbatim from describe()…"
  },
  "covenRef": "coven-uuid | null"
}
```

`tensionRollup` severity order (worst wins): `snapped` > `frayed` > `unknown`
> `stale` > `holds`. An unknown thread can never be out-ranked by a healthy
one — unknown is *worse* than holds by construction (fail-closed).

### 2.3 `ThreadView`

```jsonc
{
  "id": "thread-uuid",
  "surface": "SOUL.md",                // SurfaceId — the protected surface
  "writer": "familiar:sage",           // WriterId — who may propose writes
  "tension": { /* TensionView */ },
  "holdsUnder": ["mutation", "forced", "serialization"],   // covered channels
  "requiredStrands": {                 // per covered channel: Channel::required_strand_kinds
    "mutation": ["ContentHash"],
    "forced": ["ContentHash", "ManifestEntry"],
    "serialization": ["SerializationMarker"]
  },
  "strandCount": 3,
  "createdAt": "…"
}
```

### 2.4 `StrandView`

One shape per kind; `kind` discriminates. Committed material renders as hex.

```jsonc
{ "id": "…", "kind": "ContentHash", "algorithm": "blake3|sha256", "value": "hex" }
{ "id": "…", "kind": "Signature", "keyId": "…", "sigKind": "ed25519|principal-attestation", "value": "hex" }
{ "id": "…", "kind": "ManifestEntry", "manifestId": "…", "entryHash": "hex" }
{ "id": "…", "kind": "AuditTrail", "firstSeen": "…", "eventLogRef": "…" }
{ "id": "…", "kind": "SerializationMarker", "formatVersion": "0.1.0", "contractHash": "hex" }
```

On a Frayed thread, the strand the fray blames additionally carries the
current-vs-expected diff (`.17.5`):

```jsonc
{
  "…": "StrandView fields",
  "fray": {
    "expected": "hex | string",        // committed value from the strand
    "observed": "hex | string | null", // what the verifier saw; null = could not observe (renders blocked, not healthy)
    "observedAt": "…"
  }
}
```

### 2.5 `AuditEntryView` — one `ward_audit` row

Mirrors the append-only table (`WARD_AUDIT_SCHEMA_SQL`; RFC-0001 §5.6 set +
`validation_verdict` + `compaction_ledger`).

```jsonc
{
  "id": 412,                            // rowid — also the lineage cursor
  "eventType": "proposal_submitted|proposal_approved|proposal_rejected|proposal_vetoed|ward_updated|validation_verdict|compaction_ledger",
  "proposalId": "uuid | null",
  "familiarId": "sage",
  "wardVersion": "string | null",
  "wardHash": "hex",                    // weave_hash at decision time — binds decision to authority state
  "tier": "string | null",
  "decision": "permit | degrade_to_proposal | reject:<reason-tag> | …",
  "approver": "writer-id | null",
  "diffHash": "hex | null",
  "filesTouched": ["SOUL.md"],
  "channel": "mutation | null",
  "threadId": "uuid | null",
  "submittedAt": "…", "decidedAt": "…", "recordedAt": "…"
}
```

Lineage rule: an entry whose `proposalId`/`threadId` cannot be resolved to a
live object renders with an explicit unresolved-reference marker — it is
never dropped from the list silently (§4.R7).

### 2.6 `ProposalView` — one `~/.coven/pending/*.json` file

```jsonc
{
  "file": "<familiar-uuid>-<proposal-uuid>.json",
  "parse": "ok | corrupt",              // corrupt ⇒ payload null, §4.R6 applies
  "payload": {                          // PendingProposal, normalized
    "id": "proposal-uuid",
    "familiarId": "familiar-uuid",
    "writer": "familiar:sage",
    "channel": "mutation",
    "threadId": "thread-uuid",
    "fray": { /* FrayOrSnap verbatim for the principal, normalized like TensionView.reason */ },
    "edits": [
      { "surface": "MEMORY.md", "contents": { "encoding": "utf8|base64", "data": "…full desired contents, never diffs…" } }
    ],
    "stagedAt": "…"
  },
  "authority": { /* ProposalAuthorityView, below */ }
}
```

Phase 5 adds daemon-owned scheduling metadata without making Cave an approval
engine. The daemon adapter joins each parse-ok staged file to
`GET /api/v1/threads/proposals` by proposal id. Staged contents remain visible
for principal inspection; every lifecycle field and display label comes from
the daemon response. `proposalRevision` commits the complete authority envelope,
including staged contents. `familiarUuid` is compared directly with the staged
payload; the daemon's human `familiarId` remains display data and is not used as
the identity join key.

Revision algorithm (normative): remove top-level procedural
`decisionRequest`/`decisionState`; recursively sort every JSON object by Unicode
code-point key order while preserving array order; serialize compact UTF-8 JSON;
then lowercase-hex SHA-256 the bytes. Cave computes this revision from the same
parsed envelope used to render edits and requires equality with the daemon's
`proposalRevision`. A mismatch blocks the card before any action is offered.

```jsonc
// Scheduled proposal whose daemon metadata is verified.
{
  "state": "verified",
  "proposalRevision": "64-char lowercase hex", // daemon commitment to the complete authority envelope
  "familiarUuid": "familiar-uuid",             // canonical join key for the staged payload
  "approvalPath": {
    "variant": "auto-regression|familiar-coherence|human-approval|human-approval-with-rationale",
    "label": "auto|familiar_review|human_review|human_required", // daemon text, rendered verbatim
    "vetoDeadline": "RFC3339 | null"
  },
  "lifecycle": "awaiting-human-approval|veto-window-open|ready-for-replay|blocked",
  "blockedReason": "string | null",
  "earliestClose": "RFC3339 | null",
  "affectedRegions": ["tool_defaults"],
  "availableDecisions": ["approve", "reject"] // deterministic mapping below
}

// Pre-Phase-5 proposal. Compatibility only; no scheduled lifecycle is invented.
{ "state": "legacy", "reviewKind": "authority" }

// Scheduled envelope exists, but daemon authority cannot be joined or verified.
{
  "state": "blocked",
  "why": "daemon-unavailable|daemon-proposal-missing|daemon-unparseable|daemon-mismatch|unknown-lifecycle"
}
```

`availableDecisions` is a closed presentation mapping over verified daemon
fields, not local authorization:

The daemon wire uses snake_case enum values and nested field names. The adapter
performs only this closed normalization before the mapping:

| Daemon wire | Cave view |
|---|---|
| `auto_regression` | `auto-regression` |
| `familiar_coherence` | `familiar-coherence` |
| `human_approval` | `human-approval` |
| `human_approval_with_rationale` | `human-approval-with-rationale` |
| `awaiting_human_approval` | `awaiting-human-approval` |
| `veto_window_open` | `veto-window-open` |
| `ready_for_replay` | `ready-for-replay` |
| `blocked` | `blocked` |
| `approvalPath.veto_deadline` | `approvalPath.vetoDeadline` |

Any value outside this table is `unknown-lifecycle`; no generic case conversion
or label synthesis is permitted. `approvalPath.label` is never normalized.

| Daemon lifecycle and path | Cave action |
|---|---|
| `awaiting-human-approval` + `human-approval` | approve or reject |
| `awaiting-human-approval` + `human-approval-with-rationale` | approve with a non-empty note, or reject |
| `veto-window-open` + a path carrying a veto deadline | reject, rendered as **Veto** |
| `ready-for-replay`, `blocked`, unknown combination, or blocked authority | no action |

Cave MUST NOT compare its clock to `vetoDeadline` or `earliestClose` to change
the action set. Deadline expiry triggers daemon replay; only a fresh daemon
response may move lifecycle. Every POST remains a forwarder and the daemon
re-validates the decision. Approve/reject bodies carry
`expectedRevision: authority.proposalRevision`; the daemon rejects a changed or
missing revision before accepting a manual Phase 5 decision. This binds the
principal's action to the exact proposal contents inspected, not merely its id
and target list.

### 2.7 `DegradedFamiliarView` — a familiar whose ward config cannot be parsed

Amendment 2026-07-18 (bead `threads-k9s`, follow-up to `threads-vmf`/`threads-v60`).
A familiar with a ward.toml the daemon cannot deserialize must not vanish from
the weave rail (fail-visible) and must not abort the fleet read (coven#418).
The daemon includes one degraded entry per such familiar in the §3 route-1
response, alongside the healthy `{weave, coherence}` entries:

```jsonc
{
  "degraded": {
    "familiarId": "nova",                       // human id, same as weave.familiar_id
    "reason": "ward-config-unparseable",        // closed enum; only this value today
    "error": "missing field `principal_key_fingerprint`"  // single-line, sanitized parse error; no paths beyond the familiar home-relative ward.toml
  }
}
```

Compatibility: the entry carries no `weave` key, so pre-amendment Cave
normalizers drop it silently (older UI degrades to the previous skip
behavior); post-amendment normalizers render it per §4.R12. The daemon keeps
logging the full error to its recovery log.

## 3. API routes

All routes live in `coven-cave` `src/app/api/`, are registered in
`api-contracts.test.ts`, and return the envelope of §3.8. GET routes are
`kind: "json"`. The two POST routes are thin daemon-forwarders — they carry
the principal's decision to the daemon and return the daemon's outcome; they
never apply edits, never touch `pending/`, never write sqlite.

| # | Route | Method | Returns |
|---|---|---|---|
| 1 | `/api/weaves` | GET | `WeaveSummary[]` — optional `?familiar=<id>` filter; degraded familiars surface per §2.7 + §4.R12 |
| 2 | `/api/weaves/[id]` | GET | `WeaveDetail` |
| 3 | `/api/threads/[id]` | GET | `ThreadView` |
| 4 | `/api/threads/[id]/strands` | GET | `StrandView[]` (with `fray` diff blocks where blamed) |
| 5 | `/api/threads/[id]/audit` | GET | `AuditEntryView[]` for this thread, newest first; `?before=<rowid>` pagination |
| 6 | `/api/proposals` | GET | `ProposalView[]` (both `ok` and `corrupt` entries) |
| 7 | `/api/proposals/[id]/approve` | POST | daemon outcome verbatim + envelope (§3.7) |
| 8 | `/api/proposals/[id]/reject` | POST | daemon outcome verbatim + envelope (§3.7) |

### 3.7 Approve/reject semantics (fail-closed)

- Body:
  `{ "note": "optional principal note", "expectedRevision": "daemon-issued proposal revision" }`.
  `expectedRevision` is mandatory for manual Phase 5 decisions. `readsJson: true`,
  `invalidJson: "guarded"`.
- `daemon-present`: forward to the daemon socket; the daemon re-validates
  (staging is data, not authority — replay goes back through `validate`,
  RFC-0001 §5.4), applies or refuses, appends `proposal_approved` /
  `proposal_rejected` to `ward_audit`, and removes the pending file. The
  route returns the daemon's outcome verbatim.
- `daemon-absent` (fixtures mode) or daemon unreachable/timeout: **HTTP 503**,
  `{ "blocked": true, "why": "daemon-unavailable" }`. The UI renders the
  §4.R5 blocked treatment. No optimistic UI, no queued decisions.
- Proposal `parse: corrupt`: both actions disabled in the UI *and* the routes
  answer **HTTP 409** `{ "blocked": true, "why": "proposal-corrupt" }` if
  called anyway (§4.R6).
- Phase 5 scheduled proposals expose only `authority.availableDecisions`.
  Unknown, stale, unavailable, mismatched, or unrecognized daemon lifecycle
  metadata disables both actions. Cave never substitutes a local default.
- A veto uses the existing reject forwarder. The UI label changes to **Veto**
  only when the verified daemon lifecycle is `veto-window-open`; the request is
  still `POST /api/proposals/[id]/reject`.
- `human_required` approval requires a non-empty `note` before Cave enables the
  approve button. The daemon remains authoritative and rejects missing
  rationale even if the route is called directly.

### 3.8 Response envelope — freshness on every response

Every response (success or blocked) is wrapped:

```jsonc
{
  "data": …,                            // route-specific read model, or null when blocked
  "meta": {
    "observedAt": "2026-07-15T09:00:00Z",  // when the adapter observed the source
    "staleAfter": "2026-07-15T09:00:30Z",  // when this observation stops being trustworthy
    "sourceCursor": "ward_audit:412 | weave:<weave_hash-hex> | pending:<dir-content-hash>",
    "adapter": "daemon | fixtures",
    "verified": true                       // false ⇒ §4.R8: render blocked
  },
  "blocked": false                      // true ⇒ data null, why present
}
```

`sourceCursor` is the monotonic (or content-addressed) identifier of the
source state backing the response: max `ward_audit` rowid for audit reads,
`weave_hash` for weave/thread/strand reads, a content hash of the pending
directory listing for proposals. Honest staleness beats fresh-looking lies:
adapters never fabricate `observedAt`.

### 3.9 Staleness

A client (or test) holding a response past `staleAfter` MUST render the §2.1
`stale` state: banner + last-known state, actions disabled. Stale never
silently re-renders as healthy; only a fresh fetch with a newer
`observedAt` clears it.

## 4. Fail-closed rendering rules (complete enumeration)

Every row is a test fixture in `.17.2` and a rendering test in `.17.3`–`.17.6`.
"Blocked" = the blocked pill/banner treatment: state is visibly not-healthy,
approval actions disabled, evidence trace still reachable.

| # | State | Detection | Rendering (never healthy-by-default) |
|---|---|---|---|
| R1 | Thread/weave tension unknown | adapter returns `state: "unknown"` | Blocked pill; rollup treats unknown as worse than holds |
| R2 | Descriptor without predicate | `patternDescriptor` present, `coherence` unknown | Descriptor renders **only** with derived label; status area blocked |
| R3 | Daemon timeout | adapter timeout in `daemon-present` | Stale banner + last-known state; POST actions disabled |
| R4 | Daemon absent, no fixtures | neither source resolvable | Full-surface blocked state, "cannot verify" copy |
| R5 | Approve/reject while daemon unavailable | POST returns 503 | Action fails visibly; decision NOT queued; proposal stays pending |
| R6 | Corrupt pending file | JSON parse failure | Listed as `corrupt`, raw bytes inspectable, both actions disabled (409 if forced) |
| R7 | Audit entry with unresolvable refs | `proposalId`/`threadId` lookup fails | Entry shown with unresolved-reference marker; never silently dropped |
| R8 | Envelope missing/unverified `meta` | `meta` absent or `verified: false` | Contract violation ⇒ blocked; log loudly in dev |
| R9 | Stale response | now > `staleAfter` | §3.9 stale treatment |
| R10 | Snapped thread | `state: "snapped"` | Terminal treatment: read-only, "fresh authority ceremony required" copy; no repair affordance |
| R11 | Unknown route/id (404s) | id not in source | 404 with envelope, `blocked: true`; UI renders not-found as blocked, not empty-healthy |
| R12 | Familiar ward config unparseable | §2.7 `degraded` entry in weaves response | Blocked rail row named for the familiar, "ward unreadable — protection not verifiable" copy, sanitized error in trace, zero threads shown, all actions disabled; never silently absent (added 2026-07-18) |
| R13 | Scheduled proposal lacks daemon lifecycle metadata | staged file has Phase 5 schema but daemon join is absent/unavailable | Proposal remains inspectable with blocked authority; both actions disabled |
| R14 | Daemon proposal metadata disagrees with staged authority | id, canonical `familiarUuid`, writer, staged time, target set, or committed proposal contents mismatch | Blocked `daemon-mismatch`; no locally preferred source |
| R15 | Unknown lifecycle/path combination | unrecognized enum value or invalid combination | Blocked `unknown-lifecycle`; never fall back to legacy actions |
| R16 | Scheduled proposal metadata stale | response is past `staleAfter` | Last-known lifecycle may render as trace only; both actions disabled until a fresh daemon response |

Rollup arithmetic (R1, normative): `snapped > frayed > unknown > stale > holds`.

## 5. Metaphor-referent glossary for UI copy

Scoped to UI surfaces; each line is the binding UI copy must preserve
(`docs/glossary.md` is the full reference). Charm's voice pass (`.17.7`)
rewrites *tone*, never *referents*; Nova's review (`.17.8`) blocks on drift.

- **Weave** — this familiar's enforced pattern of threads; what the rail lists. Coherent iff its pattern predicate holds.
- **Thread** — one authority relationship, *surface → writer*; what a pane row is. Never "a conversation".
- **Strand** — one fiber of commitment inside a thread (hash, signature, manifest entry, audit anchor, serialization marker); what inspection shows.
- **Channel** — the axis of load a mutation arrives through: deliberate, forced, serialization, mutation.
- **Tension** — a thread's standing under load: **Holds** (full contract), **Frayed** (one strand failed; repairable; must surface to you), **Snapped** (terminal; fresh authority ceremony required).
- **Proposal** — a staged write awaiting the principal's decision; data, not authority.
- **Blocked** — the fail-closed rendering of anything unknown or unverifiable; honest, not alarming.
- **Derived** — descriptor-sourced summary for legibility; never what decided.
- **Trace** — the predicate evidence behind a status pill; every pill opens one.

Voice (for `.17.7`): patient, precise, evidence-first. Copy states what is
verified, what is not, and what the principal can do — it never reassures
beyond the evidence and never dramatizes a fray.

## 6. Out of scope (refuse politely, file a follow-up bead)

General policy engine; `.af` compatibility; replacing the daemon trust
boundary; any UI write path bypassing the daemon; Shape A/B portability
decision (`threads-986.16`); OpenTrust code vendoring or its Tauri/Convex
packaging; redesigning `coven-cave`'s shell beyond what the four surfaces
need; Phase 5+.

## 7. Conformance checklist for `.17.2`–`.17.6` closes

- [ ] Every route in §3 registered in `api-contracts.test.ts` with correct methods/kind/guards
- [ ] Both adapters answer every route with the §3.8 envelope
- [ ] All ten fixture states exist and are exercised
- [ ] Every §4 row has a test proving its rendering
- [ ] No code path renders healthy from unverifiable input (R1–R16 sweep)
- [ ] POST routes forward-only; grep-level check: no `fs.write`/`unlink` under the proposals routes, no sqlite writes anywhere in Cave API
- [ ] Phase 5 fixtures cover `awaiting-human-approval`,
      `veto-window-open`, `ready-for-replay`, and `blocked`, plus unavailable,
      mismatched, and unknown daemon metadata
- [ ] Cave renders daemon approval labels verbatim and never advances lifecycle
      from its own clock
- [ ] Every manual Phase 5 decision forwards the daemon-issued
      `proposalRevision`; missing or changed revisions fail closed
- [ ] `availableDecisions` follows the §2.6 closed mapping; rationale-required
      approval stays disabled until a non-empty note exists
- [ ] `pnpm typecheck`, `pnpm test:app`, `pnpm test:api` green
