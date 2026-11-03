import assert from "node:assert/strict";
import test from "node:test";
import { validateMatrix } from "../lib/authority-matrix-receipt.mjs";

// Receipt parser fixtures only. Real matrix cells build and run ring-backed
// production examples; no verifier backend is mocked here.
const operations = ["validate_request", "request_adoption", "decision_consumption", "lifecycle"];
function fixture() {
  return ["debug", "release"].flatMap((profile) => ["ordinary", "arbitrary_precision"].map((feature) => {
    const common = { node: process.version, inputSha256: "a".repeat(64), binarySha256: "b".repeat(64), build: { serde_json_arbitrary_precision: feature === "arbitrary_precision", debug_assertions: profile === "debug" } };
    return {
      profile, feature,
      general: { ...structuredClone(common), cases: 200, vectors: 132, mismatched: 0, failOpen: 0, multifaults: { pairs: 12, triples: 12 }, byOperation: Object.fromEntries(operations.map((operation) => [operation, 50])), replayResults: Object.fromEntries(operations.slice(1).map((operation) => [operation, { compared: 10, nonNullState: 10 }])) },
      keyring: { ...structuredClone(common), cases: 800, validControls: 8, mismatches: 0, byCode: { integrity_role_unknown: 1, json_duplicate_key: 1 } },
    };
  }));
}

test("complete receipts require all four actual profile/feature cells and common inputs", () => {
  assert.deepEqual(validateMatrix(fixture(), ["debug", "release"], operations, process.version), { generalInputSha256: "a".repeat(64), keyringInputSha256: "a".repeat(64) });
});

for (const [label, mutate] of [
  ["missing cell", (entries) => entries.pop()],
  ["duplicate cell", (entries) => { entries[3] = structuredClone(entries[0]); }],
  ["wrong feature", (entries) => { entries[1].general.build.serde_json_arbitrary_precision = false; }],
  ["wrong profile", (entries) => { entries[2].keyring.build.debug_assertions = true; }],
  ["missing outcome", (entries) => { delete entries[0].general.failOpen; }],
  ["fail-open", (entries) => { entries[0].general.failOpen = 1; }],
  ["keyring mismatch", (entries) => { entries[0].keyring.mismatches = 1; }],
  ["empty triple corpus", (entries) => { entries[0].general.multifaults.triples = 0; }],
  ["missing role coverage", (entries) => { delete entries[0].keyring.byCode.integrity_role_unknown; }],
  ["missing operation", (entries) => { delete entries[0].general.byOperation.validate_request; }],
  ["only null replay states", (entries) => { entries[0].general.replayResults.decision_consumption.nonNullState = 0; }],
  ["wrong tool", (entries) => { entries[0].general.node = "v0.0.0"; }],
  ["missing digest", (entries) => { delete entries[0].general.inputSha256; }],
  ["different release input", (entries) => { entries[2].general.inputSha256 = "c".repeat(64); }],
]) {
  test(`rejects ${label} instead of inventing successful matrix evidence`, () => {
    const entries = fixture();
    mutate(entries);
    assert.throws(() => validateMatrix(entries, ["debug", "release"], operations, process.version));
  });
}
