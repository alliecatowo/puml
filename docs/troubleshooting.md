# Troubleshooting

## `cargo llvm-cov` command not found

Symptoms:
- `./scripts/check-all.sh` exits early with a coverage-tooling message.

Fix:

```console
./scripts/setup.sh
```

Manual fallback:

```console
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov
```

## Coverage gate fails below 90%

Symptoms:
- `cargo llvm-cov --all-features --workspace --fail-under-lines 90` fails.

Fix approach:
- Inspect low-coverage modules in output.
- Add focused tests under `tests/` and/or fixtures under `tests/fixtures/`.
- Re-run `./scripts/check-all.sh`.

Reference: `docs/coverage-status.md`.

## Benchmark runs are noisy or unavailable

Symptoms:
- benchmark output varies significantly between runs
- `hyperfine` not found

Notes:
- `scripts/bench.sh` automatically falls back to `/usr/bin/time`.
- For more stable timing, install `hyperfine` and reduce machine load.

## `--include-root` errors in stdin mode

Symptoms:
- include-related failures when running via stdin.

Cause:
- stdin has no file-relative directory context.

Fix:

```console
cat diagram.puml | cargo run -- --check --include-root ./tests/fixtures/include -
```

## Diagnostics are hard to map back to source

Symptoms:
- validation or warning messages are unclear without location context.

Notes:
- source-related diagnostics include `line`, `column`, and a caret-marked source snippet in `--check`, `--dump`, and render modes.
- messages without source spans stay single-line by design.

## Snapshot tests fail

Symptoms:
- failing `.snap` assertions in `tests/snapshots/`.

Fix workflow:
- Review whether output change is intentional.
- If intentional, update snapshots with `INSTA_UPDATE=always cargo test`.
- If unintentional, fix code and keep existing snapshots.

Reference: `docs/fixture-snapshot-workflow.md`.
