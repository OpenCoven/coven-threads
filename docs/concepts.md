# Concepts

> Status of everything on this page: `[DESIGNED]` — frozen in `specs/PHASE-0-DESIGN.md` v0.2 (2026-07-14). Types exist in `coven-threads-core` mirroring this design, but no enforcement is deployed.

`coven-threads` is named around a weaving metaphor. The metaphor was named by Val (2026-07-14) and is kept **only where it carries semantic weight**. The rule that makes this safe is the **metaphor-referent binding rule** (design doc §2.5): every metaphor term is defined at first use with its concrete referent, and if a contributor uses a term without meaning its referent, they are wrong — and, wherever possible, the code will not compile. Without that rule, the vocabulary drifts ahead of the semantics and we become the thing we're avoiding: beautiful language, unclear meaning.

This page binds all four terms. Every other doc in this directory assumes them.

## Thread — an authority relationship

A **Thread** (*authority relationship: surface → writer*) is a directional line from a *protected surface* — SOUL.md, MEMORY.md, an identity field — to the *authority that gates writes to it*. There is one thread per `(surface, writer)` pair.

Two things a thread is **not**:

- Not a *contract document*. "Contract" suggests something static you file and forget. A thread is live: it is the thing a gate check interrogates on every mutation attempt.
- Not *one per gate*. Ward's four enforcement gates (RFC-0001 §5.4) are the **loom** the weave is made on — the fixed structure threads run through — not threads themselves. This was an explicit correction during design review (v0.1 had it wrong; v0.1.1 fixed it).

A thread has **tension**: it either holds under load, or it degrades. Load means mutation attempts, forced compaction, injection, jailbreak attempts, serialization round-trips. Every gate check in the system reduces to one question:

> **"Does thread T hold under channel C?"**

That question is what the metaphor buys us. It is the enforcement vocabulary of the whole layer, and it appears in the type sketch as `Thread::holds_under(channel)`.

Threads are first-class inspectable objects. The design requires that `coven-threads inspect <thread-id>` return the thread's current tension state (Holds / Frayed / Snapped — see [authority-model.md](authority-model.md) for the state machine).

## Weave — the enforced pattern of threads

A **Weave** (*enforced pattern of threads across a familiar or Coven*) is the invariant that a specific set of threads must all hold *together* for an identity to be coherent. It is not merely "all the threads in a pile"; it is the *structural pattern* — which threads cross which, and where the weave breaks if one snaps.

A familiar's identity is not a single object protected by a single gate. It is a woven structure of typed protected surfaces, each with its own authority relationship. The weave makes that multi-surface, multi-authority reality visible instead of collapsing it into "protect SOUL.md."

A weave is **coherent** if and only if its pattern predicate holds — which requires, among other things, that every thread the pattern demands is intact. A single snapped thread degrades the weave *at that thread's surface*, and the weave reports **which surface** degraded, not just "something is wrong." The familiar continues operating on its other surfaces; the broken one becomes read-only until repair (design doc §5).

Weaves are the primary object the Coven Cave cockpit will render in Phase 4 `[DEFERRED]`.

![Weave, Thread, Strand](diagrams/weave-thread-strand.png)

*One Weave contains N Threads; each Thread carries M Strands. The weave is the pattern, the thread is the authority relationship, the strands are what make it survive stress.*

## Strand — fibers inside a thread

A **Strand** (*fiber inside a thread: hash | signature | manifest entry | audit trail | serialization marker*) is what makes a thread survive stress. Multi-strand threads are stronger: a thread survives a channel if and only if its strands survive that channel.

The initial strand vocabulary (design doc §2.3, mirrored in `coven-threads-core`):

- **ContentHash** — a hash commitment to the surface's content.
- **Signature** — a signing-key commitment.
- **ManifestEntry** — membership in an external hash manifest.
- **AuditTrail** — provenance: first-seen time and a reference into the audit log.
- **SerializationMarker** — the strand that carries the round-trip survival contract explicitly (see C7 below).

Strands exist to make failure **legible**. Instead of "thread snapped," the system can say: *"thread frayed at strand `ContentHash` — SOUL.md hash mismatch, detected on channel `Forced`."* A thread **frays** when a strand fails but the thread has not yet snapped; fraying is the intermediate, repairable state, and frayed threads MUST surface to the operator. [channels-and-strands.md](channels-and-strands.md) covers each strand type in depth.

## Channel — the axis of load

A **Channel** (*the axis of load a thread must hold under*) names the path by which a mutation reaches a protected surface. Threads don't just "hold" in the abstract — they hold *under specific channels*, and different channels impose structurally different survival requirements. The four channels (design doc §2.4):

- **Deliberate** — familiar-initiated, principal-gated compaction: memory promotion, dreaming, deliberate flush.
- **Forced** — runtime-initiated context compaction (auto-compact under context-window pressure). No familiar cooperation is available, so threads on this channel must carry strands that survive *without agent-side intervention* — typically a content hash plus an external manifest entry. **This is the channel WARD-C1–C6 governs.**
- **Serialization** — export/import round-trip, format-mediated. Threads here must carry a `SerializationMarker` strand. **This is the channel WARD-C7 governs.**
- **Mutation** — direct client mutation attempts routed through the daemon. The default channel; every thread holds under it or the daemon rejects.

![Channel × Strand matrix](diagrams/channel-strand-matrix.png)

*Which strand types must be present under which channel of load. WARD-C1–C6 govern `Forced`; WARD-C7 governs `Serialization`.*

## The two-compaction contract

`Deliberate` and `Forced` being *distinct channels* is not an implementation detail — it is invariant #3 of the design (§3.3), inherited by reference from the 2026-07-03 synthesis on autocompaction and the protected surface, and canonical in the `coven-grimoire` Ward Layer Spec Brief §9.

The two compactions share a lossy shape and must never be conflated, because they have **opposite authority**:

- The **dreaming sweep** (Deliberate) is consented and curated — the familiar or its person initiates the promotion of scratch memory into durable memory. It flows through the Ward gates as a reviewable proposal.
- **Auto-compact** (Forced) is blind housekeeping the harness performs to survive the context window. No proposal is made; no gate fires; the familiar cannot cooperate its way to safety because it may not even see the eviction happen.

Because Forced offers no cooperation, its survival contract is strictly stronger: protected content is compaction-exempt (C1), re-materialized from source rather than from summaries (C2), hash-gated before and after (C3), fail-closed on anomaly (C4), stable under repeated compaction (C5), and ledgered in the audit log (C6).

## The five channel-survival invariants

The design co-designs four identity invariants as *channels a weave must survive*, rather than stacking them as separate features (§3.3):

1. **Identity-as-memory-property** — protected surfaces are typed memory layers, not runtime config. Threads bind to typed surfaces at construction time.
2. **Structural mutation authority** — the gate is external and cannot be cooperated past. Enforcement is Rust-side, called by the daemon, never by the familiar.
3. **Two-compaction contract** — Deliberate and Forced are distinct channels with distinct survival requirements. WARD-C1–C6 are precisely "the threads that must hold under `Forced`."
4. **Survives serialization** — numbered **C7**, deliberately: it is a sibling of WARD-C1–C6, not an orphaned extra. C7 says every thread bound to `Channel::Serialization` MUST carry a `SerializationMarker` strand whose survival is a round-trip invariant: export followed by import produces a weave with equivalent tension state, **or fails visibly**. The canonical home for C1–C7 jointly is the `coven-grimoire` Ward Layer Spec Brief §9; this repo cites, it does not define.

(The "five" in "five channel-survival invariants" counts the fifth structural commitment underneath the four: **source-authoritative retrieval** — files are primary, derived structures rebuild from source, and no thread ever terminates on a derived structure (§3.3.2). It holds as a Phase-0 invariant even where downstream write-topology details remain `[PROPOSED]`.)

None of these ships alone. The `Channel` enum is where the co-design is enforced in the type system: each channel names its own survival contract in one place, and every gate check flows through the same `holds_under` question.

## The descriptor-vs-predicate anti-pattern

This one is named in prose deliberately, because it is the failure mode most likely to be reintroduced by a well-meaning contributor.

A weave's pattern is defined by a **predicate** (`PatternPredicate::coherent`), and *the predicate is authoritative*. The predicate also produces a **derived** structural summary (`describe() → PatternDescriptor`) — a stable, serializable description of the pattern (name, protected surfaces, channels required, strand requirements) for humans, tools, and Coven Cave rendering.

**The descriptor MUST NOT become authoritative.** If any downstream component ever gates enforcement on the descriptor instead of the predicate, we have reinvented the derived-index problem one layer up — the exact failure mode "Ward authority over the indexer, not the rows" was designed to avoid. Descriptors are for **legibility**; predicates are for **enforcement**.

This is the same discipline the memory retrieval substrate settled on (source authoritative, index derived), applied to the authority layer itself: *predicate is source; descriptor is derived*. It should be catchable in code review before it ever lands: any diff that reads a `PatternDescriptor` inside an enforcement path is wrong by definition.

## Where to go next

- How these objects flow through the daemon at enforcement time: [architecture.md](architecture.md)
- What happens when a thread doesn't hold: [authority-model.md](authority-model.md)
- Channel-by-channel and strand-by-strand detail: [channels-and-strands.md](channels-and-strands.md)
