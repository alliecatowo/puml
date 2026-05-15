# Benchmark Trend

- Timestamp (UTC): `2026-05-15T20:32:57Z`
- Mode: `full`
- Baseline source: `/home/Allie/develop/puml-audit-perf/docs/benchmarks/.baseline.previous.json`
- Baseline mode match: `true`
- Baseline timestamp (UTC): `2026-05-15T20:30:48Z`
- Binary: `1832984` bytes (limit `2000000`)
- Regression gate: delta > `10.000%` and `>20.000ms`

| Scenario | Current Mean (ms) | Previous Mean (ms) | Delta (ms) | Delta (%) |
|---|---:|---:|---:|---:|
| `cold_start_help` | 90.203 | 89.887 | 0.316 | 0.352 |
| `parser_check` | 94.170 | 90.618 | 3.552 | 3.920 |
| `parser_dump_scene` | 91.035 | 92.539 | -1.504 | -1.625 |
| `render_file` | 91.008 | 92.934 | -1.926 | -2.072 |
| `render_stdin` | 90.155 | 90.067 | 0.088 | 0.098 |
| `render_stdin_multi` | 92.595 | 91.102 | 1.493 | 1.639 |

## PlantUML Oracle
- Status: `todo`
- Notes: no-Java baseline keeps oracle placeholders only.
