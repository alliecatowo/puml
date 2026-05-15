# Full sample stats

| scenario | n_samples | sample_mean_ms | sample_median_ms | sample_stddev_ms | cv_pct | min_ms | max_ms | abs_gate_pass_rate |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| cold_start_help | 3 | 194.667 | 200.000 | 8.994 | 4.6 | 182.000 | 202.000 | 100.0% |
| parser_check | 3 | 192.667 | 196.000 | 9.286 | 4.8 | 180.000 | 202.000 | 100.0% |
| parser_dump_scene | 3 | 189.333 | 190.000 | 2.494 | 1.3 | 186.000 | 192.000 | 100.0% |
| render_file | 3 | 196.667 | 194.000 | 6.799 | 3.5 | 190.000 | 206.000 | 100.0% |
| render_stdin | 3 | 199.333 | 198.000 | 1.886 | 0.9 | 198.000 | 202.000 | 100.0% |
| render_stdin_multi | 3 | 184.667 | 188.000 | 12.472 | 6.8 | 168.000 | 198.000 | 100.0% |

## sample absolute-gate outcomes

| sample | abs_gate_pass |
|---|---:|
| 1 | pass |
| 2 | pass |
| 3 | pass |

## regression-chain outcomes (immediate previous sample baseline)

| sample | regression_fail_count | failing_scenarios |
|---|---:|---|
| 1 | 0 | none |
| 2 | 0 | none |
| 3 | 0 | none |
