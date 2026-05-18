#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

MODE="${1:-quick}"

require_cmd() {
  local cmd="$1"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "[local-gate] missing required command: $cmd" >&2
    exit 1
  fi
}

require_cmd cargo

echo "[local-gate] mode=$MODE"

echo "[local-gate] cargo fmt"
cargo fmt

echo "[local-gate] cargo fmt --check"
cargo fmt --check

echo "[local-gate] cargo clippy --all-targets --all-features -- -D warnings"
cargo clippy --all-targets --all-features -- -D warnings

if [[ "$MODE" == "quick" ]]; then
  echo "[local-gate] cargo test --lib --quiet"
  cargo test --lib --quiet
  exit 0
fi

if [[ "$MODE" != "strict" ]]; then
  echo "[local-gate] unknown mode: $MODE" >&2
  exit 1
fi

echo "[local-gate] cargo test"
cargo test

echo "[local-gate] changed-file coverage gate"
./scripts/coverage-changed.sh

if command -v python3 >/dev/null 2>&1; then
  echo "[local-gate] parity harness quick doc-drift check"
  python3 ./scripts/parity_harness.py --quick --quiet --fail-on-doc-drift
else
  echo "[local-gate] python3 not found; skipping parity harness quick check"
fi

echo "[local-gate] strict gate complete"

