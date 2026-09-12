#!/usr/bin/env node

import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { existsSync, lstatSync, mkdirSync, readFileSync, realpathSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const testArgs = [
  "test", "--locked", "-p", "coven-cli", "--features", "threads-test-clock",
  "--test", "threads_e2e", "--", "--nocapture",
];

export function validateRef(ref, event) {
  if (/^[0-9a-f]{40}$/.test(ref)) return ref;
  if (event === "schedule" && ref === "main") return ref;
  throw new Error("manual daemon observation requires a full 40-character Coven commit SHA");
}

export function proveOverride(metadata, covenManifest, threadsManifest) {
  const packages = metadata.packages;
  const nodes = metadata.resolve?.nodes;
  if (!Array.isArray(packages) || !Array.isArray(nodes)) {
    throw new Error("Cargo metadata must include packages and a resolved dependency graph");
  }
  const clients = packages.filter(
    (pkg) => pkg.name === "coven-cli" && pkg.manifest_path === covenManifest,
  );
  if (clients.length !== 1) throw new Error("metadata must identify the selected coven-cli checkout");
  const client = clients[0];
  if (!client.targets?.some((target) =>
    target.name === "threads_e2e" && target.kind?.includes("test"))) {
    throw new Error("selected Coven revision has no real-daemon threads_e2e target");
  }
  const clientNodes = nodes.filter((node) => node.id === client.id);
  const deps = clientNodes.length === 1
    ? clientNodes[0].deps?.filter((dep) => dep.name === "coven_threads_core")
    : [];
  if (deps?.length !== 1) throw new Error("coven-cli must resolve exactly one Threads dependency");
  const candidates = packages.filter((pkg) => pkg.id === deps[0].pkg);
  if (candidates.length !== 1 || candidates[0].name !== "coven-threads-core" ||
      candidates[0].source !== null || candidates[0].manifest_path !== threadsManifest) {
    throw new Error("coven-cli does not use the exact current Threads checkout");
  }
  return { local_threads_override_active: true, threads_version: candidates[0].version };
}

function command(executable, args, cwd, options = {}) {
  const result = spawnSync(executable, args, {
    cwd, encoding: "utf8", maxBuffer: 64 * 1024 * 1024, ...options,
  });
  if (result.error || result.status !== 0) {
    if (result.stderr) process.stderr.write(result.stderr);
    throw new Error(`${executable} ${args[0]} failed (${result.status ?? result.signal ?? "spawn"})`);
  }
  return result.stdout?.trim() ?? "";
}

function hashFile(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

export function runCanary(covenPath, artifactPath, env = process.env, execute = command) {
  const threads = realpathSync(new URL("..", import.meta.url));
  const coven = realpathSync(covenPath);
  const artifacts = resolve(artifactPath);
  // Existing evidence cannot be reused, even on a retry of the same commit.
  mkdirSync(dirname(artifacts), { recursive: true });
  mkdirSync(artifacts);
  const receiptPath = join(artifacts, "observation.json");
  const receipt = {
    schema_version: "opencoven.threads-observation/v1",
    status: "running",
    stage: "preflight",
    run_id: env.GITHUB_RUN_ID ?? null,
    run_attempt: env.GITHUB_RUN_ATTEMPT ?? null,
    requested_coven_ref: env.COVEN_REF ?? null,
    command: ["cargo", ...testArgs],
    local_threads_override_active: false,
    started_at: new Date().toISOString(),
  };
  const save = () => writeFileSync(receiptPath, `${JSON.stringify(receipt, null, 2)}\n`);
  save();
  try {
    if (env.COVEN_REF) validateRef(env.COVEN_REF, env.GITHUB_EVENT_NAME);
    for (const [name, root] of [["threads", threads], ["coven", coven]]) {
      receipt[`${name}_sha`] = execute("git", ["rev-parse", "HEAD"], root);
      if (execute("git", ["status", "--porcelain", "--untracked-files=normal"], root)) {
        throw new Error(`${name} checkout must be clean before the disposable Cargo overlay`);
      }
    }
    if (env.COVEN_REF && env.COVEN_REF !== "main" && env.COVEN_REF !== receipt.coven_sha) {
      throw new Error("Coven checkout does not match the requested immutable revision");
    }
    const covenManifest = realpathSync(join(coven, "crates/coven-cli/Cargo.toml"));
    const threadsManifest = realpathSync(join(threads, "crates/coven-threads-core/Cargo.toml"));
    if (!existsSync(join(coven, "crates/coven-cli/tests/threads_e2e.rs"))) {
      throw new Error("selected Coven revision has no real-daemon threads_e2e target");
    }
    const configDir = join(coven, ".cargo");
    if (existsSync(configDir) && !lstatSync(configDir).isDirectory()) {
      throw new Error("downstream Cargo configuration directory must not be a symlink or file");
    }
    if (existsSync(join(configDir, "config")) || existsSync(join(configDir, "config.toml"))) {
      throw new Error("existing downstream Cargo configuration requires explicit reconciliation");
    }
    receipt.coven_lock_before_sha256 = hashFile(join(coven, "Cargo.lock"));
    mkdirSync(configDir, { recursive: true });
    writeFileSync(join(configDir, "config.toml"),
      `[patch."https://github.com/OpenCoven/coven-threads"]\n` +
      `coven-threads-core = { path = ${JSON.stringify(dirname(threadsManifest))} }\n`,
      { flag: "wx" });
    receipt.stage = "overlay-resolution";
    save();
    const metadataArgs = ["metadata", "--format-version", "1", "--features", "threads-test-clock"];
    // The overlay necessarily changes Git-source lock entries to path entries.
    // Resolve once explicitly; the test and its nested metadata use --locked.
    execute("cargo", metadataArgs, coven, { env });
    const metadata = JSON.parse(execute("cargo", [...metadataArgs, "--locked"], coven, { env }));
    Object.assign(receipt, proveOverride(metadata, covenManifest, threadsManifest));
    receipt.coven_lock_overlay_sha256 = hashFile(join(coven, "Cargo.lock"));
    receipt.overlay_resolution_command = ["cargo", ...metadataArgs];
    receipt.stage = "daemon-journeys";
    save();
    execute("cargo", testArgs, coven, {
      stdio: "inherit",
      env: {
        ...env,
        COVEN_THREADS_E2E_REQUIRE_LOCAL_OVERRIDE: "1",
        COVEN_THREADS_E2E_ARTIFACT_ROOT: join(artifacts, "journeys"),
      },
    });
    if (hashFile(join(coven, "Cargo.lock")) !== receipt.coven_lock_overlay_sha256) {
      throw new Error("daemon execution changed the resolved overlay lockfile");
    }
    const after = JSON.parse(execute("cargo", [...metadataArgs, "--locked"], coven, { env }));
    proveOverride(after, covenManifest, threadsManifest);
    receipt.status = "passed";
    receipt.stage = "complete";
  } catch (error) {
    receipt.status = "failed";
    // Paths and downstream output belong in runner logs, not this public receipt.
    receipt.failure = error instanceof Error ? error.message.replaceAll(coven, "<coven>")
      .replaceAll(threads, "<threads>") : "unexpected observation failure";
    throw error;
  } finally {
    receipt.finished_at = new Date().toISOString();
    save();
  }
  return receipt;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    if (process.argv.length !== 4) {
      throw new Error("usage: node scripts/run-daemon-canary.mjs COVEN_CHECKOUT NEW_ARTIFACT_DIRECTORY");
    }
    runCanary(process.argv[2], process.argv[3]);
  } catch (error) {
    console.error(`daemon-observation: ${error.message}`);
    process.exitCode = 1;
  }
}
