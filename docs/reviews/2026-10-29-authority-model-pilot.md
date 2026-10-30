# Authority-transition model pilot (coven-threads#95)

**Status:** research evidence for Val's review. This note makes no human
gate, Phase-5, daemon or release claim. It does not change any authority
semantics, vector, error code or public API. It recommends an adoption
decision; it does not adopt one.

**Source:** branch `research/issue-95-authority-model`, based on main
`cf2d7607d38a2931b1a7f50ad3702d4039977078`.

## 1. Question and scope

Issue #95 asks for a bounded pilot. Choose one small, useful property of the
authority transitions. Show a counterexample on a deliberately broken
transition. Evaluate Kani or an independent model differential with explicit
bounds, and recommend adopt, defer or reject.

The pilot models the **automation-authority approval lifecycle** and the two
at-most-once stores around it:

| Model | Canonical contract | Production code |
| --- | --- | --- |
| Lifecycle states and transitions | `specs/AUTOMATION-AUTHORITY-PROFILE-v1.md` §7 | `apply_lifecycle_event` in `automation_authority/lifecycle.rs` |
| Decision consumption | §6 ("consumable once"; reject and proposal-only never consumable) | `consume_decision` in `automation_authority/decision.rs` |
| Request adoption | §9 ("exactly one matching request adoption") | `adopt_authorization_request` in `automation_authority/request.rs` |

**Kept separate.** The Phase-5 veto-window state space in
`crates/coven-threads-core/src/approval.rs` (`ApprovalPath`,
`WindowCloseReason`) is a different machine under a different decision
record. It is not modeled here, and nothing here is evidence about it.
`verify_dispatch` and its TOCTOU checks (§9) are also out of scope; the
model's consumption step stands for the lifecycle half of that boundary only.

## 2. The model

`crates/coven-threads-authority-model` is a new dev-only workspace crate
(`publish = false`). Its library is pure, allocation-free Rust with no
dependencies.

Every event field the implementation reads is replaced by the **claim** it
makes about the chain it is appended to. The sequence is expected, stale or
skipped. The previous digest is the head or another digest. The subject is
the same or different. The approval digest is null, matching or other. The
approval evidence is supplied or absent. A consumption binds one of four
occurrences, three inside the recurring prefix and one outside it. A
no-launch disposition names the approval's occurrence or another one. The
phase and dispatch disposition keep their full enumerations.

A chain is `{state, sequence, consumption count, consumed-occurrence bitmask,
approved}`. `apply` returns the successor, or **every** rule the event
violates. The implementation reports only its first code, so the
differential checks that this code names one of the model's reasons.

**Bounds.** Four occurrences, three in the prefix. `max_uses` from one to
three; the profile allows up to 366, and the guard is the same comparison at
any value. One approval subject per chain. The explorer's event space has
1,058,400 events per approval: every claim combination, with each kind's
consistent `(phase, disposition)` pair and one pair malformed for every kind.

**Abstracted away.** Signatures (the differential uses real ones), approval
validity windows and `now`, timestamps, string and digest shapes, I-JSON
parsing, and the approval's own validation (the differential always supplies
a valid, signed approval).

## 3. Properties

Checked on every accepted step from every reachable chain:

1. **Terminal absorption.** No event is accepted from `rejected`, `expired`,
   `revoked` or `consumed`, so every opened approval has at most one typed
   terminal state.
2. **Append-only chain.** The sequence advances by exactly one, and the
   subject never changes after the first event.
3. **Immutable approval binding.** The approval digest is introduced by
   `approve` and repeated by every later event, and never claimed before.
4. **Consumption only from `approved`**, and only with approval evidence.
5. **Withdrawal ordering.** Reject, expire and revoke occur only from
   `requested` or `approved`; reject only from `requested`.
6. **Reachable-state invariant.** The sequence is zero exactly at
   `required`. The consumed-occurrence mask has exactly `count` bits. No
   consumption happens without approval. A single-use approval is consumed
   at most once, exactly its bound occurrence, and only in `consumed`. A
   recurring approval stays within `max_uses`, never binds the same
   occurrence twice, never leaves the prefix, and never reaches `consumed`.

For the decision and adoption stores, the pilot checks exhaustively that a
`degrade_to_proposal` or `reject` decision is never consumable. It also checks
that a second consumption or adoption is refused. Those stores are one bit
each, so this check is small; the substantive replay evidence for them
remains the conformance vectors and the existing differential.

## 4. Results

### 4.1 Faithful relation

The explorer applies every event in the space to every reachable chain, for
each approval use. Every property holds on every reachable chain and
accepted step.

| Approval use | Reachable chains | Transitions applied | Accepted | Debug | Release |
| --- | ---: | ---: | ---: | ---: | ---: |
| single use | 9 | 9,525,600 | 16 | 0.28 s | 0.04 s |
| recurring, `max_uses: 1` | 17 | 17,992,800 | 30 | 0.60 s | 0.07 s |
| recurring, `max_uses: 2` | 26 | 27,518,400 | 45 | 0.84 s | 0.11 s |
| recurring, `max_uses: 3` | 29 | 30,693,600 | 51 | 1.01 s | 0.12 s |

Every state the use allows is reached, and each recurring bound is reached,
not merely respected.

### 4.2 Negative controls

Each deliberately broken relation must yield a counterexample, and does.

| Broken rule | Counterexample found |
| --- | --- |
| Erratum 1: a consume judged by its target state alone | A recurring `consume required->approved` as the **first** event, with no `request` or `approve` |
| Recurring bound checked one use late | A second consume of a `max_uses: 1` approval |
| `revoke` accepted from `consumed` | `revoke consumed->revoked` after a single-use consumption |

### 4.3 Finding: erratum 1 was broader than its record

The shortest erratum-1 counterexample is not the case its vector covers
(`16-revoked-approval-cannot-be-consumed`). It is a consume appended to the
**empty** chain. Authority is spent with no `request` or `approve` event, and
the approval evidence is never validated on that path.

The model's counterexample was replayed against the real reference with an
ephemeral Ed25519 Threads key:

```text
validator.mjs at f5dc609^ (pre-erratum): ACCEPTED state=approved consumption_count=1 sequence=1
validator.mjs at cf2d760 (current main): refused lifecycle_transition_invalid
```

The erratum-1 commit lists consume from `revoked`, `expired`, `rejected` or
`requested`. It does not list `required`. Current main and the Rust port
refuse it, so there is **no open defect**. The test
`consume_on_an_empty_chain_is_refused` pins the Rust behavior as an
owner-local regression. The reference's own erratum-1 test in
`tests/review-regressions.test.mjs` covered `expired`, `rejected` and
`requested`; it now also covers the empty chain. That added assertion alone
fails on the pre-erratum validator ("Missing expected exception:
required") and passes on current main. Adding a conformance vector for it would change the
manifest's vector count and is left to Val (§7).

### 4.4 Finding: the single-use count guard is not load-bearing

Dropping the guard `single_use && consumption_count > 0` violates no
property. A single-use consume lands in terminal `consumed`, so a second
consume is refused by the transition table first. The guard is defense in
depth. The test `single_use_count_guard_is_defense_in_depth` records this, so
a future change to the transition table that makes the guard load-bearing
will be noticed.

### 4.5 Model-to-implementation differential

`tests/differential.rs` rebuilds every reachable model chain in the
implementation by replaying its witness path as real events. Events are
signed with an Ed25519 Threads key generated per run, and the approvals are
re-signed with a per-run principal key. No private key is stored or
printed.

From each chain it then applies:

- every single-claim perturbation of every event the model accepts there,
  including the full 42-pair `(phase, disposition)` product;
- every single-claim perturbation of a well-formed event of each kind, so
  terminal, exhausted and not-yet-approved chains are probed where their
  guards live;
- a fixed-seed sample of the full event space (default 300 per chain,
  `AUTHORITY_MODEL_DIFF_SAMPLES` to change it).

Acceptance must agree with the same state, sequence, count and consumed
occurrences. A refusal's first code must name one of the model's reasons.
The test also requires every lifecycle code the model names to be reached.

| Run | Cases | Accepted | Mismatches | Distinct codes | Time |
| --- | ---: | ---: | ---: | ---: | ---: |
| debug, 300 samples per chain | 76,424 | 3,307 | 0 | 25 | 22.0 s |
| release, 300 samples per chain | 76,424 | 3,307 | 0 | 25 | 5.2 s |
| release, 3,000 samples per chain | 295,124 | 3,309 | 0 | 25 | 12.4 s |
| debug, `serde_json/arbitrary_precision` | 76,424 | 3,307 | 0 | 25 | 19.1 s |

The 25 codes are the 12 shape codes the model's `Shape` reason allows and
all 13 chain and consumption codes.

**Cost.** The whole crate's tests take 28.3 s wall time in a debug build on
the machine below, of which the differential is 23.5 s and the explorer
4.4 s; signing and verifying dominate. A release build cuts the run to
6.3 s but costs 12.4 s to compile. In CI, the tests would join the existing
`cargo test --workspace` and nextest jobs, which run in parallel with the
quality job. No new tool, action or workflow is needed.

Measured on macOS 27.0.1 arm64 with Homebrew `rustc 1.99.1`, Node v24.22.0,
`ring 0.17.14` and `serde_json 1.0.150`. The repository pins Rust 1.88.0;
`rustup` is not installed here, so CI on the pinned toolchain is the
authoritative compile and lint check.

**The differential's own negative control.** Substituting erratum 1's
relation for the model must produce mismatches, and does: the model accepts
`consume required->consumed`, and the implementation refuses it with
`lifecycle_transition_invalid`.

### 4.6 Kani

`src/kani.rs` holds five `#[kani::proof]` harnesses over the same model. Four
are inductive steps that assume an arbitrary chain satisfying the invariant,
apply an arbitrary event, and assert the properties. The fifth, the erratum-1
relation, is expected to fail. The model has no loops apart from
`count_ones`, so no unwind bound is needed.

**They were not run.** Kani's install guide requires Rust "installed via
`rustup`", then `cargo install --locked kani-verifier` and `cargo kani setup`,
which downloads its compiler under `~/.kani/`. This machine has Homebrew
Rust without `rustup`. `agent/manifest.yaml` sets
`network_policy: bootstrap-only`, and running Kani through
`model-checking/kani-github-action@v1` would need a new workflow, which the
issue's non-goals exclude without a reviewed decision. The harnesses are
compiled only under `cfg(kani)`, so ordinary builds never see them, and they
are **unverified code**: no result in this note comes from Kani.

To run them where Kani is available:

```bash
cargo install --locked kani-verifier
cargo kani setup
cargo kani -p coven-threads-authority-model
cargo kani -p coven-threads-authority-model --harness erratum_1_is_caught  # expected to fail
```

For this model, Kani would add little the explorer does not. The explorer
already enumerates the whole bounded space exhaustively, and Kani would
prove the same properties over the same bounds. Kani earns its cost on pure
code whose input space cannot be enumerated, such as `canonicalize` or the
scope-narrowing rules, not on a seven-state machine.

## 5. Primary sources and applicability

- **Cedar** ([Cutler et al., arXiv:2403.04651](https://arxiv.org/abs/2403.04651);
  [cedar-spec](https://github.com/cedar-policy/cedar-spec)). Cedar models its
  language in Lean, proves properties there, and runs "differential
  randomized testing (DRT)" between the formalization and the Rust
  implementation. This pilot transfers that **method** at small scale: a
  small executable model, properties checked on the model, and a
  differential that holds the production code to the model. It transfers no
  Cedar policy semantics, and adds no policy engine.
- **Kani** ([install guide](https://model-checking.github.io/kani/install-guide.html);
  [Rust feature support](https://model-checking.github.io/kani/rust-feature-support.html)).
  Bounded model checking of Rust. "Concurrent features are currently out of
  scope for Kani", which "compiles as if it was sequential code". It
  therefore says nothing about the daemon's transaction boundary.
- **Shuttle** ([awslabs/shuttle](https://github.com/awslabs/shuttle)).
  Randomized concurrency testing; "not sound (a passing Shuttle test does
  not prove the code is correct)". This crate has no concurrency, so
  Shuttle does not apply. Concurrent consumption (two dispatchers racing
  one approval) is Coven's transaction boundary, and the existing
  deterministic daemon tests remain its evidence.

Any later interoperability work routes to OpenCoven/.github#16, not to a
second evaluator here.

## 6. Recommendation

| Technique | Recommendation | Why |
| --- | --- | --- |
| Executable model and exhaustive explorer | **Adopt** | Pure Rust with no dependencies. Seconds to run. Its shortest erratum-1 counterexample is a path the 132 vectors do not contain and the erratum record does not name. |
| Model-to-implementation differential | **Adopt** | Holds the port to the model with real signatures, and has its own negative control. |
| Kani | **Defer** | Not runnable here without `rustup` and a new CI workflow. Adds little over exhaustive enumeration for this model. Revisit for pure functions with unenumerable input spaces. |
| Shuttle | **Reject for this repository** | No concurrency here; races belong to Coven's daemon tests. |

"Adopt" means merging this crate so the tests run in the existing
`cargo test --workspace` jobs. That makes them part of the required checks
that run the workspace, at the cost in §4. That is Val's decision. It is
not implied by this note, and if it is declined, the crate can stay on the
branch as a reproducible research record.

## 7. Residuals and decisions for Val

- **Merge or not** (§6). The crate adds no runtime dependency and no
  published artifact.
- **Optional conformance vector.** A vector for consume-on-empty-chain would
  make the erratum-1 record complete in the profile itself. It changes the
  manifest count from 132, which several tests and docs pin, so it needs its
  own change.
- **Kani** remains unexecuted until a `rustup` toolchain or a reviewed CI
  workflow exists.
- **Model bounds.** Proofs hold only within §2's bounds. No claim is made
  for `verify_dispatch`, approval validity windows, signatures, the
  consumption snapshot, or concurrency.

## 8. Reproduction

```bash
cargo test --locked -p coven-threads-authority-model -- --nocapture
AUTHORITY_MODEL_DIFF_SAMPLES=3000 cargo test --locked --release \
  -p coven-threads-authority-model --test differential -- --nocapture
cargo test --locked -p coven-threads-authority-model \
  --features serde_json/arbitrary_precision --test differential
```

Rollback: remove `crates/coven-threads-authority-model`, its workspace
member line and its `Cargo.lock` entry. Nothing else depends on it.
