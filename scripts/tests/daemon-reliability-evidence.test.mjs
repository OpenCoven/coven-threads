import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

const modulePath = new URL("../lib/daemon-reliability-evidence.mjs", import.meta.url);

async function implementation() {
  assert(existsSync(modulePath), "native reliability evidence qualifier is not implemented");
  return import(modulePath);
}

test("libtest qualification refuses empty, repeated, failed, ignored or filtered results", async () => {
  const { parseTestCounts } = await implementation();
  const good = "test result: ok. 104 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.23s\n";
  assert.deepEqual(parseTestCounts(good), { passed: 104, failed: 0, ignored: 0, measured: 0, filtered: 0 });
  for (const output of ["", good + good, good.replace("104 passed", "0 passed"),
    good.replace("0 failed", "1 failed"), good.replace("0 ignored", "1 ignored"),
    good.replace("0 filtered out", "1 filtered out"), good.replace("ok.", "FAILED.")]) {
    assert.throws(() => parseTestCounts(output));
  }
});

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), "threads-reliability-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const journey = join(root, "journeys", "scenario-unique-run");
  mkdirSync(journey, { recursive: true });
  const expected = { coven_sha: "a".repeat(40), threads_sha: "b".repeat(40),
    coven_lock_overlay_sha256: "c".repeat(64), platform: "linux", expected_scenarios: ["bounded-write"] };
  const manifest = { run_id: "scenario-unique-run", scenario: "bounded-write",
    result: "passed", setup_completed: true, fixture_evidence_captured: true,
    coven_commit: "a".repeat(40), threads_commit: "b".repeat(40), threads_dirty: false,
    coven_lock_sha256: "c".repeat(64), local_threads_override_active: true, platform: "linux" };
  const xml = '<testsuite name="threads-e2e" tests="1" failures="0"><testcase classname="threads_e2e" name="bounded-write"/></testsuite>';
  const save = (m = manifest, x = xml) => {
    writeFileSync(join(journey, "manifest.json"), JSON.stringify(m));
    writeFileSync(join(journey, "junit.xml"), x);
    writeFileSync(join(journey, "request.json"), "null");
    writeFileSync(join(journey, "response.json"), "null");
    mkdirSync(join(journey, "logs"), { recursive: true });
    writeFileSync(join(journey, "logs", "daemon.log"), "synthetic daemon lifecycle\n");
  };
  save();
  return { root, journey, expected, manifest, xml, save };
}

test("journey qualification binds actual XML cases to source, lock, override and platform", async (t) => {
  const { qualifyJourneys } = await implementation();
  const f = fixture(t);
  assert.deepEqual(qualifyJourneys(join(f.root, "journeys"), f.expected), {
    manifests: 1, junit_cases: 1, scenarios: ["bounded-write"],
  });
});

for (const [field, value] of Object.entries({ result: "failed", setup_completed: false,
  fixture_evidence_captured: false, coven_commit: "d".repeat(40), threads_commit: "e".repeat(40),
  coven_lock_sha256: "f".repeat(64), local_threads_override_active: false,
  threads_dirty: true, platform: "windows", run_id: "different-directory" })) {
  test(`journeys refuse ${field} mismatch without editing the original evidence`, async (t) => {
    const { qualifyJourneys } = await implementation();
    const f = fixture(t);
    f.save({ ...f.manifest, [field]: value });
    const before = readFileSync(join(f.journey, "manifest.json"), "utf8");
    assert.throws(() => qualifyJourneys(join(f.root, "journeys"), f.expected));
    assert.equal(readFileSync(join(f.journey, "manifest.json"), "utf8"), before);
  });
}

test("missing/contradictory JUnit and incomplete evidence cannot qualify", async (t) => {
  const { qualifyJourneys } = await implementation();
  const f = fixture(t);
  const badXml = ["", "<testsuite>", f.xml.replace('tests="1"', 'tests="0"'),
    f.xml.replace('failures="0"', 'failures="1"'), f.xml.replace("<testcase", '<failure/><testcase'),
    f.xml.replace("<testcase", '<error/><testcase'), f.xml.replace("<testcase", '<skipped/><testcase'),
    f.xml.replace("bounded-write", "another-scenario"), f.xml.replace("<testcase", '<testcase/><testcase'),
    '<!DOCTYPE testsuite><testsuite tests="1" failures="0"/>'];
  badXml.push(f.xml.replace("</testsuite>", '<testsuite tests="1" failures="0"><testcase/></testsuite></testsuite>'));
  for (const xml of badXml) {
    f.save(f.manifest, xml);
    assert.throws(() => qualifyJourneys(join(f.root, "journeys"), f.expected));
  }
  rmSync(join(f.journey, "junit.xml"));
  assert.throws(() => qualifyJourneys(join(f.root, "journeys"), f.expected));
  rmSync(f.journey, { recursive: true });
  assert.throws(() => qualifyJourneys(join(f.root, "journeys"), f.expected));
  mkdirSync(join(f.root, "journeys", "incomplete-run"));
  assert.throws(() => qualifyJourneys(join(f.root, "journeys"), f.expected));
});

test("a missing whole scenario or capture file cannot qualify a partial collection", async (t) => {
  const { qualifyJourneys } = await implementation();
  const f = fixture(t);
  assert.throws(() => qualifyJourneys(join(f.root, "journeys"), {
    ...f.expected, expected_scenarios: ["bounded-write", "restart-drift"],
  }));
  rmSync(join(f.journey, "request.json"));
  assert.throws(() => qualifyJourneys(join(f.root, "journeys"), f.expected));
});

test("artifact index hashes regular files and refuses symlink evidence", async (t) => {
  const { writeArtifactIndex } = await implementation();
  const f = fixture(t);
  writeFileSync(join(f.root, "log.txt"), "abc");
  const index = writeArtifactIndex(f.root);
  assert.equal(index.files.find(x => x.path === "log.txt").sha256,
    "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
  assert.equal(index.files.find(x => x.path === "log.txt").bytes, 3);
  assert(!index.files.some(x => x.path === "artifact-index.json"));
  symlinkSync(join(f.root, "log.txt"), join(f.root, "linked.txt"));
  assert.throws(() => writeArtifactIndex(f.root), /regular|symlink/);
});

test("scenario inventory normalizes CRLF and refuses changed content, wrong pins and unsupported platforms", async (t) => {
  const { readJourneyInventory } = await implementation();
  const f = fixture(t);
  const source = join(f.root, "crates", "coven-cli", "tests", "threads_e2e.rs");
  mkdirSync(join(f.root, "crates", "coven-cli", "tests"), { recursive: true });
  const lfSource = "// π\n";
  writeFileSync(source, lfSource);
  const inventory = join(f.root, "inventory.json");
  writeFileSync(inventory, JSON.stringify({ schema_version: "opencoven.threads-native-scenarios/v1",
    coven_ref: "a".repeat(40), source_sha256: { "crates/coven-cli/tests/threads_e2e.rs":
      createHash("sha256").update(lfSource).digest("hex") },
    common: ["common-case"], unix: ["unix-case"], linux: ["linux-case"] }));
  assert.deepEqual(readJourneyInventory(inventory, f.root, "a".repeat(40), "windows"), ["common-case"]);
  writeFileSync(source, "// π\r\n");
  assert.deepEqual(readJourneyInventory(inventory, f.root, "a".repeat(40), "windows"), ["common-case"]);
  assert.deepEqual(readJourneyInventory(inventory, f.root, "a".repeat(40), "macos"), ["common-case", "unix-case"]);
  assert.deepEqual(readJourneyInventory(inventory, f.root, "a".repeat(40), "linux"), ["common-case", "linux-case", "unix-case"]);
  assert.throws(() => readJourneyInventory(inventory, f.root, "b".repeat(40), "linux"));
  assert.throws(() => readJourneyInventory(inventory, f.root, "a".repeat(40), "other"));
  writeFileSync(source, "changed");
  assert.throws(() => readJourneyInventory(inventory, f.root, "a".repeat(40), "linux"));
});
