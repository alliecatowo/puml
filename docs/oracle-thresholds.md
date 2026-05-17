# Oracle Conformance Thresholds

This document describes how `scripts/oracle.sh` categorizes each fixture,
what the exit-code thresholds mean, and how to run the suite locally.

## Overview

The oracle suite is comparison-only conformance tooling. The full shell oracle
compares SVG output produced by **puml** (our Rust renderer) against the
**Java PlantUML reference JAR** on every `.puml` fixture under
`tests/fixtures/` and `docs/examples/`.

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
| `1` | 50 % ≤ match% < 80 % | Soft warning; CI passes but PR comment warns |
| `2` | match% < 50 % | Hard failure; CI blocks the PR |

Only `both-fail` and `match` count as non-failures when computing match%.
Fixtures in `puml-only`, `jar-only`, and `drift` all reduce the match%.

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

The oracle comparison workflow runs automatically on PRs that touch
`src/parser.rs`, `src/normalize.rs`, `src/render.rs`, or any file under
`tests/fixtures/`.
See `.github/workflows/oracle.yml` for the full pipeline definition.
