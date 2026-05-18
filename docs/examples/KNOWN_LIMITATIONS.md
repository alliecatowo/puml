# Known Limitations (Current, May 2026)

This file tracks real constraints in the current implementation. It does not imply
that diagram families are unsupported overall.

## Scope of this page

- Focus: behavior gaps where PlantUML surface is broader than current runtime behavior.
- Non-goal: restating all supported features (see `README.md` and `docs/examples/GALLERY.md`).

## Current notable limitations

### Cross-cutting

- PNG output is implemented via deterministic SVG rasterization; advanced PlantUML raster flags beyond DPI remain out of scope.
- URL-based include/import sources are available in the native CLI for PlantUML compatibility, but disabled by default. Use `--allow-url-includes` only for trusted inputs. LSP and WASM surfaces do not fetch remote includes as side effects; see [`docs/url-includes.md`](../url-includes.md).
- Differential oracle is smoke-level in CI (not a full semantic pixel-parity oracle yet).

### Sequence

- `hide unlinked` is supported for sequence diagrams; explicit participants with no message, note, or lifecycle references are filtered.
- Teoz/parallel-message pragmas and `&` parallel messages are accepted with deterministic same-row placement, but advanced PlantUML teoz collision routing remains narrower.
- Some uncommon arrow/style combinations remain narrower than full PlantUML surface, though dotted `..>`-style sequence portability forms are accepted.

### Preprocessor / stdlib

- Dynamic invocation families (`%invoke_procedure`, `%call_user_func`) support deterministic callable dispatch, but edge cases remain narrower than PlantUML.
- Theme/source fetching from remote locations follows the URL include policy and remains narrower than PlantUML's full resolver surface.
- Stdlib macro compatibility is broad but not complete visual parity for every icon/macro variant.

### Family depth (not family availability)

- Families are implemented end-to-end, but feature depth still varies by family.
- Core UML broad partials include class-like declarations, inline node fills, object maps, use-case actors, component/deployment primitives, styled/decorated relations, state pseudostate basics, and activity branch/split/control tokens, but advanced semantic layout and styling remain partial relative to PlantUML breadth.
- Non-UML families (gantt/chronology/salt/mindmap/wbs/json/yaml/nwdiag/archimate/regex/ebnf/chart/math/sdl/ditaa) are implemented with deterministic render paths, but advanced semantics are still being expanded.
