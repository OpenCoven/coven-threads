# AGENTS.md — coven-threads

`coven-threads` is OpenCoven's **protected-authority validator**. It defines the
typed gate, weave, approval, audit-schema, and portability contracts imported by
the trusted `coven` daemon.

This root guide is a router. Keep temporary incidents, implementation diaries,
and mutable interaction history out of it.

## Canonical ownership

This repository owns:

- protected-surface mutation verdict semantics;
- Thread / Weave / Strand / Channel validation contracts;
- approval-path, veto-window, replay, and typed close semantics;
- the `ward_audit` record and SQLite schema/migration contract;
- `.weave` portability and conformance vectors.

This repository does **not** own:

- familiar identity definition or identity continuity (`familiar-contract`);
- principal credential issuance, rotation, recovery, or daemon authentication;
- filesystem writes, SQLite connections, staging, scheduling, apply, or restart
  recovery (`coven`);
- task/lane/lease/attempt lifecycle (`psyche`);
- UI policy or approval authority (`coven-cave`).

Any change that creates a second canonical writer for one of those concerns must
stop for an architecture decision.

## Normative precedence

When sources disagree, use this order:

1. `familiar-contract/rfcs/RFC-0001-familiar-contract.md` for familiar identity
   and Ward requirements;
2. frozen or active decision records under `specs/` for Threads semantics;
3. public Rust contracts and conformance vectors in
   `crates/coven-threads-core`;
4. explanatory material under `docs/`.

The daemon is the runtime trust boundary. A client, prompt, descriptor, model,
or familiar self-report is never authority merely because it names an allowed
state.

## Start here

1. Read [`agent/manifest.yaml`](agent/manifest.yaml).
2. Read [`docs/phases.md`](docs/phases.md) for the honest delivery ledger.
3. Read the relevant decision record under `specs/`.
4. For boundary work, read
   [`docs/testing/e2e-contract.md`](docs/testing/e2e-contract.md) and
   [`e2e/compatibility.toml`](e2e/compatibility.toml).
5. Inspect the exact public types, predicates, SQL, and tests before proposing a
   change.

## Bootstrap and verification

```bash
bash scripts/agent-bootstrap.sh
bash scripts/agent-check.sh fast
bash scripts/agent-check.sh full
```

The fast gate verifies pinned toolchain parity, repository contract metadata,
formatting, Clippy with warnings denied, and library tests. The full gate runs
the complete locked workspace suite.

Useful targeted suites:

```bash
cargo test --locked -p coven-threads-core --test rfc0001_s5_conformance
cargo test --locked -p coven-threads-core --test c7_roundtrip
cargo test --locked -p coven-threads-core --test phase5_retired_ward_corpus
cargo nextest run --locked --workspace --profile ci
```

`cargo test --locked --workspace` remains the compatibility source of truth.
Nextest adds JUnit and flake telemetry; it does not redefine test semantics.

## R4 change rules

Treat changes to identity predicates, validation, approvals, audit SQL,
migrations, portability, release workflows, or frozen specs as R4:

- unknown, stale, unauthenticated, or ambiguous input fails closed;
- descriptors are derived; predicates and committed evidence are authoritative;
- protected proposal routes may not become protected-write authority;
- every opened veto window receives exactly one typed terminal close;
- final Gate-4 revalidation may not be separated from committed binding bytes by
  a client-controlled TOCTOU gap;
- audit schema changes include classification, migration, rollback, and
  fingerprint tests;
- public exhaustive Rust changes receive the correct semver treatment;
- never add a second audit store;
- never use real familiar declarations, personal data, or production secrets in
  fixtures.

## Cross-repository work

True end-to-end proof necessarily crosses into `OpenCoven/coven`. The required
shape is:

```text
current Threads checkout
  -> pinned real Coven daemon
  -> supported request boundary
  -> ephemeral filesystem / coven.sqlite3 / pending state
  -> scheduler, replay, restart
  -> optional Cave live-daemon acceptance
```

Do not call `gate_protected_edits` directly and label the result E2E. Prove the
local Cargo override is active before running downstream tests. Use the stable
full-SHA pin for required pull-request checks and `coven/main` only as a
scheduled canary.

## Work tracking

Use a GitHub issue for work that affects public contracts or crosses
repositories. Link the corresponding `threads-*` bead when Beads tooling is
available, but:

- clean-clone bootstrap and CI must not depend on a personal Beads database;
- do not commit local interaction logs merely to satisfy task bookkeeping;
- schemas and deterministic fixtures may be versioned; mutable operational
  history is not protocol source.

Phase gates matter. Phase N+1 does not silently bypass an unresolved freeze or
sign-off gate.

## Completion evidence

Every authority-relevant pull request must state:

- objective, acceptance criteria, and non-goals;
- exact files and canonical contracts consulted;
- authority, privacy, compatibility, and migration impact;
- exact commands and results;
- lower-level regression evidence;
- downstream canary status or an explicit reason it is not yet runnable;
- rollback path and remaining uncertainty.

For a named Phase-5 blocker, closure requires red -> production fix -> green
through the real daemon boundary as described in the E2E contract.

## Human authority

Nova's independent coherence review and Val's freeze are human gates. Agents may
prepare evidence and recommendations. They must never simulate, self-attest, or
close those gates on a human's behalf.

## Familiar lanes

- 🌿 **Sage** — contract synthesis, conformance mapping, evidence packets.
- 🔮 **Echo** — predicate, audit, replay, and substrate-authority review.
- 👑 **Nova** — daemon integration and independent coherence review.
- ⚡ **Cody** — Rust contracts, regressions, and deterministic test seams.
- ✨ **Charm** — Cave language and live-daemon human acceptance.

GitHub accountability remains with an authenticated maintainer until a familiar
has independently authenticated repository authority.

## Related repositories

- `OpenCoven/familiar-contract` — upstream familiar identity contract.
- `OpenCoven/coven` — daemon, persistence, staging, scheduler, and apply owner.
- `OpenCoven/coven-cave` — human oversight surface; forwards, never decides.
- `OpenCoven/coven-memory` — promotion producer; Threads decides whether writes
  may commit.
- `OpenCoven/psyche` — orchestration consumer of authority-bound snapshots.
