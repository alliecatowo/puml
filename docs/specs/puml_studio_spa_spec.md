# `puml-studio` SPA Specification

A local-first browser studio for writing, drawing, styling, rendering, exporting, and sharing native `puml` sequence diagrams.

The site is the public proof that `puml` is not just a CLI. It is the fastest way to make sequence diagrams without installing Java, Graphviz, or a desktop app.

## Product position

`puml-studio` is the browser face of the product:

- paste PlantUML-compatible sequence syntax
- write with syntax highlighting, diagnostics, completions, and hover
- build diagrams visually with WYSIWYG controls
- render instantly through the same Rust engine as the CLI
- apply polished style presets
- export SVG, PNG, PDF, `.puml`, and structured JSON
- share diagrams without uploading user content to a server

The browser studio must make the CLI feel inevitable: the same engine, the same diagnostics, the same deterministic output.

## Hosting decision

Default hosting: GitHub Pages deployed by GitHub Actions.

This is a static app. Treat that as architecture, not a temporary convenience.

Vercel is allowed only if a concrete product requirement needs runtime server behavior that GitHub Pages cannot provide, such as authenticated private galleries, server-side image rendering, team workspaces, or persistent cloud projects. Until that requirement exists, there is no server.

## Non-negotiables

- Static SPA.
- No backend runtime.
- No user diagram content uploaded by default.
- No server-side rendering.
- No API requirement to render.
- No external rendering service.
- No duplicate parser in TypeScript.
- No duplicate layout engine in TypeScript.
- No duplicate syntax highlighter that disagrees with the parser.
- No hidden telemetry.
- No Graphviz.
- No JVM.
- No Mermaid fallback.
- No canvas-only renderer for primary output.
- SVG remains the canonical render artifact.
- WYSIWYG edits update source text; source text is the canonical document.
- 90% test coverage minimum for app logic and WASM bindings.

## Architecture religion

Use a local-first, WASM-first, source-of-truth architecture.

The pipeline is:

```text
source text
  -> puml-wasm compile call
  -> diagnostics + AST + semantic model + scene + SVG
  -> UI state machine
  -> editor / preview / designer / export surfaces
```

Rules:

- Rust owns parsing, normalization, layout, scene, and SVG rendering.
- TypeScript owns UI state, event handling, persistence, and browser export wrappers.
- Source text is canonical.
- Visual edits are AST/model transforms that produce canonical `.puml` source patches.
- The visual designer never stores hidden diagram data that cannot be represented in `.puml`.
- The preview never renders from stale source.
- The preview never uses a separate layout path.
- Themes are token overrides passed to the renderer, not CSS hacks over emitted SVG.
- The app state is explicit and serializable.
- All rendering work runs in a Web Worker.
- The main thread owns interaction, not compilation.
- Every user-visible command is testable without a browser when possible.

The app should feel like a small CAD tool powered by a compiler, not a textarea glued to an iframe.

## Tech stack

Use a deliberately small stack:

- TypeScript, strict mode
- Vite
- Svelte
- CodeMirror 6
- Rust `wasm-bindgen` / `wasm-pack` for `puml-wasm`
- Web Worker for compile/render/language-service calls
- Playwright for E2E
- Vitest for unit/integration tests
- pnpm or npm, one package manager only

Forbidden unless justified by measurement:

- Next.js
- Remix
- server components
- runtime backend
- Redux
- giant UI component frameworks
- diagramming libraries
- layout libraries
- SVG rendering libraries
- analytics SDKs

If the stack starts fighting the local-first model, the stack loses.

## Repository structure

```text
site/
  package.json
  vite.config.ts
  tsconfig.json
  index.html

  src/
    main.ts
    App.svelte

    app/
      state.ts                  # serializable app state
      commands.ts               # command/event model
      reducer.ts                # pure state transitions
      persistence.ts            # localStorage/indexedDB/url hash
      shortcuts.ts

    engine/
      worker.ts                 # Web Worker entry
      client.ts                 # typed worker client
      protocol.ts               # request/response types
      wasm.ts                   # puml-wasm loader

    editor/
      PumlEditor.svelte
      codemirror.ts
      completions.ts
      diagnostics.ts
      semanticTokens.ts
      hovers.ts

    preview/
      PreviewPane.svelte
      svgHost.ts
      panZoom.ts
      hitTesting.ts

    designer/
      DesignerPane.svelte
      toolbar.ts
      inspector.ts
      sourceTransforms.ts
      selection.ts
      drag.ts

    export/
      exportSvg.ts
      exportPng.ts
      exportPdf.ts
      exportPuml.ts
      exportJson.ts

    theme/
      presets.ts
      tokens.ts
      controls.ts

    components/
      SplitPane.svelte
      Toolbar.svelte
      Inspector.svelte
      DiagnosticsPanel.svelte
      ExportDialog.svelte
      ThemePicker.svelte

    routes/
      examples.ts
      gallery.ts

  tests/
    unit/
    integration/
    e2e/
    fixtures/
    snapshots/

crates/
  puml-wasm/
```

## WASM API (Target)

`puml-wasm` exposes typed functions:

```typescript
type CompileOptions = {
  theme?: string;
  includeRoot?: string | null;
  page?: number | null;
};

type CompileResult = {
  ok: boolean;
  diagnostics: DiagnosticDto[];
  ast?: AstDto;
  model?: ModelDto;
  scene?: SceneDto;
  svg?: string;
  pages: PageInfoDto[];
  participants: ParticipantDto[];
  symbols: SymbolDto[];
  semanticTokens: SemanticTokenDto[];
};
```

Required exports:

```text
compile(source, options) -> CompileResult
parse(source) -> AstResult
normalize(source) -> ModelResult
layout(source, options) -> SceneResult
render_svg(source, options) -> SvgResult
format(source, options) -> FormatResult
semantic_tokens(source) -> SemanticTokensResult
complete(source, position) -> CompletionResult
hover(source, position) -> HoverResult
definition(source, position) -> DefinitionResult
references(source, position) -> ReferencesResult
apply_transform(source, transform) -> SourcePatchResult
list_primitives() -> PrimitiveCatalog
list_themes() -> ThemeCatalog
```

The browser does not reimplement these operations.

## Worker protocol (Target)

All compile/render/language operations go through a worker.

Requests:

```text
engine/init
engine/compile
engine/format
engine/complete
engine/hover
engine/definition
engine/references
engine/applyTransform
engine/exportSvg
engine/exportPngPrepare
engine/exportPdfPrepare
engine/listThemes
engine/listPrimitives
```

Rules:

- Every request has a monotonically increasing ID.
- Stale compile responses are dropped.
- The worker can cancel or ignore obsolete work.
- The main thread never blocks on render.
- WASM load failure is a first-class error state with recovery UI.

## Runtime contract snapshot (Current, audited in issue #23)

The sections above describe the target Studio runtime. The current contract that already exists in the Rust runtime and is safe for Studio integration today is:

### Rust library surface available now

- `parse(source) -> Document | Diagnostic`
- `normalize(document) -> SequenceDocument | Diagnostic`
- `layout::layout_pages(sequence, LayoutOptions::default()) -> Scene[]`
- `render::render_svg(scene) -> string`
- `render_source_to_svg(source) -> string | Diagnostic` for single-page sequence diagrams
- `render_source_to_svgs(source) -> string[] | Diagnostic` for multi-page sequence output
- `render_source_to_svg_for_family` / `render_source_to_svgs_for_family` currently enforce sequence-only rendering and deterministic errors for unsupported families
- `extract_markdown_diagrams(source) -> DiagramInput[]` for fenced diagram extraction

### CLI/runtime contract available now

- `--dump ast|model|scene` produces deterministic JSON payloads
- `--diagnostics json` emits schema `puml.diagnostics` with `schema_version: 1`
- `--multi` is required for multi-output stdin rendering (multiple `@startuml` and/or `newpage` pages)
- `--dump-capabilities` exposes the LSP capability manifest, including custom requests:
  - `puml.applyFormat`
  - `puml.renderSvg`

### Studio binding guidance for current runtime

- Treat the current Studio worker contract as a thin wrapper over these runtime surfaces:
  - compile path = parse + normalize + layout + render
  - structural exports = `--dump` equivalents (`ast`, `model`, `scene`)
  - diagnostics payload = `puml.diagnostics` schema contract
- Do not advertise editor-LSP parity features (complete/hover/definition/references/format transforms through WASM) as shipped Studio runtime until dedicated exports land in `puml-wasm`.
- Any new Studio API in this spec must be backed by:
  - implementation in runtime crates
  - explicit contract tests guarding drift
  - docs updates in this spec and release contract docs

## Core screens

Single-page app views:

1. **Editor + Preview**
   - split-pane source editor and live SVG preview
   - diagnostics panel
   - theme picker
   - export button

2. **Designer + Preview**
   - visual toolbar
   - diagram canvas
   - selected element inspector
   - generated source panel optionally visible

3. **Examples**
   - curated examples covering every primitive
   - clicking an example loads it locally

4. **Export dialog**
   - SVG
   - PNG
   - PDF
   - `.puml`
   - JSON AST
   - JSON model
   - JSON scene
   - Markdown embed
   - HTML embed

5. **Settings**
   - theme defaults
   - editor preferences
   - privacy statement
   - local data reset

No router complexity beyond what the app needs. URL hash is enough if full client routing creates hosting friction.

## Editor requirements

The editor is CodeMirror backed by `puml-language` through WASM.

Required:

- syntax highlighting from semantic tokens
- parse/semantic diagnostics
- squiggles with messages
- completions
- snippets
- hover docs
- go-to-definition
- participant/alias rename command if feasible in browser
- formatting command
- code actions for common repairs
- bracket/block folding
- find participant references
- inline color picker for color literals
- file drag/drop
- paste handling
- keyboard shortcuts

Editor commands:

```text
Format diagram
Render now
Toggle preview
Toggle designer
Add explicit participants
Insert participant
Insert message
Insert note
Insert alt block
Insert loop block
Insert activation
Export
Share link
Reset example
```

The editor must remain useful without the visual designer.

## Live rendering

Rendering behavior:

- Compile after edits with a short debounce.
- Compile immediately on explicit render command.
- Keep showing last successful SVG when current source has errors.
- Overlay error state clearly.
- Show parse errors with line/column.
- Preserve scroll/zoom between renders when possible.
- Render selected diagram block when multiple blocks exist.
- Provide page selector for `newpage` output.

Latency targets:

| Action | Target |
| --- | ---: |
| Small source edit to diagnostics | < 50 ms |
| Small source edit to preview | < 75 ms |
| 1,000-message edit to diagnostics | < 200 ms |
| 1,000-message edit to preview | < 300 ms |
| Theme switch | < 100 ms |
| Export SVG | < 50 ms |
| Export PNG 2x | < 500 ms |

## Visual designer

The designer is not a fake diagram drawer. It is an AST editor.

Supported insertions:

- participant
- actor
- boundary
- control
- entity
- database
- collections
- queue
- participant box
- message
- self-message
- found message
- lost message
- bidirectional message
- note left/right/over/across
- `hnote`
- `rnote`
- ref
- alt
- else branch
- opt
- loop
- par
- break
- critical
- group
- activation
- deactivation
- create
- destroy
- return
- divider
- delay
- spacer
- title
- header
- footer
- caption
- legend
- footbox toggle
- skinparam/theme token override

Supported visual operations:

- select participant
- select message
- select note
- select group
- select lifecycle box
- edit label text
- edit participant display name
- edit alias
- edit participant kind
- edit arrow kind
- edit note placement
- edit group type/label
- drag participant to reorder
- drag message row to reorder when semantics allow it
- drag note side/anchor when semantics allow it
- delete selected element
- duplicate selected element
- wrap selected rows in group
- unwrap group
- add branch to `alt` or `par`
- convert participant kind
- convert message arrow style

Designer rules:

- Every operation produces a source patch.
- Every patch is recompiled before UI commits it as successful.
- Failed transforms are rejected with diagnostics.
- The designer cannot create syntax the parser does not support.
- The designer cannot silently drop comments.
- The designer shows source diff for destructive operations.
- Hidden state is forbidden.

## Hit testing

The SVG scene must expose enough metadata for browser hit testing.

Scene metadata:

```text
scene node ID
source span
semantic ID
kind
bounding box
parent group
row index
participant ID
```

SVG output for studio mode may include deterministic `data-puml-*` attributes. Public/export SVG can strip them unless the user asks for editable SVG metadata.

## Styling

Theme presets:

- Default
- Minimal
- Monochrome
- Dark
- Blueprint
- Sand
- GitHub Docs
- Terminal

Theme controls:

- background
- font family
- font size
- participant fill
- participant stroke
- participant text
- lifeline stroke
- lifeline dash
- arrow stroke
- message text
- note fill
- note stroke
- group fill
- group stroke
- activation fill
- activation stroke
- compact/spacious spacing
- show/hide footbox
- page padding
- scale

Rules:

- Themes are renderer tokens.
- Themes are serializable.
- Theme changes never mutate source unless the user chooses “write theme to source”.
- “Write theme to source” emits supported `skinparam sequence` syntax only.
- Unsupported style desires remain UI-only theme tokens until the language supports them.

## Export formats

### SVG

- canonical output from renderer
- standalone
- sanitized
- deterministic
- optional studio metadata
- optional transparent background

### PNG

- generated in browser from SVG
- supports 1x, 2x, 3x, 4x
- supports transparent background
- checks canvas tainting by avoiding external assets entirely

### PDF

- direct download when implemented
- fallback to browser print is not enough for the export button
- multi-page diagrams produce multi-page PDF
- no rasterization unless user chooses raster PDF

### `.puml`

- current source
- optionally formatted source
- optionally includes generated explicit participants

### JSON

- AST JSON
- semantic model JSON
- scene JSON
- diagnostics JSON

### Embeds

- Markdown image reference
- HTML inline SVG
- HTML `<img>` with data URI
- mdBook-friendly snippet

## Share links

Share behavior:

- Source compressed into URL hash.
- No server persistence.
- Links are deterministic.
- Very large diagrams show warning and offer file export instead.
- User can strip theme or include theme in link.

Privacy:

- Never send diagram content to analytics.
- Never send diagram content to a server by default.
- If future cloud sharing exists, it is opt-in and visibly different.

## Persistence

Local persistence:

- current source
- recent diagrams
- theme preference
- pane layout
- editor settings
- last selected example

Rules:

- Store locally in localStorage or IndexedDB.
- Provide “Reset local data”.
- Never persist secrets intentionally.
- Do not persist content if user enables private mode.

## Accessibility

Required:

- keyboard navigation for toolbar
- keyboard navigation for export dialog
- editor usable with screen readers according to CodeMirror capabilities
- preview has accessible title/description
- diagnostics panel is keyboard reachable
- color themes meet contrast targets where possible
- non-color error indicators
- reduced-motion handling
- focus states are visible

## Examples gallery

Examples cover every supported primitive:

- hello
- aliases
- participant kinds
- participant boxes
- notes
- refs
- self messages
- async arrows
- found/lost messages
- lifecycle
- alt/loop/par/group
- dividers/delays/spacers
- autonumber
- styling
- title/header/footer/caption/legend
- newpage
- includes represented as local examples where browser constraints allow

Every example must render successfully and be part of test fixtures.

## GitHub Pages deployment

Deployment path:

```text
push to main
  -> GitHub Actions
  -> install toolchain
  -> build Rust WASM
  -> run tests
  -> build SPA
  -> upload Pages artifact
  -> deploy to GitHub Pages
```

Required workflow jobs:

```text
check-rust
check-site
test-wasm
test-site
playwright
a11y
build
deploy-pages
```

Rules:

- Deploy only from protected main branch.
- Pull requests build preview artifacts but do not publish production.
- The app works under a subpath for GitHub Pages project sites.
- All asset URLs are relative or base-path aware.
- 404 fallback handles SPA routes if routes are used.

## Build budgets

Budgets are enforced in CI:

| Asset | Budget |
| --- | ---: |
| initial JS gzip | <= 350 KB |
| initial CSS gzip | <= 60 KB |
| WASM gzip | <= 1.5 MB |
| total initial transfer gzip | <= 2.0 MB |
| Lighthouse performance | >= 90 |
| Lighthouse accessibility | >= 95 |
| Lighthouse best practices | >= 95 |

If CodeMirror or WASM pushes the app over budget, split aggressively before adding new features.

## Testing

Testing is the product contract.

### Unit tests

- reducer transitions
- command model
- source transforms
- theme token conversion
- export option building
- URL hash encoding/decoding
- persistence migration
- worker protocol serialization
- diagnostics mapping
- semantic token mapping

### WASM parity tests

For every fixture:

- CLI SVG equals WASM SVG
- CLI AST equals WASM AST
- CLI model equals WASM model
- CLI scene equals WASM scene
- diagnostics match
- formatting matches

### Integration tests

- editor edit triggers compile
- diagnostics render
- completions appear
- hover appears
- formatting changes source
- theme change updates preview
- designer insertion updates source
- source edit updates designer selection metadata
- export SVG downloads valid SVG
- export PNG downloads non-empty PNG
- export PDF downloads non-empty PDF
- share link restores source and theme
- local persistence restores session

### E2E tests

Use Playwright.

Flows:

- load app
- paste hello diagram
- render preview
- switch theme
- export SVG
- export PNG
- create diagram visually from blank
- add participant
- add message
- add note
- add alt block
- edit labels
- verify generated source parses
- verify exported SVG contains expected text
- load share link
- run keyboard-only export flow

### Visual regression

- snapshot preview SVG for examples
- screenshot app shell for core flows
- deterministic viewport sizes
- tolerate font differences only where unavoidable

### Accessibility tests

- automated axe checks
- keyboard navigation smoke
- focus trap tests for dialogs
- contrast checks for presets

### Coverage

Minimum:

```text
90% line coverage for TypeScript app logic
90% line coverage for puml-wasm bindings where measurable
all source transforms covered
all export paths covered
all designer operations covered
```

Commands:

```console
pnpm test
pnpm test:e2e
pnpm test:a11y
pnpm coverage -- --coverage.lines=90
cargo llvm-cov -p puml-wasm --fail-under-lines 90
```

Do not lower coverage to pass. Add tests.

## Security

- SVG sanitized.
- No script tags in exported SVG.
- No external images.
- No external fonts.
- No remote include fetch.
- No user source in telemetry.
- No eval.
- Strict CSP for deployed site.
- Dependencies audited in CI.
- URL hash parser is bounded and rejects decompression bombs.
- Drag/drop file size limit.
- Worker messages validate payload shape.

## Error states

Required user-facing errors:

- WASM failed to load
- browser unsupported
- source parse error
- semantic error
- render error
- export error
- URL hash too large or invalid
- local storage unavailable
- unsupported file type
- file too large

Error UI must be calm, specific, and actionable.

## README requirements

The studio README includes:

- one-line pitch
- live demo link
- screenshot
- feature list
- local development commands
- architecture diagram
- WASM parity explanation
- deployment instructions
- privacy statement
- export format matrix
- browser support
- test/coverage commands

Tone:

- launch-quality
- confident
- no “toy” language
- no “phase 1” language
- no apology section

## Definition of done

`puml-studio` is done when:

- GitHub Pages deploy works from Actions.
- The app loads with no backend.
- The app renders every valid fixture.
- WASM output matches CLI output for AST, model, scene, SVG, and diagnostics.
- Code editor has semantic highlighting, diagnostics, completion, hover, and formatting.
- Designer can create and edit every supported primitive without hidden state.
- Source remains canonical.
- Theme presets work.
- SVG, PNG, PDF, `.puml`, and JSON exports work.
- Share links work without server persistence.
- Playwright E2E covers editor, designer, preview, export, and sharing.
- Accessibility checks pass.
- Coverage is at least 90%.
- Bundle budgets pass.
- No telemetry is enabled by default.
- README reads like a real product.

## Reference anchors

- GitHub Pages publishing sources and Actions deployment: https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site
- GitHub Actions deployments: https://docs.github.com/actions/deployment/about-deployments/deploying-with-github-actions
- Vercel deployments, for future server-backed comparison only: https://vercel.com/docs/deployments
