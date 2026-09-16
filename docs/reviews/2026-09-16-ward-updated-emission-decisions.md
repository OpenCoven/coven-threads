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

## Decision 2 — `principal_authorization` binds to the existing Gate-1 check

A `ward_updated` row records a structured token in the shape the conformance
corpus uses, and the daemon emits it **only when** the claimed fingerprint
matches `principal_key_fingerprint` through the comparison Gate 1 already
applies, over owner-gated local IPC.

This is stated plainly rather than dressed up: that comparison is **string
equality against a configured fingerprint, not signature verification**. The
daemon verifies no signatures anywhere today. So the recorded authorization is
**descriptor strength**, and this decision does not pretend otherwise.

Rejected alternatives:

- **Require real signature verification.** The honest predicate, and closest to
  RFC intent. It was rejected only because it blocks `ward_updated` behind
  exactly the authenticated protected-write authority that OpenCoven/coven#887
  keeps deliberately disabled fail-closed. Adopting it would make committed Ward
  state unreachable for the same reason promotion already is.
- **An unbound descriptor token.** Simplest and matches the sample literally,
  but records an authorization claim nothing tested — the descriptor-as-predicate
  mistake PHASE-0-DESIGN §2.2 exists to prevent.

**What this does not license.** Binding to the existing check is not a claim that
protected-write authority is authenticated, and must not be cited to re-enable
that route. When signature verification lands, this binding should be upgraded
to it and the descriptor-strength caveat removed.

## Status

Both decisions are design commitments, not implementation. The Threads-side
record contract landed separately in #75: `for_ward_updated` exists, and
`ward_version` and a 32-byte `ward_hash` are both required of a `ward_updated`
row so an unusable anchor cannot be recorded.

What remains is daemon work in `OpenCoven/coven` — where a Ward commit is
established, how genesis is identified, and emitting the event at those points.
