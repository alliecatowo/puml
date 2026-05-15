#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

MODE="default"
PARITY_FLAGS=()

usage() {
  cat <<'USAGE'
Usage: ./scripts/harness-check.sh [--quick] [--dry]

Options:
  --quick  run reduced parity corpus for fast local validation
  --dry    print planned commands and execute dry-capable harness checks only
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --quick)
      MODE="quick"
      PARITY_FLAGS+=("--quick")
      shift
      ;;
    --dry)
      MODE="dry"
      PARITY_FLAGS+=("--dry")
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "[harness] unknown option: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

echo "[harness] mode=$MODE"

echo "[harness] validating agent-pack manifests/contracts"
python3 ./scripts/validate_agent_pack.py

echo "[harness] running MCP smoke checks"
bash ./agent-pack/tests/mcp_smoke.sh

echo "[harness] running parity harness ${PARITY_FLAGS[*]:-(full)}"
python3 ./scripts/parity_harness.py "${PARITY_FLAGS[@]}"

echo "[harness] complete"
