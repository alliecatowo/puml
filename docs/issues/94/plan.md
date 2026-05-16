# Issue 94 Plan

## Scope

- Add parser, AST, and model support for `@startgantt`/`@endgantt`.
- Add parser, AST, and model support for `@startchronology`/`@endchronology`.
- Cover the core Gantt subset: bracketed tasks and milestones, aliases, `Project starts`, `requires`, `starts`, `ends`, task start/end constraints, and bracketed dependencies.
- Cover the core Chronology subset: bracketed events with `happens on`.
- Keep unsupported sub-syntax deterministic while the slice is intentionally partial.

## Validation Design

- Valid fixtures under `tests/fixtures/non_uml/` prove accepted Gantt and Chronology grammar.
- Invalid fixtures under `tests/fixtures/non_uml/` prove `E_GANTT_UNSUPPORTED` and `E_CHRONOLOGY_UNSUPPORTED`.
- Insta snapshots pin `--dump ast` and `--dump model` output for both families.
- Docs explicitly name supported parser/model paths and unsupported render diagnostics.

## Current Implementation Notes

- Gantt and Chronology are normalized to dedicated `NormalizedDocument` variants.
- CLI `--check` and `--dump ast|model` are supported for the declared subset.
- SVG rendering remains out of scope and fails with family-specific render diagnostics.
