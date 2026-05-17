+++
title = "Developer guide"
description = "Architecture, pipeline, and reference specs for contributors."
sort_by = "weight"
template = "section.html"
page_template = "page.html"
+++

This section is for people who want to hack on `puml` itself, ship a new diagram family, build a tool on top of the engine, or understand exactly what the compiler does.

## Start here

- [Architecture](@/developer/architecture.md) &mdash; the five-stage pipeline and the crates around it.
- [Compile pipeline](@/developer/pipeline.md) &mdash; how a `.puml` source becomes an SVG.
- [In-browser renderer](@/developer/renderer.md) &mdash; how the studio editor renders puml diagrams via the puml-wasm crate.
- [Contributing](@/developer/contributing.md) &mdash; build, test, lint, release gates.

## Reference specs

The repo's `docs/specs/` directory holds the source-of-truth specifications. They're mirrored under [`/developer/specs/`](@/developer/specs/_index.md) for browsing here:

- [PicoUML language baseline](@/developer/specs/picouml-language.md)
- [Diagram families architecture](@/developer/specs/diagram-families-architecture.md)
- [Studio SPA specification](@/developer/specs/studio-spa.md)
- [Syntax highlighting spec](@/developer/specs/syntax-highlighting.md)
- [LSP spec](@/developer/specs/lsp.md)
- [VS Code extension spec](@/developer/specs/vscode-extension.md)
- [Markdown fence renderer spec](@/developer/specs/markdown-fence-renderer.md)
- [Agent / MCP plugin spec](@/developer/specs/agent-plugin-mcp.md)
