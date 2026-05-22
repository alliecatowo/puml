# Chapter 20 — Information Engineering (IE) Diagrams Audit

Tally: 4 ✅ / 1 🟡 / 0 ❌

### 20.0 IE diagram = Class diagram with `entity` keyword — ✅
**Feature:** Use `entity Entity01 { ... }` instead of `class` in a class diagram context
**Syntax example:** `entity "User" as e01 {\n  *user_id : number <<generated>>\n  --\n  *name : text\n}`
**Status:** ✅
**Evidence:** `parse_family_declaration` now has an `entity` branch when the declaration has a block or later IE context (`src/parser/family.rs:65-90`), and class-family detection treats block `entity` declarations as class-family context (`src/parser/family.rs:231-249`, `src/parser/family.rs:1608-1621`). Parser tests assert the IE fixture normalizes as `DiagramKind::Class` with `ClassDecl` entities (`src/parser/tests.rs:1323-1342`).
**Notes:** Sequence `entity` participants still route through `src/parser/sequence.rs`; the IE branch is context-sensitive rather than removing sequence support.

### 20.1 IE relation symbols (`|o--`, `||--`, `}o--`, `}|--`) — ✅
**Feature:** Crow's-foot cardinality markers on class/IE relations
**Syntax example:** `Entity01 }|..|| Entity02`, `Entity05 ||--o{ Entity06`
**Status:** ✅
**Evidence:** IE endpoint detection maps `}o`/`o{`, `}|`/`|{`, `|o`/`o|`, and `||` to dedicated marker ids (`src/render/relation.rs:118-144`), and marker definitions render zero/one/many glyphs (`src/render/relation.rs:203-234`). Parser tests preserve dotted and solid IE arrows in relation metadata (`src/parser/tests.rs:1363-1381`); integration coverage verifies IE marker definitions and preserved `data-uml-arrow` values (`tests/integration.rs:6652-6679`).

### 20.2 Entity attribute block syntax (`*` mandatory marker, `--` separator) — ✅
**Feature:** `* identifying_attribute` (mandatory PK), `--` divides identifying vs other attrs, `optional_attribute` (no `*`)
**Syntax example:** `entity Entity01 {\n  * identifying_attribute\n  --\n  * mandatory_attribute\n  optional_attribute\n}`
**Status:** ✅
**Evidence:** Entity block members are parsed through `parse_family_decl_members` from the `entity` branch (`src/parser/family.rs:65-90`), and render code treats `--` / `..` as compartment dividers (`src/render/family.rs:2542-2551`). Leading `*` is rendered as an IE mandatory marker with `data-uml-ie-mandatory="true"` (`src/render/family.rs:2600-2610`), covered by `tests/integration.rs:6652-6667`.

### 20.3 `<<generated>>` / `<<FK>>` / `<<PK>>` stereotypes — ✅
**Feature:** Stereotype markers on attributes
**Status:** ✅
**Evidence:** Class member parsing keeps free-form `<<...>>` stereotypes intact (used throughout class diagram path)

### 20.3 `skinparam linetype ortho` (workaround for angled crow's feet) — 🟡
**Feature:** Force orthogonal edge routing on class/IE diagrams to make crow's-foot endpoints axis-aligned
**Status:** 🟡
**Evidence:** `linetype` is accepted as a class-family `SupportedNoop` skinparam (`src/theme.rs:1413-1423`), so IE fixtures with `skinparam linetype ortho` validate (`tests/fixtures/families/valid_ie_information_engineering.puml:1-27`). Class/family relation rendering already prefers orthogonal `edge_paths` when available (`src/render/family.rs:573-620`), but the `linetype ortho` value is not a user-controlled switch.
**Notes:** This is no longer a parser/render blocker for IE examples, but it is not full PlantUML `linetype` parity because non-ortho/ortho modes are not selectable.
