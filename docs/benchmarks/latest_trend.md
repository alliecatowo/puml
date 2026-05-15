# Benchmark Trend

- Timestamp (UTC): `2026-05-15T20:38:14Z`
- Mode: `full`
- Baseline source: `/home/Allie/develop/puml/docs/benchmarks/.baseline.previous.json`
- Baseline mode match: `true`
- Baseline timestamp (UTC): `2026-05-15T20:30:48Z`
- Binary: `1835328` bytes (limit `2000000`)
- Regression gate: delta > `10.000%` and `>20.000ms`

| Scenario | Current Mean (ms) | Previous Mean (ms) | Delta (ms) | Delta (%) |
|---|---:|---:|---:|---:|
| `cold_start_help` | 92.600 | 89.887 | 2.713 | 3.018 |
| `parser_check` | 91.876 | 90.618 | 1.258 | 1.388 |
| `parser_dump_scene` | 92.135 | 92.539 | -0.404 | -0.437 |
| `render_file` | 92.788 | 92.934 | -0.146 | -0.157 |
| `render_stdin` | 91.366 | 90.067 | 1.299 | 1.442 |
| `render_stdin_multi` | 89.507 | 91.102 | -1.595 | -1.751 |

## PlantUML Oracle
- Status: `todo`
- Notes: no-Java baseline keeps oracle placeholders only.
