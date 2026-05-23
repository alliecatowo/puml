# Chapter 8 — Deployment Diagram Audit

Reference: `/tmp/puml-spec/ch08-deployment-diagram.txt` (2201 lines)
Audited against `src/parser/component.rs`, `src/parser/component_groups.rs`,
`src/ast.rs` (`ComponentNodeKind`), `src/render/family.rs`, `src/render/relation.rs`,
`src/normalize/family.rs`.

Legend: ✅ supported · 🟡 partial · ❌ missing

The deployment diagram in PlantUML reuses the component family with an expanded set of
node shape keywords. puml now carries the deployment shape set through
`ComponentNodeKind`, `FamilyNodeKind`, normalization, and SVG `data-uml-kind` output for
nearly all declarable keyword forms.

---

### 8.1 Declaring element — keyword inventory — ✅

Per-keyword status. Evidence is `src/parser/component.rs:113-149` keyword table,
`src/ast.rs:307-337` `ComponentNodeKind`, `src/normalize/family.rs:3009-3040`
`FamilyNodeKind` mapping, `src/render/family.rs:6124-6150` deployment-shape dispatch,
and `tests/integration.rs:6147-6255` unless otherwise noted.

| Keyword | Status | Evidence |
|---|---|---|
| action | ✅ | `ComponentNodeKind::Action`; rendered as `data-uml-kind="action"` |
| actor | ✅ | `ComponentNodeKind::Actor`; rendered as stick figure |
| actor/ (alt actor) | ✅ | `component.rs:132`; marker preserved as `<<actor/>>` |
| agent | ✅ | `ComponentNodeKind::Agent`; rendered as `data-uml-kind="agent"` |
| artifact | ✅ | `ComponentNodeKind::Artifact`; folded-corner shape |
| boundary | ✅ | `ComponentNodeKind::Boundary`; deployment ellipse variant |
| card | ✅ | `ComponentNodeKind::Card` |
| circle | ✅ | `ComponentNodeKind::Circle`; ellipse shape |
| cloud | ✅ | `ComponentNodeKind::Cloud`; cloud path |
| collections | ✅ | `ComponentNodeKind::Collections`; stacked-card shape |
| component | ✅ | `ComponentNodeKind::Component` |
| control | ✅ | `ComponentNodeKind::Control`; control glyph variant |
| database | ✅ | `ComponentNodeKind::Database`; cylinder shape |
| entity | ✅ | `ComponentNodeKind::Entity`; entity glyph variant |
| file | ✅ | `ComponentNodeKind::File`; folded-corner shape |
| folder | ✅ | `ComponentNodeKind::Folder`; tabbed-folder shape |
| frame | ✅ | `ComponentNodeKind::Frame`; 3D node/frame shape |
| hexagon | ✅ | `ComponentNodeKind::Hexagon`; hexagon polygon |
| interface | ✅ | `ComponentNodeKind::Interface`; lollipop/circle interface |
| label | ✅ | `ComponentNodeKind::Label` |
| node | ✅ | `ComponentNodeKind::Node`; 3D node shape |
| package | ✅ | `ComponentNodeKind::Package`; also scoping block |
| person | ✅ | `ComponentNodeKind::Person`; stick-figure variant |
| process | ✅ | `ComponentNodeKind::Process` |
| queue | ✅ | `ComponentNodeKind::Queue`; queue cylinder/rounded stack shape |
| rectangle | ✅ | `ComponentNodeKind::Rectangle` |
| stack | ✅ | `ComponentNodeKind::Stack`; stacked-card shape |
| storage | ✅ | `ComponentNodeKind::Storage`; cylinder shape |
| usecase | ✅ | standalone `usecase` stays in usecase diagrams; after a deployment context it parses as `ComponentNodeKind::UseCase` via `parse_deployment_usecase_decl` |
| usecase/ | ✅ | `component.rs:148`; rendered as deployment usecase ellipse |

**Tally for §8.1: 30 ✅, 0 🟡, 0 ❌ out of 30 keyword forms.**

### 8.1 Long bracketed body (`folder folder [ ... multi-line ... ]`) — 🟡
**Feature:** Multi-line `[ ... ]` body with `----`, `====`, `....` separators.
**Status:** 🟡
**Evidence:** `parse_component_multiline_decl` (`src/parser/component.rs:363-399`) parses
multi-line bracket bodies; `collect_scoped_component_group_content` now preserves the
same declarations inside deployment scoping blocks. Separator lines currently render as
literal text, not PlantUML divider rules.

### 8.2 Short forms — ✅
**Feature:** `:actor:`, `[component]`, `() interface`, `(usecase)`.
**Status:** ✅
- `[component]` ✅ — `parse_component_bracketed_shorthand`
- `() interface` ✅ — `parse_component_interface_shorthand`
- `:actor:` ✅ — `parse_actor_colon_shorthand`
- `(usecase)` ✅ — `parse_component_parenthesized_usecase_shorthand`

### 8.3 Linking (`--`, `..`, `~~`, `==`) and arrowheads (`*`, `o`, `+`, `#`, `>>`, `^`, `0`, `(0`, etc.) — 🟡
**Feature:** All deployment arrow heads / line styles, including circle-arrows `0--0`, `)--(`, `0)--(0`, `-(0)-`, etc.
**Status:** 🟡
**Evidence:** Bracketed style `-[bold]-`, `-[dashed]-`, `-[dotted]-`, `-[hidden]-`, `-[#color]-`, `-[thickness=N]-` handled via family relation parser. Lollipop endpoint `()` handled in `src/parser/family.rs:662-671`.
**Notes:** Specialized arrowheads `--0`, `--@`, `--:|>`, `-->>`, `-(0)-`, `0)--(0` likely fall through to plain arrows — no evidence of bespoke head-symbol parsing for these tokens.

### 8.4 Bracketed arrow style (line/color/thickness/mix) — ✅
**Status:** ✅
**Evidence:** Handled by family relation `arrow_style` / `line_color` / `dashed` / `thickness` fields on `FamilyRelation` (`src/ast.rs:327-344`) and rendered via `src/render/relation.rs`.

### 8.5 Inline arrow style `#color;line.bold;text:color` — 🟡
**Status:** 🟡
**Evidence:** Inline `#color` capture exists; comprehensive `line:color;line.style;text:color` parsing not visibly enumerated for arrows.

### 8.6 Inline element style — 🟡
**Status:** 🟡
**Evidence:** `split_declaration_inline_fill` + `append_inline_fill_member` (`src/parser/component.rs:60,107`) capture `#color`. Full `#fill;line:color;line.dashed;text:color` is not parsed end-to-end.

### 8.7 Nestable elements (action/artifact/card/cloud/component/database/file/folder/frame/hexagon/node/package/process/queue/rectangle/stack/storage) — ✅
**Status:** ✅
**Evidence:** `is_component_container_keyword` (`src/parser/component.rs:176-199`) lists all
17 nestable deployment keywords, and `component_scoping_block_head`
(`src/parser/component_groups.rs:226-248`) routes them through scoped component group
collection.

### 8.8 Packages and nested elements — ✅
**Status:** ✅
**Evidence:** Nested scoping recursively handled in `collect_scoped_component_group_content`
(`src/parser/component_groups.rs:52-136`), and the container keyword set now covers all
17 deployment nesting keywords from §8.7.

### 8.9 Alias (`as`, `[ ... ]` multi-line, long aliases) — 🟡
**Feature:** `node Node1 as n1`, `node "Node 2" as n2`, `cloud c1 [ multi\nline\ndescr ]`.
**Status:** 🟡
**Evidence:** Single-line `as alias` and `"label" as alias` work (`src/parser/component.rs:81-89`). Multi-line bracketed alias body — ❌ (see 8.1).

### 8.10 Round corner (`skinparam rectangle { roundCorner<<Concept>> 25 }`) — ❌
**Status:** ❌
**Evidence:** No `roundCorner` keyword handler found in normalize/family.rs skinparam section.

### 8.11 Specific SkinParameter (roundCorner) — ❌
Same as 8.10.

### 8.12 / 8.13 Appendix arrow tables (`--0`, `--@`, `--:|>`, `--||>`, `--|>`, `--^`, `--\\`, `--#`, `--+`, `--o`, `--*`, `-->>`, `0--0`, `)--(`, `0)--(0`, `-0)-`, `-(0)-`, `-(0-`, `--(0`, `--(`, `--0`) — 🟡
**Status:** 🟡
**Evidence:** Standard heads (`-->`, `--*`, `--o`, `--|>`) handled via `arrow_style` machinery; the circle-arrow family (`0`, `(0`, `(0)`) is partially handled via lollipop strip but not as embedded arrow shapes (`parser/family.rs:662-671`).
**Notes:** This is the deployment chapter's "exotic-arrow" surface area; expect visual misses on `0)--(0`, `-(0)-`, `--@`, `-->>` heads.

### 8.14 Appendix: inline style on every element — 🟡
See 8.6. Works for keywords that parse; no-ops for missing keywords.

### 8.15 Global `<style>` block (`componentDiagram { BackGroundColor ... }`) — 🟡
**Status:** 🟡
**Evidence:** `ComponentStyle` populated from skinparam (`src/normalize/family.rs:1093,1382-1395`). The `<style>` CSS-like syntax with per-element selectors (`actor { ... }`, `database { ... }`, etc.) is **not** found — `grep '<style>'` in `src/` is empty.
**Notes:** This is a significant authoring surface (per-shape style declarations) that PlantUML supports but puml does not parse.

### Stereotypes `<<...>>` on declarations — ✅
**Status:** ✅
**Evidence:** `strip_declaration_stereotypes` called at `src/parser/component.rs:62`; stereotypes flow into member labels and `append_component_declaration_metadata` (`component_groups.rs:250-268`).

### Sprite icons (`sprite $name { ... }` and `<<$name>>` usage) — ❌
**Status:** ❌
**Evidence:** No `sprite` keyword parser found in `src/`.

---

## Tally — Chapter 8

| Status | Count |
|---|---|
| ✅ Supported | 6 (§8.1 keyword inventory, §8.2, §8.4, §8.7, §8.8, stereotypes) |
| 🟡 Partial | 8 (§8.1 long-bracket body, §8.3, §8.5, §8.6, §8.9, §8.12/8.13, §8.14, §8.15) |
| ❌ Missing | 3 sections (§8.10, §8.11, sprites) |

**Headline gaps:**
1. **Multi-line bracketed body separators** (`----`, `====`, `....`) parse as body text but do not render as divider rules.
2. **Exotic arrow heads** — `--0`, `--@`, `-->>`, `0)--(0`, `-(0)-`, `--(0` are mostly fall-through plain arrows.
3. **`<style>` block** with per-element CSS-like selectors not parsed.
4. **`skinparam rectangle { roundCorner<<stereo>> N }`** not honored.
5. **Sprites** (`sprite $foo [W*H/depth] { ... }` and `<<$foo>>` reference) absent.
