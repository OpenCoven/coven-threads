# Maintainer decision proposal — 2026-10-11

**Proposed engineering reconciliation, 2026-11-12:** this remains an unadopted proposal. This revision preserves the canonical lifecycle requirement and records no narrowing of #13 checklist item 4, no human acceptance, and no freeze/reaffirmation.

**Original attribution:** Echo recorded the October 11 proposal as following Val's direction to take the item 3, 4 and 7 recommendations from the #13 coherence review. That attribution does not establish adoption of the proposed census-only interpretation.

**Scope:** correct the proposal's inference from census classification to lifecycle completeness. It does not accept, reject or defer #13 (`threads-uqx.9`) and does not touch the scoped freeze (#14, `threads-uqx.10`). Any future contract narrowing requires Val's explicit decision against an identified revision under `specs/PHASE-5-SOLO-MAINTAINER-REVIEW.md` section 4.

## Decision 5 proposal — census observations do not replace terminal settlement

Numbering continues from the 2026-09-17 record; no decision is adopted here.

### The criterion

#13 checklist item 4, Audit completeness:

> `proposal_submitted → window_opened → close(reason)` chain is present and no gap exists between open and close. Close event has reason field.

### Why the original proposal needs correction

The coherence packet at Coven `226bfcc89ff6cad4bc9cc9618dad6fea970ecf58` ([record](2026-10-09-coherence-packet-226bfcc8.md#item-4--audit-completeness)) found opened windows whose typed settlement could not be established, including quarantined claims, unprovable applies and untrusted or missing artifacts. The census reports those observations without repair. Reporting them does not supply a missing close or establish that a retained artifact represents a valid live window.

The canonical lifecycle requirement remains: every opened veto window receives exactly one typed terminal close at its permitted conclusion. A genuinely live window may remain pending while its governing conditions permit, but the census alone does not establish those conditions. Unknown or inconsistent history must remain explicit; neither a census category nor this record authorizes fabrication of a close reason.

### What the census establishes

`coven ward audit-census` (`crates/coven-cli/src/ward_audit_census.rs`, `classify`) reports these observations:

1. **Typed terminal observed (`TypedTerminalRecorded`).** The census observed one opening and one terminal carrying typed close detail, without the inconsistencies checked by the classifier. This is recorded-history evidence, not universal proof of every lifecycle obligation.
2. **Unverified open artifact (`OpenUnverified`).** No typed terminal has been observed and at least one pending or claimed artifact name is present. The classifier does not establish that the artifact decodes to a valid proposal, that a worker owns it, that its window remains live, or that it will close. Owner-local validation and investigation remain necessary; this category does not prove that no required close is missing.
3. **Unresolved history reported (`UnprovableApply`, `QuarantinedOpening`, `UntrustedArtifacts`, `OrphanedOpening` or `InconsistentHistory`).** These categories identify observations requiring investigation. They do not satisfy a missing terminal obligation, authorize a fabricated close, or independently reopen a root accepted within a finite historical scope.

A complete classification of observed openings is not complete lifecycle settlement. A missing or unprovable required close remains unresolved even when the census reports it. The classifier's coverage cannot establish timely settlement, and its omission of proposals without an opening cannot establish submission-to-opening completeness.

The [original classifier at `226bfcc8`](https://github.com/OpenCoven/coven/blob/226bfcc89ff6cad4bc9cc9618dad6fea970ecf58/crates/coven-cli/src/ward_audit_census.rs#L312-L394) uses pending/claim artifact counts for `OpenUnverified`. The [regression at surveyed November 8 source `daace730`](https://github.com/OpenCoven/coven/blob/daace730cb1383c9af91ee9248317a02c5209073/crates/coven-cli/src/ward_audit_census.rs#L916-L937) deliberately writes invalid proposal text into a claim filename and expects that classification. This is inspection of existing source, not a newly executed regression or a demonstrated production failure.

### What this does not change

- No code, schema, trigger or test changes.
- Every required terminal still carries a typed close with a normative reason.
- Submission-to-opening completeness remains a separate unproved obligation; the original criterion's chain is not narrowed by this record.
- At the cited historical revision, SQL does not enforce `proposal_submitted` before `proposal_window_opened`; runtime assertions and tests do not become a SQL constraint.
- The finite September 15 workstation-profile census, September 17 root disposition and accepted engineering dossiers retain their exact historical scope.

### Effect on #13

Criterion 4 remains **partial on the cited historical evidence; current-source completeness is unknown without the applicable source-scoped lifecycle and recovery proof**. Trigger enforcement of terminal shape, total census classification and the September 15 census of zero opened windows do not together establish every opened window's valid liveness and eventual typed settlement.

The engineering recommendation is to preserve the original lifecycle requirement and obtain the missing owner-local evidence or production repair where needed. This record selects no human criterion disposition. Val's explicit #13 coherence decision and subsequent #14 freeze/reaffirmation remain separate; any proposal to narrow the criterion requires its own explicit contract decision first.
