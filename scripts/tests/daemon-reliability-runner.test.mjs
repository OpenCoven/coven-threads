import assert from "node:assert/strict";
import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

const modulePath = new URL("../run-daemon-reliability.mjs", import.meta.url);
async function implementation() {
  assert(existsSync(modulePath), "native reliability runner is not implemented");
  return import(modulePath);
}
function root(t) {
  const path = mkdtempSync(join(tmpdir(), "threads-native-process-"));
  t.after(() => rmSync(path, { recursive: true, force: true }));
  return path;
}
const summary = "test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s";

test("target recording reads a real process, hashes its executable and retains exact counts", async (t) => {
  const { recordTarget } = await implementation();
  const path = root(t);
  const script = `console.log(JSON.stringify({reason:'compiler-artifact',target:{name:'threads_e2e'},executable:process.execPath})); console.log(${JSON.stringify(summary)}); console.error('synthetic diagnostic');`;
  const r = recordTarget("threads_e2e", process.execPath, ["-e", script], path, path, process.env);
  assert.equal(r.status, "passed");
  assert.equal(r.counts.passed, 3);
  assert.equal(r.executables.length, 1);
  assert.match(r.executables[0].sha256, /^[0-9a-f]{64}$/);
  assert.equal(r.process.exit_code, 0);
  assert.match(readFileSync(join(path, "threads_e2e.stderr.log"), "utf8"), /synthetic diagnostic/);
});

test("a failed process keeps its first failure and cannot become a passing count receipt", async (t) => {
  const { recordTarget } = await implementation();
  const path = root(t);
  const r = recordTarget("threads_e2e", process.execPath,
    ["-e", `console.log(JSON.stringify({reason:'compiler-artifact',target:{name:'threads_e2e'},executable:process.execPath})); console.log(${JSON.stringify(summary)}); console.error('first failure'); process.exit(7)`], path, path, process.env);
  assert.equal(r.status, "failed");
  assert.equal(r.process.exit_code, 7);
  assert.equal(r.counts.passed, 3);
  assert.equal(r.executables.length, 1);
  assert.match(readFileSync(join(path, "threads_e2e.stderr.log"), "utf8"), /first failure/);
  assert.throws(() => recordTarget("threads_e2e", process.execPath, ["-e", ""], path, path, process.env), /EEXIST/);
});

test("failed test counts are retained and all later targets still execute once", async (t) => {
  const { recordTargetGroup } = await implementation();
  assert.equal(typeof recordTargetGroup, "function", "target group execution is not implemented");
  const path = root(t);
  const commands = ["threads_e2e", "threads_identity_invariants", "threads_protected_intake", "threads_terminal_recovery"]
    .map((target, index) => ({ target, executable: process.execPath,
      args: ["-e", `console.log(JSON.stringify({reason:'compiler-artifact',target:{name:${JSON.stringify(target)}},executable:process.execPath})); console.log(${JSON.stringify(index === 1 ? summary.replace("ok.", "FAILED.").replace("0 failed", "1 failed") : summary)}); process.exit(${index === 1 ? 1 : 0})`] }));
  const runs = recordTargetGroup(commands, path, path, process.env);
  assert.deepEqual(runs.map(row => row.status), ["passed", "failed", "passed", "passed"]);
  assert.equal(runs[1].counts.failed, 1);
  assert.equal(runs[3].counts.passed, 3);
  assert.equal(JSON.parse(readFileSync(join(path, "targets.json"))).length, 4);
});

test("zero tests or absent executable identities fail even when the process exits successfully", async (t) => {
  const { recordTarget } = await implementation();
  for (const [target, output] of [["threads_e2e", summary], ["threads_terminal_recovery", summary.replace("3 passed", "0 passed")]]) {
    const path = root(t);
    const r = recordTarget(target, process.execPath, ["-e", `console.log(${JSON.stringify(output)})`], path, path, process.env);
    assert.equal(r.status, "failed");
  }
});

test("runner output sanitizes checkout and home paths while preserving diagnostic text", async () => {
  const { sanitizeOutput } = await implementation();
  assert.equal(sanitizeOutput("failure at /private/tmp/example/coven/file and /synthetic-user-root/key", {
    coven: "/tmp/example/coven", HOME: "/synthetic-user-root",
  }), "failure at <coven>/file and <home>/key");
});

test("Windows escaped JSON paths are sanitized as well as ordinary paths", async () => {
  const { sanitizeOutput } = await implementation();
  const windowsRoot = "D:" + String.fromCharCode(92) + ["a", "project", "threads"].join(String.fromCharCode(92));
  const output = JSON.stringify({ manifest_path: windowsRoot + String.fromCharCode(92) + "Cargo.toml" });
  assert.equal(JSON.parse(sanitizeOutput(output, { threads: windowsRoot })).manifest_path,
    "<threads>" + String.fromCharCode(92) + "Cargo.toml");
});
