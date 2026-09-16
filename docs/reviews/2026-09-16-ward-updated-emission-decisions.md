# Decisions — emitting committed Ward state (`threads-vdv`)

Two decisions by Val on 2026-09-16, taken after `threads-vd8` anchored promotion
admissions on committed Ward state and investigation found the daemon emits no
`ward_updated` events at all.

They are recorded here because `coven-threads` is canonical for
`threads.ward-audit-schema` and both shape what a conforming row carries. The
emission itself is daemon work and is tracked in `OpenCoven/coven`.

## What forced the decisions

The daemon has **no Ward version concept**. `WardConfig` carries
`principal_key_fingerprint`, `protected_surface` and tier defaults — no version
field. `ward_version` appears in coven only as a nullable column name inside
audit SQL; nothing computes, persists or increments it. `ward_manifest` is not a
substitute: it is a per-familiar, per-surface content baseline keyed by surface,
daemon-owned state rather than RFC-0001's committed Ward state.

RFC-0001 requires the field but does not constrain its format. The normative
conformance sample uses `"0.1.0"`, with
`principal_authorization` as `"val-alexander:2026-07-26:ward-commit"` and
`source_attestation` as `"<event_type>:<event_id>"`.

## Decision 1 — `ward_version` is a declared semver in `ward.toml`

`WardConfig` gains a version field, declared by the principal, matching the
normative sample. It fits the shape of the operation: a Ward update is a
principal-authorized act outside the gate pipeline, so the principal declaring
the version is coherent rather than incidental. It also stays human-meaningful
in the ledger, which a counter or digest would not.

Rejected alternatives, and why they were tempting:

- **Monotonic counter from the audit log.** Zero config, zero migration, cannot
  drift. But append-only log order already establishes which pair is most
  recent, so the counter mostly restates what the log says.
- **Content digest of `WardConfig`.** Cannot be stale or lie. But `ward_hash`
  already carries content integrity, so this duplicates it while being opaque to
  anyone reading the ledger.

**Cost to plan for.** Existing `ward.toml` files carry no version, so this needs
a default or a migration pass, and `ward.toml` is itself Tier 0 — changing it is
a protected write. Genesis writes the first version, which is consistent with
RFC-0001: the first Ward manifest is a principal-authorized genesis write whose
authorization requires no prior committed Ward state.

## Decision 2 — no anchor is emitted until authorization is authenticated

A `ward_updated` row is **not** emitted on descriptor-strength evidence.
Committed Ward state stays absent, and anything depending on it fails closed,
until an authenticated, operation-bound principal authority path exists.

**This reverses the recommendation first made here.** The original proposal was
to record a corpus-shaped token whenever a claimed fingerprint matched
`principal_key_fingerprint` through the comparison Gate 1 already applies, and
to label the result descriptor strength. Review showed that unsound, and the
objection was correct.

`principal_key_fingerprint` is a configured descriptor. OpenCoven/coven#887's
accepted disposition is explicit that it confers no authority:

> Generic proposal intake no longer treats fingerprints or approval-reference
> text as protected-write authority.

> matching/null text or an old reference cannot upgrade authority.

> no fingerprint or invented approval identifier grants protected-write
> authority.

Under the rejected proposal, a genesis or configured Ward supplying that value
would mint a **valid** committed-state anchor, which `threads-vd8` then has
promotion resolve against. That launders a descriptor into authorization across
a trust boundary. Labelling it does not help: the label is prose, while the row
is machine-consumed — `source_attestation` validation checks that the referent
resolves and carries its required fields, not that a document called it weak.

### The floor this exposes

The anchor choice never mattered. RFC-0001 §5.6 requires `principal_authorization`
on **both** referent types, `ward_updated` and `principal_authorized_write`. So
**no conforming promotion admission is reachable until authenticated principal
authorization exists**, whichever referent `threads-vd8` had selected.

That is a floor under the whole promotion programme, not a property of one
design choice, and it is better known now than discovered during implementation.

### What this does not mean

It is not a claim that the fingerprint comparison is worthless — it remains a
useful Gate-1 check for what it does today. It is a refusal to promote that
check into provenance authority for an append-only anchor other subsystems
consume.

Nor is it a decision to build the authenticated path. That work stays where
OpenCoven/coven#887 left it: disabled fail-closed, an explicitly permitted
disposition.

## Status

Both decisions are design commitments, not implementation. The Threads-side
record contract landed separately in #75: `for_ward_updated` exists, and
`ward_version` and a 32-byte `ward_hash` are both required of a `ward_updated`
row so an unusable anchor cannot be recorded.

Decision 1 stands and is implementable whenever the daemon work proceeds.
Decision 2 means that work does not proceed yet: emitting the event requires an
authenticated, operation-bound authority path that does not exist, so
`threads-vdv` now depends on that prerequisite rather than on wiring.

The `for_ward_updated` doc comment names the field shapes Decision 1 settles, so
the call site is ready when the floor lifts.
