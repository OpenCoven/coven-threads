# Automation-authority keyring differential candidate

Status: proposed engineering disposition for [#93](https://github.com/OpenCoven/coven-threads/issues/93), covering original [#88](https://github.com/OpenCoven/coven-threads/issues/88) item 2. Maintainer approval and merged delivery remain required. This packet does not close either Phase-5 human gate.

## Objective and boundaries

Exercise synthetic keyring roles, exact principal bindings, malformed key records, PEM encodings, algorithms, cryptographic substitution, and competing first errors through the production Rust validator and real `ring` signature verification, against the Node reference. Preserve the existing 132-vector general differential and both serde_json configurations. No credential issuer, principal store, runtime effect, audit schema, wire profile, or deployment changes are proposed.

Canonical sources consulted: `specs/AUTOMATION-AUTHORITY-PROFILE-v1.md` §§2–3, 6–7, 10–13; `profiles/automation-authority/v1/validator.mjs` (`normalizeKeyring`, `keyObject`, `verifySignedArtifact`, and operation validators); `profiles/automation-authority/v1/keyring.json`, `manifest.json` and positive vectors; public `Keyring`, `SignatureVerifier`, and operation contracts under `crates/coven-threads-core/src/automation_authority`; `docs/testing/e2e-contract.md`; `e2e/compatibility.toml`; `CONTRIBUTING.md`; `agent/manifest.yaml`; and the current delivery ledger. Familiar Contract RFC-0001 §5.1 preserves external authority ownership; it does not prescribe PEM parsing or keyring error order. Its canonical GitHub source was read because no sibling checkout exists locally. Coven's `docs/SAFETY-MODEL.md` was read for the daemon boundary.

## Proposed constructor rule and compatibility

`Keyring::new` retains its existing signature and deferred validation. Every repeated id, including identical records, causes `validate` to return existing `json_duplicate_key` before record validation or signature verification. `record_mut` returns `None` for an ambiguous id. This is a proposed corrective fail-closed change to previously undocumented first-wins behavior, not an assertion that JavaScript Maps can retain duplicate ids. The reference comparison uses its actual strict JSON duplicate-key rejection, while a direct Rust regression exercises a valid signed artifact with duplicated constructor ids.

Migration: callers must supply one unambiguous record per id before constructing the keyring. Do not silently select either duplicate on the caller's behalf; reconcile out-of-band configuration with its authenticated owner. No accepted unique-id artifact changes under this rule. There is no new public enum variant, Rust signature change, wire/profile version, audit migration, or release publication. Record the correction in the next authorized patch release; this work does not independently authorize a release or version bump. Val must approve the behavioral disposition before merge under the repository's maintainer policy.

## Error precedence and transport

Preserve the reference's actual Map iteration order. For an object-origin keyring, JavaScript `Object.entries` visits array-index keys first in numeric order and other keys in insertion order. A Map's own insertion order remains authoritative. The companion batch accepts an array of `(id, record)` pairs and constructs `Keyring::new` directly; it never routes those pairs through a sorting map. Rust's `from_object` follows the supplied serde_json map's iteration order, as its documentation now states. Callers that need original external ordering must retain pairs before parsing into a sorting map.

The corpus deliberately preserves reversed insertion order, integer-like ids, and supplementary/BMP Unicode ids. Sorting the reference input to make an incidental serde_json ordering look equivalent would conceal the discrepancy. Two malformed records must yield the first reference code for the same ordered records. No reference validator or profile precedence is changed.

## PEM behavior

The fixed Ed25519 SPKI prefix commits the first DER object's length, Ed25519 OID, absent parameters, bit-string length, and unused-bit count. Node/OpenSSL accepts a complete such SPKI followed by trailing bytes. The Rust port now extracts exactly those first 32 key bytes; every signature still must verify under that key. It still refuses truncated key bytes, changed algorithms, malformed PEM, key substitution, and tampered digests/signatures. This fixes a demonstrated difference from the canonical reference without replacing reference behavior with a stricter invented encoding contract.

## Corpus and evidence

The independent keyring corpus projects signed request, decision, approval, approval-authorization request/decision, consumption snapshot, proposal, evidence-read, and event-chain artifacts from existing positive vectors through their production validators. Lifecycle results compare the entire canonical final state. Existing replay and dispatch comparisons in the general differential remain unchanged. Known-valid controls must succeed, unknown-role coverage must be nonzero, non-profile Node exceptions require a named Rust refusal with no result, and every specified outcome compares its exact code and result.

The runner reports counts by operation and reference code, platform, exact Node version, and SHA-256 of the input stream, reference, and batch binary. On failure it retains the failing synthetic cases, both answers, original reference inputs, and a batch reproduction command. These are test artifacts, not another authority store.

First observations on base `43897e5`, Node v24.22.0, Homebrew Rust 1.99.1, darwin/arm64: the initial 6,371-case object transport produced 115 mismatches; preserving ordered pairs left 23, all valid-key trailing-DER refusals. The PEM correction made that corpus green. The expanded 10,083-case corpus before the review additions passed with 36 valid controls, 360 unknown-role refusals, and 396 non-profile Node exceptions refused by Rust. These are historical intermediate counts; the final PR receipt must state the final corpus and pinned-toolchain results.

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

## Downstream, rollback, and remaining decisions

This library correction changes no persistence, scheduling, writes, or human authority. Required pinned-daemon CI must run on the final PR composition; do not infer daemon acceptance from library tests or this test companion. Broader automation-authority current-daemon certification remains in Coven's owning issues. No production-profile inspection, workflow dispatch, deployment, or publication is authorized here.

Rollback: revert the candidate commits together; the new example, corpus, and gate invocations form one test-tooling unit. Restoring earlier constructor acceptance would restore duplicate ambiguity and should be an explicit maintainer decision. Keep #88/#93 open until approved merged evidence exists; reconcile with #97 only after its shared files settle. All source and fixtures remain synthetic, with public test keys only.
