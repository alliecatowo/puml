# puml

Fast, no-Java diagram rendering for PlantUML-compatible docs, CI, editors, and agents.

[![main gate](https://github.com/alliecatowo/puml/actions/workflows/main-gate.yml/badge.svg)](https://github.com/alliecatowo/puml/actions/workflows/main-gate.yml)
[![PR gate](https://github.com/alliecatowo/puml/actions/workflows/pr-gate.yml/badge.svg)](https://github.com/alliecatowo/puml/actions/workflows/pr-gate.yml)
[![docs site](https://github.com/alliecatowo/puml/actions/workflows/pages.yml/badge.svg)](https://github.com/alliecatowo/puml/actions/workflows/pages.yml)
[![Rust 2021](https://img.shields.io/badge/rust-2021-f97316)](Cargo.toml)
[![version](https://img.shields.io/badge/version-0.1.0-0ea5e9)](Cargo.toml)
[![license: MIT](https://img.shields.io/badge/license-MIT-22c55e)](LICENSE)
[![docs](https://img.shields.io/badge/docs-alliecatowo.github.io%2Fpuml-16a34a)](https://alliecatowo.github.io/puml/)

`puml` is a Rust diagram engine and CLI for turning PlantUML-compatible text into deterministic SVG or PNG. It is meant to feel like a compiler tool: install one native binary, run it offline, commit the output, and let CI, editors, docs sites, and agents validate diagrams without a JVM, Graphviz, a browser runtime, or a rendering server.

PlantUML compatibility is the main target. PicoUML is the project-owned language direction: a smaller, ergonomic superset surface that should be easier to write, diff, validate, and repair. Mermaid and other frontends are adapter paths into the same renderer rather than separate rendering stacks.

<p>
  <a href="docs/examples/groups_notes.puml"><img src="docs/examples/groups_notes.svg" alt="Sequence diagram with groups and notes" width="48%"></a>
  <a href="docs/examples/component/06_with_arrows.puml"><img src="docs/examples/component/06_with_arrows.svg" alt="Component diagram with arrows" width="48%"></a>
</p>
<p>
  <a href="docs/examples/gantt/05_multi_task.puml"><img src="docs/examples/gantt/05_multi_task.svg" alt="Gantt chart" width="48%"></a>
  <a href="docs/examples/json/03_nested.puml"><img src="docs/examples/json/03_nested.svg" alt="Nested JSON projection" width="48%"></a>
</p>

## Why puml?

Diagrams belong in source control. They should be quick to render locally, easy to review as text, boring to run in CI, deterministic enough for snapshots, and precise enough for editor and AI-agent repair loops.

| Need | puml today |
|---|---|
| Native rendering | Rust CLI and library path; no Java runtime or server required. |
| PlantUML-compatible docs | PlantUML-style source is the compatibility lane, with feature depth tracked openly. |
| Project language evolution | PicoUML is the first-class project language direction and currently adapts into the shared pipeline. |
| Multiple frontends | Selected Mermaid families and markdown fences normalize into the same engine. |
| Automation | `--check`, JSON diagnostics, markdown extraction, pipeline dumps, `puml-lsp`, and a WASM-powered site/editor. |
| Reviewable output | SVG is the canonical render artifact; PNG export is available when raster output is needed. |

This project is young, ambitious, and run as an AI-driven swarm-development effort. Some paths are already pleasant; others are intentionally marked as partial while they grow. Small fixtures, bug reports, docs fixes, discussions, and PRs are welcome.

## Quick Start

Install from crates.io:

```bash
cargo install puml --bin puml
```

Install the latest GitHub version:

```bash
cargo install --git https://github.com/alliecatowo/puml --bin puml
```

Or install from a checkout:

```bash
git clone https://github.com/alliecatowo/puml.git
cd puml
cargo install --path . --bin puml
```

Render your first diagram:

```bash
cat > hello.puml <<'PUML'
@startuml
Alice -> Bob: Hello
Bob --> Alice: Ack
@enduml
PUML

puml hello.puml
# wrote hello.svg
```

Validate without writing output:

```bash
puml --check hello.puml
```

Render from stdin or write PNG:

```bash
cat hello.puml | puml - > hello.svg
puml --format png --dpi 192 hello.puml -o hello@2x.png
puml --format html hello.puml -o hello.html
puml --format jpg hello.puml -o hello.jpg
puml --format webp hello.puml -o hello.webp
```

Lint diagrams embedded in Markdown:

```bash
puml --from-markdown --check notes.md
```

## Language Surfaces

| Surface | Current framing |
|---|---|
| PlantUML | Primary compatibility target. Many diagram families render, but advanced feature parity varies. |
| PicoUML | Project-owned language/superset direction via `.picouml`, `@startpicouml`, `--dialect picouml`, and `picouml` fences. |
| Mermaid | Adapter frontend for selected families, including sequence, flowchart/graph, class, state, and ER work. |
| Markdown | Fenced diagram extraction for docs linting and rendering. |
| WASM/site | Browser editor and docs gallery use the same renderer through `crates/puml-wasm`. |
| LSP/editor | `puml-lsp` and the VS Code extension live in this repository. |

Honest compatibility rule: run `puml --check`, render the construct you care about, and compare output when visual parity matters. `puml` is PlantUML-compatible, not a claim of complete PlantUML 1:1 parity.

For deeper status, see the [example gallery](docs/examples/GALLERY.md), [known limitations](docs/examples/KNOWN_LIMITATIONS.md), [frontend conformance matrix](docs/plantuml_frontend_conformance_matrix.md), and [PlantUML parity source of truth](docs/audits/plantuml_parity_source_of_truth.md).

<details>
<summary>More CLI examples</summary>

```bash
# Dialects and compatibility controls
puml --dialect plantuml --compat strict --determinism strict hello.puml
puml --dialect picouml --check design.picouml
puml --dialect mermaid input.mmd

# Pipeline inspection
puml --dump ast hello.puml
puml --dump model hello.puml
puml --dump scene hello.puml
puml --metadata hello.puml

# Text output modes
puml --format txt hello.puml
puml --format utxt hello.puml -o hello.utxt

# Markdown linting
puml --from-markdown --check notes.md
puml --check --lint-glob 'docs/**/*.md' --lint-report json

# Include policy
puml --no-url-includes --check hello.puml

# Formatting
puml format hello.puml
puml format --check hello.puml
puml format --diff hello.puml
```

</details>

<details>
<summary>Frontend notes</summary>

PlantUML-compatible source is the default lane.

PicoUML inputs route through the same parser, model, layout, and renderer. Use `.picouml`, `@startpicouml` blocks, `--dialect picouml`, or `picouml` fenced code blocks. See the [PicoUML language baseline](docs/specs/picouml-language.md).

Mermaid support is an adapter path, not a JavaScript runtime dependency. Supported Mermaid inputs normalize into the shared pipeline; unsupported constructs should fail with deterministic diagnostics rather than silently switching renderers.

</details>

## Documentation

- [Docs site](https://alliecatowo.github.io/puml/) for guides, the browser editor, and the rendered gallery.
- [Getting started](https://alliecatowo.github.io/puml/guide/getting-started/) for install, first render, editor setup, and markdown fences.
- [CLI reference](https://alliecatowo.github.io/puml/guide/cli/) for modes, flags, diagnostics, dialects, includes, outputs, and exit codes.
- [Syntax primer](https://alliecatowo.github.io/puml/guide/syntax/) for the shared language model.
- [All diagram families](https://alliecatowo.github.io/puml/guide/families/) for the current family map.
- [Developer guide](https://alliecatowo.github.io/puml/developer/) for architecture, pipeline, contributing, and specs.
- [Troubleshooting](docs/troubleshooting.md) for diagnostics and common failure modes.

## Development

One-time setup:

```bash
./scripts/setup.sh
./scripts/install-hooks.sh
```

Useful local loops:

```bash
./scripts/dev.sh
./scripts/check-all.sh --quick
./scripts/bench.sh --check-artifacts
./scripts/branch-protection.sh verify
cargo run -- --check docs/examples/basic_hello.puml
cargo run -- docs/examples/basic_hello.puml
```

The static docs site lives in [site/](site/README.md). It mirrors `docs/examples/` into the gallery and mirrors `docs/specs/` into the developer reference pages.

<details>
<summary>Agent and swarm development</summary>

This repo is deliberately friendly to human plus AI-agent development. The work is sliced around fixtures, deterministic diagnostics, docs-as-tests, and harnesses so parallel contributors can improve compatibility, layout, docs, and editor support without guessing.

Useful validation loops:

```bash
./scripts/harness-check.sh --quick
./scripts/autonomy-check.sh --quick
./scripts/branch-protection.sh verify
python3 ./scripts/parity_harness.py --fail-on-doc-drift --quiet
```

Runbooks live in [docs/codex-workflow.md](docs/codex-workflow.md) and [docs/autonomous-workflow-cookbook.md](docs/autonomous-workflow-cookbook.md).

</details>

## Contributing

Open-source help is welcome: forks, PRs, issues, discussions, small docs fixes, tiny compatibility fixtures, renderer fixes, LSP/editor work, WASM/site improvements, and benchmark evidence all count.

- Use [issues](https://github.com/alliecatowo/puml/issues?q=is%3Aissue%20is%3Aopen) for reproducible bugs, compatibility gaps, regressions, and scoped tasks.
- Use [discussions](https://github.com/alliecatowo/puml/discussions) for questions, ideas, showcases, parity reports that need shaping, and AI-swarm workflow notes.
- Read [CONTRIBUTING.md](CONTRIBUTING.md) and [docs/contributing.md](docs/contributing.md) before larger changes.
- Open an issue before broad compatibility pushes so the work can be sliced clearly.

This project is transparent by design. A small failing diagram or a clear before/after render is often the most valuable contribution.

## License

MIT. See [LICENSE](LICENSE).
