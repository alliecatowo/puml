# Install puml

`puml` is a Rust-native PlantUML-compatible renderer. The CLI is the main user entry
point: it reads `.puml`, `.plantuml`, `.picouml`, Markdown fences, or stdin, then emits
SVG by default with optional PNG, JPG, WebP, PDF, HTML, and text outputs.

The current package version is `0.1.0` and the crate requires Rust `1.88` or newer when
you build from source.

---

## Recommended install

### No Rust? Use the curl installer (Linux and macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/alliecatowo/puml/main/scripts/install.sh | sh
```

The installer auto-detects your platform, verifies the SHA-256 checksum, optionally
verifies the cosign signature, and installs `puml` without requiring Rust or any other
build toolchain.

#### Installer options

| Flag | Description |
|---|---|
| `--version <tag>` | Install a specific release (e.g. `v0.2.1`) instead of latest |
| `--prefix <dir>` | Install to `<dir>/bin` instead of auto-detected prefix |
| `--dry-run` | Print what would happen without downloading or installing |
| `--no-verify-sig` | Skip cosign signature check (SHA-256 is always verified) |
| `-h`, `--help` | Print usage and exit |

Examples:

```bash
# Install a specific version
curl -fsSL https://raw.githubusercontent.com/alliecatowo/puml/main/scripts/install.sh \
  | sh -s -- --version v0.2.1

# Install to ~/.local (useful when /usr/local is not writable)
curl -fsSL https://raw.githubusercontent.com/alliecatowo/puml/main/scripts/install.sh \
  | sh -s -- --prefix ~/.local

# Preview without installing
curl -fsSL https://raw.githubusercontent.com/alliecatowo/puml/main/scripts/install.sh \
  | sh -s -- --dry-run
```

#### Trust model

1. **SHA-256 is always verified.** The checksum is fetched from the release's `SHA256SUMS`
   file over HTTPS and compared to the locally computed hash. The install aborts on mismatch.
2. **cosign keyless signature is verified when cosign is available.** Install cosign with
   `brew install cosign` or see [docs.sigstore.dev](https://docs.sigstore.dev/cosign/installation/).
   Pass `--no-verify-sig` to skip (not recommended).
3. **The binary is not executed during install.** Only `puml --version` runs after the
   binary is in place as a smoke-test.

#### Prefix selection

- If `/usr/local/bin` is writable (e.g. macOS with admin rights), the installer uses
  `/usr/local/bin` — no `sudo` prompt.
- Otherwise it falls back to `~/.local/bin`, which is XDG-standard and never requires
  elevated privileges.
- Use `--prefix` to override entirely.

#### Manual verification (without the installer)

```bash
# 1. Download the archive and SHA256SUMS for your platform
VERSION="$(curl -fsSL https://api.github.com/repos/alliecatowo/puml/releases/latest \
  | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')"
BASE="https://github.com/alliecatowo/puml/releases/download/${VERSION}"

# Linux x86-64 example:
curl -LO "${BASE}/puml-x86_64-unknown-linux-musl.tar.gz"
curl -LO "${BASE}/SHA256SUMS"

# 2. Verify checksum
grep "puml-x86_64-unknown-linux-musl.tar.gz" SHA256SUMS | sha256sum --check

# 3. (Optional) Verify cosign signature
curl -LO "${BASE}/puml-x86_64-unknown-linux-musl.tar.gz.cosign.bundle"
cosign verify-blob \
  --bundle puml-x86_64-unknown-linux-musl.tar.gz.cosign.bundle \
  --certificate-identity-regexp "https://github.com/alliecatowo/puml/" \
  --certificate-oidc-issuer "https://token.actions.githubusercontent.com" \
  puml-x86_64-unknown-linux-musl.tar.gz

# 4. Extract and install
tar -xzf puml-x86_64-unknown-linux-musl.tar.gz
mkdir -p ~/.local/bin
mv puml ~/.local/bin/puml
chmod +x ~/.local/bin/puml
puml --version
```

### Have Rust? Use Cargo

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
| curl installer | No Rust, Linux or macOS | `curl -fsSL .../install.sh \| sh` |
| Cargo release | You want the published CLI and already have Rust | `cargo install puml --bin puml` |
| GitHub HEAD | You need an unreleased fix from `main` | `cargo install --git https://github.com/alliecatowo/puml --bin puml` |
| Source checkout | You are developing or auditing the project | `cargo build --release --bin puml` |
| GitHub release asset | You do not want to compile and prefer manual steps | Download from GitHub Releases |
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
Each release includes archives, a `SHA256SUMS` file, and `.cosign.bundle` signature
files for every archive.

| Platform | CLI archive | LSP archive |
|---|---|---|
| Linux x86-64 | `puml-x86_64-unknown-linux-musl.tar.gz` | `puml-lsp-x86_64-unknown-linux-musl.tar.gz` |
| Linux arm64 | `puml-aarch64-unknown-linux-musl.tar.gz` | `puml-lsp-aarch64-unknown-linux-musl.tar.gz` |
| macOS Apple Silicon | `puml-aarch64-apple-darwin.tar.gz` | `puml-lsp-aarch64-apple-darwin.tar.gz` |
| macOS Intel | `puml-x86_64-apple-darwin.tar.gz` | `puml-lsp-x86_64-apple-darwin.tar.gz` |
| Windows x86-64 | `puml-x86_64-pc-windows-msvc.zip` | `puml-lsp-x86_64-pc-windows-msvc.zip` |

Linux example (manual):

```bash
mkdir -p ~/.local/bin
curl -L -o /tmp/puml.tar.gz \
  https://github.com/alliecatowo/puml/releases/latest/download/puml-x86_64-unknown-linux-musl.tar.gz
curl -L -o /tmp/SHA256SUMS \
  https://github.com/alliecatowo/puml/releases/latest/download/SHA256SUMS
grep "puml-x86_64-unknown-linux-musl.tar.gz" /tmp/SHA256SUMS | sha256sum --check
tar -xzf /tmp/puml.tar.gz -C /tmp
cp /tmp/puml ~/.local/bin/puml
puml --version
```

macOS Apple Silicon example (manual):

```bash
mkdir -p ~/.local/bin
curl -L -o /tmp/puml.tar.gz \
  https://github.com/alliecatowo/puml/releases/latest/download/puml-aarch64-apple-darwin.tar.gz
curl -L -o /tmp/SHA256SUMS \
  https://github.com/alliecatowo/puml/releases/latest/download/SHA256SUMS
grep "puml-aarch64-apple-darwin.tar.gz" /tmp/SHA256SUMS | shasum -a 256 --check
tar -xzf /tmp/puml.tar.gz -C /tmp
cp /tmp/puml ~/.local/bin/puml
# If macOS Gatekeeper blocks the binary:
xattr -d com.apple.quarantine ~/.local/bin/puml
puml --version
```

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

Or download the LSP archive from the GitHub Releases page alongside the CLI archive.

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
