import { createHash } from "node:crypto";
import { isAbsolute, posix, relative, resolve, sep } from "node:path";

export const sha256 = bytes => createHash("sha256").update(bytes).digest("hex");
const refuse = message => { throw new Error(`LCOV refused: ${message}`); };
const integer = (value, positive = false) => {
  if (!/^(0|[1-9][0-9]*)$/.test(value)) refuse("invalid counter");
  const n = Number(value);
  if (!Number.isSafeInteger(n) || (positive && n === 0)) refuse("unsafe or zero counter");
  return n;
};
function insert(map, key, value) {
  if (map.has(key)) refuse("duplicate record");
  map.set(key, value);
}
const metric = (covered, total) => ({covered, total, percent: total ? covered * 100 / total : null});
function add(metrics) {
  const covered = metrics.reduce((n, m) => n + m.covered, 0);
  const total = metrics.reduce((n, m) => n + m.total, 0);
  if (!Number.isSafeInteger(covered) || !Number.isSafeInteger(total)) refuse("unsafe aggregate");
  return metric(covered, total);
}

// Deliberately supports the detailed LLVM exporter subset, not every LCOV dialect.
export function parseLcov(text, {root, sources, llvm = null}) {
  if (typeof text !== "string" || text.includes("\0")) refuse("invalid text");
  const inventory = new Map();
  for (const source of sources) {
    if (!source.path || source.path.startsWith("/") || source.path.includes("\\") || posix.normalize(source.path) !== source.path || source.path.startsWith("../") || !/^[a-f0-9]{64}$/.test(source.sha256) || !Number.isSafeInteger(source.lines) || source.lines < 0) refuse("invalid source inventory");
    insert(inventory, source.path, source);
  }
  const files = new Map();
  function sourcePath(body) {
    if (typeof body !== "string" || body.includes("\\") || body.split("/").some(p => p === ".." || p === ".") || body.includes("//")) refuse("source alias");
    const path = isAbsolute(body) ? relative(resolve(root), body).split(sep).join("/") : body;
    if (!inventory.has(path) || !path.endsWith(".rs")) refuse("foreign or untracked source");
    return path;
  }
  const paired = new Map();
  if (llvm !== null) {
    if (llvm.type !== "llvm.coverage.json.export" || !["2.0.1","3.0.1"].includes(llvm.version) || llvm.data?.length !== 1 || !Array.isArray(llvm.data[0].files)) refuse("unsupported LLVM JSON summary");
    for (const file of llvm.data[0].files) {
      const summary={};
      for (const kind of ["lines","functions","instantiations","branches"]) {
        const total=integer(String(file.summary?.[kind]?.count)), covered=integer(String(file.summary?.[kind]?.covered));
        if (covered > total) refuse("invalid LLVM summary count");
        summary[kind]=metric(covered,total);
      }
      insert(paired,sourcePath(file.filename),summary);
    }
  }
  let record = null;
  function line(value) {
    const n = integer(value, true);
    if (n > record.source.lines) refuse("line exceeds source length");
    return n;
  }
  function finish() {
    for (const key of ["LF", "LH", "FNF", "FNH"]) if (!record.summaries.has(key)) refuse(`missing ${key}`);
    const counts = (map, expected, hits, hit = v => v > 0) => {
      const found = map.size;
      const covered = [...map.values()].filter(hit).length;
      if (record.summaries.get(expected) !== found || record.summaries.get(hits) !== covered) refuse("inconsistent summary");
      return metric(covered, found);
    };
    if (record.functions.size !== record.hits.size || [...record.functions.keys()].some(k => !record.hits.has(k))) refuse("unbound function count");
    const branchesPresent = record.branches.size || record.summaries.has("BRF") || record.summaries.has("BRH");
    const instantiations=metric([...record.hits.values()].filter(n=>n>0).length,record.hits.size);
    const functions=metric(record.summaries.get("FNH"),record.summaries.get("FNF"));
    if (functions.covered > functions.total || functions.total > instantiations.total || functions.covered > instantiations.covered || Boolean(functions.covered) !== Boolean(instantiations.covered) || Boolean(functions.total) !== Boolean(instantiations.total) || (instantiations.covered === instantiations.total && functions.covered !== functions.total)) refuse("inconsistent function groups");
    const file = {
      path: record.source.path, sha256: record.source.sha256,
      lines: counts(record.lines, "LF", "LH"),
      functions, instantiations,
      branches: branchesPresent ? counts(record.branches, "BRF", "BRH", v => v !== "-" && v > 0) : null,
    };
    if (llvm !== null) {
      const summary=paired.get(file.path);
      if (!summary || ["lines","functions","instantiations","branches"].some(k=>JSON.stringify(summary[k]) !== JSON.stringify(file[k]))) refuse("LCOV/LLVM summary mismatch");
    } else if (JSON.stringify(functions) !== JSON.stringify(instantiations)) refuse("function groups require paired LLVM summary");
    insert(files,record.source.path,file);
    record = null;
  }
  for (const raw of text.split(/\r?\n/)) {
    if (!raw) continue;
    if (raw === "end_of_record") {
      if (!record) refuse("orphan end_of_record");
      finish();
      continue;
    }
    const colon = raw.indexOf(":");
    if (colon < 1) refuse("malformed record");
    const tag = raw.slice(0, colon), body = raw.slice(colon + 1);
    if (tag === "SF") {
      if (record) refuse("unclosed source");
      const path = sourcePath(body);
      const source = inventory.get(path);
      if (!source || !path.endsWith(".rs")) refuse("foreign or untracked source");
      record = {source, functions:new Map(), hits:new Map(), lines:new Map(), branches:new Map(), summaries:new Map()};
      continue;
    }
    if (!record) refuse("record outside source");
    if (["LF", "LH", "FNF", "FNH", "BRF", "BRH"].includes(tag)) {
      insert(record.summaries, tag, integer(body));
    } else if (tag === "FN" || tag === "FNDA") {
      const comma = body.indexOf(","), name = body.slice(comma + 1);
      if (comma < 1 || !name || /[\r\x00-\x1f]/.test(name)) refuse("invalid function");
      insert(tag === "FN" ? record.functions : record.hits, name, tag === "FN" ? line(body.slice(0, comma)) : integer(body.slice(0, comma)));
    } else if (tag === "DA") {
      const fields = body.split(",");
      if (fields.length < 2 || fields.length > 3 || (fields.length === 3 && !/^[a-zA-Z0-9+/=_-]+$/.test(fields[2]))) refuse("invalid DA");
      insert(record.lines, line(fields[0]), integer(fields[1]));
    } else if (tag === "BRDA") {
      const fields = body.split(",");
      if (fields.length !== 4) refuse("invalid BRDA");
      const key = [line(fields[0]), integer(fields[1]), integer(fields[2])].join(",");
      insert(record.branches, key, fields[3] === "-" ? "-" : integer(fields[3]));
    } else refuse(`unsupported tag ${tag}`);
  }
  if (record) refuse("unterminated source");
  if (llvm !== null && (paired.size !== files.size || [...paired.keys()].some(p=>!files.has(p)))) refuse("LLVM/LCOV source set mismatch");
  const sorted = [...files.values()].sort((a,b) => a.path < b.path ? -1 : a.path > b.path ? 1 : 0);
  const totals = {lines:add(sorted.map(f => f.lines)), functions:add(sorted.map(f => f.functions)), instantiations:add(sorted.map(f => f.instantiations)), branches:sorted.every(f => f.branches !== null) ? add(sorted.map(f => f.branches)) : null};
  if (!totals.lines.total || !totals.functions.total || !totals.lines.covered || !totals.functions.covered) refuse("zero global denominator or covered count");
  return {files:sorted, totals, unreported:[...inventory.keys()].filter(p => !files.has(p)).sort()};
}
