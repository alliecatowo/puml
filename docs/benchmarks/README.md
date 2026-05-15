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

# corpus parity baseline report (oracle placeholders kept intentionally)
python3 scripts/parity_harness.py --output docs/benchmarks/parity_latest.json
```

## Artifacts

- `docs/benchmarks/latest.md`
- `docs/benchmarks/latest.csv`
- `docs/benchmarks/latest.json`
- `docs/benchmarks/latest_trend.md`
- `docs/benchmarks/latest_trend.json`
- `docs/benchmarks/parity_latest.json`

All benchmark artifacts are deterministic in structure and key ordering.

## Gate Profiles

- `full` (default):
- absolute per-scenario mean limit: `250ms`
- regression limit vs previous `latest.json`: `10%` with absolute delta floor `>20ms`
- binary size limit (`target/release/puml`): `2,000,000` bytes
- `quick` (`--quick`):
- absolute per-scenario mean limit: `350ms`
- regression limit vs previous `latest.json`: `20%` with absolute delta floor `>30ms`
- binary size limit (`target/release/puml`): `2,500,000` bytes

If no previous `latest.json` baseline exists, regression checks are skipped and absolute/binary checks still apply.

## Failure Handling

- `./scripts/bench.sh` reports gate warnings but exits `0` by default.
- `./scripts/bench.sh --enforce-gates` exits non-zero on any gate failure.
- `./scripts/check-all.sh` always runs benchmark gates in enforce mode.
- On failure, inspect `docs/benchmarks/latest_trend.{md,json}` to identify the exact regressing scenario and delta.

## No-Java Baseline

- PlantUML oracle remains placeholder-only (`todo`) for this repo baseline.
- `parity_latest.json` and `latest_trend.json` include explicit oracle placeholder metadata.
- Do not remove placeholders until Java/oracle execution is intentionally enabled.
