#!/usr/bin/env node
// Differential test of the Rust automation-authority port
// (coven-threads-core::automation_authority) against the reference
// profiles/automation-authority/v1/validator.mjs, over every operation the
// conformance manifest names.
//
// Each vector runs as written, then re-signed with fresh keys of the same
// roles. The fresh keys come from fixed seeds and Ed25519 signing is
// deterministic, so every run builds the same cases and a mismatch
// reproduces. Then every member of each top-level body member (the request,
// decision, policy, approval, lifecycle events, consumption snapshot,
// dispatch snapshot, proposal, evidence-read token, evidence and trusted
// time) is mutated, raw and, for a signed artifact, re-signed under its own
// domain by the key it names, which may itself be mutated to another key of
// the keyring. Each body member is also replaced whole and removed. For
// decision verification the mutated request also gets a freshly computed,
// consistently signed decision; for dispatch, a mutated request or policy
// snapshot also gets one, with the consumption snapshot's adoption moved to
// match. Signature tampering is added explicitly. A digest leaves out the
// signature, so re-signing one artifact keeps every other artifact's binding
// to it intact.
//
// Both validators must report the same first error code, or the same result:
// the full canonical decision for evaluate_request, the outcome for
// verify_decision, the canonical lifecycle state for lifecycle, the canonical
// dispatch result for verify_dispatch, and "ok" otherwise.
//
// Dispatch bodies carry every artifact at once, so their mutants are many and
// large. A mutated signed artifact in a dispatch body is sent re-signed only,
// since raw it fails that artifact's own validation, which its own operation
// already compares. The positive dispatch vectors are mutated in full, and
// each negative one contributes a fixed, evenly spaced sample of its mutants.
//
// Where the reference throws something other than an AuthorityError (a
// TypeError on a null member, say), it defines no code. Those cases are not
// compared, but the port must still refuse them with some error code.
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
const DOMAIN = V.profileConstants.domains;
// The domain each signed body member is signed under; the rest are unsigned.
const SIGNED = {
  request: DOMAIN.request,
  approval_authorization_request: DOMAIN.request,
  decision: DOMAIN.decision,
  approval_authorization_decision: DOMAIN.decision,
  approval: DOMAIN.approval,
  events: DOMAIN.event,
  consumption_snapshot: DOMAIN.consumption,
  proposal: DOMAIN.proposal,
  read: DOMAIN.evidenceRead,
};
// Body members that steer the runner, or that it does not read.
const FIXED = new Set(["keyring_mutation", "repetitions", "raw_json", "lifecycle_summary"]);
// One in this many mutants of a negative dispatch vector is kept.
const DISPATCH_SAMPLE = 9;
const read = (path) => V.strictParseJson(readFileSync(path, "utf8"));

// The reference's canonical form, to compare whole results.
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

// A body member re-signed under its domain: each event of a lifecycle chain,
// or the artifact itself.
function resignMember(member, value) {
  const domain = SIGNED[member];
  if (!domain) return value;
  if (member === "events") return Array.isArray(value) ? value.map((event) => resign(event, domain)) : value;
  return resign(value, domain);
}

function resignBody(body) {
  const copy = structuredClone(body);
  for (const member of Object.keys(copy)) copy[member] = resignMember(member, copy[member]);
  return copy;
}

// A dispatch body made consistent again around its request and policy
// snapshot: the decision recomputed by the reference and signed by the
// authority, and the consumption snapshot's adoption of `previous` moved to
// the request as it now stands, re-signed. Approvals and lifecycle events
// are left as they are. `null` when the request does not evaluate.
function rebound(body, previous, keys) {
  try {
    const decision = V.signArtifact(
      V.evaluateAuthorization(body.request, body.snapshot.policy_snapshot, {
        keyring: new Map(Object.entries(keys)),
        unsigned: true,
      }),
      DOMAIN.decision,
      signers[authority],
    );
    const before = `sha256:${V.canonicalDigest(previous, DOMAIN.request)}`;
    const consumption = structuredClone(body.consumption_snapshot);
    for (const adoption of consumption.request_adoptions) {
      if (adoption.request_digest !== before) continue;
      adoption.request_digest = `sha256:${V.canonicalDigest(body.request, DOMAIN.request)}`;
      adoption.nonce = body.request.replay.nonce;
      adoption.adoption_key = body.request.replay.adoption_key;
    }
    return { ...body, decision, consumption_snapshot: resign(consumption, DOMAIN.consumption) };
  } catch {
    return null;
  }
}

// `lifecycleState` in run-vectors.mjs.
function lifecycleState(body, keyring) {
  let state = null;
  for (const event of body.events) {
    state = V.applyLifecycleEvent(state, event, { approval: body.approval, keyring });
  }
  return state;
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
      case "validate_approval":
        V.validateApproval(body.approval, { keyring, now: body.now });
        return { code: null, result: "ok" };
      case "validate_consumption_snapshot":
        V.validateConsumptionSnapshot(body.consumption_snapshot, { keyring, now: body.now });
        return { code: null, result: "ok" };
      case "lifecycle": {
        const state = lifecycleState(body, keyring);
        if (body.replay_last) {
          V.applyLifecycleEvent(state, body.events.at(-1), { approval: body.approval, keyring });
        }
        return { code: null, result: canonical(state) };
      }
      case "verify_dispatch":
        return {
          code: null,
          result: canonical(
            V.verifyDispatch(
              {
                request: body.request,
                decision: body.decision,
                approval: body.approval ?? null,
                approval_authorization_request: body.approval_authorization_request ?? null,
                approval_authorization_decision: body.approval_authorization_decision ?? null,
                lifecycle_events: body.events ?? [],
                consumption_snapshot: body.consumption_snapshot,
                snapshot: body.snapshot,
              },
              { keyring },
            ),
          ),
        };
      case "validate_proposal":
        V.validateProposal(body.proposal, { keyring });
        return { code: null, result: "ok" };
      case "evidence_read":
        V.authorizeEvidenceRead(body.read, body.evidence, { keyring, now: body.now });
        return { code: null, result: "ok" };
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
  ["single_use", "recurring"],
  ["required", "requested", "approved", "rejected", "expired", "revoked", "consumed"],
  ["request", "approve", "reject", "expire", "revoke", "consume"],
  ["not_applicable", "not_started", "queued", "dispatching", "running", "completed"],
  ["not_applicable", "launch_authorized", "cancel_before_launch", "request_cooperative_cancel",
    "external_effects_not_rolled_back", "no_launch_rejected", "no_launch_expired"],
  ["rejected_no_launch", "expired_no_launch"],
  ["threads_authority", "principal", "protected_owner", "auditor"],
];
const TIMESTAMP = /^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?Z$/;

// Every timestamp in the case being mutated, so a timestamp can also equal
// another: the comparisons' `<`, `<=` and `>=` boundaries sit there.
let timestampPool = [];

// Alternatives for `value`, held under `key`.
function alternatives(value, key) {
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
    // Another key of the keyring: re-signed under it, the artifact verifies
    // and reaches the role and principal checks.
    if (key === "key_id" || key === "key_ref") out.push(...Object.keys(fresh));
    if (key === "occurrence_prefix" || key === "occurrence_id") {
      // A recurring approval's prefix must be at least 8 long, without `*`,
      // and lead the occurrence.
      out.push(value.slice(0, 7), value.slice(0, 8), `${value}*`, `${value}x`, value.slice(1));
    }
    return [...new Set(out)].filter((item) => item !== value);
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
    for (const alternative of alternatives(value, at[at.length - 1])) {
      yield replaced(document, at, (parent, key) => { parent[key] = alternative; });
    }
  }
}

// Each body member replaced whole, removed, and mutated member by member.
function* bodyMutants(base) {
  for (const member of Object.keys(base)) {
    if (FIXED.has(member)) continue;
    const { [member]: _, ...without } = base;
    yield [member, without];
    for (const alternative of alternatives(base[member], member)) {
      yield [member, { ...base, [member]: alternative }];
    }
    if (base[member] && typeof base[member] === "object") {
      for (const mutant of documentMutants(base[member])) yield [member, { ...base, [member]: mutant }];
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
const operations = [];
let undefinedReference = 0;
let label = "";
function add(operation, body, keys) {
  const answer = reference(operation, body, keys);
  if (answer.code?.startsWith("THROW:")) undefinedReference += 1;
  writeSync(casesFile, `${JSON.stringify({ operation, body, keys })}\n`);
  expected.push(answer);
  labels.push(label);
  operations.push(operation);
}

const vectors = read(join(PROFILE, "manifest.json")).vectors;
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
  const dispatch = operation === "verify_dispatch";
  const sample = dispatch && vector.kind === "negative" ? DISPATCH_SAMPLE : 1;
  let index = 0;
  for (const [member, mutant] of bodyMutants(base)) {
    index += 1;
    if (index % sample !== 0) continue;
    const signed = member in mutant && SIGNED[member]
      ? { ...mutant, [member]: resignMember(member, mutant[member]) }
      : mutant;
    const resigned = JSON.stringify(signed[member]) !== JSON.stringify(mutant[member]);
    // A raw mutant of a signed dispatch artifact fails that artifact's own
    // validation, which its own operation already compares.
    if (!(dispatch && resigned)) add(operation, mutant, keysFresh);
    if (dispatch && (member === "request" || member === "snapshot")) {
      // The decision and adoption follow the mutated request or policy, so
      // the checks after decision verification are compared too.
      const consistent = rebound(signed, base.request, keysFresh);
      if (consistent && JSON.stringify(consistent.decision) !== JSON.stringify(signed.decision)) {
        add(operation, consistent, keysFresh);
      }
    }
    if (!resigned) continue;
    add(operation, signed, keysFresh);
    if (operation === "verify_decision" && member === "request") {
      // A decision the reference computes and signs for the mutated
      // request, so the evaluator itself is compared.
      try {
        const decision = V.signArtifact(
          V.evaluateAuthorization(signed.request, base.policy, {
            keyring: new Map(Object.entries(keysFresh)),
            unsigned: true,
          }),
          DOMAIN.decision,
          signers[authority],
        );
        add(operation, { ...signed, decision }, keysFresh);
      } catch {
        // The mutated request does not evaluate; the raw cases cover it.
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
const failOpen = [];
const codes = new Set();
const perOperation = {};
for (let index = 0; index < expected.length; index += 1) {
  const want = expected[index];
  const got = actual[index];
  perOperation[operations[index]] = (perOperation[operations[index]] ?? 0) + 1;
  if (want.code?.startsWith("THROW:")) {
    // No reference answer to match; the port must still refuse.
    if (got.code === null) failOpen.push(index);
    continue;
  }
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
for (const index of failOpen.slice(0, 15)) {
  console.log(`FAIL-OPEN case ${index} (from ${labels[index]}): the reference throws ${expected[index].code}, the port accepts`);
}
console.log(
  `Differential: ${expected.length - undefinedReference} cases compared from ${vectors.length} vectors, ` +
    `${mismatched} mismatched; ${undefinedReference} more where the reference throws a non-profile error, ` +
    `${failOpen.length} of them accepted by the port`,
);
console.log(
  `Cases by operation: ${Object.entries(perOperation)
    .sort(([left], [right]) => (left < right ? -1 : 1))
    .map(([operation, count]) => `${operation} ${count}`)
    .join(", ")}`,
);
console.log(`Codes exercised (${codes.size}): ${[...codes].sort().join(" ")}`);
process.exit(mismatched === 0 && failOpen.length === 0 ? 0 : 1);
