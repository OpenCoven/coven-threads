import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

import { checkCiPins } from "../check-ci-pins.mjs";
import { compatibilityTargets } from "../run-daemon-canary.mjs";

function manifest() {
  return `schema_version = "opencoven.threads-compatibility/v1alpha1"
[downstream.coven]
repository = "OpenCoven/coven"
ref = "${"a".repeat(40)}"
package = "coven-cli"
threads_dependency_manifest = "crates/coven-cli/Cargo.toml"
current_threads_rev = "${"b".repeat(40)}"
required_test_target = "threads_e2e"
required_test_targets = ${JSON.stringify(compatibilityTargets)}
status = "ready"
[downstream.coven.canary]
ref = "main"
schedule = "nightly"
blocking = false
`;
}

function fixture(t, text = manifest()) {
  const target = new URL("../../target/", import.meta.url);
  mkdirSync(target, { recursive: true });
  const root = mkdtempSync(new URL("compatibility-test-", target));
  t.after(() => rmSync(root, { recursive: true }));
  const path = join(root, "compatibility.toml");
  writeFileSync(path, text);
  const execute = (...extra) => spawnSync(process.execPath, [
    fileURLToPath(new URL("../daemon-compatibility.mjs", import.meta.url)), path, ...extra,
  ], { encoding: "utf8" });
  return { root, execute };
}

test("resolves only the ready immutable manifest pin, not the advisory main reference", (t) => {
  const f = fixture(t);
  const result = f.execute();
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout.trim(), "a".repeat(40));
});

for (const [name, change] of [
  ["old schema", (s) => s.replace("v1alpha1", "v0")],
  ["other repository", (s) => s.replace('"OpenCoven/coven"', '"other/coven"')],
  ["mutable pin", (s) => s.replace(`"${"a".repeat(40)}"`, '"main"')],
  ["short dependency pin", (s) => s.replace(`"${"b".repeat(40)}"`, '"c102844"')],
  ["unready harness", (s) => s.replace('"ready"', '"harness-required"')],
  ["wrong package", (s) => s.replace('"coven-cli"', '"other"')],
  ["escaping manifest path", (s) => s.replace('"crates/coven-cli/Cargo.toml"', '"../Cargo.toml"')],
  ["missing companion", (s) => s.replace(',"threads_terminal_recovery"', "")],
  ["primary target drift", (s) => s.replace('required_test_target = "threads_e2e"', 'required_test_target = "other"')],
  ["blocking advisory", (s) => s.replace("blocking = false", "blocking = true")],
  ["duplicate TOML key", (s) => s.replace('package = "coven-cli"', 'package = "coven-cli"\npackage = "other"')],
]) {
  test(`rejects ${name} without emitting a checkout ref`, (t) => {
    const result = fixture(t, change(manifest())).execute();
    assert.notEqual(result.status, 0);
    assert.equal(result.stdout, "");
    assert.match(result.stderr, /daemon-compatibility:/);
  });
}

test("checks the selected daemon's committed Threads dependency before overlay", (t) => {
  const f = fixture(t);
  const coven = join(f.root, "coven");
  mkdirSync(join(coven, "crates/coven-cli"), { recursive: true });
  const path = join(coven, "crates/coven-cli/Cargo.toml");
  const cargo = `[package]\nname = "coven-cli"\n[dependencies]\n` +
    `coven-threads-core = { git = "https://github.com/OpenCoven/coven-threads", rev = "${"b".repeat(40)}" }\n`;
  writeFileSync(path, cargo);
  assert.equal(f.execute(coven).status, 0);
  writeFileSync(path, cargo.replace("b".repeat(40), "c".repeat(40)));
  const result = f.execute(coven);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /committed Threads dependency/);
});

test("required workflow always runs four-target pinned acceptance separately from the advisory schedule", () => {
  const workflow = readFileSync(
    new URL("../../.github/workflows/daemon-compatibility.yml", import.meta.url), "utf8",
  );
  checkCiPins(workflow, "1.95.0");
  assert.match(workflow, /pull_request:/);
  assert.match(workflow, /push:\s*\n\s*branches:\s*\[main\]/);
  assert.doesNotMatch(workflow, /schedule:|paths:|continue-on-error|pull_request_target|secrets\.|write-all/);
  assert.match(workflow, /name: Pinned daemon compatibility/);
  assert.match(workflow, /COVEN_THREADS_DAEMON_SUITE: compatibility/);
  assert.match(workflow, /ref: \$\{\{ steps\.pin\.outputs\.coven_ref \}\}/);
  assert.match(workflow, /COVEN_REF: \$\{\{ steps\.pin\.outputs\.coven_ref \}\}/);
  assert.match(workflow, /node scripts\/daemon-compatibility\.mjs e2e\/compatibility\.toml \.\.\/coven/);
  assert.match(workflow, /node scripts\/run-daemon-canary\.mjs/);
  assert.match(workflow, /contents: read/);
  assert.equal((workflow.match(/persist-credentials: false/g) ?? []).length, 2);
  assert.match(workflow, /if: always\(\)/);
  assert.match(workflow, /threads-daemon-compatibility-\$\{\{ github\.run_id \}\}-\$\{\{ github\.run_attempt \}\}/);
});
