# puml

Fast, deterministic diagram rendering to SVG/PNG with a polymorphic multi-language frontend (PicoUML, PlantUML, Mermaid), strict validation, and scriptable CLI modes.

![version](https://img.shields.io/badge/version-0.1.0-0ea5e9)
![rust](https://img.shields.io/badge/rust-2021-f97316)
![scope](https://img.shields.io/badge/scope-full%20PlantUML%20parity%20target-14b8a6)
![license](https://img.shields.io/badge/license-MIT-22c55e)
![docs parity](https://img.shields.io/badge/docs--as--tests-enabled-16a34a)
![determinism](https://img.shields.io/badge/svg-deterministic-0f766e)
![agent harness](https://img.shields.io/badge/codex%2Fclaude-harness--ready-f59e0b)

## PlantUML parity roadmap

`puml` is pursuing full 1:1 parity with PlantUML as an aggressive roadmap goal, with staged family-lane implementation to preserve deterministic parser/normalizer/layout/render contracts. Current support is not full PlantUML parity. The canonical current status is tracked in [`docs/audits/plantuml_parity_source_of_truth.md`](docs/audits/plantuml_parity_source_of_truth.md), where support is classified conservatively as `implemented`, `partial`, or `missing`.

Language and compatibility statement:
- PicoUML is the first-class canonical language surface for this engine.
- PlantUML is a first-class compatibility target across implemented and planned diagram families; full 1:1 parity remains the roadmap ambition, not the current claim.
- Mermaid is first-class for `sequenceDiagram`, `flowchart`/`graph`, `classDiagram`, `stateDiagram`/`stateDiagram-v2`, and `erDiagram` families, with deterministic diagnostics for unsupported constructs.

Source-of-truth hierarchy:
- Current support status: [`docs/audits/plantuml_parity_source_of_truth.md`](docs/audits/plantuml_parity_source_of_truth.md)
- Machine-readable exports: [`docs/audits/parity_gap_core.csv`](docs/audits/parity_gap_core.csv) and [`docs/audits/parity_gap_nonuml.csv`](docs/audits/parity_gap_nonuml.csv)
- Remaining high-impact planning slices: [`docs/audits/post_blitz_gap_table.md`](docs/audits/post_blitz_gap_table.md)
- Examples and oracle reports are evidence inputs, not independent parity claims.

## Install And Dev

```bash
# clone + enter
git clone <your-fork-or-repo-url>
cd puml

# one-time dev setup
./scripts/setup.sh

# fast local loop (fmt + clippy + test)
./scripts/dev.sh

# full quality gate (fmt + clippy + test + llvm-cov + release build + bench gates)
./scripts/check-all.sh

# quick quality gate (skips coverage + release build, keeps quick bench gates)
./scripts/check-all.sh --quick
```

## CI/CD

GitHub Actions enforces gate scripts from this repo directly:

- PR gate workflow: `.github/workflows/pr-gate.yml`
  runs `fmt` -> `clippy` -> `test` -> coverage gate -> `./scripts/check-all.sh --quick`
  uploads quick benchmark artifacts (`latest*`, `latest_trend*`)
- Main gate workflow: `.github/workflows/main-gate.yml`
  runs `./scripts/check-all.sh` (full gate)
  publishes benchmark evidence artifacts (`latest*`, `latest_trend*`, baselines, `parity_latest.json`)
- Branch protection/ruleset contract: [`docs/branch-protection.md`](docs/branch-protection.md)
  validation command: `./scripts/branch-protection.sh verify`

## CLI Usage (Explicit Modes)

```bash
# help
cargo run -- --help

# 1) FILE INPUT -> renders <input-stem>.svg
cargo run -- tests/fixtures/basic/hello.puml

# PNG output (rasterized from deterministic SVG)
cargo run -- --format png tests/fixtures/basic/hello.puml
cargo run -- --format png --dpi 192 tests/fixtures/basic/hello.puml -o hello@2x.png

# 2) STDIN INPUT (explicit '-') -> render SVG to stdout
cat tests/fixtures/basic/hello.puml | cargo run -- -

# 3) STDIN INPUT (implicit, no INPUT arg) -> render SVG to stdout
cat tests/fixtures/basic/hello.puml | cargo run --

# check-only mode (parse + normalize, no render output)
cargo run -- --check tests/fixtures/basic/hello.puml
cat tests/fixtures/basic/hello.puml | cargo run -- --check -

# batch docs/CI lint mode (repeatable inputs + globs)
cargo run -- --check --lint-input docs/examples/basic_hello.puml --lint-input docs/examples/groups_notes.puml
cargo run -- --check --lint-glob 'docs/**/*.md' --lint-report json

# dump pipeline JSON
cargo run -- --dump ast tests/fixtures/basic/hello.puml
cargo run -- --dump model tests/fixtures/basic/hello.puml
cargo run -- --dump scene tests/fixtures/basic/hello.puml

# multi-diagram mode (must be explicit)
cargo run -- --multi tests/fixtures/structure/multi_three.puml
cat tests/fixtures/structure/multi_three.puml | cargo run -- --multi -

# markdown fenced extraction mode (auto-enabled for .md/.markdown/.mdown files)
cargo run -- --from-markdown --check docs/sequence-notes.md

# machine-readable diagnostics
cargo run -- --check --diagnostics json tests/fixtures/invalid_single.puml

# LSP server (stdio)
cargo run --bin puml-lsp

# frontend + mode controls
cargo run -- --dialect auto --compat strict --determinism strict tests/fixtures/basic/hello.puml
cargo run -- --dialect plantuml --check tests/fixtures/basic/hello.puml

# stdin + include support
cat tests/fixtures/include/include_ok_child.puml | cargo run -- --check --include-root ./tests/fixtures/include -

# runtime compatibility flags
#   --duration         print elapsed wall time to stderr
#   --quiet / -q       suppress non-error stderr
#   --verbose / -v     emit per-stage parse/normalize/render timings
#   --fail-on-warn     exit 1 if any warnings are emitted
#   --overwrite        no-op (outputs are always overwritten)
#   --charset UTF-8    no-op compatibility (only UTF-8 is supported)
#   --format svg|png   render as deterministic SVG or PNG
cargo run -- --verbose --duration --check tests/fixtures/basic/hello.puml
```

Runtime parity flag notes:
- When stdin is a TTY and no input file is supplied, the CLI prints help instead of blocking forever.
- `--format png` rasterizes deterministic SVG output; use `--dpi` to control PNG scale.
- `--charset` accepts only `UTF-8` (case-insensitive); other charsets are rejected with `E_CHARSET_UNSUPPORTED`.

## Asciicast-Style Example

```console
$ cat > hello.puml <<'PUML'
@startuml
Alice -> Bob: hello
@enduml
PUML
$ cargo run -- hello.puml
$ ls hello.svg
hello.svg
$ cargo run -- --check hello.puml
# exits 0 with no validation errors
```

## Rendered Examples

Canonical examples live in [`docs/examples/README.md`](docs/examples/README.md), with committed source/output pairs.
Supported primitive catalog page: [`docs/examples/supported_primitives.md`](docs/examples/supported_primitives.md).
These examples are coverage seeds and executable documentation artifacts, not proof of full PlantUML 1:1 parity and not a replacement for the source-of-truth audit table. Use [`docs/audits/plantuml_parity_source_of_truth.md`](docs/audits/plantuml_parity_source_of_truth.md) for current implemented/partial/missing status.
Current docs corpus footprint: `docs/examples/` contains `254` `.puml` sources and `258` `.svg` artifacts.

Re-generate all committed examples:

```bash
for f in docs/examples/*.puml; do
  cargo run -- "$f"
done
```

### Basic Hello

Source: [`docs/examples/basic_hello.puml`](docs/examples/basic_hello.puml)

![Basic Hello](docs/examples/basic_hello.svg)

### Groups And Notes

Source: [`docs/examples/groups_notes.puml`](docs/examples/groups_notes.puml)

![Groups And Notes](docs/examples/groups_notes.svg)

## CLI Contract

Inputs:
- `INPUT` file path
- `-` for stdin
- omitted `INPUT` reads stdin

Modes:
- default renders SVG
- `--format svg|png` selects output format (default `svg`)
- `--dpi FLOAT` controls PNG rasterization DPI (default `96`; only used with `--format png`)
- `--check` parses + normalizes only
- `--lint-input INPUT` adds repeatable check/lint inputs (check mode only)
- `--lint-glob GLOB` adds repeatable glob-expanded check/lint inputs (check mode only)
- `--lint-report human|json` emits lint summary report format (default `human`)
- `--dump ast|model|scene` emits JSON
- `--multi` permits multiple stdin diagrams/pages (required when stdin expands to more than one diagram/page)
- `--from-markdown` treats input as markdown and only extracts fenced diagram blocks
  supported markdown fence langs: `puml`, `pumlx`, `picouml`, `plantuml`, `uml`, `puml-sequence`, `uml-sequence`, `mermaid`
- `--diagnostics human|json` controls diagnostics output format (default `human`)
- `--stdrpt` emits each diagnostic as a single tab-separated line: `<severity>\t<code>\t<file>:<line>:<col>\t<message>`, suppressing multi-line source-context output (exit codes are unchanged)
- `--dialect auto|plantuml|mermaid|picouml` selects frontend input dialect (default `auto`)
  `auto|plantuml`: parse PlantUML sequence syntax through the shared first-class pipeline
  `mermaid`: supports the following diagram families:
    - `sequenceDiagram`: participants/actors, message arrows, `Note over|left of|right of`, `activate`/`deactivate`/`destroy`, `autonumber`, `title`, `%%` comments, all group blocks (`alt`/`else`/`end`, `opt`, `loop`, `par`/`and`, `critical`/`option`, `break`, `rect rgb(...)`, `box`), `create`/`destroy`, `link` (collapsed to benign comment); unknown constructs emit deterministic `E_MERMAID_*` diagnostics
    - `flowchart TD|LR|...` / `graph TD|LR|...`: nodes with bracket/brace/paren labels, `-->` and `-->|label|` edges, subgraph blocks — adapted to PlantUML component-style
    - `classDiagram`: class declarations with `{ members }` blocks and `ClassName : member` lines, inheritance/association relations — adapted to PlantUML class diagram
    - `stateDiagram` / `stateDiagram-v2`: `[*]` pseudo-states, `-->` transitions — adapted to PlantUML state diagram
    - `erDiagram`: entity declarations and `||--o{` cardinality relations — adapted to PlantUML class-style diagram
    Unsupported diagram families (e.g. `pie`, `gitDiagram`) emit a deterministic `E_MERMAID_FAMILY_UNSUPPORTED` diagnostic
  `picouml`: canonical first-class language surface; explicit frontend selection routes through the shared deterministic baseline path
- `--compat strict|extended` sets semantic compatibility policy (default `strict`)
  `strict`: no ambient include-root fallback; stdin `!include` requires explicit `--include-root`
  `extended`: when `--include-root` is omitted, stdin `!include` falls back to current working directory
- `--determinism strict|full` sets determinism policy (default `strict`)
- `--include-root DIR` resolves `!include` when reading stdin

Outputs:
- single diagram from file writes `<input-stem>.svg`
- with `--format png`, single diagram from file writes `<input-stem>.png`
- single diagram from stdin writes SVG to stdout
- with `--format png`, single diagram from stdin writes PNG bytes to stdout
- multipage file inputs (`newpage`) write numbered files (`<stem>-1.svg`, `<stem>-2.svg`, ...)
- with `--format png`, multipage file inputs write numbered PNG files (`<stem>-1.png`, `<stem>-2.png`, ...)
- multipage stdin inputs require `--multi`; with `--multi`, stdout is a deterministic JSON array of `{name, svg}`
- multipage stdin + `--multi` is only supported for SVG output
- `ignore newpage` collapses multipage splits and keeps single-output behavior
- multi diagram from stdin + `--multi` writes JSON array to stdout
  markdown stdin naming is deterministic: `snippet-<n>.svg` (or `snippet-<n>-<page>.svg` for multipage fences)
- markdown file outputs with `--multi` are deterministic snippet files:
  `<markdown-stem>_snippet_<n>.svg` (or `<markdown-stem>_snippet_<n>-<page>.svg` for multipage fences)
- `--output PATH` writes to that path for single diagrams, and numbered paths for multi
- lint/check batch mode always emits a lint summary report on `stdout`
  `human`: single summary line + failed file lines
  `json`: `{"schema":"puml.lint_report","schema_version":1,"summary":...,"files":[...]}`
- multi-file writes are transactional: failures do not leave partially updated numbered outputs

Exit codes:
- `0` success
- `1` validation or usage failure
- `2` I/O failure
- `3` internal failure

Diagnostics:
- source warnings/errors include `line`/`column` and caret snippets when source spans exist
- unsupported `skinparam` keys and `!theme` emit deterministic non-fatal warnings on `stderr`
- `--diagnostics json` emits `{"schema":"puml.diagnostics","schema_version":1,"diagnostics":[...]}` with stable fields:
  `code`, `severity`, `message`, `span`, `line`, `column`, `snippet`, `caret`
  lint mode (`--check` + lint inputs/globs) adds optional `file` per diagnostic entry and emits one aggregated JSON payload on `stderr`
- stream contract:
  `--check`/render/`--dump` payload outputs remain on `stdout`; diagnostics (human or json) are emitted on `stderr`
  lint/check batch mode keeps the same diagnostics behavior (`stderr`) and writes lint summary reports to `stdout`

## Benchmarks And Gates

Commands:

```bash
# full benchmark refresh (records trend artifacts)
./scripts/bench.sh

# quick profile
./scripts/bench.sh --quick

# enforce perf + binary-size gates
./scripts/bench.sh --enforce-gates
./scripts/bench.sh --quick --enforce-gates

# update mode baselines after explicit approval
./scripts/bench.sh --update-baseline
./scripts/bench.sh --quick --update-baseline
```

Gate thresholds:
- `full` (default): scenario mean `<= 250ms`, regression vs `docs/benchmarks/baseline_full.json` `<= 10%` with absolute delta floor `> 20ms`, binary size `<= 2,000,000` bytes
- `quick`: scenario mean `<= 350ms`, regression vs `docs/benchmarks/baseline_quick.json` `<= 20%` with absolute delta floor `> 30ms`, binary size `<= 2,500,000` bytes
- If the baseline file for the active mode is missing, regression checks are skipped and absolute + binary gates still apply.

Artifacts:
- raw run: `docs/benchmarks/latest.{md,csv,json}`
- deterministic trend report: `docs/benchmarks/latest_trend.{md,json}`
- mode baselines: `docs/benchmarks/baseline_{full,quick}.json`
- no-Java oracle placeholder baseline: `docs/benchmarks/parity_latest.json`
- differential oracle smoke report: `docs/benchmarks/oracle_smoke_latest.json`

Differential oracle smoke command:

```bash
python3 ./scripts/differential_oracle_smoke.py --quick --strict --output docs/benchmarks/oracle_smoke_latest.json
```

## Feature Matrix

### Diagram Families

| Area | Status | Notes |
|---|---|---|
| Sequence diagrams (`@startuml`) | Supported | End-to-end parser/normalize/layout/render path with full parity for participants, arrows, notes, groups, lifecycle, metadata, and skinparam subset. |
| Class diagrams | Supported | Declarations, relations, fields/methods, visibility, stereotypes, packages/namespaces, notes, generics, association classes, lollipop notation, hide/show. |
| Object diagrams | Supported | Object instance nodes, field-value lists, map/associative-array forms, and object links. |
| Use-case diagrams | Supported | Actor declarations/styles, parenthesized `usecase (Name) as Alias`, use-case descriptions, packages/boundaries, include/extend/generalization semantics, notes, stereotypes, and direction controls. |
| Component diagrams | Supported | `component`/`interface`/`port` declarations, dependencies, packages, and notation mode baseline. |
| Deployment diagrams | Supported | `node`/`artifact`/`cloud`/`frame`/`storage`/`database`/`package`/`folder`/`file`/`card`/`rectangle` declarations, links, and nesting. |
| State diagrams | Supported | `state` declarations, `[*]` initial/final markers, transitions with guards, composite/history/fork-join forms. |
| Activity diagrams (new style) | Supported | `start`/`stop`/`end`, `:action;`, `if (cond) then (yes)`/`else`/`endif`, `while`/`endwhile`, `repeat`/`repeat while`, `fork`/`fork again`/`end fork`, `backward`, `partition`/swimlane constructs. |
| Timing diagrams | Supported | `concise`/`robust`/`clock`/`binary` signal declarations, `@<time>` instants, and `signal is state` transitions. |
| Salt / wireframe (`@startsalt`) | Supported | Widget/grid/menu/tab/tree/table primitives, nested structures, scrolling markers, and metadata blocks. |
| MindMap (`@startmindmap`) | Supported | Hierarchical OrgMode-style tree, directional controls, boxless markers, color/style hooks, deterministic layout. |
| WBS (`@startwbs`) | Supported | Work-breakdown structure trees with orientation, style, and deterministic geometry. |
| Gantt (`@startgantt`) | Supported | Task/milestone declarations, starts/ends/requires constraints, project date axis, closed weekday calendar notes, resource lanes, deterministic SVG timeline. |
| Chronology (`@startchronology`) | Supported | `happens on` event statements, timestamp placement, deterministic timeline render. |
| JSON family (`@startjson`) | Supported | Parses body as JSON via `serde_json`; flattens object/array/value tree into deterministic indented SVG node tree (falls back to raw line list on parse error). |
| YAML family (`@startyaml`) | Supported | Indentation-based two-space mapping/sequence tree; rendered as a deterministic indented SVG node tree. |
| nwdiag (`@startnwdiag`) | Supported | `network` blocks with `address` and `Node [address = "..."]` entries; horizontal swimlanes per network with deterministic node ordering. |
| Archimate (`@startarchimate`) | Supported | `archimate "Name" as alias <<layer>>` declarations, relation macros (`Rel_Association`, `Rel_Realization`, `Rel_Serving`, `Rel_Composition`, `Rel_Aggregation`, `Rel_Used_By`, `Rel_Flow`), layered strategy/business/application/technology/motivation swimlanes. |
| Regex diagrams (`@startregex`) | Supported | Parses regex literals (`a`, `[abc]`, `a*`, `a+`, `a?`, `\|`, `(...)`, `\d`, `.`, anchors) into a deterministic railroad-style SVG; unsupported quantifiers emit deterministic warnings. |
| EBNF diagrams (`@startebnf`) | Supported | Parses rules `name = body ;` with terminals, non-terminals, `\|`, `(...)`, `[...]`, `{...}`, `*`, `+`, `?` into a deterministic railroad SVG. |
| Math / LaTeX (`@startmath` / `@startlatex`) | Supported | Best-effort LaTeX SVG renderer: handles `\sum`, `\int`, `\prod`, `\frac{a}{b}`, `\sqrt{x}`, Greek letters (`\alpha`…`\omega`, `\infty`), sub/sup scripts via tspan baseline shifts and nested scaling. |
| SDL diagrams (`@startsdl`) | Supported | Parses `state Name` declarations and `from -> to : signal` transitions; renders SDL-style rounded-corner rectangles with labeled arrow transitions in a 2-column grid. |
| Ditaa diagrams (`@startditaa`) | Supported | Corner-detection parser finds `+...+` rectangles from the ASCII grid, renders them as SVG `<rect>` with extracted inner text, and converts `--->` connector runs to SVG lines with arrowheads. |
| Chart diagrams (`@startchart`) | Supported | Parses `bar`/`line`/`pie` subtype plus `"label" value` rows; renders bar columns, line plots, or labeled pie slices with deterministic palette colors. |

### Sequence Diagram Primitives

| Area | Status | Notes |
|---|---|---|
| `@startuml` / `@enduml` blocks | Supported | Also accepts plain single-diagram text input. |
| Participants + aliases | Supported | `participant`, `actor`, `boundary`, `control`, `entity`, `database`, `collections`, `queue`; PlantUML `order` placement hints such as `participant First order 10`. |
| Messages + arrows | Supported | `->`, `-->`, `->>`, `-->>`, `<-`, `<--`, `->x`, `-\`, `-\\`, `-/`, `-//`, `->o`, `<->`, `<-->`, bracketed PlantUML arrow color/style decorations such as `-[#red,dashed]>` (normalized to the portable arrow core), and synchronous/async forms. |
| Virtual endpoints | Supported | `[`, `]` incoming/outgoing messages; `[*]`, found/lost directionality semantics. |
| Notes | Supported | `note left/right/over`, `hnote`, `rnote`, across/alignment behavior; multi-line `note ... end note`; `note over A, B`. |
| Groups / fragments | Supported | `alt`/`else`, `opt`, `loop`, `par`, `critical`, `break`, `group`, `box`, `ref` (single- and multi-line `ref over A, B`), `end`; mis-nested forms produce deterministic errors. |
| Separators + dividers | Supported | `== separator ==`, `...`, `||`, `newpage`. |
| `hide unlinked` | Supported (warning only) | Parsed and recorded as a `hideUnlinked` hint; not yet a layout filter — emits a deterministic note. |
| Lifecycle / control | Supported | `activate`, `deactivate`, `create`, `destroy`, `return`, `autonumber` (start/stop/resume/format/step). |
| Metadata | Supported | `title`, `header`, `footer`, `caption`, `legend`, `hide footbox`, `show footbox`. |
| `skinparam` sequence subset | Supported | `maxmessagesize`, `footbox`/`sequenceFootbox`, `ArrowColor`/`SequenceArrowColor`, `SequenceLifeLineBorderColor`, `ParticipantBackgroundColor`, `ParticipantBorderColor`, `NoteBackgroundColor`, `NoteBorderColor`, `GroupBackgroundColor`, `GroupBorderColor` (all support `Sequence...` alias prefix). |
| Other `skinparam` keys | Accepted with warning | Deterministic `W_SKINPARAM_UNSUPPORTED`/`W_SKINPARAM_UNSUPPORTED_VALUE`; continues execution. |

### Preprocessor

| Area | Status | Notes |
|---|---|---|
| `!include` | Supported | Relative file includes, cycle/depth guards, root confinement. |
| `!include_many` | Supported | `*`/`?` glob expansion with alphabetical match order. |
| `!include_once` | Supported | Deduplication via canonical path. |
| `!includesub` | Supported | Extracts `!startsub … !endsub` named blocks. |
| `!includeurl` / `!include http(s)://…` | Rejected (deterministic) | Emits `E_INCLUDE_URL_UNSUPPORTED`; URL fetching would break determinism. |
| `!define` / `!undef` | Supported | Simple token substitution before normalizer. |
| `!if` / `!elseif` / `!else` / `!endif` | Supported | Deterministic conditional evaluation: `defined()`, `==`, `!=`, numeric/bool literals; compound `&&`/`||` and word `and`/`or` at top level (short-circuit; respects parens depth and quoted strings). |
| `!ifdef` / `!ifndef` | Supported | Defined/undefined guards. |
| `!while` / `!endwhile` | Supported (bounded) | Bounded iterations; exceeding limit emits `E_PREPROC_WHILE_LIMIT`. |
| `!foreach` / `!endfor` | Supported | `!foreach $var in v1, v2, v3` (or `$var in $listvar`) iterates over comma-separated items; nested foreach OK; restores prior `$var` on exit. |
| `!function` / `!procedure` / `!return` | Supported | User-defined functions and procedures with default/keyword/unquoted args; `!return` for early exit. |
| Preprocessor builtins | Supported | `%strlen`, `%size` (string length or JSON/list/map cardinality), `%strpos`, `%substr`, `%splitstr`/`%splitstr_regex`/`%split`, `%join`, `%list`/`%array`, `%list_get`, `%list_add`, `%list_remove`, `%list_sort`, `%list_contains`, `%list_size`/`%array_size`, `%map`/`%dict`, `%map_get`, `%map_put`, `%map_remove`, `%map_merge`, `%map_contains_key`, `%map_size`, `%get`, `%set`/`%put`, `%remove`, `%keys`/`%values` plus `%map_keys`/`%map_values`, `%trim`/`%ltrim`/`%rtrim`, `%replace`, `%startswith`/`%endswith`/`%contains`, `%intval`, `%str`/`%stringify`, `%quote`/`%unquote`, `%boolval`, `%true`/`%false`/`%not`, `%upper`/`%lower`, `%chr`/`%ord`, `%dec2hex`/`%hex2dec`, `%dirpath`/`%filename`/`%filenameroot`, `%get_json_attribute`/`%json_key_exists`, `%json_set`, `%json_remove`, `%json_merge`, `%json_type`, `%json_is_valid`, `%false_then_true`/`%true_then_false`, `%invoke_procedure`, `%feature`, `%variable_exists`, `%function_exists`, `%newline`, `%retrieve_procedure_return`. Time/env-sensitive builtins (`%date`, `%getenv`) return empty, random-sensitive builtins (`%random`, `%rand`) return `0`, and local/remote IO helpers such as `%load_json` reject with `E_PREPROC_UNSAFE_BUILTIN` for byte-stable output. |
| JSON variable assignment | Supported | `!$var = { ... }` JSON-literal assignment parsed before normalization; native dot/bracket projection supports `$var.key[0].name`. |
| `!import` / stdlib | Supported | Deterministic stdlib catalog resolution; unknown modules emit `E_IMPORT_UNSUPPORTED`. |
| `!theme` | Supported | Local theme catalog semantics; unknown themes emit a deterministic warning and continue. |
| `!assert` / `!log` / `!dump_memory` | Supported | Deterministic diagnostic forms; `!assert` failure emits `E_PREPROC_ASSERT_FAIL`. |
| Inline JSON/YAML projection partial rows | Supported | Object-like `json $alias { ... }` / `yaml $alias { ... }` blocks accept partial key rows and quoted braces without leaking into UML rendering; covered by `json_projection_accepts_partial_rows_and_quoted_braces` and `yaml_projection_accepts_partial_rows_and_quoted_braces`. |

### Frontends

| Area | Status | Notes |
|---|---|---|
| PlantUML frontend | Supported | First-class 1:1 compatibility target across all implemented diagram families. |
| Mermaid frontend (`sequenceDiagram`) | Supported | Participants/actors, message arrows, `Note over\|left of\|right of`, `activate`/`deactivate`/`destroy`, `autonumber`, `title`, `%%` comments, `alt`/`else`/`end`, `opt`, `loop`, `par`/`and`, `critical`/`option`, `break`, `rect rgb(...)`, `box "label"`, `create [participant] X`, `destroy X`, `link X: name @ url` (collapsed to benign comment). Unknown constructs emit deterministic `E_MERMAID_*` diagnostics. |
| PicoUML frontend | Supported (baseline) | Canonical first-class language surface; baseline canonical routing implemented. Full spec: `docs/specs/picouml-language.md`. |

### CLI Flags

| Flag | Status | Notes |
|---|---|---|
| `--check` | Supported | Parse + normalize only; no render output. |
| `--dump ast\|model\|scene` | Supported | Emits JSON pipeline dump to stdout. |
| `--multi` | Supported | Required for inputs with multiple `@startuml`/`@enduml` blocks. |
| `--from-markdown` | Supported | Extracts fenced diagram blocks from Markdown; auto-enabled for `.md`/`.markdown`/`.mdown`. |
| `--diagnostics human\|json` | Supported | Controls diagnostics output format; JSON form includes stable `code`/`severity`/`span`/`snippet`/`caret` fields. |
| `--include-root DIR` | Supported | Resolves `!include` for stdin input. |
| `--output PATH` | Supported | Writes to specified path for single diagrams, numbered paths for multi. |
| `--overwrite` | Supported (no-op) | Outputs are always overwritten; flag accepted for PlantUML CLI compat. |
| `--fail-on-warn` | Supported | Exits 1 if any warnings are emitted. |
| `--charset UTF-8` | Supported | Only UTF-8 accepted; other charsets emit `E_CHARSET_UNSUPPORTED`. |
| `--duration` | Supported | Prints elapsed wall time to stderr. |
| `--quiet` / `-q` | Supported | Suppresses non-error stderr output. |
| `--verbose` / `-v` | Supported | Emits per-stage parse/normalize/render timings. |
| `--format svg\|png` | Supported | `svg` writes deterministic vector output; `png` rasterizes the same deterministic SVG scene. |
| `--dialect auto\|plantuml\|mermaid\|picouml` | Supported | Selects frontend input dialect. |
| `--compat strict\|extended` | Supported | Controls stdin include-root fallback policy. |
| `--determinism strict\|full` | Supported | Controls determinism policy level. |
| `--lint-input INPUT` | Supported | Adds repeatable check/lint inputs (check mode only). |
| `--lint-glob GLOB` | Supported | Adds repeatable glob-expanded check/lint inputs (check mode only). |
| `--lint-report human\|json` | Supported | Emits lint summary report format. |
| `--stdrpt` | Supported | Single-line tab-separated diagnostics `<severity>\t<code>\t<file>:<line>:<col>\t<message>`. |

## LSP Baseline

`puml-lsp` includes a deterministic baseline for:
- diagnostics published on `didOpen`/`didChange`/`didSave` using the same `parse -> normalize` pipeline as the CLI
- completion for top-level sequence primitives plus arrow/lifecycle tokens
- hover documentation for directives and arrow forms

Contract notes:
- completion and hover do not render diagrams
- diagnostics preserve structured `code` when available from core diagnostics

## Autonomy Harness

Codex + Claude autonomous repo engineering entrypoints:

```bash
# harness-only (fastest confidence for agent-pack + MCP + parity invariants)
./scripts/harness-check.sh --quick

# full harness lane (includes docs gallery drift fail-on-drift checks)
./scripts/harness-check.sh

# VS Code scaffold smoke (LSP contract + extension build)
./scripts/vscode-smoke.sh

# ecosystem rollout closure check (LSP/VSCode/Studio/plugin contracts)
./scripts/ecosystem-rollout-check.sh --quick
./scripts/ecosystem-rollout-check.sh

# full autonomous quality chain
./scripts/autonomy-check.sh --quick
./scripts/autonomy-check.sh
```

Dry-run planning commands:

```bash
./scripts/harness-check.sh --dry
./scripts/autonomy-check.sh --dry
```

Docs gallery refresh commands (source-linked `.puml` + fenced snippets):

```bash
for f in docs/examples/*.puml; do cargo run -- "$f"; done
for f in docs/examples/*/*.puml; do [ -f "$f" ] && cargo run -- "$f"; done
cargo run -- --from-markdown docs/examples/README.md --output docs/examples/README_snippet_1.svg
cargo run -- --from-markdown --multi docs/examples/sequence/README.md
python3 ./scripts/parity_harness.py --fail-on-doc-drift --quiet
```

## Docs

- Developer flow: [`docs/codex-workflow.md`](docs/codex-workflow.md)
- Command cookbook: [`docs/autonomous-workflow-cookbook.md`](docs/autonomous-workflow-cookbook.md)
- Benchmark workflow: [`docs/benchmarks/README.md`](docs/benchmarks/README.md)
- PlantUML frontend conformance matrix: [`docs/plantuml_frontend_conformance_matrix.md`](docs/plantuml_frontend_conformance_matrix.md)
- Contribution guide: [`docs/contributing.md`](docs/contributing.md)
- Troubleshooting guide: [`docs/troubleshooting.md`](docs/troubleshooting.md)
- VS Code extension scaffold: [`extensions/vscode/README.md`](extensions/vscode/README.md)

## License

MIT. See [LICENSE](./LICENSE).
