# Chapter 11 — Display JSON Data

Audit of PlantUML JSON data syntax against the puml Rust renderer.
Source: `/tmp/puml-spec/ch11-display-json-data.txt`.

### 11.0 @startjson / @endjson block — ✅
**Feature:** Block delimiter keywords for JSON-mode diagrams.
**Syntax example:** `@startjson { "fruit":"Apple" } @endjson`
**Status:** ✅
**Evidence:** `src/parser/blocks.rs:49` (`@startjson` → `BlockKind::Json`), `src/parser/blocks.rs:87`, `src/normalize/structured.rs:3` `normalize_json_document`.
**Notes:** Routes to `NormalizedDocument::Json` (`src/model.rs:111`).

### 11.1 Complex example (nested objects + arrays + null) — ✅
**Feature:** Nested objects, arrays, booleans, numbers, strings, `null`.
**Syntax example:** `{ "address": { "city": "NY" }, "children": [], "spouse": null }`
**Status:** ✅
**Evidence:** `src/normalize/structured.rs:27-85` `flatten_json_value` handles `Object`, `Array`, `String`, `Number`, `Bool`, `Null`.
**Notes:** Parses via `serde_json`. Falls back to per-line tree if parse fails (`src/normalize/structured.rs:11-18`).

### 11.2 Highlight parts (`#highlight "path"`) — ✅
**Feature:** Highlight a JSON path with a colored background.
**Syntax example:** `#highlight "address" / "city"`
**Status:** ✅
**Evidence:** Structured normalization strips `#highlight` controls from the JSON payload before `serde_json` parsing (`src/normalize/structured.rs:112-133`), while render-time controls parse highlight paths (`src/render/data.rs:338-436`) and match them against per-row path metadata (`src/render/data.rs:252-298`, `src/render/data.rs:328-336`). Tests cover nested path highlights and root-array index highlights (`tests/json_yaml_projection_depth.rs:80-100`, `tests/json_yaml_projection_depth.rs:151-177`).

### 11.3 Different styles for highlight (`<<h1>>`, `<style>` `.h1` `.h2`) — ✅
**Feature:** Per-highlight stereotype with `<style>` blocks.
**Syntax example:** `#highlight "address" / "city" <<h1>>`
**Status:** ✅
**Evidence:** `parse_highlight` extracts optional `<<class>>` names (`src/render/data.rs:380-435`), `parse_style_lines` / `apply_style_property` collect `jsonDiagram highlight` defaults and `.class` overrides (`src/render/data.rs:438-534`), and row rendering emits `data-json-highlight-class` plus patched fill/font styling (`src/render/data.rs:252-320`). Fixture coverage includes `.hot` styling (`tests/fixtures/structured/valid_json_highlight_projection.puml:1-26`) and asserts the red fill/italic class render (`tests/json_yaml_projection_depth.rs:80-100`).

### 11.4 JSON basic elements (null/true/false/number/string/object/array) — ✅
**Feature:** All primitive JSON types and nested structures.
**Status:** ✅
**Evidence:** `src/normalize/structured.rs:34-84`.
**Notes:** JSON scalar parsing is complete; render-time inline Creole handling is tracked separately in 11.14 because it is not full PlantUML text-extension parity.

### 11.5 Array or table (root array, number/string/boolean arrays) — 🟡
**Feature:** Top-level arrays, including minimal forms `[1,2,3]`, `["a","b"]`, `[true,false]`.
**Syntax example:** `@startjson [1, 2, 3] @endjson`
**Status:** 🟡
**Evidence:** `src/normalize/structured.rs:47-58` handles arrays via `serde_json::Value::Array`.
**Notes:** Functionally parses, but renders as the same indented-tree layout as objects, not as the PlantUML horizontal-table layout for arrays. Visual fidelity gap. See `src/render/data.rs:3-98` — single per-row tree style, no table mode.

### 11.6 JSON numbers (decimals, exponents) — ✅
**Feature:** Integers, decimals, exponent notation (`1E5`).
**Status:** ✅
**Evidence:** `src/normalize/structured.rs:66-70` (serde_json handles all numeric forms).

### 11.7 JSON strings (Unicode, `\uXXXX`, escape sequences) — ✅
**Feature:** Unicode literal + `\uXXXX` escapes; standard two-character escapes.
**Status:** ✅
**Evidence:** Delegated to `serde_json` (`src/normalize/structured.rs:5`).
**Notes:** JSON string escape decoding is complete through serde_json; render-time inline Creole handling is tracked separately in 11.14.

### 11.8 Minimal JSON examples (scalar root: `"Hello"`, `42`, `true`) — ✅
**Feature:** Scalar at the root.
**Status:** ✅
**Evidence:** `src/normalize/structured.rs:60-83` — scalars produce a single node.

### 11.9 Empty table / list (`[]`, `{}`) — ✅
**Feature:** Empty array and object values.
**Status:** ✅
**Evidence:** serde_json handles; flatten emits `{...}`/`[...]` header with no children.
**Notes:** Header label format differs from PlantUML's compact `{}`/`[]` rendering.

### 11.10 Using (global) style — 🟡
**Feature:** `<style> jsonDiagram { node { ... } arrow { ... } highlight { ... } }` global skin.
**Syntax example:** `<style> jsonDiagram { node { BackGroundColor Khaki } } </style>`
**Status:** 🟡
**Evidence:** JSON `<style>` blocks are stripped from payload parsing (`src/normalize/structured.rs:112-133`) and render-time style parsing supports `jsonDiagram { highlight { ... } }` plus `.class` highlight overrides (`src/render/data.rs:338-534`). Node and arrow selectors still use renderer defaults (`RowStyle::json_node` at `src/render/data.rs:63-72`; connector colors at `src/render/data.rs:26-37`), so `node { ... }` / `arrow { ... }` are not full parity.
**Notes:** Highlight styling is now wired; global node/connector styling remains a gap.

### 11.11 JSON inside class/object diagram (`json J { ... }`) — ✅
**Feature:** Inline JSON block inside `@startuml` as a class-like entity.
**Syntax example:** `json JSON { "fruit":"Apple" }`
**Status:** ✅
**Evidence:** `src/parser/projection_salt.rs:1-78` parses `json $alias { ... }` into `StatementKind::JsonProjection`; AST node at `src/ast.rs:161-166`; model field `json_projections` at `src/model.rs:534`. Family rendering emits projection boxes through `render_family_projection_boxes` (`src/render/family.rs:1377-1459`), and tests verify aliases, nested rows, and projection connectors in UML contexts (`tests/integration.rs:7395-7435`, `tests/local_parity_blitz.rs:146-166`).
**Notes:** This covers parser and visual rendering for JSON projection boxes; broader `allowmixing` semantics are tracked in 11.12.

### 11.12 JSON on Deployment/Usecase/Component (`allowmixing`) — 🟡
**Feature:** `allowmixing` + `json JSON { ... }` mixed with `component`, `actor`, etc.
**Status:** 🟡
**Evidence:** `JsonProjection` exists (`src/ast.rs:161`); `allowmixing` handling not confirmed in this audit. Same projection path as 11.11.
**Notes:** Component/deployment projection boxes and connectors have coverage (`tests/local_parity_blitz.rs:146-166`), but the broader PlantUML `allowmixing` directive semantics and all listed families are not verified as full parity.

### 11.13 JSON on State diagram — 🟡
**Feature:** `json J { ... }` inside a state diagram.
**Status:** 🟡
**Evidence:** Same projection mechanism; state-diagram integration unverified.

### 11.14 Creole on JSON values — 🟡
**Feature:** Render Creole markup (`**bold**`, `<color:...>`, `<img:...>`, `<U+...>`, `<&icon>`) inside JSON string values.
**Status:** 🟡
**Evidence:** Structured row labels now render through `creole_text` (`src/render/data.rs:307-320`), which supports inline Creole spans and Unicode escape decoding through the shared SVG text path (`src/render/svg.rs:1-30`, `src/render/svg.rs:125-138`). JSON highlight tests assert bold scalar content emits styled tspans (`tests/json_yaml_projection_depth.rs:80-100`).
**Notes:** This covers the existing inline Creole pipeline, not every PlantUML text extension listed here; image/icon catalogue behavior is still not verified as 1:1.

---

**Tally ch11 (15 entries audited):** ✅ 10 · 🟡 5 · ❌ 0
