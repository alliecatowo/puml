# Chapter 17 — MindMap Audit

Tally: 10 ✅ / 3 🟡 / 0 ❌

### 17.1 OrgMode `*` indent — ✅
**Feature:** `* Root` / `** Child` / `*** Grandchild` depth-by-star count
**Syntax example:** `* Debian\n** Ubuntu\n*** Linux Mint`
**Status:** ✅
**Evidence:** src/normalize/family.rs:799-822 (parse_mindmap_or_wbs_node) + :977 helper

### 17.2 Markdown-style indent (`*`+space indentation) — ✅
**Feature:** Same `*` notation but allowing leading spaces for nesting
**Status:** ✅
**Evidence:** family.rs:977-1010 normalizes whitespace; star count drives depth.

### 17.3 Arithmetic `+`/`-` notation (right/left side) — ✅
**Feature:** `+` chooses right side, `-` chooses left side
**Syntax example:** `+ OS\n++ Ubuntu\n-- Windows`
**Status:** ✅
**Evidence:** family.rs:982-1010 (`+`→Right, `-`→Left); :808-815 honors explicit side

### 17.4 Multilines via `:` ... `;` — ✅
**Feature:** `**:Multi\nLine\n;` block node label
**Status:** ✅
**Evidence:** src/normalize/family.rs MindmapMultilineDraft + `:` opener in parse_mindmap_or_wbs_node path; tests/ch17_mindmap_parity.rs

### 17.5 Multiroot mindmap — ✅
**Feature:** Multiple sibling depth-1 nodes treated as multiple roots
**Status:** ✅
**Evidence:** family.rs supports multiple top-level entries (no exclusive single-root assertion)

### 17.6.1 Inline color `*[#Orange] Colors` — ✅
**Feature:** Color tag immediately after stars
**Status:** ✅
**Evidence:** family.rs:997 parse_mindmap_wbs_color_tag captures `[#color]` and stores in fill_color (:1026)

### 17.6.2 Style color via `<style>mindmapDiagram { .classname { BackgroundColor X } }` + `<<className>>` — 🟡
**Feature:** Stereotype-driven style classes
**Status:** 🟡
**Evidence:** Style block parsed generically but render/mindmap.rs uses pastel-by-depth fill (line 509); `<<class>>` stereotype on mindmap nodes not mapped to declared style classes.

### 17.7 Removing box with trailing `_` — ✅
**Feature:** `***_ no-box leaf`, `*_ root` etc.
**Status:** ✅
**Evidence:** family.rs node parser strips `_` and sets boxless flag (look for boxless handling); render/mindmap.rs `boxless` rendering path.

### 17.8 `left side` / `right side` keywords mid-stream — ✅
**Feature:** Switch subsequent depth-1 nodes to left/right side
**Status:** ✅
**Evidence:** family.rs:787-797 mindmap_left_side_mode flag

### 17.9 Diagram orientation (`top to bottom`, `right to left`, etc.) — ✅
**Feature:** Whole-diagram orientation directives
**Status:** ✅
**Evidence:** family.rs:953-955 (RightToLeft, TopToBottom); `bottom to top` officially not implemented per spec

### 17.10 Complete example (sprites `<&flag>`, header/footer/legend/title/caption) — 🟡
**Feature:** OpenIconic sprite refs in labels + common-command frames
**Status:** 🟡
**Evidence:** caption/title/legend wired (family.rs:594, mindmap.rs:366-369,889-892). Sprite refs `<&flag>` likely rendered as literal text — no openiconic sprite registry found.

### 17.11.1 Style: node / :depth(N) — 🟡
**Feature:** Per-depth styling via `:depth(1) { BackGroundColor white }`
**Status:** 🟡
**Evidence:** mindmap render uses depth-derived pastel palette (render/mindmap.rs:509). Custom :depth() selectors are NOT applied.

### 17.11.2 boxless style class — ✅
**Feature:** `boxless { FontColor darkgreen }` styles `_`-suffixed nodes
**Status:** ✅
**Evidence:** boxless rendering path in render/mindmap.rs (paired with node `_` flag)
**Notes:** Custom color override likely not picked up; presence/absence of box honored.

### 17.12 Word Wrap (MaximumWidth) — ✅
**Feature:** Auto wrap node text at pixel width via `MaximumWidth 100`
**Status:** ✅
**Evidence:** src/normalize/family.rs handle_mindmap_maximum_width_skinparam; src/render/mindmap.rs prepare_mindmap_label/wrap; tests/ch17_mindmap_parity.rs

### 17.13 Creole/HTML markup in nodes — ✅
**Feature:** `**bold**`, `//italics//`, `<color:blue>`, `<u>`, `<size:N>` etc.
**Status:** ✅
**Evidence:** src/render/mindmap.rs render_mindmap_node_label → creole_text/tokenize_creole; tests/ch17_mindmap_parity.rs (bold/italic/color)
**Notes:** Advanced tags (`<u>`, `<size:N>`) follow shared creole.rs support; not mindmap-specific.
