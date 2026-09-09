#!/usr/bin/env node

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

function scalar(value) {
  const literal = value.replace(/\s+#.*$/, "").trim();
  return literal.replace(/^(["'])(.*)\1$/, "$2");
}

export function checkCiPins(workflow, toolchain) {
  if (!/^\d+\.\d+\.\d+$/.test(toolchain)) {
    throw new Error("expected an explicit Rust toolchain version");
  }

  const lines = workflow.split(/\r?\n/);
  let rustActions = 0;
  for (let index = 0; index < lines.length; index += 1) {
    const action = lines[index].match(/^(\s*)(-\s+)?uses:\s*(.+)$/);
    if (!action) continue;
    const reference = scalar(action[3]);
    if (reference.startsWith("./")) continue;
    if (!/^[\w.-]+\/[\w./-]+@[0-9a-f]{40}$/.test(reference)) {
      throw new Error(`line ${index + 1}: external action requires a full commit SHA: ${reference}`);
    }
    if (!reference.startsWith("dtolnay/rust-toolchain@")) continue;

    rustActions += 1;
    const keyIndent = action[1].length + (action[2]?.length ?? 0);
    let inWith = false;
    const versions = [];
    // Inspect this step's block-style `with` mapping, not another step or env.
    for (let next = index + 1; next < lines.length; next += 1) {
      const line = lines[next];
      if (!line.trim() || line.trimStart().startsWith("#")) continue;
      const indent = line.length - line.trimStart().length;
      if (indent < keyIndent) break;
      if (indent === keyIndent) {
        inWith = /^with:\s*(?:#.*)?$/.test(line.trim());
      } else if (inWith && indent === keyIndent + 2) {
        const input = line.trim().match(/^toolchain:\s*(.*)$/);
        if (input) versions.push(scalar(input[1]));
      }
    }
    if (versions.length !== 1 || versions[0] !== toolchain) {
      throw new Error(
        `line ${index + 1}: Rust action requires exactly one with.toolchain: ${toolchain}`,
      );
    }
  }
  if (rustActions === 0) {
    throw new Error("CI must declare a SHA-pinned Rust toolchain action");
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const workflow = readFileSync(
    new URL("../.github/workflows/ci.yml", import.meta.url),
    "utf8",
  );
  checkCiPins(workflow, process.argv[2]);
}
