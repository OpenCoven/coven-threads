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

for command in cargo rustc git; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "error: missing required command: $command" >&2
    exit 1
  fi
done

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

if ! grep -Fq "dtolnay/rust-toolchain@${toolchain_pin}" .github/workflows/ci.yml; then
  echo "error: CI Rust pin does not match rust-toolchain.toml (${toolchain_pin})" >&2
  exit 1
fi

if ! grep -Eq '^ref = "[0-9a-f]{40}"$' e2e/compatibility.toml; then
  echo "error: e2e/compatibility.toml must pin Coven to a full 40-character commit SHA" >&2
  exit 1
fi

bash -n scripts/agent-bootstrap.sh scripts/agent-check.sh
git diff --check
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings

if [[ "$mode" == "fast" ]]; then
  cargo test --locked -p coven-threads-core --lib
else
  cargo test --locked --workspace
fi

# Verification must not rewrite dependency resolution.
git diff --exit-code -- Cargo.lock
