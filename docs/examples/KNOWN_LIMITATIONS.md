# Known Limitations (Current, May 2026)

This file tracks real constraints in the current implementation. It does not imply
that diagram families are unsupported overall.

## Scope of this page

- Focus: behavior gaps where PlantUML surface is broader than current runtime behavior.
- Non-goal: restating all supported features (see `README.md` and `docs/examples/GALLERY.md`).

## Current notable limitations

### Cross-cutting

- PNG output is implemented via deterministic SVG rasterization; advanced PlantUML raster flags beyond DPI remain out of scope.
- URL-based include/import sources are intentionally rejected for deterministic/safety reasons.
- Differential oracle is smoke-level in CI (not a full semantic pixel-parity oracle yet).

### Sequence

- `hide unlinked` is accepted as a hint/warning path; behavior is narrower than full PlantUML filtering semantics.
- Teoz/parallel-message pragmas are accepted as compatibility boundaries, but layout remains the standard deterministic sequence layout rather than PlantUML teoz-specific placement.
- Some uncommon arrow/style combinations remain narrower than full PlantUML surface.

### Preprocessor / stdlib

- Dynamic invocation families (`%invoke_procedure`, `%call_user_func`) are still narrower than PlantUML.
- Theme/source fetching from remote locations is intentionally constrained.
- Stdlib macro compatibility is broad but not complete visual parity for every icon/macro variant.

### Family depth (not family availability)

- Families are implemented end-to-end, but feature depth still varies by family.
- Core UML broad partials include class-like declarations, object maps, use-case actors, component/deployment primitives, state pseudostate basics, and activity branch/split/control tokens, but advanced semantic layout and styling remain partial relative to PlantUML breadth.
- Non-UML families (gantt/chronology/salt/mindmap/wbs/json/yaml/nwdiag/archimate/regex/ebnf/chart/math/sdl/ditaa) are implemented with deterministic render paths, but advanced semantics are still being expanded.
