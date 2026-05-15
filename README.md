# puml

Fast, deterministic sequence-diagram rendering from PlantUML-style text to SVG, with strict validation and scriptable CLI modes.

![version](https://img.shields.io/badge/version-0.1.0-0ea5e9)
![rust](https://img.shields.io/badge/rust-2021-f97316)
![scope](https://img.shields.io/badge/scope-sequence--only-14b8a6)
![license](https://img.shields.io/badge/license-MIT-22c55e)

## Why Sequence-Only

`puml` intentionally supports sequence diagrams only. Non-sequence families (state/class/etc.) are rejected so the parser, validator, layout, and SVG output stay predictable and testable.

Compatibility statement:
- Supports a focused subset of PlantUML-style **sequence** syntax (see feature matrix below).
- Does not claim full PlantUML language compatibility.

## Install And Dev

```bash
# clone + enter
git clone <your-fork-or-repo-url>
cd puml

# one-time dev setup
./scripts/setup.sh

# fast local loop (fmt + clippy + test)
./scripts/dev.sh

# full quality gate
./scripts/check-all.sh
```

## CLI Usage (Explicit Modes)

```bash
# help
cargo run -- --help

# 1) FILE INPUT -> renders <input-stem>.svg
cargo run -- tests/fixtures/basic/hello.puml

# 2) STDIN INPUT (explicit '-') -> render SVG to stdout
cat tests/fixtures/basic/hello.puml | cargo run -- -

# 3) STDIN INPUT (implicit, no INPUT arg) -> render SVG to stdout
cat tests/fixtures/basic/hello.puml | cargo run --

# check-only mode (parse + normalize, no render output)
cargo run -- --check tests/fixtures/basic/hello.puml
cat tests/fixtures/basic/hello.puml | cargo run -- --check -

# dump pipeline JSON
cargo run -- --dump ast tests/fixtures/basic/hello.puml
cargo run -- --dump model tests/fixtures/basic/hello.puml
cargo run -- --dump scene tests/fixtures/basic/hello.puml

# multi-diagram mode (must be explicit)
cargo run -- --multi tests/fixtures/structure/multi_three.puml
cat tests/fixtures/structure/multi_three.puml | cargo run -- --multi -

# markdown fenced extraction mode
cargo run -- --from-markdown --check docs/sequence-notes.md

# machine-readable diagnostics
cargo run -- --check --diagnostics json tests/fixtures/invalid_single.puml

# frontend + mode controls
cargo run -- --dialect auto --compat strict --determinism strict tests/fixtures/basic/hello.puml
cargo run -- --dialect plantuml --check tests/fixtures/basic/hello.puml

# stdin + include support
cat tests/fixtures/include/include_ok_child.puml | cargo run -- --check --include-root ./tests/fixtures/include -
```

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
- `--check` parses + normalizes only
- `--dump ast|model|scene` emits JSON
- `--multi` permits multiple diagrams
- `--from-markdown` treats input as markdown and only extracts fenced diagram blocks
- `--diagnostics human|json` controls diagnostics output format (default `human`)
- `--dialect auto|plantuml|mermaid|picouml` selects frontend input dialect (default `auto`; currently routes to PlantUML path)
- `--compat strict|extended` sets semantic compatibility policy (default `strict`)
  `strict`: no ambient include-root fallback; stdin `!include` requires explicit `--include-root`
  `extended`: when `--include-root` is omitted, stdin `!include` falls back to current working directory
- `--determinism strict|full` sets determinism policy (default `strict`)
- `--include-root DIR` resolves `!include` when reading stdin

Outputs:
- single diagram from file writes `<input-stem>.svg`
- single diagram from stdin writes SVG to stdout
- multi diagram from stdin + `--multi` writes JSON array to stdout
- `--output PATH` writes to that path for single diagrams, and numbered paths for multi

Exit codes:
- `0` success
- `1` validation or usage failure
- `2` I/O failure
- `3` internal failure

Diagnostics:
- source warnings/errors include `line`/`column` and caret snippets when source spans exist
- unsupported `skinparam` keys and `!theme` emit deterministic non-fatal warnings on `stderr`
- `--diagnostics json` emits `{"diagnostics":[...]}` with stable fields: `severity`, `message`, `span`, `line`, `column`, `snippet`, `caret`

## Benchmarks (Latest Recorded)

Source: `docs/benchmarks/latest.md` generated on **2026-05-15** (UTC timestamp `2026-05-15T07:44:44Z`).

| Scenario | Mean (ms) | Stddev (ms) | Runs | Tool |
|---|---:|---:|---:|---|
| `render_hello` | 212.000 | 74.135 | 5 | `time` |
| `check_hello` | 180.000 | 0.000 | 5 | `time` |
| `dump_model` | 176.000 | 4.899 | 5 | `time` |
| `stdin_single` | 180.000 | 0.000 | 5 | `time` |
| `stdin_multi` | 182.000 | 4.000 | 5 | `time` |

## Feature Matrix

| Area | Status | Notes |
|---|---|---|
| Sequence diagrams | Supported | Non-sequence families are rejected. |
| `@startuml` / `@enduml` blocks | Supported | Also accepts plain single-diagram text input. |
| Participants + aliases | Supported | `participant`, `actor`, `boundary`, `control`, `entity`, `database`, `collections`. |
| Messages + common arrows | Supported | Includes forms like `->`, `-->`, `<-` with optional labels. |
| Notes, groups, separators | Supported | Includes `alt`, `else`, `opt`, `loop`, `par`, `critical`, `break`, `group`, `end`, plus `...`, `||`, `newpage`. |
| Lifecycle/control statements | Supported | `activate`, `deactivate`, `create`, `destroy`, `return`, `autonumber`. |
| Metadata statements | Supported | `title`, `header`, `footer`, `caption`, `legend`, `hide footbox`, `show footbox`. |
| `skinparam` sequence styling subset | Supported | `maxmessagesize`, `footbox`/`sequenceFootbox`, `ArrowColor`, `SequenceLifeLineBorderColor`, `ParticipantBackgroundColor`, `ParticipantBorderColor`, `NoteBackgroundColor`, `NoteBorderColor`, `GroupBackgroundColor`, `GroupBorderColor`. |
| Other `skinparam` keys | Accepted with warning | Deterministic `W_SKINPARAM_UNSUPPORTED`/`W_SKINPARAM_UNSUPPORTED_VALUE` warning; continues execution. |
| `!include`, `!define`, `!undef` | Supported (scoped) | Relative includes, simple define/undef substitution, cycle/depth guards. |
| Multi-diagram input | Guarded support | Requires explicit `--multi`. |

## Autonomy Harness

Codex + Claude autonomous repo engineering entrypoints:

```bash
# harness-only (fastest confidence for agent-pack + MCP + parity invariants)
./scripts/harness-check.sh --quick

# full autonomous quality chain
./scripts/autonomy-check.sh --quick
./scripts/autonomy-check.sh
```

Dry-run planning commands:

```bash
./scripts/harness-check.sh --dry
./scripts/autonomy-check.sh --dry
```

## Docs

- Developer flow: [`docs/codex-workflow.md`](docs/codex-workflow.md)
- Benchmark workflow: [`docs/benchmarks/README.md`](docs/benchmarks/README.md)
- Contribution guide: [`docs/contributing.md`](docs/contributing.md)
- Troubleshooting guide: [`docs/troubleshooting.md`](docs/troubleshooting.md)

## License

MIT. See [LICENSE](./LICENSE).
