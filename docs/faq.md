# FAQ

## Is puml a drop-in replacement for PlantUML?

Not quite yet. It's the goal. Most common diagram families render well, and the CLI interface is designed to be familiar. But some advanced PlantUML features — deep preprocessor macros, certain skinparams, PDF output — are partial or missing. Run `puml --check` against your existing diagrams and see what happens. The honest status per feature row is tracked in [`docs/internal/parity/plantuml_parity_source_of_truth.md`](internal/parity/plantuml_parity_source_of_truth.md).

## Does it work without internet?

Yes. `puml` is a single static binary. Rendering is fully offline. The only exception is `--allow-url-includes`, which must be explicitly opted into for `!include` directives that fetch remote URLs.

## Can I use it in GitHub Actions?

Yes — see [CI integration](ci-integration.md) for ready-to-paste YAML snippets.

## What is PicoUML?

PicoUML is the project's own diagram language: a smaller, ergonomic superset of PlantUML that's easier to write, diff, and validate. Use `.picouml` files or `@startpicouml` blocks. PlantUML source continues to work as the primary compatibility lane. See the [PicoUML language spec](specs/picouml-language.md).

## How do I get the VS Code extension?

The extension lives in `extensions/vscode/` in this repo. Marketplace publishing is in progress. For now, build it from source or install via `code --install-extension`.

## What output formats are supported?

SVG (default), PNG, JPG, WebP, and self-contained HTML. ASCII and Unicode text output are also available via `--format txt` and `--format utxt`.

```bash
puml hello.puml                         # SVG
puml --format png hello.puml            # PNG
puml --format html hello.puml           # HTML
puml --format jpg hello.puml            # JPG
puml --format txt hello.puml            # ASCII text
```

## Will it ever support PDF?

Not currently planned for the near term. SVG → PDF conversion can be done with tools like Inkscape or `rsvg-convert` in the meantime. Open an issue if PDF is blocking you.

## How do I report a rendering bug?

Open a [GitHub issue](https://github.com/alliecatowo/puml/issues) with the `.puml` source that doesn't render correctly, the `puml` version (`puml --version`), and ideally a comparison against PlantUML output if you have it. A minimal reproducer is the most helpful contribution.

## Why is it called puml?

Short for PlantUML. The binary is `puml`; the crate is `puml`. PlantUML files end in `.puml`. It fits.
