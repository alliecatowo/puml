#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

MODE="full"

usage() {
  cat <<'USAGE'
Usage: ./scripts/ecosystem-rollout-check.sh [--quick]

Options:
  --quick  run contract/unit checks and harness only (skip VS Code build smoke)
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --quick)
      MODE="quick"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "[ecosystem] unknown option: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

echo "[ecosystem] mode=$MODE"
echo "[ecosystem] contract audits (LSP/VS Code/Studio/agent-pack)"
cargo test --test ecosystem_rollout_contract_audit --test studio_spa_contract_audit

echo "[ecosystem] agent-pack + parity harness"
if [[ "$MODE" == "quick" ]]; then
  ./scripts/harness-check.sh --quick
else
  ./scripts/harness-check.sh
fi

if [[ "$MODE" == "full" ]]; then
  echo "[ecosystem] vscode smoke (LSP capability manifest + extension build)"
  ./scripts/vscode-smoke.sh
fi

echo "[ecosystem] complete"
