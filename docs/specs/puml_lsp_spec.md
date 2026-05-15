# `puml-lsp` Specification

A real language server for native PlantUML-compatible sequence diagrams: parse, diagnose, complete, rename, preview, export, and refactor with full `puml` feature parity.

This is not an editor helper. This is the language intelligence layer for the whole product. The CLI, browser studio, Codex plugin, Claude plugin, MCP tools, and every future extension must be able to rely on this server and the shared language-service core.

## Product position

`puml-lsp` turns `.puml` files into first-class source code.

The promise:

- instant diagnostics while typing
- syntax highlighting from the real parser, not regex themes
- completions for every sequence primitive
- hover docs for every directive and arrow
- go-to-definition for aliases, includes, and participant declarations
- rename participant/alias safely across the diagram
- code actions that repair broken diagrams
- preview/export commands backed by the same renderer as the CLI
- deterministic behavior across VS Code, Cursor, Windsurf, Zed, Neovim, Helix, JetBrains, Claude Code, Codex, and the browser studio

The server speaks Language Server Protocol 3.17 over stdio. It is editor-agnostic. Editor integrations are disposable shells.

## Non-negotiables

- Full feature parity with the `puml` sequence-diagram language spec.
- No shadow parser.
- No regex-only syntax model.
- No editor-specific behavior in the server.
- No Node runtime in the server.
- No async runtime unless a measured protocol requirement forces it.
- No network access.
- No external process execution except invoking the local `puml` binary for explicitly requested export paths, and even that should disappear once export is in-process.
- No Graphviz.
- No JVM.
- No SVG library.
- No layout library.
- All diagnostics are structured first, then rendered into LSP diagnostics.
- Every source-attached diagnostic includes range, line, column, severity, code, and human message.
- Every request is version-aware. Stale responses are dropped.
- 90% line coverage minimum.
- Default clippy warnings are errors.

## Architecture religion

Use a compiler-service architecture and treat it as law.

The pipeline is:

```text
source text
  -> source map
  -> parser
  -> AST
  -> semantic normalization
  -> indexed semantic model
  -> language service queries
  -> LSP protocol adapter
```

Rules:

- `puml-core` owns parsing, normalization, diagnostics, layout, scene, and rendering.
- `puml-language` owns language-intelligence queries over parsed/indexed documents.
- `puml-lsp` is a protocol adapter only.
- The LSP server does not parse with ad-hoc logic.
- The LSP server does not inspect syntax by splitting strings unless it is delegating to a lexer/token cursor owned by `puml-language`.
- The language service does not know what editor is calling it.
- The CLI and LSP share the same diagnostics.
- The browser studio and LSP share the same semantic-token model.
- Every opened document is represented as an immutable versioned snapshot.
- Every workspace index is rebuilt by applying document snapshot changes, never by mutating half-valid state.
- Every public method is deterministic for the same source, workspace, and config.
- If a feature cannot be expressed cleanly through this pipeline, the feature design is wrong.

## Workspace structure

The main repository becomes a workspace:

```text
Cargo.toml

crates/
  puml/                         # CLI binary
  puml-core/                    # parser, AST, model, layout, scene, SVG render
  puml-language/                # completion, hover, refs, rename, formatting, symbols
  puml-lsp/                     # stdio LSP server
  puml-wasm/                    # browser-facing language/render bindings
  puml-mcp/                     # agent tool server, specified separately

editors/
  vscode/                       # thin extension shell
  zed/                          # optional extension shell
  helix/                        # language config
  neovim/                       # setup snippet and tests

tests/
  fixtures/
  lsp-transcripts/
  snapshots/
```

`puml-lsp` binary name:

```text
puml-lsp
```

Language ID:

```text
puml
```

File extensions:

```text
.puml
.plantuml
.iuml
.pu
```

The server must treat all supported extensions identically.

## Dependencies

Production dependencies should stay boring and explicit:

- `lsp-server`
- `lsp-types`
- `serde`
- `serde_json`
- `anyhow`
- `thiserror` only if diagnostic error composition justifies it
- shared `puml-core`
- shared `puml-language`

Forbidden in the LSP server:

- `tokio`
- `tower-lsp`
- editor-specific SDKs
- runtime plugin systems
- shell command orchestration libraries
- file watcher dependencies unless stdlib + LSP workspace notifications are insufficient after measurement

The server should look closer to rust-analyzer's protocol adapter style than an async web service.

## Source of truth

All syntax support comes from the same sequence-diagram grammar used by the CLI.

The LSP must understand every supported primitive:

- `@startuml` / `@enduml` blocks
- named diagram blocks
- comments
- quoted and unquoted text
- multiline text
- escaped strings
- inline formatting subset
- explicit participants
- implicit participants
- participant aliases
- participant kinds
- participant ordering
- participant colors
- participant boxes
- message arrows
- self messages
- found messages
- lost messages
- bidirectional messages
- async arrows
- dashed arrows
- colored arrows
- arrow modifiers
- lifecycle directives
- activation and deactivation
- create and destroy
- return inference
- notes
- `hnote`
- `rnote`
- refs
- groups
- `alt`
- `else`
- `opt`
- `loop`
- `par`
- `break`
- `critical`
- generic `group`
- dividers
- delays
- spacers
- autonumber
- title
- header
- footer
- caption
- legend
- footbox visibility
- supported `skinparam` sequence styling
- includes
- simple `!define` / `!undef` substitution
- `newpage`
- sequence-only rejection for class/activity/state/component/deployment diagrams

If the renderer supports a primitive, the LSP supports parsing, diagnostics, tokens, completion, hover, and tests for that primitive.

## LSP lifecycle

The server supports:

- `initialize`
- `initialized`
- `shutdown`
- `exit`
- `$/cancelRequest`
- `$/setTrace`
- `window/logMessage`
- `window/showMessage`
- `workspace/configuration`
- `workspace/didChangeConfiguration`
- `workspace/didChangeWorkspaceFolders`
- `workspace/didChangeWatchedFiles`
- `textDocument/didOpen`
- `textDocument/didChange`
- `textDocument/didSave`
- `textDocument/didClose`

Text synchronization:

- incremental sync supported
- full sync accepted as fallback
- CRLF and LF preserved in source maps
- diagnostics recomputed after every document version change
- no diagnostics published for stale document versions
- closed documents remain indexed if they exist on disk and are inside a workspace folder

## Diagnostics

Diagnostics are the spine of the LSP.

Diagnostic categories:

```text
parse
semantic
include
lifecycle
style
render
non_sequence
internal
```

Diagnostic severities:

- Error: source cannot produce a valid model or render.
- Warning: source renders but has ignored, unsupported, suspicious, or deprecated constructs.
- Information: useful language guidance.
- Hint: non-blocking style suggestions.

Every diagnostic has:

```rust
struct DiagnosticInfo {
    code: DiagnosticCode,
    severity: Severity,
    primary_span: Span,
    related_spans: Vec<RelatedSpan>,
    message: String,
    help: Option<String>,
    docs_key: Option<String>,
}
```

Required diagnostic examples:

- missing `@enduml`
- missing `@startuml`
- nested unsupported block
- unknown participant reference
- ambiguous participant reference
- duplicate alias
- malformed alias declaration
- malformed arrow
- unsupported arrow modifier
- invalid color
- note missing target
- note target unknown
- group missing `end`
- `else` outside `alt` / `par`
- `end` without block
- activation underflow
- return target cannot be inferred
- message to destroyed participant
- include path not found
- include cycle
- include denied outside include root
- unsupported remote include
- unsupported non-sequence diagram
- ignored skinparam
- unknown skinparam
- unescaped malformed inline formatting tag

Diagnostic quality bar:

- The message explains the defect, not the parser implementation.
- The range points to the smallest useful token.
- The help text suggests an actual fix when one exists.
- Related information points to the declaration that caused ambiguity or duplication.

## Completion

Completions are syntax-aware and context-aware.

Top-level completions:

- `@startuml`
- `@enduml`
- `title`
- `header`
- `footer`
- `caption`
- `legend`
- `participant`
- `actor`
- `boundary`
- `control`
- `entity`
- `database`
- `collections`
- `queue`
- `box`
- `end box`
- `note left of`
- `note right of`
- `note over`
- `note across`
- `hnote over`
- `rnote over`
- `ref over`
- `alt`
- `else`
- `opt`
- `loop`
- `par`
- `break`
- `critical`
- `group`
- `end`
- `activate`
- `deactivate`
- `create`
- `destroy`
- `return`
- `autoactivate on`
- `autoactivate off`
- `autonumber`
- `autonumber stop`
- `autonumber resume`
- `hide footbox`
- `show footbox`
- `skinparam sequence {}`
- `!include`
- `!define`
- `!undef`
- `newpage`
- `== divider ==`
- `... delay ...`
- `|||`

Participant-reference completions:

- participant aliases
- display names
- quoted display names when needed
- aliases preferred over display names when both exist
- completion detail shows participant kind and display name
- completion documentation shows declaration line

Arrow completions:

- `->`
- `-->`
- `<-`
- `<--`
- `->>`
- `-->>`
- `<<-`
- `<<--`
- `->x`
- `x->`
- `-x`
- `->o`
- `o->`
- `<->`
- `<-->`
- `-[#color]>`
- `-[#color,dashed]>`
- `-[#color,bold]>`
- lifecycle suffixes `++`, `--`, `**`, `!!`

Color completions:

- common named colors accepted by the parser
- recently used colors in the same document
- theme colors exposed as completion items
- `#rrggbb` snippet

Include completions:

- relative file paths
- only inside configured include roots
- only for supported extensions unless the user has configured otherwise

Skinparam completions:

- every supported sequence skinparam key
- warnings for known-but-unsupported keys
- no fake completions for unsupported diagram families

Snippet completions:

- login flow
- request/response
- async job
- database query
- alt success/error
- loop retry
- create/destroy lifecycle
- note over participants
- participant box grouping

Snippets must generate valid code that parses immediately.

## Hover

Hover is documentation, not decoration.

Hover targets:

- directives
- participant declarations
- participant references
- aliases
- arrows
- arrow modifiers
- lifecycle directives
- note directives
- group directives
- skinparams
- includes
- colors
- diagnostics

Hover content for participants includes:

- canonical ID
- display name
- alias
- kind
- declaration location
- implicit vs explicit
- lifecycle state if relevant

Hover content for arrows includes:

- arrow kind
- line style
- head style
- direction
- lifecycle modifiers
- semantic effect

Hover content for include directives includes:

- resolved path
- included block count
- cycle status if any

## Go to definition

Supported:

- participant reference -> participant declaration
- alias reference -> participant declaration
- display-name reference -> participant declaration when unambiguous
- note target -> participant declaration
- lifecycle target -> participant declaration
- include path -> included file
- style reference -> nearest style directive when applicable

Implicit participant behavior:

- If a participant was auto-created by first use, go-to-definition jumps to that first use.
- Hover marks it as implicit.
- Code action offers “Add explicit participant declaration”.

## Find references

Supported references:

- message source/target
- note anchors
- ref anchors
- lifecycle events
- participant boxes
- group spans when they imply participant coverage
- create/destroy events
- implicit first-use declarations

Reference results must distinguish declarations from uses.

## Rename

Rename is semantic and safe.

Supported:

- rename alias
- rename display name
- rename implicit participant into explicit declaration
- rename participant declaration while preserving display/alias split

Rules:

- Renaming an alias updates references that use the alias.
- Renaming a display name updates quoted declarations and unaliased references.
- If a participant has both display name and alias, rename must ask through `prepareRename` what symbol is being renamed.
- Rename refuses ambiguous references.
- Rename refuses edits that would create duplicate aliases.
- Rename preserves quoting rules.
- Rename preserves comments.
- Rename is workspace-aware for includes.

## Document symbols

The server returns a tree:

```text
DiagramBlock
  Title
  Header
  Footer
  Participants
    Participant
    Actor
    Boundary
    Control
    Entity
    Database
    Collections
    Queue
  ParticipantBoxes
  Messages
  Notes
  Groups
  Refs
  Lifecycle
  Styling
  Includes
```

Message symbols should not flood the outline by default in editor clients that support hierarchical details. They are still available as children for power users and tests.

## Workspace symbols

Workspace symbol search supports:

- diagram names
- participants
- aliases
- included files
- titles
- named refs/groups when labels exist

Results include file, range, symbol kind, and container diagram.

## Semantic tokens

Syntax highlighting comes from semantic tokens produced by `puml-language`.

Token types:

```text
keyword
operator
string
comment
number
type
class
function
variable
parameter
property
namespace
label
decorator
modifier
```

Custom token mapping through LSP token modifiers:

```text
participantDeclaration
participantReference
alias
arrow
arrowHead
arrowModifier
messageText
noteText
groupLabel
lifecycle
skinparam
include
color
diagramBoundary
unsupported
implicit
```

Required highlighting coverage:

- block delimiters
- comments
- participant kind keywords
- quoted names
- aliases
- arrows
- message labels
- note placements
- group keywords
- lifecycle directives
- style keys
- colors
- include paths
- multiline text blocks
- formatting tags

The VS Code extension may ship a coarse TextMate grammar only for pre-LSP bootstrapping. The real highlighting is semantic tokens.

## Formatting

Formatter philosophy:

- Preserve meaning.
- Normalize spacing.
- Do not rewrite prose.
- Do not reorder participants unless the user explicitly requests a code action.
- Do not move comments across semantic boundaries.
- Do not destroy unknown-but-preserved text inside supported multiline text blocks.

Formatting rules:

- one directive per line
- single space around arrows
- no space before message colon
- one space after message colon when text exists
- quoted display names preserved
- aliases preserved
- group bodies indented two spaces
- note/ref multiline bodies indented two spaces
- `skinparam sequence {}` body indented two spaces
- `box` body indented two spaces
- trailing whitespace removed
- final newline added
- CRLF preserved when the file already uses CRLF

Supported requests:

- `textDocument/formatting`
- `textDocument/rangeFormatting`
- `textDocument/onTypeFormatting` for `
`, `:`, and `end`

## Code actions

Required code actions:

- Add missing `@startuml` / `@enduml`.
- Add explicit participant declaration for implicit participant.
- Convert participant display-name references to alias references.
- Create alias for participant with long display name.
- Fix malformed arrow when there is a single obvious correction.
- Close current group with `end`.
- Close current note with `end note`.
- Close current ref with `end ref`.
- Close participant box with `end box`.
- Remove unsupported skinparam.
- Convert unsupported global skinparam to supported sequence skinparam when equivalent exists.
- Add include root config hint for denied includes.
- Replace remote include with local include stub.
- Expand `return` into explicit dashed reverse message when inference is ambiguous.
- Extract selected messages into `ref over ...`.
- Wrap selected messages in `alt`, `loop`, `opt`, or `group`.
- Convert self-message label into note right of participant when user chooses.
- Toggle footbox.
- Insert title/header/footer/caption/legend.

Every code action has a test fixture.

## Code lenses and custom commands

Standard code lenses:

- Render SVG
- Open preview
- Export diagram
- Copy SVG
- Copy Markdown image embed
- Add explicit participants
- Normalize formatting

Custom commands:

```text
puml.renderSvg
puml.openPreview
puml.export
puml.copySvg
puml.copyMarkdownImage
puml.addExplicitParticipants
puml.normalizeDocument
puml.showDiagramModel
puml.showSceneGraph
```

Commands must be implemented as request handlers over the same language-service API.

## Inlay hints

Required hints:

- display name hint for alias-only references when useful
- alias hint for quoted display-name declarations
- lifecycle depth hint for nested activations when enabled
- autonumber preview hint for message rows when autonumber is enabled
- include depth hint for included files in dump/check modes

Inlay hints are opt-in by config and off by default except alias/display hints.

## Folding ranges

Required folding:

- diagram blocks
- multiline title/header/footer/caption/legend
- notes
- refs
- participant boxes
- groups
- skinparam blocks
- include-expanded virtual blocks when supported by client

## Selection ranges

Selection ranges should expand through:

```text
token -> directive -> block branch -> group -> diagram block -> document
```

This makes refactoring and editor selections sane.

## Document links

Document links:

- local `!include` paths
- generated preview output paths when embedded in comments or supported metadata
- documentation links in hover are allowed only as trusted docs keys resolved by the client extension

No remote URL fetching.

## Color provider

Supported:

- parse color literals
- expose `textDocument/documentColor`
- expose `textDocument/colorPresentation`
- preserve original color spelling when formatting unless the user explicitly chooses a presentation

Color values appear in:

- participant declarations
- arrows
- notes
- boxes
- skinparams
- themes

## Workspace indexing

The server builds a workspace index:

```rust
WorkspaceIndex {
    documents: HashMap<Url, DocumentSnapshot>,
    diagrams: Vec<DiagramIndexEntry>,
    participants: SymbolTable,
    includes: IncludeGraph,
    diagnostics: DiagnosticStore,
}
```

Index behavior:

- Open files override disk files.
- Includes are resolved through include roots.
- Include cycles are detected globally.
- Workspace symbols are updated incrementally.
- Diagnostics propagate from included files to including files with related information.
- Index rebuilds are deterministic.
- Large workspaces avoid reparsing unchanged documents.

## Configuration

LSP configuration namespace:

```json
{
  "puml.includeRoots": ["."],
  "puml.diagnostics.enable": true,
  "puml.diagnostics.warningsForIgnoredSkinparams": true,
  "puml.format.enable": true,
  "puml.preview.theme": "default",
  "puml.preview.autoOpen": false,
  "puml.preview.debounceMs": 50,
  "puml.semanticTokens.enable": true,
  "puml.inlayHints.aliases": true,
  "puml.inlayHints.autonumber": false,
  "puml.export.defaultFormat": "svg",
  "puml.trace.server": "off"
}
```

Configuration must be schema-documented for VS Code and still work in generic clients.

## Custom LSP requests

The server exposes custom requests for clients that want richer integration.

```typescript
type RenderSvgParams = {
  textDocument: TextDocumentIdentifier;
  diagramIndex?: number;
  theme?: string;
};

type RenderSvgResult = {
  svg: string;
  width: number;
  height: number;
  diagnostics: DiagnosticInfo[];
};
```

```text
puml/renderSvg
puml/renderScene
puml/renderModel
puml/export
puml/diagramAtPosition
puml/listDiagrams
puml/listParticipants
puml/getSyntaxTree
puml/getSemanticModel
puml/getDiagnostics
```

Custom requests must not be required for baseline editor functionality.

## Editor clients

### VS Code extension

The VS Code extension is thin:

- registers language ID and extensions
- starts `puml-lsp`
- contributes configuration schema
- contributes preview webview
- contributes commands mapped to server requests
- contributes coarse TextMate grammar only as startup fallback
- contributes file icons if desired

No parser in the VS Code extension.

### Other editors

The repo includes setup docs/config for:

- Neovim
- Helix
- Zed
- Sublime LSP
- JetBrains external LSP path if feasible
- Claude Code plugin LSP config
- Codex/IDE integration notes when supported

These are integration shells, not separate implementations.

## Preview

Preview is driven by the server request `puml/renderSvg`.

Rules:

- preview uses current unsaved buffer content
- preview uses selected diagram block when cursor is inside one
- preview supports multiple pages
- preview exposes diagnostics inline
- preview never writes files unless user explicitly chooses export
- preview SVG is sanitized and has no scripts
- preview result is byte-identical to CLI render for same source/theme/config

## Export

Export commands:

- SVG
- PNG when the renderer/export stack supports it
- PDF when the renderer/export stack supports it
- `.puml` normalized source
- JSON AST
- JSON semantic model
- JSON scene graph

If PNG/PDF are not implemented in the core renderer yet, the LSP exposes them as unavailable capabilities with explicit diagnostics, not fake commands.

## Security

- No network calls.
- No remote includes.
- No shell expansion.
- Include paths must be normalized.
- Path traversal outside include roots is denied.
- Custom requests must not read arbitrary files by path unless the path belongs to the workspace or configured include root.
- SVG output must be sanitized.
- LSP logs must not dump full source text unless trace mode explicitly asks for it.
- Crash reports are local only.

## Performance contract

Latency targets on a typical developer laptop:

| Operation | Small file | 1,000-message file |
| --- | ---: | ---: |
| didOpen diagnostics | < 30 ms | < 250 ms |
| incremental edit diagnostics | < 20 ms | < 150 ms |
| completion | < 15 ms | < 30 ms |
| hover | < 10 ms | < 20 ms |
| semantic tokens full | < 25 ms | < 150 ms |
| rename prepare | < 15 ms | < 50 ms |
| rename edit build | < 30 ms | < 150 ms |
| renderSvg request | < 50 ms | < 300 ms |

Algorithmic rules:

- no quadratic scans over events in common paths
- participant lookup is indexed
- alias lookup is indexed
- include graph is cached
- semantic token generation streams from tokenized spans
- formatting does not render
- hover does not render
- completion does not render

## Test strategy

Tests define the product.

### Unit tests

For every fixture:

- parse AST snapshot
- semantic model snapshot
- token stream snapshot
- semantic tokens snapshot
- diagnostics snapshot
- completion contexts snapshot
- hover snapshot for selected positions
- formatting snapshot
- code action snapshot

### LSP transcript tests

Snapshot JSON-RPC transcripts for:

- initialize
- didOpen valid file
- didOpen invalid file
- didChange incremental edit
- diagnostics publish
- completion
- completion resolve
- hover
- definition
- references
- rename prepare
- rename execute
- document symbols
- workspace symbols
- semantic tokens full
- semantic tokens delta if implemented
- formatting
- range formatting
- folding ranges
- selection ranges
- document links
- document colors
- color presentations
- code actions
- execute command
- custom render request
- shutdown

### Integration tests

- Start `puml-lsp` as a subprocess.
- Speak JSON-RPC over stdio.
- Verify protocol correctness.
- Verify exit behavior.
- Verify no stderr spam.
- Verify invalid requests return JSON-RPC errors.
- Verify stale version responses are discarded.
- Verify multiple workspace folders.
- Verify include-root config.
- Verify file rename updates include diagnostics.

### Editor smoke tests

- VS Code extension launches server.
- Diagnostics appear.
- Completion works.
- Preview works.
- Rename works.
- Semantic tokens appear.

These smoke tests do not replace server transcript tests.

### Coverage

Minimum:

```text
90% line coverage
90% branch coverage for puml-language where practical
all diagnostic families covered
all completion contexts covered
all token types covered
all custom requests covered
```

Coverage command:

```console
cargo llvm-cov --workspace --all-features --fail-under-lines 90
```

Do not lower the threshold to pass. Add tests.

## Fixtures

The fixture tree mirrors the language spec:

```text
tests/fixtures/
  basic/
  participants/
  arrows/
  notes/
  refs/
  groups/
  lifecycle/
  styling/
  structure/
  includes/
  errors/
  lsp/
```

Every primitive has at least:

- one valid fixture
- one malformed fixture
- one completion fixture
- one hover fixture where relevant
- one semantic-token fixture
- one formatting fixture

## CLI for the server

```console
puml-lsp --stdio
puml-lsp --version
puml-lsp --check-fixture tests/fixtures/basic/hello.puml
puml-lsp --dump-capabilities
```

`--stdio` is default when no args are provided.

Exit codes:

- `0` clean shutdown
- `1` protocol/config error
- `2` IO error
- `3` internal invariant violation

## README requirements

The LSP README must include:

- one-line pitch
- editor setup
- supported features matrix
- command list
- configuration schema
- preview screenshots
- troubleshooting
- performance targets
- development commands
- coverage command

Tone:

- confident
- direct
- no “experimental” framing for core features
- no “phase 1” language

## Definition of done

`puml-lsp` is done when:

- `cargo fmt --check` passes.
- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- `cargo test --workspace` passes.
- `cargo llvm-cov --workspace --all-features --fail-under-lines 90` passes.
- LSP transcript tests cover every advertised capability.
- Every `puml` sequence primitive has diagnostics, completion, hover, semantic tokens, and tests.
- Rename works for aliases and participants.
- Formatting is deterministic.
- Preview uses unsaved buffer content.
- Include resolution is safe and tested.
- The VS Code extension is a thin client, not a parser fork.
- The server works in at least one non-VS Code client.
- The browser studio can reuse the same language-service model through WASM.
- The agent plugin can bundle the LSP server.

## Reference anchors

- Language Server Protocol: https://microsoft.github.io/language-server-protocol/
- LSP 3.17 specification: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/
