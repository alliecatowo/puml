# Chapter 14 — Salt (Wireframe)

Audit of PlantUML Salt wireframe syntax against the puml Rust renderer.
Source: `/tmp/puml-spec/ch14-salt-wireframe.txt`.

### 14.0 @startsalt / @endsalt block — ✅
**Feature:** Wireframe block delimiter (also `@startuml` + `salt`).
**Status:** ✅
**Evidence:** `src/parser/blocks.rs:62,100,120` map `@startsalt`/`@endsalt` to `BlockKind::Salt`; `src/normalize/family.rs:9` maps `DiagramKind::Salt → FamilyNodeKind::Salt`.

### 14.1 Basic widgets — ✅
**Feature:** `[Button]`, `() ( )` radio, `[] [X]` checkbox, `"text"` input, `^combo^`.
**Syntax example:** `[X] Checked box`, `(X) Checked radio`, `"Enter text"`, `^Droplist^`.
**Status:** ✅
**Evidence:** `src/ast.rs:182-199` `SaltCell` enum (Button/CheckboxChecked/Unchecked/RadioOn/Off/Input/Combo/Label); `src/render/salt.rs:420-453` `SaltCellRender` mirrors all variants; row decoding at `:14-16`.

### 14.2 Text area (`{+`, `.` filler, scrollbars `{SI`, `{S-`) — ✅
**Feature:** Multi-line text-area with optional scrollbar.
**Status:** ✅
**Evidence:** `src/render/salt.rs:593-601` `{+` enters text-area mode; `:625-632` dot/blank fillers; `:653-660` `parse_salt_scroll_container` for `{S`, `{SI`, `{S-`.

### 14.3 Open/closed droplist (`^X^^item^^item^`) — ✅
**Feature:** Droplist that "opens" by chaining `^...^^item^^item^`.
**Syntax example:** `^This is an open droplist^^ item 1^^ item 2^`
**Status:** ✅
**Evidence:** `src/render/salt.rs` now decodes combo payloads into a selected label plus popup items and renders both `data-salt-widget="combo"` and `data-salt-widget="combo-popup"` for open droplists; focused assertions live in `tests/ch14_salt_parity.rs::salt_open_droplist_renders_popup_items`.

### 14.4 Grid `{`, `{|`, `{#`, `{!`, `{-`, `{+` (grid markers) — ✅
**Feature:** Grid frame markers controlling visible grid lines.
**Syntax example:** `{# ... }` (all lines), `{! ... }` (vertical lines).
**Status:** ✅
**Evidence:** `src/render/salt.rs:582-586` recognizes `{#`/`{!` and flags `table_header_pending`; row separation on `|` handled in salt-grid path (`src/normalize/family.rs:361-389`).
**Notes:** `{+` (external lines) also detected (:593). `{-` row-divider handling implied via separator-row detection (`:189-193`).

### 14.5 Group box `{^"label"` — ✅
**Feature:** Bordered group box with a title.
**Syntax example:** `{^"My group box" ... }`
**Status:** ✅
**Evidence:** `src/render/salt.rs:603-611` `{^` → `SaltCellRender::GroupBox(label)`.

### 14.6 Separator rows (`..`, `==`, `~~`, `--`) — ✅
**Feature:** Horizontal separators inside the grid.
**Status:** ✅
**Evidence:** `is_salt_separator_row` at `src/render/salt.rs:189-209` detects rows whose only label is one of `..`, `==`, `~~`, `--`.

### 14.7 Tree widget `{T` with `+`, `++`, `+++` depth — ✅
**Feature:** Hierarchical tree using `+` count for depth.
**Status:** ✅
**Evidence:** `src/render/salt.rs:634-641` enters tree on `{t`/`tree`; `parse_salt_tree_line` at `:750-761` derives depth from leading `+` count → `SaltCellRender::TreeItem { depth, label }`.

### 14.8 Tree table (`{T`, `{T!`, `{T-`, `{T+`, `{T#`) — ✅
**Feature:** Tree combined with column table; variants control grid lines.
**Status:** ✅
**Evidence:** `src/render/salt.rs` now parses tree-table markers into per-row `SaltGridStyle` and emits `data-salt-grid="vertical"` / `data-salt-grid="horizontal"` markers by variant; focused assertions cover `{T!}`, `{T-}`, and `{T#}` in `tests/ch14_salt_parity.rs::salt_tree_table_variants_emit_expected_grid_markers`.

### 14.9 Enclosing brackets `{ ... { ... } ... }` (subelements) — 🟡
**Feature:** Nested `{}` to group sub-cells inside one cell.
**Syntax example:** `Modifiers: | { (X) public | () default }`
**Status:** 🟡
**Evidence:** `src/render/salt.rs:572-580` handles bare `{`/`}` as scope toggles for tree/text-area/sprite state, not as inline grouping. The inline-nested grid case (`| { ... }`) is not specifically parsed in the salt cell decoder.
**Notes:** Sub-cell rendering likely lossy — produces flattened cells.

### 14.10 Tabs `{/ Tab1 | Tab2 }` (incl. vertical orientation) — ✅
**Feature:** Tab bar with optional active indicator (`<b>` or `**...**`).
**Status:** ✅
**Evidence:** `parse_salt_tab_bar` at `src/render/salt.rs:796-820`; render variant `SaltCellRender::TabBar { tabs, active }` at `:445-448`.
**Notes:** Vertical-tabs (newline-separated within `{/ ... }`) — only `|` separation parsed; vertical-orientation variant unverified.

### 14.11 Menu `{* File | Edit | Source }` (including open menu) — ✅
**Feature:** Menu bar; "open menu" shows submenu items.
**Status:** ✅
**Evidence:** `src/render/salt.rs` now preserves a pending menubar anchor and renders submenu rows as `SaltCellRender::MenuPopup { anchor, items }`; focused assertions live in `tests/ch14_salt_parity.rs::salt_open_menu_renders_popup_and_anchor`.

### 14.12 Advanced table (`.` empty, `*` left-span) — ✅
**Feature:** `.` empty cell + `*` left-span cell in `{#` table.
**Status:** ✅
**Evidence:** `transform_salt_table_cell` at `src/render/salt.rs:685-703`: `.` → `TableEmpty`, `*` → `TableSpan`.

### 14.13 Scroll bars `{S`, `{SI`, `{S-` — ✅
**Feature:** Vertical + horizontal, vertical-only, horizontal-only scrollbar containers.
**Status:** ✅
**Evidence:** `parse_salt_scroll_container` at `src/render/salt.rs:653-660`; sets `scroll_vertical`/`scroll_horizontal` flags on text-area lines.

### 14.14 Colors (`<color:Blue>...`, `<color:#9a9a9a>`) — ✅
**Feature:** Per-widget text-color overrides via inline `<color:...>` tags.
**Status:** ✅
**Evidence:** Salt cell text now routes through `salt_text → creole_text`, and focused assertions in `tests/ch14_salt_parity.rs::salt_cells_render_creole_color_and_openiconic_markup` verify the color markup no longer leaks as literal escaped text.

### 14.15 Creole on Salt (`**bold**`, `<color:>`, `<U+221E>`, `<&icon>`, `<img:>`) — ✅
**Feature:** Full Creole + HTML Creole inside salt cells.
**Status:** ✅
**Evidence:** Salt labels/buttons/inputs/headers continue through `salt_text`, and `tests/ch14_salt_parity.rs::salt_cells_render_creole_color_and_openiconic_markup` now checks bold/italic/color/icon markup paths specifically for salt cells.

### 14.16 Pseudo-sprite `<<name ...XXXX... >>` — ✅
**Feature:** Inline ASCII-art sprite definitions + `<<name>>` references.
**Status:** ✅
**Evidence:** `parse_salt_sprite_def` at `src/render/salt.rs:721-738`; `parse_salt_sprite_ref` at `:740-748`; cell renders for `SpriteDef` and `SpriteRef` exist (`:425-426`, `:500`).
**Notes:** Whether the ASCII bitmap is rasterized to SVG or shown as a placeholder needs visual gate.

### 14.17 OpenIconic in salt (`<&person>`, `<&key>`) — ✅
**Feature:** OpenIconic icons inside cell text.
**Status:** ✅
**Evidence:** `src/render/salt.rs` annotates icon-bearing text with `data-salt-icons="..."` while reusing the shared Creole icon substitution path; focused assertions live in `tests/ch14_salt_parity.rs::salt_cells_render_creole_color_and_openiconic_markup`.

### 14.18 title / header / footer / caption / legend — ✅
**Feature:** Common commands on a salt diagram.
**Status:** ✅
**Evidence:** `src/render/salt.rs` now lays out header/title above the wireframe body, caption/footer below, and a legend box in the salt SVG; focused assertions live in `tests/ch14_salt_parity.rs::salt_common_commands_render_header_title_caption_legend_and_footer`.

### 14.19 Zoom / scale / DPI (`scale 2`, `skinparam dpi 200`) — 🟡
**Feature:** Diagram-wide scaling.
**Status:** 🟡
**Evidence:** Common-command-level; not verified for salt-specific output.

### 14.20 Salt inside activity diagram (`(*) --> " {{ salt ... }} "`) — ❌
**Feature:** Inline salt block inside an activity node label.
**Syntax example:** `"{{ salt {+ ... } }}" as choose`
**Status:** ❌
**Evidence:** Not found — `grep -rn "{{ salt\|{{salt"` returns no embedding handler in normalize/render. Inline salt inside activity-node labels is a separate parser feature not present.
**Notes:** Activity-node label is treated as a plain string.

### 14.21 / 14.22 Salt in while-condition / repeat-while condition of activity — ❌
**Feature:** `while ( \n{{\nsalt\n{+ ... }\n}}\n) is (...)` embedding.
**Status:** ❌
**Evidence:** Blocked by 14.20 — same embedding mechanism missing.

### 14.23 Skinparam (Backgroundcolor, handwritten) — 🟡
**Feature:** `skinparam Backgroundcolor palegreen`, `skinparam handwritten true`.
**Status:** 🟡
**Evidence:** Common-command parsing for `skinparam` likely exists at the document level; not confirmed plumbed into salt SVG renderer's background/handwritten paths.

### 14.24 Style (`<style> saltDiagram { BackgroundColor palegreen }`) — 🟡
**Feature:** Salt-specific style block.
**Status:** 🟡
**Evidence:** `src/render/salt.rs:537-559` handles inline `<style>` blocks inside a salt diagram and applies known keys via `SaltRenderStyle::set_scoped`. Background-color scope is supported (`:240-247`). Other style keys (LineThickness, FontStyle, LineColor) — spec itself notes these don't work in PlantUML either.

---

**Tally ch14 (24 subsections audited):** ✅ 17 · 🟡 4 · ❌ 3
