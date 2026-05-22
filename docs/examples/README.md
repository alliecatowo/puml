# Docs Examples Corpus

This directory is the canonical docs-as-tests corpus consumed by `scripts/render_check.py`.
The corpus is executable documentation and parity evidence, but it is not proof
of full PlantUML 1:1 parity and is not the source of truth for support status.
The authoritative current status is
[`docs/internal/spec/plantuml-spec.md`](../internal/spec/plantuml-spec.md) plus
the per-chapter audits under [`docs/internal/spec/audit/`](../internal/spec/audit/).

## Corpus location and size

- Root: `docs/examples/`
- Diagram sources: `293` `*.puml` files
- Render artifacts: `297` `*.svg` files
- Site gallery manifest: `287` paired family examples across `32` family directories
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
- If an example and the spec audit appear to disagree, treat the spec audit as authoritative and update the stale artifact or wording.
- When behavior changes, update the corresponding `.svg` artifacts in the same change.
