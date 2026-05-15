# Benchmark Results

- Timestamp (UTC): `2026-05-15T17:29:00Z`
- Binary: `/home/Allie/develop/puml/target/release/puml`
- Mode: `quick`
- PlantUML comparison: TODO (no-Java environment baseline run)

| Scenario | Mean (ms) | Stddev (ms) | Runs | Tool |
|---|---:|---:|---:|---|
| `cold_start_help` | 116.667 | 37.712 | 3 | `time` |
| `parser_check` | 100.000 | 0.000 | 3 | `time` |
| `parser_dump_scene` | 96.667 | 4.714 | 3 | `time` |
| `render_file` | 100.000 | 0.000 | 3 | `time` |
| `render_stdin` | 100.000 | 8.165 | 3 | `time` |
| `render_stdin_multi` | 100.000 | 0.000 | 3 | `time` |

## PlantUML Comparison (TODO)
Method when Java is available:
1. Run the same fixture set through `puml` and PlantUML.
2. Record parse success, render success, and elapsed time per fixture.
3. Add comparison rows labeled `plantuml_*` with timestamp + command details.
