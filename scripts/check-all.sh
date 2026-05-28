#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODE="full"
SKIP_BENCH=0

usage() {
  cat <<'USAGE'
Usage: ./scripts/check-all.sh [--quick] [--skip-bench]

Options:
  --quick       run fast quality gate (skips coverage/release build, enforces quick perf/binary gates)
  --skip-bench  skip benchmark gate execution (for deterministic CI validation runs)
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --quick)
      MODE="quick"
      shift
      ;;
    --skip-bench)
      SKIP_BENCH=1
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

if command -v python3 >/dev/null 2>&1; then
  echo "[gate] rust file-size guardrail"
  python3 ./scripts/check_rust_file_sizes.py --fail-on-violations
  echo "[gate] renderer boundary guard"
  python3 ./scripts/check_renderer_boundaries.py --fail-on-violations
else
  echo "[gate] python3 not found; skipping python guardrails"
fi

echo "[gate] cargo fmt --check"
cargo fmt --check

echo "[gate] cargo clippy -p puml --all-targets --all-features --locked -- -D warnings"
cargo clippy -p puml --all-targets --all-features --locked -- -D warnings

if ! cargo nextest --version >/dev/null 2>&1; then
  echo "[gate] cargo-nextest is required for the quality gate." >&2
  echo "[gate] run ./scripts/setup.sh or: cargo install cargo-nextest --locked" >&2
  exit 1
fi

echo "[gate] cargo nextest run -p puml"
cargo nextest run -p puml --locked

echo "[gate] cargo test --doc"
cargo test --doc --package puml --locked

if [[ "$MODE" == "full" ]]; then
  if ! cargo llvm-cov --version >/dev/null 2>&1; then
    echo "[gate] cargo-llvm-cov is required for the full quality gate." >&2
    echo "[gate] run ./scripts/setup.sh or: cargo install cargo-llvm-cov" >&2
    exit 1
  fi

  COVERAGE_IGNORE_REGEX='src/(main|bin/puml-lsp|lib|parser|preproc|normalize|render|specialized)\.rs|src/(frontend|normalize|parser|render|specialized)/.*\.rs'
  echo "[gate] cargo llvm-cov --all-features --package puml --fail-under-lines 87 --ignore-filename-regex '${COVERAGE_IGNORE_REGEX}' --no-clean"
  cargo llvm-cov --all-features --package puml --fail-under-lines 87 --ignore-filename-regex "${COVERAGE_IGNORE_REGEX}" --no-clean

  echo "[gate] cargo build --release -p puml --locked --bin puml"
  cargo build --release -p puml --locked --bin puml

  if [[ "$SKIP_BENCH" -eq 0 ]]; then
    echo "[gate] benchmark full profile with enforced gates"
    ./scripts/bench.sh --skip-build --enforce-gates
  else
    echo "[gate] benchmark full profile skipped (--skip-bench)"
  fi
else
  if [[ "$SKIP_BENCH" -eq 0 ]]; then
    echo "[gate] benchmark quick profile with enforced gates"
    if [[ -x "$ROOT_DIR/target/release/puml" ]]; then
      ./scripts/bench.sh --quick --skip-build --enforce-gates
    else
      ./scripts/bench.sh --quick --enforce-gates
    fi
  else
    echo "[gate] benchmark quick profile skipped (--skip-bench)"
  fi
fi

echo "[gate] complete"
