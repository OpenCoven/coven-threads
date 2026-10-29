#!/usr/bin/env node

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

function scalar(value) {
  const literal = value.replace(/\s+#.*$/, "").trim();
  return literal.replace(/^(["'])(.*)\1$/, "$2");
}

function requireUnconditionalNodeStep(line) {
  const property = line.trim().replace(/^-\s+/, "").match(/^["']?(if|continue-on-error)["']?\s*:\s*(.*)$/);
  if (!property) return;
  if (property[1] === "if") throw new Error("Node setup must be unconditional");
  if (scalar(property[2]) !== "false") throw new Error("Node setup cannot use continue-on-error");
}

export function checkCiPins(workflow, toolchain, nodeVersion = null) {
  if (!/^\d+\.\d+\.\d+$/.test(toolchain)) {
    throw new Error("expected an explicit Rust toolchain version");
  }

  const lines = workflow.split(/\r?\n/);
  let rustActions = 0;
  let nodeActions = 0;
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index].replace(/\s+#.*$/, "");
    const action = line.match(/^(\s*)(-\s+)?uses:\s*(.+)$/);
    if (!action) {
      if (!line.trimStart().startsWith("#") && /(?:^|[\s{,])["']?uses["']?\s*:/.test(line)) {
        throw new Error(`line ${index + 1}: actions require literal block-style uses declarations`);
      }
      continue;
    }
    const reference = scalar(action[3]);
    if (reference.startsWith("./")) continue;
    if (!/^[\w.-]+\/[\w./-]+@[0-9a-f]{40}$/.test(reference)) {
      throw new Error(`line ${index + 1}: external action requires a full commit SHA: ${reference}`);
    }
    const isRust = reference.startsWith("dtolnay/rust-toolchain@");
    const isNode = nodeVersion !== null && reference.startsWith("actions/setup-node@");
    if (!isRust && !isNode) continue;

    if (isRust) rustActions += 1;
    else nodeActions += 1;
    const inputName = isRust ? "toolchain" : "node-version";
    const expected = isRust ? toolchain : nodeVersion;
    const keyIndent = action[1].length + (action[2]?.length ?? 0);
    if (isNode && !action[2]) {
      // A named step can put its condition before `uses`.
      for (let previous = index - 1; previous >= 0; previous--) {
        const prior = lines[previous].replace(/\s+#.*$/, "");
        if (prior.trim().startsWith("- ")) {
          requireUnconditionalNodeStep(prior);
          break;
        }
        if (prior.trim() && prior.length - prior.trimStart().length < keyIndent) break;
        requireUnconditionalNodeStep(prior);
      }
    }
    let inWith = false;
    const versions = [];
    // Inspect this step's block-style `with` mapping, not another step or env.
    for (let next = index + 1; next < lines.length; next += 1) {
      const line = lines[next];
      if (!line.trim() || line.trimStart().startsWith("#")) continue;
      const indent = line.length - line.trimStart().length;
      if (indent < keyIndent) break;
      if (indent === keyIndent) {
        if (isNode) requireUnconditionalNodeStep(line);
        inWith = /^with:\s*(?:#.*)?$/.test(line.trim());
      } else if (inWith && indent === keyIndent + 2) {
        const input = line.trim().match(new RegExp(`^${inputName}:\\s*(.*)$`));
        if (input) versions.push(scalar(input[1]));
      }
    }
    if (versions.length !== 1 || versions[0] !== expected) {
      throw new Error(
        `line ${index + 1}: ${isRust ? "Rust" : "Node"} action requires exactly one with.${inputName}: ${expected}`,
      );
    }
  }
  if (rustActions === 0) {
    throw new Error("CI must declare a SHA-pinned Rust toolchain action");
  }
  if (nodeVersion !== null && nodeActions === 0) {
    throw new Error("CI must declare a SHA-pinned Node setup action");
  }
  if (nodeVersion !== null && workflow.includes("jobs:")) {
    const jobs = workflow.slice(workflow.indexOf("jobs:") + 5).split(/^  ([\w-]+):\s*$/m);
    for (let index = 1; index < jobs.length; index += 2) {
      const job = jobs[index];
      const body = jobs[index + 1].replace(/^\s*#.*$/gm, "");
      const firstNode = body.search(/\bnode\s+scripts\/|bash\s+scripts\/(?:agent-(?:check|bootstrap)|authority-differential-matrix)\.sh/);
      if (firstNode < 0) continue;
      const setup = body.indexOf("uses: actions/setup-node@");
      if (setup < 0 || setup > firstNode) throw new Error(`job ${job} must select Node before running scripts`);
    }
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const workflow = readFileSync(
    new URL("../.github/workflows/ci.yml", import.meta.url),
    "utf8",
  );
  const nodeVersion = readFileSync(new URL("../.node-version", import.meta.url), "utf8").trim();
  if (!/^\d+\.\d+\.\d+$/.test(nodeVersion) || process.versions.node !== nodeVersion) {
    throw new Error(`selected Node is ${process.versions.node}; repository requires ${nodeVersion}`);
  }
  checkCiPins(workflow, process.argv[2], nodeVersion);
  // The pinned real daemon uses its own existing Rust toolchain; do not
  // conflate that consumer pin with this library's Rust 1.88 MSRV.
  for (const name of ["daemon-compatibility.yml", "daemon-observation.yml"]) {
    checkCiPins(readFileSync(new URL(`../.github/workflows/${name}`, import.meta.url), "utf8"), "1.95.0", nodeVersion);
  }
}
