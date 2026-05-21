# puml

> **PlantUML diagrams. No Java. Native speed.**

[![main gate](https://github.com/alliecatowo/puml/actions/workflows/main-gate.yml/badge.svg)](https://github.com/alliecatowo/puml/actions/workflows/main-gate.yml)
[![PR gate](https://github.com/alliecatowo/puml/actions/workflows/pr-gate.yml/badge.svg)](https://github.com/alliecatowo/puml/actions/workflows/pr-gate.yml)
[![docs site](https://github.com/alliecatowo/puml/actions/workflows/pages.yml/badge.svg)](https://github.com/alliecatowo/puml/actions/workflows/pages.yml)
[![version](https://img.shields.io/badge/version-0.1.0-0ea5e9)](Cargo.toml)
[![license: MIT](https://img.shields.io/badge/license-MIT-22c55e)](LICENSE)
[![docs](https://img.shields.io/badge/docs-alliecatowo.github.io%2Fpuml-16a34a)](https://alliecatowo.github.io/puml/)

**puml** is a fast, offline-first PlantUML-compatible diagram renderer written in Rust.
Give it a `.puml` file and get a pixel-perfect SVG, PNG, or PDF out — no Java, no Node,
no network. It ships as a single static binary, a WebAssembly module for in-browser
editing, and a Language Server (LSP) for editor integration across 25+ diagram families.

<details>
<summary><b>How it works</b> — pipeline architecture</summary>

<br>

![Architecture overview](docs/diagrams/architecture-overview.svg)

A request enters via one of three **transports** (CLI, LSP, WASM), passes through the
**preprocessor + language service**, hits the **pipeline core** (parser → AST →
normalizer → renderer), and exits as SVG / PNG / Text. The renderer is the only
component that knows about each diagram family's visual conventions; everything
upstream is family-agnostic AST.

</details>

---

## Quick start

```bash
# 1. Install (see all install options below)
cargo install puml --bin puml

# 2. Write a diagram
cat > hello.puml <<'EOF'
@startuml
Alice -> Bob: Hello
Bob --> Alice: Ack
@enduml
EOF

# 3. Render
puml hello.puml               # writes hello.svg
puml --format png hello.puml  # writes hello.png
puml --check hello.puml       # lint without writing
```

Open `hello.svg` in any browser or SVG viewer. Done.

**[Try it live in the browser →](https://alliecatowo.github.io/puml/editor)**

---

## Gallery

puml renders **25+ diagram families** from a single `.puml` source. Here's a taste:

### UML Core

<table>
  <tr>
    <td align="center" width="33%">
      <a href="docs/examples/sequence/05_alt_opt_loop.puml">
        <img src="docs/examples/sequence/05_alt_opt_loop.svg" alt="Sequence diagram with alt/opt/loop" width="300">
      </a>
      <br><b>Sequence</b>
      <br><sub>Messages, alt/opt/loop, participants</sub>
    </td>
    <td align="center" width="33%">
      <a href="docs/examples/class/10_full_domain.puml">
        <img src="docs/examples/class/10_full_domain.svg" alt="Class diagram full domain model" width="300">
      </a>
      <br><b>Class</b>
      <br><sub>Inheritance, composition, interfaces</sub>
    </td>
    <td align="center" width="33%">
      <a href="docs/examples/state/08_full_machine.puml">
        <img src="docs/examples/state/08_full_machine.svg" alt="State machine diagram" width="300">
      </a>
      <br><b>State Machine</b>
      <br><sub>Composite states, transitions, history</sub>
    </td>
  </tr>
  <tr>
    <td align="center">
      <a href="docs/examples/activity/08_order_processing.puml">
        <img src="docs/examples/activity/08_order_processing.svg" alt="Activity diagram with swimlanes" width="300">
      </a>
      <br><b>Activity</b>
      <br><sub>Fork/join, partitions, swimlanes</sub>
    </td>
    <td align="center">
      <a href="docs/examples/usecase/05_actor_generalization_system_boundary.puml">
        <img src="docs/examples/usecase/05_actor_generalization_system_boundary.svg" alt="Use case diagram" width="300">
      </a>
      <br><b>Use Case</b>
      <br><sub>Actors, extend/include, system boundaries</sub>
    </td>
    <td align="center">
      <a href="docs/examples/component/04_deployment_style.puml">
        <img src="docs/examples/component/04_deployment_style.svg" alt="Component diagram" width="300">
      </a>
      <br><b>Component</b>
      <br><sub>Interfaces, packages, dependencies</sub>
    </td>
  </tr>
</table>

### Architecture & Infrastructure

<table>
  <tr>
    <td align="center" width="33%">
      <a href="docs/examples/deployment/06_kubernetes_pods_containers.puml">
        <img src="docs/examples/deployment/06_kubernetes_pods_containers.svg" alt="Kubernetes deployment diagram" width="300">
      </a>
      <br><b>Deployment</b>
      <br><sub>Kubernetes namespaces, pods, containers</sub>
    </td>
    <td align="center" width="33%">
      <a href="docs/examples/c4/03_containers.puml">
        <img src="docs/examples/c4/03_containers.svg" alt="C4 container diagram" width="300">
      </a>
      <br><b>C4 Container</b>
      <br><sub>Context, containers, components, code</sub>
    </td>
    <td align="center" width="33%">
      <a href="docs/examples/nwdiag/02_multiple_nets.puml">
        <img src="docs/examples/nwdiag/02_multiple_nets.svg" alt="Network diagram" width="300">
      </a>
      <br><b>Network (nwdiag)</b>
      <br><sub>Hosts, subnets, device groups</sub>
    </td>
  </tr>
</table>

### Planning & Data

<table>
  <tr>
    <td align="center" width="25%">
      <a href="docs/examples/gantt/05_multi_task.puml">
        <img src="docs/examples/gantt/05_multi_task.svg" alt="Gantt chart" width="220">
      </a>
      <br><b>Gantt</b>
      <br><sub>Tasks, dependencies, milestones</sub>
    </td>
    <td align="center" width="25%">
      <a href="docs/examples/mindmap/03_with_colors.puml">
        <img src="docs/examples/mindmap/03_with_colors.svg" alt="MindMap with colors" width="220">
      </a>
      <br><b>MindMap</b>
      <br><sub>Hierarchies, colors, styling</sub>
    </td>
    <td align="center" width="25%">
      <a href="docs/examples/wbs/04_multi_level.puml">
        <img src="docs/examples/wbs/04_multi_level.svg" alt="Work breakdown structure" width="220">
      </a>
      <br><b>WBS</b>
      <br><sub>Work breakdown structures</sub>
    </td>
    <td align="center" width="25%">
      <a href="docs/examples/chart/04_multi_series.puml">
        <img src="docs/examples/chart/04_multi_series.svg" alt="Multi-series chart" width="220">
      </a>
      <br><b>Charts</b>
      <br><sub>Bar, line, pie, multi-series</sub>
    </td>
  </tr>
</table>

### Timing & Structured Data

<table>
  <tr>
    <td align="center" width="50%">
      <a href="docs/examples/timing/05_concurrent_timelines_message_arrows.puml">
        <img src="docs/examples/timing/05_concurrent_timelines_message_arrows.svg" alt="Timing diagram with concurrent timelines" width="380">
      </a>
      <br><b>Timing</b>
      <br><sub>Concurrent timelines, signals, clocks</sub>
    </td>
    <td align="center" width="50%">
      <a href="docs/examples/json/04_deep_nesting_arrays_of_objects.puml">
        <img src="docs/examples/json/04_deep_nesting_arrays_of_objects.svg" alt="JSON diagram" width="380">
      </a>
      <br><b>JSON / YAML</b>
      <br><sub>Visualize structured data inline</sub>
    </td>
  </tr>
</table>

[Browse the full gallery — all 25+ families with 280+ examples →](docs/examples/GALLERY.md)

---

## What the source looks like

A few examples to show how compact `.puml` syntax is:

<details>
<summary><b>Sequence diagram</b></summary>

```plantuml
@startuml
autonumber
actor User
participant "API Gateway" as GW
participant AuthService
participant Database

User -> GW: POST /login
GW -> AuthService: validateCredentials(user, pass)
AuthService -> Database: SELECT * FROM users WHERE email=?
Database --> AuthService: user row
alt credentials valid
  AuthService --> GW: JWT token
  GW --> User: 200 OK { token }
else invalid
  AuthService --> GW: 401
  GW --> User: 401 Unauthorized
end
@enduml
```

</details>

<details>
<summary><b>C4 Container diagram</b></summary>

```plantuml
@startuml
!include <C4/C4_Container>

Person(user, "Customer", "Uses the app")
System_Boundary(app, "E-Commerce Platform") {
  Container(web, "Web App", "React", "SPA served from CDN")
  Container(api, "API", "Rust/Axum", "Business logic")
  ContainerDb(db, "Database", "PostgreSQL", "Orders, products, users")
  Container(cache, "Cache", "Redis", "Session + catalogue cache")
}
System_Ext(payment, "Stripe", "Payment processing")

Rel(user, web, "Uses", "HTTPS")
Rel(web, api, "Calls", "REST/JSON")
Rel(api, db, "Reads/writes", "SQL")
Rel(api, cache, "Reads/writes", "Redis protocol")
Rel(api, payment, "Charges cards", "HTTPS")
@enduml
```

</details>

<details>
<summary><b>Class diagram — design patterns</b></summary>

```plantuml
@startuml
package "Observer Pattern" {
  interface Subject {
    +attach(o: Observer)
    +detach(o: Observer)
    +notify()
  }
  interface Observer {
    +update(event: Event)
  }
  class EventBus implements Subject {
    -observers: List<Observer>
    +publish(event: Event)
  }
  class MetricsCollector implements Observer
  class AuditLogger implements Observer
  Subject <|.. EventBus
  EventBus --> Observer
}
@enduml
```

</details>

<details>
<summary><b>Gantt chart</b></summary>

```plantuml
@startgantt
Project starts 2025-06-01
[Design] lasts 5 days
[Backend API] lasts 8 days
[Backend API] starts at [Design]'s end
[Frontend] lasts 10 days
[Frontend] starts at [Design]'s end
[Integration] lasts 3 days
[Integration] starts at [Backend API]'s end
[Integration] starts at [Frontend]'s end
[Release] happens at [Integration]'s end
@endgantt
```

</details>

---

<details>
<summary><b>Install options (Cargo, binary, Homebrew, npm, Docker)</b></summary>

### Pre-built binary — no Rust required

Download the latest release for your platform from the
[Releases page](https://github.com/alliecatowo/puml/releases):

| Platform | Asset |
|---|---|
| Linux x86-64 | `puml-x86_64-unknown-linux-musl.tar.gz` |
| macOS (Apple Silicon) | `puml-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `puml-x86_64-apple-darwin.tar.gz` |
| Windows x86-64 | `puml-x86_64-pc-windows-msvc.zip` |

Extract and place the `puml` binary on your `$PATH`.

### Homebrew (macOS / Linux)

```bash
brew install alliecatowo/tap/puml
```

### npm / npx — Node users

```bash
npx puml-cli hello.puml          # one-off, no install needed
npm install -g puml-cli          # global install
```

### Docker

```bash
docker run --rm -v "$PWD":/work ghcr.io/alliecatowo/puml:latest hello.puml
```

### Cargo — Rust toolchain

```bash
cargo install puml --bin puml
```

### Build from source

```bash
git clone https://github.com/alliecatowo/puml.git
cd puml
cargo build --release
./target/release/puml hello.puml
```

See the full [install guide](docs/install.md) for proxy settings, checksum verification,
and platform-specific notes.

</details>

---

<details>
<summary><b>Why puml — not PlantUML or Mermaid?</b></summary>

| | PlantUML | Mermaid | puml |
|---|---|---|---|
| Runtime | JVM required | Node + browser | Single static Rust binary |
| Offline | Yes (with Java installed) | No (needs browser) | Yes, always |
| Output | SVG, PNG, PDF | SVG (browser-rendered) | SVG, PNG, JPG, WebP, HTML |
| Determinism | Varies by JVM version | Varies by browser | Deterministic across platforms |
| CLI | Yes | Limited | Yes — designed as a compiler tool |
| LSP / editor | Third-party | Third-party | Built-in (`puml-lsp`) |
| WASM | No | Yes | Yes (`crates/puml-wasm`) |

**PlantUML** is the gold standard for feature breadth. Use it if you need full parity
today and can accept the JVM dependency.

**Mermaid** is great for quick diagrams embedded in GitHub Markdown and wikis. It needs
a browser runtime to render and does not produce diff-friendly offline artifacts.

**puml** is for teams that want diagrams in source control, reviewed as text, rendered
offline, and wired into CI and editors without installing Java or Node.

</details>

---

<details>
<summary><b>What diagram families are supported?</b></summary>

Around 25 families:

- **UML** — sequence, class, object, use case, component, deployment, state, activity, timing
- **Planning** — Gantt, chronology, WBS, MindMap
- **Structured data** — JSON, YAML, EBNF, regex, math, Salt wireframes
- **Architecture** — C4-style, Archimate, nwdiag
- **Other** — SDL, ditaa, chart

**PicoUML** is the project's own ergonomic dialect — a smaller, cleaner superset of
PlantUML syntax that is easier to write, diff, validate, and repair. Mermaid sequence
and flowchart inputs are also accepted via an adapter into the same renderer.

Browse the [examples gallery](docs/examples/GALLERY.md) for rendered output from every
family.

</details>

---

<details>
<summary><b>CLI, LSP, WASM, and VS Code details</b></summary>

### CLI

```bash
# Render
puml hello.puml                          # → hello.svg
puml --format png --dpi 192 hello.puml   # → hello.png at 2x
puml --format html hello.puml            # → hello.html (self-contained)

# Lint
puml --check hello.puml                  # exit 0 = valid
puml --from-markdown --check notes.md    # lint all fenced puml blocks in a Markdown file

# Pipeline inspection (for debugging and tooling)
puml --dump ast hello.puml
puml --dump model hello.puml
puml --dump scene hello.puml
```

Full flag reference, dialect options, and exit codes: [CLI reference](https://alliecatowo.github.io/puml/guide/cli/)

### Language Server (LSP)

`puml-lsp` ships in this repo. It provides diagnostics, hover, completions, and semantic
tokens for any editor that speaks the Language Server Protocol (Neovim, Emacs, Helix,
Zed, and others via generic LSP config).

```bash
cargo install --git https://github.com/alliecatowo/puml --bin puml-lsp
```

Point your editor's LSP config at `puml-lsp` for `.puml` and `.picouml` files.

### WebAssembly

The renderer compiles to WebAssembly via `crates/puml-wasm`. The live browser editor at
[alliecatowo.github.io/puml/editor](https://alliecatowo.github.io/puml/editor) runs the
full pipeline client-side with no server.

### VS Code extension

A VS Code extension lives under `extensions/vscode/` in this repo. It wraps `puml-lsp`
and adds preview, syntax highlighting, and snippet support. *(Screenshot pending —
tracked separately.)*

</details>

---

<details>
<summary><b>PlantUML compatibility status</b></summary>

`puml` is PlantUML-compatible — not a claim of complete 1:1 parity. Many diagram
families render well today; some advanced features are partial and tracked openly.

Run `puml --check` on your files and compare output when pixel-perfect parity matters.
The feature-by-feature status lives in
[`docs/internal/parity/plantuml_parity_source_of_truth.md`](docs/internal/parity/plantuml_parity_source_of_truth.md).

</details>

---

<details>
<summary><b>Project status and roadmap</b></summary>

`puml` is at v0.1.0 — young, ambitious, and developed with significant AI assistance.
Baseline rendering across all major diagram families landed in the parity blitz
(May 2025); advanced feature depth is an ongoing effort.

Active epics:
- [#82](https://github.com/alliecatowo/puml/issues/82) — Truth-reset parity
- [#88](https://github.com/alliecatowo/puml/issues/88) — Oracle conformance suite
- [#89](https://github.com/alliecatowo/puml/issues/89) — CI hardening
- [#399](https://github.com/alliecatowo/puml/issues/399) — Language service
- [#590](https://github.com/alliecatowo/puml/issues/590) — Layout engine (stages 1-4)

See the [GitHub milestone view](https://github.com/alliecatowo/puml/milestones) for
what is planned next.

</details>

---

<details>
<summary><b>How it works — pipeline internals</b></summary>

puml is structured as a three-layer pipeline:

- **Frontends** — PlantUML, PicoUML, and a Mermaid adapter translate source text into a
  shared internal format.
- **Pipeline core** — the preprocessor resolves `!include` directives and macros; the
  winnow-based parser produces a span-annotated AST; the normalizer detects the diagram
  family and builds a canonical model; the renderer emits deterministic SVG.
- **Transports** — the CLI binary, `puml-lsp`, and `puml-wasm` all drive the same
  pipeline. The `language_service` module provides hover, completion, semantic tokens, and
  diagnostics uniformly across all three surfaces.

Full breakdown with sequence, lifecycle, class, and parity diagrams:
[docs/architecture.md](docs/architecture.md)

</details>

---

<details>
<summary><b>Development setup</b></summary>

Prerequisites: Rust 1.78+ stable — [rustup.rs](https://rustup.rs)

```bash
git clone https://github.com/alliecatowo/puml.git
cd puml

# Build
cargo build --release

# Lint + test (required before any commit to main)
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test --release

# Render a diagram
./target/release/puml docs/examples/sequence/01_basic.puml -o /tmp/out.svg

# Regenerate the full PNG audit corpus (for visual QA)
python3 scripts/render_corpus.py --force

# Quick harness check
./scripts/harness-check.sh --quick
```

Read [CONTRIBUTING.md](CONTRIBUTING.md) for the full workflow, branch naming, commit
format, and CI gate requirements.

</details>

---

## Documentation

- [Install guide](docs/install.md)
- [Quickstart](docs/quickstart.md)
- [CLI reference](https://alliecatowo.github.io/puml/guide/cli/)
- [Comparison vs PlantUML / Mermaid](docs/comparison.md)
- [FAQ](docs/faq.md)
- [CI integration](docs/ci-integration.md)
- [Troubleshooting](docs/troubleshooting.md)
- [Examples gallery](docs/examples/GALLERY.md)
- [Architecture](docs/architecture.md)

## Contributing

Open a [GitHub issue](https://github.com/alliecatowo/puml/issues) for bugs,
compatibility gaps, or feature requests. Use
[Discussions](https://github.com/alliecatowo/puml/discussions) for questions and ideas.
Renderer fixes, fixture additions, and documentation improvements are especially
welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) before larger changes.

---

<sub>
<a href="CONTRIBUTING.md">Contributing</a> &nbsp;·&nbsp;
<a href="CODE_OF_CONDUCT.md">Code of Conduct</a> &nbsp;·&nbsp;
<a href="SECURITY.md">Security</a> &nbsp;·&nbsp;
<a href="LICENSE">MIT License</a>
</sub>
