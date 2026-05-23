# Chapter 18 — Work Breakdown Structure (WBS) Audit

Tally: 6 ✅ / 4 🟡 / 2 ❌

### 18.1 OrgMode syntax `*`/`**`/`***` — ✅
**Feature:** Hierarchical decomposition by star depth
**Syntax example:** `* Business Process Modelling WBS\n** Launch the project`
**Status:** ✅
**Evidence:** src/normalize/family.rs:799 parse_mindmap_or_wbs_node (shared with mindmap); FamilyNodeKind::Wbs at :803

### 18.2 Direction with `<` and `>` (`***< node`, `****> node`) — 🟡
**Feature:** Explicit per-node left/right placement
**Status:** 🟡
**Evidence:** family.rs:983-987 recognizes `>`/`<` after stars and sets MindMapSide::Right/Left
**Notes:** WBS render orientation (top-down tree) differs from mindmap horizontal; effective placement in render/family.rs WBS path may not honor side flips visually.

### 18.3 Arithmetic notation `+`/`-` (with `++`, `+++`, `++-` mixing) — ✅
**Feature:** `+` right-side, `-` left-side; depth by symbol count
**Status:** ✅
**Evidence:** family.rs:982-1010

### 18.4 Multilines `***:Linux Mint\nOpen Source;` — 🟡
**Feature:** Multi-line node label via `:`...`;`
**Status:** 🟡
**Evidence:** Single-line parser; no `:`...`;` block consumer.

### 18.5 Boxless trailing `_` (mixed and all) — ✅
**Feature:** `***_ Task` no-box marker
**Status:** ✅
**Evidence:** Same _-suffix handling as mindmap (family.rs node parser + render boxless)

### 18.6 Inline color `*[#SkyBlue] node` — ✅
**Feature:** Color tag after depth indicator
**Status:** ✅
**Evidence:** family.rs:997 parse_mindmap_wbs_color_tag stores fill_color

### 18.6 Style color via `<<className>>` + `<style>wbsDiagram { .pink {...} }` — 🟡
**Feature:** Stereotype-mapped class style
**Status:** 🟡
**Evidence:** style block parsed; WBS render uses fixed/derived colors, not declared style classes

### 18.7 Using style `wbsDiagram { :depth(N) {...} arrow { LineColor X } boxless {...} }` — 🟡
**Feature:** Per-depth, arrow, boxless style targeting
**Status:** 🟡
**Evidence:** No targeting logic in render/family.rs for `:depth(N)` selectors on WBS
**Theme note:** Built-in `!theme` presets now seed WBS depth fill/border colors through the shared tree depth style carrier; `docs/examples/wbs/07_theme_vibrant.puml` and `tests/ch18_wbs_parity.rs` cover this narrow preset slice.

### 18.8 Word Wrap MaximumWidth — ❌
**Feature:** Auto-wrap node text by pixel width
**Status:** ❌
**Evidence:** No MaximumWidth handling

### 18.9 Arrows between WBS elements (`t2 -> c1`, with `<<style>>` or `#color`) plus paren-alias `**(b) A topic` — ❌
**Feature:** Cross-tree relations between aliased WBS nodes
**Status:** ❌
**Evidence:** No `(alias)` style parser; no relation arrow ingestion in WBS path. WBS node parser doesn't recognize `as alias` either.
**Notes:** Top-level `t2 -> c1` arrow may be parsed as FamilyRelation but referenced WBS nodes lack alias resolution.

### 18.10 Creole / HTML markup in node labels — 🟡
**Feature:** **bold**, //italic//, <color:blue>, sprites <&flag>, etc.
**Status:** 🟡
**Evidence:** Shared with mindmap; render uses escape_text so most creole inline tags rendered literally.

### Title / caption / legend on WBS — ✅
**Feature:** Common-command frames (caption, title, legend)
**Status:** ✅
**Evidence:** family.rs:594 (Caption), title/header/footer wired via FamilyDocument
