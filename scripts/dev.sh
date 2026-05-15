#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "[dev] running fast local loop"
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test

echo "[dev] complete"
