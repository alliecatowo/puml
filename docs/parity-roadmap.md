# Compatibility Roadmap

This page is a short planning index for PlantUML compatibility work. It is not a
support scoreboard, not a parity source of truth, and not a place to preserve
dated wave logs. Current implementation evidence lives in tests, examples,
rendered fixtures, and the linked GitHub issues.

For renderer architecture and visual-quality work, start with
[`docs/internal/architecture/renderer-refactor-roadmap.md`](internal/architecture/renderer-refactor-roadmap.md).

## Current Direction

The next compatibility work should serve the architecture program instead of
adding isolated one-off syntax patches:

1. Make unsupported syntax explicit instead of overloading `Unknown(String)`.
2. Add source maps for PlantUML, Mermaid, and PicoUML frontends.
3. Build a mixed-element graph model for class/object/usecase/component/deployment/C4.
4. Move style/theme/skinparam handling into a shared cascade.
5. Validate typed render scenes before SVG serialization.
6. Promote high-risk visual cases into baselines and geometry invariant tests.

## Active Issue Backbone

- [#590](https://github.com/alliecatowo/puml/issues/590) — renderer architecture:
  shared layout, routing, scene hooks, and invariant gates.
- [#870](https://github.com/alliecatowo/puml/issues/870) — shared scene hooks and
  canonical render contracts.
- [#816](https://github.com/alliecatowo/puml/issues/816) — render invariants.
- [#399](https://github.com/alliecatowo/puml/issues/399) — shared language service
  and compile surface.
- [#400](https://github.com/alliecatowo/puml/issues/400) — compile/language API and
  worker protocol.
- [#402](https://github.com/alliecatowo/puml/issues/402) — syntax package and editor
  grammar coverage.
- [#738](https://github.com/alliecatowo/puml/issues/738) — style block support.
- [#450](https://github.com/alliecatowo/puml/issues/450) — theme presets across
  families.

## Missing Or Partial Families

These are useful compatibility slices, but they should not bypass the shared
frontend, style, and scene contracts:

- [#725](https://github.com/alliecatowo/puml/issues/725) — WireDiagram
  (`@startwire`).
- [#448](https://github.com/alliecatowo/puml/issues/448) — Board/Kanban and Files.
- [#447](https://github.com/alliecatowo/puml/issues/447) — Chen/EER.
- [#727](https://github.com/alliecatowo/puml/issues/727) — chronology rendering.
- [#726](https://github.com/alliecatowo/puml/issues/726) — nwdiag groups and
  multi-network topology.
- [#1090](https://github.com/alliecatowo/puml/issues/1090) — stdlib inventory and
  include parity.

## Fixtures To Keep Visible

These examples should remain part of compatibility and visual-quality planning.
They are intentionally listed as concrete paths so tests can verify references:

- `tests/fixtures/basic/hello.puml`
- `tests/fixtures/families/valid_class_with_relations.puml`
- `tests/fixtures/families/valid_component.puml`
- `tests/fixtures/families/valid_deployment.puml`
- `tests/fixtures/families/valid_state.puml`
- `tests/fixtures/families/valid_timing_advanced_geometry.puml`
- `tests/fixtures/families/valid_salt_bootstrap.puml`
- `tests/fixtures/non_sequence/valid_json.puml`
- `tests/fixtures/non_sequence/valid_yaml.puml`
- `tests/fixtures/non_sequence/valid_nwdiag.puml`
- `tests/fixtures/non_sequence/valid_archimate.puml`
- `tests/fixtures/non_sequence/valid_regex.puml`
- `tests/fixtures/non_sequence/valid_ebnf.puml`
- `tests/fixtures/non_sequence/valid_math.puml`
- `tests/fixtures/non_sequence/valid_ditaa.puml`
- `tests/fixtures/non_sequence/valid_chart_bar.puml`
- `tests/fixtures/timeline/valid_gantt_baseline.puml`
- `tests/fixtures/timeline/valid_chronology_baseline.puml`

## Visual Cases To Promote

These examples should become reviewed visual baselines or typed geometry
invariant fixtures:

- `docs/examples/component/07_ports_lollipop_interfaces.puml`
- `docs/examples/deployment/06_kubernetes_pods_containers.puml`
- `docs/examples/usecase/06_multi_system_boundary.puml`
- `docs/examples/class/32_association_class_deep_packages.puml`
- `docs/examples/sequence/48_complex_ref_over_multibox.puml`
- `docs/examples/activity/16_nested_swimlanes_parallel_forks.puml`
- `docs/examples/state/09_three_level_composite.puml`
- `docs/examples/timing/05_concurrent_timelines_message_arrows.puml`
- `docs/examples/c4/11_system_landscape.puml`
- `docs/examples/chart/06_multi_series_line.puml`
- `docs/diagrams/language-service-layers.puml`
- `docs/diagrams/architecture-overview.puml`

## Maintenance Rule

When a compatibility ticket lands, update the owning issue and the executable
tests or fixtures. Do not create another broad parity ledger unless it has a
specific parser, test, or release gate consuming it.
