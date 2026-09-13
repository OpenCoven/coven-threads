#!/usr/bin/env node

import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { existsSync, lstatSync, mkdirSync, readFileSync, realpathSync, writeFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const compatibilityTargets = [
  "threads_e2e", "threads_identity_invariants",
  "threads_protected_intake", "threads_terminal_recovery",
];

const argsForTargets = (targets) => [
  "test", "--locked", "-p", "coven-cli", "--features", "threads-test-clock",
  ...targets.flatMap((target) => ["--test", target]), "--", "--nocapture",
];
export const testArgs = argsForTargets(["threads_e2e"]);

export function validateRef(ref, event) {
  if (/^[0-9a-f]{40}$/.test(ref)) return ref;
  if (event === "schedule" && ref === "main") return ref;
  throw new Error("manual daemon observation requires a full 40-character Coven commit SHA");
}

export function proveOverride(metadata, covenManifest, threadsManifest, targets = ["threads_e2e"]) {
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
  for (const name of targets) {
    if (!client.targets?.some((target) =>
      target.name === name && target.kind?.includes("test"))) {
      throw new Error(`selected Coven revision has no real-daemon ${name} target`);
    }
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

function downstreamConfig(coven) {
  const configDir = join(coven, ".cargo");
  const directory = lstatSync(configDir, { throwIfNoEntry: false });
  if (directory && !directory.isDirectory()) {
    throw new Error("downstream Cargo configuration directory must not be a symlink or file");
  }
  const configs = ["config", "config.toml"].map((name) => join(configDir, name))
    .filter((path) => lstatSync(path, { throwIfNoEntry: false }));
  if (configs.length > 1) {
    throw new Error("ambiguous downstream Cargo configuration files require reconciliation");
  }
  if (configs.length && !lstatSync(configs[0]).isFile()) {
    throw new Error("downstream Cargo configuration must be a regular file");
  }
  return configs[0] ?? null;
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
    const suite = env.COVEN_THREADS_DAEMON_SUITE ?? "advisory";
    if (suite !== "advisory" && suite !== "compatibility") {
      throw new Error("unknown daemon suite");
    }
    if (suite === "compatibility" && !/^[0-9a-f]{40}$/.test(env.COVEN_REF ?? "")) {
      throw new Error("compatibility requires an immutable full Coven commit SHA");
    }
    const targets = suite === "compatibility" ? compatibilityTargets : ["threads_e2e"];
    const args = argsForTargets(targets);
    Object.assign(receipt, { suite, test_targets: targets, command: ["cargo", ...args] });
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
    receipt.rustc = execute("rustc", ["--version"], coven, { env });
    receipt.cargo = execute("cargo", ["--version"], coven, { env });
    const covenManifest = realpathSync(join(coven, "crates/coven-cli/Cargo.toml"));
    const threadsManifest = realpathSync(join(threads, "crates/coven-threads-core/Cargo.toml"));
    for (const target of targets) {
      if (!existsSync(join(coven, `crates/coven-cli/tests/${target}.rs`))) {
        throw new Error(`selected Coven revision has no real-daemon ${target} target`);
      }
    }
    const configDir = join(coven, ".cargo");
    const existingConfig = downstreamConfig(coven);
    const configPath = existingConfig ?? join(configDir, "config.toml");
    receipt.coven_config_before_sha256 = existingConfig ? hashFile(configPath) : null;
    receipt.coven_lock_before_sha256 = hashFile(join(coven, "Cargo.lock"));
    mkdirSync(configDir, { recursive: true });
    // Preserve the daemon's existing network/build settings. Cargo rejects
    // conflicting patch tables rather than silently overriding their meaning.
    writeFileSync(configPath,
      `\n[patch."https://github.com/OpenCoven/coven-threads"]\n` +
      `coven-threads-core = { path = ${JSON.stringify(dirname(threadsManifest))} }\n`,
      { flag: existingConfig ? "a" : "wx" });
    receipt.coven_config_overlay_sha256 = hashFile(configPath);
    const proveSourcesUnchanged = () => {
      for (const [name, root] of [["threads", threads], ["coven", coven]]) {
        if (execute("git", ["rev-parse", "HEAD"], root) !== receipt[`${name}_sha`]) {
          throw new Error(`${name} checkout revision changed during observation`);
        }
        // Only the recorded downstream overlay may differ from its commit.
        const exclusions = name === "coven"
          ? [":(top,literal,exclude)Cargo.lock",
            `:(top,literal,exclude)${relative(coven, configPath).split("\\").join("/")}`]
          : [];
        if (execute("git", ["status", "--porcelain", "--untracked-files=all",
          "--ignore-submodules=none", "--", ".", ...exclusions], root)) {
          throw new Error(`${name} checkout source changed during observation`);
        }
      }
    };
    const proveConfigUnchanged = () => {
      if (downstreamConfig(coven) !== configPath ||
          hashFile(configPath) !== receipt.coven_config_overlay_sha256) {
        throw new Error("Cargo configuration no longer matches the recorded overlay");
      }
    };
    receipt.stage = "overlay-resolution";
    save();
    const metadataArgs = ["metadata", "--format-version", "1", "--features", "threads-test-clock"];
    // The overlay necessarily changes Git-source lock entries to path entries.
    // Resolve once explicitly; the test and its nested metadata use --locked.
    execute("cargo", metadataArgs, coven, { env });
    proveConfigUnchanged();
    proveSourcesUnchanged();
    const metadata = JSON.parse(execute("cargo", [...metadataArgs, "--locked"], coven, { env }));
    proveConfigUnchanged();
    proveSourcesUnchanged();
    Object.assign(receipt, proveOverride(metadata, covenManifest, threadsManifest, targets));
    receipt.coven_lock_overlay_sha256 = hashFile(join(coven, "Cargo.lock"));
    receipt.overlay_resolution_command = ["cargo", ...metadataArgs];
    receipt.stage = "daemon-journeys";
    save();
    execute("cargo", args, coven, {
      stdio: "inherit",
      env: {
        ...env,
        COVEN_THREADS_E2E_REQUIRE_LOCAL_OVERRIDE: "1",
        COVEN_THREADS_E2E_ARTIFACT_ROOT: join(artifacts, "journeys"),
      },
    });
    const proveOverlayUnchanged = () => {
      proveConfigUnchanged();
      proveSourcesUnchanged();
      if (hashFile(join(coven, "Cargo.lock")) !== receipt.coven_lock_overlay_sha256) {
        throw new Error("daemon execution changed the resolved overlay lockfile");
      }
    };
    proveOverlayUnchanged();
    const after = JSON.parse(execute("cargo", [...metadataArgs, "--locked"], coven, { env }));
    proveOverlayUnchanged();
    proveOverride(after, covenManifest, threadsManifest, targets);
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
