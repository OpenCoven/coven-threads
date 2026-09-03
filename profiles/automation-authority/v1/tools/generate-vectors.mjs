import {
  generateKeyPairSync,
} from "node:crypto";
import {
  mkdirSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import {
  applyLifecycleEvent,
  canonicalDigest,
  evaluateAuthorization,
  signArtifact,
} from "../validator.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const VECTOR_DIR = resolve(ROOT, "vectors");
mkdirSync(VECTOR_DIR, { recursive: true });
for (const name of readdirSync(VECTOR_DIR)) {
  if (name.endsWith(".json")) rmSync(resolve(VECTOR_DIR, name));
}

function pair() {
  return generateKeyPairSync("ed25519");
}

const authority = pair();
const alice = pair();
const bob = pair();
const owner = pair();
const auditor = pair();
const keys = {
  "key:threads:test-authority": {
    role: "threads_authority",
    public_key: authority.publicKey,
    private_key: authority.privateKey,
  },
  "key:principal:alice": {
    role: "principal",
    principal_id: "principal:alice",
    public_key: alice.publicKey,
    private_key: alice.privateKey,
  },
  "key:principal:bob": {
    role: "principal",
    principal_id: "principal:bob",
    public_key: bob.publicKey,
    private_key: bob.privateKey,
  },
  "key:protected-owner": {
    role: "protected_owner",
    principal_id: "principal:owner",
    public_key: owner.publicKey,
    private_key: owner.privateKey,
  },
  "key:auditor": {
    role: "auditor",
    principal_id: "principal:auditor",
    public_key: auditor.publicKey,
    private_key: auditor.privateKey,
  },
};
const keyring = new Map(Object.entries(keys));
const signer = {
  authority: {
    key_id: "key:threads:test-authority",
    private_key: authority.privateKey,
  },
  alice: {
    key_id: "key:principal:alice",
    private_key: alice.privateKey,
  },
  bob: {
    key_id: "key:principal:bob",
    private_key: bob.privateKey,
  },
  owner: {
    key_id: "key:protected-owner",
    private_key: owner.privateKey,
  },
  auditor: {
    key_id: "key:auditor",
    private_key: auditor.privateKey,
  },
};

writeFileSync(
  resolve(ROOT, "keyring.json"),
  `${JSON.stringify(
    {
      schema_version: "opencoven.automation-authority-test-keyring/v1",
      keys: Object.fromEntries(
        Object.entries(keys).map(([id, value]) => [
          id,
          {
            role: value.role,
            ...(value.principal_id ? { principal_id: value.principal_id } : {}),
            public_key_pem: value.public_key.export({ type: "spki", format: "pem" }),
          },
        ]),
      ),
    },
    null,
    2,
  )}\n`,
);

const digest = (byte) => `sha256:${byte.repeat(64)}`;
const fsScope = (path, access = "write") => ({
  kind: "filesystem",
  root: "workspace",
  path,
  access,
  recursive: false,
});
const networkScope = () => ({
  kind: "network",
  scheme: "https",
  host: "api.example.invalid",
  port: 443,
  path_prefix: "/v1/releases",
  methods: ["POST"],
});

function request({
  id = "r1",
  principal = "principal:alice",
  familiar = "familiar:sage",
  embodiment = digest("1"),
  automation = "automation:daily-report",
  revision = 7,
  definition = digest("2"),
  occurrence = "occ:2026-09-03",
  run = "run:01",
  attempt = 1,
  fence = 4,
  action = "artifact.create",
  actionDigest = digest("3"),
  risk = "R1",
  proposalSafe = true,
  capabilities = ["artifact.write"],
  scopes = [fsScope("reports/2026-09-03.md")],
  runtimeCapabilities = capabilities,
  project = "project:coven",
  workspace = "workspace:main",
  runtime = "runtime:node",
  runtimeDigest = digest("4"),
  previousApproval = null,
  conditions = [],
  extra = {},
  key = "alice",
} = {}) {
  const value = {
    schema_version: "opencoven.automation-authorization-request/v1",
    request_id: `request:${id}`,
    principal: {
      id: principal,
      authorization_proof_ref: `proof:${id}`,
    },
    replay: {
      nonce: `nonce:${id}`,
      adoption_key: `adopt:${id}`,
      issued_at: "2026-09-03T13:00:00Z",
      expires_at: "2026-09-03T14:00:00Z",
    },
    familiar: {
      id: familiar,
      embodiment_digest: embodiment,
    },
    automation: {
      id: automation,
      definition_revision: revision,
      definition_digest: definition,
    },
    execution: {
      occurrence_id: occurrence,
      run_id: run,
      attempt,
      fence_generation: fence,
    },
    action: {
      type: action,
      digest: actionDigest,
      risk_class: risk,
      proposal_safe: proposalSafe,
    },
    requested_capabilities: capabilities,
    scopes,
    context: {
      project_id: project,
      workspace_id: workspace,
      runtime: {
        id: runtime,
        descriptor_digest: runtimeDigest,
        capabilities: runtimeCapabilities,
      },
    },
    versions: {
      profile: "1.0.0",
      policy: "policy:2026-09-03",
      policy_digest: digest("5"),
      manifest: "manifest:1",
      manifest_digest: digest("6"),
    },
    previous_approval_digest: previousApproval,
    conditions,
    data: {
      sensitivity: "internal",
      retention: "authority_evidence_90d",
    },
    ...extra,
  };
  return signArtifact(value, "opencoven:automation-request:v1", signer[key]);
}

function resign(value, domain, which = "alice") {
  const copy = structuredClone(value);
  delete copy.integrity;
  return signArtifact(copy, domain, signer[which]);
}

function policyFor(req, {
  capabilities = req.requested_capabilities,
  scopes = req.scopes,
  grant = true,
  protectedOwner = false,
} = {}) {
  return {
    now: "2026-09-03T13:05:00Z",
    policy: req.versions.policy,
    policy_digest: req.versions.policy_digest,
    manifest: req.versions.manifest,
    manifest_digest: req.versions.manifest_digest,
    recurring_grants: grant
      ? [
          {
            grant_id: `grant:${req.request_id}`,
            principal_id: req.principal.id,
            familiar_id: req.familiar.id,
            familiar_embodiment_digest: req.familiar.embodiment_digest,
            automation_id: req.automation.id,
            definition_revision: req.automation.definition_revision,
            definition_digest: req.automation.definition_digest,
            action_type: req.action.type,
            action_digest: req.action.digest,
            project_id: req.context.project_id,
            workspace_id: req.context.workspace_id,
            runtime_id: req.context.runtime.id,
            runtime_descriptor_digest: req.context.runtime.descriptor_digest,
            runtime_capabilities: [...req.context.runtime.capabilities].sort(),
            risk_classes: ["R0", "R1"],
            capabilities,
            scopes,
            expires_at: "2026-10-01T00:00:00Z",
            max_uses: 31,
            uses: 1,
          },
        ]
      : [],
    protected_owner_approval: protectedOwner,
    recurring_approval_allowed: false,
  };
}

function decision(req, policy) {
  return evaluateAuthorization(req, policy, { keyring });
}

function approval(req, dec, overrides = {}, approver = "alice") {
  const approvers = {
    alice: { id: "principal:alice", key: "key:principal:alice" },
    owner: { id: "principal:owner", key: "key:protected-owner" },
  };
  const value = {
    schema_version: "opencoven.automation-approval/v1",
    approval_id: `approval:${req.request_id}`,
    approving_principal: {
      id: approvers[approver].id,
      key_ref: approvers[approver].key,
    },
    request_digest: `sha256:${canonicalDigest(req, "opencoven:automation-request:v1")}`,
    decision_digest: `sha256:${canonicalDigest(dec, "opencoven:automation-decision:v1")}`,
    familiar_id: req.familiar.id,
    familiar_embodiment_digest: req.familiar.embodiment_digest,
    automation: structuredClone(req.automation),
    authorized_principal_id: req.principal.id,
    occurrence_id: req.execution.occurrence_id,
    run_id: req.execution.run_id,
    attempt: req.execution.attempt,
    fence_generation: req.execution.fence_generation,
    action_digest: req.action.digest,
    capabilities: structuredClone(req.requested_capabilities),
    scopes: structuredClone(req.scopes),
    project_id: req.context.project_id,
    workspace_id: req.context.workspace_id,
    runtime_id: req.context.runtime.id,
    runtime_descriptor_digest: req.context.runtime.descriptor_digest,
    runtime_capabilities: structuredClone(req.context.runtime.capabilities),
    versions: structuredClone(req.versions),
    use: { kind: "single_use" },
    issued_at: "2026-09-03T13:05:00Z",
    expires_at: "2026-09-03T14:00:00Z",
    nonce: `approval-nonce:${req.request_id}`,
    rationale: {
      text: "Approve this exact operation.",
      privacy: "internal",
    },
    ...overrides,
  };
  return signArtifact(value, "opencoven:automation-approval:v1", signer[approver]);
}

function event(app, sequence, previous, from, to, type, {
  phase = "not_applicable",
  disposition = "not_applicable",
  consumption = null,
  occurrenceDisposition = null,
  actor = "threads_authority",
} = {}) {
  return signArtifact(
    {
      schema_version: "opencoven.automation-approval-event/v1",
      event_id: `event:${app.approval_id}:${sequence}:${type}`,
      approval_id: app.approval_id,
      request_digest: app.request_digest,
      decision_digest: app.decision_digest,
      approval_digest:
        type === "request" || (from === "requested" && type !== "approve")
          ? null
          : `sha256:${canonicalDigest(app, "opencoven:automation-approval:v1")}`,
      sequence,
      previous_event_digest: previous,
      from_state: from,
      to_state: to,
      event: type,
      occurred_at: `2026-09-03T13:0${sequence}:00Z`,
      actor,
      execution_phase: phase,
      dispatch_disposition: disposition,
      consumption,
      occurrence_disposition: occurrenceDisposition,
    },
    "opencoven:automation-approval-event:v1",
    signer.authority,
  );
}

function lifecycle(app, terminal = null, options = {}) {
  const first = event(app, 1, null, "required", "requested", "request");
  const firstDigest = `sha256:${canonicalDigest(first, "opencoven:automation-approval-event:v1")}`;
  const second = event(app, 2, firstDigest, "requested", "approved", "approve");
  const secondDigest = `sha256:${canonicalDigest(second, "opencoven:automation-approval-event:v1")}`;
  if (!terminal) return [first, second];
  const consumption =
    terminal === "consume"
      ? options.consumption ?? {
          request_digest: app.request_digest,
          decision_digest: app.decision_digest,
          occurrence_id: app.occurrence_id,
          run_id: app.run_id,
          attempt: app.attempt,
          fence_generation: app.fence_generation,
        }
      : null;
  return [
    first,
    second,
    event(
      app,
      3,
      secondDigest,
      "approved",
      terminal === "consume" && app.use.kind === "recurring"
        ? "approved"
        : terminal === "consume"
          ? "consumed"
          : "revoked",
      terminal,
      { ...options, consumption },
    ),
  ];
}

function runConsumption(req, dec) {
  return {
    request_digest: `sha256:${canonicalDigest(req, "opencoven:automation-request:v1")}`,
    decision_digest: `sha256:${canonicalDigest(dec, "opencoven:automation-decision:v1")}`,
    occurrence_id: req.execution.occurrence_id,
    run_id: req.execution.run_id,
    attempt: req.execution.attempt,
    fence_generation: req.execution.fence_generation,
  };
}

function snapshot(req, overrides = {}) {
  return {
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
    runtime_id: req.context.runtime.id,
    runtime_descriptor_digest: req.context.runtime.descriptor_digest,
    runtime_capabilities: req.context.runtime.capabilities,
    project_id: req.context.project_id,
    workspace_id: req.context.workspace_id,
    policy: req.versions.policy,
    policy_digest: req.versions.policy_digest,
    manifest: req.versions.manifest,
    manifest_digest: req.versions.manifest_digest,
    consumption_revision: 7,
    ...overrides,
  };
}

function consumptionSnapshot(req, dec, app = null, events = [], overrides = {}) {
  let state = null;
  if (app) {
    for (const lifecycleEvent of events) {
      state = applyLifecycleEvent(state, lifecycleEvent, {
        approval: app,
        keyring,
      });
    }
  }
  return signArtifact(
    {
      schema_version: "opencoven.automation-consumption-snapshot/v1",
      snapshot_id: `consumption:${req.request_id}`,
      recorded_at: "2026-09-03T13:05:30Z",
      store_revision: 7,
      request_adoptions: [
        {
          request_digest: `sha256:${canonicalDigest(
            req,
            "opencoven:automation-request:v1",
          )}`,
          nonce: req.replay.nonce,
          adoption_key: req.replay.adoption_key,
        },
      ],
      decision_consumptions: [],
      approval_heads:
        app && state
          ? [
              {
                approval_id: app.approval_id,
                head_event_digest: state.last_event_digest,
                usage_count: state.consumption_count,
              },
            ]
          : [],
      ...overrides,
    },
    "opencoven:automation-consumption-snapshot:v1",
    signer.authority,
  );
}

const manifest = {
  schema_version: "opencoven.automation-authority-conformance-manifest/v1",
  profile_version: "1.0.0",
  vector_root: "vectors",
  categories: Array.from({ length: 18 }, (_, index) => index + 1),
  vectors: [],
};

function add(id, category, kind, operation, body, {
  ok = kind === "positive",
  error = null,
  outcome = null,
} = {}) {
  if (operation === "verify_dispatch" && !body.consumption_snapshot) {
    body.consumption_snapshot = consumptionSnapshot(
      body.request,
      body.decision,
      body.approval ?? null,
      body.events ?? [],
    );
  }
  if (
    operation === "verify_dispatch" &&
    !Object.hasOwn(body, "approval_authorization_request")
  ) {
    body.approval_authorization_request = null;
  }
  if (
    operation === "verify_dispatch" &&
    !Object.hasOwn(body, "approval_authorization_decision")
  ) {
    body.approval_authorization_decision = null;
  }
  const file = `${id}.json`;
  writeFileSync(resolve(VECTOR_DIR, file), `${JSON.stringify(body, null, 2)}\n`);
  manifest.vectors.push({
    id,
    category,
    kind,
    file,
    operation,
    expected: {
      ok,
      error_code: error,
      outcome,
    },
  });
}

const r0 = request({
  id: "r0-read",
  action: "analysis.read",
  risk: "R0",
  capabilities: ["analysis.read"],
  scopes: [fsScope("inputs/report.json", "read")],
});
const r0Policy = policyFor(r0);
add("01-r0-read-permit", 1, "positive", "verify_decision", {
  request: r0,
  policy: r0Policy,
  decision: decision(r0, r0Policy),
}, { outcome: "permit" });

const r1 = request({ id: "r1-recurring" });
const r1Policy = policyFor(r1);
const r1Decision = decision(r1, r1Policy);
add("02-r1-narrow-recurring-permit", 2, "positive", "verify_decision", {
  request: r1,
  policy: r1Policy,
  decision: r1Decision,
}, { outcome: "permit" });
for (const [name, field, value] of [
  ["familiar-embodiment", "familiar_embodiment_digest", digest("e")],
  ["definition-revision", "definition_revision", 999],
  ["action-digest", "action_digest", digest("e")],
  ["project", "project_id", "project:other"],
  ["workspace", "workspace_id", "workspace:other"],
  ["runtime-id", "runtime_id", "runtime:other"],
  ["runtime-descriptor", "runtime_descriptor_digest", digest("f")],
  ["runtime-capabilities", "runtime_capabilities", ["analysis.read"]],
]) {
  const mismatchedPolicy = structuredClone(r1Policy);
  mismatchedPolicy.recurring_grants[0][field] = value;
  add(`02-recurring-grant-${name}-mismatch`, 2, "positive", "evaluate_request", {
    request: r1,
    policy: mismatchedPolicy,
  }, { outcome: "degrade_to_proposal" });
}

const r2 = request({
  id: "r2-migration",
  action: "state.migrate",
  risk: "R2",
  proposalSafe: false,
  capabilities: ["state.mutate"],
  scopes: [fsScope("state/schema-v2.sqlite")],
  conditions: ["deterministic_validation", "rollback_plan"],
});
const r2Policy = policyFor(r2, { grant: false });
const r2Decision = decision(r2, r2Policy);
add("03-r2-requires-approval", 3, "positive", "verify_decision", {
  request: r2,
  policy: r2Policy,
  decision: r2Decision,
}, { outcome: "requires_approval" });
const importedR1 = request({
  id: "imported-r1",
  conditions: ["automation_imported"],
});
const importedPolicy = policyFor(importedR1);
add("03-imported-automation-paused-for-review", 3, "positive", "verify_decision", {
  request: importedR1,
  policy: importedPolicy,
  decision: decision(importedR1, importedPolicy),
}, { outcome: "requires_approval" });
const unsafeR2 = request({
  id: "r2-missing-safeguards",
  action: "state.migrate",
  risk: "R2",
  proposalSafe: false,
  capabilities: ["state.mutate"],
  scopes: [fsScope("state/schema-v2.sqlite")],
});
add("03-r2-missing-safeguards", 3, "negative", "validate_request", {
  request: unsafeR2,
}, { error: "r2_safeguards_missing" });
const runtimeMissingR2 = request({
  id: "r2-runtime-capability-missing",
  action: "state.migrate",
  risk: "R2",
  proposalSafe: false,
  capabilities: ["state.mutate"],
  runtimeCapabilities: ["analysis.read"],
  scopes: [fsScope("state/schema-v2.sqlite")],
  conditions: ["deterministic_validation", "rollback_plan"],
});
const runtimeMissingPolicy = policyFor(runtimeMissingR2, { grant: false });
add("03-approval-runtime-capability-missing-at-evaluation", 3, "negative", "evaluate_request", {
  request: runtimeMissingR2,
  policy: runtimeMissingPolicy,
}, { error: "runtime_capability_missing" });

const r3 = request({
  id: "r3-publication",
  action: "external.publish",
  risk: "R3",
  capabilities: ["network.publish"],
  scopes: [networkScope()],
});
const r3Policy = policyFor(r3, { grant: false });
const r3Decision = decision(r3, r3Policy);
add("04-r3-publication-proposal-only", 4, "positive", "verify_decision", {
  request: r3,
  policy: r3Policy,
  decision: r3Decision,
}, { outcome: "degrade_to_proposal" });

const r4 = request({
  id: "r4-identity",
  action: "identity.mutate",
  risk: "R4",
  proposalSafe: false,
  capabilities: ["identity.mutate"],
  scopes: [fsScope("identity/SOUL.md")],
});
const r4Policy = policyFor(r4, { grant: false, protectedOwner: false });
add("05-r4-without-owner-reject", 5, "positive", "verify_decision", {
  request: r4,
  policy: r4Policy,
  decision: decision(r4, r4Policy),
}, { outcome: "reject" });
const r4ApprovedPolicy = policyFor(r4, { grant: false, protectedOwner: true });
const r4ApprovedDecision = decision(r4, r4ApprovedPolicy);
const r4OwnerApproval = approval(r4, r4ApprovedDecision, {}, "owner");
const r4OwnerEvents = lifecycle(r4OwnerApproval);
add("05-r4-protected-owner-approval-dispatch", 5, "positive", "verify_dispatch", {
  request: r4,
  decision: r4ApprovedDecision,
  approval: r4OwnerApproval,
  events: r4OwnerEvents,
  snapshot: snapshot(r4),
});
const r4PrincipalApproval = approval(r4, r4ApprovedDecision);
const r4PrincipalEvents = lifecycle(r4PrincipalApproval);
add("05-r4-principal-cannot-substitute-for-owner", 5, "negative", "verify_dispatch", {
  request: r4,
  decision: r4ApprovedDecision,
  approval: r4PrincipalApproval,
  events: r4PrincipalEvents,
  snapshot: snapshot(r4),
}, { error: "approval_role_mismatch" });

const unknownAction = structuredClone(r1);
unknownAction.action.type = "prompt.says.allowed";
add("06-unknown-action", 6, "negative", "validate_request", {
  request: resign(unknownAction, "opencoven:automation-request:v1"),
}, { error: "action_unknown" });
const unknownCapability = structuredClone(r1);
unknownCapability.requested_capabilities = ["shell.unbounded"];
add("06-unknown-capability", 6, "negative", "validate_request", {
  request: resign(unknownCapability, "opencoven:automation-request:v1"),
}, { error: "capability_unknown" });
const promptEscalation = structuredClone(r1);
promptEscalation.prompt_grants = ["credential.use"];
add("06-prompt-cannot-grant-capability", 6, "negative", "validate_request", {
  request: resign(promptEscalation, "opencoven:automation-request:v1"),
}, { error: "schema_unknown_field" });
const forgedPrincipal = structuredClone(r1);
forgedPrincipal.principal.id = "principal:bob";
add("06-forged-principal-string", 6, "negative", "validate_request", {
  request: resign(forgedPrincipal, "opencoven:automation-request:v1"),
}, { error: "principal_key_mismatch" });

const narrowReq = request({
  id: "scope-narrow",
  scopes: [fsScope("reports/allowed.md"), fsScope("reports/not-granted.md")],
});
const narrowPolicy = policyFor(narrowReq, { scopes: [narrowReq.scopes[0]] });
add("07-scope-narrowing", 7, "positive", "verify_decision", {
  request: narrowReq,
  policy: narrowPolicy,
  decision: decision(narrowReq, narrowPolicy),
}, { outcome: "permit" });
const broadReq = request({
  id: "scope-broad",
  scopes: [{
    kind: "filesystem",
    root: "workspace",
    path: "*",
    access: "write",
    recursive: true,
  }],
});
add("07-wildcard-scope-refused", 7, "negative", "validate_request", {
  request: broadReq,
}, { error: "scope_too_broad" });
const credentialWithoutScope = request({
  id: "credential-without-scope",
  action: "network.fetch",
  risk: "R3",
  capabilities: ["credential.use"],
  scopes: [networkScope()],
});
add("07-credential-requires-exact-scope", 7, "negative", "validate_request", {
  request: credentialWithoutScope,
}, { error: "capability_scope_mismatch" });
const wildcardCredential = request({
  id: "credential-wildcard",
  action: "network.fetch",
  risk: "R3",
  capabilities: ["credential.use"],
  scopes: [{
    kind: "credential",
    credential_ref: "credential:publisher",
    audience: "*",
    operations: ["publish"],
  }],
});
add("07-credential-wildcard-audience-refused", 7, "negative", "validate_request", {
  request: wildcardCredential,
}, { error: "scope_too_broad" });

const partialReq = request({
  id: "partial-grant",
  capabilities: ["analysis.read", "artifact.write"],
  runtimeCapabilities: ["analysis.read", "artifact.write"],
});
const partialPolicy = policyFor(partialReq, { capabilities: ["artifact.write"] });
add("08-partial-capability-grant", 8, "positive", "verify_decision", {
  request: partialReq,
  policy: partialPolicy,
  decision: decision(partialReq, partialPolicy),
}, { outcome: "permit" });

const proposal = signArtifact(
  {
    schema_version: "opencoven.automation-proposal/v1",
    proposal_id: "proposal:r3-publication",
    request_digest: `sha256:${canonicalDigest(r3, "opencoven:automation-request:v1")}`,
    action_digest: r3.action.digest,
    intended_target: "https://api.example.invalid/v1/releases",
    status: "not_executed",
    protected_effects_performed: false,
    result_claim: "proposal_only",
    requires_new_adoption: true,
    content_digest: digest("7"),
    created_at: "2026-09-03T13:06:00Z",
    privacy: "internal",
  },
  "opencoven:automation-proposal:v1",
  signer.authority,
);
add("09-proposal-has-no-protected-effect", 9, "positive", "validate_proposal", { proposal });
const forgedProposal = structuredClone(proposal);
forgedProposal.protected_effects_performed = true;
add("09-proposal-cannot-claim-execution", 9, "negative", "validate_proposal", {
  proposal: resign(forgedProposal, "opencoven:automation-proposal:v1", "authority"),
}, { error: "proposal_effect_forbidden" });

const r2Approval = approval(r2, r2Decision);
const r2ApprovedEvents = lifecycle(r2Approval);
add("03-human-per-run-principal-dispatch", 3, "positive", "verify_dispatch", {
  request: r2,
  decision: r2Decision,
  approval: r2Approval,
  events: r2ApprovedEvents,
  snapshot: snapshot(r2),
});
const r2OwnerApproval = approval(r2, r2Decision, {}, "owner");
const r2OwnerEvents = lifecycle(r2OwnerApproval);
add("03-human-per-run-refuses-protected-owner-substitution", 3, "negative", "verify_dispatch", {
  request: r2,
  decision: r2Decision,
  approval: r2OwnerApproval,
  events: r2OwnerEvents,
  snapshot: snapshot(r2),
}, { error: "approval_role_mismatch" });
add("03-approval-runtime-capability-missing-at-dispatch", 3, "negative", "verify_dispatch", {
  request: r2,
  decision: r2Decision,
  approval: r2Approval,
  events: r2ApprovedEvents,
  snapshot: snapshot(r2, { runtime_capabilities: ["analysis.read"] }),
}, { error: "dispatch_runtime_capability_missing" });
add("10-single-use-approval-consumed", 10, "positive", "lifecycle", {
  approval: r2Approval,
  events: lifecycle(r2Approval, "consume", {
    phase: "dispatching",
    disposition: "launch_authorized",
  }),
});
add("10-single-use-approval-replay", 10, "negative", "lifecycle", {
  approval: r2Approval,
  events: lifecycle(r2Approval, "consume", {
    phase: "dispatching",
    disposition: "launch_authorized",
  }),
  replay_last: true,
}, { error: "lifecycle_replay" });
add("10-request-adoption-replay", 10, "negative", "request_adoption", {
  request: r1,
  repetitions: 2,
  now: "2026-09-03T13:05:00Z",
}, { error: "request_replayed" });
add("10-decision-consumption-replay", 10, "negative", "decision_consumption", {
  decision: r1Decision,
  repetitions: 2,
}, { error: "decision_replayed" });
add("10-dispatch-requires-request-adoption", 10, "negative", "verify_dispatch", {
  request: r1,
  decision: r1Decision,
  approval: null,
  events: [],
  consumption_snapshot: consumptionSnapshot(r1, r1Decision, null, [], {
    request_adoptions: [],
  }),
  snapshot: snapshot(r1),
}, { error: "request_not_adopted" });
add("10-dispatch-refuses-consumed-decision", 10, "negative", "verify_dispatch", {
  request: r1,
  decision: r1Decision,
  approval: null,
  events: [],
  consumption_snapshot: consumptionSnapshot(r1, r1Decision, null, [], {
    decision_consumptions: [
      `sha256:${canonicalDigest(r1Decision, "opencoven:automation-decision:v1")}`,
    ],
  }),
  snapshot: snapshot(r1),
}, { error: "decision_replayed" });

const recurringGrantRequest = request({
  id: "recurring-approval-grant",
  occurrence: "occ:daily:0001",
  run: "run:daily:0001",
  action: "state.migrate",
  risk: "R2",
  proposalSafe: false,
  capabilities: ["state.mutate"],
  scopes: [fsScope("state/schema-v2.sqlite")],
  conditions: ["deterministic_validation", "rollback_plan"],
});
const recurringApprovalPolicy = {
  ...policyFor(recurringGrantRequest, { grant: false }),
  recurring_approval_allowed: true,
};
const recurringGrantDecision = decision(recurringGrantRequest, recurringApprovalPolicy);
const recurringApproval = approval(recurringGrantRequest, recurringGrantDecision, {
  occurrence_id: null,
  run_id: null,
  attempt: null,
  fence_generation: null,
  use: {
    kind: "recurring",
    grant_id: "recurring-approval:daily-state",
    max_uses: 2,
    occurrence_prefix: "occ:daily:",
  },
});
const recurringApprovalEvents = lifecycle(recurringApproval);
const recurringApproveHead = `sha256:${canonicalDigest(
  recurringApprovalEvents.at(-1),
  "opencoven:automation-approval-event:v1",
)}`;
const firstRecurringConsumption = event(
  recurringApproval,
  3,
  recurringApproveHead,
  "approved",
  "approved",
  "consume",
  {
    phase: "dispatching",
    disposition: "launch_authorized",
    consumption: runConsumption(recurringGrantRequest, recurringGrantDecision),
  },
);
const recurringEventsAfterOne = [...recurringApprovalEvents, firstRecurringConsumption];
const recurringSecondRequest = request({
  id: "recurring-approval-second",
  occurrence: "occ:daily:0002",
  run: "run:daily:0002",
  action: "state.migrate",
  risk: "R2",
  proposalSafe: false,
  capabilities: ["state.mutate"],
  scopes: [fsScope("state/schema-v2.sqlite")],
  conditions: ["deterministic_validation", "rollback_plan"],
});
const recurringSecondPolicy = {
  ...policyFor(recurringSecondRequest, { grant: false }),
  recurring_approval_allowed: true,
};
const recurringSecondDecision = decision(recurringSecondRequest, recurringSecondPolicy);
add("10-recurring-approval-reuses-bounded-authority", 10, "positive", "verify_dispatch", {
  request: recurringSecondRequest,
  decision: recurringSecondDecision,
  approval: recurringApproval,
  approval_authorization_request: recurringGrantRequest,
  approval_authorization_decision: recurringGrantDecision,
  events: recurringEventsAfterOne,
  snapshot: snapshot(recurringSecondRequest),
});
add("10-recurring-approval-requires-grant-decision", 10, "negative", "verify_dispatch", {
  request: recurringSecondRequest,
  decision: recurringSecondDecision,
  approval: recurringApproval,
  approval_authorization_request: null,
  approval_authorization_decision: null,
  events: recurringEventsAfterOne,
  snapshot: snapshot(recurringSecondRequest),
}, { error: "approval_recurring_authorization_missing" });
add("10-recurring-approval-refuses-same-occurrence-replay", 10, "negative", "verify_dispatch", {
  request: recurringGrantRequest,
  decision: recurringGrantDecision,
  approval: recurringApproval,
  approval_authorization_request: recurringGrantRequest,
  approval_authorization_decision: recurringGrantDecision,
  events: recurringEventsAfterOne,
  snapshot: snapshot(recurringGrantRequest),
}, { error: "approval_replayed" });
const firstConsumptionHead = `sha256:${canonicalDigest(
  firstRecurringConsumption,
  "opencoven:automation-approval-event:v1",
)}`;
const secondRecurringConsumption = event(
  recurringApproval,
  4,
  firstConsumptionHead,
  "approved",
  "approved",
  "consume",
  {
    phase: "dispatching",
    disposition: "launch_authorized",
    consumption: runConsumption(recurringSecondRequest, recurringSecondDecision),
  },
);
const recurringEventsExhausted = [...recurringEventsAfterOne, secondRecurringConsumption];
const recurringThirdRequest = request({
  id: "recurring-approval-third",
  occurrence: "occ:daily:0003",
  run: "run:daily:0003",
  action: "state.migrate",
  risk: "R2",
  proposalSafe: false,
  capabilities: ["state.mutate"],
  scopes: [fsScope("state/schema-v2.sqlite")],
  conditions: ["deterministic_validation", "rollback_plan"],
});
const recurringThirdPolicy = {
  ...policyFor(recurringThirdRequest, { grant: false }),
  recurring_approval_allowed: true,
};
const recurringThirdDecision = decision(recurringThirdRequest, recurringThirdPolicy);
add("10-recurring-approval-usage-bound-exhausted", 10, "negative", "verify_dispatch", {
  request: recurringThirdRequest,
  decision: recurringThirdDecision,
  approval: recurringApproval,
  approval_authorization_request: recurringGrantRequest,
  approval_authorization_decision: recurringGrantDecision,
  events: recurringEventsExhausted,
  snapshot: snapshot(recurringThirdRequest),
}, { error: "approval_usage_exhausted" });

const expiredApproval = approval(r2, r2Decision, {
  expires_at: "2026-09-03T13:05:30Z",
});
add("11-expired-approval", 11, "negative", "validate_approval", {
  approval: expiredApproval,
  now: "2026-09-03T13:06:00Z",
}, { error: "approval_expired" });
const futureRequest = structuredClone(r1);
futureRequest.replay.issued_at = "2026-09-03T13:10:00Z";
const signedFutureRequest = resign(futureRequest, "opencoven:automation-request:v1", "alice");
add("11-request-not-yet-valid-at-dispatch", 11, "negative", "verify_dispatch", {
  request: signedFutureRequest,
  decision: r1Decision,
  snapshot: snapshot(signedFutureRequest),
}, { error: "request_not_yet_valid" });
const futureDecision = structuredClone(r1Decision);
futureDecision.validity.not_before = "2026-09-03T13:10:00Z";
const signedFutureDecision = resign(
  futureDecision,
  "opencoven:automation-decision:v1",
  "authority",
);
add("11-decision-not-yet-valid-at-dispatch", 11, "negative", "verify_dispatch", {
  request: r1,
  decision: signedFutureDecision,
  snapshot: snapshot(r1),
}, { error: "decision_not_yet_valid" });
const futureApproval = approval(r2, r2Decision, {
  issued_at: "2026-09-03T13:10:00Z",
});
add("11-approval-not-yet-valid-at-dispatch", 11, "negative", "verify_dispatch", {
  request: r2,
  decision: r2Decision,
  approval: futureApproval,
  events: lifecycle(futureApproval),
  snapshot: snapshot(r2),
}, { error: "approval_not_yet_valid" });
const revokedEvents = lifecycle(r2Approval, "revoke", {
  phase: "queued",
  disposition: "cancel_before_launch",
});
add("11-revoked-approval-cannot-dispatch", 11, "negative", "verify_dispatch", {
  request: r2,
  decision: r2Decision,
  approval: r2Approval,
  events: revokedEvents,
  snapshot: snapshot(r2),
}, { error: "approval_state_invalid" });
const consumedApprovalEvents = lifecycle(r2Approval, "consume", {
  phase: "dispatching",
  disposition: "launch_authorized",
});
add("11-consumed-approval-cannot-dispatch", 11, "negative", "verify_dispatch", {
  request: r2,
  decision: r2Decision,
  approval: r2Approval,
  events: consumedApprovalEvents,
  snapshot: snapshot(r2),
}, { error: "approval_state_invalid" });
add("17-lifecycle-summary-cannot-authorize-dispatch", 17, "negative", "verify_dispatch", {
  request: r2,
  decision: r2Decision,
  approval: r2Approval,
  events: [],
  lifecycle_summary: { state: "approved" },
  consumption_snapshot: consumptionSnapshot(
    r2,
    r2Decision,
    r2Approval,
    r2ApprovedEvents,
  ),
  snapshot: snapshot(r2),
}, { error: "approval_lifecycle_required" });
add("17-forged-lifecycle-head-cannot-dispatch", 17, "negative", "verify_dispatch", {
  request: r2,
  decision: r2Decision,
  approval: r2Approval,
  events: r2ApprovedEvents,
  consumption_snapshot: consumptionSnapshot(r2, r2Decision, r2Approval, r2ApprovedEvents, {
    approval_heads: [
      {
        approval_id: r2Approval.approval_id,
        head_event_digest: digest("f"),
        usage_count: 0,
      },
    ],
  }),
  snapshot: snapshot(r2),
}, { error: "approval_lifecycle_head_mismatch" });
for (const [type, target, phase, disposition] of [
  ["reject", "rejected", "not_started", "no_launch_rejected"],
  ["expire", "expired", "not_started", "no_launch_expired"],
  ["revoke", "revoked", "queued", "cancel_before_launch"],
]) {
  const requested = event(r2Approval, 1, null, "required", "requested", "request");
  const requestedDigest =
    `sha256:${canonicalDigest(requested, "opencoven:automation-approval-event:v1")}`;
  add(`11-requested-${target}`, 11, "positive", "lifecycle", {
    approval: r2Approval,
    events: [
      requested,
      event(
        r2Approval,
        2,
        requestedDigest,
        "requested",
        target,
        type,
        {
          phase,
          disposition,
          occurrenceDisposition:
            type === "reject"
              ? {
                  occurrence_id: r2Approval.occurrence_id,
                  run_id: r2Approval.run_id,
                  disposition: "rejected_no_launch",
                }
              : type === "expire"
                ? {
                    occurrence_id: r2Approval.occurrence_id,
                    run_id: r2Approval.run_id,
                    disposition: "expired_no_launch",
                  }
                : null,
        },
      ),
    ],
  });
}
for (const [type, target, error] of [
  ["reject", "rejected", "rejection_disposition_invalid"],
  ["expire", "expired", "expiration_disposition_invalid"],
]) {
  const requested = event(r2Approval, 1, null, "required", "requested", "request");
  const requestedDigest =
    `sha256:${canonicalDigest(requested, "opencoven:automation-approval-event:v1")}`;
  add(`11-${type}-requires-no-launch-disposition`, 11, "negative", "lifecycle", {
    approval: r2Approval,
    events: [
      requested,
      event(
        r2Approval,
        2,
        requestedDigest,
        "requested",
        target,
        type,
      ),
    ],
  }, { error });
}

for (const [name, field, value, error] of [
  ["definition", "definition", digest("8"), "approval_definition_changed"],
  ["action", "action", digest("8"), "approval_binding_mismatch"],
  ["runtime", "runtime", digest("8"), "approval_binding_mismatch"],
  ["familiar", "familiar", digest("8"), "approval_binding_mismatch"],
  ["occurrence", "occurrence", "occ:other", "approval_binding_mismatch"],
]) {
  const changed = structuredClone(r2Approval);
  if (field === "definition") changed.automation.definition_digest = value;
  if (field === "action") changed.action_digest = value;
  if (field === "runtime") changed.runtime_descriptor_digest = value;
  if (field === "familiar") changed.familiar_embodiment_digest = value;
  if (field === "occurrence") changed.occurrence_id = value;
  const changedApproval = resign(changed, "opencoven:automation-approval:v1");
  add(`12-changed-${name}-invalidates-approval`, 12, "negative", "verify_dispatch", {
    request: r2,
    decision: r2Decision,
    approval: changedApproval,
    events: lifecycle(changedApproval),
    snapshot: snapshot(r2),
  }, { error });
}
const changedCapabilities = structuredClone(r2Approval);
changedCapabilities.capabilities = ["analysis.read"];
changedCapabilities.runtime_capabilities = ["analysis.read", "state.mutate"];
const changedCapabilitiesApproval = resign(
  changedCapabilities,
  "opencoven:automation-approval:v1",
);
add("12-changed-capabilities-invalidates-approval", 12, "negative", "verify_dispatch", {
  request: r2,
  decision: r2Decision,
  approval: changedCapabilitiesApproval,
  events: lifecycle(changedCapabilitiesApproval),
  snapshot: snapshot(r2),
}, { error: "approval_capabilities_changed" });
const changedScopes = structuredClone(r2Approval);
changedScopes.scopes = [fsScope("state/other.sqlite")];
const changedScopesApproval = resign(changedScopes, "opencoven:automation-approval:v1");
add("12-changed-scopes-invalidates-approval", 12, "negative", "verify_dispatch", {
  request: r2,
  decision: r2Decision,
  approval: changedScopesApproval,
  events: lifecycle(changedScopesApproval),
  snapshot: snapshot(r2),
}, { error: "approval_scopes_changed" });
const changedRuntimeCapabilities = structuredClone(r2Approval);
changedRuntimeCapabilities.runtime_capabilities = ["analysis.read", "state.mutate"];
const changedRuntimeCapabilitiesApproval = resign(
  changedRuntimeCapabilities,
  "opencoven:automation-approval:v1",
);
add("12-changed-runtime-capabilities-invalidates-approval", 12, "negative", "verify_dispatch", {
  request: r2,
  decision: r2Decision,
  approval: changedRuntimeCapabilitiesApproval,
  events: lifecycle(changedRuntimeCapabilitiesApproval),
  snapshot: snapshot(r2),
}, { error: "approval_runtime_capabilities_changed" });
const impossibleRuntimeApproval = structuredClone(r2Approval);
impossibleRuntimeApproval.runtime_capabilities = ["analysis.read"];
add("12-approval-capability-absent-from-runtime", 12, "negative", "validate_approval", {
  approval: resign(impossibleRuntimeApproval, "opencoven:automation-approval:v1"),
}, { error: "approval_runtime_capability_missing" });

const previousReq = request({
  id: "previous-approval",
  previousApproval: digest("b"),
});
const previousPolicy = {
  ...policyFor(previousReq),
  previous_approval_digest: previousReq.previous_approval_digest,
};
add("12-previous-approval-exact-binding", 12, "positive", "verify_decision", {
  request: previousReq,
  policy: previousPolicy,
  decision: decision(previousReq, previousPolicy),
}, { outcome: "permit" });
const stalePreviousPolicy = policyFor(previousReq);
add("12-previous-approval-stale", 12, "negative", "verify_decision", {
  request: previousReq,
  policy: stalePreviousPolicy,
  decision: decision(previousReq, previousPolicy),
}, { error: "previous_approval_stale" });

for (const [name, override, error] of [
  ["principal", { principal_id: "principal:bob" }, "dispatch_principal_mismatch"],
  ["familiar", { familiar_id: "familiar:echo" }, "dispatch_familiar_mismatch"],
  ["project", { project_id: "project:other" }, "dispatch_project_mismatch"],
  ["workspace", { workspace_id: "workspace:other" }, "dispatch_workspace_mismatch"],
]) {
  add(`13-confused-deputy-${name}`, 13, "negative", "verify_dispatch", {
    request: r1,
    decision: r1Decision,
    snapshot: snapshot(r1, override),
  }, { error });
}

add("14-runtime-capability-downgrade", 14, "negative", "verify_dispatch", {
  request: r1,
  decision: r1Decision,
  snapshot: snapshot(r1, { runtime_capabilities: ["analysis.read"] }),
}, { error: "dispatch_runtime_downgrade" });
add("14-runtime-substitution", 14, "negative", "verify_dispatch", {
  request: r1,
  decision: r1Decision,
  snapshot: snapshot(r1, { runtime_descriptor_digest: digest("8") }),
}, { error: "dispatch_runtime_changed" });
add("14-runtime-id-substitution", 14, "negative", "verify_dispatch", {
  request: r1,
  decision: r1Decision,
  snapshot: snapshot(r1, { runtime_id: "runtime:substitute" }),
}, { error: "dispatch_runtime_changed" });

add("15-policy-changed-before-dispatch", 15, "negative", "verify_dispatch", {
  request: r1,
  decision: r1Decision,
  snapshot: snapshot(r1, { policy_digest: digest("8") }),
}, { error: "dispatch_policy_stale" });
add("15-manifest-changed-before-dispatch", 15, "negative", "verify_dispatch", {
  request: r1,
  decision: r1Decision,
  snapshot: snapshot(r1, { manifest_digest: digest("8") }),
}, { error: "dispatch_manifest_stale" });
add("15-stale-fence-before-dispatch", 15, "negative", "verify_dispatch", {
  request: r1,
  decision: r1Decision,
  snapshot: snapshot(r1, { fence_generation: 5 }),
}, { error: "dispatch_stale_fence" });
add("15-stale-consumption-snapshot", 15, "negative", "verify_dispatch", {
  request: r1,
  decision: r1Decision,
  approval: null,
  events: [],
  snapshot: snapshot(r1, { consumption_revision: 8 }),
}, { error: "consumption_snapshot_stale" });

for (const [name, phase, disposition] of [
  ["queued", "queued", "cancel_before_launch"],
  ["dispatching", "dispatching", "cancel_before_launch"],
  ["running", "running", "request_cooperative_cancel"],
]) {
  add(`16-revocation-${name}`, 16, "positive", "lifecycle", {
    approval: r2Approval,
    events: lifecycle(r2Approval, "revoke", { phase, disposition }),
  });
}
add("16-running-revocation-cannot-claim-rollback", 16, "negative", "lifecycle", {
  approval: r2Approval,
  events: lifecycle(r2Approval, "revoke", {
    phase: "running",
    disposition: "cancel_before_launch",
  }),
}, { error: "revocation_disposition_invalid" });

const tamperedDecision = structuredClone(r1Decision);
tamperedDecision.reason_codes = ["forged_reason"];
add("17-tampered-decision", 17, "negative", "verify_decision", {
  request: r1,
  policy: r1Policy,
  decision: tamperedDecision,
}, { error: "integrity_digest_mismatch" });
const forgedDecisionPayload = structuredClone(r1Decision);
forgedDecisionPayload.outcome = "requires_approval";
forgedDecisionPayload.approval_requirement = {
  profile: "human_per_run",
  recurring_allowed: false,
};
forgedDecisionPayload.reason_codes = ["forged_reason"];
const semanticallyForgedDecision = resign(
  forgedDecisionPayload,
  "opencoven:automation-decision:v1",
  "authority",
);
add("17-signed-but-forged-decision-semantics", 17, "negative", "verify_decision", {
  request: r1,
  policy: r1Policy,
  decision: semanticallyForgedDecision,
}, { error: "decision_semantic_mismatch" });
const tamperedApproval = structuredClone(r2Approval);
tamperedApproval.action_digest = digest("9");
add("17-tampered-approval", 17, "negative", "validate_approval", {
  approval: tamperedApproval,
}, { error: "integrity_digest_mismatch" });
const noncanonicalBase64Request = structuredClone(r1);
noncanonicalBase64Request.integrity.signature_b64 =
  `${noncanonicalBase64Request.integrity.signature_b64.slice(0, 12)} \n` +
  noncanonicalBase64Request.integrity.signature_b64.slice(12);
add("17-noncanonical-signature-base64", 17, "negative", "validate_request", {
  request: noncanonicalBase64Request,
}, { error: "integrity_signature_noncanonical" });
const forgedEvent = lifecycle(r2Approval)[0];
forgedEvent.actor = "client";
add("17-client-forged-lifecycle-state", 17, "negative", "lifecycle", {
  approval: r2Approval,
  events: [resign(forgedEvent, "opencoven:automation-approval-event:v1", "authority")],
}, { error: "lifecycle_actor_forged" });

function evidenceRead(which, requester, subject, signingRole = null) {
  const selectedSigner =
    signingRole ?? (requester === "principal:alice" ? "alice" : "bob");
  return signArtifact(
    {
      schema_version: "opencoven.automation-evidence-read/v1",
      read_id: `read:${which}`,
      requesting_principal_id: requester,
      authorization_proof_ref: `proof:read:${which}`,
      subject_principal_id: subject,
      automation_id: "automation:daily-report",
      maximum_sensitivity: "internal",
      retention_classes: ["authority_evidence_90d"],
      issued_at: "2026-09-03T13:00:00Z",
      expires_at: "2026-09-03T14:00:00Z",
      nonce: `read-nonce:${which}`,
    },
    "opencoven:automation-evidence-read:v1",
    signer[selectedSigner],
  );
}
const evidence = {
  principal_id: "principal:alice",
  automation_id: "automation:daily-report",
  sensitivity: "internal",
  retention: "authority_evidence_90d",
  payload_digest: digest("a"),
};
add("18-authorized-evidence-read", 18, "positive", "evidence_read", {
  read: evidenceRead("self", "principal:alice", "principal:alice"),
  evidence,
  now: "2026-09-03T13:05:00Z",
});
add("18-unauthorized-evidence-read", 18, "negative", "evidence_read", {
  read: evidenceRead("cross", "principal:bob", "principal:alice"),
  evidence,
  now: "2026-09-03T13:05:00Z",
}, { error: "evidence_read_unauthorized" });
add("18-auditor-cross-principal-evidence-read", 18, "positive", "evidence_read", {
  read: evidenceRead("auditor", "principal:auditor", "principal:alice", "auditor"),
  evidence,
  now: "2026-09-03T13:05:00Z",
});
add("18-evidence-read-not-yet-valid", 18, "negative", "evidence_read", {
  read: evidenceRead("early", "principal:alice", "principal:alice"),
  evidence,
  now: "2026-09-03T12:59:59Z",
}, { error: "evidence_read_not_yet_valid" });
add("18-evidence-read-expired", 18, "negative", "evidence_read", {
  read: evidenceRead("expired", "principal:alice", "principal:alice"),
  evidence,
  now: "2026-09-03T14:00:00Z",
}, { error: "evidence_read_expired" });

add("06-json-duplicate-key", 6, "negative", "strict_parse", {
  raw_json: "{\"schema_version\":\"one\",\"schema_version\":\"two\"}",
}, { error: "json_duplicate_key" });
add("06-json-unsafe-integer", 6, "negative", "strict_parse", {
  raw_json: "{\"attempt\":9007199254740992}",
}, { error: "json_unsafe_integer" });
add("06-json-unpaired-surrogate", 6, "negative", "strict_parse", {
  raw_json: "\"\\ud800\"",
}, { error: "json_non_ijson" });
const unknownVersion = structuredClone(r1);
unknownVersion.schema_version = "opencoven.automation-authorization-request/v2";
add("06-unknown-schema-version", 6, "negative", "validate_request", {
  request: resign(unknownVersion, "opencoven:automation-request:v1"),
}, { error: "schema_unknown_version" });
const unknownField = structuredClone(r1);
unknownField.client_receipt = "successful";
add("06-unknown-schema-field", 6, "negative", "validate_request", {
  request: resign(unknownField, "opencoven:automation-request:v1"),
}, { error: "schema_unknown_field" });
const underclassified = request({
  id: "underclassified",
  action: "external.publish",
  risk: "R1",
  capabilities: ["network.publish"],
  scopes: [networkScope()],
});
add("06-model-cannot-lower-risk", 6, "negative", "validate_request", {
  request: underclassified,
}, { error: "action_risk_underclassified" });

manifest.vectors.sort((left, right) => left.id.localeCompare(right.id));
writeFileSync(resolve(ROOT, "manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`generated ${manifest.vectors.length} vectors across 18 categories`);
