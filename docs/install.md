# Install puml

`puml` is a single static binary with no runtime dependencies. Pick the method that
fits your workflow.

---

## Quick install (cargo)

```bash
cargo install puml --bin puml
```

Requires Rust 1.88 or later. Run `rustup update stable` if needed.

After install, verify:

```bash
puml --version
```

---

## Install methods at a glance

| Method | When to use |
|---|---|
| `cargo install puml` | Rust toolchain already present; want the stable release |
| `cargo install --git …` | Want unreleased HEAD; actively tracking development |
| Build from source | Modifying the code; want a custom build |
| Pre-built binary (GitHub Releases) | No Rust installed; just want the CLI |
| npm / pnpm (`puml-wasm`) | Embedding the renderer in a Node.js or browser project |

---

## From crates.io (stable release)

```bash
cargo install puml --bin puml
```

This installs the `puml` CLI binary to `~/.cargo/bin/puml`. Add `~/.cargo/bin` to
your `$PATH` if it isn't already.

The release on crates.io tracks tagged versions. Check
[crates.io/crates/puml](https://crates.io/crates/puml) for the latest version number.

---

## Latest from GitHub (unreleased HEAD)

```bash
cargo install --git https://github.com/alliecatowo/puml --bin puml
```

This always builds from the current `main` branch. Use it to get fixes not yet in a
tagged release, or to track development. The downside is that unreleased code may
contain work-in-progress features.

To also install the language server at HEAD:

```bash
cargo install --git https://github.com/alliecatowo/puml --bin puml-lsp
```

---

## Build from source

Use this when you want to modify the code or pin to a specific commit.

```bash
git clone https://github.com/alliecatowo/puml.git
cd puml
cargo build --release
```

The binary lands at `target/release/puml`. Copy it to any directory on your `$PATH`:

```bash
cp target/release/puml ~/.local/bin/puml        # Linux/macOS, user-local
# or
sudo cp target/release/puml /usr/local/bin/puml  # system-wide
```

Build the language server at the same time:

```bash
cargo build --release --bin puml-lsp
cp target/release/puml-lsp ~/.local/bin/puml-lsp
```

---

## Pre-built binaries (GitHub Releases)

Pre-built binaries for Linux x86\_64 and macOS x86\_64 are attached to each tagged
release on the [GitHub Releases page](https://github.com/alliecatowo/puml/releases).

```bash
# Linux x86_64 — substitute the actual release tag
curl -Lo puml https://github.com/alliecatowo/puml/releases/latest/download/puml-linux-x86_64
chmod +x puml
mv puml ~/.local/bin/puml
```

```bash
# macOS x86_64
curl -Lo puml https://github.com/alliecatowo/puml/releases/latest/download/puml-macos-x86_64
chmod +x puml
mv puml ~/.local/bin/puml
```

Verify the download:

```bash
puml --version
```

> **Note:** macOS may quarantine the binary. Run
> `xattr -d com.apple.quarantine ~/.local/bin/puml` to clear the quarantine flag.

---

## npm / pnpm (WebAssembly package)

The renderer is also published as a WebAssembly package for Node.js and browser
projects. This is useful when you need to embed diagram rendering in a JavaScript
or TypeScript project without shelling out to a native binary.

```bash
npm install puml-wasm
# or
pnpm add puml-wasm
```

Usage in Node.js:

```js
import { render } from 'puml-wasm';

const svg = render(`
@startuml
Alice -> Bob: Hello
Bob --> Alice: Ack
@enduml
`);
console.log(svg);
```

See [`crates/puml-wasm/README.md`](../crates/puml-wasm/README.md) for the full API
and browser-bundle instructions.

---

## LSP (language server)

`puml-lsp` provides hover documentation, completions, diagnostics, and semantic tokens
for editors that speak the Language Server Protocol.

```bash
# From crates.io (when published):
cargo install puml --bin puml-lsp

# From GitHub HEAD:
cargo install --git https://github.com/alliecatowo/puml --bin puml-lsp
```

### Neovim (nvim-lspconfig)

```lua
require('lspconfig').puml_lsp.setup({
  cmd = { 'puml-lsp' },
  filetypes = { 'puml', 'picouml', 'plantuml' },
  root_dir = require('lspconfig.util').root_pattern('.git', '*.puml'),
})
```

### Helix (`languages.toml`)

```toml
[[language]]
name = "puml"
language-servers = ["puml-lsp"]

[language-server.puml-lsp]
command = "puml-lsp"
```

### Zed

Add to your Zed settings:

```json
{
  "lsp": {
    "puml-lsp": {
      "binary": {
        "path": "puml-lsp"
      }
    }
  }
}
```

---

## VS Code extension

A VS Code extension ships in this repo under `extensions/vscode/`. It wraps
`puml-lsp` and adds syntax highlighting, snippets, and a preview panel.

**Install from the repo:**

```bash
cd extensions/vscode
npm install
npm run package          # produces puml-*.vsix
code --install-extension puml-*.vsix
```

Marketplace publishing is in progress. Watch
[GitHub Releases](https://github.com/alliecatowo/puml/releases) for a marketplace
link.

---

## Docker

No official Docker image is published yet. You can build a minimal image from the
pre-built binary:

```dockerfile
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY puml-linux-x86_64 /usr/local/bin/puml
RUN chmod +x /usr/local/bin/puml
ENTRYPOINT ["puml"]
```

Or build from source in a multi-stage image:

```dockerfile
FROM rust:1.88-slim AS builder
WORKDIR /src
COPY . .
RUN cargo build --release --bin puml

FROM debian:bookworm-slim
COPY --from=builder /src/target/release/puml /usr/local/bin/puml
ENTRYPOINT ["puml"]
```

Build and run:

```bash
docker build -t puml .
docker run --rm -v "$PWD:/work" -w /work puml hello.puml
```

---

## Homebrew

A Homebrew tap is not yet published. Install via cargo or the pre-built binary in the
meantime. Open a [GitHub issue](https://github.com/alliecatowo/puml/issues) if a tap
would help you — community-maintained taps are welcome.

---

## Platform notes

| Platform | Status |
|---|---|
| Linux x86\_64 | Primary development and CI platform; fully tested |
| macOS x86\_64 | Expected to work; pre-built binary available; not in CI yet |
| macOS arm64 (Apple Silicon) | Compiles from source; not yet tested in CI |
| Windows | Not tested; contributions and issue reports welcome |
| WebAssembly (browser / Node) | Supported via `crates/puml-wasm` |

---

## Uninstall

```bash
# If installed via cargo:
cargo uninstall puml

# If installed manually:
rm ~/.local/bin/puml ~/.local/bin/puml-lsp
```

---

## Next steps

- [Quickstart: your first diagram in 5 minutes](quickstart.md)
- [CI integration](ci-integration.md)
- [CLI reference](https://alliecatowo.github.io/puml/guide/cli/)
