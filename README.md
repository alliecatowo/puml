# puml

`puml` is a Rust CLI for rendering a focused, validated subset of PlantUML-style sequence diagrams to SVG.

![crate](https://img.shields.io/badge/crate-0.1.0-blue)
![rust edition](https://img.shields.io/badge/rust-2021-orange)
![license](https://img.shields.io/badge/license-MIT-green)
![tests](https://img.shields.io/badge/tests-cargo%20test-informational)

## Why puml

- Purpose-built for sequence diagrams, with clear validation boundaries.
- Deterministic rendering for stable snapshots and CI checks.
- Practical CLI modes for rendering, validation, and JSON introspection.

See the decision log for intentional contract boundaries and deviations: [docs/decision-log.md](./docs/decision-log.md).

## Quickstart

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
| Other `skinparam` keys | Rejected intentionally | Return validation warning error behavior. |
| `!include`, `!define`, `!undef` | Recognized but rejected intentionally | Parsed, then fail normalization as unsupported directives. |
| Multi-diagram input | Guarded support | Requires explicit `--multi`. |

Checklist:
- [x] Render SVG from file or stdin
- [x] Validate syntax/normalization via `--check`
- [x] Dump AST/model/scene JSON for tooling
- [x] Deterministic render behavior covered by snapshots
- [ ] Full PlantUML compatibility (explicitly out of scope)

## Development

```bash
cargo fmt
cargo test
```

Coverage target (line coverage >= 90%):

```bash
cargo llvm-cov --workspace --all-features --lcov --output-path target/lcov.info
```

If `cargo llvm-cov` is not installed locally:

```bash
cargo install cargo-llvm-cov
```

Fallback guidance when LLVM coverage tooling is unavailable in the environment:
- Keep the target command above as the canonical CI/local coverage command.
- Run `cargo test` to validate behavior and use targeted branch tests under `tests/**` as a proxy signal until `cargo llvm-cov` is available.
- Optionally produce a rough per-file heuristic with `cargo test -- --nocapture` plus test-to-module mapping, then rerun the exact `cargo llvm-cov` command once installed.

Current coverage-oriented suites include:
- Parser/preprocess and normalization edge-path tests in `tests/coverage_edges.rs`
- CLI integration and exit-code contract tests in `tests/integration.rs` and `tests/coverage_contract.rs`
- Render/layout deterministic and edge rendering tests in `tests/render_e2e.rs`

Current test suites:
- CLI integration: `tests/integration.rs`
- Render end-to-end snapshots: `tests/render_e2e.rs`
- Exit-code contract: `tests/coverage_contract.rs`

## License

MIT. See [LICENSE](./LICENSE).
