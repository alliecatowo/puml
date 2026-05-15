#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

require_cmd() {
  local cmd="$1"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "[setup] missing required command: $cmd" >&2
    exit 1
  fi
}

ensure_component() {
  local component="$1"
  if rustup component list --installed | grep -qx "$component"; then
    echo "[setup] rustup component present: $component"
  else
    echo "[setup] installing rustup component: $component"
    rustup component add "$component"
  fi
}

ensure_cargo_tool() {
  local tool="$1"
  local crate_name="$2"
  if cargo "$tool" --version >/dev/null 2>&1; then
    echo "[setup] cargo-$tool already installed"
  else
    echo "[setup] installing cargo-$crate_name"
    cargo install "$crate_name"
  fi
}

require_cmd cargo
require_cmd rustup

cd "$ROOT_DIR"

echo "[setup] preparing Rust toolchain components"
ensure_component rustfmt
ensure_component clippy
ensure_component llvm-tools-preview

echo "[setup] preparing cargo tooling"
ensure_cargo_tool llvm-cov llvm-cov

echo "[setup] fetching and building workspace"
cargo fetch
cargo build

echo "[setup] complete"
echo "[setup] next: ./scripts/check-all.sh"
