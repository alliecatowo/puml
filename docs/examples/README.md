# Docs Examples Corpus

This directory is the canonical docs-as-tests corpus consumed by `scripts/parity_harness.py`.
The corpus is executable documentation and parity evidence, but it is not proof
of full PlantUML 1:1 parity and is not the source of truth for support status.
The canonical current status is
[`docs/internal/parity/plantuml_parity_source_of_truth.md`](../internal/parity/plantuml_parity_source_of_truth.md),
where support is tracked conservatively as `implemented`, `partial`, or `missing`.

## Corpus location and size

- Root: `docs/examples/`
- Diagram sources: `255` `*.puml` files
- Render artifacts: `258` `*.svg` files
- Site gallery manifest: `248` paired family examples across `31` family directories
- Additional docs/index files are also present in this tree

## Primary indexes

- [GALLERY.md](GALLERY.md): family-by-family browse index for examples and renders
- [KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md): current behavior gaps (feature-depth limits, not family rejection)
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
- If an example and the audit table appear to disagree, treat the audit table as authoritative and update the stale artifact or wording.
- When behavior changes, update the corresponding `.svg` artifacts in the same change.
