#!/usr/bin/env node

import { readdirSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import {
  AuthorityError,
  adoptAuthorizationRequest,
  applyLifecycleEvent,
  authorizeEvidenceRead,
  consumeDecision,
  strictParseJson,
  validateApproval,
  validateAuthorizationRequest,
  validateProposal,
  verifyDecisionBundle,
  verifyDispatch,
} from "./validator.mjs";

const ROOT = dirname(fileURLToPath(import.meta.url));

function readJson(path) {
  return strictParseJson(readFileSync(path, "utf8"));
}

function loadKeyring() {
  const document = readJson(resolve(ROOT, "keyring.json"));
  if (document.schema_version !== "opencoven.automation-authority-test-keyring/v1") {
    throw new Error("unknown test keyring version");
  }
  return new Map(Object.entries(document.keys));
}

function validateManifest(manifest) {
  const fields = ["schema_version", "profile_version", "vector_root", "categories", "vectors"];
  if (
    !manifest ||
    typeof manifest !== "object" ||
    Array.isArray(manifest) ||
    Object.keys(manifest).some((key) => !fields.includes(key)) ||
    fields.some((key) => !Object.hasOwn(manifest, key))
  ) {
    throw new Error("manifest is not a closed v1 object");
  }
  if (
    manifest.schema_version !== "opencoven.automation-authority-conformance-manifest/v1" ||
    manifest.profile_version !== "1.0.0" ||
    manifest.vector_root !== "vectors"
  ) {
    throw new Error("manifest version mismatch");
  }
  const requiredCategories = Array.from({ length: 18 }, (_, index) => index + 1);
  if (JSON.stringify(manifest.categories) !== JSON.stringify(requiredCategories)) {
    throw new Error("manifest must enumerate categories 1 through 18 exactly");
  }
  const ids = new Set();
  const files = new Set();
  for (const vector of manifest.vectors) {
    const vectorFields = ["id", "category", "kind", "file", "operation", "expected"];
    if (
      Object.keys(vector).some((key) => !vectorFields.includes(key)) ||
      vectorFields.some((key) => !Object.hasOwn(vector, key))
    ) {
      throw new Error(`manifest vector ${vector.id ?? "<unknown>"} is not closed`);
    }
    if (ids.has(vector.id) || files.has(vector.file)) {
      throw new Error(`manifest vector id/file is duplicated: ${vector.id}`);
    }
    ids.add(vector.id);
    files.add(vector.file);
    if (!requiredCategories.includes(vector.category)) {
      throw new Error(`manifest vector ${vector.id} has invalid category`);
    }
    if (!["positive", "negative"].includes(vector.kind)) {
      throw new Error(`manifest vector ${vector.id} has invalid kind`);
    }
    if (!/^[a-z0-9][a-z0-9-]+\.json$/.test(vector.file)) {
      throw new Error(`manifest vector ${vector.id} has unsafe file path`);
    }
    const expectedFields = ["ok", "error_code", "outcome"];
    if (
      Object.keys(vector.expected).some((key) => !expectedFields.includes(key)) ||
      expectedFields.some((key) => !Object.hasOwn(vector.expected, key))
    ) {
      throw new Error(`manifest vector ${vector.id} expected result is not closed`);
    }
    if (vector.kind === "positive" && !vector.expected.ok) {
      throw new Error(`positive vector ${vector.id} must expect success`);
    }
    if (vector.kind === "negative" && (vector.expected.ok || !vector.expected.error_code)) {
      throw new Error(`negative vector ${vector.id} must name its semantic error code`);
    }
  }
  const represented = new Set(manifest.vectors.map((vector) => vector.category));
  for (const category of requiredCategories) {
    if (!represented.has(category)) throw new Error(`category ${category} has no vector`);
  }
  const actualFiles = readdirSync(resolve(ROOT, manifest.vector_root))
    .filter((name) => name.endsWith(".json"))
    .sort();
  const declaredFiles = [...files].sort();
  if (JSON.stringify(actualFiles) !== JSON.stringify(declaredFiles)) {
    throw new Error("manifest file list does not exactly match vectors directory");
  }
}

function lifecycleState(body, keyring) {
  let state = null;
  for (const event of body.events) {
    state = applyLifecycleEvent(state, event, { approval: body.approval, keyring });
  }
  return state;
}

function execute(vector, body, keyring) {
  switch (vector.operation) {
    case "strict_parse":
      strictParseJson(body.raw_json);
      return null;
    case "validate_request":
      validateAuthorizationRequest(body.request, { keyring });
      return null;
    case "request_adoption": {
      let state = null;
      for (let index = 0; index < body.repetitions; index += 1) {
        state = adoptAuthorizationRequest(body.request, state, {
          keyring,
          now: body.now,
        });
      }
      return null;
    }
    case "validate_approval":
      validateApproval(body.approval, { keyring, now: body.now });
      return null;
    case "verify_decision":
      return verifyDecisionBundle(body.request, body.decision, body.policy, { keyring });
    case "decision_consumption": {
      let state = null;
      for (let index = 0; index < body.repetitions; index += 1) {
        state = consumeDecision(body.decision, state, { keyring });
      }
      return null;
    }
    case "verify_dispatch": {
      const state = body.events ? lifecycleState(body, keyring) : null;
      return verifyDispatch(
        {
          request: body.request,
          decision: body.decision,
          approval: body.approval ?? null,
          lifecycle: state,
          snapshot: body.snapshot,
        },
        { keyring },
      );
    }
    case "lifecycle": {
      const state = lifecycleState(body, keyring);
      if (body.replay_last) {
        applyLifecycleEvent(state, body.events.at(-1), {
          approval: body.approval,
          keyring,
        });
      }
      return state;
    }
    case "validate_proposal":
      return validateProposal(body.proposal, { keyring });
    case "evidence_read":
      return authorizeEvidenceRead(body.read, body.evidence, { keyring });
    default:
      throw new Error(`unknown manifest operation ${vector.operation}`);
  }
}

const manifest = readJson(resolve(ROOT, "manifest.json"));
validateManifest(manifest);
const keyring = loadKeyring();
let passed = 0;
const failures = [];

for (const vector of manifest.vectors) {
  let result = null;
  let error = null;
  try {
    const body = readJson(resolve(ROOT, manifest.vector_root, vector.file));
    result = execute(vector, body, keyring);
  } catch (caught) {
    error = caught;
  }

  if (vector.expected.ok) {
    if (error) {
      failures.push(`${vector.id}: expected success, got ${error.code ?? error.name}: ${error.message}`);
      continue;
    }
    if (vector.expected.outcome !== null && result?.outcome !== vector.expected.outcome) {
      failures.push(
        `${vector.id}: expected outcome ${vector.expected.outcome}, got ${result?.outcome ?? "<none>"}`,
      );
      continue;
    }
  } else {
    if (!error) {
      failures.push(`${vector.id}: expected ${vector.expected.error_code}, got success`);
      continue;
    }
    if (!(error instanceof AuthorityError)) {
      failures.push(`${vector.id}: expected AuthorityError, got ${error.name}: ${error.message}`);
      continue;
    }
    if (error.code !== vector.expected.error_code) {
      failures.push(`${vector.id}: expected ${vector.expected.error_code}, got ${error.code}`);
      continue;
    }
  }
  passed += 1;
}

if (failures.length > 0) {
  console.error(`FAIL ${passed}/${manifest.vectors.length} vectors`);
  failures.forEach((failure) => console.error(`- ${failure}`));
  process.exitCode = 1;
} else {
  console.log(
    `PASS ${passed}/${manifest.vectors.length} vectors; 18/18 categories; manifest exact; read-only runner`,
  );
}
