# Renderer Refactor Roadmap

Status: operational architecture plan as of 2026-05-25.

This document is the compact assignment map for renderer architecture work. It
replaces ad hoc parity-audit and wave-log planning as the starting point for new
agents. It is not a support scoreboard. Use tests, examples, current GitHub
issues, and targeted PlantUML reference checks as evidence for specific behavior.

## Diagnosis

PUML has broad implemented behavior, but the architecture is still shaped around:

```text
PlantUML-ish source -> broad AST/model structs -> direct SVG strings
```

That shape causes the current problems:

- Mermaid and PicoUML still adapt through generated PlantUML strings in some
  paths, so source identity and feature-loss diagnostics need continued hardening.
- Some normalizers still match `StatementKind::Unknown(String)` beside the typed
  unsupported, deferred, malformed, and comment-lowered variants.
- Layout, routing, text measurement, color parsing, label placement, and SVG
  emission remain partly duplicated across diagram families.
- Typed scene validation exists, but many callers still only pass SVG/output bytes.
- Visual defects are geometry-contract defects: routes through group headers,
  detached labels, excessive whitespace, weak group/lane ownership, and
  inconsistent port/anchor selection.

## Target Shape

The long-term pipeline should be:

```text
frontend -> preprocess -> parse/lower -> normalize -> build_scene -> validate_scene -> backend
```

### `src/frontend/`

Own language identity and source mapping:

- `Frontend`
- `FrontendMatch`
- `FrontendDocument`
- `SourceFile`
- `SourceId`
- `SourceMap`
- `MappedSpan`

Mermaid and PicoUML are frontends with temporary PlantUML adapters. Their active
follow-up is to keep source spans and feature-loss diagnostics precise as more
syntax is migrated away from string lowering.

### Future `src/ir/`

Own language-neutral diagram data:

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

### Future `src/families/`

Own semantic family behavior. Each family should expose a narrow spec:

```rust
trait FamilySpec {
    type Ir;

    fn normalize(&self, doc: FrontendDocument) -> Result<Self::Ir, Diagnostic>;
    fn build_scene(&self, ir: &Self::Ir, ctx: &RenderContext)
        -> Result<RenderScene, Diagnostic>;
}
```

### `src/render_core/`

Own renderer-agnostic geometry, validation, and backend contracts:

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

SVG should be one backend, not the first place geometry becomes inspectable.

### Renderer Adapters

Renderers should become thin family adapters that build `RenderScene` or return
a render artifact carrying SVG plus optional typed scene data. Renderers should
not own generic text wrapping, edge routing, bbox math, marker definitions,
color parsing, or scene validity.

## Completed First Slice

These contracts landed in the 2026-05-24/25 merge wave. Treat them as baseline
architecture, not as new assignment targets:

- #1111: registry and scene-contract foundation.
- #1112: typed `render_core::RenderScene` geometry contract.
- #1113: initial visual quality gates and promoted corpus cases.
- #1114: frontend source-map and feature-loss diagnostic spine.
- #1115: PlantUML element registry and mixed-element graph-model foundation.
- #1116: authored Rust file-size split program and warning guardrail.
- #1117: shared diagnostics, output, text, style, geometry, and directive utilities.
- #1150: typed unsupported/deferred/malformed syntax variants beyond raw
  `Unknown(String)`.
- Refactor wave splits through #1160-#1180: parser, normalize, CLI, LSP, frontend,
  graph-layout, graph-family, sprite/icon, salt, timing, and shared render modules.

Do not reopen these broad issues for follow-up work. File or use focused issues
that name the remaining caller, contract, fixture, and verification command.

## Active Follow-Ups

Start here when assigning renderer architecture work:

- #1181: enforce the Rust file-size guardrail in CI. Do not perform more splits
  in that issue.
- #1182: return render artifacts carrying SVG, dimensions/format metadata,
  diagnostics, and optional typed `RenderScene` validation data.
- #1183: eliminate remaining normalizer paths that still treat
  `StatementKind::Unknown` like unsupported, deferred, malformed, or comment-lowered
  syntax.
- #1184: build the shared style cascade for theme presets, skinparam,
  stereotypes, inline tokens, and `<style>` blocks.
- #1185: add LSP workspace commands for `renderScene`, `export`, and
  `explainDiagnostic` through shared library APIs.
- #592: finish hierarchical graph-layout adoption across node-and-edge families.
- #593: converge orthogonal routing on shared route channels.
- #594: keep visual quality work tied to compactness, labels, routing, corpus
  baselines, and invariant tests.
- #816 and #1145: expand typed pre-SVG scene validation, especially through the
  graph-family render path.
- #870: continue converting family renderers around shared scene hooks and
  invariants.

The active issues above are the assignment surface. #590 remains the parent epic.
Closed epics such as #399, #436, and #525 are historical context only; do not
present them as active work.

## Next Migration Order

1. Enforce the file-size guardrail (#1181).
   - `scripts/check_rust_file_sizes.py --fail-on-violations` is already wired into
     `scripts/check-all.sh`; keep generated artifacts allowlisted only when they
     are actually generated.

2. Attach typed scenes to render artifacts (#1182, #1145).
   - Start with the graph-family path.
   - Preserve existing public SVG APIs with adapters.
   - Prove the attached scene is the scene validated before SVG output.

3. Finish typed unknown cleanup (#1183).
   - Keep intentional raw pass-through as `DeferredRaw` or another explicit typed
     category.
   - Do not collapse malformed, unsupported, comment-lowered, and deferred content
     back into one fallback state.

4. Promote shared style cascade call sites (#1184).
   - Start with one or two graph/sequence paths.
   - Keep current non-fatal warning behavior unless the issue explicitly changes it.

5. Expand renderer invariants and route channels (#592, #593, #594, #816, #870).
   - Move checks before SVG wherever typed scene data exists.
   - Keep post-SVG validation only as a compatibility backstop.

6. Add command surfaces on top of shared contracts (#1185).
   - LSP, CLI, WASM, Studio, and MCP should route through the same render and
     diagnostic APIs rather than re-parsing or reconstructing output locally.

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

- Finish removing remaining `StatementKind::Unknown` normalizer fallbacks after
  #1150.
- Keep Mermaid/PicoUML feature-loss diagnostics mapped to original source spans.
- Preserve unknown nested state blocks or diagnose them explicitly; do not skip them.
- Complete mixed-element graph support for `allowmixing` on top of the registry.
- Extend the PlantUML element registry for declarations, aliases, shape kind,
  stereotypes, legal families, parser, and renderer.
- Promote `<style>`, `skinparam`, stereotypes, and themes into the shared cascade.
- Expand deployment/component/usecase/class declaration vocabulary after the
  registry exists.
- Treat Gantt/chronology as baseline subsets until their own family model is deep
  enough for full status claims.

## Assignment Rule

Stale parity ledgers, dated wave logs, and closed broad epics should not be used
to assign work. Current issues should include concrete files, fixture paths,
acceptance criteria, and verification commands. If the docs and an executable
test disagree, trust the test and update the doc in the same PR.
