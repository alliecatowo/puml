# Chapter 9 — State Diagram audit

Scope: PlantUML Language Reference Guide (1.2025.0), §9.1–§9.25.
Repo paths referenced are relative to `/Users/allison.coleman/Develop/puml`.

Legend: ✅ supported · 🟡 partial / cosmetic gaps · ❌ not implemented

---

### 9.1 Simple State — ✅
**Feature:** `[*]` start/end pseudo-state, `-->` transitions, `:` description lines
**Syntax example:** `[*] --> State1` / `State1 : description`
**Status:** ✅
**Evidence:** `src/parser/state.rs:157,162` (transition + internal action), `src/normalize/state.rs:158-180` ([*] split into Initial + synthetic `[*]__end`), `src/render/state.rs:1482` (`start-end` class)
**Notes:** Initial/final pseudo-state is split when `[*]` is used as both source and target (creates `[*]__end`). `State : foo` is captured as `StateInternalAction` rather than a `display` description; rendered via `internal_actions` (`render/state.rs`).

### 9.2 Change state rendering (`hide empty description`) — ❌
**Feature:** `hide empty description` to render state as a simple box
**Syntax example:** `hide empty description`
**Status:** ❌
**Evidence:** no match for `empty description` in `src/`. `HideOption` exists but no `empty description` branch in `src/normalize/state.rs`.
**Notes:** `class` diagram has `hide_empty_members` (`src/render/family.rs:1745`) — no state equivalent. Hides are silently dropped.

### 9.3 Composite state — ✅
**Feature:** `state X { … }` with nested children, including sub-state to sub-state transitions
**Syntax example:** `state NotShooting { [*] --> Idle … }`
**Status:** ✅
**Evidence:** `src/parser/state.rs:132-152,171-291` (`parse_state_block`), `src/normalize/state.rs:358-460` (`state_decl_to_node`, recursive `collect_decl_transitions`)
**Notes:** Cross-region transitions are flattened into the top-level transition list. The "Long name dot syntax" `state A.X` (§9.3.2 ref. QA-3300) is not specially parsed — treated as a normal name containing a `.`.

### 9.4 Long name (`state "Long" as alias`) — ✅
**Feature:** Quoted long description + alias
**Syntax example:** `state "Accumulate Enough Data" as long1`
**Status:** ✅
**Evidence:** `src/parser/state.rs:120-126` (`as` split), `clean_ident` strips quotes
**Notes:** Display label is preserved as `display: Some(decl.name.clone())` (`normalize/state.rs:454`).

### 9.5 History `[H]` / `[H*]` — 🟡
**Feature:** Shallow/deep history pseudo-states
**Syntax example:** `State2 --> State3[H*] : DeepResume`
**Status:** 🟡
**Evidence:** `src/parser/state.rs:46-51,203-218`, `src/normalize/state.rs:47-72,248-275`, `src/render/state.rs:1483-1484,1581` (renders H glyph)
**Notes:** Bare `[H]` / `[H*]` statement and stereotype `<<history>>` / `<<history*>>` (`state s2 as "H 2" <<history>>`) are recognized as a node, BUT the form `State3[H*]` as a transition endpoint (history of a specific composite) is not specifically parsed — `clean_bracketed_ident` treats the `[H*]` suffix as part of the identifier, so the edge becomes a transition to a node named `State3[H*]` rather than to the scoped history pseudo-state inside `State3`.

### 9.6 Fork / Join (`<<fork>>`, `<<join>>`) — ✅
**Feature:** Fork/join pseudo-states rendered as bars
**Syntax example:** `state fork_state <<fork>>`
**Status:** ✅
**Evidence:** `src/parser/state.rs:53-93,102-110` (stereotype extraction), `src/normalize/state.rs:359-365`, `src/render/state.rs:712,1485-1486,1595-1607` (bar rendering, width adjustment in `adjust_fork_join_bar_widths` at 1060)

### 9.7 Concurrent state (`--`, `||`) — ✅
**Feature:** Multiple concurrent regions inside a composite
**Syntax example:** `state Active { ... -- ... || ... }`
**Status:** ✅
**Evidence:** `src/parser/state.rs:196-200` (both `--` and `||` push divider), `src/normalize/state.rs:368-377,416-428` (region splitting + per-region `[*]__in__Parent__rN` scoping)
**Notes:** Region index is included in synthetic `[*]` names so multiple regions don't collide.

### 9.8 Conditional `<<choice>>` — ✅
**Feature:** Choice diamond pseudo-state
**Syntax example:** `state c <<choice>>`
**Status:** ✅
**Evidence:** `src/parser/state.rs:54` (keyword form), stereotype branch at `src/normalize/state.rs:361`, render at `src/render/state.rs:713,1487,1609`

### 9.9 Stereotypes full example (start, choice, fork, join, end, history, history\*) — 🟡
**Feature:** All UML stereotype shapes
**Syntax example:** `state start1 <<start>>` / `state end3 <<end>>` / `state sdlreceive <<sdlreceive>>`
**Status:** 🟡
**Evidence:** `src/parser/state.rs:53-58` matches only `choice|fork|join|end` as keyword stereotypes. `<<start>>`, `<<history>>`, `<<history*>>`, `<<sdlreceive>>` fall through to `state … <<...>>` general form (`src/parser/state.rs:103-110`).
**Notes:** `<<start>>` is parsed as a stereotype string but `state_decl_to_node` (`normalize/state.rs:359-365`) only maps `fork|join|choice|end` to a kind — `start`, `history`, `history*`, `sdlreceive` all render as a Normal rounded rectangle with `data-state-stereotype` attribute only. `<<end>>` maps to a node kind but renders as a `StateNodeKind::End` filled circle.

### 9.10 Points (`<<entryPoint>>`, `<<exitPoint>>`) — ❌
**Feature:** Entry/exit point pseudo-states on the boundary of a composite
**Syntax example:** `state entry1 <<entryPoint>>`
**Status:** ❌
**Evidence:** not found — no `entryPoint`/`exitPoint` handling in `src/normalize/state.rs` or `src/render/state.rs`. Falls through to Normal node with stereotype data attribute.

### 9.11 Pins (`<<inputPin>>`, `<<outputPin>>`) — ❌
**Feature:** Pin pseudo-states
**Syntax example:** `state entry1 <<inputPin>>`
**Status:** ❌
**Evidence:** not found — no inputPin/outputPin recognition.

### 9.12 Expansion (`<<expansionInput>>`, `<<expansionOutput>>`) — ❌
**Feature:** Expansion port pseudo-states
**Syntax example:** `state entry1 <<expansionInput>>`
**Status:** ❌
**Evidence:** not found.

### 9.13 Arrow direction — 🟡
**Feature:** `-up->`, `-down->`, `-left->`, `-right->` plus shortened forms (`-d-`, `-do-`)
**Syntax example:** `First -right-> Second`
**Status:** 🟡
**Evidence:** `direction` is captured into `StateTransition.direction` (`src/parser/state.rs:310`, via `split_family_arrow_styled`).
**Notes:** Captured at parse time but state layout uses `graph_layout` (`src/render/state.rs` calls `place_state_nodes`/`adjust_fork_join_bar_widths`) — direction hints are not honored by the layout engine in the state family (only used as `data-direction` for edges).

### 9.14 Change line color and style — 🟡
**Feature:** `-[#color]->`, `-[dashed]->`, `-[dotted]->`, `-[#color,bold]->`, direction + style combined
**Syntax example:** `S1 -[#DD00AA]-> S2` / `S1 -up[#red,dashed]-> S4`
**Status:** 🟡
**Evidence:** `src/parser/state.rs:306-311` captures `line_color`, `dashed`, `hidden`, `thickness` via `split_family_arrow_styled`.
**Notes:** `dotted` and `bold` rendering on edges is family-arrow logic shared with class diagrams; for state edges these are applied through `src/render/relation.rs`. `hidden` is honored (transitions skipped). Verify `dotted` reaches state renderer.

### 9.15 Note (`note left of`, `note right of`, `note top of`, `note bottom of`, multi-line, floating) — 🟡
**Feature:** Floating and attached notes on states
**Syntax example:** `note left of Active : this is a short note`
**Status:** 🟡
**Evidence:** `StatementKind::Note` is produced by the parser (`src/parser/multiline.rs`), normalized into `StateNodeKind::Note` (`src/normalize/state.rs`), and rendered as folded note shapes with dashed connectors (`src/render/state.rs`). Covered by `tests/state_ch09_parity.rs`.
**Notes:** Attached notes render adjacent to the target state for `left` / `right` / `top` / `bottom`. Floating notes without a target are not yet separately laid out.

### 9.16 Note on link — ✅
**Feature:** `note on link … end note` attached to the most recent transition
**Syntax example:** `State1 --> State2` then `note on link` block
**Status:** ✅
**Evidence:** `note on link` / `<side> on link` is recognized in `src/parser/multiline.rs`, normalized as a note connector to the previous transition in `src/normalize/state.rs`, and rendered next to the transition midpoint in `src/render/state.rs`. Covered by `tests/state_ch09_parity.rs`.

### 9.17 Note on composite state — ✅
**Feature:** `note right of NotShooting : This is a note on a composite state`
**Status:** ✅
**Evidence:** Composite states are normal `StateNode`s in the state model, so attached notes use the same normalization and renderer placement as simple states.

### 9.18 Inline color (`state Foo #pink { … }`) — ❌
**Feature:** Background color on state, including gradients (`#red-green`) and inside composites
**Syntax example:** `state CurrentSite #pink { state HardwareSetup #lightblue { … } }`
**Status:** ❌
**Evidence:** `StateDecl` (`src/ast.rs:200-208`) has no color field. Parser does not extract `#color` from `state Foo #pink`. `StateNode` has no `fill_color` field (`src/model.rs:57-66`). State render uses `StateStyle.background_color` globally.

### 9.19 Skinparam (state-specific) — 🟡
**Feature:** `skinparam state { StartColor … BackgroundColor … BorderColor … FontName … }`, stereotype-scoped `BackgroundColor<<Warning>>`
**Status:** 🟡
**Evidence:** `src/normalize/state.rs:78-121` handles BackgroundColor, BorderColor, ArrowColor, StartColor, FontColor, FontSize.
**Notes:** EndColor, AttributeFontColor, AttributeFontName, AttributeFontSize, AttributeFontStyle, FontName, FontStyle, stereotype-scoped overrides (`BackgroundColor<<Warning>>`) all emit `[W_SKINPARAM_UNSUPPORTED]` warnings.

### 9.20 Changing style (`<style> stateDiagram { … }`) — 🟡
**Feature:** `<style>` block scoped to `stateDiagram`, `arrow`, `diamond`
**Status:** 🟡
**Evidence:** Style blocks are recognized at the document level (general theme infra); state-diagram-specific style block selectors (`stateDiagram { … }`, `arrow { … }`, `diamond { … }`) are not specifically applied to state nodes.
**Notes:** No `style.rs` handler for `stateDiagram` selector found. Falls back to global theme.

### 9.21 Change state color and style (inline) — ❌
**Feature:** `state FooGradient #red-green ##00FFFF` / `##[dashed]blue` / `#color;line:c;line.dashed;text:color`
**Syntax example:** `state FooDashed #red|green ##[dashed]blue { }`
**Status:** ❌
**Evidence:** `StateDecl` has no border/fill color or border-style fields. Inline `#`/`##` modifiers on `state X` are not parsed.

### 9.22 Alias — ✅
**Feature:** `state alias1`, `state "long name" as alias3`, `state alias4 as "long name"`, plus the description form `state alias1 : "..."`
**Syntax example:** `state "long name" as alias3`
**Status:** ✅
**Evidence:** `src/parser/state.rs:113-126` handles all four forms via `as` split + quote stripping.

### 9.23 Display JSON Data on State diagram — ✅
**Feature:** `json $alias { … }` / `yaml $alias { … }` inline block embedded in a state diagram
**Syntax example:** `json jsonJ { "fruit":"Apple", … }`
**Status:** ✅
**Evidence:** `src/parser/projection_salt.rs` parses JSON/YAML projection blocks, `src/parser/state.rs` accepts them inside composite states, `src/normalize/state.rs` normalizes them to `StateNodeKind::JsonProjection`, and `src/render/state.rs` renders formatted projection cards with key/value rows and nested connectors.

### 9.24 State description (multi-line on `state` and composite) — 🟡
**Feature:** `state s3: long descr.` and `s4: long descr.` outside the `state s4` decl
**Status:** 🟡
**Evidence:** Bare `state Foo : description` and external `Foo : description` are parsed as `StateInternalAction` (`src/parser/state.rs:162,341-363`). Multiple `: description` lines accumulate in `internal_actions`.
**Notes:** Treating descriptions as internal actions is a misclassification but renders correctly; entry/exit `/` action parsing reuses the same struct.

### 9.25 Style for Nested State Body (`<style> .foo { state,stateBody { … } } state X <<foo>> { … }`) — ❌
**Feature:** Custom-named style class applied via stereotype, scoping `state` + `stateBody`
**Status:** ❌
**Evidence:** No `stateBody` selector handler; user-defined style classes (`.foo`) are not wired to state rendering.

---

## Tally

| Status | Count |
|--------|-------|
| ✅ supported | 11 |
| 🟡 partial | 5 |
| ❌ missing | 9 |

Top gaps blocking parity:
1. **Inline color & style on `state` (9.18, 9.21, 9.25)** — `StateDecl` lacks color/border fields.
2. **`hide empty description` (9.2)** — silently ignored, layout regresses visually.
3. **Points / Pins / Expansion stereotypes (9.10–9.12)** — render as plain rectangles or detached boxes instead of snapping to composite boundaries.
4. **History inside transition endpoint (`State3[H*]`)** — endpoint string is left intact, not scoped.
5. **Composite/parallel-region layout fidelity** — current rendering has known pseudo-state and divider-placement visual gaps tracked on the board.
