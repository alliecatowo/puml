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

## Tips for More Stable Results

- Close unrelated heavy workloads.
- Run multiple times and compare trends, not single-run outliers.
- Use the same machine/toolchain when tracking regressions over time.
