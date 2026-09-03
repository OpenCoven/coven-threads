import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

import { strictParseJson } from "../validator.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");

function assertObjectSchemasAreClosed(schema, path = "$") {
  if (!schema || typeof schema !== "object") return;
  if (schema.type === "object") {
    assert.equal(
      schema.additionalProperties,
      false,
      `${path} must set additionalProperties:false`,
    );
  }
  if (Array.isArray(schema)) {
    schema.forEach((item, index) => assertObjectSchemasAreClosed(item, `${path}[${index}]`));
  } else {
    for (const [key, value] of Object.entries(schema)) {
      assertObjectSchemasAreClosed(value, `${path}.${key}`);
    }
  }
}

test("all published JSON Schemas parse and close every object", () => {
  const schemaDirectory = resolve(ROOT, "schemas");
  const schemas = readdirSync(schemaDirectory).filter((name) => name.endsWith(".schema.json"));
  assert.equal(schemas.length, 10);
  for (const name of schemas) {
    const schema = strictParseJson(readFileSync(resolve(schemaDirectory, name), "utf8"));
    assert.equal(schema.$schema, "https://json-schema.org/draft/2020-12/schema");
    assertObjectSchemasAreClosed(schema, name);
  }
});

test("keyring publishes public verification keys only", () => {
  const keyring = strictParseJson(readFileSync(resolve(ROOT, "keyring.json"), "utf8"));
  for (const key of Object.values(keyring.keys)) {
    assert.match(key.public_key_pem, /^-----BEGIN PUBLIC KEY-----/);
    assert.equal(Object.hasOwn(key, "private_key"), false);
    assert.equal(Object.hasOwn(key, "private_key_pem"), false);
  }
});

test("manifest runner passes without writing profile artifacts", () => {
  const before = snapshotFiles(ROOT);
  const result = spawnSync(process.execPath, [resolve(ROOT, "run-vectors.mjs")], {
    cwd: ROOT,
    encoding: "utf8",
  });
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /PASS \d+\/\d+ vectors; 18\/18 categories; manifest exact/);
  assert.deepEqual(snapshotFiles(ROOT), before);
});

function snapshotFiles(root) {
  const files = [];
  const visit = (directory) => {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      if (entry.name === "tools") continue;
      const path = resolve(directory, entry.name);
      if (entry.isDirectory()) visit(path);
      else files.push([path.slice(root.length + 1), readFileSync(path).toString("base64")]);
    }
  };
  visit(root);
  return files.sort(([left], [right]) => left.localeCompare(right));
}
