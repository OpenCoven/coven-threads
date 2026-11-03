import assert from "node:assert/strict";
import test from "node:test";
import { compareAnswer } from "../lib/authority-differential-answer.mjs";

const codes = new Set(["json_duplicate_key", "schema_type"]);
const throwing = { code: "THROW:TypeError", result: null };
const refused = { code: "schema_type", result: null };
const accepted = { code: null, result: "canonical document" };

test("a normative duplicate-key refusal cannot become an unspecified exception", () => {
  const result = compareAnswer(throwing, refused, { code: "json_duplicate_key" }, null, null, codes);
  assert.deepEqual(result.failures, [
    "reference did not refuse with json_duplicate_key",
    "port did not refuse with json_duplicate_key",
  ]);
  assert.equal(result.failOpen, false);
});

test("an equivalent spelling cannot throw instead of matching a successful anchor", () => {
  const result = compareAnswer(throwing, refused, { equals: 0 }, accepted, accepted, codes);
  assert.deepEqual(result.failures, ["reference answer differs from case 0", "port answer differs from case 0"]);
});

test("genuinely unspecified exceptions still require a named refusal with no result", () => {
  assert.deepEqual(compareAnswer(throwing, refused, null, null, null, codes), { failures: [], failOpen: false });
  for (const got of [accepted, { code: "made_up", result: null }, { code: "schema_type", result: "leak" }]) {
    assert.equal(compareAnswer(throwing, got, null, null, null, codes).failOpen, true);
  }
});

test("specified outcomes and successful normalized results remain exact", () => {
  assert.deepEqual(compareAnswer(accepted, accepted, { equals: 0 }, accepted, accepted, codes), { failures: [], failOpen: false });
  assert.deepEqual(compareAnswer(refused, accepted, null, null, null, codes).failures, ["answers differ"]);
  assert.deepEqual(compareAnswer(accepted, { code: null, result: "different" }, null, null, null, codes).failures, ["answers differ"]);
});
