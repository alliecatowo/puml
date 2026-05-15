#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

require_cmd() {
  local cmd="$1"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "[gate] missing required command: $cmd" >&2
    exit 1
  fi
}

cd "$ROOT_DIR"

require_cmd cargo

if ! cargo llvm-cov --version >/dev/null 2>&1; then
  echo "[gate] cargo-llvm-cov is required for the full quality gate." >&2
  echo "[gate] run ./scripts/setup.sh or: cargo install cargo-llvm-cov" >&2
  exit 1
fi

echo "[gate] cargo fmt --check"
cargo fmt --check

echo "[gate] cargo clippy --all-targets --all-features -- -D warnings"
cargo clippy --all-targets --all-features -- -D warnings

echo "[gate] cargo test"
cargo test

echo "[gate] cargo llvm-cov --all-features --workspace --fail-under-lines 90"
cargo llvm-cov --all-features --workspace --fail-under-lines 90

echo "[gate] complete"
