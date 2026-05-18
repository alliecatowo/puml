# puml vs PlantUML vs Mermaid

## Feature comparison

| Feature | PlantUML | Mermaid | puml |
|---|---|---|---|
| Runtime | JVM (Java 8+) | Node.js + browser | Single Rust binary, no runtime |
| Offline rendering | Yes (with Java installed) | Requires browser/Node | Yes, always |
| Output formats | SVG, PNG, PDF, ASCII, LaTeX | SVG (browser), PNG (via plugin) | SVG, PNG, JPG, WebP, HTML, ASCII |
| Deterministic output | Varies across JVM versions | Varies across browser/Node versions | Deterministic across platforms |
| CLI | Yes | Limited (`mmdc` via Node) | Yes — designed as a compiler tool |
| LSP / editor support | Third-party plugins | Third-party plugins | Built-in `puml-lsp` |
| WASM / browser editor | No | Yes | Yes (`crates/puml-wasm`) |
| Diagram families | ~30 families, deep coverage | ~15 families | ~25 families, coverage varies |
| PlantUML parity | Reference implementation | Separate language | Target, not 100% yet |
| Mermaid support | No | Reference implementation | Adapter for selected families |
| License | GPL | MIT | MIT |

## Where each tool wins

### PlantUML

PlantUML is the most feature-complete PlantUML renderer. If you need the full official surface, advanced preprocessor macros, PDF output, or pixel-identical output against the reference, PlantUML is the right choice. The tradeoff is the JVM dependency, which adds friction to CI environments and developer machines without Java.

### Mermaid

Mermaid excels in browser-native contexts — GitHub Markdown previews, Notion, Confluence, and wikis that render it inline. Its diagram language is simpler and more approachable. The tradeoff is that it needs a browser or Node runtime to render, making offline and CI workflows less convenient, and it doesn't interoperate with PlantUML source.

### puml

puml targets the intersection of "I have PlantUML diagrams" and "I don't want to install Java." One native binary, offline by default, deterministic SVG output that diffs cleanly in git, and a built-in language server for editors. The tradeoff is that parity with PlantUML is a goal in progress, not a guarantee — some advanced features are partial. Check [compatibility status](internal/parity/plantuml_parity_source_of_truth.md) for details.

## Choosing

- **Already using PlantUML and happy with Java** → stay on PlantUML.
- **Need browser-inline diagrams in GitHub/Notion** → Mermaid.
- **Want diagrams in CI/CD, editors, and offline workflows without Java** → try puml.
- **Writing new diagrams from scratch** → puml's PicoUML dialect or PlantUML syntax both work.
