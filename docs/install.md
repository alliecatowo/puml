# Install puml

## From crates.io

```bash
cargo install puml --bin puml
```

This installs the `puml` CLI binary. Requires Rust 1.88+.

## Latest from GitHub

```bash
cargo install --git https://github.com/alliecatowo/puml --bin puml
```

Use this to get the newest unreleased changes.

## Build from source

```bash
git clone https://github.com/alliecatowo/puml.git
cd puml
cargo build --release
```

The binary lands at `target/release/puml`. Copy it anywhere on your `$PATH`.

## Pre-built binaries

Pre-built binaries are coming soon. Watch [GitHub Releases](https://github.com/alliecatowo/puml/releases) for downloads.

## LSP (language server)

```bash
cargo install --git https://github.com/alliecatowo/puml --bin puml-lsp
```

Point your editor's LSP config at `puml-lsp` for `.puml` and `.picouml` files.

## VS Code extension

The extension is in this repo under `extensions/vscode/`. Marketplace publishing is coming soon.

## Platform notes

- **Linux x86_64**: primary development and CI platform; fully tested.
- **macOS**: expected to work; not yet in CI.
- **Windows**: not tested; contributions welcome.
