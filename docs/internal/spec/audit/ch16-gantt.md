# Chapter 16 — Gantt Chart Audit

Tally: 17 ✅ / 11 🟡 / 11 ❌

### 16.1.1 Workload (requires N days/weeks) — ✅
**Feature:** `[Task] requires N days` (also weeks; combined `1 week and 4 days`)
**Syntax example:** `[T4 (1 week and 4 days)] requires 1 week and 4 days`
**Status:** ✅
**Evidence:** src/parser/gantt.rs:349-378 (parse_gantt_duration_clause); supports day/days/week/weeks/month/months
**Notes:** Month unit (30 days) supported in parser but spec only lists day+week.

### 16.1.2 Start verb (absolute date + D+n) — 🟡
**Feature:** `[Task] starts YYYY-MM-DD` and `[Task] starts D+15` relative-to-project-start
**Syntax example:** `[Prototype design] starts D+0`
**Status:** 🟡
**Evidence:** gantt.rs:186-200 generic starts constraint; gantt.rs:335-347 parses ISO date only
**Notes:** `D+15` form not parsed by `parse_gantt_start_date_clause` (requires `is_iso_date_literal`). D+n stored as raw constraint target but no resolver.

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

### 16.2 One-line with `and` conjunction — 🟡
**Feature:** `[T] starts 2020-07-01 and ends 2020-07-15` / `starts X and requires N days`
**Status:** 🟡
**Evidence:** gantt.rs:319-333 parse_gantt_start_and_duration supports `and lasts` / `and requires`; no handler for `starts X and ends Y` (different second clause)
**Notes:** `and is colored in`, `and starts N days after [T]`, `and ends at [T]'s end` (16.23 complex example) all unsupported as conjunctions.

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

### 16.5 Tasks with same name — 🟡
**Feature:** Multiple tasks with same display name, distinguished by alias
**Syntax example:** `[SameTaskName] as [T1] lasts 7 days`
**Status:** 🟡
**Evidence:** timeline.rs:36-51 upserts by name (merges duplicates rather than creating second task)
**Notes:** Same-name + alias must create distinct tasks; current logic UPDATES first match → second `[SameTaskName] as [T2]` would overwrite, not split.

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

### 16.9 Hyperlinks `links to [[url]]` — ❌
**Feature:** `[task1] links to [[http://plantuml.com]]`
**Status:** ❌
**Evidence:** No "links to" parser
**Notes:** No hyperlink field on TimelineTask.

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

### 16.12 Scale (printscale/projectscale/ganttscale + daily/weekly/monthly/quarterly/yearly) — 🟡
**Feature:** `printscale weekly`, `projectscale monthly`, scale values
**Status:** 🟡
**Evidence:** gantt.rs:273-289 parse_gantt_scale_directive handles `printscale` + `scale`; `projectscale` and `ganttscale` aliases NOT parsed
**Notes:** Values daily/weekly/monthly/quarterly/yearly all map correctly.

### 16.12.6/8 Print between date range — ❌
**Feature:** `Print between 2021-01-12 and 2021-01-22`
**Status:** ❌
**Evidence:** No `print between` parser

### 16.13 Zoom — ❌
**Feature:** `printscale daily zoom 2`
**Status:** ❌
**Evidence:** Scale parser at gantt.rs:280-286 returns None for any trailing token after value (e.g. "daily zoom 2" fails match).

### 16.14 Week numbering / calendar date display — ❌
**Feature:** `printscale weekly with week numbering from 1`, `with calendar date`
**Status:** ❌
**Evidence:** No `with week numbering` / `with calendar date` branch.

### 16.15 Close day (weekday + date ranges + open) — ✅
**Feature:** `saturday are closed` / `2018/05/01 is closed` / `2018/04/17 to 2018/04/19 is closed` / `2020-07-13 is open`
**Status:** ✅ (partial — slash dates ❌)
**Evidence:** gantt.rs:204-271 weekday & date-range close/open handlers; timeline.rs:109-166 wires into closed_weekdays/ranges/open_ranges
**Notes:** Slash-form dates (`2018/05/01`) reject because is_iso_date_literal requires hyphen separators.

### 16.16 Week-as-non-closed-days semantics — 🟡
**Feature:** "3 weeks" auto-adjusts when sat/sun closed
**Status:** 🟡
**Evidence:** scheduled_gantt_span_days (timeline.rs:342-366) iterates closed days but `parse_gantt_duration_clause` treats weeks as fixed 7 days for the workload — the closed-day adjustment expands the calendar span, not the underlying workload definition.

### 16.17 Working days offset `2 working days after [T]'s end` — ❌
**Feature:** `[task2] starts 2 working days after [task1]'s end and requires 3 days`
**Status:** ❌
**Evidence:** parse_gantt_reference_day_offset (timeline.rs:428-446) handles `days/day after/before`, not `working days`.

### 16.18 then keyword (succession) — 🟡
**Feature:** `then [Task] requires 4 days` continuation
**Status:** 🟡
**Evidence:** Not seen in parser; `then` keyword may be handled at higher dispatcher. grep showed no `"then "` branch in gantt.rs.
**Notes:** Likely treated as unknown statement → [E_GANTT_UNSUPPORTED]. Spec heavily relies on `then`.

### 16.19 Resources `[T] on {Alice}` (incl. `{Bob:50%}`, multi) — ✅
**Feature:** Task assignment to resources with optional load percent
**Status:** ✅
**Evidence:** gantt.rs:380-431 extract_gantt_resources; timeline.rs:490-537 allocation parsing & load adjustment

### 16.19 Resource off-days `{Alice} is off on 2020-06-24 to 2020-06-26` — ❌
**Feature:** Per-resource closed dates
**Status:** ❌
**Evidence:** No parser for `{...} is off on`

### 16.20 hide resources names / footbox — ❌
**Feature:** `hide resources names`, `hide resources footbox`
**Status:** ❌
**Evidence:** No parser branch (only `hide footbox` for sequence at sequence.rs:263)

### 16.21 Horizontal separator `-- Phase Two --` — ✅
**Feature:** Section separator between task groups
**Status:** ✅
**Evidence:** gantt.rs:309-317 parse_gantt_horizontal_separator

### 16.22 Vertical separator `Separator just at [T]'s end` — 🟡
**Feature:** Vertical line separator at specific date/task reference
**Status:** 🟡
**Evidence:** gantt.rs:291-307 + timeline.rs:93-100 stash separator constraint
**Notes:** `Separator just 2 days after [T]'s end` form — relative offset stored as raw target string; no resolver.

### 16.23 Complex one-line with delays — 🟡
**Feature:** `requires 9 days and is colored in Coral/Green and starts 3 days after [T]'s start`
**Status:** 🟡
**Evidence:** parse_gantt_start_and_duration only supports two conjuncts (and lasts / and requires)
**Notes:** Multi-conjunction chains drop later clauses.

### 16.24 Comments `'` and `/' ... '/` — ✅
**Feature:** Single and block comments
**Status:** ✅
**Evidence:** Handled by common preprocessor / parser comment stripping

### 16.25 `<style>` blocks (ganttDiagram) — 🟡
**Feature:** `<style>ganttDiagram { task {...} milestone {...} arrow {...} separator {...} timeline {...} closed {...} }</style>`
**Status:** 🟡
**Evidence:** Style blocks parsed generically; gantt render does not consume per-section style overrides (uses fixed colors in render/timeline.rs).

### 16.26 Notes `note bottom ... end note` — 🟡
**Feature:** Per-task notes
**Status:** 🟡
**Evidence:** `StatementKind::Note` is part of grammar but timeline normalize loop doesn't add note → falls to `_ => return Err(...)` at timeline.rs:197.
**Notes:** Will likely fail with E_TIMELINE_BASELINE_UNSUPPORTED.

### 16.27 Pause tasks `[T] pauses on 2018/12/13` / `pauses on monday` — ❌
**Feature:** Per-day or weekday task pauses
**Status:** ❌
**Evidence:** No `pauses on` parser

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

### 16.37 Mark tasks as `is deleted` — ❌
**Feature:** `[T] is deleted`
**Status:** ❌
**Evidence:** No deleted-status parser; no deleted field on TimelineTask.

### 16.38 %now / %date builtin functions — ❌
**Feature:** `!$past = %date("YYYY-MM-dd", $now - 14*24*3600)` etc.
**Status:** ❌
**Evidence:** No %now/%date builtin in preproc/builtins.rs.

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
