# Benchmark Trend

- Timestamp (UTC): `2026-05-15T20:04:58Z`
- Mode: `quick`
- Baseline timestamp (UTC): `2026-05-15T20:04:55Z`
- Binary: `1832984` bytes (limit `2500000`)
- Regression gate: delta > `20.000%` and `>30.000ms`

| Scenario | Current Mean (ms) | Previous Mean (ms) | Delta (ms) | Delta (%) |
|---|---:|---:|---:|---:|
| `cold_start_help` | 80.000 | 96.000 | -16.000 | -16.667 |
| `parser_check` | 83.333 | 96.000 | -12.667 | -13.195 |
| `parser_dump_scene` | 86.667 | 92.000 | -5.333 | -5.797 |
| `render_file` | 86.667 | 88.000 | -1.333 | -1.515 |
| `render_stdin` | 83.333 | 80.000 | 3.333 | 4.166 |
| `render_stdin_multi` | 83.333 | 82.000 | 1.333 | 1.626 |

## PlantUML Oracle
- Status: `todo`
- Notes: no-Java baseline keeps oracle placeholders only.
