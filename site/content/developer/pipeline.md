+++
title = "Compile pipeline"
description = "Source text to deterministic SVG, stage by stage."
weight = 20
+++

Every input &mdash; from any frontend dialect, in any rendering mode &mdash; flows through five stages.

```text
source text
   │
   ▼  parse
[ AST ]                       span-rich, dialect-shaped
   │
   ▼  normalize
[ normalized model ]          canonical semantic representation
   │
   ▼  layout
[ scene ]                     positioned geometry, theme tokens folded in
   │
   ▼  render
[ SVG / PNG ]                 deterministic bytes
```

## 1. Parse

`src/parser.rs` is built on [winnow](https://crates.io/crates/winnow). Each block marker (`@startuml`, `@startclass`, `@startgantt`, etc.) routes into a family-specific parser. Diagnostics use spans referencing the original source so editor tooling can underline the right characters.

Dump the AST:

```bash
puml --dump ast input.puml | jq .
```

## 2. Normalize

`src/normalize.rs` converts the family-shaped AST into the canonical model. PlantUML, PicoUML, and Mermaid all converge here.

For Mermaid, the adapter is responsible for collapsing dialect-specific shapes (e.g. `flowchart` &rarr; PlantUML component-style) into the same model nodes.

Dump the model:

```bash
puml --dump model input.puml | jq .
```

## 3. Model

`src/model.rs` defines the canonical types: `SequenceDocument`, `ClassDocument`, `StateDocument`, `TimelineDocument`, `ChartDocument`, etc., all wrapped in a `FamilyDocument` discriminant and a `NormalizedDocument` container that carries theme tokens.

## 4. Layout

`src/layout.rs` provides shared primitives (text measurement, box-packing, lifeline geometry, edge routing). Each family-specific renderer in `src/render.rs` calls into layout for geometry and then emits SVG.

## 5. Render

`src/render.rs` is per-family:

- `render_svg` &mdash; sequence
- `render_class_svg`, `render_family_tree_svg`
- `render_activity_svg`
- `render_state_svg`
- `render_timing_svg`
- `render_gantt_svg`, `render_chart_svg`, `render_mindmap_svg`, `render_wbs_svg`
- `render_salt_svg`, `render_archimate_svg`, `render_nwdiag_svg`
- `render_json_svg`, `render_yaml_svg`, `render_regex_svg`, `render_ebnf_svg`
- `render_math_svg`, `render_ditaa_svg`, `render_sdl_svg`
- `render_component_svg`, `render_deployment_svg`

The PNG path rasterizes the same SVG through `resvg` / `tiny-skia` &mdash; SVG remains the canonical artifact.

## Dump intermediate stages

```bash
puml --dump ast    input.puml > ast.json
puml --dump model  input.puml > model.json
puml --dump scene  input.puml > scene.json
```

These are stable JSON shapes the test suite asserts against; they're also the right unit of work for any tool wanting to introspect or transform diagrams.

## Tracing

```bash
puml --verbose --duration input.puml
# stderr: per-stage parse/normalize/render timings and total wall time
```

## Where to start reading code

1. `src/lib.rs` &mdash; the public API and `render_source_to_svg*` entry points.
2. `src/parser.rs` &mdash; how block markers route into family parsers.
3. `src/model.rs` &mdash; the canonical types.
4. `src/render.rs` &mdash; the per-family deterministic emitters.
