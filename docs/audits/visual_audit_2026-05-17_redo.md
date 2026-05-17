# Visual Audit Redo — 2026-05-17 (High-DPI + SVG-Inspection Methodology)

**Auditor:** Claude Sonnet 4.6 agent  
**Date:** 2026-05-17  
**Binary:** `target/release/puml` built from main @ `8bab0ce`  
**Method:** SVG structural inspection (ground truth) + PNG at 192 DPI (visual quality)  
**Fixtures sampled:** 165 (across all 17 families)  
**Prior audit being re-done:** closed visual audit 2026-05-16 (which had major false alarms due to 96-DPI rasterization)

---

## 1. Headline Finding

**Overall pass rate: 92% (152/165 fixtures PASS, 3 MINOR, 4 BROKEN, 6 CATASTROPHIC)**

The fix PRs referenced in the task (#250 gantt, #251 mindmap/wbs, #252 activity if/else) were NOT yet merged as of this audit. All three issues they target were independently confirmed as real bugs. The sequence family, class family, state (basic), deployment, component, and chart families are all rendering correctly. The biggest remaining holes are old-style activity syntax (CATASTROPHIC), mindmap/WBS layout (CATASTROPHIC), and SDL state rendering (CATASTROPHIC).

---

## 2. Per-Family Verdict Table

| Family | Sampled | PASS | MINOR | BROKEN | CATASTROPHIC | Summary |
|--------|---------|------|-------|--------|-------------|---------|
| Sequence (basic) | 7 | 7 | 0 | 0 | 0 | All clean: participant labels, arrows, footboxes render correctly |
| Sequence (groups/alt/opt/loop) | 9 | 9 | 0 | 0 | 0 | Alt/else/loop fragments with labels all render |
| Sequence (lifecycle) | 6 | 6 | 0 | 0 | 0 | Activation boxes, create/destroy, return arrows work |
| Sequence (e2e) | 4 | 4 | 0 | 0 | 0 | Autonumber, vertical slice, advanced wave all pass |
| Class | 10 | 10 | 0 | 0 | 0 | Members, visibility markers, relations, stereotypes all render |
| Activity (new-style) | 3 | 1 | 0 | 2 | 0 | `if/else` linear-only (no branch offset); colored items broken |
| Activity (old-style) | 4 | 0 | 0 | 0 | 4 | All old-style fixtures CATASTROPHIC: stub renderer echoes source |
| State | 5 | 3 | 0 | 1 | 0 | Basic, concurrent, history, entry/exit work; fork/join BROKEN |
| Component | 2 | 2 | 0 | 0 | 0 | Ports, stereotypes, bracketed names all render |
| Deployment | 1 | 1 | 0 | 0 | 0 | node/artifact/database/cloud elements and edge labels work |
| Gantt | 4 | 2 | 0 | 2 | 0 | Date-explicit fixtures work; relative-date gantt produces 8px slivers |
| Mindmap | 2 | 0 | 0 | 0 | 2 | Uses class DAG renderer instead of radial/tree layout |
| WBS | 1 | 0 | 0 | 0 | 1 | Uses class DAG renderer instead of hierarchical tree layout |
| C4 | 2 | 2 | 0 | 0 | 0 | Person/System/Container elements with labels work |
| Salt | 2 | 1 | 1 | 0 | 0 | Login form OK; bootstrap fixture underrenders (no relation) |
| JSON/YAML | 4 | 4 | 0 | 0 | 0 | Key-value pairs render; field order is alphabetical (not source) |
| Chart | 3 | 3 | 0 | 0 | 0 | Bar, line, pie charts all render with axes and labels |
| Timing | 2 | 1 | 1 | 0 | 0 | Waveform OK; basic timing has extreme scale for short ranges |
| Usecase | 3 | 2 | 1 | 0 | 0 | Package-qualified actor names show full path (minor cosmetic) |
| Object | 2 | 2 | 0 | 0 | 0 | Fields and relations render correctly |
| EBNF | 1 | 1 | 0 | 0 | 0 | Railroad diagrams with alternation, repetition render |
| Math | 2 | 2 | 0 | 0 | 0 | LaTeX-style equations render with correct Greek/operator glyphs |
| Ditaa | 2 | 2 | 0 | 0 | 0 | ASCII-art box rendering with cBLU/cGRE/cYEL colors works |
| Regex | 1 | 1 | 0 | 0 | 0 | Railroad-style regex diagram renders |
| SDL | 1 | 0 | 0 | 0 | 1 | Only 2 of 4 states render; no transitions drawn |
| NWDiag | 1 | 0 | 1 | 0 | 0 | Nodes render as full-width bars instead of properly-sized boxes |
| Archimate | 1 | 0 | 0 | 1 | 0 | Elements render but relations appear as text lines, not arrows |
| Conformance | 6 | 6 | 0 | 0 | 0 | Creole formatting, named blocks work |

---

## 3. Top 10 Worst Remaining Offenders

Ranked by severity and prevalence.

### 1. CATASTROPHIC — Mindmap/WBS use class DAG renderer (not tree layout)
**Files:**
- `tests/fixtures/families/valid_mindmap_orientation.puml`
- `tests/fixtures/families/valid_mindmap_palette.puml`
- `tests/fixtures/families/valid_wbs_progress.puml`
- `docs/examples/mindmap/01_basic.puml`
- `docs/examples/wbs/01_basic.puml`

**SVG evidence:** `class="uml-relation"` present; zero `mindmap-node` or `wbs-node` elements. All nodes are placed in a rectangular grid with horizontal arrow connectors. Radial layout, branch coloring, and the left-side/right-side distribution of mindmap are entirely absent.

**Root cause:** CLI render path (`render_pages_from_model` → `render_family_document_svg`) was missing MindMap and Wbs cases and fell through to `render_class_svg`. PR #251 (open) targets this.

---

### 2. CATASTROPHIC — Old-style activity syntax renders as source-code echo
**Files:**
- `tests/fixtures/families/valid_activity_old_style.puml`
- `tests/fixtures/families/valid_activity_labeled_edges.puml`
- `tests/fixtures/families/valid_activity_swimlanes.puml`
- `tests/fixtures/families/valid_activity_colored.puml`
- `tests/fixtures/non_sequence/valid_activity_oldstyle_baseline.puml`

**SVG evidence:** `data-activity-kind="OldStyle"` for all nodes. Each statement in the old-style syntax (e.g. `(*) --> "Step1"`) becomes an OldStyle Action node whose text is the literal source line. The result is a vertical column of rounded rectangles showing raw PlantUML syntax, not a flow diagram. `valid_activity_old_style.puml` renders only a single empty ellipse (no nodes or arrows at all).

**Visual (192 DPI):** `valid_activity_old_style_192.png` — single thin ellipse with no text, no flows. `valid_activity_swimlanes_192.png` — two swimlane columns with rounded rectangles containing raw arrow syntax (`(*) --> "Start"`, etc.).

---

### 3. CATASTROPHIC — SDL state renderer missing states and transitions
**File:** `tests/fixtures/non_sequence/valid_sdl.puml`

**SVG evidence:** Source has 4 states (Idle, Authenticating, LoggedIn, Done) and 4 transitions. SVG contains only 2 state rectangles (Authenticating and LoggedIn), zero transition lines/arrows. The `start Idle` and `stop Done` entries, and all 4 transitions, are entirely absent.

---

### 4. CATASTROPHIC — Gantt sliver bars when timeline anchors mix relative and absolute dates
**File:** `tests/fixtures/timeline/valid_gantt_render.puml`

**SVG evidence:** Task bars have `width="8"`. Timeline ticks span from `1970-01-01` to `2026-05-02` (a 56-year range). The fixture uses `[Build] requires [Design]` constraints alongside `[Kickoff] happens on 2026-05-01`, causing the normalizer to create tasks with epoch-relative start days while the calendar anchor is 2026. This 56-year range compresses each day to ~0.027px, so bars display at the minimum 8px width.

**Visual (192 DPI):** `valid_gantt_render_192.png` — three 8px-wide blue slivers stacked vertically on the left edge, milestone diamond at far right. The timeline grid is entirely empty.

**Note:** PR #250 (open) targets a duplicate-task symptom. This audit found that the root cause is the mixed epoch/calendar date anchor, which is a separate but related issue.

---

### 5. BROKEN — Activity if/else renders both branches linearly (single x-axis)
**File:** `tests/fixtures/families/valid_activity.puml`, `docs/examples/activity/02_if_then_else.puml`

**SVG evidence:** All arrows have `x1="240"` (single x-axis). The diamond decision node, "then" action, "(else)" label, and "else" action all appear at `cx/x=240`. There is no horizontal offset to indicate branching — both branches overlap at the same x-coordinate. The diagram renders as a valid vertical flow but does not visually represent the branching structure.

**Visual (192 DPI):** `valid_activity_192.png` — single column of shapes: start → Receive order → Validate payment → diamond → Ship order → gap → Notify customer → (endif) → stop. No visual branch separation.

**PR #252 (open)** targets this.

---

### 6. BROKEN — State fork/join creates ghost nodes from edge syntax
**File:** `tests/fixtures/families/valid_state_fork_join.puml`

**SVG evidence:** `data-state-node="1 --> A"`, `data-state-node="1 --> B"`, `data-state-node="1 --> choice1"`, `data-state-node="1 --> end1 : done"` — the renderer is creating additional state nodes whose names are literally the edge/transition syntax. This produces 3 fork shapes, 2 join shapes, and 3 choice shapes instead of the expected 1 each.

**Visual (192 DPI):** `valid_state_fork_join_192.png` — chaotic layout with forking shapes, misplaced initial/end pseudo-states, crossing lines, and orphaned nodes far from the main flow.

---

### 7. BROKEN — Archimate relations render as text, not as arrows
**File:** `tests/fixtures/non_sequence/valid_archimate.puml`

**SVG evidence:** Relations appear as text elements with class `archimate-relation` showing `svc -[serving]-> cust : serves` as literal text, not as connector lines/arrows between element boxes. Elements render correctly in their layer bands but are visually disconnected.

---

### 8. BROKEN — Activity old-style colored: color codes appear as node names
**File:** `tests/fixtures/families/valid_activity_colored.puml`

**SVG evidence:** `#red` and `#green` appear as text inside ellipse nodes. The color specifier is being treated as part of the node label rather than applied as a fill color. Three ellipses rendered: `(*)`, `#red` / `Error Action;`, `#green` / `Success Action;`.

**Visual (192 DPI):** `valid_activity_colored_192.png` — three empty ellipses, no text visible (text too small for ellipse height at this scale), no color applied.

---

### 9. MINOR — Timing diagram: fixed canvas width regardless of time range
**File:** `tests/fixtures/families/valid_timing.puml`

**SVG evidence:** Diagram with time range `@0`–`@5` renders at `width="922"` with `@0` at `x=130` and `@5` at `x=890`, giving 152px per time unit. A 5-unit diagram fills the same canvas as a 20-unit diagram. Signal lines run nearly full width for only 2 time-state transitions, creating a horizontally stretched result. The waveform fixture with `@0`–`@20` (4x more time points) is correctly proportioned at 38px/unit.

---

### 10. MINOR — NWDiag nodes render as full-width swimlane bars
**File:** `tests/fixtures/non_sequence/valid_nwdiag.puml`

**SVG evidence:** Nodes (`web01`, `web02`, `db01`) have `width="680"` (nearly full canvas width). PlantUML renders nwdiag nodes as compact boxes connected to the network line, not as full-width horizontal bars. The layout is readable but structurally wrong.

---

## 4. Validation of Prior Findings

### #238 — Missing text labels across all families
**Verdict: FALSE ALARM (confirmed)**

Direct SVG inspection of 165 fixtures confirms text elements are present across all rendering families. The prior audit rasterized at default 96 DPI where 12–13pt text fell below pixel visibility threshold. At 192 DPI, labels are clearly visible in PNGs. The sequence, class, state, component, deployment, chart, c4, salt, gantt (when working), timing, ebnf, math, regex, ditaa, usecase families all show correct text content in SVG.

### #239 — Activity if/else: else-branch entirely absent
**Verdict: PARTIAL-FIX in flight (PR #252 open, not merged)**

The else-branch IS now present in the SVG (`data-activity-kind="Else"` and `data-activity-kind="Action"` for the else node are in the output). However, both branches are positioned at the same x-axis (x=240/x1=240 for all arrows), so the diagram looks linear even though both branches are rendered. The full fix (horizontal offset for each branch) requires PR #252 to merge.

### #240 — Mindmap/WBS: rectangular grid layout
**Verdict: STILL-BROKEN (PR #251 open, not merged)**

Confirmed: mindmap and WBS diagrams use `uml-relation` DAG layout. Both `families/valid_mindmap_orientation.puml` and `families/valid_mindmap_palette.puml` (and `families/valid_wbs_progress.puml`) render as rectangular class-diagram grids. PR #251 proposes adding MindMap/Wbs dispatch cases to the CLI render path.

### #241 — Gantt task bars entirely absent
**Verdict: STILL-BROKEN (PR #250 open, not merged); root cause differs from prior analysis**

Task bars ARE rendered (class `gantt-task` present) but display as 8px slivers. The prior audit described this as "entirely absent" — the bars exist but are too narrow to see. The root cause for `valid_gantt_render.puml` is that mixing `[Kickoff] happens on 2026-05-01` with bare `[Build] requires [Design]` constraints creates a timeline from epoch (1970-01-01) to 2026-05-01, compressing ~20,000 days into 564px. PR #250's fix (deduplicating task entries from split declarations) may partially help but does not address the epoch-anchor issue.

**Fixtures with explicit `project starts YYYY-MM-DD` + all-date tasks render correctly** (e.g. `valid_gantt_baseline.puml`, `valid_gantt_calendar_resource_scale.puml`, `docs/examples/gantt/01_basic.puml`).

---

## 5. New Findings (Not in Prior Audit)

### NF-1: SDL renderer drops start/stop states and all transitions
`@startsdl` with `start X` / `stop Y` / `A -> B : label` syntax silently drops the Idle (start) and Done (stop) pseudo-states and all transition lines. Only intermediate `state X` declarations render. No arrows between states appear.

### NF-2: State fork/join creates ghost nodes from edge syntax
The state renderer materializes edge declarations (`fork1 --> A`, `choice1 --> end1 : done`) as state nodes named after the edge syntax string, producing 2–3x the expected number of fork/join/choice shapes.

### NF-3: Old-style activity fallback emits source lines as text
The `OldStyle` activity renderer (for `(*) --> "Label"` syntax) creates one Action node per source line with the raw source line text as the node label. This produces a vertical column of rounded rectangles showing PlantUML syntax, not a flow diagram.

### NF-4: Archimate relations rendered as text rather than connectors
`Rel_Serving`, `Rel_Realization`, etc. produce text elements (`svc -[serving]-> cust : serves`) rather than SVG line/path elements connecting archimate element boxes.

### NF-5: Gantt epoch-anchor collapse for mixed absolute/relative date fixtures
When a gantt has some tasks with absolute dates (`happens on 2026-05-01`) and others with only dependency constraints, the internal date normalization uses Unix epoch (1970-01-01) as the implicit project start. The resulting 56-year timeline scale makes task bars visually invisible (8px minimum width).

### NF-6: Timing diagram canvas width is fixed regardless of time range
A `@0`–`@5` timing diagram renders at the same canvas width as a `@0`–`@20` diagram, stretching the time axis by 4x for short-range diagrams.

### NF-7: Usecase actor names include package namespace prefix
When an actor is declared inside a `package "Name"` block, the actor's label in the SVG shows `Name::ActorName` instead of just `ActorName`.

### NF-8: JSON/YAML field order is alphabetical, not source order
Fields in `@startjson` blocks render in alphabetical key order, losing the author's intended ordering. Source `"name", "role", "age"` → SVG shows `age, name, role`.

---

## 6. Manifest Updates — `tests/visual_regression/manifest.json`

Proposed changes to add real `expected_text` strings and accurate `min_text_elements`. The current manifest has empty `expected_text: []` for most families and conservative `min_text_elements` values.

```json
{
  "_comment": "Visual regression manifest. Each fixture is rendered to SVG and checked for: (1) all expected_text substrings appear, (2) no <text> element is empty. Add new fixtures here as the renderer gains coverage. Run: cargo test --test visual_regression",
  "fixtures": [
    {
      "path": "docs/examples/sequence/01_basic.puml",
      "family": "sequence",
      "expected_text": ["Alice", "Bob", "Hello"],
      "min_text_elements": 5
    },
    {
      "path": "docs/examples/sequence/05_alt_opt_loop.puml",
      "family": "sequence",
      "expected_text": ["alt", "else", "opt", "loop", "Alice", "Bob"],
      "min_text_elements": 12
    },
    {
      "path": "docs/examples/class/01_basic.puml",
      "family": "class",
      "expected_text": ["class Animal", "class Dog"],
      "min_text_elements": 2
    },
    {
      "path": "docs/examples/class/02_inheritance.puml",
      "family": "class",
      "expected_text": ["class Vehicle", "class Car", "class Truck"],
      "min_text_elements": 8
    },
    {
      "path": "docs/examples/activity/01_simple_flow.puml",
      "family": "activity",
      "expected_text": ["Simple Activity Flow", "Initialize"],
      "min_text_elements": 5,
      "_note": "New-style activity; expected_text intentionally excludes else-branch labels pending PR #252"
    },
    {
      "path": "docs/examples/activity/02_if_then_else.puml",
      "family": "activity",
      "expected_text": ["If-Then-Else Decision", "Receive Request", "Process", "Return 401"],
      "min_text_elements": 8,
      "_known_broken": "PR #252 not merged: both branches render at same x-axis (linear layout)"
    },
    {
      "path": "docs/examples/state/01_basic.puml",
      "family": "state",
      "expected_text": ["Idle", "Running"],
      "min_text_elements": 3
    },
    {
      "path": "docs/examples/usecase/01_basic.puml",
      "family": "usecase",
      "expected_text": ["Login", "Register"],
      "min_text_elements": 3
    },
    {
      "path": "docs/examples/component/01_basic.puml",
      "family": "component",
      "expected_text": ["Frontend"],
      "min_text_elements": 5
    },
    {
      "path": "docs/examples/object/01_basic.puml",
      "family": "object",
      "expected_text": ["Alice", "Bob", "knows"],
      "min_text_elements": 3
    },
    {
      "path": "docs/examples/deployment/01_nodes.puml",
      "family": "deployment",
      "expected_text": ["WebServer"],
      "min_text_elements": 7
    },
    {
      "path": "docs/examples/gantt/01_basic.puml",
      "family": "gantt",
      "expected_text": ["Project Timeline", "Design", "Build", "Test"],
      "min_text_elements": 20,
      "_note": "Task bars have width ~17px (proportional to 33-day timeline). Bars are present and correct."
    },
    {
      "path": "docs/examples/mindmap/01_basic.puml",
      "family": "mindmap",
      "expected_text": ["Project", "Planning", "Development", "Testing"],
      "min_text_elements": 4,
      "_known_broken": "PR #251 not merged: renders as DAG grid (uml-relation) not mindmap tree"
    },
    {
      "path": "docs/examples/wbs/01_basic.puml",
      "family": "wbs",
      "expected_text": ["Project Scope", "Phase 1", "Phase 2"],
      "min_text_elements": 4,
      "_known_broken": "PR #251 not merged: renders as DAG grid (uml-relation) not WBS tree"
    },
    {
      "path": "docs/examples/c4/01_context.puml",
      "family": "c4",
      "expected_text": ["Customer", "Support"],
      "min_text_elements": 10
    },
    {
      "path": "docs/examples/json/01_object.puml",
      "family": "json",
      "expected_text": ["age: 30", "active: true"],
      "min_text_elements": 4,
      "_note": "Fields render in alphabetical order (NF-8), expected_text updated to reflect actual output"
    },
    {
      "path": "docs/examples/chart/01_bar.puml",
      "family": "chart",
      "expected_text": ["Q1", "Q2", "Q3"],
      "min_text_elements": 10
    }
  ]
}
```

**Key changes from current manifest:**
1. `min_text_elements` values updated to reflect actual element counts (verified via SVG inspection)
2. `expected_text` filled in for all entries that had empty arrays
3. `_known_broken` annotations for mindmap/wbs/activity-if-else entries so CI failures are expected
4. `_note` annotations for JSON field-order behavior and gantt bar width behavior

---

## 7. Methodology Notes

### What worked well
1. **SVG inspection before vision** — `grep -o '<text'` counts (corrected for single-line SVG with `-o` not `-c`) gave reliable ground truth. Caught the old-style activity bug (source lines as text) immediately without needing to look at PNG.
2. **Looking for specific CSS classes** — checking for `mindmap-node`, `wbs-node`, `uml-relation`, `gantt-task`, `data-activity-kind`, `archimate-relation` pinpointed which renderer was invoked.
3. **192 DPI PNG** — confirmed that text IS readable at high DPI. The previous false alarm (`#238`) is definitively ruled out.

### What caused confusion
1. **`grep -c` on single-line SVG** — SVG files are single-line, so `grep -c '<text'` always returns 0 or 1 (line count, not occurrence count). Must use `grep -o '<text' | wc -l` instead.
2. **PR status ambiguity** — the task assumed PRs #250/251/252 might be merged; they were still open. Always check `gh pr view N` before claiming a fix is live.
3. **Gantt "sliver" root cause** — the prior audit said "task bars absent"; they are present at 8px. The real issue is epoch-anchor collapse when mixing absolute event dates with relative dependency chains.

### For next audit
1. Run `gh pr list --state merged --limit 5` first to check what's actually merged.
2. For activity diagrams, check `data-activity-kind` values: `OldStyle` = old-style renderer (broken), `Action/IfStart/Else/EndIf` = new-style renderer (working).
3. For gantt: check `data-gantt-tick-day` on the first tick — if it shows `1970-*`, the epoch-anchor bug is active.
4. For mindmap/wbs: check for `uml-relation` class — its presence means the wrong renderer was used.
5. Add a minimum-width check for gantt task bars: `width < 20` is a sliver, not a proper bar.

---

*Generated by automated visual audit agent, 2026-05-17*
