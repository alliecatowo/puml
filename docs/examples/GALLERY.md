# PicoUML Example Gallery (Current)

This gallery indexes the docs-as-tests corpus under `docs/examples/`.

## Corpus totals

- `298` source diagrams (`*.puml`)
- `302` render artifacts (`*.svg`)
- Location: `docs/examples/` and its family subdirectories

## Layout-Stressing Fixtures

### sequence/
- `46_nested_alt_with_par.puml` — 3-branch alt with activate/deactivate nesting
- `47_create_participant_midflow.puml` — dynamic `create` + `destroy` lifecycle
- `48_complex_ref_over_multibox.puml` — multi-box grouping, autonumber stop/resume, ref-over

### activity/
- `16_nested_swimlanes_parallel_forks.puml` — 4-lane swimlane with parallel fork/join
- `17_switch_case_with_detach.puml` — switch/case with detach and kill terminals
- `18_repeat_while_nested_partition.puml` — repeat-while inside partition with parallel fork (ETL)

### state/
- `09_three_level_composite.puml` — 3-level nested composite states
- `10_parallel_regions_shared_events.puml` — parallel state machines sharing powerOff event
- `11_entry_exit_actions_history.puml` — entry/exit/do-activity annotations; history [H] pseudo-state

### component/
- `07_ports_lollipop_interfaces.puml` — provided/required lollipop interfaces
- `08_cloud_db_queue_stereotypes.puml` — multi-package component graph (CDN/API/Storage/EventBus)

### usecase/
- `05_actor_generalization_system_boundary.puml` — 3-level actor hierarchy, extend/include
- `06_multi_system_boundary.puml` — 3 system boundaries, automation triggers

### deployment/
- `05_three_tier_cloud_onprem.puml` — cloud + VPN + on-prem nesting, replication links
- `06_kubernetes_pods_containers.puml` — namespace/pod/container nesting, StatefulSet

### gantt/
- `07_dependencies_with_lag_holidays.puml` — lag-start dependencies, weekend closures
- `08_milestones_critical_path.puml` — milestone `happens at` markers, sprint milestone pattern

### class/
- `31_generic_types_container.puml` — generic type params (`Collection<T>`, `Map<K,V>`, `Optional<T>`)
- `32_association_class_deep_packages.puml` — 3-level packages, association classes

### c4/
- `11_system_landscape.puml` — 5+ systems, 3 personas, external partners (system landscape)
- `12_container_with_databases.puml` — DB, cache, message bus, external services

### mindmap/
- `05_four_levels_asymmetric.puml` — 5 levels deep, asymmetric branching
- `06_multiline_node_labels.puml` — multiline labels with `\n` across 4 levels

### wbs/
- `05_four_levels_deep.puml` — 4-level WBS (platform migration)

### json/
- `04_deep_nesting_arrays_of_objects.puml` — nested objects + arrays-of-objects, null values

### timing/
- `05_concurrent_timelines_message_arrows.puml` — 4 concise lanes + clock (CPU/cache/memory/IO)
- `06_robust_states_value_annotations.puml` — robust timing with string value annotations
- `10_manual_time_axis.puml` — manual axis labels only on state-change ticks

### chart/
- `05_stacked_bar.puml` — stacked bar chart with 3 series (6 months)
- `06_multi_series_line.puml` — multi-series line chart (desktop/mobile/tablet, 8 weeks)

## Family directories

### Core UML families

- `sequence/`
- `class/`
- `object/`
- `usecase/`
- `component/`
- `deployment/`
- `state/`
- `activity/`
- `activity_new/`
- `activity_old/`
- `timing/`

### Timeline and planning

- `gantt/`
- `chronology/`

### Non-UML / specialized families

- `salt/`
- `json/`
- `yaml/`
- `nwdiag/`
- `archimate/`
- `regex/`
- `ebnf/`
- `chart/`
- `math/`
- `sdl/`
- `ditaa/`
- `mindmap/`
- `wbs/`

### Compatibility and styling surfaces

- `c4/`
- `themes/`
- `skinparams/`
- `preprocessor/`
- `creole/`

## Top-level examples in this folder

- `basic_hello.puml` -> `basic_hello.svg`
- `groups_notes.puml` -> `groups_notes.svg`
- `lifecycle_autonumber.puml` -> `lifecycle_autonumber.svg`
- `supported_primitives_*.puml` -> corresponding `*.svg`

## Status framing

- Families are no longer documented here as “parse-only” or “not yet parsed”.
- Current status should be interpreted as:
  - family availability: implemented
  - feature depth inside each family: mixed (`implemented` and `partial`), tracked in the spec audits

See:
- [../internal/spec/plantuml-spec.md](../internal/spec/plantuml-spec.md)
- [../internal/spec/audit/](../internal/spec/audit/)
