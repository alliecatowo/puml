# Chapter 20 — Information Engineering (IE) Diagrams Audit

Tally: 1 ✅ / 2 🟡 / 2 ❌

### 20.0 IE diagram = Class diagram with `entity` keyword — ❌
**Feature:** Use `entity Entity01 { ... }` instead of `class` in a class diagram context
**Syntax example:** `entity "User" as e01 {\n  *user_id : number <<generated>>\n  --\n  *name : text\n}`
**Status:** ❌
**Evidence:** Class declaration parser at src/parser/family.rs:1317-1330 (declaration_keywords) does NOT include `entity`. Conversely src/parser/sequence.rs:7 maps `entity` to ParticipantRole::Entity (sequence diagrams only). At src/parser/family.rs:231, `lower.starts_with("entity ")` is in the *is_sequence_keyword* test — so an `entity Entity01 { ... }` declaration is routed to the sequence parser, not the class/IE family.
**Notes:** This is the root blocker for ch20 — IE entity declarations are misclassified as sequence participants and the `{...}` block of attributes is dropped/errored.

### 20.1 IE relation symbols (`|o--`, `||--`, `}o--`, `}|--`) — 🟡
**Feature:** Crow's-foot cardinality markers on class/IE relations
**Syntax example:** `Entity01 }|..|| Entity02`, `Entity05 ||--o{ Entity06`
**Status:** 🟡
**Evidence:** Relation parsing in src/parser/family.rs supports `left_cardinality` / `right_cardinality` string capture (parse_relation_side_annotations at 626-649, 764-861). The crow's-foot ASCII markers (`|o`, `||`, `}o`, `}|`) are part of the *arrow body*, not side annotations like `"1"` / `"0..*"` — so they would need arrow-glyph parsing.
**Notes:** Grep for `||--` / `}|` in main code (excluding mermaid frontend) returned no hits. Arrow shape table in render/family.rs (arrowhead glyphs) does not include crow's-foot endpoints. The Mermaid frontend (src/frontend/mermaid.rs:726-812) translates Mermaid IE-style arrows to puml class-cardinality strings — but this is a *converter*, not native IE rendering. Verdict: IE relation symbols not natively interpreted; will likely be tokenized as unknown arrow body and fall back to `--`.

### 20.2 Entity attribute block syntax (`*` mandatory marker, `--` separator) — 🟡
**Feature:** `* identifying_attribute` (mandatory PK), `--` divides identifying vs other attrs, `optional_attribute` (no `*`)
**Syntax example:** `entity Entity01 {\n  * identifying_attribute\n  --\n  * mandatory_attribute\n  optional_attribute\n}`
**Status:** 🟡
**Evidence:** Class member parsing recognizes `--` / `..` separators and visibility markers `+`, `-`, `#`, `~`. The leading `*` for mandatory is NOT in the visibility table. Plus, blocker from 20.0 means this block is never reached.
**Notes:** Once `entity` is wired to class family, `*` should be added as a visibility modifier mapping to "mandatory".

### 20.3 `<<generated>>` / `<<FK>>` / `<<PK>>` stereotypes — ✅
**Feature:** Stereotype markers on attributes
**Status:** ✅
**Evidence:** Class member parsing keeps free-form `<<...>>` stereotypes intact (used throughout class diagram path)

### 20.3 `skinparam linetype ortho` (workaround for angled crow's feet) — ❌
**Feature:** Force orthogonal edge routing on class/IE diagrams to make crow's-foot endpoints axis-aligned
**Status:** ❌
**Evidence:** No `linetype ortho` skinparam handler found for class/family render; class layout uses default edge routing.
**Notes:** Orthogonal routing exists in puml's layout engine for sequence/state — extending to class/IE relations would address the spec's stated workaround. Independent of the larger crow's-foot rendering gap.
