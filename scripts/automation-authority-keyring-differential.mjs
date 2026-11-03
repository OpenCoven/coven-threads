#!/usr/bin/env node
// Production companion differential over ordered keyring pairs. Artifact
// bodies cross the strict parser; transport never sorts or collapses ids.
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import * as V from "../profiles/automation-authority/v1/validator.mjs";
import { keyringCases } from "./lib/authority-keyring-corpus.mjs";
import { parseBatchAnswers } from "./lib/authority-batch-output.mjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const profile = join(root, "profiles/automation-authority/v1");
const read = (name) => V.strictParseJson(readFileSync(join(profile, name), "utf8"));
const binary = process.argv[2];
if (!binary || process.argv.slice(3).some((argument) => argument !== "--ordered")) {
  throw new Error("usage: automation-authority-keyring-differential.mjs <ordered-keyring-batch> [--ordered]");
}
const keys = read("keyring.json").keys;
const cases = [];
const expected = [];
let validControls = 0;
const references = {
  validate_request: (body, keyring) => V.validateAuthorizationRequest(body.request, { keyring }),
  validate_decision: (body, keyring) => V.validateDecision(body.decision, { keyring }),
  validate_approval: (body, keyring) => V.validateApproval(body.approval, { keyring, now: body.now }),
  validate_consumption_snapshot: (body, keyring) => V.validateConsumptionSnapshot(body.consumption_snapshot, { keyring, now: body.now }),
  validate_proposal: (body, keyring) => V.validateProposal(body.proposal, { keyring }),
  evidence_read: (body, keyring) => V.authorizeEvidenceRead(body.read, body.evidence, { keyring, now: body.now }),
  lifecycle: (body, keyring) => {
    let state = null;
    for (const event of body.events) state = V.applyLifecycleEvent(state, event, { approval: body.approval, keyring });
    return state;
  },
};
function canonical(value) {
  if (value === null || typeof value !== "object") return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${canonical(value[key])}`).join(",")}}`;
}
function answer(operation, body, keys) {
  const result = references[operation](body, new Map(Object.entries(keys)));
  return { code: null, result: operation === "lifecycle" ? canonical(result) : "ok" };
}
// Project every signed artifact in positive vectors through its production
// validator, including approval/consumption records embedded in dispatches.
// This isolates key verification from unrelated decision/lifecycle checks.
const sources = read("manifest.json").vectors.filter((vector) => vector.kind === "positive");
for (const vector of sources) {
  const sourceBody = read(`vectors/${vector.file}`);
  const operations = [
    ["request", "validate_request"], ["decision", "validate_decision"], ["approval", "validate_approval"],
    ["consumption_snapshot", "validate_consumption_snapshot"],
    ["proposal", "validate_proposal"], ["read", "evidence_read"],
    ["approval_authorization_request", "validate_request", "request"],
    ["approval_authorization_decision", "validate_decision", "decision"],
    ["events", "lifecycle"],
  ].filter(([member]) => sourceBody[member] && (member !== "events" || sourceBody.events.length));
  for (const [sourceMember, operation, targetMember] of operations) {
    const member = targetMember ?? sourceMember;
    const body = { ...sourceBody, [member]: sourceBody[sourceMember] };
    for (const mutation of keyringCases(keys)) {
      cases.push({ operation, body, keys: mutation.keys, label: `${vector.id}/${sourceMember}/${mutation.label}` });
      try {
        const expectedAnswer = answer(operation, body, mutation.keys);
        if (mutation.label === "valid-control") validControls++;
        expected.push(expectedAnswer);
      } catch (error) {
        if (mutation.label === "valid-control") throw error;
        expected.push({ code: error instanceof V.AuthorityError ? error.code : `THROW:${error.name}`, result: null });
      }
    }
    for (const [field, value] of [["alg", "rsa"], ["signed_digest", "0".repeat(64)], ["signature_b64", "A".repeat(86) + "=="]]) {
      const changed = structuredClone(body);
      if (member === "events") changed.events[0].integrity[field] = value;
      else changed[member].integrity[field] = value;
      cases.push({ operation, body: changed, keys, label: `${vector.id}/${operation}/tampered:${field}` });
      try {
        expected.push(answer(operation, changed, keys));
      } catch (error) {
        expected.push({ code: error instanceof V.AuthorityError ? error.code : `THROW:${error.name}`, result: null });
      }
    }
  }
}
// A JavaScript Map cannot contain duplicates. Compare the strict JSON
// keyring boundary to Rust's pair constructor without collapsing either.
for (const role of ["principal", "threads_authority", "unknown"]) {
  const records = [["duplicate", { role: "threads_authority" }], ["duplicate", { role }]];
  const rawKeys = `{${records.map(([id, record]) => `${JSON.stringify(id)}:${JSON.stringify(record)}`).join(",")}}`;
  cases.push({ operation: "validate_keyring", body: {}, keys: {}, key_records: records, label: `duplicate-constructor:${role}`, raw_keyring: rawKeys });
  try {
    V.validateKeyring(V.strictParseJson(rawKeys));
    expected.push({ code: null, result: "ok" });
  } catch (error) {
    expected.push({ code: error instanceof V.AuthorityError ? error.code : `THROW:${error.name}`, result: null });
  }
}
const serialize = (item) => {
  const body = JSON.stringify(item.body);
  return `${JSON.stringify({ operation: item.operation, body_text: body, key_records: item.key_records ?? Object.entries(item.keys) })}\n`;
};
const input = cases.map(serialize).join("");
const probe = spawnSync(binary, ["--describe"], { encoding: "utf8" });
if (probe.status !== 0) throw new Error(`build probe failed: ${probe.stderr}`);
const build = JSON.parse(probe.stdout);
const run = spawnSync(binary, [], { input, encoding: "utf8", maxBuffer: 64 << 20 });
let actual;
try {
  actual = parseBatchAnswers(run.stdout ?? "", cases.length);
  if (run.status !== 0) throw Object.assign(new Error(`batch failed (${run.status}): ${run.stderr}`), { caseIndex: cases.length - 1 });
} catch (error) {
  const directory = mkdtempSync(join(tmpdir(), "authority-keyring-failures-"));
  const index = error.caseIndex ?? 0;
  writeFileSync(join(directory, "cases.txt"), serialize(cases[index]));
  writeFileSync(join(directory, "reference-cases.json"), JSON.stringify([cases[index]], null, 2));
  writeFileSync(join(directory, "report.json"), JSON.stringify({
    kind: "batch transport/execution failure", error: error.message, status: run.status, signal: run.signal,
    index, label: cases[index].label, node: process.version, v8: process.versions.v8,
    openssl: process.versions.openssl, platform: `${process.platform}/${process.arch}`, build,
    inputSha256: createHash("sha256").update(input).digest("hex"),
    binarySha256: createHash("sha256").update(readFileSync(binary)).digest("hex"),
  }, null, 2));
  console.error(`Batch failure retained in ${directory}; reproduce: ${binary} < ${join(directory, "cases.txt")}`);
  process.exit(1);
}
const source = readFileSync(join(profile, "validator.mjs"), "utf8");
const codes = new Set([
  /\bfail\(\s*"([a-z0-9_]+)"/g,
  /\bexact\(\s*[^,]+,\s*[^,]+,\s*"([a-z0-9_]+)"/g,
  /\bassertBinding\(\s*[^,]+,\s*[^,]+,\s*[^,]+,\s*"([a-z0-9_]+)"/g,
].flatMap((pattern) => [...source.matchAll(pattern)].map((match) => match[1])));
const mismatches = [];
const byOperation = {};
const byCode = {};
let undefinedCount = 0;
let accepted = 0;
for (let index = 0; index < cases.length; index++) {
  const want = expected[index];
  const got = actual[index];
  byOperation[cases[index].operation] = (byOperation[cases[index].operation] ?? 0) + 1;
  byCode[want.code ?? "success"] = (byCode[want.code ?? "success"] ?? 0) + 1;
  if (want.code?.startsWith("THROW:")) {
    undefinedCount++;
    if (codes.has(got.code) && got.result === null) continue;
  } else if (want.code === got.code && want.result === got.result) {
    if (!want.code) accepted++;
    continue;
  }
  mismatches.push({ index, label: cases[index].label, expected: want, actual: got });
}
const hash = (bytes) => createHash("sha256").update(bytes).digest("hex");
if (!validControls || !byCode.integrity_role_unknown) throw new Error("missing valid controls or unknown-role coverage");
const summary = { node: process.version, openssl: process.versions.openssl, platform: `${process.platform}/${process.arch}`, build, cases: cases.length, validControls, accepted, undefinedReference: undefinedCount, mismatches: mismatches.length, byOperation, byCode, inputSha256: hash(input), binarySha256: hash(readFileSync(binary)), referenceSha256: hash(source) };
console.log(JSON.stringify(summary));
for (const mismatch of mismatches.slice(0, 20)) console.log(JSON.stringify(mismatch));
if (mismatches.length) {
  const directory = mkdtempSync(join(tmpdir(), "authority-keyring-failures-"));
  writeFileSync(join(directory, "cases.txt"), mismatches.map(({ index }) => serialize(cases[index])).join(""));
  writeFileSync(join(directory, "report.json"), JSON.stringify({ ...summary, failures: mismatches }, null, 2));
  writeFileSync(join(directory, "reference-cases.json"), JSON.stringify(mismatches.map(({ index }) => cases[index]), null, 2));
  console.log(`Retained synthetic failing cases: ${directory}; reproduce: ${binary} < ${join(directory, "cases.txt")}`);
}
process.exitCode = mismatches.length ? 1 : 0;
