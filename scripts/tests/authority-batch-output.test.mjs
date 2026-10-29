import assert from "node:assert/strict";
import test from "node:test";
import { parseBatchAnswers } from "../lib/authority-batch-output.mjs";

const valid = '{"code":null,"result":"canonical state"}\n';
test("batch answers preserve exact successful and refused results", () => {
  assert.deepEqual(parseBatchAnswers(valid + '{"code":"schema_type","result":null}\n', 2), [{ code: null, result: "canonical state" }, { code: "schema_type", result: null }]);
});
for (const value of [null, [], {}, { code: null }, { code: null, result: 1 }, { code: false, result: null }, { code: null, result: "ok", extra: true }]) {
  test(`malformed output is retained against its input index: ${JSON.stringify(value)}`, () => {
    assert.throws(() => parseBatchAnswers(valid + JSON.stringify(value) + "\n", 2), (error) => error.caseIndex === 1);
  });
}
test("short, extra, and non-JSON answers identify a bounded reproduction index", () => {
  assert.throws(() => parseBatchAnswers("", 2), (error) => error.caseIndex === 0);
  assert.throws(() => parseBatchAnswers(valid, 2), (error) => error.caseIndex === 1);
  assert.throws(() => parseBatchAnswers(valid + valid, 1), (error) => error.caseIndex === 0);
  assert.throws(() => parseBatchAnswers(valid + "not json\n", 2), (error) => error.caseIndex === 1);
  assert.throws(() => parseBatchAnswers(valid + '{"code":null,"code":"schema_type","result":null}\n', 2), (error) => error.caseIndex === 1);
  assert.throws(() => parseBatchAnswers(valid + '{"code":null,"result":"ok","__proto__":1}\n', 2), (error) => error.caseIndex === 1);
});
