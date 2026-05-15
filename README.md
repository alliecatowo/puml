# puml

`puml` is a PlantUML-style sequence diagram CLI.

## Quick Start

```bash
cargo run -- --help
cargo run -- tests/fixtures/single_valid.puml
cat tests/fixtures/single_valid.puml | cargo run -- --check -
cargo run -- --dump ast tests/fixtures/single_valid.puml
cargo run -- --dump model tests/fixtures/single_valid.puml
cargo run -- --dump scene tests/fixtures/single_valid.puml
cargo run -- --multi tests/fixtures/multi_valid.puml
```

## CLI Behavior

- Input:
- file path argument
- `-` for stdin
- omitted path reads stdin
- Modes:
- default renders SVG
- `--check` parses + normalizes only
- `--dump ast|model|scene` prints JSON
- `--multi` permits multiple diagrams from one input
- Output:
- single diagram from file writes `<input-stem>.svg`
- single diagram from stdin writes SVG to stdout
- multi diagram from stdin + `--multi` writes JSON array to stdout
- explicit `--output` writes that path for single diagrams, numbered paths for multi

Exit codes:
- `0` success
- `1` validation/usage failure
- `2` I/O failure
- `3` internal failure

## Testing

```bash
cargo fmt
cargo test
```

Test suites:
- CLI integration: `tests/integration.rs`
- Render e2e: `tests/render_e2e.rs`
- Exit-code contract: `tests/coverage_contract.rs`

## License

MIT. See [LICENSE](./LICENSE).
