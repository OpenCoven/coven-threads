import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { lstatSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";

export function parseTestCounts(output, { allowFailure = false } = {}) {
  const results = [...output.matchAll(/^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out; finished in [^\r\n]+$/gm)];
  if (results.length !== 1) throw new Error("expected exactly one complete libtest result");
  const [, status, ...values] = results[0];
  const [passed, failed, ignored, measured, filtered] = values.map(Number);
  if (![passed, failed, ignored, measured, filtered].every(value => Number.isSafeInteger(value) && value >= 0)) {
    throw new Error("invalid libtest counts");
  }
  if (!allowFailure && (status !== "ok" || passed <= 0 ||
      [failed, ignored, measured, filtered].some(value => value !== 0))) {
    throw new Error("all tests must pass with nonzero cases and no ignored, measured or filtered cases");
  }
  return { passed, failed, ignored, measured, filtered };
}

function regularFile(path) {
  if (!lstatSync(path).isFile()) throw new Error("evidence must be a regular file, never a symlink");
  return readFileSync(path);
}

// Parse XML with the standard library, rather than treating a success-looking
// attribute or an escaped failure element in log text as a JUnit verdict.
const verifyXml = `import json, sys, xml.etree.ElementTree as ET
rows = json.load(sys.stdin)
for row in rows:
    text = row['xml']
    if '<!' in text:
        raise ValueError('DTD and declaration content is unsupported')
    root = ET.fromstring(text)
    if root.tag != 'testsuite' or root.get('tests') != '1' or root.get('failures') != '0':
        raise ValueError('expected one passing testsuite case')
    if root.get('errors', '0') != '0' or root.get('skipped', '0') != '0':
        raise ValueError('errors or skips cannot qualify')
    if any(e.tag not in ('testsuite', 'testcase') for e in root.iter()) or len(list(root.iter('testsuite'))) != 1:
        raise ValueError('unsupported nested or additional JUnit elements')
    cases = list(root.iter('testcase'))
    if len(cases) != 1 or cases[0].get('name') != row['scenario'] or cases[0].get('classname') != 'threads_e2e':
        raise ValueError('case identity differs from manifest')
    if any(e.tag in ('failure', 'error', 'skipped') for e in root.iter()):
        raise ValueError('contradictory failure, error or skip')
print(json.dumps({'cases': len(rows)}))
`;

export function qualifyJourneys(root, expected) {
  if (!lstatSync(root).isDirectory()) throw new Error("journey root must be a regular directory");
  const entries = readdirSync(root).sort();
  if (entries.length === 0) throw new Error("no journey evidence captured");
  const rows = [];
  const runIds = new Set();
  if (!Array.isArray(expected.expected_scenarios) || expected.expected_scenarios.length === 0 ||
      new Set(expected.expected_scenarios).size !== expected.expected_scenarios.length) {
    throw new Error("an explicit unique pinned scenario inventory is required");
  }
  for (const name of entries) {
    const directory = join(root, name);
    if (!lstatSync(directory).isDirectory()) throw new Error("journey entries must be regular directories");
    const m = JSON.parse(regularFile(join(directory, "manifest.json")));
    if (m.run_id !== basename(directory) || runIds.has(m.run_id) ||
        typeof m.scenario !== "string" || !m.scenario || m.result !== "passed" ||
        m.setup_completed !== true || m.fixture_evidence_captured !== true ||
        m.local_threads_override_active !== true || m.threads_dirty !== false ||
        m.coven_commit !== expected.coven_sha || m.threads_commit !== expected.threads_sha ||
        m.coven_lock_sha256 !== expected.coven_lock_overlay_sha256 || m.platform !== expected.platform) {
      throw new Error(`journey ${name} has incomplete, failed or mismatched provenance`);
    }
    runIds.add(m.run_id);
    for (const capture of ["request.json", "response.json"]) {
      JSON.parse(regularFile(join(directory, capture)));
    }
    if (regularFile(join(directory, "logs", "daemon.log")).length === 0) {
      throw new Error("missing captured daemon log");
    }
    rows.push({ scenario: m.scenario, xml: regularFile(join(directory, "junit.xml")).toString("utf8") });
  }
  const actualScenarios = rows.map(row => row.scenario).sort();
  if (JSON.stringify(actualScenarios) !== JSON.stringify([...expected.expected_scenarios].sort())) {
    throw new Error("captured journeys differ from the complete pinned platform scenario inventory");
  }
  const parsed = spawnSync("python3", ["-I", "-c", verifyXml], {
    input: JSON.stringify(rows), encoding: "utf8", maxBuffer: 16 * 1024 * 1024,
  });
  if (parsed.error || parsed.status !== 0) throw new Error("journey JUnit is missing, invalid or contradicts passing manifests");
  const cases = JSON.parse(parsed.stdout).cases;
  if (cases !== rows.length) throw new Error("journey JUnit count differs from manifest count");
  return { manifests: rows.length, junit_cases: cases, scenarios: [...new Set(rows.map(row => row.scenario))].sort() };
}

export function readJourneyInventory(path, coven, pin, platform) {
  const inventory = JSON.parse(regularFile(path));
  if (inventory.schema_version !== "opencoven.threads-native-scenarios/v1" || inventory.coven_ref !== pin ||
      !["linux", "macos", "windows"].includes(platform) ||
      !inventory.source_sha256 || Object.keys(inventory.source_sha256).length === 0) {
    throw new Error("unsupported or mismatched native scenario inventory");
  }
  for (const [source, digest] of Object.entries(inventory.source_sha256)) {
    if (!/^crates\/coven-cli\/tests\/(?:support\/)?threads_[a-z0-9_]+\.rs$/.test(source) ||
        !/^[0-9a-f]{64}$/.test(digest)) {
      throw new Error("native scenario inventory does not match its pinned daemon source content");
    }
    const contents = regularFile(join(coven, source)).toString("latin1").replace(/\r\n/g, "\n");
    if (createHash("sha256").update(contents, "latin1").digest("hex") !== digest) {
      throw new Error("native scenario inventory does not match its pinned daemon source content");
    }
  }
  const groups = [inventory.common, ...(platform !== "windows" ? [inventory.unix] : []),
    ...(platform === "linux" ? [inventory.linux] : [])];
  if (groups.some(group => !Array.isArray(group) || group.some(name => typeof name !== "string" || !name))) {
    throw new Error("malformed native scenario inventory groups");
  }
  const scenarios = groups.flat();
  if (scenarios.length === 0 || new Set(scenarios).size !== scenarios.length) throw new Error("ambiguous native scenario inventory");
  return scenarios.sort();
}

export function writeArtifactIndex(root) {
  const files = [];
  const visit = (directory, prefix = "") => {
    for (const name of readdirSync(directory).sort()) {
      const path = join(directory, name);
      const relative = prefix + name;
      if (relative === "artifact-index.json") continue;
      const stat = lstatSync(path);
      if (stat.isDirectory()) visit(path, relative + "/");
      else {
        const contents = regularFile(path);
        files.push({ path: relative, bytes: contents.length,
          sha256: createHash("sha256").update(contents).digest("hex") });
      }
    }
  };
  if (!lstatSync(root).isDirectory()) throw new Error("artifact root must be a regular directory");
  visit(root);
  const index = { schema_version: "opencoven.threads-artifact-index/v1", files };
  writeFileSync(join(root, "artifact-index.json"), JSON.stringify(index, null, 2) + "\n", { flag: "wx" });
  return index;
}
