# PicoUML Language Baseline

Issue link: #128

## Scope

This baseline defines first-class PicoUML surface routing into the shared sequence model. `puml` is the Rust renderer binary/engine; PicoUML is the project-owned language surface that adapts into that shared pipeline; PlantUML remains the compatibility target.

## Canonical block markers

- Supported canonical markers:
  - `@startpicouml`
  - `@endpicouml`
- Canonical PicoUML markers are normalized to PlantUML markers internally before parsing.

## Frontend routing

- `--dialect picouml` routes through PicoUML adaptation and into the shared parser/normalize pipeline.
- Files ending in `.picouml` route through PicoUML adaptation when the CLI dialect is `auto`.
- Markdown fenced code blocks tagged `picouml` are treated as first-class PicoUML frontend input.
- Compact sequence arrows route deterministically through the shared PlantUML model:
  - `A => B : msg` becomes a sync call message from `A` to `B`.
  - `A <= B : msg` becomes a sync call message from `B` to `A`.
  - `A ~> B : msg` becomes an async signal message from `A` to `B`.
  - `A <~ B : msg` becomes an async signal message from `B` to `A`.
- Shorthand multi-target notes route through PlantUML `note over`:
  - `note A,B : text` becomes `note over A,B: text`.
  - `note over A,B : text` is accepted directly.

## Deterministic diagnostics

- Mixed marker forms are rejected for PicoUML frontend input.
- Diagnostic code: `E_PICOUML_MARKER_MIXED`
- Trigger condition: source contains any canonical PicoUML marker and any PlantUML marker in the same input.

## Security and architecture constraints

- No network fetch is performed by PicoUML routing.
- No additional production dependencies are introduced.
- PicoUML remains an adapter layer mapped onto the shared model pipeline.
