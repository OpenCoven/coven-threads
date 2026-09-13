# Phase-5 solo-maintainer review policy

**Status: ADOPTED by Val on 2026-09-13; implementation acceptance remains gated.**
**Scope:** Phase-5 human review ownership only.
**Tracking:** [#59](https://github.com/OpenCoven/coven-threads/issues/59),
under [#31](https://github.com/OpenCoven/coven-threads/issues/31).

This policy lets you, Val Alexander (`@BunsDev`), review your own Phase-5
work and remain its sole human acceptance authority. It replaces the
requirement for a separate human Nova sign-off. It does not reduce the
engineering acceptance criteria.

Val explicitly approved the published policy at revision
`ae12e3ccf656cdce493cb748b7692fb8172a8ecd`. The
[adoption record](https://github.com/OpenCoven/coven-threads/issues/59#issuecomment-5653889552)
identifies the human instruction and scope; it is not an approval inferred
from an agent's account or publication. This active-reference reconciliation
implements that decision. Coherence acceptance and a current-scope freeze
remain separate gates in
[the Phase-5 decision record](PHASE-5-APPROVAL-SEMANTICS.md#7-phase-status-initial-implementation-landed-remediation-blocks-sign-off).

## 1. Adopted decision

While you are the sole human maintainer, you may author, review, and accept
Phase-5 work. Keep three decisions distinct:

| Decision | Human authority | What it establishes |
| --- | --- | --- |
| Adopt this review policy | Val | Changes who may perform the Phase-5 human coherence review |
| Accept or reject coherence | Val, after reviewing the engineering packet | Decides whether identified implementation revisions satisfy the Phase-5 contract |
| Freeze, reaffirm, defer, or reject Phase 5 | Val, after coherence acceptance | Decides whether the accepted scope is complete and change-controlled |

Nova, Echo, Sage, Cody, Charm, and other agent roles may investigate, review,
run checks, prepare evidence, and recommend a disposition. Their output is
engineering evidence, not a separate human decision. An agent cannot accept
its own work on your behalf or infer your decision from general permission
to continue.

This is explicitly self-review, not independent human review. A separate
non-author technical pass is recommended where available, with its actual
reviewer and limitations recorded. Agent review and CI do not eliminate the
risk of shared assumptions or replace an independent human reviewer.

## 2. What does not change

RFC-0001, protected-write authority, canonical ownership, the eight Phase-5
design defaults, and the eight-item coherence checklist remain unchanged.
Threads remains a validator; Coven remains the runtime authority and sole
owner of effects and audit persistence.

In particular, this policy does not:

- permit protected writes through proposal routes or client authority claims;
- weaken predicate enforcement, final Gate-4 revalidation, minimum visibility,
  typed terminal close, replay, or recovery requirements;
- treat quarantine as a completed typed close or repair historical corruption
  by inventing audit evidence;
- waive the real-daemon topology, synthetic-fixture rules, negative evidence,
  migration/rollback obligations, or source-provenance requirements;
- replace a required pinned compatibility lane with an advisory run;
- alter branch protection, authorize a release or deployment, close an issue,
  or open a later phase.

The [solo-maintainer merge policy](../CONTRIBUTING.md#solo-maintainer-merge-policy)
continues to govern ordinary PR landing. Approval to merge a bounded change
does not imply acceptance of Phase 5.

## 3. Engineering packet required for your review

Before requesting coherence acceptance, prepare a packet against identified
Threads, Coven, and applicable Cave revisions. Use full commit SHAs, not
mutable branch names. Map the existing
[#13 checklist](https://github.com/OpenCoven/coven-threads/issues/13) to exact
types, predicates, runtime call sites, scenarios, and results.

For each root issue, preserve its complete stated criteria:

| Root | Required evidence focus |
| --- | --- |
| OpenCoven/coven#885 | Every known intake and deadline/recovery identity boundary, authoritative evidence binding, and fail-closed negatives |
| OpenCoven/coven#886 | The complete typed-terminal matrix, single consumption, restart, and explicit handling of inconsistent history |
| OpenCoven/coven#887 | Every known protected proposal route, cross-target/replayed authority refusal, forwarding-only clients, and non-protected compatibility |
| OpenCoven/coven#888 | All seven retired-corpus scenarios through migration, supported intake, scheduling, restart, and typed close |

Each dossier must identify the original red, production fix, accepted green,
lower-level regression, exact commands/results, artifact identity/digest,
platform scope, and migration/rollback implications. A component PR or a
checked issue box is not a substitute for the dossier. Unsupported protected
write authority may remain disabled where the governing contract allows it;
do not invent a positive authority case.

Apply the [E2E contract](../docs/testing/e2e-contract.md) and
[delivery acceptance targets](../docs/strategy.md#4-complete-resilience-and-human-acceptance).
Show that the actual current Threads checkout is used by the real daemon.
Keep required-pin, latest-main, native-platform, Cave, and reliability evidence
distinct. Record failures and remaining gaps rather than transferring a result
between source revisions or counting retry success as first-attempt success.

## 4. How to review your own work

1. Read the packet's objective, non-goals, exact revisions, and remaining gaps.
   Confirm that its scope matches the change you intend to accept.
2. Follow each #13 checklist item to its source and evidence. Pay particular
   attention to refusal paths, restart, duplicate work, and ambiguous state,
   not only successful application.
3. Review all four root dossiers. Require missing evidence or corrections
   before accepting a criterion. Engineering closure can proceed under the
   existing authorization, but it cannot supply your human acceptance.
4. Record an explicit coherence decision on #13. Identify this policy's
   adoption record and the packet you reviewed. Accept, reject, or defer;
   do not approve unspecified future commits.
5. Record or reaffirm the final freeze decision on
   [#14](https://github.com/OpenCoven/coven-threads/issues/14), mirrored under
   #31 and `threads-uqx.10`, referencing the completed coherence decision and
   exact accepted revisions. If a required criterion remains unmet, defer or
   reject the freeze.

You do not need to impersonate another reviewer or claim another person
approved your work. A later source or policy change requires review of the
affected criteria; approval does not automatically transfer to a new head.
Residual risks must be explicit. Recording an exception or a follow-up issue
does not waive a required criterion; changing one requires its own authorized
contract decision.

## 5. Decision records you can use

These are templates, not approvals. Replace the bracketed fields after your
review and record the decision yourself on GitHub. An agent-created comment,
authenticated account name, commit trailer, or passing check alone does not
prove that you made the decision.

The adoption record belongs on #59. Future policy amendments also require an
explicit human decision against an identified revision:

```text
Human decision-maker: Val Alexander (@BunsDev)
Decision: adopt / reject / request changes
Policy revision: [full commit SHA containing the reviewed policy]
Scope: Phase-5 solo-maintainer human review ownership
Self-review limitation: [acknowledgment of no independent human approval]
Rationale: [your reason and any requested changes]
This decision does not accept the implementation or freeze Phase 5.
```

Coherence acceptance belongs on #13 after the engineering packet is complete:

```text
Human decision-maker: Val Alexander (@BunsDev)
Decision: accept / reject / defer Phase-5 coherence
Policy adoption: [link to your adoption decision]
Reviewed packet: [immutable revision or evidence links]
Accepted source revisions: [full Threads, Coven, and applicable Cave SHAs]
Checklist and root dispositions: [links to all eight items and four dossiers]
Residual risks and limitations: [explicit list, or none identified]
Rationale: [your assessment]
This is my human self-review decision, not independent human Nova approval.
This decision does not freeze Phase 5.
```

The final freeze record belongs on #14, with a link under #31:

```text
Human decision-maker: Val Alexander (@BunsDev)
Decision: freeze / reaffirm / defer / reject Phase 5
Coherence decision: [link to your completed coherence review]
Accepted scope and source revisions: [exact boundaries and full SHAs]
Required acceptance evidence: [links, including outstanding dispositions]
Rollback and remaining risks: [explicit record]
Rationale: [your final assessment]
This decision does not independently authorize a release or deployment.
```

## 6. Adoption, tracking, and rollback

The adoption requires consistent active references: the Phase-5 decision
record's human-gate descriptions, `AGENTS.md`, contributor guidance, delivery
ledger and strategy, #13, #14, and the two human-gate beads. This reconciliation
uses a normal PR and links the human adoption decision. Apply the same change
control to future reviewer-model amendments.

Retain `threads-uqx.9` as the coherence gate, relabelled as Val's agent-assisted
or self-reviewed human acceptance, and retain `threads-uqx.10` as the subsequent
freeze gate. Preserve their dependencies on engineering closure. Do not close
either gate merely because the policy was adopted.

Keep historical Nova refusals, unsigned recommendations, and earlier decisions
unchanged and dated. #14 already contains Val's July 29 freeze statement and
was closed while #13's independent coherence gate remained outstanding. Do not
erase that human decision or claim it was never given. This policy
does not retroactively satisfy its missing prerequisite or extend its scope
to later implementation revisions. A current decision may reaffirm or
supersede that record after the applicable coherence review.

Reopening #14 to track a current-scope freeze or reaffirmation does not revoke
its historical decision. Its issue state alone is not acceptance evidence.

If you later withdraw this policy or another human maintainer joins,
record and reconcile the replacement reviewer model before further acceptance.
Do not delete prior evidence, silently revoke historical decisions, or weaken
runtime enforcement as a governance rollback.
