# Mermaid — Spec Ingestion + Architecture Notes

This directory is the **in-repo source-of-truth** for Mermaid's syntax and rendering
architecture. It exists so PUML can implement a Mermaid-compatible front-end without
shipping Mermaid as a runtime or git submodule. All material here was extracted from
the public mermaid-js repository (https://github.com/mermaid-js/mermaid, MIT licensed).

Ingestion date: **2026-06-01**
Upstream: depth-1 clone of `main`. The `package.json` is tagged `10.2.4` but the
source tree is post-v11 (`@mermaid-js/parser` is Langium-based, not Jison-only).

## Layout

```
docs/internal/spec/mermaid/
  README.md                                 — this index
  syntax/<type>.md                          — verbatim syntax docs per diagram type
  architecture/
    flowchart.jison                         — the canonical flowchart grammar (Jison)
    langium-parser-overview.md              — newer Langium-based parser overview
    syntax-reference-index.md               — top-level Mermaid syntax overview
    configuration.md                        — mermaid config schema overview
    theming.md                              — theme variable reference
    layouts.md                              — pluggable layout engines (dagre, elk, tidy-tree, cose-bilkent)
  examples/                                 — reserved for canonical .mmd fixtures
```

## Diagram families ingested

| Family | Mermaid keyword | Syntax doc | Notes |
|---|---|---|---|
| Flowchart | `flowchart` / `graph` | `syntax/flowchart.md` | **Spike target.** Jison grammar at `architecture/flowchart.jison`. |
| Sequence | `sequenceDiagram` | `syntax/sequenceDiagram.md` | Closest analog to PUML `sequence`. |
| Class | `classDiagram` | `syntax/classDiagram.md` | Direct analog to PUML `class`. |
| State | `stateDiagram` / `stateDiagram-v2` | `syntax/stateDiagram.md` | Maps to PUML `state`. |
| Gantt | `gantt` | `syntax/gantt.md` | Maps to PUML `gantt`. |
| Pie | `pie` | `syntax/pie.md` | No direct PUML analog; pie-chart only. |
| ER | `erDiagram` | `syntax/entityRelationshipDiagram.md` | Maps to PUML `chen` or class. |
| User journey | `journey` | `syntax/userJourney.md` | Could map to swimlane activity. |
| Git graph | `gitGraph` | `syntax/gitgraph.md` | Direct analog to PUML `gitgraph`. |
| Mindmap | `mindmap` | `syntax/mindmap.md` | Direct analog to PUML `mindmap`. |
| Requirement | `requirementDiagram` | `syntax/requirementDiagram.md` | SysML-style; no PUML equivalent yet. |
| C4 | `C4Context` / `C4Container` / etc. | `syntax/c4.md` | Direct analog to PUML `!include C4`. |
| Sankey | `sankey-beta` | `syntax/sankey.md` | Flow-quantity diagram; PUML has no analog. |
| Timeline | `timeline` | `syntax/timeline.md` | Maps to PUML `chronology`. |
| Quadrant | `quadrantChart` | `syntax/quadrantChart.md` | No PUML analog. |
| XY chart | `xychart-beta` | `syntax/xyChart.md` | Maps to PUML `chart`. |

## Mermaid architecture, at a glance

### Parsing

Mermaid is mid-migration from a Jison-based parser (per-family `.jison` files such as
`packages/mermaid/src/diagrams/flowchart/parser/flow.jison`, 634 lines) to a
**Langium**-based parser package (`packages/parser`). Both shapes coexist today:

- **Legacy (flowchart, sequence, class, state, gantt, etc.):** Each family ships a
  hand-written Jison grammar that produces an in-memory model object (the per-family
  "DB", e.g. `flowDb.ts`). The DB is mutated as the parser walks the grammar.
- **New (info, pie, packet, gitgraph in progress):** A single Langium-based parser
  in `@mermaid-js/parser` exposes `parse(diagramType, text)` and returns a typed AST.

Mermaid does NOT have a single unified IR — every family carries its own DB shape.
This is the opposite of PUML's `Document { kind, statements }` model.

### Layout

Mermaid is an SVG renderer running in the browser. It delegates **layout** to
pluggable engines (see `architecture/layouts.md`):

- **dagre** — default for flowchart / state / class. Layered Sugiyama-style layout.
- **elk** — Eclipse Layout Kernel, opt-in via `config: { layout: elk }`.
- **tidy-tree** — for hierarchical diagrams (mindmap, treeView).
- **cose-bilkent** — force-directed for some experimental families.

Layout is NOT in Mermaid's core — it is a JS dependency. Mermaid passes the
parsed graph to the layout engine, then walks the result and emits SVG.

### Rendering

Per-family renderers under `packages/mermaid/src/diagrams/<type>/` (e.g.
`flowRenderer-v3-unified.ts`) consume the laid-out graph and emit SVG via D3 selections.
Node shapes are hand-coded as SVG path builders (`flowChartShapes.js`).

### How would PUML plug in?

There are three viable integration points; we recommend (3):

1. **As a stand-alone front-end.** Add a `--mermaid` flag and a parallel parser tree
   that produces our existing `Document` IR. Reuse PUML's normalize → layout →
   render pipeline. Best for parity with our deterministic-output guarantee.
2. **As a transpiler.** Parse Mermaid, emit equivalent PUML text, then pipe through
   the existing parser. Cheapest but loses source-map fidelity and can't
   support Mermaid-only features (sankey, journey, quadrant) without first
   inventing PUML syntax for them.
3. **Recommended hybrid.** Per-family Mermaid parsers under `src/parser/mermaid/`
   that emit directly into the **shared IR** (`crate::ast::Document` for
   families with a PUML analog, new IR for Mermaid-only families). Detect via
   diagram-type keyword on the first non-blank line. Render via existing
   per-family renderers where the IR is shared; add new renderers only for
   Mermaid-only families.

This spike implements (3) for flowchart. It adds `src/parser/mermaid/`,
detects `flowchart`/`graph` as the first keyword, parses node declarations + arrows,
and translates to PUML's `DiagramKind::Component` + `FamilyRelation` model. Rendering
goes through the existing component renderer with zero modification.

## License + provenance

All material in `syntax/` and `architecture/` is copied verbatim from the
mermaid-js MIT-licensed source. We do not modify any file in those directories —
edit upstream and re-extract. The `examples/` directory is for PUML-authored
canonical fixtures.

## Follow-up tickets

The spike PR files P1 tickets for the remaining diagram families and architectural
extensions. See the PR body for the live list.
