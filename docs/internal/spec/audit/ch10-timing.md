# Chapter 10 — Timing Diagram audit

Scope: PlantUML Language Reference Guide (1.2025.0), §10.1–§10.29.
Repo paths referenced are relative to `/Users/allison.coleman/Develop/puml`.

Legend: ✅ supported · 🟡 partial / cosmetic gaps · ❌ not implemented

---

### 10.1 Declaring element or participant — ✅
**Feature:** `concise`, `robust`, `clock`, `binary`, `analog` participant kinds with optional `"label" as alias` and `with period N pulse M offset K` controls
**Syntax example:** `clock "Clock_0" as C0 with period 50`
**Status:** ✅
**Evidence:** `parse_timing_decl` handles `compact`, `concise`, `robust`, `clock`, `binary`, quoted labels, aliases, and `with …` controls (`src/parser/timing.rs:1-55`, `src/parser/timing.rs:111-125`). `analog` declarations route through `parse_timing_analog_decl` and are marked with `__timing:analog` controls (`src/parser/timing.rs:57-109`); rendering recognizes that marker as the analog signal kind (`src/render/timing.rs:340-350`, `src/render/timing.rs:710-715`).
**Notes:** Implementation stores analog as a robust-family node plus internal marker rather than adding `TimingDeclKind::Analog`, but the canonical declaration form renders.

### 10.2 Binary and Clock — ✅
**Feature:** `binary "Enable" as EN`, `clock clk with period 1`
**Status:** ✅
**Evidence:** `src/parser/timing.rs:11-16` (kinds), `src/render/timing.rs:411-471` (binary waveform), `src/render/timing.rs:473-536` (clock waveform with period/pulse/offset honored via `timing_control_i64` at `src/render/timing.rs:696-708`)

### 10.3 Adding message (`WU -> WB : URL`) — ✅
**Feature:** Arrow between two lanes at a given time
**Syntax example:** `WU -> WB : URL` inside a `@100` block
**Status:** ✅
**Evidence:** `parse_timing_relation` parses `->`, `<-`, `-->`, `<--`, and `<->` timing relations with labels (`src/parser/timing.rs:260-293`). Timing normalization attaches missing endpoint times to the current timing cursor (`src/normalize/family.rs:1302-1336`, `src/normalize/family.rs:1808-1822`), and `render_timing_relations` draws cross-lane message lines, arrowheads, and labels (`src/render/timing.rs:811-867`). Fixture coverage asserts `C -> S@+2` resolves to `C@5` → `S@7` and renders a `timing-message` (`tests/timing_advanced_geometry.rs:321-397`).

### 10.4 Relative time (`@+N`, `@+50`, `WB -> DNS@+50`) — ✅
**Feature:** Relative time offsets and per-message relative offsets
**Syntax example:** `@+100` / `WB -> DNS@+50 : Resolve URL`
**Status:** ✅
**Evidence:** `normalize_timing_time` resolves `@+N` and `@-N` against the timing cursor (`src/normalize/family.rs:1750-1779`), and `normalize_timing_endpoint` applies the same logic to relation endpoints such as `S@+2` (`src/normalize/family.rs:1808-1822`). Tests assert a relative event resolves from `@5` to `8` and a message endpoint resolves to `S@7` (`tests/timing_advanced_geometry.rs:174-184`, `tests/timing_advanced_geometry.rs:356-368`).

### 10.5 Anchor Points (`@5 as :name`, `@:name+6`) — ✅
**Feature:** Named anchor times referenced as `@:anchor`, `@:anchor+N`, `@:anchor-N`
**Syntax example:** `@5 as :en_high` then `@:en_high-2 as :en_highMinus2` and `@:en_high`
**Status:** ✅
**Evidence:** `parse_timing_anchor` recognizes `@N as :name` anchor definitions (`src/parser/timing.rs:196-202`, `src/parser/timing.rs:250-258`), and `normalize_timing_anchor_expr` resolves `@:name`, `@:name+N`, and `@:name-N` through the anchor map (`src/normalize/family.rs:1750-1806`). Chapter-10 fixture coverage uses `@0 as :start`, `@:start+5 as :send`, and `@:send+5` (`tests/fixtures/families/valid_timing_ch10_parity.puml:10-22`).

### 10.6 Participant oriented (`@WB` then `0 is idle`, `+200 is Proc.`) — ✅
**Feature:** Declare events grouped by participant, with `0 is …`, `+N is …` relative shorthand
**Status:** ✅
**Evidence:** `parse_timing_oriented_state` recognizes `<time> is <state>` lines starting with a digit, `+`, `-`, or `:` (`src/parser/timing.rs:295-307`). `@SignalName` switches `timing_current_signal`, and subsequent oriented events inherit it (`src/normalize/family.rs:1241-1262`). Relative and anchor times in that mode use the same `normalize_timing_time` path (`src/normalize/family.rs:1263-1274`, `src/normalize/family.rs:1750-1806`).

### 10.7 Setting scale (`scale 100 as 50 pixels`, dates `2592000 as 50 pixels`) — ✅
**Feature:** Map clock units to pixels
**Status:** ✅
**Evidence:** `parse_timing_event` stores `scale ... as ...` as a timing option (`src/parser/timing.rs:156-163`), and `timing_scaled_chart_width` maps units to pixels when computing `chart_w` (`src/render/timing.rs:147`, `src/render/timing.rs:782-804`). Chapter-10 tests assert `scale 5 as 120 pixels` narrows the rendered chart (`tests/timing_advanced_geometry.rs:371-397`).

### 10.8 Initial state (bare `WB is Initializing` before any `@`) — 🟡
**Feature:** Declare initial state outside any `@` block
**Status:** 🟡
**Evidence:** `split_is` (`src/parser/timing.rs:382-391`) accepts a bare `X is Y` line; normalize attaches it as a `TimingEvent` with empty `time` (`src/normalize/family.rs:1263-1300`). Render filters by `parse::<i64>().ok()` for time values and signal events (`src/render/timing.rs:107-117`, `src/render/timing.rs:369-381`), so the initial-state value is effectively lost.
**Notes:** Parses without error but is not painted.

### 10.9 Intricated / undefined state (`is {0,1}`, `is {0,1} #SlateGrey`) — 🟡
**Feature:** Multi-value brace state and per-event inline color
**Status:** 🟡
**Evidence:** `normalize_timing_state_literal` strips `{}` and preserves trailing style metadata (`src/parser/timing.rs:309-327`), and `timing_state_style` applies `#color` / `line:` styling during render (`src/render/timing.rs:636-645`, `src/render/timing.rs:723-758`). However, multi-value brace states such as `{0,1}` still render as a text label rather than PlantUML's uncertain-state visualization.

### 10.10 Hidden state (`is {-}`, `is {hidden}`) — ✅
**Feature:** Hide a segment of the waveform
**Status:** ✅
**Evidence:** `timing_state_hidden` recognizes `-` and `hidden` (`src/render/timing.rs:769-774`), and concise/robust/binary renderers skip labels and draw dashed hidden-state segments (`src/render/timing.rs:460-470`, `src/render/timing.rs:568-573`, `src/render/timing.rs:630-635`). Chapter-10 fixture coverage includes both `S is {hidden}` and `R is {-}` (`tests/fixtures/families/valid_timing_ch10_parity.puml:24-25`), with tests asserting `timing-hidden-state` output (`tests/timing_advanced_geometry.rs:371-397`).

### 10.11 Hide time axis (`hide time-axis`) — ✅
**Feature:** Suppress the top time axis
**Status:** ✅
**Evidence:** `parse_timing_event` records `hide time-axis` (`src/parser/timing.rs:138-145`), `render_timing_svg` suppresses the axis and tick labels when the option is present (`src/render/timing.rs:57-64`, `src/render/timing.rs:210-273`), and tests assert no `timing-tick` labels render for the parity fixture (`tests/timing_advanced_geometry.rs:371-397`).

### 10.12 Using Time and Date (`@2019/07/02`, `@1:15:00`) — ❌
**Feature:** Use absolute date or wall-clock time as tick label
**Status:** ❌
**Evidence:** `src/render/timing.rs:107-117` uses `e.name.parse::<i64>()` / endpoint integer parsing for tick values. ISO dates and `HH:MM:SS` strings fail to parse and are dropped from `time_vals`.

### 10.13 Change Date Format (`use date format "YY-MM-dd"`) — ❌
**Feature:** Format dates on the axis
**Status:** ❌
**Evidence:** No `date format` handler in timing parse/render. Lines fall through as Unknown.

### 10.14 Manage time axis labels (`manual time-axis` vs default) — 🟡
**Feature:** Default label-per-tick vs label-on-state-change
**Status:** 🟡
**Evidence:** `manual time-axis` is parsed as a timing option (`src/parser/timing.rs:146-153`), and the renderer suppresses tick labels unless a signal event contributes a manual tick value (`src/render/timing.rs:57-64`, `src/render/timing.rs:243-272`). This covers the numeric tick-label switch, but date/time label semantics are still absent (see §10.12-10.13).

### 10.15 Adding constraint (`WB@0 <-> @50 : {50 ms lag}`, `@200 <-> @+150 : {150 ms}`) — 🟡
**Feature:** Bi-directional time-range constraint arrow
**Status:** 🟡
**Evidence:** `parse_timing_range_after_time` (`src/parser/timing.rs:343-354`) handles the `@start <-> @end : label` form, and ranges render as yellow shaded strips (`src/render/timing.rs:225-242`). The braces `{50 ms lag}` are preserved as label text.
**Notes:** Renders as a highlighted band, NOT as a constraint arrow with end caps. `WB@0` prefix (participant-anchored constraint) is not specifically parsed — the `WB` segment is ignored. `@+150` relative in constraint is parsed but rendering uses absolute coords.

### 10.16 Highlighted period (`highlight 200 to 450 #Gold;line:DimGrey : caption`) — 🟡
**Feature:** Coloured highlight band with optional inline style
**Status:** 🟡
**Evidence:** `parse_timing_highlight` (`src/parser/timing.rs:356-380`) recognizes `highlight S to E : label`. Renders as yellow strip (`src/render/timing.rs:225-242`) hard-coded to `#fde68a`/`#f59e0b`.
**Notes:** Inline `#Gold;line:DimGrey` color/style is not parsed; all highlights are the same yellow.

### 10.17 Using notes (`note top of WU : …`, `note bottom of WU : …`) — ❌
**Feature:** Top/bottom-of-participant notes (concise/binary only per spec)
**Status:** ❌
**Evidence:** Notes parsed by the global note parser but never wired into `src/render/timing.rs` — no note-of-participant rendering. The family normalizer stores `StatementKind::Note` as generic note nodes (`src/normalize/family.rs:1338-1341`), and `render_timing_svg` only renders signals, timing events, ranges, and timing relations.

### 10.18 Adding texts (title, header, footer, legend, caption) — 🟡
**Feature:** Standard document-level texts
**Status:** 🟡
**Evidence:** Title is rendered (`src/render/timing.rs:190-205`). Header, footer, legend, caption are normalized at document level but `src/render/timing.rs` only consumes `doc.title` — others not emitted in the SVG.

### 10.19 Complete example (mixed @Client/@Server/@Cache + `+N is …` + cross-lane arrows + range) — ✅
**Feature:** End-to-end realistic timing diagram
**Status:** ✅
**Evidence:** Participant-oriented blocks (§10.6), relative times (§10.4), cross-lane arrows (§10.3), and range/highlight bands (§10.15-10.16) are all now covered by parser/normalizer/render paths. The advanced geometry fixture and tests verify relative event resolution, range metadata, range geometry, clock controls, and waveform geometry (`tests/fixtures/families/valid_timing_advanced_geometry.puml:1-22`, `tests/timing_advanced_geometry.rs:141-318`).

### 10.20 Digital Example (binary + concise mix with `:anchor` references and constraint arrows) — 🟡
**Feature:** Multi-signal digital protocol diagram with anchored constraints (`@:write_beg-3`, `db@:write_beg-1 <-> @:write_end : setup time`, `db@:write_beg-1 -> addr@:write_end+1 : hold`)
**Status:** 🟡
**Evidence:** Anchors and signal-prefixed message endpoints are now normalized (`src/normalize/family.rs:1750-1822`), and message arrows render across lanes (`src/render/timing.rs:811-867`). Signal-prefixed `<->` constraint examples such as `db@:write_beg-1 <-> @:write_end` still render through the generic timing-message path rather than PlantUML's dedicated constraint-arrow visualization.

### 10.21 Adding color (`LR is AtPlace #palegreen`, per-event `100 is Lowered #pink`) — ✅
**Feature:** Per-state-segment background colour
**Status:** ✅
**Evidence:** `split_timing_state_style` separates trailing `#color` tokens outside quotes (`src/parser/timing.rs:329-341`), and render applies fill/line styling through `timing_state_style` (`src/render/timing.rs:636-645`, `src/render/timing.rs:723-758`). The parity fixture uses `#palegreen`, `#LightCyan;line:Aqua`, and `#Gold`, with tests asserting the expected SVG fill/stroke values (`tests/fixtures/families/valid_timing_ch10_parity.puml:11-19`, `tests/timing_advanced_geometry.rs:371-397`).

### 10.22 Using (global) style (`<style> timingDiagram { document { … } constraintArrow { … } }`) — ❌
**Feature:** Style block selectors for timing diagram
**Status:** ❌
**Evidence:** No `timingDiagram` or `constraintArrow` selector in style infra. Renderer falls back to `TimingStyle` defaults (`src/render/timing.rs:29-37`), with only skinparam/theme-derived `TimingStyle` values wired (`src/theme.rs:1694-1769`, `src/normalize/family.rs:1508-1552`).

### 10.23 Applying Colors to specific lines (`<style> .red { LineColor red }` then `binary IS2 <<red>>`) — ❌
**Feature:** User-defined style class via stereotype on a signal
**Status:** ❌
**Evidence:** `TimingDecl` parser (`src/parser/timing.rs:1-55`) does not recognize `<<class>>` stereotypes on signal declarations.

### 10.24 Compact mode (`mode compact`, `compact robust …`) — ✅
**Feature:** Vertically compact layout, both global and per-element
**Status:** ✅
**Evidence:** `parse_timing_event` records `mode compact` (`src/parser/timing.rs:127-137`), `parse_timing_decl` accepts per-element `compact` prefix and stores `__timing:compact` (`src/parser/timing.rs:1-10`, `src/parser/timing.rs:42-45`), and the renderer reduces row height when compact mode is active (`src/render/timing.rs:57-64`, `src/render/timing.rs:142`). Tests assert the compact fixture's viewBox height shrinks (`tests/timing_advanced_geometry.rs:371-397`).

### 10.25 Scaling analog signal (`analog "Analog" between 350 and 450 as A`) — ✅
**Feature:** Analog signal with min/max range
**Status:** ✅
**Evidence:** `parse_timing_analog_decl` parses `analog "label" between MIN and MAX as A` and stores `__timing:analog_between MIN MAX` (`src/parser/timing.rs:57-109`). `render_timing_analog_signal` uses that min/max range to scale analog y positions (`src/render/timing.rs:882-940`). The parity fixture uses `analog "Vcore" between 0 and 6 as V`, and tests assert analog line/points render (`tests/fixtures/families/valid_timing_ch10_parity.puml:9`, `tests/timing_advanced_geometry.rs:371-397`).

### 10.26 Customise analog signal (`VCC ticks num on multiple 3`, `VCC is 200 pixels height`) — ❌
**Feature:** Analog ticks/height customisation
**Status:** ❌
**Evidence:** Analog declaration and range rendering are supported (§10.25), but there is no parser or renderer handling for `ticks num on multiple ...` or `is ... pixels height` in `src/parser/timing.rs` / `src/render/timing.rs`.

### 10.27 Order state of robust signal (`rate has high,low,none`, `rate has "35 gpm" as high`) — ❌
**Feature:** Pre-declare value ordering and label aliases for a robust signal
**Status:** ❌
**Evidence:** `has` keyword is not handled by `src/parser/timing.rs` — would parse as Unknown. Without ordering, robust signal value rows are assigned by first-seen order in `state_order` (`src/render/timing.rs:538-549`).

### 10.28 Defining a timing diagram (by clock `@clk*N`, by signal `@S1`, by time `@T`) — 🟡
**Feature:** Three event-block addressing modes
**Status:** 🟡
**Evidence:** By-time and by-signal (`@SignalName`) modes work via `timing_current_signal` tracking (`src/normalize/family.rs:1241-1262`). `normalize_timing_time` now recognizes `@clk*N`-shaped strings by taking the multiplier after `*` (`src/normalize/family.rs:1759-1763`), so those events no longer drop solely because the raw time is non-integer. It does **not** multiply by the referenced clock's declared period, so this remains partial.

### 10.29 Annotate signal with comment (`D is low: idle`, `R is lo: idle`, `@-3` negative time) — 🟡
**Feature:** Per-event trailing `: comment` annotation, negative-time events
**Status:** 🟡
**Evidence:** `split_is` returns everything after `is` (including `: idle`) as the state (`src/parser/timing.rs:382-391`). The colon and comment become part of the rendered state label. Negative times are normalized numerically (`src/normalize/family.rs:1771-1777`) and can render because the axis computes `t_min`/`t_max` from numeric event times.
**Notes:** Negative time `@-3` is accepted (parses as `-3`) but `t_min` can go negative — `time_to_x` handles it correctly only if the highlight ranges also accommodate. Annotations are not separated from the state token.

---

## Tally

| Status | Count |
|--------|-------|
| ✅ supported | 13 |
| 🟡 partial | 9 |
| ❌ missing | 7 |

Top gaps blocking parity:

1. **Date/time axis values (§10.12, 10.13)** — renderer remains integer-time only; `use date format` is not parsed.
2. **Timing `<style>` selectors and stereotype classes (§10.22, 10.23)** — skinparam/theme values exist, but PlantUML `<style> timingDiagram { ... }` and `<<class>>` signal styling are still absent.
3. **Analog customization (§10.26)** — analog declaration/range rendering works, but ticks and per-signal pixel height are missing.
4. **Robust `has` value ordering (§10.27)** — state order is still first-seen, not declared.
5. **Top/bottom participant notes (§10.17)** — notes are normalized as generic nodes and not emitted by `render_timing_svg`.
6. **Constraint arrows vs highlight bands (§10.15, 10.20)** — ranges render as shaded bands, not PlantUML constraint arrows with end caps.
7. **Multi-value/undefined state visualization (§10.9)** — `{0,1}` is preserved as a label and can be colored, but the uncertain-state glyph is not implemented.
8. **Header/footer/legend/caption (§10.18)** — title renders; the other standard document texts do not.
9. **Manual axis and `@clk*N` semantics (§10.14, 10.28)** — both parse enough to avoid drops, but do not yet match PlantUML's full clock/date axis behavior.
10. **Trailing `: comment` on `is` (§10.29)** is folded into the state string instead of treated as a per-event annotation.
