import assert from "node:assert/strict";
import { createPrivateKey, createPublicKey, generateKeyPairSync } from "node:crypto";
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

test("evidence-read tokens enforce principal-self and auditor-cross role separation", () => {
  for (const file of [
    "vectors/18-authorized-evidence-read.json",
    "vectors/18-auditor-cross-principal-evidence-read.json",
  ]) {
    const vector = read(file);
    assert.doesNotThrow(() =>
      profile.authorizeEvidenceRead(vector.read, vector.evidence, {
        keyring,
        now: vector.now,
      }),
    );
  }
  for (const [file, code] of [
    [
      "vectors/18-threads-authority-cannot-self-read.json",
      "integrity_principal_id_unexpected",
    ],
    [
      "vectors/18-protected-owner-cannot-self-read.json",
      "evidence_read_role_mismatch",
    ],
    [
      "vectors/18-auditor-cannot-use-self-read-role.json",
      "evidence_read_role_mismatch",
    ],
  ]) {
    const vector = read(file);
    const alteredKeyring = new Map(
      [...keyring].map(([id, record]) => [id, structuredClone(record)]),
    );
    if (vector.keyring_mutation?.set) {
      Object.assign(
        alteredKeyring.get(vector.keyring_mutation.key_id),
        vector.keyring_mutation.set,
      );
    }
    assert.throws(
      () =>
        profile.authorizeEvidenceRead(vector.read, vector.evidence, {
          keyring: alteredKeyring,
          now: vector.now,
        }),
      (error) =>
        error instanceof profile.AuthorityError &&
        error.code === code,
    );
  }
});

const throwsCode = (code) => (error) => error instanceof profile.AuthorityError && error.code === code;

test("consumption is valid only from approved (erratum 1)", () => {
  // Vector 16-revoked-approval-cannot-be-consumed covers `revoked`; this
  // covers the other states. The events are re-signed with an authority key
  // made here, swapped into a copy of the keyring as that vector swaps its own.
  const vector = read("vectors/10-recurring-approval-reuses-bounded-authority.json");
  const [request, approve, consume] = vector.events;
  const authority = generateKeyPairSync("ed25519");
  const signer = { key_id: "key:threads:test-authority", private_key: authority.privateKey };
  const keys = new Map([...keyring].map(([id, record]) => [id, structuredClone(record)]));
  keys.get(signer.key_id).public_key_pem = authority.publicKey.export({ type: "spki", format: "pem" });
  const domain = "opencoven:automation-approval-event:v1";
  const chain = (drafts) => {
    const events = [];
    for (const { integrity: _integrity, ...draft } of drafts) {
      const previous = events.length ? `sha256:${profile.canonicalDigest(events.at(-1), domain)}` : null;
      events.push(
        profile.signArtifact({ ...draft, sequence: events.length + 1, previous_event_digest: previous }, domain, signer),
      );
    }
    return events;
  };
  const apply = (events) => {
    let state = null;
    for (const event of events) {
      state = profile.applyLifecycleEvent(state, event, { approval: vector.approval, keyring: keys });
    }
    return state;
  };
  const consumeFrom = (from) => ({
    ...consume,
    event_id: `event:consume-from-${from}`,
    from_state: from,
    to_state: "approved",
    approval_digest: null,
  });
  const expire = {
    ...approve,
    event_id: "event:expire",
    from_state: "approved",
    to_state: "expired",
    event: "expire",
    execution_phase: "not_started",
    dispatch_disposition: "no_launch_expired",
    occurrence_disposition: { occurrence_id: "occ:daily:0001", run_id: "run:daily:0001", disposition: "expired_no_launch" },
  };
  const reject = {
    ...request,
    event_id: "event:reject",
    from_state: "requested",
    to_state: "rejected",
    event: "reject",
    execution_phase: "not_started",
    dispatch_disposition: "no_launch_rejected",
    occurrence_disposition: { occurrence_id: "occ:daily:0001", run_id: "run:daily:0001", disposition: "rejected_no_launch" },
  };
  assert.equal(apply(chain([request, approve, consume])).state, "approved");
  for (const [before, from] of [
    [[request, approve, expire], "expired"],
    [[request, reject], "rejected"],
    [[request], "requested"],
  ]) {
    assert.equal(apply(chain(before)).state, from);
    assert.throws(() => apply(chain([...before, consumeFrom(from)])), throwsCode("lifecycle_transition_invalid"), from);
  }
  // A consume as the first event, with no request or approve. The
  // pre-erratum reference accepted it too: it is the shortest erratum-1
  // counterexample of the coven-threads#95 lifecycle model.
  assert.throws(() => apply(chain([consumeFrom("required")])), throwsCode("lifecycle_transition_invalid"), "required");
});

test("evidence sensitivity must be one of the names (erratum 2)", () => {
  const vector = read("vectors/18-authorized-evidence-read.json");
  const authorize = (sensitivity) =>
    profile.authorizeEvidenceRead(vector.read, { ...vector.evidence, sensitivity }, { keyring, now: vector.now });
  assert.equal(authorize("internal").ok, true);
  for (const sensitivity of [["internal"], [["internal"]], ["internal", null], 1, true, {}, null]) {
    assert.throws(() => authorize(sensitivity), throwsCode("evidence_sensitivity_unknown"), JSON.stringify(sensitivity));
  }
});


test("Node private-PEM acceptance is an explicit extension of the Rust public-key contract", () => {
  const der = Buffer.concat([Buffer.from("302e020100300506032b657004220420", "hex"), Buffer.alloc(32, 88)]);
  const privateKey = createPrivateKey({ key: der, format: "der", type: "pkcs8" });
  const value = profile.signArtifact({ kind: "synthetic-public-key-boundary" }, "opencoven:automation-decision:v1", { key_id: "key:boundary", private_key: privateKey });
  for (const pem of [createPublicKey(privateKey).export({ type: "spki", format: "pem" }), privateKey.export({ type: "pkcs8", format: "pem" })]) {
    const record = { role: "threads_authority", public_key_pem: pem };
    const verified = profile.verifySignedArtifact(value, "opencoven:automation-decision:v1", new Map([["key:boundary", record]]), "threads_authority");
    assert.equal(verified.role, "threads_authority");
  }
});

test("signArtifact refuses an empty key_id or a missing private key before signing (#99)", () => {
  // The producer helper's own guard, with a real ephemeral Ed25519 key. The
  // Rust `sign_artifact` must refuse the representable empty id the same way;
  // a missing private key is a type matter there, since `ArtifactSigner`
  // supplies the signature.
  const authority = generateKeyPairSync("ed25519");
  const domain = "opencoven:automation-decision:v1";
  const artifact = { kind: "test", value: 1 };
  for (const signer of [
    { key_id: "", private_key: authority.privateKey },
    { key_id: "key:test" },
    { key_id: "", private_key: undefined },
    undefined,
    null,
  ]) {
    assert.throws(
      () => profile.signArtifact(artifact, domain, signer),
      (error) => throwsCode("integrity_signer_required")(error) && error.path === "$",
      JSON.stringify(signer ?? null),
    );
  }

  // The receiving side fails closed on its own: the same real signature filed
  // under an empty id is refused at `$.integrity.key_id` before any keyring
  // lookup, so an escaped artifact never verifies.
  const signed = profile.signArtifact(artifact, domain, { key_id: "key:test", private_key: authority.privateKey });
  const keys = new Map([
    ["key:test", { role: "threads_authority", public_key_pem: authority.publicKey.export({ type: "spki", format: "pem" }) }],
  ]);
  assert.equal(profile.verifySignedArtifact(signed, domain, keys, "threads_authority").role, "threads_authority");
  const emptied = { ...signed, integrity: { ...signed.integrity, key_id: "" } };
  assert.throws(
    () => profile.verifySignedArtifact(emptied, domain, keys, "threads_authority"),
    (error) => throwsCode("schema_type")(error) && error.path === "$.integrity.key_id",
  );
});
