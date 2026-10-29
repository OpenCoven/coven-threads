import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { checkCiPins } from "../check-ci-pins.mjs";

const SHA = "a".repeat(40);
const rust = `      - uses: dtolnay/rust-toolchain@${SHA} # immutable action
        with:
          toolchain: 1.88.0
          components: rustfmt, clippy
`;

test("current CI pins every external action and selects the repository toolchain", () => {
  const workflow = readFileSync(new URL("../../.github/workflows/ci.yml", import.meta.url), "utf8");
  const config = readFileSync(new URL("../../rust-toolchain.toml", import.meta.url), "utf8");
  const version = config.match(/^channel = "([^"]+)"$/m)?.[1];
  const nodeVersion = readFileSync(new URL("../../.node-version", import.meta.url), "utf8").trim();
  checkCiPins(workflow, version, nodeVersion);
});

test("Node actions must select exactly the reviewed release", () => {
  const node = `      - uses: actions/setup-node@${SHA}\n        with:\n          node-version: 24.22.0\n`;
  checkCiPins(rust + node, "1.88.0", "24.22.0");
  for (const replacement of ["24", "lts/*", "24.22.1", "${{ inputs.node }}"]) {
    assert.throws(() => checkCiPins(rust + node.replace("24.22.0", replacement), "1.88.0", "24.22.0"), /with.node-version/);
  }
  assert.throws(() => checkCiPins(rust, "1.88.0", "24.22.0"), /Node/);
  assert.throws(() => checkCiPins(rust + node + node.replace("24.22.0", "24.22.1"), "1.88.0", "24.22.0"), /with.node-version/);
  const jobs = `jobs:\n  quality:\n    steps:\n${rust}${node}      - run: bash scripts/agent-check.sh fast\n  privacy:\n    steps:\n      - run: node scripts/privacy-guard.mjs\n`;
  assert.throws(() => checkCiPins(jobs, "1.88.0", "24.22.0"), /privacy.*Node/);
  assert.throws(() => checkCiPins(jobs.replace("privacy:", "release:").replace("node scripts/privacy-guard.mjs", "bash scripts/authority-differential-matrix.sh release"), "1.88.0", "24.22.0"), /release.*Node/);
  for (const setting of ["if: false", "continue-on-error: true", "continue-on-error: ${{ inputs.ignore }}"]) {
    assert.throws(() => checkCiPins(rust + node.replace("        with:", `        ${setting}\n        with:`), "1.88.0", "24.22.0"), /unconditional|continue-on-error/);
  }
  const named = node.replace("- uses:", "- name: Node\n        if: false\n        uses:");
  assert.throws(() => checkCiPins(rust + named, "1.88.0", "24.22.0"), /unconditional/);
  for (const leader of ["if: false", "continue-on-error: true", '"if": false', '"continue-on-error": true']) {
    assert.throws(() => checkCiPins(rust + node.replace("- uses:", `- ${leader}\n        uses:`), "1.88.0", "24.22.0"), /unconditional|continue-on-error/);
  }
});

test("accepts immutable actions, local actions, named steps, and quoted versions", () => {
  const named = rust.replace("- uses:", "- name: Rust\n        uses:");
  checkCiPins(
    `      - uses: actions/checkout@${SHA}\n      - uses: ./local\n`
      + named.replace("toolchain: 1.88.0", 'toolchain: "1.88.0" # version'),
    "1.88.0",
  );
});

for (const reference of ["v6", "main", "a".repeat(39), "a".repeat(41), "${{ inputs.ref }}"]) {
  test(`rejects non-immutable action reference ${reference}`, () => {
    assert.throws(
      () => checkCiPins(`${rust}      - uses: actions/checkout@${reference}\n`, "1.88.0"),
      /full commit SHA/,
    );
  });
}

test("checks every Rust step rather than accepting one matching reference", () => {
  assert.throws(
    () => checkCiPins(rust + rust.replace("1.88.0", "1.89.0"), "1.88.0"),
    /with.toolchain: 1.88.0/,
  );
});

test("rejects unsupported action declarations rather than overlooking mutable refs", () => {
  for (const declaration of [
    "      - { uses: actions/checkout@v6 }\n",
    '      - "uses": actions/checkout@v6\n',
    "      - uses:\n",
  ]) {
    assert.throws(
      () => checkCiPins(rust + declaration, "1.88.0"),
      /literal block-style/,
    );
  }
});

for (const replacement of [
  "",
  "          # toolchain: 1.88.0\n",
  "          toolchain: stable\n",
  "          toolchain: ${{ inputs.toolchain }}\n",
  "          toolchain: 1.88.0\n          toolchain: 1.88.0\n",
]) {
  test(`rejects absent, ambiguous, or nonliteral toolchain input ${JSON.stringify(replacement)}`, () => {
    assert.throws(
      () => checkCiPins(rust.replace("          toolchain: 1.88.0\n", replacement), "1.88.0"),
      /exactly one with.toolchain/,
    );
  });
}

test("does not borrow a toolchain from env or the next action", () => {
  assert.throws(
    () => checkCiPins(rust.replace("with:", "env:"), "1.88.0"),
    /with.toolchain/,
  );
  assert.throws(
    () => checkCiPins(
      `      - uses: dtolnay/rust-toolchain@${SHA}\n` + rust,
      "1.88.0",
    ),
    /with.toolchain/,
  );
});

test("rejects workflows without an active Rust action and invalid expected versions", () => {
  assert.throws(() => checkCiPins(`# ${rust}`, "1.88.0"), /must declare/);
  for (const version of [undefined, "", "stable", "1.88", "1.88.0\n"]) {
    assert.throws(() => checkCiPins(rust, version), /explicit Rust toolchain/);
  }
});
