## Objective

<!-- What concrete invariant or user-visible outcome changes? -->

## Acceptance criteria

- [ ]

## Non-goals

<!-- State what this PR deliberately does not own or solve. -->

## Risk and authority impact

- Risk class: `R0 | R1 | R2 | R3 | R4`
- Protected surfaces affected:
- Principal/approval implications:
- Audit or privacy implications:
- Migration/compatibility implications:

## Canonical contracts consulted

- [ ] Familiar Contract / RFC-0001, when identity or Ward semantics are involved
- [ ] Relevant decision record under `specs/`
- [ ] Public Rust/SQL contracts and conformance vectors
- [ ] Downstream Coven/Cave contract, when the boundary is crossed

## Evidence

### Before

<!-- For a defect: failing regression or E2E evidence against pre-fix behavior. -->

### Verification commands and results

```text
bash scripts/agent-check.sh fast
bash scripts/agent-check.sh full
```

### Cross-repository canary

- Pinned Coven commit:
- Current Threads checkout proven active: `yes | no | not yet runnable`
- Daemon E2E result/artifact:
- Latest-main canary result:

## Rollback and residual risk

- Rollback:
- Remaining uncertainty:

## Human gates

<!-- Agents may prepare evidence but may not simulate Nova or Val approval. -->

- [ ] No human gate is being self-attested or bypassed
