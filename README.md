# puml

`puml` is a Rust CLI for rendering a focused, validated subset of PlantUML-style sequence diagrams to SVG.

![crate](https://img.shields.io/badge/crate-0.1.0-blue)
![rust edition](https://img.shields.io/badge/rust-2021-orange)
![license](https://img.shields.io/badge/license-MIT-green)
![tests](https://img.shields.io/badge/tests-cargo%20test-informational)

## Developer Experience Quickstart

One-command setup:

```bash
./scripts/setup.sh
```

One-command full quality gate:

```bash
./scripts/check-all.sh
```

One-command benchmark workflow:

```bash
./scripts/bench.sh
```

Outputs:
- `docs/benchmarks/latest.md`
- `docs/benchmarks/latest.csv`
- `docs/benchmarks/latest.json`

## Daily Development Commands

Fast local loop:

```bash
./scripts/dev.sh
```

CLI quick checks:

```bash
# Explore CLI options
cargo run -- --help

# Render a diagram file to <input-stem>.svg
cargo run -- tests/fixtures/single_valid.puml

# Validate from stdin without rendering
cat tests/fixtures/single_valid.puml | cargo run -- --check -

# Inspect pipeline stages
cargo run -- --dump ast tests/fixtures/single_valid.puml
cargo run -- --dump model tests/fixtures/single_valid.puml
cargo run -- --dump scene tests/fixtures/single_valid.puml

# Enable multi-diagram mode
cargo run -- --multi tests/fixtures/multi_valid.puml
```

## CLI Contract

Inputs:
- `INPUT` file path
- `-` for stdin
- omitted `INPUT` reads stdin

Modes:
- default mode renders SVG
- `--check` parses + normalizes only
- `--dump ast|model|scene` emits JSON
- `--multi` allows multiple diagrams from one input
- `--include-root DIR` sets `!include` resolution root when reading stdin

Outputs:
- single diagram from file writes `<input-stem>.svg`
- single diagram from stdin writes SVG to stdout
- multi diagram from stdin + `--multi` writes a JSON array to stdout
- `--output PATH` writes to that path for single diagrams, and numbered paths for multi

Exit codes:
- `0` success
- `1` validation or usage failure
- `2` I/O failure
- `3` internal failure

Warnings:
- unsupported `skinparam` keys and `!theme` are emitted to `stderr` as deterministic non-fatal warnings in `--check`, `--dump`, and render modes
- warnings do not change exit code when no hard validation error occurs

Diagnostics:
- source-related warnings and validation errors include `line`/`column` plus a source caret snippet in `--check`, `--dump`, and render modes
- diagnostics without source spans remain plain one-line messages

Example:

```text
[E_ARROW_INVALID] malformed arrow syntax at line 2, column 1
A -x B: malformed
^^^^^^
```

## Feature Matrix

| Area | Status | Notes |
|---|---|---|
| Sequence diagrams | Supported | Non-sequence families are rejected. |
| `@startuml` / `@enduml` blocks | Supported | Also accepts plain single-diagram text input. |
| Participants + aliases | Supported | `participant`, `actor`, `boundary`, `control`, `entity`, `database`, `collections`. |
| Messages + common arrows | Supported | Includes forms like `->`, `-->`, `<-` with optional labels. |
| Notes, groups, separators | Supported | Includes `alt`, `else`, `opt`, `loop`, `par`, `critical`, `break`, `group`, `end`, plus `...`, `||`, `newpage`. |
| Lifecycle/control statements | Supported | `activate`, `deactivate`, `create`, `destroy`, `return`, `autonumber`. |
| Metadata statements | Supported | `title`, `header`, `footer`, `caption`, `legend`, `hide footbox`, `show footbox`. |
| `skinparam maxmessagesize` | Supported | Accepted and normalized. |
| Other `skinparam` keys | Accepted with warning | Deterministic `stderr` warning; continues execution. |
| `!include`, `!define`, `!undef` | Supported (scoped) | Relative includes, simple define/undef substitution, cycle/depth guards. |
| Multi-diagram input | Guarded support | Requires explicit `--multi`. |

## Text Overflow Policy

`LayoutOptions` supports two participant/title label overflow behaviors:

- `TextOverflowPolicy::WrapAndGrow` (default): wraps long labels and grows label containers vertically.
- `TextOverflowPolicy::EllipsisSingleLine`: keeps single-line labels and truncates overflow with `…`.

## Docs Map

- Developer flow: [`docs/codex-workflow.md`](docs/codex-workflow.md)
- Contribution guide: [`docs/contributing.md`](docs/contributing.md)
- Troubleshooting guide: [`docs/troubleshooting.md`](docs/troubleshooting.md)
- Fixture and snapshot workflow: [`docs/fixture-snapshot-workflow.md`](docs/fixture-snapshot-workflow.md)
- Benchmark details: [`docs/benchmarks/README.md`](docs/benchmarks/README.md)
- Parity roadmap: [`docs/parity-roadmap.md`](docs/parity-roadmap.md)
- Release checklist: [`docs/release-checklist.md`](docs/release-checklist.md)
- Coverage status: [`docs/coverage-status.md`](docs/coverage-status.md)
- Decision log: [`docs/decision-log.md`](docs/decision-log.md)

## License

MIT. See [LICENSE](./LICENSE).
