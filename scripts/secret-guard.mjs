#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { git, indexedFiles } from "./privacy-guard.mjs";

let snapshot;
try {
  if (process.argv.length !== 2) throw new Error("unsupported-arguments");
  const expected = readFileSync(new URL("../.gitleaks-version", import.meta.url), "utf8").trim();
  const version = spawnSync("gitleaks", ["version"], { encoding: "utf8" });
  if (!/^\d+\.\d+\.\d+$/.test(expected) || version.status !== 0 || version.stdout.trim() !== expected) {
    throw new Error("scanner-version-unavailable");
  }
  if (git("rev-parse", "--is-shallow-repository").toString().trim() !== "false") {
    throw new Error("shallow-history");
  }
  git("rev-parse", "--verify", "HEAD");
  snapshot = mkdtempSync(join(tmpdir(), "threads-secret-snapshot-"));
  const tree = join(snapshot, "tree");
  mkdirSync(tree);
  for (const { path, data } of indexedFiles()) {
    const destination = join(tree, path);
    mkdirSync(dirname(destination), { recursive: true });
    writeFileSync(destination, data, { mode: 0o600 });
  }
  const ignore = join(snapshot, "empty-ignore");
  writeFileSync(ignore, "", { mode: 0o600 });
  const common = [
    "--config", fileURLToPath(new URL("../.gitleaks-default.toml", import.meta.url)),
    "--ignore-gitleaks-allow", "--gitleaks-ignore-path", ignore,
    "--redact=100", "--no-banner", "--no-color", "--exit-code=10", "--timeout=120",
  ];
  let blocked = false;
  for (const [name, args] of [
    ["indexed", ["dir", tree]],
    ["history", ["git", "--log-opts=HEAD", "."]],
  ]) {
    // Even redacted scanner diagnostics may contain private filenames or context.
    // Discard all raw output rather than uploading a report or echoing matches.
    const result = spawnSync("gitleaks", [...args, ...common], {
      stdio: "ignore", timeout: 150_000,
    });
    if (result.error || ![0, 10].includes(result.status)) throw new Error("scanner-failed");
    console.log(`${name} secrets: ${result.status === 10 ? "BLOCKED" : "clean"}`);
    blocked ||= result.status === 10;
  }
  process.exitCode = blocked ? 1 : 0;
} catch {
  console.error("secret-guard: scanner or Git input unavailable/unsupported; fail closed");
  process.exitCode = 2;
} finally {
  if (snapshot) rmSync(snapshot, { recursive: true, force: true });
}
