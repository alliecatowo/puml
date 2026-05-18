# Benchmark Trend

- Timestamp (UTC): `2026-05-18T00:17:52Z`
- Mode: `full`
- Baseline source: `/home/Allie/develop/puml/.agents/worktrees/issue-378-benchmark-truth/docs/benchmarks/.baseline.previous.json`
- Baseline mode match: `true`
- Baseline timestamp (UTC): `2026-05-18T00:13:12Z`
- Binary: `10503720` bytes (limit `12000000`)
- Regression gate: delta > `10.000%` and `>40.000ms`

| Scenario | Current Mean (ms) | Previous Mean (ms) | Delta (ms) | Delta (%) |
|---|---:|---:|---:|---:|
| `cold_start_help` | 99.737 | 95.684 | 4.053 | 4.236 |
| `parser_check` | 96.765 | 93.300 | 3.465 | 3.714 |
| `parser_dump_scene` | 94.806 | 96.422 | -1.616 | -1.676 |
| `render_file` | 97.095 | 95.068 | 2.027 | 2.132 |
| `render_stdin` | 94.295 | 93.679 | 0.616 | 0.658 |
| `render_stdin_multi` | 94.455 | 95.361 | -0.906 | -0.950 |

## PlantUML Oracle
- Status: `todo`
- Notes: no-Java baseline keeps oracle placeholders only.
