#!/usr/bin/env node

import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { lstatSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, realpathSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { isDeepStrictEqual } from "node:util";
import { classifyMetadata, qualifyReport, summarizeRetained } from "./lib/daemon-retention.mjs";
import { extractArtifact, publishArchive, verifyArtifactIndex } from "./lib/daemon-artifact-archive.mjs";
import { parseTestCounts, qualifyJourneys, writeArtifactIndex } from "./lib/daemon-reliability-evidence.mjs";
import { qualifyTerminalCaptures } from "./lib/daemon-window-terminals.mjs";

const repository = "OpenCoven/coven-threads";
const inventory = JSON.parse(readFileSync(new URL("../e2e/native-daemon-scenarios.json", import.meta.url)));
const hash = bytes => createHash("sha256").update(bytes).digest("hex");

function verifySnapshots(run, after, jobs, jobsAfter, artifacts, artifactsAfter) {
  const ordered = rows => [...rows].sort((a, b) => a.id - b.id);
  if (!isDeepStrictEqual(run, after) || !isDeepStrictEqual(ordered(jobs), ordered(jobsAfter)) ||
      !isDeepStrictEqual(ordered(artifacts), ordered(artifactsAfter))) {
    throw new Error("complete authoritative snapshots changed during collection");
  }
}

function githubRead(endpoint) {
  const result = spawnSync("gh", ["api", endpoint], { encoding: null, maxBuffer: 256 * 1024 * 1024 });
  if (result.error || result.status !== 0) throw new Error("authenticated GitHub REST read failed");
  return result.stdout;
}

function collectorIdentity() {
  const sources = ["retain-daemon-evidence.mjs", "lib/daemon-retention.mjs", "lib/daemon-artifact-archive.mjs",
    "lib/extract-daemon-artifact.py", "lib/daemon-reliability-evidence.mjs", "lib/daemon-window-terminals.mjs", "run-daemon-canary.mjs",
    "../e2e/native-daemon-scenarios.json"];
  const python = spawnSync("python3", ["-I", "-c", "import sys; assert sys.version_info >= (3,11); print(sys.version.split()[0])"], { encoding: "utf8" });
  if (python.error || python.status !== 0) throw new Error("Python 3.11+ is required for retained evidence validation");
  return { node: process.version, python: python.stdout.trim(),
    source_sha256: Object.fromEntries(sources.map(path => [path, hash(readFileSync(new URL(path, import.meta.url)))])) };
}

function qualifyExtracted(root, row) {
  verifyArtifactIndex(root);
  const report = JSON.parse(readFileSync(join(root, "reliability.json")));
  const observation = JSON.parse(readFileSync(join(root, "observation.json")));
  const targets = JSON.parse(readFileSync(join(root, "targets.json")));
  if (JSON.stringify(targets) !== JSON.stringify(report.target_runs)) throw new Error("contradictory target receipts");
  const proof = qualifyReport(row, report, observation);
  for (const target of report.target_runs) {
    const actual = parseTestCounts(readFileSync(join(root, `${target.target}.stdout.log`), "utf8"));
    if (JSON.stringify(actual) !== JSON.stringify(target.counts)) throw new Error("counts contradict original libtest output");
  }
  const expected = [...inventory.common, ...(row.platform !== "windows" ? inventory.unix : []),
    ...(row.platform === "linux" ? inventory.linux : [])];
  const journeys = qualifyJourneys(join(root, "journeys"), { coven_sha: report.coven_sha,
    threads_sha: report.threads_sha, coven_lock_overlay_sha256: report.coven_lock_overlay_sha256,
    platform: row.platform, expected_scenarios: expected });
  if (JSON.stringify(journeys) !== JSON.stringify(report.journeys)) throw new Error("journey summary contradicts captured evidence");
  return { ...proof, journeys };
}

function packet(record) {
  return `# Native daemon retained attempt ${record.run_id}/${record.attempt}\n\n` +
    `Source: ${record.threads_sha}. Event: ${record.event}. Created: ${record.created_at}. Started: ${record.started_at ?? "not started"}.\n\n` +
    "| OS | Job outcome | Evidence | First-attempt qualified |\n| --- | --- | --- | --- |\n" +
    record.rows.map(row => `| ${row.os} | ${row.job?.conclusion ?? "missing"} | ${row.state} / ${row.artifact_state ?? "absent"} | ${row.first_attempt_qualified} |`).join("\n") +
    "\n\nOriginal metadata and downloaded ZIP bytes are retained with file hashes. Reruns do not replace first attempts. This packet does not establish missing schedule slots, a 30-day rate, core Nextest results, deployment or human acceptance.\n";
}

export function collectAttempt(runId, attempt, archiveRoot, read = githubRead) {
  if (!Number.isSafeInteger(runId) || runId <= 0 || !Number.isSafeInteger(attempt) || attempt <= 0) throw new Error("positive explicit run ID and attempt required");
  mkdirSync(resolve(archiveRoot), { recursive: true });
  if (!lstatSync(resolve(archiveRoot)).isDirectory()) throw new Error("archive root must be a regular directory");
  const root = realpathSync(archiveRoot);
  const staging = mkdtempSync(join(root, `.collect-${runId}-${attempt}-`));
  const snapshots = join(staging, "metadata"); mkdirSync(snapshots);
  const snapshot = (name, endpoint) => {
    const bytes = read(endpoint);
    writeFileSync(join(snapshots, name), bytes, { flag: "wx" });
    return JSON.parse(bytes);
  };
  const pages = (name, endpoint, key) => {
    const result = [];
    let total;
    for (let page = 1; page <= 100; page++) {
      const data = snapshot(`${name}-${page}.json`, `${endpoint}?per_page=100&page=${page}`);
      if (!Number.isSafeInteger(data.total_count) || data.total_count < 0 || !Array.isArray(data[key])) throw new Error("invalid paginated metadata");
      total ??= data.total_count;
      if (total !== data.total_count) throw new Error("metadata changed during pagination");
      result.push(...data[key]);
      if (result.length === total) return result;
      if (result.length > total || data[key].length === 0) throw new Error("incomplete or conflicting metadata pages");
    }
    throw new Error("metadata exceeds collection bound");
  };
  let record;
  try {
    const prefix = `repos/${repository}/actions/runs/${runId}`;
    const run = snapshot("run.json", `${prefix}/attempts/${attempt}`);
    if (run.id !== runId || run.run_attempt !== attempt) throw new Error("requested attempt differs from authoritative response");
    const jobs = pages("jobs", `${prefix}/attempts/${attempt}/jobs`, "jobs");
    const artifacts = pages("artifacts", `${prefix}/artifacts`, "artifacts");
    record = classifyMetadata({ run, jobs, artifacts });
    record.collector = collectorIdentity();
    for (const row of record.rows) {
      row.first_attempt_qualified = false; row.execution_passed = false;
      row.metadata_state = row.state;
      if (!row.artifact || row.artifact.expired !== false) continue;
      const directory = join(staging, row.os); mkdirSync(directory);
      try {
        const bytes = read(`repos/${repository}/actions/artifacts/${row.artifact.id}/zip`);
        const zip = join(directory, "original.zip");
        writeFileSync(zip, bytes, { flag: "wx" });
        row.original_zip_sha256 = hash(bytes);
        const extracted = join(directory, "evidence");
        extractArtifact(zip, row.artifact.digest?.slice(7), extracted);
        Object.assign(row, qualifyExtracted(extracted, row));
        row.artifact_state = "verified_evidence";
        if (row.first_attempt_qualified) row.state = "qualified";
      } catch {
        row.artifact_state = row.original_zip_sha256 ? "unqualified_evidence" : "download_failed";
        if (row.state === "artifact_pending") row.state = row.original_zip_sha256 ? "artifact_validation_failed" : "artifact_download_failed";
      }
    }
    const jobsAfter = pages("jobs-after", `${prefix}/attempts/${attempt}/jobs`, "jobs");
    const artifactsAfter = pages("artifacts-after", `${prefix}/artifacts`, "artifacts");
    const after = snapshot("run-after.json", `${prefix}/attempts/${attempt}`);
    verifySnapshots(run, after, jobs, jobsAfter, artifacts, artifactsAfter);
    writeFileSync(join(staging, "record.json"), JSON.stringify(record, null, 2) + "\n", { flag: "wx" });
    writeFileSync(join(staging, "packet.md"), packet(record), { flag: "wx" });
  } catch (error) {
    writeFileSync(join(staging, "collection-failure.json"), JSON.stringify({ run_id: runId, attempt,
      status: "collection_failed", reason: "metadata_or_archive_validation_failed" }, null, 2) + "\n", { flag: "wx" });
    writeArtifactIndex(staging);
    throw new Error("collection refused; incomplete snapshot retained outside canonical attempts", { cause: error });
  }
  writeArtifactIndex(staging);
  const publication = publishArchive(staging, join(root, String(runId), String(attempt)));
  return { ...publication, record };
}

function savedPages(metadata, prefix, key) {
  const names = readdirSync(metadata).filter(name => new RegExp(`^${prefix}-[1-9][0-9]*\\.json$`).test(name))
    .sort((a, b) => Number(a.match(/-(\d+)\./)[1]) - Number(b.match(/-(\d+)\./)[1]));
  const rows = [];
  let total;
  names.forEach((name, index) => {
    if (name !== `${prefix}-${index + 1}.json`) throw new Error("retained metadata pages are incomplete");
    const page = JSON.parse(readFileSync(join(metadata, name)));
    total ??= page.total_count;
    if (!Number.isSafeInteger(total) || total < 0 || page.total_count !== total || !Array.isArray(page[key])) throw new Error("retained metadata pages disagree");
    rows.push(...page[key]);
  });
  if (rows.length !== total) throw new Error("retained metadata count is incomplete");
  return rows;
}

export function readRetainedArchive(archiveRoot, {terminalEvidence=false}={}) {
  const root = resolve(archiveRoot);
  if (!lstatSync(root).isDirectory()) throw new Error("archive root must be a regular directory");
  const records = [];
  let separateCollections = 0;
  for (const runId of readdirSync(root).sort()) {
    if (runId.startsWith(".collect-")) { separateCollections += 1; continue; }
    if (!/^[1-9][0-9]*$/.test(runId) || !lstatSync(join(root, runId)).isDirectory()) throw new Error("unknown archive identity");
    for (const attempt of readdirSync(join(root, runId)).sort()) {
      if (!/^[1-9][0-9]*$/.test(attempt)) throw new Error("unknown retained attempt identity");
      const directory = join(root, runId, attempt);
      verifyArtifactIndex(directory);
      const record = JSON.parse(readFileSync(join(directory, "record.json")));
      const metadata = join(directory, "metadata");
      const run = JSON.parse(readFileSync(join(metadata, "run.json")));
      const jobs = savedPages(metadata, "jobs", "jobs"), artifacts = savedPages(metadata, "artifacts", "artifacts");
      verifySnapshots(run, JSON.parse(readFileSync(join(metadata, "run-after.json"))),
        jobs, savedPages(metadata, "jobs-after", "jobs"), artifacts, savedPages(metadata, "artifacts-after", "artifacts"));
      const original = classifyMetadata({ run, jobs, artifacts });
      if (record.run_id !== Number(runId) || record.attempt !== Number(attempt) ||
          record.run_id !== original.run_id || record.attempt !== original.attempt || record.threads_sha !== original.threads_sha ||
          record.event !== original.event || record.created_at !== original.created_at || record.started_at !== original.started_at ||
          record.first_attempt !== original.first_attempt || !Array.isArray(record.rows) || record.rows.length !== original.rows.length) {
        throw new Error("archive record contradicts original metadata identity");
      }
      for (const row of record.rows) {
        const expected = original.rows.find(candidate => candidate.os === row.os);
        if (!expected || row.metadata_state !== expected.state || !isDeepStrictEqual(row.job, expected.job) ||
            !isDeepStrictEqual(row.artifact, expected.artifact)) {
          throw new Error("archive row contradicts original job/artifact identity");
        }
        if (row.artifact_state !== undefined && !["verified_evidence", "unqualified_evidence", "download_failed"].includes(row.artifact_state)) {
          throw new Error("unknown retained artifact state");
        }
        if (!row.execution_passed && !row.first_attempt_qualified) {
          const derived = row.metadata_state === "artifact_pending"
            ? { unqualified_evidence: "artifact_validation_failed", download_failed: "artifact_download_failed" }[row.artifact_state]
            : row.metadata_state;
          if (row.state !== derived || row.artifact_state === "verified_evidence") throw new Error("negative row contradicts original evidence state");
          continue;
        }
        const scratch = mkdtempSync(join(tmpdir(), "threads-retention-audit-"));
        try {
          const evidence = join(scratch, "evidence");
          extractArtifact(join(directory, row.os, "original.zip"), expected.artifact?.digest?.slice(7), evidence);
          const proof = qualifyExtracted(evidence, expected);
          if (proof.execution_passed !== row.execution_passed || proof.first_attempt_qualified !== row.first_attempt_qualified ||
              JSON.stringify(proof.source_series) !== JSON.stringify(row.source_series) || JSON.stringify(proof.counts) !== JSON.stringify(row.counts) ||
              !isDeepStrictEqual(proof.journeys,row.journeys)) {
            throw new Error("retained qualification contradicts original ZIP evidence");
          }
          if(terminalEvidence) {
            try {row.terminal_proof=qualifyTerminalCaptures(join(evidence,"journeys"));}
            catch {row.terminal_proof={qualified:false,reason:"missing_or_invalid_terminal_capture"};}
          }
        } finally { rmSync(scratch, { recursive: true }); }
      }
      records.push(record);
    }
  }
  return {records,separately_retained_collections:separateCollections};
}

export function summarizeArchive(archiveRoot) {
  const {records,separately_retained_collections}=readRetainedArchive(archiveRoot);
  return {...summarizeRetained(records),separately_retained_collections};
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    if (process.argv.length === 4 && process.argv[2] === "--summary") {
      console.log(JSON.stringify(summarizeArchive(process.argv[3]), null, 2));
    } else {
    if (process.argv.length !== 5 || !/^\d+$/.test(process.argv[2]) || !/^\d+$/.test(process.argv[3])) throw new Error("usage: node scripts/retain-daemon-evidence.mjs RUN_ID ATTEMPT ARCHIVE_ROOT");
    const result = collectAttempt(Number(process.argv[2]), Number(process.argv[3]), process.argv[4]);
    console.log(`${result.state}: ${result.record.rows.filter(row => row.first_attempt_qualified).length}/3 qualified first-attempt OS observations`);
    }
  } catch (error) { console.error(error.message); process.exitCode = 1; }
}
