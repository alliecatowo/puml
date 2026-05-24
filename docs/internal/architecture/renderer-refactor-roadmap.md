# Renderer Refactor Roadmap

Status: current architecture plan as of 2026-05-24.

This document is the durable result of a fresh code, issue, and visual corpus
investigation. It replaces ad hoc parity-audit and wave-log planning as the
starting point for renderer architecture work. It is not a support scoreboard.
Use tests, examples, current GitHub issues, and targeted PlantUML reference
checks as evidence for specific behavior.

## Diagnosis

PUML has broad implemented behavior, but the architecture is still shaped around:

```text
PlantUML-ish source -> broad AST/model structs -> direct SVG strings
```

That shape causes the current problems:

- Mermaid and PicoUML adapt through generated PlantUML strings, losing source
  identity and making diagnostics imprecise.
- `StatementKind::Unknown(String)` represents too many states: unsupported
  syntax, deferred raw content, malformed content, benign pass-through, and
  feature loss.
- Layout, routing, text measurement, color parsing, label placement, and SVG
  emission are duplicated across diagram families.
- Visual checks happen too late. `src/render/validate.rs` reconstructs geometry
  from serialized SVG instead of validating a typed scene before backend output.
- The largest files are no longer maintainable as ownership units:
  `src/render/family.rs`, `src/normalize/family.rs`, `src/main.rs`,
  `src/render/state.rs`, `src/theme.rs`, `src/parser/family.rs`,
  `src/layout.rs`, and `src/render/graph_layout.rs`.

The main visual defects are not isolated SVG polish. They are missing shared
geometry contracts: relation routes through group headers, detached labels,
excessive whitespace, weak group/lane ownership, and inconsistent port/anchor
selection.

## Target Shape

The long-term pipeline should be:

```text
frontend -> preprocess -> parse/lower -> normalize -> build_scene -> validate_scene -> backend
```

### `src/frontend/`

Own language identity and source mapping.

- `Frontend`
- `FrontendMatch`
- `FrontendDocument`
- `SourceFile`
- `SourceId`
- `SourceMap`
- `MappedSpan`

PlantUML, Mermaid, and PicoUML should be sibling frontends. Mermaid and PicoUML
may keep temporary PlantUML adapters during migration, but those adapters must
report feature loss instead of silently lowering unknown lines to comments.

### `src/ir/`

Own language-neutral diagram data.

- `DiagramIr`
- `DiagramHeader`
- `DiagramMetadata`
- `StyleSet`
- `GraphIr`
- `SequenceIr`
- `StatechartIr`
- `ActivityIr`
- `TimelineIr`
- `TreeIr`
- `StructuredDataIr`
- `RawDiagramIr`

### `src/families/`

Own semantic family behavior.

Initial modules:

- `sequence`
- `graph`
- `statechart`
- `activity`
- `timeline`
- `tree`
- `wireframe`
- `structured`
- `raw`

Each family should expose a narrow spec:

```rust
trait FamilySpec {
    type Ir;

    fn normalize(&self, doc: FrontendDocument) -> Result<Self::Ir, Diagnostic>;
    fn build_scene(&self, ir: &Self::Ir, ctx: &RenderContext)
        -> Result<RenderScene, Diagnostic>;
}
```

### `src/render_core/`

Own renderer-agnostic geometry and backend contracts.

Core types:

- `RenderScene`
- `SceneNode`
- `SceneEdge`
- `SceneGroup`
- `SceneLabel`
- `Shape`
- `Point`
- `Size`
- `Rect`
- `Insets`
- `Polyline`
- `Segment`
- `Port`
- `Anchor`
- `LabelBox`
- `NodeBox`
- `GroupFrame`
- `LaneFrame`
- `RouteChannel`
- `GeometryIssue`
- `Style`
- `Layer`

Core traits:

- `LayoutStrategy`
- `Router`
- `LabelPlacer`
- `ShapeRenderer`
- `Backend`

SVG should become one backend, not the first place geometry becomes inspectable.

### `src/renderers/`

Renderers should become thin family adapters that build `RenderScene`.

Suggested modules:

- `sequence.rs`
- `graph/adapter.rs`
- `graph/class.rs`
- `graph/component.rs`
- `graph/deployment.rs`
- `graph/usecase.rs`
- `graph/c4.rs`
- `graph/shapes.rs`
- `graph/relations.rs`
- `graph/projections.rs`
- `statechart.rs`
- `activity.rs`
- `timing.rs`
- `tree.rs`
- `salt.rs`
- `chart.rs`

Renderers should not own generic text wrapping, edge routing, bbox math, marker
definitions, color parsing, or scene validity.

## Migration Order

1. Add guardrails.
   - Add a warning-only authored Rust file line-count check with a 600-line target.
   - Allowlist generated icon tables and other generated artifacts.
   - Do not block work until the first split issues are open and assigned.

2. Introduce the frontend boundary.
   - Add `SourceMap` and `FrontendResult`.
   - Wrap existing Mermaid/PicoUML adapters so diagnostics map back to original
     source spans.
   - Stop silent comment-lowering from counting as clean success.

3. Replace broad dispatch with a registry.
   - Add `DiagramRegistry` or equivalent family registry.
   - Move parser aliases, normalizer selection, renderer selection, text output,
     metadata support, LSP support, and frontend compatibility into registry data.

4. Extract shared low-risk modules.
   - `src/diagnostic.rs`: public diagnostic code splitting, line/column helpers,
     warning extraction, render options.
   - `src/output.rs`: CLI/watch/WASM output conversion and format metadata.
   - `src/render/text_metrics.rs`: shared wrapping and measurement.
   - `src/theme/color.rs` or `src/style.rs`: color tokens and line style parsing.
   - `src/normalize/common.rs`: title/header/footer/caption/legend/scale/common
     directive handling.

5. Add typed geometry.
   - Create the `render_core` geometry primitives.
   - Migrate duplicate `Rect`/`Segment`/intersection helpers from family,
     activity, state, graph layout, and SVG validation.

6. Move validation before SVG.
   - Keep `src/render/validate.rs` as a compatibility backstop initially.
   - Add pre-SVG checks for routes, labels, groups, ports, and viewport bounds.

7. Split graph layout and graph renderers.
   - Split `src/render/graph_layout.rs` into `types`, `rank`, `crossing`,
     `coordinates`, `groups`, and `router`.
   - Split `src/render/family.rs` into graph-family modules.
   - Route class/object/usecase/component/deployment/C4 through one graph adapter.

8. Converge sequence and state label geometry.
   - Replace sequence-only raw `src/scene.rs` usage with either a
     `sequence::SceneDraft` or the shared scene contract.
   - Use one `LabelPlacer` for state transition labels, sequence ref/group labels,
     graph edge labels, and timing/chart labels where applicable.

9. Split CLI/LSP/WASM orchestration.
   - Split `src/main.rs` into CLI run/input/output/dump/diagnostic modules.
   - Make library compile/render APIs authoritative for CLI, watch, LSP, WASM,
     Studio, and MCP surfaces.

## Invariants To Enforce

These should be expressed as typed scene checks where possible:

- Deterministic output ordering.
- Source spans survive preprocessing, includes, Mermaid, and PicoUML adaptation.
- Unknown or unsupported syntax is explicit; feature loss cannot silently render.
- SVG dimensions and viewBox contain every visible element.
- Edge routes do not cross non-endpoint nodes.
- Edge routes do not cross package headers or group labels.
- Edge endpoints attach to declared ports/anchors.
- Labels stay close to their owning route segment.
- Labels do not sit directly on strokes unless intentionally backed.
- Child nodes stay inside parent group/lane content areas.
- Lanes own the actions visually assigned to them.
- Route channels may cross group frames only when semantically entering or exiting.
- Canvas aspect ratio and empty gutters are tracked as non-fatal quality metrics.
- Theme and skinparam warnings remain non-fatal where they are currently non-fatal.
- Creole, sprites, inline icons, and text escaping preserve current behavior.

## Visual Cases To Promote

Promote these to visual baseline or invariant fixtures after review:

- `docs/examples/component/07_ports_lollipop_interfaces.puml`
- `docs/examples/deployment/06_kubernetes_pods_containers.puml`
- `docs/examples/deployment/05_three_tier_cloud_onprem.puml`
- `docs/examples/usecase/06_multi_system_boundary.puml`
- `docs/examples/class/32_association_class_deep_packages.puml`
- `docs/examples/class/31_generic_types_container.puml`
- `docs/examples/sequence/48_complex_ref_over_multibox.puml`
- `docs/examples/activity/16_nested_swimlanes_parallel_forks.puml`
- `docs/examples/activity_new/08_notes_split_partitions.puml`
- `docs/examples/state/09_three_level_composite.puml`
- `docs/examples/state/10_parallel_regions_shared_events.puml`
- `docs/examples/timing/05_concurrent_timelines_message_arrows.puml`
- `docs/examples/c4/11_system_landscape.puml`
- `docs/examples/chart/06_multi_series_line.puml`
- `docs/diagrams/language-service-layers.puml`
- `docs/diagrams/architecture-overview.puml`

## Feature Gaps That Affect Architecture

Do not add these as one-off syntax patches without checking whether the shared
contract should change first:

- Replace `StatementKind::Unknown(String)` with typed unsupported/deferred/malformed
  variants.
- Report Mermaid/PicoUML feature loss instead of silently emitting comments.
- Preserve unknown nested state blocks or diagnose them; do not skip them.
- Add true mixed-element graph support for `allowmixing`.
- Build a PlantUML element registry for declarations, aliases, shape kind,
  stereotypes, legal families, parser, and renderer.
- Promote `<style>`, `skinparam`, stereotypes, and themes into a shared cascade.
- Expand deployment/component/usecase/class declaration vocabulary after the
  registry exists.
- Treat Gantt/chronology as baseline subsets until their own family model is deep
  enough for full status claims.

## Issue Program

The issue board should be organized around this backbone:

1. Renderer architecture registry and scene contract.
2. Typed render scene and pre-SVG invariants.
3. Graph layout adoption and legacy grid deletion.
4. Shared geometry/router/label placer.
5. `<600 LOC` authored-file split program.
6. Frontend source-map and typed frontend results.
7. Style cascade and element registry.
8. Visual quality corpus and invariant gates.

Stale parity ledgers and dated wave logs should not be used to assign work.
Current issues should include concrete files, fixture paths, acceptance criteria,
and verification commands.
