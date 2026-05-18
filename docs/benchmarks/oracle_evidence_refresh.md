# Oracle Evidence Refresh

This note records the current live-oracle evidence shape without committing
timestamped raw reports. It distinguishes Java-backed evidence from the
checked-in skip/dry-run sentinel artifacts.

## CI Oracle Wiring

- Full oracle workflow: `.github/workflows/oracle.yml`.
- Reference runtime: Temurin JDK plus pinned `plantuml-1.2024.7.jar`.
- Download URL: `https://github.com/plantuml/plantuml/releases/download/v1.2024.7/plantuml-1.2024.7.jar`.
- Full oracle command path: `PUML_ORACLE_JAR=<jar> ./scripts/oracle.sh`.
- Smoke workflow: `.github/workflows/differential-oracle-smoke.yml`.
- Smoke command path: `python3 ./scripts/differential_oracle_smoke.py --quick --strict --quiet`.
- Checked-in sentinel path: `scripts/oracle.sh` writes a skipped JSON report when `PUML_ORACLE_JAR` is unset; `scripts/differential_oracle_smoke.py --dry-run` writes metadata only and does not execute Java, PlantUML, or `cargo run`.

The Java PlantUML JAR is comparison-only evidence. It is not a runtime
dependency, build dependency, fallback renderer, or normal `cargo test`
dependency for `puml`.

## Local Java-Backed Runs

Local environment confirmed:

```console
java -version
# openjdk version "21.0.10" 2026-01-20

curl -fL -o plantuml-1.2024.7.jar \
  https://github.com/plantuml/plantuml/releases/download/v1.2024.7/plantuml-1.2024.7.jar

java -jar plantuml-1.2024.7.jar -version
# PlantUML version 1.2024.7 (Sat Sep 07 04:18:17 PDT 2024)
```

Live smoke command:

```console
python3 ./scripts/differential_oracle_smoke.py \
  --oracle-command 'java -jar plantuml-1.2024.7.jar -tsvg -pipe' \
  --strict \
  --quiet \
  --output docs/benchmarks/oracle_smoke_latest.json
```

Result: Java PlantUML actually ran. The strict live smoke exited `5` after
attempting all 13 representative fixtures: 5 passed and 8 failed. The raw JSON
was not kept because it contains a real timestamp and absolute checkout path.

Focused shell-oracle command:

```console
PUML_ORACLE_JAR="$PWD/plantuml-1.2024.7.jar" \
  ./scripts/oracle.sh \
  --corpus-dir tests/fixtures/basic \
  --examples-dir docs/examples/sequence \
  --report-file target/oracle_report_core_slice.json
```

Result: Java PlantUML actually ran. The shell oracle built the release binary,
processed 52 fixtures, and exited `3` because the promoted fixture gate failed.
Summary: 0 match, 50 drift, 1 `puml-only`, 1 `jar-only`, 0 `both-fail`.

## Top Drift Buckets

Live smoke expected-drift categories:

| Category | Fixtures | Expected categories |
|---|---:|---|
| `family-partial` | 5 | `drift` |
| `preprocessor-advanced` | 1 | `jar-only` |
| `styling-partial` | 1 | `drift` |

Live smoke actual failures:

| Area | Evidence |
|---|---|
| Sequence fragments and notes | Java and local renderers both exited 0, but expected semantic tokens were missing in one or both SVG outputs. |
| Unsupported styling | Java and local renderers both exited 0, but token checks failed for the unsupported-skinparam fixture. |
| Advanced preprocessor invocation | Local renderer exited 1 and Java PlantUML exited 200 for the empty dynamic invocation fixture. |
| Salt widget/layout fixtures | Local renderer exited 0 but emitted SVG without a valid `viewBox` for both Salt smoke fixtures. |
| Gantt calendar/resource fixture | Local renderer exited 0, Java PlantUML exited 200, and expected tokens were missing. |
| Component style oracle slice | Local renderer exited 0, Java PlantUML exited 200, and expected tokens were missing. |

Focused shell-oracle drift buckets:

| Bucket | Count |
|---|---:|
| `color_set` mismatch | 50 |
| `viewBox` geometry drift | 49 |
| `text_set` mismatch | 49 |
| Element-count drift over 10% | 11 |
| `jar-only` | 1 |
| `puml-only` | 1 |

The promoted fixtures `tests/fixtures/basic/hello.puml` and
`docs/examples/sequence/01_basic.puml` both classified as `drift` under the
strict shell metrics even though `basic/hello.puml` passed the smoke harness.
That means the current strict metrics are geometry/style/text-set evidence, not
a semantic-only parity pass/fail signal.

## Durable Artifact Policy

- Keep `docs/benchmarks/oracle_smoke_latest.json` in dry-run form unless the
  intent is to commit a timestamped live report.
- Prefer local live reports under `target/` for investigation:
  `target/oracle_report_core_slice.json`,
  `target/oracle_report_summary_sample.*`, or another ignored path.
- Treat checked-in skip sentinels as "comparison did not run"; they are not conformance evidence.
- Treat this markdown file as the stable audit summary for the local live run;
  update it only when intentionally refreshing oracle evidence.
