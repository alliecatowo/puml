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
**Notes:** Indent fallback via `flatten_yaml_by_indent` at `:169` if YAML parse fails.

### 12.2 Specific keys (symbols / unicode: `@fruit`, `$size`, `&color`, `‰`) — ✅
**Feature:** Non-alphanumeric / unicode mapping keys.
**Status:** ✅
**Evidence:** `src/normalize/structured.rs:149-154` `yaml_key_label` preserves string keys verbatim.
**Notes:** yaml_rust2 accepts these as scalar strings.

### 12.3 Highlight parts (`#highlight "key"`) — ❌
**Feature:** Highlight a YAML path.
**Syntax example:** `#highlight "xmas-fifth-day" / "partridges"`
**Status:** ❌
**Evidence:** not found — no `highlight` handling in `src/normalize/structured.rs` or `src/render/data.rs`. The `#highlight` lines would either be stripped as comments by the indent fallback (`:180` skips `#`-prefixed lines) or break the YAML parse.
**Notes:** No `YamlHighlight` AST/model node.

### 12.3.2 / 12.4 Custom highlight styles (`<style> yamlDiagram { highlight { ... } }`, `<<h1>>` stereotypes) — ❌
**Feature:** Per-highlight stereotypes with `<style>` blocks.
**Status:** ❌
**Evidence:** not found. No highlight pipeline + no style engine hook for YAML.

### 12.5 Using (global) style (`yamlDiagram { node { ... } arrow { ... } }`) — ❌
**Feature:** Global YAML diagram skinning via `<style>`.
**Status:** ❌
**Evidence:** `src/render/data.rs:102-194` uses hard-coded colors (`#fef9c3`, `#ca8a04`, `#0f172a`); no style-engine lookup.
**Notes:** Same gap as JSON 11.10.

### 12.6 Creole on YAML values — ❌
**Feature:** Render Creole and HTML Creole inside YAML scalar values.
**Status:** ❌
**Evidence:** `src/render/data.rs:183-188` emits values via plain `escape_text` — no inline-run / tspan pipeline. Tags appear as literal text.

### 12.x YAML inside class/object diagram (`yaml Y { ... }`) — 🟡
**Feature:** Inline YAML block inside `@startuml`.
**Syntax example:** `yaml Y { key: value }`
**Status:** 🟡
**Evidence:** `src/parser/projection_salt.rs:13` recognizes `yaml ` prefix; emits `StatementKind::YamlProjection` (`src/ast.rs:167-172`).
**Notes:** Cross-family rendering integration unverified (spec ch12 doesn't show this but the parser supports it).

---

**Tally ch12 (8 subsections audited):** ✅ 3 · 🟡 1 · ❌ 4
