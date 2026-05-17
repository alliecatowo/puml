# PicoUML Example Gallery (Current)

This gallery indexes the docs-as-tests corpus under `docs/examples/`.

## Corpus totals

- `255` source diagrams (`*.puml`)
- `258` render artifacts (`*.svg`)
- Site gallery manifest: `248` paired family examples across `31` family directories
- Location: `docs/examples/` and its family subdirectories

## Family directories

### Core UML families

- `sequence/`
- `class/`
- `object/`
- `usecase/`
- `component/`
- `deployment/`
- `state/`
- `activity/`
- `activity_new/`
- `activity_old/`
- `timing/`

### Timeline and planning

- `gantt/`
- `chronology/`

### Non-UML / specialized families

- `salt/`
- `json/`
- `yaml/`
- `nwdiag/`
- `archimate/`
- `regex/`
- `ebnf/`
- `chart/`
- `math/`
- `sdl/`
- `ditaa/`
- `mindmap/`
- `wbs/`

### Compatibility and styling surfaces

- `c4/`
- `themes/`
- `skinparams/`
- `preprocessor/`
- `creole/`

## Top-level examples in this folder

- `basic_hello.puml` -> `basic_hello.svg`
- `groups_notes.puml` -> `groups_notes.svg`
- `lifecycle_autonumber.puml` -> `lifecycle_autonumber.svg`
- `supported_primitives_*.puml` -> corresponding `*.svg`

## Status framing

- Families are no longer documented here as “parse-only” or “not yet parsed”.
- Current status should be interpreted as:
  - family availability: implemented
  - feature depth inside each family: mixed (`implemented` and `partial`), tracked in audits and limitations docs

See:
- [KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md)
- [../audits/parity_gap_core.csv](../audits/parity_gap_core.csv)
- [../audits/parity_gap_nonuml.csv](../audits/parity_gap_nonuml.csv)
