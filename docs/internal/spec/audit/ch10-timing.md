# Chapter 10 — Timing Diagram audit

Scope: PlantUML Language Reference Guide (1.2025.0), §10.1–§10.29.
Repo paths referenced are relative to `/Users/allison.coleman/Develop/puml`.

Legend: ✅ supported · 🟡 partial / cosmetic gaps · ❌ not implemented

---

### 10.1 Declaring element or participant — 🟡
**Feature:** `concise`, `robust`, `clock`, `binary`, `analog` participant kinds with optional `"label" as alias` and `with period N pulse M offset K` controls
**Syntax example:** `clock "Clock_0" as C0 with period 50`
**Status:** 🟡
**Evidence:** `src/parser/timing.rs:1-43` parses concise/robust/clock/binary; `split_timing_decl_controls` (45-59) captures `with …` clause. `TimingDeclKind` (`src/ast.rs:292-298`) has 4 variants.
**Notes:** **`analog` is NOT supported** — no `TimingDeclKind::Analog` variant. Documents using `analog` fall through to Unknown and break the diagram. Affects §10.1, §10.25, §10.26.

### 10.2 Binary and Clock — ✅
**Feature:** `binary "Enable" as EN`, `clock clk with period 1`
**Status:** ✅
**Evidence:** `src/parser/timing.rs:3-8` (kinds), `src/render/timing.rs:347-403` (binary waveform), `src/render/timing.rs:405-468` (clock waveform with period/pulse/offset honored via `timing_control_i64`)

### 10.3 Adding message (`WU -> WB : URL`) — ✅
**Feature:** Arrow between two lanes at a given time
**Syntax example:** `WU -> WB : URL` inside a `@100` block
**Status:** ✅
**Evidence:** `parse_timing_relation` in `src/parser/timing.rs:260-297` accepts timing arrows without mis-parsing `@:anchor` endpoints; `normalize_timing_endpoint` in `src/normalize/family.rs:2554-2568` resolves endpoint-relative `Signal@+N` forms; `render_timing_relations` in `src/render/timing.rs:825-880` emits the arrow line, head, and label; exercised by `tests/timing_advanced_geometry.rs:473-486`.

### 10.4 Relative time (`@+N`, `@+50`, `WB -> DNS@+50`) — 🟡
**Feature:** Relative time offsets and per-message relative offsets
**Syntax example:** `@+100` / `WB -> DNS@+50 : Resolve URL`
**Status:** 🟡
**Evidence:** `normalize_timing_time` is called (`src/normalize/family.rs:1233-1236`) and tracks `timing_current_time` so successive `@+N` resolves to absolute. `@+N` as a standalone block tick works.
**Notes:** `target@+N` syntax on a message arrow has no rendering path (see §10.3). Negative offsets like `@-3` (§10.29) are accepted by the parser regex but produce negative tick positions that may push outside the chart.

### 10.5 Anchor Points (`@5 as :name`, `@:name+6`) — 🟡
**Feature:** Named anchor times referenced as `@:anchor`, `@:anchor+N`, `@:anchor-N`
**Syntax example:** `@5 as :en_high` then `@:en_high-2 as :en_highMinus2` and `@:en_high`
**Evidence:** `parse_timing_anchor` plus `normalize_timing_anchor_expr` in `src/parser/timing.rs:250-257` and `src/normalize/family.rs:2527-2552` resolve named anchors and `+/-` offsets; `normalize_timing_endpoint` in `src/normalize/family.rs:2554-2568` applies them inside cross-lane message endpoints; exercised by `tests/timing_advanced_geometry.rs:435-470` and `tests/timing_advanced_geometry.rs:321-368`.
**Status:** 🟡
**Notes:** Numeric anchor references and anchored message endpoints work. Anchored highlight/constraint bands still lag exact PlantUML behavior.

### 10.6 Participant oriented (`@WB` then `0 is idle`, `+200 is Proc.`) — 🟡
**Feature:** Declare events grouped by participant, with `0 is …`, `+N is …` relative shorthand
**Status:** 🟡
**Evidence:** `parse_timing_oriented_state` (`src/parser/timing.rs:138-150`) recognizes `<time> is <state>` lines starting with digit, `+`, `-`, or `:`. `@WB` (with no numeric time) is handled by `timing_current_signal` tracking (`src/normalize/family.rs:1208-1225`).
**Notes:** Works for the simple case. `+200` relative-to-prior-event within a participant block tracks via `timing_current_time` but anchor references (`:`) and `@WB` interleaved with `@time` blocks may confuse the state machine.

### 10.7 Setting scale (`scale 100 as 50 pixels`, dates `2592000 as 50 pixels`) — ❌
**Feature:** Map clock units to pixels
**Status:** ❌
**Evidence:** Detection of timing scale exists at `src/parser/detect.rs:95` (recognizes `scale N as N`), but renderer (`src/render/timing.rs`) uses a fixed `chart_w = 760` and computes `time_to_x` purely from `t_min`/`t_max`. The parsed scale never influences layout.

### 10.8 Initial state (bare `WB is Initializing` before any `@`) — 🟡
**Feature:** Declare initial state outside any `@` block
**Status:** 🟡
**Evidence:** `split_is` (`src/parser/timing.rs:205-218`) accepts a bare `X is Y` line; normalize attaches it as a `TimingEvent` with empty `time`. Render filters by `parse::<i64>().ok()` (`src/render/timing.rs:100`) so empty-time events drop from `time_vals` but feed the per-signal `sig_events` filter (alias-matched but with non-numeric time → filtered out via `parse::<i64>().ok()?` at line 328). Initial-state value is effectively lost.
**Notes:** Parses without error but is not painted.

### 10.9 Intricated / undefined state (`is {0,1}`, `is {0,1} #SlateGrey`) — ❌
**Feature:** Multi-value brace state and per-event inline color
**Status:** ❌
**Evidence:** `normalize_timing_state_literal` (`src/parser/timing.rs:152-164`) strips `{` and `}` and returns the inner text verbatim (e.g. `"0,1"` becomes the state label). It renders as text only, no shaded uncertain-state visualization. Inline `#SlateGrey` is not parsed — appended into the state string.

### 10.10 Hidden state (`is {-}`, `is {hidden}`) — ❌
**Feature:** Hide a segment of the waveform
**Status:** ❌
**Evidence:** `normalize_timing_state_literal` strips outer `{` `}` and returns `"-"` or `"hidden"` as the literal state, which then renders as a normal coloured box labeled "-" or "hidden".

### 10.11 Hide time axis (`hide time-axis`) — ✅
**Feature:** Suppress the top time axis
**Status:** ✅
**Evidence:** `src/parser/timing.rs:138-143` parses `hide time-axis` into `__timing:hide-time-axis`; `src/render/timing.rs:62` reads the option and `src/render/timing.rs:171-176,264-335` suppresses the axis panel, tick marks, tick labels, range labels, and minor ticks. Exercised by `tests/timing_advanced_geometry.rs:321-390` and `docs/examples/timing/07_chapter10_parity.puml`.

### 10.12 Using Time and Date (`@2019/07/02`, `@1:15:00`) — 🟡
**Feature:** Use absolute date or wall-clock time as tick label
**Status:** 🟡
**Evidence:** `timing_time_value`, `parse_timing_hms`, and `parse_timing_date` in `src/render/timing.rs:985-1029` map `HH:MM:SS` and `YYYY/MM/DD` values into chart positions while `time_labels` preserves the original axis text in `src/render/timing.rs:65-73` and `276-285`; exercised by `tests/timing_advanced_geometry.rs:489-510`.
**Notes:** Absolute time/date ticks render, but `use date format` still remains separate work.

### 10.13 Change Date Format (`use date format "YY-MM-dd"`) — ❌
**Feature:** Format dates on the axis
**Status:** ❌
**Evidence:** No `date format` handler in timing parse/render. Lines fall through as Unknown.

### 10.14 Manage time axis labels (`manual time-axis` vs default) — 🟡
**Feature:** Default label-per-tick vs label-on-state-change
**Status:** 🟡
**Evidence:** `src/parser/timing.rs:146-151` parses `manual time-axis` into `__timing:manual-time-axis`; `src/render/timing.rs:62-63` reads the option and `src/render/timing.rs:301-335` labels only state-change ticks when manual mode is active. Exercised by `tests/timing_advanced_geometry.rs:400-441` and `docs/examples/timing/10_manual_time_axis.puml`.
**Notes:** Manual-mode label suppression is covered for state-change vs message/endpoint-only ticks. Broader PlantUML parity around generated scale ticks and `use date format` remains partial.

### 10.15 Adding constraint (`WB@0 <-> @50 : {50 ms lag}`, `@200 <-> @+150 : {150 ms}`) — 🟡
**Feature:** Bi-directional time-range constraint arrow
**Status:** 🟡
**Evidence:** `parse_timing_range_after_time` (`src/parser/timing.rs:166-177`) handles the `@start <-> @end : label` form, and ranges render as yellow shaded strips (`src/render/timing.rs:208-223`). The braces `{50 ms lag}` are preserved as label text.
**Notes:** Renders as a highlighted band, NOT as a constraint arrow with end caps. `WB@0` prefix (participant-anchored constraint) is not specifically parsed — the `WB` segment is ignored. `@+150` relative in constraint is parsed but rendering uses absolute coords.

### 10.16 Highlighted period (`highlight 200 to 450 #Gold;line:DimGrey : caption`) — 🟡
**Feature:** Coloured highlight band with optional inline style
**Status:** 🟡
**Evidence:** `parse_timing_highlight` (`src/parser/timing.rs:179-203`) recognizes `highlight S to E : label`. Renders as yellow strip (`src/render/timing.rs:208-223`) hard-coded to `#fde68a`/`#f59e0b`.
**Notes:** Inline `#Gold;line:DimGrey` color/style is not parsed; all highlights are the same yellow.

### 10.17 Using notes (`note top of WU : …`, `note bottom of WU : …`) — ❌
**Feature:** Top/bottom-of-participant notes (concise/binary only per spec)
**Status:** ❌
**Evidence:** Notes parsed by global note parser but never wired into `src/render/timing.rs` — no note-of-participant rendering. The timing normalizer in `src/normalize/family.rs:1201-1265` consumes `TimingEvent` but doesn't have a `Note(_)` branch in the timing-family path; behavior depends on the generic family fallback.

### 10.18 Adding texts (title, header, footer, legend, caption) — 🟡
**Feature:** Standard document-level texts
**Status:** 🟡
**Evidence:** Title is rendered (`src/render/timing.rs:174-185`). Header, footer, legend, caption are normalized at document level but `src/render/timing.rs` only consumes `doc.title` — others not emitted in the SVG.

### 10.19 Complete example (mixed @Client/@Server/@Cache + `+N is …` + cross-lane arrows + range) — 🟡
**Feature:** End-to-end realistic timing diagram
**Status:** 🟡
**Evidence:** The chapter example now renders as a fixture at `tests/fixtures/families/valid_timing_distributed_trace.puml` and is asserted in `tests/timing_advanced_geometry.rs:473-486`, including multiple cross-lane request/response arrows and the cache-freshness interval.
**Notes:** The scenario is now covered end-to-end, but constraint/highlight styling is still not 1:1 with PlantUML.

### 10.20 Digital Example (binary + concise mix with `:anchor` references and constraint arrows) — 🟡
**Feature:** Multi-signal digital protocol diagram with anchored constraints (`@:write_beg-3`, `db@:write_beg-1 <-> @:write_end : setup time`, `db@:write_beg-1 -> addr@:write_end+1 : hold`)
**Status:** 🟡
**Evidence:** Anchored point references and anchored cross-lane messages now resolve through `normalize_timing_anchor_expr` / `normalize_timing_endpoint` (`src/normalize/family.rs:2527-2568`) and render through `render_timing_relations` (`src/render/timing.rs:825-880`); exercised by `tests/timing_advanced_geometry.rs:435-470`.
**Notes:** The anchored message portion now works. Constraint-arrow rendering and anchored highlight bands still remain partial.

### 10.21 Adding color (`LR is AtPlace #palegreen`, per-event `100 is Lowered #pink`) — ❌
**Feature:** Per-state-segment background colour
**Status:** ❌
**Evidence:** `split_is` (`src/parser/timing.rs:205-218`) trims quotes but does not parse trailing `#color`. The `#palegreen` becomes part of the state string and renders as text "AtPlace #palegreen".

### 10.22 Using (global) style (`<style> timingDiagram { document { … } constraintArrow { … } }`) — ❌
**Feature:** Style block selectors for timing diagram
**Status:** ❌
**Evidence:** No `timingDiagram` or `constraintArrow` selector in style infra. Falls back to `TimingStyle` defaults (`src/render/timing.rs:34`).

### 10.23 Applying Colors to specific lines (`<style> .red { LineColor red }` then `binary IS2 <<red>>`) — ❌
**Feature:** User-defined style class via stereotype on a signal
**Status:** ❌
**Evidence:** `TimingDecl` parser (`src/parser/timing.rs:1-43`) does not recognize `<<class>>` stereotypes on signal declarations.

### 10.24 Compact mode (`mode compact`, `compact robust …`) — ❌
**Feature:** Vertically compact layout, both global and per-element
**Status:** ❌
**Evidence:** No `mode compact` handling. Per-element `compact` keyword prefix is not parsed. Renderer always uses `row_h = 64` (`src/render/timing.rs:127`).

### 10.25 Scaling analog signal (`analog "Analog" between 350 and 450 as A`) — ❌
**Feature:** Analog signal with min/max range
**Status:** ❌
**Evidence:** `analog` kind missing (see §10.1). `between … and …` clause never parsed.

### 10.26 Customise analog signal (`VCC ticks num on multiple 3`, `VCC is 200 pixels height`) — ❌
**Feature:** Analog ticks/height customisation
**Status:** ❌
**Evidence:** Analog itself not supported.

### 10.27 Order state of robust signal (`rate has high,low,none`, `rate has "35 gpm" as high`) — ❌
**Feature:** Pre-declare value ordering and label aliases for a robust signal
**Status:** ❌
**Evidence:** `has` keyword is not handled by `src/parser/timing.rs` — would parse as Unknown. Without ordering, robust signal value rows are assigned by first-seen order in `state_order` (`src/render/timing.rs:473-478`).

### 10.28 Defining a timing diagram (by clock `@clk*N`, by signal `@S1`, by time `@T`) — ✅
**Feature:** Three event-block addressing modes
**Status:** ✅
**Evidence:** By-signal blocks still route through `timing_current_signal` in `src/normalize/family.rs:1798-1832`; by-clock `@clk*N` now resolves via the per-clock period map in `src/normalize/family.rs:1722-1737` and `2490-2509`; rendered tick positions are asserted in `tests/timing_advanced_geometry.rs:399-432`.

### 10.29 Annotate signal with comment (`D is low: idle`, `R is lo: idle`, `@-3` negative time) — 🟡
**Feature:** Per-event trailing `: comment` annotation, negative-time events
**Status:** 🟡
**Evidence:** `split_is` strips a trailing `: comment` only implicitly — actually it returns everything after `is` (including `: idle`) as the state. The colon and comment become part of the rendered state label.
**Notes:** Negative time `@-3` is accepted (parses as `-3`) but `t_min` can go negative — `time_to_x` handles it correctly only if the highlight ranges also accommodate. Annotations are not separated from the state token.

---

## Tally

| Status | Count |
|--------|-------|
| ✅ supported | 4 |
| 🟡 partial | 12 |
| ❌ missing | 13 |

Top gaps blocking parity:

1. **`analog` participant kind (§10.1, 10.25, 10.26)** — missing variant entirely; documents break.
2. **Anchor-heavy constraint/highlight cases (§10.5, 10.15, 10.20)** — anchored messages work, but constraint-arrow rendering and anchored highlight-band fidelity still lag PlantUML.
3. **Date/time axis formatting controls (§10.12, 10.13, 10.14)** — absolute date/time ticks and `manual time-axis` label suppression now render, but `use date format` and generated scale tick semantics remain partial.
5. **Inline per-event color (§10.21) and brace states `{-}`/`{hidden}` (§10.9, 10.10)** — `#` and `{}` modifiers are absorbed into state label text.
6. **Constraint arrows vs highlight bands (§10.15)** — currently rendered as a yellow band, not a constraint with end caps.
7. **`scale N as N pixels`, `mode compact` (§10.7, 10.24)** — global layout switches need a focused audit refresh; `hide time-axis` is now covered.
8. **`has` keyword to declare robust signal value ordering (§10.27)** — missing.
9. **Trailing `: comment` on `is` (§10.29)** is folded into the state string instead of treated as a per-event annotation.
10. **Header/footer/legend/caption (§10.18)** rendered nowhere in `render_timing_svg`.
