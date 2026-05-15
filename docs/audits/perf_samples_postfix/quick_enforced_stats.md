# quick_enforced stats

| scenario | n_samples | sample_mean_ms | sample_stddev_ms | cv_pct | min_ms | max_ms | abs_pass_rate |
|---|---:|---:|---:|---:|---:|---:|---:|
| cold_start_help | 5 | 90.776 | 0.802 | 0.88 | 89.442 | 91.882 | 100.0% |
| parser_check | 5 | 89.471 | 0.636 | 0.71 | 88.733 | 90.590 | 100.0% |
| parser_dump_scene | 5 | 89.970 | 1.820 | 2.02 | 87.771 | 93.060 | 100.0% |
| render_file | 5 | 90.799 | 1.545 | 1.70 | 89.480 | 93.646 | 100.0% |
| render_stdin | 5 | 89.331 | 1.234 | 1.38 | 87.443 | 90.751 | 100.0% |
| render_stdin_multi | 5 | 89.668 | 1.746 | 1.95 | 87.041 | 91.719 | 100.0% |

| sample | exit_code | gate_result |
|---:|---:|---|
| 1 | 0 | pass |
| 2 | 0 | pass |
| 3 | 0 | pass |
| 4 | 0 | pass |
| 5 | 0 | pass |
