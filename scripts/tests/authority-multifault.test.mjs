import assert from "node:assert/strict";
import test from "node:test";
import { minimizeFaults, sampleMultifaults } from "../lib/authority-multifault.mjs";

const body = { a: 1, b: 2, c: 3, d: 4, e: 5, f: 6 };

test("same seed and input reproduce a bounded mix of actual pairs and triples", () => {
  const options = { seed: 88, budget: 12 };
  const first = sampleMultifaults(body, options);
  assert.equal(first.length, 12);
  assert.deepEqual(sampleMultifaults(body, options), first);
  assert.notDeepEqual(sampleMultifaults(body, { ...options, seed: 89 }), first);
  assert.deepEqual(new Set(first.map((item) => item.arity)), new Set([2, 3]));
  for (const item of first) {
    const changes = Object.keys(body).filter((key) => body[key] !== item.body[key]);
    assert.equal(changes.length, item.arity);
    assert.equal(item.paths.length, item.arity);
  }
  assert.equal(new Set(first.map((item) => JSON.stringify(item.body))).size, first.length);
  assert.deepEqual(body, { a: 1, b: 2, c: 3, d: 4, e: 5, f: 6 });
});

test("sampling never mutates repetition counts or vector keyring metadata", () => {
  const source = { request: { id: "r", enabled: true, attempt: 1 }, repetitions: 2, keyring_mutation: { set: { role: "principal" } } };
  const copy = structuredClone(source);
  const cases = sampleMultifaults(source, { seed: 0, budget: 8 });
  assert.ok(cases.length > 0);
  for (const item of cases) {
    assert.equal(item.body.repetitions, 2);
    assert.deepEqual(item.body.keyring_mutation, source.keyring_mutation);
  }
  assert.deepEqual(source, copy);
});

test("small documents terminate without inventing repeated or overlapping faults", () => {
  assert.deepEqual(sampleMultifaults({ a: 1 }, { seed: 88, budget: 12 }), []);
  const cases = sampleMultifaults({ a: 1, b: 2 }, { seed: 88, budget: 12 });
  assert.equal(cases.length, 1);
  assert.equal(cases[0].arity, 2);
});

test("invalid seeds and budgets fail instead of silently disabling the corpus", () => {
  for (const seed of [-1, 2 ** 32, 1.5, NaN]) assert.throws(() => sampleMultifaults(body, { seed, budget: 12 }), /seed/);
  for (const budget of [0, 1, -1, 1.5, 257, NaN]) assert.throws(() => sampleMultifaults(body, { seed: 88, budget }), /budget/);
});

test("nested arrays and empty containers change only disjoint semantic fields", () => {
  const source = { request: { values: [1, false, {}, []], integrity: { alg: "ed25519", signature_b64: "synthetic" } }, snapshot: {} };
  const copy = structuredClone(source);
  const cases = sampleMultifaults(source, { seed: 88, budget: 24 });
  assert.ok(cases.length > 0);
  for (const item of cases) {
    assert.deepEqual(item.body.request.integrity, source.request.integrity);
    for (const path of item.paths) {
      const at = (object) => path.reduce((value, key) => value[key], object);
      assert.notDeepEqual(at(item.body), at(source));
      for (const other of item.paths) {
        if (path === other) continue;
        assert.ok(!path.every((key, index) => other[index] === key));
      }
    }
    assert.deepEqual(JSON.parse(JSON.stringify(item.body)), item.body);
  }
  assert.deepEqual(source, copy);
});

test("counterexample reduction finds the smallest failing sampled field set", () => {
  const original = { a: 1, b: 2, nested: { c: 3 } };
  const mutant = { a: 0, b: 0, nested: { c: 0 } };
  const paths = [["a"], ["b"], ["nested", "c"]];
  let probes = 0;
  const minimal = minimizeFaults(original, mutant, paths, (candidate) => {
    probes++;
    return candidate.a === 0 && candidate.nested.c === 0;
  });
  assert.deepEqual(minimal.body, { a: 0, b: 2, nested: { c: 0 } });
  assert.deepEqual(minimal.paths, [["a"], ["nested", "c"]]);
  assert.ok(probes <= 8);
  assert.deepEqual(original, { a: 1, b: 2, nested: { c: 3 } });
  assert.deepEqual(minimizeFaults(original, mutant, paths, () => false), null);
});

test("the production vocabulary composes replacements, deletions, and unknown members", () => {
  const source = { request: { a: 1, b: 2, c: 3 }, snapshot: { enabled: true } };
  const options = { seed: 88, budget: 256, structural: true, alternativesFor: (value) => typeof value === "number" ? [7, 8] : [null, {}] };
  const cases = sampleMultifaults(source, options);
  assert.deepEqual(sampleMultifaults(source, options), cases);
  assert.ok(cases.some((item) => item.body.request?.a === 7 || item.body.request?.a === 8));
  assert.ok(cases.some((item) => item.paths.some((path) => JSON.stringify(path) === '["request","a"]') && item.body.request && !Object.hasOwn(item.body.request, "a")));
  assert.ok(cases.some((item) => item.body.request?.AAA === 1 || item.body.request?.zz_unknown === 1));
  for (const item of cases) {
    for (const path of item.paths) {
      const at = (root) => path.reduce((value, key) => value?.[key], root);
      assert.notDeepEqual(at(item.body), at(source));
      for (const other of item.paths) if (other !== path) assert.ok(!path.every((key, index) => other[index] === key));
    }
  }
});

test("counterexample reduction preserves actual deletion instead of setting undefined", () => {
  const base = { a: 1, b: 2, c: 3 };
  const mutant = { a: 0, c: 0 };
  const result = minimizeFaults(base, mutant, [["a"], ["b"], ["c"]], (candidate) => !Object.hasOwn(candidate, "b") && candidate.c === 0);
  assert.deepEqual(result?.body, { a: 1, c: 0 });
  assert.deepEqual(result?.paths, [["b"], ["c"]]);
});
