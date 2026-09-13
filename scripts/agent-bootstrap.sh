#!/usr/bin/env bash
set -Eeuo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

readonly REQUIRED_TOOLCHAIN="1.88.0"

if ! command -v rustup >/dev/null 2>&1; then
  echo "error: rustup is required to install the repository-pinned Rust toolchain" >&2
  exit 1
fi

rustup toolchain install "$REQUIRED_TOOLCHAIN" \
  --profile minimal \
  --component rustfmt \
  --component clippy

actual="$(rustc "+${REQUIRED_TOOLCHAIN}" --version)"
case "$actual" in
  "rustc ${REQUIRED_TOOLCHAIN} "*) ;;
  *)
    echo "error: expected rustc ${REQUIRED_TOOLCHAIN}, got: ${actual}" >&2
    exit 1
    ;;
esac

# Fetch only the dependency graph committed in Cargo.lock. Verification uses
# --locked and must not rewrite dependency resolution.
cargo "+${REQUIRED_TOOLCHAIN}" fetch --locked

cat <<EOF
Bootstrap complete.

Fast verification:
  bash scripts/agent-check.sh fast

Full verification:
  bash scripts/agent-check.sh full
EOF
