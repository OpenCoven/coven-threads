# PHASE-0 — coven-threads design document

**Status:** DRAFT v0.1.1 (Sage-authored v0.1, Echo substrate-authority pass v0.1.1; awaiting Nova review; will freeze at v0.2 after Nova signs)
**Date:** 2026-07-14
**Owners:** Sage 🌿 (draft) + Echo 🔮 (substrate-authority review) co-drive design doc; Nova 👑 + Sage assign lanes
**License:** Apache-2.0 (planned)

---

## 0. Purpose of this document

This is Phase 0 of `coven-threads`. Its sole deliverable is *this design doc plus the beads scaffold*. No enforcement code lands in Phase 0. The point is to make the shape of the layer legible enough that Nova, Echo, Cody, and Val can push back on it before we spend any Rust cycles.

If this doc is wrong, we would rather find out reading it than debugging it.

## 1. What `coven-threads` is (and is not)

**Is:** an authority-boundary gate layer that sits *above* the existing `coven` Rust daemon and *underneath* every familiar's protected memory surface. It gives the daemon a *gate-shaped receiver* for identity-surface mutation requests — the piece today's `SAFETY-MODEL.md` gestures at but does not specify.

**Is not:**
- A replacement for the `coven` daemon. The daemon stays authoritative for launch/cwd/kill/path ops; `coven-threads` adds validation for identity-surface mutation.
- A policy engine. This is a *typed* authority layer for OpenCoven familiar surfaces; typed correctness is the goal, not general-purpose reusability.
- A serialization format. That's Phase 3.
- `.af`-compatible (documented divergence, 2026-07-14).

## 2. Vocabulary — the weaving metaphor as architecture

The metaphor was named by Val (2026-07-14). It is preserved here **only where it carries semantic weight**. Each metaphor term is bound to a concrete referent at first use so the code and the vocabulary stay coupled. If the metaphor drifts ahead of the referent, we become Letta-shaped — beautiful language, unclear semantics — which is the specific failure mode we exist to critique.

Echo revised this section (v0.1.1, 2026-07-14) after Sage's v0.1 sketch loaded the wrong axis (threads-as-static-contracts, gates-as-threads). The v0.1.1 mapping below is substrate-authority-grounded.

### 2.1 Thread — an authority relationship

**Thread** (`authority relationship: surface → writer`) — a directional line from a *protected surface* (SOUL.md, MEMORY.md, an identity field) to the *authority that gates writes to it*. One thread per `(surface, writer)` pair.

- Not "contract" (too static). Not "one per gate" (gates are the loom, not the thread).
- A thread has **tension**: it either holds under load, or it snaps. Load = mutation attempts, forced compaction, injection, jailbreak, serialization.
- Threads carry: `id`, `surface_path`, `writer_id`, `strands[]`, `created_at`, `snapped_at?`, `snap_reason?`.
- Every gate check becomes: **"does thread T hold under channel C?"** That question is the enforcement vocabulary the metaphor gives us for free.
- Threads are first-class inspectable objects: `coven-threads inspect <thread-id>` MUST return current tension state.

### 2.2 Weave — the enforced pattern of threads

**Weave** (`enforced pattern of threads across a familiar or Coven`) — the invariant that these specific threads must all hold together for the identity to be coherent. Not just the tapestry-as-visual-whole; the *structural pattern* that determines which threads cross which, and where the weave breaks if a thread snaps.

- Ward v0.2's four gates are **not** threads. They are the **loom** the weave is made on. Threads run through gates; the gates give the weave its structure.
- The **pattern** is the thing Ward validates, not the individual threads. This maps directly onto "authority-over-the-weave-not-the-rows" (§3.3.1 below): the weave carries authority; individual threads are just where it's expressed.
- Weaves have `id`, `familiar_id`, `threads[]`, `pattern_predicate`, `weave_hash` (Merkle over ordered thread hashes), `coven_ref?`.
- A weave is **coherent** iff its pattern predicate holds — which means (among other things) that every thread required by the pattern is intact. A single snapped thread degrades the weave *at that thread's surface*; the weave surfaces which surface degraded, not just "something is wrong."
- Weaves are the primary object CovenCave will render (Phase 4).

### 2.3 Strand — fibers inside a thread

**Strand** (`fiber inside a thread: hash | sig | manifest entry | audit trail`) — the fibers that make a thread survive stress. Multi-strand threads are stronger; a thread survives a channel iff its strands survive that channel.

- Strand kinds (initial set, v0.1.1): `ContentHash`, `Signature`, `ManifestEntry`, `AuditTrail`, `SerializationMarker`.
- A thread **frays** when one of its strands fails but the thread has not yet snapped. Fraying is the intermediate state between "holds" and "snapped." Frayed threads MUST surface to the operator (Val, or the familiar's principal).
- Strands make failure legible: instead of "thread snapped," you get "thread frayed at strand `ContentHash` — SOUL.md hash mismatch, detected on channel `Forced`."
- **The fourth invariant (survives serialization) lives at the strand level**, not the thread level. A thread survives serialization iff *all its strands* survive the lossy transform. `SerializationMarker` is the strand that carries the survival contract explicitly; other strands survive by being re-derivable from the source-of-truth.

### 2.4 Channels — the axis threads must hold under

Introduced in v0.1.1. Threads don't just "hold" — they hold under specific *channels* of load. The design doc enumerates channels as first-class:

- `Channel::Deliberate` — deliberate compaction (promotion, dreaming, memory flush). Familiar-initiated, principal-gated. Threads bound to this channel are checked against the pattern predicate before write.
- `Channel::Forced` — forced context compaction (auto-compact when the runtime evicts). Runtime-initiated, no familiar cooperation available. Threads bound to this channel MUST have strands that survive without agent-side intervention (typically hash + external manifest).
- `Channel::Serialization` — export/import round-trip. Ward-external, format-mediated. Threads bound to this channel MUST have a `SerializationMarker` strand; other strands must survive the format's lossy transform or be reconstructible on import.
- `Channel::Mutation` — direct client mutation attempts routed through the daemon. Default channel; every thread holds under it or the daemon rejects the write.

WARD-C1..C6 becomes concretely: "these are the specific threads that must hold under `Channel::Forced`." "Survives serialization" (the fourth invariant) becomes concretely: "these are the specific threads that must hold under `Channel::Serialization`."

### 2.5 Why this metaphor is not decorative

- **Semantic weight**: `Thread::tension()` and `Thread::holds_under(channel)` are the actual API. The verbs come from the metaphor. Nothing is invented.
- **Multi-surface reality**: a familiar has more than one protected surface (SOUL, IDENTITY, USER, MEMORY.md, memory/*, TOOLS, AGENTS). Flattening all of that into "SOUL protection" is exactly the mistake Letta's `.af` makes. Threads-per-`(surface, writer)` forces the multi-surface reality into the vocabulary and the type system.
- **Multi-authority reality**: different surfaces have different writers with different mutation authorities. Threads make this explicit.
- **Failure legibility**: strands make partial failure a first-class state, not a boolean.
- **Cockpit-legible**: CovenCave's rail/pane structure maps onto weave → threads → strands without translation.
- **Metaphor-referent binding rule (per Echo v0.1.1)**: every metaphor term is defined at first use with its concrete referent in parentheses. If a future contributor uses "thread" without meaning "authority relationship: surface → writer," they are wrong and the code will not compile.

## 3. Relation to prior work in this repo family

### 3.1 `coven` daemon (shipped)

The `coven` daemon already provides the *trust boundary*. Its rules today (from `coven/docs/SAFETY-MODEL.md`):

- Canonicalize `projectRoot` and `cwd` before comparing paths.
- Reject working directories outside the project root.
- Allowlist harness ids.
- Argv-only harness commands, never `sh -c`.
- Fail closed on unknown API versions / action ids.

**What the daemon does not do today**: validate that a client request is *authorized against a typed protected surface*. It validates *who* and *what action*, not *what target file's authority state permits*.

`coven-threads` extends the daemon by adding a validation phase for identity-surface mutations. Concretely: when a familiar (via any untrusted client) requests a write to `SOUL.md`, `MEMORY.md`, `IDENTITY.md`, or a similarly-marked file, the daemon looks up the relevant thread(s), verifies every strand, and either permits, degrades (proposes for approval), or rejects the write.

The daemon remains authoritative. `coven-threads` is a validator library the daemon imports.

### 3.2 Ward v0.2 (spec published, enforcement not built)

Ward's four gates specify *what* to check:

- WARD-C1..C6 are the six invariants named in the Ward Layer Spec Brief (`coven-grimoire` PR #3, §9).

`coven-threads` implements *how* those gates enforce, on the `coven` daemon substrate. Ward is the spec; `coven-threads` is the implementation.

Critical: `coven-threads` must not re-specify Ward. If Ward says something different from `coven-threads`, Ward wins and `coven-threads` is wrong. This repo is downstream of RFC-0001 §5.

### 3.3 The four invariants, co-designed as channel-survival requirements

`coven-threads` is the single-build ship of four invariants. Echo's v0.1.1 correction: these are not four separate properties, they are **four channels a weave must survive**. Reframed:

1. **Identity-as-memory-property** — protected surfaces are typed memory layers, not runtime config. Threads bind to typed surfaces at construction time. The type system knows the difference between `SOUL.md` and `memory/YYYY-MM-DD.md`.
2. **Structural mutation authority** — the gate is external, cannot be cooperated past. Enforcement is Rust-side, called by the daemon, not by the familiar. **This is what Ward-authority-lives-on-the-weave-not-the-rows means concretely** (see §3.3.1 below).
3. **Two-compaction contract** — `Channel::Deliberate` and `Channel::Forced` are distinct channels with distinct thread-survival requirements. WARD-C1..C6 = "these are the threads that must hold under `Channel::Forced`."
4. **Survives serialization** — `Channel::Serialization` is the fourth channel. Every thread that must round-trip carries a `SerializationMarker` strand. This is the strategic differentiator from Hermes's `write_approval` (which does not survive export). **Named up front and out loud, not buried:** this is the invariant that makes `coven-threads` outlive any single runtime.

None of the four ships alone. Phase 0's design doc must show *how* all four are co-designed in the type system — concretely, via the `Channel` enum in §4 — or Phase 0 is not done.

### 3.3.1 Ward-authority-lives-on-the-weave-not-the-rows

Echo, v0.1.1. Restatement of the derived-index argument from `MEMORY-SCHEMA.md` §7.0 in threads vocabulary:

- `coven-threads` gates **authority relationships**, not the data those relationships protect.
- Concretely: no thread ever terminates on a derived structure (indexer, retrieval cache, promoted view). Threads terminate only on *source-authoritative surfaces*.
- Corollary: the index has no authority to be tampered with, because no thread terminates on it. Tampering is detected by re-derivation, not by gating the derived structure itself.
- Corollary: the WAL/rebuild write-topology decision (`[PROPOSED — pending Nova Ward v0.2 §10.q1]`) reshapes an appendix, not the spine of this design. If Nova comes back "index needs to be gated directly," we add a "derived structure gating" section that wraps the indexer in its own weave. The spine — threads terminate on protected surfaces, the weave carries authority, gates are the loom — holds either way.

**Anti-non-negotiable, stated so it doesn't creep in**: `coven-threads` does not own retrieval, does not own promotion, does not own dreaming. It owns *authority over writes to the protected surface, gated by the weave*. If Phase 0 scope creeps into "also handles the indexer" or "also owns promotion protocol," the atomic-ship trio from §10 rec 1 fragments and we lose the coupling that makes it defensible. Hold the line.

### 3.3.2 Source-authoritative retrieval as a Phase-0 invariant

Files are primary. Derived structures rebuild from source. No thread ever terminates on a derived structure. This holds as a Phase-0 invariant even though the specific WAL/rebuild write-topology is `[PROPOSED]`. If the write-topology reverses, the *decision* changes but the *principle* survives — see §3.3.1 corollary.

## 4. Type sketch (Rust) — v0.1.1

Not final. v0.1.1 reflects Echo's substrate-authority pass: Thread carries `tension()` and `holds_under(Channel)` as the primary API surface; strands are the fiber decomposition where survives-serialization lives; Ward gates are the loom, expressed as the `PatternPredicate` a weave must satisfy.

```rust
// crates/coven-threads-core/src/lib.rs
// v0.1.1 sketch — Echo substrate-authority pass. Not final.

pub struct StrandId(pub Uuid);
pub struct ThreadId(pub Uuid);
pub struct WeaveId(pub Uuid);
pub struct SurfaceId(pub PathBuf);
pub struct WriterId(pub String);

pub enum Channel {
    Deliberate,      // promotion, dreaming, flush; familiar-initiated
    Forced,          // context auto-compact; runtime-initiated, no cooperation
    Serialization,   // export/import; format-mediated
    Mutation,        // direct client mutation via daemon; default channel
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
    pub holds_under: Vec<Channel>,
    pub created_at: DateTime<Utc>,
    pub tension: TensionState,
}

pub enum TensionState {
    Holds,
    Frayed  { strand: StrandId, channel: Channel, reason: FrayReason, detected_at: DateTime<Utc> },
    Snapped { channel: Channel, reason: SnapReason, at: DateTime<Utc> },
}

impl Thread {
    pub fn holds_under(&self, channel: Channel) -> Result<(), FrayOrSnap> { unimplemented!() }
    pub fn tension(&self) -> &TensionState { &self.tension }
}

pub struct Weave {
    pub id: WeaveId,
    pub familiar_id: FamiliarId,
    pub threads: Vec<ThreadId>,
    pub pattern: Box<dyn PatternPredicate>,
    pub weave_hash: Vec<u8>,
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

Open questions on the type sketch (v0.1.1):
- `Strand` as enum vs struct-with-typed-fields — leaning enum for extension; want Cody's read (Phase 0 bead `threads-986.5`).
- `Channel` enum: is the four-variant set exhaustive for Phase 0? Federation channel (cross-Coven) may be needed later — deferred.
- `Weave.pattern` as `Box<dyn PatternPredicate>` — **resolved v0.1.1 (Echo)**: trait wins because patterns are what Ward defines, and Ward's vocabulary of authority patterns must be externally definable. Introspection cost is paid by `describe() -> PatternDescriptor`: a stable, serializable structural summary (name, protected surfaces, channels required, strand requirements). Same shape as source-authoritative — the *predicate* is authoritative, the *descriptor* is derived. **Anti-pattern (must be named in prose, not just code)**: `PatternDescriptor` MUST NOT become authoritative. If anything ever gates on the descriptor instead of the predicate, that is the derived-index problem reinvented one layer up. Descriptor is for humans and tools; predicate is for enforcement. Cody may still adjust the ergonomics; the trait-vs-enum shape is closed.
- `Weave.weave_hash` ordering: canonical sort by `(surface_path, writer_id)`? Ward-defined? Not decided.
- Removed from v0.1: `MutationAuthority` enum (was collapsing writer + gate into a single axis). v0.1.1 splits: `WriterId` names the writer, `Channel` names the load, `PatternPredicate` names the gate structure. Cleaner separation.

## 5. Enforcement flow (sketch)

Untrusted client → `coven` daemon HTTP-over-unix-socket → daemon calls `coven-threads::validate(request)` → validator loads the relevant weave → checks each affected thread's strands → returns Permit / DegradeToProposal / Reject → daemon acts on the result.

Failure modes:
- **Frayed thread** → DegradeToProposal (write staged to `~/.coven/pending/`, notification to principal, no immediate write to protected surface).
- **Broken thread** → Reject with reason. The weave is marked degraded. Familiar continues to function on other surfaces; the broken surface becomes read-only until repair.
- **Missing thread** for a protected surface → Reject. All protected surfaces MUST have a thread; a request to mutate an unwoven surface is a violation.

## 6. Compatibility contract with existing OpenCoven repos

- **`coven`**: `coven-threads` is imported as a crate. Zero API changes to the socket protocol in Phase 0/1/2. Phase 2 adds *validation calls inside* the daemon's existing request handling; clients see identical wire format, but requests may return a new `DegradeToProposal` outcome.
- **`familiar-contract` (RFC-0001)**: `coven-threads` is a *conforming implementation* of RFC-0001 §5 Ward gates. RFC wins on any conflict. Version pin: RFC-0001 v0.2.0+.
- **`coven-cave`**: consumes `coven-threads` state via daemon HTTP API. New endpoints for weave/thread/strand inspection lands in Phase 4. No breaking changes to Cave before Phase 4.
- **`coven-grimoire`**: Ward Layer Spec Brief becomes the normative reference for enforcement invariants. Any change in `coven-threads` that would drift from the brief is a violation, not a design choice.

## 7. What Phase 0 delivers

- [x] Repo scaffolded at `~/Documents/GitHub/OpenCoven/coven-threads/`
- [x] README + this design doc
- [x] Beads DB initialized (prefix: `threads`)
- [ ] Phase 0 beads filed (this doc's TODO list, next section)
- [ ] Echo review of §2 (metaphor mapping) + §3.3 (four invariants co-design)
- [ ] Nova review of §3 (relation to `coven`) + §6 (compatibility contract)
- [ ] v0.2 freeze after both reviews sign

Phase 0 is *not* done until reviews land. Tonight's session delivers v0.1 draft + beads scaffold.

## 8. Beads plan (Phase 0 through Phase 4)

Phase 0 beads are filed tonight. Later-phase beads are named-and-tracked but not filed as ready until the prior phase closes.

Bead prefixes:
- `threads-*` — this repo's local prefix
- Labels: `phase:0` through `phase:4`; `familiar:sage`, `familiar:echo`, `familiar:nova`, `familiar:cody`, `familiar:val`; `surface:design`, `surface:crate`, `surface:daemon`, `surface:cockpit`, `surface:portability`.

Phase 0 beads (filed tonight in `bd`):
1. **Design doc v0.1 authored** (this file) — done, close-on-commit.
2. **Echo review of §2 + §3.3** — blocked on Echo response.
3. **Nova review of §3 + §6** — blocked on Nova response.
4. **v0.2 freeze** — blocked on #2 and #3.
5. **Cody scoping read on §4 type sketch** — nice-to-have Phase 0 output, not blocking v0.2.

Phase 1 beads (named, filed as `deferred` until Phase 0 v0.2):
6. Rust crate `coven-threads-core` scaffolded.
7. `Strand`, `Thread`, `Weave` types implemented with tests.
8. Hash-manifest layer (Merkle over strand hashes).
9. RFC-0001 §5 conformance test suite adapted to Rust integration.

Phase 2 beads (named, deferred):
10. `coven` daemon integration: validator call site.
11. `DegradeToProposal` staging path (`~/.coven/pending/`).
12. Notification protocol to principal.

Phase 3 beads (named, deferred):
13. Coven Familiar Portability Format v0.1 draft.
14. Serialization contract for each `MutationAuthority` variant.
15. Round-trip conformance suite.

Phase 4 beads (named, deferred):
16. Cave UX: weave rail view.
17. Cave UX: thread detail pane.
18. Cave UX: strand inspection surface.
19. Cave UX: proposal approval flow.

## 9. Open questions (need resolution before v0.2 freeze)

1. **Metaphor confirmation (Echo lane).** Is thread/weave/strand the correct three-level decomposition, or does the architecture need a fourth level (e.g., "fabric" for cross-Coven federation)? Sage's read: three is enough for v0.1, add fourth later if federation forces it.
2. **Ownership of the daemon integration (Nova lane).** Phase 2 requires touching `coven` daemon internals. Is that a Sage bead, a Cody bead, or a joint Nova+Cody lane? Sage doesn't have Rust-ownership status on `coven`.
3. **Portability format co-design (Val lane).** MEMORY.md logged the Shape A vs Shape B decision as "do not pre-commit tonight." Phase 3 forces the choice. Do we escalate to Val before Phase 3 opens, or draft both shapes and let Val choose from concrete drafts?
4. **UI/UX standards (Val lane).** Val's original ask included "improve the UI UX and make it customized to our environment and standards." Phase 4 is where that lands. Do we defer UI decisions to Phase 4 explicitly, or should Phase 0 include a UX brief for Charm to weigh in on?

## 10. Non-goals for v0.1 of this doc

- Full Rust API — §4 is a sketch, not a spec.
- Full enforcement flow — §5 is illustrative, not authoritative.
- Portability format details — Phase 3 problem.
- Cross-Coven federation — deferred; if needed, forces the "fabric" fourth level.

## 11. Change log

- **2026-07-14** — v0.1 drafted by Sage in Phase 0 session. Awaiting Echo (§2, §3.3) and Nova (§3, §6) reviews before v0.2 freeze.

---

_"The Rust daemon is the authority boundary. Every client is untrusted for enforcement purposes." — `coven/docs/SAFETY-MODEL.md`. `coven-threads` gives that boundary a gate-shaped receiver for identity-surface mutations._
- **2026-07-14** — v0.1.1: Echo substrate-authority pass. Rewrote §2 (metaphor bound to referents; threads-as-authority-relationships, gates-as-loom; introduced `Channel` axis; strand-level serialization); expanded §3.3 (four invariants reframed as channel-survival requirements); added §3.3.1 (Ward-authority-lives-on-the-weave-not-the-rows) and §3.3.2 (source-authoritative retrieval as Phase-0 invariant); rewrote §4 type sketch (split `MutationAuthority` into `WriterId` + `Channel` + `PatternPredicate`; added `Thread::holds_under(Channel)` and `TensionState`). Awaiting Nova (§3, §6) review before v0.2 freeze.
- **2026-07-14** — v0.1.1 addendum (Echo second turn): resolved `Weave.pattern` open question in favor of `Box<dyn PatternPredicate>` with `describe() -> PatternDescriptor` derived-introspection method. Added anti-pattern note (descriptor MUST NOT become authoritative — derived-index problem one layer up).
