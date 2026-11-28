# What the held audit candidate proves

The November 28 replay reproduced eight audit journeys and 25 read-only census observations. Two journeys reached an applied terminal. Six ended with an opening and no recorded terminal. Those six include deliberately damaged synthetic histories. They explain why a successful diagnostic suite still leaves the universal close requirement unresolved. [November 10 experiment and case descriptions](2026-11-10-audit-census-evidence.md) · [Val’s criterion dispositions](https://github.com/OpenCoven/coven-threads/issues/13#issuecomment-7207324591)

## Revisions and method

The producer was the held Coven test candidate `8edeb939d35cd40c49a58f56153fd3326864087f` with the local Threads core at `aba2e226c6809b823f39e0a838342e9ea330f59b`. Its resolved lockfile SHA-256 was `fb6b254620308c5e1be11c1eaf697723a18223d3a3780bf8ff1807a3b0800a3e`. The retained macOS ARM64 daemon SHA-256 was `68459a80d149d9d80e77c7224dcfc26329c4af58c502d3f651940d92c2e0b3bb`. All five retained executable digests matched the original run record. The original four test targets passed 131/45/39/34. The candidate is an engineering reference pair. Selection of a release or common acceptance revision remains open. [Coven checkpoint](https://github.com/OpenCoven/coven/issues/1716#issuecomment-6767028202)

On November 28, the retained qualifier ran against the original manifests, JUnit, intake, census snapshots and owned-process receipts with a fresh output directory. Its matrix matched the original byte for byte (SHA-256 `9fae1c5ed4c4abcb67467dc8634028660678733c1e8614f2f6638a8bbf63afd3`). It accounted for 24 owned daemon exits, including two intentional interruptions. Fresh offline Cargo tests ran on the exact Threads revision with rustc 1.99.1. The remaining daemon observations below come from the original executions.

| # | Criterion | Evidence on this producer/core pair | Remaining work |
| --- | --- | --- | --- |
| 1 | RFC round-trip | Fresh core runs passed all 14 RFC §5 cases and all 17 C7 portability cases. | Map the Phase-5 defaults and daemon behavior to the RFC on the eventual review pair. |
| 2 | Live Gate-4 revalidation | The recorded final-commit identity-drift case closed the opened window without applying. Output-format auto retained the stronger approval floors. | Trace the final check through committed bytes for each reachable approval path. Record the limit on bounded auto. |
| 3 | Descriptor and writer authority | Protected-intake tests refused alternative proposal/staging writes and rejected a stale protected proposal after restart. | Inventory the reachable daemon and Cave writers, including writers outside the API. |
| 4 | Audit completeness and typed reason | A live window and an interrupted claim with intact Ward each settled once. Malformed pending, malformed claim and expired claim-name controls ended quarantined. The orphan remained open. The synthetic intent and real interrupted claim with missing Ward remained unprovable. | Coven owns recovery and current-pair ordering/loss evidence. Preserve each unresolved history and Val’s exactly-one-close requirement. |
| 5 | Replay hash | The original suite passed restart replay refusal and deadline rejection after valid evidence drift. | Bind the stored hash, rederived hash, decision and final bytes for the reviewed paths. |
| 6 | Visibility and pending state | The daemon live-window control covers pre-deadline observation. The inspected Cave harness at `f3a22518c930e53850b4debcec73db4218eb74d0` requires producer `b07935f0` and core `7b5a9e68`. | Qualify the browser journeys against a reviewed matching producer and lockfile. Evidence for that other producer stays with it. |
| 7 | Tier mapping and load refusal | All 17 retained journey manifests matched the producer/core pair. Four valid configurations returned HTTP 200. The unknown tier and twelve mismatches returned HTTP 500 with `ward_config_invalid`. | Preserve the 17 cases on the eventual review pair, including before/after effects. |
| 8 | Advisory probes | The bounded output-auto case retained stronger ceremonies and kept other probes advisory. | Trace the Gate-3 boundary and document whether a model probe is reachable on the selected pair. |

## Reproduction and handoff

Run the core checks from the named Threads checkout. For the retained daemon evidence, use the original qualifier with a scratch directory containing copies of its source proof and final result and a read-only reference to its journey directory.

```sh
cargo test --offline --locked -p coven-threads-core --test rfc0001_s5_conformance
cargo test --offline --locked -p coven-threads-core --test c7_roundtrip
python3 qualify-lifecycle.py "$scratch"
cmp "$scratch/matrix-qualified.json" "$original/matrix-qualified.json"
```

At Coven main `9415c3098f9635963db593970b8943370e90bf81`, the two-file candidate diff passed an applicability check. The shared allocator refused a thirteenth task worktree under its limit of twelve. The new base still needs the signed candidate, full local gates, Policy guard and a new real-daemon run. The #1716 owner can resume when an owned slot is available. #91 holds the evidence packet for eventual review on #13. [Coven owner checkpoint](https://github.com/OpenCoven/coven/issues/1716#issuecomment-7160042048) · [Freeze deferral](https://github.com/OpenCoven/coven-threads/issues/14#issuecomment-7214040820)
