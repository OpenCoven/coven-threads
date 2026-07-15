# Phases

> This page is the honest status ledger. Labels used throughout: `[FROZEN]` (design complete and change-controlled), `[MERGED]` (landed on the `main` branch of a downstream system that will call this crate at runtime once cut into a release; buildable and integration-tested but **not in a released binary** yet), `[RELEASED]` (in a tagged downstream release users can install), `[ENGINEERING FROZEN]` (code complete, tests green, awaiting one named decision to reach a deployed system), `[BLOCKED]` (waiting on a named decision), `[NOT STARTED]`.
>
> The one-sentence truth: **the design is frozen, the core crate is written and tested, and no enforcement exists anywhere in production.** No daemon in the wild calls this code. If a doc or deck implies otherwise, this page wins.

Vocabulary (bound in [concepts.md](concepts.md)): **Thread** = authority relationship *surface → writer*; **Weave** = enforced pattern of threads; **Strand** = fiber inside a thread; **Channel** = axis of load.

## Phase 0 — Design `[FROZEN]`

**Deliverable:** the design doc `specs/PHASE-0-DESIGN.md`, plus the repo scaffold and beads plan. No enforcement code was in scope, deliberately: *"If this doc is wrong, we would rather find out reading it than debugging it."*

**Status:** FROZEN v0.2, 2026-07-14, tag `v0.2-phase0-design`. The freeze gate was real: Sage's v0.1 draft, Echo's substrate-authority pass (which rebound the metaphor to correct referents and introduced the `Channel` enum), Nova's non-negotiables folded in, and a formal Nova sign-off including a verified round-trip against RFC-0001 §5.1 / §5.4 / §5.6. A post-freeze citation amendment (2026-07-15, bead `threads-986.12`) pointed C7's canonical home at the `coven-grimoire` Ward Layer Spec Brief §9 — a citation fix, not a design change.

The frozen doc is change-controlled. These docs describe it; they do not amend it.

## Phase 1 — Core crate `[MERGED, NOT RELEASED]`

**Scope (beads `threads-986.6`–`.11`, Cody's Rust lane):** the `coven-threads-core` crate — `Strand`, `Thread`, `Weave`, `Channel`, `TensionState` types; the `PatternPredicate` trait with its derived `describe()` introspection; the hash-manifest layer (Merkle over strand hashes in canonical `(surface_path, writer_id)` order); and the RFC-0001 §5 conformance test suite mirrored into Rust.

**Status:** landed on `main` (commit `86550d8`), beads `986.6`–`.11` closed with evidence; **Phase 1 FREEZE recorded in `threads-986.18` (closed).** The full workspace test suite is green — 98 tests as of 2026-07-15 (72 unit, 12 C7 round-trip, 14 §5 conformance). `unsafe_code = "forbid"` at the workspace level.

**What "implemented" does not mean:** the crate is a library. It has no side effects by design — no filesystem verification, no audit writes, no staging I/O. Until a daemon calls it (Phase 2), it enforces nothing.

## Phase 2 — Daemon integration `[FROZEN; MERGED TO COVEN MAIN]`

**Scope:** the validator call site inside the `coven` daemon's existing socket handling; the `DegradeToProposal` staging path at `~/.coven/pending/`; the `ward.audit` table live in `coven.sqlite3`; the notification protocol to the principal.

**Status, in two halves:**

- **Crate side — landed** (commit `5e68957`): `audit.rs` defines the `ward.audit` record shape and DDL (append-only via triggers, RFC-0001 §5.6 event vocabulary); `staging.rs` defines the pending-proposal record shape. The crate owns the *contracts*; the daemon owns the connection, the writes, and the directory.
- **Daemon side — merged** `[MERGED, NOT RELEASED]`: the validator call site (on `POST /familiars/{id}/edits`), staging path, and the live audit table landed on coven `main` via **PR https://github.com/OpenCoven/coven/pull/382** (branch `feat/threads-gate-validator`, squash-merged 2026-07-15). The Phase 2 epic bead `threads-986.14` is **closed** (engineering complete), **`threads-986.20` (Phase 2 FREEZE) is closed**, and **`threads-986.19` (merge gate) is closed** — resolved by flipping `coven-threads` public so coven CI can fetch the pinned git dependency (deny.toml `[sources]` allow-lists it). Cross-repo write authority for the branch work itself was already gated and Val-granted (bead `threads-986.15`, closed).

Every `POST /familiars/{id}/edits` touching a tier-0 surface on a daemon built from coven `main` now flows through this gate.

## Phase 3 — Portability format `[ENGINEERING FROZEN; ENVELOPE DECIDED]`

**Scope:** the Coven Familiar Portability Format — the artifact a familiar exports to and imports from, with C7 enforced across the round-trip.

**Status:** **Phase 3 FREEZE recorded in `threads-986.21` (closed).** The *semantics* are implemented and tested (`portability.rs` + the 12-test `c7_roundtrip.rs` suite): the `PortableWeave` envelope, the `SerializationContract` with its drift-visible contract hash, `export_weave`/`import_weave` with the full fail-visibly matrix (tamper → hash mismatch; version skew, contract skew, duplicate pairs → typed refusals; import never widens authority). The *interchange encoding* is **decided** (`threads-986.16` closed, 2026-07-15): **Shape B — the net-new `.weave` envelope — is canonical**, plus a clearly-marked lossy one-way `.af` exporter for Letta handoff (follow-up bead `threads-jq4`; no `.af` import path, ever). Decision record: `specs/PHASE-3-PORTABILITY.md` §6.

**Not `.af`-compatible — documented divergence.** Whatever shape wins, the format will not be a compatible `.af` round-trip surface. The reason is factual, source-verified 2026-07-14 against `letta-ai/letta/main/letta/serialize_schemas/pydantic_agent_schema.py`: Letta's `CoreMemoryBlockSchema` has no protection field, and the runtime `read_only` flag is stripped at export. An artifact format that cannot represent the protection contract cannot satisfy C7 — silent downgrade on import is precisely the failure mode C7 exists to refuse. This is a neutral engineering constraint, not a judgment of `.af` for its own goals; see the [FAQ](faq.md#why-isnt-it-af-compatible).

## Phase 4 — Coven Cave UX `[NOT STARTED]`

**Scope (bead `threads-986.17`):** cockpit surfaces in Coven Cave — the weave rail view, a thread detail pane with tension state, strand inspection, and the proposal approval flow for staged writes from `~/.coven/pending/`. Charm owns the voice/copy pass on all four surfaces.

**Status:** not started, correctly so — the Phase 4 surfaces render objects that only become live when Phase 2 ships. New daemon HTTP endpoints for weave/thread/strand inspection arrive with this phase; no breaking changes to `coven-cave` before then.

## Summary table

| Phase | What it is | Status | Gate to next step |
|---|---|---|---|
| 0 | Design doc + scaffold | `[FROZEN]` v0.2, tag `v0.2-phase0-design` | — (done) |
| 1 | `coven-threads-core` crate | `[MERGED, NOT RELEASED]` — imported by coven `main` (Cargo.toml git dep at tag `v0.1.2`); the running coven daemon binary (release `v0.0.54`, 2026-07-14) predates PR #382 and does not yet call it; 98 tests green; `.18` closed | — (frozen) |
| 2 | Daemon integration | `[MERGED, NOT RELEASED]`, `.14` + `.20` + `.19` closed; PR #382 merged 2026-07-15 as commit `f745117`; next coven release will include it (current release `v0.0.54` predates the merge) | — (release-cut) |
| 3 | Portability format | `[ENGINEERING FROZEN]`, `.21` + `.16` closed; envelope `[DECIDED: Shape B + lossy .af export]` | follow-up `threads-jq4` (exporter) |
| 4 | Coven Cave UX | `[NOT STARTED]` | Phase 2 shipping |

## Known housekeeping discrepancies

Tracked in `docs/STATUS-2026-07-15.md` and worth knowing when reading the repo:

- **License mismatch:** the design doc and README say *Apache-2.0 (planned)*; the committed `LICENSE` file is MIT (with a separate `PATENTS` file). Needs a deliberate reconciliation; until then, treat the license as unsettled.
- **`.bak` files in `specs/`:** three pre-freeze backups sit beside the frozen doc; git history already preserves them.
