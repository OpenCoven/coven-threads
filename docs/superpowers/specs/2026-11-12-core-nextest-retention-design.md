# Core Nextest retained telemetry

Implement the user-approved #110 continuation objective as an additive read-only core CI evidence profile. Native daemon collection and #111's archive, cadence, declarations and source hold remain untouched. Retained core telemetry must preserve first-attempt results, in-process test retries and later workflow attempts separately, including negative and incomplete observations.

The existing native collector deliberately refuses core CI. Extend neither its accepted workflow nor its qualification meaning. A separate `retain-nextest-evidence.mjs` command reuses the existing bounded ZIP extractor, immutable archive publisher and artifact file index. Its canonical identity is run/attempt under a caller-selected dedicated core archive. The archive is CI evidence, never a runtime audit store.

Retain authenticated complete REST snapshots, original ZIP, decoded job log, exact checkout commit/tree, tracked source Git identities and configuration/lock bytes. Bind the legacy generic `nextest-junit` artifact to its unique upload step's time interval; never guess an attempt from an artifact name alone. Bind JUnit UUID and timestamp to the actual Nextest invocation in the job log. Where setup or missing evidence prevents source/tool binding, retain a negative observation with unknown fields rather than inventing identities.

A Python-standard-library parser validates bounded Nextest JUnit, derives suite and global counts from unique testcase identities, and retains terminal failure/error/skipped plus flaky/rerun markers. A retry pass is not a first-attempt pass. Unknown XML structure, duplicate identities, contradictory summaries, external entities, zero tests or unbound reports cannot qualify. Originals remain retained even when parsing refuses.

Deterministic summaries verify archives and replay positive claims, preserve per-event/source/tool/platform scope and observation gaps, and expose original versus later attempt outcomes. They cover observed records only; no expected CI schedule, causal flake diagnosis, native acceptance or elapsed 30-day completion is inferred.

Alternative approaches were broadening the native collector or changing the producer workflow first. The first would blur an explicit acceptance boundary; the second would change held main and could not authenticate existing evidence. The selected additive consumer works against existing completed runs while preserving the source hold. Producer upgrades and automatic cadence remain separate decisions, with actual retention gaps reported explicitly.

Deliver a reviewed unmerged PR, real original-artifact replay, negative controls, appropriate gates, an indexed evidence package, a retention-gap report and exact acceptance dispositions for #107/#109/#115. No merge, workflow dispatch/rerun, native collector modification, pin/check/threshold promotion, window reset, release/deployment, issue closure or human acceptance is authorized.
