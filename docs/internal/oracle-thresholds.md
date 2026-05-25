# Oracle Conformance Thresholds

This document describes how `scripts/oracle.sh` categorizes each fixture,
what the exit-code thresholds mean, and how to run the suite locally.

## Overview

The oracle suite is comparison-only conformance tooling. The full shell oracle
compares SVG output produced by **puml** (our Rust renderer) against the
**Java PlantUML reference JAR** on every `.puml` fixture under
`tests/fixtures/` and `docs/examples/`.

In CI, `.github/workflows/oracle.yml` downloads the pinned PlantUML release
JAR `plantuml-1.2026.3.jar` from the official PlantUML GitHub release URL,
caches it by version, verifies that it exists, prints `java -jar ... -version`,
builds the release `puml` binary, and then runs `scripts/oracle.sh`. The
workflow intentionally uses a pinned JAR; it does not use `latest`.

The Java PlantUML JAR is never part of the `puml` runtime path, never a fallback
renderer, and never required for normal CLI/library rendering. It is used only
when an audit, local run, or CI workflow explicitly sets `PUML_ORACLE_JAR`.
Normal `cargo test`, `cargo run`, and rendering paths do not need Java and do
not download or execute a JAR.

For Java-free audits, `scripts/differential_oracle_smoke.py --dry-run` emits the
same fixture manifest, expected comparison categories, ranked top drift
categories, and next-ticket hints without executing the local renderer,
PlantUML, Java, or any JAR. This makes partial PlantUML gaps visible as
fixture-backed expected drift categories before an optional external oracle is
available.

Each fixture is placed into exactly one category:

| Category | Meaning |
|---|---|
| `match` | Both sides render; all metrics are within 10 % of each other |
| `drift` | Both sides render; at least one metric deviates by more than 10 % |
| `puml-only` | Our renderer produces SVG; the reference JAR fails or produces nothing |
| `jar-only` | The reference JAR produces SVG; our renderer fails |
| `both-fail` | Neither side produces usable SVG |

These categories are report classifications, not runtime behavior. A fixture
classified as `drift`, `jar-only`, or `puml-only` is evidence for a known parity
gap; it does not enable a fallback renderer.

## Promoted Fixture Gate

The broad oracle corpus remains advisory except for the below-50% match hard
failure described later in this document. A smaller promoted fixture list in
`tests/oracle_promoted_fixtures.json` is the blocking parity contract: every
promoted fixture defaults to `allowed_categories: ["match"]`.

Validate the promoted manifest without Java, PlantUML, or an oracle report:

```bash
python3 scripts/oracle_promoted_gate.py \
  --manifest tests/oracle_promoted_fixtures.json \
  --validate-manifest-only \
  --repo-root .
```

When `scripts/oracle.sh` completes a live JAR-backed run, it invokes
`scripts/oracle_promoted_gate.py` against the generated `oracle_report.json`.
The gate annotates the report with a `promoted_gate` object and exits `3` if a
promoted fixture appears as unexpected `drift`, `puml-only`, `jar-only`, or
parse-fail (`both-fail`) output, or if the promoted fixture disappeared from the
oracle report. CI treats exit `3` as blocking.

Intentional promoted gaps must be explicit in the manifest, for example:

```json
{
  "path": "tests/fixtures/basic/some_fixture.puml",
  "allowed_categories": ["match", "drift"],
  "reason": "tracked layout-only drift while semantic parity work continues"
}
```

That explicit allowance is for fixtures that are still important enough to keep
promoted, but whose current oracle category is a known and reviewed exception.
The exploratory full-corpus oracle report still lists all drift, JAR-only,
puml-only, and parse-fail categories for triage; those rows are advisory unless
they are also in the promoted manifest without an explicit expectation.

## Dry-run schema

`scripts/differential_oracle_smoke.py --dry-run` writes
`docs/benchmarks/oracle_smoke_latest.json` by default, or a custom path via
`--output`. Schema `1.2.0` adds `classification` metadata to every fixture:

```json
{
  "fixture": "families/valid_salt_login_form.puml",
  "classification": {
    "category": "family-partial",
    "support_status": "partial",
    "expected_oracle_category": "drift",
    "drift_area": "Salt widget breadth",
    "drift_reason": "Salt widget breadth is intentionally narrower than the Java PlantUML reference",
    "next_ticket": "Expand Salt widget/layout parity around form controls, menus, tables, and style propagation.",
    "plantuml_reference": "https://plantuml.com/salt"
  },
  "local": { "attempted": false },
  "oracle": { "attempted": false },
  "comparison": { "state": "not-run", "passed": null }
}
```

The dry-run summary includes:

- `not_run` — all selected fixtures in dry-run mode.
- `by_fixture_category` — deterministic fixture category counts.
- `by_support_status` — implemented vs partial fixture counts.
- `by_expected_oracle_category` — expected `match`, `drift`, `jar-only`, `puml-only`, or `both-fail` counts.
- `top_expected_drift_categories` — ranked fixture categories excluding expected matches.
- `top_expected_drift_areas` — ranked implementation areas excluding expected matches, with representative fixtures and next-ticket hints.

The report also states `comparison_only: true`, `runtime_dependency: false`,
`build_dependency: false`, and `normal_cargo_test_uses_oracle: false`.
Dry-run `generated_at_utc` is pinned to `1970-01-01T00:00:00Z` and `tool.cwd`
is reported as `repo-root` so the checked-in artifact is deterministic.
Optional live oracle runs keep a real UTC timestamp and absolute working
directory for debugging.

## Metrics

When both sides render, four metric families are compared:

### 1. SVG element count (`elem_count`)

Counts occurrences of the structural SVG tags: `<rect>`, `<text>`, `<line>`,
`<polygon>`, `<circle>`, `<path>`.

Drift threshold: **10 %** (absolute percentage difference relative to the
reference count).

### 2. viewBox dimensions (`viewbox`)

Extracts the width (`W`) and height (`H`) from the `viewBox="x y W H"`
attribute of the root `<svg>` element.

Both `W` and `H` are compared independently; each must be within **10 %** of
the reference value.

### 3. Text content set (`text_set`)

Extracts the inner text of every `<text>` element, strips whitespace, sorts
the unique strings, and compares the resulting sets.

Threshold: **exact set equality**.  Any difference in the sorted, unique text
strings counts as a mismatch.

### 4. Colour palette (`color_set`)

Extracts all `fill="#…"` and `stroke="#…"` hex colour values (3–8 hex digits),
lower-cases them, de-duplicates, and sorts them.

Threshold: **exact set equality**.  Any difference in the sorted, unique colour
strings counts as a mismatch.

## Categorization algorithm

```
if neither side renders  → both-fail
if only ours renders     → puml-only
if only JAR renders      → jar-only
if elem_count drift > 10%   OR
   viewbox W drift > 10%    OR
   viewbox H drift > 10%    OR
   text_set differs          OR
   color_set differs         → drift
otherwise                    → match
```

## Exit codes

| Exit code | Condition | Meaning |
|---|---|---|
| `0` | `PUML_ORACLE_JAR` unset | Skip sentinel (CI-safe) |
| `0` | match% ≥ 80 % | Conformance is good |
| `1` | 50 % ≤ match% < 80 % | Advisory warning; CI passes and PR comment reports `WARN` |
| `2` | match% < 50 % | Hard failure; CI blocks the PR |
| `3` | promoted fixture gate failed | Hard failure; CI blocks the PR |

Only `match` contributes to match%. Fixtures in `drift`, `puml-only`,
`jar-only`, and `both-fail` all reduce the match percentage because the
denominator is the full fixture count.

The GitHub Actions gate in `.github/workflows/oracle.yml` treats exit codes `2`
and `3` as blocking. Exit codes `0` and `1` are converted to a successful
workflow step, so broad SVG metric drift, JAR-only fixtures, puml-only fixtures,
and parse failures are advisory unless the overall match percentage drops below
50% or the row belongs to the promoted fixture list without an explicit
expectation. The PR comment summarizes the categories so reviewers can triage
regressions even when the full-corpus report remains advisory.

The generated CI summary is intentionally worded as conformance evidence. It is
not a pixel-perfect parity claim: puml and Java PlantUML use different layout
engines, so SVG geometry drift is useful triage data rather than proof that a
supported construct is semantically wrong.

## Report format

After every run (when `PUML_ORACLE_JAR` is set and the JAR is valid), the full
report is written to `docs/benchmarks/oracle_report.json` with this schema:

```json
{
  "schema_version": "1.0",
  "timestamp": "<iso-utc>",
  "jar_version": "<java -jar plantuml.jar -version first line>",
  "summary": {
    "total": 0,
    "match": 0,
    "drift": 0,
    "puml_only": 0,
    "jar_only": 0,
    "both_fail": 0
  },
  "promoted_gate": {
    "status": "pass",
    "total": 2,
    "passed": 2,
    "violations": []
  },
  "fixtures": [
    {
      "path": "tests/fixtures/basic/hello.puml",
      "category": "match",
      "metrics": {
        "elem_count": { "ours": 12, "ref": 11, "drift_pct": 9 },
        "viewbox":    { "ours": "200 100", "ref": "198 102", "w_drift_pct": 1, "h_drift_pct": 1 },
        "text_set":   { "match": true },
        "color_set":  { "match": true }
      }
    }
  ]
}
```

CI then runs `scripts/oracle_report_summary.py` over that raw report and writes:

- `docs/benchmarks/oracle_report.md` — human-readable CI summary.
- `docs/benchmarks/oracle_report_summary.json` — compact machine-readable
  summary with `jar_version`, `fixture_count`, `match_pct`, fixture-level
  pass/advisory/fail counts, category counts, and ranked top drift families.
- `target/oracle-report-pages/` — static `index.html` plus the raw JSON,
  compact JSON, and Markdown summary. On `main` pushes the oracle workflow
  uploads this directory as a named Pages artifact so the latest JAR-backed
  report can be retained separately from the checked-in skip sentinel.

`oracle_report.json` remains the raw fixture-level evidence. The summary files
exist to make CI and Pages artifacts easier to inspect without asking reviewers
to hand-query JSON.

## Running the oracle locally

### Prerequisites

- Java 17+ on your PATH (or set `PUML_ORACLE_JAVA` to the full path)
- `plantuml.jar` downloaded somewhere on disk

### Download the reference JAR

```sh
curl -fsSL \
  https://github.com/plantuml/plantuml/releases/download/v1.2024.7/plantuml-1.2024.7.jar \
  -o /tmp/plantuml-1.2024.7.jar
```

### Build the release binary

```sh
cargo build --release
```

### Run the suite

```sh
PUML_ORACLE_JAR=/tmp/plantuml-1.2024.7.jar ./scripts/oracle.sh
```

The summary is printed to stdout as JSON and the full report is written to
`docs/benchmarks/oracle_report.json`.

### Limit the corpus to a specific directory

```sh
PUML_ORACLE_JAR=/tmp/plantuml-1.2024.7.jar \
  ./scripts/oracle.sh --corpus-dir tests/fixtures/basic
```

### Run against examples only

```sh
PUML_ORACLE_JAR=/tmp/plantuml-1.2024.7.jar \
  ./scripts/oracle.sh --corpus-dir /dev/null --examples-dir docs/examples
```

### Skip the oracle entirely

Unset `PUML_ORACLE_JAR` (or just don't set it).  The script exits 0 and
writes the skip sentinel to the report file — safe to call unconditionally
from any CI pipeline.

The skip sentinel means "comparison not run"; it does not mean parity passed or
failed.

### Run the Java-free dry-run manifest

```sh
python3 ./scripts/differential_oracle_smoke.py --dry-run --output target/oracle-smoke-dry.json
```

This command is suitable for normal Rust development environments because it
does not execute Java, does not require `plantuml.jar`, and does not invoke
`cargo run`. It is the preferred always-available oracle-p0 artifact path.

## CI integration

The oracle comparison workflow runs on every pull request targeting `main`,
on pushes to `main`, and on manual `workflow_dispatch`. It is not path-filtered.
See `.github/workflows/oracle.yml` for the full pipeline definition.

The CI workflow publishes one artifact named `oracle-report-<run_number>` with:

- `docs/benchmarks/oracle_report.json`
- `docs/benchmarks/oracle_report_summary.json`
- `docs/benchmarks/oracle_report.md`
- `target/oracle-report-pages/**`

The normal artifact is retained for 90 days. On pushes to `main`, the workflow
also uploads `target/oracle-report-pages` as a named Pages artifact
`oracle-report-pages-<run_number>`.

When the optional sentinel test step runs, CI also publishes a separate
`oracle-smoke-test-log-<run_number>` artifact containing:

- `/tmp/oracle_smoke_test.log`

On pull requests, the workflow also posts a Markdown summary comment when
`scripts/oracle.sh` produced a real report with a numeric fixture total.

After the shell oracle, CI runs `cargo test --test oracle_smoke` without
`--include-ignored`. That means the always-on sentinel tests run, while the
ignored `oracle_report_schema_is_stable` integration test is skipped in CI
because the full JAR-backed corpus run already happened in `scripts/oracle.sh`.
This smoke-test step is `continue-on-error: true`; failures are uploaded in the
artifact log for diagnosis but do not block the PR. The shell oracle step is the
only blocking oracle gate, and only when it returns exit code `2`.

CI does not commit or push generated reports, update baselines, or change source
files. The generated report exists only in the job workspace and uploaded
artifact unless a developer explicitly commits a local report update.

For local investigation, prefer writing reports under `target/` when you do not
intend to update checked-in benchmark evidence:

```sh
PUML_ORACLE_JAR=/tmp/plantuml-1.2024.7.jar \
  ./scripts/oracle.sh --report-file target/oracle_report.json
```
