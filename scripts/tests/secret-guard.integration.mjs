import assert from "node:assert/strict";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { test } from "node:test";

const script = new URL("../secret-guard.mjs", import.meta.url).pathname;
const synthetic = "gh" + "p_" + createHash("sha256").update("threads synthetic credential, never issued").digest("hex").slice(0, 36);

test("real pinned scanner: clean, staged secret, removed historical secret; no bypass or output leak", (t) => {
  const root = mkdtempSync(join(tmpdir(), "threads-secret-test-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  function git(...args) {
    const result = spawnSync("git", args, { cwd: root, encoding: "utf8" });
    assert.equal(result.status, 0);
  }
  const scan = () => spawnSync(process.execPath, [script], { cwd: root, encoding: "utf8" });
  git("init", "-q");
  writeFileSync(join(root, "public.txt"), "deterministic synthetic fixture\n");
  git("add", ".");
  git("-c", "user.name=Synthetic Fixture", "-c", "user.email=fixture@example.invalid",
    "-c", "core.hooksPath=/dev/null", "commit", "-qm", "synthetic clean");
  assert.equal(scan().status, 0);
  writeFileSync(join(root, ".gitleaks.toml"), '[allowlist]\npaths = [".*"]\n');
  writeFileSync(join(root, "sensitive-name.txt"), synthetic + " # gitleaks:allow\n");
  git("add", ".");
  let result = scan();
  assert.equal(result.status, 1);
  assert.match(result.stdout + result.stderr, /indexed secrets: BLOCKED/);
  assert.ok(!(result.stdout + result.stderr).includes(synthetic));
  assert.ok(!(result.stdout + result.stderr).includes("sensitive-name"));
  git("-c", "user.name=Synthetic Fixture", "-c", "user.email=fixture@example.invalid",
    "-c", "core.hooksPath=/dev/null", "commit", "-qm", "synthetic red");
  git("rm", "-q", "sensitive-name.txt");
  git("-c", "user.name=Synthetic Fixture", "-c", "user.email=fixture@example.invalid",
    "-c", "core.hooksPath=/dev/null", "commit", "-qm", "synthetic removal");
  result = scan();
  assert.equal(result.status, 1);
  assert.match(result.stdout + result.stderr, /history secrets: BLOCKED/);
  assert.ok(!(result.stdout + result.stderr).includes(synthetic));
});

test("shallow history fails closed", (t) => {
  const root = mkdtempSync(join(tmpdir(), "threads-shallow-test-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  assert.equal(spawnSync("git", ["init", "-q"], { cwd: root }).status, 0);
  // A synthetic shallow boundary; no network or external repository involved.
  writeFileSync(join(root, ".git", "shallow"), "1".repeat(40) + "\n");
  const result = spawnSync(process.execPath, [script], { cwd: root, encoding: "utf8" });
  assert.equal(result.status, 2);
});

test("missing scanner fails closed with sanitized diagnostics", (t) => {
  const root = mkdtempSync(join(tmpdir(), "threads-no-scanner-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const result = spawnSync(process.execPath, [script], {
    cwd: root, encoding: "utf8", env: { ...process.env, PATH: root },
  });
  assert.equal(result.status, 2);
  assert.ok(!(result.stdout + result.stderr).includes(root));
});
