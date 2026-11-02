import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { writeArtifactIndex } from "../lib/daemon-reliability-evidence.mjs";

const path = new URL("../lib/daemon-artifact-archive.mjs", import.meta.url);
async function implementation() {
  assert(existsSync(path), "immutable artifact archive is not implemented");
  return import(path);
}
function root(t) {
  const path = mkdtempSync(join(tmpdir(), "threads-archive-"));
  t.after(() => rmSync(path, { recursive: true, force: true }));
  return path;
}
function zip(path, members) {
  execFileSync("python3", ["-I", "-c", `import sys,json,zipfile
with zipfile.ZipFile(sys.argv[1], 'w') as z:
 for name,body,mode in json.load(sys.stdin):
  info=zipfile.ZipInfo(name)
  if mode: info.external_attr=mode<<16
  z.writestr(info,body)
`, path], { input: JSON.stringify(members), stdio: ["pipe", "pipe", "pipe"] });
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

test("ZIP extraction verifies exact bytes/CRC and refuses ambiguous or escaping members", async (t) => {
  const { extractArtifact } = await implementation();
  const directory = root(t);
  const archive = join(directory, "artifact.zip");
  const digest = zip(archive, [["log.txt", "original diagnostic", 0]]);
  extractArtifact(archive, digest, join(directory, "good"));
  assert.equal(readFileSync(join(directory, "good", "log.txt"), "utf8"), "original diagnostic");
  assert.throws(() => extractArtifact(archive, "a".repeat(64), join(directory, "wrong-digest")));
  assert(!existsSync(join(directory, "wrong-digest")));
  const attacks = [[["../outside.txt", "escape", 0]], [["/absolute.txt", "escape", 0]],
    [["D:/drive.txt", "escape", 0]], [["back\\slash.txt", "escape", 0]],
    [["duplicate", "first", 0], ["duplicate", "second", 0]],
    [["Case.txt", "first", 0], ["case.txt", "second", 0]],
    [["link", "outside", 0o120777]], [["parent", "file", 0], ["parent/child", "data", 0]],
    [[".. /outside.txt", "escape", 0]], [["NUL", "device", 0]], [["folder./file", "alias", 0]],
    [["Parent/a", "first", 0], ["parent/b", "second", 0]]];
  for (let i = 0; i < attacks.length; i++) {
    const badDigest = zip(archive, attacks[i]);
    assert.throws(() => extractArtifact(archive, badDigest, join(directory, "bad-" + i)));
    assert(!existsSync(join(directory, "outside.txt")));
  }
  zip(archive, [["data", "crc payload sentinel", 0]]);
  const bytes = readFileSync(archive);
  const offset = bytes.indexOf(Buffer.from("crc payload sentinel"));
  assert(offset >= 0); bytes[offset] ^= 1; writeFileSync(archive, bytes);
  assert.throws(() => extractArtifact(archive, createHash("sha256").update(bytes).digest("hex"), join(directory, "bad-crc")));
});

test("file index verification refuses missing, extra, modified or duplicate evidence", async (t) => {
  const { verifyArtifactIndex } = await implementation();
  const directory = root(t);
  writeFileSync(join(directory, "log.txt"), "abc");
  writeArtifactIndex(directory);
  assert.equal(verifyArtifactIndex(directory).files.length, 1);
  writeFileSync(join(directory, "log.txt"), "changed");
  assert.throws(() => verifyArtifactIndex(directory));
  writeFileSync(join(directory, "log.txt"), "abc");
  writeFileSync(join(directory, "extra"), "unindexed");
  assert.throws(() => verifyArtifactIndex(directory));
  rmSync(join(directory, "extra"));
  const index = JSON.parse(readFileSync(join(directory, "artifact-index.json")));
  index.files.push(index.files[0]);
  writeFileSync(join(directory, "artifact-index.json"), JSON.stringify(index));
  assert.throws(() => verifyArtifactIndex(directory));
});

test("archive publication is atomic, idempotent for identical bytes and refuses conflicts", async (t) => {
  const { publishArchive } = await implementation();
  const directory = root(t);
  const stage = () => {
    const path = mkdtempSync(join(directory, ".stage-"));
    writeFileSync(join(path, "record.json"), '{"source":"synthetic"}\n');
    writeArtifactIndex(path); return path;
  };
  const destination = join(directory, "42", "1");
  assert.equal(publishArchive(stage(), destination).state, "created");
  assert.equal(publishArchive(stage(), destination).state, "identical");
  const conflicting = stage();
  rmSync(join(conflicting, "artifact-index.json"));
  writeFileSync(join(conflicting, "record.json"), "different");
  writeArtifactIndex(conflicting);
  assert.throws(() => publishArchive(conflicting, destination), /conflict/);
  assert.equal(readFileSync(join(destination, "record.json"), "utf8"), '{"source":"synthetic"}\n');
});
