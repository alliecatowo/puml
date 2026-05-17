# Known Limitations (Current, May 2026)

This file tracks real constraints in the current implementation. It does not imply
that diagram families are unsupported overall.

## Scope of this page

- Focus: behavior gaps where PlantUML surface is broader than current runtime behavior.
- Non-goal: restating all supported features (see `README.md` and `docs/examples/GALLERY.md`).

## Current notable limitations

### Cross-cutting

- PNG output is not implemented (`--format png` is a deterministic rejection).
- URL-based include/import sources are intentionally rejected for deterministic/safety reasons.
- Differential oracle is smoke-level in CI (not a full semantic pixel-parity oracle yet).

### Sequence

- `hide unlinked` is accepted as a hint/warning path; behavior is narrower than full PlantUML filtering semantics.
- Teoz/parallel-message semantics are not fully equivalent to PlantUML teoz behavior.
- Some uncommon arrow/style combinations remain narrower than full PlantUML surface.

### Preprocessor / stdlib

- Dynamic invocation families (`%invoke_procedure`, `%call_user_func`) are still narrower than PlantUML.
- Theme/source fetching from remote locations is intentionally constrained.
- Stdlib macro compatibility is broad but not complete visual parity for every icon/macro variant.

### Family depth (not family availability)

- Families are implemented end-to-end, but feature depth still varies by family.
- Some advanced constructs in component/deployment/state/activity/timing/class/object/usecase are partial relative to PlantUML breadth.
- Non-UML families (salt/json/yaml/nwdiag/archimate/regex/ebnf/chart/math/sdl/ditaa) are implemented with deterministic render paths, but advanced semantics are still being expanded.
