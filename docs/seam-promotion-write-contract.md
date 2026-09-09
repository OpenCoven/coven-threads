# SEAM CONTRACT — promotion-write ↔ weave

**Bead:** `threads-ot6` (this repo) · coven-memory mirror bead unresolved; repo-owned coven-memory docs track the seam as `SEAM` (see `threads-5mn`)
**Status:** `[NORMATIVE FOR WHEN BUILT — contract text only; no conforming implementation exists today]`
**Co-owners:** Echo (contract language) · Cody (language-correctness review only)

---

## 0. What this contract is for

`coven memory promote` is planned coven-memory M2 work, not a shipping command.
When that promotion surface produces a write, and that write lands on a surface
`coven-threads` guards, this document is the **single** integration contract
between the substrate side that decides *what* to promote and the authority side
that decides *whether the write is permitted*.

Unless a paragraph explicitly says it is describing current observed behavior,
the sections below are normative for a conforming future implementation.

It is one seam, described once, and tracked in both repos.

---

## 1. Boundary — non-negotiable

PHASE-0-DESIGN §3.3.1 anti-non-negotiable governs and is quoted verbatim:

> **Anti-non-negotiable:** `coven-threads` does not own retrieval, promotion, or dreaming. It owns *authority over writes to the protected surface, gated by the weave*. Scope creep here fragments the atomic-ship trio. Hold the line.

Therefore:

| Concern | Owner |
| --- | --- |
| What is worth promoting | coven-memory |
| Candidate selection, ranking, dedup, dreaming | coven-memory |
| Promotion trigger and cadence | coven-memory |
| Whether a promotion write may commit | coven-threads (via daemon) |
| Classification, gate, approval, and audit-record contracts | coven-threads |
| Staging, commit-time replay, filesystem writes, and audit persistence | coven daemon |

`coven-threads` MUST NOT gain a promotion policy. `coven-memory` MUST NOT gain a
permit decision. Neither side may infer the other's answer.

**Corollary from §3.3.2 (source-authoritative retrieval):** a promotion write
targets a *source-authoritative surface* only. No thread terminates on a
promoted view, retrieval cache, or index. If a promotion write's declared target
is a derived structure, it is outside this contract entirely — it needs no weave
authority, because nothing has authority to be tampered with there. Tampering on
derived structures is detected by re-derivation, not by gating.

---

## 2. Channel — `Channel::Deliberate`

For a conforming promotion implementation, the effective channel MUST be
`Channel::Deliberate`.

This is not a new decision; PHASE-0-DESIGN §2.4 already defines the channel as
*"deliberate compaction (promotion, dreaming, memory flush). Familiar-initiated,
principal-gated."* Promotion is the named referent. The contract records it so
neither side re-litigates it.

**Hard constraint (Phase 5 decision 1):** `ApprovalPath` MUST NOT be derived from
`Channel::Deliberate`. Channel is the load axis; ApprovalPath is the promotion
ceremony. They are orthogonal and both first-class. A promotion write being
`Deliberate` says nothing about which ceremony clears it.

`Channel::Deliberate` is normative contract language here, not a description of
current daemon behavior.

**NOT YET IMPLEMENTED — `threads-xpo`:** current daemon code does not expose a
promotion submission path and does not reach `Channel::Deliberate`. When the
boundary exists, any caller-supplied channel label is descriptive only: the
daemon MUST authoritatively determine or revalidate the effective channel and
reject a mismatch rather than coerce it.

---

## 3. Target classification — the fork that matters

The daemon classifies the promotion write's **materialized** target. Declared
target is advisory; materialized target is authoritative (Gate 4).

### 3.1 Proposal-eligible target (Tier 1–3)

Runs the normal Phase-5 pipeline: `ProposalClassification` → gates → staging →
`ApprovalPath` → applicable approval ceremony → live Gate-4 replay → apply.
Only the veto-window approval paths open a window; escalation to a human path
does not create one.

**[PROPOSED] Recommended default ceremony:** `ApprovalPath::FamiliarCoherence { veto }`.

Rationale: promotion is familiar-initiated compaction of the familiar's own
memory. The familiar-coherence gate is the ceremony whose question ("does this
still cohere with who this familiar is?") matches what promotion actually risks.
`AutoRegression` is too thin — promotion has no deterministic regression suite
that would catch semantic drift. Human paths are too heavy for routine promotion
and would make the feature unusable at cadence.

This default remains proposed pending Cody's language review and later Nova
ratification. It is a floor, not a ceiling. Highest ceremony still wins for the
proposal as a unit: a promotion write touching a high-risk semantic region
elevates normally.

### 3.2 Protected target (Tier 0)

**Rejected from the proposal pipeline. No exceptions, no fingerprint escape.**

RFC-0001 §5.4 and Phase-5 §3.1 both hold: every declared *or materialized* diff
touching the protected surface is rejected at Gates 1, 2, and 4. A promotion
write is not privileged. "The memory substrate wanted it" is not authority.

If a principal genuinely wants promoted content on a protected surface, that is a
**principal-authorized Ward update** — a separate, daemon-owned, independently
authorized path outside `ApprovalPath`, audited as `principal_authorized_write`.
It is not reachable by `coven memory promote` and coven-memory MUST NOT offer it
as a promotion outcome.

> **Cross-bead dependency:** this clause inherits whatever `threads-dgg` lands.
> `threads-dgg` exists precisely because current `coven` still stages and
> approves protected SOUL.md edits through `/threads/proposals` after a principal
> fingerprint is supplied. Until `threads-dgg` closes, §3.2 describes intended
> behavior, not observed behavior. A language-only review of this document
> cannot certify the runtime behavior; that proof remains in `threads-lm4`.

---

## 4. Fail-closed on unknown edges

Every ambiguity resolves to reject. Never to permit, never to a default surface,
never to LLM judgment.

Two existing vocabularies cover every edge, at the two stages where a promotion
write can die. They are distinct types and must not be conflated: `RejectReason`
(`validate.rs`) is the *admission* vocabulary — the write never enters the
proposal pipeline; `WindowCloseReason` (audit layer, trigger-enforced) is the
*lifecycle* vocabulary — the write entered the pipeline and its opened window
must close with a typed terminal reason (§5.2).

Before any `RejectReason` mapping applies, a frayed thread returns
`Verdict::DegradeToProposal`; fray is staged, not encoded as a reject reason.

**Admission rejections — `RejectReason`:**

| Condition | Verdict | Existing `RejectReason` |
| --- | --- | --- |
| Target surface not registered in the weave | reject | `UnknownSurface` |
| Promoting familiar holds no thread to that surface | reject | `WriterNotBound` |
| Thread does not cover `Channel::Deliberate` | reject | `ChannelNotCovered` |
| Thread snapped | reject | `ThreadSnapped` |
| Weave pattern predicate does not hold | reject | `WeaveBroken` |
| Surface degraded at this surface | reject | `SurfaceDegraded` |
| Validator panicked | reject | `ValidatorPanic` |

**Lifecycle terminations — `WindowCloseReason`:**

| Condition | Verdict | Existing `WindowCloseReason` |
| --- | --- | --- |
| Materialized target ≠ declared target (Gate-4 replay divergence) | reject | `evidence_diverged` |
| Evidence cannot be re-derived at deadline | reject | `evidence_diverged` |
| Live revalidation fails at apply time | reject | `revalidation_failed` |
| A newer proposal supersedes this one | reject | `superseded` |

No new reason is required in either vocabulary. That is deliberate — if the
promotion seam needed its own rejection vocabulary, it would be evidence the seam
had grown policy it should not own.

**Rejection output constraint:** a rejection concerning a protected target MUST
NOT echo protected values back to the caller. coven-memory receives the verdict
and the reason kind, never the guarded content.

---

## 5. Audit

### 5.1 Admission

A committed promotion write emits `memory_entry_admitted`, carrying
`entry_hash` and `source_attestation`.

`source_attestation` is coven-memory's assertion of provenance — where the
promoted content came from in the substrate. It is **evidence, not authority**.
The daemon records it; the daemon does not trust it to make the permit decision.
This is the same predicate-vs-descriptor discipline as PHASE-0-DESIGN §2.2: attestation is
descriptive, gate results are enforcing.

### 5.2 Proposal lifecycle

A promotion write routed through a veto-window approval path in §3.1 emits:
`proposal_submitted` → `proposal_window_opened` → exactly one terminal event
(`proposal_approved`/`applied`, `proposal_vetoed`/`vetoed`, or `proposal_rejected` with `evidence_diverged`, `revalidation_failed`, or `superseded`). No opened window may be left without a typed close.

Human approval paths do not emit `proposal_window_opened` and have no
`window_close` payload. They must not be substituted for an already-open
window to evade its terminal-close obligation.

> **Cross-bead dependency:** `threads-980` is open precisely because rejection
> branches currently emit `window_close=None`. Promotion writes inherit that
> defect. This contract cannot be verified end-to-end until `threads-980` closes.

### 5.3 Defect found while drafting — `channel` is dropped

`WardAuditRecord::for_memory_entry_admitted` (`crates/coven-threads-core/src/audit.rs`)
constructs its row with `channel: None`.

This contract requires promotion writes to be `Channel::Deliberate`, and requires
that fact to be *auditable*. As written, the admission row cannot distinguish a
promotion-driven admission from any other. That is a legibility hole in the
single daemon-owned `ward.audit` store Phase 0 §3.4 exists to keep coherent.

**Requested change (Cody):** `for_memory_entry_admitted` should accept and record
the channel. Filed as a follow-up bead rather than patched here, since it touches
the audit row shape and belongs with the other audit work.

---

## 6. What this contract does not decide

- Promotion cadence, batching, or retry — coven-memory M2.
- Whether promotion runs during dreaming — coven-memory M2.
- Any new `Channel` variant. §2 uses an existing one.
- Any new strand kind. Phase-5 §3.2 default holds: predicate-first, no new strand
  until implementation evidence forces one.
- Cave surfacing of pending promotion proposals. Phase 4 §4 governs: unknown or
  stale veto state renders blocked. No optimistic UI.

---

## 7. Language-only acceptance

This document satisfies the language-only acceptance criteria when:

1. Echo and Cody both agree the language as a specification; Cody's review is
   limited to language correctness, not an implementation shape for a command
   that does not yet exist.
2. §2 stays explicit that it is normative for when built and carries the
   `NOT YET IMPLEMENTED — threads-xpo` marker while no
   `Channel::Deliberate` submission path exists.
3. This document does not ratify a phantom coven-memory mirror ID. It may point
   to `threads-5mn` and the coven-memory `SEAM` tracker row, but it MUST NOT
   invent a bead identifier.
4. §3.1 stays marked `[PROPOSED]`; the default ceremony is not frozen as a
   decision here.
5. This section stays aligned with `threads-ot6` so the bead and contract do
   not encode divergent gates.
6. The status header continues to say the contract is normative for when built
   and that no conforming implementation exists today.

Observed-runtime proof for protected-route prohibition, typed terminal closes,
and admission-channel audit remains outside this bead (`threads-lm4`,
`threads-dgg`, `threads-980`, `threads-55s`).

---

_Sage review pass, 2026-08-06: §4 split into the two actual vocabularies —
`RejectReason` (admission) vs `WindowCloseReason` (lifecycle) — after verifying
both against `validate.rs` and `audit.rs`. The original table implied one
rejection vocabulary where the code has two. No semantic change._

_Echo, 2026-08-06. Original draft was written from the coven-threads side only.
Repository-owned coven-memory docs checked in this review confirm the seam
exists and is tracked there as `SEAM`, but they do not establish a specific
mirror bead ID or ratify the rest of the substrate-side behavior. `threads-5mn`
remains open for the exact linkage, and implementation claims on the memory side
still need Cody plus the coven-memory owner to confirm._
