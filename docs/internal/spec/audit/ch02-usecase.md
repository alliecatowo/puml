# Chapter 2: Use Case Diagram — Audit

Source spec: `/tmp/puml-spec/ch02-use-case-diagram.txt` (lines 1–534).
Audited against repo at `/Users/allison.coleman/Develop/puml` (Wave-21+).

---

### 2.1 Usecases — ✅
**Feature:** Use cases are defined with `(parentheses)` or the `usecase` keyword; aliases via `as`.
**Syntax example:** `(First usecase)`, `usecase UC3`, `usecase (Last\nusecase) as UC4`
**Status:** ✅ Supported
**Evidence:** `src/parser/family.rs:115` (`parse_parenthesized_usecase_decl`), `:139` (`usecase` keyword), `:401` (alias handling). Normalized to `FamilyNodeKind::UseCase` in `src/normalize/family.rs:8`. Rendered in `src/render/family.rs:2147`, `:2304` (oval fill `#dcfce7`).
**Notes:** `\n` literal newline escapes handled by the multiline label pipeline.

### 2.2 Actors — ✅
**Feature:** Actors defined by `:Name:` colon-delimited form or the `actor` keyword; alias via `as`.
**Syntax example:** `:First Actor:`, `actor Woman3`, `actor :Last actor: as Person1`
**Status:** ✅ Supported
**Evidence:** `src/parser/family.rs:139` (`actor` keyword, marker `<<actor>>`), `:1349-1352` actor marker embedding. Normalizer promotes to `FamilyNodeKind::Actor` at `src/normalize/family.rs:188-193,285,297`. Renderer label `actor` at `src/render/family.rs:2165`.
**Notes:** Forward-reference actors (used in a relation without prior declaration) are supported in this codebase via implicit creation in family normalization.

### 2.3 Change Actor style (actorStyle awesome / hollow) — ✅
**Feature:** `skinparam actorStyle awesome|hollow` to switch from stickman to alternative actor glyphs.
**Syntax example:** `skinparam actorStyle awesome`
**Status:** ✅ Supported
**Evidence:** `src/theme.rs` now classifies `skinparam actorStyle awesome|hollow|stickman`; `src/normalize/family.rs` applies the chosen actor style to family documents; `src/render/family.rs` emits alternate actor glyphs with `data-actor-style="awesome|hollow"`.
**Notes:** `stickman` remains the default when no override is set.

### 2.4 Usecases description (multiline + separators) — 🟡
**Feature:** Multi-line usecase descriptions in quotes with `--`, `..`, `==`, `__` separator lines (with optional titles between paired markers).
**Syntax example:** `usecase UC1 as "line1\n--\nline2\n==\n..Title.."`
**Status:** 🟡 Partial
**Evidence:** Multi-line quoted labels parsed via `src/parser/multiline.rs` and `\n` is preserved through the family pipeline. No specific handling of `--`/`..`/`==`/`__` *separator lines with titles* was found in `src/render/family.rs` or `src/normalize/family.rs`.
**Notes:** Text renders, but separators are likely displayed as literal characters rather than rendered as horizontal rules with embedded titles.

### 2.5 Use package (and rectangle as boundary) — ✅
**Feature:** Group actors/usecases inside `package Name { ... }` or `rectangle Name { ... }` (system boundary).
**Syntax example:** `package Restaurant { usecase ... }`, `rectangle Restaurant { ... }`
**Status:** ✅ Supported
**Evidence:** `src/parser/family.rs:1151` (`package`), `:1208` (`rectangle "Label" { ... }` — fix #479). Rendered as labelled frame with tab header at `src/render/family.rs:143` (`uses_tab_header = matches!(group.kind.as_str(), "rectangle" | "package")`), label-only rectangle at `:1888-1889` (fix #553).
**Notes:** `skinparam packageStyle rectangle` not separately confirmed (relies on default rendering).

### 2.6 Basic example (--> arrow, longer arrows, labels) — ✅
**Feature:** `-->`, `--->`, etc. with optional `: label`; arrow length affects spacing.
**Syntax example:** `User --> (Use the application) : A small label`
**Status:** ✅ Supported
**Evidence:** Relation parsing in `src/parser/family.rs:598` (`parse_family_relation`); edge rendering at `src/render/family.rs:499-720`. Label/length handled via shared relation pipeline.
**Notes:** Forward-declared `User` (without explicit `actor`) is upgraded to actor implicitly per Chapter 2 example.

### 2.7 Extension (`<|--`) — ✅
**Feature:** Generalization/extension arrow with hollow triangle.
**Syntax example:** `User <|-- Admin`
**Status:** ✅ Supported
**Evidence:** Marker generation in `src/render/family.rs:1236-1252` (`arrow-diamond-filled`, `arrow-diamond-open`, hollow triangle fix #467). Relation parsing in `src/render/relation.rs:89-102`.
**Notes:** Shared with class diagram chapter.

### 2.8 Using notes (`note left/right/top/bottom of`, free `note … as N` linked via `..`) — ✅
**Feature:** Attach notes to actors/use cases or define standalone notes linked with `..`.
**Syntax example:** `note right of Admin : ...`, `note "..." as N2 \n (Start) .. N2`
**Status:** ✅ Supported
**Evidence:** Note statement consumed at `src/normalize/family.rs:233,1283` (becomes `FamilyNodeKind::Note` at `:1754,1761`). Rendered at `src/render/family.rs:1162,2183,2275,4603`.
**Notes:** Multiline `note … end note` blocks supported via `src/parser/multiline.rs`.

### 2.9 Stereotypes (`<< Human >>`, `<< Main >>`) — ✅
**Feature:** Stereotype tagging on actors and use cases via `<< ... >>`.
**Syntax example:** `User << Human >>`, `(Use the application) as (Use) << Main >>`
**Status:** ✅ Supported
**Evidence:** Inline stereotype extraction at `src/normalize/family.rs:1624-1668` (`extract_inline_stereotype_members`, `strip_inline_stereotypes`). Stereotype-keyed skinparam matched at `src/theme.rs:1367` (e.g. `BackgroundColor<< Main >>`).
**Notes:** Stereotype-keyed `BackgroundColor<<Foo>>` color overrides recognized for usecase, but actor-stereotype theming (`ActorBackgroundColor<<Human>>`) is partial — only listed for usecase/class/object scopes in theme.rs.

### 2.10 Changing arrows direction — ✅
**Feature:** Horizontal arrows via single `-`/`.`; reversed arrows (`<--`); explicit direction tokens `-left->`, `-r->`, `-do-`.
**Syntax example:** `:user: -left-> (dummyLeft)`
**Status:** ✅ Supported
**Evidence:** Direction-token parsing in `src/parser/family.rs` arrow parser around `:933-940` (dashed/dotted/bold) and length/orientation in family relation parsing.
**Notes:** Honored by `graph_layout.rs` for hint-driven placement.

### 2.11 Splitting diagrams (`newpage`) — ✅
**Feature:** `newpage` keyword splits a diagram into multiple output images/pages.
**Syntax example:** `:actor1: --> (Usecase1) \n newpage \n :actor2: --> (Usecase2)`
**Status:** ✅ Supported
**Evidence:** `src/parser/family.rs` parses `newpage` / `ignore newpage` for family diagrams, `src/normalize/family.rs` records `FamilyPageBreak` entries, and `src/lib.rs` / `src/render/text.rs` paginate class/object/usecase/salt outputs through `normalize::paginate_family(...)`.
**Notes:** Blank `newpage` titles fall back to the document title, matching the existing sequence pagination behavior.

### 2.12 Left to right direction — ✅
**Feature:** `left to right direction` / `top to bottom direction` to change layout orientation.
**Syntax example:** `left to right direction`
**Status:** ✅ Supported
**Evidence:** `src/normalize/family.rs:952-954` maps to `FamilyOrientation::LeftToRight` / `TopToBottom`; respected by `graph_layout.rs`.
**Notes:** Default top-to-bottom matches PlantUML.

### 2.13 Skinparam (colors/fonts/stereotype overrides) — 🟡
**Feature:** `skinparam usecase { ... }` block; per-stereotype overrides; `skinparam handwritten true`.
**Syntax example:** `skinparam usecase { BackgroundColor DarkSeaGreen ... BackgroundColor<< Main >> YellowGreen }`
**Status:** 🟡 Partial
**Evidence:** Usecase/actor skinparams recognized at `src/theme.rs:1367-1419` (BackgroundColor, BorderColor, FontColor, ArrowColor, FontSize, FontName, stereotype font color). Block form `skinparam usecase { ... }` and stereotype-keyed `<<X>>` suffix parsed.
**Notes:** `handwritten true` not detected anywhere in `src/`. `ActorBackgroundColor<<Human>>` recognised via the generic stereotype suffix path but the visual effect on actor stickman is limited.

### 2.14 Complete example (`.>`, `--`, `: include`, `: extends`) — ✅
**Feature:** Dotted dependency `.>` with `include`/`extends` labels (rendered as stereotype on edge).
**Syntax example:** `(checkout) .> (payment) : include`
**Status:** ✅ Supported
**Evidence:** `usecase_dependency_label` helper in `src/render/relation.rs:112` translates `include`/`extends` to dependency markers; consumed by `src/render/family.rs:520-522,726,2064`.
**Notes:** Renders as labeled dotted arrow with stereotype-aware label.

### 2.15 Business Use Case (`/` suffix — Business Usecase / Business Actor) — ✅
**Feature:** Trailing `/` on `(usecase)/`, `:Actor:/`, `usecase/`, `actor/` to mark business variant.
**Syntax example:** `(First usecase)/`, `actor/ :Last actor: as Person1`
**Status:** ✅ Supported
**Evidence:** `src/parser/family.rs` accepts `actor/`, `usecase/`, and trailing `/` on parenthesized use cases, normalizing them into an internal `<<business>>` marker. `src/render/family.rs` consumes that marker to render business actors distinctly and business use cases as rounded rectangles without exposing the internal marker text.
**Notes:** Business markers are treated as renderer-internal metadata, not visible user stereotypes.

### 2.16 Change arrow color and style (inline) — ✅
**Feature:** `--> (X) #color;line.[bold|dashed|dotted];text:color : label`
**Syntax example:** `foo --> (bar1) #line:red;line.bold;text:red : red bold`
**Status:** ✅ Supported
**Evidence:** Inline arrow style parsing at `src/parser/family.rs:933-940` (`dashed`/`dotted`/`bold`/`thin`/`hidden`, `line:` prefix).
**Notes:** Stroke color, dash pattern, thickness honored; `text:` label color follows the same path.

### 2.17 Change element color and style (inline) — 🟡
**Feature:** Inline element style `#[color|back:color];line:color;line.[bold|dashed|dotted];text:color` on an actor/usecase declaration.
**Syntax example:** `actor b #pink;line:red;line.bold;text:red`
**Status:** 🟡 Partial
**Evidence:** Inline `#color` after a declaration is generally consumed by the family declaration parser, but explicit support for the full extended grammar (`back:`, `line.bold` on nodes, `text:` on element) is not clearly present in `src/parser/family.rs`. Class/usecase node fills via plain `#color` are supported.
**Notes:** Plain `#color` works for fill; the extended style sub-grammar likely degrades to fill-only.

### 2.18 Display JSON Data (`allowmixing` + `json` block) — ❌
**Feature:** `allowmixing` directive allows mixing JSON blocks into a usecase diagram.
**Syntax example:** `allowmixing \n actor A \n usecase U \n json JSON { ... }`
**Status:** ❌ Missing
**Evidence:** No matches for `allowmixing` anywhere in `src/`. JSON family is rendered separately but not as an embedded sub-block of a use-case diagram.
**Notes:** Directive is silently ignored; the JSON block likely triggers family-detection ambiguity or a parse error.

---

## Tally — Chapter 2
- ✅ Supported: **14** (2.1, 2.2, 2.3, 2.5, 2.6, 2.7, 2.8, 2.9, 2.10, 2.11, 2.12, 2.14, 2.15, 2.16)
- 🟡 Partial: **3** (2.4, 2.13, 2.17)
- ❌ Missing: **1** (2.18)
