# Automation Authority Profile v1

**Status:** normative profile v1.0.0; implementation and conformance artifacts
landed for `OpenCoven/coven-threads#29`. This document does not freeze Phase 5
or satisfy any human Nova/Val gate.

**Canonical artifacts:** `profiles/automation-authority/v1/`

**Authority owner:** Threads decides whether an exact automation operation may
execute, wait for approval, degrade to a proposal, or be rejected. Coven remains
the only runtime authority that persists evidence, consumes decisions, launches
work, and performs effects.

## 1. Scope and precedence

This profile is an operation-specific authority contract. It does not define a
scheduler, familiar identity continuity, runtime selection, Cave state, leases,
retries, or exactly-once external effects.

When sources conflict:

1. `familiar-contract` remains canonical for familiar identity.
2. This profile is canonical for automation authorization artifacts and their
   validation.
3. The JSON Schemas define the closed wire shapes.
4. `validator.mjs` defines the portable reference validation and canonical
   preimages.
5. The manifest vectors define required interoperable outcomes.

Unknown versions, fields, action types, capabilities, scope kinds, keys,
algorithms, lifecycle transitions, or ambiguous authority MUST fail closed.

## 2. Artifact set and encoding

The profile publishes these closed JSON Schema 2020-12 artifacts:

- `AutomationAuthorizationRequest`
- `AutomationAuthorizationDecision`
- `AutomationApproval`
- `AutomationApprovalEvent`
- `AutomationProposal`
- `AutomationEvidenceRead`
- common risk, capability, scope, runtime, privacy, and integrity definitions
- the conformance manifest

All objects are closed with `additionalProperties: false`. Consumers MUST parse
the original UTF-8 JSON text before ordinary object decoding and reject:

- duplicate object keys;
- a byte-order mark;
- invalid JSON;
- unpaired Unicode surrogates;
- non-finite numbers;
- integers outside `[-9007199254740991, 9007199254740991]`.

These are the profile's I-JSON interoperability requirements. A parser that
silently keeps the first or last duplicate key does not conform.

## 3. Integrity and authentication

Every authority artifact carries:

```json
{
  "alg": "ed25519",
  "key_id": "out-of-band key identifier",
  "signed_digest": "64 lowercase hexadecimal characters",
  "signature_b64": "Ed25519 signature"
}
```

The digest preimage is exact:

```text
UTF8(domain-separation-string) || 0x00 || UTF8(canonical-json(unsigned-artifact))
```

`unsigned-artifact` is the top-level artifact with `integrity` omitted.
Canonical JSON recursively sorts object keys by ECMAScript UTF-16 ordering,
uses JSON string escaping, preserves array order, and uses the ECMAScript JSON
number serialization accepted by the strict I-JSON parser. The digest is
SHA-256. `signature_b64` is an Ed25519 signature over the raw 32 digest bytes,
not over the hexadecimal text.

| Artifact | Domain |
|---|---|
| request | `opencoven:automation-request:v1` |
| decision | `opencoven:automation-decision:v1` |
| approval | `opencoven:automation-approval:v1` |
| approval event | `opencoven:automation-approval-event:v1` |
| proposal | `opencoven:automation-proposal:v1` |
| evidence read | `opencoven:automation-evidence-read:v1` |

Key material and role assignment are out-of-band authority inputs. Requests
MUST authenticate the requesting principal. Decisions, proposal receipts, and
lifecycle events MUST authenticate Threads authority. Approvals MUST
authenticate the named approving principal/key. The repository keyring is
synthetic conformance material and contains public keys only.

## 4. Authorization request

A v1 request binds:

- authenticated principal ID and proof reference;
- request ID, nonce, adoption key, issue/expiry, and replay context;
- familiar ID and exact embodiment digest supplied by the canonical identity
  system;
- automation ID, definition revision, and definition digest;
- occurrence ID, run ID, attempt number, and fence generation supplied by the
  scheduler owner;
- canonical action type, action digest, declared risk class, and whether a
  non-effecting proposal is representable;
- requested capabilities and bounded scopes;
- project and workspace IDs;
- intended runtime ID, descriptor digest, and capabilities;
- profile, policy, policy digest, manifest, and manifest digest;
- previous approval digest when one is part of the operation;
- conditions, sensitivity, and retention.

Prompt text, model output, tags, runtime names, and client lifecycle labels are
never capability declarations. They may be separately protected inputs, but
cannot widen authority.

Request adoption is single-use. A durable consumer MUST atomically refuse a
previously adopted nonce, adoption key, or request digest. Expired requests are
not adoptable.

## 5. Risk, actions, capabilities, and scopes

### 5.1 Risk classes

| Class | Meaning | Default ceremony |
|---|---|---|
| R0 | read-only/local analysis; no protected mutation | narrow recurring grant may permit |
| R1 | bounded local artifact creation | narrow recurring grant may permit |
| R2 | mutable local state or deterministic migration | per-run human approval |
| R3 | network, credentials, user data, publication, remote effects | proposal-only when safe; otherwise per-run approval |
| R4 | identity, authority, persistence control, release, deletion, security-critical mutation | protected-owner per-run approval or reject |

An action or capability has a minimum risk floor. A requester or model cannot
lower it. Unknown actions and capabilities are errors, not R0.

### 5.2 Capability vocabulary

The v1 vocabulary is:

`analysis.read`, `artifact.write`, `state.mutate`, `network.fetch`,
`network.publish`, `credential.use`, `evidence.read`, `identity.mutate`,
`authority.admin`, `release.publish`, and `resource.delete`.

Credentials, filesystem access, and network access are never inferred from
runtime availability. Runtime capability presence is necessary but not
sufficient; a policy grant and matching scope are also required.

### 5.3 Bounded scopes

Filesystem scopes use `project` or `workspace` roots and a relative path.
Absolute paths, `..`, wildcards, root-wide paths, drive paths, and shallow
recursive grants fail closed.

Network scopes require HTTPS, an exact non-local DNS host, a port, a path prefix
narrower than `/`, and explicit methods. Credential scopes bind an exact
credential reference, audience, and operations. Evidence scopes bind principal,
automation, and retention classes.

Decisions may narrow scopes and partially grant capabilities. A partial grant
must list every denied capability with a stable reason code. It never implies
that an omitted capability was granted.

## 6. Authorization decision

A decision is a signed Threads artifact binding the exact request digest and
also repeating enforcement-critical fields:

- principal and familiar IDs;
- familiar embodiment digest;
- automation revision/digest;
- occurrence/run/attempt/fence;
- action digest;
- project/workspace;
- intended runtime ID, descriptor digest, and capabilities;
- previous approval digest;
- policy/profile/manifest versions and digests.

It records one outcome:

- `permit`
- `requires_approval`
- `degrade_to_proposal`
- `reject`

It also records granted, denied, and degraded capabilities; exact scopes;
validity; approval profile; machine reason codes; producer/verifier identity;
issue/record times; replay semantics; and privacy/retention.

The reference policy permits R0/R1 only when an unexpired, usage-bounded
recurring grant matches principal, familiar, automation, definition digest,
action, risk, capability, and scope. R2 requires per-run approval. Proposal-safe
R3 degrades; other R3 requires approval. R4 requires protected-owner policy to
reach per-run approval and otherwise rejects.

An R2 request must declare both `deterministic_validation` and `rollback_plan`
conditions. A request marked `automation_new` or `automation_imported` requires
review even if an otherwise matching R0/R1 recurring grant exists.

A decision is consumable once. Coven MUST atomically persist a unique decision
ID/digest consumption record with the immutable dispatch binding. A reject or
proposal-only decision is never consumable for dispatch.

## 7. Approval and append-only lifecycle

`AutomationApproval` is immutable signed evidence. It binds:

- approving principal/key and authorized principal;
- exact request and decision digests;
- familiar embodiment;
- automation ID/revision/digest;
- occurrence/run/attempt/fence;
- action digest;
- capabilities and scopes;
- project/workspace;
- runtime ID, descriptor digest, and capabilities;
- issue/expiry, nonce, rationale privacy;
- single-use or bounded recurring-use semantics.

Single-use is the default. Recurring approvals require an occurrence prefix,
expire, are revocable, and are capped at 366 uses. R3/R4 decisions emitted by
the reference policy request per-run approval; a recurring artifact cannot
silently change that decision.

Approval state is not rewritten into the approval. It is derived from signed,
append-only `AutomationApprovalEvent` records chained by sequence and previous
event digest. Pre-approval events bind the intended approval ID plus exact
request/decision digests and carry `approval_digest: null`; the `approve` event
introduces the immutable approval digest, and every post-approval event repeats
it:

```text
required -> requested -> approved | rejected | expired | revoked
approved -> consumed | revoked | expired
```

Only a Threads-authority key may author lifecycle events. A client-authored
`approved`, receipt, or run state is forged state and must fail. Consumption is
valid only from `approved` and is atomic with `launch_authorized`.

Revocation disposition is explicit:

- before launch (`not_started`, `queued`, or `dispatching`):
  `cancel_before_launch`;
- while running: `request_cooperative_cancel` or
  `external_effects_not_rolled_back`.

The latter is deliberately honest: the profile does not claim rollback or
exactly-once external effects.

## 8. Degrade to proposal

A proposal-only decision grants no protected effect. Its proposal artifact MUST
state:

- `status: "not_executed"`;
- `protected_effects_performed: false`;
- `result_claim: "proposal_only"`;
- `requires_new_adoption: true`;
- the original request and action digests;
- the intended target and proposal content digest.

It cannot be rendered as successful execution. It cannot publish, mutate a
protected surface, use credentials, or perform an external action. Adoption
requires a new request and decision. If a safe non-effecting representation
does not exist, the outcome is reject or requires approval, never a fake
proposal.

## 9. Final dispatch and TOCTOU

Immediately before launch, Coven MUST verify from one immutable snapshot:

- request and decision signatures/digests;
- exact principal, familiar embodiment, automation definition, occurrence,
  run, attempt, fence, action, project, workspace, and runtime;
- runtime capability non-downgrade;
- unchanged policy and manifest versions/digests;
- unexpired request, decision, and approval;
- matching approval and `approved` append-only lifecycle head when required;
- unconsumed request, decision, and approval evidence.

The resulting dispatch-binding digest commits to the request digest, decision
digest, and exact snapshot. The unique consumption record and launch
authorization must share the daemon's transaction/immutable commit boundary
where possible. A client must not recompute a decision from partial fields after
commit.

Changed definition, action, runtime, familiar embodiment, principal, project,
workspace, policy, manifest, attempt, occurrence, run, or fence refuses
dispatch. Stale workers cannot recover authority by retrying with a newer
client-authored fence.

## 10. Privacy and evidence reads

Authority evidence is classified independently from action output. Reads bind
the requesting principal and proof, subject principal, automation, maximum
sensitivity, retention classes, validity, and nonce. A principal may read its
own evidence within those bounds. Cross-principal reads require an out-of-band
auditor key role. Subject, sensitivity, or retention mismatch fails closed.

Evidence history is append-only. Revocation or expiry adds evidence; it never
rewrites or deletes the immutable authorization artifacts. Retention enforcement
is Coven's storage lane, not a client or Cave decision.

## 11. Stable semantic error families

The reference validator emits stable codes, including:

- `json_duplicate_key`, `json_non_ijson`, `json_unsafe_integer`;
- `schema_unknown_version`, `schema_unknown_field`;
- `action_unknown`, `capability_unknown`, `action_risk_underclassified`,
  `capability_risk_underclassified`;
- `scope_too_broad`, `scope_kind_unknown`;
- `integrity_key_unknown`, `integrity_role_mismatch`,
  `integrity_digest_mismatch`, `integrity_signature_invalid`;
- `request_replayed`, `decision_replayed`, `approval_replayed`;
- `policy_stale`, `manifest_stale`, `previous_approval_stale`;
- `decision_semantic_mismatch`, `decision_binding_mismatch`;
- `approval_binding_mismatch`, `approval_definition_changed`,
  `approval_expired`, `approval_state_invalid`;
- `lifecycle_replay`, `lifecycle_chain_mismatch`,
  `lifecycle_state_forged`, `lifecycle_actor_forged`;
- `dispatch_stale_fence`, `dispatch_runtime_downgrade`,
  `dispatch_policy_stale`, `dispatch_manifest_stale`;
- `proposal_dispatch_forbidden`, `proposal_effect_forbidden`,
  `proposal_success_forged`;
- `evidence_read_unauthorized`, `evidence_sensitivity_denied`,
  `evidence_retention_denied`.

The manifest pins the exact expected code for every negative vector. Consumers
may add diagnostics, but must not reinterpret a named failure as success.

## 12. Conformance

`manifest.json` exactly enumerates every vector file, operation, category,
positive/negative expectation, semantic error code, and expected outcome.
`run-vectors.mjs` uses Node core only and performs no writes.

The 18 required categories are:

1. R0 permit
2. narrow recurring R1
3. R2 approval
4. R3 proposal-only
5. R4 rejection
6. unknown/malformed authority
7. scope narrowing/broad refusal
8. partial grant
9. non-effecting proposal
10. single-use and replay consumption
11. expiry/revocation/replay
12. changed bound inputs
13. confused deputy
14. runtime downgrade/substitution
15. TOCTOU policy/manifest/fence
16. queued/dispatching/running revocation
17. tampering/forged lifecycle
18. authorized/unauthorized evidence reads

Relevant edge vectors cover duplicate keys, unsafe integers, invalid Unicode,
unknown versions/fields, prompt-based escalation, risk lowering, wildcard
scope, request/decision replay, signed semantic forgery, and proposal success
forgery.

Run:

```sh
node --test profiles/automation-authority/v1/tests/*.test.mjs
node profiles/automation-authority/v1/run-vectors.mjs
```

## 13. Integration obligations and non-goals

Coven integration (`OpenCoven/coven#857`) must pin these artifacts at an
immutable revision and prove its daemon boundary consumes them. SDK, Cave,
Psyche, and independent validators may display or transport artifacts, but
cannot author Threads decisions, approval lifecycle state, or success receipts.

This profile does not implement:

- schedules, occurrences, runs, attempts, leases, retries, or liveness;
- familiar root/revision continuity;
- runtime selection or execution;
- persistence, credentials, or protected effects;
- Cave-authored authority state;
- exactly-once external effects.
