# Automation-authority keyring differential evidence

Status: #93 and #88 item 2 delivered through [PR #98](https://github.com/OpenCoven/coven-threads/pull/98), checked head `544e46d448b4ea7946e92783cdeac489dddbdffb`, actual merge `872e05d0bdba9ac87cadf4ef9059848b1802a4c4`. The actual merge tree `e9e8998734fc7cdc00e2827b4534fd75d092749f` matches the frozen local full gate and GitHub test merge. Val approved duplicate-id refusal/migration and the public-key-only contract with explicit Node private-container divergence. This delivery does not close either Phase-5 human gate.

## Objective and boundaries

Exercise synthetic keyring roles, exact principal bindings, malformed key records, PEM encodings, algorithms, cryptographic substitution, and competing first errors through the production Rust validator and real `ring` signature verification, against the Node reference. Preserve the existing 132-vector general differential and both serde_json configurations. No credential issuer, principal store, runtime effect, audit schema, wire profile, or deployment changes are proposed.

Canonical sources consulted: `specs/AUTOMATION-AUTHORITY-PROFILE-v1.md` §§2–3, 6–7, 10–13; `profiles/automation-authority/v1/validator.mjs` (`normalizeKeyring`, `keyObject`, `verifySignedArtifact`, and operation validators); `profiles/automation-authority/v1/keyring.json`, `manifest.json` and positive vectors; public `Keyring`, `SignatureVerifier`, and operation contracts under `crates/coven-threads-core/src/automation_authority`; `docs/testing/e2e-contract.md`; `e2e/compatibility.toml`; `CONTRIBUTING.md`; `agent/manifest.yaml`; and the current delivery ledger. Familiar Contract RFC-0001 §5.1 preserves external authority ownership; it does not prescribe PEM parsing or keyring error order. Its canonical GitHub source was read because no sibling checkout exists locally. Coven's `docs/SAFETY-MODEL.md` was read for the daemon boundary.

## Constructor rule and compatibility

`Keyring::new` retains its existing signature and deferred validation. Every repeated id, including identical records, causes `validate` to return existing `json_duplicate_key` before record validation or signature verification. `record_mut` returns `None` for an ambiguous id. This is an approved corrective fail-closed change to previously undocumented first-wins behavior, not an assertion that JavaScript Maps can retain duplicate ids. The reference comparison uses its actual strict JSON duplicate-key rejection, while a direct Rust regression exercises a valid signed artifact with duplicated constructor ids.

Migration: callers must supply one unambiguous record per id before constructing the keyring. Do not silently select either duplicate on the caller's behalf; reconcile out-of-band configuration with its authenticated owner. No accepted unique-id artifact changes under this rule. There is no new public enum variant, Rust signature change, wire/profile version, audit migration, or release publication. Record the correction in the next authorized patch release; this work does not independently authorize a release or version bump. Val approved this behavioral disposition before the normal merge under the repository's maintainer policy.

## Error precedence and transport

Preserve the reference's actual Map iteration order. For an object-origin keyring, JavaScript `Object.entries` visits array-index keys first in numeric order and other keys in insertion order. A Map's own insertion order remains authoritative. The companion batch accepts an array of `(id, record)` pairs and constructs `Keyring::new` directly; it never routes those pairs through a sorting map. Rust's `from_object` follows the supplied serde_json map's iteration order, as its documentation now states. Callers that need original external ordering must retain pairs before parsing into a sorting map.

The corpus deliberately preserves reversed insertion order, integer-like ids, and supplementary/BMP Unicode ids. Sorting the reference input to make an incidental serde_json ordering look equivalent would conceal the discrepancy. Two malformed records must yield the first reference code for the same ordered records. No reference validator or profile precedence is changed.

## PEM behavior

The public Ed25519 SPKI decoder checks the exact Ed25519 OID, absent algorithm parameters, a single public BIT STRING, and exactly 32 resulting key bytes. It preserves Node/OpenSSL's accepted BER long-form and indefinite lengths, bounded nested single-child BIT STRING wrappers, unused-bit masking, and ignored bytes after the complete outer SPKI. It refuses extra internal elements, algorithm substitution, truncated keys, excessive length octets, split BIT STRING children, and malformed framing. Every signed artifact still verifies through the caller's real crypto backend under the decoded key. This replaces the earlier fixed-prefix decoder, which omitted valid Node-accepted public SPKI encodings.

The expanded PEM boundary work preserves plain-text preambles/postambles, the first public-key block, an initial UTF-8 BOM, and valid marker-line whitespace. It refuses inline markers, leading marker indentation, CR-only framing, repeated/interior blank separators, and invalid body characters instead of accepting them through a whitespace-stripping decoder. The implementation was checked against the public OpenSSL 3.5.8 PEM reader and Node v24.22.0 crypto adapter source and real public-only BER/signature probes; this is public SPKI compatibility, not general-purpose OpenSSL container decoding.

Val approved the public-key-only verification contract and explicit Node private-key divergence on 2026-10-28. Paired Rust/Node tests construct the same synthetic seed and private encoding only in memory, sign through real crypto, verify the public control, and assert the intentional private-container difference. Neither private PEM nor private reference input is retained in differential artifacts. The profile clarification records the caller migration: export public SPKI before constructing the Rust verification keyring.

Expanded red evidence: 31,803 cases, 1,608 mismatches; the direct Rust public-PEM boundary regression failed before the parser correction. After correction, ordinary-mode comparison produced zero mismatches, 60 valid controls, 600 unknown-role refusals, and 2,412 non-profile exceptions refused. Input SHA-256: `a3b7e574f728aa3e59d1cc6a31a5143bac8c66e70f4130d1c94850abc659b154`. Review subsequently found accepted BER encodings missing from the initial corpus; the direct long-form-length test failed before the bounded public-SPKI correction. The final merged evidence below supersedes this intermediate result; earlier 16,803-case hosted receipts remain historical.

## Corpus and evidence

The independent keyring corpus projects signed request, decision, approval, approval-authorization request/decision, consumption snapshot, proposal, evidence-read, and event-chain artifacts from existing positive vectors through their production validators. Lifecycle results compare the entire canonical final state. Existing replay and dispatch comparisons in the general differential remain unchanged. Known-valid controls must succeed, unknown-role coverage must be nonzero, non-profile Node exceptions require a named Rust refusal with no result, and every specified outcome compares its exact code and result.

The runner reports counts by operation and reference code, platform, exact Node version, and SHA-256 of the input stream, reference, and batch binary. On failure it retains the failing synthetic cases, both answers, original reference inputs, and a batch reproduction command. These are test artifacts, not another authority store.

First observations on base `43897e5`, Node v24.22.0, Homebrew Rust 1.99.1, darwin/arm64: the initial 6,371-case object transport produced 115 mismatches; preserving ordered pairs left 23, all valid-key trailing-DER refusals. The PEM correction made that corpus green. The expanded 10,083-case corpus before the review additions passed with 36 valid controls, 360 unknown-role refusals, and 396 non-profile Node exceptions refused by Rust. These are historical intermediate counts; the final delivery evidence below states the final corpus and pinned-toolchain results.

Focused regressions `duplicate_constructor_ids_are_refused_before_record_validation` and `pem_uses_the_first_complete_spki_like_the_reference_crypto_backend` both failed before the production fix and passed afterward. Review prompted direct swapped-order and signed-duplicate regressions and expansion to decisions and lifecycle events. The original Homebrew evidence is not substituted for the required Rust 1.88.0 gates; the pinned toolchain was bootstrapped task-locally for final verification.

```sh
bash scripts/agent-bootstrap.sh
bash scripts/agent-check.sh fast
bash scripts/agent-check.sh full
cargo test --locked -p coven-threads-core --lib automation_authority::keys
cargo build --locked -p coven-threads-core --example automation_authority_keyring_batch
node scripts/automation-authority-keyring-differential.mjs target/debug/examples/automation_authority_keyring_batch --ordered
```

Both gates retain the existing general differential and add the companion in ordinary and `serde_json/arbitrary_precision` configurations. Original vectors, refusal/no-result comparisons, dispatch results, and replay states remain required. Release-profile, pinned-Node, bounded multifault, and unreachable-code accounting remain #94 work, with #92/#93 integration dependencies.

## Final merged evidence

`bash scripts/agent-check.sh full` passed at the checked/merged tree on task-local rustc/cargo 1.88.0, Node v24.22.0 / OpenSSL 3.5.8, macOS 27.0.1 darwin/arm64, and gitleaks 8.30.1. Both unchanged guards passed; 145 script tests, 3 secret-guard integration tests, 28 Node tests, 235 Rust library tests plus integration suites, all 132 vectors, formatting, warnings-denied Clippy, and the complete locked workspace passed. One pre-existing doctest was ignored; Cargo.lock did not change.

Both serde_json configurations compared 126,132 specified general outcomes with zero mismatches and required refusal of all 1,601 non-profile exceptions. The expanded keyring corpus compared 45,603 cases in each configuration, with 60 successful controls, 34,404 successes, 600 unknown-role refusals, 2,948 non-profile exceptions refused, and zero mismatches. Input SHA-256 is `11438594ce47e7cfb92ca968644f7f47b9be8d3e491185dc408615032b245db6`; the reference SHA-256 is `89aec82f9c20092007f24c64ef9c0e6aa66f86986925a71ead7edd55d8c2b8a0`.

All seven hosted checks passed at the checked head. The [quality job](https://github.com/OpenCoven/coven-threads/actions/runs/40711940948/job/122899242586) ran in 666 seconds on Node v22.23.4 / OpenSSL 3.5.9, Linux x64, and reproduced the same keyring input digest with zero mismatches in both feature modes. The [pinned-daemon job](https://github.com/OpenCoven/coven-threads/actions/runs/40711940920/job/122899243113) passed its real four targets (104/44/38/33) in 279 seconds at Coven `226bfcc89ff6cad4bc9cc9618dad6fea970ecf58`. Node enforcement and the release matrix are separately owned by #94.

The first expanded full attempt stopped at the unchanged secret guard on a literal synthetic test marker, with no encoded private key in source. The test constructs that marker at runtime; both guards then passed unchanged. A superseded verification run was stopped when review found missing BER cases. Final full verification used frozen source/index, and engineering review found no remaining blockers before the authorized normal merge. Historical failures and retries remain recorded in the [delivery receipt](https://github.com/OpenCoven/coven-threads/issues/93#issuecomment-6472871575), not rewritten as first-attempt success.

## Downstream, rollback, and remaining decisions

This library correction changes no persistence, scheduling, writes, or human authority. The final composition passed the required pinned-daemon compatibility check; do not infer broader daemon acceptance from library tests or this test companion. Broader automation-authority current-daemon certification remains in Coven's owning issues. No production-profile inspection, workflow dispatch, deployment, or publication is authorized here.

Rollback: revert the candidate commits together; the new example, corpus, and gate invocations form one test-tooling unit. Restoring earlier constructor acceptance would restore duplicate ambiguity and should be an explicit maintainer decision. #93 closed only after approved merged evidence was verified. Keep #88 open until all original gaps have merged evidence; compose #97/#94 with the actual delivered source. All fixtures remain synthetic, with only public test keys retained; private boundary-test material exists only in test memory.
