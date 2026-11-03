import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { writeArtifactIndex } from "../lib/daemon-reliability-evidence.mjs";
import { verifyArtifactIndex } from "../lib/daemon-artifact-archive.mjs";

const modulePath = new URL("../retain-daemon-evidence.mjs", import.meta.url);
async function implementation() {
  assert(existsSync(modulePath), "read-only retention collector is not implemented");
  return import(modulePath);
}
const hash = value => createHash("sha256").update(value).digest("hex");
const targets = ["threads_e2e", "threads_identity_invariants", "threads_protected_intake", "threads_terminal_recovery"];
function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), "threads-collector-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const run = { id: 42, run_attempt: 1, head_sha: "a".repeat(40), status: "completed", conclusion: "failure",
    event: "schedule", path: ".github/workflows/daemon-reliability.yml", created_at: "2026-11-02T07:47:00Z",
    run_started_at: "2026-11-02T08:02:00Z", repository: { full_name: "OpenCoven/coven-threads" } };
  const jobs = ["ubuntu-24.04", "macos-15", "windows-2022"].map((os, i) => ({ id: 100 + i, run_id: 42,
    run_attempt: 1, head_sha: "a".repeat(40), name: `Native daemon / ${os}`, status: "completed", conclusion: "failure" }));
  const f = { root, run, jobs, artifacts: [], zip: null };
  f.read = endpoint => {
    if (endpoint.includes("/jobs?")) return Buffer.from(JSON.stringify({ total_count: f.jobs.length, jobs: f.jobs }));
    if (endpoint.includes("/artifacts?")) return Buffer.from(JSON.stringify({ total_count: f.artifacts.length, artifacts: f.artifacts }));
    if (endpoint.endsWith("/zip")) return f.zip;
    if (endpoint.endsWith("/attempts/1")) return Buffer.from(JSON.stringify(f.run, null, 2));
    throw new Error("unexpected read");
  };
  return f;
}
function addValidSyntheticArtifact(f) {
  const dir = join(f.root, "producer"); mkdirSync(dir);
  const inventory = JSON.parse(readFileSync(new URL("../../e2e/native-daemon-scenarios.json", import.meta.url)));
  const scenarios = [...inventory.common, ...inventory.unix, ...inventory.linux].sort();
  for (const scenario of scenarios) {
    const runId = scenario + "-unique";
    const path = join(dir, "journeys", runId); mkdirSync(join(path, "logs"), { recursive: true });
    writeFileSync(join(path, "manifest.json"), JSON.stringify({ run_id: runId, scenario, result: "passed",
      setup_completed: true, fixture_evidence_captured: true, coven_commit: inventory.coven_ref,
      threads_commit: "a".repeat(40), threads_dirty: false, coven_lock_sha256: "c".repeat(64),
      local_threads_override_active: true, platform: "linux" }));
    writeFileSync(join(path, "junit.xml"), `<testsuite name="threads-e2e" tests="1" failures="0"><testcase classname="threads_e2e" name="${scenario}"/></testsuite>`);
    writeFileSync(join(path, "request.json"), "null"); writeFileSync(join(path, "response.json"), "null");
    writeFileSync(join(path, "logs", "daemon.log"), "synthetic tooling control; no daemon executed\n");
  }
  const runs = targets.map(target => ({ target, status: "passed",
    command: ["cargo", "test", "--locked", "--message-format=json", "-p", "coven-cli", "--features", "threads-test-clock", "--test", target, "--", "--nocapture"],
    process: { exit_code: 0, signal: null, spawn_error: null }, counts: { passed: 3, failed: 0, ignored: 0, measured: 0, filtered: 0 },
    executables: [{ target, file: target, sha256: "d".repeat(64) }, { target: "coven", file: "coven", sha256: "e".repeat(64) }] }));
  const observation = { status: "passed", suite: "compatibility", run_id: "42", run_attempt: "1", test_targets: targets,
    coven_sha: inventory.coven_ref, threads_sha: "a".repeat(40), local_threads_override_active: true,
    coven_lock_overlay_sha256: "c".repeat(64) };
  const report = { schema_version: "opencoven.threads-native-reliability/v1", status: "passed", run_id: "42", run_attempt: "1",
    event: "schedule", platform: "linux", runner_os: "Linux", arch: "x64", kernel: "synthetic", node: "v24.22.0",
    rustc: "rustc 1.95.0 (synthetic)", cargo: "cargo 1.95.0 (synthetic)", first_attempt: true, first_attempt_qualified: true,
    coven_sha: inventory.coven_ref, threads_sha: "a".repeat(40), coven_lock_overlay_sha256: "c".repeat(64), target_runs: runs,
    journeys: { manifests: 112, junit_cases: 112, scenarios } };
  for (const [name, data] of [["reliability.json", report], ["observation.json", observation], ["targets.json", runs]]) {
    writeFileSync(join(dir, name), JSON.stringify(data));
  }
  for (const target of targets) writeFileSync(join(dir, `${target}.stdout.log`), "test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n");
  writeArtifactIndex(dir);
  const archive = join(f.root, "synthetic.zip");
  execFileSync("python3", ["-I", "-c", "import sys,pathlib,zipfile\nr=pathlib.Path(sys.argv[1])\nwith zipfile.ZipFile(sys.argv[2],'w') as z:\n for p in sorted(r.rglob('*')):\n  if p.is_file(): z.write(p,p.relative_to(r).as_posix())", dir, archive]);
  f.zip = readFileSync(archive);
  f.artifacts = [{ id: 200, name: "threads-native-ubuntu-24.04-42-1", expired: false,
    digest: "sha256:" + hash(f.zip), workflow_run: { id: 42, head_sha: "a".repeat(40) } }];
  f.jobs[0].conclusion = "success";
}

test("failed setup attempts retain original metadata and remain idempotent negative records", async (t) => {
  const { collectAttempt } = await implementation();
  const f = fixture(t), destination = join(f.root, "archive");
  const first = collectAttempt(42, 1, destination, f.read);
  assert.equal(first.state, "created");
  assert.deepEqual(first.record.rows.map(r => r.state), ["job_failure", "job_failure", "job_failure"]);
  assert(first.record.rows.every(r => r.first_attempt_qualified === false));
  assert.equal(readFileSync(join(first.destination, "metadata", "run.json"), "utf8"), JSON.stringify(f.run, null, 2));
  const before = readFileSync(join(first.destination, "record.json"));
  assert.equal(collectAttempt(42, 1, destination, f.read).state, "identical");
  f.jobs[0].conclusion = "cancelled";
  assert.throws(() => collectAttempt(42, 1, destination, f.read));
  assert.deepEqual(readFileSync(join(first.destination, "record.json")), before);
  for (const name of readdirSync(destination).filter(name => name.startsWith(".collect-"))) verifyArtifactIndex(join(destination, name));
});

test("full synthetic archived bytes qualify only the explicitly successful OS row", async (t) => {
  const { collectAttempt, summarizeArchive } = await implementation();
  const f = fixture(t); addValidSyntheticArtifact(f);
  const result = collectAttempt(42, 1, join(f.root, "archive"), f.read);
  assert.equal(result.record.rows[0].state, "qualified");
  assert.equal(result.record.rows[0].journeys.manifests, 112);
  assert.deepEqual(result.record.rows.map(r => r.first_attempt_qualified), [true, false, false]);
  assert.deepEqual(readFileSync(join(result.destination, "ubuntu-24.04", "original.zip")), f.zip);
  verifyArtifactIndex(result.destination);
  assert.equal(typeof summarizeArchive, "function", "retained archive trend audit is not implemented");
  const summary = summarizeArchive(join(f.root, "archive"));
  assert.equal(summary.first_attempt_os_observations, 3);
  assert.equal(summary.qualified_first_attempt_os_observations, 1);
  assert.equal(summary.schedule_window_qualified, false);
  const report = join(result.destination, "record.json");
  writeFileSync(report, "tampered");
  assert.throws(() => summarizeArchive(join(f.root, "archive")));
});

test("corrupt downloaded bytes are retained as unqualified evidence without retry replacement", async (t) => {
  const { collectAttempt } = await implementation();
  const f = fixture(t); addValidSyntheticArtifact(f);
  f.zip = Buffer.from("not the original zip");
  const result = collectAttempt(42, 1, join(f.root, "archive"), f.read);
  assert.equal(result.record.rows[0].state, "artifact_validation_failed");
  assert.equal(result.record.rows[0].first_attempt_qualified, false);
  assert.deepEqual(readFileSync(join(result.destination, "ubuntu-24.04", "original.zip")), f.zip);
});

test("changed complete run, job or artifact snapshots never publish a canonical attempt", async (t) => {
  const { collectAttempt } = await implementation();
  for (const kind of ["run", "jobs", "artifacts"]) {
    const f = fixture(t), destination = join(f.root, "archive");
    let runReads = 0, jobReads = 0, artifactReads = 0;
    const read = endpoint => {
      const data = JSON.parse(f.read(endpoint));
      if (endpoint.endsWith("/attempts/1") && ++runReads > 1 && kind === "run") {
        data.event = "workflow_dispatch"; data.path = ".github/workflows/ci.yml"; data.repository.full_name = "Other/repo";
      }
      if (endpoint.includes("/jobs?") && ++jobReads > 1 && kind === "jobs") data.jobs[0].conclusion = "cancelled";
      if (endpoint.includes("/artifacts?") && ++artifactReads > 1 && kind === "artifacts") {
        data.total_count = 1; data.artifacts = [{ id: 500, name: "later artifact" }];
      }
      return Buffer.from(JSON.stringify(data));
    };
    assert.throws(() => collectAttempt(42, 1, destination, read));
    assert(!existsSync(join(destination, "42", "1")));
    for (const name of readdirSync(destination)) verifyArtifactIndex(join(destination, name));
  }
});

test("summary refuses contradictory final snapshots and copied negative job states", async (t) => {
  const { collectAttempt, summarizeArchive } = await implementation();
  for (const kind of ["final-run", "negative-job", "negative-state", "copied-timestamp"]) {
    const f = fixture(t), destination = join(f.root, "archive");
    const result = collectAttempt(42, 1, destination, f.read);
    if (kind === "final-run") {
      const after = { ...f.run, event: "workflow_dispatch" };
      writeFileSync(join(result.destination, "metadata", "run-after.json"), JSON.stringify(after));
    } else {
      const record = JSON.parse(readFileSync(join(result.destination, "record.json")));
      if (kind === "negative-job") record.rows[0].job.conclusion = "success";
      else if (kind === "copied-timestamp") record.created_at = "2020-01-01T00:00:00Z";
      else record.rows[0].state = "invented-state";
      writeFileSync(join(result.destination, "record.json"), JSON.stringify(record));
    }
    rmSync(join(result.destination, "artifact-index.json")); writeArtifactIndex(result.destination);
    assert.throws(() => summarizeArchive(destination));
  }
});

test("complete multi-page job snapshots are revalidated rather than trusting page one", async (t) => {
  const { collectAttempt } = await implementation();
  const f = fixture(t), destination = join(f.root, "archive");
  let pageTwoReads = 0;
  const read = endpoint => {
    if (!endpoint.includes("/jobs?")) return f.read(endpoint);
    const page = endpoint.endsWith("page=1") ? f.jobs.slice(0, 2) : f.jobs.slice(2);
    const data = { total_count: 3, jobs: structuredClone(page) };
    if (endpoint.endsWith("page=2") && ++pageTwoReads > 1) data.jobs[0].conclusion = "cancelled";
    return Buffer.from(JSON.stringify(data));
  };
  assert.throws(() => collectAttempt(42, 1, destination, read));
  assert(!existsSync(join(destination, "42", "1")));
});
