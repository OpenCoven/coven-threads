#!/usr/bin/env bash
set -Eeuo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
version="$(tr -d '\r\n' < "$ROOT/.gitleaks-version")"
[[ "$version" == "8.30.1" ]] || { echo "unsupported Gitleaks version" >&2; exit 1; }
case "$(uname -s)/$(uname -m)" in
  Linux/x86_64)
    platform=linux_x64
    checksum=551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb
    ;;
  Darwin/arm64)
    platform=darwin_arm64
    checksum=b40ab0ae55c505963e365f271a8d3846efbc170aa17f2607f13df610a9aeb6a5
    ;;
  *) echo "unsupported Gitleaks platform" >&2; exit 1 ;;
esac
destination="${1:?usage: bash scripts/install-gitleaks.sh DESTINATION}"
mkdir -p "$destination"
scratch="$(mktemp -d)"
trap 'rm -f "$scratch/archive.tar.gz" "$scratch/gitleaks"; rmdir "$scratch"' EXIT
curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 \
  "https://github.com/gitleaks/gitleaks/releases/download/v${version}/gitleaks_${version}_${platform}.tar.gz" \
  -o "$scratch/archive.tar.gz"
printf '%s  %s\n' "$checksum" "$scratch/archive.tar.gz" | shasum -a 256 --check --status
tar -xzf "$scratch/archive.tar.gz" -C "$scratch" gitleaks
install -m 755 "$scratch/gitleaks" "$destination/gitleaks"
