import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import test from "node:test";

const path = new URL("../lib/daemon-retention.mjs", import.meta.url);
async function implementation() {
  assert(existsSync(path), "native evidence retention is not implemented");
  return import(path);
}

const targets = ["threads_e2e", "threads_identity_invariants", "threads_protected_intake", "threads_terminal_recovery"];
const runners = ["ubuntu-24.04", "macos-15", "windows-2022"];
function fixture() {
  const run = { id: 42, run_attempt: 1, head_sha: "a".repeat(40), status: "completed", conclusion: "success",
    event: "schedule", path: ".github/workflows/daemon-reliability.yml", created_at: "2026-11-02T07:47:00Z",
    run_started_at: "2026-11-02T08:02:00Z", repository: { full_name: "OpenCoven/coven-threads" } };
  const jobs = runners.map((os, i) => ({ id: i + 100, run_id: 42, run_attempt: 1, head_sha: "a".repeat(40),
    name: `Native daemon / ${os}`, status: "completed", conclusion: "success" }));
  const artifacts = runners.map((os, i) => ({ id: i + 200, name: `threads-native-${os}-42-1`, expired: false,
    digest: "sha256:" + "b".repeat(64), workflow_run: { id: 42, head_sha: "a".repeat(40) } }));
  return { run, jobs, artifacts };
}

test("completed metadata yields all three expected OS identities without claiming execution", async () => {
  const { classifyMetadata } = await implementation();
  const result = classifyMetadata(fixture());
  assert.deepEqual(result.rows.map(r => r.state), ["artifact_pending", "artifact_pending", "artifact_pending"]);
  assert.equal(result.run_id, 42);
  assert.equal(result.attempt, 1);
  assert.equal(result.first_attempt, true);
  assert.equal(result.created_at, "2026-11-02T07:47:00Z");
});

test("failed, cancelled, skipped and absent jobs remain unqualified observations", async () => {
  const { classifyMetadata } = await implementation();
  for (const state of ["failure", "cancelled", "skipped", "timed_out", "action_required", "neutral"]) {
    const f = fixture(); f.jobs[0].conclusion = state;
    assert.equal(classifyMetadata(f).rows[0].state, "job_" + state);
  }
  const f = fixture(); f.jobs.shift(); f.artifacts.splice(1, 1); f.artifacts[1].expired = true;
  assert.deepEqual(classifyMetadata(f).rows.map(r => r.state), ["missing_job", "missing_artifact", "expired_artifact"]);
});

test("foreign, duplicate, running or mismatched attempt metadata fails closed", async () => {
  const { classifyMetadata } = await implementation();
  for (const mutate of [f => { f.run.repository.full_name = "Other/repo"; }, f => { f.run.path = ".github/workflows/ci.yml"; },
    f => { f.run.status = "in_progress"; }, f => { f.jobs[0].run_attempt = 2; }, f => { f.jobs.push(f.jobs[0]); },
    f => { f.artifacts.push(f.artifacts[0]); }, f => { f.artifacts[0].workflow_run.id = 99; },
    f => { f.jobs[1].id = f.jobs[0].id; },
    f => { f.jobs[0].head_sha = "d".repeat(40); }, f => { f.run.run_attempt = 0; }]) {
    const f = fixture(); mutate(f);
    assert.throws(() => classifyMetadata(f));
  }
});

test("missing and malformed artifact digests never qualify as authenticated bytes", async () => {
  const { classifyMetadata } = await implementation();
  for (const digest of [undefined, "", "sha256:abc", "md5:" + "b".repeat(64)]) {
    const f = fixture(); f.artifacts[0].digest = digest;
    assert.equal(classifyMetadata(f).rows[0].state, "unverifiable_artifact");
  }
});

test("a rerun is a distinct attempt and never becomes the first attempt", async () => {
  const { classifyMetadata } = await implementation();
  const f = fixture(); f.run.run_attempt = 2;
  f.jobs.forEach(j => { j.run_attempt = 2; });
  f.artifacts.forEach(a => { a.name = a.name.replace("-42-1", "-42-2"); });
  const result = classifyMetadata(f);
  assert.equal(result.first_attempt, false);
  assert.equal(result.attempt, 2);
});

test("a cancelled attempt that never started remains a retained negative observation", async () => {
  const { classifyMetadata } = await implementation();
  const f = fixture(); f.run.conclusion = "cancelled"; f.run.run_started_at = null;
  f.jobs = []; f.artifacts = [];
  const record = classifyMetadata(f);
  assert.equal(record.started_at, null);
  assert.deepEqual(record.rows.map(row => row.state), ["missing_job", "missing_job", "missing_job"]);
});

test("trend counts preserve failed first attempts, missing originals and event separation", async () => {
  const { summarizeRetained } = await implementation();
  const first = { schema_version: "opencoven.threads-native-retention/v1", run_id: 42, attempt: 1, event: "schedule", threads_sha: "a".repeat(40),
    rows: runners.map(os => ({ os, state: "qualified", first_attempt_qualified: true, execution_passed: true })) };
  first.rows[0] = { os: runners[0], state: "job_failure", first_attempt_qualified: false, execution_passed: false };
  const retry = structuredClone(first); retry.attempt = 2;
  retry.rows.forEach(row => { row.first_attempt_qualified = false; row.execution_passed = true; row.state = "job_failure"; });
  const missingOriginal = structuredClone(retry); missingOriginal.run_id = 43;
  const manual = structuredClone(first); manual.run_id = 44; manual.event = "workflow_dispatch";
  const result = summarizeRetained([first, retry, missingOriginal, manual]);
  assert.equal(result.first_attempt_os_observations, 6);
  assert.equal(result.qualified_first_attempt_os_observations, 4);
  assert.equal(result.os_observations_recovered_on_rerun, 1);
  assert.equal(result.missing_first_attempt_runs, 1);
  assert.equal(result.by_event.schedule.first_attempt_os_observations, 3);
  assert.equal(result.by_event.workflow_dispatch.first_attempt_os_observations, 3);
  assert.equal(result.schedule_window_qualified, false);
  assert.throws(() => summarizeRetained([first, first]));
  const duplicateOs = structuredClone(first); duplicateOs.rows[1] = duplicateOs.rows[0];
  assert.throws(() => summarizeRetained([duplicateOs]));
});

test("passing report bindings require all four actual nonzero target results", async () => {
  const { qualifyReport, classifyMetadata } = await implementation();
  const row = classifyMetadata(fixture()).rows[0];
  const observation = { status: "passed", suite: "compatibility", run_id: "42", run_attempt: "1", test_targets: targets,
    coven_sha: "226bfcc89ff6cad4bc9cc9618dad6fea970ecf58",
    threads_sha: "a".repeat(40), local_threads_override_active: true, coven_lock_overlay_sha256: "c".repeat(64) };
  const report = { schema_version: "opencoven.threads-native-reliability/v1", status: "passed", run_id: "42", run_attempt: "1",
    event: "schedule", platform: "linux", runner_os: "Linux", arch: "x64", kernel: "synthetic", node: "v24.22.0", rustc: "rustc 1.95.0 (synthetic)",
    cargo: "cargo 1.95.0 (synthetic)", coven_sha: observation.coven_sha, threads_sha: "a".repeat(40),
    coven_lock_overlay_sha256: "c".repeat(64), first_attempt: true, first_attempt_qualified: true,
    target_runs: targets.map(target => ({ target, status: "passed", command: ["cargo", "test", "--locked", "--message-format=json", "-p", "coven-cli", "--features", "threads-test-clock", "--test", target, "--", "--nocapture"],
      process: { exit_code: 0, signal: null, spawn_error: null },
      counts: { passed: 3, failed: 0, ignored: 0, measured: 0, filtered: 0 },
      executables: [{ target, sha256: "d".repeat(64), file: target }, { target: "coven", sha256: "e".repeat(64), file: "coven" }] })) };
  assert.equal(qualifyReport(row, report, observation).first_attempt_qualified, true);
  for (const [index, nodePlatform, runnerOs] of [[1, "darwin", "macOS"], [2, "win32", "Windows"]]) {
    const platformReport = { ...report, platform: nodePlatform, runner_os: runnerOs, arch: index === 1 ? "arm64" : "x64" };
    const fixtureRow = classifyMetadata(fixture()).rows[index];
    if (index === 1) fixtureRow.job.labels = ["macos-15"];
    assert.equal(qualifyReport(fixtureRow, platformReport, observation).first_attempt_qualified, true);
  }
  const macRow = classifyMetadata(fixture()).rows[1];
  const macReport = { ...report, platform: "darwin", runner_os: "macOS", arch: "x64" };
  macRow.job.labels = ["macos-15-intel"];
  assert.equal(qualifyReport(macRow, macReport, observation).source_series.arch, "x64");
  for (const labels of [undefined, [], ["macos-15"], ["macos-26-intel"], ["macos-15", "macos-15-intel"]]) {
    const wrong = structuredClone(macRow); wrong.job.labels = labels;
    assert.throws(() => qualifyReport(wrong, macReport, observation), /runner|platform/);
  }
  assert.throws(() => qualifyReport(macRow, { ...macReport, arch: "arm64" }, observation), /runner|platform/);
  assert.throws(() => qualifyReport(row, report, { ...observation, run_attempt: "2" }));
  for (const mutate of [r => { r.target_runs.pop(); }, r => { r.target_runs[0].counts.passed = 0; },
    r => { r.target_runs[0].counts.filtered = 1; }, r => { r.target_runs[0].process.exit_code = 7; },
    r => { r.target_runs[0].command.push("--ignored"); }, r => { r.threads_sha = "b".repeat(40); },
    r => { r.run_attempt = "2"; }, r => { r.first_attempt_qualified = false; }]) {
    const bad = structuredClone(report); mutate(bad);
    assert.throws(() => qualifyReport(row, bad, observation));
  }
});
