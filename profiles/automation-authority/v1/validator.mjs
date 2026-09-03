import {
  createHash,
  createPublicKey,
  sign as cryptoSign,
  verify as cryptoVerify,
} from "node:crypto";

const REQUEST_VERSION = "opencoven.automation-authorization-request/v1";
const DECISION_VERSION = "opencoven.automation-authorization-decision/v1";
const APPROVAL_VERSION = "opencoven.automation-approval/v1";
const EVENT_VERSION = "opencoven.automation-approval-event/v1";
const CONSUMPTION_VERSION = "opencoven.automation-consumption-snapshot/v1";
const PROPOSAL_VERSION = "opencoven.automation-proposal/v1";
const EVIDENCE_READ_VERSION = "opencoven.automation-evidence-read/v1";

const DOMAIN = Object.freeze({
  request: "opencoven:automation-request:v1",
  decision: "opencoven:automation-decision:v1",
  approval: "opencoven:automation-approval:v1",
  event: "opencoven:automation-approval-event:v1",
  consumption: "opencoven:automation-consumption-snapshot:v1",
  proposal: "opencoven:automation-proposal:v1",
  evidenceRead: "opencoven:automation-evidence-read:v1",
});

const CAPABILITY_RISK = new Map([
  ["analysis.read", 0],
  ["artifact.write", 1],
  ["state.mutate", 2],
  ["network.fetch", 3],
  ["network.publish", 3],
  ["credential.use", 3],
  ["evidence.read", 3],
  ["identity.mutate", 4],
  ["authority.admin", 4],
  ["release.publish", 4],
  ["resource.delete", 4],
]);

const ACTIONS = new Map([
  ["analysis.read", 0],
  ["artifact.create", 1],
  ["state.migrate", 2],
  ["network.fetch", 3],
  ["external.publish", 3],
  ["identity.mutate", 4],
  ["authority.change", 4],
  ["release.publish", 4],
  ["resource.delete", 4],
]);

const RISK_NUMBER = Object.freeze({ R0: 0, R1: 1, R2: 2, R3: 3, R4: 4 });
const SENSITIVITY = Object.freeze({ public: 0, internal: 1, confidential: 2, restricted: 3 });

export class AuthorityError extends Error {
  constructor(code, message, path = "$") {
    super(message);
    this.name = "AuthorityError";
    this.code = code;
    this.path = path;
  }
}

function fail(code, message, path = "$") {
  throw new AuthorityError(code, message, path);
}

class StrictJsonParser {
  constructor(text) {
    if (typeof text !== "string") {
      fail("json_invalid", "JSON input must be a string");
    }
    if (text.charCodeAt(0) === 0xfeff) {
      fail("json_non_ijson", "I-JSON input must not contain a BOM");
    }
    this.text = text;
    this.index = 0;
  }

  parse() {
    this.skipWhitespace();
    const value = this.parseValue("$");
    this.skipWhitespace();
    if (this.index !== this.text.length) {
      fail("json_invalid", "trailing data after JSON value");
    }
    validateIJsonValue(value);
    return value;
  }

  skipWhitespace() {
    while (" \t\r\n".includes(this.text[this.index] ?? "\0")) this.index += 1;
  }

  parseValue(path) {
    const char = this.text[this.index];
    if (char === "{") return this.parseObject(path);
    if (char === "[") return this.parseArray(path);
    if (char === '"') return this.parseString(path);
    if (char === "t" && this.consumeLiteral("true")) return true;
    if (char === "f" && this.consumeLiteral("false")) return false;
    if (char === "n" && this.consumeLiteral("null")) return null;
    if (char === "-" || (char >= "0" && char <= "9")) return this.parseNumber(path);
    fail("json_invalid", `unexpected token at byte ${this.index}`, path);
  }

  consumeLiteral(literal) {
    if (this.text.slice(this.index, this.index + literal.length) !== literal) return false;
    this.index += literal.length;
    return true;
  }

  parseObject(path) {
    this.index += 1;
    const value = {};
    const seen = new Set();
    this.skipWhitespace();
    if (this.text[this.index] === "}") {
      this.index += 1;
      return value;
    }
    while (true) {
      this.skipWhitespace();
      if (this.text[this.index] !== '"') fail("json_invalid", "object key must be a string", path);
      const key = this.parseString(path);
      if (seen.has(key)) {
        fail("json_duplicate_key", `duplicate object key ${JSON.stringify(key)}`, `${path}.${key}`);
      }
      seen.add(key);
      this.skipWhitespace();
      if (this.text[this.index] !== ":") fail("json_invalid", "missing colon after object key", path);
      this.index += 1;
      this.skipWhitespace();
      value[key] = this.parseValue(`${path}.${key}`);
      this.skipWhitespace();
      const separator = this.text[this.index];
      if (separator === "}") {
        this.index += 1;
        return value;
      }
      if (separator !== ",") fail("json_invalid", "missing comma in object", path);
      this.index += 1;
    }
  }

  parseArray(path) {
    this.index += 1;
    const value = [];
    this.skipWhitespace();
    if (this.text[this.index] === "]") {
      this.index += 1;
      return value;
    }
    let position = 0;
    while (true) {
      this.skipWhitespace();
      value.push(this.parseValue(`${path}[${position}]`));
      position += 1;
      this.skipWhitespace();
      const separator = this.text[this.index];
      if (separator === "]") {
        this.index += 1;
        return value;
      }
      if (separator !== ",") fail("json_invalid", "missing comma in array", path);
      this.index += 1;
    }
  }

  parseString(path) {
    const start = this.index;
    this.index += 1;
    while (this.index < this.text.length) {
      const char = this.text[this.index];
      if (char === '"') {
        this.index += 1;
        let value;
        try {
          value = JSON.parse(this.text.slice(start, this.index));
        } catch {
          fail("json_invalid", "invalid JSON string", path);
        }
        ensureUnicodeScalarString(value, path);
        return value;
      }
      if (char === "\\") {
        this.index += 2;
        continue;
      }
      if (char.charCodeAt(0) < 0x20) fail("json_invalid", "control character in string", path);
      this.index += 1;
    }
    fail("json_invalid", "unterminated JSON string", path);
  }

  parseNumber(path) {
    const tail = this.text.slice(this.index);
    const match = /^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?/.exec(tail);
    if (!match) fail("json_invalid", "invalid JSON number", path);
    const token = match[0];
    this.index += token.length;
    const number = Number(token);
    if (!Number.isFinite(number)) fail("json_non_ijson", "non-finite number is not I-JSON", path);
    if (!token.includes(".") && !/[eE]/.test(token) && !Number.isSafeInteger(number)) {
      fail("json_unsafe_integer", "integer exceeds the interoperable I-JSON range", path);
    }
    return number;
  }
}

export function strictParseJson(text) {
  return new StrictJsonParser(text).parse();
}

function ensureUnicodeScalarString(value, path) {
  for (let i = 0; i < value.length; i += 1) {
    const code = value.charCodeAt(i);
    if (code >= 0xd800 && code <= 0xdbff) {
      const next = value.charCodeAt(i + 1);
      if (!(next >= 0xdc00 && next <= 0xdfff)) {
        fail("json_non_ijson", "unpaired high surrogate is not I-JSON", path);
      }
      i += 1;
    } else if (code >= 0xdc00 && code <= 0xdfff) {
      fail("json_non_ijson", "unpaired low surrogate is not I-JSON", path);
    }
  }
}

function validateIJsonValue(value, path = "$") {
  if (typeof value === "string") {
    ensureUnicodeScalarString(value, path);
  } else if (typeof value === "number") {
    if (!Number.isFinite(value)) fail("json_non_ijson", "number must be finite", path);
    if (Number.isInteger(value) && !Number.isSafeInteger(value)) {
      fail("json_unsafe_integer", "integer exceeds the interoperable I-JSON range", path);
    }
  } else if (Array.isArray(value)) {
    value.forEach((item, index) => validateIJsonValue(item, `${path}[${index}]`));
  } else if (value && typeof value === "object") {
    for (const [key, item] of Object.entries(value)) {
      ensureUnicodeScalarString(key, `${path}.${key}`);
      validateIJsonValue(item, `${path}.${key}`);
    }
  }
}

function canonicalize(value) {
  validateIJsonValue(value);
  if (value === null || typeof value === "boolean" || typeof value === "number") {
    return JSON.stringify(value);
  }
  if (typeof value === "string") return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map(canonicalize).join(",")}]`;
  const keys = Object.keys(value).sort();
  return `{${keys.map((key) => `${JSON.stringify(key)}:${canonicalize(value[key])}`).join(",")}}`;
}

function unsignedArtifact(value) {
  if (!value || typeof value !== "object" || Array.isArray(value)) return value;
  const copy = { ...value };
  delete copy.integrity;
  return copy;
}

export function canonicalDigest(value, domain) {
  if (typeof domain !== "string" || domain.length === 0) {
    fail("integrity_domain_invalid", "digest domain must be a non-empty string");
  }
  return createHash("sha256")
    .update(domain, "utf8")
    .update(Buffer.from([0]))
    .update(canonicalize(unsignedArtifact(value)), "utf8")
    .digest("hex");
}

function normalizeKeyring(keyring) {
  if (keyring instanceof Map) return keyring;
  return new Map(Object.entries(keyring ?? {}));
}

function keyObject(record) {
  if (record.public_key) return record.public_key;
  if (record.public_key_pem) return createPublicKey(record.public_key_pem);
  fail("integrity_key_unknown", "key record does not contain a public key");
}

export function verifySignedArtifact(value, domain, keyring, expectedRole = undefined) {
  requireObject(value, "$");
  const integrity = value.integrity;
  requireObject(integrity, "$.integrity");
  closed(integrity, ["alg", "key_id", "signed_digest", "signature_b64"], "$.integrity");
  exact(integrity.alg, "ed25519", "integrity_algorithm_unknown", "$.integrity.alg");
  requireString(integrity.key_id, "$.integrity.key_id");
  requireDigestHex(integrity.signed_digest, "$.integrity.signed_digest", false);
  requireString(integrity.signature_b64, "$.integrity.signature_b64");
  if (!/^[A-Za-z0-9+/]{86}==$/.test(integrity.signature_b64)) {
    fail(
      "integrity_signature_noncanonical",
      "signature_b64 must be canonical padded Base64",
      "$.integrity.signature_b64",
    );
  }

  const keys = normalizeKeyring(keyring);
  const record = keys.get(integrity.key_id);
  if (!record) fail("integrity_key_unknown", `unknown key ${integrity.key_id}`, "$.integrity.key_id");
  if (expectedRole && record.role !== expectedRole) {
    fail("integrity_role_mismatch", `key ${integrity.key_id} is not a ${expectedRole} key`);
  }
  const digest = canonicalDigest(value, domain);
  if (digest !== integrity.signed_digest) {
    fail("integrity_digest_mismatch", "signed digest does not match canonical artifact bytes");
  }
  const signature = Buffer.from(integrity.signature_b64, "base64");
  if (signature.toString("base64") !== integrity.signature_b64) {
    fail("integrity_signature_noncanonical", "signature Base64 does not round-trip canonically");
  }
  if (
    signature.length !== 64 ||
    !cryptoVerify(null, Buffer.from(digest, "hex"), keyObject(record), signature)
  ) {
    fail("integrity_signature_invalid", "Ed25519 signature verification failed");
  }
  return record;
}

export function signArtifact(value, domain, signer) {
  if (!signer?.key_id || !signer?.private_key) {
    fail("integrity_signer_required", "a key_id and Ed25519 private_key are required");
  }
  const digest = canonicalDigest(value, domain);
  return {
    ...value,
    integrity: {
      alg: "ed25519",
      key_id: signer.key_id,
      signed_digest: digest,
      signature_b64: cryptoSign(null, Buffer.from(digest, "hex"), signer.private_key).toString(
        "base64",
      ),
    },
  };
}

function authoritySigner(keyring) {
  for (const [key_id, record] of normalizeKeyring(keyring)) {
    if (record.role === "threads_authority" && record.private_key) {
      return { key_id, private_key: record.private_key };
    }
  }
  fail("integrity_signer_required", "reference evaluator requires a Threads authority signing key");
}

function requireObject(value, path) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    fail("schema_type", "expected object", path);
  }
}

function requireArray(value, path, { nonempty = false, unique = false } = {}) {
  if (!Array.isArray(value)) fail("schema_type", "expected array", path);
  if (nonempty && value.length === 0) fail("schema_min_items", "array must not be empty", path);
  if (unique && new Set(value).size !== value.length) {
    fail("schema_duplicate_item", "array items must be unique", path);
  }
}

function requireString(value, path, { nonempty = true } = {}) {
  if (typeof value !== "string" || (nonempty && value.length === 0)) {
    fail("schema_type", "expected non-empty string", path);
  }
  ensureUnicodeScalarString(value, path);
}

function requireInteger(value, path, min = 0) {
  if (!Number.isSafeInteger(value) || value < min) {
    fail("schema_integer", `expected safe integer >= ${min}`, path);
  }
}

function requireBoolean(value, path) {
  if (typeof value !== "boolean") fail("schema_type", "expected boolean", path);
}

function requireEnum(value, allowed, path) {
  if (!allowed.includes(value)) {
    fail("schema_enum", `expected one of ${allowed.join(", ")}`, path);
  }
}

function requireTimestamp(value, path) {
  requireString(value, path);
  if (!/^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?Z$/.test(value) || Number.isNaN(Date.parse(value))) {
    fail("schema_timestamp", "expected an RFC 3339 UTC timestamp", path);
  }
}

function requireDigestHex(value, path, prefixed = true) {
  requireString(value, path);
  const pattern = prefixed ? /^sha256:[0-9a-f]{64}$/ : /^[0-9a-f]{64}$/;
  if (!pattern.test(value)) fail("schema_digest", "expected lowercase SHA-256 digest", path);
}

function closed(value, allowed, path) {
  requireObject(value, path);
  const allowedSet = new Set(allowed);
  for (const key of Object.keys(value)) {
    if (!allowedSet.has(key)) fail("schema_unknown_field", `unknown field ${key}`, `${path}.${key}`);
  }
}

function required(value, names, path) {
  for (const name of names) {
    if (!Object.hasOwn(value, name)) fail("schema_required", `missing required field ${name}`, path);
  }
}

function exact(actual, expected, code, path) {
  if (actual !== expected) fail(code, `expected ${expected}`, path);
}

function validateIntegrityShape(integrity, path = "$.integrity") {
  requireObject(integrity, path);
  required(integrity, ["alg", "key_id", "signed_digest", "signature_b64"], path);
  closed(integrity, ["alg", "key_id", "signed_digest", "signature_b64"], path);
}

function validateCapabilities(capabilities, path) {
  requireArray(capabilities, path, { nonempty: true, unique: true });
  for (const [index, capability] of capabilities.entries()) {
    requireString(capability, `${path}[${index}]`);
    if (!CAPABILITY_RISK.has(capability)) {
      fail("capability_unknown", `unknown capability ${capability}`, `${path}[${index}]`);
    }
  }
}

function normalizedPath(path) {
  return path.replaceAll("\\", "/");
}

function validateScope(scope, path) {
  requireObject(scope, path);
  requireString(scope.kind, `${path}.kind`);
  if (scope.kind === "filesystem") {
    required(scope, ["kind", "root", "path", "access", "recursive"], path);
    closed(scope, ["kind", "root", "path", "access", "recursive"], path);
    requireEnum(scope.root, ["project", "workspace"], `${path}.root`);
    requireString(scope.path, `${path}.path`);
    requireEnum(scope.access, ["read", "write"], `${path}.access`);
    requireBoolean(scope.recursive, `${path}.recursive`);
    const candidate = normalizedPath(scope.path);
    if (
      candidate === "." ||
      candidate === "/" ||
      candidate.includes("*") ||
      candidate.includes("..") ||
      candidate.startsWith("/") ||
      /^[A-Za-z]:\//.test(candidate) ||
      (scope.recursive && candidate.split("/").filter(Boolean).length < 2)
    ) {
      fail("scope_too_broad", "filesystem scope must be a narrow relative path", `${path}.path`);
    }
  } else if (scope.kind === "network") {
    required(scope, ["kind", "scheme", "host", "port", "path_prefix", "methods"], path);
    closed(scope, ["kind", "scheme", "host", "port", "path_prefix", "methods"], path);
    exact(scope.scheme, "https", "scope_insecure_network", `${path}.scheme`);
    requireString(scope.host, `${path}.host`);
    if (
      scope.host === "localhost" ||
      scope.host === "0.0.0.0" ||
      scope.host.includes("*") ||
      !/^[a-z0-9.-]+$/.test(scope.host)
    ) {
      fail("scope_too_broad", "network host must be an exact non-local DNS name", `${path}.host`);
    }
    requireInteger(scope.port, `${path}.port`, 1);
    if (scope.port > 65535) fail("schema_integer", "port must be <= 65535", `${path}.port`);
    requireString(scope.path_prefix, `${path}.path_prefix`);
    if (!scope.path_prefix.startsWith("/") || scope.path_prefix === "/") {
      fail("scope_too_broad", "network path_prefix must be narrower than /", `${path}.path_prefix`);
    }
    requireArray(scope.methods, `${path}.methods`, { nonempty: true, unique: true });
    for (const method of scope.methods) requireEnum(method, ["GET", "POST", "PUT"], `${path}.methods`);
  } else if (scope.kind === "credential") {
    required(scope, ["kind", "credential_ref", "audience", "operations"], path);
    closed(scope, ["kind", "credential_ref", "audience", "operations"], path);
    requireString(scope.credential_ref, `${path}.credential_ref`);
    requireString(scope.audience, `${path}.audience`);
    if (scope.audience.includes("*")) fail("scope_too_broad", "credential audience must be exact", path);
    requireArray(scope.operations, `${path}.operations`, { nonempty: true, unique: true });
    scope.operations.forEach((operation, index) =>
      requireString(operation, `${path}.operations[${index}]`),
    );
  } else if (scope.kind === "evidence") {
    required(scope, ["kind", "principal_id", "automation_id", "retention_classes"], path);
    closed(scope, ["kind", "principal_id", "automation_id", "retention_classes"], path);
    requireString(scope.principal_id, `${path}.principal_id`);
    requireString(scope.automation_id, `${path}.automation_id`);
    requireArray(scope.retention_classes, `${path}.retention_classes`, {
      nonempty: true,
      unique: true,
    });
    scope.retention_classes.forEach((entry, index) =>
      requireEnum(
        entry,
        ["ephemeral_24h", "authority_evidence_90d", "authority_evidence_1y"],
        `${path}.retention_classes[${index}]`,
      ),
    );
  } else {
    fail("scope_kind_unknown", `unknown scope kind ${scope.kind}`, `${path}.kind`);
  }
}

function validateScopes(scopes, path = "$.scopes") {
  requireArray(scopes, path, { nonempty: true });
  const canonical = new Set();
  scopes.forEach((scope, index) => {
    validateScope(scope, `${path}[${index}]`);
    const encoded = canonicalize(scope);
    if (canonical.has(encoded)) fail("scope_duplicate", "duplicate scope", `${path}[${index}]`);
    canonical.add(encoded);
  });
}

function validateCapabilityScopePairing(capabilities, scopes) {
  const kinds = new Set(scopes.map((scope) => scope.kind));
  const requiredKinds = new Map([
    ["credential.use", "credential"],
    ["network.fetch", "network"],
    ["network.publish", "network"],
    ["evidence.read", "evidence"],
  ]);
  for (const capability of capabilities) {
    const requiredKind = requiredKinds.get(capability);
    if (requiredKind && !kinds.has(requiredKind)) {
      fail(
        "capability_scope_mismatch",
        `${capability} requires an explicit ${requiredKind} scope`,
        "$.scopes",
      );
    }
  }
}

function validateRuntime(runtime, path) {
  requireObject(runtime, path);
  required(runtime, ["id", "descriptor_digest", "capabilities"], path);
  closed(runtime, ["id", "descriptor_digest", "capabilities"], path);
  requireString(runtime.id, `${path}.id`);
  requireDigestHex(runtime.descriptor_digest, `${path}.descriptor_digest`);
  validateCapabilities(runtime.capabilities, `${path}.capabilities`);
}

export function validateAuthorizationRequest(value, { keyring } = {}) {
  requireObject(value, "$");
  required(
    value,
    [
      "schema_version",
      "request_id",
      "principal",
      "replay",
      "familiar",
      "automation",
      "execution",
      "action",
      "requested_capabilities",
      "scopes",
      "context",
      "versions",
      "previous_approval_digest",
      "conditions",
      "data",
      "integrity",
    ],
    "$",
  );
  closed(
    value,
    [
      "schema_version",
      "request_id",
      "principal",
      "replay",
      "familiar",
      "automation",
      "execution",
      "action",
      "requested_capabilities",
      "scopes",
      "context",
      "versions",
      "previous_approval_digest",
      "conditions",
      "data",
      "integrity",
    ],
    "$",
  );
  exact(value.schema_version, REQUEST_VERSION, "schema_unknown_version", "$.schema_version");
  requireString(value.request_id, "$.request_id");

  required(value.principal, ["id", "authorization_proof_ref"], "$.principal");
  closed(value.principal, ["id", "authorization_proof_ref"], "$.principal");
  requireString(value.principal.id, "$.principal.id");
  requireString(value.principal.authorization_proof_ref, "$.principal.authorization_proof_ref");

  required(value.replay, ["nonce", "adoption_key", "issued_at", "expires_at"], "$.replay");
  closed(value.replay, ["nonce", "adoption_key", "issued_at", "expires_at"], "$.replay");
  requireString(value.replay.nonce, "$.replay.nonce");
  requireString(value.replay.adoption_key, "$.replay.adoption_key");
  requireTimestamp(value.replay.issued_at, "$.replay.issued_at");
  requireTimestamp(value.replay.expires_at, "$.replay.expires_at");
  if (Date.parse(value.replay.expires_at) <= Date.parse(value.replay.issued_at)) {
    fail("request_invalid_interval", "request expiry must be after issue time", "$.replay");
  }

  required(value.familiar, ["id", "embodiment_digest"], "$.familiar");
  closed(value.familiar, ["id", "embodiment_digest"], "$.familiar");
  requireString(value.familiar.id, "$.familiar.id");
  requireDigestHex(value.familiar.embodiment_digest, "$.familiar.embodiment_digest");

  required(
    value.automation,
    ["id", "definition_revision", "definition_digest"],
    "$.automation",
  );
  closed(value.automation, ["id", "definition_revision", "definition_digest"], "$.automation");
  requireString(value.automation.id, "$.automation.id");
  requireInteger(value.automation.definition_revision, "$.automation.definition_revision", 1);
  requireDigestHex(value.automation.definition_digest, "$.automation.definition_digest");

  required(
    value.execution,
    ["occurrence_id", "run_id", "attempt", "fence_generation"],
    "$.execution",
  );
  closed(
    value.execution,
    ["occurrence_id", "run_id", "attempt", "fence_generation"],
    "$.execution",
  );
  requireString(value.execution.occurrence_id, "$.execution.occurrence_id");
  requireString(value.execution.run_id, "$.execution.run_id");
  requireInteger(value.execution.attempt, "$.execution.attempt", 1);
  requireInteger(value.execution.fence_generation, "$.execution.fence_generation", 1);

  required(value.action, ["type", "digest", "risk_class", "proposal_safe"], "$.action");
  closed(value.action, ["type", "digest", "risk_class", "proposal_safe"], "$.action");
  requireString(value.action.type, "$.action.type");
  if (!ACTIONS.has(value.action.type)) {
    fail("action_unknown", `unknown action ${value.action.type}`, "$.action.type");
  }
  requireDigestHex(value.action.digest, "$.action.digest");
  requireEnum(value.action.risk_class, Object.keys(RISK_NUMBER), "$.action.risk_class");
  requireBoolean(value.action.proposal_safe, "$.action.proposal_safe");
  const actionFloor = ACTIONS.get(value.action.type);
  if (RISK_NUMBER[value.action.risk_class] < actionFloor) {
    fail("action_risk_underclassified", "declared risk is lower than the action floor", "$.action");
  }

  validateCapabilities(value.requested_capabilities, "$.requested_capabilities");
  const capabilityFloor = Math.max(
    ...value.requested_capabilities.map((capability) => CAPABILITY_RISK.get(capability)),
  );
  if (RISK_NUMBER[value.action.risk_class] < capabilityFloor) {
    fail(
      "capability_risk_underclassified",
      "declared risk is lower than a requested capability floor",
      "$.requested_capabilities",
    );
  }
  validateScopes(value.scopes);
  validateCapabilityScopePairing(value.requested_capabilities, value.scopes);

  required(value.context, ["project_id", "workspace_id", "runtime"], "$.context");
  closed(value.context, ["project_id", "workspace_id", "runtime"], "$.context");
  requireString(value.context.project_id, "$.context.project_id");
  requireString(value.context.workspace_id, "$.context.workspace_id");
  validateRuntime(value.context.runtime, "$.context.runtime");

  required(
    value.versions,
    ["profile", "policy", "policy_digest", "manifest", "manifest_digest"],
    "$.versions",
  );
  closed(
    value.versions,
    ["profile", "policy", "policy_digest", "manifest", "manifest_digest"],
    "$.versions",
  );
  exact(value.versions.profile, "1.0.0", "profile_version_unknown", "$.versions.profile");
  requireString(value.versions.policy, "$.versions.policy");
  requireDigestHex(value.versions.policy_digest, "$.versions.policy_digest");
  requireString(value.versions.manifest, "$.versions.manifest");
  requireDigestHex(value.versions.manifest_digest, "$.versions.manifest_digest");
  if (value.previous_approval_digest !== null) {
    requireDigestHex(value.previous_approval_digest, "$.previous_approval_digest");
  }
  requireArray(value.conditions, "$.conditions", { unique: true });
  value.conditions.forEach((condition, index) =>
    requireString(condition, `$.conditions[${index}]`),
  );
  if (
    value.action.risk_class === "R2" &&
    (!value.conditions.includes("deterministic_validation") ||
      !value.conditions.includes("rollback_plan"))
  ) {
    fail(
      "r2_safeguards_missing",
      "R2 operations require deterministic_validation and rollback_plan conditions",
      "$.conditions",
    );
  }
  required(value.data, ["sensitivity", "retention"], "$.data");
  closed(value.data, ["sensitivity", "retention"], "$.data");
  requireEnum(value.data.sensitivity, Object.keys(SENSITIVITY), "$.data.sensitivity");
  requireEnum(
    value.data.retention,
    ["ephemeral_24h", "authority_evidence_90d", "authority_evidence_1y"],
    "$.data.retention",
  );
  validateIntegrityShape(value.integrity);
  const key = verifySignedArtifact(value, DOMAIN.request, keyring);
  if (key.principal_id && key.principal_id !== value.principal.id) {
    fail("principal_key_mismatch", "request signing key belongs to another principal");
  }
  if (key.role !== "principal") {
    fail("integrity_role_mismatch", "request must be authenticated by a principal key");
  }
  return value;
}

export function adoptAuthorizationRequest(value, state = null, { keyring, now } = {}) {
  validateAuthorizationRequest(value, { keyring });
  if (now && Date.parse(now) < Date.parse(value.replay.issued_at)) {
    fail("request_not_yet_valid", "request has not reached its issue time");
  }
  if (now && Date.parse(now) >= Date.parse(value.replay.expires_at)) {
    fail("request_expired", "request has expired");
  }
  const current = state ?? { nonces: [], adoption_keys: [], request_digests: [] };
  const digest = `sha256:${canonicalDigest(value, DOMAIN.request)}`;
  if (
    current.nonces.includes(value.replay.nonce) ||
    current.adoption_keys.includes(value.replay.adoption_key) ||
    current.request_digests.includes(digest)
  ) {
    fail("request_replayed", "request nonce, adoption key, or digest was already adopted");
  }
  return {
    nonces: [...current.nonces, value.replay.nonce],
    adoption_keys: [...current.adoption_keys, value.replay.adoption_key],
    request_digests: [...current.request_digests, digest],
  };
}

function grantMatchesRequest(grant, request, now) {
  return (
    grant.principal_id === request.principal.id &&
    grant.familiar_id === request.familiar.id &&
    grant.familiar_embodiment_digest === request.familiar.embodiment_digest &&
    grant.automation_id === request.automation.id &&
    grant.definition_revision === request.automation.definition_revision &&
    grant.definition_digest === request.automation.definition_digest &&
    grant.action_type === request.action.type &&
    grant.action_digest === request.action.digest &&
    grant.project_id === request.context.project_id &&
    grant.workspace_id === request.context.workspace_id &&
    grant.runtime_id === request.context.runtime.id &&
    grant.runtime_descriptor_digest === request.context.runtime.descriptor_digest &&
    canonicalize([...grant.runtime_capabilities].sort()) ===
      canonicalize([...request.context.runtime.capabilities].sort()) &&
    grant.risk_classes.includes(request.action.risk_class) &&
    Date.parse(grant.expires_at) > Date.parse(now) &&
    grant.uses < grant.max_uses
  );
}

function validateRecurringGrant(grant, index) {
  const path = `$.policy_snapshot.recurring_grants[${index}]`;
  const fields = [
    "grant_id",
    "principal_id",
    "familiar_id",
    "familiar_embodiment_digest",
    "automation_id",
    "definition_revision",
    "definition_digest",
    "action_type",
    "action_digest",
    "project_id",
    "workspace_id",
    "runtime_id",
    "runtime_descriptor_digest",
    "runtime_capabilities",
    "risk_classes",
    "capabilities",
    "scopes",
    "expires_at",
    "max_uses",
    "uses",
  ];
  required(grant, fields, path);
  closed(grant, fields, path);
  for (const name of [
    "grant_id",
    "principal_id",
    "familiar_id",
    "automation_id",
    "action_type",
    "project_id",
    "workspace_id",
    "runtime_id",
  ]) {
    requireString(grant[name], `${path}.${name}`);
  }
  requireInteger(grant.definition_revision, `${path}.definition_revision`, 1);
  requireDigestHex(
    grant.familiar_embodiment_digest,
    `${path}.familiar_embodiment_digest`,
  );
  requireDigestHex(grant.definition_digest, `${path}.definition_digest`);
  requireDigestHex(grant.action_digest, `${path}.action_digest`);
  requireDigestHex(grant.runtime_descriptor_digest, `${path}.runtime_descriptor_digest`);
  validateCapabilities(grant.runtime_capabilities, `${path}.runtime_capabilities`);
  requireArray(grant.risk_classes, `${path}.risk_classes`, { nonempty: true, unique: true });
  grant.risk_classes.forEach((risk, riskIndex) =>
    requireEnum(risk, Object.keys(RISK_NUMBER), `${path}.risk_classes[${riskIndex}]`),
  );
  validateCapabilities(grant.capabilities, `${path}.capabilities`);
  validateScopes(grant.scopes, `${path}.scopes`);
  requireTimestamp(grant.expires_at, `${path}.expires_at`);
  requireInteger(grant.max_uses, `${path}.max_uses`, 1);
  requireInteger(grant.uses, `${path}.uses`, 0);
  if (grant.uses > grant.max_uses) {
    fail("recurring_grant_usage_invalid", "recurring grant usage exceeds its bound", path);
  }
}

function scopeKey(scope) {
  return canonicalize(scope);
}

function decisionBase(request, snapshot, outcome, grants, denied, degraded, reasonCodes) {
  const requestDigest = canonicalDigest(request, DOMAIN.request);
  return {
    schema_version: DECISION_VERSION,
    decision_id: `decision:${request.request_id}:${requestDigest.slice(0, 16)}`,
    request_id: request.request_id,
    request_digest: `sha256:${requestDigest}`,
    correlation: {
      occurrence_id: request.execution.occurrence_id,
      run_id: request.execution.run_id,
      attempt: request.execution.attempt,
      fence_generation: request.execution.fence_generation,
    },
    bindings: {
      principal_id: request.principal.id,
      familiar_id: request.familiar.id,
      familiar_embodiment_digest: request.familiar.embodiment_digest,
      automation_id: request.automation.id,
      definition_revision: request.automation.definition_revision,
      definition_digest: request.automation.definition_digest,
      action_digest: request.action.digest,
      project_id: request.context.project_id,
      workspace_id: request.context.workspace_id,
      runtime_id: request.context.runtime.id,
      runtime_descriptor_digest: request.context.runtime.descriptor_digest,
      runtime_capabilities: [...request.context.runtime.capabilities].sort(),
      previous_approval_digest: request.previous_approval_digest,
    },
    outcome,
    granted_capabilities: grants.sort(),
    denied_capabilities: denied.sort((a, b) => a.capability.localeCompare(b.capability)),
    degraded_capabilities: degraded.sort(),
    scopes: snapshot.scopes,
    validity: {
      not_before: snapshot.now,
      not_after: request.replay.expires_at,
    },
    approval_requirement:
      outcome === "requires_approval"
        ? {
            profile: request.action.risk_class === "R4" ? "protected_owner_per_run" : "human_per_run",
            recurring_allowed:
              request.action.risk_class === "R2" &&
              snapshot.recurring_approval_allowed === true,
          }
        : null,
    versions: { ...request.versions },
    reason_codes: reasonCodes,
    producer: {
      id: "coven-threads",
      verifier_profile: "automation-authority/1.0.0",
    },
    issued_at: snapshot.now,
    recorded_at: snapshot.now,
    replay: {
      dispatches: 1,
      consumption_required: outcome === "permit" || outcome === "requires_approval",
    },
    privacy: request.data,
  };
}

export function evaluateAuthorization(request, snapshot, { keyring, unsigned = false } = {}) {
  validateAuthorizationRequest(request, { keyring });
  requireObject(snapshot, "$.policy_snapshot");
  required(
    snapshot,
    [
      "now",
      "policy",
      "policy_digest",
      "manifest",
      "manifest_digest",
      "recurring_grants",
      "protected_owner_approval",
      "recurring_approval_allowed",
    ],
    "$.policy_snapshot",
  );
  closed(
    snapshot,
    [
      "now",
      "policy",
      "policy_digest",
      "manifest",
      "manifest_digest",
      "recurring_grants",
      "protected_owner_approval",
      "recurring_approval_allowed",
      "previous_approval_digest",
    ],
    "$.policy_snapshot",
  );
  requireTimestamp(snapshot.now, "$.policy_snapshot.now");
  requireString(snapshot.policy, "$.policy_snapshot.policy");
  requireDigestHex(snapshot.policy_digest, "$.policy_snapshot.policy_digest");
  requireString(snapshot.manifest, "$.policy_snapshot.manifest");
  requireDigestHex(snapshot.manifest_digest, "$.policy_snapshot.manifest_digest");
  requireBoolean(snapshot.protected_owner_approval, "$.policy_snapshot.protected_owner_approval");
  requireBoolean(
    snapshot.recurring_approval_allowed,
    "$.policy_snapshot.recurring_approval_allowed",
  );
  if (snapshot.previous_approval_digest !== undefined) {
    requireDigestHex(
      snapshot.previous_approval_digest,
      "$.policy_snapshot.previous_approval_digest",
    );
  }
  if (Date.parse(snapshot.now) < Date.parse(request.replay.issued_at)) {
    fail("request_not_yet_valid", "request issue time is in the future");
  }
  if (Date.parse(snapshot.now) >= Date.parse(request.replay.expires_at)) {
    fail("request_expired", "request has expired");
  }
  if (
    snapshot.policy !== request.versions.policy ||
    snapshot.policy_digest !== request.versions.policy_digest
  ) {
    fail("policy_stale", "request policy snapshot is stale");
  }
  if (
    snapshot.manifest !== request.versions.manifest ||
    snapshot.manifest_digest !== request.versions.manifest_digest
  ) {
    fail("manifest_stale", "request manifest snapshot is stale");
  }
  if (
    request.previous_approval_digest !== null &&
    snapshot.previous_approval_digest !== request.previous_approval_digest
  ) {
    fail("previous_approval_stale", "previous approval evidence is absent or changed");
  }
  request.context.runtime.capabilities.forEach((capability) => {
    if (!CAPABILITY_RISK.has(capability)) fail("runtime_capability_unknown", capability);
  });
  requireArray(snapshot.recurring_grants, "$.policy_snapshot.recurring_grants");
  snapshot.recurring_grants.forEach(validateRecurringGrant);
  const requestedRisk = RISK_NUMBER[request.action.risk_class];
  const runtimeCapabilities = new Set(request.context.runtime.capabilities);
  const approvalCandidate =
    request.conditions.includes("automation_imported") ||
    request.conditions.includes("automation_new") ||
    requestedRisk === 2 ||
    (requestedRisk === 3 && !request.action.proposal_safe) ||
    (requestedRisk === 4 && snapshot.protected_owner_approval === true);
  if (
    approvalCandidate &&
    request.requested_capabilities.some((capability) => !runtimeCapabilities.has(capability))
  ) {
    fail(
      "runtime_capability_missing",
      "approval cannot authorize a capability absent from the bound runtime",
    );
  }

  const matchingGrant = (snapshot.recurring_grants ?? []).find((grant) =>
    grantMatchesRequest(grant, request, snapshot.now),
  );
  const allowedCapabilities = new Set(matchingGrant?.capabilities ?? []);
  const granted = request.requested_capabilities.filter(
    (capability) => allowedCapabilities.has(capability) && runtimeCapabilities.has(capability),
  );
  const denied = request.requested_capabilities
    .filter((capability) => !granted.includes(capability))
    .map((capability) => ({ capability, reason_code: "capability_not_granted" }));

  const grantScopes = new Set((matchingGrant?.scopes ?? []).map(scopeKey));
  const scopes = request.scopes.filter((scope) => grantScopes.has(scopeKey(scope)));
  let outcome;
  let degraded = [];
  let reasonCodes = [];

  if (
    request.conditions.includes("automation_imported") ||
    request.conditions.includes("automation_new")
  ) {
    outcome = "requires_approval";
    reasonCodes = ["new_or_imported_automation_review_required"];
  } else if (requestedRisk <= 1 && granted.length > 0 && scopes.length > 0) {
    outcome = "permit";
    reasonCodes = ["bounded_recurring_grant"];
  } else if (requestedRisk === 2) {
    outcome = "requires_approval";
    reasonCodes = ["risk_r2_per_run_approval"];
  } else if (requestedRisk === 3 && request.action.proposal_safe) {
    outcome = "degrade_to_proposal";
    degraded = [...request.requested_capabilities];
    reasonCodes = ["risk_r3_proposal_only"];
  } else if (requestedRisk === 3) {
    outcome = "requires_approval";
    reasonCodes = ["risk_r3_per_run_approval"];
  } else if (requestedRisk === 4 && snapshot.protected_owner_approval === true) {
    outcome = "requires_approval";
    reasonCodes = ["risk_r4_protected_owner_per_run"];
  } else if (requestedRisk === 4) {
    outcome = "reject";
    reasonCodes = ["risk_r4_protected_owner_missing"];
  } else {
    outcome = request.action.proposal_safe ? "degrade_to_proposal" : "reject";
    degraded = [...request.requested_capabilities];
    reasonCodes = ["no_matching_authority"];
  }

  const payload = decisionBase(
    request,
    { ...snapshot, scopes },
    outcome,
    granted,
    denied,
    degraded,
    reasonCodes,
  );
  if (unsigned) return payload;
  return signArtifact(payload, DOMAIN.decision, authoritySigner(keyring));
}

export function validateDecision(value, { keyring } = {}) {
  requireObject(value, "$.decision");
  required(
    value,
    [
      "schema_version",
      "decision_id",
      "request_id",
      "request_digest",
      "correlation",
      "bindings",
      "outcome",
      "granted_capabilities",
      "denied_capabilities",
      "degraded_capabilities",
      "scopes",
      "validity",
      "approval_requirement",
      "versions",
      "reason_codes",
      "producer",
      "issued_at",
      "recorded_at",
      "replay",
      "privacy",
      "integrity",
    ],
    "$.decision",
  );
  closed(
    value,
    [
      "schema_version",
      "decision_id",
      "request_id",
      "request_digest",
      "correlation",
      "bindings",
      "outcome",
      "granted_capabilities",
      "denied_capabilities",
      "degraded_capabilities",
      "scopes",
      "validity",
      "approval_requirement",
      "versions",
      "reason_codes",
      "producer",
      "issued_at",
      "recorded_at",
      "replay",
      "privacy",
      "integrity",
    ],
    "$.decision",
  );
  exact(value.schema_version, DECISION_VERSION, "schema_unknown_version", "$.decision.schema_version");
  requireString(value.decision_id, "$.decision.decision_id");
  requireString(value.request_id, "$.decision.request_id");
  requireDigestHex(value.request_digest, "$.decision.request_digest");
  required(
    value.correlation,
    ["occurrence_id", "run_id", "attempt", "fence_generation"],
    "$.decision.correlation",
  );
  closed(
    value.correlation,
    ["occurrence_id", "run_id", "attempt", "fence_generation"],
    "$.decision.correlation",
  );
  requireString(value.correlation.occurrence_id, "$.decision.correlation.occurrence_id");
  requireString(value.correlation.run_id, "$.decision.correlation.run_id");
  requireInteger(value.correlation.attempt, "$.decision.correlation.attempt", 1);
  requireInteger(value.correlation.fence_generation, "$.decision.correlation.fence_generation", 1);
  required(
    value.bindings,
    [
      "principal_id",
      "familiar_id",
      "familiar_embodiment_digest",
      "automation_id",
      "definition_revision",
      "definition_digest",
      "action_digest",
      "project_id",
      "workspace_id",
      "runtime_id",
      "runtime_descriptor_digest",
      "runtime_capabilities",
      "previous_approval_digest",
    ],
    "$.decision.bindings",
  );
  closed(
    value.bindings,
    [
      "principal_id",
      "familiar_id",
      "familiar_embodiment_digest",
      "automation_id",
      "definition_revision",
      "definition_digest",
      "action_digest",
      "project_id",
      "workspace_id",
      "runtime_id",
      "runtime_descriptor_digest",
      "runtime_capabilities",
      "previous_approval_digest",
    ],
    "$.decision.bindings",
  );
  for (const name of [
    "principal_id",
    "familiar_id",
    "automation_id",
    "project_id",
    "workspace_id",
    "runtime_id",
  ]) {
    requireString(value.bindings[name], `$.decision.bindings.${name}`);
  }
  for (const name of [
    "familiar_embodiment_digest",
    "definition_digest",
    "action_digest",
    "runtime_descriptor_digest",
  ]) {
    requireDigestHex(value.bindings[name], `$.decision.bindings.${name}`);
  }
  requireInteger(value.bindings.definition_revision, "$.decision.bindings.definition_revision", 1);
  validateCapabilities(value.bindings.runtime_capabilities, "$.decision.bindings.runtime_capabilities");
  if (value.bindings.previous_approval_digest !== null) {
    requireDigestHex(
      value.bindings.previous_approval_digest,
      "$.decision.bindings.previous_approval_digest",
    );
  }
  requireEnum(
    value.outcome,
    ["permit", "requires_approval", "degrade_to_proposal", "reject"],
    "$.decision.outcome",
  );
  validateCapabilitiesOrEmpty(value.granted_capabilities, "$.decision.granted_capabilities");
  validateCapabilitiesOrEmpty(value.degraded_capabilities, "$.decision.degraded_capabilities");
  requireArray(value.denied_capabilities, "$.decision.denied_capabilities");
  value.denied_capabilities.forEach((denial, index) => {
    const path = `$.decision.denied_capabilities[${index}]`;
    required(denial, ["capability", "reason_code"], path);
    closed(denial, ["capability", "reason_code"], path);
    requireString(denial.capability, `${path}.capability`);
    if (!CAPABILITY_RISK.has(denial.capability)) {
      fail("capability_unknown", denial.capability, `${path}.capability`);
    }
    requireString(denial.reason_code, `${path}.reason_code`);
  });
  validateScopesOrEmpty(value.scopes, "$.decision.scopes");
  required(value.validity, ["not_before", "not_after"], "$.decision.validity");
  closed(value.validity, ["not_before", "not_after"], "$.decision.validity");
  requireTimestamp(value.validity.not_before, "$.decision.validity.not_before");
  requireTimestamp(value.validity.not_after, "$.decision.validity.not_after");
  if (Date.parse(value.validity.not_after) <= Date.parse(value.validity.not_before)) {
    fail("decision_invalid_interval", "decision expiry must follow its start");
  }
  if (value.approval_requirement === null) {
    if (value.outcome === "requires_approval") {
      fail("decision_approval_profile_missing", "approval outcome must name an approval profile");
    }
  } else {
    required(
      value.approval_requirement,
      ["profile", "recurring_allowed"],
      "$.decision.approval_requirement",
    );
    closed(
      value.approval_requirement,
      ["profile", "recurring_allowed"],
      "$.decision.approval_requirement",
    );
    requireEnum(
      value.approval_requirement.profile,
      ["human_per_run", "protected_owner_per_run"],
      "$.decision.approval_requirement.profile",
    );
    requireBoolean(
      value.approval_requirement.recurring_allowed,
      "$.decision.approval_requirement.recurring_allowed",
    );
    if (
      value.approval_requirement.profile === "protected_owner_per_run" &&
      value.approval_requirement.recurring_allowed
    ) {
      fail(
        "decision_recurring_approval_forbidden",
        "protected-owner approval is always per-run",
      );
    }
    if (value.outcome !== "requires_approval") {
      fail("decision_approval_profile_unexpected", "non-approval outcome carries approval policy");
    }
  }
  required(
    value.versions,
    ["profile", "policy", "policy_digest", "manifest", "manifest_digest"],
    "$.decision.versions",
  );
  closed(
    value.versions,
    ["profile", "policy", "policy_digest", "manifest", "manifest_digest"],
    "$.decision.versions",
  );
  exact(value.versions.profile, "1.0.0", "profile_version_unknown", "$.decision.versions.profile");
  requireString(value.versions.policy, "$.decision.versions.policy");
  requireDigestHex(value.versions.policy_digest, "$.decision.versions.policy_digest");
  requireString(value.versions.manifest, "$.decision.versions.manifest");
  requireDigestHex(value.versions.manifest_digest, "$.decision.versions.manifest_digest");
  requireArray(value.reason_codes, "$.decision.reason_codes", { nonempty: true, unique: true });
  value.reason_codes.forEach((reason, index) =>
    requireString(reason, `$.decision.reason_codes[${index}]`),
  );
  required(value.producer, ["id", "verifier_profile"], "$.decision.producer");
  closed(value.producer, ["id", "verifier_profile"], "$.decision.producer");
  exact(value.producer.id, "coven-threads", "decision_producer_forged", "$.decision.producer.id");
  exact(
    value.producer.verifier_profile,
    "automation-authority/1.0.0",
    "profile_version_unknown",
    "$.decision.producer.verifier_profile",
  );
  requireTimestamp(value.issued_at, "$.decision.issued_at");
  requireTimestamp(value.recorded_at, "$.decision.recorded_at");
  required(value.replay, ["dispatches", "consumption_required"], "$.decision.replay");
  closed(value.replay, ["dispatches", "consumption_required"], "$.decision.replay");
  exact(value.replay.dispatches, 1, "decision_replay_policy_invalid", "$.decision.replay.dispatches");
  requireBoolean(value.replay.consumption_required, "$.decision.replay.consumption_required");
  required(value.privacy, ["sensitivity", "retention"], "$.decision.privacy");
  closed(value.privacy, ["sensitivity", "retention"], "$.decision.privacy");
  requireEnum(value.privacy.sensitivity, Object.keys(SENSITIVITY), "$.decision.privacy.sensitivity");
  requireEnum(
    value.privacy.retention,
    ["ephemeral_24h", "authority_evidence_90d", "authority_evidence_1y"],
    "$.decision.privacy.retention",
  );
  validateIntegrityShape(value.integrity, "$.decision.integrity");
  verifySignedArtifact(value, DOMAIN.decision, keyring, "threads_authority");
  return value;
}

export function verifyDecisionBundle(request, decision, snapshot, { keyring } = {}) {
  validateDecision(decision, { keyring });
  const expected = evaluateAuthorization(request, snapshot, { keyring, unsigned: true });
  if (canonicalize(unsignedArtifact(decision)) !== canonicalize(expected)) {
    fail(
      "decision_semantic_mismatch",
      "signed decision does not match the reference policy evaluation",
    );
  }
  return { ok: true, outcome: decision.outcome };
}

export function consumeDecision(decision, state = null, { keyring } = {}) {
  validateDecision(decision, { keyring });
  if (!["permit", "requires_approval"].includes(decision.outcome)) {
    fail("decision_not_consumable", `${decision.outcome} decisions cannot authorize dispatch`);
  }
  const current = state ?? { decision_ids: [], decision_digests: [] };
  const digest = `sha256:${canonicalDigest(decision, DOMAIN.decision)}`;
  if (
    current.decision_ids.includes(decision.decision_id) ||
    current.decision_digests.includes(digest)
  ) {
    fail("decision_replayed", "decision was already consumed");
  }
  return {
    decision_ids: [...current.decision_ids, decision.decision_id],
    decision_digests: [...current.decision_digests, digest],
  };
}

export function validateConsumptionSnapshot(value, { keyring, now } = {}) {
  requireObject(value, "$.consumption_snapshot");
  required(
    value,
    [
      "schema_version",
      "snapshot_id",
      "recorded_at",
      "store_revision",
      "request_adoptions",
      "decision_consumptions",
      "approval_heads",
      "integrity",
    ],
    "$.consumption_snapshot",
  );
  closed(
    value,
    [
      "schema_version",
      "snapshot_id",
      "recorded_at",
      "store_revision",
      "request_adoptions",
      "decision_consumptions",
      "approval_heads",
      "integrity",
    ],
    "$.consumption_snapshot",
  );
  exact(
    value.schema_version,
    CONSUMPTION_VERSION,
    "schema_unknown_version",
    "$.consumption_snapshot.schema_version",
  );
  requireString(value.snapshot_id, "$.consumption_snapshot.snapshot_id");
  requireTimestamp(value.recorded_at, "$.consumption_snapshot.recorded_at");
  requireInteger(value.store_revision, "$.consumption_snapshot.store_revision", 1);
  if (now && Date.parse(value.recorded_at) > Date.parse(now)) {
    fail("consumption_snapshot_from_future", "consumption snapshot is newer than trusted now");
  }
  requireArray(value.request_adoptions, "$.consumption_snapshot.request_adoptions");
  const adoptionKeys = new Set();
  value.request_adoptions.forEach((adoption, index) => {
    const path = `$.consumption_snapshot.request_adoptions[${index}]`;
    required(adoption, ["request_digest", "nonce", "adoption_key"], path);
    closed(adoption, ["request_digest", "nonce", "adoption_key"], path);
    requireDigestHex(adoption.request_digest, `${path}.request_digest`);
    requireString(adoption.nonce, `${path}.nonce`);
    requireString(adoption.adoption_key, `${path}.adoption_key`);
    const key = canonicalize(adoption);
    if (adoptionKeys.has(key)) fail("consumption_snapshot_duplicate", "duplicate request adoption");
    adoptionKeys.add(key);
  });
  requireArray(value.decision_consumptions, "$.consumption_snapshot.decision_consumptions", {
    unique: true,
  });
  value.decision_consumptions.forEach((digest, index) =>
    requireDigestHex(digest, `$.consumption_snapshot.decision_consumptions[${index}]`),
  );
  requireArray(value.approval_heads, "$.consumption_snapshot.approval_heads");
  const approvalIds = new Set();
  value.approval_heads.forEach((head, index) => {
    const path = `$.consumption_snapshot.approval_heads[${index}]`;
    required(head, ["approval_id", "head_event_digest", "usage_count"], path);
    closed(head, ["approval_id", "head_event_digest", "usage_count"], path);
    requireString(head.approval_id, `${path}.approval_id`);
    requireDigestHex(head.head_event_digest, `${path}.head_event_digest`);
    requireInteger(head.usage_count, `${path}.usage_count`, 0);
    if (approvalIds.has(head.approval_id)) {
      fail("consumption_snapshot_duplicate", "duplicate approval head");
    }
    approvalIds.add(head.approval_id);
  });
  validateIntegrityShape(value.integrity, "$.consumption_snapshot.integrity");
  verifySignedArtifact(value, DOMAIN.consumption, keyring, "threads_authority");
  return value;
}

function validateCapabilitiesOrEmpty(capabilities, path) {
  requireArray(capabilities, path, { unique: true });
  capabilities.forEach((capability, index) => {
    requireString(capability, `${path}[${index}]`);
    if (!CAPABILITY_RISK.has(capability)) fail("capability_unknown", capability, path);
  });
}

function validateScopesOrEmpty(scopes, path) {
  requireArray(scopes, path);
  scopes.forEach((scope, index) => validateScope(scope, `${path}[${index}]`));
}

export function validateApproval(value, { keyring, now } = {}) {
  requireObject(value, "$");
  required(
    value,
    [
      "schema_version",
      "approval_id",
      "approving_principal",
      "request_digest",
      "decision_digest",
      "familiar_id",
      "familiar_embodiment_digest",
      "automation",
      "authorized_principal_id",
      "occurrence_id",
      "run_id",
      "attempt",
      "fence_generation",
      "action_digest",
      "capabilities",
      "scopes",
      "project_id",
      "workspace_id",
      "runtime_id",
      "runtime_descriptor_digest",
      "runtime_capabilities",
      "versions",
      "use",
      "issued_at",
      "expires_at",
      "nonce",
      "rationale",
      "integrity",
    ],
    "$",
  );
  closed(
    value,
    [
      "schema_version",
      "approval_id",
      "approving_principal",
      "request_digest",
      "decision_digest",
      "familiar_id",
      "familiar_embodiment_digest",
      "automation",
      "authorized_principal_id",
      "occurrence_id",
      "run_id",
      "attempt",
      "fence_generation",
      "action_digest",
      "capabilities",
      "scopes",
      "project_id",
      "workspace_id",
      "runtime_id",
      "runtime_descriptor_digest",
      "runtime_capabilities",
      "versions",
      "use",
      "issued_at",
      "expires_at",
      "nonce",
      "rationale",
      "integrity",
    ],
    "$",
  );
  exact(value.schema_version, APPROVAL_VERSION, "schema_unknown_version", "$.schema_version");
  requireString(value.approval_id, "$.approval_id");
  required(value.approving_principal, ["id", "key_ref"], "$.approving_principal");
  closed(value.approving_principal, ["id", "key_ref"], "$.approving_principal");
  requireString(value.approving_principal.id, "$.approving_principal.id");
  requireString(value.approving_principal.key_ref, "$.approving_principal.key_ref");
  [
    ["request_digest", value.request_digest],
    ["decision_digest", value.decision_digest],
    ["familiar_embodiment_digest", value.familiar_embodiment_digest],
    ["action_digest", value.action_digest],
    ["runtime_descriptor_digest", value.runtime_descriptor_digest],
  ].forEach(([name, digest]) => requireDigestHex(digest, `$.${name}`));
  requireString(value.familiar_id, "$.familiar_id");
  required(value.automation, ["id", "definition_revision", "definition_digest"], "$.automation");
  closed(value.automation, ["id", "definition_revision", "definition_digest"], "$.automation");
  requireString(value.automation.id, "$.automation.id");
  requireInteger(value.automation.definition_revision, "$.automation.definition_revision", 1);
  requireDigestHex(value.automation.definition_digest, "$.automation.definition_digest");
  requireString(value.authorized_principal_id, "$.authorized_principal_id");
  if (value.use?.kind === "recurring") {
    exact(value.occurrence_id, null, "approval_recurring_shape_invalid", "$.occurrence_id");
    exact(value.run_id, null, "approval_recurring_shape_invalid", "$.run_id");
    exact(value.attempt, null, "approval_recurring_shape_invalid", "$.attempt");
    exact(value.fence_generation, null, "approval_recurring_shape_invalid", "$.fence_generation");
  } else {
    requireString(value.occurrence_id, "$.occurrence_id");
    requireString(value.run_id, "$.run_id");
    requireInteger(value.attempt, "$.attempt", 1);
    requireInteger(value.fence_generation, "$.fence_generation", 1);
  }
  validateCapabilities(value.capabilities, "$.capabilities");
  validateScopes(value.scopes);
  requireString(value.project_id, "$.project_id");
  requireString(value.workspace_id, "$.workspace_id");
  requireString(value.runtime_id, "$.runtime_id");
  validateCapabilities(value.runtime_capabilities, "$.runtime_capabilities");
  const approvalRuntimeCapabilities = new Set(value.runtime_capabilities);
  if (value.capabilities.some((capability) => !approvalRuntimeCapabilities.has(capability))) {
    fail(
      "approval_runtime_capability_missing",
      "approval grants a capability absent from its bound runtime",
    );
  }
  required(
    value.versions,
    ["profile", "policy", "policy_digest", "manifest", "manifest_digest"],
    "$.versions",
  );
  closed(
    value.versions,
    ["profile", "policy", "policy_digest", "manifest", "manifest_digest"],
    "$.versions",
  );
  exact(value.versions.profile, "1.0.0", "profile_version_unknown", "$.versions.profile");
  requireString(value.versions.policy, "$.versions.policy");
  requireDigestHex(value.versions.policy_digest, "$.versions.policy_digest");
  requireString(value.versions.manifest, "$.versions.manifest");
  requireDigestHex(value.versions.manifest_digest, "$.versions.manifest_digest");
  requireObject(value.use, "$.use");
  requireString(value.use.kind, "$.use.kind");
  if (value.use.kind === "single_use") {
    closed(value.use, ["kind"], "$.use");
  } else if (value.use.kind === "recurring") {
    required(value.use, ["kind", "grant_id", "max_uses", "occurrence_prefix"], "$.use");
    closed(value.use, ["kind", "grant_id", "max_uses", "occurrence_prefix"], "$.use");
    requireString(value.use.grant_id, "$.use.grant_id");
    requireInteger(value.use.max_uses, "$.use.max_uses", 1);
    if (value.use.max_uses > 366) fail("approval_use_too_broad", "recurring max_uses exceeds 366");
    requireString(value.use.occurrence_prefix, "$.use.occurrence_prefix");
    if (
      value.use.occurrence_prefix.length < 8 ||
      value.use.occurrence_prefix.includes("*")
    ) {
      fail("approval_use_too_broad", "recurring occurrence prefix is too broad");
    }
  } else {
    fail("approval_use_unknown", `unknown approval use ${value.use.kind}`, "$.use.kind");
  }
  requireTimestamp(value.issued_at, "$.issued_at");
  requireTimestamp(value.expires_at, "$.expires_at");
  if (Date.parse(value.expires_at) <= Date.parse(value.issued_at)) {
    fail("approval_invalid_interval", "approval expiry must be after issue time");
  }
  if (now && Date.parse(now) < Date.parse(value.issued_at)) {
    fail("approval_not_yet_valid", "approval has not reached its issue time");
  }
  if (now && Date.parse(now) >= Date.parse(value.expires_at)) {
    fail("approval_expired", "approval has expired");
  }
  requireString(value.nonce, "$.nonce");
  required(value.rationale, ["text", "privacy"], "$.rationale");
  closed(value.rationale, ["text", "privacy"], "$.rationale");
  requireString(value.rationale.text, "$.rationale.text");
  requireEnum(value.rationale.privacy, Object.keys(SENSITIVITY), "$.rationale.privacy");
  validateIntegrityShape(value.integrity);
  const key = verifySignedArtifact(value, DOMAIN.approval, keyring);
  if (!["principal", "protected_owner"].includes(key.role)) {
    fail("integrity_role_mismatch", "approval must be signed by a principal authority key");
  }
  if (key.principal_id && key.principal_id !== value.approving_principal.id) {
    fail("approval_principal_mismatch", "approval key belongs to another principal");
  }
  if (value.integrity.key_id !== value.approving_principal.key_ref) {
    fail("approval_key_mismatch", "approval key_ref does not match signing key");
  }
  return value;
}

function validateLifecycleEvent(event, { keyring } = {}) {
  requireObject(event, "$.event");
  required(
    event,
    [
      "schema_version",
      "event_id",
      "approval_id",
      "request_digest",
      "decision_digest",
      "approval_digest",
      "sequence",
      "previous_event_digest",
      "from_state",
      "to_state",
      "event",
      "occurred_at",
      "actor",
      "execution_phase",
      "dispatch_disposition",
      "consumption",
      "occurrence_disposition",
      "integrity",
    ],
    "$.event",
  );
  closed(
    event,
    [
      "schema_version",
      "event_id",
      "approval_id",
      "request_digest",
      "decision_digest",
      "approval_digest",
      "sequence",
      "previous_event_digest",
      "from_state",
      "to_state",
      "event",
      "occurred_at",
      "actor",
      "execution_phase",
      "dispatch_disposition",
      "consumption",
      "occurrence_disposition",
      "integrity",
    ],
    "$.event",
  );
  exact(event.schema_version, EVENT_VERSION, "schema_unknown_version", "$.event.schema_version");
  requireString(event.event_id, "$.event.event_id");
  requireString(event.approval_id, "$.event.approval_id");
  requireDigestHex(event.request_digest, "$.event.request_digest");
  requireDigestHex(event.decision_digest, "$.event.decision_digest");
  if (event.approval_digest !== null) {
    requireDigestHex(event.approval_digest, "$.event.approval_digest");
  }
  requireInteger(event.sequence, "$.event.sequence", 1);
  if (event.previous_event_digest !== null) {
    requireDigestHex(event.previous_event_digest, "$.event.previous_event_digest");
  }
  requireEnum(
    event.from_state,
    ["required", "requested", "approved", "rejected", "expired", "revoked", "consumed"],
    "$.event.from_state",
  );
  if (event.consumption !== null) {
    const path = "$.event.consumption";
    required(
      event.consumption,
      [
        "request_digest",
        "decision_digest",
        "occurrence_id",
        "run_id",
        "attempt",
        "fence_generation",
      ],
      path,
    );
    closed(
      event.consumption,
      [
        "request_digest",
        "decision_digest",
        "occurrence_id",
        "run_id",
        "attempt",
        "fence_generation",
      ],
      path,
    );
    requireDigestHex(event.consumption.request_digest, `${path}.request_digest`);
    requireDigestHex(event.consumption.decision_digest, `${path}.decision_digest`);
    requireString(event.consumption.occurrence_id, `${path}.occurrence_id`);
    requireString(event.consumption.run_id, `${path}.run_id`);
    requireInteger(event.consumption.attempt, `${path}.attempt`, 1);
    requireInteger(event.consumption.fence_generation, `${path}.fence_generation`, 1);
  }
  if (event.event !== "consume" && event.consumption !== null) {
    fail("lifecycle_consumption_unexpected", "only consume events may carry per-run binding");
  }
  if (event.occurrence_disposition !== null) {
    const path = "$.event.occurrence_disposition";
    required(event.occurrence_disposition, ["occurrence_id", "run_id", "disposition"], path);
    closed(event.occurrence_disposition, ["occurrence_id", "run_id", "disposition"], path);
    requireString(event.occurrence_disposition.occurrence_id, `${path}.occurrence_id`);
    requireString(event.occurrence_disposition.run_id, `${path}.run_id`);
    requireEnum(
      event.occurrence_disposition.disposition,
      ["rejected_no_launch", "expired_no_launch"],
      `${path}.disposition`,
    );
  }
  if (
    !["reject", "expire"].includes(event.event) &&
    event.occurrence_disposition !== null
  ) {
    fail(
      "lifecycle_occurrence_disposition_unexpected",
      "only rejection and expiry may carry a no-launch occurrence disposition",
    );
  }
  requireEnum(
    event.to_state,
    ["requested", "approved", "rejected", "expired", "revoked", "consumed"],
    "$.event.to_state",
  );
  requireEnum(event.event, ["request", "approve", "reject", "expire", "revoke", "consume"], "$.event.event");
  requireTimestamp(event.occurred_at, "$.event.occurred_at");
  exact(event.actor, "threads_authority", "lifecycle_actor_forged", "$.event.actor");
  requireEnum(
    event.execution_phase,
    ["not_applicable", "not_started", "queued", "dispatching", "running", "completed"],
    "$.event.execution_phase",
  );
  requireEnum(
    event.dispatch_disposition,
    [
      "not_applicable",
      "launch_authorized",
      "cancel_before_launch",
      "request_cooperative_cancel",
      "external_effects_not_rolled_back",
      "no_launch_rejected",
      "no_launch_expired",
    ],
    "$.event.dispatch_disposition",
  );
  if (event.event === "reject") {
    if (
      event.execution_phase !== "not_started" ||
      event.dispatch_disposition !== "no_launch_rejected"
    ) {
      fail("rejection_disposition_invalid", "rejection must record an explicit no-launch disposition");
    }
    if (
      event.occurrence_disposition?.disposition !== "rejected_no_launch"
    ) {
      fail("rejection_occurrence_missing", "rejection must identify the no-launch occurrence");
    }
  } else if (event.event === "expire") {
    if (
      event.execution_phase !== "not_started" ||
      event.dispatch_disposition !== "no_launch_expired"
    ) {
      fail("expiration_disposition_invalid", "expiry must record an explicit no-launch disposition");
    }
    if (event.occurrence_disposition?.disposition !== "expired_no_launch") {
      fail("expiration_occurrence_missing", "expiry must identify the no-launch occurrence");
    }
  } else if (event.event === "revoke") {
    if (
      ["not_started", "queued", "dispatching"].includes(event.execution_phase) &&
      event.dispatch_disposition !== "cancel_before_launch"
    ) {
      fail("revocation_disposition_invalid", "pre-launch revocation must cancel before launch");
    }
    if (
      event.execution_phase === "running" &&
      !["request_cooperative_cancel", "external_effects_not_rolled_back"].includes(
        event.dispatch_disposition,
      )
    ) {
      fail(
        "revocation_disposition_invalid",
        "running revocation must request cancellation or preserve completed external effects",
      );
    }
  } else if (event.event === "consume") {
    if (event.consumption === null) {
      fail("consumption_binding_missing", "consumption event must bind the exact per-run operation");
    }
    if (
      event.execution_phase !== "dispatching" ||
      event.dispatch_disposition !== "launch_authorized"
    ) {
      fail("consumption_disposition_invalid", "consumption must be atomic with launch authorization");
    }
  } else if (
    event.execution_phase !== "not_applicable" ||
    event.dispatch_disposition !== "not_applicable" ||
    event.consumption !== null ||
    event.occurrence_disposition !== null
  ) {
    fail("lifecycle_disposition_invalid", "non-dispatch lifecycle event has a dispatch disposition");
  }
  validateIntegrityShape(event.integrity, "$.event.integrity");
  verifySignedArtifact(event, DOMAIN.event, keyring, "threads_authority");
}

const TRANSITIONS = new Map([
  ["required:request", "requested"],
  ["requested:approve", "approved"],
  ["requested:reject", "rejected"],
  ["requested:expire", "expired"],
  ["requested:revoke", "revoked"],
  ["approved:revoke", "revoked"],
  ["approved:expire", "expired"],
]);

export function applyLifecycleEvent(state, event, { approval, keyring } = {}) {
  validateLifecycleEvent(event, { keyring });
  const needsApprovalEvidence = event.event === "approve" || event.from_state === "approved";
  if (needsApprovalEvidence) {
    if (!approval) fail("lifecycle_approval_missing", "transition requires immutable approval evidence");
    validateApproval(approval, { keyring });
    const approvalDigest = `sha256:${canonicalDigest(approval, DOMAIN.approval)}`;
    if (
      event.approval_id !== approval.approval_id ||
      event.approval_digest !== approvalDigest ||
      event.request_digest !== approval.request_digest ||
      event.decision_digest !== approval.decision_digest
    ) {
      fail("lifecycle_approval_mismatch", "lifecycle event binds a different approval");
    }
  } else if (event.approval_digest !== null) {
    fail("lifecycle_approval_premature", "pre-approval transition cannot claim approval evidence");
  }
  if (
    ["reject", "expire"].includes(event.event) &&
    approval?.use.kind === "single_use" &&
    (event.occurrence_disposition.occurrence_id !== approval.occurrence_id ||
      event.occurrence_disposition.run_id !== approval.run_id)
  ) {
    fail("lifecycle_occurrence_mismatch", "no-launch disposition changed the occurrence");
  }
  const expectedSequence = state ? state.sequence + 1 : 1;
  if (event.sequence !== expectedSequence) fail("lifecycle_replay", "event sequence is stale or skipped");
  const expectedState = state?.state ?? "required";
  const expectedPrevious = state?.last_event_digest ?? null;
  if (
    state &&
    (state.approval_id !== event.approval_id ||
      state.request_digest !== event.request_digest ||
      state.decision_digest !== event.decision_digest)
  ) {
    fail("lifecycle_subject_mismatch", "event changes the approval subject");
  }
  if (event.from_state !== expectedState) fail("lifecycle_state_forged", "event from_state is not current");
  if (event.previous_event_digest !== expectedPrevious) {
    fail("lifecycle_chain_mismatch", "event previous digest does not match append-only head");
  }
  const expectedTo = TRANSITIONS.get(`${event.from_state}:${event.event}`);
  const consumptionTarget =
    event.event === "consume"
      ? approval.use.kind === "recurring"
        ? "approved"
        : "consumed"
      : null;
  if (
    (event.event === "consume" && event.to_state !== consumptionTarget) ||
    (event.event !== "consume" && (!expectedTo || expectedTo !== event.to_state))
  ) {
    fail("lifecycle_transition_invalid", "approval lifecycle transition is not allowed");
  }
  const consumptionCount = (state?.consumption_count ?? 0) + (event.event === "consume" ? 1 : 0);
  const consumedOccurrences = [...(state?.consumed_occurrences ?? [])];
  if (event.event === "consume") {
    const consumptionKey = canonicalize(event.consumption);
    if (consumedOccurrences.includes(consumptionKey)) {
      fail("approval_replayed", "approval already consumed for this exact occurrence/run");
    }
    if (approval.use.kind === "single_use" && state?.consumption_count > 0) {
      fail("approval_replayed", "single-use approval was already consumed");
    }
    if (approval.use.kind === "single_use") {
      const expectedConsumption = {
        request_digest: approval.request_digest,
        decision_digest: approval.decision_digest,
        occurrence_id: approval.occurrence_id,
        run_id: approval.run_id,
        attempt: approval.attempt,
        fence_generation: approval.fence_generation,
      };
      if (canonicalize(event.consumption) !== canonicalize(expectedConsumption)) {
        fail("approval_consumption_mismatch", "single-use consumption changed the approved operation");
      }
    }
    if (
      approval.use.kind === "recurring" &&
      !event.consumption.occurrence_id.startsWith(approval.use.occurrence_prefix)
    ) {
      fail("approval_occurrence_out_of_scope", "recurring approval occurrence is outside its pattern");
    }
    if (approval.use.kind === "recurring" && consumptionCount > approval.use.max_uses) {
      fail("approval_usage_exhausted", "recurring approval usage bound exceeded");
    }
    consumedOccurrences.push(consumptionKey);
  }
  return {
    approval_id: event.approval_id,
    request_digest: event.request_digest,
    decision_digest: event.decision_digest,
    state: event.to_state,
    sequence: event.sequence,
    last_event_digest: `sha256:${canonicalDigest(event, DOMAIN.event)}`,
    consumption_count: consumptionCount,
    consumed_occurrences: consumedOccurrences,
    event_ids: [...(state?.event_ids ?? []), event.event_id],
  };
}

export function verifyLifecycleChain(events, { approval, keyring, now } = {}) {
  requireArray(events, "$.lifecycle_events", { nonempty: true });
  validateApproval(approval, { keyring, now });
  let state = null;
  for (const event of events) {
    state = applyLifecycleEvent(state, event, { approval, keyring });
  }
  return state;
}

function assertBinding(name, actual, expected, code) {
  if (actual !== expected) fail(code, `${name} changed after authorization`);
}

export function verifyDispatch(bundle, { keyring } = {}) {
  requireObject(bundle, "$.dispatch_bundle");
  required(
    bundle,
    [
      "request",
      "decision",
      "approval",
      "approval_authorization_request",
      "approval_authorization_decision",
      "lifecycle_events",
      "consumption_snapshot",
      "snapshot",
    ],
    "$.dispatch_bundle",
  );
  closed(
    bundle,
    [
      "request",
      "decision",
      "approval",
      "approval_authorization_request",
      "approval_authorization_decision",
      "lifecycle_events",
      "consumption_snapshot",
      "snapshot",
    ],
    "$.dispatch_bundle",
  );
  const {
    request,
    decision,
    approval,
    approval_authorization_request: approvalAuthorizationRequest,
    approval_authorization_decision: approvalAuthorizationDecision,
    lifecycle_events: lifecycleEvents,
    consumption_snapshot: consumptionSnapshot,
    snapshot,
  } = bundle;
  requireArray(lifecycleEvents, "$.dispatch_bundle.lifecycle_events");
  requireTimestamp(snapshot.now, "$.dispatch_bundle.snapshot.now");
  validateAuthorizationRequest(request, { keyring });
  validateDecision(decision, { keyring });
  if (Date.parse(snapshot.now) < Date.parse(request.replay.issued_at)) {
    fail("request_not_yet_valid", "dispatch request has not reached its issue time");
  }
  if (Date.parse(snapshot.now) >= Date.parse(request.replay.expires_at)) {
    fail("request_expired", "dispatch request has expired");
  }
  validateConsumptionSnapshot(consumptionSnapshot, { keyring, now: snapshot.now });
  assertBinding(
    "consumption store revision",
    consumptionSnapshot.store_revision,
    snapshot.consumption_revision,
    "consumption_snapshot_stale",
  );
  const requestDigest = `sha256:${canonicalDigest(request, DOMAIN.request)}`;
  if (
    consumptionSnapshot.request_adoptions.some(
      (adoption) =>
        (adoption.nonce === request.replay.nonce ||
          adoption.adoption_key === request.replay.adoption_key) &&
        adoption.request_digest !== requestDigest,
    )
  ) {
    fail("request_replayed", "nonce or adoption key was previously used by another request");
  }
  const matchingAdoptions = consumptionSnapshot.request_adoptions.filter(
    (adoption) =>
      adoption.request_digest === requestDigest &&
      adoption.nonce === request.replay.nonce &&
      adoption.adoption_key === request.replay.adoption_key,
  );
  if (matchingAdoptions.length === 0) {
    fail("request_not_adopted", "dispatch request has no authenticated adoption evidence");
  }
  if (matchingAdoptions.length !== 1) {
    fail("request_replayed", "dispatch request has duplicate adoption evidence");
  }
  const decisionDigest = `sha256:${canonicalDigest(decision, DOMAIN.decision)}`;
  if (consumptionSnapshot.decision_consumptions.includes(decisionDigest)) {
    fail("decision_replayed", "dispatch decision was already consumed");
  }
  assertBinding("request digest", decision.request_digest, requestDigest, "decision_request_mismatch");
  assertBinding("request id", decision.request_id, request.request_id, "decision_request_mismatch");
  assertBinding(
    "decision occurrence",
    decision.correlation.occurrence_id,
    request.execution.occurrence_id,
    "decision_binding_mismatch",
  );
  assertBinding(
    "decision run",
    decision.correlation.run_id,
    request.execution.run_id,
    "decision_binding_mismatch",
  );
  assertBinding(
    "decision attempt",
    decision.correlation.attempt,
    request.execution.attempt,
    "decision_binding_mismatch",
  );
  assertBinding(
    "decision fence",
    decision.correlation.fence_generation,
    request.execution.fence_generation,
    "decision_binding_mismatch",
  );
  const bindings = decision.bindings;
  const expected = {
    principal_id: request.principal.id,
    familiar_id: request.familiar.id,
    familiar_embodiment_digest: request.familiar.embodiment_digest,
    automation_id: request.automation.id,
    definition_revision: request.automation.definition_revision,
    definition_digest: request.automation.definition_digest,
    action_digest: request.action.digest,
    project_id: request.context.project_id,
    workspace_id: request.context.workspace_id,
    runtime_id: request.context.runtime.id,
    runtime_descriptor_digest: request.context.runtime.descriptor_digest,
    previous_approval_digest: request.previous_approval_digest,
  };
  for (const [name, value] of Object.entries(expected)) {
    assertBinding(name, bindings[name], value, "decision_binding_mismatch");
  }

  assertBinding(
    "principal",
    snapshot.principal_id,
    request.principal.id,
    "dispatch_principal_mismatch",
  );
  assertBinding("familiar", snapshot.familiar_id, request.familiar.id, "dispatch_familiar_mismatch");
  assertBinding(
    "familiar embodiment",
    snapshot.familiar_embodiment_digest,
    request.familiar.embodiment_digest,
    "dispatch_familiar_changed",
  );
  assertBinding(
    "automation",
    snapshot.automation_id,
    request.automation.id,
    "dispatch_automation_mismatch",
  );
  assertBinding(
    "definition revision",
    snapshot.definition_revision,
    request.automation.definition_revision,
    "dispatch_definition_changed",
  );
  assertBinding(
    "definition digest",
    snapshot.definition_digest,
    request.automation.definition_digest,
    "dispatch_definition_changed",
  );
  assertBinding(
    "occurrence",
    snapshot.occurrence_id,
    request.execution.occurrence_id,
    "dispatch_occurrence_mismatch",
  );
  assertBinding("run", snapshot.run_id, request.execution.run_id, "dispatch_run_mismatch");
  assertBinding("attempt", snapshot.attempt, request.execution.attempt, "dispatch_attempt_mismatch");
  assertBinding(
    "fence",
    snapshot.fence_generation,
    request.execution.fence_generation,
    "dispatch_stale_fence",
  );
  assertBinding("action", snapshot.action_digest, request.action.digest, "dispatch_action_changed");
  assertBinding("project", snapshot.project_id, request.context.project_id, "dispatch_project_mismatch");
  assertBinding(
    "workspace",
    snapshot.workspace_id,
    request.context.workspace_id,
    "dispatch_workspace_mismatch",
  );
  assertBinding(
    "runtime id",
    snapshot.runtime_id,
    request.context.runtime.id,
    "dispatch_runtime_changed",
  );
  assertBinding(
    "runtime descriptor",
    snapshot.runtime_descriptor_digest,
    request.context.runtime.descriptor_digest,
    "dispatch_runtime_changed",
  );
  assertBinding("policy", snapshot.policy, request.versions.policy, "dispatch_policy_stale");
  assertBinding(
    "policy digest",
    snapshot.policy_digest,
    request.versions.policy_digest,
    "dispatch_policy_stale",
  );
  assertBinding("manifest", snapshot.manifest, request.versions.manifest, "dispatch_manifest_stale");
  assertBinding(
    "manifest digest",
    snapshot.manifest_digest,
    request.versions.manifest_digest,
    "dispatch_manifest_stale",
  );
  const runtimeCapabilities = new Set(snapshot.runtime_capabilities);
  for (const capability of decision.granted_capabilities) {
    if (!runtimeCapabilities.has(capability)) {
      fail("dispatch_runtime_downgrade", `runtime no longer exposes ${capability}`);
    }
  }
  if (Date.parse(snapshot.now) < Date.parse(decision.validity.not_before)) {
    fail("decision_not_yet_valid", "decision validity interval has not started");
  }
  if (Date.parse(snapshot.now) >= Date.parse(decision.validity.not_after)) {
    fail("decision_expired", "decision validity interval has ended");
  }
  if (decision.outcome === "reject") fail("dispatch_rejected", "rejected decision cannot dispatch");
  if (decision.outcome === "degrade_to_proposal") {
    fail("proposal_dispatch_forbidden", "proposal-only decision cannot dispatch protected effects");
  }
  if (decision.outcome === "requires_approval") {
    if (!approval) fail("approval_required", "dispatch requires operation-specific approval");
    validateApproval(approval, { keyring, now: snapshot.now });
    if (approval.use.kind === "recurring" && !decision.approval_requirement.recurring_allowed) {
      fail("approval_recurring_not_allowed", "decision requires a single per-run approval");
    }
    if (approval.use.kind === "recurring") {
      if (!approvalAuthorizationRequest || !approvalAuthorizationDecision) {
        fail(
          "approval_recurring_authorization_missing",
          "recurring approval requires its immutable grant request and decision",
        );
      }
      validateAuthorizationRequest(approvalAuthorizationRequest, { keyring });
      validateDecision(approvalAuthorizationDecision, { keyring });
      const authorizationRequestDigest =
        `sha256:${canonicalDigest(approvalAuthorizationRequest, DOMAIN.request)}`;
      const authorizationDecisionDigest =
        `sha256:${canonicalDigest(approvalAuthorizationDecision, DOMAIN.decision)}`;
      if (
        authorizationRequestDigest !== approval.request_digest ||
        authorizationDecisionDigest !== approval.decision_digest ||
        approvalAuthorizationDecision.request_digest !== approval.request_digest ||
        approvalAuthorizationDecision.request_id !== approvalAuthorizationRequest.request_id ||
        approvalAuthorizationDecision.outcome !== "requires_approval" ||
        approvalAuthorizationDecision.approval_requirement.recurring_allowed !== true
      ) {
        fail(
          "approval_recurring_authorization_mismatch",
          "recurring approval grant decision does not authorize recurring use",
        );
      }
      const grantBindings = approvalAuthorizationDecision.bindings;
      const authorizationCorrelation = {
        occurrence_id: approvalAuthorizationRequest.execution.occurrence_id,
        run_id: approvalAuthorizationRequest.execution.run_id,
        attempt: approvalAuthorizationRequest.execution.attempt,
        fence_generation: approvalAuthorizationRequest.execution.fence_generation,
      };
      if (
        canonicalize(approvalAuthorizationDecision.correlation) !==
        canonicalize(authorizationCorrelation)
      ) {
        fail(
          "approval_recurring_authorization_mismatch",
          "recurring grant decision correlation does not match its request",
        );
      }
      const authorizationRequestBindings = {
        principal_id: approvalAuthorizationRequest.principal.id,
        familiar_id: approvalAuthorizationRequest.familiar.id,
        familiar_embodiment_digest: approvalAuthorizationRequest.familiar.embodiment_digest,
        automation_id: approvalAuthorizationRequest.automation.id,
        definition_revision: approvalAuthorizationRequest.automation.definition_revision,
        definition_digest: approvalAuthorizationRequest.automation.definition_digest,
        action_digest: approvalAuthorizationRequest.action.digest,
        project_id: approvalAuthorizationRequest.context.project_id,
        workspace_id: approvalAuthorizationRequest.context.workspace_id,
        runtime_id: approvalAuthorizationRequest.context.runtime.id,
        runtime_descriptor_digest:
          approvalAuthorizationRequest.context.runtime.descriptor_digest,
      };
      const approvalGrantBindings = {
        principal_id: approval.authorized_principal_id,
        familiar_id: approval.familiar_id,
        familiar_embodiment_digest: approval.familiar_embodiment_digest,
        automation_id: approval.automation.id,
        definition_revision: approval.automation.definition_revision,
        definition_digest: approval.automation.definition_digest,
        action_digest: approval.action_digest,
        project_id: approval.project_id,
        workspace_id: approval.workspace_id,
        runtime_id: approval.runtime_id,
        runtime_descriptor_digest: approval.runtime_descriptor_digest,
      };
      for (const [name, expectedValue] of Object.entries(approvalGrantBindings)) {
        assertBinding(
          name,
          authorizationRequestBindings[name],
          expectedValue,
          "approval_recurring_authorization_mismatch",
        );
        assertBinding(
          name,
          grantBindings[name],
          expectedValue,
          "approval_recurring_authorization_mismatch",
        );
      }
      if (
        canonicalize(approvalAuthorizationRequest.requested_capabilities) !==
          canonicalize(approval.capabilities) ||
        canonicalize(approvalAuthorizationRequest.scopes) !== canonicalize(approval.scopes) ||
        approvalAuthorizationRequest.action.risk_class !== "R2" ||
        canonicalize(approvalAuthorizationRequest.context.runtime.capabilities) !==
          canonicalize(approval.runtime_capabilities) ||
        canonicalize(grantBindings.runtime_capabilities) !==
          canonicalize(approval.runtime_capabilities) ||
        canonicalize(approvalAuthorizationRequest.versions) !== canonicalize(approval.versions) ||
        canonicalize(approvalAuthorizationDecision.versions) !==
          canonicalize(approval.versions)
      ) {
        fail(
          "approval_recurring_authorization_mismatch",
          "recurring approval exceeds or changes its grant decision",
        );
      }
    } else if (
      approvalAuthorizationRequest !== null ||
      approvalAuthorizationDecision !== null
    ) {
      fail(
        "approval_recurring_authorization_unexpected",
        "single-use approval cannot carry a recurring grant decision",
      );
    }
    const approvalKey = normalizeKeyring(keyring).get(approval.integrity.key_id);
    if (decision.approval_requirement.profile === "protected_owner_per_run") {
      if (approvalKey?.role !== "protected_owner") {
        fail(
          "approval_role_mismatch",
          "protected_owner_per_run requires a protected_owner signing key",
        );
      }
    } else if (decision.approval_requirement.profile === "human_per_run") {
      if (
        approvalKey?.role !== "principal" ||
        approval.approving_principal.id !== request.principal.id
      ) {
        fail(
          "approval_role_mismatch",
          "human_per_run requires the authorized principal's signing key",
        );
      }
    }
    const approvalExpected = {
      familiar_id: request.familiar.id,
      familiar_embodiment_digest: request.familiar.embodiment_digest,
      authorized_principal_id: request.principal.id,
      action_digest: request.action.digest,
      project_id: request.context.project_id,
      workspace_id: request.context.workspace_id,
      runtime_id: request.context.runtime.id,
      runtime_descriptor_digest: request.context.runtime.descriptor_digest,
    };
    for (const [name, value] of Object.entries(approvalExpected)) {
      assertBinding(name, approval[name], value, "approval_binding_mismatch");
    }
    if (approval.automation.id !== request.automation.id) {
      fail("approval_binding_mismatch", "approval automation changed");
    }
    if (
      approval.automation.definition_revision !== request.automation.definition_revision ||
      approval.automation.definition_digest !== request.automation.definition_digest
    ) {
      fail("approval_definition_changed", "approval definition binding changed");
    }
    if (canonicalize(approval.capabilities) !== canonicalize(request.requested_capabilities)) {
      fail("approval_capabilities_changed", "approval capability binding changed");
    }
    if (canonicalize(approval.scopes) !== canonicalize(request.scopes)) {
      fail("approval_scopes_changed", "approval scope binding changed");
    }
    if (
      canonicalize(approval.runtime_capabilities) !==
      canonicalize(request.context.runtime.capabilities)
    ) {
      fail("approval_runtime_capabilities_changed", "approval runtime capabilities changed");
    }
    if (canonicalize(approval.versions) !== canonicalize(request.versions)) {
      fail("approval_policy_changed", "approval policy or manifest binding changed");
    }
    for (const capability of request.requested_capabilities) {
      if (!runtimeCapabilities.has(capability)) {
        fail(
          "dispatch_runtime_capability_missing",
          `approval-required dispatch runtime lacks ${capability}`,
        );
      }
    }
    if (approval.use.kind === "single_use") {
      const singleUseExpected = {
        request_digest: requestDigest,
        decision_digest: decisionDigest,
        occurrence_id: request.execution.occurrence_id,
        run_id: request.execution.run_id,
        attempt: request.execution.attempt,
        fence_generation: request.execution.fence_generation,
      };
      for (const [name, value] of Object.entries(singleUseExpected)) {
        assertBinding(name, approval[name], value, "approval_binding_mismatch");
      }
    } else if (!request.execution.occurrence_id.startsWith(approval.use.occurrence_prefix)) {
      fail("approval_occurrence_out_of_scope", "occurrence is outside recurring approval pattern");
    }
    if (!Array.isArray(lifecycleEvents) || lifecycleEvents.length === 0) {
      fail("approval_lifecycle_required", "authenticated append-only lifecycle evidence is required");
    }
    const lifecycle = verifyLifecycleChain(lifecycleEvents, {
      approval,
      keyring,
      now: snapshot.now,
    });
    const committedHead = consumptionSnapshot.approval_heads.find(
      (head) => head.approval_id === approval.approval_id,
    );
    if (
      !committedHead ||
      committedHead.head_event_digest !== lifecycle.last_event_digest ||
      committedHead.usage_count !== lifecycle.consumption_count
    ) {
      fail(
        "approval_lifecycle_head_mismatch",
        "lifecycle chain does not match authenticated consumption state",
      );
    }
    if (lifecycle.state !== "approved") {
      fail("approval_state_invalid", "approval is not in dispatchable approved state");
    }
    if (
      approval.use.kind === "recurring" &&
      lifecycle.consumption_count >= approval.use.max_uses
    ) {
      fail("approval_usage_exhausted", "recurring approval usage bound is exhausted");
    }
    const currentConsumption = canonicalize({
      request_digest: requestDigest,
      decision_digest: decisionDigest,
      occurrence_id: request.execution.occurrence_id,
      run_id: request.execution.run_id,
      attempt: request.execution.attempt,
      fence_generation: request.execution.fence_generation,
    });
    if (lifecycle.consumed_occurrences.includes(currentConsumption)) {
      fail("approval_replayed", "approval already consumed for this exact dispatch");
    }
  } else {
    if (
      approval !== null ||
      approvalAuthorizationRequest !== null ||
      approvalAuthorizationDecision !== null ||
      lifecycleEvents.length !== 0
    ) {
      fail("approval_unexpected", "non-approval decision cannot carry approval authority");
    }
  }
  return {
    ok: true,
    dispatch_binding_digest: `sha256:${canonicalDigest(
      {
        request_digest: requestDigest,
        decision_digest: `sha256:${canonicalDigest(decision, DOMAIN.decision)}`,
        snapshot,
      },
      "opencoven:automation-dispatch-binding:v1",
    )}`,
    required_consumption: {
      decision_digest: decisionDigest,
      approval_id: approval?.approval_id ?? null,
      approval_usage_before: approval
        ? consumptionSnapshot.approval_heads.find(
            (head) => head.approval_id === approval.approval_id,
          )?.usage_count ?? null
        : null,
    },
  };
}

export function validateProposal(value, { keyring } = {}) {
  requireObject(value, "$");
  required(
    value,
    [
      "schema_version",
      "proposal_id",
      "request_digest",
      "action_digest",
      "intended_target",
      "status",
      "protected_effects_performed",
      "result_claim",
      "requires_new_adoption",
      "content_digest",
      "created_at",
      "privacy",
      "integrity",
    ],
    "$",
  );
  closed(
    value,
    [
      "schema_version",
      "proposal_id",
      "request_digest",
      "action_digest",
      "intended_target",
      "status",
      "protected_effects_performed",
      "result_claim",
      "requires_new_adoption",
      "content_digest",
      "created_at",
      "privacy",
      "integrity",
    ],
    "$",
  );
  exact(value.schema_version, PROPOSAL_VERSION, "schema_unknown_version", "$.schema_version");
  requireString(value.proposal_id, "$.proposal_id");
  requireDigestHex(value.request_digest, "$.request_digest");
  requireDigestHex(value.action_digest, "$.action_digest");
  requireString(value.intended_target, "$.intended_target");
  exact(value.status, "not_executed", "proposal_status_forged", "$.status");
  exact(
    value.protected_effects_performed,
    false,
    "proposal_effect_forbidden",
    "$.protected_effects_performed",
  );
  exact(value.result_claim, "proposal_only", "proposal_success_forged", "$.result_claim");
  exact(value.requires_new_adoption, true, "proposal_adoption_required", "$.requires_new_adoption");
  requireDigestHex(value.content_digest, "$.content_digest");
  requireTimestamp(value.created_at, "$.created_at");
  requireEnum(value.privacy, Object.keys(SENSITIVITY), "$.privacy");
  validateIntegrityShape(value.integrity);
  verifySignedArtifact(value, DOMAIN.proposal, keyring, "threads_authority");
  return value;
}

export function authorizeEvidenceRead(value, evidence, { keyring, now } = {}) {
  requireObject(value, "$");
  required(
    value,
    [
      "schema_version",
      "read_id",
      "requesting_principal_id",
      "authorization_proof_ref",
      "subject_principal_id",
      "automation_id",
      "maximum_sensitivity",
      "retention_classes",
      "issued_at",
      "expires_at",
      "nonce",
      "integrity",
    ],
    "$",
  );
  closed(
    value,
    [
      "schema_version",
      "read_id",
      "requesting_principal_id",
      "authorization_proof_ref",
      "subject_principal_id",
      "automation_id",
      "maximum_sensitivity",
      "retention_classes",
      "issued_at",
      "expires_at",
      "nonce",
      "integrity",
    ],
    "$",
  );
  exact(value.schema_version, EVIDENCE_READ_VERSION, "schema_unknown_version", "$.schema_version");
  requireString(value.read_id, "$.read_id");
  requireString(value.requesting_principal_id, "$.requesting_principal_id");
  requireString(value.authorization_proof_ref, "$.authorization_proof_ref");
  requireString(value.subject_principal_id, "$.subject_principal_id");
  requireString(value.automation_id, "$.automation_id");
  requireEnum(value.maximum_sensitivity, Object.keys(SENSITIVITY), "$.maximum_sensitivity");
  requireArray(value.retention_classes, "$.retention_classes", { nonempty: true, unique: true });
  value.retention_classes.forEach((entry, index) =>
    requireEnum(
      entry,
      ["ephemeral_24h", "authority_evidence_90d", "authority_evidence_1y"],
      `$.retention_classes[${index}]`,
    ),
  );
  requireTimestamp(value.issued_at, "$.issued_at");
  requireTimestamp(value.expires_at, "$.expires_at");
  if (Date.parse(value.expires_at) <= Date.parse(value.issued_at)) {
    fail("evidence_read_invalid_interval", "evidence-read expiry must be after issue time");
  }
  if (!now) fail("evidence_read_time_required", "trusted current time is required");
  requireTimestamp(now, "$.trusted_now");
  if (Date.parse(now) < Date.parse(value.issued_at)) {
    fail("evidence_read_not_yet_valid", "evidence-read token is not yet valid");
  }
  if (Date.parse(now) >= Date.parse(value.expires_at)) {
    fail("evidence_read_expired", "evidence-read token has expired");
  }
  requireString(value.nonce, "$.nonce");
  validateIntegrityShape(value.integrity);
  const key = verifySignedArtifact(value, DOMAIN.evidenceRead, keyring);
  if (key.principal_id !== value.requesting_principal_id) {
    fail("evidence_reader_mismatch", "read request signing principal mismatch");
  }
  if (value.requesting_principal_id !== value.subject_principal_id && key.role !== "auditor") {
    fail("evidence_read_unauthorized", "cross-principal evidence read requires auditor authority");
  }
  if (evidence.principal_id !== value.subject_principal_id || evidence.automation_id !== value.automation_id) {
    fail("evidence_subject_mismatch", "evidence does not match the authorized subject");
  }
  if (SENSITIVITY[evidence.sensitivity] > SENSITIVITY[value.maximum_sensitivity]) {
    fail("evidence_sensitivity_denied", "evidence exceeds reader sensitivity clearance");
  }
  if (!value.retention_classes.includes(evidence.retention)) {
    fail("evidence_retention_denied", "evidence retention class is outside read scope");
  }
  return { ok: true };
}

export const profileConstants = Object.freeze({
  versions: {
    request: REQUEST_VERSION,
    decision: DECISION_VERSION,
    approval: APPROVAL_VERSION,
    event: EVENT_VERSION,
    consumption: CONSUMPTION_VERSION,
    proposal: PROPOSAL_VERSION,
    evidenceRead: EVIDENCE_READ_VERSION,
  },
  domains: DOMAIN,
  capabilities: [...CAPABILITY_RISK.keys()],
  actions: [...ACTIONS.keys()],
  riskClasses: Object.keys(RISK_NUMBER),
});
