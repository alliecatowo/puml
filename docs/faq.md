# FAQ

Frequently asked questions about `puml`.

---

## Is puml a drop-in replacement for PlantUML?

Not for every diagram yet. `puml` is PlantUML-compatible by goal and supports many
common diagram families, but the reference implementation is still PlantUML. Advanced
preprocessor behavior, some `skinparam` combinations, optional backends, and edge-case
syntax can differ.

The fastest compatibility test is:

```bash
find . -name '*.puml' -not -path './target/*' -exec puml --check {} +
```

Then visually inspect rendered output for important diagrams. The conservative status
trackers are:

- [`docs/internal/spec/plantuml-spec.md`](internal/spec/plantuml-spec.md)
- [`docs/internal/parity/plantuml_parity_source_of_truth.md`](internal/parity/plantuml_parity_source_of_truth.md)

---

## What formats can puml write?

The CLI help on current main lists these render formats:

| Format | Example |
|---|---|
| SVG | `puml diagram.puml` or `puml --format svg diagram.puml` |
| HTML | `puml --format html diagram.puml` |
| PNG | `puml --format png diagram.puml` |
| JPG | `puml --format jpg diagram.puml` |
| WebP | `puml --format webp diagram.puml` |
| PDF | `puml --format pdf diagram.puml` |
| ASCII text | `puml --format txt diagram.puml` |
| ASCII text variant | `puml --format atxt diagram.puml` |
| Unicode text | `puml --format utxt diagram.puml` |

PNG supports `--dpi`, for example:

```bash
puml --format png --dpi 192 diagram.puml
```

---

## Does puml support PDF?

Yes. Current main supports native CLI PDF export through SVG-to-PDF conversion:

```bash
puml --format pdf diagram.puml
```

SVG remains the canonical render path internally, so if a PDF looks wrong, also attach
the source `.puml` and the SVG output when filing a bug.

---

## Does puml work without internet access?

Yes for normal local rendering. The parser, preprocessor, layout engine, and exporters
run locally.

Remote URL includes are disabled by default. Opt in explicitly when your workflow needs
network includes:

```bash
puml --allow-url-includes diagram.puml
```

Local includes are supported. When reading source from stdin, use `--include-root` so
relative includes have a directory to resolve from:

```bash
cat diagram.puml | puml --include-root . -o - > diagram.svg
```

---

## How do I use stdin and stdout?

For general stdout output, use `-o -`:

```bash
puml diagram.puml -o - > diagram.svg
```

For PlantUML-compatible pipe mode, use `--pipe`:

```bash
cat diagram.puml | puml --pipe > diagram.svg
```

`--pipe` always reads stdin and writes stdout, so it conflicts with a positional input
file and with `-o`.

---

## Can I validate Markdown diagrams?

Yes. Put diagrams in fenced code blocks and run:

```bash
puml --from-markdown --check README.md
```

Supported fence labels include `puml`, `pumlx`, `picouml`, `plantuml`, `uml`,
`puml-sequence`, `uml-sequence`, and `mermaid`.

---

## Can I use puml in CI?

Yes. The common pattern is to install `puml`, then run `--check` over all diagram
sources:

```bash
find . -name '*.puml' -not -path './target/*' -exec puml --check {} +
```

For ready-to-paste GitHub Actions, GitLab CI, and pre-commit examples, see
[ci-integration.md](ci-integration.md).

---

## How do I get machine-readable diagnostics?

Use JSON diagnostics:

```bash
puml --diagnostics json --check diagram.puml
```

Or use single-line tab-separated diagnostics:

```bash
puml --stdrpt --check diagram.puml
```

For batch linting through top-level flags, use `--lint-report json`:

```bash
puml --check --lint-glob 'docs/**/*.puml' --lint-report json
```

The `lint` subcommand also supports JSON for one file:

```bash
puml lint --format json diagram.puml
```

---

## What is PicoUML?

PicoUML is this project's stricter, ergonomic dialect for diagrams. It is intended to be
easier to parse, validate, and repair while still sharing the same renderer pipeline.
Use `.picouml`, `@startpicouml` / `@endpicouml`, or `--dialect picouml` when you want to
select it explicitly.

---

## Does puml render Mermaid?

`puml` has selected Mermaid adapter support, especially for workflows that route
Markdown fences through the same renderer. It is not a complete Mermaid implementation.
If you rely on Mermaid-specific diagram types or host-native Mermaid rendering, keep
Mermaid in that workflow unless your examples pass `puml --check` and visual review.

---

## Why is the SVG different from PlantUML's SVG?

`puml` is an independent renderer. It parses compatible source but lays out and emits
SVG through its own Rust pipeline. Differences in spacing, edge routing, colors, font
metrics, and SVG structure are expected.

Please file a bug when the difference changes diagram meaning: missing nodes, incorrect
edges, wrong labels, broken grouping, unreadable overlaps, or unsupported syntax that
should be covered.

---

## How stable is puml?

The current crate version is `0.1.0`. The CLI is useful for docs and CI workflows, but
PlantUML parity is still growing. For production docs pipelines:

- Pin a release version or a Git commit.
- Run `puml --check` against your diagram corpus before upgrading.
- Keep generated SVG/PDF/PNG artifacts reviewable in pull requests.
- Subscribe to [GitHub Releases](https://github.com/alliecatowo/puml/releases) for
  changes.

---

## How do I report a rendering bug?

Open a [GitHub issue](https://github.com/alliecatowo/puml/issues) and include:

1. The smallest `.puml` source that reproduces the problem.
2. `puml --version`.
3. The command you ran.
4. What you expected and what happened instead.
5. PlantUML reference output, if you have it.

Small reproducers get fixed faster than large private diagrams.

---

## Can I use puml as a Rust library?

Yes. The crate exposes library APIs as well as the CLI binary. Pin the version in your
`Cargo.toml` and check release notes before upgrading, because the library API is less
stable than the CLI at this stage.

---

## Why Rust?

Rust gives `puml` its main operational properties: a native binary, fast startup,
memory-safe parsing of untrusted input, and a shared codebase for CLI and WASM targets.

---

## Why is it called puml?

`.puml` is the common PlantUML file extension, and `puml` is short enough to use often
in terminals, docs scripts, and CI logs.
