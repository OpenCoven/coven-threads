# Producer-helper signer parity candidate

Status: engineering candidate for [#99](https://github.com/OpenCoven/coven-threads/issues/99), a follow-up found during [#94](https://github.com/OpenCoven/coven-threads/issues/94) accounting for [#88](https://github.com/OpenCoven/coven-threads/issues/88). It adds no completion dependency to #88's six gaps. Maintainer approval of the typed rule and merged delivery remain required. This packet does not close either Phase-5 human gate.

## Objective and boundaries

Make the Rust producer helper `sign_artifact` refuse an empty signer key id the way the reference's `signArtifact` does, without inventing a credential owner, changing a public Rust signature, or loosening any receiving validator. Threads owns the validator and producer-utility contract; authenticated runtime key configuration, custody and effects stay in Coven. All material is synthetic: ephemeral Ed25519 keys generated in the test process and never written out.

Canonical sources consulted: `specs/AUTOMATION-AUTHORITY-PROFILE-v1.md` §§6–7 and §11; `profiles/automation-authority/v1/validator.mjs` (`signArtifact`, `authoritySigner`, `verifySignedArtifact`, `requireString`, `fail`); the public `ArtifactSigner`, `sign_artifact`, `ErrorCode` and `AuthorityError` contracts in `crates/coven-threads-core/src/automation_authority`; `CONTRIBUTING.md`; `AGENTS.md`'s normative precedence and semver rule. RFC-0001 keeps identity authority upstream and says nothing about producer-helper guards.

## Reproduced divergence

Base `872e05d` (main, including #98). The reference guard is one line:

```js
export function signArtifact(value, domain, signer) {
  if (!signer?.key_id || !signer?.private_key) {
    fail("integrity_signer_required", "a key_id and Ed25519 private_key are required");
  }
```

JavaScript's falsy test covers a missing signer, a missing private key and an empty `key_id`. In Rust the first two are type matters: `sign_artifact` takes `&dyn ArtifactSigner`, and the trait supplies the signature, so there is no representable "no signer" or "no private key". An empty id is representable: `key_id()` returns `&str`. Before this change `sign_artifact` accepted it and produced a well-formed artifact with a real signature filed under `""`.

Red run of the new unit test against unchanged `sign_artifact` (Homebrew rustc 1.99.1, `ring` 0.17.14 as locked, Node v24.22.0, macOS Darwin 27.0.0, 2026-10-28):

```text
test automation_authority::keys::tests::an_empty_signer_key_id_is_refused_before_signing_like_the_reference ... FAILED
assertion `left == right` failed
  left: Ok(Object {"integrity": Object {"alg": String("ed25519"), "key_id": String(""), "signature_b64": String("O073hjO/…"), "signed_digest": String("59276c13…")}, "kind": String("test"), "value": Number(1)})
 right: Err((IntegritySignerRequired, "$"))
```

The reference refuses the same input today; its new regression (`signArtifact refuses an empty key_id or a missing private key before signing (#99)`) passed on first run and guards the behavior the port now mirrors.

## Proposed rule and normative precedence

`sign_artifact` refuses before signing when `signer.key_id()` is empty, with the reference's code, message and default path: `integrity_signer_required`, "a key_id and Ed25519 private_key are required", `$`. Everything else in the helper is unchanged.

Precedence: the spec's §11 lists stable error families "including" the integrity codes and does not name `integrity_signer_required`; it neither forbids nor requires a producer-side guard. The reference validator is the conformance oracle the Rust port is documented to match ("the profile's error codes, as the reference validator raises them"), and it raises this code. No decision record is contradicted. The distinction the issue asks for stays explicit: a missing signer or private key is a type boundary in Rust and has no runtime code; an empty id is the one representable case and gets the reference's code.

Alternatives considered and not proposed:

- Reusing `schema_type` at `$.integrity.key_id`, as the receiving validator would say. Diverges from the reference's producer code for the same condition.
- A `debug_assert!` or panic. A producer bug would take the daemon down instead of refusing one artifact; refusals are the profile's shape for bad input.
- Leaving it to the receiver. The receiver already fails closed (below), but the issue is producer parity, and Coven's signed-decision producer (coven#1602) will call this helper.

## Error-code, API and versioning disposition

- `ErrorCode` gains `IntegritySignerRequired => "integrity_signer_required"`. The enum is `#[non_exhaustive]`, so downstream `match` arms already carry a wildcard and keep compiling. This is an additive change, not an exhaustive-enum expansion; no new semver treatment is triggered. No other exhaustive public type changes.
- `sign_artifact`'s signature is unchanged; it already returns `AuthorityResult<Value>`. Its documented `# Errors` now lists the new refusal first.
- Behavior: a caller whose signer returns an empty id now receives `Err` where it previously received a signed artifact that no valid keyring could verify (a keyring refuses an empty id at `$.keyring.key_id`, and a verifier refuses an empty `$.integrity.key_id`, both `schema_type`). This is corrective and fail-closed. `coven-threads-core` 0.2.0 is unreleased, so the rule ships with the port's first release; the changelog records it. No profile or wire version changes; signed bytes for valid signers are byte-identical.
- No release, version bump, deployment or workflow change is authorized or made here.

## Receiving side stays fail-closed

Both regressions also take a real signature made under a valid id, empty the artifact's `integrity.key_id`, and verify against a keyring that holds the signing key. Node refuses `schema_type` at `$.integrity.key_id` (`requireString`), and the port refuses the same code at the same path (`shape::string`), before any keyring lookup. The producer gap therefore never produced a verifiable artifact; this delivery closes the producer-side parity gap and documents the receiving guard.

## Non-goals

No new credential owner, principal store or issuer. No change to `Keyring`, `verify_signed_artifact`, the general or keyring differential, the 132 vectors, the manifest, the CI workflow or any gate. The reference validator is unchanged. The only spec change is the additive §11 sentence below, applied at the maintainer's direction.

## Verification

Red-to-green above. Gates run individually on the candidate (the pinned 1.88.0 toolchain cannot be installed here, so `scripts/agent-check.sh` refuses to start; CI runs the pinned gate): `cargo fmt`, `cargo clippy -D warnings`, the workspace tests, the 132-vector suite in debug and release and in ordinary and `serde_json/arbitrary_precision` configurations, the reference test suites and vector runner, the script tests, and the general and keyring differentials in both configurations. All passed on the candidate: 236 library tests, 132/132 vectors in debug and release and in both configurations, 29 reference tests, 145 script tests, the general differential at 126,132 cases with 0 mismatched in both configurations, and the keyring differential at 45,603 cases with 0 mismatches in both configurations. The differentials do not exercise `sign_artifact`; they show the existing refusal, result and replay comparisons are untouched. The PR body carries the exact command lines, timings and tool versions.

## Downstream, rollback and remaining decisions

Coven's signed-decision producer (coven#1602) is the intended caller of `sign_artifact`; with this change an empty configured key id surfaces as a refusal at signing time rather than as an unverifiable artifact later. No persistence, scheduling, write or human-authority path changes. Required pinned-daemon CI runs on the PR; it does not establish daemon acceptance.

Rollback: revert the single commit. Restoring acceptance of an empty signer id would restore the producer divergence and should be an explicit maintainer decision.

Decision for Val: approve the typed rule (empty signer id refused with `integrity_signer_required`; missing signer and private key remain type boundaries) and the additive `ErrorCode` variant. The additive §11 clarification offered with this packet, "The reference's producer helper `signArtifact` and the Rust `sign_artifact` refuse a missing or empty signer key id with `integrity_signer_required` before signing; receiving validators independently refuse an empty `integrity.key_id` as `schema_type`.", was approved by Val on 2026-10-29 together with the rule and the variant, and is applied in `specs/AUTOMATION-AUTHORITY-PROFILE-v1.md` §11 in the same PR. It names behavior the reference already had and changes no vector, code or wire format.
