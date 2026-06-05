# `<style>` Block Cascade — Architecture Reference

> Phase E of [#1404](https://github.com/alliecatowo/puml/issues/1404).
> Design doc: `docs/internal/design/1404-style-cascade-design.md`.

---

## Overview

The `<style>` block cascade is a CSS-inspired style resolution system that mirrors
[PlantUML's upstream `StyleParser.java`](https://github.com/plantuml/plantuml/blob/master/src/main/java/net/sourceforge/plantuml/style/parser/StyleParser.java).
It allows diagram authors to set element colours, fonts, line styles, and geometry
via a nested selector syntax:

```plantuml
<style>
classDiagram {
  class {
    BackgroundColor #DBEAFE
    FontColor #1E3A5F
    LineColor #60A5FA
  }
}
</style>
```

---

## Data flow (runtime)

```
.puml source
    │
    ▼
┌──────────────────────┐
│  Parser (Phase A)    │  src/parser/directives.rs
│  parse_style_block   │  src/parser/style_block.rs
│                      │
│  → StatementKind::   │
│    StyleBlock(block) │
└──────────────────────┘
    │
    ▼  (in the statement stream alongside SkinParam, Theme, …)
┌──────────────────────────────────────────────────────────┐
│  Normalizer                                              │
│                                                          │
│  Each family normalizer calls push_with_warnings(rule)   │
│  on a per-document StyleBuilder:                         │
│                                                          │
│  • sequence/state.rs  — SequenceNormalizer               │
│  • family/stub.rs     — stub families (class, usecase …) │
│  • family/extended.rs — extended families (activity …)   │
│                                                          │
│  push_with_warnings also fires diagnostic validation:    │
│  • W_STYLE_UNKNOWN_TAG      (Unknown selector segment)   │
│  • W_STYLE_UNKNOWN_PROPERTY (key not in PName catalogue) │
│  • E_STYLE_BAD_VALUE        (malformed hex colour)       │
└──────────────────────────────────────────────────────────┘
    │
    ▼  StyleBuilder is attached to the per-family FamilyStyle / SequenceStyle
┌──────────────────────────────────────────────────────────┐
│  Renderer                                                │
│                                                          │
│  Per-element style query:                                │
│    let style = builder.lookup(&StyleQuery::tags([        │
│        SName::ClassDiagram, SName::Class_               │
│    ]));                                                  │
│                                                          │
│  The result (EffectiveStyle) is folded into CascadeInput │
│  at tier 5 (style_block), sitting between:               │
│    • tier 3: skinparam                                   │
│    • tier 6: inline style                                │
└──────────────────────────────────────────────────────────┘
```

---

## Key types

| Type | Location | Role |
|---|---|---|
| `StyleBlock` | `src/ast/style.rs` | Parsed `<style>` block AST; contains `Vec<StyleRule>` |
| `StyleRule` | `src/ast/style.rs` | One rule: selector path + property map + unknown properties |
| `SName` | `src/ast/style.rs` | Catalogue of recognised selector tag names (mirrors `SName.java`) |
| `PName` | `src/ast/style.rs` | Catalogue of recognised property names (mirrors `PName.java`) |
| `SelectorSegment` | `src/ast/style.rs` | `Tag(SName)` / `Stereotype` / `Wildcard` / `Depth(u32)` / `Unknown(String)` |
| `StyleValue` | `src/ast/style.rs` | `Color` / `Number` / `Keyword` / `Raw` |
| `StyleBuilder` | `src/theme/style_builder.rs` | Accumulates rules; resolves per-element via specificity cascade |
| `StyleQuery` | `src/theme/style_builder.rs` | Query: ancestor tag chain + stereotypes + optional depth |
| `EffectiveStyle` | `src/theme/style_builder.rs` | Result: `BTreeMap<PName, StyleValue>` |

---

## Specificity scoring

Mirrors `StyleParser.java`'s `AutomaticCounterBasic`:

| Match kind | Score |
|---|---|
| Each `Tag` segment matched | +100 |
| Each `Stereotype` segment matched | +1000 |
| `Wildcard` segment | +1 |
| Tie-break | later `source_order` wins |

A rule matches when every segment in its `selector_path` appears as a subsequence
in `query.tags`. Stereotypes must all appear in `query.stereotypes`. `Unknown`
segments **never** match (mirrors upstream `SName.retrieve()` returning `null`).

---

## Diagnostic codes

| Code | Severity | Trigger |
|---|---|---|
| `W_STYLE_UNKNOWN_TAG` | Warning | Selector name not in `SName` catalogue → `SelectorSegment::Unknown` |
| `W_STYLE_UNKNOWN_PROPERTY` | Warning | Property name not in `PName` catalogue → stored in `StyleRule::unknown_properties` |
| `E_STYLE_BAD_VALUE` | Error | Hex colour has wrong digit count (e.g. `#12345`) on a colour-type `PName` |
| `E_STYLE_BLOCK_UNCLOSED` | Error | `<style>` block missing `</style>` close tag |

Diagnostic constants are defined in `src/diagnostic.rs`.

Emission: `StyleBuilder::push_with_warnings` in `src/theme/style_builder.rs`.
Call sites: all three normalizer entry points (`sequence/state.rs`,
`family/stub.rs`, `family/extended/styles.rs`).

---

## Extending the cascade

### Adding a new property (`PName`)

1. Add the variant to `PName` enum in `src/ast/style.rs`.
2. Add the case-insensitive match arm to `PName::from_name`.
3. Wire up consumption: read from `EffectiveStyle` in the relevant renderer, add
   it to `CascadeInput` at tier 5.
4. Add a fixture under `tests/fixtures/style_cascade/` and a visual regression
   entry in `tests/visual_regression/manifest.json`.

### Adding a new selector (`SName`)

1. Add the variant to `SName` enum in `src/ast/style.rs`.
2. Add the match arm to `SName::retrieve` (case-insensitive).
3. Build a `StyleQuery` with the new tag in the renderer and pass it to
   `builder.lookup(...)`.

### Dark scheme support

The `@media dark { … }` blocks parse into `StyleScheme::Dark` rules — they are
accumulated by the parser but not yet applied at render time. Rules are
stored in the `StyleBlock` AST for future `--scheme dark` support (design §8.1).
Regular-scheme rules only are pushed into `StyleBuilder` today.

---

## Phase history

| Phase | Issue | What landed |
|---|---|---|
| A | #1413 | `<style>` parser + `StyleBlock` / `StyleRule` AST |
| B | #1414 | Cascade resolver (`StyleBuilder`) + class / component migration |
| C | #1415 | Sequence + activity migration |
| D | #1416 | 10 missing properties wired (LineThickness, Padding, Margin, …) |
| E | #1417 | Diagnostics, cyborg/reddress-darkblue baselines, compat-shim removal |
