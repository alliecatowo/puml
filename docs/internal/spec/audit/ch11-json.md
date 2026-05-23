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

### 11.2 Highlight parts (`#highlight "path"`) — ❌
**Feature:** Highlight a JSON path with a colored background.
**Syntax example:** `#highlight "address" / "city"`
**Status:** ❌
**Evidence:** not found — `grep -rn "highlight" src/normalize/structured.rs src/render/data.rs` returns nothing; the only `highlight` parser is in `src/parser/timing.rs` (timing diagrams). The raw block is fed directly to `serde_json::from_str`, so `#highlight` lines would cause a parse failure and trigger the per-line fallback.
**Notes:** No `JsonHighlight` AST node exists. Lines starting with `#` would be treated as comments by yaml fallback but not stripped from JSON input → parse error path.

### 11.3 Different styles for highlight (`<<h1>>`, `<style>` `.h1` `.h2`) — ❌
**Feature:** Per-highlight stereotype with `<style>` blocks.
**Syntax example:** `#highlight "address" / "city" <<h1>>`
**Status:** ❌
**Evidence:** not found. No highlight feature at all.
**Notes:** Blocked by 11.2.

### 11.4 JSON basic elements (null/true/false/number/string/object/array) — ✅
**Feature:** All primitive JSON types and nested structures.
**Status:** ✅
**Evidence:** `src/normalize/structured.rs:34-84`.
**Notes:** Inline color tokens like `"<color:green>TBC"` are preserved as raw text in the label but not interpreted (no Creole pass on JSON values).

### 11.5 Array or table (root array, number/string/boolean arrays) — 🟡
**Feature:** Top-level arrays, including minimal forms `[1,2,3]`, `["a","b"]`, `[true,false]`.
**Syntax example:** `@startjson [1, 2, 3] @endjson`
**Status:** 🟡
**Evidence:** `src/normalize/structured.rs:47-58` handles arrays via `serde_json::Value::Array`.
**Notes:** Functionally parses and now renders through the shared key/value table mode with indentation and tree connectors (`src/render/data.rs`). This improves the PlantUML-like horizontal-table feel for arrays, but remains 🟡 until broader styling/projection parity is complete.

### 11.6 JSON numbers (decimals, exponents) — ✅
**Feature:** Integers, decimals, exponent notation (`1E5`).
**Status:** ✅
**Evidence:** `src/normalize/structured.rs:66-70` (serde_json handles all numeric forms).

### 11.7 JSON strings (Unicode, `\uXXXX`, escape sequences) — ✅
**Feature:** Unicode literal + `\uXXXX` escapes; standard two-character escapes.
**Status:** ✅
**Evidence:** Delegated to `serde_json` (`src/normalize/structured.rs:5`).
**Notes:** Inline Creole tags (`<color:blue>`) preserved as text but not rendered as styled inline runs in the SVG (data.rs uses plain `<text>`).

### 11.8 Minimal JSON examples (scalar root: `"Hello"`, `42`, `true`) — ✅
**Feature:** Scalar at the root.
**Status:** ✅
**Evidence:** `src/normalize/structured.rs:60-83` — scalars produce a single node.

### 11.9 Empty table / list (`[]`, `{}`) — ✅
**Feature:** Empty array and object values.
**Status:** ✅
**Evidence:** serde_json handles; flatten emits `{...}`/`[...]` header with no children.
**Notes:** Header label format differs from PlantUML's compact `{}`/`[]` rendering.

### 11.10 Using (global) style — ❌
**Feature:** `<style> jsonDiagram { node { ... } arrow { ... } highlight { ... } }` global skin.
**Syntax example:** `<style> jsonDiagram { node { BackGroundColor Khaki } } </style>`
**Status:** ❌
**Evidence:** not found. `render_json_svg` (`src/render/data.rs:3-98`) uses hard-coded colors (`#f1f5f9`, `#94a3b8`, `#0f172a`) with no style lookup.
**Notes:** No connection to the style engine for JSON/YAML renderers.

### 11.11 JSON inside class/object diagram (`json J { ... }`) — 🟡
**Feature:** Inline JSON block inside `@startuml` as a class-like entity.
**Syntax example:** `json JSON { "fruit":"Apple" }`
**Status:** 🟡
**Evidence:** `src/parser/projection_salt.rs:1-78` parses `json $alias { ... }` into `StatementKind::JsonProjection`; AST node at `src/ast.rs:161-166`; model field `json_projections` at `src/model.rs:534`.
**Notes:** Parsed and stored on the document but visual rendering as a JSON tree node attached to class/object diagrams is not verified — needs visual gate. No object-diagram arrow integration (`Agent -> J`) confirmed.

### 11.12 JSON on Deployment/Usecase/Component (`allowmixing`) — 🟡
**Feature:** `allowmixing` + `json JSON { ... }` mixed with `component`, `actor`, etc.
**Status:** 🟡
**Evidence:** `JsonProjection` exists (`src/ast.rs:161`); `allowmixing` handling not confirmed in this audit. Same projection path as 11.11.
**Notes:** Cross-family rendering depth unverified.

### 11.13 JSON on State diagram — 🟡
**Feature:** `json J { ... }` inside a state diagram.
**Status:** 🟡
**Evidence:** Same projection mechanism; state-diagram integration unverified.

### 11.14 Creole on JSON values — ❌
**Feature:** Render Creole markup (`**bold**`, `<color:...>`, `<img:...>`, `<U+...>`, `<&icon>`) inside JSON string values.
**Status:** ❌
**Evidence:** `src/render/data.rs:87-92` writes the label via `escape_text` and a plain `<text>` element — no Creole inline-run pipeline. No `<tspan>` segmentation, no color/font emission per inline tag.
**Notes:** Creole tags appear in the SVG as literal text.

---

**Tally ch11 (14 subsections audited):** ✅ 7 · 🟡 4 · ❌ 3
