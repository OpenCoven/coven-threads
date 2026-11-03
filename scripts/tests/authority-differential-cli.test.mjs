import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";

const root = fileURLToPath(new URL("../..", import.meta.url));
for (const [name, value] of [["SEED", "-1"], ["SEED", "1.5"], ["SEED", "4294967296"], ["BUDGET", "0"], ["BUDGET", "1"], ["BUDGET", "257"]]) {
  test(`invalid ${name}=${value} fails before invoking a batch or generating cases`, () => {
    const result = spawnSync(process.execPath, ["scripts/automation-authority-differential.mjs", "not-invoked-batch"], { cwd: root, encoding: "utf8", env: { ...process.env, [`AUTHORITY_DIFFERENTIAL_${name}`]: value } });
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, new RegExp(name.toLowerCase()));
    assert.doesNotMatch(result.stderr, /batch binary --describe/);
  });
}
test("the companion refuses obsolete transport modes before invoking a batch", () => {
  const result = spawnSync(process.execPath, ["scripts/automation-authority-keyring-differential.mjs", "not-invoked-batch", "--framed"], { cwd: root, encoding: "utf8" });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /usage:.*ordered-keyring-batch/);
});
