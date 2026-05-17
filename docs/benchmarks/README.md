# Benchmarks

## Commands

```console
# full benchmark artifact refresh (records trend, does not fail on gates)
./scripts/bench.sh

# quick local benchmark
./scripts/bench.sh --quick

# enforce perf + binary gates (used by check-all)
./scripts/bench.sh --enforce-gates
./scripts/bench.sh --quick --enforce-gates

# refresh mode baseline only after an intentional review
./scripts/bench.sh --update-baseline
./scripts/bench.sh --quick --update-baseline

# corpus parity baseline report (oracle placeholders kept intentionally)
python3 scripts/parity_harness.py --output docs/benchmarks/parity_latest.json

# differential oracle smoke report (PlantUML runtime required)
python3 scripts/differential_oracle_smoke.py --quick --strict --output docs/benchmarks/oracle_smoke_latest.json
```

## Artifacts

- `docs/benchmarks/latest.md`
- `docs/benchmarks/latest.csv`
- `docs/benchmarks/latest.json`
- `docs/benchmarks/latest_trend.md`
- `docs/benchmarks/latest_trend.json`
- `docs/benchmarks/baseline_full.json`
- `docs/benchmarks/baseline_quick.json`
- `docs/benchmarks/parity_latest.json`
- `docs/benchmarks/oracle_smoke_latest.json`

All benchmark artifacts are deterministic in structure and key ordering. Value fields like timestamps, host metadata, and measured timings are expected to change run-to-run.

## Gate Profiles

- `full` (default):
- absolute per-scenario mean limit: `250ms`
- regression limit vs previous `baseline_full.json`: `10%` with absolute delta floor `>20ms`
- binary size limit (`target/release/puml`): `2,000,000` bytes
- `quick` (`--quick`):
- absolute per-scenario mean limit: `350ms`
- regression limit vs previous `baseline_quick.json`: `20%` with absolute delta floor `>30ms`
- binary size limit (`target/release/puml`): `2,500,000` bytes

If no matching mode baseline exists, regression checks are skipped and absolute/binary checks still apply.

## Failure Handling

- `./scripts/bench.sh` reports gate warnings but exits `0` by default.
- `./scripts/bench.sh --enforce-gates` exits non-zero on any gate failure.
- `./scripts/check-all.sh` always runs benchmark gates in enforce mode.
- On failure, inspect `docs/benchmarks/latest_trend.{md,json}` to identify the exact regressing scenario and delta.
- Baselines are not auto-updated. Use `--update-baseline` only after reviewing variance and approving movement.

## No-Java Baseline

- PlantUML oracle comparison is opt-in and remains skip-sentinel based unless `PUML_ORACLE_JAR` is set.
- `parity_latest.json`, `latest_trend.json`, and `oracle_report.json` may include explicit oracle placeholder or skip metadata.
- Oracle artifacts are comparison evidence only. They are not runtime inputs and do not imply that Java/PlantUML is used by the `puml` CLI or library.
- Do not remove placeholders until Java/oracle execution is intentionally enabled for the relevant workflow.

## Differential Oracle Smoke

- CI workflow: `.github/workflows/differential-oracle-smoke.yml`
- Gate mode: deterministic smoke checks only (small fixed corpus, token-presence checks, viewBox checks, structured JSON report).
- Runtime requirements: `plantuml`, `java` (headless), and `graphviz`.
