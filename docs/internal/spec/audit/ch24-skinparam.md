# Chapter 24 — Skinparam Command: PUML Renderer Audit

Status legend: ✅ implemented · 🟡 partial · ❌ not implemented

---

### 24.1 Usage (in-diagram + included file) — ✅
**Feature:** `skinparam <key> <value>` accepted anywhere in source (including included files).
**Status:** ✅
**Evidence:** Parsed in `src/parser/sequence.rs:242` (`StatementKind::SkinParam{key,value}`), AST node `src/ast.rs:120`. Included files run through the same parser path.
**Notes:** Config-file CLI flag (`-config foo.cfg`) not present.

### 24.2 Nested form `skinparam family { ... }` — ✅
**Feature:** Block syntax sets multiple params with shared prefix.
**Syntax example:** `skinparam sequence { ArrowColor red\n  ActorBorderColor blue }`
**Status:** ✅
**Evidence:** Parser handles the brace form in `src/parser/sequence.rs:242` block (used heavily — see `classify_sequence_skinparam` cases with `sequenceXxx` aliases at `src/theme.rs:1099`).
**Notes:** Verified by presence of dual key forms (`arrowcolor` | `sequencearrowcolor`) at theme.rs:1106.

### 24.3 `skinparam monochrome true` — ❌
**Feature:** Black-and-white output mode.
**Status:** ❌
**Evidence:** Grep for `monochrome` against theme/normalize returns nothing. No `monochrome` key in any classifier.
**Notes:** Would land in the GenericSkinParamValue family but is not handled.

### 24.4 `skinparam shadowing` (+ stereotype-scoped `shadowing<<...>>`) — 🟡
**Feature:** Disable drop shadows globally or per stereotype.
**Status:** 🟡 — global `shadowing` is parsed (`src/theme.rs:1171,1422`) and stored, but stereotype-scoped `shadowing<<no_shadow>> false` not parsed (no `<<…>>` key suffix support seen in `classify_*_skinparam`).
**Notes:** `pub shadowing: bool` field exists at theme.rs:192, default false (theme.rs:276).

### 24.5 `skinparam monochrome reverse` — ❌
**Feature:** Reverse-colour scheme for dark backgrounds.
**Status:** ❌
**Evidence:** No `monochrome` handler at all.

### 24.6 Colors (named + RGB hex) — ✅
**Feature:** Standard CSS color names + `#RRGGBB` and `transparent` for backgroundColor.
**Status:** ✅
**Evidence:** `parse_color_value` used throughout theme.rs; `transparent` accepted for background.

### 24.7 Font color/name/size (`xxxFontColor`, `xxxFontSize`, `xxxFontName`, `defaultFontName`) — 🟡
**Feature:** Per-element font controls plus global `defaultFont*`.
**Status:** 🟡 — most diagram families have `FontColor` value variants (sequence/class/state/component/activity/timing/chart/generic at theme.rs:1346, 1459, 1527, 1642, 1727, 1810, 1855). `defaultFontName` and `defaultFontSize` supported for sequence (theme.rs:1180, 1190). Generic `fontname` is SupportedNoop (theme.rs:2116). `FontSize` per-element coverage is partial (sequence has ParticipantFontSize via SupportedNoop or unsupported).
**Notes:** Many `*FontName`/`*FontSize` keys land in SupportedNoop or UnsupportedKey paths and emit `W_SKINPARAM_UNSUPPORTED` warnings.

### 24.8 Text Alignment (`sequenceMessageAlign`, `sequenceReferenceAlign`) — 🟡
**Feature:** `left|center|right|direction|reverseDirection`
**Status:** 🟡 — `sequencemessagealign` parses `left|center|right` only (theme.rs:1235); `direction` / `reverseDirection` not handled → `UnsupportedValue`. `sequenceReferenceAlign` not classified.
**Notes:** Mapped to `MessageAlign` enum.

### 24.9 Example skinparam families covered (sequence/actor/usecase/class/interface/component/node/database) — 🟡
**Feature:** Per-family skinparams used in spec examples.
**Status:** 🟡
**Evidence:** Classifiers exist for: sequence (theme.rs:1099), class (1361), state (1463), component (1531), activity (1646), timing, chart (1813), generic (1855+). **Missing:** dedicated classifiers for `actor`, `usecase`, `interface`, `node`, `database`, `object`, `package`, `rectangle`, `agent`, `cloud`, `queue`, `stack`, `frame`, `folder`, `boundary`, `control`, `entity`, `archimate`, `salt`, `nwdiag`. Many of these fall through to a generic classifier that handles only the most common keys.
**Notes:** Stereotype-scoped `<<Apache>>` syntax (`BackgroundColor<<Apache>> LightCoral`) — no evidence of explicit `<<…>>` stripping/dispatch in classify_*; appears unsupported.

### 24.10 List all params — ❌
**Feature:** `-language` CLI flag, `help skinparams` / `skinparameters` special diagrams.
**Status:** ❌
**Evidence:** No `-language` CLI arg in `src/cli.rs`. No `help`/`skinparameters` keyword in parser. Grep empty.

### Skinparam: `handwritten true` — ❌
**Feature:** Hand-drawn sketch style.
**Status:** ❌
**Evidence:** Grep for `handwritten` returns nothing. Likely accepted-but-ignored or warned via `UnsupportedKey`.

### Skinparam: `backgroundColor` — ✅
**Feature:** Diagram background color.
**Status:** ✅
**Evidence:** Sequence (theme.rs:1199), chart (chart.rs:42), and generic families handle it.

### Skinparam replacement: `<style>` blocks — 🟡
**Feature:** CSS-like style blocks that override skinparam values.
**Status:** 🟡
**Evidence:** `src/parser/core.rs` lowers a narrow style subset to existing `SkinParam` statements. Sequence support covers `sequenceDiagram` plus `participant`, `note`, and `group` selectors. Component support covers `componentDiagram { component { BackgroundColor/BorderColor/FontColor ... } }` with override and SVG evidence in `tests/fixtures/styling/valid_style_block_component.puml` and `tests/ch07_component_parity.rs`. State and activity support covers root `ArrowColor` plus nested state/activity/start/diamond/bar color selectors, with visual examples at `docs/examples/state/13_style_block.puml` and `docs/examples/activity_new/08_style_block.puml`.
**Notes:** This intentionally reuses the skinparam classifiers/renderers for now. Broader selector matching, title/header/footer style properties, stereotype-scoped selectors, and class/timing/deployment renderer families remain open.

### Skinparam: `roundcorner` — ✅
**Feature:** Corner radius on boxes.
**Evidence:** theme.rs:1164 sequence variant.

### Skinparam: stereotype-scoped (`backgroundColor<<Apache>>`) — ❌
**Feature:** Per-stereotype overrides via `<<Tag>>` suffix.
**Status:** ❌
**Evidence:** No `<<` parsing inside `classify_*_skinparam` keys.

---

## Tally

| Feature | Status |
|---|---|
| Basic `skinparam k v` | ✅ |
| Nested block form | ✅ |
| Colors (named/hex/transparent) | ✅ |
| backgroundColor | ✅ |
| roundcorner | ✅ |
| monochrome (true/reverse) | ❌ |
| handwritten | ❌ |
| shadowing global / stereotype-scoped | 🟡 / ❌ |
| FontColor/Name/Size families | 🟡 |
| sequenceMessageAlign | 🟡 (no `direction`) |
| `<<Stereotype>>` scoped overrides | ❌ |
| Family coverage (actor/usecase/node/db/...) | 🟡 |
| `<style>` block replacement syntax | 🟡 |
| `-language` / `help skinparams` | ❌ |

**Score:** 5 ✅ · 5 🟡 · 5 ❌ out of 15. Basic skinparam plumbing is solid; many family-specific keys, broad style-block selectors, stereotype scoping, and the global `monochrome`/`handwritten` flags are missing.
