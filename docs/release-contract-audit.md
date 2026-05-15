# Release Contract Audit

Audit issue: `#30`  
Audit date: `2026-05-15` (America/Los_Angeles)

## Full Gate Contract (Deterministic Order)

`./scripts/check-all.sh` full mode must execute:

1. `cargo fmt --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo test`
4. `cargo llvm-cov --all-features --workspace --fail-under-lines 90 --ignore-filename-regex 'src/(main|bin/puml-lsp)\.rs'`
5. `cargo build --release`
6. `./scripts/bench.sh --enforce-gates`

Quick mode contract:

- `./scripts/check-all.sh --quick` skips coverage + release build.
- Quick mode still enforces benchmark perf and binary-size gates.
- Regression gate semantics are two-part: percentage (`10%` full / `20%` quick) plus absolute slowdown floors (`>20ms` full / `>30ms` quick).
- Regression baselines are mode-scoped (`docs/benchmarks/baseline_full.json`, `docs/benchmarks/baseline_quick.json`) to prevent cross-mode noise.
- Baselines only move with explicit `--update-baseline`.

## Contract Guards Added

- Script gate enforcement:
  - [x] `scripts/check-all.sh` full mode now includes `cargo build --release`.
  - [x] `scripts/check-all.sh` full mode scopes coverage to core workspace files and excludes CLI entrypoint binaries (`src/main.rs`, `src/bin/puml-lsp.rs`) while keeping the 90% line gate.
- Deterministic regression checks:
  - [x] `tests/release_contract_audit.rs` validates required full-gate command ordering.
  - [x] `tests/release_contract_audit.rs` verifies release docs mention coverage + release build contract and pins scoped coverage regex.
  - [x] `tests/fixtures/contract/release_gate_full_commands.txt` is the canonical command-order fixture.
- Documentation sync:
  - [x] `README.md` now documents full + quick gate usage.
  - [x] `docs/release-checklist.md` now includes explicit full-gate command contract.
  - [x] `docs/decision-log.md` includes D-014 for release-build validation policy.

## Epic #30 Closure Evidence

- `./scripts/check-all.sh` passed end-to-end (fmt, clippy, tests, scoped 90% coverage gate, release build, full benchmark gates).
- `./scripts/check-all.sh --quick` passed end-to-end (fmt, clippy, tests, quick benchmark gates).
- `./scripts/harness-check.sh` passed (agent-pack contracts, MCP smoke checks, parity harness).
- `tests/svg_bounds_audit.rs` now enforces docs-example parity closure with `doc_examples.summary.failed == 0`.
- `scripts/parity_harness.py` now canonicalizes trailing SVG newlines before equality checks, removing false drift from stdin-vs-artifact newline differences.

## Remaining Known Deviations

- None in current branch context for full/quick gate execution.
