# Phases

> This page is the honest status ledger. Labels used throughout: `[FROZEN]` (design complete and change-controlled), `[IMPLEMENTED, NOT ENFORCING]` (code exists with green tests in this repo, but nothing deployed runs it), `[BLOCKED]` (waiting on a named decision), `[NOT STARTED]`.
>
> The one-sentence truth: **the design is frozen, the core crate is written and tested, and no enforcement exists anywhere in production.** No daemon in the wild calls this code. If a doc or deck implies otherwise, this page wins.

Vocabulary (bound in [concepts.md](concepts.md)): **Thread** = authority relationship *surface → writer*; **Weave** = enforced pattern of threads; **Strand** = fiber inside a thread; **Channel** = axis of load.

## Phase 0 — Design `[FROZEN]`

**Deliverable:** the design doc `specs/PHASE-0-DESIGN.md`, plus the repo scaffold and beads plan. No enforcement code was in scope, deliberately: *"If this doc is wrong, we would rather find out reading it than debugging it."*

**Status:** FROZEN v0.2, 2026-07-14, tag `v0.2-phase0-design`. The freeze gate was real: Sage's v0.1 draft, Echo's substrate-authority pass (which rebound the metaphor to correct referents and introduced the `Channel` enum), Nova's non-negotiables folded in, and a formal Nova sign-off including a verified round-trip against RFC-0001 §5.1 / §5.4 / §5.6. A post-freeze citation amendment (2026-07-15, bead `threads-986.12`) pointed C7's canonical home at the `coven-grimoire` Ward Layer Spec Brief §9 — a citation fix, not a design change.

The frozen doc is change-controlled. These docs describe it; they do not amend it.

## Phase 1 — Core crate `[IMPLEMENTED, NOT ENFORCING]`

**Scope (beads `threads-986.6`–`.11`, Cody's Rust lane):** the `coven-threads-core` crate — `Strand`, `Thread`, `Weave`, `Channel`, `TensionState` types; the `PatternPredicate` trait with its derived `describe()` introspection; the hash-manifest layer (Merkle over strand hashes in canonical `(surface_path, writer_id)` order); and the RFC-0001 §5 conformance test suite mirrored into Rust.

**Status:** landed on `main` (commit `86550d8`), beads `986.6`–`.11` closed with evidence. The full workspace test suite is green — 98 tests as of 2026-07-15 (72 unit, 12 C7 round-trip, 14 §5 conformance). `unsafe_code = "forbid"` at the workspace level.

**What "implemented" does not mean:** the crate is a library. It has no side effects by design — no filesystem verification, no audit writes, no staging I/O. Until a daemon calls it (Phase 2), it enforces nothing.

## Phase 2 — Daemon integration `[IMPLEMENTED CRATE-SIDE; DAEMON MERGE BLOCKED]`

**Scope:** the validator call site inside the `coven` daemon's existing socket handling; the `DegradeToProposal` staging path at `~/.coven/pending/`; the `ward.audit` table live in `coven.sqlite3`; the notification protocol to the principal.

**Status, in two halves:**

- **Crate side — landed** (commit `5e68957`): `audit.rs` defines the `ward.audit` record shape and DDL (append-only via triggers, RFC-0001 §5.6 event vocabulary); `staging.rs` defines the pending-proposal record shape. The crate owns the *contracts*; the daemon owns the connection, the writes, and the directory.
- **Daemon side — written but unmerged** `[BLOCKED]`: the validator call site (on `POST /familiars/{id}/edits`), staging, and the live audit table exist on the `coven` branch `feat/threads-gate-validator`. **Merge is gated on a Val decision (bead `threads-986.19`)** — integration touches `coven` internals, and cross-repo write authority was itself gated (bead `threads-986.15`, Val-granted for the branch work; the merge remains Val's call).

Until that merge lands and ships, **no request anywhere flows through this gate.** That is the honest current state.

## Phase 3 — Portability format `[IMPLEMENTED SEMANTICS; ENVELOPE BLOCKED]`

**Scope:** the Coven Familiar Portability Format — the artifact a familiar exports to and imports from, with C7 enforced across the round-trip.

**Status:** the *semantics* are implemented and tested (`portability.rs` + the 11-test `c7_roundtrip.rs` suite): the `PortableWeave` envelope, the `SerializationContract` with its drift-visible contract hash, `export_weave`/`import_weave` with the full fail-visibly matrix (tamper → hash mismatch; version skew, contract skew, duplicate pairs → typed refusals; import never widens authority). The *interchange encoding* — Shape A (`.af` superset) vs Shape B (net-new `.weave`) — is drafted in `specs/PHASE-3-PORTABILITY.md` and **blocked on a Val decision (bead `threads-986.16`)**.

**Not `.af`-compatible — documented divergence.** Whatever shape wins, the format will not be a compatible `.af` round-trip surface. The reason is factual, source-verified 2026-07-14 against `letta-ai/letta/main/letta/serialize_schemas/pydantic_agent_schema.py`: Letta's `CoreMemoryBlockSchema` has no protection field, and the runtime `read_only` flag is stripped at export. An artifact format that cannot represent the protection contract cannot satisfy C7 — silent downgrade on import is precisely the failure mode C7 exists to refuse. This is a neutral engineering constraint, not a judgment of `.af` for its own goals; see the [FAQ](faq.md#why-isnt-it-af-compatible).

## Phase 4 — Coven Cave UX `[NOT STARTED]`

**Scope (bead `threads-986.17`):** cockpit surfaces in Coven Cave — the weave rail view, a thread detail pane with tension state, strand inspection, and the proposal approval flow for staged writes from `~/.coven/pending/`. Charm owns the voice/copy pass on all four surfaces.

**Status:** not started, correctly so — the Phase 4 surfaces render objects that only become live when Phase 2 ships. New daemon HTTP endpoints for weave/thread/strand inspection arrive with this phase; no breaking changes to `coven-cave` before then.

## Summary table

| Phase | What it is | Status | Gate to next step |
|---|---|---|---|
| 0 | Design doc + scaffold | `[FROZEN]` v0.2, tag `v0.2-phase0-design` | — (done) |
| 1 | `coven-threads-core` crate | `[IMPLEMENTED, NOT ENFORCING]`, 98 tests green | — (landed on main) |
| 2 | Daemon integration | Crate contracts landed; daemon branch unmerged `[BLOCKED]` | Val decision `threads-986.19` |
| 3 | Portability format | C7 semantics implemented; envelope shape `[BLOCKED]` | Val decision `threads-986.16` |
| 4 | Coven Cave UX | `[NOT STARTED]` | Phase 2 shipping |

## Known housekeeping discrepancies

Tracked in `docs/STATUS-2026-07-15.md` and worth knowing when reading the repo:

- **License mismatch:** the design doc and README say *Apache-2.0 (planned)*; the committed `LICENSE` file is MIT (with a separate `PATENTS` file). Needs a deliberate reconciliation; until then, treat the license as unsettled.
- **`.bak` files in `specs/`:** three pre-freeze backups sit beside the frozen doc; git history already preserves them.
