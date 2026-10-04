#!/usr/bin/env node
// Differential test of the Rust automation-authority port
// (coven-threads-core::automation_authority) against the reference
// profiles/automation-authority/v1/validator.mjs, over the operations the port
// covers.
//
// Each evaluation vector runs as written, then re-signed with fresh keys of
// the same roles. The fresh keys come from fixed seeds and Ed25519 signing is
// deterministic, so every run builds the same cases and a mismatch reproduces. Then every member of its request, decision and policy is
// mutated, raw and re-signed, and for decision verification the mutated
// request also gets a freshly computed, consistently signed decision. Signature
// tampering is added explicitly. Both validators must report the same first
// error code, or the same result: the full canonical decision for
// evaluate_request, the outcome for verify_decision.
//
// Where the reference throws something other than an AuthorityError (a
// TypeError on a null member, say), it defines no code, and the case is
// counted but not compared.
//
// Usage: node scripts/automation-authority-differential.mjs <batch binary>

import { createHash, createPrivateKey, createPublicKey } from "node:crypto";
import { closeSync, mkdtempSync, openSync, readFileSync, rmSync, writeSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import * as V from "../profiles/automation-authority/v1/validator.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const PROFILE = join(ROOT, "profiles/automation-authority/v1");
const batch = process.argv[2];
if (!batch) {
  console.error("usage: automation-authority-differential.mjs <batch binary>");
  process.exit(2);
}
const PORTED = new Set([
  "strict_parse",
  "validate_request",
  "request_adoption",
  "evaluate_request",
  "verify_decision",
  "decision_consumption",
]);
const DOMAIN = {
  request: "opencoven:automation-request:v1",
  decision: "opencoven:automation-decision:v1",
};
const read = (path) => V.strictParseJson(readFileSync(path, "utf8"));

// The reference's canonical form, to compare whole decisions.
function canonical(value) {
  if (value === null || typeof value !== "object") return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  return `{${Object.keys(value)
    .sort()
    .map((key) => `${JSON.stringify(key)}:${canonical(value[key])}`)
    .join(",")}}`;
}

// The test keyring, and a fresh one with the same roles whose private keys
// this script holds, so mutated artifacts can be re-signed.
const original = read(join(PROFILE, "keyring.json")).keys;
const fresh = {};
const signers = {};
// An Ed25519 PKCS#8 private key is this prefix followed by the 32-byte seed.
const PKCS8_ED25519 = Buffer.from("302e020100300506032b657004220420", "hex");
for (const [id, record] of Object.entries(original)) {
  const seed = createHash("sha256").update(`automation-authority-differential:${id}`).digest();
  const privateKey = createPrivateKey({
    key: Buffer.concat([PKCS8_ED25519, seed]),
    format: "der",
    type: "pkcs8",
  });
  const publicKey = createPublicKey(privateKey);
  fresh[id] = { ...record, public_key_pem: publicKey.export({ type: "spki", format: "pem" }) };
  signers[id] = { key_id: id, private_key: privateKey };
}
const authority = Object.keys(fresh).find((id) => fresh[id].role === "threads_authority");

function mutateKeyring(keys, mutation) {
  const copy = structuredClone(keys);
  if (mutation) {
    const record = copy[mutation.key_id];
    if (mutation.remove) delete record[mutation.remove];
    if (mutation.set) Object.assign(record, mutation.set);
  }
  return copy;
}

function resign(artifact, domain) {
  const keyId = artifact?.integrity?.key_id;
  if (!artifact || typeof artifact !== "object" || Array.isArray(artifact) || !signers[keyId]) {
    return artifact;
  }
  try {
    return V.signArtifact(artifact, domain, signers[keyId]);
  } catch {
    return artifact;
  }
}

function resignBody(body) {
  const copy = structuredClone(body);
  if (copy.request) copy.request = resign(copy.request, DOMAIN.request);
  if (copy.decision) copy.decision = resign(copy.decision, DOMAIN.decision);
  return copy;
}

// The reference, run as run-vectors.mjs runs it.
function reference(operation, body, keys) {
  const keyring = new Map(Object.entries(keys));
  try {
    switch (operation) {
      case "strict_parse":
        V.strictParseJson(body.raw_json);
        return { code: null, result: "ok" };
      case "validate_request":
        V.validateAuthorizationRequest(body.request, { keyring });
        return { code: null, result: "ok" };
      case "request_adoption": {
        let state = null;
        for (let index = 0; index < (body.repetitions ?? 1); index += 1) {
          state = V.adoptAuthorizationRequest(body.request, state, { keyring, now: body.now });
        }
        return { code: null, result: "ok" };
      }
      case "evaluate_request":
        return {
          code: null,
          result: canonical(V.evaluateAuthorization(body.request, body.policy, { keyring, unsigned: true })),
        };
      case "verify_decision":
        return {
          code: null,
          result: V.verifyDecisionBundle(body.request, body.decision, body.policy, { keyring }).outcome,
        };
      case "decision_consumption": {
        let state = null;
        for (let index = 0; index < (body.repetitions ?? 1); index += 1) {
          state = V.consumeDecision(body.decision, state, { keyring });
        }
        return { code: null, result: "ok" };
      }
      default:
        throw new Error(`unsupported operation ${operation}`);
    }
  } catch (error) {
    if (error instanceof V.AuthorityError) return { code: error.code, result: null };
    return { code: `THROW:${error.name}`, result: null };
  }
}

// Mutation vocabulary: the profile's enums, so a member can take each other
// legal value as well as illegal ones.
const ENUMS = [
  ["R0", "R1", "R2", "R3", "R4"],
  ["permit", "requires_approval", "degrade_to_proposal", "reject"],
  ["analysis.read", "artifact.write", "state.mutate", "network.fetch", "network.publish",
    "credential.use", "evidence.read", "identity.mutate", "authority.admin", "release.publish",
    "resource.delete"],
  ["analysis.read", "artifact.create", "state.migrate", "network.fetch", "external.publish",
    "identity.mutate", "authority.change", "release.publish", "resource.delete"],
  ["public", "internal", "confidential", "restricted"],
  ["ephemeral_24h", "authority_evidence_90d", "authority_evidence_1y"],
  ["filesystem", "network", "credential", "evidence"],
  ["project", "workspace"],
  ["read", "write"],
  ["GET", "POST", "PUT"],
  ["human_per_run", "protected_owner_per_run"],
  ["deterministic_validation", "rollback_plan", "automation_imported", "automation_new"],
  ["principal:alice", "principal:bob", "principal:owner"],
];
const TIMESTAMP = /^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?Z$/;

// Every timestamp in the case being mutated, so a timestamp can also equal
// another: the comparisons' `<`, `<=` and `>=` boundaries sit there.
let timestampPool = [];

function alternatives(value) {
  if (typeof value === "string") {
    const out = ["", "x"];
    for (const list of ENUMS) if (list.includes(value)) out.push(...list.filter((item) => item !== value));
    if (/^sha256:[0-9a-f]{64}$/.test(value)) {
      out.push(value.slice(0, -1) + (value.endsWith("0") ? "1" : "0"), value.slice(7));
    }
    if (/^[0-9a-f]{64}$/.test(value)) out.push(value.slice(0, -1) + (value.endsWith("0") ? "1" : "0"));
    if (/^[A-Za-z0-9+/]{86}==$/.test(value)) {
      // A canonical signature over something else, and one that is not canonical.
      out.push(value.slice(0, 10) + (value[10] === "A" ? "B" : "A") + value.slice(11), value.slice(0, 85) + "B==");
    }
    if (TIMESTAMP.test(value)) {
      const ms = Date.parse(value);
      for (const delta of [-86400000, -1000, -1, 1, 1000, 86400000]) out.push(new Date(ms + delta).toISOString());
      out.push("2026-02-30T00:00:00Z", "2026-01-01T24:00:00Z", value.replace("Z", "+00:00"),
        value.replace(/(\.\d+)?Z$/, ".5Z"), value.replace(/(\.\d+)?Z$/, ".123456Z"),
        "2000-01-01T00:00:00Z", "2999-12-31T23:59:59Z", ...timestampPool);
    }
    if (value.startsWith("project/") || value.includes("/")) out.push("*", "../x", "/abs", "a");
    return out.filter((item) => item !== value);
  }
  if (typeof value === "number") return [value + 1, value - 1, 0, 1.5, 65536, 9007199254740992];
  if (typeof value === "boolean") return [!value, String(value)];
  if (value === null) return ["x", {}];
  if (Array.isArray(value)) return [[], value.length ? [...value, value[0]] : ["x"], [...value].reverse()];
  return [{}, null, "x"];
}

function* nodes(value, at = []) {
  if (value && typeof value === "object") {
    for (const [key, item] of Object.entries(value)) {
      const here = [...at, Array.isArray(value) ? Number(key) : key];
      yield [here, item, Array.isArray(value)];
      yield* nodes(item, here);
    }
  }
}

function replaced(root, at, update) {
  const copy = structuredClone(root);
  let parent = copy;
  for (const key of at.slice(0, -1)) parent = parent[key];
  update(parent, at[at.length - 1]);
  return copy;
}

function* documentMutants(document) {
  for (const [at, value, inArray] of nodes(document)) {
    if (!inArray) yield replaced(document, at, (parent, key) => { delete parent[key]; });
    for (const alternative of alternatives(value)) {
      yield replaced(document, at, (parent, key) => { parent[key] = alternative; });
    }
  }
}

function* textMutants(text) {
  yield `\ufeff${text}`;
  yield `${text} x`;
  yield text.replace(/"(\w+)":/, '"$1": 1, "$1":');
  yield text.replace(/"/, '"\\ud800');
  yield text.replace(/(\d+)/, "9007199254740993");
  yield text.replace(/(\d+)/, "1e400");
  yield text.replace(/(\d+)/, "1e16");
  yield text.replace(/(\d+)/, "01");
  yield `[${"[".repeat(40)}${"]".repeat(40)}]`;
}

// Cases stream to a JSON Lines file for the batch binary; expectations stay in
// memory.
const scratch = mkdtempSync(join(tmpdir(), "automation-authority-differential-"));
process.on("exit", () => rmSync(scratch, { recursive: true, force: true }));
const casesPath = join(scratch, "cases.jsonl");
const casesFile = openSync(casesPath, "w");
const expected = [];
const labels = [];
let undefinedReference = 0;
let label = "";
function add(operation, body, keys) {
  const answer = reference(operation, body, keys);
  if (answer.code?.startsWith("THROW:")) {
    undefinedReference += 1;
    return;
  }
  writeSync(casesFile, `${JSON.stringify({ operation, body, keys })}\n`);
  expected.push(answer);
  labels.push(label);
}

const vectors = manifest().filter((vector) => PORTED.has(vector.operation));
function manifest() {
  return read(join(PROFILE, "manifest.json")).vectors;
}
for (const vector of vectors) {
  label = vector.id;
  const body = read(join(PROFILE, "vectors", vector.file));
  const operation = vector.operation;
  const keysOriginal = mutateKeyring(original, body.keyring_mutation);
  const keysFresh = mutateKeyring(fresh, body.keyring_mutation);
  add(operation, body, keysOriginal);
  const base = resignBody(body);
  timestampPool = [...new Set(
    [...nodes(base)].map(([, item]) => item).filter((item) => typeof item === "string" && TIMESTAMP.test(item)),
  )];
  add(operation, base, keysFresh);
  if (operation === "strict_parse") {
    for (const text of textMutants(body.raw_json)) add(operation, { ...body, raw_json: text }, keysFresh);
    continue;
  }
  for (const role of ["request", "decision", "policy", "now"]) {
    if (base[role] === undefined) continue;
    const mutants = role === "now" ? alternatives(base.now).map((now) => now) : [...documentMutants(base[role])];
    for (const mutant of mutants) {
      add(operation, { ...base, [role]: mutant }, keysFresh);
      if (role === "request" || role === "decision") {
        const signed = resign(mutant, DOMAIN[role]);
        if (signed !== mutant) add(operation, { ...base, [role]: signed }, keysFresh);
        if (operation === "verify_decision" && role === "request") {
          // A decision the reference computes and signs for the mutated
          // request, so the evaluator itself is compared.
          try {
            const decision = V.signArtifact(
              V.evaluateAuthorization(signed, base.policy, {
                keyring: new Map(Object.entries(keysFresh)),
                unsigned: true,
              }),
              DOMAIN.decision,
              signers[authority],
            );
            add(operation, { ...base, request: signed, decision }, keysFresh);
          } catch {
            // The mutated request does not evaluate; the raw cases cover it.
          }
        }
      }
    }
  }
}
closeSync(casesFile);

const input = openSync(casesPath, "r");
const run = spawnSync(batch, [], { stdio: [input, "pipe", "inherit"], maxBuffer: 1 << 30 });
closeSync(input);
if (run.status !== 0) {
  console.error(`batch binary exited with ${run.status}`);
  process.exit(1);
}
const actual = run.stdout.toString().trimEnd().split("\n").map((line) => JSON.parse(line));
if (actual.length !== expected.length) {
  console.error(`batch binary answered ${actual.length} of ${expected.length} cases`);
  process.exit(1);
}
let mismatched = 0;
const codes = new Set();
for (let index = 0; index < expected.length; index += 1) {
  const want = expected[index];
  const got = actual[index];
  if (want.code) codes.add(want.code);
  if (want.code !== got.code || want.result !== got.result) {
    mismatched += 1;
    if (mismatched <= 15) {
      console.log(`MISMATCH case ${index} (from ${labels[index]})`);
      console.log(`  reference: ${JSON.stringify(want).slice(0, 300)}`);
      console.log(`  rust:      ${JSON.stringify(got).slice(0, 300)}`);
    }
  }
}
console.log(
  `Differential: ${expected.length} cases from ${vectors.length} vectors, ${mismatched} mismatched; ` +
    `${undefinedReference} cases skipped where the reference throws a non-profile error`,
);
console.log(`Codes exercised (${codes.size}): ${[...codes].sort().join(" ")}`);
process.exit(mismatched === 0 ? 0 : 1);
