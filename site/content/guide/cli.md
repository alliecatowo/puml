+++
title = "CLI reference"
description = "Every mode, flag, and exit code of the puml command-line tool."
weight = 20
+++

The `puml` CLI is the canonical reference implementation of the engine. Everything the studio editor will eventually do in your browser, the CLI already does on disk.

## Modes at a glance

| Mode                         | Trigger                              | Output                                          |
|------------------------------|--------------------------------------|-------------------------------------------------|
| Render single file           | `puml input.puml`                    | writes `input.svg` (or `.png` with `--format`)  |
| Render from stdin            | `cat x.puml \| puml -` or `\| puml`  | writes SVG to stdout                            |
| Check / lint                 | `puml --check input.puml`            | exit code only, diagnostics on stderr           |
| Multi-input lint             | `puml --check --lint-input ...`      | aggregated lint report on stdout                |
| Markdown fence extraction    | `puml --from-markdown notes.md`      | renders each fenced diagram block               |
| AST / model / scene dump     | `puml --dump ast input.puml`         | JSON on stdout                                  |
| Multi-page mode              | `puml --multi input.puml`            | `newpage` splits into numbered files / JSON     |
| Language server              | `puml-lsp`                           | LSP over stdio                                  |

## Inputs

- `INPUT` &mdash; a file path.
- `-` &mdash; read from stdin explicitly.
- omitted &mdash; read from stdin implicitly when stdin is piped (TTY stdin prints help).

## Flags

```text
--format svg|png             output format (default: svg)
--dpi FLOAT                  PNG rasterization DPI (default: 96)
--check                      parse + normalize only, no render output
--check-syntax               PlantUML-compatible alias for --check
--lint-input INPUT           repeatable check input (check mode only)
--lint-glob GLOB             repeatable glob-expanded check input
--lint-report human|json     lint summary report format (default: human)
--dump ast|model|scene       dump pipeline JSON instead of rendering
--multi                      allow multiple stdin diagrams / pages
--from-markdown              treat input as markdown, extract fenced blocks
--diagnostics human|json     diagnostics output format (default: human)
--stdrpt                     one-line diagnostics: severity\tcode\tfile:line:col\tmessage
--dialect auto|plantuml|mermaid|picouml
                             select frontend input dialect; auto uses file extensions and fences
--compat strict|extended     semantic compatibility policy (default: strict)
--determinism strict|full    determinism policy (default: strict)
--include-root DIR           resolve `!include` from this root for stdin
--allow-url-includes         allow URL includes for trusted compatibility runs
--no-url-includes            compatibility no-op; URL includes are disabled by default
--duration                   print elapsed wall time to stderr
--quiet / -q                 suppress non-error stderr
--verbose / -v               emit per-stage parse/normalize/render timings
--fail-on-warn               exit 1 if any warnings are emitted
--overwrite                  no-op (outputs are always overwritten)
--htmlcss                    no-op PlantUML compatibility flag for HTML output
--charset UTF-8              no-op (only UTF-8 is supported)
--output / -o PATH           write to PATH instead of the derived path
```

## Frontend dialects

```bash
# explicit dialect (default is auto)
puml --dialect plantuml input.puml
puml --dialect mermaid input.mmd
puml --dialect picouml input.picouml
```

- `auto` uses input hints first: `.picouml` files and `picouml` markdown fences route through the PicoUML adapter, while `mermaid` fences route through the Mermaid adapter.
- `plantuml` parses PlantUML-compatible source through the shared pipeline.
- `mermaid` accepts `sequenceDiagram`, `flowchart`/`graph`, `classDiagram`, `stateDiagram`/`stateDiagram-v2`, and `erDiagram`.
- `picouml` routes through PicoUML adapter rewrites first, then the same parser, model, layout, and renderer as PlantUML-compatible inputs.

## Includes and remote sources

The native CLI supports URL includes for PlantUML compatibility when explicitly
enabled: `!include https://...`, `!includeurl`, URL `!include_many`, URL
`!import`, and `file://` targets can fetch or read source and cache HTTP(S)
responses locally. Pass `--allow-url-includes` only for trusted inputs; without
it, URL targets fail with `E_INCLUDE_URL_DISABLED`.

Embedded surfaces are stricter by design. The LSP does not fetch remote includes
while publishing diagnostics or previews, the WASM/browser renderer rejects
filesystem and URL includes, and bundled MCP/agent tools keep URL includes
disabled unless a tool call explicitly sets `allow_url_includes: true`. See the
[URL include policy](https://github.com/alliecatowo/puml/blob/main/docs/url-includes.md)
for the surface-by-surface contract.

## Output paths

- Single diagram from file &rarr; `<input-stem>.svg` (or `.png`).
- Single diagram from stdin &rarr; SVG on stdout (or PNG bytes with `--format png`).
- Multi-page file inputs &rarr; numbered files `<stem>-1.svg`, `<stem>-2.svg`, &hellip;
- Multi-page stdin requires `--multi`; with it, stdout is a deterministic JSON array `[{"name": "...", "svg": "..."}, ...]`. Only SVG is supported in this mode.
- `ignore newpage` collapses multi-page splits into a single output.

## Exit codes

| Code | Meaning                       |
|------|-------------------------------|
| `0`  | success                       |
| `1`  | validation or usage failure   |
| `2`  | I/O failure                   |
| `3`  | internal failure              |

## Diagnostics

- Errors and warnings include `line` / `column` and caret snippets when source spans are available.
- Unsupported `skinparam` keys and `!theme` directives emit deterministic non-fatal warnings on stderr.
- `--diagnostics json` emits a stable schema:

```json
{
  "schema": "puml.diagnostics",
  "schema_version": 1,
  "diagnostics": [
    {
      "code": "E_PICOUML_MARKER_MIXED",
      "severity": "error",
      "message": "...",
      "span": { "start": 12, "end": 24 },
      "line": 2, "column": 1,
      "snippet": "@startpicouml",
      "caret": "^^^^^^^^^^^^^"
    }
  ]
}
```

## Stream contract

Always:

- Render / `--check`, `--check-syntax`, and `--dump` payloads go to **stdout**.
- Diagnostics (human or JSON) go to **stderr**.
- Lint batch mode keeps diagnostics on stderr and prints the lint summary on stdout.
