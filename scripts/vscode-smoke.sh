#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "[vscode-smoke] check puml capability manifest includes extension commands"
capabilities="$(cargo run -- --dump-capabilities)"
echo "$capabilities" | rg '"puml.applyFormat"' >/dev/null
echo "$capabilities" | rg '"puml.renderSvg"' >/dev/null

echo "[vscode-smoke] build extension scaffold"
(
  cd extensions/vscode
  if [[ ! -d node_modules ]]; then
    npm install
  fi
  npm run build
  npm run smoke
)

echo "[vscode-smoke] complete"
