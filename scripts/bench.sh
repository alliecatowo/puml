#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

printf 'Running placeholder CLI benchmarks for puml\n'
printf 'Project root: %s\n' "$ROOT_DIR"

printf '\nBuilding release binary for benchmark runs...\n'
cargo build --quiet --release --manifest-path "$ROOT_DIR/Cargo.toml"
BIN="$ROOT_DIR/target/release/puml"

printf '\n[1/5] single diagram text output\n'
time "$BIN" "$ROOT_DIR/tests/fixtures/single_valid.puml" >/dev/null

printf '\n[2/5] single diagram json output\n'
time "$BIN" --format json "$ROOT_DIR/tests/fixtures/single_valid.puml" >/dev/null

printf '\n[3/5] multi diagram json output\n'
time "$BIN" --multi "$ROOT_DIR/tests/fixtures/structure/multi_three.puml" >/dev/null

printf '\n[4/5] check mode success path\n'
time "$BIN" --check "$ROOT_DIR/tests/fixtures/basic/valid_start_end.puml" >/dev/null

printf '\n[5/5] check mode failure path (expected non-zero)\n'
time "$BIN" --check "$ROOT_DIR/tests/fixtures/errors/invalid_plain.txt" >/dev/null 2>&1 || true

printf '\nDone. For rigorous timing, run this repeatedly on an idle machine and prefer a dedicated tool (e.g. hyperfine).\n'
