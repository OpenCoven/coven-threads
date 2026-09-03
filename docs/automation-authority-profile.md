# Automation Authority Profile

The Automation Authority Profile answers one narrow question immediately before
an automation operation: **may this exact familiar operation act now?**

Threads returns `permit`, `requires_approval`, `degrade_to_proposal`, or
`reject`. It binds the answer to the principal, familiar embodiment, automation
revision, occurrence/run/attempt/fence, action, capabilities/scopes,
project/workspace/runtime, and policy/manifest snapshot.
Final dispatch recomputes the canonical policy result from the policy snapshot
pinned in its trusted immutable input. A valid Threads signature cannot make a
semantically widened or forged decision authoritative.

The profile deliberately does not schedule or run anything. Coven owns durable
scheduler state, final revalidation, consumption, launch, and effects.

Final revalidation consumes the raw signed approval-event chain plus a
Threads-signed, revision-bound consumption snapshot. A client-authored
`approved` summary is never authority. Human approval requires the authorized
principal's key; R4 protected-owner approval requires a protected-owner key.
Recurring R2 approval remains narrowly reusable only through exact per-run
signed consumption evidence and a bounded usage counter.
Request, decision, and current runtime capability snapshots must match exactly.
Evidence with missing or unknown sensitivity/retention metadata fails closed,
and impossible UTC calendar timestamps are rejected rather than normalized.

## Why proposal-only matters

A familiar may still reason and prepare when it lacks authority to act. A
proposal is cryptographically tied to the original action and target, visibly
states that it was not executed, performs no protected effect, and requires a
new adoption decision before execution.

## Portable artifacts

The versioned schemas, validator, public conformance keyring, exact manifest,
and 122 vectors are under
[`profiles/automation-authority/v1/`](../profiles/automation-authority/v1/).
The normative contract is
[`specs/AUTOMATION-AUTHORITY-PROFILE-v1.md`](../specs/AUTOMATION-AUTHORITY-PROFILE-v1.md).

Run the dependency-free reference checks:

```sh
node --test profiles/automation-authority/v1/tests/*.test.mjs
node profiles/automation-authority/v1/run-vectors.mjs
```
