# puml vs PlantUML vs Mermaid

An objective look at where each tool wins, where it doesn't, and how to choose.

---

## Feature matrix

| Feature | PlantUML | Mermaid | puml |
|---|---|---|---|
| **Runtime dependency** | JVM (Java 8+) | Node.js + browser | None — single static binary |
| **Install size** | ~10 MB JAR + JDK (~200 MB) | ~100 MB Node + packages | ~10 MB binary |
| **Offline rendering** | Yes (Java must be installed) | Requires browser or Node runtime | Yes, always |
| **Output: SVG** | Yes | Yes (browser-rendered) | Yes |
| **Output: PNG** | Yes | Via plugin / headless browser | Yes (native, no browser) |
| **Output: PDF** | Yes | No | No (SVG → PDF via external tool) |
| **Output: HTML** | No | No | Yes (self-contained) |
| **Output: JPG, WebP** | No | No | Yes |
| **Output: ASCII / Unicode text** | Yes | No | Yes (`--format txt`, `--format utxt`) |
| **Deterministic output** | Varies across JVM versions | Varies across browser/Node versions | Deterministic across platforms |
| **CLI** | Yes (`java -jar plantuml.jar`) | Limited (`mmdc` requires Node) | Yes — designed as a compiler tool |
| **LSP / editor** | Third-party plugins | Third-party plugins | Built-in `puml-lsp` |
| **WASM / browser embed** | No | Yes | Yes (`crates/puml-wasm`) |
| **Diagram families** | ~30, deep coverage | ~15 | ~25, coverage varies |
| **PlantUML syntax** | Reference implementation | Separate language | Target, not yet 100% |
| **Mermaid syntax** | No | Reference implementation | Adapter for selected families |
| **PicoUML dialect** | No | No | Yes (ergonomic superset) |
| **Layout engine** | Graphviz (external) | Dagre | Built-in (hierarchical + orthogonal) |
| **`!include` support** | Yes (full preprocessor) | No | Yes (file + URL with opt-in) |
| **Semantic tokens** | Via extensions | No | Built-in |
| **License** | GPL | MIT | MIT |
| **Language** | Java | JavaScript / TypeScript | Rust |
| **Binary size** | N/A (JAR + JRE) | N/A (Node ecosystem) | ~10 MB |

---

## Install size comparison

| Tool | What you install | Approximate disk |
|---|---|---|
| PlantUML | plantuml.jar + Java runtime | 200–500 MB (JDK) |
| Mermaid CLI (`mmdc`) | Node.js + npm packages + Chromium | 300–600 MB |
| puml | Single static binary | ~10 MB |

For CI environments, install time matters. `cargo install puml` from crates.io takes
under 2 minutes on a cached Rust toolchain. The pre-built binary download takes under
5 seconds.

---

## Diagram family coverage

### PlantUML (~30 families, reference implementation)

PlantUML is the most feature-complete option. It covers every diagram type it has ever
added, with years of refinement. This includes advanced families like:

- Sequence, class, object, use case, component, deployment, state, activity
- Timing, Gantt, chronology, MindMap, WBS
- Salt wireframes, JSON, YAML, nwdiag, Archimate
- EBNF, regex, math (LaTeX), SDL, ditaa, C4-style
- Network diagrams, entity-relationship

### Mermaid (~15 families)

Mermaid covers the most commonly used families well:

- Flowchart, sequence, class, state, ER, Gantt
- Pie charts, Git graph, user journey, quadrant charts
- C4, block diagrams, packet diagrams

The language syntax is simpler and more approachable than PlantUML. The tradeoff is
less expressive power for complex diagrams and no offline-first story.

### puml (~25 families, coverage varies)

puml targets the same set as PlantUML. As of v0.1.0, baseline rendering across all
major families has landed. Depth of coverage varies: sequence, class, component, and
state diagrams are the most polished; some advanced features (complex skinparam
overrides, deep preprocessor macros, PDF) are partial or missing.

See the [parity tracking document](internal/parity/plantuml_parity_source_of_truth.md)
for the current per-feature status.

---

## Layout engine

| | PlantUML | Mermaid | puml |
|---|---|---|---|
| Engine | Graphviz (external `dot`) | Dagre (JS port of Graphviz) | Built-in hierarchical layout |
| External dependency | Yes (`dot` must be on PATH) | No (bundled) | No (compiled in) |
| Orthogonal edge routing | Via Graphviz | Partial | Built-in (Wave-21+) |
| Customizable layout | Via `left to right direction` etc. | Limited | Growing |

PlantUML's layout quality is excellent because Graphviz is a mature, proven engine.
The downside is the external dependency and the overhead of JVM + Graphviz interop.

Mermaid uses Dagre, a JS port of Graphviz's algorithms, which gives reasonable results
in the browser without any external process.

puml's layout engine is built in Rust, eliminating all external dependencies. Stage 1
(hierarchical layout) and orthogonal edge routing are complete. Advanced layout options
(force-directed, custom rank strategies) are on the roadmap.

---

## Performance

Informal benchmarks on a single sequence diagram (~30 elements):

| Tool | Cold start | Render time |
|---|---|---|
| PlantUML | ~2–3 s (JVM startup) | ~100–300 ms |
| Mermaid CLI | ~3–5 s (Chromium startup) | ~200–500 ms |
| puml | ~10 ms | ~5–50 ms |

For a single diagram, the difference is noticeable but not usually blocking. For CI
pipelines rendering dozens or hundreds of diagrams, the JVM/Chromium startup cost
per invocation adds up quickly. `puml` amortizes nothing because there's nothing to
start — each invocation is just process startup + parse + render.

---

## Determinism

PlantUML's output varies between JVM versions and sometimes between runs (timestamp
metadata, font metrics that depend on the JVM's font stack). This makes byte-for-byte
comparison in CI unreliable.

Mermaid's output varies between browser versions and across operating systems because
rendering depends on the browser's SVG engine and layout calculations.

`puml` is designed for deterministic output: same input → byte-identical output across
platforms and runs. This is a hard invariant enforced by CI. It makes diff-based review
and content-addressed caching reliable.

---

## Where each tool wins

### Choose PlantUML when:

- You need complete PlantUML feature parity today (complex preprocessor macros, full
  skinparam surface, LaTeX math rendering, PDF output).
- Your team already uses Java and JVM tooling is standard.
- You need pixel-identical compatibility with existing PlantUML tooling.
- You need the `ditaa` or `jlatexmath` backends.

### Choose Mermaid when:

- You want diagrams that render natively in GitHub Markdown, Notion, Confluence, or
  other wikis that support Mermaid inline.
- Your diagrams are simple and the browser-native rendering is good enough.
- You don't need offline rendering or a CLI workflow.
- You want a simpler language syntax that non-engineers can learn quickly.

### Choose puml when:

- You have PlantUML diagrams and want to stop installing Java.
- You need deterministic, diff-friendly SVG output in git.
- You want to render diagrams in CI without a JVM or Chromium.
- You want a built-in LSP without third-party extensions.
- You're writing new diagrams and want offline-first + editor integration.
- You want to embed diagram rendering in a Rust or WASM project.

---

## The honest tradeoffs of puml

- **Parity is a goal, not a guarantee.** Some advanced PlantUML features are partial.
  Run `puml --check` against your diagrams to see what works.
- **v0.1.0 is young.** The renderer is solid for common cases; edge cases may need
  workarounds or fixes. File issues — they get fixed fast.
- **No PDF.** If you need PDF output today, use `rsvg-convert` or Inkscape to convert
  SVG → PDF.
- **Homebrew and platform packages** aren't yet available. Install via cargo or the
  pre-built binary.

---

## Migration guide: PlantUML → puml

Most PlantUML diagrams work as-is. Common friction points:

1. **Run `puml --check` on your corpus** — get a clear view of what's supported.
2. **skinparam overrides** — many work; complex themes may need adjustment.
3. **`!include` with remote URLs** — requires `--allow-url-includes` flag.
4. **PDF output** — no native PDF; convert SVG with `rsvg-convert` or Inkscape.
5. **`!pragma layout` directives** — some are honored; Graphviz-specific ones are
   ignored.

See [troubleshooting.md](troubleshooting.md) for specific error messages and fixes.

---

## Further reading

- [Quickstart](quickstart.md) — get your first diagram rendered in 5 minutes
- [Install guide](install.md) — all install methods
- [FAQ](faq.md) — common questions about compatibility and workflow
- [Parity tracking](internal/parity/plantuml_parity_source_of_truth.md) — per-feature
  status against PlantUML
