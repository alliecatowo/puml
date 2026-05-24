# Chapter 16 — Gantt Chart Audit

Tally: 30 ✅ / 5 🟡 / 4 ❌

### 16.1.1 Workload (requires N days/weeks) — ✅
**Feature:** `[Task] requires N days` (also weeks; combined `1 week and 4 days`)
**Syntax example:** `[T4 (1 week and 4 days)] requires 1 week and 4 days`
**Status:** ✅
**Evidence:** src/parser/gantt.rs:349-378 (parse_gantt_duration_clause); supports day/days/week/weeks/month/months
**Notes:** Month unit (30 days) supported in parser but spec only lists day+week.

### 16.1.2 Start verb (absolute date + D+n) — ✅
**Feature:** `[Task] starts YYYY-MM-DD` and `[Task] starts D+15` relative-to-project-start
**Syntax example:** `[Prototype design] starts D+0`
**Status:** ✅
**Evidence:** `src/parser/gantt.rs` `parse_gantt_start_date_clause` accepts ISO/slash/verbal dates and `D+n`; `src/normalize/timeline.rs` resolves `D+n` relative to the project/anchor day in `resolve_gantt_absolute_day`. Covered by `gantt_ch16_completion_notes_resource_off_and_hide_options_render` and `gantt_date_builtins_and_same_line_start_end_drive_task_span` in `tests/parity_wave_csv_timeline_activity.rs`.
**Notes:** Relative `D+n` is deterministic and works in task declarations and compound clauses.

### 16.1.3 Ends verb — ✅
**Feature:** `[Task] ends YYYY-MM-DD`
**Syntax example:** `[Prototype design] ends 2020-07-15`
**Status:** ✅
**Evidence:** gantt.rs:186-200 (kind="ends"); normalize/timeline.rs:404-410 (ends adjusts start_day)
**Notes:** D+n form same gap as starts.

### 16.1.4 Start/End combination — ✅
**Feature:** Specifying both start and end across two lines
**Status:** ✅
**Evidence:** timeline.rs:368-426 reference constraints resolver; multiple constraints on same subject re-applied

### 16.2 One-line with `and` conjunction — ✅
**Feature:** `[T] starts 2020-07-01 and ends 2020-07-15` / `starts X and requires N days`
**Status:** ✅
**Evidence:** `src/parser/gantt.rs` routes `and` clauses through `GanttCompound`, `src/normalize/timeline.rs` splits and applies duration/color/completion/link/baseline/deleted/start/end/require clauses, and paired absolute `starts`+`ends` now derive task duration. Covered by `gantt_date_builtins_and_same_line_start_end_drive_task_span`, `gantt_ch16_verbal_slash_relative_dates_then_and_working_lag_render`, and `gantt_ch16_same_display_name_aliases_remain_distinct` in `tests/parity_wave_csv_timeline_activity.rs`.
**Notes:** Per-link visual styling remains tracked separately in 16.28.

### 16.3 Constraints `starts at [T]'s end` — ✅
**Feature:** Task relative to another task's start/end
**Syntax example:** `[Test prototype] starts at [Prototype design]'s end`
**Status:** ✅
**Evidence:** normalize/timeline.rs:448-459 parse_gantt_task_reference; 368-426 applies them

### 16.4 Short names / alias with `as [D]` — ✅
**Feature:** `[Prototype design] as [D] requires 15 days`
**Status:** ✅
**Evidence:** parser handles `[name] as [alias]` through bracket-subject parsing; tasks indexed by name allowing alias references in subsequent constraints.
**Notes:** Verify in src/parser/blocks/bracket_subject.

### 16.5 Tasks with same name — ✅
**Feature:** Multiple tasks with same display name, distinguished by alias
**Syntax example:** `[SameTaskName] as [T1] lasts 7 days`
**Status:** ✅
**Evidence:** `src/normalize/timeline.rs` keys task updates by alias when present (`gantt_task_ref` / `gantt_task_matches`), and `tests/parity_wave_csv_timeline_activity.rs::gantt_ch16_same_display_name_aliases_remain_distinct` verifies two same-display-name tasks retain distinct aliases, colors, and dependency endpoints.
**Notes:** Display names can repeat when aliases differ.

### 16.6 / 16.34 is colored in (task bars; legend color partial) — ✅/🟡
**Feature:** `[Task] is colored in Fuchsia/FireBrick`
**Status:** ✅ Supported for task bars
**Evidence:** `src/parser/gantt.rs` parses task color metadata, `src/normalize/timeline.rs` applies fill/stroke colors, and `src/render/timeline.rs` emits colored Gantt task bars. Covered by `gantt_issue_779_named_date_marker_and_task_color_render` in `tests/parity_wave_csv_timeline_activity.rs` and `docs/examples/gantt/09_ch16_parity.puml`.
**Notes:** Legend text itself renders, but full PlantUML legend cell-color fidelity still depends on broader Creole/table support.

### 16.7 Completion percentage — ✅
**Feature:** `[foo] is 40% completed` / `is 40% complete` / `requires N days and is 10% complete`
**Status:** ✅
**Evidence:** `src/parser/gantt.rs` parses `% complete` and `% completed` clauses into Gantt compound/constraint metadata; `src/model.rs` stores `TimelineTask::completion_percent`; `src/normalize/timeline.rs` applies completion constraints; `src/render/timeline.rs` emits `data-gantt-completion` plus a `gantt-task-completion` overlay. Covered by `parses_gantt_completion_percentage_forms` in `src/parser/tests.rs`, `gantt_ch16_completion_notes_resource_off_and_hide_options_render` in `tests/parity_wave_csv_timeline_activity.rs`, and `docs/examples/gantt/09_ch16_parity.puml`.
**Notes:** Percentages are clamped to 100 during parse/normalize.

### 16.8.1/2/3 Milestone happens — ✅
**Feature:** `[M] happens at [T]'s end` / `happens 2020-07-10` / `happens N days after [T]'s end`
**Status:** ✅
**Evidence:** gantt.rs:180-185 + 433-449 parse_gantt_happens_target; timeline.rs:72-85 + reference offset 428-446
**Notes:** Max-end semantics for multiple milestones unclear (last-write-wins).

### 16.9 Hyperlinks `links to [[url]]` — ✅
**Feature:** `[task1] links to [[http://plantuml.com]]`
**Status:** ✅
**Evidence:** `src/parser/gantt.rs` accepts `links to [[...]]` clauses; `src/model.rs` stores `TimelineTask::hyperlink`; `src/normalize/timeline.rs` applies link constraints; `src/render/timeline.rs` emits SVG anchors/data attributes. Covered by `parses_gantt_task_hyperlink_forms` in `src/parser/tests.rs`, `gantt_ch16_task_hyperlink_renders_anchor` in `tests/parity_wave_csv_timeline_activity.rs`, and `docs/examples/gantt/09_ch16_parity.puml`.
**Notes:** The first token inside the link is used as the href, matching the URL-only Gantt form while tolerating an optional label.

### 16.10 Calendar / Project starts (verbal forms) — 🟡
**Feature:** `Project starts the 20th of september 2017` (English natural form)
**Status:** 🟡
**Evidence:** gantt.rs:14-28 only accepts ISO `YYYY-MM-DD` after stripping "on "/"the "
**Notes:** "20th of september 2017" not parsed. Only ISO works.

### 16.11 Coloring / naming days `2020/09/07 is colored in salmon` — 🟡
**Feature:** Single-date and range colors for days, plus `are named [...]` naming
**Status:** 🟡 Partial
**Evidence:** Named date/range markers render via `GanttNamedDate` support covered by `gantt_issue_779_named_date_marker_and_task_color_render` and `docs/examples/gantt/09_ch16_parity.puml`.
**Notes:** Named ranges are supported; single-day color bands remain less complete than PlantUML's full calendar-color semantics.

### 16.12 Scale (printscale/projectscale/ganttscale + daily/weekly/monthly/quarterly/yearly) — ✅
**Feature:** `printscale weekly`, `projectscale monthly`, scale values
**Status:** ✅
**Evidence:** `src/parser/gantt.rs:419-441` accepts `printscale`, `projectscale`, `ganttscale`, and `scale` with daily/weekly/monthly/quarterly/yearly units; `src/normalize/timeline.rs:106-109` stores normalized scale/options; `src/render/timeline.rs:69-77` emits scale metadata and `src/render/timeline.rs:1000-1016` applies scale tick stride. Covered by `gantt_scale_single_day_calendar_and_multi_resource_semantics_render`, `gantt_separator_relative_constraints_resource_metadata_and_month_scale_render`, and `gantt_weekly_scale_display_options_affect_tick_labels` in `tests/parity_wave_csv_timeline_activity.rs`.
**Notes:** Display modifiers are tracked in 16.13/16.14.

### 16.12.6/8 Print between date range — ✅
**Feature:** `Print between 2021-01-12 and 2021-01-22`
**Status:** ✅
**Evidence:** `src/parser/gantt.rs:443-459` parses `Print between <date> and <date>` into a project print-window constraint; `src/normalize/timeline.rs:115-131` normalizes ordered start/end dates onto `TimelineDocument`; `src/render/timeline.rs:80-86` emits print-window metadata and `src/render/timeline.rs:345-350,401-405,557-580` limits the rendered axis and clips task/baseline bars to the requested window. Covered by `gantt_print_between_clips_axis_and_zoom_widens_chart` in `tests/parity_wave_csv_timeline_activity.rs` and `docs/examples/gantt/09_ch16_parity.puml`.

### 16.13 Zoom — ✅
**Feature:** `printscale daily zoom 2`
**Status:** ✅
**Evidence:** `src/parser/gantt.rs:419-441` preserves trailing scale options, and `src/render/timeline.rs:21-31,1021-1052` parses `zoom N`, widens the Gantt chart area, and exposes `data-gantt-zoom`. Covered by `gantt_print_between_clips_axis_and_zoom_widens_chart` in `tests/parity_wave_csv_timeline_activity.rs`.

### 16.14 Week numbering / calendar date display — ✅
**Feature:** `printscale weekly with week numbering from 1`, `with calendar date`
**Status:** ✅
**Evidence:** `src/parser/gantt.rs:419-441` preserves weekly display modifiers; `src/render/timeline.rs:1021-1052` recognizes `with week numbering from N` and `with calendar date`; `src/render/timeline.rs:1112-1135` renders weekly tick labels as `Week N`, raw calendar dates, or the default `Wk <date>`. Covered by `gantt_weekly_scale_display_options_affect_tick_labels` in `tests/parity_wave_csv_timeline_activity.rs`.

### 16.15 Close day (weekday + date ranges + open) — ✅
**Feature:** `saturday are closed` / `2018/05/01 is closed` / `2018/04/17 to 2018/04/19 is closed` / `2020-07-13 is open`
**Status:** ✅ (partial — slash dates ❌)
**Evidence:** gantt.rs:204-271 weekday & date-range close/open handlers; timeline.rs:109-166 wires into closed_weekdays/ranges/open_ranges
**Notes:** Slash-form dates (`2018/05/01`) reject because is_iso_date_literal requires hyphen separators.

### 16.16 Week-as-non-closed-days semantics — 🟡
**Feature:** "3 weeks" auto-adjusts when sat/sun closed
**Status:** 🟡
**Evidence:** scheduled_gantt_span_days (timeline.rs:342-366) iterates closed days but `parse_gantt_duration_clause` treats weeks as fixed 7 days for the workload — the closed-day adjustment expands the calendar span, not the underlying workload definition.

### 16.17 Working days offset `2 working days after [T]'s end` — ✅
**Feature:** `[task2] starts 2 working days after [task1]'s end and requires 3 days`
**Status:** ✅
**Evidence:** `src/normalize/timeline.rs` parses `working day(s) after/before` in `parse_gantt_reference_day_offset` and skips closed weekdays/date ranges via `add_gantt_working_days`. Covered by `gantt_ch16_verbal_slash_relative_dates_then_and_working_lag_render` and `gantt_ch16_completion_notes_resource_off_and_hide_options_render`.

### 16.18 then keyword (succession) — ✅
**Feature:** `then [Task] requires 4 days` continuation
**Status:** ✅
**Evidence:** `src/parser/gantt.rs` parses `then [Task] ...` into a `GanttCompound` with `after_previous=true`; `src/normalize/timeline.rs` turns it into a start-at-previous-end constraint. Covered by `gantt_ch16_verbal_slash_relative_dates_then_and_working_lag_render`.

### 16.19 Resources `[T] on {Alice}` (incl. `{Bob:50%}`, multi) — ✅
**Feature:** Task assignment to resources with optional load percent
**Status:** ✅
**Evidence:** gantt.rs:380-431 extract_gantt_resources; timeline.rs:490-537 allocation parsing & load adjustment

### 16.19 Resource off-days `{Alice} is off on 2020-06-24 to 2020-06-26` — ✅
**Feature:** Per-resource closed dates
**Status:** ✅
**Evidence:** `src/parser/gantt.rs` parses `{Resource} is off on <date/range>` into `resource_off`; `src/normalize/timeline.rs` stores `TimelineResourceOffRange`, applies matching resource off-days to task scheduling, and `src/render/timeline.rs` emits visible in-bar resource-off bands plus metadata/labels. Covered by `gantt_ch16_completion_notes_resource_off_and_hide_options_render`, `gantt_ch16_task_pauses_and_resource_off_days_extend_and_render`, `docs/examples/gantt/09_ch16_parity.puml`, and `docs/examples/gantt/10_pauses_resource_calendars.puml`.

### 16.20 hide resources names / footbox — ✅
**Feature:** `hide resources names`, `hide resources footbox`
**Status:** ✅
**Evidence:** `src/parser/gantt.rs` maps `hide resources names` and `hide resources footbox` to `HideOption`; `src/normalize/timeline.rs` stores the flags, and `src/render/timeline.rs` emits `data-gantt-hide-resource-*` metadata. Covered by `gantt_ch16_completion_notes_resource_off_and_hide_options_render`.

### 16.21 Horizontal separator `-- Phase Two --` — ✅
**Feature:** Section separator between task groups
**Status:** ✅
**Evidence:** gantt.rs:309-317 parse_gantt_horizontal_separator

### 16.22 Vertical separator `Separator just at [T]'s end` — 🟡
**Feature:** Vertical line separator at specific date/task reference
**Status:** 🟡
**Evidence:** gantt.rs:291-307 + timeline.rs:93-100 stash separator constraint
**Notes:** `Separator just 2 days after [T]'s end` form — relative offset stored as raw target string; no resolver.

### 16.23 Complex one-line with delays — ✅
**Feature:** `requires 9 days and is colored in Coral/Green and starts 3 days after [T]'s start`
**Status:** ✅
**Evidence:** `src/normalize/timeline.rs` applies multi-conjunction `GanttCompound` clauses for workload, color, completion, link, baseline, deleted, and start/end/reference constraints. Covered by `gantt_ch16_verbal_slash_relative_dates_then_and_working_lag_render`, `gantt_ch16_completion_notes_resource_off_and_hide_options_render`, and `gantt_date_builtins_and_same_line_start_end_drive_task_span`.
**Notes:** Link color/style suffixes remain separate 16.28 work.

### 16.24 Comments `'` and `/' ... '/` — ✅
**Feature:** Single and block comments
**Status:** ✅
**Evidence:** Handled by common preprocessor / parser comment stripping

### 16.25 `<style>` blocks (ganttDiagram) — 🟡
**Feature:** `<style>ganttDiagram { task {...} milestone {...} arrow {...} separator {...} timeline {...} closed {...} }</style>`
**Status:** 🟡
**Evidence:** Style blocks parsed generically; gantt render does not consume per-section style overrides (uses fixed colors in render/timeline.rs).

### 16.26 Notes `note bottom ... end note` — ✅
**Feature:** Per-task notes
**Status:** ✅
**Evidence:** `src/normalize/timeline.rs` stores `StatementKind::Note` as `TimelineNote`, defaulting bare notes to the previous task; `src/render/timeline.rs` renders note boxes. Covered by `gantt_ch16_completion_notes_resource_off_and_hide_options_render`.

### 16.27 Pause tasks `[T] pauses on 2018/12/13` / `pauses on monday` — ✅
**Feature:** Per-day or weekday task pauses
**Status:** ✅
**Evidence:** `src/parser/gantt.rs` accepts task-level `pauses on <date/range>` and weekday forms; `src/normalize/timeline.rs` stores pause ranges/weekdays, expands scheduled spans around paused days, and re-applies dependent task references after pause-aware scheduling; `src/render/timeline.rs` draws in-bar pause bands. Covered by `gantt_ch16_task_pauses_and_resource_off_days_extend_and_render` and `docs/examples/gantt/10_pauses_resource_calendars.puml`.

### 16.28 Link colors `with blue dotted link` and `-[#FF00FF]->` — ❌
**Feature:** Per-link arrow color/style
**Status:** ❌
**Evidence:** No `with .* link` parsing; arrow constraints `[T1] -> [T2]` likely parsed but color suffix ignored.

### 16.29 displays on same row — ❌
**Feature:** `[T2] displays on same row as [T1]`
**Status:** ❌
**Evidence:** No parser branch

### 16.30 Highlight today / `today is N days after start and is colored in #AAF` — ❌
**Feature:** Today marker with color/position
**Status:** ❌
**Evidence:** No `today is` parser

### 16.31 Task between two milestones `occurs from [A] to [B]` — ❌
**Feature:** Task duration bounded by milestone refs
**Status:** ❌
**Evidence:** No `occurs from` parser

### 16.32 Grammar table (informational) — n/a

### 16.33 Title/header/footer/caption/legend — ✅
**Feature:** Common commands accepted on gantt
**Status:** ✅
**Evidence:** timeline.rs:170-179 handles Title/Header/Footer/Caption/Legend

### 16.34 Add color on legend — 🟡
**Feature:** Colored legend rows/cells in Gantt diagrams.
**Status:** 🟡 Partial
**Evidence:** Legend blocks render via the timeline common-command path, and task color support is now present (see 16.6). Full PlantUML colored legend fidelity still depends on broader Creole table/cell-color support.

### 16.35 hide footbox — 🟡
**Feature:** Remove footer date row
**Status:** 🟡
**Evidence:** Parsed as HideOption (timeline.rs:188-193 silently accepts) but renderer in render/timeline.rs may not honor it for gantt — needs trace.

### 16.36 Calendar language `language de/ja/zh/ko/en` — ❌
**Feature:** Localized month/day names
**Status:** ❌
**Evidence:** No `language XX` parser

### 16.37 Mark tasks as `is deleted` — ✅
**Feature:** `[T] is deleted`
**Status:** ✅
**Evidence:** `src/parser/gantt.rs` parses `is deleted`; `src/normalize/timeline.rs` sets `TimelineTask::is_deleted`; `src/render/timeline.rs` emits deleted metadata/strike-through. Covered by `gantt_ch16_completion_notes_resource_off_and_hide_options_render`.

### 16.38 %now / %date builtin functions — ✅
**Feature:** `!$past = %date("YYYY-MM-dd", $now - 14*24*3600)` etc.
**Status:** ✅
**Evidence:** `src/preproc/builtins.rs` implements deterministic `%now()` and `%date(format, epoch_seconds_expr)` using a reproducible UTC epoch clock (`PUML_NOW` injection override); `tests/integration/preprocessor.rs::preprocessor_date_builtin_formats_deterministic_epoch_and_arithmetic_offsets` verifies format tokens and arithmetic offsets. `tests/parity_wave_csv_timeline_activity.rs::gantt_date_builtins_and_same_line_start_end_drive_task_span` verifies `%date` output drives Gantt project/task dates.
**Notes:** `%now()` intentionally does not read the host wall clock; this preserves strict same-input/same-output determinism.

### 16.39 Label position `Label on first column and left aligned` — ❌
**Feature:** Reposition task label column
**Status:** ❌
**Evidence:** No `Label on` parser branch.

### 16.7.4 baseline / planned — ✅
**Feature:** `[Task] has baseline 2026-01-01 to 2026-01-15` (puml extension)
**Status:** ✅ (puml-extension)
**Evidence:** gantt.rs:160-178 + timeline.rs:574-587 + parse_gantt_baseline_target:590-616. Spec doesn't explicitly list this — appears to be a puml-side enrichment.

### Critical path (puml extension) — ✅
**Feature:** `[T] is critical` / `[T] is on critical path` / `Project critical_path` auto-inference
**Status:** ✅
**Evidence:** gantt.rs:150-159 + timeline.rs:253-259 + mark_inferred_gantt_critical_path 644-676
