# puml

Fast, no-Java diagram rendering for PlantUML-compatible docs, CI, editors, and agents.

[![main gate](https://github.com/alliecatowo/puml/actions/workflows/main-gate.yml/badge.svg)](https://github.com/alliecatowo/puml/actions/workflows/main-gate.yml)
[![PR gate](https://github.com/alliecatowo/puml/actions/workflows/pr-gate.yml/badge.svg)](https://github.com/alliecatowo/puml/actions/workflows/pr-gate.yml)
[![docs site](https://github.com/alliecatowo/puml/actions/workflows/pages.yml/badge.svg)](https://github.com/alliecatowo/puml/actions/workflows/pages.yml)
[![version](https://img.shields.io/badge/version-0.1.0-0ea5e9)](Cargo.toml)
[![Rust 2021](https://img.shields.io/badge/rust-2021-f97316)](Cargo.toml)
[![license: MIT](https://img.shields.io/badge/license-MIT-22c55e)](LICENSE)
[![docs](https://img.shields.io/badge/docs-alliecatowo.github.io%2Fpuml-16a34a)](https://alliecatowo.github.io/puml/)

`puml` is a Rust renderer and CLI for turning diagram source into deterministic SVG or PNG. The goal is simple: make PlantUML-compatible rendering feel like a normal compiler tool, with no JVM, no Graphviz install, no browser runtime, and no rendering server required at runtime.

PlantUML is the compatibility target. `picouml` is this project's ergonomic superset path: a smaller, project-owned language surface that stays easy to write, diff, validate, and repair. Mermaid input already routes through adapter frontends for supported families, and the same architecture is intended to host future Markdown/HTML-facing frontends without moving rendering into JavaScript.

<p>
  <a href="docs/examples/groups_notes.puml"><img src="docs/examples/groups_notes.svg" alt="Sequence diagram with groups and notes" width="48%"></a>
  <a href="docs/examples/component/06_with_arrows.puml"><img src="docs/examples/component/06_with_arrows.svg" alt="Component diagram with arrows" width="48%"></a>
</p>
<p>
  <a href="docs/examples/gantt/05_multi_task.puml"><img src="docs/examples/gantt/05_multi_task.svg" alt="Gantt chart" width="48%"></a>
  <a href="docs/examples/json/03_nested.puml"><img src="docs/examples/json/03_nested.svg" alt="Nested JSON projection" width="48%"></a>
</p>

## Why puml?

Diagrams belong in source control. They should be quick to render locally, boring to run in CI, easy for editors to validate, and deterministic enough that humans and AI agents can safely collaborate on them.

`puml` is built around that compiler-shaped workflow:

- Native Rust binary: install one CLI and render offline.
- Deterministic output: committed SVG fixtures can act as real regression evidence.
- PlantUML compatibility: broad current coverage, tracked honestly as an ongoing target.
- Multiple frontends: PlantUML-compatible source, first-class `picouml`, selected Mermaid adapters, and a path for future Markdown/HTML surfaces.
- Automation-friendly diagnostics: `--check`, JSON diagnostics, markdown fence extraction, metadata, and pipeline dumps for bots, editors, and review tools.

## Quick Start

Install from GitHub:

```bash
cargo install --git https://github.com/alliecatowo/puml --bin puml
```

Or from a checkout:

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

Validate without rendering:

```bash
puml --check hello.puml
```

Render from stdin or choose PNG:

```bash
cat hello.puml | puml - > hello.svg
puml --format png --dpi 192 hello.puml -o hello@2x.png
```

Try the same pipeline on markdown fences:

```bash
puml --from-markdown --check docs/examples/sequence/README.md
puml --from-markdown docs/examples/sequence/README.md
```

## What Works Today

`puml` already renders a wide corpus of sequence, class, object, use-case, component, deployment, state, activity, timing, Gantt, chronology, mindmap, WBS, nwdiag, Archimate, C4-style, JSON, YAML, EBNF, regex, math, SDL, Ditaa, chart, theme, skinparam, preprocessor, and Creole examples.

The project is not claiming perfect PlantUML parity. Compatibility is a serious target, but the honest workflow is: run `puml --check`, compare output when a construct matters, and file or fix gaps with a small fixture.

- Browse the [example gallery](docs/examples/GALLERY.md) or the [site gallery](https://alliecatowo.github.io/puml/gallery/).
- Check current limits in [known limitations](docs/examples/KNOWN_LIMITATIONS.md).
- Read the detailed compatibility evidence in the [PlantUML parity source of truth](docs/audits/plantuml_parity_source_of_truth.md), [frontend conformance matrix](docs/plantuml_frontend_conformance_matrix.md), and [oracle threshold notes](docs/oracle-thresholds.md).

<details>
<summary>More CLI examples</summary>

```bash
# Dialects and determinism controls
puml --dialect plantuml --compat strict --determinism strict hello.puml
puml --dialect picouml --check design.picouml
puml --dialect mermaid input.mmd

# Pipeline inspection
puml --dump ast hello.puml
puml --dump model hello.puml
puml --dump scene hello.puml
puml --metadata hello.puml

# Text output modes
puml -txt hello.puml
puml --format utxt hello.puml -o hello.utxt

# Markdown linting
puml --from-markdown --check docs/examples/sequence/README.md
puml --check --lint-glob 'docs/**/*.md' --lint-report json

# Security posture for includes
puml --no-url-includes --check hello.puml

# Formatting
puml format hello.puml
puml format --check hello.puml
puml format --diff hello.puml
```

</details>

<details>
<summary>Frontend notes</summary>

PlantUML-compatible source is the default compatibility lane.

`picouml` files, `@startpicouml` blocks, `--dialect picouml`, and `picouml` fenced code blocks route through the PicoUML adapter and then into the shared parser, model, layout, and render pipeline. See the [PicoUML language baseline](docs/specs/picouml-language.md).

Mermaid support is an adapter path, not a JavaScript runtime dependency. Supported Mermaid inputs are normalized into the same shared pipeline; unsupported constructs should fail with deterministic diagnostics rather than silently falling back.

</details>

## Documentation

- [Docs site](https://alliecatowo.github.io/puml/) for the polished guide and browser experience.
- [Getting started](https://alliecatowo.github.io/puml/guide/getting-started/) for first render, browser editor, editor setup, and markdown fences.
- [CLI reference](https://alliecatowo.github.io/puml/guide/cli/) for modes, flags, dialects, includes, outputs, and exit codes.
- [Syntax primer](https://alliecatowo.github.io/puml/guide/syntax/) for the supported language surface.
- [All diagram families](https://alliecatowo.github.io/puml/guide/families/) for the current family map.
- [In-browser renderer](https://alliecatowo.github.io/puml/developer/renderer/) for the WASM bridge used by the site.
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
./scripts/check-all.sh
cargo run -- --help
cargo run -- --check docs/examples/basic_hello.puml
cargo run -- docs/examples/basic_hello.puml
```

Regenerate committed example SVGs after renderer changes:

```bash
for f in docs/examples/*.puml; do cargo run -- "$f"; done
for f in docs/examples/*/*.puml; do [ -f "$f" ] && cargo run -- "$f"; done
```

The static docs site lives in [site/](site/README.md). It mirrors `docs/examples/` into the gallery and mirrors `docs/specs/` into the developer reference pages.

<details>
<summary>Agent and swarm development context</summary>

This repo is deliberately friendly to human plus AI-agent development. The point is not novelty for its own sake; it is to make diagram work measurable enough that parallel contributors can add fixtures, tighten diagnostics, improve layout, and update docs without guessing.

Useful harnesses:

```bash
./scripts/harness-check.sh --quick
./scripts/harness-check.sh
./scripts/autonomy-check.sh --quick
./scripts/autonomy-check.sh
python3 ./scripts/parity_harness.py --fail-on-doc-drift --quiet
```

Runbooks live in [docs/codex-workflow.md](docs/codex-workflow.md) and [docs/autonomous-workflow-cookbook.md](docs/autonomous-workflow-cookbook.md).

</details>

## Contributing

Open-source help is welcome: forks, PRs, issues, discussions, small docs fixes, tiny compatibility fixtures, renderer fixes, LSP/editor work, WASM/site improvements, and benchmark evidence all count.

- Use [open issues](https://github.com/alliecatowo/puml/issues?q=is%3Aissue%20is%3Aopen) for bugs, compatibility gaps, and scoped tasks.
- Use [discussions](https://github.com/alliecatowo/puml/discussions) for questions, ideas, showcases, parity reports that need shaping, and development-workflow notes.
- Read [CONTRIBUTING.md](CONTRIBUTING.md) and [docs/contributing.md](docs/contributing.md) before larger changes.
- Open an issue before broad compatibility pushes so the work can be sliced clearly.
- Send small docs and fixture PRs without ceremony; fork PRs are very welcome.

This project is young, ambitious, and intentionally transparent. Some parts are polished; some edges are still sharp. Good reports and small reproducible examples are especially valuable.

## License

MIT. See [LICENSE](LICENSE).
