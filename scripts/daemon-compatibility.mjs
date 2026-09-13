#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { compatibilityTargets } from "./run-daemon-canary.mjs";

function readToml(path) {
  const result = spawnSync("python3", ["-I", "-c",
    "import json, sys, tomllib; json.dump(tomllib.loads(sys.stdin.read()), sys.stdout)",
  ], { input: readFileSync(path, "utf8"), encoding: "utf8" });
  if (result.error) {
    throw new Error("Python 3.11+ is required to read the compatibility manifest", { cause: result.error });
  }
  if (result.status !== 0) {
    throw new Error("invalid TOML or unavailable Python 3.11+ tomllib");
  }
  return JSON.parse(result.stdout);
}

export function readCompatibility(path, covenPath) {
  const manifest = readToml(path);
  const coven = manifest.downstream?.coven;
  if (manifest.schema_version !== "opencoven.threads-compatibility/v1alpha1" ||
      coven?.repository !== "OpenCoven/coven" || coven.package !== "coven-cli" ||
      coven.threads_dependency_manifest !== "crates/coven-cli/Cargo.toml") {
    throw new Error("unsupported compatibility manifest contract");
  }
  if (coven.status !== "ready" || !/^[0-9a-f]{40}$/.test(coven.ref) ||
      !/^[0-9a-f]{40}$/.test(coven.current_threads_rev)) {
    throw new Error("compatibility manifest requires ready status and immutable full-SHA pins");
  }
  if (coven.required_test_target !== "threads_e2e" ||
      JSON.stringify(coven.required_test_targets) !== JSON.stringify(compatibilityTargets)) {
    throw new Error("compatibility manifest must select all four required daemon targets");
  }
  if (coven.canary?.ref !== "main" || coven.canary.schedule !== "nightly" ||
      coven.canary.blocking !== false) {
    throw new Error("compatibility manifest must retain the separate non-blocking main canary");
  }
  if (covenPath) {
    const cargo = readToml(join(covenPath, coven.threads_dependency_manifest));
    const dependency = cargo.dependencies?.["coven-threads-core"];
    if (cargo.package?.name !== "coven-cli" ||
        dependency?.git !== "https://github.com/OpenCoven/coven-threads" ||
        dependency.rev !== coven.current_threads_rev) {
      throw new Error("selected daemon's committed Threads dependency does not match the manifest");
    }
  }
  return coven.ref;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    if (process.argv.length < 3 || process.argv.length > 4) {
      throw new Error("usage: node scripts/daemon-compatibility.mjs MANIFEST [COVEN_CHECKOUT]");
    }
    console.log(readCompatibility(process.argv[2], process.argv[3]));
  } catch (error) {
    console.error(`daemon-compatibility: ${error.message}`);
    process.exitCode = 1;
  }
}
