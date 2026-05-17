+++
title = "In-browser renderer"
description = "How the studio editor renders puml diagrams entirely in your browser."
weight = 30
+++

The [studio editor](@/editor.md) on this site runs the puml renderer entirely in your browser. There is no server, no remote service, no PlantUML JAR &mdash; the same Rust code that powers the `puml` CLI is compiled to WebAssembly and loaded by the page.

## How it works

The `crates/puml-wasm/` crate exposes the lib API (`render_source_to_svg`, `render_source_to_svgs`, `detect_diagram_family`) to JavaScript via `wasm-bindgen`. The Pages CI job builds it with `wasm-pack build --release --target web` and drops the artifact in `site/static/wasm/`. The editor dynamically imports `puml_wasm.js`, initializes the `.wasm` module, and calls into it on every keystroke (debounced 400 ms).

```typescript
// Simplified view of the JS surface the editor uses.
declare function render_svg(source: string): string;
declare function render_svgs(source: string): string[];
declare function render_svgs_json(source: string): string;
declare function detect_family(source: string): string;
```

`render_svgs_json` returns a JSON string of `{ ok: string[] } | { error: Diagnostic }`, which is convenient because diagnostics survive the WASM boundary as one round-trip.

## What's different from the CLI

- **No filesystem or URL fetching.** `!include`, `!include_many`, `!includesub`, `!import`, and `!include <stdlib/...>` all return deterministic include diagnostics in WASM. This differs from the native CLI, which enables URL includes by default for PlantUML compatibility. Everything else &mdash; parser, normaliser, layout, SVG render &mdash; is identical to the native build.
- **No PNG output.** The browser already has a DOM, so the page consumes the SVG directly. The `resvg` / `tiny-skia` / `image` deps that the CLI uses for PNG rasterization are feature-gated to the `cli` feature and never enter the WASM binary.
- **One single-page renderer per call.** Multi-page diagrams come back as an array of SVG strings; the studio currently concatenates them vertically.

## Binary size

The release-mode WASM file is around 1.3 MB. It's served once, browser-cached, and decodes quickly. If size becomes a problem, `wasm-opt -Oz` (run automatically by `wasm-pack`) is the first lever; trimming `winnow`'s monomorphizations is the second.

## Local development

```bash
# One-time
rustup target add wasm32-unknown-unknown
curl -fsSL https://rustwasm.github.io/wasm-pack/installer/init.sh | sh

# Rebuild after touching Rust code
wasm-pack build --release --target web --out-dir ../../site/static/wasm crates/puml-wasm

# Then run Zola
cd site && zola serve
```

The CI job in `.github/workflows/pages.yml` runs the same commands.
