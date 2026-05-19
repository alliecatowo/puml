# Benchmark Results

- Timestamp (UTC): `2026-05-19T05:45:17Z`
- Binary: `/home/Allie/develop/puml/.worktrees/main-green-consolidation/target/release/puml`
- Mode: `full`
- Baseline: `/home/Allie/develop/puml/.worktrees/main-green-consolidation/docs/benchmarks/baseline_full.json`
- Timing tool: `python-perf-counter`
- Environment: `pink-allie-cat` / `Linux` `6.18.10-200.fc43.x86_64` / `x86_64` / `rustc 1.95.0 (59807616e 2026-04-14)`
- Benchmark policy: `bench-gate-v2-2026-05-17`
- Gate profile: abs mean <= `250ms`, regression <= `10%%`, binary <= `12000000` bytes
- PlantUML comparison: TODO (no-Java environment baseline run)

| Scenario | Mean (ms) | Stddev (ms) | Runs | Tool |
|---|---:|---:|---:|---|
| `cold_start_help` | 93.250 | 1.937 | 12 | `python-perf-counter` |
| `parser_check` | 95.380 | 2.475 | 12 | `python-perf-counter` |
| `parser_dump_scene` | 94.169 | 2.517 | 12 | `python-perf-counter` |
| `render_file` | 94.937 | 2.533 | 12 | `python-perf-counter` |
| `render_stdin` | 95.788 | 1.448 | 12 | `python-perf-counter` |
| `render_stdin_multi` | 93.463 | 2.124 | 12 | `python-perf-counter` |

## PlantUML Comparison (TODO)
Method when Java is available:
1. Run the same fixture set through `puml` and PlantUML.
2. Record parse success, render success, and elapsed time per fixture.
3. Add comparison rows labeled `plantuml_*` with timestamp + command details.
