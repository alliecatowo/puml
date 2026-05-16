# Benchmark Artifact History

## Overview

Every push to `main` that passes the `Main Gate` workflow produces a set of
benchmark artifacts that are uploaded to GitHub Actions as named artifacts and
retained for **90 days**.  Two separate artifact bundles are produced per run:

| Artifact name | Retention | Contents |
|---|---|---|
| `main-benchmarks-<sha>` | 90 days | Full benchmark suite (all files listed below) |
| `parity-report-<sha>` | 90 days | Parity/oracle subset: `parity_latest.json`, `latest_trend.json`, `latest_trend.md` |

## Artifact Files

### `parity_latest.json`
Machine-readable parity report produced by `scripts/parity_harness.py`.

Schema version: `1.0.0`

Top-level keys:
- `schema_version` — always `"1.0.0"` so consumers can version-gate parsing.
- `generated_at_utc` — ISO-8601 UTC timestamp of the run.
- `tool` — metadata about the runner (name, CWD, quick mode flag).
- `oracle` — oracle integration status (`mode: "todo"` until PlantUML JAR is wired in).
- `summary` — aggregate counts: `total`, `check_passed`, `check_failed`, `render_passed`, `render_failed`.
- `fixtures` — per-fixture records (see schema below).
- `doc_examples` — drift detection results for every `.puml` snippet or linked file in `docs/examples/`.

Each `fixtures[]` entry contains:
```json
{
  "fixture": "basic/hello.puml",
  "check":  { "passed": true, "exit_code": 0, "diagnostics": [], "stderr": "" },
  "render": { "attempted": true, "passed": true, "exit_code": 0, "stderr": "",
              "metadata": { "svg_bytes": 1234, "viewbox": { "x":0,"y":0,"width":400,"height":200 } } },
  "oracle": { "status": "todo", "comparison": null, "notes": "..." }
}
```

When the PlantUML oracle is active (`oracle.status != "todo"`), the `oracle.comparison`
field will hold a structured diff summary (see `scripts/oracle.sh` output format).

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

1. Download the `parity-report-<sha>` artifact from the Actions run you care about.
2. Open `latest_trend.md` for a quick human summary.
3. For programmatic analysis, parse `latest_trend.json`:
   - Any `gate_passed: false` entry is a regression against the baseline.
   - `mean_ms` values > `gate_limit_ms` indicate absolute limit breaches.

## Oracle Diffs (future)

When `scripts/oracle.sh` is enabled (PlantUML JAR present), each fixture's
`oracle.comparison` will contain:
```json
{
  "strategy": "byte",
  "identical": false,
  "diff_bytes": 42,
  "notes": "SVG whitespace difference at byte offset 1024"
}
```
The `parity-report-<sha>` artifact will then include a `oracle_diff_count` summary
field at the top level.

## Artifact Retention Policy

- PR runs: 14 days (uploaded by `pr-gate.yml`)
- Main runs: 90 days (uploaded by `main-gate.yml`)
- Release binaries: 7 days staging (GitHub Release assets are permanent)

Artifacts older than the retention window are automatically deleted by GitHub.
To preserve a specific run permanently, download the artifact and store it
externally, or attach it to a GitHub Release.
