# puml vs PlantUML vs Mermaid

PlantUML, Mermaid, and `puml` solve overlapping problems with different tradeoffs.
This page is intentionally practical: choose the tool whose constraints match your
team rather than treating any one renderer as universally best.

For detailed PlantUML compatibility status, see the internal support matrix at
[`docs/internal/spec/plantuml-spec.md`](internal/spec/plantuml-spec.md) and the parity
source of truth at
[`docs/internal/parity/plantuml_parity_source_of_truth.md`](internal/parity/plantuml_parity_source_of_truth.md).

---

## Summary

| Need | Best fit |
|---|---|
| Full PlantUML language compatibility today | PlantUML |
| Diagrams rendered directly by GitHub or common wikis | Mermaid |
| Offline deterministic CLI rendering without Java or Node | puml |
| Existing `.puml` corpus with CI validation | PlantUML or puml, depending on feature coverage |
| Browser-first authoring with a simple syntax | Mermaid |
| Rust/WASM embedding using this renderer | puml |

---

## Feature matrix

| Feature | PlantUML | Mermaid | puml |
|---|---|---|---|
| Primary runtime | JVM | Browser / Node | Native Rust binary |
| Java required | Yes | No | No |
| Node required for CLI | No | Usually yes (`mmdc`) | No |
| Offline rendering | Yes after install | Depends on CLI/browser setup | Yes |
| SVG output | Yes | Yes | Yes |
| PNG output | Yes | Via CLI/headless browser | Yes |
| PDF output | Yes | Not a core CLI output | Yes |
| JPG / WebP output | No | Not a core CLI output | Yes |
| HTML output | No | Usually host-provided | Yes, self-contained |
| ASCII / Unicode text | Yes | No | Yes (`txt`, `atxt`, `utxt`) |
| PlantUML syntax | Reference implementation | No | Targeted compatibility, partial |
| Mermaid syntax | No | Reference implementation | Selected adapter support |
| PicoUML syntax | No | No | Yes |
| Preprocessor includes | Full PlantUML preprocessor | No | Local includes; URL includes opt in |
| Deterministic CLI artifacts | Not guaranteed byte-identical | Browser/runtime dependent | Design goal and CI invariant |
| Language server | Third-party ecosystem | Third-party ecosystem | `puml-lsp` in this repo |
| WASM/browser renderer | No | Yes | Yes, via `crates/puml-wasm` |
| License | GPL | MIT | MIT |

---

## Where PlantUML wins

Choose PlantUML when you need the reference implementation or the deepest language
coverage right now. It remains the safest choice for complex preprocessor macros,
large legacy corpora, complete `skinparam` behavior, LaTeX/math backends, `ditaa`, and
pixel expectations based on PlantUML itself.

Tradeoffs:

- You need Java, and some diagrams also depend on Graphviz or optional backends.
- Startup cost matters in CI when invoking the renderer many times.
- Output can vary with JVM, Graphviz, fonts, and platform details.

---

## Where Mermaid wins

Choose Mermaid when your host already renders Mermaid fences and the supported diagram
families are enough. It is especially convenient in GitHub Markdown, docs platforms,
and browser-first authoring flows where the diagram source stays inline.

Tradeoffs:

- It is a different language, not PlantUML-compatible syntax.
- CLI rendering typically brings a Node and browser/headless-browser dependency.
- Output can differ across browser/runtime versions.

---

## Where puml wins

Choose `puml` when you want a compiler-like CLI for diagrams:

- One native binary for rendering and validation.
- No Java, Node, browser, or network dependency for local file renders.
- Deterministic SVG-first artifacts that are friendly to code review and caching.
- Native outputs for SVG, PNG, JPG, WebP, PDF, HTML, and text.
- Built-in `--check`, JSON diagnostics, Markdown fence extraction, and LSP support.
- A Rust/WASM codebase that can power both local CI and a browser editor.

Tradeoffs:

- PlantUML parity is incomplete. Common families are useful, but edge cases and deep
  language features still need the parity matrix.
- Visual output is not pixel-identical to PlantUML because `puml` has its own layout
  and rendering engine.
- Some distribution channels are not yet stable user-facing install paths: no supported
  Homebrew tap, Docker image, npm CLI, or Marketplace extension is documented here.

---

## Migration from PlantUML to puml

Start with validation before changing artifacts:

```bash
find . -name '*.puml' -not -path './target/*' -exec puml --check {} +
```

Then render a small representative set and compare the SVG or PNG output visually:

```bash
puml docs/architecture.puml
puml --format png --dpi 192 docs/architecture.puml
```

Common migration checks:

| Area | What to verify |
|---|---|
| Preprocessor | Deep macros and conditionals may not match PlantUML yet. |
| Includes | Local includes are supported; URL includes require `--allow-url-includes`. |
| Themes / skinparam | Many common cases work; complex cascades should be checked. |
| Layout | Topology should match intent, but spacing and routes are renderer-specific. |
| PDF | `puml --format pdf` is available; verify output in your PDF viewer. |
| Markdown | `puml --from-markdown --check file.md` validates supported fenced blocks. |

If a diagram is critical and uses advanced PlantUML features, keep PlantUML in that
path until `puml --check` and visual review both pass.

---

## Migration from Mermaid to puml

`puml` is not a Mermaid replacement for every Mermaid feature. It has selected Mermaid
adapter support so teams can route simple Mermaid-style inputs into the same renderer,
but the native language target is PlantUML-compatible `.puml` plus PicoUML.

Choose a migration only if you need offline deterministic artifacts or want diagrams as
checked-in rendered files. If your current platform renders Mermaid directly and that is
enough, Mermaid may remain the simpler option.

---

## Further reading

- [Quickstart](quickstart.md)
- [Install guide](install.md)
- [FAQ](faq.md)
- [CI integration](ci-integration.md)
- [Examples gallery](examples/GALLERY.md)
