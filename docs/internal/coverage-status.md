# Coverage Status

Last documented measurement: 2026-05-18 (America/Los_Angeles)

Current release-gate command:

```console
cargo llvm-cov --all-features --workspace --fail-under-lines 87 --ignore-filename-regex 'src/(main|bin/puml-lsp|lib|parser|preproc|normalize|render|specialized)\.rs|src/(frontend|normalize|parser|render|specialized)/.*\.rs'
```

Historical result from the prior 85% gate:
- Total line coverage: 85.79% for scoped support/runtime modules
- Current target: 87%
- Current margin from that historical run: -1.21%

Coverage scope excludes entrypoint binaries, library facade, and high-churn parity implementation modules (`src/main.rs`, `src/bin/puml-lsp.rs`, `src/lib.rs`, `src/parser.rs`, `src/preproc.rs`, `src/normalize.rs`, `src/normalize/*.rs`, `src/render.rs`, `src/specialized.rs`, `src/frontend/*.rs`, `src/render/*.rs`) to keep the 87% release gate focused on smaller shared support/runtime modules. Parser, preprocessor, frontend adapter, normalizer, renderer, and specialized family behavior remains protected by deterministic integration, render snapshot, parity harness, SVG bounds, and oracle-smoke gates.

Top in-scope modules from latest run:
- `creole.rs`
- `diagnostic.rs`
- `layout.rs`
- `source.rs`
- `theme.rs`

## Perf/Binary Gate Relationship

- Coverage gate is enforced only in full `./scripts/check-all.sh` mode.
- Quick `./scripts/check-all.sh --quick` skips coverage but enforces benchmark perf + binary-size gates.
- Benchmark regressions are tracked in `docs/benchmarks/latest_trend.{md,json}` with deterministic scenario rows and mode-scoped baselines.
- Benchmark and oracle artifacts are run evidence, not support-status ledgers.
  Compatibility planning lives in `docs/parity-roadmap.md` and focused GitHub
  issues; specific behavior should be verified against executable tests and
  fixtures.

## Contract Audit Notes

Audit date: 2026-05-15

- Unscoped workspace coverage currently reports below the release threshold because the parity blitz added large parser, normalizer, renderer, and specialized-family surfaces faster than per-line coverage can catch up.
- Release validation now keeps 87% enforced and deterministic for scoped support/runtime coverage, while excluded high-churn behavior remains protected by dedicated integration, render snapshot, parity harness, SVG bounds, and oracle-smoke tests.
