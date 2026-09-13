import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { test } from "node:test";

const script = new URL("../privacy-guard.mjs", import.meta.url);
const cases = [
  ["coven-session-key", ["agent", "synthetic", "webchat", "direct", "synthetic"].join(":")],
  ["messenger-chat-id", ["telegram", "direct", "123456789"].join(":")],
  ["absolute-home-path", ["/Users", "synthetic-person"].join("/")],
  ["absolute-home-path", ["/home", "synthetic-person", "data"].join("/")],
  ["runtime-internal-path", ["~", ".coven", "sessions", "synthetic"].join("/")],
  ["phone-number", "+" + "12025550123"],
  ["invite-or-handoff-url", "https://example.invalid/" + "invite?token=synthetic"],
];

function repository(t) {
  const root = mkdtempSync(join(tmpdir(), "threads-privacy-test-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  function git(...args) {
    const result = spawnSync("git", args, { cwd: root, encoding: "utf8" });
    assert.equal(result.status, 0);
    return result.stdout;
  }
  git("init", "-q");
  return { root, git };
}

function scan(root) {
  return spawnSync(process.execPath, [script.pathname], { cwd: root, encoding: "utf8" });
}

test("clean index passes; untracked private files are never scanned", (t) => {
  const { root, git } = repository(t);
  writeFileSync(join(root, "public.txt"), "FAMILIAR_ROOT; <user>; synthetic fixture\n");
  git("add", "public.txt");
  writeFileSync(join(root, "private.txt"), cases[0][1]);
  assert.equal(scan(root).status, 0);
});

for (const [rule, value] of cases) {
  test(`rejects synthetic ${rule} without echoing content or filenames`, (t) => {
    const { root, git } = repository(t);
    const filename = "sensitive-name\nfixture.txt";
    writeFileSync(join(root, filename), `${value} gitleaks:allow guard-scan-allow\n`);
    git("add", "--", filename);
    // The index is authoritative even when the worktree is clean or deleted.
    rmSync(join(root, filename));
    const result = scan(root);
    assert.equal(result.status, 1);
    assert.match(result.stdout + result.stderr, new RegExp(rule));
    assert.ok(!(result.stdout + result.stderr).includes(value));
    assert.ok(!(result.stdout + result.stderr).includes(filename));
  });
}

test("a harmless placeholder on the same line cannot hide another match", (t) => {
  const { root, git } = repository(t);
  writeFileSync(join(root, "fixture.txt"), "/home/<user> " + cases[3][1]);
  git("add", ".");
  assert.equal(scan(root).status, 1);
});

test("binary extension and NUL bytes cannot bypass privacy scanning", (t) => {
  const { root, git } = repository(t);
  writeFileSync(join(root, "fixture.png"), "\0" + cases[0][1]);
  git("add", ".");
  assert.equal(scan(root).status, 1);
});

test("non-repository fails closed without exposing local paths", (t) => {
  const root = mkdtempSync(join(tmpdir(), "threads-no-git-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const result = scan(root);
  assert.equal(result.status, 2);
  assert.ok(!(result.stdout + result.stderr).includes(root));
});

test("tracked symlinks fail closed without following their targets", (t) => {
  const { root, git } = repository(t);
  symlinkSync("nonexistent-private-store", join(root, "link"));
  git("add", "link");
  assert.equal(scan(root).status, 2);
});

test("running below the repository root fails closed rather than scanning a subset", (t) => {
  const { root } = repository(t);
  mkdirSync(join(root, "nested"));
  assert.equal(scan(join(root, "nested")).status, 2);
  assert.equal(scan(join(root, ".git")).status, 2);
});
