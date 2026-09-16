#!/usr/bin/env bash
set -Eeuo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

mode="${1:-fast}"
case "$mode" in
  fast|full) ;;
  *)
    echo "usage: bash scripts/agent-check.sh [fast|full]" >&2
    exit 2
    ;;
esac

for command in cargo rustc git node python3; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "error: missing required command: $command" >&2
    exit 1
  fi
done

# scripts/daemon-compatibility.mjs parses the manifest with Python's tomllib,
# which is 3.11+. macOS ships 3.9 at /usr/bin/python3, so an unguarded run
# fails later as an opaque assertion instead of a missing prerequisite.
if ! python3 -c "import tomllib" >/dev/null 2>&1; then
  echo "error: python3 at $(command -v python3) lacks tomllib (requires 3.11+)" >&2
  echo "note: daemon-compatibility manifest parsing needs it; put a 3.11+ python3 first on PATH" >&2
  exit 1
fi

toolchain_pin="$(sed -n 's/^channel = "\([^"]*\)"/\1/p' rust-toolchain.toml)"
cargo_msrv="$(sed -n 's/^rust-version = "\([^"]*\)"/\1/p' Cargo.toml)"
actual_rust="$(rustc --version | awk '{print $2}')"

if [[ -z "$toolchain_pin" || -z "$cargo_msrv" ]]; then
  echo "error: could not read Rust pins from rust-toolchain.toml and Cargo.toml" >&2
  exit 1
fi

if [[ "$actual_rust" != "$toolchain_pin" ]]; then
  echo "error: selected rustc is ${actual_rust}; repository requires ${toolchain_pin}" >&2
  echo "run: bash scripts/agent-bootstrap.sh" >&2
  exit 1
fi

if [[ "${toolchain_pin%.*}" != "$cargo_msrv" ]]; then
  echo "error: rust-toolchain.toml (${toolchain_pin}) and Cargo.toml MSRV (${cargo_msrv}) diverge" >&2
  exit 1
fi

node scripts/check-ci-pins.mjs "$toolchain_pin"
node --test scripts/tests/*.test.mjs

# Required-check parity. "Privacy policy guard" and "Secret scanning" gate main
# through the repository ruleset, so the documented local gate must be able to
# reproduce their failures rather than passing and deferring the red run to CI.
#
# Both guards read the git INDEX, not the working tree. A fix is invisible to
# them until it is staged, so warn when tracked files carry unstaged changes.
if ! git diff --quiet; then
  unstaged_count="$(git diff --name-only | wc -l | tr -d ' ')"
  echo "notice: ${unstaged_count} tracked file(s) have unstaged changes; the guards" >&2
  echo "        below read the git index, so those edits are not being scanned:" >&2
  git diff --name-only | head -5 | sed 's/^/          /' >&2
  if (( unstaged_count > 5 )); then
    echo "          ... and $((unstaged_count - 5)) more" >&2
  fi
  echo "        stage them before trusting a clean guard result." >&2
fi

node scripts/privacy-guard.mjs

# secret-guard needs the checksum-pinned scanner. network_policy is
# bootstrap-only, so never fetch it here; report precisely instead.
if command -v gitleaks >/dev/null 2>&1; then
  node --test scripts/tests/secret-guard.integration.mjs
  node scripts/secret-guard.mjs
else
  echo "notice: gitleaks not on PATH; skipping the secret scans this gate would" >&2
  echo "        otherwise run. CI check \"Secret scanning\" still enforces them," >&2
  echo "        so a clean local gate is NOT sufficient evidence here." >&2
  echo "        install the pinned scanner, then re-run:" >&2
  echo "          bash scripts/install-gitleaks.sh \"\$PWD/.tooling/bin\"" >&2
  echo "          export PATH=\"\$PWD/.tooling/bin:\$PATH\"" >&2
fi

if ! grep -Eq '^ref = "[0-9a-f]{40}"$' e2e/compatibility.toml; then
  echo "error: e2e/compatibility.toml must pin Coven to a full 40-character commit SHA" >&2
  exit 1
fi

bash -n scripts/agent-bootstrap.sh scripts/agent-check.sh
git diff --check
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
node --test profiles/automation-authority/v1/tests/*.test.mjs
node profiles/automation-authority/v1/run-vectors.mjs

if [[ "$mode" == "fast" ]]; then
  cargo test --locked -p coven-threads-core --lib
else
  cargo test --locked --workspace
fi

# Verification must not rewrite dependency resolution.
git diff --exit-code -- Cargo.lock
