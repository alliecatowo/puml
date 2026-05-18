# puml

> PlantUML diagrams. No Java. Native speed.

[![main gate](https://github.com/alliecatowo/puml/actions/workflows/main-gate.yml/badge.svg)](https://github.com/alliecatowo/puml/actions/workflows/main-gate.yml)
[![PR gate](https://github.com/alliecatowo/puml/actions/workflows/pr-gate.yml/badge.svg)](https://github.com/alliecatowo/puml/actions/workflows/pr-gate.yml)
[![docs site](https://github.com/alliecatowo/puml/actions/workflows/pages.yml/badge.svg)](https://github.com/alliecatowo/puml/actions/workflows/pages.yml)
[![Rust 2021](https://img.shields.io/badge/rust-2021-f97316)](Cargo.toml)
[![version](https://img.shields.io/badge/version-0.1.0-0ea5e9)](Cargo.toml)
[![license: MIT](https://img.shields.io/badge/license-MIT-22c55e)](LICENSE)
[![docs](https://img.shields.io/badge/docs-alliecatowo.github.io%2Fpuml-16a34a)](https://alliecatowo.github.io/puml/)

<p>
  <a href="docs/examples/groups_notes.puml"><img src="docs/examples/groups_notes.svg" alt="Sequence diagram with groups and notes" width="48%"></a>
  <a href="docs/examples/component/06_with_arrows.puml"><img src="docs/examples/component/06_with_arrows.svg" alt="Component diagram with arrows" width="48%"></a>
</p>
<p>
  <a href="docs/examples/gantt/05_multi_task.puml"><img src="docs/examples/gantt/05_multi_task.svg" alt="Gantt chart" width="48%"></a>
  <a href="docs/examples/json/03_nested.puml"><img src="docs/examples/json/03_nested.svg" alt="Nested JSON projection" width="48%"></a>
</p>

<details open>
<summary><b>📦 Install &amp; render in 60 seconds</b></summary>

```bash
cargo install puml --bin puml

cat > hello.puml <<'EOF'
@startuml
Alice -> Bob: Hello
Bob --> Alice: Ack
@enduml
EOF

puml hello.puml         # writes hello.svg
puml --check hello.puml # lint without writing
```

Need a different install path? See the [install guide](docs/install.md).

</details>

<details>
<summary><b>🤔 Why puml — and why not just PlantUML or Mermaid?</b></summary>

| | PlantUML | Mermaid | puml |
|---|---|---|---|
| Runtime | JVM required | Node + browser | Single static Rust binary |
| Offline | Yes (with Java) | No (needs browser) | Yes, always |
| Output | SVG, PNG, PDF | SVG (browser-rendered) | SVG, PNG, JPG, WebP, HTML |
| Determinism | Varies by JVM version | Varies by browser | Deterministic across platforms |
| CLI | Yes | Limited | Yes — designed as a compiler tool |
| LSP / editor | Third-party | Third-party | Built-in (`puml-lsp`) |
| WASM | No | Yes | Yes (`crates/puml-wasm`) |

**PlantUML** is the gold standard for feature breadth. Use it if you need full parity today and can accept the JVM dependency.

**Mermaid** is great for quick diagrams in GitHub Markdown and wikis. It needs a browser runtime to render and doesn't produce diff-friendly offline artifacts.

**puml** is for teams that want diagrams in source control, reviewed as text, rendered offline, and integrated into CI and editors without installing Java or Node.

</details>

<details>
<summary><b>🎨 What diagrams can I make?</b></summary>

Around 25 diagram families: sequence, class, object, use case, component, deployment, state, activity, timing, Gantt, chronology, MindMap, WBS, Salt wireframes, JSON, YAML, nwdiag, Archimate, regex, EBNF, math, SDL, ditaa, chart, and C4-style.

PicoUML is the project's own ergonomic dialect — a smaller, cleaner superset of PlantUML that's easier to write, diff, validate, and repair. Mermaid sequence and flowchart inputs are also accepted as an adapter path into the same renderer.

Browse the [examples gallery](docs/examples/GALLERY.md) to see what's rendered today.

</details>

<details>
<summary><b>🛠️ CLI · LSP · WASM · VS Code</b></summary>

### CLI

```bash
# Render
puml hello.puml                          # → hello.svg
puml --format png --dpi 192 hello.puml   # → hello.png at 2x
puml --format html hello.puml            # → hello.html (self-contained)

# Lint
puml --check hello.puml                  # exit 0 = valid
puml --from-markdown --check notes.md    # lint all fenced puml blocks

# Pipeline inspection
puml --dump ast hello.puml
puml --dump model hello.puml
puml --dump scene hello.puml
```

See the [CLI reference](https://alliecatowo.github.io/puml/guide/cli/) for all flags, dialects, and exit codes.

### LSP

`puml-lsp` ships in this repo. It provides diagnostics, hover, completions, and semantic tokens for editors that speak Language Server Protocol.

```bash
cargo install --git https://github.com/alliecatowo/puml --bin puml-lsp
```

Point your editor's LSP config at `puml-lsp` for `.puml` and `.picouml` files.

### WASM

The renderer compiles to WebAssembly via `crates/puml-wasm`. The live browser editor at [alliecatowo.github.io/puml/editor](https://alliecatowo.github.io/puml/editor) runs it client-side with no server.

### VS Code

A VS Code extension is in this repo under `extensions/vscode/`. *(Screenshot pending — follow-up tracked separately.)*

</details>

<details>
<summary><b>📊 PlantUML compatibility status</b></summary>

`puml` is PlantUML-compatible, not a claim of complete 1:1 parity. Many diagram families render well; some advanced features are partial and tracked openly. Run `puml --check` and compare output when visual parity matters. The honest conservative status per feature row lives in [`docs/internal/parity/plantuml_parity_source_of_truth.md`](docs/internal/parity/plantuml_parity_source_of_truth.md).

</details>

<details>
<summary><b>🚀 Project status &amp; roadmap</b></summary>

`puml` is at v0.1.0 — young, ambitious, and developed with significant AI assistance. Baseline rendering across all major diagram families landed in the parity blitz (May 2025); advanced feature depth is an ongoing effort. See the [GitHub milestone view](https://github.com/alliecatowo/puml/milestones) for what's planned next.

</details>

<details>
<summary><b>How PUML works</b></summary>

![Architecture overview](docs/diagrams/architecture-overview.svg)

PUML is structured as a three-layer pipeline. **Frontends** (PlantUML, PicoUML, Mermaid adapter) translate source text into a shared internal format. The **pipeline core** runs that text through a preprocessor (include resolution, macro expansion), a winnow-based parser that produces a span-annotated AST, a normalizer that detects the diagram family and builds a canonical model, and a renderer that emits deterministic SVG. **Transports** — the CLI binary, the `puml-lsp` LSP server, and the `puml-wasm` WebAssembly crate — all drive the same pipeline, with the `language_service` module providing hover, completion, semantic tokens, and diagnostics across all three surfaces.

See [the architecture doc](docs/architecture.md) for the full system breakdown with sequence, lifecycle, class, and parity diagrams.

</details>

## Documentation

- [Install guide](docs/install.md)
- [Quickstart](docs/quickstart.md)
- [Comparison vs PlantUML/Mermaid](docs/comparison.md)
- [FAQ](docs/faq.md)
- [CI integration](docs/ci-integration.md)
- [Troubleshooting](docs/troubleshooting.md)
- [Examples gallery](docs/examples/GALLERY.md)

## Contributing

Open a [GitHub issue](https://github.com/alliecatowo/puml/issues) for bugs, compatibility gaps, or scoped tasks. Use [discussions](https://github.com/alliecatowo/puml/discussions) for questions and ideas. Small fixtures, docs fixes, and renderer PRs are especially welcome — read [CONTRIBUTING.md](CONTRIBUTING.md) before larger changes.

## License

MIT. See [LICENSE](LICENSE).
