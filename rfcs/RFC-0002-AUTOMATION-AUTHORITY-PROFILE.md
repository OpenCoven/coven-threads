# RFC-0002: Automation Authority Profile

**Status:** Accepted for profile v1 implementation

**Issue:** `OpenCoven/coven-threads#29`

**Version:** 1.0.0
**Normative specification:** [`../specs/AUTOMATION-AUTHORITY-PROFILE-v1.md`](../specs/AUTOMATION-AUTHORITY-PROFILE-v1.md)

## Decision

Threads defines a portable, versioned Automation Authority Profile that returns
one of four operation-specific outcomes: permit, requires approval, degrade to
proposal, or reject.

Authority is bound to immutable request bytes, not prompt text, routine tags,
runtime names, or client lifecycle state. The request and decision bind the
principal, familiar embodiment digest, automation definition revision/digest,
occurrence/run/attempt/fence, action digest, capabilities/scopes,
project/workspace, runtime descriptor/capabilities, policy/manifest versions,
and previous approval where applicable.

## Security posture

The profile is fail-closed. Wire objects are closed; strict I-JSON parsing
rejects duplicate keys, invalid Unicode, and unsafe integers. SHA-256
domain-separated canonical digests and Ed25519 signatures authenticate
requests, decisions, approvals, lifecycle events, proposals, and evidence-read
requests.

Approval evidence is immutable. Lifecycle, consumption, expiry, and revocation
are append-only signed events with replay-safe sequence/digest chaining. A
proposal-only outcome cannot perform a protected effect or claim success.
Dispatch repeats all material checks against one immutable snapshot so changed
policy, manifest, identity, definition, runtime, scope, action, or fence cannot
cross a TOCTOU gap.

Final dispatch accepts no client-authored lifecycle summary. It verifies the
signed event chain against a signed, revision-bound consumption snapshot.
It also re-evaluates the request against the policy snapshot pinned in the
trusted dispatch snapshot and requires the signed decision to match that
canonical result; signature validity alone is insufficient. Runtime capability
sets remain exact across request, decision, and dispatch, and grants cannot
exceed the decision-bound runtime.
`human_per_run` requires the authorized principal's key;
`protected_owner_per_run` requires a protected-owner key. Recurring R2
approvals separate immutable grant authorization from exact per-run signed
consumption evidence and remain occurrence-pattern, expiry, revocation, and
usage bounded.

RFC 3339 UTC timestamps are component-validated without calendar
normalization. Evidence reads reject missing or unknown sensitivity and
retention metadata before authorization comparisons.

Identity-bearing principal, protected-owner, and auditor keys always carry an
exact `principal_id`; omission never means wildcard identity. Generic
`evidence.read` authority is self-only. Cross-principal evidence access is
available only through the dedicated signed auditor flow.
Threads-authority keys cannot carry principal identity. Dedicated self-read
tokens require a principal key, and cross-principal tokens require an auditor
key; protected-owner and Threads-authority roles cannot substitute.

## Ownership

Threads owns classification, capability/scope semantics, decision and approval
contracts, replay/revocation rules, integrity preimages, and golden vectors.

Threads does not own scheduler state, familiar identity continuity, runtime
selection/execution, persistence, Cave lifecycle state, or protected effects.
Coven remains the trusted runtime authority and must consume this profile at an
immutable revision.

## Compatibility

The v1 schemas and semantic error codes are additive only within the `v1`
directory. Any incompatible shape, canonicalization, preimage, lifecycle, or
decision-policy change requires a new profile version and new vectors.

The mandatory manifest covers all 18 issue categories and every published edge
mutation. Its Node-core runner must pass from a read-only checkout.
