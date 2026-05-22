#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "[dev] running fast local loop"
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings

if ! cargo nextest --version >/dev/null 2>&1; then
  echo "[dev] cargo-nextest is required; run ./scripts/setup.sh or cargo install cargo-nextest --locked" >&2
  exit 1
fi

cargo nextest run
cargo test --doc

echo "[dev] complete"
