#!/usr/bin/env bash
# Regenerate every diagram in this directory from src/*.mmd.
#
# One command, deterministic: each src/NAME.mmd renders to NAME.svg and a 2x
# NAME.png, both through the shared mermaid-config.json theme (explicit fills
# on every node, cluster, edge label, and note, so output is legible on both
# light and dark page backgrounds despite the transparent canvas).
#
# Toolchain (pinned):
#   - @mermaid-js/mermaid-cli 11.16.0, installed locally at
#     slides/community-explainer/node_modules (never global)
#   - a Chromium for puppeteer-core: $PUPPETEER_EXECUTABLE_PATH if set,
#     else Playwright's cached Chromium, else Google Chrome
set -euo pipefail
cd "$(dirname "$0")"

MMDC="../../slides/community-explainer/node_modules/.bin/mmdc"
if [ ! -x "$MMDC" ]; then
  echo "mermaid-cli not found at $MMDC — run 'npm install' in slides/community-explainer first" >&2
  exit 1
fi

resolve_chromium() {
  if [ -n "${PUPPETEER_EXECUTABLE_PATH:-}" ]; then
    echo "$PUPPETEER_EXECUTABLE_PATH"
    return
  fi
  local candidate
  for candidate in \
    "$HOME"/Library/Caches/ms-playwright/chromium-*/chrome-mac*/Chromium.app/Contents/MacOS/Chromium \
    "$HOME"/.cache/ms-playwright/chromium-*/chrome-linux/chrome \
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
    /usr/bin/chromium /usr/bin/chromium-browser /usr/bin/google-chrome; do
    if [ -x "$candidate" ]; then
      echo "$candidate"
      return
    fi
  done
  echo ""
}

CHROMIUM="$(resolve_chromium)"
if [ -z "$CHROMIUM" ]; then
  echo "no Chromium found for puppeteer; set PUPPETEER_EXECUTABLE_PATH" >&2
  exit 1
fi

PUPPETEER_CONFIG="$(mktemp -t coven-diagrams-puppeteer.XXXXXX.json)"
trap 'rm -f "$PUPPETEER_CONFIG"' EXIT
printf '{ "executablePath": %s, "args": ["--no-sandbox"] }\n' \
  "$(printf '%s' "$CHROMIUM" | python3 -c 'import json,sys; print(json.dumps(sys.stdin.read()))')" \
  > "$PUPPETEER_CONFIG"

echo "chromium: $CHROMIUM"
for src in src/*.mmd; do
  name="$(basename "$src" .mmd)"
  echo "render: $name"
  "$MMDC" -i "$src" -o "$name.svg" -b transparent -c mermaid-config.json -p "$PUPPETEER_CONFIG" --quiet
  "$MMDC" -i "$src" -o "$name.png" -b transparent -s 2 -c mermaid-config.json -p "$PUPPETEER_CONFIG" --quiet
done
echo "done: $(ls -1 ./*.svg | wc -l | tr -d ' ') svg, $(ls -1 ./*.png | wc -l | tr -d ' ') png"
