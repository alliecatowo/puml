# PlantUML Parity Source of Truth

Status date: 2026-05-17  
Canonical status artifact for machine/human parity review.

## Contract

- This markdown table is the canonical parity source of truth.
- Status vocabulary is strict: `implemented`, `partial`, `missing`.
- `docs/audits/parity_gap_core.csv` and `docs/audits/parity_gap_nonuml.csv` are secondary aligned exports for tooling compatibility.
- `docs/audits/post_blitz_gap_table.md` is the post-blitz planning companion for remaining partial rows, exact evidence, GitHub project status, and next implementation slices.
- Examples, closed issues, merged PRs, and oracle reports are evidence inputs only; they do not override this table.
- Be conservative: do not mark as `implemented` where evidence only shows baseline or partial depth.
- Project board checks are advisory for review flow only; do not close issues from this artifact unless the implementation commits are merged into the branch being audited.
- Oracle reports are comparison-only conformance evidence. The Java PlantUML oracle is never part of the `puml` runtime contract.

## Core UML + Preprocessor

| Family | Slice | Feature | Status | Evidence | PlantUML Reference |
|---|---|---|---|---|---|
| sequence | core | Basic messages and participants | implemented | `tests/fixtures/basic/hello.puml` | https://plantuml.com/sequence-diagram |
| sequence | advanced | Arrow/style breadth, participant placement, and teoz semantics | partial | `tests/fixtures/arrows/valid_expanded_forms.puml`, `tests/render_e2e.rs::render_sequence_participant_order_controls_lifeline_placement` | https://plantuml.com/sequence-diagram |
| class | core | Core declarations and relations | implemented | `tests/fixtures/families/valid_class_with_relations.puml` | https://plantuml.com/class-diagram |
| class | advanced | Full class styling/edge semantics | partial | `tests/fixtures/families/valid_class_visibility.puml`, `tests/fixtures/styling/valid_skinparam_class.puml`, `tests/integration.rs::family_notes_render_for_core_uml_families` | https://plantuml.com/class-diagram |
| object | core | Object declarations/relations | implemented | `tests/fixtures/families/valid_object_bootstrap.puml` | https://plantuml.com/object-diagram |
| object | advanced | Full object semantics breadth | partial | `tests/fixtures/families/valid_object_bootstrap.puml`, `tests/integration.rs` | https://plantuml.com/object-diagram |
| usecase | core | Actor/usecase declarations and relations | implemented | `tests/fixtures/families/valid_usecase_with_relations.puml` | https://plantuml.com/use-case-diagram |
| usecase | advanced | Include/extend/style depth | partial | `tests/fixtures/families/valid_usecase_with_relations.puml`, `tests/integration.rs` | https://plantuml.com/use-case-diagram |
| component | core | Component declarations/dependencies/packages | implemented | `tests/fixtures/families/valid_component.puml` | https://plantuml.com/component-diagram |
| component | advanced | Interface/port/style breadth | partial | `tests/fixtures/families/valid_component.puml`, `tests/fixtures/styling/valid_skinparam_component.puml`, `tests/integration.rs::family_notes_render_for_core_uml_families` | https://plantuml.com/component-diagram |
| deployment | core | Node/artifact topology | implemented | `tests/fixtures/families/valid_deployment.puml` | https://plantuml.com/deployment-diagram |
| deployment | advanced | Advanced deployment styling/controls | partial | `tests/fixtures/families/valid_deployment.puml` | https://plantuml.com/deployment-diagram |
| state | core | State/transitions/history/concurrency | implemented | `tests/fixtures/families/valid_state.puml` | https://plantuml.com/state-diagram |
| state | advanced | Full state styling/edge semantics | partial | `tests/fixtures/families/valid_state_history.puml`, `tests/fixtures/styling/valid_skinparam_state.puml` | https://plantuml.com/state-diagram |
| activity | core | Legacy/old-style activity baseline | implemented | `tests/fixtures/families/valid_activity_old_style.puml` | https://plantuml.com/activity-diagram-legacy |
| activity | advanced | New-style activity breadth | partial | `tests/fixtures/families/valid_activity_colored_lane.puml`, `tests/fixtures/styling/valid_skinparam_activity.puml`, `tests/integration.rs::family_notes_render_for_core_uml_families` | https://plantuml.com/activity-diagram-beta |
| timing | core | Timing participants/state transitions baseline | implemented | `tests/fixtures/families/valid_timing.puml` | https://plantuml.com/timing-diagram |
| timing | advanced | Full timing semantics breadth | partial | `tests/fixtures/families/valid_timing_waveform.puml`, `tests/parity_state_activity_timing_depth.rs` | https://plantuml.com/timing-diagram |
| preprocessor | core | include/define/undef/conditionals/loops/functions/procedures | implemented | `tests/fixtures/preprocessor/valid_if_elseif_else.puml` | https://plantuml.com/preprocessing |
| preprocessor | advanced | Dynamic invocation and full expression/data-helper breadth | partial | `tests/fixtures/errors/invalid_preproc_concat_unsupported.puml`, `tests/integration.rs::preproc_nested_json_mutation_and_projection_helpers_expand` | https://plantuml.com/preprocessing |
| preprocessor | policy | URL include/remote source behavior | partial | `tests/fixtures/errors/invalid_include_url.puml` | https://plantuml.com/preprocessing |
| preprocessor | core | Theme + stdlib integration baseline | implemented | `tests/fixtures/include/valid_c4_context.puml` | https://plantuml.com/skinparam |
| preprocessor-json | advanced | JSON preprocessor surface | partial | `tests/fixtures/non_sequence/valid_json.puml` | https://plantuml.com/preprocessing-json |

## Non-UML Families

| Family | Slice | Feature | Status | Evidence | PlantUML Reference |
|---|---|---|---|---|---|
| gantt+chronology | core | Parser/semantic model baseline | implemented | `tests/fixtures/timeline/valid_gantt_baseline.puml`, `tests/fixtures/timeline/valid_chronology_baseline.puml` | https://plantuml.com/gantt-diagram |
| gantt+chronology | advanced | Full scale/resource semantics | partial | `tests/fixtures/timeline/valid_gantt_render.puml`, `tests/fixtures/timeline/valid_gantt_dates_proportional.puml`, `tests/fixtures/timeline/valid_chronology_render.puml`, `tests/parity_wave_csv_timeline_activity.rs` | https://plantuml.com/chronology-diagram |
| mindmap+wbs | core | Family parsing and baseline rendering | implemented | `tests/fixtures/non_sequence/invalid_mindmap_diagram.puml`, `tests/fixtures/non_sequence/invalid_wbs_diagram.puml` | https://plantuml.com/mindmap-diagram |
| mindmap+wbs | advanced | Orientation/styling parity depth | partial | `docs/examples/mindmap/01_basic.svg`, `docs/examples/wbs/01_basic.svg` | https://plantuml.com/wbs-diagram |
| salt | core | Salt parser + baseline widget render | implemented | `tests/fixtures/families/valid_salt_bootstrap.puml` | https://plantuml.com/salt |
| salt | advanced | Full Salt widget/style breadth | partial | `tests/fixtures/families/valid_salt_login_form.puml`, `tests/integration.rs`; supports label/input/button/combo/checkbox/radio rows, prefix-label rows, separators, tree/menu/tab/scroll/table widgets, header cells, and deterministic Salt color/font style directives, but advanced styling remains partial | https://plantuml.com/salt |
| nwdiag | core | Network grammar + baseline render | implemented | `tests/fixtures/non_sequence/valid_nwdiag.puml` | https://plantuml.com/nwdiag |
| nwdiag | advanced | Full network topology semantics | partial | `docs/examples/nwdiag/01_single_net.svg` | https://plantuml.com/nwdiag |
| json | core | @startjson standalone family | implemented | `tests/fixtures/non_sequence/valid_json.puml` | https://plantuml.com/json |
| yaml | core | @startyaml standalone family | implemented | `tests/fixtures/non_sequence/valid_yaml.puml` | https://plantuml.com/yaml |
| json+yaml | advanced | Cross-diagram projection adapters | partial | `tests/fixtures/families/valid_yaml_projection.puml` | https://plantuml.com/yaml |
| archimate | core | Archimate parser + baseline render | implemented | `tests/fixtures/non_sequence/valid_archimate.puml` | https://plantuml.com/archimate-diagram |
| archimate | advanced | Full relation/style breadth | partial | `docs/examples/archimate/01_layered.svg`, `tests/integration.rs` | https://plantuml.com/archimate-diagram |
| regex | core | @startregex baseline parser/render | implemented | `tests/fixtures/non_sequence/valid_regex.puml` | https://plantuml.com/regex |
| regex | advanced | Full descriptive/localized regex semantics | partial | `docs/examples/regex/01_character_classes.svg`, `tests/integration.rs`; includes localized labels and exact/ranged counted quantifier evidence, but broader Unicode/category semantics remain partial | https://plantuml.com/regex |
| ebnf | core | @startebnf baseline parser/render | implemented | `tests/fixtures/non_sequence/valid_ebnf.puml` | https://plantuml.com/ebnf |
| ebnf | advanced | Full railroad style breadth | partial | `docs/examples/ebnf/01_simple_grammar.svg`, `tests/integration.rs`; includes styled rule notes and exact/ranged counted repeat evidence, but broader railroad styling remains partial | https://plantuml.com/ebnf |
| math | core+advanced | @startmath/@startlatex baseline plus LaTeX-ish fractions, roots, paired scripts, accents, fences, matrix environments, Greek/operators/symbols, text constructs, and big-operator layout | implemented | `tests/fixtures/non_sequence/valid_math.puml`, `tests/fixtures/families/valid_math_complex.puml`, `tests/integration.rs` | https://plantuml.com/ascii-math |
| sdl | core+advanced | @startsdl parser/model/render with stereotype-driven start/end/input/output/decision shapes; deeper activity-beta semantics remain tracked under activity advanced | implemented | `tests/fixtures/non_sequence/valid_sdl.puml`, `tests/integration.rs` | https://plantuml.com/activity-diagram-beta#SDL-Specification-and-Description-Language-with-SDL-sterotype |
| ditaa | core+advanced | @startditaa baseline plus options, color hints, advanced box kinds, junction connectors, diagonal connectors, and arrowheads | implemented | `tests/fixtures/non_sequence/valid_ditaa.puml`, `tests/fixtures/families/valid_ditaa_complex.puml`, `tests/integration.rs` | https://plantuml.com/ditaa |
| chart | core | @startchart baseline parser/render | implemented | `tests/fixtures/non_sequence/valid_chart_bar.puml`, `tests/fixtures/non_sequence/valid_chart_pie.puml` | https://plantuml.com/chart-diagram |
| chart | advanced | Full axis/legend/style integration | partial | `docs/examples/chart/01_bar.svg`, `tests/integration.rs`, `tests/chart_parity.rs`; accepts PlantUML-style chart labels, colon-delimited points, normalized palette/caption/annotations, explicit v-axis tick step, legend off/positioning, and selected skinparams, but full axis/legend semantics remain partial | https://plantuml.com/chart-diagram |

## Board / Issue Consistency Checks

- 2026-05-17: `gh issue view 103 --json number,title,state,projectItems,url` verified tracking issue `#103` is on the `PUML` project with status `Human Review`.
- 2026-05-17: `#197`, `#202`, `#205`, `#206`, and `#208` were closed as completed from branch evidence and moved to `Done` on the `PUML` project.
- 2026-05-17: Branch-local evidence now covers `#207` deterministic LaTeX-ish math rendering with matrices/environments, Greek/operators/symbols, text constructs, and big-operator layout on `origin/codex/local-parity-blitz-20260516`; `#207` was closed as completed and moved to `Done`.
- 2026-05-17: Branch-local evidence plus `gh issue view 105` confirms SDL is implemented as a special-adapter baseline under closed/Done issue `#105`; deeper activity-beta parity remains represented by the existing `activity,advanced` partial row.
- 2026-05-17: Latest pushed commit `13f92c0` strengthens partial evidence for Salt, regex, EBNF, and chart advanced rows; those rows remain `partial` because the official PlantUML surface is still wider than the current deterministic subset.
- 2026-05-16: PR `#224` was merged as commit `c68b2a701953c0a018cef1847448334377933cf3` (`Merge pull request #224 from alliecatowo/parity/swarm-gap-wave-b2e311d`). Treat this as landed progress for the rows evidenced above, not as a blanket full-parity claim.
