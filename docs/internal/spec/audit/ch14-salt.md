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

### 14.3 Open/closed droplist (`^X^^item^^item^`) — 🟡
**Feature:** Droplist that "opens" by chaining `^...^^item^^item^`.
**Syntax example:** `^This is an open droplist^^ item 1^^ item 2^`
**Status:** 🟡
**Evidence:** `SaltCell::Combo` exists (`src/ast.rs:189-190`) and `SaltCellRender::Combo` renders a single dropdown. The expanded-list visual (showing items below the field) is not in `SaltCellRender` — only the closed-droplist representation found.
**Notes:** Inputs parse but rendering shows only the field label; the item-list popover is not painted.

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

### 14.8 Tree table (`{T`, `{T!`, `{T-`, `{T+`, `{T#`) — 🟡
**Feature:** Tree combined with column table; variants control grid lines.
**Status:** 🟡
**Evidence:** Tree mode detected (`:634`); table cells handled via grid path. Variants `T!`, `T-`, `T+`, `T#` are all matched by the generic `{t` prefix at `:634` — explicit per-variant grid-line styling is not visible.
**Notes:** Likely renders as a generic tree-table without per-variant border style. Visual gate needed.

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

### 14.11 Menu `{* File | Edit | Source }` (including open menu) — 🟡
**Feature:** Menu bar; "open menu" shows submenu items.
**Status:** 🟡
**Evidence:** `parse_salt_items(line, &["{*", "menu"])` at `src/render/salt.rs:620-623` produces `SaltCellRender::MenuBar(items)`. Open-menu (submenu expansion) — only top-level items captured; submenu rendering not visible.
**Notes:** Basic menu bar: ✅. Open-menu (sub-items popover): ❌ within the same node.

### 14.12 Advanced table (`.` empty, `*` left-span) — ✅
**Feature:** `.` empty cell + `*` left-span cell in `{#` table.
**Status:** ✅
**Evidence:** `transform_salt_table_cell` at `src/render/salt.rs:685-703`: `.` → `TableEmpty`, `*` → `TableSpan`.

### 14.13 Scroll bars `{S`, `{SI`, `{S-` — ✅
**Feature:** Vertical + horizontal, vertical-only, horizontal-only scrollbar containers.
**Status:** ✅
**Evidence:** `parse_salt_scroll_container` at `src/render/salt.rs:653-660`; sets `scroll_vertical`/`scroll_horizontal` flags on text-area lines.

### 14.14 Colors (`<color:Blue>...`, `<color:#9a9a9a>`) — 🟡
**Feature:** Per-widget text-color overrides via inline `<color:...>` tags.
**Status:** 🟡
**Evidence:** No inline-color pipeline found in `src/render/salt.rs` for the cell label rendering — labels are emitted via standard text escaping. Style scope (`<style> saltDiagram { ... }`) IS partially handled (`:537-559`, `:282-311` color setters).
**Notes:** Per-label `<color:...>` runs likely appear as literal text in output. Diagram-wide colors via `<style>` partially work.

### 14.15 Creole on Salt (`**bold**`, `<color:>`, `<U+221E>`, `<&icon>`, `<img:>`) — 🟡
**Feature:** Full Creole + HTML Creole inside salt cells.
**Status:** 🟡
**Evidence:** No Creole inline-run renderer hook in salt SVG path; cell text emitted via `escape_text` style helper. OpenIconic via `<&icon>` not substituted in salt-specific path.
**Notes:** Most Creole tags render as literal text.

### 14.16 Pseudo-sprite `<<name ...XXXX... >>` — ✅
**Feature:** Inline ASCII-art sprite definitions + `<<name>>` references.
**Status:** ✅
**Evidence:** `parse_salt_sprite_def` at `src/render/salt.rs:721-738`; `parse_salt_sprite_ref` at `:740-748`; cell renders for `SpriteDef` and `SpriteRef` exist (`:425-426`, `:500`).
**Notes:** Whether the ASCII bitmap is rasterized to SVG or shown as a placeholder needs visual gate.

### 14.17 OpenIconic in salt (`<&person>`, `<&key>`) — 🟡
**Feature:** OpenIconic icons inside cell text.
**Status:** 🟡
**Evidence:** No OpenIconic icon substitution found in salt cell renderer; tokens treated as text.

### 14.18 title / header / footer / caption / legend — 🟡
**Feature:** Common commands on a salt diagram.
**Status:** 🟡
**Evidence:** Common-command parsing exists at the block level but flow-through to salt SVG output (`src/render/family.rs:2096` "Render a `@startsalt` wireframe grid") is not verified in this audit.

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

**Tally ch14 (24 subsections audited):** ✅ 10 · 🟡 11 · ❌ 3
