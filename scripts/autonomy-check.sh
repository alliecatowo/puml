#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

MODE="default"
BENCH_FLAGS=()
PARITY_FLAGS=()

usage() {
  cat <<'USAGE'
Usage: ./scripts/autonomy-check.sh [--quick] [--dry]

Runs full autonomous chain:
  fmt check -> clippy -> tests -> bench -> parity harness -> agent-pack smoke/contract checks

Options:
  --quick  run reduced benchmark/parity loops
  --dry    run dry-capable steps and skip heavy compile/lint/test execution
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --quick)
      MODE="quick"
      BENCH_FLAGS+=("--quick")
      PARITY_FLAGS+=("--quick")
      shift
      ;;
    --dry)
      MODE="dry"
      BENCH_FLAGS+=("--dry")
      PARITY_FLAGS+=("--dry")
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "[autonomy] unknown option: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

echo "[autonomy] mode=$MODE"
if [[ "$MODE" == "dry" ]]; then
  echo "[autonomy] dry mode: skipping cargo fmt/clippy/test"
else
  echo "[autonomy] cargo fmt --check"
  cargo fmt --check

  echo "[autonomy] cargo clippy --all-targets --all-features -- -D warnings"
  cargo clippy --all-targets --all-features -- -D warnings

  echo "[autonomy] cargo test"
  cargo test
fi

echo "[autonomy] benchmark step ${BENCH_FLAGS[*]:-(default)}"
bash ./scripts/bench.sh "${BENCH_FLAGS[@]}"

echo "[autonomy] harness step ${PARITY_FLAGS[*]:-(default)}"
bash ./scripts/harness-check.sh "${PARITY_FLAGS[@]}"

echo "[autonomy] complete"
