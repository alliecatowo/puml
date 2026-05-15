# Benchmarks

## One-Command Benchmark Run

```console
./scripts/bench.sh
```

## What the Script Produces

- `docs/benchmarks/latest.md`
- `docs/benchmarks/latest.csv`
- `docs/benchmarks/latest.json`

## Behavior

- Builds `target/release/puml` before benchmarking.
- Uses `hyperfine` when available (preferred; more stable).
- Falls back to `/usr/bin/time` when `hyperfine` is unavailable.
- Runs file, check, dump, stdin single, and stdin multi scenarios.

## Latest Recorded Run

- Date: `2026-05-15`
- UTC timestamp: `2026-05-15T07:44:44Z`
- Source artifact: `docs/benchmarks/latest.md`

| Scenario | Mean (ms) | Stddev (ms) | Runs | Tool |
|---|---:|---:|---:|---|
| `render_hello` | 212.000 | 74.135 | 5 | `time` |
| `check_hello` | 180.000 | 0.000 | 5 | `time` |
| `dump_model` | 176.000 | 4.899 | 5 | `time` |
| `stdin_single` | 180.000 | 0.000 | 5 | `time` |
| `stdin_multi` | 182.000 | 4.000 | 5 | `time` |

## Re-run And Compare

```console
# regenerate latest benchmark artifacts
./scripts/bench.sh

# inspect markdown report
sed -n '1,120p' docs/benchmarks/latest.md
```

## Tips for More Stable Results

- Close unrelated heavy workloads.
- Run multiple times and compare trends, not single-run outliers.
- Use the same machine/toolchain when tracking regressions over time.
