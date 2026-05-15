# Benchmark Trend

- Timestamp (UTC): `2026-05-15T19:58:55Z`
- Mode: `full`
- Baseline timestamp (UTC): `2026-05-15T19:58:35Z`
- Binary: `1829280` bytes (limit `2000000`)
- Regression gate: delta > `10.000%` and `>20.000ms`

| Scenario | Current Mean (ms) | Previous Mean (ms) | Delta (ms) | Delta (%) |
|---|---:|---:|---:|---:|
| `cold_start_help` | 96.000 | 93.333 | 2.667 | 2.858 |
| `parser_check` | 92.000 | 90.000 | 2.000 | 2.222 |
| `parser_dump_scene` | 100.000 | 96.667 | 3.333 | 3.448 |
| `render_file` | 98.000 | 96.667 | 1.333 | 1.379 |
| `render_stdin` | 100.000 | 100.000 | 0.000 | 0.000 |
| `render_stdin_multi` | 96.000 | 90.000 | 6.000 | 6.667 |

## PlantUML Oracle
- Status: `todo`
- Notes: no-Java baseline keeps oracle placeholders only.
