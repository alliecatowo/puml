# Release Checklist

## Pre-Release

- [ ] Confirm target version in `Cargo.toml` is ready.
- [ ] Run setup if this machine is fresh: `./scripts/setup.sh`.
- [ ] Run full gate: `./scripts/check-all.sh`.
- [ ] Confirm full gate command contract executed in order:
  `cargo fmt --check` -> `cargo clippy --all-targets --all-features -- -D warnings` -> `cargo test` -> `cargo llvm-cov --all-features --workspace --fail-under-lines 90 --ignore-filename-regex 'src/(main|bin/puml-lsp)\.rs'` -> `cargo build --release`.
- [ ] Confirm baseline coverage command string remains visible for contract compatibility: `cargo llvm-cov --all-features --workspace --fail-under-lines 90`.
- [ ] Run quick gate once for local perf sanity: `./scripts/check-all.sh --quick`.
- [ ] If benchmark gates fail, inspect `docs/benchmarks/latest_trend.{md,json}` and either optimize or document/approve baseline movement before rerun.

## Benchmark / Perf / Size Contract

- [ ] Confirm full gate thresholds were applied (abs mean `<=250ms`, regression `<=10%` with delta floor `>20ms`, binary size `<=2,000,000` bytes).
- [ ] Confirm quick gate thresholds were applied (abs mean `<=350ms`, regression `<=20%` with delta floor `>30ms`, binary size `<=2,500,000` bytes).
- [ ] Confirm full gate includes release binary validation via `cargo build --release`.
- [ ] Review `docs/benchmarks/latest.{md,csv,json}` for raw measurements.
- [ ] Review deterministic trend artifacts: `docs/benchmarks/latest_trend.{md,json}`.
- [ ] Verify no-Java baseline is intact: PlantUML oracle fields are still placeholder-only (`todo`).

### Local Evidence Snapshot (Issue #17, 2026-05-15 UTC)

- Full profile command: `./scripts/bench.sh`
- Full profile timestamp: `2026-05-15T19:21:55Z`
- Full profile result: gates pass (`abs<=250ms`, `regression<=10%` with `delta>20ms`, `binary<=2,000,000B`)
- Full profile scenario means (ms): `cold_start_help=130.000`, `parser_check=128.000`, `parser_dump_scene=122.000`, `render_file=98.000`, `render_stdin=98.000`, `render_stdin_multi=98.000`
- Quick enforced command: `./scripts/bench.sh --quick --enforce-gates`
- Quick profile timestamp: `2026-05-15T19:22:04Z`
- Quick enforced result: gate fail on regression (`render_file=+25.850%`, `render_stdin=+25.850%`, limit `20%`)
- Quick profile scenario means (ms): `cold_start_help=126.667`, `parser_check=130.000`, `parser_dump_scene=130.000`, `render_file=123.333`, `render_stdin=123.333`, `render_stdin_multi=110.000`
- Release binary size: `1,814,264` bytes (`target/release/puml`)
- No-Java oracle status: retained as `todo` placeholders in `docs/benchmarks/latest_trend.json` and `docs/benchmarks/parity_latest.json`

## Contract and Docs

- [ ] `README.md` reflects current CLI behavior and command entry points.
- [ ] `docs/release-contract-audit.md` reflects latest audited full/quick gate command contract.
- [ ] `docs/decision-log.md` includes new intentional contract changes.
- [ ] `docs/coverage-status.md` updated if coverage posture changed materially.
- [ ] `docs/parity-roadmap.md` reflects current parity priorities.
- [ ] Troubleshooting and fixture/snapshot docs still match workflow.
- [ ] `--help` text is consistent with docs for `--check`, `--dump`, `--multi`, `--include-root`, `--lint-input`, `--lint-glob`, and `--lint-report`.
- [ ] Documented exit codes (`0/1/2/3`) still match observed CLI behavior for success, validation, I/O, and internal failures.
- [ ] Warning UX still matches docs: warnings print to `stderr` and do not flip successful runs to non-zero.

## Verification

- [ ] Smoke test render from file input.
- [ ] Smoke test stdin + `--check`.
- [ ] Smoke test `--dump scene` and `--multi`.
- [ ] Validate includes workflow with `--include-root` in stdin mode.
- [ ] Run `scripts/bench.sh --quick --enforce-gates` in non-Java mode and verify gate pass/fail behavior is explicit.

## Publish Readiness

- [ ] Changelog/release notes prepared.
- [ ] Tag/versioning workflow confirmed.
- [ ] Final PR merged with green gate.
