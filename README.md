# puml

`puml` is a CLI scaffold for a PlantUML-style processing pipeline.

## Quick Launch

```bash
cargo run -- --help
cargo run -- tests/fixtures/single_valid.puml
cat tests/fixtures/single_valid.puml | cargo run -- --check -
cargo run -- --multi tests/fixtures/multi_valid.puml
```

## CLI Contract (Worker C)

- Clap v4 argument parsing and help output.
- Exit codes:
- `0`: success.
- `2`: command/argument usage error.
- `3`: IO read failures.
- `4`: input contract failure (empty or multi without `--multi`).
- `5`: `--check` validation failure.
- Input sources:
- positional file path.
- `-` for stdin.
- omitted path reads stdin.
- Modes:
- `--check` validates without render output.
- `--dump` prints JSON diagram records.
- `--multi` allows multiple diagrams and returns JSON array.

## Testing

```bash
cargo test
```

Integration tests live in `tests/integration.rs` and use `insta` snapshots for output contracts.

## Benchmark Placeholder

Use `scripts/bench.sh` to run baseline micro-bench placeholders.

| Benchmark | Input | Status | Notes |
|---|---|---|---|
| cli_single_text | single_valid.puml | placeholder | add criterion or hyperfine run |
| cli_multi_json | multi_valid.puml | placeholder | validate `--multi` throughput |
| check_mode | single_valid.puml | placeholder | capture validation-only timing |

## License

MIT. See [LICENSE](./LICENSE).
