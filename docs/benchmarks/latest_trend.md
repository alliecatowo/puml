# Benchmark Trend

- Timestamp (UTC): `2026-05-15T17:54:53Z`
- Mode: `quick`
- Baseline timestamp (UTC): `2026-05-15T17:54:29Z`
- Binary: `1638888` bytes (limit `2500000`)

| Scenario | Current Mean (ms) | Previous Mean (ms) | Delta (ms) | Delta (%) |
|---|---:|---:|---:|---:|
| `cold_start_help` | 123.333 | 108.000 | 15.333 | 14.197 |
| `parser_check` | 96.667 | 100.000 | -3.333 | -3.333 |
| `parser_dump_scene` | 93.333 | 92.000 | 1.333 | 1.449 |
| `render_file` | 100.000 | 100.000 | 0.000 | 0.000 |
| `render_stdin` | 100.000 | 100.000 | 0.000 | 0.000 |
| `render_stdin_multi` | 96.667 | 98.000 | -1.333 | -1.360 |

## PlantUML Oracle
- Status: `todo`
- Notes: no-Java baseline keeps oracle placeholders only.
