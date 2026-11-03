import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import test from "node:test";

const workflow = readFileSync(new URL("../../.github/workflows/ci.yml", import.meta.url), "utf8");
const job = workflow.split("\n  quality:\n")[1]?.split("\n  cargo-test:\n")[0];
assert.ok(job, "required aggregate job must exist");
const block = job.split("        run: |\n")[1];
assert.ok(block, "execute the actual aggregate shell body");
const body = block.split("\n").filter((line) => line.startsWith("          ")).map((line) => line.slice(10)).join("\n");
const valid = { DEBUG_RESULT: "success", RELEASE_RESULT: "success", DEBUG_GENERAL: "a".repeat(64), RELEASE_GENERAL: "a".repeat(64), DEBUG_KEYRING: "b".repeat(64), RELEASE_KEYRING: "b".repeat(64) };
const run = (env) => spawnSync("bash", ["-e", "-c", body], { encoding: "utf8", env: { ...process.env, ...valid, ...env } });

test("the required context depends on both lanes and runs after failed dependencies", () => {
  assert.match(job, /needs: \[quality_checks, authority_release\]/);
  assert.match(job, /if: \$\{\{ always\(\) \}\}/);
  assert.match(job, /name: Rust quality and repository contract/);
  assert.doesNotMatch(job, /continue-on-error/);
  assert.match(job, /DEBUG_GENERAL: \$\{\{ needs\.quality_checks\.outputs\.general_input_sha256 \}\}/);
  assert.match(job, /RELEASE_GENERAL: \$\{\{ needs\.authority_release\.outputs\.general_input_sha256 \}\}/);
  assert.match(job, /DEBUG_KEYRING: \$\{\{ needs\.quality_checks\.outputs\.keyring_input_sha256 \}\}/);
  assert.match(job, /RELEASE_KEYRING: \$\{\{ needs\.authority_release\.outputs\.keyring_input_sha256 \}\}/);
  assert.equal(run({}).status, 0);
});

for (const lane of ["DEBUG_RESULT", "RELEASE_RESULT"]) {
  for (const result of ["failure", "cancelled", "skipped", ""]) {
    test(`the actual required gate refuses ${lane}=${result || "missing"}`, () => {
      assert.notEqual(run({ [lane]: result }).status, 0);
    });
  }
}
for (const field of ["DEBUG_GENERAL", "RELEASE_GENERAL", "DEBUG_KEYRING", "RELEASE_KEYRING"]) {
  test(`the actual required gate refuses missing or different ${field}`, () => {
    assert.notEqual(run({ [field]: "" }).status, 0);
    assert.notEqual(run({ [field]: "c".repeat(64) }).status, 0);
  });
}
