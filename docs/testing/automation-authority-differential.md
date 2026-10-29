# Automation-authority differential evidence

The mandatory [profile manifest](../../profiles/automation-authority/v1/manifest.json) owns the 132 vectors and their operation/outcome contracts. The general differential compares the Rust production validator with the Node reference over those vectors, all existing structural mutants, raw strict-JSON rewrites, and bounded seeded combinations. The keyring companion adds synthetic role, exact principal, PEM, algorithm, signature, record-order, and duplicate-constructor coverage. Test reports are execution evidence; they are not an authority store, runtime acceptance, or human approval.

## Reproduction and toolchain

[`.node-version`](../../.node-version) pins Node 24.22.0, a supported [Node 24 LTS release](https://nodejs.org/en/download/archive/v24.22.0) checked on 2026-10-28. Bootstrap, the local gate, matrix driver, and relevant CI jobs enforce the exact release. Setup actions use immutable commit refs and cannot be conditional or continue after a failed setup. Rust remains 1.88.0 for this library; the pinned real-daemon compatibility jobs retain their separate existing 1.95.0 consumer toolchain.

```sh
bash scripts/agent-bootstrap.sh
bash scripts/agent-check.sh fast
bash scripts/agent-check.sh full
AUTHORITY_DIFFERENTIAL_SEED=88 AUTHORITY_DIFFERENTIAL_BUDGET=24 bash scripts/authority-differential-matrix.sh all
```

Fast verification runs debug artifacts in ordinary and `serde_json/arbitrary_precision` configurations. Full verification adds both release configurations. The matrix driver can select `debug`, `release`, or `all`; it always runs both feature configurations for the selected profile. Each cell builds real `ring`-backed examples, runs automation-authority library guards and all vectors, copies the actual binaries into a unique evidence directory, probes their feature/debug-assertion configuration, and executes both differentials. No accepting verifier is substituted.

The seed defaults to uint32 `88`. A stable hash of the seed and vector id selects each vector's sub-seed; every sampled case records that sub-seed and its field paths. The per-vector logical case budget defaults to 24 and is bounded to 2–256. Invalid values fail rather than disable the corpus. Actual pair/triple and raw/re-signed counts are reported, including documents too small for the requested sample. This is bounded sampling, not exhaustive pairwise coverage or a claim that every changed field independently causes refusal.

Production sampling uses the existing mutation vocabulary, including legal alternatives, invalid values, object-member deletion, and unknown members. Selected paths cannot overlap; identical serialized mutants are deduplicated. Direct signature fields and unbounded repetition controls are excluded so re-signing cannot erase the selected changes or create an unbounded loop. The general corpus retains its existing explicit signature and `__proto__` cases. The latter preserve the documented fail-closed parser divergence, rather than presenting dropped JavaScript prototype members as parity.

## Exact outputs and retained failures

Every general body crosses the production strict parser as raw text. Specified codes and canonical results compare exactly; an explicit malformed-text or equivalence assertion runs even when the reference throws a non-profile exception. Only genuinely unspecified outcomes use the named-refusal/no-result fallback. Positive single-use controls and the existing replay mutants compare complete final adoption, decision-consumption, and lifecycle states, including non-null states.

The keyring companion carries ordered pairs without a sorting map. It projects all signed artifact domains from the positive vectors through the production validators, with mandatory successful controls. Duplicate constructor ids are compared to the reference's strict JSON duplicate-key boundary; JavaScript Maps themselves cannot retain duplicate ids. Rust constructor behavior, error precedence and migration are documented in the [keyring packet](../reviews/2026-10-28-keyring-differential.md).

Each run reports actual counts by operation and outcome, parser rewrite/equivalence/refusal counts, keyring codes, stateful result counts, exact Node/V8/OpenSSL/platform information, feature probes, and SHA-256 of the input stream, reference and tested binaries. Counts establish what ran; they do not establish broader conformance by themselves. The matrix rejects missing cells, unexpected profiles/features, absent outcomes or controls, empty required coverage, mismatched input hashes, and missing non-null replay-state comparisons.

On a mismatch the runner retains exact synthetic frames, case labels, both answers, the seed/tool tuple and a reproduction command. For the first eight sampled multifault failures, it probes at most eight subsets each with the actual Node/Rust validators to find the smallest failing subset of sampled fields. It retains the full signed artifact; this is not a claim of globally minimal JSON. A diagnostic reduction failure leaves the original mismatch and input intact. Backend crashes, malformed output, and answer-count failures retain a bounded input reproduction with the first missing/invalid answer index; a process failing after all answers is attributed to its last case, explicitly as an inference.

## Required CI composition

The existing required `Rust quality and repository contract` context aggregates the debug quality lane and the release differential lane. Both must succeed; failure, cancellation, skipping, missing outputs or different input hashes refuse the aggregate. Each lane verifies ordinary/arbitrary_precision input identity, and the aggregate compares debug/release hashes for both general and keyring corpora. This follows GitHub's documented [job outputs](https://docs.github.com/en/actions/how-tos/write-workflows/choose-what-workflows-do/pass-job-outputs) and [dependency/result semantics](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#jobsjob_idneeds). No protected setting or required context is removed.

Each execution lane retains a 20-minute budget. It uploads exact-attempt logs, probed binaries, receipts and sanitized failure reproductions with run-id/attempt names and 14-day retention. First failures remain failures; a later run does not rewrite their disposition. The final PR must record measured local and hosted durations, platform/backend tuples, any skips and first-attempt failures. The unchanged default corpus runs on every pull request; larger bounded runs are available through the documented command. No new automatic large scheduled corpus is warranted before the default cost is measured, and none is dispatched by this work.

## Error-code accounting

The reference currently names 152 codes. The fixed-role operation corpus does not produce nine of them, one more than the original eight-code inventory in #88. The runner prints actual observed and unobserved codes on every execution; an unobserved code without a documented disposition is described only as not observed, never automatically as unreachable.

| Code | Public-path disposition at this call graph | Direct or separate evidence |
| --- | --- | --- |
| `decision_request_mismatch` | Full evaluated-decision semantic equality runs before dispatch request bindings. | Private `validate_decision_request_bindings` regression changes request digest/id without claiming an externally reachable outcome. |
| `decision_binding_mismatch` | The same semantic equality fixes correlation and stable binding fields before the later guard. | The binding-guard regression changes correlation and principal binding fields directly. |
| `decision_expired` | The evaluated decision's expiry equals request expiry; the earlier request-expired guard wins. | Private `validate_decision_time` tests inclusive start and exclusive expiry. |
| `dispatch_runtime_downgrade` | Granted capabilities fit the decision-bound runtime, and decision/request/current runtime sets already match. | Private `validate_granted_runtime_capabilities` tests a missing granted capability. |
| `dispatch_runtime_capability_missing` | Approval evaluation already requires requested capabilities in the runtime, followed by semantic/set equality. | Private `validate_approval_runtime_capabilities` tests missing requested support. |
| `runtime_capability_unknown` | Request capability validation emits `capability_unknown` before evaluation reaches this check. | Private `validate_runtime_vocabulary` directly tests an unknown capability. |
| `integrity_role_unknown` | Fixed-role corpus limitation, not an unreachable guard. | Required synthetic keyring companion produces this code with successful controls. |
| `integrity_domain_invalid` | Reachable public digest utility; manifest operations supply nonempty fixed domains. | Existing Rust empty-domain regression and explicit Node utility test. |
| `integrity_signer_required` | Node dynamic producer-utility boundary; Rust requires a typed signer. Empty returned key ids remain representable and differ. | Node missing-material/empty-id utility regression; reproduced Rust empty-id producer parity follow-up [#99](https://github.com/OpenCoven/coven-threads/issues/99). |

The six shadowed Rust guards were extracted without changing their bodies, messages, error codes or call order. Their tests bypass earlier prerequisites explicitly as lower-level guard coverage. They are not evidence that impossible public inputs can reach those codes. No new exhaustive Rust variant or profile-version change is introduced. The separate producer-helper #99 does not become a new blocker for #88's original six gaps.

## Delivery boundaries

Parser delivery remains [#92 / #97](https://github.com/OpenCoven/coven-threads/pull/97), keyring delivery remains [#93 / #98](https://github.com/OpenCoven/coven-threads/pull/98), and reproducibility, multifaults, code accounting and build profiles remain [#94](https://github.com/OpenCoven/coven-threads/issues/94). Completion requires the sibling deliveries and final composed exact-head evidence; local composition does not attest that a sibling has merged. The [parent #88](https://github.com/OpenCoven/coven-threads/issues/88) must map every original gap to merged source and toolchain proof before closure.

The pinned daemon gate retains its four real targets and immutable Coven pin. Library/differential evidence does not certify current-daemon automation effects, production installation, human coherence or freeze. Coven remains the authenticated writer/runtime, Familiar Contract owns identity, Psyche owns orchestration, and SDK/Cave consume authority evidence. Rollback restores the candidate commits together, preserving the matrix/aggregate dependency so release failures cannot become a green required context.
