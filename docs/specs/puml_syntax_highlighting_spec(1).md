# `puml-syntax` Shared Syntax + Highlighting Specification

One syntax contract for every editor, Markdown renderer, browser surface, LSP, agent pack, and documentation pipeline.

This is the grammar layer. It is not a nice-to-have. It is the reason every surface can make `puml` feel native instead of bolted on.

## Name

Package family:

- Rust/library: `puml-syntax`
- Tree-sitter grammar: `tree-sitter-puml`
- VS Code/TextMate grammar artifact: `puml.tmLanguage.json`
- Highlight query package: `@puml/syntax`

Language ID:

```text
puml
```

Accepted aliases:

```text
plantuml
uml-sequence
puml-sequence
```

File extensions:

```text
.puml
.plantuml
.iuml
```

Code fences:

```text
```puml
```plantuml
```puml-sequence
```uml-sequence
```

`puml` is the canonical fence. The rest are compatibility aliases.

## Product position

`puml-syntax` is the shared language surface.

It powers:

- syntax highlighting in editors
- Markdown code fence highlighting
- semantic tokens from the language server
- browser editor highlighting in `puml-studio`
- VS Code, Cursor, Windsurf, Zed, Neovim, Helix, JetBrains bridges
- Codex and Claude authoring tools that need token-aware repair
- docs, examples, generated screenshots, and fixture visualization

No surface gets its own private grammar. No product owns a shadow parser. Syntax drift is treated as a product bug.

## Clarification: Tree-sitter, not Treehouse

The thing you probably meant is **Tree-sitter**.

Tree-sitter is the incremental parser/highlighting engine used by many editors and code hosts. In this stack, Tree-sitter is used for fast syntax highlighting and embedded-language support. It is not the renderer, not the semantic model, and not the source of truth for diagram correctness.

The source of truth remains `puml-core`.

## Non-negotiables

- One syntax taxonomy for all products.
- One canonical language ID: `puml`.
- TextMate grammar exists because VS Code needs immediate lexical highlighting.
- Tree-sitter grammar exists because modern editors and hosted renderers need robust incremental highlighting.
- LSP semantic tokens exist because regex highlighting cannot resolve aliases, participant identity, includes, lifecycle state, or semantic errors.
- No feature ships unless syntax coverage ships with it.
- No regex-only product surface.
- No editor-specific primitive names.
- No Markdown renderer with a different tokenizer.
- No web editor with a different tokenizer.
- No agent prompt that invents its own syntax categories.
- No syntax highlighter that performs filesystem IO.
- No highlighter expands includes.
- No highlighter runs the preprocessor.
- No highlighter executes directives.
- Highlighting must be safe on hostile input.
- 90% line coverage minimum for syntax tooling code.
- 100% primitive coverage across fixture snapshots.

## Architecture religion

Use a layered grammar contract and treat it as law.

```text
puml language spec
  -> puml-core parser and AST
  -> token taxonomy
  -> syntax fixtures
  -> TextMate grammar
  -> Tree-sitter grammar + queries
  -> LSP semantic token legend
  -> editor / Markdown / browser consumers
```

Rules:

- `puml-core` decides what is valid.
- `puml-syntax` decides how valid and partially valid source is tokenized.
- TextMate is a lexical bootstrap layer only.
- Tree-sitter is an incremental syntax layer only.
- LSP semantic tokens are the semantic layer.
- TextMate, Tree-sitter, and semantic tokens must share the same taxonomy.
- Every taxonomy change updates every grammar artifact in the same PR.
- Every grammar artifact is tested against the same fixture corpus.
- Syntax highlighting never affects parse or render output.
- Syntax tooling must tolerate malformed source better than the parser.
- Diagnostics come from `puml-core` / `puml-language`, not from the highlighter.
- If the TextMate grammar, Tree-sitter grammar, and parser disagree on a fixture, the build fails until the disagreement is explained and snapshotted.

## Repository layout

```text
crates/
  puml-core/
  puml-language/
  puml-syntax/

packages/
  puml-syntax/
    package.json
    README.md
    grammars/
      puml.tmLanguage.json
    tree-sitter-puml/
      grammar.js
      tree-sitter.json
      queries/
        highlights.scm
        injections.scm
        locals.scm
        folds.scm
        indents.scm
    themes/
      puml-light.json
      puml-dark.json
    fixtures/
      valid/
      invalid/
      markdown/
    snapshots/
```

`crates/puml-syntax` owns Rust-side token classification and semantic token translation.

`packages/puml-syntax` owns editor-consumable grammar artifacts.

## Language scopes

Canonical TextMate root scope:

```text
source.puml
```

Specific scopes:

```text
comment.line.apostrophe.puml
constant.character.escape.puml
constant.language.directive.puml
constant.numeric.puml
entity.name.participant.puml
entity.name.alias.puml
entity.name.section.puml
invalid.illegal.puml
keyword.control.group.puml
keyword.control.lifecycle.puml
keyword.control.note.puml
keyword.declaration.participant.puml
keyword.other.skinparam.puml
keyword.other.include.puml
markup.heading.title.puml
markup.raw.message.puml
punctuation.definition.comment.puml
punctuation.definition.string.begin.puml
punctuation.definition.string.end.puml
punctuation.separator.comma.puml
storage.type.participant-kind.puml
string.quoted.double.puml
string.unquoted.puml
support.constant.color.puml
support.function.arrow.puml
variable.other.participant-ref.puml
```

Do not overfit to one theme. Use standard-ish scopes where possible, but make `puml`-specific scopes available for exact styling.

## Semantic token legend

Semantic token types:

```text
namespace
class
type
variable
parameter
property
enumMember
function
method
keyword
modifier
comment
string
number
regexp
operator
decorator
label
participant
action
message
note
group
lifecycle
style
directive
alias
```

Semantic token modifiers:

```text
declaration
definition
reference
implicit
readonly
defaultLibrary
deprecated
invalid
unresolved
ambiguous
created
destroyed
activated
deactivated
self
found
lost
generated
```

Mapping rules:

- Explicit participant names: `participant declaration`.
- Participant aliases in declarations: `alias declaration`.
- Participant references in messages: `participant reference`.
- Auto-created participants: `participant implicit`.
- Unknown references: `participant unresolved invalid`.
- Ambiguous references: `participant ambiguous invalid`.
- Message labels: `message`.
- Notes: `note`.
- Group labels: `group`.
- Lifecycle commands: `lifecycle keyword`.
- Arrows: `operator` plus `self`, `found`, or `lost` when applicable.
- Unsupported non-sequence syntax: `invalid`.

## Token taxonomy

Every primitive in the sequence spec must be tokenized.

### Document block directives

Required tokens:

```text
@startuml
@enduml
@startuml name
@startuml "Display Name"
```

Rules:

- Outside-block content is tokenized as `comment` or `source.ignored.puml` depending on host support.
- Multiple blocks in one file are highlighted independently.
- Unterminated blocks produce invalid ranges without breaking the rest of the file.

### Comments

Supported:

```text
' full-line comment
Alice -> Bob: hello ' inline comment
Alice -> Bob: "don't break quoted text"
```

Rules:

- Apostrophe starts a comment outside quoted strings.
- Apostrophe inside quoted strings is text.
- Comments never create semantic tokens.

### Participants

Tokenize declarations for:

```text
participant
actor
boundary
control
entity
database
collections
queue
```

Tokenize modifiers:

```text
as
order
#color
```

Tokenize participant groups:

```text
box "Frontend"
box "Backend" #e0f2fe
end box
```

### Messages and arrows

Every arrow operator is a first-class operator token:

```text
->
-->
<-
<--
->>
-->>
<<-
<<--
->x
x->
-x
->o
o->
<->
<-->
-[#red]>
-[#ff0000]>
-[#red,dashed]>
-[#red,bold]>
```

Lifecycle suffixes attached to messages are tokenized separately:

```text
++
--
**
!!
```

Message label separator:

```text
:
```

Rules:

- Arrow color/style bracket is part of arrow syntax but exposes nested color/style tokens.
- Empty labels remain valid.
- Multiline escaped labels tokenize `\n` as escape.

### Self, found, and lost messages

Tokenize:

```text
Alice -> Alice: internal
[-> Alice: found
Alice ->]: lost
[--> Alice: dashed found
Alice -->>]: async lost
```

Rules:

- `[` and `]` are endpoint punctuation tokens.
- Found/lost semantics come from LSP semantic tokens, not TextMate.

### Lifecycle

Tokenize:

```text
activate Alice
activate Alice #ff0000
deactivate Alice
destroy Alice
create Bob
return
return value
autoactivate on
autoactivate off
```

### Notes

Tokenize inline and multiline:

```text
note left of Alice: text
note right of Alice: text
note over Alice, Bob: text
note across: text
note left of Alice
  text
end note
hnote over Alice: hex note
rnote over Bob: rounded note
```

Rules:

- `note`, `hnote`, `rnote`, `left`, `right`, `over`, `across`, `of`, `end note` are structural tokens.
- Note body is text, not parsed as directives.
- Colors after placement are color tokens.

### References

Tokenize:

```text
ref over Alice: external flow
ref over Alice, Bob
  external flow
end ref
```

### Groups

Tokenize:

```text
alt condition
else other condition
opt condition
loop condition
par branch
break condition
critical condition
group label
group label [secondary label]
end
```

Rules:

- `else` is a branch separator token.
- Nested groups must highlight cleanly even before semantic validation.

### Dividers, delays, spacers

Tokenize:

```text
== Initialization ==
...
...5 minutes later...
|||
||45||
```

### Autonumber

Tokenize:

```text
autonumber
autonumber 10
autonumber 10 5
autonumber "<b>[000]"
autonumber stop
autonumber resume
autonumber resume 100
autonumber off
```

### Titles, headers, footers, captions, legends

Tokenize:

```text
title Login flow
title
  Login flow
end title
header text
footer text
caption text
legend
  text
end legend
legend left
legend right
```

### Styling

Tokenize supported `skinparam` primitives:

```text
skinparam backgroundColor #ffffff
skinparam sequence {
  ArrowColor #111827
  LifeLineBorderColor #9ca3af
}
skinparam sequenceArrowColor #111827
```

Rules:

- Known-but-unsupported skinparams are styled as style directives and receive LSP warnings.
- Unknown garbage inside `skinparam sequence {}` gets invalid syntax highlighting only when structurally impossible.

### Includes and preprocessing

Tokenize:

```text
!include path
!include ./relative/path.puml
!define NAME value
!undef NAME
!theme name
```

Rules:

- Syntax highlighter does not load includes.
- LSP resolves includes.
- Remote include directives tokenize but receive diagnostics from semantic validation.

### Pages

Tokenize:

```text
newpage
newpage Title for next page
```

## TextMate grammar contract

TextMate grammar is required for immediate VS Code highlighting.

File:

```text
packages/puml-syntax/grammars/puml.tmLanguage.json
```

Rules:

- JSON format only.
- Deterministic key ordering.
- No generated unreadable blob unless the generator is checked in.
- Regexes must be documented with comments in the generator or adjacent source.
- It must highlight partially typed files without waiting for LSP.
- It must not attempt semantic resolution.
- It must not parse includes.
- It must not treat every unquoted word as a participant declaration.
- It must prefer useful degraded highlighting over false precision.

Required grammar tests:

- Full fixture corpus scope snapshots.
- Partial-line editing snapshots.
- Broken quote snapshots.
- Broken block snapshots.
- Markdown fenced block injection snapshots.
- Large-file smoke test with 10,000 message lines.

## Tree-sitter grammar contract

Tree-sitter grammar is required for robust cross-editor syntax.

Files:

```text
packages/puml-syntax/tree-sitter-puml/grammar.js
packages/puml-syntax/tree-sitter-puml/tree-sitter.json
packages/puml-syntax/tree-sitter-puml/queries/highlights.scm
packages/puml-syntax/tree-sitter-puml/queries/injections.scm
packages/puml-syntax/tree-sitter-puml/queries/locals.scm
packages/puml-syntax/tree-sitter-puml/queries/folds.scm
packages/puml-syntax/tree-sitter-puml/queries/indents.scm
```

Tree-sitter scope:

```text
source.puml
```

Tree-sitter file types:

```text
puml
plantuml
iuml
```

Required node families:

```text
document
ignored_text
diagram_block
start_directive
end_directive
participant_declaration
participant_kind
participant_reference
alias_declaration
message
arrow
arrow_head
arrow_line
arrow_style
message_label
note
note_body
ref_block
group_block
group_branch
lifecycle_event
divider
delay
spacer
autonumber
title_block
header
footer
caption
legend
skinparam
include_directive
preprocessor_directive
newpage
comment
string
color
number
error
```

Rules:

- Grammar accepts incomplete source.
- Grammar is optimized for incremental editing, not final semantic validation.
- Query captures map to the same taxonomy as TextMate and LSP semantic tokens.
- `locals.scm` captures participant declarations and references where possible.
- `folds.scm` captures diagram blocks, notes, refs, groups, title blocks, legends, and skinparam blocks.
- `indents.scm` captures notes, refs, groups, boxes, legends, title blocks, and skinparam blocks.

## Markdown injection contract

Markdown code fences for `puml` and compatibility aliases must highlight as `source.puml`.

Supported host surfaces:

- VS Code Markdown editor
- VS Code Markdown preview support package
- `puml-studio` embedded Markdown panes
- static docs generated by `puml-markdown`
- docs examples in README and website

Rules:

- Syntax highlighting is independent from rendering.
- A fence can be highlighted even if rendering fails.
- A rendered diagram can show diagnostics even if highlighting is unavailable.
- Code fences use the same language ID registry as file extensions.

## Error highlighting contract

Malformed source must remain navigable.

Required degraded-highlighting cases:

- missing `@enduml`
- missing `end note`
- missing `end ref`
- missing `end`
- unclosed quote
- malformed arrow
- malformed color
- malformed skinparam block
- dangling `else`
- invalid participant alias
- non-sequence syntax inside a diagram block

Rules:

- Highlight errors locally.
- Never flood the entire rest of the file as invalid unless the structure is truly unrecoverable.
- Semantic errors are marked by LSP diagnostics, not by TextMate.

## Theme contract

Ship default light and dark highlight themes for screenshots and tests.

Do not force themes in editors. Provide suggested token colors only.

Theme tokens:

```text
puml.directive
puml.participant
puml.alias
puml.arrow
puml.message
puml.note
puml.group
puml.lifecycle
puml.style
puml.include
puml.comment
puml.string
puml.color
puml.error
```

## Fixtures

Every syntax fixture mirrors the renderer fixture suite.

Required fixture groups:

```text
basic/
participants/
arrows/
notes/
groups/
lifecycle/
styling/
structure/
preprocessor/
markdown/
errors/
partial/
large/
```

Every valid fixture snapshots:

1. Parser AST from `puml-core`.
2. Rust token stream from `puml-syntax`.
3. TextMate scope ranges.
4. Tree-sitter parse tree.
5. Tree-sitter highlight captures.
6. LSP semantic token ranges.

Every invalid fixture snapshots:

1. Degraded token stream.
2. Tree-sitter error nodes.
3. TextMate scopes.
4. LSP diagnostics.

## Drift detection

Add a required test binary:

```text
cargo test -p puml-syntax syntax_contract
```

It must:

- parse every fixture with `puml-core`
- tokenize every fixture with `puml-syntax`
- run TextMate grammar snapshots through a grammar test harness
- run Tree-sitter parse and query snapshots
- compare token categories across all three systems
- fail if a known primitive is missing from any layer

No fixture can be marked “expected mismatch” without a written reason in the snapshot metadata.

## Browser editor contract

`puml-studio` may use one of two highlighting paths:

1. LSP semantic tokens through `puml-language` compiled to WASM.
2. Tree-sitter highlighting through `tree-sitter-puml` compiled for the browser.

Rules:

- Browser syntax highlighting must share the same token taxonomy.
- Browser syntax highlighting must not duplicate grammar rules in ad-hoc TypeScript.
- Browser syntax highlighting must continue working while WASM rendering is loading.
- Highlighting large files must not block the main thread.

## Agent tooling contract

Agent-facing packages use syntax metadata for repair loops.

Required capabilities:

- extract participant declarations
- extract aliases
- identify unresolved participant references
- identify diagram block boundaries
- identify message rows
- identify group/note/ref block ranges
- identify syntax-invalid lines

Agents must never rely on prompt-only syntax descriptions when token metadata is available.

## Security

Syntax tooling handles untrusted text.

Rules:

- No filesystem IO.
- No include expansion.
- No network access.
- No `eval`.
- No regex with catastrophic backtracking.
- No unbounded recursion.
- No panics on malformed Unicode.
- No raw HTML injection in generated highlighted output.
- No embedding of source text into HTML/SVG without escaping.

## Performance

Targets:

- Initial tokenization of a 1,000-message diagram: under 20ms in native tests.
- Incremental Tree-sitter edit response: under 5ms for common single-line edits.
- TextMate tokenization should not exhibit pathological behavior on 10,000-line files.
- Semantic token generation is linear in source size plus include graph size.

Benchmarks:

```text
scripts/bench-syntax.sh
```

Benchmark cases:

- hello fixture
- 1,000-message fixture
- 10,000-message fixture
- pathological malformed quote fixture
- deeply nested group fixture
- Markdown file with 100 puml fences

## Development commands

```console
cargo fmt
cargo clippy --workspace -- -D warnings
cargo test -p puml-syntax
npm test --workspace @puml/syntax
npm run test:tree-sitter --workspace @puml/syntax
npm run test:textmate --workspace @puml/syntax
cargo llvm-cov --package puml-syntax --fail-under-lines 90
```

## Definition of done

- Every sequence primitive has TextMate highlighting.
- Every sequence primitive has Tree-sitter grammar coverage.
- Every sequence primitive has highlight query coverage.
- Every sequence primitive has LSP semantic token mapping.
- Markdown fences highlight as `source.puml`.
- `.puml`, `.plantuml`, and `.iuml` files activate the language.
- Drift detection test passes across parser, Rust tokenizer, TextMate, Tree-sitter, and LSP semantic tokens.
- Invalid source highlights locally and remains editable.
- No regex catastrophes on large or hostile input.
- No syntax tool performs IO or network calls.
- 90% line coverage passes.
- Default clippy passes with warnings denied.
- The syntax package is usable by VS Code, Markdown renderer, SPA, LSP, and agent pack.

## Reference docs checked

- VS Code uses TextMate grammars as its main tokenization engine and semantic tokens as an additional layer.
- VS Code language extensions can define language configuration for comments, brackets, folding, indentation, and word patterns.
- Tree-sitter supports syntax highlighting through grammar repositories and query files.

Reference URLs:

- https://code.visualstudio.com/api/language-extensions/semantic-highlight-guide
- https://code.visualstudio.com/api/language-extensions/syntax-highlight-guide
- https://code.visualstudio.com/api/language-extensions/language-configuration-guide
- https://tree-sitter.github.io/tree-sitter/3-syntax-highlighting.html
- https://tree-sitter.github.io/tree-sitter/using-parsers/queries/1-syntax.html
