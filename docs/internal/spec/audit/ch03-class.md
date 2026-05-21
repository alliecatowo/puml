# Chapter 3 — Class Diagram audit

Spec reference: `/tmp/puml-spec/ch03-class-diagram.txt` (PlantUML Language Reference Guide 1.2025.0, source lines 2413..4355).

Repo: `/Users/allison.coleman/Develop/puml`. Key files:
- Parser: `src/parser/family.rs`, `src/parser/core.rs`, `src/parser/sequence.rs` (shared note-head + skinparam handling)
- AST: `src/ast.rs`
- Normalize: `src/normalize/family.rs`
- Theme/skinparam classifier: `src/theme.rs` (`classify_class_skinparam`)
- Render: `src/render/family.rs`, `src/render/relation.rs`

Legend: ✅ supported, 🟡 partial / quirky, ❌ not implemented.

---

### 3.1 Declaring element — 🟡
**Feature:** keywords `abstract`, `abstract class`, `annotation`, `circle`, `()`, `class`, `class <<stereo>>`, `diamond`, `<>`, `entity`, `enum`, `exception`, `interface`, `metaclass`, `protocol`, `stereotype`, `struct`.
**Syntax example:** `interface Foo`
**Status:** 🟡
**Evidence:** `src/parser/family.rs:7-14` keyword table covers `interface, enum, annotation, protocol, struct, abstract, class`. `entity ` accepted at `src/parser/family.rs:231` (treated generically). `object` handled separately at line 65.
**Notes:** Missing keywords: `circle`, `()`, `diamond`, `<>`, `entity`-as-distinct-shape, `exception`, `metaclass`, `stereotype` (as a kind). `entity` is recognized but rendered as a plain class.

### 3.2 Relations between classes — 🟡
**Feature:** Extension `<|--`, implementation `<|..`, composition `*--`, aggregation `o--`, dependency `-->`, weak dep `..>`. Dotted variants via `..` swap. Extra exotic forms `#--`, `x--`, `}--`, `+--`, `^--`.
**Syntax example:** `Class01 <|-- Class02`
**Status:** 🟡
**Evidence:** Arrow marker resolution in `src/render/relation.rs:74-110` recognises `<`, `<|`, `*`, `o`, `>`, `|>`. Dashed via `..` at line 76. Parser arrow detection: `src/parser/family.rs:905` `normalize_family_arrow_token`.
**Notes:** Exotic arrowheads `#--`, `x--`, `}--`, `+--`, `^--` (chapter 3.2 last example) are not modelled — `arrow_style()` only branches on `< * o > |>`; other heads fall through and render as plain lines.

### 3.3 Label on relations + directional `<`/`>` in label — 🟡
**Feature:** `:` label, double-quoted cardinality on each side, `<`/`>` direction marker inside label.
**Syntax example:** `Class01 "1" *-- "many" Class02 : contains`
**Status:** 🟡
**Evidence:** Cardinality + label parsing in `src/parser/family.rs` family-relation block (around `parse_family_relation` ~620-660). Labels and cardinalities flow through `ModelFamilyRelation` in `src/model.rs`.
**Notes:** `<` / `>` direction marker inside labels (e.g. `: drives >`) is not specially rendered as a label arrow indicator.

### 3.4 Non-letter names + `as` alias / quotes — ✅
**Feature:** `class "This is my class" as class1`, `class class2 as "It works this way too"`.
**Syntax example:** `class "foo bar" as F`
**Status:** ✅
**Evidence:** `src/parser/family.rs:287` `let (name_raw, alias_raw) = ... split_once(" as ")`.

### 3.4.1 Names starting with `$` (tags) — ❌
**Feature:** `$tag` interpretation; `remove $tag` semantics.
**Syntax example:** `class $C1` → `remove $C1`
**Status:** ❌
**Evidence:** No `$tag`, no `remove`/`restore`. Searched parser+normalize: nothing matches.
**Notes:** Names with `$` will be parsed as identifiers literally; no tag system.

### 3.5 Adding methods — ✅
**Feature:** Members declared via `Class : member`, or grouped in `{ ... }`; `{field}` / `{method}` modifiers.
**Syntax example:** `class Dummy { String data \n void m() }`
**Status:** ✅
**Evidence:** `src/parser/family.rs:581-584` recognises `field|method|abstract|static|class` member modifiers; member parsing in `src/parser/family.rs:368-460` (class body block).

### 3.6 Visibility for methods/fields (`-`, `#`, `~`, `+`) — ✅
**Feature:** Prefix characters set visibility; rendered as icons.
**Syntax example:** `+field2` / `-field1`
**Status:** ✅
**Evidence:** `src/render/family.rs:2099` `parse_visibility_member`; `uml_visibility_name` at line 2110; renderer block around line 1949 ("Render members with visibility markers").

### 3.6 `skinparam classAttributeIconSize 0` to disable icons — ❌
**Feature:** Toggle off the +/-/~/# circles.
**Status:** ❌
**Evidence:** Not in `classify_class_skinparam` (`src/theme.rs:1361-1424`). Not handled.

### 3.6 Escape leading `\~`, `\+` etc — ❌
**Feature:** Allow members literally starting with reserved visibility chars via `\`.
**Status:** ❌
**Evidence:** No `\~`/`\-` escape handling in `parse_visibility_member` (`src/render/family.rs:2099`).

### 3.6.2 Visibility prefix on class itself (`-class`, `#class`, `~class`, `+class`) — ❌
**Feature:** Apply visibility to the class declaration.
**Status:** ❌
**Evidence:** Keyword table `src/parser/family.rs:7-14` expects bare keywords; no leading `-#~+` stripping before keyword match.

### 3.7 `{static}` / `{abstract}` / `{classifier}` member modifiers — 🟡
**Feature:** Modifiers at start or end of a member line.
**Syntax example:** `{static} String id`
**Status:** 🟡
**Evidence:** `src/parser/family.rs:550, 565, 583` map `abstract`, `static`, `class` to `MemberModifier`.
**Notes:** `{classifier}` alias not in the match arms; only literal `static`/`class`. End-of-line position support depends on the parsing loop — not separately verified.

### 3.8 Member separators `-- .. == __` + titled separators — 🟡
**Feature:** Reorder/group members; titles like `.. Simple Getter ..`.
**Status:** 🟡
**Evidence:** `src/normalize/family.rs:268` accepts blank/`--`/`..` lines inside bodies. `src/normalize/family.rs:397` handles `---` as a row separator.
**Notes:** Lines are tolerated but no special separator-with-title rendering; appears as plain dividers / blanks.

### 3.9 Stereotypes (`<<...>>`) + notes — ✅
**Feature:** `class Object << general >>`; floating notes via `note "..." as N`; note linked with `..`.
**Status:** ✅
**Evidence:** Stereotype stripping `src/normalize/family.rs:1654-1668` `strip_inline_stereotypes_with_values`; declaration stereotype members at line 1671. Note parsing in `src/parser/sequence.rs:305-335` is shared by class family.

### 3.10 Creole / HTML inline tags in notes (`<b>`, `<color:..>`, `<size:nn>`, `<img:..>`) — 🟡
**Feature:** Rich text inside notes.
**Status:** 🟡
**Evidence:** No dedicated creole renderer in render/family.rs note paths; only newline normalisation (`\n` → `\n`) in `src/normalize/family.rs:1004`.
**Notes:** Tags will likely be rendered literally.

### 3.11 Note on field/method (`note right of A::counter`) — ❌
**Feature:** Member-level note targeting via `Class::member`.
**Status:** ❌
**Evidence:** Grep for `::` in parser/family.rs/normalize/family.rs shows no member-qualified target handling.

### 3.12 Note on links (`note on link`, `note right on link`) — ❌
**Feature:** Attach note to the most recent relation.
**Status:** ❌
**Evidence:** No match for `note on link` / `on link` anywhere in parser, normalize, or render.

### 3.13 Abstract class and interface keywords — ✅
**Feature:** `abstract class X`, `abstract X`, `interface X`, `enum X`, `annotation X` plus italic styling for abstract.
**Status:** ✅
**Evidence:** Keyword table `src/parser/family.rs:7-14`. Italic styling for abstract not explicitly verified but kind is propagated.

### 3.14 Hide attributes / methods / members / circle / stereotype — 🟡
**Feature:** `hide empty members`, `hide methods`, `hide fields`, `hide circle`, `hide stereotype`, `show <class> methods`, etc.
**Status:** 🟡
**Evidence:** `src/normalize/family.rs:256-267` reads `hide_options` for `stereotype`, `circle`, `empty members|methods|fields`. `src/render/family.rs:1748-1749` honours `hide_circle` / `hide_stereotype`.
**Notes:** `hide private members` / `hide protected members` / per-stereotype hide / `show` overrides are not in the code paths surfaced — likely unhandled.

### 3.15 Hide classes — ❌
**Feature:** `hide Foo2` removes a specific class.
**Status:** ❌
**Evidence:** Only set-level options (`empty members`, `circle`, `stereotype`) accumulate in `hide_options`. No per-class hide.

### 3.16 Remove classes — ❌
**Feature:** `remove Foo2` deletes a class.
**Status:** ❌
**Evidence:** No `remove` keyword handling in parser.

### 3.17 Hide/Remove/Restore tagged element (`$tag`, `*`) — ❌
**Feature:** Tag-based show/hide/remove/restore.
**Status:** ❌
**Evidence:** No `$tag` parsing, no `restore`.

### 3.18 Hide/Remove `@unlinked` — ❌
**Feature:** Filter unlinked classes.
**Status:** ❌
**Evidence:** Not found in parser or normalize.

### 3.19 Generics `<T>` / `<? extends X>` — ❌
**Feature:** Render generic angle brackets in class name.
**Status:** ❌
**Evidence:** No generic handling in keyword parsing (`src/parser/family.rs:7-14`); class name parser will not separate the `<...>` segment. `skinparam genericDisplay old` also absent.

### 3.20 Specific Spot `<<(C,#color) Foo>>` — ❌
**Feature:** Custom spot letter + color via stereotype.
**Status:** ❌
**Evidence:** Stereotype extraction strips text verbatim; no `(LETTER, #color)` parsing.

### 3.21 Packages with optional background color, nesting — 🟡
**Feature:** `package "Name" #color { ... }` and nested.
**Status:** 🟡
**Evidence:** `src/parser/family.rs:1174` produces a `package` group node; nested via parser block stack.
**Notes:** `#color` background tail handling not separately verified; nesting is handled.

### 3.22 Package styles (`<<Node>>`, `<<Rectangle>>`, `<<Folder>>`, `<<Frame>>`, `<<Cloud>>`, `<<Database>>`) — ❌
**Feature:** Switch package shape via stereotype.
**Status:** ❌
**Evidence:** No skinparam `packageStyle` matched in `classify_class_skinparam`; no per-package stereotype shape override search hits.

### 3.23 Namespaces (synonymous with packages since 2023.2) — 🟡
**Feature:** `namespace ns { ... }` block.
**Status:** 🟡
**Evidence:** `src/parser/family.rs:1199` builds a `namespace` group; treated similarly to package.

### 3.24 Automatic package creation via `set separator ::` / `none` — ❌
**Feature:** Dotted/`::`-prefixed FQNs auto-create wrapper packages.
**Status:** ❌
**Evidence:** Only `set namespaceSeparator <sep>` is parsed (`src/parser/sequence.rs:296-302`), stored as `namespaceSeparator` key. No FQN splitting / auto-wrapping observed.

### 3.25 Lollipop interface (`bar ()- foo`, `foo -() bar`) — ✅
**Feature:** Lollipop endpoint markers on a relation.
**Status:** ✅
**Evidence:** `src/parser/family.rs:628-658` `strip_lollipop_endpoint`. Renderer `src/render/relation.rs:169` `render_lollipop_endpoint`.

### 3.26 Changing arrow orientation (`-left-`, `-r-`, `-up->`, `-d-`) and `left to right direction` — ✅
**Feature:** Direction modifiers inside arrows; short forms.
**Status:** ✅
**Evidence:** `src/parser/family.rs:961-988` `parse_family_relation_direction` covers `left|right|up|down|l|r|u|d`. `left to right direction` mapped in `src/normalize/family.rs:952`.

### 3.27 Association classes (`(A, B) .. Enrollment`) — ❌
**Feature:** Bind a class to an existing relation between two others.
**Status:** ❌
**Evidence:** No `(name, name)` parenthesised-pair parsing.

### 3.28 Association on same class via `<> diamond` connector — ❌
**Feature:** Reusable n-ary diamond connector.
**Status:** ❌
**Evidence:** No `diamond` keyword; `<>` short form not parsed.

### 3.29 Skinparam — 🟡
**Feature:** `skinparam class { BackgroundColor ... ArrowColor ... BorderColor ... }` and `skinparam stereotypeCBackgroundColor`.
**Status:** 🟡
**Evidence:** `src/theme.rs:1361-1424` classifies `BackgroundColor`, `BorderColor`, `ClassHeaderBackgroundColor`, font colors/sizes, `ArrowColor`. Stereotype-specific noop accepted (`classstereotypefontcolor`).
**Notes:** Many class skinparams accepted as noop only. Bracketed-block form `skinparam class { ... }` requires parser to expand to per-key calls — not verified here.

### 3.30 Skinned stereotypes (`BackgroundColor<<Foo>> Wheat`) — ❌
**Feature:** Per-stereotype skinparam overrides.
**Status:** ❌
**Evidence:** `classify_class_skinparam` keys are pure-keyword matches; no `<<...>>` suffix routing.

### 3.31 Color gradient `#color/color`, `#color|color`, `#color\color`, `#color-color` — ❌
**Feature:** Gradient backgrounds.
**Status:** ❌
**Evidence:** `parse_color_value` (in `src/theme.rs`) returns a single color; no gradient split observed in normalize.

### 3.32 `together { ... }` and `[hidden]` link — ✅
**Feature:** Layout grouping; hidden link as layout helper.
**Status:** ✅
**Evidence:** `together` block at `src/parser/family.rs:1124-1142`. `hidden` bracket token in `src/parser/family.rs:937`.

### 3.33 Splitting large files (`page hpages x vpages`) — ❌
**Feature:** `page 2x2` plus `skinparam pageMargin`, `pageExternalColor`, `pageBorderColor`.
**Status:** ❌
**Evidence:** No `page ` directive parsing in family parser.

### 3.34 `extends` / `implements` keywords (and comma list) — ❌
**Feature:** `class ArrayList extends AbstractList implements List`; `class A extends B, C`.
**Status:** ❌
**Evidence:** Grep for `extends`/`implements` in parser shows nothing relevant; class header parser doesn't consume these.

### 3.35 Bracketed relations: `[bold]`, `[dashed]`, `[dotted]`, `[hidden]`, `[plain]`, `[#color]`, `[thickness=N]`, mixes — ✅
**Feature:** Inline relation style bracket.
**Status:** ✅
**Evidence:** `src/parser/family.rs:916-958` `parse_family_relation_style` handles `dashed|dotted|hidden|bold|thin|plain*` plus `thickness=`, hex/named colors.
**Notes:** `plain` keyword isn't in the match arms — explicit `plain` token may noop.

### 3.36 Inline relation styling `#color;line.[bold|dashed|dotted];text:color` — 🟡
**Feature:** Colon-list style after relation arrow.
**Status:** 🟡
**Evidence:** Inline `line.` / `line:` prefix accepted inside brackets (`src/parser/family.rs:932-934`), but the spec form lives outside brackets (`foo --> bar1 #line:red;line.bold;text:red`). That tail-style after the relation is not specifically parsed in `parse_family_relation`.

### 3.37 Inline class styling `#bg ##[style]border` and `#[back:..];header:..;line:..;line.[..];text:..` — 🟡
**Feature:** Per-class inline color + border styles.
**Status:** 🟡
**Evidence:** Fill color is captured via `\x1fstyle:fill:` sentinel in `src/parser/family.rs:311,1304,1356,1436,1471`. No `##[bold]red` border-line parsing or `;line:`/`;header:`/`;text:` decomposition surfaced.

### 3.38 Arrows from/to class members (`Foo::field1 --> Bar::field3`) — ❌
**Feature:** Member-level relation endpoints.
**Status:** ❌
**Evidence:** No `::` splitting in family relation parser.

### 3.39 `skinparam groupInheritance N` — ❌
**Feature:** Merge inheritance arrowheads when ≥ N children share a parent.
**Status:** ❌
**Evidence:** Not in `classify_class_skinparam` (`src/theme.rs:1361-1424`). No grouping pass.

### 3.40 Display JSON data on class diagram (`json X { ... }`) — ❌ (out of scope here)
**Feature:** Embed a JSON block alongside classes.
**Status:** ❌
**Evidence:** No JSON block keyword in family parser.

### 3.41 Packages and Namespaces enhancement (dotted FQN, `!pragma useIntermediatePackages false`) — ❌
**Feature:** `class A.B.C.D.Z` auto-creates intermediate packages.
**Status:** ❌
**Evidence:** No FQN auto-wrap; no `!pragma useIntermediatePackages` handling.

### 3.42 Qualified associations `class1 [Qualifier] - class2` — ❌
**Feature:** Bracket qualifier on relation source end; works with generic angle types too.
**Status:** ❌
**Evidence:** `[ ]` after class name is not handled as a qualifier; the relation parser's bracket logic operates on the arrow token itself, not the LHS.

### 3.43 Change diagram orientation (`top to bottom direction`, `left to right direction`) — 🟡
**Feature:** Whole-diagram orientation swap.
**Status:** 🟡
**Evidence:** `left to right` → `FamilyOrientation::LeftToRight` at `src/normalize/family.rs:952`.
**Notes:** `top to bottom direction` (the explicit default) presumably ignored / falls through; not explicitly mapped.

### 3.43.3 / 3.43.6 `!pragma layout smetana` — ❌
**Feature:** Switch to internal layout engine.
**Status:** ❌
**Evidence:** No `!pragma layout` handling.

---

## Tally

- ✅ supported: 9 (3.4, 3.5, 3.6 visibility, 3.9, 3.13, 3.25, 3.26, 3.32, 3.35)
- 🟡 partial: 12 (3.1, 3.2, 3.3, 3.7, 3.8, 3.10, 3.14, 3.21, 3.23, 3.29, 3.36, 3.37, 3.43)
- ❌ missing: ~24 — most notably `extends`/`implements` (3.34), member-level notes/arrows (3.11, 3.38), `note on link` (3.12), `remove`/`restore`/`$tag`/`@unlinked` (3.15–3.18), generics (3.19), spot customization (3.20), package styles (3.22), `set separator` FQN expansion (3.24, 3.41), association classes (3.27, 3.28), gradients (3.31), page splitting (3.33), skinned stereotypes (3.30), `groupInheritance` (3.39), JSON block (3.40), qualified associations (3.42), smetana pragma (3.43.3/.6).

Hot files for any class-diagram parity push: `src/parser/family.rs` (keyword table + arrow/bracket parsing), `src/normalize/family.rs` (hide_options, stereotype stripping, orientation), `src/theme.rs` (`classify_class_skinparam`), `src/render/family.rs` (member rendering, visibility icons), `src/render/relation.rs` (arrowhead repertoire).
