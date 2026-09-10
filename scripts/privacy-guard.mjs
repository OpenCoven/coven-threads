#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

// Categories adapted from OpenCoven's guards; no line or file exemptions.
const rules = [
  ["coven-session-key", /agent:[a-z0-9_-]+:(?:telegram|imessage|discord|whatsapp|signal|webchat):[a-z]+:[^\s"']+/],
  ["messenger-chat-id", /(?:telegram|imessage|discord|whatsapp|signal):(?:direct:)?\d{6,}/],
  ["absolute-home-path", /(?:\/Users\/|\/home\/)[a-z0-9._-]+/i],
  ["runtime-internal-path", /~\/\.(?:openclaw|coven)\/(?:agents|workspaces|credentials|sessions)[^\s"']*/],
  ["phone-number", /(?<!\d)\+[1-9]\d{7,14}(?!\d)/],
  ["invite-or-handoff-url", /https?:\/\/[^\s"']*(?:invite|handoff|ts\.net)[^\s"']*token[^\s"']*/],
];

export function git(...args) {
  const result = spawnSync("git", args, { maxBuffer: 64 * 1024 * 1024 });
  if (result.error || result.status !== 0) throw new Error("git-input-unavailable");
  return result.stdout;
}

export function* indexedFiles() {
  if (git("rev-parse", "--is-inside-work-tree").toString().trim() !== "true" ||
      git("rev-parse", "--show-prefix").toString().trim()) {
    throw new Error("repository-root-required");
  }
  const records = new TextDecoder("utf-8", { fatal: true })
    .decode(git("ls-files", "--stage", "-z")).split("\0");
  for (const record of records) {
    if (!record) continue;
    const match = record.match(/^(\d{6}) ([0-9a-f]{40,64}) ([0-3])\t([\s\S]+)$/);
    if (!match || match[3] !== "0") throw new Error("invalid-or-unmerged-index");
    const [, mode, oid, , path] = match;
    if (!["100644", "100755"].includes(mode)) throw new Error("unsupported-index-mode");
    if (path.startsWith("/") || path.split("/").some((part) => [".", "..", ".git"].includes(part))) {
      throw new Error("unsafe-index-path");
    }
    yield { path, data: git("cat-file", "blob", oid) };
  }
}

export function privacyFindings(data) {
  // Latin-1 preserves every byte, including NULs and invalid UTF-8.
  const text = data.toString("latin1");
  return rules.filter(([, pattern]) => pattern.test(text)).map(([name]) => name);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    if (process.argv.length !== 2) throw new Error("unsupported-arguments");
    const counts = new Map();
    for (const { path, data } of indexedFiles()) {
      const findings = new Set([
        ...privacyFindings(Buffer.from(path)),
        ...privacyFindings(data),
      ]);
      for (const rule of findings) counts.set(rule, (counts.get(rule) ?? 0) + 1);
    }
    for (const [rule, count] of counts) console.error(`privacy-guard: ${rule}: ${count} indexed file(s)`);
    console.log(counts.size ? "privacy-guard: BLOCKED" : "privacy-guard: clean index");
    process.exitCode = counts.size ? 1 : 0;
  } catch {
    // Git/decoder errors can contain private paths. Never forward raw diagnostics.
    console.error("privacy-guard: input unavailable or unsupported; fail closed");
    process.exitCode = 2;
  }
}
