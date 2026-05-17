# PlantUML Parity Source of Truth

Status date: 2026-05-17  
Canonical status artifact for machine/human parity review.

## Contract

- This markdown table is the canonical parity source of truth.
- Status vocabulary is strict: `implemented`, `partial`, `missing`.
- `docs/audits/parity_gap_core.csv` and `docs/audits/parity_gap_nonuml.csv` are secondary aligned exports for tooling compatibility.
- Be conservative: do not mark as `implemented` where evidence only shows baseline or partial depth.
- Project board checks are advisory for review flow only; do not close issues from this artifact unless `origin/main` already contains the implementation commits.

## Core UML + Preprocessor

| Family | Slice | Feature | Status | Evidence | PlantUML Reference |
|---|---|---|---|---|---|
| sequence | core | Basic messages and participants | implemented | `tests/fixtures/basic/hello.puml` | https://plantuml.com/sequence-diagram |
| sequence | advanced | Arrow/style breadth and teoz semantics | partial | `tests/fixtures/arrows/valid_expanded_forms.puml` | https://plantuml.com/sequence-diagram |
| class | core | Core declarations and relations | implemented | `tests/fixtures/families/valid_class_with_relations.puml` | https://plantuml.com/class-diagram |
| class | advanced | Full class styling/edge semantics | partial | `tests/fixtures/families/valid_class_visibility.puml`, `tests/fixtures/styling/valid_skinparam_class.puml` | https://plantuml.com/class-diagram |
| object | core | Object declarations/relations | implemented | `tests/fixtures/families/valid_object_bootstrap.puml` | https://plantuml.com/object-diagram |
| object | advanced | Full object semantics breadth | partial | `tests/fixtures/families/valid_object_bootstrap.puml` | https://plantuml.com/object-diagram |
| usecase | core | Actor/usecase declarations and relations | implemented | `tests/fixtures/families/valid_usecase_with_relations.puml` | https://plantuml.com/use-case-diagram |
| usecase | advanced | Include/extend/style depth | partial | `tests/fixtures/families/valid_usecase_with_relations.puml` | https://plantuml.com/use-case-diagram |
| component | core | Component declarations/dependencies/packages | implemented | `tests/fixtures/families/valid_component.puml` | https://plantuml.com/component-diagram |
| component | advanced | Interface/port/style breadth | partial | `tests/fixtures/families/valid_component.puml`, `tests/fixtures/styling/valid_skinparam_component.puml` | https://plantuml.com/component-diagram |
| deployment | core | Node/artifact topology | implemented | `tests/fixtures/families/valid_deployment.puml` | https://plantuml.com/deployment-diagram |
| deployment | advanced | Advanced deployment styling/controls | partial | `tests/fixtures/families/valid_deployment.puml` | https://plantuml.com/deployment-diagram |
| state | core | State/transitions/history/concurrency | implemented | `tests/fixtures/families/valid_state.puml` | https://plantuml.com/state-diagram |
| state | advanced | Full state styling/edge semantics | partial | `tests/fixtures/families/valid_state_history.puml`, `tests/fixtures/styling/valid_skinparam_state.puml` | https://plantuml.com/state-diagram |
| activity | core | Legacy/old-style activity baseline | implemented | `tests/fixtures/families/valid_activity_old_style.puml` | https://plantuml.com/activity-diagram-legacy |
| activity | advanced | New-style activity breadth | partial | `tests/fixtures/families/valid_activity_colored_lane.puml`, `tests/fixtures/styling/valid_skinparam_activity.puml` | https://plantuml.com/activity-diagram-beta |
| timing | core | Timing participants/state transitions baseline | implemented | `tests/fixtures/families/valid_timing.puml` | https://plantuml.com/timing-diagram |
| timing | advanced | Full timing semantics breadth | partial | `tests/fixtures/families/valid_timing_waveform.puml`, `tests/parity_state_activity_timing_depth.rs` | https://plantuml.com/timing-diagram |
| preprocessor | core | include/define/undef/conditionals/loops/functions/procedures | implemented | `tests/fixtures/preprocessor/valid_if_elseif_else.puml` | https://plantuml.com/preprocessing |
| preprocessor | advanced | Dynamic invocation and full expression breadth | partial | `tests/fixtures/errors/invalid_preproc_concat_unsupported.puml` | https://plantuml.com/preprocessing |
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
| salt | advanced | Full Salt widget/style breadth | partial | `tests/fixtures/families/valid_salt_login_form.puml` | https://plantuml.com/salt |
| nwdiag | core | Network grammar + baseline render | implemented | `tests/fixtures/non_sequence/valid_nwdiag.puml` | https://plantuml.com/nwdiag |
| nwdiag | advanced | Full network topology semantics | partial | `docs/examples/nwdiag/01_single_net.svg` | https://plantuml.com/nwdiag |
| json | core | @startjson standalone family | implemented | `tests/fixtures/non_sequence/valid_json.puml` | https://plantuml.com/json |
| yaml | core | @startyaml standalone family | implemented | `tests/fixtures/non_sequence/valid_yaml.puml` | https://plantuml.com/yaml |
| json+yaml | advanced | Cross-diagram projection adapters | partial | `tests/fixtures/families/valid_yaml_projection.puml` | https://plantuml.com/yaml |
| archimate | core | Archimate parser + baseline render | implemented | `tests/fixtures/non_sequence/valid_archimate.puml` | https://plantuml.com/archimate-diagram |
| archimate | advanced | Full relation/style breadth | partial | `docs/examples/archimate/01_layered.svg`, `tests/integration.rs` | https://plantuml.com/archimate-diagram |
| regex | core | @startregex baseline parser/render | implemented | `tests/fixtures/non_sequence/valid_regex.puml` | https://plantuml.com/regex |
| regex | advanced | Full descriptive/localized regex semantics | partial | `docs/examples/regex/01_character_classes.svg` | https://plantuml.com/regex |
| ebnf | core | @startebnf baseline parser/render | implemented | `tests/fixtures/non_sequence/valid_ebnf.puml` | https://plantuml.com/ebnf |
| ebnf | advanced | Full railroad style breadth | partial | `docs/examples/ebnf/01_simple_grammar.svg` | https://plantuml.com/ebnf |
| math | core | @startmath/@startlatex baseline support | implemented | `tests/fixtures/non_sequence/valid_math.puml` | https://plantuml.com/ascii-math |
| ditaa | core | @startditaa baseline support | implemented | `tests/fixtures/non_sequence/valid_ditaa.puml`, `tests/fixtures/families/valid_ditaa_complex.puml`, `tests/integration.rs` | https://plantuml.com/ditaa |
| chart | core | @startchart baseline parser/render | implemented | `tests/fixtures/non_sequence/valid_chart_bar.puml`, `tests/fixtures/non_sequence/valid_chart_pie.puml` | https://plantuml.com/chart-diagram |
| chart | advanced | Full axis/legend/style integration | partial | `docs/examples/chart/01_bar.svg` | https://plantuml.com/chart-diagram |

## Board / Issue Consistency Checks

- 2026-05-17: `gh issue view 103 --json number,title,state,projectItems,url` verified tracking issue `#103` is on the `PUML` project with status `Human Review`.
- 2026-05-17: `gh issue view <n> --json number,title,state,projectItems,url` for `#197`, `#202`, `#205`, `#206`, `#207`, and `#208` verified all six issues remain open and on the `PUML` project with status `Human Review`.
- 2026-05-17: Branch-local evidence exists for `#197` PNG output, `#202` family skinparams, `#205` Gantt date/duration rendering, `#206` timing waveform rendering, and `#208` Ditaa shape rendering. `#207` remains baseline math rendering only and is not promoted to full LaTeX/math-symbol parity.
- 2026-05-17: `git branch -r --contains 37a49d2 23e212a 4738f7c` showed the inspected issue-related commits are contained by `origin/codex/local-parity-blitz-20260516`, not `origin/main`; no issue closure was performed.
- No issue closure was performed during this audit.
