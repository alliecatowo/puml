# `puml-vscode` Extension Specification

A first-class VS Code extension for native `puml` sequence diagrams: syntax highlighting, LSP autocomplete, diagnostics, hover, rename, live preview, Markdown preview rendering, exports, and visual editing hooks.

This is not a thin grammar extension. This is the desktop surface for the whole product.

## Runtime contract snapshot (Current, audited in issue #24)

The sections below define the target extension surface. The current shipped VS Code runtime surface that is implemented today is:

- language id `puml` with extensions `.puml`, `.plantuml`, `.iuml`
- starter TextMate grammar + language configuration + snippets
- activation events:
  - `onLanguage:puml`
  - `onCommand:puml.preview.open`
  - `onCommand:puml.lsp.restart`
- commands:
  - `puml.preview.open` (PUML: Open Preview)
  - `puml.lsp.restart` (PUML: Restart Language Server)
- thin `puml-lsp` client startup controlled by:
  - `puml.lsp.enabled`
  - `puml.lsp.path`
  - `puml.lsp.trace`
- preview path delegates to LSP `workspace/executeCommand` using `puml.renderSvg` (no private parser in extension host/webview)

Current baseline guardrails:

- `./scripts/vscode-smoke.sh` checks `--dump-capabilities` includes `puml.applyFormat` and `puml.renderSvg`.
- extension smoke script verifies preview stays LSP-backed and build artifact exists.
- advanced VS Code features listed later in this spec stay target-state until landed in source + tests.

## Name

Marketplace name:

```text
puml
```

Extension ID:

```text
puml.puml-vscode
```

Display name:

```text
puml — Instant PlantUML Sequence Diagrams
```

Language ID:

```text
puml
```

Supported files:

```text
*.puml
*.plantuml
*.iuml
```

Supported Markdown fences:

```text
puml
plantuml
puml-sequence
uml-sequence
```

## Product position

VS Code is where a large share of diagram authoring happens. `puml-vscode` makes sequence diagrams feel like code:

- instant syntax highlighting
- semantic highlighting from the real language server
- diagnostics while typing
- autocomplete for every primitive
- hover docs for directives and arrows
- go-to-definition and rename for participants and aliases
- live SVG preview
- Markdown preview rendering for `puml` fences
- export current file, selection, or Markdown fence to SVG/PNG/PDF
- open a visual designer without losing source-of-truth text

No Java. No Graphviz. No hidden renderer. No syntax drift.

## Dependencies

`puml-vscode` depends on these product layers:

```text
puml-core              parser, model, layout, SVG renderer
puml-language          completions, hover, symbols, semantic tokens
puml-lsp               desktop LSP server
puml-syntax            TextMate grammar, Tree-sitter grammar, token taxonomy
puml-markdown          Markdown fence renderer contract
puml-wasm              web extension and webview rendering path
puml-studio components optional visual designer surface
```

The extension is not allowed to implement a private parser.

## Non-negotiables

- No Java.
- No Graphviz.
- No Mermaid fallback.
- No duplicate parser in TypeScript.
- No duplicate renderer in TypeScript.
- No network download on activation.
- No postinstall binary fetch.
- No telemetry unless explicitly added later with opt-in product review.
- No arbitrary shell execution.
- No remote includes by default.
- No raw SVG from untrusted source injected into webviews without renderer safety guarantees and CSP.
- No preview that disagrees with CLI output.
- No Markdown preview renderer that disagrees with `puml-markdown`.
- No syntax highlighter that disagrees with `puml-syntax` fixtures.
- Extension host stays responsive.
- Webview rendering does not block the extension host.
- 90% coverage minimum for extension logic.
- Desktop and web behavior are both specified, even when web has reduced capability.

## Architecture religion

Use a shell-and-services architecture and treat it as law.

```text
VS Code extension host
  -> language registration
  -> TextMate grammar from puml-syntax
  -> puml-lsp native server over stdio on desktop
  -> puml-wasm worker fallback for web extension
  -> preview webview using puml renderer
  -> Markdown preview adapter using puml-markdown
  -> command/export adapters
```

Rules:

- The VS Code extension is a shell.
- The LSP server owns language intelligence.
- The renderer owns SVG.
- The Markdown adapter owns fence discovery/rendering in Markdown preview.
- The webview owns presentation and interaction.
- The extension host owns command routing and VS Code integration.
- The extension host does not parse source except to locate active editor selections/fences when delegating to services.
- The preview never renders stale source intentionally.
- The preview and export commands use the same engine.
- Every command has an integration test.
- Every UI state has a deterministic backing model.

## Workspace layout

```text
extensions/
  vscode/
    package.json
    README.md
    CHANGELOG.md
    LICENSE
    src/
      extension.ts
      client/
        lsp.ts
        commands.ts
        config.ts
        language.ts
        markdownPreview.ts
        previewPanel.ts
        export.ts
        workspaceTrust.ts
      web/
        extension.web.ts
        wasmLanguageService.ts
      webview/
        preview.ts
        preview.css
        preview.html
        designerBridge.ts
      test/
        unit/
        integration/
        e2e/
    syntaxes/
      puml.tmLanguage.json
    language-configuration.json
    snippets/
      puml.json
    media/
      preview.js
      preview.css
    bin/
      darwin-arm64/puml-lsp
      darwin-x64/puml-lsp
      linux-x64/puml-lsp
      linux-arm64/puml-lsp
      win32-x64/puml-lsp.exe
    wasm/
      puml_wasm_bg.wasm
      puml_wasm.js
```

Platform-specific packaging may split binaries into platform VSIX packages, but the source layout remains explicit.

## VS Code contribution points

Required `package.json` contributions:

```json
{
  "contributes": {
    "languages": [{
      "id": "puml",
      "aliases": ["puml", "PlantUML Sequence"],
      "extensions": [".puml", ".plantuml", ".iuml"],
      "configuration": "./language-configuration.json"
    }],
    "grammars": [{
      "language": "puml",
      "scopeName": "source.puml",
      "path": "./syntaxes/puml.tmLanguage.json"
    }],
    "semanticTokenTypes": [],
    "semanticTokenModifiers": [],
    "snippets": [{
      "language": "puml",
      "path": "./snippets/puml.json"
    }],
    "commands": [],
    "menus": {},
    "configuration": {},
    "markdown.markdownItPlugins": true,
    "markdown.previewStyles": ["./media/markdown-preview.css"],
    "markdown.previewScripts": ["./media/markdown-preview.js"]
  }
}
```

Rules:

- Contribution declarations are exhaustive and documented.
- Commands use stable IDs under `puml.*`.
- Settings use stable IDs under `puml.*`.
- Markdown preview integration activates lazily when Markdown preview opens.

## Language configuration

`language-configuration.json` must support:

- comment toggling with `'`
- word pattern for participant aliases and quoted names
- bracket/paired token awareness where useful
- indentation for blocks
- folding markers for diagram blocks and sequence blocks

Required block pairs:

```text
@startuml / @enduml
note / end note
hnote / end note
rnote / end note
ref / end ref
title / end title
legend / end legend
skinparam sequence { / }
box / end box
alt / end
opt / end
loop / end
par / end
break / end
critical / end
group / end
```

Rules:

- Language configuration gives basic editing behavior before LSP starts.
- Folding from LSP overrides or augments declarative folding when available.

## Syntax highlighting

Required layers:

1. TextMate grammar from `puml-syntax` for immediate lexical highlighting.
2. Semantic tokens from `puml-lsp` for participant identity, aliases, unresolved refs, lifecycle state, and primitive semantics.

Rules:

- TextMate grammar is copied or generated from `puml-syntax`, never edited privately in the extension.
- Semantic token legend matches `puml-syntax` spec.
- Themes can style `puml` scopes without requiring custom extension themes.
- Highlighting works before the LSP server finishes starting.
- Semantic highlighting improves correctness after the server is ready.

## LSP client

Desktop mode launches `puml-lsp` over stdio.

Required capabilities:

```text
textDocument/didOpen
textDocument/didChange
textDocument/didSave
textDocument/didClose
textDocument/publishDiagnostics
textDocument/completion
completionItem/resolve
textDocument/hover
textDocument/definition
textDocument/references
textDocument/documentSymbol
workspace/symbol
textDocument/rename
textDocument/prepareRename
textDocument/codeAction
workspace/executeCommand
textDocument/formatting
textDocument/rangeFormatting
textDocument/foldingRange
textDocument/selectionRange
textDocument/semanticTokens/full
textDocument/semanticTokens/range
textDocument/inlayHint
```

Required custom LSP commands:

```text
puml.renderSvg
puml.renderScene
puml.export
puml.explainDiagnostic
puml.applyStyle
puml.insertParticipant
puml.convertSelectionToDiagram
puml.listDiagramsInMarkdown
```

Rules:

- LSP process is restarted on crash with backoff.
- Crashes are visible in output channel.
- Stale responses are discarded.
- Workspace configuration changes are sent to the server.
- The extension does not hide server diagnostics.

## Web extension mode

VS Code for the Web cannot spawn a native `puml-lsp` process.

Required web behavior:

- Use `puml-wasm` in a worker for parse, diagnostics, render, and limited completions.
- Use TextMate grammar for highlighting.
- Use semantic tokens if WASM language service exposes them cheaply.
- Disable filesystem includes unless virtual workspace support is explicit.
- Disable native binary configuration.
- Keep preview/export for SVG and source.
- PNG/PDF export depends on browser APIs.

Rules:

- Web mode is a first-class reduced mode, not an afterthought.
- Unsupported commands produce explicit messages.
- Desktop and web share test fixtures.

## Commands

Required commands:

```text
puml.preview.open
puml.preview.openToSide
puml.preview.refresh
puml.preview.lockToFile
puml.export.svg
puml.export.png
puml.export.pdf
puml.export.source
puml.export.jsonModel
puml.copy.svg
puml.copy.png
puml.copy.source
puml.check.currentFile
puml.check.workspace
puml.render.selection
puml.create.newDiagram
puml.insert.participant
puml.insert.message
puml.insert.note
puml.insert.group
puml.open.visualDesigner
puml.markdown.renderFence
puml.markdown.exportFence
puml.restartLanguageServer
puml.showOutput
puml.showAst
puml.showModel
puml.showScene
```

Rules:

- Every command is available from Command Palette where appropriate.
- Editor context menu exposes preview/export/check commands for `.puml` files.
- Markdown editor context menu exposes render/export fence commands when cursor is inside a supported fence.
- Explorer context menu exposes export/check for `.puml` files.

## Live preview

Preview panel behavior:

- Opens beside active editor by default.
- Updates on edit with debounce.
- Supports manual update mode.
- Shows inline diagnostics when render fails.
- Supports pan and zoom.
- Supports fit-to-screen.
- Supports theme selection.
- Supports export buttons.
- Supports copy buttons.
- Supports source-position mapping.

Preview pipeline:

```text
active document version
  -> render request
  -> LSP or WASM renderer
  -> SVG + scene metadata
  -> webview render
  -> click/hover maps scene node to source range
```

Rules:

- The preview webview has a strict CSP.
- SVG is treated as a render artifact, not arbitrary HTML.
- No external scripts.
- No external images.
- No source text embedded in comments.
- Preview update cancels stale render requests.
- Large diagrams show progress only when rendering is measurably slow.

## Markdown preview support

Required behavior:

- Render `puml`, `plantuml`, `puml-sequence`, and `uml-sequence` fences in VS Code Markdown preview.
- Preserve optional source display according to settings.
- Render inline at the fence location.
- Show diagnostics inline for invalid diagrams.
- Support export/copy controls when enabled.
- Support theme selection through settings.
- Re-render changed fences on Markdown preview update.

Implementation:

- Register `markdown.markdownItPlugins`.
- Contribute preview CSS with `markdown.previewStyles`.
- Use preview script only for controls/hydration that cannot be done statically.
- Reuse `puml-markdown` renderer contract.

Rules:

- The Markdown adapter does not parse diagrams itself.
- The Markdown adapter must not break non-`puml` fences.
- Invalid diagrams do not break the whole Markdown preview.
- Workspace trust controls include behavior.

## Visual designer

Required command:

```text
puml.open.visualDesigner
```

Behavior:

- Opens a webview designer for the current `.puml` document.
- Uses `puml-studio` components where possible.
- Source text remains canonical.
- Visual edits generate source patches.
- The user can inspect source diffs before applying destructive rewrites.
- Designer supports participants, messages, notes, groups, lifecycle, and styling presets.

Rules:

- No hidden diagram JSON stored beside source.
- No visual edit creates syntax outside the language spec.
- Designer cannot save an invalid diagram unless user explicitly chooses to keep invalid source.

## Autocomplete

Completion sources:

- sequence primitives
- participant names
- aliases
- note placements
- group kinds
- lifecycle commands
- skinparam names
- arrow operators
- theme names
- include paths
- snippets

Rules:

- Context-aware completions come from LSP.
- Snippets provide fast templates before LSP resolves context.
- Completion labels are concise.
- Completion docs explain syntax and include examples.
- Completions never insert unsupported syntax.

## Hover

Hover required for:

- directives
- participant declarations
- aliases
- participant references
- arrows
- note directives
- group directives
- lifecycle commands
- skinparams
- diagnostics

Participant hover includes:

- display name
- alias
- kind
- declaration location
- lifecycle state when meaningful
- references count when cheaply available

## Rename and refactor

Required:

- Rename participant alias.
- Rename participant display name where safe.
- Rename updates references across diagram block and includes when enabled.
- Code action to add explicit participant declaration for implicit participant.
- Code action to resolve ambiguous reference.
- Code action to fix common arrow typos.
- Code action to wrap selected messages in `alt`, `loop`, `opt`, or `group`.
- Code action to extract selected messages to a `ref` block.

Rules:

- Rename refuses ambiguous edits.
- Refactors preserve comments where possible.
- Refactors are backed by source spans from parser/model.

## Export

Required export formats:

```text
SVG
PNG
PDF
PUML source
JSON model
```

Export surfaces:

- current `.puml` file
- active selection
- active Markdown fence
- all Markdown fences in current file
- all `.puml` files in workspace

Rules:

- SVG is canonical.
- PNG/PDF use renderer output, not a second drawing path.
- Exports never overwrite without confirmation unless explicit setting allows it.
- Exports fail with precise diagnostics.
- Batch export reports per-file status.

## Settings

Required settings:

```json
{
  "puml.lsp.enabled": true,
  "puml.lsp.path": "",
  "puml.lsp.trace": "off",
  "puml.preview.updateMode": "onEdit",
  "puml.preview.debounceMs": 150,
  "puml.preview.theme": "default",
  "puml.preview.fit": true,
  "puml.preview.controls": true,
  "puml.markdown.enabled": true,
  "puml.markdown.source": "collapsible",
  "puml.markdown.controls": true,
  "puml.markdown.theme": "default",
  "puml.export.defaultFormat": "svg",
  "puml.export.defaultDirectory": "",
  "puml.includes.enabled": false,
  "puml.includes.roots": [],
  "puml.security.allowRemoteIncludes": false,
  "puml.telemetry.enabled": false
}
```

Rules:

- Defaults are secure.
- Remote includes default to false permanently.
- Settings are documented in the extension README.
- Settings changes apply without reload when possible.

## Snippets

Required snippets:

```text
startuml/enduml
participant
participant alias
actor/control/database
message
async message
dashed return
self message
note left/right/over
alt/else/end
loop/end
opt/end
par/else/end
activate/deactivate
create/destroy
ref block
title block
skinparam sequence block
```

Snippets must use valid syntax from the language spec only.

## Workspace trust and security

Rules:

- In untrusted workspaces, includes are disabled.
- In untrusted workspaces, export still works for the active in-memory document.
- No arbitrary command execution.
- No automatic binary download.
- No remote renderer.
- Webview CSP blocks inline scripts except nonce-controlled extension scripts.
- Webview cannot read local files directly.
- Markdown preview scripts only hydrate known `puml` elements.
- SVG output contains no script tags.
- File writes require explicit command or configured export path.

## Packaging

Desktop packaging:

- Publish platform-specific VSIX packages where native `puml-lsp` size requires it.
- Include native `puml-lsp` binaries in platform packages.
- Include no postinstall downloader.
- Include checksums for bundled binaries.

Web packaging:

- Publish web-compatible extension bundle.
- Include `puml-wasm`.
- No native binary assumptions.

Marketplace targets:

```text
Visual Studio Marketplace
Open VSX Registry
```

Rules:

- Marketplace README shows live preview, Markdown preview, completions, diagnostics, and exports.
- Release notes list bundled `puml-core` and `puml-lsp` versions.
- Extension versioning tracks product compatibility, not just extension wrapper changes.

## Testing contract

Unit tests:

- configuration parsing
- command routing
- export path generation
- Markdown fence detection delegation
- webview message validation
- workspace trust behavior
- LSP client lifecycle

Integration tests:

- extension activates for `.puml`
- syntax grammar loads
- LSP starts on desktop
- diagnostics appear for invalid source
- completions appear for primitives and participants
- hover appears for participant and arrow
- rename updates aliases
- preview opens and renders SVG
- export SVG writes file
- Markdown preview renders `puml` fence
- invalid Markdown fence shows inline diagnostic
- web mode loads WASM fallback

E2E tests:

- create new diagram from command
- edit source and preview updates
- click preview element and source range is revealed
- export current file to SVG/PNG/PDF
- export Markdown fence
- run check workspace
- restart LSP after crash
- open visual designer and apply source patch

Snapshot tests:

- generated package.json contributions
- language configuration
- snippets
- preview webview HTML shell
- Markdown preview rendered HTML
- extension diagnostics rendering

Coverage:

```console
npm test --workspace puml-vscode
npm run test:integration --workspace puml-vscode
npm run test:e2e --workspace puml-vscode
npm run coverage --workspace puml-vscode -- --coverage.threshold.lines=90
```

Do not lower coverage. Add tests.

## Performance

Targets:

- Extension activation under 100ms excluding LSP process startup.
- Preview update under 50ms for small diagrams after server is warm.
- Markdown preview with 20 small fences remains responsive.
- LSP diagnostics debounce avoids flooding during typing.
- Extension host never performs heavy rendering synchronously.

Benchmarks:

```text
scripts/bench-vscode-extension.sh
```

Scenarios:

- activate extension
- open hello.puml
- render preview
- edit message line and update preview
- Markdown preview with 100 fences
- export 100 diagrams

## README contract

The extension README must read like a product, not a wrapper.

Required sections:

1. One-line pitch:

   ```text
   PlantUML sequence diagrams in VS Code, but instant: no JVM, no Graphviz, live SVG preview.
   ```

2. GIF or screenshot placeholders:

   - diagnostics while typing
   - autocomplete
   - live preview
   - Markdown preview rendering
   - export menu

3. Features checklist:

   - syntax highlighting
   - semantic highlighting
   - autocomplete
   - diagnostics
   - hover
   - rename
   - live preview
   - Markdown preview support
   - export SVG/PNG/PDF
   - visual designer

4. Security stance:

   - no Java
   - no Graphviz
   - no remote renderer
   - remote includes disabled

5. Settings table.

6. Troubleshooting:

   - LSP binary missing
   - workspace trust
   - Markdown preview disabled
   - web extension reduced mode

## Definition of done

- Extension activates for `.puml`, `.plantuml`, and `.iuml`.
- TextMate syntax highlighting works immediately.
- Semantic highlighting works through LSP.
- Native LSP starts on desktop.
- WASM fallback works in VS Code Web.
- Diagnostics report line/column and update while typing.
- Completion, hover, definition, references, rename, code actions, folding, symbols, and semantic tokens work.
- Live preview renders SVG and updates on edit.
- Markdown preview renders `puml` and `plantuml` fences inline.
- Export commands work for SVG, PNG, PDF, source, and JSON model where supported.
- Visual designer opens and writes source patches.
- Workspace trust restrictions are enforced.
- No remote renderer or postinstall binary download exists.
- All commands have integration tests.
- E2E suite covers authoring, preview, Markdown, export, and failure paths.
- 90% coverage passes.
- Marketplace package includes README, changelog, license, and screenshots/GIF placeholders.

## Reference docs checked

- VS Code language server extensions use a TypeScript/JavaScript language client that communicates with a separate language server process over LSP.
- VS Code uses TextMate grammars for main tokenization and semantic tokens as an additional semantic layer.
- VS Code Markdown preview can be extended with markdown-it plugins, preview styles, and preview scripts.
- VS Code supports platform-specific extension packages.
- VS Code web extensions run in a browser sandbox and cannot rely on Node APIs/native process spawning.

Reference URLs:

- https://code.visualstudio.com/api/language-extensions/language-server-extension-guide
- https://code.visualstudio.com/api/language-extensions/semantic-highlight-guide
- https://code.visualstudio.com/api/language-extensions/language-configuration-guide
- https://code.visualstudio.com/api/extension-guides/markdown-extension
- https://code.visualstudio.com/api/extension-guides/web-extensions
- https://code.visualstudio.com/api/working-with-extensions/publishing-extension
