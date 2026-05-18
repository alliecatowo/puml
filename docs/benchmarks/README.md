# Benchmarks

## Commands

```console
# full benchmark artifact refresh (records trend, does not fail on gates)
./scripts/bench.sh

# validate checked-in benchmark artifacts against the current gate policy
./scripts/bench.sh --check-artifacts

# quick local benchmark
./scripts/bench.sh --quick

# enforce perf + binary gates (used by check-all)
./scripts/bench.sh --enforce-gates
./scripts/bench.sh --quick --enforce-gates

# refresh mode baseline only after an intentional review
./scripts/bench.sh --update-baseline
./scripts/bench.sh --quick --update-baseline

# corpus parity baseline report (oracle placeholders kept intentionally)
python3 scripts/parity_harness.py --output docs/benchmarks/parity_latest.json

# deterministic Java-free differential oracle metadata report
python3 scripts/differential_oracle_smoke.py --dry-run --quiet --output docs/benchmarks/oracle_smoke_latest.json

# optional live differential oracle evidence when PlantUML is installed
python3 scripts/differential_oracle_smoke.py --quick --strict --quiet --output docs/benchmarks/oracle_smoke_latest.json

# summarize a JAR-backed oracle_report.json for CI/Pages artifacts
python3 scripts/oracle_report_summary.py \
  --input docs/benchmarks/oracle_report.json \
  --markdown docs/benchmarks/oracle_report.md \
  --json docs/benchmarks/oracle_report_summary.json \
  --pages-dir target/oracle-report-pages
```

## Artifacts

- `docs/benchmarks/latest.md`
- `docs/benchmarks/latest.csv`
- `docs/benchmarks/latest.json`
- `docs/benchmarks/latest_trend.md`
- `docs/benchmarks/latest_trend.json`
- `docs/benchmarks/baseline_full.json`
- `docs/benchmarks/baseline_quick.json`
- `docs/benchmarks/parity_latest.json`
- `docs/benchmarks/oracle_report.json`
- `docs/benchmarks/oracle_report.md` (generated in CI from live JAR evidence)
- `docs/benchmarks/oracle_report_summary.json` (generated in CI from live JAR evidence)
- `docs/benchmarks/oracle_smoke_latest.json`
- `docs/benchmarks/oracle_evidence_refresh.md` (stable local live-oracle audit summary)

All benchmark artifacts are deterministic in structure and key ordering. Value fields like timestamps, host metadata, and measured timings are expected to change run-to-run.
Gate-bearing JSON artifacts (`latest.json`, `latest_trend.json`, `baseline_full.json`, and `baseline_quick.json`) include `benchmark_policy.version` metadata. Run `./scripts/bench.sh --check-artifacts` before release review or after changing gate limits so stale committed evidence cannot silently drift from the active policy.

## Gate Profiles

Current policy version: `bench-gate-v2-2026-05-17`.

- `full` (default):
- absolute per-scenario mean limit: `250ms`
- regression limit vs previous `baseline_full.json`: `10%` with absolute delta floor `>40ms`
- binary size limit (`target/release/puml`): `12,000,000` bytes
- `quick` (`--quick`):
- absolute per-scenario mean limit: `350ms`
- regression limit vs previous `baseline_quick.json`: `20%` with absolute delta floor `>50ms`
- binary size limit (`target/release/puml`): `12,000,000` bytes

If no matching mode baseline exists, regression checks are skipped and absolute/binary checks still apply.

The binary gate was recalibrated after URL include support added the `ureq`/`rustls`/`ring`
dependency path. Current release builds are about 10 MB, so the 12 MB ceiling preserves
headroom for normal metadata and compiler variance while still catching large dependency or
asset growth. Treat binary-size reduction as a separate product goal with an explicit issue
or optimization plan instead of blocking all main merges on the pre-URL-include 2 MB limit.

## Failure Handling

- `./scripts/bench.sh` reports gate warnings but exits `0` by default.
- `./scripts/bench.sh --enforce-gates` exits non-zero on any gate failure.
- `./scripts/check-all.sh` always runs benchmark gates in enforce mode.
- On failure, inspect `docs/benchmarks/latest_trend.{md,json}` to identify the exact regressing scenario and delta.
- Baselines are not auto-updated. Use `--update-baseline` only after reviewing variance and approving movement.
- If benchmark gate limits or policy metadata change, refresh committed artifacts with `./scripts/bench.sh --quick --update-baseline` and `./scripts/bench.sh --update-baseline`, then confirm `./scripts/bench.sh --check-artifacts` passes.

## No-Java Baseline

- PlantUML oracle comparison is opt-in and remains skip-sentinel based unless `PUML_ORACLE_JAR` is set.
- `parity_latest.json`, `latest_trend.json`, and `oracle_report.json` may include explicit oracle placeholder or skip metadata.
- CI-generated `oracle_report.md` and `oracle_report_summary.json` are derived from the live JAR-backed `oracle_report.json`; they are uploaded as artifacts and, on `main`, as a named Pages artifact directory.
- Oracle artifacts are comparison evidence only. They are not runtime inputs and do not imply that Java/PlantUML is used by the `puml` CLI or library.
- Do not remove placeholders until Java/oracle execution is intentionally enabled for the relevant workflow.

## Differential Oracle Smoke

- CI workflow: `.github/workflows/differential-oracle-smoke.yml`
- Default artifact mode: `--dry-run` metadata, fixed fixture corpus, expected drift categories, ranked top drift areas, no renderer execution.
- Optional live mode: small fixed corpus, token-presence checks, viewBox checks, structured JSON report.
- Runtime requirements: none for `--dry-run`; optional live mode requires `plantuml`, `java` (headless), and any PlantUML-side dependencies such as `graphviz`.
- Stable local live-run findings are summarized in `docs/benchmarks/oracle_evidence_refresh.md`; keep timestamped live JSON under `target/` unless intentionally refreshing checked-in evidence.
