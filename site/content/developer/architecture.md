+++
title = "Architecture"
description = "Modules, crates, and the boundaries the engine enforces."
weight = 10
+++

`puml` is organized around one rule: **source text is canonical**. Everything else &mdash; AST, model, scene, SVG &mdash; is a deterministic projection of the source.

## Crate layout

The repository is a single workspace crate today, with module-level seams that make a future split into sub-crates straightforward.

```text
src/
  ast.rs           parsed syntax tree
  parser.rs        winnow-based PlantUML / PicoUML parser
  normalize.rs     AST -> normalized model (dialect-independent)
  model.rs         canonical semantic model (Sequence/State/...)
  layout.rs        layout primitives shared by family renderers
  scene.rs         scene graph consumed by the SVG emitter
  render.rs        per-family deterministic SVG emitters
  creole.rs        PlantUML "creole" rich-text parser
  diagnostic.rs    error codes, severity, JSON schema
  source.rs        spans and source-region utilities
  theme.rs         token bag for skinparams / themes
  specialized.rs   non-UML families (json/yaml/regex/...)
  cli.rs           CLI argument plumbing
  main.rs          puml binary entry point
  bin/puml-lsp.rs  LSP server binary entry point
```

## Module boundaries

The boundaries are enforced by code review and tests:

- **Parser** never makes layout decisions. It returns a span-rich AST or a diagnostic.
- **Normalizer** turns dialect-specific shapes into a single canonical model. PlantUML, PicoUML, and Mermaid all flow through here.
- **Model** is the language-independent representation. Every family renderer reads from the model and never from the AST.
- **Layout** is pure geometry. It does not emit SVG.
- **Render** is pure SVG emission. It does not invent geometry.

## Determinism

The engine guarantees byte-identical SVG output for identical inputs. This is the single most important property of the project; many design choices follow from it:

- No hash-based iteration over unordered collections.
- No system clock, no environment lookups, no random IDs.
- Floating-point values rounded with a deterministic strategy at the layout/render boundary.
- Theme tokens are folded into the output, not left for downstream CSS.

## Diagnostics

Every error and warning carries a stable code (e.g. `E_PICOUML_MARKER_MIXED`). The full set is enumerated in `src/diagnostic.rs`. The JSON schema is documented in the [CLI reference](@/guide/cli.md) and used by editor integrations and the LSP.

## What's not in-process today

- A standalone `puml-syntax` crate for textmate / tree-sitter grammars &mdash; tracked in the [syntax highlighting spec](@/developer/specs/syntax-highlighting.md). The studio editor uses a CodeMirror `StreamLanguage` (in `site/static/js/puml-lang.js`) until that crate exists.

The rendered Markdown fence previews on this site are separate from syntax highlighting. They hydrate supported `puml`, `plantuml`, and `picouml` fences in the browser and call the real `puml-wasm` renderer, so preview correctness comes from the engine rather than the temporary CodeMirror highlighter.

The renderer itself is shipped end-to-end: native via the CLI and in-browser via WASM. See [In-browser renderer](@/developer/renderer.md) for the puml-wasm bridge.
