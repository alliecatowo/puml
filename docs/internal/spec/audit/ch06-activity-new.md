# Chapter 6 — Activity Diagram (new syntax) audit

Reference spec: `/tmp/puml-spec/ch06-activity-diagram-new-syntax.txt` (1595 lines)
Auditor: read-only audit, 2026-05-21
Repo entry points:
- `src/parser/activity.rs` (1–411) — line-level keyword recognition
- `src/normalize/family.rs` (1088–1567) — activity stream normalization, partition/fork tracking
- `src/render/activity/` — `mod.rs` (orchestration), `nodes.rs`, `branches.rs`, `arrows.rs`, `layout.rs`, `swimlanes.rs`
- `src/ast.rs:300-324` — `ActivityStep`, `ActivityStepKind` (Start, Stop, End, Action, IfStart, Else, EndIf, RepeatStart, RepeatWhile, WhileStart, EndWhile, Fork, ForkAgain, EndFork, PartitionStart, PartitionEnd)

Status legend: ✅ supported, 🟡 partial/lossy, ❌ not supported

Note: the `ActivityStepKind` enum is intentionally small; many distinct PlantUML features are collapsed onto the same variants (e.g. `switch`/`case` → `IfStart`/`Else`, `elseif` → `Else`, `split` → `Fork`, `end merge` → `EndFork`). This means many features "parse and render something" but with reduced fidelity.

---

### 6.1 Simple action — ✅
**Feature:** `:label;` action nodes, implicit sequential linking, multi-line bodies.
**Syntax example:** `:Hello world;`
**Status:** ✅
**Evidence:** `src/parser/activity.rs:16-26` parses `:body;`. Multi-line not explicitly joined in parser — see notes.
**Notes:** Creole formatting (`**bold**`) inside labels is passed through verbatim (not rendered). Multi-line `:foo\nbar;` depends on upstream raw-line handling; activity parser only sees one line at a time.

### 6.2 Start / Stop / End — ✅
**Feature:** `start`, `stop`, `end` keywords.
**Status:** ✅
**Evidence:** `activity.rs:27-44`; `normalize/family.rs:1725-1729` maps to `FamilyNodeKind::ActivityStart`/`ActivityStop`. Render in `nodes.rs:196-207`.

### 6.3 Conditional (if/then/else/endif) — 🟡
**Feature:** `if (...) then (...) ... else (...) ... endif`, with `is (...)` and `equals (...)` variants.
**Syntax example:** `if (Graphviz?) then (yes) ... else (no) ... endif`
**Status:** 🟡
**Evidence:** `activity.rs:110-115` parses `if `; `parse_activity_if_label` (351) extracts condition + branch label. `else` @ 45, `endif` @ 56.
**Notes:** `is (...)` and `equals (...)` are absorbed via `parse_activity_condition_with_branches` (368) which joins condition+branch with `" / "` — lossy but renders. Branch labels on edges are stored on the condition node label, not as edge labels.

#### 6.3.1 elseif (horizontal mode) — 🟡
**Feature:** `elseif (cond) then (label) ... (no) elseif (...) ...`.
**Status:** 🟡
**Evidence:** `activity.rs:137-142` maps `elseif` to `ActivityStepKind::Else` with `"elseif "` prefix in the label.
**Notes:** Multiple sequential `Else` steps inside a single `IfStart`/`EndIf` block are emitted; renderer treats them as parallel else-columns via `branches.rs` (line 19). Horizontal vs vertical layout: no per-elseif horizontal column layout; renders as a flat ladder.

#### 6.3.2 Vertical mode (`!pragma useVerticalIf on`) — ❌
**Feature:** Toggle horizontal vs vertical elseif via pragma.
**Status:** ❌
**Evidence:** No `useVerticalIf` pragma handler found in repo (`grep -r useVerticalIf` returns nothing).

### 6.4 Switch / case / endswitch — 🟡
**Feature:** `switch (test?) case (A) ... case (B) ... endswitch`.
**Status:** 🟡
**Evidence:** `activity.rs:116-136` maps `switch` → `IfStart` (with `"switch "` prefix), `case` → `Else`, `endswitch` → `EndIf`.
**Notes:** Functionally rendered as an if/elseif/elseif/... ladder. Visually NOT a true switch (no diamond-with-n-branches shape); diamond label says `"switch test?"`. Acceptable for many uses, not strict-parity.

### 6.5 kill / detach (stop on action) — ✅
**Feature:** `kill` / `detach` keywords inside if/fork branches to terminate without joining.
**Status:** ✅
**Evidence:** `activity.rs:190-200` parses both, emits `ActivityStepKind::Kill` / `ActivityStepKind::Detach`. Renderer (`nodes.rs`) distinguishes by `step_kind`: kill renders as an X-in-circle; detach renders as a horizontal bar. Both suppress outgoing arrows via `emit_predecessor_arrow` (`nodes.rs:274`, checks `"Kill" | "Detach"`).
**Notes:** Within fork branches, kill/detach still draw arrows to the join bar (a deeper layout change would be needed to fully suppress fork-branch joins). The visual shape distinction is correct for linear-flow contexts.
**Implemented:** 2026-05-21 (PR #948)

### 6.6 Repeat loop — 🟡
#### 6.6.1 Simple repeat — 🟡
**Feature:** `repeat ... repeat while (cond) is (yes) not (no)`.
**Status:** 🟡
**Evidence:** `repeat` @ activity.rs:104, `repeat while` @ 155-161.
**Notes:** `is`/`not` branch labels are concatenated into the condition label via `parse_activity_condition_with_branches` — they render as text, not as edge labels.

#### 6.6.2 Repeat with action target + backward — 🟡
**Feature:** `repeat :foo as starting label;` and `backward:label;`.
**Status:** 🟡
**Evidence:** `backward` parsed at activity.rs:205-219 but flattened into a regular `Action` with `"backward ..."` label prefix.
**Notes:** No back-edge routing; the action becomes a forward node. Visually wrong for true backward-arrow semantics. `as` alias on `repeat :foo as label` not parsed.

### 6.7 Break on a repeat loop — ❌
**Feature:** `break` keyword to exit a loop early.
**Status:** ❌
**Evidence:** `activity.rs:226-230` parses `break`/`continue` as a plain `Action` node labeled `"break"`.
**Notes:** No actual control-flow edge to loop exit. Text appears as an action; loop topology unchanged.

### 6.8 Goto / label — ❌
**Feature:** `label <name>` / `goto <name>` (experimental in PlantUML).
**Status:** ❌
**Evidence:** `activity.rs:193-204` parses both but emits them as plain `Action` nodes labeled `"label sp_lab0"` / `"goto lab"`.
**Notes:** No graph-level resolution; renders the strings as activity boxes.

### 6.9 While loop — 🟡
#### 6.9.1 Simple while / endwhile + `is`/`not` labels — 🟡
**Evidence:** `while ` @ activity.rs:149, `endwhile` @ 98 and 162.
**Notes:** Like repeat, branch labels stored in condition text.

#### 6.9.2 While + backward — 🟡
See 6.6.2 — backward flattened.

#### 6.9.3 Infinite while via `detach` + `-[hidden]->` — ❌
**Feature:** Suppress trailing arrow to form infinite loop.
**Status:** ❌
**Evidence:** No arrow-style handling for activity arrows (`-[hidden]->`, `-[#blue]->`) in `src/parser/activity.rs`. `arrows.rs` in render renders edges with default styling only.

### 6.10 Fork / fork again / end fork / end merge — 🟡
#### 6.10.1 Simple fork — ✅
**Evidence:** `activity.rs:62-79`. Renderer handles fork columns: `mod.rs:62-79` counts max branch count for canvas sizing; `nodes.rs:137` draws fork bar; arrows in `mod.rs:308` (`fork-bar→branch and branch→join-bar`).

#### 6.10.2 fork with end merge — 🟡
**Evidence:** `end merge` → `EndFork` with `"end split"` label (activity.rs:92-97).
**Notes:** Rendered as a regular join bar; visual distinction between `end fork` (synchronization) and `end merge` (merge) is lost.

#### 6.10.3 Label on end fork `{or}` / `{and}` (joinspec) — ❌
**Evidence:** No `{or}`/`{and}` parsing.

#### 6.10.4 Fork inside if — 🟡
**Evidence:** Should work compositionally (depth counters in normalize/family.rs:1089-1153). Visual quality not audited.

### 6.11 Split processing — 🟡
**Feature:** `split` / `split again` / `end split` for multi-start / multi-end shapes.
**Status:** 🟡
**Evidence:** `activity.rs:80-97` aliases split to fork. `end merge` also maps here.
**Notes:** Multi-start ("input split" via `-[hidden]->` at the top) not supported — hidden-arrow directives aren't parsed. Multi-end (each branch ends with `kill`/`detach`/`stop`/`end`) renders, but the no-join visual semantic is incorrect because all branches still connect to a synthetic join bar.

### 6.12 Notes — 🟡
**Feature:** `note left:`, `note right`, `floating note`, multi-line `note ... end note`, attached to actions, backward steps, partitions.
**Status:** 🟡
**Evidence:** `parse_activity_note_step` (activity.rs:326-349) recognizes `note left/right/top/bottom` and `floating note*` prefixes — but stores the text on a synthetic `Action` step labeled e.g. `"note right: This is a note"`.
**Notes:** Notes render as activity boxes in the flow, not as attached side-notes. Multi-line `note ... end note` blocks: no multi-line aggregation here; depends on upstream block handler. Notes on `backward`/`partition` definitely not attached correctly.

### 6.13 Colors (`#red:label;`, gradient) — 🟡
**Feature:** `#HotPink:label;` per-action background; `#red/white` partition gradient; `#blue\green:` action gradient.
**Status:** 🟡
**Evidence:** `parse_activity_colored_action` (activity.rs:315-320) recognizes `#color:body;` but **discards the color** — only the body is kept.
**Notes:** Action color stripped. Partition color stripped (activity.rs:172-175). Gradient (`#a/b`, `#a\b`) not recognized.

### 6.14 Lines without arrows (`skinparam ArrowHeadColor none`) — ❌
**Evidence:** No `ArrowHeadColor` handling. Arrows always rendered with heads.

### 6.15 Arrows: text, color, dotted/dashed/bold/hidden — ❌
**Feature:** `-> label;`, `-[#blue]->`, `-[#green,dashed]-> text`, `-[#gray,bold]->`.
**Status:** ❌
**Evidence:** `src/parser/activity.rs` has no handler for activity arrow directive lines starting with `->` or `-[...]->`. They will not match any branch and be dropped silently.
**Notes:** This is a significant gap — all arrow styling and inline arrow labels for new-syntax activities are unsupported.

### 6.16 Connector `(A)` — ❌
**Feature:** Parenthesized single-letter connector nodes.
**Syntax example:** `(A) detach` / `(A) :Other activity;`
**Status:** ❌
**Evidence:** No `(X)` connector parser branch in activity.rs.

### 6.17 Color on connector (`#blue:(B)`) — ❌
See 6.16; not parsed.

### 6.18 Grouping or partition — 🟡
#### 6.18.1 `group ... end group` — ❌
**Evidence:** No `group ` / `end group` handler in `src/parser/activity.rs`. Lines silently dropped.

#### 6.18.2 Partition `partition Name { ... }` — 🟡
**Evidence:** `activity.rs:168-185` + close-brace at 186. Normalize tracks active partition (`family.rs:1088, 1138-1143`).
**Notes:** Color stripped. Partition link in label (`partition "[[url name]]"`) — link not parsed as a hyperlink.

#### 6.18.3 `package`, `rectangle`, `card` — ❌
**Evidence:** Not recognized as activity grouping constructs.

### 6.19 Swimlanes `|Lane|` (with color + alias) — 🟡
**Feature:** `|Lane|`, `|#color|Lane|`, `|alias| Title`, `|#color|alias| Title`.
**Status:** 🟡
**Evidence:** `parse_activity_swimlane` (activity.rs:302-313) — line must start and end with `|`. Splits on `|`, filters out `#color` tokens, returns the last non-color non-empty segment as the lane name. Maps to `PartitionStart`. Renderer: `src/render/activity/swimlanes.rs` emits lane bands; `mod.rs:101-105` distinguishes sequential vs concurrent partition lanes.
**Notes:** Color discarded. Alias syntax `|alias| Title` collapses to "Title" only — alias-vs-title distinction lost; subsequent `|alias|` references may not find the same lane. Color-coded activity within a swimlane (`#pink:**action red**;`) loses the color (see 6.13).

### 6.20 detach / kill (removing arrows in forks/ifs) — ✅
Same as 6.5: kill now renders as X-in-circle, detach as horizontal bar; both suppress outgoing arrows in sequential flow. (Fork-branch join-suppression remains partial — see 6.5 notes.)
**Implemented:** 2026-05-21 (PR #948)

### 6.21 SDL (Specification and Description Language) — 🟡
#### 6.21.1 Table of SDL shapes — n/a (reference)
#### 6.21.2 Final separator variants (`|`, `<`, `>`, `/`, `\\`, `]`, `}`) — ✅
**Evidence:** `parse_activity_action_terminator` (activity.rs) detects trailing terminator chars and emits `\x1fsdl:<shape>\x1f<label>`. Normalize (`family.rs`) calls `extract_activity_sdl_shape` to populate the `sdl=` alias key. Renderer (`nodes.rs:emit_activity_action_box`) renders each shape distinctly: `>` → right chevron, `<` → left notch, `/` → right parallelogram, `\` → left parallelogram, `|` → plain rectangle, `]` → slightly rounded rect, `}` → hexagonal/octagonal shape. Colors from `#color:body>` also combine correctly with SDL terminators.
**Implemented:** 2026-05-21 (PR #948)
#### 6.21.3 Stereotype form (`:label; <<input>>` etc.) — ❌
**Evidence:** No `<<...>>` stereotype recognition in activity parsing or rendering.
**Notes:** A separate `src/render/sdl.rs` exists for full SDL diagrams (top-level family detection), but it does NOT participate in activity rendering when SDL shapes are embedded inline.

### 6.22 Complete example — 🟡
**Evidence:** Uses 6.2, 6.3 (if/else/endif with labels), nested ifs, `stop`. All of those are at least 🟡 — example will render approximately, with branch labels embedded in condition text rather than on edges.

### 6.23 Condition Style (`skinparam conditionStyle inside|diamond|InsideDiamond`) — ❌
**Evidence:** No `conditionStyle` skinparam handler.

### 6.24 Condition End Style (`diamond` vs `hline`) — ❌
**Evidence:** No `ConditionEndStyle` skinparam handler. Renderer uses a fixed end-of-condition rejoin style.

### 6.25 Global style block (`<style> activityDiagram { ... } </style>`) — ❌
**Evidence:** A small subset of activity style fields is read (`family.rs:1430` `fork_color`), but full nested style blocks (`diamond {}`, `arrow {}`, `partition {}`, `note {}`) are not parsed into activity rendering.

---

## Tally — Chapter 6

| Status | Count | Sections |
|---|---|---|
| ✅ Full | 6 | 6.1, 6.2, 6.5, 6.10.1, 6.20, 6.21.2 |
| 🟡 Partial | 12 | 6.3, 6.3.1, 6.4, 6.6.1, 6.6.2, 6.9.1, 6.9.2, 6.10.2, 6.10.4, 6.11, 6.12, 6.13, 6.18.2, 6.19, 6.21, 6.22 |
| ❌ Missing | 12 | 6.3.2, 6.7, 6.8, 6.9.3, 6.10.3, 6.14, 6.16, 6.17, 6.18.1, 6.18.3, 6.21.3, 6.23, 6.24, 6.25 |

_Updated 2026-05-21: promoted 6.5 (kill/detach shapes), 6.20 (kill/detach arrow suppression), 6.21.2 (SDL terminators) from ❌→✅._

(Counts include sub-sections individually; some rows cover multiple sub-features.)

### Cross-cutting gaps (highest impact)
1. **Activity arrow styling (6.15)** — no parser for `->`, `-[#blue]->`, `-[#…,dashed]->`, inline arrow labels. Blocks 6.9.3, 6.11 input-split, 6.14, and ch6.22 fidelity.
2. **Color on actions/partitions (6.13, 6.18.2, 6.19)** — colors are recognized and stripped; rendering ignores them.
3. **Detach/kill no-arrow semantic (6.5, 6.20, 6.11 multi-end)** — both keywords collapse to Stop nodes; the "suppress trailing arrow" behavior isn't implemented.
4. **Branch labels become condition text, not edge labels (6.3, 6.6, 6.9)** — visually misplaced.
5. **Backward action (6.6.2, 6.9.2)** — flattened to forward action; no back-edge.
6. **Connectors `(A)` (6.16, 6.17)** — completely missing.
7. **Group / package / rectangle / card grouping (6.18.1, 6.18.3)** — only `partition` is honored.
8. **SDL inline shapes (6.21)** — terminator-based and stereotype-based forms both unrecognized inside activity flow.
9. **Skinparam / style blocks (6.23, 6.24, 6.25)** — `conditionStyle`, `ConditionEndStyle`, full `<style>` activity sub-blocks not wired through.
10. **Goto / label / break (6.7, 6.8)** — text-only; no graph effect.

### Architecturally easy wins (small diff, big parity)
- Parse activity arrow directives (`->`, `-[..]->`) into AST so labels/colors/styles can flow to `arrows.rs`.
- Preserve background color from `parse_activity_colored_action` into the `ActivityStep` (add a `color` field or fold into `label` with a marker the renderer can split). The renderer already has `act_style.fork_color` plumbing to mimic.
- Recognize `group`/`end group`, `package`/`}`/`}` as partition-equivalents.
- Add `(X)` connector as a new `ActivityStepKind::Connector` variant; renderer can treat as small circle/diamond.
- Honor `kill`/`detach` distinct from `Stop` by suppressing the outgoing arrow in `arrows.rs`.

### Architecturally harder
- True back-edges (backward action), label-based gotos, infinite-loop topology, split multi-start/multi-end semantics, switch as a true n-way diamond, vertical-elseif layout — all require enriching `ActivityStepKind` and the `layout.rs` graph builder.

**Bottom line:** new-syntax activity diagrams are usable for the common cases (sequential actions, simple if/else, fork/join, basic swimlanes/partitions), but most styling, coloring, edge-label, and advanced-control-flow features are either parsed-and-discarded or not parsed at all. The activity module is one of the more developed renderers in the codebase (per PR #860 split), and many gaps are shallow parser fixes once an `ActivityStep` carries richer metadata.
