import { readFileSync } from "node:fs";
import { compatibilityTargets } from "../run-daemon-canary.mjs";

export const nativeRunners = ["ubuntu-24.04", "macos-15", "windows-2022"];
const platforms = ["linux", "macos", "windows"];
const pin = JSON.parse(readFileSync(new URL("../../e2e/native-daemon-scenarios.json", import.meta.url))).coven_ref;
const sha = value => typeof value === "string" && /^[0-9a-f]{40}$/.test(value);
const digest = value => typeof value === "string" && /^[0-9a-f]{64}$/.test(value);
const id = value => Number.isSafeInteger(value) && value > 0;
const terminalConclusions = ["success", "failure", "cancelled", "skipped", "timed_out", "action_required", "neutral", "stale", "startup_failure"];

export function classifyMetadata({ run, jobs, artifacts }) {
  if (!run || !id(run.id) || !id(run.run_attempt) || !sha(run.head_sha) ||
      run.status !== "completed" || run.repository?.full_name !== "OpenCoven/coven-threads" ||
      run.path !== ".github/workflows/daemon-reliability.yml" ||
      !["schedule", "workflow_dispatch"].includes(run.event) ||
      !terminalConclusions.includes(run.conclusion) || !Number.isFinite(Date.parse(run.created_at)) ||
      !(Number.isFinite(Date.parse(run.run_started_at)) || (run.run_started_at === null && run.conclusion !== "success")) ||
      !Array.isArray(jobs) || !Array.isArray(artifacts)) {
    throw new Error("completed native workflow metadata with explicit source and attempt is required");
  }
  if (new Set(jobs.map(job => job.id)).size !== jobs.length ||
      new Set(artifacts.map(artifact => artifact.id)).size !== artifacts.length) throw new Error("duplicate metadata IDs across pages or OS rows");
  const rows = nativeRunners.map((os, i) => {
    const matches = jobs.filter(job => job.name === `Native daemon / ${os}`);
    const candidates = artifacts.filter(artifact => artifact.name === `threads-native-${os}-${run.id}-${run.run_attempt}`);
    if (matches.length > 1 || candidates.length > 1) throw new Error("ambiguous native job or artifact identity");
    const job = matches[0] ?? null;
    const artifact = candidates[0] ?? null;
    if (job && (!id(job.id) || job.run_id !== run.id || job.run_attempt !== run.run_attempt ||
        job.head_sha !== run.head_sha || job.status !== "completed" ||
        !terminalConclusions.includes(job.conclusion))) {
      throw new Error("job differs from the completed native run/attempt/source");
    }
    if (artifact && (!id(artifact.id) || artifact.workflow_run?.id !== run.id ||
        artifact.workflow_run?.head_sha !== run.head_sha)) throw new Error("artifact belongs to another run/source");
    let state = "artifact_pending";
    if (!job) state = "missing_job";
    else if (job.conclusion !== "success") state = `job_${job.conclusion}`;
    else if (!artifact) state = "missing_artifact";
    else if (artifact.expired !== false) state = "expired_artifact";
    else if (!/^sha256:[0-9a-f]{64}$/.test(artifact.digest ?? "")) state = "unverifiable_artifact";
    return { os, platform: platforms[i], run_id: run.id, attempt: run.run_attempt,
      threads_sha: run.head_sha, event: run.event, state, job, artifact };
  });
  return { schema_version: "opencoven.threads-native-retention/v1", run_id: run.id,
    attempt: run.run_attempt, threads_sha: run.head_sha, event: run.event,
    created_at: run.created_at, started_at: run.run_started_at,
    first_attempt: run.run_attempt === 1, rows };
}

export function qualifyReport(row, report, observation) {
  const first = row.attempt === 1;
  const status = first ? "passed" : "retry-passed";
  const expectedPlatform = { linux: "linux", macos: "darwin", windows: "win32" }[row.platform];
  const expectedRunner = { linux: "Linux", macos: "macOS", windows: "Windows" }[row.platform];
  // The macos-15 evidence slot may contain historical ARM or candidate Intel
  // jobs. Bind the report architecture to the authoritative physical label;
  // never transfer qualification between architectures or accept both labels.
  if (row.platform === "macos") {
    const architectures = { "macos-15": "arm64", "macos-15-intel": "x64" };
    const labels = row.job?.labels;
    if (!Array.isArray(labels) || labels.length !== 1 || !Object.hasOwn(architectures, labels[0]) ||
        report?.arch !== architectures[labels[0]]) {
      throw new Error("macOS runner label and report architecture must agree");
    }
  }
  if (report?.schema_version !== "opencoven.threads-native-reliability/v1" || report.status !== status ||
      report.run_id !== String(row.run_id) || report.run_attempt !== String(row.attempt) ||
      report.event !== row.event || report.threads_sha !== row.threads_sha || report.coven_sha !== pin ||
      !expectedPlatform || report.platform !== expectedPlatform || report.runner_os !== expectedRunner ||
      !report.arch || !report.kernel || report.node !== "v24.22.0" ||
      !/^rustc 1\.95\.0 /.test(report.rustc ?? "") || !/^cargo 1\.95\.0 /.test(report.cargo ?? "") ||
      !digest(report.coven_lock_overlay_sha256) || report.first_attempt !== first ||
      report.first_attempt_qualified !== first ||
      observation?.status !== "passed" || observation.suite !== "compatibility" ||
      observation.run_id !== String(row.run_id) || observation.run_attempt !== String(row.attempt) ||
      JSON.stringify(observation.test_targets) !== JSON.stringify(compatibilityTargets) ||
      observation.local_threads_override_active !== true || observation.coven_sha !== report.coven_sha ||
      observation.threads_sha !== report.threads_sha || observation.coven_lock_overlay_sha256 !== report.coven_lock_overlay_sha256 ||
      !Array.isArray(report.target_runs) || report.target_runs.length !== compatibilityTargets.length) {
    throw new Error("native report does not bind the exact source/platform/attempt and successful guarded observation");
  }
  const seen = new Set();
  let daemon = false;
  for (const result of report.target_runs) {
    const expectedCommand = ["cargo", "test", "--locked", "--message-format=json", "-p", "coven-cli",
      "--features", "threads-test-clock", "--test", result.target, "--", "--nocapture"];
    if (!compatibilityTargets.includes(result.target) || seen.has(result.target) ||
        result.status !== "passed" || result.process?.exit_code !== 0 || result.process.signal !== null ||
        result.process.spawn_error !== null || JSON.stringify(result.command) !== JSON.stringify(expectedCommand) ||
        !id(result.counts?.passed) || ["failed", "ignored", "measured", "filtered"].some(key => result.counts[key] !== 0) ||
        !Array.isArray(result.executables) || result.executables.filter(binary => binary.target === result.target).length !== 1 ||
        result.executables.some(binary => ![result.target, "coven"].includes(binary.target) || !digest(binary.sha256) ||
          typeof binary.file !== "string" || !binary.file || /[\\/]/.test(binary.file))) {
      throw new Error("native target lacks a unique actual passing command/count/executable receipt");
    }
    seen.add(result.target);
    daemon ||= result.executables.some(binary => binary.target === "coven");
  }
  if (!daemon) throw new Error("native report has no real daemon executable identity");
  return { execution_passed: true, first_attempt_qualified: first && row.job?.conclusion === "success",
    source_series: { coven_sha: report.coven_sha, threads_sha: report.threads_sha,
      lock_sha256: report.coven_lock_overlay_sha256, platform: report.platform, arch: report.arch,
      rustc: report.rustc, cargo: report.cargo, node: report.node },
    counts: report.target_runs.map(result => ({ target: result.target, ...result.counts })) };
}

export function summarizeRetained(records) {
  const identities = new Set();
  const groups = new Map();
  for (const record of records) {
    if (record.schema_version !== "opencoven.threads-native-retention/v1" || !id(record.run_id) || !id(record.attempt) ||
        !sha(record.threads_sha) || !["schedule", "workflow_dispatch"].includes(record.event) ||
        !Array.isArray(record.rows) || record.rows.length !== nativeRunners.length ||
        new Set(record.rows.map(row => row.os)).size !== nativeRunners.length ||
        record.rows.some(row => !nativeRunners.includes(row.os) || typeof row.first_attempt_qualified !== "boolean" ||
          typeof row.execution_passed !== "boolean" || typeof row.state !== "string" ||
          (row.first_attempt_qualified && (record.attempt !== 1 || row.state !== "qualified" || !row.execution_passed)))) {
      throw new Error("unsupported or contradictory retained attempt record");
    }
    const key = `${record.run_id}/${record.attempt}`;
    if (identities.has(key)) throw new Error("duplicate retained attempt cannot change denominators");
    identities.add(key);
    const group = groups.get(record.run_id) ?? [];
    if (group.some(prior => prior.event !== record.event || prior.threads_sha !== record.threads_sha)) throw new Error("run scope changed across attempts");
    group.push(record); groups.set(record.run_id, group);
  }
  let qualified = 0, observed = 0, missingFirstAttempts = 0, recovered = 0;
  const byEvent = Object.fromEntries(["schedule", "workflow_dispatch"].map(event => [event,
    { observed_runs: 0, first_attempt_os_observations: 0, qualified_first_attempt_os_observations: 0,
      missing_first_attempt_runs: 0, os_observations_recovered_on_rerun: 0 }]));
  for (const group of groups.values()) {
    const counters = byEvent[group[0].event]; counters.observed_runs += 1;
    const first = group.find(record => record.attempt === 1);
    if (!first) { missingFirstAttempts += 1; counters.missing_first_attempt_runs += 1; continue; }
    for (const row of first.rows) {
      observed += 1; counters.first_attempt_os_observations += 1;
      if (row.first_attempt_qualified) { qualified += 1; counters.qualified_first_attempt_os_observations += 1; }
      else if (group.some(record => record.attempt > 1 && record.rows.some(later => later.os === row.os && later.execution_passed))) {
        recovered += 1; counters.os_observations_recovered_on_rerun += 1;
      }
    }
  }
  return { observed_runs: groups.size, retained_attempts: records.length, first_attempt_os_observations: observed,
    qualified_first_attempt_os_observations: qualified, missing_first_attempt_runs: missingFirstAttempts,
    os_observations_recovered_on_rerun: recovered, by_event: byEvent, schedule_window_qualified: false,
    limitation: "Observed records only; missing schedule slots, 30-day rates and causal flake diagnosis are not inferred." };
}
