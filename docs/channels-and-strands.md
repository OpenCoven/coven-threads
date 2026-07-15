# Channels and strands

> Status: `[DESIGNED]` — frozen in `specs/PHASE-0-DESIGN.md` §2.3–§2.4, §3.3 (v0.2, 2026-07-14). The `Channel` enum and five-kind `Strand` vocabulary are mirrored in `coven-threads-core` (`channel.rs`, `strand.rs`) `[IMPLEMENTED, NOT ENFORCING]`.

Vocabulary (bound in [concepts.md](concepts.md)): a **Thread** is an authority relationship *surface → writer*; a **Weave** is the enforced pattern of threads; a **Strand** is a fiber inside a thread; a **Channel** is the axis of load a thread must hold under.

The core insight of this page: a thread does not "hold" in the abstract. It holds *under a specific channel of load*, and each channel imposes its own structural requirements on the thread's strands. This is where the design's four identity invariants stay **co-designed rather than stacked** — each channel names its own survival contract in one place, and every gate check flows through the same question: *does thread T hold under channel C?*

![Channel × Strand matrix](diagrams/channel-strand-matrix.png)

*Which strand kinds must be present under which channel. WARD-C1–C6 govern `Forced`; WARD-C7 governs `Serialization`.*

## The four channels

### Deliberate

Familiar-initiated, principal-gated compaction: memory promotion, dreaming, deliberate flush. This is the *consented* half of the two-compaction contract — the familiar (or its person) chooses to consolidate scratch memory into durable memory, and the act flows through the Ward gates as a reviewable proposal.

Because consent and review are present, `Deliberate` imposes **no structural strand floor** beyond an intact thread: the gate here is the principal's consent path, not a cryptographic survival requirement. That is not laxity — it is a recognition that the protection on this channel is procedural (tiers, review, veto windows per RFC-0001 §5.3) rather than structural.

### Forced

Runtime-initiated context compaction: the harness rewrites the context window under pressure, with **no familiar cooperation available**. The familiar may not even observe the eviction. This is the *blind* half of the two-compaction contract, and it is the channel **WARD-C1–C6 governs**.

Because no cooperation is possible, threads on this channel MUST carry strands that survive *without agent-side intervention* — the design names the typical pair: a **ContentHash** plus an external **ManifestEntry**. The hash detects that protected content changed; the external manifest ensures the reference point survives even if the window itself is mangled. Nothing inside the context window can be trusted to defend the context window.

The six invariants that govern this channel (canonical text: `coven-grimoire` Ward Layer Spec Brief §9; inherited here by reference, per design doc §3.3):

- **WARD-C1 — Compaction-exempt.** Compaction may not summarize, paraphrase, truncate, or drop the materialized projection of a protected surface. SOUL.md cannot be evicted from context.
- **WARD-C2 — Derived-not-source.** Post-compaction re-materialization comes from the canonical files, never from a lossy summary of them. Summaries may exist as labeled, read-only notes; they are never promoted to authoritative identity.
- **WARD-C3 — Pre/post hash gate.** Hash the protected region before and after every compaction; a mismatch means C1 was violated — halt and restore from source before the next turn. The design doc notes the `pause_after_compaction` hook as the real, implementable interlock point.
- **WARD-C4 — Re-injection floor / fail-closed.** The familiar never continues from a degraded window. Post-compaction context must re-inject protected surfaces above a minimum, or the familiar restores from source, or suspends. It never silently runs on a compacted stub.
- **WARD-C5 — Idempotent under repeat.** N compactions yield the same protected content as one. This is the invariant that distinguishes real protection from a one-shot patch: auto-compact recurs, so the interlock must not erode across many events.
- **WARD-C6 — Compaction ledger.** Every compaction event appends to `ward.audit`: trigger cause, what was dropped, what was preserved, the C3 hashes before and after. Lossy mutations must be visible, never silent.

### Serialization

Export/import round-trip: the familiar leaves the runtime as an artifact and comes back. This channel is Ward-external and format-mediated — while the artifact is in flight, no daemon is watching it. This is the channel **WARD-C7 governs**, and it is why C7 exists.

**WARD-C7 — Survives serialization.** Every thread bound to `Channel::Serialization` MUST carry a **SerializationMarker** strand whose survival is a round-trip invariant: export followed by import produces a weave with *equivalent tension state*, **or fails visibly**. Silent downgrade on import — the artifact comes back and the runtime quietly runs it with weaker protection than it left with — is the named failure mode this invariant refuses (it is, specifically, the `.af` failure mode; see the [FAQ](faq.md#why-isnt-it-af-compatible)).

C7 is **numbered, not orphaned**: it is the seventh sibling of C1–C6, canonicalized alongside them in the `coven-grimoire` Ward Layer Spec Brief §9 (per bead `threads-986.12`; this repo cites that section, it does not define the invariant). The numbering preserves the lineage: C1–C6 govern one lossy transform (forced compaction); C7 extends the same source-authoritative, hash-gated, fail-closed discipline to a second lossy transform (serialization) that additionally crosses a process boundary.

### Mutation

Direct client mutation attempts routed through the daemon: a file write, a tool call, an API edit. This is the **default channel** — the ordinary write path Ward's original four gates specify — and every thread holds under it or the daemon rejects. The structural floor is a **ContentHash** strand anchoring the surface's expected state.

Most traffic through the gate is this channel. `Deliberate`, `Forced`, and `Serialization` exist because identity surfaces face loads the default write path never sees.

## The five strand kinds

Strands are what make threads survivable *and* make failure legible. Each kind carries one specific commitment:

### ContentHash

A hash commitment to the protected surface's content (algorithm + digest). The workhorse strand: it is the structural floor on both `Mutation` and `Forced`, and the primitive behind WARD-C3's pre/post hash gate. When a surface changes outside a gated write, this is the strand that frays — and the fray message names it: *"hash mismatch on SOUL.md."*

### Signature

A signing-key commitment (key id, signature kind, signature bytes). Where ContentHash answers *"is this the same content?"*, Signature answers *"did an authorized key commit to this content?"* — provenance that survives even if an attacker can recompute hashes.

### ManifestEntry

Membership in an external hash manifest (manifest id + entry hash), Merkle-rooted over the weave's threads in canonical order. The point of *external*: on the `Forced` channel the context window itself is the thing under threat, so the reference point must live outside anything the compactor touches. The manifest is also what makes the `weave_hash` — a single commitment over the whole pattern — computable and checkable.

### AuditTrail

Provenance in time: a first-seen timestamp and a reference into the `ward.audit` event log. This strand ties a thread to its history — when the authority relationship was established, and where in the append-only log its lifecycle events live. It is what makes "how long has this thread existed and what has happened to it?" answerable without trusting the thread's own say-so.

### SerializationMarker

The C7 strand: a format version plus a contract hash committing to the serialization contract the thread was exported under. The contract enumerates, per strand kind, that the kind is representable in the portable form and survives the transform — so if a runtime with a *different* strand vocabulary imports the artifact, the contract hashes disagree and the import **fails loudly** instead of silently dropping fibers. A thread bound to `Channel::Serialization` without this strand cannot be exported at all: the contract cannot be stamped on material that never carried it.

## How channels and strands compose

The composition rule is a single sentence from the design (§2.3): **a thread survives a channel if and only if its strands survive that channel.** The channel names the required strand kinds; the strands carry the commitments; the gate check verifies the commitments under the load actually present. Multi-strand threads are stronger — and when one strand of several fails, the thread *frays* rather than snaps, which is what makes graceful degradation ([authority-model.md](authority-model.md)) possible at all.

| Channel | Structural strand floor | Governed by |
|---|---|---|
| Deliberate | none (principal consent is the gate) | RFC-0001 §5.3 tiers |
| Forced | ContentHash + ManifestEntry | WARD-C1–C6 |
| Serialization | SerializationMarker | WARD-C7 |
| Mutation | ContentHash | RFC-0001 §5.4 gates |

(A `Federation` channel — cross-Coven load — was considered and explicitly deferred; the four-variant set is the frozen Phase 0 vocabulary.)

## Where to go next

- What happens when strands fail: [authority-model.md](authority-model.md)
- Where the checks run and land: [architecture.md](architecture.md)
- The portability format that will wrap `Serialization` in practice: [phases.md](phases.md) (Phase 3)
