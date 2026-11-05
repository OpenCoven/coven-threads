# Maintainer decisions — 2026-10-11

**Decided by:** Val (solo-maintainer policy, `specs/PHASE-5-SOLO-MAINTAINER-REVIEW.md`)
**Recorded by:** Echo, on Val's direction of 2026-10-11 to take the item 3, 4
and 7 recommendations from the #13 coherence review
**Takes effect:** when this record merges. Val's merge is the act of recording it.
**Scope:** one contract clarification for #13 checklist item 4. It does not
accept, reject or defer #13 (`threads-uqx.9`), and it does not touch the scoped
freeze (#14, `threads-uqx.10`).

## Decision 5 — "no gap between open and close" covers census-reported windows, never fabricated closes

Numbering continues from the 2026-09-17 record.

### The criterion

#13 checklist item 4, Audit completeness:

> `proposal_submitted → window_opened → close(reason)` chain is present and no
> gap exists between open and close. Close event has reason field.

### Why a clarification is needed

The coherence packet at Coven `226bfcc89ff6cad4bc9cc9618dad6fea970ecf58`
([record](2026-10-09-coherence-packet-226bfcc8.md#item-4--audit-completeness))
found that an opened window can stay without a typed close. That happens when
its claim is quarantined, its apply cannot be proven, or its artifacts are
untrusted or gone. The daemon does not write a close in those cases, and the
census reports them without repair.

Read literally, "no gap exists between open and close" would require a close row
for every opened window. The only way to provide one in those cases is to invent
a terminal the daemon cannot prove. The design forbids that: the core trigger
rejects terminal events without typed close detail, and census tooling must never
fabricate history (2026-09-17 Decision 1, and the #886 root's "explicit handling
of inconsistent history"). Under policy section 4, a required criterion cannot be
waived by naming the gap, so its meaning is clarified here instead.

### The clarified meaning

"No gap exists between open and close" means that every `proposal_window_opened`
row for a proposal is in exactly one of these states, as classified by
`coven ward audit-census` (`crates/coven-cli/src/ward_audit_census.rs`,
`classify`):

1. **Closed.** It has exactly one typed terminal whose detail carries a normative
   `WindowCloseReason` (`TypedTerminalRecorded`).
2. **Still open.** Its window is live, with a pending or claimed artifact still
   present (`OpenUnverified`). This is not a gap, because the window has not
   ended.
3. **Reported, never repaired.** The census reports it as `UnprovableApply`,
   `QuarantinedOpening`, `UntrustedArtifacts`, `OrphanedOpening` or
   `InconsistentHistory`. Nothing writes a close for it. A window in this state on
   a deployed profile is a defect to investigate, as Decision 1 already provides.
   It does not reopen a closed root.

A **gap** is therefore an opened window in none of these states: one that has a
terminal without typed close detail, or one that the census cannot see. The first
is rejected by the `ward_audit` triggers. The second would be a census defect.

### What this does not change

- No code, schema, trigger or test changes.
- It does not relax "Close event has reason field". Every terminal still requires
  a typed close with a normative reason.
- It does not cover the interval between `proposal_submitted` and
  `proposal_window_opened`. The opening row is written at the first scheduler pass
  or at decision time, and the criterion's "no gap" is between open and close.
- It does not add enforcement that `proposal_submitted` precedes
  `proposal_window_opened`. Tests assert that order; SQL does not enforce it. That
  is recorded as a residual risk on #13, not resolved here.

### Effect on #13

With this clarification, the packet's item 4 evidence supports the criterion as
clarified:

- the triggers that enforce one typed terminal with a valid reason;
- the census classifications above;
- the tests named in the packet;
- the 2026-09-15 census of the workstation profile, which found 0 opened windows.

Whether to accept item 4 on that basis remains part of Val's #13 decision.
