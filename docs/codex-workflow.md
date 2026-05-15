# Codex Workflow

This repository is optimized for terminal-first agent/human loops.

## Fast local loop

```console
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## Full quality gate (same as CI/local release gate)

```console
./scripts/check-all.sh
```

Equivalent explicit commands:

```console
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo llvm-cov --all-features --workspace --fail-under-lines 90
```

## Render commands

```console
# file -> adjacent svg
cargo run -- tests/fixtures/basic/hello.puml

# explicit output path
cargo run -- tests/fixtures/basic/hello.puml -o out.svg

# stdin single-page -> SVG stdout
cat tests/fixtures/basic/hello.puml | cargo run -- -

# stdin multi-page/diagram -> JSON array
cat tests/fixtures/structure/multi_three.puml | cargo run -- --multi -
```

## Debug/dump commands

```console
cargo run -- --check tests/fixtures/basic/hello.puml
cargo run -- --dump ast tests/fixtures/basic/hello.puml
cargo run -- --dump model tests/fixtures/basic/hello.puml
cargo run -- --dump scene tests/fixtures/basic/hello.puml
```

## Include behavior

- File input: includes resolve relative to the input file directory.
- Stdin input: requires `--include-root DIR`.

```console
cat diagram.puml | cargo run -- --check --include-root ./tests/fixtures/include -
```
