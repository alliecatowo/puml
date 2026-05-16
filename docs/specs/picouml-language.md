# PicoUML Language Baseline

Issue link: #128

## Scope

This baseline defines first-class PicoUML surface routing into the shared sequence model.

## Canonical block markers

- Supported canonical markers:
  - `@startpicouml`
  - `@endpicouml`
- Canonical PicoUML markers are normalized to PlantUML markers internally before parsing.

## Frontend routing

- `--dialect picouml` routes through PicoUML adaptation and into the shared parser/normalize pipeline.
- Markdown fenced code blocks tagged `picouml` are treated as first-class PicoUML frontend input.

## Deterministic diagnostics

- Mixed marker forms are rejected for PicoUML frontend input.
- Diagnostic code: `E_PICOUML_MARKER_MIXED`
- Trigger condition: source contains any canonical PicoUML marker and any PlantUML marker in the same input.

## Security and architecture constraints

- No network fetch is performed by PicoUML routing.
- No additional production dependencies are introduced.
- PicoUML remains an adapter layer mapped onto the shared model pipeline.
