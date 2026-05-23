# Chapter 7 — Component Diagram Audit

Reference: `/tmp/puml-spec/ch07-component-diagram.txt` (773 lines)
Audited against puml main, `src/parser/component*.rs`, `src/ast.rs`, `src/render/family.rs`,
`src/render/relation.rs`, `src/normalize/family.rs`.

Legend: ✅ supported · 🟡 partial · ❌ missing

---

### 7.1 Components — ✅
**Feature:** Bracketed `[Name]`, `component Name`, `component "label" as alias`, multi-line names with `\n`.
**Syntax example:** `component [Last\ncomponent] as Comp4`
**Status:** ✅
**Evidence:** `src/parser/component.rs:4` (keyword table), `:64-93` (label/`as` parsing), `:116-148` (anonymous `[Name]` shorthand). `ComponentNodeKind::Component` at `src/ast.rs:275`.
**Notes:** `\n` literal handling in labels not specifically verified; relies on generic label pass-through.

### 7.1.1 Naming exceptions (`$` tag vs component name) — ✅
**Feature:** `component [$C1]` declares component named `$C1`; `remove $C1` would match the tag, not the component, unless aliased.
**Syntax example:** `component [$C2] $C2` then `remove dollarC2`
**Status:** ✅
**Evidence:** `src/parser/component.rs` preserves `$` inside bracketed component names while collecting trailing `$tag` tokens into component metadata. `src/normalize/family.rs` applies tag metadata only for component/deployment visibility controls. Tests: `tests/ch07_component_parity.rs` (`dollar_named_component_is_not_treated_as_tag_without_tag_marker`, plus tagged hide/remove/restore coverage).
**Notes:** `$`-prefixed bracketed names and trailing `$tag` markers are now intentionally disambiguated.

### 7.2 Interfaces — ✅
**Feature:** `() Name`, `() "label" as alias`, `interface Name`, `interface "label" as alias`.
**Syntax example:** `() "Another interface" as Interf2`
**Status:** ✅
**Evidence:** `src/parser/component.rs:5` (`interface` keyword), `:149-186` (anonymous `()` shorthand), `:204-215` (bare `() Name`). `ComponentNodeKind::Interface` at `src/ast.rs:276`.

### 7.3 Basic example (arrows `-`, `..`, `..>`, `-->`) — ✅
**Feature:** Component family relations using dashed/solid/arrow combinations.
**Syntax example:** `DataAccess - [First Component]`, `[First Component] ..> HTTP : use`
**Status:** ✅
**Evidence:** Family relations parsed in `src/parser/family.rs`; arrow style handling in `src/render/relation.rs`.

### 7.4 Using notes — ✅
**Feature:** `note left/right/top/bottom of`, floating `note as N ... end note`, attached via `..`.
**Syntax example:** `note top of C: A top note`
**Status:** ✅ (general family-note machinery)
**Evidence:** Generic family note parsing/rendering used across class/component diagrams.
**Notes:** Component-specific multimodal verification not run here.

### 7.5 Grouping Components (package/node/folder/frame/cloud/database) — ✅
**Feature:** All six group keywords as scoping blocks with `{ ... }`.
**Syntax example:** `cloud { [Example 1] }`, `database "MySql" { folder ... { ... } }`
**Status:** ✅
**Evidence:** `src/parser/component_groups.rs:8-49` accepts `package`, `node`, `frame`, `cloud`, `rectangle`, `namespace` as scoping blocks. `folder` and `database` as containers go through `parse_component_decl` + `find_family_decl_end` (`:113-152`).
**Notes:** Explicit scoping list in `component_groups.rs` only lists `package|namespace|node|frame|cloud|rectangle` — `folder`/`database` group support relies on the flat-declaration `{` fallthrough; nested `database { folder { ... } }` likely works via the same path but isn't independently visual-verified.

### 7.6 Changing arrows direction (`-->`, `-left->`, `-d->`, `<--`) — ✅
**Feature:** Direction-injected arrows and reverse arrows; `left to right direction` keyword.
**Syntax example:** `[Component] -left-> left`
**Status:** ✅
**Evidence:** `src/normalize/family.rs:952` (`"left to right"` → `FamilyOrientation::LeftToRight`). Arrow direction tokens handled by family relation parser.

### 7.7 UML2 notation (default) — 🟡
**Feature:** Default ball-and-socket / lollipop interface rendering.
**Syntax example:** `() HTTP` rendered as circle interface.
**Status:** 🟡
**Evidence:** Interface kind stored but UML2 vs UML1 styling toggle not found.
**Notes:** Lollipop endpoints rendered via `render_lollipop_endpoint` (`src/render/relation.rs:169`) when relation has `left_lollipop`/`right_lollipop` flags set via `()` prefix on side (`parser/family.rs:662-671`).

### 7.8 UML1 notation (`skinparam componentStyle uml1`) — ✅
**Feature:** Switch to UML1 style with `«component»` stereotype rectangle and badge in top-right corner.
**Syntax example:** `skinparam componentStyle uml1`
**Status:** ✅
**Evidence:** `src/normalize/family.rs` parses `componentStyle` skinparam. `ComponentStyleKind::Uml1` variant stored on `ComponentDocument`. `src/render/family.rs` renders badge/stereotype decoration when `uml1` style is active. Tests: `tests/ch07_component_parity.rs` (`component_style_uml1_sets_badge_attribute`, `component_style_uml1_badges_are_in_top_right`).

### 7.9 Rectangle notation (`skinparam componentStyle rectangle`) — ✅
**Feature:** Suppress component icon, render as bare rectangle with no badge or stereotype label.
**Status:** ✅
**Evidence:** `ComponentStyleKind::Rectangle` variant. Renderer suppresses badge and stereotype decorators in rectangle mode. Tests: `tests/ch07_component_parity.rs` (`component_style_rectangle_sets_attribute`, `component_style_rectangle_no_badges`, `component_style_rectangle_hides_stereotype`).

### 7.10 Long description (bracketed body) — 🟡
**Feature:** `component comp1 [ multi-line description ]`
**Syntax example:** `component comp1 [\nThis component\nhas a long comment\n]`
**Status:** 🟡
**Evidence:** Parser handles `"label"` and `[label]` (single-line) at `src/parser/component.rs:64-77`. Multi-line `[ ... ]` block bodies not handled here.
**Notes:** Multi-line `[...]` body collection likely needs a dedicated path; not present in `parse_component_decl`.

### 7.11 Individual colors — 🟡
**Feature:** `component [Web Server] #Yellow`
**Status:** 🟡
**Evidence:** `split_declaration_inline_fill` is called at `src/parser/component.rs:60` and `append_inline_fill_member` at `:107` — basic `#color` capture present.
**Notes:** Inline style with `#color;line:...;line.dashed;text:...` (covered also by ch8 §8.6) — partial; verify dashed/dotted/thickness propagate.

### 7.12 Sprite in Stereotype (`<<$sprite>>`) — ❌
**Feature:** Apply a defined sprite as a stereotype to a component/rectangle.
**Syntax example:** `rectangle " End to End\nbusiness process" <<$businessProcess>>`
**Status:** ❌
**Evidence:** Sprite definitions (`sprite $foo [16x16/16] { ... }`) not found in parser.

### 7.13 Skinparam (component/interface scoping, `<<stereo>>` overrides) — 🟡
**Feature:** `skinparam component { BackgroundColor<<Apache>> Pink ... }` with stereotype-scoped sub-keys.
**Status:** 🟡
**Evidence:** `src/normalize/family.rs:1382-1395` parses `BackgroundColor`/`BorderColor`/`FontColor`/`ArrowColor`/`InterfaceColor` into a `ComponentStyle` struct.
**Notes:** Stereotype-scoped variants (`<<Apache>>` suffix) not visibly handled.

### 7.14 Specific SkinParameter — componentStyle uml2/rectangle — ✅
See 7.8/7.9. `uml2` is now the explicit default; `uml1` and `rectangle` are also supported.

### 7.15 Hide / Remove unlinked (`hide @unlinked`, `remove @unlinked`) — ✅
**Status:** ✅
**Evidence:** `src/normalize/family.rs` handles `hide @unlinked` and `remove @unlinked` directives, filtering out orphan nodes (those with no edges) from the component graph before rendering. Tests: `tests/ch07_component_parity.rs` (`hide_unlinked_removes_orphan_nodes`, `remove_unlinked_removes_orphan_nodes`, `hide_unlinked_keeps_all_when_all_linked`, `hide_unlinked_does_not_affect_sequence_diagrams`).

### 7.16 Hide/Remove/Restore tagged (`$tag`, `*`) — ✅
**Status:** ✅
**Evidence:** `src/parser/component.rs` records trailing component `$tag` tokens as non-rendered metadata; `src/parser/family.rs` parses `hide`/`remove`/`restore` controls; `src/normalize/family.rs` filters tagged component/deployment nodes and incident relations, with `restore $tag` after `hide *` support. Tests: `tests/ch07_component_parity.rs` (`hide_component_tag_removes_tagged_nodes_and_edges`, `remove_component_tag_removes_all_matching_nodes`, `restore_component_tag_after_hide_all_keeps_tagged_nodes`, `component_tags_do_not_render_as_member_text`).

### 7.17 Display JSON Data (`allowmixing` + `json`) — ❌
**Status:** ❌
**Evidence:** `grep allowmixing` returns no hits; no `json` block parser found alongside component diagrams.

### 7.18 Port / PortIn / PortOut — 🟡
**Feature:** `port p1`, `portin p1`, `portout p1` inside a `component C { ... }` block.
**Syntax example:** `component C { portin p1; portout po1; component c1 }`
**Status:** 🟡
**Evidence:** `src/parser/component.rs:6-8` accepts `port|portin|portout`. Distinguishes by inserting `<<portin>>` / `<<portout>>` marker members (`:97-104`). `ComponentNodeKind::Port` at `src/ast.rs:277`. `src/render/family.rs` renders component interfaces as circles and ports as small square nodes with direction metadata. `src/render/graph_layout.rs` now keeps staggered component package groups from overlapping so lollipop/interface edge lanes do not collapse onto package header avoidance lines. Tests: `render::graph_layout::tests::staggered_component_groups_do_not_overlap`.
**Notes:** Standalone ports render as square port nodes, and the component lollipop example no longer overlaps unrelated packages. Full boundary-attached port placement for `port` declared inside a component block remains partial.

---

## Tally — Chapter 7

| Status | Count |
|---|---|
| ✅ Supported | 12 (§7.1, 7.1.1, 7.2, 7.3, 7.4, 7.5, 7.6, 7.8, 7.9, 7.14, 7.15, 7.16) |
| 🟡 Partial | 5 (§7.7, 7.10, 7.11, 7.13, 7.18) |
| ❌ Missing | 2 (§7.12, 7.17) |

**Headline gaps:** sprites in stereotypes; `allowmixing` + JSON; multi-line bracketed component bodies; UML2 interface/port visual fidelity.
