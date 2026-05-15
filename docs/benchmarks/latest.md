# Benchmark Results

- Timestamp (UTC): `2026-05-15T20:32:57Z`
- Binary: `/home/Allie/develop/puml-audit-perf/target/release/puml`
- Mode: `full`
- Baseline: `/home/Allie/develop/puml-audit-perf/docs/benchmarks/baseline_full.json`
- Timing tool: `python-perf-counter`
- Environment: `pink-allie-cat` / `Linux` `6.18.10-200.fc43.x86_64` / `x86_64` / `rustc 1.95.0 (59807616e 2026-04-14)`
- Gate profile: abs mean <= `250ms`, regression <= `10%%`, binary <= `2000000` bytes
- PlantUML comparison: TODO (no-Java environment baseline run)

| Scenario | Mean (ms) | Stddev (ms) | Runs | Tool |
|---|---:|---:|---:|---|
| `cold_start_help` | 90.203 | 2.048 | 12 | `python-perf-counter` |
| `parser_check` | 94.170 | 4.679 | 12 | `python-perf-counter` |
| `parser_dump_scene` | 91.035 | 2.677 | 12 | `python-perf-counter` |
| `render_file` | 91.008 | 1.711 | 12 | `python-perf-counter` |
| `render_stdin` | 90.155 | 1.323 | 12 | `python-perf-counter` |
| `render_stdin_multi` | 92.595 | 1.780 | 12 | `python-perf-counter` |

## PlantUML Comparison (TODO)
Method when Java is available:
1. Run the same fixture set through `puml` and PlantUML.
2. Record parse success, render success, and elapsed time per fixture.
3. Add comparison rows labeled `plantuml_*` with timestamp + command details.
