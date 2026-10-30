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
// time) is mutated (deleted, replaced, or joined by an unknown member), raw
// and, for a signed artifact, re-signed under its own domain by the key it
// names, which may itself be mutated to another key of the keyring. Each
// body member is also replaced whole and removed. For
// decision verification the mutated request also gets a freshly computed,
// consistently signed decision; for dispatch, a mutated request or policy
// snapshot also gets one, with the consumption snapshot's adoption moved to
// match. Signature tampering is added explicitly. A digest leaves out the
// signature, so re-signing one artifact keeps every other artifact's binding
// to it intact.
//
// Every case crosses to the port as text. The batch binary reads frames: an
// envelope line, which is transport, then the serialized artifact's bytes,
// which the port parses with its production strict parser and nothing else,
// while the reference parses the same text with strictParseJson before
// running the operation. So both parsers see the same bytes, and the
// comparison covers them. Each re-signed
// vector body is also rewritten at the text level, at every depth: a key
// written twice, written \u-escaped, and written both ways (a duplicate the
// parser must detect after unescaping); an object's members reversed; an
// integer written with a fraction or an exponent, as negative zero, at and
// just beyond the safe-integer bound, with and without a fraction, and
// non-finite; a string fully \u-escaped, carrying a lone surrogate, and
// extended by the same characters as an escaped surrogate pair and literally.
// A rewrite that spells the same document must produce exactly the answer the
// plain text does, from both validators: that compares the normalized parse
// result through the whole operation. A rewrite that breaks the document must
// produce exactly the profile code it specifies. Each positive vector body is
// also parsed on its own as raw_json, with the rewrites the parser accepts, so
// the canonical text of the parsed documents is compared directly.
//
// Both validators must report the same first error code, or the same result:
// the full canonical decision for evaluate_request, the outcome for
// verify_decision, the final canonical replay state for request_adoption and
// decision_consumption, the canonical lifecycle state for lifecycle, the
// canonical dispatch result for verify_dispatch, the canonical parsed
// document for strict_parse, and "ok" otherwise.
//
// Dispatch bodies carry every artifact at once, so their mutants are many and
// large. A mutated signed artifact in a dispatch body is sent re-signed only,
// since raw it fails that artifact's own validation, which its own operation
// already compares. The positive dispatch vectors and the negative permit
// (no approval) ones are mutated in full, and each other negative one
// contributes a fixed, evenly spaced sample of its mutants. Every dispatch
// body contributes the same sample of its text rewrites, since the artifacts
// it composes are rewritten in full under their own operations.
//
// Where the reference throws something other than an AuthorityError (a
// TypeError on a null member, say), it defines no code. Those cases are not
// compared, but the port must still refuse them: a code the reference can
// raise, and no result.
//
// One documented divergence is asserted rather than compared. The reference
// parser cannot hold an object key named __proto__ (JavaScript assigns the
// prototype instead), while the port refuses the text as json_invalid. A
// mutant carrying an own __proto__ member therefore expects json_invalid from
// the port, and the reference is given the member in memory and counted.
//
// On a mismatch the failing cases are kept, as frames the batch binary reads,
// with their labels and both answers, and the path is printed.
//
// Usage: node scripts/automation-authority-differential.mjs <batch binary>

import { createHash, createPrivateKey, createPublicKey } from "node:crypto";
import { closeSync, mkdtempSync, openSync, readFileSync, readSync, rmSync, writeFileSync, writeSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import * as V from "../profiles/automation-authority/v1/validator.mjs";
import { compareAnswer } from "./lib/authority-differential-answer.mjs";
import { minimizeFaults, sampleMultifaults } from "./lib/authority-multifault.mjs";
import { parseBatchAnswers } from "./lib/authority-batch-output.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const PROFILE = join(ROOT, "profiles/automation-authority/v1");
const batch = process.argv[2];
if (!batch) {
  console.error("usage: automation-authority-differential.mjs <batch binary>");
  process.exit(2);
}
const DOMAIN = V.profileConstants.domains;
const parseBound = (text, name, min, max) => {
  if (!/^(?:0x[0-9a-f]+|[0-9]+)$/i.test(text)) throw new Error(`${name} must be an integer`);
  const value = Number(text);
  if (!Number.isSafeInteger(value) || value < min || value > max) throw new Error(`${name} must be ${min}..${max}`);
  return value;
};
const MULTI_SEED = parseBound(process.env.AUTHORITY_DIFFERENTIAL_SEED ?? "88", "seed", 0, 0xffffffff);
const MULTI_BUDGET = parseBound(process.env.AUTHORITY_DIFFERENTIAL_BUDGET ?? "24", "budget", 2, 256);
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

// Every code the reference can raise: the first argument of `fail`, the code
// argument of `exact` and `assertBinding`.
const referenceSource = readFileSync(join(PROFILE, "validator.mjs"), "utf8");
const REFERENCE_CODES = new Set(
  [
    /\bfail\(\s*"([a-z0-9_]+)"/g,
    /\bexact\(\s*[^,]+,\s*[^,]+,\s*"([a-z0-9_]+)"/g,
    /\bassertBinding\(\s*[^,]+,\s*[^,]+,\s*[^,]+,\s*"([a-z0-9_]+)"/g,
  ].flatMap((pattern) => [...referenceSource.matchAll(pattern)].map((match) => match[1])),
);
const CODE_DISPOSITIONS = {
  decision_request_mismatch: "shadowed by verified full decision semantic equality; direct private binding guard tests",
  decision_binding_mismatch: "shadowed by verified full decision semantic equality; direct private binding guard tests",
  decision_expired: "evaluated expiry equals request expiry, whose earlier guard wins; direct private decision-time guard tests",
  dispatch_runtime_downgrade: "shadowed by granted-subset validation and exact runtime set equality; direct private runtime guard tests",
  dispatch_runtime_capability_missing: "shadowed by approval evaluation and exact runtime set equality; direct private approval-runtime guard tests",
  runtime_capability_unknown: "request validation emits capability_unknown first; direct private vocabulary guard tests",
  integrity_role_unknown: "exercised by the required keyring companion, not this fixed-role corpus",
  integrity_domain_invalid: "public digest utility tested directly in Rust and Node; batch uses fixed domains",
  integrity_signer_required: "producer helper tested directly in Rust and Node for empty ids; missing signer/private material stays a Rust type boundary, outside manifest operations",
};
if (REFERENCE_CODES.size < 100) {
  console.error(`found only ${REFERENCE_CODES.size} reference codes; the extraction is broken`);
  process.exit(2);
}

// The build under test, so the run records which serde_json it compared.
const described = spawnSync(batch, ["--describe"], { encoding: "utf8" });
if (described.status !== 0) {
  console.error(`batch binary --describe exited with ${described.status}`);
  process.exit(1);
}
const build = JSON.parse(described.stdout);

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

function withoutKeyMaterial(mutation) {
  if (!mutation?.set) return mutation;
  const { public_key: _key, public_key_pem: _pem, ...set } = mutation.set;
  return { ...mutation, set };
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

// The reference, run as run-vectors.mjs runs it, on a parsed body.
function reference(operation, body, keys) {
  const keyring = new Map(Object.entries(keys));
  try {
    switch (operation) {
      case "strict_parse":
        return { code: null, result: canonical(V.strictParseJson(body.raw_json)) };
      case "validate_request":
        V.validateAuthorizationRequest(body.request, { keyring });
        return { code: null, result: "ok" };
      case "request_adoption": {
        let state = null;
        for (let index = 0; index < (body.repetitions ?? 1); index += 1) {
          state = V.adoptAuthorizationRequest(body.request, state, { keyring, now: body.now });
        }
        return { code: null, result: canonical(state) };
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
        return { code: null, result: canonical(state) };
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

// The reference on the body as text: the strict parser first, as the port.
function referenceText(operation, text, keys) {
  let body;
  try {
    body = V.strictParseJson(text);
  } catch (error) {
    if (error instanceof V.AuthorityError) return { code: error.code, result: null };
    return { code: `THROW:${error.name}`, result: null };
  }
  return reference(operation, body, keys);
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

// Unknown members, sorting first and last. A computed `__proto__` key is an
// own member, as a parsed document's would be.
const UNKNOWN_MEMBERS = ["AAA", "zz_unknown", "__proto__"];
const isPlainObject = (value) => value !== null && typeof value === "object" && !Array.isArray(value);

function hasOwnProto(value) {
  if (!value || typeof value !== "object") return false;
  if (!Array.isArray(value) && Object.hasOwn(value, "__proto__")) return true;
  return Object.values(value).some(hasOwnProto);
}

function* documentMutants(document) {
  if (isPlainObject(document)) {
    for (const member of UNKNOWN_MEMBERS) yield { ...structuredClone(document), [member]: 1 };
  }
  for (const [at, value, inArray] of nodes(document)) {
    if (!inArray) yield replaced(document, at, (parent, key) => { delete parent[key]; });
    for (const alternative of alternatives(value, at[at.length - 1])) {
      yield replaced(document, at, (parent, key) => { parent[key] = alternative; });
    }
    if (isPlainObject(value)) {
      for (const member of UNKNOWN_MEMBERS) {
        yield replaced(document, at, (parent, key) => { parent[key] = { ...structuredClone(value), [member]: 1 }; });
      }
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

// Rewrites of a strict_parse vector's own raw text.
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

// `value` as JSON.stringify writes it, except at `target`, where `mode`
// applies: `node` rewrites the value there, `key` rewrites the member there
// (given its key and its value's text), `reverse` writes the object there
// with its members in reverse order.
function emit(value, target, mode, at = []) {
  const depth = at.length;
  if (depth > target.length || at.some((key, index) => key !== target[index])) return JSON.stringify(value);
  const here = depth === target.length;
  if (here && mode.node) return mode.node(value);
  if (Array.isArray(value)) {
    return `[${value.map((item, index) => emit(item, target, mode, [...at, index])).join(",")}]`;
  }
  if (isPlainObject(value)) {
    let keys = Object.keys(value);
    if (here && mode.reverse) keys = keys.reverse();
    const members = keys.map((key) => {
      const text = emit(value[key], target, mode, [...at, key]);
      if (mode.key && depth + 1 === target.length && key === target[depth]) return mode.key(key, text);
      return `${JSON.stringify(key)}:${text}`;
    });
    return `{${members.join(",")}}`;
  }
  return JSON.stringify(value);
}

// A string with every UTF-16 code unit \u-escaped.
function escapedUnits(text) {
  let out = '"';
  for (let index = 0; index < text.length; index += 1) {
    out += `\\u${text.charCodeAt(index).toString(16).padStart(4, "0")}`;
  }
  return `${out}"`;
}

const pathText = (at) => at.map((key) => (typeof key === "number" ? `[${key}]` : `.${key}`)).join("");
const MAX_SAFE = "9007199254740991";
const UNSAFE = "9007199254740992";

// Text rewrites of `document`, at every node. `texts` spell one document each;
// a rewrite with `code` must be refused with exactly that code, one with
// `equivalent` spells the same document as `document` and must get its
// answer, and a `pair` spells the same document twice, which must get one
// answer. A rewrite that both parsers accept, like a number at the safe
// bound, is compared as any other case.
function* textRewrites(document) {
  for (const [at, value, inArray] of nodes(document)) {
    const where = pathText(at);
    if (!inArray) {
      yield {
        variant: `duplicate key ${where}`,
        code: "json_duplicate_key",
        texts: [emit(document, at, { key: (key, text) => `${JSON.stringify(key)}:${text},${JSON.stringify(key)}:${text}` })],
      };
      yield {
        variant: `escaped key ${where}`,
        equivalent: true,
        texts: [emit(document, at, { key: (key, text) => `${escapedUnits(key)}:${text}` })],
      };
      yield {
        variant: `escaped duplicate key ${where}`,
        code: "json_duplicate_key",
        texts: [emit(document, at, { key: (key, text) => `${JSON.stringify(key)}:${text},${escapedUnits(key)}:${text}` })],
      };
    }
    if (isPlainObject(value) && Object.keys(value).length > 1) {
      yield { variant: `reversed members ${where}`, equivalent: true, texts: [emit(document, at, { reverse: true })] };
    }
    if (typeof value === "number") {
      const literal = (text) => emit(document, at, { node: () => text });
      if (Number.isInteger(value) && Math.abs(value) < 1e21) {
        yield { variant: `integer with fraction ${where}`, equivalent: true, texts: [literal(`${value}.0`)] };
        yield { variant: `integer with exponent ${where}`, equivalent: true, texts: [literal(`${value}e0`)] };
        yield { variant: `integer with fraction and exponent ${where}`, equivalent: true, texts: [literal(`${value}.00E+0`)] };
      }
      if (value === 0) yield { variant: `negative zero ${where}`, equivalent: true, texts: [literal("-0")] };
      for (const [text, code] of [
        [MAX_SAFE, null],
        [`-${MAX_SAFE}`, null],
        [`${MAX_SAFE}.0`, null],
        [UNSAFE, "json_unsafe_integer"],
        [`-${UNSAFE}`, "json_unsafe_integer"],
        [`${UNSAFE}.0`, "json_unsafe_integer"],
        ["9.007199254740992e15", "json_unsafe_integer"],
        ["1e400", "json_non_ijson"],
        ["-1e400", "json_non_ijson"],
      ]) {
        yield { variant: `number ${text} ${where}`, code, texts: [literal(text)] };
      }
    }
    if (typeof value === "string") {
      const quoted = JSON.stringify(value);
      const literal = (text) => emit(document, at, { node: () => text });
      yield { variant: `escaped string ${where}`, equivalent: true, texts: [literal(escapedUnits(value))] };
      yield {
        variant: `lone surrogate ${where}`,
        code: "json_non_ijson",
        // A lone high surrogate at the end, or a lone low one at the start.
        texts: [at.length % 2 ? `${quoted.slice(0, -1)}\\ud800"` : `"\\udc00${quoted.slice(1)}`].map(literal),
      };
      yield {
        variant: `appended astral and latin characters ${where}`,
        pair: true,
        texts: [
          literal(JSON.stringify(`${value}\u{1f600}\u00e9`)),
          literal(`${quoted.slice(0, -1)}\\ud83d\\ude00\\u00e9"`),
        ],
      };
    }
  }
}

// Rewrites of the whole document's text.
function* documentRewrites(document) {
  const text = JSON.stringify(document);
  yield { variant: "BOM", code: "json_non_ijson", texts: [`\ufeff${text}`] };
  yield { variant: "trailing data", code: "json_invalid", texts: [`${text} x`] };
  yield { variant: "indented", equivalent: true, texts: [JSON.stringify(document, null, 2)] };
  yield { variant: "surrounding whitespace", equivalent: true, texts: [` \t\r\n${text}\n\r\t `] };
  if (isPlainObject(document)) {
    // Bounded deep and large documents: both parsers must get through them.
    const members = Object.keys(document);
    const first = members[0];
    yield {
      variant: `member ${first} 1000 levels deep`,
      texts: [emit(document, [first], { node: () => `${"[".repeat(1000)}${"]".repeat(1000)}` })],
    };
    yield {
      variant: `member ${first} 64 KiB string`,
      texts: [emit(document, [first], { node: () => JSON.stringify("x".repeat(65536)) })],
    };
  }
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
// For a text rewrite: its variant, the index its answer must equal, and the
// code it must be refused with.
const rewrites = [];
// Each case is a frame: an envelope line, `{"operation", "body_bytes",
// "keys"?}`, then the body text, then a newline. The keys go only when they
// change, and the body goes as bytes rather than as a JSON string, so the
// port parses each body exactly once, with its strict parser. Where each
// body starts in the file, its length, and its keys are kept, so a failing
// case can be written out whole: the file is too large to read back.
const bodyOffsets = [];
const bodyLengths = [];
const caseKeys = [];
const inputHash = createHash("sha256");
const multifaultCounts = { pairs: 0, triples: 0, raw: 0, resigned: 0 };
const multifaultCases = new Map();
let lastKeys = null;
let undefinedReference = 0;
let protoCases = 0;
let protoRefused = 0;
let label = "";
let fileBytes = 0;
let bodyBytes = 0;
function frame(operation, body, keys) {
  const head = Buffer.from(`${JSON.stringify({ operation, body_bytes: body.length, ...(keys ? { keys } : {}) })}\n`, "utf8");
  return Buffer.concat([head, body, Buffer.from("\n")]);
}
function record(operation, text, keys, answer, rewrite = null) {
  if (answer.code?.startsWith("THROW:")) undefinedReference += 1;
  const body = Buffer.from(text, "utf8");
  const bytes = frame(operation, body, keys === lastKeys ? null : keys);
  lastKeys = keys;
  bodyOffsets.push(fileBytes + bytes.length - body.length - 1);
  bodyLengths.push(body.length);
  caseKeys.push(keys);
  fileBytes += bytes.length;
  bodyBytes += body.length;
  writeSync(casesFile, bytes);
  inputHash.update(bytes);
  expected.push(answer);
  labels.push(label);
  operations.push(operation);
  rewrites.push(rewrite);
  return expected.length - 1;
}
function add(operation, body, keys) {
  const text = JSON.stringify(body);
  if (hasOwnProto(body)) {
    protoCases += 1;
    const inMemory = reference(operation, body, keys).code;
    if (inMemory !== null && !inMemory.startsWith("THROW:")) protoRefused += 1;
    record(operation, text, keys, { code: "json_invalid", result: null }, { variant: "own __proto__ member" });
    return { code: "json_invalid", result: null };
  }
  const answer = referenceText(operation, text, keys);
  record(operation, text, keys, answer);
  return answer;
}
// The rewrites of `document`, as `wrap` embeds each text in a body; one in
// `sample` of them is kept. `baseIndex` is the plain text's case.
function addRewrites(operation, document, keys, wrap, baseIndex, sample, { accepted = false } = {}) {
  let index = 0;
  for (const rewrite of [...documentRewrites(document), ...textRewrites(document)]) {
    // Inside raw_json, a rewrite the parser refuses gets the code the body's
    // own rewrite already compared; only the ones it accepts add anything.
    if (accepted && rewrite.code) continue;
    index += 1;
    if (index % sample !== 0) continue;
    let first = null;
    for (const text of rewrite.texts) {
      const wrapped = wrap(text);
      const answer = referenceText(operation, wrapped, keys);
      const equals = rewrite.equivalent ? baseIndex : rewrite.pair ? first : null;
      const here = record(operation, wrapped, keys, answer, { variant: rewrite.variant, equals, code: rewrite.code ?? null });
      first ??= here;
    }
  }
}

function poolTimestamps(base) {
  timestampPool = [...new Set(
    [...nodes(base)].map(([, item]) => item).filter((item) => typeof item === "string" && TIMESTAMP.test(item)),
  )];
}

const started = performance.now();
const vectors = read(join(PROFILE, "manifest.json")).vectors;
const repairedPermits = new Map();
for (const vector of vectors) {
  label = vector.id;
  const body = read(join(PROFILE, "vectors", vector.file));
  const operation = vector.operation;
  const keysOriginal = mutateKeyring(original, body.keyring_mutation);
  // A vector may bring its own key for artifacts it signs itself. The fresh
  // keyring keeps its own key material, because every artifact here is
  // re-signed with it.
  const keysFresh = mutateKeyring(fresh, withoutKeyMaterial(body.keyring_mutation));
  add(operation, body, keysOriginal);
  const base = resignBody(body);
  poolTimestamps(base);
  const baseAnswer = add(operation, base, keysFresh);
  const baseIndex = expected.length - 1;
  if ((operation === "request_adoption" && baseAnswer.code === "request_replayed") ||
      (operation === "decision_consumption" && baseAnswer.code === "decision_replayed")) {
    label = `${vector.id} (single-use state control)`;
    const once = add(operation, { ...base, repetitions: 1 }, keysFresh);
    if (once.code !== null || once.result === "null") throw new Error(`${vector.id}: replay fixture lacks a successful first state`);
    label = vector.id;
  }
  // Every vector gets its own stable sub-seed. This is a bounded sample of
  // simultaneous semantic field changes, not an exhaustive pairwise proof.
  const seed = createHash("sha256").update(`${MULTI_SEED}:${vector.id}`).digest().readUInt32BE(0);
  for (const mutant of sampleMultifaults(base, { seed, budget: MULTI_BUDGET, structural: true, alternativesFor: alternatives })) {
    const variant = `multifault seed=${seed} paths=${JSON.stringify(mutant.paths)}`;
    const text = JSON.stringify(mutant.body);
    const rawIndex = record(operation, text, keysFresh, referenceText(operation, text, keysFresh), { variant: `${variant} raw` });
    multifaultCases.set(rawIndex, { base, paths: mutant.paths, resigned: false });
    multifaultCounts[mutant.arity === 2 ? "pairs" : "triples"]++;
    multifaultCounts.raw++;
    const resigned = JSON.stringify(resignBody(mutant.body));
    if (resigned !== text) {
      const signedIndex = record(operation, resigned, keysFresh, referenceText(operation, resigned, keysFresh), { variant: `${variant} resigned` });
      multifaultCases.set(signedIndex, { base, paths: mutant.paths, resigned: true });
      multifaultCounts.resigned++;
    }
  }
  // A permit dispatch, with no approval, is the launch path R0 and R1 runs
  // take, and no positive vector covers it, so its negatives go in full, and
  // the first mutant of each that both validators accept is kept to be
  // mutated in full itself below.
  const permit = operation === "verify_dispatch" && base.decision?.outcome === "permit";
  const sample = operation === "verify_dispatch" && vector.kind === "negative" && !permit ? DISPATCH_SAMPLE : 1;
  // The body's text, rewritten at every depth. A dispatch body composes the
  // artifacts the other operations' bodies carry, and those are rewritten in
  // full, so every dispatch body contributes a sample.
  const textSample = operation === "verify_dispatch" ? DISPATCH_SAMPLE : 1;
  addRewrites(operation, base, keysFresh, (text) => text, baseIndex, textSample);
  if (operation === "strict_parse") {
    for (const text of textMutants(body.raw_json)) add(operation, { ...body, raw_json: text }, keysFresh);
    continue;
  }
  if (vector.kind === "positive") {
    // The body parsed on its own: the canonical parsed document is the
    // result, so the equivalent rewrites compare the normalized parse.
    label = `${vector.id} (raw_json)`;
    const raw = add("strict_parse", { raw_json: JSON.stringify(base) }, keysFresh);
    if (raw.code !== null) throw new Error(`${vector.id}: the reference refuses its own body text`);
    addRewrites("strict_parse", base, keysFresh, (text) => JSON.stringify({ raw_json: text }), expected.length - 1, textSample, {
      accepted: true,
    });
    label = vector.id;
  }
  let repaired = null;
  mutateAll(operation, base, keysFresh, sample, permit && vector.kind === "negative"
    ? (accepted) => { if (accepted.decision?.outcome === "permit") repaired ??= accepted; }
    : null);
  if (repaired && !repairedPermits.has(JSON.stringify(repaired))) {
    repairedPermits.set(JSON.stringify(repaired), { id: vector.id, body: repaired, keys: keysFresh });
  }
}

// Every accepted permit dispatch above is a mutant that happens to repair a
// negative vector. Mutating each first repair in full compares the checks
// around an authorized permit launch from an accepted starting point, as the
// positive vectors do for the approval path.
for (const { id, body, keys } of repairedPermits.values()) {
  label = `${id} (repaired)`;
  poolTimestamps(body);
  mutateAll("verify_dispatch", body, keys, 1, null);
}

// Every single-member mutant of `base`, raw and re-signed, keeping one in
// `sample`. `onAccepted` sees each body the reference accepts.
function mutateAll(operation, base, keysFresh, sample, onAccepted) {
  const dispatch = operation === "verify_dispatch";
  const check = (body) => {
    const answer = add(operation, body, keysFresh);
    if (onAccepted && answer.code === null) onAccepted(body);
  };
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
    if (!(dispatch && resigned)) check(mutant);
    if (dispatch && (member === "request" || member === "snapshot")) {
      // The decision and adoption follow the mutated request or policy, so
      // the checks after decision verification are compared too.
      const consistent = rebound(signed, base.request, keysFresh);
      if (consistent && JSON.stringify(consistent.decision) !== JSON.stringify(signed.decision)) {
        check(consistent);
      }
    }
    if (!resigned) continue;
    check(signed);
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
        check({ ...signed, decision });
      } catch {
        // The mutated request does not evaluate; the raw cases cover it.
      }
    }
  }
}
closeSync(casesFile);
const built = performance.now();

const input = openSync(casesPath, "r");
const run = spawnSync(batch, [], { stdio: [input, "pipe", "inherit"], maxBuffer: 1 << 30 });
closeSync(input);
const answered = performance.now();
function retainBatchFailure(error) {
  const kept = mkdtempSync(join(tmpdir(), "automation-authority-differential-failures-"));
  const index = Math.max(0, Math.min(expected.length - 1, error.caseIndex ?? 0));
  if (expected.length) {
    const descriptor = openSync(casesPath, "r");
    const body = Buffer.alloc(bodyLengths[index]);
    readSync(descriptor, body, 0, body.length, bodyOffsets[index]);
    closeSync(descriptor);
    writeFileSync(join(kept, "cases.stream"), frame(operations[index], body, caseKeys[index]));
  }
  writeFileSync(join(kept, "report.json"), JSON.stringify({
    kind: "batch transport/execution failure", error: error.message, status: run.status, signal: run.signal,
    case: index, label: labels[index], operation: operations[index],
    inference: "first missing/invalid answer; a process failing after all answers is attributed to the last case",
    seed: MULTI_SEED, budgetPerVector: MULTI_BUDGET, build, batch, node: process.version,
    v8: process.versions.v8, openssl: process.versions.openssl, platform: `${process.platform}/${process.arch}`,
    inputSha256: inputHash.copy().digest("hex"), binarySha256: createHash("sha256").update(readFileSync(batch)).digest("hex"),
  }, null, 2) + "\n");
  console.error(`Batch failure retained in ${kept}; reproduce: ${batch} < ${join(kept, "cases.stream")}`);
}
let actual;
try {
  actual = parseBatchAnswers(run.stdout?.toString() ?? "", expected.length);
} catch (error) {
  retainBatchFailure(error);
  console.error(error.message);
  process.exit(1);
}
if (run.status !== 0) {
  retainBatchFailure(Object.assign(new Error(`batch binary exited with ${run.status}`), { caseIndex: expected.length - 1 }));
  console.error(`batch binary exited with ${run.status}`);
  process.exit(1);
}
const failures = [];
const failOpen = [];
const codes = new Set();
const perOperation = {};
const byOperationOutcome = {};
const replayResults = {};
let rewritten = 0;
let equivalent = 0;
let refusing = 0;
for (let index = 0; index < expected.length; index += 1) {
  const want = expected[index];
  const got = actual[index];
  const rewrite = rewrites[index];
  perOperation[operations[index]] = (perOperation[operations[index]] ?? 0) + 1;
  const outcome = want.code === null ? "success" : want.code.startsWith("THROW:") ? "unspecified" : "refusal";
  const counts = byOperationOutcome[operations[index]] ??= { success: 0, refusal: 0, unspecified: 0 };
  counts[outcome]++;
  if (outcome === "success" && ["request_adoption", "decision_consumption", "lifecycle"].includes(operations[index])) {
    const replay = replayResults[operations[index]] ??= { compared: 0, nonNullState: 0 };
    replay.compared++;
    if (want.result !== "null") replay.nonNullState++;
  }
  if (rewrite?.variant && rewrite.code !== undefined) rewritten += 1;
  if (want.code && !want.code.startsWith("THROW:")) codes.add(want.code);
  if (rewrite?.equals !== null && rewrite?.equals !== undefined) {
    equivalent += 1;
  }
  if (rewrite?.code) refusing += 1;
  // Normative rewrite assertions run even if the reference throws a non-profile
  // exception. Only unconstrained outcomes may use generic fail-closed fallback.
  const comparison = compareAnswer(want, got, rewrite, expected[rewrite?.equals], actual[rewrite?.equals], REFERENCE_CODES);
  for (const reason of comparison.failures) failures.push({ index, reason });
  if (comparison.failOpen) failOpen.push(index);
}
const mismatched = new Set(failures.map(({ index }) => index)).size;
for (const { index, reason } of failures.slice(0, 15)) {
  const rewrite = rewrites[index];
  console.log(`MISMATCH case ${index} (from ${labels[index]}, ${operations[index]}${rewrite ? `, ${rewrite.variant}` : ""}): ${reason}`);
  console.log(`  reference: ${JSON.stringify(expected[index]).slice(0, 300)}`);
  console.log(`  rust:      ${JSON.stringify(actual[index]).slice(0, 300)}`);
}
for (const index of failOpen.slice(0, 15)) {
  console.log(
    `FAIL-OPEN case ${index} (from ${labels[index]}): the reference throws ${expected[index].code}, ` +
      `the port answers ${JSON.stringify(actual[index]).slice(0, 200)}`,
  );
}
if (failures.length > 0 || failOpen.length > 0) {
  // Keep the failing cases: the exact body bytes the batch binary read, each
  // in a frame of its own with its keys, with their labels and both answers.
  // The keys are the synthetic test keyring.
  const kept = mkdtempSync(join(tmpdir(), "automation-authority-differential-failures-"));
  const keep = new Set([...failures.map(({ index }) => index), ...failOpen]);
  const cases = openSync(casesPath, "r");
  const casesOut = [];
  const report = [];
  let reduced = 0;
  for (const index of [...keep].sort((left, right) => left - right)) {
    const body = Buffer.alloc(bodyLengths[index]);
    readSync(cases, body, 0, bodyLengths[index], bodyOffsets[index]);
    casesOut.push(frame(operations[index], body, caseKeys[index]));
    let minimal = null;
    const sampled = multifaultCases.get(index);
    if (sampled && reduced < 8) {
      reduced++;
      try {
        const prepare = (candidate) => sampled.resigned ? resignBody(candidate) : candidate;
        const stillFails = (candidate) => {
          const text = JSON.stringify(prepare(candidate));
          const want = referenceText(operations[index], text, caseKeys[index]);
          const run = spawnSync(batch, [], { input: frame(operations[index], Buffer.from(text), caseKeys[index]), encoding: "utf8", maxBuffer: 1 << 20 });
          if (run.status !== 0) throw new Error(`counterexample batch exited ${run.status}`);
          const got = parseBatchAnswers(run.stdout, 1)[0];
          const checked = compareAnswer(want, got, null, null, null, REFERENCE_CODES);
          return checked.failOpen || checked.failures.length > 0;
        };
        // Only JSON.stringify-generated multifaults enter this reducer. Using
        // JSON.parse here retains their intentionally unsafe numeric values;
        // each candidate still crosses both real strict parsers below.
        const result = minimizeFaults(sampled.base, JSON.parse(body.toString("utf8")), sampled.paths, stillFails);
        if (result) {
          const minimalPath = join(kept, `minimal-${index}.stream`);
          writeFileSync(minimalPath, frame(operations[index], Buffer.from(JSON.stringify(prepare(result.body))), caseKeys[index]));
          minimal = { paths: result.paths, file: minimalPath, scope: "minimum failing subset of sampled fields; full signed artifact retained" };
        }
      } catch (error) {
        // A diagnostic failure never converts the original mismatch to success.
        minimal = { error: error.message };
      }
    }
    report.push({
      case: index,
      label: labels[index],
      operation: operations[index],
      rewrite: rewrites[index],
      reasons: failures.filter((failure) => failure.index === index).map(({ reason }) => reason),
      reference: expected[index],
      rust: actual[index],
      minimal,
    });
  }
  closeSync(cases);
  writeFileSync(join(kept, "cases.stream"), Buffer.concat(casesOut));
  writeFileSync(join(kept, "report.json"), `${JSON.stringify({ build, batch, node: process.version, v8: process.versions.v8, openssl: process.versions.openssl, platform: `${process.platform}/${process.arch}`, seed: MULTI_SEED, budgetPerVector: MULTI_BUDGET, inputSha256: inputHash.copy().digest("hex"), binarySha256: createHash("sha256").update(readFileSync(batch)).digest("hex"), failures: report }, null, 2)}\n`);
  console.log(`Failing cases kept in ${kept}: cases.stream (one frame per case, in report order) and report.json`);
  console.log(`Reproduce: ${batch} < ${join(kept, "cases.stream")}`);
}
console.log(
  `Differential: ${expected.length - undefinedReference} cases compared from ${vectors.length} vectors, ` +
    `${mismatched} mismatched; ${undefinedReference} more where the reference throws a non-profile error, ` +
    `${failOpen.length} of them not refused by the port`,
);
console.log(
  `Text rewrites: ${rewritten} (${equivalent} spelling the same document, ${refusing} refused by a specified code); ` +
    `${protoCases} own __proto__ members refused by the port's parser as json_invalid, ` +
    `${protoRefused} of them refused by the reference in memory; ${(bodyBytes / 1048576).toFixed(1)} MiB of body text parsed by both`,
);
console.log(
  `Cases by operation: ${Object.entries(perOperation)
    .sort(([left], [right]) => (left < right ? -1 : 1))
    .map(([operation, count]) => `${operation} ${count}`)
    .join(", ")}`,
);
console.log(`Codes exercised (${codes.size}): ${[...codes].sort().join(" ")}`);
console.log(`Code accounting: ${JSON.stringify([...REFERENCE_CODES].filter((code) => !codes.has(code)).sort().map((code) => ({ code, disposition: CODE_DISPOSITIONS[code] ?? "not observed in this bounded corpus; no reachability claim" })))}`);
console.log(`Build: serde_json arbitrary_precision ${build.serde_json_arbitrary_precision ? "on" : "off"} (${batch})`);
const evidence = {
  node: process.version, v8: process.versions.v8, openssl: process.versions.openssl,
  platform: `${process.platform}/${process.arch}`, build,
  seed: MULTI_SEED, budgetPerVector: MULTI_BUDGET, multifaults: multifaultCounts,
  cases: expected.length, vectors: vectors.length, mismatched, failOpen: failOpen.length,
  textRewrites: { total: rewritten, equivalent, refusing },
  inputSha256: inputHash.digest("hex"),
  binarySha256: createHash("sha256").update(readFileSync(batch)).digest("hex"),
  referenceSha256: createHash("sha256").update(referenceSource).digest("hex"),
  byOperation: perOperation, byOperationOutcome, replayResults,
  success: expected.filter((answer) => answer.code === null).length,
  refusal: expected.filter((answer) => answer.code && !answer.code.startsWith("THROW:")).length,
  unspecified: undefinedReference,
};
console.log(`Evidence: ${JSON.stringify(evidence)}`);
console.log(
  `Timing: cases built and answered by the reference in ${((built - started) / 1000).toFixed(0)}s, ` +
    `answered by the port in ${((answered - built) / 1000).toFixed(0)}s`,
);
process.exit(failures.length === 0 && failOpen.length === 0 ? 0 : 1);
