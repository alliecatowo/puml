# FAQ

Frequently asked questions about `puml`.

---

## Is puml a drop-in replacement for PlantUML?

Not quite yet — and it's honest about that. Most common diagram families (sequence,
class, component, state, activity, Gantt, MindMap, JSON, YAML) render well. Advanced
PlantUML features — deep preprocessor macros, complex `skinparam` cascades, PDF output,
`ditaa`, `jlatexmath` — are partial or missing.

The fastest way to find out if `puml` works for your diagrams is to run:

```bash
find . -name '*.puml' | xargs puml --check
```

Exit code 0 on every file = you're likely good. Errors point to specific unsupported
constructs.

The honest per-feature status is tracked in
[`docs/internal/parity/plantuml_parity_source_of_truth.md`](internal/parity/plantuml_parity_source_of_truth.md).

---

## Why did you build this instead of just using PlantUML?

Two reasons:

1. **No Java.** PlantUML requires a JVM. Installing and managing a JDK adds friction
   to developer machines, Docker images, and CI pipelines. A single 10 MB static binary
   with zero runtime dependencies removes that entirely.

2. **Deterministic output.** PlantUML's output varies between JVM versions and can
   include timestamp metadata that makes diffs noisy. `puml` produces byte-identical
   output across platforms and runs — reliable for content-addressed caching, byte-level
   CI checks, and clean git diffs.

A third reason emerged during development: Rust's ownership model makes the renderer
easier to reason about for correctness, and the native speed makes tight CI feedback
loops feasible.

---

## Does puml work without internet access?

Yes, fully. `puml` is a single static binary. Rendering is entirely local: the
parser, normalizer, layout engine, and SVG emitter all run in-process with no
network calls.

The one exception is `!include` directives that reference remote URLs. Those require
explicitly opting in:

```bash
puml --allow-url-includes hello.puml
```

Without that flag, remote URL includes are rejected with a diagnostic. Local file
includes always work without any flag.

---

## Can I use puml in GitHub Actions?

Yes — see the full [CI integration guide](ci-integration.md). The short version:

```yaml
- uses: dtolnay/rust-toolchain@stable
- run: cargo install puml --bin puml
- run: find . -name '*.puml' -exec puml --check {} +
```

For faster CI, you can use the pre-built binary from GitHub Releases instead of
compiling from source:

```yaml
- name: Install puml
  run: |
    curl -Lo puml https://github.com/alliecatowo/puml/releases/latest/download/puml-linux-x86_64
    chmod +x puml
    mv puml /usr/local/bin/puml
```

---

## Is there a browser editor?

Yes. The renderer compiles to WebAssembly and runs entirely client-side. The live
editor is at:

[alliecatowo.github.io/puml/editor](https://alliecatowo.github.io/puml/editor)

No install, no server, no account. Paste PlantUML or PicoUML source into the left
pane; the diagram renders in the right pane as you type. You can download the output
as SVG or PNG directly from the browser.

The WASM module is also available as an npm package (`puml-wasm`) for embedding in
your own Node.js or browser applications.

---

## What output formats are supported?

| Format | Flag | Notes |
|---|---|---|
| SVG | `--format svg` (default) | Vector; embeds cleanly in HTML and Markdown |
| PNG | `--format png` | Raster; `--dpi 192` for high-DPI |
| JPG | `--format jpg` | Raster; lossy |
| WebP | `--format webp` | Raster; good compression |
| HTML | `--format html` | Self-contained; SVG embedded inline |
| ASCII | `--format txt` | Text art; useful for terminal output |
| Unicode | `--format utxt` | Unicode box-drawing characters |

```bash
puml hello.puml                          # SVG (default)
puml --format png --dpi 192 hello.puml   # high-DPI PNG
puml --format html hello.puml            # self-contained HTML
puml --format txt hello.puml             # ASCII art
```

---

## Will puml ever support PDF?

Not in the near term. The renderer emits SVG natively; PDF is a separate output
pipeline that would require embedding a PDF library.

In the meantime, SVG → PDF conversion is straightforward:

```bash
# Using rsvg-convert (from librsvg):
rsvg-convert -f pdf hello.svg -o hello.pdf

# Using Inkscape:
inkscape --export-type=pdf hello.svg

# Using Cairo:
cairosvg hello.svg -o hello.pdf
```

If PDF output is blocking you, open a
[GitHub issue](https://github.com/alliecatowo/puml/issues) — demand tracking helps
prioritization.

---

## What is PicoUML?

PicoUML is the project's own diagram dialect: a smaller, cleaner superset of PlantUML
that's designed to be easier to write, diff, validate, and repair.

Key differences from PlantUML syntax:
- Fewer sigils and keywords for common constructs
- Stricter parser that gives clearer error messages
- All PlantUML keywords continue to work (PicoUML is a superset, not a replacement)

Use PicoUML with `.picouml` file extensions or `@startpicouml` / `@endpicouml` block
markers. See the [PicoUML language spec](specs/picouml-language.md) for the full
reference.

---

## How do I install the VS Code extension?

The extension lives in `extensions/vscode/` in this repository. Marketplace publishing
is in progress.

To install from source:

```bash
cd extensions/vscode
npm install
npm run package       # produces a .vsix file
code --install-extension puml-*.vsix
```

The extension provides:
- Syntax highlighting for `.puml` and `.picouml` files
- Live diagram preview panel
- Diagnostics and error squiggles (via `puml-lsp`)
- Hover documentation and completions
- Code snippets for common diagram patterns

---

## How do I use puml with other editors?

`puml` ships `puml-lsp`, a full Language Server Protocol implementation. Any
LSP-capable editor can use it.

```bash
cargo install --git https://github.com/alliecatowo/puml --bin puml-lsp
```

Configure your editor to start `puml-lsp` for `.puml` and `.picouml` files.
See [install.md](install.md) for configuration examples for Neovim, Helix, and Zed.

---

## How do I report a rendering bug?

Open a [GitHub issue](https://github.com/alliecatowo/puml/issues) with:

1. The `.puml` source that doesn't render correctly (paste it inline, not as an attachment).
2. `puml --version` output.
3. A description of what you expected vs. what you got.
4. If you have PlantUML available, the PlantUML reference output is extremely helpful.

A minimal reproducer (fewest lines that still show the bug) gets issues fixed faster.
Complex diagrams are harder to debug — strip yours down before filing if you can.

---

## How do I check what version I have?

```bash
puml --version
```

---

## Can I use puml as a Rust library?

Yes. `puml` is both a CLI and a Rust library crate. Add it to your `Cargo.toml`:

```toml
[dependencies]
puml = { version = "0.1", default-features = false }
```

(Omit `default-features = false` if you want the CLI dependencies included.)

The primary public API is:

```rust
use puml::render;

let svg = render("@startuml\nAlice -> Bob: Hello\n@enduml\n")?;
println!("{}", svg);
```

The library API is evolving; pin to a specific version and check the changelog when
upgrading.

---

## Why is the output SVG different from PlantUML's?

`puml` is an independent renderer — it produces standards-compliant SVG from its own
layout engine rather than wrapping PlantUML. Visual output will be similar in structure
but not pixel-identical. Differences include:

- Font metrics (puml uses its own text measurement)
- Edge routing paths (puml's orthogonal router vs. Graphviz's spline engine)
- Default color schemes and spacing
- Arrow styles for some diagram families

If a visual difference looks like a bug (wrong topology, missing element, incorrect
label), please report it. If it's a stylistic difference (slightly different spacing,
different shade of a color), note it in the issue but understand it's lower priority
than correctness bugs.

---

## Is the project stable enough to use in production?

`puml` is at v0.1.0. The CLI interface and core diagram families are stable enough for
production use in documentation and CI pipelines. The Rust library API is evolving and
may have breaking changes between minor versions.

Recommended production posture:
- Pin to a specific version in `Cargo.toml` or use a pre-built binary from a tagged release.
- Run `puml --check` on your diagram corpus as part of the upgrade test.
- Subscribe to [GitHub Releases](https://github.com/alliecatowo/puml/releases) for changelog updates.

---

## Why Rust?

Three practical reasons:

1. **Single static binary.** Rust links everything — the renderer, layout engine, PNG
   encoder, SVG serializer — into one executable with no runtime or shared library
   dependencies. This is the key property that makes "no Java, no Node" possible.

2. **Correctness under load.** The ownership system catches data races and
   use-after-free at compile time. A renderer that handles arbitrary user input benefits
   from this.

3. **WASM as a first-class target.** Rust compiles cleanly to WebAssembly. The same
   renderer that runs in CI runs in the browser at near-native speed — no separate
   JavaScript reimplementation.

---

## Why is it called puml?

Short for PlantUML. The binary is `puml`; the crate is `puml`. PlantUML files end in
`.puml`. It fits.
