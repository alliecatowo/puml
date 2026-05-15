#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

printf 'Running placeholder CLI benchmarks for puml\n'
printf 'Project root: %s\n' "$ROOT_DIR"

printf '\n[1/3] single diagram text output\n'
time cargo run --quiet --manifest-path "$ROOT_DIR/Cargo.toml" -- "$ROOT_DIR/tests/fixtures/single_valid.puml" >/dev/null

printf '\n[2/3] multi diagram json output\n'
time cargo run --quiet --manifest-path "$ROOT_DIR/Cargo.toml" -- --multi "$ROOT_DIR/tests/fixtures/multi_valid.puml" >/dev/null

printf '\n[3/3] check mode\n'
time cargo run --quiet --manifest-path "$ROOT_DIR/Cargo.toml" -- --check "$ROOT_DIR/tests/fixtures/single_valid.puml" >/dev/null

printf '\nDone. Replace with hyperfine/criterion for stable benchmarking.\n'
