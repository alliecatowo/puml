# Quick sample stats

| scenario | n_samples | sample_mean_ms | sample_median_ms | sample_stddev_ms | cv_pct | min_ms | max_ms | abs_gate_pass_rate |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| cold_start_help | 5 | 174.667 | 140.000 | 111.646 | 63.9 | 93.333 | 393.333 | 80.0% |
| parser_check | 5 | 160.000 | 173.333 | 39.384 | 24.6 | 90.000 | 206.667 | 100.0% |
| parser_dump_scene | 5 | 131.333 | 123.333 | 38.099 | 29.0 | 86.667 | 200.000 | 100.0% |
| render_file | 5 | 183.333 | 150.000 | 97.593 | 53.2 | 90.000 | 360.000 | 80.0% |
| render_stdin | 5 | 206.667 | 140.000 | 177.726 | 86.0 | 96.667 | 560.000 | 80.0% |
| render_stdin_multi | 5 | 175.333 | 100.000 | 147.401 | 84.1 | 96.667 | 470.000 | 80.0% |

## sample absolute-gate outcomes

| sample | abs_gate_pass |
|---|---:|
| 1 | fail |
| 2 | fail |
| 3 | pass |
| 4 | pass |
| 5 | pass |

## regression-chain outcomes (immediate previous sample baseline)

| sample | regression_fail_count | failing_scenarios |
|---|---:|---|
| 1 | 0 | none |
| 2 | 1 | cold_start_help (243.3ms, 162.2%) |
| 3 | 0 | none |
| 4 | 2 | render_file (103.3ms, 96.9%); render_stdin (43.3ms, 44.8%) |
| 5 | 0 | none |
