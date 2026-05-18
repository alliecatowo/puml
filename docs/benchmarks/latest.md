# Benchmark Results

- Timestamp (UTC): `2026-05-18T00:17:52Z`
- Binary: `/home/Allie/develop/puml/.agents/worktrees/issue-378-benchmark-truth/target/release/puml`
- Mode: `full`
- Baseline: `/home/Allie/develop/puml/.agents/worktrees/issue-378-benchmark-truth/docs/benchmarks/baseline_full.json`
- Timing tool: `python-perf-counter`
- Environment: `pink-allie-cat` / `Linux` `6.18.10-200.fc43.x86_64` / `x86_64` / `rustc 1.95.0 (59807616e 2026-04-14)`
- Benchmark policy: `bench-gate-v2-2026-05-17`
- Gate profile: abs mean <= `250ms`, regression <= `10%%`, binary <= `12000000` bytes
- PlantUML comparison: TODO (no-Java environment baseline run)

| Scenario | Mean (ms) | Stddev (ms) | Runs | Tool |
|---|---:|---:|---:|---|
| `cold_start_help` | 99.737 | 2.606 | 12 | `python-perf-counter` |
| `parser_check` | 96.765 | 2.345 | 12 | `python-perf-counter` |
| `parser_dump_scene` | 94.806 | 2.608 | 12 | `python-perf-counter` |
| `render_file` | 97.095 | 2.385 | 12 | `python-perf-counter` |
| `render_stdin` | 94.295 | 1.978 | 12 | `python-perf-counter` |
| `render_stdin_multi` | 94.455 | 2.455 | 12 | `python-perf-counter` |

## PlantUML Comparison (TODO)
Method when Java is available:
1. Run the same fixture set through `puml` and PlantUML.
2. Record parse success, render success, and elapsed time per fixture.
3. Add comparison rows labeled `plantuml_*` with timestamp + command details.
