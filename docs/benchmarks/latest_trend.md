# Benchmark Trend

- Timestamp (UTC): `2026-05-15T20:17:06Z`
- Mode: `full`
- Baseline timestamp (UTC): `2026-05-15T20:16:56Z`
- Binary: `1832984` bytes (limit `2000000`)
- Regression gate: delta > `10.000%` and `>20.000ms`

| Scenario | Current Mean (ms) | Previous Mean (ms) | Delta (ms) | Delta (%) |
|---|---:|---:|---:|---:|
| `cold_start_help` | 140.000 | 112.000 | 28.000 | 25.000 |
| `parser_check` | 136.000 | 96.000 | 40.000 | 41.667 |
| `parser_dump_scene` | 134.000 | 98.000 | 36.000 | 36.735 |
| `render_file` | 112.000 | 98.000 | 14.000 | 14.286 |
| `render_stdin` | 88.000 | 100.000 | -12.000 | -12.000 |
| `render_stdin_multi` | 90.000 | 100.000 | -10.000 | -10.000 |

## PlantUML Oracle
- Status: `todo`
- Notes: no-Java baseline keeps oracle placeholders only.
