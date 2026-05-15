# Benchmark Trend

- Timestamp (UTC): `2026-05-15T19:22:04Z`
- Mode: `quick`
- Baseline timestamp (UTC): `2026-05-15T19:21:55Z`
- Binary: `1814264` bytes (limit `2500000`)

| Scenario | Current Mean (ms) | Previous Mean (ms) | Delta (ms) | Delta (%) |
|---|---:|---:|---:|---:|
| `cold_start_help` | 126.667 | 130.000 | -3.333 | -2.564 |
| `parser_check` | 130.000 | 128.000 | 2.000 | 1.562 |
| `parser_dump_scene` | 130.000 | 122.000 | 8.000 | 6.557 |
| `render_file` | 123.333 | 98.000 | 25.333 | 25.850 |
| `render_stdin` | 123.333 | 98.000 | 25.333 | 25.850 |
| `render_stdin_multi` | 110.000 | 98.000 | 12.000 | 12.245 |

## PlantUML Oracle
- Status: `todo`
- Notes: no-Java baseline keeps oracle placeholders only.
