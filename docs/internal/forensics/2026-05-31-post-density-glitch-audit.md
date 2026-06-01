# 2026-05-31 Post-density / post-W22 visual glitch audit

**Auditor:** Claude Opus 4.7 (subagent)
**Branch:** `chore/wave-23-glitch-audit-forensic-doc`
**Scope:** 35-fixture PUML corpus rendered with binary at commit `04d0bb68` (top of `main`).
**Recently-merged PRs in scrutiny window:**

| PR | Subject |
| --- | --- |
| #1438 | fan actor→usecase edges 20 px |
| #1437 | component family per-node density retune |
| #1435 | class family density retune |
| #1431 | deployment family per-shape density retune |
| #1432 | remove opaque header-band repaint |
| #1387 | label collision push + frame width fits header |
| #1409 | spot stereotype badge |
| #1408 | stereotype-scoped skinparam block |
| #1407 | inline relation tail-style |
| #1406 | inline sprite |

**Method:** rendered all 35 fixtures from the parity corpus to PNG in `/tmp/glitch_hunt/`, then Read every PNG. Cross-checked suspicious cases against `/tmp/parity_audit_v4/*-PlantUML.png` references. For each finding, inspected the generated SVG to confirm a geometric root cause (not just an aesthetic difference).

## Findings summary

12 real glitches found across the corpus. Each is filed as a separate GitHub issue and linked below.

| # | Issue | Priority | Fixture | One-liner |
| --- | --- | --- | --- | --- |
| 1 | [#1440](https://github.com/alliecatowo/puml/issues/1440) | P0 | `deployment/06_kubernetes_pods_containers.puml` | Kubernetes Cluster outer group placed at `y=-34`; header clipped above viewBox; visible dark sliver at top of PNG |
| 2 | [#1441](https://github.com/alliecatowo/puml/issues/1441) | P1 | `docs/diagrams/architecture-overview.puml` | Stray `uml-edge-label-bg` white rects layered on top of package header bands; "package Frontends / Shared Services / Pipeline Core / Output Formats" headers visibly mottled |
| 3 | [#1442](https://github.com/alliecatowo/puml/issues/1442) | P1 | `deployment/06_kubernetes_pods_containers.puml` | "Ingress Controller" (162 px), "queue-consumer" (126 px), "sidecar-logger" (126 px) overflow 110 px node bbox |
| 4 | [#1443](https://github.com/alliecatowo/puml/issues/1443) | P1 | `component/07_ports_lollipop_interfaces.puml` | "NotificationSender", "IOrderRepository", "OrderController", "OrderRepository" overflow 130 px component bbox |
| 5 | [#1444](https://github.com/alliecatowo/puml/issues/1444) | P2 | `deployment/03_cloud.puml` | "Lambda Function" (135 px) overflows 110 px node bbox |
| 6 | [#1445](https://github.com/alliecatowo/puml/issues/1445) | P1 | `usecase/05_actor_generalization_system_boundary.puml` | Vertical edge fan tangles through Premium User actor; `<<extend>>` orphaned; >3-actor fan exceeds available width |
| 7 | [#1446](https://github.com/alliecatowo/puml/issues/1446) | P1 | `usecase/06_multi_system_boundary.puml` | Actor edges pass through all three system-boundary frame headers and through unrelated usecase ovals; long back-edge crosses all frames |
| 8 | [#1447](https://github.com/alliecatowo/puml/issues/1447) | P1 | `activity/09_error_handling.puml` | Extra stop circle overlapping Complete node; yes/no branch labels swapped vs. PlantUML reference; stray "yes" label far from any edge |
| 9 | [#1448](https://github.com/alliecatowo/puml/issues/1448) | P1 | `state/10_parallel_regions_shared_events.puml` | Bidirectional transition labels (play/pause, stop/resume, unmute/mute) stack on top of each other; PR #1387 push pass not detecting parallel-edge pairs |
| 10 | [#1449](https://github.com/alliecatowo/puml/issues/1449) | P2 | `state/07_nested.puml` | "data ready" transition label rendered orphaned, no visible attached edge |
| 11 | [#1450](https://github.com/alliecatowo/puml/issues/1450) | P1 | `component/07_ports_lollipop_interfaces.puml` | "uses" edge segment crosses OrderRepository node body; "SQL" label overlaps parallel edges |
| 12 | [#1451](https://github.com/alliecatowo/puml/issues/1451) | P1 | `component/08_cloud_db_queue_stereotypes.puml` | "origin pull" arrowhead lands inside API Cluster header band region; regression introduced when PR #1432 removed band repaint |

**Total real glitches: 12** (1 P0, 9 P1, 2 P2).

## Glitch families

### Family A: density retune dropped node width below text minimum

Affects [#1442](https://github.com/alliecatowo/puml/issues/1442), [#1443](https://github.com/alliecatowo/puml/issues/1443), [#1444](https://github.com/alliecatowo/puml/issues/1444).

Recently-merged density retunes (PR #1431 deployment, PR #1437 component) shrank node widths to hit density targets but skipped the `width = max(density_target, measure_text(label) + 2 * padding)` floor.

**Shared fix:** all `*_density_retune.rs` paths should clamp final width to the maximum of the density target and the longest contained text run. Add an invariant test that walks every node in the corpus and asserts `node.width >= label_width + 2 * h_padding`.

### Family B: usecase edge routing does not avoid obstacles

Affects [#1445](https://github.com/alliecatowo/puml/issues/1445), [#1446](https://github.com/alliecatowo/puml/issues/1446).

PR #1438 introduced a 20 px horizontal fan for actor→usecase edges but the routing still treats only the immediate source/target as anchors; intermediate actor figures and intermediate system-boundary frames are not in the obstacle set.

**Shared fix:** for the usecase family, include all sibling actor figures and all group frames (boundary frames) in the obstacle set used by the orthogonal/spline router. Consider redistributing actor x-positions before edge routing rather than fanning endpoints after-the-fact.

### Family C: label-collision push pass has blind spots

Affects [#1441](https://github.com/alliecatowo/puml/issues/1441) (group header text incorrectly treated as collision target), [#1448](https://github.com/alliecatowo/puml/issues/1448) (bidirectional edges' labels not detected as colliding pair).

PR #1387 added a collision-only push pass. It misses two cases:

1. Group-frame header `<text>` should be excluded from the edge-label background pass (the header has its own dark band and white text; an opacity-0.85 white rect on top mottles it).
2. Two opposite-direction edges between the same node pair render their labels at overlapping anchors; the push pass does not detect this configuration as a collision because each label is alone at its own midpoint coordinate before push.

**Shared fix:** exclude `data-uml-label-role="group-header"` (or equivalent) from the edge-label-bg pass; treat reverse-direction edges between the same node pair as a single "label cluster" and force-split their label anchors.

### Family D: header-band edge-endpoint interaction

Affects [#1451](https://github.com/alliecatowo/puml/issues/1451).

PR #1432 removed an opaque header-band repaint that was hiding cross-package edges. The fix worked for the *body* of cross-package edges, but the *endpoint* of an entering edge now visually intrudes into the band.

**Shared fix:** clip edge segments to stop at the node-area boundary (band-bottom), not at the node-rect top. Or, draw the band *after* edges with a transparent fill so edges pass through cleanly but the text remains readable.

### Family E: layout/geometry bugs unrelated to recent PRs

Affects [#1440](https://github.com/alliecatowo/puml/issues/1440) (kubernetes cluster at y=-34), [#1447](https://github.com/alliecatowo/puml/issues/1447) (activity extra stop + swapped labels), [#1449](https://github.com/alliecatowo/puml/issues/1449) (state orphan label), [#1450](https://github.com/alliecatowo/puml/issues/1450) (component edge-through-node).

Likely pre-existing bugs surfaced by the audit rather than caused by recent merges. The Kubernetes y=-34 bug is the most serious (P0) — the outermost group frame is placed above the viewBox and its header label is clipped entirely.

## Methodology notes

- Re-rendered the full 35-fixture corpus from `04d0bb68` rather than using the v4 cache, because every renderer-touching PR in the scrutiny window invalidated the cache.
- Used `grep` over the generated SVG to confirm geometric root causes (negative y coordinates, polyline points, label-bg rect positions) rather than guessing from PNGs.
- Did **not** file tickets for chrome/style differences (theme, font sizing, header band color) — those are scoped to #1375 / PUML-mode-vs-PlantUML.
- Did **not** file tickets for density-only differences where layout was unchanged but boxes are tighter — those are tracked under the density retune PRs themselves.
- Diagrams that audited clean: activity_05_while_loop, activity_07_partition, class_03_composition_aggregation, class_05_visibility, class_11_generics, component_02_interfaces (acceptable orthogonal routing), gantt_05_multi_task, mindmap_02_multi_level, mindmap_05_four_levels, nwdiag_02_multi_network, object_02_with_attributes, object_05_ch04_parity (small diamond is intentional layout), salt_01_basic_widgets, sequence_03_autonumber, sequence_07_notes, sequence_11_activation, sequence_12_create_destroy, state_03_concurrent, timing_01_concise, wbs_02_with_tasks, c4_12_container_with_databases (passes with minor edge crossings but no overflow), deployment_02_databases, class_01_basic.

## Confidence

Found **12 real glitches** across the 35-fixture corpus. Each is grounded in either a measured SVG coordinate, a comparison with the cached PlantUML reference, or both. None of the findings is a chrome/style nit or a density-tracked artifact.
