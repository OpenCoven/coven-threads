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
  checkCiPins(workflow, version);
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
