# Benchmarks

Run:

```console
./scripts/bench.sh
```

Outputs:
- `docs/benchmarks/latest.md`
- `docs/benchmarks/latest.csv`
- `docs/benchmarks/latest.json`

Notes:
- Uses `hyperfine` when available (preferred).
- Falls back to `/usr/bin/time` when `hyperfine` is unavailable.
- Current suite is `puml`-only (no Java/PlantUML dependency required).
