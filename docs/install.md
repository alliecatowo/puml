# Install puml

`puml` is a Rust-native PlantUML-compatible renderer. The CLI is the main user entry
point: it reads `.puml`, `.plantuml`, `.picouml`, Markdown fences, or stdin, then emits
SVG by default with optional PNG, JPG, WebP, PDF, HTML, and text outputs.

The current package version is `0.1.0` and the crate requires Rust `1.88` or newer when
you build from source.

---

## Recommended install

If you already have Rust installed, use Cargo:

```bash
cargo install puml --bin puml
puml --version
```

This installs the `puml` binary into Cargo's bin directory, usually
`~/.cargo/bin`. Make sure that directory is on your `PATH`.

To update later:

```bash
cargo install puml --bin puml --force
```

---

## Install methods

| Method | Use it when | Command or source |
|---|---|---|
| Cargo release | You want the published CLI and already have Rust | `cargo install puml --bin puml` |
| GitHub HEAD | You need an unreleased fix from `main` | `cargo install --git https://github.com/alliecatowo/puml --bin puml` |
| Source checkout | You are developing or auditing the project | `cargo build --release --bin puml` |
| GitHub release asset | You do not want to compile | Download from GitHub Releases |
| WASM crate | You are embedding rendering in the browser/site build | Build `crates/puml-wasm` from this repo |

There is not currently a supported Homebrew tap, Docker image, npm CLI package, or VS
Code Marketplace package documented as a stable install path in this repository.

---

## Cargo release

```bash
cargo install puml --bin puml
```

Verify the install:

```bash
puml --version
puml --help
```

If Cargo reports that your compiler is too old, update Rust first:

```bash
rustup update stable
```

---

## Latest from GitHub

Use this when you need the latest `main` branch rather than the latest published crate:

```bash
cargo install --git https://github.com/alliecatowo/puml --bin puml
```

You can pin an exact commit for reproducible automation:

```bash
cargo install --git https://github.com/alliecatowo/puml --rev <commit-sha> --bin puml
```

Unreleased HEAD can contain work in progress. For team-wide CI, prefer a tagged release
or a pinned commit.

---

## Build from source

```bash
git clone https://github.com/alliecatowo/puml.git
cd puml
cargo build --release --bin puml
./target/release/puml --version
```

Install the built binary somewhere on your `PATH`:

```bash
mkdir -p ~/.local/bin
cp target/release/puml ~/.local/bin/puml
```

For a system-wide install on Linux or macOS:

```bash
sudo cp target/release/puml /usr/local/bin/puml
```

The language server binary is also in this repository:

```bash
cargo build --release --bin puml-lsp
cp target/release/puml-lsp ~/.local/bin/puml-lsp
```

---

## GitHub release assets

Tagged releases are published at
[github.com/alliecatowo/puml/releases](https://github.com/alliecatowo/puml/releases).
The current release workflow builds these CLI assets:

| Platform | Asset name |
|---|---|
| Linux x86_64 | `puml-linux-x86_64` |
| macOS x86_64 | `puml-macos-x86_64` |

Linux example:

```bash
mkdir -p ~/.local/bin
curl -L -o ~/.local/bin/puml \
  https://github.com/alliecatowo/puml/releases/latest/download/puml-linux-x86_64
chmod +x ~/.local/bin/puml
puml --version
```

macOS Intel example:

```bash
mkdir -p ~/.local/bin
curl -L -o ~/.local/bin/puml \
  https://github.com/alliecatowo/puml/releases/latest/download/puml-macos-x86_64
chmod +x ~/.local/bin/puml
puml --version
```

If macOS blocks a downloaded binary, clear the quarantine attribute for the path you
installed:

```bash
xattr -d com.apple.quarantine ~/.local/bin/puml
```

Apple Silicon users can build with Cargo today. Native Apple Silicon release assets are
not listed in the current release workflow.

---

## Browser and WebAssembly builds

The in-repo site uses the `crates/puml-wasm` package and builds it with `wasm-pack`.
This is the supported path for the browser editor and site integration in this repo:

```bash
wasm-pack build --release --target web --out-dir ../../site/static/wasm crates/puml-wasm
```

If you need a JavaScript package boundary, treat the WASM crate as a repo-local build
artifact unless a published package is announced in a tagged release.

---

## Editor integration

`puml-lsp` provides diagnostics, hover, completion, and semantic tokens for editors that
speak the Language Server Protocol.

Install from source or GitHub HEAD:

```bash
cargo install --git https://github.com/alliecatowo/puml --bin puml-lsp
```

### Neovim

```lua
require('lspconfig').puml_lsp.setup({
  cmd = { 'puml-lsp' },
  filetypes = { 'puml', 'plantuml', 'picouml' },
})
```

### Helix

```toml
[[language]]
name = "puml"
language-servers = ["puml-lsp"]

[language-server.puml-lsp]
command = "puml-lsp"
```

### VS Code extension from source

A VS Code extension lives under `extensions/vscode/`:

```bash
cd extensions/vscode
npm install
npm run package
code --install-extension puml-*.vsix
```

Marketplace publishing is not documented as a stable install channel yet.

---

## First render after install

```bash
cat > hello.puml <<'EOF_DIAGRAM'
@startuml
Alice -> Bob: Hello
Bob --> Alice: Ack
@enduml
EOF_DIAGRAM

puml hello.puml
```

This writes `hello.svg`. Continue with the [quickstart](quickstart.md) for validation,
Markdown, stdin/stdout, and CI-friendly workflows.
