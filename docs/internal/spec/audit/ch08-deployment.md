# Chapter 8 — Deployment Diagram Audit

Reference: `/tmp/puml-spec/ch08-deployment-diagram.txt` (2201 lines)
Audited against `src/parser/component.rs`, `src/parser/component_groups.rs`,
`src/ast.rs` (`ComponentNodeKind`), `src/render/family.rs`, `src/render/relation.rs`,
`src/normalize/family.rs`.

Legend: ✅ supported · 🟡 partial · ❌ missing

The deployment diagram in PlantUML reuses the component family with an expanded set of
node shape keywords. The single biggest gap in puml is the set of shape keywords that
parse to `ComponentNodeKind` — only **15** kinds exist (`src/ast.rs:273-290`), but the
PlantUML spec lists **~28** declarable keywords.

---

### 8.1 Declaring element — keyword inventory — 🟡

Per-keyword status. Evidence is `src/parser/component.rs:3-21` keyword table and
`src/ast.rs:273-290` `ComponentNodeKind` enum unless otherwise noted.

| Keyword | Status | Evidence |
|---|---|---|
| action | ❌ | not in `ComponentNodeKind`; activity-only |
| actor | ✅ | `component.rs:20`, `ComponentNodeKind::Actor` |
| actor/ (alt actor) | ❌ | no `actor/` variant token recognized |
| agent | ❌ | absent from keyword table |
| artifact | ✅ | `component.rs:19`, `Artifact` |
| boundary | ❌ | only as sequence `ParticipantRole::Boundary` (`parser/sequence.rs:5`); no deployment declaration |
| card | ✅ | `component.rs:18`, `Card` |
| circle | ❌ | no shape kind |
| cloud | ✅ | `component.rs:11`, `Cloud` |
| collections | ❌ | sequence-only role |
| component | ✅ | `component.rs:4`, `Component` |
| control | ❌ | sequence-only role |
| database | ✅ | `component.rs:10`, `Database` |
| entity | ❌ | sequence-only role |
| file | ✅ | `component.rs:17`, `File` |
| folder | ✅ | `component.rs:16`, `Folder` |
| frame | ✅ | `component.rs:12`, `Frame` |
| hexagon | ❌ | not in keyword table |
| interface | ✅ | `component.rs:5`, `Interface` |
| label | ❌ | not in keyword table |
| node | ✅ | `component.rs:9`, `Node` |
| package | ✅ | `component.rs:14`, `Package`; also scoping block (`component_groups.rs:9-15`) |
| person | ❌ | exists as `C4Person` only (`render/family.rs:2185`) |
| process | ❌ | not in keyword table |
| queue | ❌ | sequence-only `ParticipantRole::Queue`; no component kind |
| rectangle | ✅ | `component.rs:15`, `Rectangle` |
| stack | ❌ | not in keyword table |
| storage | ✅ | `component.rs:13`, `Storage` |
| usecase | 🟡 | parsed via `parser/family.rs:139` (`usecase` keyword in family decls) but not as a `ComponentNodeKind` |
| usecase/ | ❌ | no alt-syntax variant |

**Tally for §8.1: 14 ✅, 1 🟡, 15 ❌ out of 30 keyword forms.**

### 8.1 Long bracketed body (`folder folder [ ... multi-line ... ]`) — ❌
**Feature:** Multi-line `[ ... ]` body with `----`, `====`, `....` separators.
**Status:** ❌
**Evidence:** `parse_component_decl` (`src/parser/component.rs:64-77`) reads `[label]` as single-line only; no multi-line block scanning.

### 8.2 Short forms — 🟡
**Feature:** `:actor:`, `[component]`, `() interface`, `(usecase)`.
**Status:** 🟡
- `[component]` ✅ — `component.rs:116-146`
- `() interface` ✅ — `component.rs:149-186`, `:204-214`
- `:actor:` ❌ — no parser path for the `:Name:` shorthand
- `(usecase)` ❌ — no parser path for the `(Name)` shorthand in component diagrams

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

### 8.7 Nestable elements (action/artifact/card/cloud/component/database/file/folder/frame/hexagon/node/package/process/queue/rectangle/stack/storage) — 🟡
**Status:** 🟡
**Evidence:** Of 17 nestable keywords, puml supports `artifact, card, cloud, component, database, file, folder, frame, node, package, rectangle, storage` (12). Missing: `action`, `hexagon`, `process`, `queue`, `stack`. Explicit scoping-block list at `src/parser/component_groups.rs:113-127` only enumerates `package|namespace|node|frame|cloud|rectangle`; other containers depend on the generic `{`-fallthrough in `find_family_decl_end`.

### 8.8 Packages and nested elements — 🟡
**Status:** 🟡
**Evidence:** Nested scoping recursively handled in `collect_scoped_component_group_content` (`src/parser/component_groups.rs:91-203`). Deep alphabetical / reverse-alphabetical full nesting (§8.8.3) is gated by which keywords parse as containers — only the 12 listed in §8.7 work.

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
**Evidence:** `strip_declaration_stereotypes` called at `src/parser/component.rs:62`; stereotypes flow into member labels and `append_component_declaration_metadata` (`component_groups.rs:225-243`).

### Sprite icons (`sprite $name { ... }` and `<<$name>>` usage) — ❌
**Status:** ❌
**Evidence:** No `sprite` keyword parser found in `src/`.

---

## Tally — Chapter 8

| Status | Count |
|---|---|
| ✅ Supported | 3 (§8.4, stereotypes, +12 of 30 keywords in §8.1) |
| 🟡 Partial | 10 (§8.2, 8.3, 8.5, 8.6, 8.7, 8.8, 8.9, 8.12/8.13, 8.14, 8.15) |
| ❌ Missing | 5 sections + ~15 keywords (§8.1 long-bracket body, 8.10, 8.11, sprites; keywords: action, actor/, agent, boundary, circle, collections, control, entity, hexagon, label, person, process, queue, stack, usecase/) |

**Headline gaps:**
1. **15 shape keywords missing** — `agent, hexagon, queue, stack, process, action, person, label, circle, boundary, control, entity, collections, actor/, usecase/`. `ComponentNodeKind` needs ~15 new variants and matching render shapes.
2. **Multi-line bracketed bodies** (`folder f [\n line1\nline2\n]`) — single-line only.
3. **Exotic arrow heads** — `--0`, `--@`, `-->>`, `0)--(0`, `-(0)-`, `--(0` are mostly fall-through plain arrows.
4. **`<style>` block** with per-element CSS-like selectors not parsed.
5. **`skinparam rectangle { roundCorner<<stereo>> N }`** not honored.
6. **Sprites** (`sprite $foo [W*H/depth] { ... }` and `<<$foo>>` reference) absent.
