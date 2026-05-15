# full_enforced stats

| scenario | n_samples | sample_mean_ms | sample_stddev_ms | cv_pct | min_ms | max_ms | abs_pass_rate |
|---|---:|---:|---:|---:|---:|---:|---:|
| cold_start_help | 3 | 90.114 | 1.113 | 1.24 | 89.181 | 91.678 | 100.0% |
| parser_check | 3 | 90.431 | 0.975 | 1.08 | 89.194 | 91.576 | 100.0% |
| parser_dump_scene | 3 | 88.579 | 1.267 | 1.43 | 87.058 | 90.160 | 100.0% |
| render_file | 3 | 89.841 | 0.488 | 0.54 | 89.329 | 90.498 | 100.0% |
| render_stdin | 3 | 89.439 | 0.981 | 1.10 | 88.099 | 90.422 | 100.0% |
| render_stdin_multi | 3 | 89.418 | 1.162 | 1.30 | 87.775 | 90.296 | 100.0% |

| sample | exit_code | gate_result |
|---:|---:|---|
| 1 | 0 | pass |
| 2 | 0 | pass |
| 3 | 0 | pass |
