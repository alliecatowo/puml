# Benchmark Results

- Timestamp (UTC): `2026-05-15T20:17:06Z`
- Binary: `/home/Allie/develop/puml/target/release/puml`
- Mode: `full`
- Gate profile: abs mean <= `250ms`, regression <= `10%%`, binary <= `2000000` bytes
- PlantUML comparison: TODO (no-Java environment baseline run)

| Scenario | Mean (ms) | Stddev (ms) | Runs | Tool |
|---|---:|---:|---:|---|
| `cold_start_help` | 140.000 | 6.325 | 5 | `time` |
| `parser_check` | 136.000 | 4.899 | 5 | `time` |
| `parser_dump_scene` | 134.000 | 4.899 | 5 | `time` |
| `render_file` | 112.000 | 16.000 | 5 | `time` |
| `render_stdin` | 88.000 | 4.000 | 5 | `time` |
| `render_stdin_multi` | 90.000 | 0.000 | 5 | `time` |

## PlantUML Comparison (TODO)
Method when Java is available:
1. Run the same fixture set through `puml` and PlantUML.
2. Record parse success, render success, and elapsed time per fixture.
3. Add comparison rows labeled `plantuml_*` with timestamp + command details.
