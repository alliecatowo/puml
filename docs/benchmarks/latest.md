# Benchmark Results

- Timestamp (UTC): `2026-05-15T20:04:58Z`
- Binary: `/home/Allie/develop/puml-wt-30/target/release/puml`
- Mode: `quick`
- Gate profile: abs mean <= `350ms`, regression <= `20%%`, binary <= `2500000` bytes
- PlantUML comparison: TODO (no-Java environment baseline run)

| Scenario | Mean (ms) | Stddev (ms) | Runs | Tool |
|---|---:|---:|---:|---|
| `cold_start_help` | 80.000 | 0.000 | 3 | `time` |
| `parser_check` | 83.333 | 4.714 | 3 | `time` |
| `parser_dump_scene` | 86.667 | 4.714 | 3 | `time` |
| `render_file` | 86.667 | 4.714 | 3 | `time` |
| `render_stdin` | 83.333 | 4.714 | 3 | `time` |
| `render_stdin_multi` | 83.333 | 4.714 | 3 | `time` |

## PlantUML Comparison (TODO)
Method when Java is available:
1. Run the same fixture set through `puml` and PlantUML.
2. Record parse success, render success, and elapsed time per fixture.
3. Add comparison rows labeled `plantuml_*` with timestamp + command details.
