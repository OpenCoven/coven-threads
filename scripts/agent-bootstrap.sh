#!/usr/bin/env bash
set -Eeuo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

readonly REQUIRED_TOOLCHAIN="1.88.0"

if ! command -v node >/dev/null 2>&1; then
  echo "error: install Node $(cat .node-version) before bootstrap" >&2
  exit 1
fi
node scripts/check-ci-pins.mjs "$REQUIRED_TOOLCHAIN"

if ! command -v python3 >/dev/null 2>&1 || ! python3 -I -c 'import tomllib' 2>/dev/null; then
  echo "error: Python 3.11+ with standard-library tomllib is required for compatibility metadata" >&2
  exit 1
fi

if ! command -v rustup >/dev/null 2>&1; then
  echo "error: rustup is required to install the repository-pinned Rust toolchain" >&2
  if command -v rustc >/dev/null 2>&1; then
    echo "note: found $(rustc --version) at $(command -v rustc), which cannot select ${REQUIRED_TOOLCHAIN}." >&2
    echo "      A package-manager Rust without rustup fails the gate's pin check." >&2
  fi
  echo "install rustup (https://rustup.rs, or 'brew install rustup' on macOS), put its" >&2
  echo "toolchain proxies first on PATH, then re-run this bootstrap" >&2
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

# Bootstrap is the only step manifest network_policy (bootstrap-only) lets
# fetch, so it also provisions the checksum-pinned scanner the gate's secret
# scans need and the documentation website's locked dependencies. The gate
# itself never fetches; it reports either one missing instead.
tooling_bin="$ROOT/.tooling/bin"
if command -v gitleaks >/dev/null 2>&1; then
  echo "gitleaks: using $(command -v gitleaks)"
elif [[ -x "$tooling_bin/gitleaks" ]]; then
  echo "gitleaks: using $tooling_bin/gitleaks"
else
  case "$(uname -s)/$(uname -m)" in
    Linux/x86_64|Darwin/arm64)
      bash scripts/install-gitleaks.sh "$tooling_bin"
      ;;
    *)
      echo "notice: no pinned gitleaks build for $(uname -s)/$(uname -m); the gate" >&2
      echo "        will skip its secret scans and CI \"Secret scanning\" still enforces them" >&2
      ;;
  esac
fi

if command -v npm >/dev/null 2>&1; then
  npm ci --ignore-scripts --no-audit --no-fund --prefix website
else
  echo "notice: npm not found; the gate will skip the documentation website checks" >&2
fi

path_hint=""
if [[ -x "$tooling_bin/gitleaks" ]] && ! command -v gitleaks >/dev/null 2>&1; then
  path_hint="
Put the pinned scanner on PATH first:
  export PATH=\"$tooling_bin:\$PATH\"
"
fi

cat <<EOF
Bootstrap complete.
${path_hint}
Fast verification:
  bash scripts/agent-check.sh fast

Full verification:
  bash scripts/agent-check.sh full
EOF
