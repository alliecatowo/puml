+++
title = "WASM roadmap"
description = "Where in-browser live rendering is headed."
weight = 30
+++

The [studio editor](@/editor.md) on this site uses a v1 "manifest lookup" renderer today: it matches the source you type against the 248 baked examples in `docs/examples/` and shows their committed SVG. That lets you experiment with syntax highlighting and the example corpus without any backend, but it doesn't let you author novel diagrams in-browser.

The next step is shipping a `puml-wasm` crate so the live renderer is the actual engine, running in a Web Worker.

## Target API

```typescript
type CompileOptions = {
  theme?: string;
  includeRoot?: string | null;
  page?: number | null;
  dialect?: "auto" | "plantuml" | "picouml" | "mermaid";
};

type Diagnostic = {
  code: string;
  severity: "error" | "warning" | "hint";
  message: string;
  span?: { start: number; end: number };
  line?: number; column?: number;
};

type CompileResult = {
  svg?: string;
  diagnostics: Diagnostic[];
  ast?: unknown;
  model?: unknown;
  scene?: unknown;
};

declare function compile(source: string, opts?: CompileOptions): CompileResult;
```

## Plan

1. **Feature-gate heavyweight deps.** `resvg`, `tiny-skia`, and `image` are only needed for PNG rasterization. Gate them behind a non-default `png` feature in `Cargo.toml` so the WASM target can drop them.
2. **New crate.** Add `crates/puml-wasm/` with `wasm-bindgen` exports of `compile()` and helpers.
3. **CI build.** Add a GitHub Actions job that runs `wasm-pack build --release crates/puml-wasm --target web`, uploads the artifact, and downloads it into the site build step.
4. **Wire into the studio.** Replace the manifest-lookup `RenderEngine` in `site/static/js/editor.js` with a `WasmEngine` that posts messages to a Web Worker loading the `puml_wasm.js` shim.

The editor already programs against a `RenderEngine` interface so the swap is local to one module.

## Why this approach

The full studio spec is in [`/developer/specs/studio-spa/`](@/developer/specs/studio-spa.md) and calls out hard constraints:

- No duplicate parser in TypeScript.
- No duplicate layout engine in TypeScript.
- All rendering work runs in a Web Worker.
- WASM-first, source-of-truth architecture.

Shipping a real WASM build is the right path for these constraints. Until the build job is in CI, we get the visual and editorial polish in place against the manifest engine, then swap the backend without touching the UI.

## Tracking

Open issues that block this on the repository as you do the work &mdash; "split renderer features", "add `crates/puml-wasm/`", "wasm-pack CI job", "studio engine swap". Each is a small, mergeable PR.
