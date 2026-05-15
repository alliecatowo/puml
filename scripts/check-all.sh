#!/usr/bin/env bash
set -euo pipefail

cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo llvm-cov --all-features --workspace --fail-under-lines 90
