#!/usr/bin/env node

import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { existsSync, lstatSync, mkdirSync, readFileSync, realpathSync, writeFileSync } from "node:fs";
import { arch, platform, release } from "node:os";
import { basename, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { readCompatibility } from "./daemon-compatibility.mjs";
import { compatibilityTargets, runCanary } from "./run-daemon-canary.mjs";
import { parseTestCounts, qualifyJourneys, readJourneyInventory, writeArtifactIndex } from "./lib/daemon-reliability-evidence.mjs";

export function sanitizeOutput(value, paths) {
  let result = value;
  const replacements = Object.entries(paths).filter(([, path]) => path)
    .sort((a, b) => b[1].length - a[1].length);
  for (const [name, path] of replacements) {
    const label = ["HOME", "USERPROFILE"].includes(name) ? "home" : name;
    if (path.startsWith("/tmp/") || path.startsWith("/var/")) {
      result = result.replaceAll("/private" + path, `<${label}>`);
    }
    result = result.replaceAll(JSON.stringify(path).slice(1, -1), `<${label}>`)
      .replaceAll(path, `<${label}>`)
      .replaceAll(path.replaceAll("\\", "/"), `<${label}>`);
  }
  return result;
}

function hashExecutable(path) {
  if (!lstatSync(path).isFile()) throw new Error("compiler executable must be a regular file");
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

export function recordTarget(target, executable, args, cwd, artifacts, env, paths = {}) {
  // Exclusive creation prevents overwriting a failed invocation with a retry.
  const stdoutPath = join(artifacts, `${target}.stdout.log`);
  const stderrPath = join(artifacts, `${target}.stderr.log`);
  writeFileSync(stdoutPath, "", { flag: "wx" });
  writeFileSync(stderrPath, "", { flag: "wx" });
  const started = new Date().toISOString();
  const processResult = spawnSync(executable, args, {
    cwd, env, encoding: "utf8", maxBuffer: 128 * 1024 * 1024,
  });
  const stdout = processResult.stdout ?? "";
  const stderr = processResult.stderr ?? "";
  const sanitize = text => sanitizeOutput(text, paths);
  writeFileSync(stdoutPath, sanitize(stdout));
  writeFileSync(stderrPath, sanitize(stderr));
  const result = {
    target, command: [executable, ...args].map(sanitize), status: "failed",
    started_at: started, finished_at: new Date().toISOString(),
    process: { exit_code: processResult.status, signal: processResult.signal,
      spawn_error: processResult.error?.code ?? null },
    stdout: basename(stdoutPath), stderr: basename(stderrPath),
    counts: null, executables: [],
  };
  // Failed invocations retain their actual summary without qualifying it.
  try { result.counts = parseTestCounts(stdout, { allowFailure: true }); } catch { /* no complete summary */ }
  try {
    const compilerArtifacts = stdout.split(/\r?\n/).filter(line => line.startsWith("{"))
      .flatMap(line => {
        try { return [JSON.parse(line)]; } catch { return []; }
      }).filter(row => row.reason === "compiler-artifact" && row.executable &&
        [target, "coven"].includes(row.target?.name));
    result.executables = compilerArtifacts.map(row => ({
      target: row.target.name, file: basename(row.executable), sha256: hashExecutable(row.executable),
    }));
    if (processResult.error || processResult.status !== 0) throw new Error("target process failed; retained original output");
    result.counts = parseTestCounts(stdout);
    if (compilerArtifacts.filter(row => row.target.name === target).length !== 1) {
      throw new Error("missing or ambiguous target executable from Cargo compiler artifacts");
    }
    result.status = "passed";
  } catch (error) {
    result.failure = sanitize(error.message);
  }
  return result;
}

export function recordTargetGroup(commands, cwd, artifacts, env, paths = {}) {
  const results = [];
  for (const { target, executable, args } of commands) {
    results.push(recordTarget(target, executable, args, cwd, artifacts, env, paths));
    writeFileSync(join(artifacts, "targets.json"), JSON.stringify(results, null, 2) + "\n");
  }
  return results;
}

export function runNativeReliability(covenPath, artifactPath, env = process.env) {
  const artifacts = resolve(artifactPath);
  if (existsSync(artifacts)) throw new Error("native reliability requires a new artifact directory for each invocation");
  const threads = realpathSync(new URL("..", import.meta.url));
  const coven = realpathSync(covenPath);
  const paths = { threads, coven, artifacts, HOME: env.HOME, USERPROFILE: env.USERPROFILE };
  const report = {
    schema_version: "opencoven.threads-native-reliability/v1", status: "failed",
    run_id: env.GITHUB_RUN_ID ?? null, run_attempt: env.GITHUB_RUN_ATTEMPT ?? null,
    event: env.GITHUB_EVENT_NAME ?? "local", runner_os: env.RUNNER_OS ?? null,
    platform: platform(), arch: arch(), kernel: release(), node: process.version,
    first_attempt: env.GITHUB_RUN_ATTEMPT === "1", first_attempt_qualified: false,
    started_at: new Date().toISOString(), target_runs: [],
  };
  let error;
  let commandNumber = 0;
  try {
    const pin = readCompatibility(join(threads, "e2e/compatibility.toml"), coven);
    const nativePlatform = { darwin: "macos", win32: "windows", linux: "linux" }[platform()];
    const expectedScenarios = readJourneyInventory(join(threads, "e2e/native-daemon-scenarios.json"), coven, pin, nativePlatform);
    if (env.COVEN_REF && env.COVEN_REF !== pin) throw new Error("native reliability must use the reviewed manifest pin");
    const execute = (executable, args, cwd, options = {}) => {
      if (executable === "cargo" && args[0] === "test") {
        // Each target runs once, including after another target has failed.
        const commands = compatibilityTargets.map(target => ({ target, executable: "cargo",
          args: ["test", "--locked", "--message-format=json", "-p", "coven-cli",
            "--features", "threads-test-clock", "--test", target, "--", "--nocapture"],
        }));
        report.target_runs = recordTargetGroup(commands, cwd, artifacts, options.env, paths);
        for (const result of report.target_runs) {
          const target = result.target;
          console.log(`${target}: ${result.status}; passed=${result.counts?.passed ?? "unqualified"}`);
        }
        if (report.target_runs.some(row => row.status !== "passed")) throw new Error("one or more native targets failed qualification");
        if (!report.target_runs.some(row => row.executables.some(binary => binary.target === "coven"))) {
          throw new Error("missing real daemon executable identity");
        }
        return "";
      }
      const result = spawnSync(executable, args, { cwd, env: options.env ?? env,
        encoding: "utf8", maxBuffer: 128 * 1024 * 1024 });
      if (executable === "cargo") {
        const prefix = `cargo-preflight-${++commandNumber}`;
        for (const stream of ["stdout", "stderr"]) {
          writeFileSync(join(artifacts, `${prefix}.${stream}.log`),
            sanitizeOutput(result[stream] ?? "", paths), { flag: "wx" });
        }
      }
      if (result.error || result.status !== 0) throw new Error(`${executable} ${args[0]} failed (${result.status ?? result.signal ?? "spawn"})`);
      if (["rustc", "cargo"].includes(executable) && args[0] === "--version" &&
          !result.stdout.startsWith(`${executable} 1.95.0 `)) {
        throw new Error("native matrix requires the existing daemon Rust/Cargo 1.95.0 tuple");
      }
      return result.stdout.trim();
    };
    const observation = runCanary(coven, artifacts, {
      ...env, COVEN_REF: pin, COVEN_THREADS_DAEMON_SUITE: "compatibility",
    }, execute);
    report.observation = "observation.json";
    report.coven_sha = observation.coven_sha;
    report.threads_sha = observation.threads_sha;
    report.rustc = observation.rustc;
    report.cargo = observation.cargo;
    report.coven_lock_overlay_sha256 = observation.coven_lock_overlay_sha256;
    if (!/^rustc 1\.95\.0 /.test(observation.rustc) || !/^cargo 1\.95\.0 /.test(observation.cargo)) {
      throw new Error("native matrix requires the existing daemon Rust/Cargo 1.95.0 tuple");
    }
    report.journeys = qualifyJourneys(join(artifacts, "journeys"), {
      ...observation, platform: nativePlatform, expected_scenarios: expectedScenarios,
    });
    report.status = "passed";
    report.first_attempt_qualified = report.first_attempt;
    if (env.GITHUB_RUN_ATTEMPT && env.GITHUB_RUN_ATTEMPT !== "1") {
      report.status = "retry-passed";
      throw new Error("rerun execution passed but cannot qualify as a first-attempt CI pass");
    }
  } catch (caught) {
    error = caught;
    report.failure = sanitizeOutput(caught.message, paths);
  } finally {
    mkdirSync(artifacts, { recursive: true });
    report.finished_at = new Date().toISOString();
    writeFileSync(join(artifacts, "reliability.json"), JSON.stringify(report, null, 2) + "\n", { flag: "wx" });
    writeArtifactIndex(artifacts);
  }
  if (error) throw error;
  return report;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    if (process.argv.length !== 4) throw new Error("usage: node scripts/run-daemon-reliability.mjs COVEN_CHECKOUT NEW_ARTIFACT_DIRECTORY");
    const report = runNativeReliability(process.argv[2], process.argv[3]);
    console.log(`native daemon reliability: ${report.status}; ${report.journeys.manifests} authenticated journey pairs`);
  } catch (error) {
    console.error(`native daemon reliability: ${error.message}`);
    process.exitCode = 1;
  }
}
