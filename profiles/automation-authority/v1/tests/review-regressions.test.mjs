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

test("dispatch rejects a Threads-signed decision that is not the pinned policy result", () => {
  const vector = read("vectors/17-dispatch-refuses-threads-signed-forged-permit.json");
  assert.throws(
    () =>
      profile.verifyDispatch(
        {
          request: vector.request,
          decision: vector.decision,
          approval: null,
          approval_authorization_request: null,
          approval_authorization_decision: null,
          lifecycle_events: [],
          consumption_snapshot: vector.consumption_snapshot,
          snapshot: vector.snapshot,
        },
        { keyring },
      ),
    (error) =>
      error instanceof profile.AuthorityError &&
      error.code === "decision_semantic_mismatch",
  );
});

test("decision and dispatch runtime capability snapshots remain exact", () => {
  for (const [file, code] of [
    [
      "vectors/14-decision-runtime-capabilities-must-match-request.json",
      "decision_runtime_capabilities_changed",
    ],
    [
      "vectors/14-dispatch-runtime-capabilities-must-match-request.json",
      "dispatch_runtime_capabilities_changed",
    ],
  ]) {
    const vector = read(file);
    assert.throws(
      () =>
        profile.verifyDispatch(
          {
            request: vector.request,
            decision: vector.decision,
            approval: null,
            approval_authorization_request: null,
            approval_authorization_decision: null,
            lifecycle_events: [],
            consumption_snapshot: vector.consumption_snapshot,
            snapshot: vector.snapshot,
          },
          { keyring },
        ),
      (error) =>
        error instanceof profile.AuthorityError &&
        error.code === code,
    );
  }
});

test("evidence reads reject missing authority metadata before comparison", () => {
  for (const [file, code] of [
    ["vectors/18-evidence-missing-sensitivity.json", "evidence_sensitivity_unknown"],
    ["vectors/18-evidence-missing-retention.json", "evidence_retention_unknown"],
  ]) {
    const vector = read(file);
    assert.throws(
      () =>
        profile.authorizeEvidenceRead(vector.read, vector.evidence, {
          keyring,
          now: vector.now,
        }),
      (error) =>
        error instanceof profile.AuthorityError &&
        error.code === code,
    );
  }
});

test("timestamps reject impossible UTC calendar dates", () => {
  const vector = read("vectors/06-impossible-request-timestamp.json");
  assert.throws(
    () => profile.validateAuthorizationRequest(vector.request, { keyring }),
    (error) =>
      error instanceof profile.AuthorityError &&
      error.code === "schema_timestamp",
  );
  const approvalNow = read("vectors/06-impossible-approval-validation-now.json");
  assert.throws(
    () =>
      profile.validateApproval(approvalNow.approval, {
        keyring,
        now: approvalNow.now,
      }),
    (error) =>
      error instanceof profile.AuthorityError &&
      error.code === "schema_timestamp",
  );
  const consumptionNow = read(
    "vectors/06-impossible-consumption-validation-now.json",
  );
  assert.throws(
    () =>
      profile.validateConsumptionSnapshot(
        consumptionNow.consumption_snapshot,
        {
          keyring,
          now: consumptionNow.now,
        },
      ),
    (error) =>
      error instanceof profile.AuthorityError &&
      error.code === "schema_timestamp",
  );
  const adoptionNow = read("vectors/06-impossible-request-adoption-now.json");
  assert.throws(
    () =>
      profile.adoptAuthorizationRequest(adoptionNow.request, null, {
        keyring,
        now: adoptionNow.now,
      }),
    (error) =>
      error instanceof profile.AuthorityError &&
      error.code === "schema_timestamp",
  );
  for (const [file, invoke] of [
    [
      "vectors/06-empty-request-adoption-now.json",
      (vector) =>
        profile.adoptAuthorizationRequest(vector.request, null, {
          keyring,
          now: vector.now,
        }),
    ],
    [
      "vectors/06-empty-approval-validation-now.json",
      (vector) =>
        profile.validateApproval(vector.approval, {
          keyring,
          now: vector.now,
        }),
    ],
    [
      "vectors/06-empty-consumption-validation-now.json",
      (vector) =>
        profile.validateConsumptionSnapshot(vector.consumption_snapshot, {
          keyring,
          now: vector.now,
        }),
    ],
  ]) {
    const emptyNow = read(file);
    assert.throws(
      () => invoke(emptyNow),
      (error) =>
        error instanceof profile.AuthorityError &&
        error.code === "schema_timestamp",
    );
  }
});

test("identity-bearing keyring roles require an exact principal_id", () => {
  assert.equal(typeof profile.validateKeyring, "function");
  for (const [file, invoke] of [
    [
      "vectors/17-keyring-principal-request-missing-principal-id.json",
      (vector, alteredKeyring) =>
        profile.validateAuthorizationRequest(vector.request, {
          keyring: alteredKeyring,
        }),
    ],
    [
      "vectors/17-keyring-principal-approval-missing-principal-id.json",
      (vector, alteredKeyring) =>
        profile.validateApproval(vector.approval, { keyring: alteredKeyring }),
    ],
    [
      "vectors/17-keyring-protected-owner-missing-principal-id.json",
      (vector, alteredKeyring) =>
        profile.validateApproval(vector.approval, { keyring: alteredKeyring }),
    ],
    [
      "vectors/18-keyring-auditor-missing-principal-id.json",
      (vector, alteredKeyring) =>
        profile.authorizeEvidenceRead(vector.read, vector.evidence, {
          keyring: alteredKeyring,
          now: vector.now,
        }),
    ],
  ]) {
    const vector = read(file);
    const alteredKeyring = new Map(
      [...keyring].map(([id, record]) => [id, structuredClone(record)]),
    );
    delete alteredKeyring.get(vector.keyring_mutation.key_id).principal_id;
    assert.throws(
      () => invoke(vector, alteredKeyring),
      (error) =>
        error instanceof profile.AuthorityError &&
        error.code === "integrity_principal_id_missing",
    );
  }
});

test("generic evidence.read cannot cross principals through approval and dispatch", () => {
  const vector = read("vectors/18-generic-cross-principal-evidence-dispatch.json");
  assert.throws(
    () =>
      profile.verifyDispatch(
        {
          request: vector.request,
          decision: vector.decision,
          approval: vector.approval,
          approval_authorization_request: null,
          approval_authorization_decision: null,
          lifecycle_events: vector.events,
          consumption_snapshot: vector.consumption_snapshot,
          snapshot: vector.snapshot,
        },
        { keyring },
      ),
    (error) =>
      error instanceof profile.AuthorityError &&
      error.code === "evidence_scope_cross_principal_forbidden",
  );
});
