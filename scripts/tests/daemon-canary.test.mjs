import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import test from "node:test";

import { checkCiPins } from "../check-ci-pins.mjs";
import { proveOverride, runCanary, testArgs, validateRef } from "../run-daemon-canary.mjs";

const COVEN = "/synthetic/coven/crates/coven-cli/Cargo.toml";
const THREADS = "/synthetic/threads/crates/coven-threads-core/Cargo.toml";
const COMPATIBILITY_TARGETS = [
  "threads_e2e", "threads_identity_invariants",
  "threads_protected_intake", "threads_terminal_recovery",
];

function metadata() {
  return {
    packages: [
      { id: "cli", name: "coven-cli", manifest_path: COVEN,
        targets: [{ name: "threads_e2e", kind: ["test"] }] },
      { id: "threads", name: "coven-threads-core", source: null,
        manifest_path: THREADS, version: "0.2.0" },
    ],
    resolve: { nodes: [
      { id: "cli", deps: [{ name: "coven_threads_core", pkg: "threads" }] },
    ] },
  };
}

test("accepts only immutable manual refs and the separate scheduled main canary", () => {
  const sha = "a".repeat(40);
  assert.equal(validateRef(sha, "workflow_dispatch"), sha);
  assert.equal(validateRef("main", "schedule"), "main");
  for (const ref of ["main", "refs/heads/main", "", "a".repeat(39), "a".repeat(41), "--help"]) {
    assert.throws(() => validateRef(ref, "workflow_dispatch"), /full 40-character/);
  }
});

test("proves the exact Threads package used by this coven-cli, not package presence", () => {
  assert.deepEqual(proveOverride(metadata(), COVEN, THREADS), {
    local_threads_override_active: true, threads_version: "0.2.0",
  });
  const wrongEdge = metadata();
  wrongEdge.packages.push({ ...wrongEdge.packages[1], id: "old", source: "git+historical" });
  wrongEdge.resolve.nodes[0].deps[0].pkg = "old";
  assert.throws(() => proveOverride(wrongEdge, COVEN, THREADS), /exact current/);
});

for (const [name, mutate] of [
  ["historical Git dependency", (data) => { data.packages[1].source = "git+historical"; }],
  ["different local checkout", (data) => { data.packages[1].manifest_path = "/synthetic/other/Cargo.toml"; }],
  ["wrong CLI checkout", (data) => { data.packages[0].manifest_path = "/synthetic/other/Cargo.toml"; }],
  ["no resolve graph", (data) => { delete data.resolve; }],
  ["no daemon target", (data) => { data.packages[0].targets = []; }],
  ["non-test target", (data) => { data.packages[0].targets[0].kind = ["bin"]; }],
  ["no dependency edge", (data) => { data.resolve.nodes[0].deps = []; }],
  ["duplicate dependency edge", (data) => { data.resolve.nodes[0].deps.push(data.resolve.nodes[0].deps[0]); }],
  ["duplicate CLI node", (data) => { data.resolve.nodes.push(data.resolve.nodes[0]); }],
  ["duplicate package id", (data) => { data.packages.push(data.packages[1]); }],
  ["missing source field", (data) => { delete data.packages[1].source; }],
]) {
  test(`fails closed for ${name}`, () => {
    const data = metadata();
    mutate(data);
    assert.throws(() => proveOverride(data, COVEN, THREADS));
  });
}

test("the recorded command enables controlled time and runs the actual daemon target locked", () => {
  assert.deepEqual(testArgs, [
    "test", "--locked", "-p", "coven-cli", "--features", "threads-test-clock",
    "--test", "threads_e2e", "--", "--nocapture",
  ]);
});

test("observation is separate from required PR CI and pins its actions", () => {
  const workflow = readFileSync(
    new URL("../../.github/workflows/daemon-observation.yml", import.meta.url), "utf8",
  );
  checkCiPins(workflow, "1.95.0");
  const coreCi = readFileSync(new URL("../../.github/workflows/ci.yml", import.meta.url), "utf8");
  checkCiPins(coreCi, "1.88.0");
  assert.match(workflow, /schedule:\s*\n\s*- cron:/);
  assert.match(workflow, /workflow_dispatch:/);
  assert.doesNotMatch(workflow, /pull_request|continue-on-error|secrets\.|write-all/);
  assert.match(workflow, /contents: read/);
  assert.match(workflow, /threads-observation-\$\{\{ github\.run_id \}\}-\$\{\{ github\.run_attempt \}\}/);
  assert.match(workflow, /persist-credentials: false/);
});

function fixture(t) {
  const target = new URL("../../target/", import.meta.url);
  mkdirSync(target, { recursive: true });
  const root = mkdtempSync(new URL("canary-test-", target));
  t.after(() => rmSync(root, { recursive: true }));
  const coven = join(root, "coven");
  mkdirSync(join(coven, "crates/coven-cli/tests"), { recursive: true });
  writeFileSync(join(coven, "crates/coven-cli/Cargo.toml"), "");
  writeFileSync(join(coven, "crates/coven-cli/tests/threads_e2e.rs"), "");
  writeFileSync(join(coven, "Cargo.lock"), "original");
  const artifacts = join(root, "artifacts");
  const graph = metadata();
  graph.packages[0].manifest_path = realpathSync(join(coven, "crates/coven-cli/Cargo.toml"));
  graph.packages[1].manifest_path = realpathSync(
    new URL("../../crates/coven-threads-core/Cargo.toml", import.meta.url),
  );
  const invocations = [];
  const execute = (exe, args, cwd, options) => {
    invocations.push({ exe, args, cwd, options });
    if (exe === "git") return args[0] === "rev-parse" ? "a".repeat(40) : "";
    if (args[0] === "--version") return `${exe} 1.95.0 (synthetic)`;
    assert.equal(exe, "cargo");
    if (args[0] === "metadata") {
      if (!args.includes("--locked")) writeFileSync(join(coven, "Cargo.lock"), "resolved overlay");
      return JSON.stringify(graph);
    }
    assert.deepEqual(args, testArgs);
    return "";
  };
  return { coven, artifacts, graph, execute, invocations };
}

test("records explicit overlay resolution, inherited patch, exact invocation and attempt", (t) => {
  const f = fixture(t);
  const receipt = runCanary(f.coven, f.artifacts, {
    COVEN_REF: "a".repeat(40), GITHUB_RUN_ID: "12", GITHUB_RUN_ATTEMPT: "2",
  }, f.execute);
  assert.equal(receipt.status, "passed");
  assert.equal(receipt.run_attempt, "2");
  assert.equal(receipt.rustc, "rustc 1.95.0 (synthetic)");
  assert.equal(receipt.cargo, "cargo 1.95.0 (synthetic)");
  assert.equal(receipt.local_threads_override_active, true);
  assert.notEqual(receipt.coven_lock_before_sha256, receipt.coven_lock_overlay_sha256);
  assert.match(readFileSync(join(f.coven, ".cargo/config.toml"), "utf8"), /\[patch\./);
  const testCall = f.invocations.find((call) => call.exe === "cargo" && call.args[0] === "test");
  assert.equal(testCall.options.env.COVEN_THREADS_E2E_REQUIRE_LOCAL_OVERRIDE, "1");
  assert.equal(testCall.options.env.COVEN_THREADS_E2E_ARTIFACT_ROOT, join(f.artifacts, "journeys"));
  assert.deepEqual(JSON.parse(readFileSync(join(f.artifacts, "observation.json"))), receipt);
  assert.throws(() => runCanary(f.coven, f.artifacts, {}, f.execute), /EEXIST/);
});

function compatibilityFixture(t) {
  const f = fixture(t);
  f.graph.packages[0].targets = COMPATIBILITY_TARGETS.map((name) => ({
    name, kind: ["test"],
  }));
  for (const name of COMPATIBILITY_TARGETS) {
    writeFileSync(join(f.coven, `crates/coven-cli/tests/${name}.rs`), "");
  }
  return f;
}

test("compatibility runs all four real-daemon targets under the same overlay guards", (t) => {
  const f = compatibilityFixture(t);
  let invocation;
  const execute = (exe, args, cwd, options) => {
    if (exe === "cargo" && args[0] === "test") {
      invocation = { args, cwd, options };
      return "";
    }
    return f.execute(exe, args, cwd, options);
  };
  const receipt = runCanary(f.coven, f.artifacts, {
    COVEN_REF: "a".repeat(40),
    COVEN_THREADS_DAEMON_SUITE: "compatibility",
    GITHUB_EVENT_NAME: "pull_request",
  }, execute);
  assert.deepEqual(invocation.args, [
    "test", "--locked", "-p", "coven-cli", "--features", "threads-test-clock",
    ...COMPATIBILITY_TARGETS.flatMap((name) => ["--test", name]),
    "--", "--nocapture",
  ]);
  assert.deepEqual(receipt.command, ["cargo", ...invocation.args]);
  assert.equal(receipt.suite, "compatibility");
  assert.deepEqual(receipt.test_targets, COMPATIBILITY_TARGETS);
  assert.equal(invocation.options.env.COVEN_THREADS_E2E_REQUIRE_LOCAL_OVERRIDE, "1");
  assert.equal(receipt.status, "passed");
});

for (const target of COMPATIBILITY_TARGETS.slice(1)) {
  test(`compatibility refuses missing metadata target ${target}`, () => {
    const graph = metadata();
    graph.packages[0].targets = COMPATIBILITY_TARGETS.filter((name) => name !== target)
      .map((name) => ({ name, kind: ["test"] }));
    assert.throws(
      () => proveOverride(graph, COVEN, THREADS, COMPATIBILITY_TARGETS),
      new RegExp(target),
    );
  });

  test(`compatibility refuses missing source target ${target} before Cargo`, (t) => {
    const f = compatibilityFixture(t);
    rmSync(join(f.coven, `crates/coven-cli/tests/${target}.rs`));
    assert.throws(() => runCanary(f.coven, f.artifacts, {
      COVEN_REF: "a".repeat(40), COVEN_THREADS_DAEMON_SUITE: "compatibility",
    }, f.execute), new RegExp(target));
    assert(!f.invocations.some((call) => call.exe === "cargo" && call.args[0] === "test"));
    assert.equal(JSON.parse(readFileSync(join(f.artifacts, "observation.json"))).status, "failed");
  });
}

test("compatibility cannot use a missing or mutable daemon pin", (t) => {
  for (const ref of [undefined, "main"]) {
    const f = fixture(t);
    assert.throws(() => runCanary(f.coven, f.artifacts, {
      COVEN_REF: ref, COVEN_THREADS_DAEMON_SUITE: "compatibility",
      GITHUB_EVENT_NAME: "schedule",
    }, f.execute), /compatibility.*immutable/);
  }
});

test("unknown daemon suite cannot silently select advisory coverage", (t) => {
  const f = fixture(t);
  assert.throws(() => runCanary(f.coven, f.artifacts, {
    COVEN_THREADS_DAEMON_SUITE: "compatibilty",
  }, f.execute), /unknown daemon suite/);
});

for (const mode of ["companion-failed", "source-drift", "config-drift", "lock-drift",
  "final-metadata-target"]) {
  test(`compatibility retains a failed receipt for ${mode}`, (t) => {
    const f = compatibilityFixture(t);
    let afterTests = false;
    const execute = (exe, args, cwd, options) => {
      if (afterTests && mode === "source-drift" && exe === "git" && args[0] === "status") {
        return " M crates/coven-cli/src/lib.rs";
      }
      if (exe === "cargo" && args[0] === "test") {
        assert(args.includes("threads_identity_invariants"));
        assert(args.includes("threads_protected_intake"));
        assert(args.includes("threads_terminal_recovery"));
        if (mode === "companion-failed") throw new Error("synthetic identity companion failure");
        if (mode === "config-drift") writeFileSync(join(f.coven, ".cargo/config.toml"), "# changed");
        if (mode === "lock-drift") writeFileSync(join(f.coven, "Cargo.lock"), "changed");
        if (mode === "final-metadata-target") {
          f.graph.packages[0].targets = f.graph.packages[0].targets
            .filter((target) => target.name !== "threads_terminal_recovery");
        }
        afterTests = true;
        return "";
      }
      return f.execute(exe, args, cwd, options);
    };
    assert.throws(() => runCanary(f.coven, f.artifacts, {
      COVEN_REF: "a".repeat(40), COVEN_THREADS_DAEMON_SUITE: "compatibility",
    }, execute));
    const receipt = JSON.parse(readFileSync(join(f.artifacts, "observation.json")));
    assert.equal(receipt.status, "failed");
    assert.equal(receipt.suite, "compatibility");
    assert(receipt.finished_at);
  });
}

for (const name of ["config", "config.toml"]) {
  test(`preserves existing downstream ${name} network settings and records its overlay`, (t) => {
    const f = fixture(t);
    mkdirSync(join(f.coven, ".cargo"));
    const config = join(f.coven, ".cargo", name);
    const original = "# Existing daemon setting\n[net]\ngit-fetch-with-cli = true\n";
    writeFileSync(config, original);
    const receipt = runCanary(f.coven, f.artifacts, {}, f.execute);
    assert.equal(receipt.status, "passed");
    assert(readFileSync(config, "utf8").startsWith(original));
    assert.notEqual(receipt.coven_config_before_sha256, receipt.coven_config_overlay_sha256);
  });
}

test("rejects ambiguous config names without modifying either file", (t) => {
  const f = fixture(t);
  mkdirSync(join(f.coven, ".cargo"));
  for (const name of ["config", "config.toml"]) writeFileSync(join(f.coven, ".cargo", name), "original");
  assert.throws(() => runCanary(f.coven, f.artifacts, {}, f.execute), /ambiguous downstream/);
  for (const name of ["config", "config.toml"]) {
    assert.equal(readFileSync(join(f.coven, ".cargo", name), "utf8"), "original");
  }
});

for (const name of ["config", "config.toml"]) {
  for (const phase of ["resolution", "locked-metadata", "daemon", "final-metadata"]) {
    for (const mode of ["changed", "missing", "ambiguous", "symlink"]) {
      test(`rejects ${name} ${mode} during ${phase} despite an unchanged dependency graph`, (t) => {
        const f = fixture(t);
        mkdirSync(join(f.coven, ".cargo"));
        const config = join(f.coven, ".cargo", name);
        writeFileSync(config, "[net]\ngit-fetch-with-cli = true\n");
        let metadataCalls = 0;
        const execute = (exe, args, cwd, options) => {
          const result = f.execute(exe, args, cwd, options);
          if (exe === "cargo" && args[0] === "metadata") metadataCalls += 1;
          const mutate = exe === "cargo" && (
            (phase === "resolution" && args[0] === "metadata" && metadataCalls === 1) ||
            (phase === "locked-metadata" && args[0] === "metadata" && metadataCalls === 2) ||
            (phase === "daemon" && args[0] === "test") ||
            (phase === "final-metadata" && args[0] === "metadata" && metadataCalls === 3)
          );
          if (mutate) {
            if (mode === "changed") writeFileSync(config, "\n[build]\njobs = 1\n", { flag: "a" });
            if (mode === "missing") rmSync(config);
            if (mode === "ambiguous") {
              writeFileSync(join(f.coven, ".cargo", name === "config" ? "config.toml" : "config"),
                readFileSync(config));
            }
            if (mode === "symlink") {
              const replacement = join(f.coven, "replacement-config");
              writeFileSync(replacement, readFileSync(config));
              rmSync(config);
              symlinkSync(replacement, config);
            }
          }
          return result;
        };
        assert.throws(() => runCanary(f.coven, f.artifacts, {}, execute), /configuration/);
        const receipt = JSON.parse(readFileSync(join(f.artifacts, "observation.json")));
        assert.equal(receipt.status, "failed");
        assert(receipt.finished_at);
        if (phase === "resolution" || phase === "locked-metadata") {
          assert(!f.invocations.some((call) => call.exe === "cargo" && call.args[0] === "test"));
        }
      });
    }
  }
}

test("rejects lockfile drift during final metadata before marking the receipt passed", (t) => {
  const f = fixture(t);
  let metadataCalls = 0;
  const execute = (exe, args, cwd, options) => {
    const result = f.execute(exe, args, cwd, options);
    if (exe === "cargo" && args[0] === "metadata" && ++metadataCalls === 3) {
      writeFileSync(join(f.coven, "Cargo.lock"), "changed during final metadata");
    }
    return result;
  };
  assert.throws(() => runCanary(f.coven, f.artifacts, {}, execute), /lockfile/);
  assert.equal(JSON.parse(readFileSync(join(f.artifacts, "observation.json"))).status, "failed");
});

for (const mode of ["wrong-override", "daemon-failed", "lock-changed", "missing-target", "dirty"]) {
  test(`retains failure evidence and never reports green for ${mode}`, (t) => {
    const f = fixture(t);
    if (mode === "wrong-override") f.graph.packages[1].source = "git+historical";
    if (mode === "missing-target") rmSync(join(f.coven, "crates/coven-cli/tests/threads_e2e.rs"));
    const execute = (exe, args, cwd, options) => {
      if (mode === "dirty" && exe === "git" && args[0] === "status") return " M Cargo.toml";
      if (exe === "cargo" && args[0] === "test") {
        if (mode === "daemon-failed") throw new Error("synthetic daemon failure");
        if (mode === "lock-changed") writeFileSync(join(f.coven, "Cargo.lock"), "changed during test");
      }
      return f.execute(exe, args, cwd, options);
    };
    assert.throws(() => runCanary(f.coven, f.artifacts, {}, execute));
    const receipt = JSON.parse(readFileSync(join(f.artifacts, "observation.json")));
    assert.equal(receipt.status, "failed");
    assert(receipt.failure);
    assert(receipt.finished_at);
  });
}

for (const checkout of ["threads", "coven"]) {
  for (const phase of ["resolution", "locked-metadata", "daemon", "final-metadata"]) {
    for (const mode of ["head", "tracked", "untracked"]) {
      test(`rejects ${checkout} ${mode} drift during ${phase}`, (t) => {
        const f = fixture(t);
        let metadataCalls = 0;
        let changed = false;
        const execute = (exe, args, cwd, options) => {
          const result = f.execute(exe, args, cwd, options);
          if (exe === "git" && changed && (checkout === "coven") === (cwd === f.coven)) {
            if (mode === "head" && args[0] === "rev-parse") return "b".repeat(40);
            if (mode !== "head" && args[0] === "status") {
              return mode === "tracked" ? " M src/lib.rs" : "?? injected.rs";
            }
          }
          if (exe === "cargo" && args[0] === "metadata") metadataCalls += 1;
          if (exe === "cargo" && (
            (phase === "resolution" && args[0] === "metadata" && metadataCalls === 1) ||
            (phase === "locked-metadata" && args[0] === "metadata" && metadataCalls === 2) ||
            (phase === "daemon" && args[0] === "test") ||
            (phase === "final-metadata" && args[0] === "metadata" && metadataCalls === 3)
          )) changed = true;
          return result;
        };
        assert.throws(() => runCanary(f.coven, f.artifacts, {}, execute),
          new RegExp(`${checkout} checkout (revision|source)`));
        const receipt = JSON.parse(readFileSync(join(f.artifacts, "observation.json")));
        assert.equal(receipt.status, "failed");
        assert.equal(receipt[`${checkout}_sha`], "a".repeat(40));
        assert(receipt.finished_at);
        if (phase === "resolution" || phase === "locked-metadata") {
          assert(!f.invocations.some((call) => call.exe === "cargo" && call.args[0] === "test"));
        }
      });
    }
  }
}

for (const mode of ["overlay-only", "tracked", "staged", "untracked", "other-config",
  "submodule-tracked", "submodule-untracked"]) {
  test(`real Git source check handles ${mode} without exempting other source paths`, (t) => {
    const f = fixture(t);
    const git = (args) => execFileSync("git", args, { cwd: f.coven, encoding: "utf8" }).trim();
    git(["init", "--quiet"]);
    if (mode.startsWith("submodule-")) {
      const source = join(f.coven, "../submodule-source");
      git(["init", "--quiet", source]);
      writeFileSync(join(source, "source.rs"), "// original\n");
      git(["-C", source, "add", "."]);
      git(["-C", source, "-c", "user.name=Synthetic Fixture",
        "-c", "user.email=fixture@example.invalid", "-c", "commit.gpgsign=false",
        "commit", "--quiet", "-m", "Synthetic submodule"]);
      git(["-c", "protocol.file.allow=always", "submodule", "add", "--quiet",
        source, "vendor/nested"]);
      git(["config", "submodule.vendor/nested.ignore", "all"]);
    }
    git(["add", "."]);
    git(["-c", "user.name=Synthetic Fixture", "-c", "user.email=fixture@example.invalid",
      "-c", "commit.gpgsign=false", "commit", "--quiet", "-m", "Synthetic baseline"]);
    const execute = (exe, args, cwd, options) => {
      if (exe === "git" && cwd === f.coven) return git(args);
      const result = f.execute(exe, args, cwd, options);
      if (exe === "cargo" && args[0] === "test") {
        if (mode === "tracked" || mode === "staged") {
          writeFileSync(join(f.coven, "crates/coven-cli/tests/threads_e2e.rs"), "// drift\n");
          if (mode === "staged") git(["add", "."]);
        }
        if (mode === "untracked") writeFileSync(join(f.coven, "injected.rs"), "// drift\n");
        if (mode === "other-config") writeFileSync(join(f.coven, ".cargo/extra.toml"), "# drift\n");
        if (mode.startsWith("submodule-")) {
          writeFileSync(join(f.coven, "vendor/nested",
            mode === "submodule-tracked" ? "source.rs" : "injected.rs"), "// drift\n");
        }
      }
      return result;
    };
    if (mode === "overlay-only") {
      assert.equal(runCanary(f.coven, f.artifacts, {}, execute).status, "passed");
    } else {
      assert.throws(() => runCanary(f.coven, f.artifacts, {}, execute), /coven checkout source/);
      assert.equal(JSON.parse(readFileSync(join(f.artifacts, "observation.json"))).status, "failed");
    }
  });
}
