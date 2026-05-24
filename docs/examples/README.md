# Docs Examples Corpus

This directory is the docs-as-tests corpus consumed by `scripts/render_check.py`.
The corpus is executable documentation and compatibility evidence, but it is not
proof of full PlantUML 1:1 parity and is not a support-status authority. Current
planning lives in [`docs/parity-roadmap.md`](../parity-roadmap.md) and focused
GitHub issues.

## Corpus location and size

- Root: `docs/examples/`
- Diagram sources: `298` `*.puml` files
- Render artifacts: `302` `*.svg` files
- Render-check scope: `301` total examples (`298` passing, `3` intentionally excluded)
- Site gallery manifest: paired family examples across `32` family directories
- Additional docs/index files are also present in this tree

## Primary indexes

- [GALLERY.md](GALLERY.md): family-by-family browse index for examples and renders
- [supported_primitives.md](supported_primitives.md): primitive-level quick reference

## Coverage evidence summary (high level)

| Area | Status |
|---|---|
| Core UML families (sequence/class/object/usecase/component/deployment/state/activity/timing) | Exercised with mixed depth; many core rows are implemented, while advanced rows remain partial |
| Non-UML families (gantt/chronology/salt/mindmap/wbs/json/yaml/nwdiag/archimate/regex/ebnf/chart/math/sdl/ditaa) | Baseline render paths are exercised for many families; deeper semantics vary and several advanced rows remain partial |
| Preprocessor/themes/skinparams/creole | Broad exercised support with deterministic boundaries and documented partial areas |

## Notes

- This corpus is intentionally larger than minimal fixtures; it is used as executable documentation evidence.
- Examples are coverage seeds for implemented behavior, not exhaustive proof of PlantUML compatibility.
- If an example, test, and planning doc disagree, verify current behavior with the
  renderer and update the stale artifact or wording.
- When behavior changes, update the corresponding `.svg` artifacts in the same change.
