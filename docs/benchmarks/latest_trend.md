# Benchmark Trend

- Timestamp (UTC): `2026-05-19T05:45:17Z`
- Mode: `full`
- Baseline source: `/home/Allie/develop/puml/.worktrees/main-green-consolidation/docs/benchmarks/.baseline.previous.json`
- Baseline mode match: `true`
- Baseline timestamp (UTC): `2026-05-18T00:17:52Z`
- Binary: `11477872` bytes (limit `16000000`)
- Regression gate: delta > `10.000%` and `>40.000ms`

| Scenario | Current Mean (ms) | Previous Mean (ms) | Delta (ms) | Delta (%) |
|---|---:|---:|---:|---:|
| `cold_start_help` | 93.250 | 99.737 | -6.487 | -6.504 |
| `parser_check` | 95.380 | 96.765 | -1.385 | -1.431 |
| `parser_dump_scene` | 94.169 | 94.806 | -0.637 | -0.672 |
| `render_file` | 94.937 | 97.095 | -2.158 | -2.223 |
| `render_stdin` | 95.788 | 94.295 | 1.493 | 1.583 |
| `render_stdin_multi` | 93.463 | 94.455 | -0.992 | -1.050 |

## PlantUML Oracle
- Status: `todo`
- Notes: no-Java baseline keeps oracle placeholders only.
