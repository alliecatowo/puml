# `<style>` Block Cascade — Design (epic #1404)

**Date:** 2026-05-31
**Author:** Claude Opus 4.7 (scoping pass for epic #1404)
**Parent epic:** [#1404](https://github.com/alliecatowo/puml/issues/1404)
**Audit source:** `docs/internal/forensics/2026-05-31-plantuml-upstream-source-audit.md` §3 Gap A (merged in PR [#1405](https://github.com/alliecatowo/puml/pull/1405))
**Upstream reference:** PlantUML master @ `6c308a2` — `src/main/java/net/sourceforge/plantuml/style/`
**LRG citations:** v1.2025.0 §6.25 (activity), §8.15.3 (deployment), §8.16.1 (stereotype), §9.25 (state), §11.x (sequence), cyborg/reddress-darkblue bundled themes

> This is a **design + decomposition** doc. No implementation lands from this PR.
> The output is one design doc + 5 child issues + an updated epic body.

---

## 0. One-page summary

PlantUML's `<style>` block is the language's only true *cascading* styling system:
selectors compose by descendant nesting (`activityDiagram { partition { ... } }`),
match by stereotype (`.Apache { BackgroundColor … }`), wildcard (`*`), and pseudo
(`:depth(N)`); properties merge by selector specificity into a fully-resolved
per-element `Style`. The 44 bundled PlantUML themes — and most real-world user
themes (AWS/Azure/GCP stacks built on stdlib) — *are* `<style>` blocks. PUML
currently parses `<style>` as a thin wrapper that flattens each rule to a
hard-coded skinparam key for one of 8 family names, drops anything that does not
map, has no descendant nesting, no stereotype selectors, no cascade resolution
order distinct from skinparams, and supports only ~10 of the 30 `PName`
properties.

This design replaces that thin wrapper with a real cascade. We add a
`StyleRule { selector_path: Vec<SelectorSegment>, properties:
BTreeMap<PName, Value> }` AST node, parse `<style>` blocks into a flat
`Vec<StyleRule>` (selector chains "unfold" the nesting tree so each leaf rule is
self-contained), and resolve effective styles via a per-element lookup
(`StyleBuilder::lookup(element_signature) -> EffectiveStyle`) that walks
candidate rules in specificity order. The new tier slots between Stereotype (4)
and Inline (6) in the existing `StyleSource` cascade in
`src/theme/shared_cascade.rs`, so the new resolver layers cleanly on top of the
existing skinparam → stereotype → inline chain without disturbing any
already-correct family. Backward compatibility: the existing
`*_style_skinparam_key` translator becomes a *fallback* — when a property has a
skinparam analog it is also exported as a `SkinParam`-tier value so that
families that have not yet wired the new resolver continue to render
identically.

The work decomposes into 5 child issues that can ship sequentially without
mid-stream rebases on the same files: **(a)** parser + AST (lays the
`StyleRule` foundation, lands behind a feature gate, all renderers still call
the legacy path); **(b)** cascade resolver + integration with skinparam
(introduces `StyleBuilder`, plumbs it through `FamilyStyle`, swaps the
class/component families first); **(c)** stereotype + wildcard selectors
(unlocks reddress-* and the 44 themes that key off `.Apache`-style selectors);
**(d)** 10 missing properties batch 1: `LineThickness`, `LineStyle`, `Padding`,
`Margin`, `RoundCorner`, `FontWeight`, `Shadowing`, `HorizontalAlignment`,
`MaximumWidth`, `MinimumWidth` (the highest-leverage subset); **(e)**
validation harness + diagnostics + docs + bless against `!theme cyborg` and
`!theme reddress-darkblue` fixtures. Each child is sized for a single Sonnet
agent (≤120 min implementation, file-touch list ≤8 files).

---

## 1. Goals and non-goals

### Goals

- Parse the full `<style>` grammar PlantUML accepts: descendant nesting,
  stereotype selectors (`.foo`), wildcard (`*`), pseudo (`:depth(N)`), `@media`
  (regular vs dark scheme), `--css-var` definitions, comma-separated selectors.
- Provide a cascade resolver that returns a fully-merged `EffectiveStyle` for
  any `(SName-chain, stereotypes, depth)` query.
- Cover the 30 `PName` properties (10 already mapped + ~20 missing). Properties
  that the renderer does not yet honor must still be **parsed and stored** so
  later renderer work can consume them.
- Backward-compatible: every existing `<style>` block that PUML currently
  accepts must continue to render byte-identically until the family-renderer
  wiring (Phase B/C) explicitly opts that family into the new resolver.
- `!theme cyborg` and `!theme reddress-darkblue` should render with the correct
  dark backgrounds and accent colors after epic completion (subjective
  spot-check; not pixel-perfect parity).

### Non-goals (deferred)

- **Pixel-perfect upstream parity** for sketchy / sketchy-outline / vibrant
  themes — these depend on hand-drawn stroke filters and gradient compositing
  outside the cascade scope.
- **CSS-var expansion in arbitrary value positions** beyond what
  `CssVariables.java` does (single-variable substitution, no calc/expressions).
- **`@media print` / `@media dark` runtime switching** — store the dark variant
  in `StyleRule` but only emit regular-scheme styles in v1; dark scheme can be
  selected by a follow-up `--scheme dark` CLI flag.
- **Source-mapped style provenance** in the diagnostic output (nice-to-have for
  the language service; can be added in a follow-up after the cascade is live).
- **Replacing `skinparam` parsing.** Skinparams stay as their own statement
  kind and continue to flow through tier 3 of `shared_cascade`. The
  `<style>`-block-only properties (e.g. `MaximumWidth`, `Padding`) get a new
  tier-5 lookup; legacy properties keep working through tier 3.

---

## 2. Upstream behavior (citations)

All file paths below are relative to the PlantUML repo
(`github.com/plantuml/plantuml` master @ `6c308a2`, locally cloned to
`/tmp/plantuml-src` for this audit).

### 2.1 Entry point

`src/main/java/net/sourceforge/plantuml/style/CommandStyleMultilinesCSS.java`
lines 54-93 — captures everything between `<style>` and `</style>`, hands it
to `StyleParser.parse(BlocLines)`. The single-line variant
`CommandStyleSingleLineCSS.java` handles `<style> selector { prop: val; } </style>`.

### 2.2 Parser

`src/main/java/net/sourceforge/plantuml/style/parser/StyleParser.java` (361
LOC) does its own lexer (`StyleTokenType`: `STRING`, `COLON`, `SEMICOLON`,
`NEWLINE`, `STAR`, `OPEN_BRACKET`, `CLOSE_BRACKET`, `COMMA`, `AROBASE_MEDIA`)
followed by a recursive-descent parse that pushes/pops a `Context` stack:

```
selector { ... }            # push selector, then recurse
sel1, sel2 { ... }          # push comma-joined selector chain
foo bar { ... }             # nested = descendant — pushes "bar" onto "foo"
--my-var: #ff0000;          # variable definition (stored in CssVariables)
Property Value              # or `Property: Value`
}                           # pop context and emit Style{signature, properties}
@media dark { ... }         # switches StyleScheme.REGULAR ↔ StyleScheme.DARK
```

Selectors are simple identifiers; the parser does no per-name validation. Each
unknown name becomes a free-form selector segment that `SName.retrieve()`
later tries to map to an `SName` enum (returns `null` if unknown).

### 2.3 Selector grammar

Tokens that may appear in a selector position:

- **Bare identifier** (e.g. `participant`, `node`, `arrow`, `partition`) —
  resolves to `SName` enum (~130 values, ranging from family names like
  `activityDiagram` to elements like `participant` and pseudo-categories like
  `groupHeader` / `lifeLine` / `caption`).
- **Stereotype** prefixed with `.` (e.g. `.Apache`, `.entity`, `.Foo`) — stored
  as a stereotype set on `StyleSignatureBasic`.
- **Wildcard** `*` — appended via `addStar()`, matches any descendant level.
- **Comma-separated** `a, b, c { ... }` — synonym for three sibling blocks.
- **Pseudo** `:depth(N)` — only used by mindmap/wbs depth styling.
- **Nesting** — `a { b { ... } }` is shorthand for `a b { ... }`; nesting depth
  is unbounded.

### 2.4 Property names (`PName`)

`src/main/java/net/sourceforge/plantuml/style/PName.java` enumerates exactly
30 properties:

```
Shadowing, FontName, FontColor, FontSize, FontStyle, FontWeight,
BackGroundColor, RoundCorner, LineThickness, DiagonalCorner,
HyperLinkColor, HyperlinkUnderlineStyle, HyperlinkUnderlineThickness,
HeadColor, LineColor, LineStyle, Padding, Margin, MaximumWidth,
MinimumWidth, ExportedName, Image, HorizontalAlignment, ShowStereotype,
ImagePosition, MarkerShape, MarkerSize, MarkerColor, BarWidth, Width
```

`PName.getFromName(name, scheme)` is case-insensitive — `BackGroundColor`,
`backgroundColor`, `BACKGROUNDCOLOR` all resolve to the same enum variant.

### 2.5 Style + StyleBuilder + StyleSignature

- `Style` (343 LOC) = `(StyleSignatureBasic signature, Map<PName, Value> map)`.
  Key method: `mergeWith(other, MergeStrategy)` does
  property-by-property merge with priority comparison
  (`ValueImpl.getPriority()`).
- `StyleBuilder` (163 LOC) holds a `StyleStorage` (a list of (signature,
  Style) pairs) and exposes `getMergedStyle(StyleSignatureBasic)` which walks
  all storage entries calling `signature.matchAll(query)` and merges matches
  in priority order. Result is memoized in `mergedStyleCache`.
- `StyleSignatureBasic` (311 LOC) = `(StyleKey key, Set<String> stereotypes)`
  where `StyleKey = { EnumSet<SName> snames, int level, boolean isStared }`.
  `matchAll(other)` returns true iff `other` contains all of this's snames
  *and* all of this's stereotypes — i.e. selector is a *subset* of element's
  classification.

### 2.6 Cascade priority

`ValueImpl` carries an `int priority` that increases with selector specificity:

- Counter-based: `AutomaticCounterBasic.getNextInt()` is called when a value
  is parsed, so later rules win over earlier rules for equal-specificity
  cases.
- `DELTA_PRIORITY_FOR_STEREOTYPE = +1000` (from `StyleLoader.java`) bumps
  stereotype-bearing rules above non-stereotype rules.
- `deltaPriority(int)` is applied for starred wildcards (lower priority than
  exact matches).

This is the same merge rule as CSS "specificity": more-specific wins, ties
broken by source order.

---

## 3. PUML current state (citations)

### 3.1 Parser

`src/parser/directives.rs` lines 73-218 has the `parse_style_block`
function. Limitations vs upstream:

1. Hard-codes 8 family-name outer selectors (`sequenceDiagram`,
   `classDiagram`, `usecaseDiagram`, `componentDiagram`, `deploymentDiagram`,
   `stateDiagram`, `activityDiagram`, `saltDiagram`). Any other top-level
   selector (e.g. `root`, `document`, `node`, bare element selectors at
   top-level) is silently dropped (line 250-260, "Preserve unsupported style
   blocks as raw lines").
2. Nesting depth is capped at 2 (`nested_selector: Option<String>` — single
   level). Upstream is unbounded.
3. No stereotype selector parse (`.foo` is just a token that fails to match
   any open-selector branch).
4. No wildcard or pseudo-state.
5. No comma-separated selectors.
6. No `@media` / no CSS variables.
7. Each accepted `key: value` pair is immediately translated to a skinparam
   key via `StyleBlockTarget::skinparam_key()`. Unknown properties are dropped
   (the property/key/value triple is still stored in `StatementKind::StyleParam`
   with `key: None`, but the cascade ignores it).

### 3.2 AST

`src/ast/mod.rs` lines 151-156 defines:

```rust
StyleParam {
    selector: Option<String>,
    property: String,
    key: Option<String>,
    value: String,
},
```

This is a *flat* per-property record, not a rule. It encodes one
`(selector, property, value)` triple. There is no notion of a `StyleRule`
that owns multiple properties or carries a multi-segment selector chain.

### 3.3 Cascade

`src/theme/values.rs` defines `StyleSource`:

```rust
pub enum StyleSource {
    Default, ThemePreset, SkinParam, StyleBlock, Stereotype, Inline,
}
```

`src/theme/shared_cascade.rs` (530 LOC) ships the resolver. Tier order
(lowest → highest): `Default → ThemePreset → SkinParam → Stereotype →
StyleBlock → Inline`. Each color property goes through `resolve_color()`
which picks the highest-tier non-`None` value. The cascade is implemented but
the `style_block` tier is currently only populated by the family-specific
`apply_style_param` paths in `cascade.rs` (line 96), which call back into the
skinparam classifier — so the `style_block` tier is effectively a
re-application of skinparam under a different label.

### 3.4 Family wiring

`src/theme/cascade.rs` lines 96-110 implements `apply_style_param` for the
class/component family by delegating to `apply_style_value`. Sequence,
activity, state, timing, salt, and mindmap each have their own
family-specific `apply_style_param` (the normalize layer dispatches via the
match arms in `src/normalize/family/extended.rs:395`,
`src/normalize/state.rs:226`, `src/normalize/family/tree.rs:231`,
`src/normalize/family/stub.rs:62`, `src/normalize/sequence/state.rs:91`).

---

## 4. Design

### 4.1 AST surface (Phase A)

Replace the flat `StatementKind::StyleParam` triples with a single rich
`StatementKind::StyleBlock(StyleBlock)` carrying a `Vec<StyleRule>`:

```rust
pub struct StyleBlock {
    pub rules: Vec<StyleRule>,
    pub variables: BTreeMap<String, String>,   // --css-var assignments
    pub scheme: StyleScheme,                   // Regular | Dark
    pub span: Span,
}

pub struct StyleRule {
    /// Full unfolded selector chain. `activityDiagram { partition { … } }`
    /// becomes `[Tag("activityDiagram"), Tag("partition")]`.
    pub selector_path: Vec<SelectorSegment>,
    /// `BTreeMap` for deterministic iteration (CLAUDE.md §6 invariant).
    pub properties: BTreeMap<PName, StyleValue>,
    /// Specificity counter assigned at parse time
    /// (later rule wins on tie — matches upstream `AutomaticCounterBasic`).
    pub source_order: u32,
}

pub enum SelectorSegment {
    /// Bare identifier resolved to one of the `SName` enum values
    /// (`participant`, `node`, `arrow`, `activityDiagram`, …).
    Tag(SName),
    /// `.foo` — stereotype filter.
    Stereotype(String),
    /// `*` — wildcard descendant.
    Wildcard,
    /// `:depth(N)` — used by mindmap / wbs.
    Depth(u32),
    /// Free-form unrecognised tag — preserved so unknown family names
    /// don't error out; cascade lookup simply never matches them.
    Unknown(String),
}

pub enum StyleValue {
    Color(String),       // raw, validated by theme::color::parse_color_value
    Number(f64),         // for sizes, thicknesses, etc.
    Length(LengthSpec),  // padding/margin: "4 8 4 8" form (clockwise TRBL)
    Keyword(String),     // FontStyle bold, LineStyle dashed, etc.
    Raw(String),         // anything else; the renderer decides how to read it
}

pub enum StyleScheme { Regular, Dark }
```

`SName` is mirrored as a Rust enum 1:1 from
`/tmp/plantuml-src/.../style/SName.java` (130+ variants). Unknown identifiers
fall back to `SelectorSegment::Unknown(String)`. Comma-separated selectors at
parse time fan out into N rules with identical bodies.

The legacy `StyleParam { selector, property, key, value }` enum variant
remains *for one release cycle* as a deprecated alias so in-flight branches
don't break; emit it alongside `StyleBlock` and remove in a follow-up cleanup
(filed as #1404f task or rolled into Phase E doc work — see decomposition).

### 4.2 Parser (Phase A)

New module `src/parser/style_block.rs` (~400 LOC). Hand-rolled lexer +
recursive-descent parser modeled directly on
`/tmp/plantuml-src/.../style/parser/StyleParser.java`:

1. Tokens: `Word`, `Colon`, `Semicolon`, `Newline`, `Star`, `OpenBrace`,
   `CloseBrace`, `Comma`, `At` (for `@media`).
2. Comments: `// …`, `/' … '/`, `/* … */` all skipped at lex time (matches
   upstream).
3. Strings: bare-word identifiers stop at whitespace, `{`, `}`, `;`, `,`,
   `:`, `\t`; quoted strings consume between `"..."` (only used for values).
4. The parser maintains a `Vec<SelectorSegment>` context stack. Each `{`
   pushes the just-read selector onto the stack; each `}` pops and emits one
   rule per accumulated property set.
5. CSS variables (`--foo: bar`) are stored in a `BTreeMap<String, String>`
   and expanded in subsequent value positions via a single-pass replace
   (matches `CssVariables.java`).
6. `@media dark { … }` flips `StyleScheme::Dark` for the contained block.

Hook into `src/parser/directives.rs` `parse_style_block` — the new module
**replaces** the existing function. The old `StyleBlockTarget` enum and its
8 hardcoded family checks (lines 175-265 of `directives.rs`) are deleted; the
new parser accepts any selector and validates later at lookup time.

### 4.3 Cascade resolver (Phase B)

New module `src/theme/style_builder.rs` (~250 LOC):

```rust
pub struct StyleBuilder {
    rules: Vec<StyleRule>,             // accumulated across diagrams/themes
    cache: BTreeMap<StyleQuery, EffectiveStyle>,
}

pub struct StyleQuery {
    pub tags: Vec<SName>,              // e.g. [activityDiagram, partition]
    pub stereotypes: BTreeSet<String>,
    pub depth: Option<u32>,
}

pub struct EffectiveStyle {
    pub properties: BTreeMap<PName, StyleValue>,
}

impl StyleBuilder {
    pub fn push(&mut self, rule: StyleRule);
    pub fn lookup(&self, query: &StyleQuery) -> EffectiveStyle;
}
```

`lookup` computes specificity for every rule whose selector_path is a
*subsequence* of `query.tags` (with stereotype matching) and merges properties
in ascending specificity order so the highest-specificity rule wins. The
specificity calculation matches upstream:

- `+100` per matched tag segment
- `+1000` per matched stereotype (matches PlantUML's
  `DELTA_PRIORITY_FOR_STEREOTYPE`)
- `+1` for wildcard match (so `*` always loses to a concrete match)
- ties resolved by `source_order` (later rule wins)

Results are memoized in `cache`. Cache key is the full `StyleQuery`; since
the resolver is pure, the cache never needs invalidation within one
`FamilyStyle` lifetime.

### 4.4 Integration with skinparam (Phase B)

The `<style>` tier slots into `shared_cascade::CascadeInput` between
Stereotype (tier 4) and Inline (tier 6) — i.e. takes over the existing
`style_block` tier 5 slot. Concretely:

1. Normalize layer collects every `StatementKind::StyleBlock` and pushes its
   rules into a per-`FamilyStyle` `StyleBuilder`.
2. The legacy `StatementKind::StyleParam` flat triples (still emitted by the
   compat shim) are also pushed: each becomes a one-segment-rule with the
   bare `selector` tag and one property.
3. At render time, every per-element style query (e.g.
   `family_node_inline_style`) builds a `StyleQuery` from the element's
   kind + stereotypes and asks `StyleBuilder::lookup`. The result fills the
   `style_block` tier of the existing `CascadeInput`.
4. `skinparam` continues to flow through tier 3 unchanged. When a property
   has both a skinparam value (tier 3) and a `<style>` value (tier 5), the
   `<style>` value wins — matching upstream's `StyleLoader` priority order.
5. *Compatibility shim* (deletion path): for the duration of the
   migration, the new `StyleBuilder::lookup()` *also* emits each resolved
   property as a synthetic skinparam through `apply_style_value()` so any
   family not yet wired to `EffectiveStyle` still renders correctly. This
   shim is removed in Phase E once all families are migrated.

### 4.5 Per-family render dispatch (Phase B/C)

Each family already has an `effective_*_node_style` function in
`src/theme/effective.rs` (lines 117-289). The migration pattern per family is
small:

1. Add a `StyleQuery` builder helper: `family_node_style_query(node) ->
   StyleQuery`. Walks the `FamilyNode` ancestor chain to build the SName
   tag list (e.g. `[componentDiagram, package, component]`) and stereotypes.
2. Call `builder.lookup(&query)` to get the merged `EffectiveStyle`.
3. Layer the result into the existing `CascadeInput` at the `style_block`
   tier and keep using `resolve_color` etc.
4. Renderer consumes the resulting `EffectiveColor` exactly as today — no
   render-call-site changes.

Order of family migration (lowest risk first):

1. **Component / deployment** (#1404b acceptance) — these already use
   `shared_cascade` and have the largest test corpus.
2. **Class / usecase** — same shared cascade.
3. **Sequence / state / activity** — each is touched by its own
   `apply_style_param` in normalize/; smallest risk.
4. **Salt / timing / mindmap / chart** — last; lowest theme usage.

### 4.6 Stereotype + wildcard selectors (Phase C)

Already covered by §4.1 (parser) and §4.3 (resolver). Phase C is the
*integration* ticket: ensure `family_node_style_query` walks the node's
inline stereotype markers (the `\x1fstyle:…` and `<<foo>>` patterns already
extracted in `family_node_stereotype_key`, `src/theme/effective.rs:71-81`)
and feeds them into `query.stereotypes`. Adds 4-6 fixtures from
`.plantuml-reference-raw.txt` §8.16.1 lines 9160-9199 (the `.stereo`
selector applied to 22 deployment element kinds).

### 4.7 Missing properties batch 1 (Phase D)

Add parser→ast→cascade plumbing for the 10 highest-leverage missing
properties (the audit's "20 missing" list, pared to the 10 that unblock the
most themes):

| `PName`              | Render hook(s)                                           |
|----------------------|----------------------------------------------------------|
| `LineThickness`      | every shape stroke; `font_family` family_node_inline_style |
| `LineStyle`          | `dashed`, `dotted`, `bold` keyword → SVG stroke-dasharray |
| `Padding`            | family bbox inflation in `effective.rs`                   |
| `Margin`             | external margin in graph_layout                            |
| `RoundCorner`        | already wired for some shapes — extend coverage           |
| `FontWeight`         | font config; map `bold` / `normal` / numeric              |
| `Shadowing`          | SVG drop-shadow filter intensity                          |
| `HorizontalAlignment`| label paint alignment in `render::text`                   |
| `MaximumWidth`       | text wrap upper bound                                     |
| `MinimumWidth`       | node bbox min-width clamp                                 |

Properties **not** in batch 1 (deferred to a follow-up; parse-store-only):
`DiagonalCorner`, `HyperLinkColor`, `HyperlinkUnderlineStyle`,
`HyperlinkUnderlineThickness`, `HeadColor`, `ExportedName`, `Image`,
`ShowStereotype`, `ImagePosition`, `MarkerShape`, `MarkerSize`, `MarkerColor`,
`BarWidth`, `Width`. These get a parse-and-store path so the rules survive
round-trip; rendering is a follow-up wave.

### 4.8 Validation, tests, docs (Phase E)

- 15-20 unit tests covering: descendant nesting (3 deep), stereotype on
  arbitrary tag, wildcard, comma-separated selectors, `--css-var`
  substitution, `@media dark` block, ill-formed input (missing `}`, missing
  `;` recoverable), specificity tie-break.
- 8-12 integration fixtures under
  `tests/fixtures/style_cascade/*.puml` rendering each upstream LRG `<style>`
  example from §6.25 / §8.15.3 / §8.16.1 / §9.25.
- Visual baselines for `!theme cyborg` and `!theme reddress-darkblue` —
  follows the standard bless flow once parser + cascade + properties land.
- New diagnostic codes:
  - `[W_STYLE_UNKNOWN_PROPERTY]` for unrecognised `PName`.
  - `[W_STYLE_UNKNOWN_TAG]` for unrecognised `SName` selector.
  - `[E_STYLE_BLOCK_UNCLOSED]` (already exists — keep).
  - `[E_STYLE_BAD_VALUE]` for value parse failure (e.g. malformed colour).
- Docs:
  - New section in `docs/internal/architecture/` for the cascade design
    (link back to this design doc).
  - Update `docs/internal/spec/audit/ch24-skinparam.md` to note the
    `<style>` cascade coverage.

---

## 5. Risks and mitigations

| Risk | Mitigation |
|------|------------|
| `BTreeMap` ordering for `PName` keys must be deterministic | `PName` is an enum with `#[derive(PartialOrd, Ord)]`; safe to use as `BTreeMap` key (CLAUDE.md §6 invariant). |
| Backward compat: an existing diagram that uses `<style>` today renders differently after Phase B | Phase B ships the compat shim from §4.4 so the *first* per-family migration only affects the family being migrated. Each family migration commit re-blesses just that family's baselines after manual PNG inspection. |
| Cascade resolver O(N·M) for N rules × M elements | Acceptable: real-world themes have ≤200 rules; per-diagram element counts ≤ ~200 in worst case. Cache keyed on `StyleQuery` removes redundant work for repeated queries within a render. |
| Unknown selector identifiers must not silently break the resolver | `SelectorSegment::Unknown(String)` is parsed but never matches — produces a `[W_STYLE_UNKNOWN_TAG]` warning at lookup time, and the rule contributes nothing. Mirrors upstream's silent `null` from `SName.retrieve()`. |
| Property name case-insensitivity | All `PName` lookups go through a single case-insensitive helper; matches `PName.getFromName(name, scheme)` upstream. |
| `@media dark` block parsing without dark-scheme switch wired through CLI | Phase A stores the dark rules in `StyleBlock { scheme: Dark }`. Phase B/C/D ignore them at lookup. Phase E (or a follow-up) adds a `--scheme dark` flag. Forward-compat: no syntax errors during parse. |

---

## 6. Decomposition into child issues

Each child issue is scoped for one Sonnet agent (≤120 min implementation,
small file-touch list, isolated test scope).

| Issue | Title | Scope | Files touched |
|-------|-------|-------|---------------|
| **A** | `<style>` parser + AST surface | New `StyleBlock` / `StyleRule` / `SelectorSegment` types; new `parser/style_block.rs` module; replace `parse_style_block` body in `parser/directives.rs`. Legacy `StyleParam` emitted alongside via compat shim. | `src/ast/mod.rs` (+1 variant), `src/parser/directives.rs` (rewrite §73-218), new `src/parser/style_block.rs`, new `src/theme/sname.rs` (SName enum), new `src/theme/pname.rs` (PName enum), 6-8 parser unit tests under `src/parser/tests/`. |
| **B** | Cascade resolver + skinparam integration | New `StyleBuilder` in `src/theme/style_builder.rs`; `StyleQuery`, `EffectiveStyle`; wire into `shared_cascade::CascadeInput` tier 5 for class + component families only. Compat shim from §4.4 active. | New `src/theme/style_builder.rs`, `src/theme/shared_cascade.rs` (extend tier 5 producer), `src/theme/cascade.rs` (delegate to builder), `src/theme/effective.rs` (use lookup result), `src/normalize/family/directives.rs` (push rules into builder), 4 cascade unit tests. |
| **C** | Stereotype + wildcard selectors end-to-end | Wire `family_node_stereotype_key` + ancestor-chain SName walk into `StyleQuery`. Add fixtures from LRG §8.16.1 (`.stereo` on 22 deployment kinds). Migrate sequence + activity families to new resolver. | `src/theme/effective.rs` (StyleQuery builder), `src/normalize/sequence/state.rs`, `src/normalize/family/extended.rs`, `tests/fixtures/style_cascade/stereo_*.puml` (6 files), visual bless of those 6 fixtures. |
| **D** | 10 missing properties (batch 1) | Parser already stores them (Phase A). Wire each into the per-family renderer that consumes it. Order: `LineThickness` → `LineStyle` → `Padding` → `Margin` → `RoundCorner` → `FontWeight` → `Shadowing` → `HorizontalAlignment` → `MaximumWidth` → `MinimumWidth`. | `src/render/family.rs`, `src/render/graph_layout.rs`, `src/render/text.rs`, `src/theme/effective.rs` (extend `EffectiveClassNodeStyle` etc. with new fields), 10 integration fixtures (one per property), visual bless. |
| **E** | Validation, diagnostics, docs, theme bless | Add the 3 new diagnostic codes. Bless `!theme cyborg` + `!theme reddress-darkblue` visual baselines. Delete the legacy `StyleParam` compat shim. Update `docs/internal/architecture/` and `docs/internal/spec/audit/ch24-skinparam.md`. | `src/diagnostic.rs` (3 codes), `src/theme/style_builder.rs` (warning emission), `tests/visual_baselines/themes/{cyborg,reddress-darkblue}.png`, `docs/internal/architecture/style-cascade.md`, `docs/internal/spec/audit/ch24-skinparam.md`. |

### Execution order

`A → B → C` are strictly sequential (each depends on the previous).
`D` can start *after* B lands (does not depend on C — stereotype selectors
are orthogonal to per-property rendering).
`E` runs last, after `D` and `C` have both merged.

```
A ────► B ────► C
        │
        └────► D
                │
                ▼
                E (after both C and D)
```

Estimated agent hours (Sonnet, single-agent): A ≈ 100min, B ≈ 110min,
C ≈ 90min, D ≈ 120min (largest), E ≈ 80min. Total ≈ 8 agent-hours,
shippable across 2-3 calendar days with the standard PR-per-agent flow.

---

## 7. Acceptance criteria for the epic

(Restating from #1404 with the new design context.)

- `!theme cyborg` produces a dark background with the cyborg accent colors
  (subjective spot-check; not pixel-perfect).
- `!theme reddress-darkblue` produces a dark blue background with white text.
- LRG `<style>` examples in §6.25 / §8.15.3 / §8.16.1 / §9.25 round-trip
  without warnings and render with the documented colors.
- Existing PUML `<style>` tests (those currently passing under the legacy
  flat-StyleParam path) continue to pass (backward compat).
- New unit + integration tests added in Phase A + E pass.

---

## 8. Out-of-scope items captured for follow-up

These should be filed as separate tickets (not children of #1404) after
the epic closes:

1. **Dark scheme runtime switch** — `--scheme dark` CLI flag; consumes the
   `@media dark` rules already stored by Phase A.
2. **Remaining 10 properties** (parse-stored in Phase D but not rendered):
   `DiagonalCorner`, `HyperLinkColor`, `HyperlinkUnderlineStyle`,
   `HyperlinkUnderlineThickness`, `HeadColor`, `ExportedName`, `Image`,
   `ShowStereotype`, `ImagePosition`, `MarkerShape`, `MarkerSize`,
   `MarkerColor`, `BarWidth`, `Width`. Most map cleanly to existing render
   primitives; one or two (e.g. `Image`, `Shadowing` for sketchy themes) need
   new render machinery.
3. **Source-mapped style provenance** for the language service — each
   resolved property carries `(rule_source_span, rule_specificity, tier)`
   so hover / "which rule wins?" tools can report it.
4. **CSS-var expansion in expressions** (e.g. `var(--foo, fallback)`) —
   upstream doesn't support this yet either, so deferred.
5. **`<style>` import** via `!style filename.style` — already filed
   adjacent (mirrors `CommandStyleImport.java`). Track as a follow-up.

---

*This design is the source of truth for epic #1404 implementation. If a
child issue's acceptance criteria conflict with this doc, the doc wins —
file an issue and reconcile.*
