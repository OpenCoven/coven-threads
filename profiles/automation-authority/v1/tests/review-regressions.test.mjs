import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

import * as profile from "../validator.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const read = (path) => profile.strictParseJson(readFileSync(resolve(ROOT, path), "utf8"));
const keyringDocument = read("keyring.json");
const keyring = new Map(Object.entries(keyringDocument.keys));

test("dispatch exposes authenticated lifecycle-chain and consumption-snapshot validators", () => {
  assert.equal(typeof profile.verifyLifecycleChain, "function");
  assert.equal(typeof profile.validateConsumptionSnapshot, "function");
});

test("dispatch rejects a caller-authored lifecycle summary field", () => {
  const vector = read("vectors/03-human-per-run-principal-dispatch.json");
  assert.throws(
    () =>
      profile.verifyDispatch(
        {
          request: vector.request,
          decision: vector.decision,
          approval: vector.approval,
          approval_authorization_request:
            vector.approval_authorization_request,
          approval_authorization_decision:
            vector.approval_authorization_decision,
          lifecycle_events: vector.events,
          consumption_snapshot: vector.consumption_snapshot,
          snapshot: vector.snapshot,
          lifecycle: { state: "approved" },
        },
        { keyring },
      ),
    (error) =>
      error instanceof profile.AuthorityError &&
      error.code === "schema_unknown_field",
  );
});

test("signature verification rejects noncanonical Base64", () => {
  const vector = read("vectors/02-r1-narrow-recurring-permit.json");
  const request = structuredClone(vector.request);
  request.integrity.signature_b64 =
    `${request.integrity.signature_b64.slice(0, 8)} \n${request.integrity.signature_b64.slice(8)}`;
  assert.throws(
    () =>
      profile.verifySignedArtifact(
        request,
        "opencoven:automation-request:v1",
        keyring,
      ),
    (error) =>
      error instanceof profile.AuthorityError &&
      error.code === "integrity_signature_noncanonical",
  );
});

test("recurring grant matching includes revision, project, workspace, and runtime snapshot", () => {
  const vector = read("vectors/02-r1-narrow-recurring-permit.json");
  const policy = structuredClone(vector.policy);
  Object.assign(policy.recurring_grants[0], {
    definition_revision: 999,
    project_id: "project:other",
    workspace_id: "workspace:other",
    runtime_id: "runtime:other",
    runtime_descriptor_digest: `sha256:${"f".repeat(64)}`,
    runtime_capabilities: ["analysis.read"],
  });
  const result = profile.evaluateAuthorization(vector.request, policy, {
    keyring,
    unsigned: true,
  });
  assert.notEqual(result.outcome, "permit");
});

test("evidence reads reject not-yet-valid and expired signed tokens using trusted now", () => {
  const vector = read("vectors/18-authorized-evidence-read.json");
  assert.throws(
    () =>
      profile.authorizeEvidenceRead(vector.read, vector.evidence, {
        keyring,
        now: "2026-09-03T12:59:59Z",
      }),
    (error) =>
      error instanceof profile.AuthorityError &&
      error.code === "evidence_read_not_yet_valid",
  );
  assert.throws(
    () =>
      profile.authorizeEvidenceRead(vector.read, vector.evidence, {
        keyring,
        now: "2026-09-03T14:00:00Z",
      }),
    (error) =>
      error instanceof profile.AuthorityError &&
      error.code === "evidence_read_expired",
  );
});
