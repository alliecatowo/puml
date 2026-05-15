# Codex Workflow

This repository is optimized for terminal-first agent and human collaboration.

## One-Command DX Entry Points

```console
./scripts/setup.sh      # one-time workstation setup
./scripts/dev.sh        # fast daily loop
./scripts/check-all.sh  # full quality gate
./scripts/bench.sh      # benchmark workflow
```

## What Each Command Does

`./scripts/setup.sh`
- verifies `cargo` and `rustup`
- ensures `rustfmt`, `clippy`, and `llvm-tools-preview`
- installs `cargo-llvm-cov` if missing
- runs `cargo fetch` and `cargo build`

`./scripts/dev.sh`
- runs `cargo fmt`
- runs `cargo clippy --all-targets --all-features -- -D warnings`
- runs `cargo test`

`./scripts/check-all.sh`
- runs `cargo fmt --check`
- runs `cargo clippy --all-targets --all-features -- -D warnings`
- runs `cargo test`
- runs `cargo llvm-cov --all-features --workspace --fail-under-lines 90`

`./scripts/bench.sh`
- builds `target/release/puml`
- runs benchmark scenarios via `hyperfine` when available
- falls back to `/usr/bin/time` when `hyperfine` is unavailable
- writes benchmark artifacts to `docs/benchmarks/latest.{md,csv,json}`

## Useful Render/Debug Commands

```console
cargo run -- tests/fixtures/basic/hello.puml
cargo run -- tests/fixtures/basic/hello.puml -o out.svg
cat tests/fixtures/basic/hello.puml | cargo run -- -
cat tests/fixtures/structure/multi_three.puml | cargo run -- --multi -

cargo run -- --check tests/fixtures/basic/hello.puml
cargo run -- --dump ast tests/fixtures/basic/hello.puml
cargo run -- --dump model tests/fixtures/basic/hello.puml
cargo run -- --dump scene tests/fixtures/basic/hello.puml
```

## Include Behavior

- File input: includes resolve relative to the input file directory.
- Stdin input: requires `--include-root DIR`.

```console
cat diagram.puml | cargo run -- --check --include-root ./tests/fixtures/include -
```
