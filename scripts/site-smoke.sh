#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

usage() {
  cat <<'USAGE'
Usage: ./scripts/site-smoke.sh [--require-wasm]

Builds the static site and runs the inline Markdown graph/toggle smoke.

Options:
  --require-wasm  also require the built puml-wasm bundle under site/public/wasm
USAGE
}

for arg in "$@"; do
  case "$arg" in
    --require-wasm)
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "[site-smoke] unknown option: $arg" >&2
      usage >&2
      exit 1
      ;;
  esac
done

require_cmd() {
  local cmd="$1"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "[site-smoke] missing required command: $cmd" >&2
    exit 1
  fi
}

cd "$ROOT_DIR"

require_cmd node
require_cmd zola

echo "[site-smoke] build example manifest"
node scripts/build-site.mjs

echo "[site-smoke] zola build"
(
  cd site
  zola build
)

echo "[site-smoke] inline graph/toggle smoke"
node site/scripts/smoke-inline-fence-preview.mjs site "$@"
