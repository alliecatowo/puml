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

Input:
- file path argument
- `-` for stdin
- omitted path reads stdin

Modes:
- default renders SVG
- `--check` parses + normalizes only
- `--dump ast|model|scene` prints JSON
- `--multi` permits multiple diagrams from one input
- `--include-root DIR` enables `!include` when reading from stdin

Output:
- single diagram from file writes `<input-stem>.svg`
- single diagram from stdin writes SVG to stdout
- multi diagram from stdin + `--multi` writes JSON array to stdout
- explicit `--output` writes that path for single diagrams, numbered paths for multi

## What Works Today

- Sequence diagrams only.
- `@startuml` / `@enduml` blocks and plain single-diagram text input.
- Participants: `participant`, `actor`, `boundary`, `control`, `entity`, `database`, `collections`, including aliases.
- Messages with common arrow forms (for example `->`, `-->`, `<-`) and optional labels.
- Notes, groups (`alt`, `else`, `opt`, `loop`, `par`, `critical`, `break`, `group`, `end`), and separators (`...`, `||`, `newpage`).
- Lifecycle and control statements: `activate`, `deactivate`, `create`, `destroy`, `return`, `autonumber`.
- Document metadata statements: `title`, `header`, `footer`, `caption`, `legend`, `hide footbox`, `show footbox`.
- `skinparam maxmessagesize` is accepted; other `skinparam` keys currently return a warning error.
- `!include`, `!define`, and `!undef` are parsed but intentionally fail normalization with a placeholder warning.
- Non-sequence diagram syntax (for example class/state diagrams) is rejected during validation.

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
