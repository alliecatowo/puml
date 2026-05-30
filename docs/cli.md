# CLI Reference

`puml` defaults to SVG output. Use `--format` or its PlantUML-compatible alias
`--output-format` to select another supported renderer:

```bash
puml --format png diagram.puml
puml --output-format html diagram.puml
```

Supported formats are `svg`, `html`, `png`, `jpg`, `webp`, `pdf`, `txt`, `atxt`,
and `utxt`.

PlantUML-style single-dash format aliases are accepted for supported output
formats: `-tsvg`, `-thtml`, `-tpng`, `-tjpg`, `-tjpeg`, `-twebp`, `-tpdf`,
`-ttxt`, `-tatxt`, and `-tutxt`. Legacy text aliases `-txt`, `-atxt`, and
`-utxt` remain accepted.

Unsupported parity formats such as `-tlatex` and `-tlatex:nopreamble` are parsed
and exit with code `2` using a deterministic diagnostic that lists the supported
formats.

## Color and output control

`puml` supports ANSI color in human-readable diagnostic output.

```bash
puml --color auto   # default: color when stderr is a TTY and NO_COLOR is unset
puml --color always # force color even when piped
puml --color never  # suppress color unconditionally
```

Setting the environment variable `NO_COLOR=1` has the same effect as `--color never`
when `--color auto` (the default) is in use. JSON (`--diagnostics json`) and stdrpt
(`--stdrpt`) output are never colored regardless of `--color`.

When rendering multiple diagrams (from a multi-block input, `--multi`, or a Markdown
file with `--from-markdown`), progress lines are written to stderr:

```
[1/3] rendering diagram_snippet_1...
[2/3] rendering diagram_snippet_2...
[3/3] rendering diagram_snippet_3...
```

For common errors, `puml` emits a `hint:` line suggesting a corrective flag:

- When URL includes are rejected: `hint: rerun with --allow-url-includes to permit URL includes`
- When multiple `@startuml` blocks are found on stdin without `--multi`: hint to add `--multi`
- When a Markdown file has no recognized fence block: hint to wrap source in ` ```puml ``` `

Compatibility flags:

| Flag | Behavior |
|---|---|
| `--pipe` | Read stdin and write the rendered diagram to stdout. |
| `--check-syntax` | Alias for `--check`. |
| `--preproc` | Print preprocessed source after include and macro expansion. |
| `--htmlcss` | No-op; HTML output is already self-contained. |
| `--metadata` | Print structured JSON metadata after parse and normalization; combine with `--metadata-output FILE` to write to a file instead of stdout. |
| `--metadata-output FILE` | Write `--metadata` JSON to FILE instead of stdout (requires `--metadata`). |
| `--threads N` | Accepted as a worker-count hint; output ordering remains deterministic. |
| `--failfast2` | Remap validation exit code 1 → 2 to match PlantUML convention for diagram errors. |
| `--extract` | Split a multi-diagram input into deterministic `.puml` source files; stdin writes split sources to stdout. |
| `--pattern REGEX` | Filter lint/check file selection by regex over resolved paths. |

For batch syntax checks, combine glob expansion with PlantUML-compatible path
filtering:

```bash
puml --check --lint-glob 'docs/**/*.puml' --pattern '/sequence/'
```

To split a file containing several `@startuml` blocks without rendering:

```bash
puml --extract diagrams/batch.puml
```
