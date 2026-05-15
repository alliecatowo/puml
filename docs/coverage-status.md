# Coverage Status

Last measured: 2026-05-15 (America/Los_Angeles)

Command:

```console
cargo llvm-cov --all-features --workspace --fail-under-lines 90 --ignore-filename-regex 'src/(main|bin/puml-lsp)\.rs'
```

Result:
- Gate: passed
- Total line coverage: 91.53%
- Target: 90%
- Margin: +1.53 points

Coverage scope excludes CLI entrypoint binaries (`src/main.rs`, `src/bin/puml-lsp.rs`) to keep the 90% gate focused on shared core library/runtime modules exercised by CLI, tests, and renderer pipelines.

Top low-coverage modules from latest run:
- `normalize.rs`: 86.19% lines
- `layout.rs`: 91.34% lines
- `lib.rs`: 91.67% lines

## Perf/Binary Gate Relationship

- Coverage gate is enforced only in full `./scripts/check-all.sh` mode.
- Quick `./scripts/check-all.sh --quick` skips coverage but enforces benchmark perf + binary-size gates.
- Benchmark regressions are tracked in `docs/benchmarks/latest_trend.{md,json}` with deterministic scenario rows and mode-scoped baselines.

## Contract Audit Notes

Audit date: 2026-05-15

- Unscoped workspace coverage currently reports ~`80.61%` lines because CLI and LSP entrypoint binaries include substantial orchestration branches that are not representative of the shared core runtime gate.
- Release validation keeps 90% enforced and deterministic for scoped core coverage, while CLI/LSP behavior remains protected by dedicated integration/unit contract tests.
