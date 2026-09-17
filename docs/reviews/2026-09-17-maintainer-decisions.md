# Maintainer decisions — 2026-09-17

**Decided by:** Val (solo-maintainer policy, `specs/PHASE-5-SOLO-MAINTAINER-REVIEW.md`)
**Recorded by:** session agent, on Val's direction, same day
**Scope:** the four open decisions surfaced by the 2026-09-17 tracking
consolidation (`threads-9cr`). Nothing here touches the human coherence gate
(`threads-uqx.9`, #13) or the scoped freeze (`threads-uqx.10`, #14).

## Decision 1 — OpenCoven/coven#886 is closed on the workstation-profile audit census

The root's two closure requirements are met and it is closed without repair:

- **Engineering.** All five terminal close families (`applied`, `vetoed`,
  `evidence_diverged`, `revalidation_failed`, `superseded`) emit typed
  `ProposalWindowCloseAuditDetail` and are exercised end to end at accepted
  daemon `226bfcc89ff6cad4bc9cc9618dad6fea970ecf58` under CI 34784497791. The
  core trigger still rejects any terminal event without typed close detail
  after `proposal_window_opened`.
- **Deployed history.** The read-only census of 2026-09-15
  ([record](2026-09-15-deployed-audit-census.md)) is scoped to the maintainer
  workstation profile, the only deployed profile inspected. It found 0 opened
  windows, 0 unresolved, 0 unattributed artifacts, corroborated by independent
  SQL. Val accepts that profile as the deployed inventory for this root; no
  other deployed profile is known.

There is no history to repair in the inspected profile. A future profile that surfaces an opened window
without a typed close is a new defect against the census tooling and triggers,
not a reopening of this root. `threads-980` is closed with this disposition,
which makes `threads-uqx.9` dependency-clear. Dependency-clear is not
acceptance; #13 remains Val's separate decision.

## Decision 2 — the authenticated operation-bound authority path stays fail-closed indefinitely

`threads-19p` established on 2026-09-16 that no conforming promotion admission
is reachable until an authenticated, operation-bound principal authority path
exists, and that emitting a `ward_updated` anchor on descriptor-strength
evidence is rejected ([Decision 2 of 2026-09-16](2026-09-16-ward-updated-emission-decisions.md)).

Val now takes the deferral branch that bead's acceptance criteria explicitly
permit: **the dependent work stays fail-closed indefinitely.** Consequences:

- No `ward_updated` or `principal_authorized_write` row is emitted by the
  daemon. The Threads-side `for_ward_updated` constructor (#75) remains the
  ready call site for whenever that changes.
- `Channel::Deliberate` remains specified and implemented in the library and
  unreachable from the daemon. The seam contract (#25) keeps
  `coven memory promote` labelled planned, not available.
- The promotion-seam conformance obligations in `threads-lm4` are satisfied to
  the extent a library can satisfy them (`threads-bv2`, #78); the runtime items
  gated on a daemon promotion route are moot until that route is scoped.
- `threads-19p`, `threads-vdv`, `threads-xpo`, and `threads-lm4` are closed on
  this recorded decision, not on implementation.

Reopening any of them requires a new, daemon-owned scope decision that names
what authenticates the principal and binds the operation. Until then, "no
fingerprint or invented approval identifier grants protected-write authority"
(OpenCoven/coven#887) is the standing rule.

## Decision 3 — `threads-76z` is closed won't-fix; strict SQL stays

The `ward_audit` approval trigger contains two clauses that cannot both be
satisfied for a human-path proposal with an opened window. That state is
unreachable through the Rust types, which are the only sanctioned writer.

Val's ruling: the SQL is **not** relaxed. Allowing a null or absent close on
that path would weaken the Phase-5 invariant that every opened window closes
with exactly one typed reason, which is the same reason PR #27 was closed
without merge on 2026-09-06. An out-of-band writer that manufactures the
inconsistent state gets an abort, which is the correct fail-closed outcome. The
draft branch is preserved as tag `archive/fix-threads-76z-window-close-human-path`
and is not a fix to resurrect.

## Decision 4 — OpenCoven/coven#1082 merges

The fixture-only correction for the legacy Windows named-pipe connection order
(`threads-4v8.1`) had required CI green on native Windows and portable cases.
The one open review thread asserted a synchronization circularity; the inline
reply source-disproved it (the health call is gated on the server's own
`continue_tx`, not the reverse). Val resolved the thread and merged. This
closes the fixture bead only; the runtime observations in
OpenCoven/coven#1051 (`threads-4v8`) stay open awaiting new evidence.

## What remains open after these decisions

| Item | Bead | Waiting on |
|---|---|---|
| Val coherence acceptance | `threads-uqx.9` (#13) | Val, now dependency-clear |
| Val scoped freeze / reaffirmation | `threads-uqx.10` (#14) | #13 |
| Native Windows IPC / reservation latency | `threads-4v8` | new evidence on OpenCoven/coven#1051 |
| Native Linux HTTP-read timeouts | `threads-xsd` | new evidence on OpenCoven/coven#1050 |
| Cold Windows CLI startup outlier | `threads-bp5.4` | source-bound trace on OpenCoven/coven#1047 |
| Familiar inbox / handoff ledger | `threads-5rr` (deferred to 2026-11-01) | Val scoping |
| Live Cave acceptance | — | coven-cave#5256 |
