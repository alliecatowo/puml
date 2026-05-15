# Release Checklist

## Pre-Release

- [ ] Confirm target version in `Cargo.toml` is ready.
- [ ] Run setup if this machine is fresh: `./scripts/setup.sh`.
- [ ] Run full gate: `./scripts/check-all.sh`.
- [ ] Run quick gate once for local perf sanity: `./scripts/check-all.sh --quick`.
- [ ] If benchmark gates fail, inspect `docs/benchmarks/latest_trend.{md,json}` and either optimize or document/approve baseline movement before rerun.

## Benchmark / Perf / Size Contract

- [ ] Confirm full gate thresholds were applied (abs mean `<=250ms`, regression `<=10%`, binary size `<=2,000,000` bytes).
- [ ] Confirm quick gate thresholds were applied (abs mean `<=350ms`, regression `<=20%`, binary size `<=2,500,000` bytes).
- [ ] Review `docs/benchmarks/latest.{md,csv,json}` for raw measurements.
- [ ] Review deterministic trend artifacts: `docs/benchmarks/latest_trend.{md,json}`.
- [ ] Verify no-Java baseline is intact: PlantUML oracle fields are still placeholder-only (`todo`).

## Contract and Docs

- [ ] `README.md` reflects current CLI behavior and command entry points.
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
