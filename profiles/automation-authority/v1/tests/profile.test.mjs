import assert from "node:assert/strict";
import { generateKeyPairSync, sign } from "node:crypto";
import { test } from "node:test";

import {
  AuthorityError,
  applyLifecycleEvent,
  canonicalDigest,
  evaluateAuthorization,
  strictParseJson,
  validateApproval,
  validateAuthorizationRequest,
  validateProposal,
  verifyDispatch,
  verifySignedArtifact,
} from "../validator.mjs";

const principalPair = generateKeyPairSync("ed25519");
const authorityPair = generateKeyPairSync("ed25519");
const keyId = "key:principal:alice";
const authorityKeyId = "key:threads:test-authority";
const keyring = new Map([
  [
    keyId,
    {
      role: "principal",
      principal_id: "principal:alice",
      public_key: principalPair.publicKey,
      private_key: principalPair.privateKey,
    },
  ],
  [
    authorityKeyId,
    {
      role: "threads_authority",
      public_key: authorityPair.publicKey,
      private_key: authorityPair.privateKey,
    },
  ],
]);

function integrityFor(value, domain, authority = false) {
  const digest = canonicalDigest(value, domain);
  const signingKey = authority ? authorityPair.privateKey : principalPair.privateKey;
  return {
    alg: "ed25519",
    key_id: authority ? authorityKeyId : keyId,
    signed_digest: digest,
    signature_b64: sign(null, Buffer.from(digest, "hex"), signingKey).toString("base64"),
  };
}

function signed(value, domain, authority = false) {
  return { ...value, integrity: integrityFor(value, domain, authority) };
}

function request(overrides = {}) {
  const value = {
    schema_version: "opencoven.automation-authorization-request/v1",
    request_id: "req:01",
    principal: {
      id: "principal:alice",
      authorization_proof_ref: "proof:session:01",
    },
    replay: {
      nonce: "nonce:01",
      adoption_key: "adopt:01",
      issued_at: "2026-09-03T13:00:00Z",
      expires_at: "2026-09-03T14:00:00Z",
    },
    familiar: {
      id: "familiar:sage",
      embodiment_digest: "sha256:" + "11".repeat(32),
    },
    automation: {
      id: "automation:daily-report",
      definition_revision: 7,
      definition_digest: "sha256:" + "22".repeat(32),
    },
    execution: {
      occurrence_id: "occ:2026-09-03",
      run_id: "run:01",
      attempt: 1,
      fence_generation: 4,
    },
    action: {
      type: "artifact.create",
      digest: "sha256:" + "33".repeat(32),
      risk_class: "R1",
      proposal_safe: true,
    },
    requested_capabilities: ["artifact.write"],
    scopes: [
      {
        kind: "filesystem",
        root: "workspace",
        path: "reports/2026-09-03.md",
        access: "write",
        recursive: false,
      },
    ],
    context: {
      project_id: "project:coven",
      workspace_id: "workspace:main",
      runtime: {
        id: "runtime:node",
        descriptor_digest: "sha256:" + "44".repeat(32),
        capabilities: ["artifact.write"],
      },
    },
    versions: {
      profile: "1.0.0",
      policy: "policy:2026-09-03",
      policy_digest: "sha256:" + "55".repeat(32),
      manifest: "manifest:1",
      manifest_digest: "sha256:" + "66".repeat(32),
    },
    previous_approval_digest: null,
    conditions: [],
    data: {
      sensitivity: "internal",
      retention: "authority_evidence_90d",
    },
    ...overrides,
  };
  return signed(value, "opencoven:automation-request:v1");
}

function policy(overrides = {}) {
  return {
    now: "2026-09-03T13:05:00Z",
    policy: "policy:2026-09-03",
    policy_digest: "sha256:" + "55".repeat(32),
    manifest: "manifest:1",
    manifest_digest: "sha256:" + "66".repeat(32),
    recurring_grants: [
      {
        principal_id: "principal:alice",
        familiar_id: "familiar:sage",
        automation_id: "automation:daily-report",
        definition_digest: "sha256:" + "22".repeat(32),
        action_type: "artifact.create",
        risk_classes: ["R0", "R1"],
        capabilities: ["artifact.write"],
        scopes: [
          {
            kind: "filesystem",
            root: "workspace",
            path: "reports/2026-09-03.md",
            access: "write",
            recursive: false,
          },
        ],
        expires_at: "2026-10-01T00:00:00Z",
        max_uses: 31,
        uses: 1,
      },
    ],
    protected_owner_approval: false,
    ...overrides,
  };
}

test("strict JSON rejects duplicate keys and unsafe I-JSON values", () => {
  assert.throws(
    () => strictParseJson('{"schema_version":"x","schema_version":"y"}'),
    (error) => error instanceof AuthorityError && error.code === "json_duplicate_key",
  );
  assert.throws(
    () => strictParseJson('{"attempt":9007199254740992}'),
    (error) => error instanceof AuthorityError && error.code === "json_unsafe_integer",
  );
  assert.throws(
    () => strictParseJson('"\\ud800"'),
    (error) => error instanceof AuthorityError && error.code === "json_non_ijson",
  );
});

test("request schema is closed and versioned", () => {
  const valid = request();
  assert.doesNotThrow(() => validateAuthorizationRequest(valid, { keyring }));
  assert.throws(
    () => validateAuthorizationRequest({ ...valid, client_approved: true }, { keyring }),
    (error) => error instanceof AuthorityError && error.code === "schema_unknown_field",
  );
  assert.throws(
    () =>
      validateAuthorizationRequest(
        { ...valid, schema_version: "opencoven.automation-authorization-request/v2" },
        { keyring },
      ),
    (error) => error instanceof AuthorityError && error.code === "schema_unknown_version",
  );
});

test("signatures cover canonical immutable bytes and reject tampering", () => {
  const valid = request();
  assert.doesNotThrow(() =>
    verifySignedArtifact(valid, "opencoven:automation-request:v1", keyring),
  );
  const tampered = structuredClone(valid);
  tampered.action.digest = "sha256:" + "ff".repeat(32);
  assert.throws(
    () => verifySignedArtifact(tampered, "opencoven:automation-request:v1", keyring),
    (error) => error instanceof AuthorityError && error.code === "integrity_digest_mismatch",
  );
});

test("narrow R1 recurring authority permits while broad scope fails closed", () => {
  const allowed = evaluateAuthorization(request(), policy(), { keyring });
  assert.equal(allowed.outcome, "permit");
  assert.deepEqual(allowed.granted_capabilities, ["artifact.write"]);

  const broad = request({
    scopes: [
      {
        kind: "filesystem",
        root: "workspace",
        path: "*",
        access: "write",
        recursive: true,
      },
    ],
  });
  assert.throws(
    () => evaluateAuthorization(broad, policy(), { keyring }),
    (error) => error instanceof AuthorityError && error.code === "scope_too_broad",
  );
});

test("partial grants preserve denied capabilities and exact narrowed scope", () => {
  const requested = request({
    requested_capabilities: ["analysis.read", "artifact.write"],
  });
  const decision = evaluateAuthorization(requested, policy(), { keyring });
  assert.equal(decision.outcome, "permit");
  assert.deepEqual(decision.granted_capabilities, ["artifact.write"]);
  assert.deepEqual(decision.denied_capabilities, [
    { capability: "analysis.read", reason_code: "capability_not_granted" },
  ]);
  assert.deepEqual(decision.scopes, policy().recurring_grants[0].scopes);
});

test("proposal downgrade cannot claim or perform protected effects", () => {
  const proposal = signed(
    {
      schema_version: "opencoven.automation-proposal/v1",
      proposal_id: "proposal:01",
      request_digest:
        "sha256:" + canonicalDigest(request(), "opencoven:automation-request:v1"),
      action_digest: "sha256:" + "33".repeat(32),
      intended_target: "https://example.invalid/release",
      status: "not_executed",
      protected_effects_performed: false,
      result_claim: "proposal_only",
      requires_new_adoption: true,
      content_digest: "sha256:" + "77".repeat(32),
      created_at: "2026-09-03T13:06:00Z",
      privacy: "internal",
    },
    "opencoven:automation-proposal:v1",
    true,
  );
  assert.doesNotThrow(() => validateProposal(proposal, { keyring }));
  const forgedSuccess = structuredClone(proposal);
  forgedSuccess.protected_effects_performed = true;
  assert.throws(
    () => validateProposal(forgedSuccess, { keyring }),
    (error) =>
      error instanceof AuthorityError &&
      ["proposal_effect_forbidden", "integrity_digest_mismatch"].includes(error.code),
  );
});

test("approval lifecycle consumption is append-only and replay safe", () => {
  const approval = signed(
    {
      schema_version: "opencoven.automation-approval/v1",
      approval_id: "approval:01",
      approving_principal: {
        id: "principal:alice",
        key_ref: keyId,
      },
      request_digest: "sha256:" + "81".repeat(32),
      decision_digest: "sha256:" + "82".repeat(32),
      familiar_id: "familiar:sage",
      familiar_embodiment_digest: "sha256:" + "11".repeat(32),
      automation: {
        id: "automation:daily-report",
        definition_revision: 7,
        definition_digest: "sha256:" + "22".repeat(32),
      },
      authorized_principal_id: "principal:alice",
      occurrence_id: "occ:2026-09-03",
      run_id: "run:01",
      attempt: 1,
      fence_generation: 4,
      action_digest: "sha256:" + "33".repeat(32),
      capabilities: ["artifact.write"],
      scopes: request().scopes,
      project_id: "project:coven",
      workspace_id: "workspace:main",
      runtime_id: "runtime:node",
      runtime_descriptor_digest: "sha256:" + "44".repeat(32),
      runtime_capabilities: ["artifact.write"],
      use: { kind: "single_use" },
      issued_at: "2026-09-03T13:05:00Z",
      expires_at: "2026-09-03T14:00:00Z",
      nonce: "approval-nonce:01",
      rationale: {
        text: "Approve this exact artifact.",
        privacy: "internal",
      },
    },
    "opencoven:automation-approval:v1",
  );
  assert.doesNotThrow(() => validateApproval(approval, { keyring }));

  const requested = signed(
    {
      schema_version: "opencoven.automation-approval-event/v1",
      event_id: "approval-event:01",
      approval_id: "approval:01",
      request_digest: approval.request_digest,
      decision_digest: approval.decision_digest,
      approval_digest: null,
      sequence: 1,
      previous_event_digest: null,
      from_state: "required",
      to_state: "requested",
      event: "request",
      occurred_at: "2026-09-03T13:01:00Z",
      actor: "threads_authority",
      execution_phase: "not_applicable",
      dispatch_disposition: "not_applicable",
    },
    "opencoven:automation-approval-event:v1",
    true,
  );
  const state1 = applyLifecycleEvent(null, requested, { approval, keyring });
  assert.equal(state1.state, "requested");

  const approved = signed(
    {
      schema_version: "opencoven.automation-approval-event/v1",
      event_id: "approval-event:02",
      approval_id: "approval:01",
      request_digest: approval.request_digest,
      decision_digest: approval.decision_digest,
      approval_digest:
        "sha256:" + canonicalDigest(approval, "opencoven:automation-approval:v1"),
      sequence: 2,
      previous_event_digest: state1.last_event_digest,
      from_state: "requested",
      to_state: "approved",
      event: "approve",
      occurred_at: "2026-09-03T13:05:00Z",
      actor: "threads_authority",
      execution_phase: "not_applicable",
      dispatch_disposition: "not_applicable",
    },
    "opencoven:automation-approval-event:v1",
    true,
  );
  const state2 = applyLifecycleEvent(state1, approved, { approval, keyring });
  assert.equal(state2.state, "approved");

  const consumed = signed(
    {
      schema_version: "opencoven.automation-approval-event/v1",
      event_id: "approval-event:03",
      approval_id: "approval:01",
      request_digest: approval.request_digest,
      decision_digest: approval.decision_digest,
      approval_digest:
        "sha256:" + canonicalDigest(approval, "opencoven:automation-approval:v1"),
      sequence: 3,
      previous_event_digest: state2.last_event_digest,
      from_state: "approved",
      to_state: "consumed",
      event: "consume",
      occurred_at: "2026-09-03T13:06:00Z",
      actor: "threads_authority",
      execution_phase: "dispatching",
      dispatch_disposition: "launch_authorized",
    },
    "opencoven:automation-approval-event:v1",
    true,
  );
  const state3 = applyLifecycleEvent(state2, consumed, { approval, keyring });
  assert.equal(state3.state, "consumed");
  assert.throws(
    () => applyLifecycleEvent(state3, consumed, { approval, keyring }),
    (error) => error instanceof AuthorityError && error.code === "lifecycle_replay",
  );
});

test("dispatch revalidation detects stale fence, policy, definition, runtime, and approval", () => {
  const req = request();
  const decision = evaluateAuthorization(req, policy(), { keyring });
  const snapshot = {
    now: "2026-09-03T13:06:00Z",
    principal_id: req.principal.id,
    familiar_id: req.familiar.id,
    familiar_embodiment_digest: req.familiar.embodiment_digest,
    automation_id: req.automation.id,
    definition_revision: req.automation.definition_revision,
    definition_digest: req.automation.definition_digest,
    occurrence_id: req.execution.occurrence_id,
    run_id: req.execution.run_id,
    attempt: req.execution.attempt,
    fence_generation: req.execution.fence_generation,
    action_digest: req.action.digest,
    runtime_descriptor_digest: req.context.runtime.descriptor_digest,
    runtime_capabilities: req.context.runtime.capabilities,
    project_id: req.context.project_id,
    workspace_id: req.context.workspace_id,
    policy: req.versions.policy,
    policy_digest: req.versions.policy_digest,
    manifest: req.versions.manifest,
    manifest_digest: req.versions.manifest_digest,
  };
  assert.doesNotThrow(() =>
    verifyDispatch({ request: req, decision, approval: null, lifecycle: null, snapshot }, { keyring }),
  );
  assert.throws(
    () =>
      verifyDispatch(
        {
          request: req,
          decision,
          approval: null,
          lifecycle: null,
          snapshot: { ...snapshot, fence_generation: snapshot.fence_generation + 1 },
        },
        { keyring },
      ),
    (error) => error instanceof AuthorityError && error.code === "dispatch_stale_fence",
  );
});
