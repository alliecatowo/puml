# puml

`puml` is a CLI scaffold for a PlantUML-style processing pipeline.

## Quick Launch

```bash
cargo run -- --help
cargo run -- tests/fixtures/single_valid.puml
cat tests/fixtures/single_valid.puml | cargo run -- --check -
cargo run -- --multi tests/fixtures/multi_valid.puml
cargo run -- --format json tests/fixtures/single_valid.puml
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

Coverage breadth:
- CLI integration coverage: `tests/integration.rs` (valid/invalid fixtures, stdin/path modes, multi/single flows, exit behavior).
- Render e2e checks: `tests/render_e2e.rs` (deterministic SVG output and SVG safety invariants).
- Exit code contract: `tests/coverage_contract.rs`.

Snapshot files live in `tests/snapshots/` and are asserted with `insta`.

## Benchmark Placeholder

Use `scripts/bench.sh` to run baseline CLI timing checks with the release binary.

| Benchmark | Input | Status | Notes |
|---|---|---|---|
| cli_single_text | single_valid.puml | implemented placeholder | text output path |
| cli_single_json | single_valid.puml | implemented placeholder | `--format json` path |
| cli_multi_json | structure/multi_three.puml | implemented placeholder | validate `--multi` throughput |
| check_mode_ok | basic/valid_start_end.puml | implemented placeholder | validation-only success timing |
| check_mode_err | errors/invalid_plain.txt | implemented placeholder | expected failure path timing |

## Compatibility Notes

- Dependencies are intentionally unchanged in this pass.
- Output snapshots include absolute fixture source paths and are currently asserted as-is.
- Render e2e tests validate deterministic scaffolded SVG output and basic SVG active-content safety patterns.

## License

MIT. See [LICENSE](./LICENSE).
