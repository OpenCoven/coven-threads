// Bounded, deterministic sampling of distinct field faults. Control fields
// belong to the runner, not to an authority artifact, and cannot be sampled.
const CONTROL = new Set(["repetitions", "keyring_mutation", "raw_json", "lifecycle_summary"]);

function* sites(value, path, inArray, structural, alternativesFor) {
  const container = value && typeof value === "object" && Object.keys(value).length;
  if (path.length && (structural || !container)) {
    const choices = new Map();
    const original = JSON.stringify(value);
    for (const alternative of alternativesFor(value, path.at(-1))) {
      const encoded = JSON.stringify(alternative);
      if (encoded !== undefined && encoded !== original) choices.set(`set:${encoded}`, { value: alternative });
    }
    if (structural && !inArray) choices.set("delete", { remove: true });
    if (choices.size) yield { path, choices: [...choices.values()] };
  }
  if (structural && path.length && value && typeof value === "object" && !Array.isArray(value)) {
    for (const key of ["AAA", "zz_unknown"]) {
      if (!Object.hasOwn(value, key)) yield { path: [...path, key], choices: [{ value: 1 }] };
    }
  }
  if (container) {
    for (const [key, child] of Object.entries(value)) {
      if (path.length === 0 && CONTROL.has(key)) continue;
      // Re-signing must not repair away a sampled change. Signature mutation
      // already has its own full corpus in the main differential.
      if (key === "integrity") continue;
      yield* sites(child, [...path, Array.isArray(value) ? Number(key) : key], Array.isArray(value), structural, alternativesFor);
    }
  }
}

function changed(value) {
  if (typeof value === "string") return `${value}:multifault`;
  if (typeof value === "number") return value === 0 ? 1 : 0;
  if (typeof value === "boolean") return !value;
  return value === null ? "multifault" : null;
}

export function sampleMultifaults(body, { seed = 88, budget = 24, structural = false, alternativesFor = (value) => [changed(value)] } = {}) {
  if (!Number.isInteger(seed) || seed < 0 || seed > 0xffffffff) throw new Error("seed must be a uint32");
  if (!Number.isInteger(budget) || budget < 2 || budget > 256) throw new Error("budget must be 2..256");
  const fields = [...sites(body, [], false, structural, alternativesFor)].sort((a, b) => {
    const left = JSON.stringify(a.path);
    const right = JSON.stringify(b.path);
    return left < right ? -1 : left > right ? 1 : 0;
  });
  if (fields.length < 2) return [];
  let state = seed;
  const random = () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let value = state;
    value = Math.imul(value ^ (value >>> 15), value | 1);
    value ^= value + Math.imul(value ^ (value >>> 7), value | 61);
    return ((value ^ (value >>> 14)) >>> 0) / 2 ** 32;
  };
  const seen = new Set();
  const bodies = new Set();
  const result = [];
  for (let attempt = 0; attempt < budget * 32 && result.length < budget; attempt++) {
    const arity = fields.length > 2 && attempt % 2 ? 3 : 2;
    const pool = fields.map((_, index) => index);
    for (let index = 0; index < arity; index++) {
      const selected = index + Math.floor(random() * (pool.length - index));
      [pool[index], pool[selected]] = [pool[selected], pool[index]];
    }
    const selected = pool.slice(0, arity).sort((a, b) => a - b);
    if (selected.some((index, position) => selected.slice(position + 1).some((other) => {
      const left = fields[index].path;
      const right = fields[other].path;
      return left.every((key, depth) => right[depth] === key) || right.every((key, depth) => left[depth] === key);
    }))) continue;
    const variants = selected.map((index) => Math.floor(random() * fields[index].choices.length));
    const identity = selected.map((index, position) => `${index}:${variants[position]}`).join(",");
    if (seen.has(identity)) continue;
    seen.add(identity);
    const mutant = structuredClone(body);
    const paths = selected.map((index) => fields[index].path);
    for (const [position, index] of selected.entries()) {
      const { path, choices } = fields[index];
      const choice = choices[variants[position]];
      let parent = mutant;
      for (const key of path.slice(0, -1)) parent = parent[key];
      if (choice.remove) delete parent[path.at(-1)];
      else Object.defineProperty(parent, path.at(-1), { value: structuredClone(choice.value), writable: true, enumerable: true, configurable: true });
    }
    const encoded = JSON.stringify(mutant);
    if (bodies.has(encoded)) continue;
    bodies.add(encoded);
    result.push({ body: mutant, paths, arity, seed });
  }
  return result;
}

export function minimizeFaults(base, mutant, paths, isFailure) {
  if (paths.length > 3) throw new Error("at most three sampled fields may be reduced");
  const subsets = Array.from({ length: 2 ** paths.length }, (_, mask) =>
    paths.filter((_, index) => mask & (1 << index)));
  subsets.sort((left, right) => left.length - right.length);
  for (const subset of subsets) {
    const body = structuredClone(base);
    for (const path of subset) {
      let parent = body;
      let source = mutant;
      for (const key of path.slice(0, -1)) {
        parent = parent[key];
        source = source[key];
      }
      if (!Object.hasOwn(source, path.at(-1))) delete parent[path.at(-1)];
      else Object.defineProperty(parent, path.at(-1), { value: structuredClone(source[path.at(-1)]), writable: true, enumerable: true, configurable: true });
    }
    if (isFailure(body)) return { body, paths: subset };
  }
  return null;
}
