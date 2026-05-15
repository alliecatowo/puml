#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODE="full"

usage() {
  cat <<'USAGE'
Usage: ./scripts/check-all.sh [--quick]

Options:
  --quick  run fast quality gate (skips coverage/release build, enforces quick perf/binary gates)
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
      echo "[gate] unknown option: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

require_cmd() {
  local cmd="$1"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "[gate] missing required command: $cmd" >&2
    exit 1
  fi
}

cd "$ROOT_DIR"

require_cmd cargo

echo "[gate] mode=$MODE"

echo "[gate] cargo fmt --check"
cargo fmt --check

echo "[gate] cargo clippy --all-targets --all-features -- -D warnings"
cargo clippy --all-targets --all-features -- -D warnings

echo "[gate] cargo test"
cargo test

if [[ "$MODE" == "full" ]]; then
  if ! cargo llvm-cov --version >/dev/null 2>&1; then
    echo "[gate] cargo-llvm-cov is required for the full quality gate." >&2
    echo "[gate] run ./scripts/setup.sh or: cargo install cargo-llvm-cov" >&2
    exit 1
  fi

  echo "[gate] cargo llvm-cov --all-features --workspace --fail-under-lines 90"
  cargo llvm-cov --all-features --workspace --fail-under-lines 90

  echo "[gate] cargo build --release"
  cargo build --release

  echo "[gate] benchmark full profile with enforced gates"
  ./scripts/bench.sh --enforce-gates
else
  echo "[gate] benchmark quick profile with enforced gates"
  ./scripts/bench.sh --quick --enforce-gates
fi

echo "[gate] complete"
