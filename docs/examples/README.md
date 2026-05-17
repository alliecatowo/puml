# Docs Examples Corpus

This directory is the canonical docs-as-tests corpus consumed by `scripts/parity_harness.py`.

## Corpus location and size

- Root: `docs/examples/`
- Diagram sources: `254` `*.puml` files
- Render artifacts: `258` `*.svg` files
- Additional docs/index files are also present in this tree

## Primary indexes

- [GALLERY.md](GALLERY.md): family-by-family browse index for examples and renders
- [KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md): current behavior gaps (feature-depth limits, not family rejection)
- [supported_primitives.md](supported_primitives.md): primitive-level quick reference

## Coverage summary (high level)

| Area | Status |
|---|---|
| Core UML families (sequence/class/object/usecase/component/deployment/state/activity/timing) | Implemented with mixed depth (`implemented` + `partial` features) |
| Non-UML families (salt/json/yaml/nwdiag/archimate/regex/ebnf/chart/math/sdl/ditaa) | Implemented baseline render paths; deeper semantics vary by family |
| Preprocessor/themes/skinparams/creole | Broad support with deterministic boundaries and documented partial areas |

## Notes

- This corpus is intentionally larger than minimal fixtures; it is used as executable documentation evidence.
- When behavior changes, update the corresponding `.svg` artifacts in the same change.
