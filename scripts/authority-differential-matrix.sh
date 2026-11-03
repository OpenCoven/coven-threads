#!/usr/bin/env bash
set -Eeuo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
mode="${1:-all}"
case "$mode" in
  debug) profiles=(debug) ;;
  release) profiles=(release) ;;
  all) profiles=(debug release) ;;
  *) echo "usage: bash scripts/authority-differential-matrix.sh [debug|release|all]" >&2; exit 2 ;;
esac

matrix_rust_pin="$(sed -n 's/^channel = "\([^"]*\)"/\1/p' rust-toolchain.toml)"
node scripts/check-ci-pins.mjs "$matrix_rust_pin"
if [[ "$(rustc --version | awk '{print $2}')" != "$matrix_rust_pin" ]]; then
  echo "error: differential matrix requires rustc $matrix_rust_pin" >&2
  exit 1
fi
mkdir -p target
evidence="$(mktemp -d "$ROOT/target/authority-differential.XXXXXX")"
printf 'Differential artifacts: %s\n' "$evidence"
{
  git rev-parse HEAD
  git write-tree
  git status --porcelain=v1 --untracked-files=no
  rustc --version
  cargo --version
  node -p 'JSON.stringify({node:process.version,v8:process.versions.v8,openssl:process.versions.openssl,platform:process.platform,arch:process.arch})'
} > "$evidence/tools.txt"

for profile in "${profiles[@]}"; do
  # Nonempty argument arrays also work under macOS's Bash 3.2 with nounset.
  profile_args=(--profile dev)
  debug_assertions=true
  if [[ "$profile" == release ]]; then
    profile_args=(--release)
    debug_assertions=false
  fi
  for feature in ordinary arbitrary_precision; do
    feature_args=(--features "")
    arbitrary_precision=false
    if [[ "$feature" == arbitrary_precision ]]; then
      feature_args=(--features serde_json/arbitrary_precision)
      arbitrary_precision=true
    fi
    destination="$evidence/$profile-$feature"
    mkdir -p "$destination"
    # These are direct guard tests, not externally reachable-code claims.
    cargo test --locked -p coven-threads-core "${profile_args[@]}" "${feature_args[@]}" --lib automation_authority 2>&1 | tee "$destination/guards.log"
    cargo test --locked -p coven-threads-core "${profile_args[@]}" "${feature_args[@]}" --test automation_authority_vectors 2>&1 | tee "$destination/vectors.log"
    cargo build --locked -p coven-threads-core "${profile_args[@]}" "${feature_args[@]}" --example automation_authority_batch --example automation_authority_keyring_batch 2>&1 | tee "$destination/build.log"
    for binary in automation_authority_batch automation_authority_keyring_batch; do
      cp "${CARGO_TARGET_DIR:-target}/$profile/examples/$binary" "$destination/$binary"
      "$destination/$binary" --describe > "$destination/$binary.build.json"
      node --input-type=module - "$destination/$binary.build.json" "$arbitrary_precision" "$debug_assertions" <<'JS'
import { readFileSync } from "node:fs";
const build = JSON.parse(readFileSync(process.argv[2], "utf8"));
if (build.serde_json_arbitrary_precision !== (process.argv[3] === "true") || build.debug_assertions !== (process.argv[4] === "true")) {
  throw new Error(`batch artifact does not match the selected feature/profile: ${JSON.stringify(build)}`);
}
JS
    done
    node scripts/automation-authority-differential.mjs "$destination/automation_authority_batch" 2>&1 | tee "$destination/general.log"
    node scripts/automation-authority-keyring-differential.mjs "$destination/automation_authority_keyring_batch" --ordered 2>&1 | tee "$destination/keyring.log"
  done
done

node --input-type=module - "$evidence" "${profiles[@]}" <<'JS'
import { appendFileSync, readFileSync, writeFileSync } from "node:fs";
import { validateMatrix } from "./scripts/lib/authority-matrix-receipt.mjs";
import { join } from "node:path";
const root = process.argv[2];
const entries = [];
const profiles = process.argv.slice(3);
for (const profile of profiles) {
  for (const feature of ["ordinary", "arbitrary_precision"]) {
    const directory = join(root, `${profile}-${feature}`);
    const generalLine = readFileSync(join(directory, "general.log"), "utf8").split("\n").find((line) => line.startsWith("Evidence: "));
    const keyringLine = readFileSync(join(directory, "keyring.log"), "utf8").split("\n").find((line) => line.startsWith('{"node":'));
    if (!generalLine || !keyringLine) throw new Error("matrix receipt is missing");
    const general = JSON.parse(generalLine.slice("Evidence: ".length));
    const keyring = JSON.parse(keyringLine);
    entries.push({ profile, feature, general, keyring });
  }
}
const manifest = JSON.parse(readFileSync("profiles/automation-authority/v1/manifest.json", "utf8"));
const hashes = validateMatrix(entries, profiles, [...new Set(manifest.vectors.map((vector) => vector.operation))], process.version);
if (process.env.GITHUB_OUTPUT) appendFileSync(process.env.GITHUB_OUTPUT, `general_input_sha256=${hashes.generalInputSha256}\nkeyring_input_sha256=${hashes.keyringInputSha256}\n`);
writeFileSync(join(root, "matrix.json"), JSON.stringify({ entries }, null, 2) + "\n");
console.log(`Verified ${entries.length} nonzero matrix cells; identical input digests across profiles/features. Receipt: ${join(root, "matrix.json")}`);
JS
