# Authority model

> Status: `[DESIGNED]` — frozen in `specs/PHASE-0-DESIGN.md` §1, §5 (v0.2, 2026-07-14). The verdict types and tension state machine are mirrored in `coven-threads-core` (`validate.rs`, `thread.rs`), and the daemon-side call site is merged to coven `main` (PR https://github.com/OpenCoven/coven/pull/382) — daemons built from `main` act on these verdicts; no cut release includes them yet.

Vocabulary (bound in [concepts.md](concepts.md)): a **Thread** is an authority relationship *surface → writer*; a **Weave** is the enforced pattern of threads; a **Strand** is a fiber inside a thread; a **Channel** is the axis of load a thread must hold under.

## Fail-closed is a conformance requirement, not a feature

This is the first substantive sentence of the frozen design doc, and it belongs first here too.

RFC-0001 §5.4 defines four enforcement gates. Gate 4 — the final canonical diff check immediately before a proposal is applied — is *the real security boundary*; Gates 1–3 are defense-in-depth. And the RFC is explicit:

> *"Gate 4 MUST NOT be skippable. An implementation that allows Gate 4 to be bypassed DOES NOT conform to this RFC."*

So fail-closed is not a hardening milestone coven-threads reaches in some later phase, and it is not a feature you can toggle. **An implementation with a bypassable Gate 4 is not a nonconforming-but-working implementation; it is not an implementation of RFC-0001 at all.** The design doc states this at line one (§1), by Nova non-negotiable, precisely so nobody schedules it as "Phase-1 hardening."

Concretely, fail-closed means every *unknown* resolves to rejection (design doc §5):

- Unknown surface path → **Reject**.
- A protected surface with no thread bound to it → **Reject**. (All protected surfaces MUST have threads; a missing thread is a configuration failure, not permission.)
- Unknown channel → **Reject**.
- Validator panic → the daemon catches it and treats it as **Reject** with a diagnostic.
- Any path that would bypass a gate → non-conformant, and rejected at compile time via the type system where possible.

There is no code path from "the validator doesn't know what this is" to "the write happens."

## The three verdicts

Every mutation request that reaches the gate resolves to exactly one of three outcomes. The set is deliberately small: three verdicts are enough to express "yes," "not without a human," and "no," and anything richer would create corners for ambiguity to hide in.

### Permit

The targeted thread holds under the request's channel, and the weave is coherent at that surface. The daemon applies the mutation and appends the verdict to `ward.audit`. This is the boring path, and it should be the overwhelmingly common one.

### DegradeToProposal

The thread is **frayed** — one of its strands failed, but the thread has not snapped (see the state machine below). The write does **not** touch the protected surface. Instead, the daemon stages it to `~/.coven/pending/` as a proposal and notifies the principal. A human decides.

Two properties worth stating plainly:

- Degradation is *graceful refusal*, not partial permission. Nothing is written to the protected surface, ever, on this path.
- A staged proposal is **data, not authority**. Replaying it later still goes back through `validate` — staging never becomes a bypass around Gate 4. If the thread has snapped in the meantime, the replay is rejected like anything else.

**Phase 5 layers approval semantics on this path** (open, not frozen; authoritative record: `specs/PHASE-5-APPROVAL-SEMANTICS.md`). The verdict is unchanged — the three-verdict set is frozen Phase 0 — but what *promotes* a staged proposal is now typed. At intake, daemon-side classification assigns the proposal an **`ApprovalPath`**: the promotion ceremony required before apply (auto-regression with a veto window, familiar-coherence review, human approval, or human approval with rationale — the RFC-0001 §5.3 tiers). The proposal then applies only through the **delayed-apply flow**: it stays visibly pending for at least a minimum duration, and at the deadline the daemon replays the gate evidence by live re-materialization before any write — matching evidence applies, diverging evidence rejects. "A human decides" above is now, precisely, "the ceremony named by the proposal's ApprovalPath decides, and the daemon applies only after the window closes clean." Full flow and audit lifecycle: [architecture.md — Phase 5](architecture.md#phase-5-approval-semantics-and-delayed-apply).

### Reject

The mutation must not happen: the thread snapped, no thread exists, the surface or channel is unknown, or the validator itself failed. A rejection always carries a named reason, because every rejection is appended to `ward.audit` and an audit entry that says "no, for reasons" is useless. The reject reasons are typed (unknown surface, snapped thread, degraded weave, channel not covered, validator failure), so the audit trail stays machine-legible.

## The thread tension state machine

![Thread tension state machine](diagrams/thread-tension-state.png)

*Holds ↔ Frayed → Snapped, with the verdict each state maps to. Frayed is repairable in place; Snapped requires a fresh authority ceremony.*

A thread's **tension** is its current standing under load. Three states:

### Holds

All strands intact; the thread carries its full authority contract. Gate checks against this thread yield **Permit** (assuming weave coherence at the surface).

### Frayed

One strand failed, but the thread has not snapped. Fraying is the deliberately-engineered *intermediate* state — it exists so that failure is legible and gradual rather than binary and opaque. A frayed thread records which strand failed, on which channel, why, and when — so the operator sees *"thread frayed at strand `ContentHash` — SOUL.md hash mismatch, detected on channel `Forced`"* rather than a bare alarm.

Two hard requirements attach to fraying:

- **Frayed threads MUST surface to the operator** (design doc §2.3). Fraying is never silent.
- Mutations against a frayed thread yield **DegradeToProposal** — the human is now in the loop for that surface until the fray is repaired.

**Repair path:** a fray is repairable *in place*, because the thread's identity and the rest of its strands are intact. Repair means restoring the failed strand — for example, re-verifying the surface against source and re-committing its hash after a legitimate, gated write, or restoring the surface from source when the mismatch reflects tampering. Once the strand verifies again, the thread returns to **Holds**. (The mechanics of repair are the daemon's lane and land with Phase 2 `[DESIGNED]`; the crate defines the states and transitions.)

### Snapped

Terminal severance. The thread no longer carries authority, and mutations against it yield **Reject**. When a thread snaps:

- The weave is marked **degraded at that thread's surface** — and it reports *which* surface, not just "something is wrong."
- The broken surface becomes **read-only until repair**.
- The familiar **continues operating on its other surfaces**. A snapped thread on one surface does not brick the familiar; it quarantines the one surface whose authority state is no longer trustworthy.

**Repair path:** unlike a fray, a snap cannot be repaired in place. Recovering a snapped thread requires a **fresh authority ceremony** — deliberately re-establishing the authority relationship (new thread construction with new strand commitments), under principal authority, rather than patching the old one. The asymmetry is intentional: if a thread's authority contract failed badly enough to snap, "quietly fix it and resume" is exactly the wrong affordance.

The state machine also distinguishes multi-strand failure: if several strands fail simultaneously, in-place repair is not possible and the thread snaps rather than frays.

## Why three verdicts and three states line up

The mapping is one-to-one and worth internalizing:

| Tension state | Verdict on mutation | Human involvement |
|---|---|---|
| Holds | Permit | None (audited) |
| Frayed | DegradeToProposal | Principal reviews staged proposal |
| Snapped / missing / unknown | Reject | Repair ceremony required |

Everything the gate layer does reduces to: assess tension under the request's channel, emit the corresponding verdict, and leave an audit record. The simplicity is the point — a gate you can hold in your head is a gate you can review.

## Where to go next

- What each channel demands of strands (and hence what can fray): [channels-and-strands.md](channels-and-strands.md)
- Where the verdicts flow and land in `ward.audit`: [architecture.md](architecture.md)
