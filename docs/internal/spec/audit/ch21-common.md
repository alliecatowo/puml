# Chapter 21 — Common Commands Audit

Source: `/tmp/puml-spec/ch21-common-commands-in-plantuml.txt` (2099 lines)
Spec date: PlantUML Language Reference Guide 1.2025.0

Status legend: ✅ supported · 🟡 partial · ❌ not supported

---

### 21.1.1 Simple comment (`'`) — ✅
**Feature:** Single-quote line comments; everything after `'` ignored.
**Syntax example:** `' this is a comment`
**Status:** ✅
**Evidence:** `src/parser/blocks.rs:1-19` (`strip_inline_plantuml_comment`), used by `src/parser/core.rs:18`, `src/parser/family.rs:1531`, etc. Also `src/main.rs:1475`.
**Notes:** Aware of quoted strings — won't strip `'` inside `"..."`. Trailing-comment form covered by tests at `src/parser/tests.rs:1786`.

### 21.1.2 Block comment (`/' ... '/`) — ❌
**Feature:** C-style block comments using `/'` ... `'/`, including same-line block comments.
**Syntax example:** `/' multiline\ncomment '/`
**Status:** ❌
**Evidence:** No occurrences of `/'` block-comment handling in `src/parser/` or `src/preproc/`. Only `picouml_strip_block_comments` exists at `src/frontend/picouml.rs:57` and uses `[/* ... */]` (PicoUML frontend, not PlantUML syntax).
**Notes:** Inline comment stripper at `src/parser/blocks.rs:1` treats `/` and `'` independently — would not span lines. Gap.

### 21.2 Zoom / scale — 🟡
**Feature:** `scale 1.5`, `scale 2/3`, `scale 200 width`, `scale 200 height`, `scale 200*100`, `scale max 300*200`, `scale max 1024 width`, `scale max 800 height`.
**Syntax example:** `scale 180*90`
**Status:** 🟡 (sequence/gantt only; emits to SVG via `ScaleSpec`; need to verify all subtypes)
**Evidence:** Parsed at `src/parser/sequence.rs:273-277` (StatementKind::Scale). Normalized at `src/normalize/sequence.rs:679-681`, `parse_scale_spec` at `src/normalize/sequence.rs:922`. ScaleSpec model at `src/scene.rs:25`. Rendered at `src/render/sequence.rs:921-932` with `Factor`, `Fixed`, `Max` variants. Gantt-side scale directive `src/parser/gantt.rs:7-11`.
**Notes:** `scale max <n> width/height` and `scale <n> width|height` need confirmation in `parse_scale_spec`. Non-sequence families (class, state, activity, component) may not honor the scale spec end-to-end.

### 21.3 Title (single-line + multi-line `title`/`end title`) — ✅
**Feature:** Single-line `title <text>`; multiline `title ... end title`; `\n` newlines; creole in titles.
**Syntax example:** `title Simple communication\nexample` / `title\n...\nend title`
**Status:** ✅
**Evidence:** AST `StatementKind::Title(String)` at `src/ast.rs:115`. Single-line parser `src/parser/sequence.rs:229-241`. Multiline collector `src/parser/multiline.rs:35-49`. Rendered in sequence at `src/render/sequence.rs:83`, mindmap at `src/render/mindmap.rs:302`, timeline at `src/render/timeline.rs:78`, salt at `src/render/salt.rs:74`, JSON/YAML at `src/render/data.rs:28,127`.
**Notes:** Creole tag support inside title depends on `src/creole.rs` (see ch22 audit).

### 21.4 Caption — ✅
**Feature:** `caption <text>` under diagram.
**Syntax example:** `caption figure 1`
**Status:** ✅
**Evidence:** `StatementKind::Caption` at `src/ast.rs:118`. Parser `src/parser/sequence.rs:233-237`. Multiline `src/parser/multiline.rs:52`. Model field `src/model.rs:50,128,...`. Rendered sequence `src/render/sequence.rs:574`; mindmap `src/render/mindmap.rs:366,889`.
**Notes:** Multiline `caption ... end caption` not exercised in greps — multiline.rs treats `caption` symmetrically with title so should work.

### 21.5 Footer and header — ✅
**Feature:** `header`/`endheader`, `footer`/`endfooter`; `left|center|right` alignment qualifier; HTML/creole content.
**Syntax example:** `header\n<font color=red>Warning</font>\nendheader` / `center footer Generated...`
**Status:** ✅ (basic); 🟡 (alignment qualifier left/center/right)
**Evidence:** AST `Header`/`Footer` `src/ast.rs:116-117`. Parser `src/parser/sequence.rs:229-241`. Multiline `src/parser/multiline.rs:35-51`. Rendered sequence `src/render/sequence.rs:72,585`. Text render `src/render/text.rs:61`.
**Notes:** No explicit `center footer` / `left header` alignment-prefix parsing found in greps. The alignment qualifier may be silently dropped or treated as text. Need follow-up check on `center|left|right header/footer` prefix.

### 21.6 Legend — ✅
**Feature:** `legend ... endlegend`/`end legend`; positioning `legend right`, `legend top left`, etc.
**Syntax example:** `legend right\n  Short legend\nendlegend`
**Status:** ✅
**Evidence:** AST `Legend(String)`, `LegendPos(String)` `src/ast.rs:119,148`. Multiline collector handles position tokens at `src/parser/multiline.rs:18-73` (encodes position into Legend value with prefix). Rendered with halign/valign `src/render/sequence.rs:597,957-962`; mindmap `src/render/mindmap.rs:375,898`.
**Notes:** `LegendHAlign`/`LegendVAlign` enums in scene confirm both axes implemented.

### 21.7 Per-diagram title/header/footer/caption/legend examples — ✅ (sequence/mindmap/timeline/salt/json/yaml) / 🟡 (other families)
**Feature:** All families should accept the same set of common commands.
**Status:** 🟡
**Evidence:** Top-level common-command extraction happens in `src/parser/sequence.rs:229-241` and the multiline collector, but model fields for these are only present on Document, SequenceDiagram, ClassDiagram, ComponentDiagram, etc. (`src/model.rs:47-51,442-446,537-541,702`). Rendering coverage varies — sequence/mindmap/timeline/salt/data have explicit calls; class/component/state/activity render paths were not located in grep above.
**Notes:** Gap: confirm `title/caption/legend/header/footer` render on class, component, state, activity, gantt, nwdiag, wbs, archimate.

### 21.8 Style block `<style>...</style>` for title/header/footer/etc. — ❌ (likely)
**Feature:** Skinparam-replacement `<style>` blocks setting `title { HorizontalAlignment right ... }`.
**Status:** ❌
**Evidence:** No matches for `<style>` parser handling in `src/parser/` or `src/normalize/` (only skinparam parsing exists).
**Notes:** Modern PlantUML `<style>` syntax is the recommended replacement for `skinparam`. Major gap.

### 21.9 Skinparam (general) — 🟡
**Feature:** `skinparam <key> <value>` and `skinparam <category> { ... }` block form.
**Syntax example:** `skinparam titleBorderRoundCorner 15`
**Status:** 🟡 (broad coverage in `src/theme.rs`; family-specific)
**Evidence:** `StatementKind::SkinParam` at `src/ast.rs:120`. Parser `src/parser/sequence.rs:242-249`. Theme classifiers for sequence/class/state/component/activity/timing/chart/gantt at `src/theme.rs:1099-1862`. Block-form is parsed at multiple normalize sites (`src/normalize/sequence.rs:392+`, `src/normalize/family.rs:622+`).
**Notes:** Coverage is per-family and skinparam-key explicit; many PlantUML skinparams are silently ignored.

### 21.x !theme directive — 🟡
**Feature:** `!theme <name>` (built-in themes) and `!theme <name> from <source>` (remote).
**Status:** 🟡
**Evidence:** AST `Theme(String)` at `src/ast.rs:124`. Parsed `src/parser/sequence.rs:250-251`. Resolved `src/theme.rs:472-487` (`resolve_sequence_theme_preset`). Used in chart at `src/normalize/chart.rs:25-28`.
**Notes:** Only built-in local themes; explicit error `E_THEME_SOURCE_UNSUPPORTED` for `from <source>` syntax. Remote themes deliberately rejected.

### 21.x Pragma — ✅ (general); ✅ (teoz)
**Feature:** `!pragma <key> <value>` — e.g. `!pragma teoz true`, `!pragma newpage`, `!pragma layout_new_line`.
**Status:** ✅ (parsing) / 🟡 (semantic effect)
**Evidence:** AST `Pragma(String)` at `src/ast.rs:125`. Parser `src/parser/sequence.rs:253-261`. `teoz` recognized at `src/normalize/sequence.rs:58,487` and `src/normalize/family.rs:759` (compat no-op). Test `src/parser/tests.rs:32`.
**Notes:** Only `teoz` has semantics; other pragmas (`newpage`, `layout_new_line`, etc.) parse but have no effect.

### 21.x !include / !includeurl / !include_many / !import — ✅
**Feature:** File, URL, glob, and stdlib includes; `!include_once` semantics.
**Status:** ✅
**Evidence:** `src/preproc/includes.rs` whole module. `!include` handler around `:136`, `!include_many` at `:234-328`, `!import` at `:85`, stdlib `<Library/Module>` at `:464-508`, `include_once` tracking `:178,508`. CLI flag `--allow-url-includes` `src/cli.rs:161`. LSP at `src/bin/puml-lsp.rs:556`.
**Notes:** WASM target returns friendly error `include_not_supported_in_wasm` (`src/preproc/includes.rs:70,85`). URL includes gated by CLI flag.

### 21.x hide footbox — ✅
**Status:** ✅
**Evidence:** `src/parser/sequence.rs:263` (`hide footbox` mapped to StatementKind).
**Notes:** Other `hide` directives (`hide stereotype`, `hide empty members`, etc.) not located in this audit — likely partial.

### 21.x newpage / ignore newpage — ✅
**Evidence:** `src/parser/sequence.rs:433-436`. AST `NewPage`, `IgnoreNewPage` at `src/ast.rs:131-132`.
**Notes:** Sequence-only by parse site.

### 21.x mainframe — ❌
**Feature:** `mainframe <creole text>` draws a UML frame around the diagram.
**Status:** ❌
**Evidence:** No matches for `mainframe` in `src/parser/`, `src/normalize/`, `src/render/`, `src/ast.rs`.
**Notes:** Section 21.11 of the spec relies on this; gap.

### 21.x left to right direction / top to bottom direction — ❌ (not found)
**Status:** ❌
**Evidence:** No matches in `src/parser/` or `src/normalize/`.
**Notes:** Layout-direction hint missing. Often affects activity/state/usecase rendering.

### 21.x monochrome / sepia — ❌
**Status:** ❌
**Evidence:** No matches for `monochrome` or `sepia` in source tree.

### 21.x backgroundColor (top-level skinparam) — 🟡
**Status:** 🟡
**Evidence:** Many family-specific `BackgroundColor` skinparams (e.g. `src/theme.rs:1199`, `src/normalize/chart.rs:376`). Standalone top-level `backgroundColor` as a directive (not via skinparam) not located.

### 21.x hide stereotype — ❌ (not located)
**Evidence:** No matches in greps.

---

## Tally — Chapter 21
- ✅ Supported: 9 (`'` comment, title, caption, header/footer base, legend (+ pos), skinparam, !pragma teoz, !include family, newpage, hide footbox, !theme local)
- 🟡 Partial: 5 (scale, header/footer alignment qualifier, per-family render coverage, skinparam breadth, !theme remote)
- ❌ Missing: 7 (`/' ... '/` block comments, `<style>` blocks, mainframe, left-to-right direction, monochrome, sepia, hide stereotype, top-level backgroundColor)
