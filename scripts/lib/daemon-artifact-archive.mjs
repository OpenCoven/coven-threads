import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { existsSync, lstatSync, mkdirSync, readFileSync, readdirSync, renameSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const hash = bytes => createHash("sha256").update(bytes).digest("hex");
function regular(path) {
  if (!lstatSync(path).isFile()) throw new Error("archive evidence must be a regular file");
  return readFileSync(path);
}
function validPath(path) {
  return typeof path === "string" && path && !/[\\:\u0000]/.test(path) &&
    path.split("/").every(part => part && part !== "." && part !== "..");
}

export function extractArtifact(archive, expectedSha256, destination) {
  if (!/^[0-9a-f]{64}$/.test(expectedSha256) || hash(regular(archive)) !== expectedSha256) {
    throw new Error("downloaded artifact bytes differ from the authenticated GitHub digest");
  }
  const result = spawnSync("python3", ["-I", fileURLToPath(new URL("./extract-daemon-artifact.py", import.meta.url)), archive, destination],
    { encoding: "utf8", maxBuffer: 1024 * 1024 });
  if (result.error || result.status !== 0) throw new Error("artifact ZIP failed CRC, path, size or regular-file validation");
}

export function verifyArtifactIndex(root) {
  if (!lstatSync(root).isDirectory()) throw new Error("archive root must be a regular directory");
  const index = JSON.parse(regular(join(root, "artifact-index.json")));
  if (index.schema_version !== "opencoven.threads-artifact-index/v1" || !Array.isArray(index.files) || index.files.length === 0) {
    throw new Error("a nonempty original artifact index is required");
  }
  const expected = new Map();
  for (const file of index.files) {
    if (!validPath(file.path) || file.path === "artifact-index.json" || expected.has(file.path) ||
        !Number.isSafeInteger(file.bytes) || file.bytes < 0 || !/^[0-9a-f]{64}$/.test(file.sha256)) {
      throw new Error("ambiguous or invalid artifact file index");
    }
    expected.set(file.path, file);
  }
  const seen = new Set();
  const visit = (directory, prefix = "") => {
    for (const name of readdirSync(directory).sort()) {
      const path = join(directory, name), relative = prefix + name;
      if (relative === "artifact-index.json") continue;
      const stat = lstatSync(path);
      if (stat.isDirectory()) visit(path, relative + "/");
      else {
        const bytes = regular(path), record = expected.get(relative);
        if (!record || record.bytes !== bytes.length || record.sha256 !== hash(bytes)) {
          throw new Error("missing, extra or modified indexed evidence");
        }
        seen.add(relative);
      }
    }
  };
  visit(root);
  if (seen.size !== expected.size) throw new Error("indexed evidence is missing");
  return index;
}

export function publishArchive(staging, destination) {
  const proposed = verifyArtifactIndex(staging);
  const compare = () => {
    const existing = verifyArtifactIndex(destination);
    const canonical = index => JSON.stringify([...index.files].sort((a, b) => a.path < b.path ? -1 : a.path > b.path ? 1 : 0));
    if (canonical(existing) !== canonical(proposed)) throw new Error("conflicting archive identity; original bytes preserved");
    rmSync(staging, { recursive: true });
    return { state: "identical", destination };
  };
  if (existsSync(destination)) return compare();
  mkdirSync(dirname(destination), { recursive: true });
  try { renameSync(staging, destination); }
  catch (error) {
    if (["EEXIST", "ENOTEMPTY"].includes(error.code) && existsSync(destination)) return compare();
    throw error;
  }
  return { state: "created", destination };
}
