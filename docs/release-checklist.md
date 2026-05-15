# Release Checklist

## Pre-Release

- [ ] Confirm target version in `Cargo.toml` is ready.
- [ ] Run setup if this machine is fresh: `./scripts/setup.sh`.
- [ ] Run full gate: `./scripts/check-all.sh`.
- [ ] Run benchmark suite: `./scripts/bench.sh`.
- [ ] Review benchmark artifacts in `docs/benchmarks/latest.{md,csv,json}` for unexpected regressions.

## Contract and Docs

- [ ] `README.md` reflects current CLI behavior and command entry points.
- [ ] `docs/decision-log.md` includes new intentional contract changes.
- [ ] `docs/coverage-status.md` updated if coverage posture changed materially.
- [ ] `docs/parity-roadmap.md` reflects current parity priorities.
- [ ] Troubleshooting and fixture/snapshot docs still match workflow.
- [ ] `--help` text is consistent with docs for `--check`, `--dump`, `--multi`, and `--include-root`.
- [ ] Documented exit codes (`0/1/2/3`) still match observed CLI behavior for success, validation, I/O, and internal failures.
- [ ] Warning UX still matches docs: warnings print to `stderr` and do not flip successful runs to non-zero.

## Verification

- [ ] Smoke test render from file input.
- [ ] Smoke test stdin + `--check`.
- [ ] Smoke test `--dump scene` and `--multi`.
- [ ] Validate includes workflow with `--include-root` in stdin mode.
- [ ] Run `scripts/bench.sh --quick` in non-Java mode and verify cold-start/parser/render rows are produced.

## Publish Readiness

- [ ] Changelog/release notes prepared.
- [ ] Tag/versioning workflow confirmed.
- [ ] Final PR merged with green gate.
