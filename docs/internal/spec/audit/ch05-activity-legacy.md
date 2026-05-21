# Chapter 5 — Activity Diagram (legacy syntax) audit

Reference spec: `/tmp/puml-spec/ch05-activity-diagram-legacy.txt` (374 lines)
Auditor: read-only audit, 2026-05-21
Repo entry points:
- `src/parser/activity.rs` (legacy detection in `looks_like_old_activity_flow` and `parse_activity_old_style_flow`, lines 235–293)
- `src/normalize/family.rs` activity normalization (`activity_step_node_kind` @ 1725; old-style flow @ ~1553–1567)
- `src/render/activity/` (mod.rs, nodes.rs, branches.rs, swimlanes.rs, arrows.rs, layout.rs)
- `src/ast.rs` ActivityStep / ActivityStepKind (300–324)

Status legend: ✅ supported, 🟡 partial/lossy, ❌ not supported

---

### 5.1 Simple Action — 🟡
**Feature:** `(*)` start/end terminators and `"Action"` nodes joined with `-->`.
**Syntax example:** `(*) --> "First Action"` / `"First Action" --> (*)`
**Status:** 🟡
**Evidence:** `src/parser/activity.rs:235-293` detects `(*)` / `-->` lines and emits `Start`/`Action`/`Stop` `ActivityStep`s. `parse_quoted_activity_label` strips the quotes. No support for `(*top)` variant.
**Notes:** Lossy: the legacy graph (arbitrary node→node edges) is flattened to a sequential start→actions→stop chain. Backward jumps and named-target reuse (e.g. `A1 --> "Short action"`) are not honored — every parsed line just appends an action.

### 5.2 Label on arrows — ❌
**Feature:** `[label]` between arrow and target.
**Syntax example:** `-->[You can put also labels] "Second Action"`
**Status:** ❌
**Evidence:** `src/parser/activity.rs:269-272` recognizes `[...]` and skips past it; the label text is discarded.
**Notes:** Parsed-and-dropped, not rendered.

### 5.3 Changing arrow direction — ❌
**Feature:** `-up->`, `-down->`, `-left->`, `-right->`, `->`.
**Syntax example:** `(*) -up-> "First Action"`
**Status:** ❌
**Evidence:** Only `-->` is searched (`activity.rs:244`, `looks_like_old_activity_flow` @ 235). `-up->` / `-left->` / `->` lines are not parsed as legacy activity flow.
**Notes:** Directional arrows silently produce no nodes for affected lines, breaking 5.3 examples.

### 5.4 Branches (if/then/else/endif) — ❌
**Feature:** legacy `if "..." then` / `else` / `endif`, with `-->[true]`/`-->[false]` labeled branches.
**Syntax example:** `if "Some Test" then` ... `else` ... `endif`
**Status:** ❌
**Evidence:** The parser's `if `/`else`/`endif` cases (activity.rs:110, 45, 56) only match the **new-syntax** `if (cond) then (label)` shape, not the legacy `if "..." then` form. Legacy `If` (capital I) won't match (case-sensitive). Bracketed arrow labels are discarded (see 5.2).
**Notes:** Even if the keywords matched, the legacy diagram's free-form arrow-driven branching is fundamentally incompatible with the linear new-syntax model the renderer uses.

### 5.5 More on Branches (nested + linked-to-if) — ❌
**Feature:** Nested if/endif and `(*) --> if "Test" then` linkage.
**Status:** ❌
**Evidence:** No handling of `--> if` continuation pattern. `looks_like_old_activity_flow` requires `(*)` or `-->` in the line; mixed forms aren't reconciled with the if-stack.

### 5.6 Synchronization bars `=== B1 ===` — ❌
**Feature:** Named synchronization bars used as fork/join points.
**Syntax example:** `(*) --> ===B1===` / `===B1=== --> "Parallel Action 2"`
**Status:** ❌
**Evidence:** No `===` token handling anywhere in `src/parser/activity.rs` or `src/normalize/family.rs`. `grep "===" src/parser/activity.rs` returns nothing.
**Notes:** Bars are dropped entirely; parallel topology is lost.

### 5.7 Long action description / `as` alias — ❌
**Feature:** Multi-line activity text, `\n`, `<size>`/`<color>` creole, and `as <alias>` for back-reference.
**Status:** ❌
**Evidence:** `parse_quoted_activity_label` (activity.rs:295) reads a single-line quoted string. No `as` alias handling, no multi-line continuation, no back-reference resolution.
**Notes:** Multi-line legacy actions break parsing; `as A1` and later `A1 --> ...` won't link.

### 5.8 Notes on activities — ❌
**Feature:** `note right:`, `note left`, `endnote` / `end note` attached to legacy activities.
**Status:** ❌
**Evidence:** `parse_activity_note_step` (activity.rs:326-349) recognizes note prefixes only when they appear as the start of an activity step (new-syntax). For legacy diagrams the note becomes a free-standing action labeled `"note right: ..."`. No attachment to a target node.

### 5.9 Partition `partition Name { ... }` — 🟡
**Feature:** Background-colored partitions wrapping legacy activities.
**Syntax example:** `partition Conductor { (*) --> "Climbs on Platform" }`
**Status:** 🟡
**Evidence:** `partition` keyword parsed at `activity.rs:168-185`; lone `}` closes it (186). Normalize tracks `activity_active_partition` (`family.rs:1088, 1138-1143`). The renderer treats partitions as swimlanes (see ch06 audit).
**Notes:** Background color stripped (`activity.rs:172-175`). Combined with 5.1's lossy `(*)`/`-->` parsing, partitioned legacy graphs render only the residual activities, not the legacy edges.

### 5.10 Skinparam (activity colors, stereotypes) — ❌
**Feature:** `skinparam activity { ... }`, `<< stereotype >>` markers.
**Status:** ❌
**Evidence:** `family.rs:1430` reads `activity_style.fork_color` but the broader `skinparam activity { StartColor / BarColor / EndColor / BackgroundColor / BackgroundColor<<stereo>> / BorderColor / FontName }` block is not parsed into activity styling. Stereotypes on legacy actions (`"Climbs" << Begin >>`) are not extracted.

### 5.11 Octagon (`skinparam activityShape octagon`) — ❌
**Feature:** Switch activity shape from roundBox to octagon.
**Status:** ❌
**Evidence:** No `activityShape` handling found in normalize or render.

### 5.12 Complete example — ❌
**Feature:** Realistic Servlet diagram exercising 5.1–5.10 combined.
**Status:** ❌
**Evidence:** Depends on labels-on-arrows (5.2), branches (5.4), sync bars (5.6), `as` alias (5.7) — none of which are supported. Rendering would produce a degenerate linear chain with most semantics dropped.

---

## Tally — Chapter 5

| Status | Count | Sections |
|---|---|---|
| ✅ Full | 0 | — |
| 🟡 Partial | 2 | 5.1, 5.9 |
| ❌ Missing | 10 | 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 5.8, 5.10, 5.11, 5.12 |

**Verdict:** Legacy activity syntax is effectively unsupported. The parser detects `(*)` / `-->` lines well enough to emit a degenerate linear chain, and partition keywords are honored, but every richer feature (labels on arrows, directional arrows, branches, sync bars, aliases, notes, skinparam, octagon shape, stereotypes) is missing. The strategic posture matches PlantUML's own guidance: migrate to chapter 6 new syntax. If parity for ch5 becomes a goal, a fresh graph-builder pass in `normalize/family.rs` would be required — the current linearization model cannot represent legacy free-form activity graphs.
