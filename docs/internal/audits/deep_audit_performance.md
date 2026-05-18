# Deep Audit: Performance Gate Reliability

Date: 2026-05-15 (America/Los_Angeles)
Branch: `codex/audit-perf-deep`
Worktree: `/home/Allie/develop/puml-audit-perf`

## Scope

1. Audit benchmark methodology and gate stability across `scripts/bench.sh`, `scripts/check-all.sh`, and benchmark artifacts/trend logic.
2. Run repeated measurements (>=5 quick samples, >=3 full samples) and characterize variance.
3. Evaluate binary-size gate and cold-start claim reproducibility.
4. Propose and implement immediate mitigation + robust long-term methodology.

## Baseline Findings (Before Fixes)

### Major instability sources

1. Cross-mode baseline coupling: `full` runs compared against most recent `quick` `latest.json` (and vice versa), producing false regression alerts.
2. Baseline drift: baseline auto-moved every run by overwriting `latest.json`, so transient outliers became the next regression reference.
3. Coarse fallback timing: non-`hyperfine` path used `/usr/bin/time -f %e` with low precision and very low run counts (`quick=3`, `full=5`).
4. Environment sensitivity not captured: benchmark artifacts had no host/toolchain metadata to explain shifts.
5. Claimed `cold_start_help` is not a true cold-start signal: it measures `--help` startup in warm process/file-cache conditions and is highly noise-sensitive.

### Measured variance before fixes

Quick profile sample outcomes (5 runs):
- Absolute-gate pass rate by sample: `3/5` (40% flaky failures).
- Very high CV on several scenarios:
  - `render_stdin`: `86.0%`
  - `render_stdin_multi`: `84.1%`
  - `cold_start_help`: `63.9%`
- Range examples:
  - `render_stdin`: `96.667ms` to `560.000ms`
  - `cold_start_help`: `93.333ms` to `393.333ms`

Full profile sample outcomes (3 runs):
- Absolute-gate pass rate by sample: `3/3`.
- But first full run after quick baseline triggered mass regression warnings due to cross-mode comparison.

## Binary Size / Cold-Start Reproducibility

1. Binary size signal was stable in sampled runs: `1,832,984` bytes throughout the collected pre/post audit runs.
2. Reproducibility risk remains without toolchain pinning (`rust-toolchain.toml` absent): size can drift across compiler/target environments.
3. `cold_start_help` is reproducible only as a lightweight CLI startup proxy, not as a true cold-start metric.

## Implemented Mitigations

### Scripts and gate logic

1. Refactored gate/trend logic into testable helper: `scripts/bench_gate.py`.
2. Added mode-scoped baselines:
- `docs/benchmarks/baseline_full.json`
- `docs/benchmarks/baseline_quick.json`
3. Regression checks now only compare matching modes.
4. Baseline updates are explicit (`--update-baseline`) instead of implicit drift.
5. Replaced coarse fallback timer with high-resolution Python `perf_counter_ns` sampler.
6. Increased fallback sample counts (`quick=7`, `full=12`) and warmup consistency.
7. Added benchmark environment metadata to artifacts (host, OS, kernel, arch, rustc, timing tool).

### Tests and coverage uplift

1. Added `tests/bench_gate_audit.rs` covering:
- binary + absolute + regression failure reporting
- regression skip on mode mismatch
- trend baseline mode-mismatch behavior
2. Extended `tests/coverage_utilities.rs` with targeted `lib.rs` branch coverage tests:
- `DiagramFamily::as_str` variants
- non-sequence family error path
- markdown fence extraction edge behavior
- mermaid short-arrow support + unsupported declaration path
3. Coverage improved (full gate `llvm-cov`):
- Total line coverage: `91.10%` -> `91.53%`
- `lib.rs` line coverage: `85.51%` -> `91.67%`

## Post-Fix Stability Validation

Repeated enforced runs with the new methodology:
- Quick enforced: `5/5` pass, all exit code `0`
- Full enforced: `3/3` pass, all exit code `0`

Observed CV after fixes (sample-mean variability):
- Quick: `0.71%` to `2.02%`
- Full: `0.54%` to `1.43%`

This is a large stability improvement versus the pre-fix quick profile (up to `86.0%` CV and 40% sample-level absolute-gate failure).

## Recommendations

### Immediate (already implemented)

1. Keep mode-scoped baselines and explicit baseline updates.
2. Keep high-resolution fallback timing and increased fallback sample counts.
3. Keep gate logic under test via `tests/bench_gate_audit.rs`.

### Long-term methodology

1. Pin toolchain for size/perf reproducibility (`rust-toolchain.toml`, explicit target and profile flags).
2. Run perf gates on dedicated CI runners (isolated CPU governor/affinity) and treat shared-runner results as advisory.
3. Separate startup proxy (`--help`) from true cold-start measurement; add a dedicated cold-start harness with cache-state control.
4. Move regression decisions from single-point baseline to windowed baselines (median + MAD/IQR or confidence intervals).
5. Require `hyperfine` in perf CI for canonical sampling; keep fallback as local-only or advisory.
6. Keep baseline updates as reviewable PR events with explicit rationale.

## Raw Summary Tables (Appended)

### Pre-fix quick raw

| sample | scenario | mean_ms | stddev_ms | runs | tool | timestamp_utc |
|---:|---|---:|---:|---:|---|---|
| 1 | cold_start_help | 150.000 | 40.825 | 3 | time | 2026-05-15T20:21:53Z |
| 1 | parser_check | 206.667 | 9.428 | 3 | time | 2026-05-15T20:21:53Z |
| 1 | parser_dump_scene | 200.000 | 8.165 | 3 | time | 2026-05-15T20:21:53Z |
| 1 | render_file | 360.000 | 166.733 | 3 | time | 2026-05-15T20:21:53Z |
| 1 | render_stdin | 560.000 | 40.825 | 3 | time | 2026-05-15T20:21:53Z |
| 1 | render_stdin_multi | 470.000 | 71.181 | 3 | time | 2026-05-15T20:21:53Z |
| 2 | cold_start_help | 393.333 | 33.993 | 3 | time | 2026-05-15T20:22:26Z |
| 2 | parser_check | 180.000 | 14.142 | 3 | time | 2026-05-15T20:22:26Z |
| 2 | parser_dump_scene | 136.667 | 16.997 | 3 | time | 2026-05-15T20:22:26Z |
| 2 | render_file | 150.000 | 14.142 | 3 | time | 2026-05-15T20:22:26Z |
| 2 | render_stdin | 140.000 | 24.495 | 3 | time | 2026-05-15T20:22:26Z |
| 2 | render_stdin_multi | 110.000 | 8.165 | 3 | time | 2026-05-15T20:22:26Z |
| 3 | cold_start_help | 140.000 | 21.602 | 3 | time | 2026-05-15T20:22:30Z |
| 3 | parser_check | 173.333 | 49.889 | 3 | time | 2026-05-15T20:22:30Z |
| 3 | parser_dump_scene | 110.000 | 14.142 | 3 | time | 2026-05-15T20:22:30Z |
| 3 | render_file | 106.667 | 16.997 | 3 | time | 2026-05-15T20:22:30Z |
| 3 | render_stdin | 96.667 | 4.714 | 3 | time | 2026-05-15T20:22:30Z |
| 3 | render_stdin_multi | 96.667 | 4.714 | 3 | time | 2026-05-15T20:22:30Z |
| 4 | cold_start_help | 96.667 | 4.714 | 3 | time | 2026-05-15T20:22:33Z |
| 4 | parser_check | 150.000 | 29.439 | 3 | time | 2026-05-15T20:22:33Z |
| 4 | parser_dump_scene | 123.333 | 12.472 | 3 | time | 2026-05-15T20:22:33Z |
| 4 | render_file | 210.000 | 35.590 | 3 | time | 2026-05-15T20:22:33Z |
| 4 | render_stdin | 140.000 | 0.000 | 3 | time | 2026-05-15T20:22:33Z |
| 4 | render_stdin_multi | 100.000 | 8.165 | 3 | time | 2026-05-15T20:22:33Z |
| 5 | cold_start_help | 93.333 | 4.714 | 3 | time | 2026-05-15T20:22:35Z |
| 5 | parser_check | 90.000 | 0.000 | 3 | time | 2026-05-15T20:22:35Z |
| 5 | parser_dump_scene | 86.667 | 4.714 | 3 | time | 2026-05-15T20:22:35Z |
| 5 | render_file | 90.000 | 8.165 | 3 | time | 2026-05-15T20:22:35Z |
| 5 | render_stdin | 96.667 | 4.714 | 3 | time | 2026-05-15T20:22:35Z |
| 5 | render_stdin_multi | 100.000 | 0.000 | 3 | time | 2026-05-15T20:22:35Z |

### Pre-fix full raw

| sample | scenario | mean_ms | stddev_ms | runs | tool | timestamp_utc |
|---:|---|---:|---:|---:|---|---|
| 1 | cold_start_help | 200.000 | 0.000 | 5 | time | 2026-05-15T20:22:50Z |
| 1 | parser_check | 202.000 | 4.000 | 5 | time | 2026-05-15T20:22:50Z |
| 1 | parser_dump_scene | 192.000 | 11.662 | 5 | time | 2026-05-15T20:22:50Z |
| 1 | render_file | 206.000 | 13.565 | 5 | time | 2026-05-15T20:22:50Z |
| 1 | render_stdin | 198.000 | 4.000 | 5 | time | 2026-05-15T20:22:50Z |
| 1 | render_stdin_multi | 198.000 | 4.000 | 5 | time | 2026-05-15T20:22:50Z |
| 2 | cold_start_help | 182.000 | 16.000 | 5 | time | 2026-05-15T20:22:56Z |
| 2 | parser_check | 180.000 | 26.077 | 5 | time | 2026-05-15T20:22:56Z |
| 2 | parser_dump_scene | 186.000 | 23.324 | 5 | time | 2026-05-15T20:22:56Z |
| 2 | render_file | 194.000 | 4.899 | 5 | time | 2026-05-15T20:22:56Z |
| 2 | render_stdin | 198.000 | 4.000 | 5 | time | 2026-05-15T20:22:56Z |
| 2 | render_stdin_multi | 188.000 | 7.483 | 5 | time | 2026-05-15T20:22:56Z |
| 3 | cold_start_help | 202.000 | 4.000 | 5 | time | 2026-05-15T20:23:03Z |
| 3 | parser_check | 196.000 | 4.899 | 5 | time | 2026-05-15T20:23:03Z |
| 3 | parser_dump_scene | 190.000 | 27.568 | 5 | time | 2026-05-15T20:23:03Z |
| 3 | render_file | 190.000 | 8.944 | 5 | time | 2026-05-15T20:23:03Z |
| 3 | render_stdin | 202.000 | 4.000 | 5 | time | 2026-05-15T20:23:03Z |
| 3 | render_stdin_multi | 168.000 | 44.000 | 5 | time | 2026-05-15T20:23:03Z |

### Post-fix quick enforced raw

| sample | mode | exit_code | scenario | mean_ms | stddev_ms | runs | tool | binary_bytes | timestamp_utc |
|---:|---|---:|---|---:|---:|---:|---|---:|---|
| 1 | quick | 0 | cold_start_help | 91.882 | 1.689 | 7 | python-perf-counter | 1832984 | 2026-05-15T20:31:11Z |
| 1 | quick | 0 | parser_check | 90.590 | 1.470 | 7 | python-perf-counter | 1832984 | 2026-05-15T20:31:11Z |
| 1 | quick | 0 | parser_dump_scene | 93.060 | 2.170 | 7 | python-perf-counter | 1832984 | 2026-05-15T20:31:11Z |
| 1 | quick | 0 | render_file | 90.090 | 2.871 | 7 | python-perf-counter | 1832984 | 2026-05-15T20:31:11Z |
| 1 | quick | 0 | render_stdin | 90.626 | 2.092 | 7 | python-perf-counter | 1832984 | 2026-05-15T20:31:11Z |
| 1 | quick | 0 | render_stdin_multi | 91.719 | 1.561 | 7 | python-perf-counter | 1832984 | 2026-05-15T20:31:11Z |
| 2 | quick | 0 | cold_start_help | 90.927 | 1.881 | 7 | python-perf-counter | 1832984 | 2026-05-15T20:31:16Z |
| 2 | quick | 0 | parser_check | 89.052 | 1.270 | 7 | python-perf-counter | 1832984 | 2026-05-15T20:31:16Z |
| 2 | quick | 0 | parser_dump_scene | 88.992 | 2.165 | 7 | python-perf-counter | 1832984 | 2026-05-15T20:31:16Z |
| 2 | quick | 0 | render_file | 89.601 | 1.968 | 7 | python-perf-counter | 1832984 | 2026-05-15T20:31:16Z |
| 2 | quick | 0 | render_stdin | 88.985 | 1.321 | 7 | python-perf-counter | 1832984 | 2026-05-15T20:31:16Z |
| 2 | quick | 0 | render_stdin_multi | 88.692 | 1.165 | 7 | python-perf-counter | 1832984 | 2026-05-15T20:31:16Z |
| 3 | quick | 0 | cold_start_help | 89.442 | 2.070 | 7 | python-perf-counter | 1832984 | 2026-05-15T20:31:21Z |
| 3 | quick | 0 | parser_check | 89.331 | 1.590 | 7 | python-perf-counter | 1832984 | 2026-05-15T20:31:21Z |
| 3 | quick | 0 | parser_dump_scene | 89.230 | 2.975 | 7 | python-perf-counter | 1832984 | 2026-05-15T20:31:21Z |
| 3 | quick | 0 | render_file | 91.180 | 2.174 | 7 | python-perf-counter | 1832984 | 2026-05-15T20:31:21Z |
| 3 | quick | 0 | render_stdin | 87.443 | 1.238 | 7 | python-perf-counter | 1832984 | 2026-05-15T20:31:21Z |
| 3 | quick | 0 | render_stdin_multi | 89.451 | 2.545 | 7 | python-perf-counter | 1832984 | 2026-05-15T20:31:21Z |
| 4 | quick | 0 | cold_start_help | 91.119 | 2.706 | 7 | python-perf-counter | 1832984 | 2026-05-15T20:31:27Z |
| 4 | quick | 0 | parser_check | 88.733 | 1.023 | 7 | python-perf-counter | 1832984 | 2026-05-15T20:31:27Z |
| 4 | quick | 0 | parser_dump_scene | 87.771 | 1.378 | 7 | python-perf-counter | 1832984 | 2026-05-15T20:31:27Z |
| 4 | quick | 0 | render_file | 93.646 | 2.839 | 7 | python-perf-counter | 1832984 | 2026-05-15T20:31:27Z |
| 4 | quick | 0 | render_stdin | 90.751 | 2.586 | 7 | python-perf-counter | 1832984 | 2026-05-15T20:31:27Z |
| 4 | quick | 0 | render_stdin_multi | 91.437 | 2.380 | 7 | python-perf-counter | 1832984 | 2026-05-15T20:31:27Z |
| 5 | quick | 0 | cold_start_help | 90.511 | 1.734 | 7 | python-perf-counter | 1832984 | 2026-05-15T20:31:32Z |
| 5 | quick | 0 | parser_check | 89.651 | 2.453 | 7 | python-perf-counter | 1832984 | 2026-05-15T20:31:32Z |
| 5 | quick | 0 | parser_dump_scene | 90.795 | 2.707 | 7 | python-perf-counter | 1832984 | 2026-05-15T20:31:32Z |
| 5 | quick | 0 | render_file | 89.480 | 0.840 | 7 | python-perf-counter | 1832984 | 2026-05-15T20:31:32Z |
| 5 | quick | 0 | render_stdin | 88.851 | 0.879 | 7 | python-perf-counter | 1832984 | 2026-05-15T20:31:32Z |
| 5 | quick | 0 | render_stdin_multi | 87.041 | 1.083 | 7 | python-perf-counter | 1832984 | 2026-05-15T20:31:32Z |

### Post-fix full enforced raw

| sample | mode | exit_code | scenario | mean_ms | stddev_ms | runs | tool | binary_bytes | timestamp_utc |
|---:|---|---:|---|---:|---:|---:|---|---:|---|
| 1 | full | 0 | cold_start_help | 91.678 | 2.364 | 12 | python-perf-counter | 1832984 | 2026-05-15T20:31:37Z |
| 1 | full | 0 | parser_check | 90.522 | 1.981 | 12 | python-perf-counter | 1832984 | 2026-05-15T20:31:37Z |
| 1 | full | 0 | parser_dump_scene | 90.160 | 1.351 | 12 | python-perf-counter | 1832984 | 2026-05-15T20:31:37Z |
| 1 | full | 0 | render_file | 90.498 | 1.317 | 12 | python-perf-counter | 1832984 | 2026-05-15T20:31:37Z |
| 1 | full | 0 | render_stdin | 88.099 | 1.673 | 12 | python-perf-counter | 1832984 | 2026-05-15T20:31:37Z |
| 1 | full | 0 | render_stdin_multi | 87.775 | 1.296 | 12 | python-perf-counter | 1832984 | 2026-05-15T20:31:37Z |
| 2 | full | 0 | cold_start_help | 89.181 | 1.299 | 12 | python-perf-counter | 1832984 | 2026-05-15T20:31:45Z |
| 2 | full | 0 | parser_check | 89.194 | 2.068 | 12 | python-perf-counter | 1832984 | 2026-05-15T20:31:45Z |
| 2 | full | 0 | parser_dump_scene | 87.058 | 1.244 | 12 | python-perf-counter | 1832984 | 2026-05-15T20:31:45Z |
| 2 | full | 0 | render_file | 89.696 | 2.180 | 12 | python-perf-counter | 1832984 | 2026-05-15T20:31:45Z |
| 2 | full | 0 | render_stdin | 90.422 | 2.265 | 12 | python-perf-counter | 1832984 | 2026-05-15T20:31:45Z |
| 2 | full | 0 | render_stdin_multi | 90.182 | 2.947 | 12 | python-perf-counter | 1832984 | 2026-05-15T20:31:45Z |
| 3 | full | 0 | cold_start_help | 89.482 | 1.519 | 12 | python-perf-counter | 1832984 | 2026-05-15T20:31:53Z |
| 3 | full | 0 | parser_check | 91.576 | 4.624 | 12 | python-perf-counter | 1832984 | 2026-05-15T20:31:53Z |
| 3 | full | 0 | parser_dump_scene | 88.519 | 1.645 | 12 | python-perf-counter | 1832984 | 2026-05-15T20:31:53Z |
| 3 | full | 0 | render_file | 89.329 | 1.697 | 12 | python-perf-counter | 1832984 | 2026-05-15T20:31:53Z |
| 3 | full | 0 | render_stdin | 89.796 | 1.415 | 12 | python-perf-counter | 1832984 | 2026-05-15T20:31:53Z |
| 3 | full | 0 | render_stdin_multi | 90.296 | 1.394 | 12 | python-perf-counter | 1832984 | 2026-05-15T20:31:53Z |
