# Benchmark Artifact History

## Overview

Every push to `main` that passes the `Main Gate` workflow produces a set of
benchmark artifacts that are uploaded to GitHub Actions as named artifacts and
retained for **90 days**.  Two separate artifact bundles are produced per run:

| Artifact name | Retention | Contents |
|---|---|---|
| `main-benchmarks-<sha>` | 90 days | Full benchmark suite (all files listed below) |
| `render-check-report-<sha>` | 90 days | Render check subset: `render_check_latest.json`, `latest_trend.json`, `latest_trend.md` |

## Artifact Files

### `render_check_latest.json`
Machine-readable docs render check report produced by `scripts/render_check.py`.

Schema version: `2.0.0`

Top-level keys:
- `schema_version` — always `"2.0.0"` so consumers can version-gate parsing.
- `generated_at_utc` — ISO-8601 UTC timestamp of the run.
- `summary` — aggregate counts: `total`, `passed`, `excluded`, `failed`.
- `entries` — per-source-file render results (see schema below).

Each `entries[]` entry contains:
```json
{
  "source_kind": "source_file",
  "source_ref": "docs/examples/basic/hello.puml",
  "artifact_svg": "docs/examples/basic/hello.svg",
  "artifact_exists": true,
  "artifact_up_to_date": true,
  "excluded": false,
  "exclusion_reason": null,
  "status": "pass",
  "notes": []
}
```

### `oracle_smoke_latest.json`
Machine-readable differential smoke report produced by
`scripts/differential_oracle_smoke.py`.

Schema version: `1.1.0`

Top-level keys:
- `schema_version` — currently `"1.1.0"`.
- `generated_at_utc` — ISO-8601 UTC timestamp of the run.
- `tool` — runner metadata, including `dry_run`.
- `oracle` — comparison-only contract flags and optional oracle command.
- `summary` — aggregate counts, including `not_run`, `by_fixture_category`,
  `by_support_status`, and `by_expected_oracle_category`.
- `fixtures` — per-fixture render/comparison records plus `classification`.

Dry-run mode is Java-free and command-free. It sets `oracle.enabled: false`,
marks every fixture `comparison.state: "not-run"`, and records expected oracle
categories such as `match`, `drift`, and `jar-only` so remaining partial
PlantUML gaps stay fixture-backed even when the external oracle is unavailable.

### `latest_trend.json`
Trend data produced by `scripts/bench.sh`. Tracks per-scenario mean render
times across successive runs.  Keys: `generated_at_utc`, `mode` (`full`|`quick`),
`scenarios[]` each with `name`, `mean_ms`, `gate_limit_ms`, `gate_passed`.

### `latest_trend.md`
Human-readable Markdown table rendered from `latest_trend.json`.
Suitable for pasting into GitHub comments or issue comments.

### `latest.json` / `latest.csv` / `latest.md`
Raw hyperfine benchmark output for the most recent run.

### `baseline_full.json` / `baseline_quick.json`
Gate baselines.  Regression checks compare `latest*.json` against the
appropriate baseline.  Baselines are **not** auto-updated; use
`./scripts/bench.sh --update-baseline` after intentional review.

## How to Read a Trend

1. Download the `render-check-report-<sha>` artifact from the Actions run you care about.
2. Open `latest_trend.md` for a quick human summary.
3. For programmatic analysis, parse `latest_trend.json`:
   - Any `gate_passed: false` entry is a regression against the baseline.
   - `mean_ms` values > `gate_limit_ms` indicate absolute limit breaches.

## Oracle Diffs

When `scripts/oracle.sh` is enabled explicitly with `PUML_ORACLE_JAR`, each
fixture is classified as:
```json
{
  "path": "tests/fixtures/basic/hello.puml",
  "category": "drift",
  "metrics": {
    "elem_count": { "ours": 12, "ref": 11, "drift_pct": 9 }
  }
}
```
The JAR is not a runtime dependency, not a build dependency, and is not used by
normal `cargo test` or render commands.

### `oracle_report_summary.json` and `oracle_report.md`

Machine-readable and human-readable summaries produced by
`scripts/oracle_report_summary.py` from a live JAR-backed `oracle_report.json`.
The compact summary records the PlantUML JAR version, fixture count, match
percentage, fixture-level pass/advisory/fail counts, raw category counts, and
top drift families with representative fixtures. The Markdown report repeats
those details for CI summaries and reviewer comments.

These files are generated in CI and uploaded with the oracle artifact. On
`main`, the same data is copied into `target/oracle-report-pages/` and uploaded
as a named Pages artifact. The report is conformance evidence, not a
pixel-perfect parity claim.

## Artifact Retention Policy

- PR runs: 14 days (uploaded by `pr-gate.yml`)
- Main runs: 90 days (uploaded by `main-gate.yml`)
- JAR-backed oracle reports: 90 days (`oracle-report-<run_number>`, plus
  `oracle-report-pages-<run_number>` on `main`)
- Release binaries: 7 days staging (GitHub Release assets are permanent)

Artifacts older than the retention window are automatically deleted by GitHub.
To preserve a specific run permanently, download the artifact and store it
externally, or attach it to a GitHub Release.
