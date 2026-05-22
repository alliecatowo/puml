# Chapter 4: Object Diagram — Audit

Source spec: `/tmp/puml-spec/ch04-object-diagram.txt` (lines 1–305).
Audited against repo at `/Users/allison.coleman/Develop/puml` (Wave-21+).

---

### 4.1 Definition of objects — ✅
**Feature:** Declare object instances with the `object` keyword and optional alias.
**Syntax example:** `object firstObject`, `object "My Second Object" as o2`
**Status:** ✅ Supported
**Evidence:** `src/parser/family.rs:65` (keyword table `("object", None)`), `:736` (`StatementKind::ObjectDecl`). Normalized to `FamilyNodeKind::Object` at `src/normalize/family.rs:7`. Rendered at `src/render/family.rs:2146`.
**Notes:** Quoted labels and alias-via-`as` handled by the shared family decl parser.

### 4.2 Relations between objects (`<|--`, `<|..`, `*--`, `o--`, `-->`, `..>`, cardinality, label) — ✅
**Feature:** Inheritance/realization/composition/aggregation/dependency arrows; dotted variants; `"N"` cardinality strings; `: label`.
**Syntax example:** `Object05 o-- "4" Object06`, `Object07 .. Object08 : some labels`
**Status:** ✅ Supported
**Evidence:** Relation arrowhead recognition in `src/render/relation.rs:89-102` (`*` → `arrow-diamond-filled`, `o` → `arrow-diamond-open`). Marker SVG at `src/render/family.rs:1246-1252`. Cardinality/label parsed by `parse_family_relation` (`src/parser/family.rs:598`).
**Notes:** Same edge code path as class/usecase — feature-complete.

### 4.3 Associations objects (`diamond` keyword) — ❌
**Feature:** A `diamond` node acts as an association point that multiple objects connect to (n-ary association).
**Syntax example:** `diamond dia \n o1 --> dia \n o2 --> dia`
**Status:** ❌ Missing
**Evidence:** No `diamond` keyword in the family parser keyword tables (`src/parser/family.rs:65,139,1326-1329,1398-1401`). `FamilyNodeKind` enum (`src/model.rs:613-650`) has no `Diamond` variant. Existing diamond markers are arrowheads only.
**Notes:** Line beginning with `diamond dia` will fail family decl detection and likely fall through to an error or be ignored.

### 4.4 Adding fields (`object : key = value` and `object { key = value }` block) — ✅
**Feature:** Add fields to an object using the `:` field syntax or a brace block.
**Syntax example:** `user : name = "Dummy"` or `object user { name = "Dummy" \n id = 123 }`
**Status:** ✅ Supported
**Evidence:** Member-row parsing at `src/parser/family.rs:716` (`parse_family_member_row`), brace-block body collection at `:1133,1176,1201,1226`. Members carried as `Vec<ClassMember>` through `ObjectDecl` (`src/ast.rs:267` area).
**Notes:** Shared with class members — supports `+`/`-`/`#` visibility despite spec not using them on objects.

### 4.5 Common features with class diagrams — 🟡
**Feature:** Hide attributes/methods (`hide attributes`), notes, packages, skinning.
**Syntax example:** `hide attributes`, `note left of obj : foo`, `package P { ... }`, `skinparam objectBackgroundColor LightBlue`
**Status:** 🟡 Partial
**Evidence:** `hide` options recognized at `src/normalize/family.rs:256` (`hide_options.contains("stereotype")` etc.). Notes per 4.8. Packages per Chapter 2 §2.5. Skin: `objectbackgroundcolor`/`objectbordercolor`/`objectfontcolor`/`objectarrowcolor`/`objectfontsize`/`objectfontname` at `src/theme.rs:1367-1403`.
**Notes:** `hide stereotype` confirmed; specific `hide attributes`/`hide methods` for objects partially mirrors class behavior — verify on a fixture.

### 4.6 Map table or associative array (`map` keyword, `key => value`, `Bar::abc` qualified IDs, links) — 🟡
**Feature:** `map Name { key => value }` renders an associative-array table; entries can be link targets via `Name::key`; links into rows with `*->`.
**Syntax example:** `map CapitalCity { UK => London \n USA => Washington }` then `NewYork --> CapitalCity::USA`
**Status:** 🟡 Partial
**Evidence:** `map` keyword recognised at `src/parser/family.rs:65` (`("map", Some("<<map>>"))`), `:1327,1399`. Parses as an object-like node with `<<map>>` stereotype member. No dedicated `Map` variant in `FamilyNodeKind` (`src/model.rs:613`); no `=>` separator parsing distinct from `=` field assignment found; no `Name::key` qualified-anchor edge routing in `src/render/family.rs` or `src/render/relation.rs`.
**Notes:** Map likely renders as a generic object box with stereotype `<<map>>`. The `=>` row layout (two-column key→value rendering) and `Bar::abc` row-level link anchors are not implemented. Entries collapse to plain member rows.

### 4.7 PERT with map (`title`, `left to right direction`, chains of maps) — 🟡
**Feature:** Use maps to build PERT charts with sequential map-to-map dependencies and labelled edges.
**Syntax example:** `map Kick.Off { } \n map task.1 { Start => End } \n Kick.Off --> task.1 : Label 1`
**Status:** 🟡 Partial
**Evidence:** `title`, `left to right direction`, and edges between named nodes all supported (§2.12 above; `src/normalize/family.rs:952`). But because `map` rendering is degraded (§4.6) and `Name.Part` dotted aliases aren't specifically handled, the resulting layout will not match the PERT-style 2-column row layout.
**Notes:** Edges between map-nodes will route, but rows will not show as `Start | End` cells. Visually degraded PERT.

### 4.8 Display JSON Data (class + object + json mixed) — ❌
**Feature:** Mix `class`, `object`, and `json` blocks in one diagram.
**Syntax example:** `class C \n object O \n json JSON { ... }`
**Status:** ❌ Missing
**Evidence:** No `allowmixing` handling in `src/`. JSON family in `src/normalize/` is separate. Mixing JSON into a class/object diagram is not wired through `src/normalize/family.rs`.
**Notes:** Without `allowmixing` semantics, the `json` block likely confuses family detection or is rendered as a separate diagram.

---

## Tally — Chapter 4
- ✅ Supported: **3** (4.1, 4.2, 4.4)
- 🟡 Partial: **3** (4.5, 4.6, 4.7)
- ❌ Missing: **2** (4.3, 4.8)
