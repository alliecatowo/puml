# Benchmark Results

- Timestamp (UTC): `2026-05-15T20:38:14Z`
- Binary: `/home/Allie/develop/puml/target/release/puml`
- Mode: `full`
- Baseline: `/home/Allie/develop/puml/docs/benchmarks/baseline_full.json`
- Timing tool: `python-perf-counter`
- Environment: `pink-allie-cat` / `Linux` `6.18.10-200.fc43.x86_64` / `x86_64` / `rustc 1.95.0 (59807616e 2026-04-14)`
- Gate profile: abs mean <= `250ms`, regression <= `10%%`, binary <= `2000000` bytes
- PlantUML comparison: TODO (no-Java environment baseline run)

| Scenario | Mean (ms) | Stddev (ms) | Runs | Tool |
|---|---:|---:|---:|---|
| `cold_start_help` | 92.600 | 1.693 | 12 | `python-perf-counter` |
| `parser_check` | 91.876 | 2.860 | 12 | `python-perf-counter` |
| `parser_dump_scene` | 92.135 | 2.886 | 12 | `python-perf-counter` |
| `render_file` | 92.788 | 2.059 | 12 | `python-perf-counter` |
| `render_stdin` | 91.366 | 1.572 | 12 | `python-perf-counter` |
| `render_stdin_multi` | 89.507 | 1.411 | 12 | `python-perf-counter` |

## PlantUML Comparison (TODO)
Method when Java is available:
1. Run the same fixture set through `puml` and PlantUML.
2. Record parse success, render success, and elapsed time per fixture.
3. Add comparison rows labeled `plantuml_*` with timestamp + command details.
