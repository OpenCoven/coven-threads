# PHASE-0 — coven-threads design document

**Status:** FROZEN v0.2 (Sage v0.1 draft; Echo substrate-authority pass; Nova non-negotiables folded + formal sign-off 2026-07-14; RFC-0001 §5 round-trip verified; Phase 0 complete)
**Date:** 2026-07-14
**Owners:** Sage 🌿 + Echo 🔮 co-drive design doc; Nova 👑 + Sage assign lanes after freeze
**External correctness anchor:** RFC-0001 v0.2.0 §5 (must round-trip; paper cites this as the referenced-but-unbuilt companion)
**License:** Apache-2.0 (planned)

---

## 0. Purpose of this document

This is Phase 0 of `coven-threads`. Sole deliverable: this design doc plus the beads scaffold. No enforcement code lands in Phase 0. The point is to make the shape of the layer legible enough that Nova, Echo, Cody, and Val can push back before we spend any Rust cycles.

If this doc is wrong, we would rather find out reading it than debugging it.

## 1. What `coven-threads` is (and is not)

**Is:** an authority-boundary gate layer that sits *above* the existing `coven` Rust daemon and *underneath* every familiar's protected memory surface. It gives the daemon a *gate-shaped receiver* for identity-surface mutation requests — the missing piece that turns the boundary into structural enforcement (RFC-0001 §5.1 specifies the boundary; Ward v0.2 specifies the four gates; this repo is the receiver that binds them).

**Is not:**
- A replacement for the `coven` daemon. The daemon stays authoritative for launch/cwd/kill/path ops (`coven/docs/SAFETY-MODEL.md`); `coven-threads` extends it with typed protected-surface validation.
- A policy engine. This is a *typed* authority layer for OpenCoven familiar surfaces; typed correctness is the goal, not general-purpose reusability.
- A serialization format. That's Phase 3.
- `.af`-compatible (documented divergence, 2026-07-14 — see MEMORY.md → Architecture Principles → Portability Decisions).

**Line-one conformance requirement (Nova non-negotiable, RFC-0001 §5.4 Gate 4):** Gate 4 fail-closed is a conformance property. An implementation that allows Gate 4 to be bypassed DOES NOT conform to RFC-0001. `coven-threads` states this at line one, not as Phase-1 hardening.

## 2. Vocabulary — the weaving metaphor as architecture

The metaphor was named by Val (2026-07-14). It is preserved here **only where it carries semantic weight**. Each metaphor term is bound to a concrete referent at first use so the code and vocabulary stay coupled. If the metaphor drifts ahead of the referent, we become Letta-shaped: beautiful language, unclear semantics.

Echo revised this section (v0.1.1) after Sage's v0.1 sketch loaded the wrong axis (threads-as-static-contracts, gates-as-threads). The v0.1.1 mapping below is substrate-authority-grounded.

### 2.1 Thread — an authority relationship

**Thread** (`authority relationship: surface → writer`) — a directional line from a *protected surface* (SOUL.md, MEMORY.md, an identity field) to the *authority that gates writes to it*. One thread per `(surface, writer)` pair.

- Not "contract" (too static). Not "one per gate" (gates are the loom, not the thread).
- A thread has **tension**: it either holds under load, or it snaps. Load = mutation attempts, forced compaction, injection, jailbreak, serialization.
- Every gate check becomes: **"does thread T hold under channel C?"** That question is the enforcement vocabulary the metaphor gives us for free.
- Threads are first-class inspectable objects: `coven-threads inspect <thread-id>` MUST return current tension state.

### 2.2 Weave — the enforced pattern of threads

**Weave** (`enforced pattern of threads across a familiar or Coven`) — the invariant that these specific threads must all hold together for the identity to be coherent. Not just the tapestry-as-visual-whole; the *structural pattern* that determines which threads cross which, and where the weave breaks if a thread snaps.

- Ward v0.2's four gates are **not** threads. They are the **loom** the weave is made on. Threads run through gates; the gates give the weave its structure.
- The **pattern** is what Ward validates, not individual threads. This maps directly onto "authority-over-the-weave-not-the-rows" (§3.3.1): the weave carries authority; individual threads are just where it's expressed.
- **Anti-pattern (prose, not just code — Echo second turn 2026-07-14):** the pattern is defined by a *predicate* (§4 `PatternPredicate::coherent`), and that predicate is authoritative. The predicate also carries a *derived* structural summary (§4 `PatternPredicate::describe -> PatternDescriptor`) for humans, tools, and CovenCave rendering. **The descriptor MUST NOT become authoritative.** If any downstream component ever gates enforcement on the descriptor instead of the predicate, we have reinvented the derived-index problem one layer up — the exact failure mode Ward-authority-over-indexer-not-rows was designed to avoid. Descriptors are for legibility; predicates are for enforcement. This is the same source-authoritative discipline the memory retrieval substrate resolved (source authoritative, index derived) applied to the authority layer itself (predicate authoritative, descriptor derived).
- A weave is **coherent** iff its pattern predicate holds — which means (among other things) that every thread required by the pattern is intact. A single snapped thread degrades the weave *at that thread's surface*; the weave surfaces which surface degraded, not just "something is wrong."
- Weaves are the primary object CovenCave will render (Phase 4).

### 2.3 Strand — fibers inside a thread

**Strand** (`fiber inside a thread: hash | sig | manifest entry | audit trail | serialization marker`) — the fibers that make a thread survive stress. Multi-strand threads are stronger; a thread survives a channel iff its strands survive that channel.

- Strand kinds (initial set, v0.1.1): `ContentHash`, `Signature`, `ManifestEntry`, `AuditTrail`, `SerializationMarker`.
- A thread **frays** when one of its strands fails but the thread has not yet snapped. Fraying is the intermediate state between "holds" and "snapped." Frayed threads MUST surface to the operator.
- Strands make failure legible: instead of "thread snapped," you get "thread frayed at strand `ContentHash` — SOUL.md hash mismatch, detected on channel `Forced`."
- **The fourth invariant (survives serialization) lives at the strand level.** A thread survives serialization iff *all its strands* survive the lossy transform. `SerializationMarker` is the strand that carries the survival contract explicitly.

### 2.4 Channels — the axis threads must hold under

Introduced in v0.1.1. Threads don't just "hold" — they hold under specific *channels* of load:

- `Channel::Deliberate` — deliberate compaction (promotion, dreaming, memory flush). Familiar-initiated, principal-gated.
- `Channel::Forced` — forced context compaction (auto-compact when the runtime evicts). Runtime-initiated, no familiar cooperation available. Threads here MUST have strands surviving without agent-side intervention (typically hash + external manifest). **This is the channel WARD-C1–C6 governs.**
- `Channel::Serialization` — export/import round-trip. Ward-external, format-mediated. Threads here MUST have a `SerializationMarker` strand. **This is the channel the new C7 invariant governs.**
- `Channel::Mutation` — direct client mutation attempts routed through the daemon. Default channel; every thread holds under it or the daemon rejects.

### 2.5 Metaphor-referent binding rule

Every metaphor term is defined at first use with its concrete referent in parentheses. If a future contributor uses "thread" without meaning "authority relationship: surface → writer," they are wrong and the code will not compile.

## 3. Relation to prior work in this repo family

### 3.1 `coven` daemon (shipped)

The `coven` daemon provides the trust boundary. Its rules today, verified on disk 2026-07-14 in `coven/docs/SAFETY-MODEL.md` (166 lines, dated 2026-06-25):

> *"The Rust daemon is the authority boundary. Every client is untrusted for enforcement purposes."*

Also enforced:
- Canonicalize `projectRoot` and `cwd` before comparing paths.
- Reject working directories outside the project root.
- Allowlist harness ids.
- Argv-only harness commands, never `sh -c`.
- Fail closed on unknown API versions / action ids.
- Runtime state at `~/.coven/coven.sock` (unix socket) + `~/.coven/coven.sqlite3` (daemon DB) + `~/.coven/memory/archival.sqlite3` (memory store).

**What the daemon does not do today:** validate that a client request is *authorized against a typed protected surface*. It validates *who* and *what action*, not *what target-file's authority state permits*. `coven-threads` fills that gap.

### 3.2 RFC-0001 (upstream normative source)

RFC-0001 §5.1 (authority-layer separation) is the *external correctness anchor* for this repo. Verbatim (Ward v0.2 publication audit, 2026-07-04, citation #4):

> *"Rust-side validation logic operating on the execution harness directly; F-controlled processes have no write path to daemon or to W(F,P)."*

Three MUST-NOTs from §5.1 (Nova non-negotiable #1):
1. F MUST NOT modify the Ward file.
2. F MUST NOT restart the authority process.
3. F MUST NOT bypass gates.

`coven-threads` is the gate-shaped receiver reachable *only* by the privileged daemon, never by a familiar-controlled process. This is the "shipped boundary, no gate-shaped receiver" gap made real: the boundary (§5.1) is spec'd and the daemon exists; `coven-threads` is the receiver.

RFC-0001 §5.4 defines Gates 1–4. Gate 4 fail-closed is a conformance requirement. RFC-0001 §5.6 defines `ward_hash` as the audit-log field.

**RFC wins on any conflict.** If this repo disagrees with RFC-0001, this repo is wrong.

### 3.3 The four (now five) invariants, co-designed as channel-survival requirements

`coven-threads` is the single-build ship of these invariants. Echo's v0.1.1 correction: these are not separate properties, they are **channels a weave must survive**. Reframed:

1. **Identity-as-memory-property** — protected surfaces are typed memory layers, not runtime config. Threads bind to typed surfaces at construction time.
2. **Structural mutation authority** — the gate is external, cannot be cooperated past. Enforcement is Rust-side, called by the daemon, not by the familiar. **This is what Ward-authority-lives-on-the-weave-not-the-rows means concretely** (see §3.3.1).
3. **Two-compaction contract** — `Channel::Deliberate` and `Channel::Forced` are distinct channels with distinct thread-survival requirements. WARD-C1..C6 = "these are the threads that must hold under `Channel::Forced`." Preserved from `autocompacting-and-the-protected-surface-2026-07-03.md` §6, cross-linked to `coven-grimoire` PR #3 (Ward Layer Spec Brief §9). Not invented here; inherited by reference. Nova non-negotiable #4.
4. **Survives serialization (new, numbered C7)** — `Channel::Serialization` is the fourth channel. Every thread that must round-trip carries a `SerializationMarker` strand. Named as **C7** so lineage from C1–C6 is preserved and the addition is legible, not orphaned. Nova non-negotiable #4. **Canonical home: `coven-grimoire` Ward Layer Spec Brief §9, where C1–C7 are jointly canonical** (C7 landed there per bead `threads-986.12`; this section cites, it does not define).

None of the four ships alone. Phase 0's design doc must show *how* all four are co-designed in the type system — concretely, via the `Channel` enum in §4 — or Phase 0 is not done.

**WARD-C1–C6 recap (from 2026-07-03 synthesis, inherited by reference — authoritative text: `coven-grimoire` Ward Layer Spec Brief §9, which now lists C1–C7 jointly):**
- C1: protected surfaces are exempt from compaction eviction (compaction cannot delete SOUL from context)
- C2: derived-not-source (compaction summaries are derived; source remains authoritative)
- C3: pre/post hash gate on compaction boundary — the `pause_after_compaction` API hook is the real implementable interlock
- C4: re-injection floor (post-compaction context must re-inject protected surfaces above a minimum)
- C5: fail-closed on compaction anomaly
- C6: compaction ledger appended to `ward.audit`

**C7 (new tonight):** every thread bound to `Channel::Serialization` MUST carry a `SerializationMarker` strand whose survival is a round-trip invariant. Export followed by import produces a weave with equivalent tension state, or fails visibly.

### 3.3.1 Ward-authority-lives-on-the-weave-not-the-rows

Echo, v0.1.1. Derived-index argument from `MEMORY-SCHEMA.md` §7.0 restated:

- `coven-threads` gates **authority relationships**, not the data those relationships protect.
- No thread ever terminates on a derived structure (indexer, retrieval cache, promoted view). Threads terminate only on *source-authoritative surfaces*.
- The index has no authority to be tampered with because no thread terminates on it. Tampering is detected by re-derivation, not by gating the derived structure itself.
- The WAL/rebuild write-topology decision (`[PROPOSED — pending Nova Ward v0.2 §10.q1]`) reshapes an appendix, not the spine. If the answer is "index needs to be gated directly," we add a "derived structure gating" section wrapping the indexer in its own weave. Spine holds either way.

**Anti-non-negotiable:** `coven-threads` does not own retrieval, promotion, or dreaming. It owns *authority over writes to the protected surface, gated by the weave*. Scope creep here fragments the atomic-ship trio. Hold the line.

### 3.3.2 Source-authoritative retrieval as a Phase-0 invariant

Files are primary. Derived structures rebuild from source. No thread ever terminates on a derived structure. Holds as a Phase-0 invariant even though the specific WAL/rebuild write-topology is `[PROPOSED]`.

### 3.4 Single daemon-owned audit store (Nova non-negotiable #3)

`coven-threads`' event/audit log **extends** the existing `~/.coven/coven.sqlite3` and socket API, not stands up a parallel store. Two audit stores = drift.

Phase 0 decision: `ward.audit` is a **table inside `coven.sqlite3`**, reachable through the existing socket, daemon-owned. Rationale: WARD-C6 compaction ledger appends to `ward.audit`; RFC-0001 §5.6 defines `ward_hash` as an audit-log field; the daemon already owns `coven.sqlite3`. Any alternative (sidecar file, separate DB) creates two sources of audit truth. One store.

### 3.5 Ward v0.2 (spec published, enforcement not built)

Ward's four gates specify *what* to check. `coven-threads` implements *how* those gates enforce, on the `coven` daemon substrate. Ward is the spec; `coven-threads` is the implementation. Version pin: RFC-0001 v0.2.0+.

## 4. Type sketch (Rust, v0.1.1)

Not final. Placeholder for Cody's Phase 0 scoping read (bead threads-986.5).

```rust
// crates/coven-threads-core/src/lib.rs

pub struct StrandId(pub Uuid);
pub struct ThreadId(pub Uuid);
pub struct WeaveId(pub Uuid);
pub struct SurfaceId(pub PathBuf);   // typed surface, not raw path — §3.3 invariant #1
pub struct WriterId(pub String);     // opaque; matches MutationAuthority variant

pub enum Channel {
    Deliberate,      // promotion, dreaming, flush; familiar-initiated
    Forced,          // context auto-compact; runtime-initiated, no cooperation
    Serialization,   // export/import; format-mediated
    Mutation,        // direct client mutation via daemon; default
}

pub enum Strand {
    ContentHash         { algorithm: HashAlgo, value: Vec<u8> },
    Signature           { key_id: String, kind: SigKind, value: Vec<u8> },
    ManifestEntry       { manifest_id: Uuid, entry_hash: Vec<u8> },
    AuditTrail          { first_seen: DateTime<Utc>, event_log_ref: EventRef },
    SerializationMarker { format_version: SemVer, contract_hash: Vec<u8> },
}

pub struct Thread {
    pub id: ThreadId,
    pub surface: SurfaceId,
    pub writer: WriterId,
    pub strands: Vec<Strand>,
    pub holds_under: Vec<Channel>,   // channels this thread must survive
    pub created_at: DateTime<Utc>,
    pub tension: TensionState,
}

pub enum TensionState {
    Holds,
    Frayed  { strand: StrandId, channel: Channel, reason: FrayReason, detected_at: DateTime<Utc> },
    Snapped { channel: Channel, reason: SnapReason, at: DateTime<Utc> },
}

impl Thread {
    /// Does this thread hold under this channel? The load-bearing question.
    pub fn holds_under(&self, channel: Channel) -> Result<(), FrayOrSnap>;
    /// Current tension state.
    pub fn tension(&self) -> &TensionState;
}

pub struct Weave {
    pub id: WeaveId,
    pub familiar_id: FamiliarId,
    pub threads: Vec<ThreadId>,
    pub pattern: Box<dyn PatternPredicate>,
    pub weave_hash: Vec<u8>,   // Merkle over threads sorted by (surface_path, writer_id)
    pub coven_ref: Option<CovenId>,
}

pub trait PatternPredicate {
    /// Authoritative gate: does this set of threads, in current tension state, satisfy the pattern?
    fn coherent(&self, threads: &[Thread]) -> WeaveCoherence;

    /// Derived, non-authoritative structural summary. For humans, tools, and CovenCave rendering.
    /// MUST NOT be gated on downstream — if anything ever enforces on the descriptor instead of
    /// the predicate, that is the derived-index problem reinvented one layer up.
    fn describe(&self) -> PatternDescriptor;
}

/// Serializable, stable structural summary of a PatternPredicate.
/// Introspection surface only; not authoritative.
pub struct PatternDescriptor {
    pub name: String,
    pub protected_surfaces: Vec<SurfaceId>,
    pub channels_required: Vec<Channel>,
    pub strand_requirements: Vec<StrandRequirement>,
}

pub struct StrandRequirement {
    pub kind: StrandKind,             // discriminant over Strand variants
    pub required_on_channels: Vec<Channel>,
}

pub enum WeaveCoherence {
    Coherent,
    Degraded { degraded_surfaces: Vec<SurfaceId>, reason: String },
    Broken   { reason: String },
}
```

Open questions (v0.1.1) → Phase 0 bead `threads-986.5` (Cody):
- `Strand` enum vs struct-with-typed-fields — leaning enum for extension.
- `Channel` — four-variant set exhaustive for Phase 0? `Federation` variant deferred.
- `Weave.pattern` as `Box<dyn PatternPredicate>` — **resolved v0.1.1 (Echo second turn)**: trait wins because patterns are what Ward defines, and Ward's vocabulary of authority patterns must be externally definable. Introspection cost is paid by `describe() -> PatternDescriptor`: a stable, serializable structural summary (name, protected surfaces, channels required, strand requirements). Same shape as source-authoritative — the *predicate* is authoritative, the *descriptor* is derived. **Anti-pattern (must be named in prose, not just code)**: `PatternDescriptor` MUST NOT become authoritative. If anything ever gates on the descriptor instead of the predicate, that is the derived-index problem reinvented one layer up. Descriptor is for humans and tools; predicate is for enforcement. Cody may still adjust the ergonomics; the trait-vs-enum shape is closed.
- `Weave.weave_hash` canonical ordering: `(surface_path, writer_id)` lexicographic. Confirm with Cody.
- Removed from v0.1: `MutationAuthority` enum (was collapsing writer + gate into a single axis). v0.1.1 splits: `WriterId` names the writer, `Channel` names the load, `PatternPredicate` names the gate structure. Cleaner separation.

## 5. Enforcement flow (v0.1.1)

Untrusted client → `coven` daemon HTTP-over-unix-socket → daemon calls `coven-threads::validate(request)` → validator loads the relevant weave → checks each affected thread's strands under the request's channel → returns `Permit` / `DegradeToProposal` / `Reject` → daemon acts on the result and appends to `ward.audit` in `coven.sqlite3`.

**Fail-closed on every unknown (Nova non-negotiable #2, RFC-0001 §5.4 Gate 4 conformance):**
- Unknown surface path → Reject.
- Unknown thread for a protected surface → Reject (all protected surfaces MUST have threads).
- Unknown channel → Reject.
- Validator panic → daemon catches and treats as Reject with diagnostic.
- Bypass of any gate → non-conformant to RFC-0001; reject at compile-time via type system where possible.

Failure modes:
- **Frayed thread** → `DegradeToProposal` (write staged to `~/.coven/pending/`, notification to principal, no immediate write to protected surface).
- **Snapped thread** → `Reject`. Weave marked degraded. Familiar continues on other surfaces; broken surface becomes read-only until repair.
- **Missing thread** for protected surface → `Reject`.

## 6. Compatibility contract with existing OpenCoven repos

- **`coven`**: `coven-threads` imported as a crate. Zero socket-protocol changes in Phase 0/1/2. Phase 2 adds *validation calls inside* the daemon's existing request handling; clients see identical wire format, but requests may return a new `DegradeToProposal` outcome. Audit store: `ward.audit` table in existing `coven.sqlite3`.
- **`familiar-contract` (RFC-0001)**: `coven-threads` is a *conforming implementation* of RFC-0001 §5. RFC wins. Version pin: v0.2.0+. **v0.2 freeze gate: round-trip against §5.1 / §5.4 / §5.6 as external correctness anchor** (Nova refinement).
- **`coven-cave`**: consumes state via daemon HTTP API. New endpoints for weave/thread/strand inspection in Phase 4. No breaking changes before Phase 4.
- **`coven-grimoire`**: Ward Layer Spec Brief §9 (WARD-C1–C6) is the normative reference for compaction invariants. `coven-threads` inherits by reference; C7 is a numbered addition, not a replacement.

## 7. What Phase 0 delivers

- [x] Repo scaffolded at `~/Documents/GitHub/OpenCoven/coven-threads/`
- [x] README, AGENTS.md, SECURITY.md, CONTRIBUTING.md, LICENSE (MIT via coven sibling), PATENTS, .gitignore
- [x] Beads DB initialized (prefix: `threads`)
- [x] Design doc v0.1 (Sage-authored)
- [x] Design doc v0.1.1 (Echo substrate-authority pass + Nova non-negotiables folded)
- [ ] Nova formal review + RFC-0001 §5 conformance round-trip → v0.2 freeze
- [ ] Cody scoping read on §4 (non-blocking for v0.2, blocking for Phase 1)

## 8. Beads plan

Phase 0 filed tonight in `bd`; later-phase beads named but deferred until prior phase closes.

Bead prefix: `threads-*`. Labels: `phase:0` through `phase:4`; `familiar:sage`, `familiar:echo`, `familiar:nova`, `familiar:cody`, `familiar:charm`, `familiar:val`; `surface:design`, `surface:crate`, `surface:daemon`, `surface:cockpit`, `surface:portability`.

**Filed tonight (Phase 0):**
- `threads-986` [EPIC] coven-threads authority-boundary gate layer
- `threads-986.1` ✓ Design doc v0.1 authored (closed)
- `threads-986.2` Echo review of §2 + §3.3 (folded into v0.1.1; may need re-review of final v0.1.1)
- `threads-986.3` Nova review of §3 + §6 (partially folded into v0.1.1; needs Nova formal sign + RFC §5 round-trip → v0.2)
- `threads-986.4` v0.2 freeze after reviews (blocked by .2 and .3)
- `threads-986.5` Cody scoping read on §4 (P2, non-blocking for v0.2)

**Phase 1 (named, deferred until v0.2 freeze):**
- Rust crate `coven-threads-core` scaffolded
- `Strand`, `Thread`, `Weave`, `Channel`, `TensionState` types with tests
- Hash-manifest layer (Merkle over strand hashes, canonical ordering)
- `PatternPredicate` trait + at-least-one concrete pattern for SOUL.md
- RFC-0001 §5 conformance test suite adapted to Rust integration

**Phase 2 (named, deferred):**
- `coven` daemon integration: validator call site inside existing socket handler
- `DegradeToProposal` staging path (`~/.coven/pending/`)
- `ward.audit` table schema in `coven.sqlite3` (extending, not replacing)
- Notification protocol to principal (via daemon's existing channels)

**Phase 3 (named, deferred):**
- Coven Familiar Portability Format v0.1 draft (working name deferred per MEMORY.md)
- Serialization contract for each Strand variant
- Round-trip conformance suite — `Channel::Serialization` invariant enforced
- Shape A vs Shape B decision escalated to Val (per MEMORY.md 2026-07-14 daily note)

**Phase 4 (named, deferred):**
- Cave UX: weave rail view
- Cave UX: thread detail pane with tension state
- Cave UX: strand inspection surface
- Cave UX: proposal approval flow (staged writes from `~/.coven/pending/`)
- Charm UX voice/copy pass on all four surfaces

## 9. Open questions (need resolution before v0.2 freeze)

1. **Metaphor: is the three-level decomposition (thread / weave / strand) sufficient, or does federation force a fourth level (`fabric` for cross-Coven)?** Sage's read: three is enough for v0.1; add fourth later if federation forces it. Deferred to Phase 3+.
2. **Ownership of the daemon integration (Phase 2).** Requires touching `coven` internals. Sage doesn't have Rust-ownership status on `coven`. Nova+Cody joint lane? Nova to assign after v0.2 freeze.
3. **Portability format co-design (Phase 3).** Shape A (extend `.af` as superset) vs Shape B (net-new). MEMORY.md logged "do not pre-commit tonight." Escalate to Val when Phase 3 opens.
4. **UI/UX standards (Phase 4).** Val's original ask included UX customization. Loop Charm in via bead when Phase 4 opens; brief her on weave/thread/strand vocabulary and rail/pane mapping.

## 10. Non-goals for v0.1.1 of this doc

- Full Rust API — §4 is a sketch, not a spec.
- Full enforcement flow — §5 is illustrative, not authoritative.
- Portability format details — Phase 3 problem.
- Cross-Coven federation — deferred.

## 11. Change log

- **2026-07-14 (v0.1)** — Sage draft. Sketched metaphor + type sketch + phase plan.
- **2026-07-14 (v0.1.1)** — Echo substrate-authority pass: metaphor rebound to concrete referents (thread = authority relationship, weave = enforced pattern, gates = loom, strand = fibers); `Channel` enum introduced as first-class; §3.3 reframed as channel-survival requirements; §3.3.1 Ward-authority-on-weave-not-rows added; anti-non-negotiable stated. Nova non-negotiables folded: RFC-0001 §5.1 cited alongside `SAFETY-MODEL.md` (verified on disk 2026-07-14, 166 lines, 2026-06-25); Gate 4 fail-closed stated at line one (§1); one daemon-owned audit store (`ward.audit` in `coven.sqlite3`, §3.4); WARD-C1–C6 inherited by reference with lineage preserved; "survives serialization" numbered as **C7**; RFC-0001 §5 named as external correctness anchor for v0.2 freeze gate.
- **2026-07-14 (v0.1.1 addendum, Echo second turn)** — resolved `Weave.pattern` open question in favor of `Box<dyn PatternPredicate>` with `describe() -> PatternDescriptor` derived-introspection method. Added `PatternDescriptor` + `StrandRequirement` types. Named anti-pattern: descriptor MUST NOT become authoritative — derived-index problem one layer up. `MutationAuthority` enum removed from v0.1; writer/gate/load axes split cleanly across `WriterId` / `PatternPredicate` / `Channel`.
- **2026-07-14 (v0.1.1 confirmation, Echo third turn)** — descriptor-vs-predicate anti-pattern promoted from code comment to prose §2.2. Explicit link named between this authority-layer discipline and the memory-substrate source-authoritative discipline (Nova/Echo 2026-07-06): same shape, different layer. Predicate is source; descriptor is derived. Anti-pattern must be catchable in review before it lands in code.
- **2026-07-14 (v0.2 FREEZE)** — Nova formal sign-off received: RFC-0001 §5 round-trip passes (§5.1 MUST-NOTs, §5.4 Gate 4 fail-closed as *type property*, §5.6 ward_hash, "RFC wins on conflict" load-bearing); §6 compatibility contract correct with one Phase-2 ownership watch-item flagged (not a defect); C7-as-addition reads right (channel framing makes it structural, not tacked-on). No blocking changes at freeze. Cleared to move Phase 0 → Phase 1 kickoff after (a) Cody's `threads-986.5` scoping read on §4 type sketch lands, (b) Val's decisions on repo visibility flip / portability format shape / UX brief timing. Beads: `threads-986.3` closed (Nova review), `threads-986.4` closed (this freeze), `threads-986.5` remaining open as Phase-1-blocking. Design doc `PHASE-0-DESIGN.md` at 358 lines, 12 sections, is the Phase 0 canonical deliverable.
- **2026-07-15 (post-freeze citation amendment, bead `threads-986.12`)** — no design change. C7 canonicalized in `coven-grimoire` Ward Layer Spec Brief §9 (branch `article/ward-layer-spec-brief`, commit 4d35c70; Nova review of the grimoire PR remains the merge gate); §3.3 here updated to cite grimoire §9 as the single authoritative home for C1–C7. Closes the two-locations drift risk the bead named.

## 12. Verification log

- **`coven/docs/SAFETY-MODEL.md`**: verified on disk 2026-07-14. 166 lines, dated 2026-06-25, six copies (main + ru + es + daemon/ + dist/). Nova's memory index missed it (2026-07-14 20:xx CDT flag corrected on receipt); document exists and is normative. Cited alongside RFC-0001 §5.1 rather than as replacement.
- **RFC-0001 §5.1 three MUST-NOTs**: verbatim from Ward v0.2 publication audit 2026-07-04, citation #4.
- **RFC-0001 §5.4 Gate 4 fail-closed conformance requirement**: Nova non-negotiable #2, verified against RFC-0001 v0.2.0 tag.
- **WARD-C1–C6 lineage**: `research/synthesis/autocompacting-and-the-protected-surface-2026-07-03.md` §6; cross-linked to `coven-grimoire` PR #3.
- **`.af` non-adoption**: source-verified 2026-07-14 against `letta-ai/letta/main/letta/serialize_schemas/pydantic_agent_schema.py`. `CoreMemoryBlockSchema` has no protection field; runtime `read_only` stripped at export.
- **Existing daemon runtime state**: `~/.coven/coven.sock`, `~/.coven/coven.sqlite3`, `~/.coven/memory/archival.sqlite3`.

---

_"The Rust daemon is the authority boundary. Every client is untrusted for enforcement purposes." — `coven/docs/SAFETY-MODEL.md`. RFC-0001 §5.1 makes this a conformance requirement. `coven-threads` is the gate-shaped receiver behind that boundary — reachable only by the privileged daemon, never by a familiar-controlled process._
