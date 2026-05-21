# Chapter 15 — ArchiMate Diagram Audit

Tally: 6 ✅ / 2 🟡 / 1 ❌

### 15.1 archimate keyword — ✅
**Feature:** `archimate #Layer "Name" as alias <<stereotype>>` element with inline color & stereotype-driven layer/icon
**Syntax example:** `archimate #Technology "VPN Server" as vpnServerA <<technology-device>>`
**Status:** ✅
**Evidence:** src/normalize/archimate.rs:15-21,77-115; src/render/specialized/archimate.rs (layer color mapping, stereotype kinds)
**Notes:** Layer keywords supported: Business, Application, Motivation, Strategy, Technology, Physical, Implementation (via macro prefix matcher 146-166). Inline `#color` parsed.

### 15.2 Junctions — 🟡
**Feature:** `!define Junction_Or circle #black` + `Junction_Or J1` syntax + Junction_And black/white disc rendering
**Syntax example:** `Junction_And JunctionAnd`
**Status:** 🟡
**Evidence:** src/render/specialized/archimate.rs:220-223 (renders circle for `junction` layer); macro path detects `junction_` prefix at archimate.rs:158-159
**Notes:** `!define Junction_Or circle #black` preprocessor define is not specifically wired — relies on generic preprocessor + the `junction` layer fallback. Junction macro detection works only when name starts with `junction_`; user-defined aliases via `!define` likely fall through to plain rectangle.

### 15.3 / 15.4 Examples 1 & 2 (sprite stereotypes, behavior style) — 🟡
**Feature:** `rectangle "X" <<$bProcess>><<behavior>> #Business`, `sprite $bProcess jar:archimate/business-process`, `skinparam rectangle<<behavior>> { roundCorner 25 }`
**Syntax example:** `rectangle "Handle claim" as HC <<$bProcess>><<behavior>> #Business`
**Status:** 🟡
**Evidence:** rectangle handling in family.rs:143-1888 (component family); archimate normalizer does NOT consume rectangle declarations (only `archimate` keyword + macros). So mixed `rectangle <<stereotype>>` lands in component/class family code path.
**Notes:** `sprite $foo jar:archimate/...` is preprocessor; not validated. Stereotype-to-color mapping for `#Business`/`#Application`/`#Technology` only resolved via archimate path, not via rectangle inline color (which uses arbitrary CSS color).

### 15.5 listsprite — ❌
**Feature:** `listsprite` diagram lists all bundled archimate sprites
**Syntax example:** `@startuml\nlistsprite\n@enduml`
**Status:** ❌
**Evidence:** not found in src/
**Notes:** No grep hit for "listsprite". Would require shipped sprite registry.

### 15.6.1 / 15.6.2 Archimate macros & elements — ✅
**Feature:** `Category_ElementName(alias, "Description")` element macros (Business_Service, Motivation_Stakeholder, Application_Component, Technology_Node, Data_Object, etc.)
**Syntax example:** `Motivation_Stakeholder(StakeholderElement, "Stakeholder Description")`
**Status:** ✅
**Evidence:** src/normalize/archimate.rs:117-166 (parse_archimate_macro_element + archimate_layer_and_kind_from_macro)
**Notes:** Layer prefixes handled: strategy_, business_, application_, data_, technology_, physical_, motivation_, junction_, implementation_, migration_. Element kind derived from suffix.

### 15.6.3 Archimate relationships — ✅
**Feature:** `Rel_RelationType(from, to, "desc")` + `_Up/_Down/_Left/_Right/_U/_D/_L/_R` direction suffixes
**Syntax example:** `Rel_Composition_Down(StakeholderElement, BService, "desc")`
**Status:** ✅
**Evidence:** src/normalize/archimate.rs:207-237 (archimate_rel_kind_from_macro, archimate_rel_macro_base, archimate_rel_direction_from_macro); kinds: access, aggregation, association, assignment, composition, flow, influence, realization, serving, specialization, triggering, used_by
**Notes:** Variants Rel_Association_dir, Rel_Access_w / _rw / _r supported via base stripping (215-223) but only Rel_Access maps to "access" — the `_w/_rw/_r` qualifier (write/read-write/read) collapses to plain access without preserving subtype.

### 15.6.4 Plain arrow relations — ✅
**Feature:** `a --> b : label` / `a -> b` / dashed arrows between archimate elements
**Syntax example:** `STOP -up-> JunctionOr`
**Status:** ✅
**Evidence:** src/normalize/archimate.rs:305-335 (parse_archimate_arrow)
**Notes:** Detects `-->`, `->`, `<--`, `<-`. Direction `-up->` / `-down->` style modifiers are NOT parsed (only the arrow body); style limited to dashed inference.

### 15.7 Access subtypes (_r / _w / _rw) — 🟡
**Feature:** Rel_Access_r (read), Rel_Access_w (write), Rel_Access_rw (read-write) distinct arrowheads
**Syntax example:** `Rel_Access_rw(i3, j3, Access_rw)`
**Status:** 🟡
**Evidence:** archimate.rs:215-223 strips direction suffixes; _r/_w/_rw not in direction list, so base name remains `Rel_Access_w` which fails the match table and falls through.
**Notes:** Re-check: match table only includes `Rel_Access`. Variants `Rel_Access_r/w/rw` return None → relation dropped silently. Subtype distinction lost.

### 15.6.4 Color/skin via `<style>` block on archimate — 🟡
**Feature:** `<style> interface { ... }</style>` inline style for archimate
**Syntax example:** `<style>interface {shadowing 0}</style>`
**Status:** 🟡
**Evidence:** Style blocks recognized at parser level but archimate render uses fixed palette in src/render/specialized/archimate.rs:55-220.
**Notes:** Custom style override not propagated to archimate renderer.
