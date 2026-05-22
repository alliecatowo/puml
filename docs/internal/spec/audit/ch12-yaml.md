# Chapter 12 — Display YAML Data

Audit of PlantUML YAML data syntax against the puml Rust renderer.
Source: `/tmp/puml-spec/ch12-display-yaml-data.txt`.

### 12.0 @startyaml / @endyaml block — ✅
**Feature:** Block delimiter keywords for YAML-mode diagrams.
**Syntax example:** `@startyaml fruit: Apple @endyaml`
**Status:** ✅
**Evidence:** `src/parser/blocks.rs:50` (`@startyaml` → `BlockKind::Yaml`), `src/normalize/structured.rs:87` `normalize_yaml_document`.

### 12.1 Complex example (nested maps + sequences + scalars) — ✅
**Feature:** Nested mappings, sequences, scalars of all types.
**Syntax example:** `xmas-fifth-day:\n  partridges:\n    count: 1`
**Status:** ✅
**Evidence:** `src/normalize/structured.rs:87-147` uses `yaml_rust2::YamlLoader`, handling `Hash`, `Array`, scalars.
**Notes:** Indent fallback via `flatten_yaml_by_indent` at `src/normalize/structured.rs:195-215` if YAML parse fails.

### 12.2 Specific keys (symbols / unicode: `@fruit`, `$size`, `&color`, `‰`) — ✅
**Feature:** Non-alphanumeric / unicode mapping keys.
**Status:** ✅
**Evidence:** `src/normalize/structured.rs:175-180` `yaml_key_label` preserves string keys verbatim.
**Notes:** yaml_rust2 accepts these as scalar strings.

### 12.3 Highlight parts (`#highlight "key"`) — ✅
**Feature:** Highlight a YAML path.
**Syntax example:** `#highlight "xmas-fifth-day" / "partridges"`
**Status:** ✅
**Evidence:** Structured normalization strips `#highlight` controls before YAML parsing (`src/normalize/structured.rs:112-133`), render-time controls parse highlight paths (`src/render/data.rs:338-436`), and YAML rows carry deterministic path metadata through `flatten_yaml_render_value` (`src/render/data.rs:620-686`). Tests cover nested YAML highlight paths and root-sequence index highlights (`tests/json_yaml_projection_depth.rs:102-123`, `tests/json_yaml_projection_depth.rs:151-177`).

### 12.3.2 / 12.4 Custom highlight styles (`<style> yamlDiagram { highlight { ... } }`, `<<h1>>` stereotypes) — ✅
**Feature:** Per-highlight stereotypes with `<style>` blocks.
**Status:** ✅
**Evidence:** `parse_highlight` extracts `<<class>>` names (`src/render/data.rs:380-435`), and style parsing supports `yamlDiagram { highlight { ... } }` plus `.class` patches (`src/render/data.rs:438-534`). Fixture coverage includes `<<h2>>` and `.h2` style overrides (`tests/fixtures/structured/valid_yaml_highlight_projection.puml:1-25`), with assertions for `data-yaml-highlight-class`, green fill, and italic text (`tests/json_yaml_projection_depth.rs:102-123`).

### 12.5 Using (global) style (`yamlDiagram { node { ... } arrow { ... } }`) — 🟡
**Feature:** Global YAML diagram skinning via `<style>`.
**Status:** 🟡
**Evidence:** YAML `<style>` blocks are stripped from parse payloads (`src/normalize/structured.rs:112-133`) and render-time style parsing supports highlight defaults and class overrides (`src/render/data.rs:338-534`). Node and connector styles still come from hard-coded defaults (`RowStyle::yaml_node` at `src/render/data.rs:74-82`; connector colors at `src/render/data.rs:26-37`), so `node { ... }` / `arrow { ... }` remain partial.
**Notes:** Highlight styling is wired; global node/arrow styling is not.

### 12.6 Creole on YAML values — 🟡
**Feature:** Render Creole and HTML Creole inside YAML scalar values.
**Status:** 🟡
**Evidence:** Structured row labels now render through `creole_text` (`src/render/data.rs:307-320`), using the shared inline Creole text path (`src/render/svg.rs:1-30`). YAML highlight tests assert bold scalar content emits styled tspans (`tests/json_yaml_projection_depth.rs:102-123`).
**Notes:** This covers the existing inline Creole pipeline, not every PlantUML text extension.

### 12.x YAML inside class/object diagram (`yaml Y { ... }`) — ✅
**Feature:** Inline YAML block inside `@startuml`.
**Syntax example:** `yaml Y { key: value }`
**Status:** ✅
**Evidence:** `src/parser/projection_salt.rs:13` recognizes `yaml ` prefix and emits `StatementKind::YamlProjection` (`src/ast.rs:167-172`). Family rendering extracts parser-backed YAML rows via `parse_projection_yaml_value` / `collect_projection_yaml_rows` (`src/render/family.rs:1377-1418`), and tests verify nested sequence projection content renders in a family diagram (`tests/yaml_parser_projection.rs:27-50`).
**Notes:** This remains a puml-supported projection extension beyond the standalone ch12 examples, but it is now parser-and-render verified.

---

**Tally ch12 (8 entries audited):** ✅ 6 · 🟡 2 · ❌ 0
