## Specialized + non-UML — Wave 7 audit (agent sonnet-4-6)

Audited 2026-05-18. 33 PNGs across GANTT, MINDMAP, WBS, JSON/YAML, CHART, SDL, SALT, ARCHIMATE, NWDIAG, TIMING, CHRONOLOGY, EBNF, REGEX, MATH, DITAA, PREPROCESSOR families.

---

### Per-PNG verdict table

| # | File | Status | Issues |
|---|------|--------|--------|
| 1 | gantt/01_basic.puml.png | PASS | Clean. Bars, milestone diamond, dashed dependency arrows, date header all correct. |
| 2 | gantt/02_milestones.puml.png | WARN | Milestone-only chart renders correctly; but "GA happens 2026-05-01" footer entry missing from legend (only Alpha/Beta/RC1 listed). |
| 3 | gantt/03_constraints.puml.png | PASS | All 5 tasks, dependency arrows, and header correct. No overlap. |
| 4 | gantt/04_dated.puml.png | WARN | "Beta release" milestone dot is cut off at right edge of grid — partially outside the chart area. |
| 5 | gantt/05_multi_task.puml.png | PASS | Mixed bar+arrow layout renders correctly. Dependencies shown as dashed arrows. |
| 6 | gantt/06_with_legend.puml.png | BUG | Gantt bars are very small (3 tasks squeezed into left ~20% of canvas); Phase 4 Launch milestone floats outside grid at far right. Canvas width poorly scaled to task durations. |
| 7 | mindmap/01_basic.puml.png | PASS | Root + 3 branches, correct colors, no clipping. |
| 8 | mindmap/02_multi_level.puml.png | PASS | 3-level tree, correct green/blue/yellow node colors. All text legible. |
| 9 | mindmap/03_with_colors.puml.png | PASS | Flat layout. All nodes visible and distinct. |
| 10 | mindmap/04_learning_map.puml.png | PASS | Balanced 4-branch map, all text fits. |
| 11 | wbs/01_basic.puml.png | PASS | Root + 3 leaf boxes, correct hierarchy lines. |
| 12 | wbs/02_with_tasks.puml.png | PASS | 3-column hierarchy. All labels legible. |
| 13 | wbs/03_checkboxes.puml.png | PASS | Checkbox notation [x] and [ ] rendered as plain text in boxes — acceptable fallback, consistent. |
| 14 | wbs/04_multi_level.puml.png | PASS | 4-level hierarchy with unbalanced branches. All nodes visible, no clipping. |
| 15 | json/03_nested.puml.png | PASS | Tree-indented rows, monospace font, correct nesting. (Flat list style per #528 already filed.) |
| 16 | yaml/02_sequence.puml.png | PASS | Array items with [0]/[1]/[2] indexing. Yellow background correct for YAML. |
| 17 | yaml/03_nested.puml.png | PASS | Nested database/credentials sub-objects rendered correctly. |
| 18 | chart/01_bar.puml.png | WARN | Bars and value labels correct; Y-axis label reads "Value" but chart title "Bar Chart" and axis label both render — consistent with #545 (chart family title/label issue partially resolved). |
| 19 | chart/02_line.puml.png | BUG | Data label "10" at origin overlaps "Jan" x-axis tick — already filed as #491. |
| 20 | chart/03_pie.puml.png | PASS | Three slices correct proportions; legend correct; labels inside slices legible. |
| 21 | chart/04_multi_series.puml.png | BUG | Source PUML defines multi-series but output renders as single-series bar chart (series B absent). Title reads "Bar Chart" not the multi-series title. NEW ISSUE. |
| 22 | sdl/02_with_transitions.puml.png | BUG | "Idle" start node is a solid black circle (UML start marker) not an SDL process rectangle/lozenge — already filed as #496. Additionally "retry" label on Waiting->Idle edge is clipped ("re▪y" visible). NEW CLIPPING ISSUE. |
| 23 | salt/01_basic_widgets.puml.png | PASS | Name/Age fields and OK/Cancel buttons render correctly. |
| 24 | salt/04_tabs.puml.png | PASS | Tab bar and content area visible. Minor: tab selector shows "{/ Tab1 | Tab2 | Tab3 }" with curly brace artifact but this may be intentional SALT notation. |
| 25 | archimate/02_with_relations.puml.png | PASS | Two-layer diagram (business/application), assigned arrow, element labels correct. Layer colors use yellow/blue (Archimate standard #529 for wrong colors filed for other families, these look OK). |
| 26 | nwdiag/02_multiple_nets.puml.png | BUG | Nodes render as flat indented list items inside network band, not as boxes on horizontal network bar — already filed as #505. The "lb" node appearing in both networks has no visual link between them. |
| 27 | timing/04_binary.puml.png | BUG | Canvas is cut off at right — "@40" tick mark and Ack signal trailing state partially clipped. Additionally "timing diagram" caption appears in small blue text top-left instead of a title — already related to #506. |
| 28 | chronology/02_timeline.puml.png | PASS | Vertical timeline, three dated entries with blue dots, event labels legible. Clean. |
| 29 | ebnf/02_optional_repetition.puml.png | BUG | factor rule is truncated: "(" token railroad symbol cut off at bottom — already filed as #510. |
| 30 | regex/02_repetition.puml.png | BUG | Title bar "| b+ | c? | d{3} | e{2" — right edge cut off at canvas boundary — already filed as #514. |
| 31 | math/02_complex.puml.png | PASS | Gaussian integral formula renders correctly with proper sqrt symbol and fraction. |
| 32 | ditaa/02_components.puml.png | BUG | Node labels render with spaces between every character ("A u t h", "A p p", "D B") — already filed as #523. |
| 33 | preprocessor/04_function.puml.png | BUG | Arrow label shows raw macro expansion artifacts: "Alice\n\"Alice\" + \" calls \"\n+ \"Bob\"" as multi-line text above the arrow. The `!function` return value is not being interpolated cleanly — concatenation seams leak into the rendered label. NEW ISSUE. |

---

### FIXED candidates (previously filed, now passing)

- gantt/01_basic — Clean render, dependencies correct.
- mindmap family (all 4) — All pass; #550 and #547 (right-leaf clipping) not reproduced in these examples.
- wbs family (all 4) — All pass.
- yaml/json — All pass.
- archimate/02_with_relations — Layer colors, arrows correct for this example.
- chronology/02_timeline — Clean.
- math/02_complex — Clean.
- salt/01_basic_widgets and 04_tabs — Clean.
- chart/01_bar — Substantially better (title and Y-label present, contra #545).
- chart/03_pie — Clean.

---

### Still-present (confirmed in this wave)

| Issue # | Family | Observation |
|---------|--------|-------------|
| #491 | chart | Line chart "10" label overlaps "Jan" tick |
| #496 | sdl | Idle renders as UML start circle not SDL shape |
| #505 | nwdiag | Node list instead of topology bar |
| #506 | timing | Canvas clipping at right edge |
| #510 | ebnf | factor rule bottom truncated |
| #514 | regex | Title bar right-edge cut off |
| #523 | ditaa | Spaced character labels |

---

### New issues filed in this wave

| Issue # | Family | Description |
|---------|--------|-------------|
| #576 | gantt | 06_with_legend: task bars poorly scaled, Phase 4 milestone outside grid |
| #577 | sdl | 02_with_transitions: "retry" edge label clipped to "re▪y" |
| #580 | chart | 04_multi_series: second series absent, single-series bar rendered |
| #582 | preprocessor | 04_function: macro concatenation seams leak into arrow label as multi-line raw expression |

---

### Top systemic issues

1. **Canvas auto-sizing** — Multiple families (gantt, timing, regex, ebnf) clip content at right or bottom edge. The viewport calculation does not account for all rendered elements.
2. **SDL shape mapping** — SDL start/stop nodes use generic UML state shapes rather than SDL-specific process rectangles/lozenges/terminators.
3. **Multi-series chart** — `04_multi_series.puml` with two declared series renders only one; series grouping logic absent.
4. **Preprocessor macro expansion** — `!function` return values are not fully interpolated before label rendering; concatenation artifacts appear.
5. **NWDiag topology** — Network nodes render as list items; the horizontal bus-bar topology is not implemented.

---

## Sequence + class — Wave 7 audit (agent claude-sonnet-4-6, batch 2)

Audited 2026-05-18. 37 PNGs examined (21 sequence + 16 class). 1 sequence file missing from disk (15_large_diagram).

### Verdict by PNG

| file | status | observation | known issue # | new issue # |
|------|--------|-------------|---------------|-------------|
| sequence/01_basic | ✓ | Clean. Solid forward arrow, dashed return, labels clear. | — | — |
| sequence/02_participants | ✓ | box, actor, database, cylinder, queue participant shapes all correct. All arrow labels clear. | — | — |
| sequence/03_autonumber | ✓ | Autonumbers 1–4 prefix messages. Labels above shaft, no overlap. | — | — |
| sequence/04_autonumber_format | ✓ | Format 10–13 applied. Forward + return both numbered correctly. | — | — |
| sequence/05_alt_opt_loop | ✓ | alt/opt/loop group boxes render with correct label tabs, dashed else-divider. All labels clear. | — | — |
| sequence/06_par | ✓ | par/else group box renders correctly. Arrows stay within partitions. | — | — |
| sequence/07_notes | ✓ | Yellow sticky notes correctly attached to lifelines and floating. No text overlap. | — | — |
| sequence/08_ref | ⚠ | 'session token' return label visually collides with lower border of the ref box frame. | #535, #503 | #561 |
| sequence/11_activation | ⚠ | Bob activation bar absent; Carol bar present but slightly misplaced vs 'result' arrowhead. | #489 | #563 |
| sequence/12_create_destroy | ✓ | create (green circle endpoint), destroy (red X) markers correct. Dashed lifeline for Session. | — | — |
| sequence/13_arrows | ✓ | All 7 arrow variants (sync, async, dashed, lost, open, bidirectional) render with correct heads. | — | — |
| sequence/14_separator | ✓ | Two == separator == dividers render as full-width solid lines with centered bold text. | — | — |
| sequence/15_large_diagram | ✗ | FILE MISSING from disk — not rendered. | — | — |
| sequence/16_arrow_variants | ✓ | All 10 variants (reverse sync/thin, circle/cross endpoints, bidirectional) correct. Title present. | — | — |
| sequence/17_all_groups | ⚠ | 'abort' self-message in break block renders as open rectangle with no arrowhead, not a self-loop. | #498 | #566 |
| sequence/18_activation_stack | ⚠ | 'self call' on B renders as small open square, not a UML self-loop. 'done' dashed return also malformed as open rect. | #498 | — |
| sequence/19_lifecycle | ✓ | spawn (green circle), destroy (red X), activation bar on Worker, processing label all correct. | — | — |
| sequence/22_ref_over | ⚠ | 'response' arrow label overlaps/emerges from lower-left corner of ref box spanning Alice/Bob/Charlie. | #535, #503 | #562 |
| sequence/23_dividers | ⚠ | 'cleanup' self-call renders as open rectangle with no arrowhead. Three == dividers correct and bold. | #498 | #568 |
| sequence/26_theme_aws | ✓ | AWS theme: orange boxes, orange dashed lifelines. Labels clear, arrows correct. | — | — |
| sequence/37_theme_sketchy | ⚠ | Sketchy theme applied to participant boxes (rounded/cream) but arrows remain crisp — no hand-drawn waviness. | #518 | — |
| sequence/44_theme_mono | ✓ | Mono/grayscale theme: grey participant boxes, monochrome treatment. All labels legible. | — | — |
| class/01_basic | ✓ | Animal + Dog each have 2 compartments; 'owns' association arrow. Members legible. (#544 appears fixed.) | #544 fixed | — |
| class/02_inheritance | ✓ | Vehicle←Car and Vehicle←Truck hollow-triangle inheritance arrows. Members correct. | — | — |
| class/03_composition_aggregation | ⚠ | 'contains' label clips into House–Room box border; 'may have' clips Room–Furniture border. | #469, #554 | — |
| class/05_visibility | ⚠ | Visibility color coding present; compartment divider between attributes/methods absent; sigils (+/-/#) absent from some rows. | #552, #516 | — |
| class/06_abstract_interface | ✓ | Shape (abstract), Circle, Rectangle — hollow-triangle arrows. Abstract methods in italic. Clean. | — | — |
| class/07_stereotypes | ⚠ | 'delegates' and 'persists' edge labels clipped by box borders. | #554 | — |
| class/08_packages | ✓ | 5 classes in flat package grouping. Arrows with correct filled/open heads. | — | — |
| class/10_full_domain | ⚠ | 'reference' label on OrderItem→Product partially clipped by box border. Dense layout, no outright text overlap. | #469 | — |
| class/11_generics | ✓ | Container<T>, Stack<E>, Map<K,V> — generic params in header correct. Members legible. | — | — |
| class/12_all_relations | ⚠ | 'association' label clips into box A border; 'aggregation (F belongs to F)' truncated at canvas right edge. Double-headed association also present. | #469, #471, #521 | — |
| class/14_nested_packages | ⚠ | 'package repository' label overlaps with 'service::UserService'/'service::ProductService' node text inside nested package. | — | #570 |
| class/15_enum_annotation | ✓ | Status/Priority enums and Task class render correctly. No compartment divider for enum (acceptable). | — | — |
| class/17_pattern_observer | ✓ | Observer pattern: hollow-triangle and association arrows correct. All 5 classes legible. | — | — |
| class/21_microservices | ⚠ | Gateway box member '+authenticate(token): boolea' clipped at right edge — trailing 'n' cut off. | #469, #514 | #572 |
| class/22_ddd | ✓ | Order aggregate root, OrderItem, OrderId, Money — composition and association arrows correct. | — | — |
| class/30_command_pattern | ✓ | Command, Invoker, Receiver, ConcreteCommand — inheritance and association arrows correct. | — | — |

### Issues confirmed FIXED (recommend closing)

- #544 — class/01_basic now renders two compartments (attributes + methods) per box. Animal shows `+name: String`, `+age: Int`, `+speak()` and Dog shows `+breed: String`, `+fetch()`. Prior waves reported name-only boxes.

### Issues still present

- #498 — Self-call arrows: confirmed still broken in 18_activation_stack (open square), 17_all_groups abort in break block (open rect), 23_dividers cleanup (open rect).
- #489 — Activation bars: confirmed partially broken in 11_activation (Bob bar absent).
- #518 — Sketchy theme: arrows/lifelines remain crisp in 37_theme_sketchy — no hand-drawn waviness applied.
- #535 / #503 — ref fragment: header row still missing; label collisions at box edges in 08_ref and 22_ref_over.
- #469 / #554 — Class edge label clipping: still present in 03_composition_aggregation, 07_stereotypes, 10_full_domain, 12_all_relations.
- #521 — 12_all_relations: 'aggregation (F belongs to F)' label still truncated at canvas right edge.
- #552 — class/05_visibility: compartment divider between attributes/methods still missing.
- #516 — class/05_visibility: visibility sigils (+/-/#) still absent from member text.

### NEW issues filed during this audit

- #561 — [P1][sequence] 08_ref: 'session token' return label collides with ref box lower border
- #562 — [P2][sequence] 22_ref_over: 'response' arrow label overlaps lower-left corner of ref box
- #563 — [P1][sequence] 11_activation: Bob activation bar absent; Carol bar misplaced
- #566 — [P2][sequence] 17_all_groups: 'abort' self-message in break block renders as open rectangle, no arrowhead
- #568 — [P2][sequence] 23_dividers: 'cleanup' self-call renders as open rectangle, no arrowhead
- #570 — [P2][class] 14_nested_packages: 'package repository' label overlaps service:: node text
- #572 — [P1][class] 21_microservices: Gateway member text '+authenticate(token): boolea' clipped at box right edge

### Top systemic concerns

1. **Self-call arrow geometry** — Every diagram with a self-message (A -> A) produces an open rectangular shape instead of the standard three-segment UML self-loop with arrowhead. Reproduced in 17_all_groups, 18_activation_stack, 23_dividers. Root cause: self-source/self-target edge case not handled in routing.
2. **Class edge label placement** — Labels on associations float too close to (or inside) adjacent box borders. Affects 03_composition, 07_stereotypes, 10_full_domain, 12_all_relations. Label anchor not correctly offset from box edges.
3. **ref fragment label routing** — ref box does not reserve space between its lower border and next outgoing message label. Affects 08_ref and 22_ref_over identically.
4. **Activation bar completeness** — Implicit activation (no explicit `activate`/`deactivate`) does not reliably produce bars. 11_activation shows Bob with zero bar.
5. **Sketchy theme arrow style** — Rounded-corner styling applied to participant boxes but arrows remain crisp. Hand-drawn jitter not implemented for arrows/lifelines.

---

## State + activity + themes/skinparams — Wave 7 audit (agent sonnet-4-6 / run-3)

**Date:** 2026-05-18  
**Scope:** state (8), activity new (8), activity_new dir + activity_old (4), themes (6), skinparams (6) — 32 PNGs total  
**Auditor:** claude-sonnet-4-6

---

### Per-PNG verdict table

| # | File | Status | Notes |
|---|------|--------|-------|
| 1 | state/01_basic.puml.png | WARN | No diagram title or type header — #541 (known) |
| 2 | state/02_transitions.puml.png | FAIL | "approved()" label doubled/overlapping arrowhead; submit() crowds arrow crossing — #483 (known) |
| 3 | state/03_concurrent.puml.png | FAIL | Dashed region divider absent; transitions missing into substates; initial pseudo-state unconnected inside composite — #555 (known P0) |
| 4 | state/04_history.puml.png | FAIL | "resume/pause" label text overlaps arrowhead; H* label partially clipped by adjacent arrow — #483 scope |
| 5 | state/05_fork_join_choice.puml.png | FAIL | Choice renders as open square not diamond; "error" label floats disconnected mid-canvas — #556 (known) |
| 6 | state/06_entry_exit.puml.png | PASS | Entry/exit compartment list correct; "timeout" label well-spaced |
| 7 | state/07_nested.puml.png | WARN | Initial pseudo-state arrows connect outside composite box; "shutdown" edge label close but readable |
| 8 | state/08_full_machine.puml.png | FAIL | "in stock/confirm" label text runs together (missing space); "stock"/"packed"/"delivered" overlap crossing arrows; Delivered→end arrow exits below canvas — NEW |
| 9 | activity/01_simple_flow.puml.png | PASS | Start/end correct; activities well-spaced; no overlaps |
| 10 | activity/02_if_then_else.puml.png | WARN | "Return 200"/"Return 401" share same horizontal row; merge dog-leg has no explicit join node |
| 11 | activity/03_nested_if.puml.png | WARN | Three-branch merge converges at bare point with no join node; two bare arrow stubs before end node |
| 12 | activity/04_fork_join.puml.png | PASS | Fork/join bars present; all branches correctly routed |
| 13 | activity/05_while_loop.puml.png | FAIL | `(endwhile)` rendered as literal rounded-rect process node instead of loop back-edge annotation — NEW |
| 14 | activity/06_repeat_until.puml.png | FAIL | `(repeat)` and `repeat while` both rendered as literal process nodes; back-edge condition not shown as edge label — NEW |
| 15 | activity/07_partition.puml.png | WARN | `partition` keyword in source; no visible lane borders or headers drawn — partition boundaries absent — NEW (distinct from #501) |
| 16 | activity/12_deployment.puml.png | FAIL | "Rollback" node duplicated (two instances with crossing lines); three dangling end arrows below Done — fork logic broken — NEW |
| 17 | activity_new/03_fork.puml.png | WARN | No diagram title (only "activity diagram" subtitle shown); fork/join bars and branches correct |
| 18 | activity_new/06_partition.puml.png | FAIL | Start dot overlaps "Backend" lane header (#543 known); "render" node in Frontend lane has no connecting arrows to Backend flow — cross-lane edges missing — NEW |
| 19 | activity_old/02_swimlanes.puml.png | FAIL | Start dot overlaps "[Build]" lane header (#543 known); no arrows connecting nodes across lanes — cross-lane edges missing — NEW |
| 20 | activity_old/03_colored.puml.png | PASS | Linear flow correct; no overlaps |
| 21 | themes/01_plain.puml.png | FAIL | Message label "Hello with plain theme" word-wraps and overlaps dashed lifeline — #511 (known P0) |
| 22 | themes/03_plain_sequence.puml.png | PASS | Title + three-participant sequence clean; no overlaps |
| 23 | themes/05_plain_class.puml.png | PASS | Two-class inheritance; compartments and arrow clean |
| 24 | themes/06_spacelab_state.puml.png | WARN | Spacelab blue-gray palette confirmed; "start"/"stop" labels crowd arrow intersection — #520 (known) |
| 25 | themes/10_spacelab_box.puml.png | PASS | Spacelab palette on sequence; no overlaps |
| 26 | themes/theme_sunlust.puml.png | FAIL | `else error` combined-fragment header overflows divider strip into body; self-message arrows ("log success"/"log error") lack visible arrowheads — NEW |
| 27 | skinparams/03_note_colors.puml.png | FAIL | Note floats disconnected with no tether line — #524 (known P1) |
| 28 | skinparams/08_combined.puml.png | PASS | Pink theme applied; note offset but recognizable; labels clear |
| 29 | skinparams/13_note_styles.puml.png | FAIL | Multi-line note floats disconnected — #527 (known P1) |
| 30 | skinparams/16_all_colors.puml.png | PASS | Dark cinematic theme; styled contrast acceptable |
| 31 | skinparams/17_minimal.puml.png | PASS | Minimal two-participant sequence; clean |
| 32 | skinparams/18_corporate.puml.png | PASS | Corporate navy/blue theme; "DB" label slightly tight but readable |

**Totals: PASS 10 / WARN 7 / FAIL 15**

---

### Known issues confirmed still-present

| # | File(s) | Confirmed |
|---|---------|-----------|
| #541 | state/01_basic | No title — still present |
| #483 | state/02, 04 | Label/arrowhead overlap — still present |
| #555 | state/03_concurrent | Dashed divider/transitions absent — still present (P0) |
| #556 | state/05_fork_join_choice | Choice square + tangled arrows — still present |
| #511 | themes/01_plain | Message label overlaps lifeline — still present (P0) |
| #524 | skinparams/03_note_colors | Note disconnected — still present |
| #527 | skinparams/13_note_styles | Note disconnected — still present |
| #520 | themes/06_spacelab_state | Label crowding at crossing — still present |
| #543 | activity_new/06_partition + activity_old/02_swimlanes | Start dot overlaps lane header — still present |

---

### New issues filed this wave

| Issue # | File(s) | Description |
|---------|---------|-------------|
| #583 | state/08_full_machine | "in stock/confirm" label space missing; "stock"/"packed"/"delivered" overlap crossing arrows; end arrow exits canvas |
| #584 | activity/05_while_loop | `(endwhile)` rendered as literal process node instead of loop back-edge annotation |
| #585 | activity/06_repeat_until | `(repeat)` + `repeat while` rendered as literal nodes; back-edge condition absent |
| #586 | activity/07_partition | `partition` keyword: no lane borders or headers rendered (activity-new family) |
| #587 | activity/12_deployment | "Rollback" duplicated; dangling end arrows — fork/join logic broken |
| #588 | activity_new/06_partition + activity_old/02_swimlanes | Cross-lane edges absent — swimlane nodes unconnected across lanes |
| #589 | themes/theme_sunlust | `else error` header overflows combined fragment body; self-message arrowheads absent |

---

### Top systemic issues

1. **Activity loop keywords rendered as literal nodes** — `(endwhile)`, `(repeat)`, `repeat while` appear as process boxes rather than being consumed as control-flow back-edge annotations.
2. **Cross-lane edge routing absent in swimlane diagrams** — Both activity_new/06_partition and activity_old/02_swimlanes produce lane nodes with no connecting arrows between lanes.
3. **Fork/join deduplication failure** — "Rollback" in activity/12_deployment appears twice with tangled routing and dangling end arrows.
4. **State edge labels crowd crossing arrows** — Systemic in state/02, 04, 07, 08 (broad scope of #483).
5. **Note tether lines absent** — Systemic across skinparam family; notes render as disconnected floating islands.
6. **Combined fragment `else` header overflow** — `theme_sunlust` exposes divider strip too narrow for `else error` label text.

---

## Structural UML + creole + arch — Wave 7 audit (agent sonnet-4-6, batch 3)

Audited 2026-05-18. 27 PNGs: usecase (4), object (4), component (5), deployment (4), C4 (5), creole (2), arch fresh renders (arch1/arch4/arch5). Render warnings: arch1 unsupported skinparam `packageStyle`; arch3 unsupported `classAttributeIconSize` — both benign, diagrams render.

---

### Per-PNG verdict table

| # | File | Status | Issues |
|---|------|--------|--------|
| 1 | usecase/01_basic.puml.png | PASS | Clean. Stick-figure actor, two ellipses, "leads to" label readable. |
| 2 | usecase/02_with_actors.puml.png | WARN | "leads to" edge label overlaps Admin actor body. Filed #574. |
| 3 | usecase/03_extends_includes.puml.png | WARN | `<<extend>>` and `<<include>>` labels overlap near ApplyCoupon ellipse. Filed #575. |
| 4 | usecase/04_with_packages.puml.png | BUG | (1) 'rectangle' keyword leaks — #553 confirmed. (2) Package labels at bottom not top-left tab. (3) Ellipses show namespace-prefixed names ("Online Store::Browse", "Back Office::MP"). Filed #578. |
| 5 | object/01_basic.puml.png | PASS | Clean. Two boxes, "knows" label readable. |
| 6 | object/02_with_attributes.puml.png | WARN | "placedBy" label truncated by Customer box border. Filed #564. |
| 7 | object/03_with_links.puml.png | WARN | "connects" label truncated by Database box border. Filed #564. |
| 8 | object/04_with_stereotypes.puml.png | WARN | "hasSession" truncated to "haSessi..." by mySession box border. Filed #564. |
| 9 | component/01_basic.puml.png | BUG | "Backend" component box clipped at canvas right edge. Filed #565. |
| 10 | component/02_interfaces.puml.png | BUG | "REST" circle, "provides" and "interface" labels clipped at right canvas edge. Filed #565. |
| 11 | component/03_packages.puml.png | PASS | Clean two-package layout, no clipping. |
| 12 | component/05_with_notes.puml.png | BUG | "UserService" truncated to "UserServ..." at right canvas edge. Filed #565. |
| 13 | component/06_with_arrows.puml.png | BUG | "calls"/"uses"/"compose..." labels and arrowheads cluster and overlap on B; B itself clipped. Filed #567. |
| 14 | deployment/01_nodes.puml.png | BUG | "AppServer" clipped; "HTTP" label collides with box border; nodes flat not 3D. Filed #569, #571. |
| 15 | deployment/02_databases.puml.png | BUG | "PostgreSQL" clipped; "caches" label cut. Database cylinder correct. Filed #569. |
| 16 | deployment/03_cloud.puml.png | BUG | "RDS Instance" clipped; "stores" and "reads..." labels cut. Filed #569. |
| 17 | deployment/04_mixed.puml.png | WARN | "Load Balancer" and "Primary" database clipped at right. Title renders correctly. Filed #569. |
| 18 | c4/01_context.puml.png | WARN | "Manages" overlaps Support actor; "Transfers funds" partially clipped. Filed #579. |
| 19 | c4/03_containers.puml.png | WARN | "sends vi..." label on Worker to Email edge truncated. Filed #579. |
| 20 | c4/04_components.puml.png | PASS | Clean. All labels readable. |
| 21 | c4/07_microservices.puml.png | PASS | Dense, all containers visible. |
| 22 | c4/10_security_zones.puml.png | WARN | "HTTPS" overlaps Threat Actor; "API call" clipped at App Tier border. Filed #579. |
| 23 | creole/01_bold_italic.puml.png | PASS | Bold, italic, underline, strikethrough all correct. No tag leakage. |
| 24 | creole/02_color_size.puml.png | BUG | P0: `<color:red>` and `<size:18>` closing tags render as literal text. Filed #573. |
| 25 | arch1 (architecture-overview.puml) | BUG | Central routing area has severe label overlap. #557 confirmed still present. |
| 26 | arch4 (diagram-family-lifecycle.puml) | BUG | (1) Composite state boxes absent — #558 confirmed. (2) NEW: edge labels overlap node text. Filed #581. |
| 27 | arch5 (parity-status.puml) | BUG | Right-side leaf nodes clipped — #547/#550 confirmed. |

---

### FIXED candidates (newly passing in this audit)

- component/03_packages: clean, no clipping
- object/01_basic: clean
- c4/04_components: clean, all labels readable
- c4/07_microservices: substantially clean despite density
- creole/01_bold_italic: all Creole text styles correct

---

### Still-present (confirmed)

| Issue # | Family | Observation |
|---------|--------|-------------|
| #553 | usecase | 04_with_packages: 'rectangle' keyword leaks as literal text |
| #547/#550 | mindmap/arch | arch5/parity-status: right-leaf clipping confirmed |
| #557 | arch/component | arch1: dense edge label overlap confirmed |
| #558 | state | arch4: composite state boxes absent confirmed |

---

### New issues filed

| Issue # | Priority | Family | Description |
|---------|----------|--------|-------------|
| #564 | P1 | object | Edge label clipped by adjacent object box border (02, 03, 04) |
| #565 | P1 | component | Rightmost component box clipped at canvas edge (01, 02, 05) |
| #567 | P1 | component | 06_with_arrows: label/arrowhead collision on component B |
| #569 | P1 | deployment | Deployment node boxes clipped at right canvas edge (01-04) |
| #571 | P2 | deployment | Nodes render as flat rectangles, no 3D cube UML notation |
| #573 | P0 | creole | HTML color/size tags render as literal text |
| #574 | P2 | usecase | 02_with_actors: "leads to" label overlaps Admin actor body |
| #575 | P1 | usecase | 03_extends_includes: extend/include labels overlap |
| #578 | P2 | usecase | 04_with_packages: package labels at bottom not top-left tab; namespace-prefixed names |
| #579 | P1 | c4 | Context/security-zones: edge labels clipped or overlap actors |
| #581 | P1 | state/arch | arch4: state transition edge labels severely overlap node text |

---

### Top systemic issues (structural UML wave)

1. **Canvas right-edge clipping** — Pervasive across component, deployment, C4, arch. Canvas width not computed from full rightmost-element bounding box. Trackers: #565, #569, #579.
2. **Edge label clearance from element borders** — Labels placed at arc midpoints with no clearance from box/actor edges. Affects object, usecase, C4, deployment, arch. Trackers: #564, #574, #575, #579, #581.
3. **Creole HTML tag parsing incomplete** — `<color:X>` and `<size:N>` inline markup unrecognised; closing tags leak as literal text. Tracker: #573.
4. **Deployment 3D cube shape absent** — UML spec requires nodes as 3D cubes; current render uses flat rectangle identical to packages. Tracker: #571.
5. **Package/boundary label position** — Package labels appear at bottom of boundary rectangles instead of UML top-left tab. Tracker: #578.

---

## Post-Wave-15 audit (agent claude-sonnet-4-6)

Audited 2026-05-18. Corpus regenerated fresh: 322/322 OK. Commit 2896c3710085e54e87b0c4557669292e9dc8534f. 30 PNGs reviewed across ARCH (5), CLASS/OBJECT/USECASE (7), COMPONENT/DEPLOYMENT/C4 (3), SEQUENCE/STATE/ACTIVITY (6), SPECIALIZED (5), plus arch self-host diagrams (5).

---

### Per-PNG verdict table

| # | File | Status | Observations |
|---|------|--------|--------------|
| 1 | arch: architecture-overview.png | WARN | Overall layout correct; 3 nested packages render cleanly (Transports, Frontends, Pipeline Core, Output Formats). Arrow-through-box issue (#539) partially improved: most arrows now route around nodes, but the dense multi-arrow convergence at top-centre (CLI Binary → Parser region) still shows several arrows crossing the "package Pipeline Core" title bar. Edge label overlap (#540) substantially reduced but not eliminated: "source text" and "expanded source" labels still crowd near the Frontends-to-Pipeline boundary. Package label tab positions correct (top-left). |
| 2 | arch: pipeline-sequence.png | PASS | All 7 lifelines present and correctly labelled. Alt/opt/loop combined fragments render cleanly with solid top-left labels and dashed dividers. Activation boxes on Normalizer and Renderer correct. Self-return arrows (resolve/apply) render as proper loopback arcs not open rectangles. Minor: "build AST nodes" step at Parser has very small horizontal extent — almost a zero-width return but readable. |
| 3 | arch: language-service-layers.png | WARN | Wide class diagram (two package containers). Text is very small due to extreme aspect ratio; all class names legible at full resolution. Compartment dividers and stereotypes present and correct. Connectors between classes route correctly. Main concern: the diagram is wider than tall at ~15:1 ratio, making it nearly unreadable without zoom. Not a rendering bug per se but a layout issue for documentation use. |
| 4 | arch: diagram-family-lifecycle.png | BUG | Confirmed #581 still open: state-machine edge labels severely overlap node text. "Preprocessor expands includes and macros" label collides with both the "Tokenized" state and "Parsed" state borders. "Parser builds AST" partially obscures "Parsed". "Engine applies s..." (truncated label) overlaps "Normalized". The initial transition arrow from the start dot fires two simultaneous arrows (one to "Source", one crossing behind "Tokenized") — ambiguous routing. |
| 5 | arch: parity-status.png | PASS | Mind-map style parity overview renders cleanly. Color coding (green/red/orange/blue nodes) correct. All branch labels legible. Node sizes proportionate to label lengths. No clipping. |
| 6 | class/01_basic.puml.png | PASS | Animal/Dog class boxes with header, attribute compartment, method compartment — all dividers present and correct (previously #468 P0 — now fixed). Inheritance arrow is hollow triangle (previously #467 P0 — now fixed). "owns" edge label well-spaced. Attribute text in green (#467 fix confirmed). |
| 7 | class/12_all_relations.puml.png | WARN | Title "Relation Arrow Types in Class Diagrams" correct. Hollow triangle for A→B (inheritance) confirmed fixed. Composition (filled diamond at C) correct. Aggregation (open diamond at E) correct. Bug: A→B has two overlapping labels: "«extends»" and "association" are printed on top of each other at the midpoint of the arrow. One is the stereotype label, one is the relation label — they should be on opposite sides of the arrow or de-collided. Not previously filed; new issue candidate. |
| 8 | class/14_nested_packages.puml.png | WARN | Confirmed #570 still open: the diagram renders all 7 nodes in a single flat horizontal strip inside three dashed package boxes. Packages "domain", "service", "repository" each show their label in the top-left correctly. However the layout is entirely horizontal with no vertical spacing — all nodes at the same Y. Cross-package arrows are horizontal connecting lines with arrowheads. Readable but extremely wide (2200px+). The inner label overlap (#570) is not visible in this sample — labels fit their boxes here. |
| 9 | class/21_microservices.puml.png | WARN | Confirmed #572 still open: Gateway class box member text "authenticate(token): boolean" is truncated to "authenticate(token): boolea" — cut at the box right boundary. All other classes lack member compartments (empty boxes with just the name) which is correct since they have no declared attributes. Arrow routing is clean tree structure. |
| 10 | object/03_with_links.puml.png | PASS | Server/Cache/Database object boxes with yellow background, underlined names, attribute lists. "uses" and "connects" edge labels correctly placed away from node borders. Arrows have proper filled arrowheads. Previously #564 (edge label clipped by box border) — not reproduced here; this sample passes. |
| 11 | usecase/02_with_actors.puml.png | WARN | Actor stick figures correctly rendered (previously #475 P0 — now fixed). Use-case ellipses present. "leads to" (#574) label confirmed still overlapping the Admin actor body — label baseline sits inside the stick-figure torso area. "triggers" label also slightly close to BrowseProducts ellipse but readable. Arrows route with right-angle bends (staircase routing) rather than straight lines — acceptable but not ideal for use-case diagrams which conventionally use straight connectors. |
| 12 | usecase/04_with_packages.puml.png | WARN | Actor stick figures present (post #475 fix). Package boundary boxes rendered (post #479 fix). Confirmed #578 still open: "Back Office" and "Online Store" package labels appear at top-left inside the border — actually this looks correct in the current render; the label IS at top-left. However, the diagram renders all elements in a single flat horizontal strip — BackOffice left, two actors center, OnlineStore right — producing an extremely wide (2144px) very short (608px) canvas with nearly all elements at the same vertical position. Actor lines connect horizontally across the diagram at near-zero angle, making the two double-headed arrows ("depends" and the double-arrow to AddToCart/Browse) look like a single thick horizontal line. This layout is essentially unreadable. |
| 13 | component/06_with_arrows.puml.png | WARN | Confirmed #567 still open: "uses" dashed arrow label and "composed" plain label render on top of each other between A→C. "calls" label on A→B is placed correctly. The <<extend>> label on D→B is well-spaced. Component box shapes (with port stubs) are correct. The overlap is specifically the two labels from A going to C — only one label is clearly visible. |
| 14 | deployment/04_mixed.puml.png | PASS | Substantially improved: CDN, Load Balancer, Web Server 1/2, Primary DB (cylinder), Replica DB (cylinder), Cache (artifact/folded-corner shape) all render correctly with their UML stereotypes ("node", "database", "artifact"). Node boxes no longer clipped at right (#569 appears resolved for this sample — need to verify 01-03 separately). 3D cube issue (#571) still present — nodes are flat rectangles not cubes, but this is a known open issue. Cylinder shapes for databases are a positive new development. |
| 15 | c4/07_microservices.puml.png | PASS | Person actor (Client) renders as blue stick figure. Container boxes (API Gateway, Order/User/Product Services) with blue fill and [Container] label. External system boxes (Order DB, User DB) in grey with [System, ext]. Edge labels ("API calls", "Routes", "Reads/writes") all correctly placed without overlap. Previously #579 (C4 edge labels clipped/overlap) — not reproduced in this sample; passes. Good improvement. |
| 16 | sequence/05_alt_opt_loop.puml.png | PASS | Alt/opt/loop combined fragments with bold top-left keyword labels and dashed dividers for "else" clauses. Filled arrowheads for sync calls, dashed-line open-head for returns. Actor boxes at top and bottom. Sequence reads correctly. Clean. |
| 17 | sequence/11_activation.puml.png | PASS | Three lifelines (Alice, Bob, Carol). Activation boxes (narrow rectangles on lifeline) on Bob for the sub-call duration — correct UML activation notation. Arrow routing correct. "response" return dashes correctly cross Carol's lifeline. Clean. |
| 18 | state/07_nested.puml.png | WARN | Composite state "Operational" contains sub-states "Idle" and nested composite "Working" (containing "Fetching" and "Processing"). Nesting renders correctly with rounded rectangle containment. Bug: the initial transition from the filled-circle start node fires TWO arrows simultaneously — one going UP-LEFT toward "shutdown" label and the initial "into Operational" arrow going right. The "shutdown" arrow (which should exit Operational back to the terminal) appears to originate from the initial dot, making the diagram look like the start state has a shutdown self-loop. This is an edge routing artefact: the final/terminate transition is sharing the same visual origin as the initial. Not previously filed at this specificity. |
| 19 | state/03_concurrent.puml.png | WARN | Same double-arrow-from-start-dot issue as state/07: the start node fires two arrows, one of which is labelled — but in this case the dashed concurrent-region separator inside "Processing" is correctly rendered (horizontal dashed line dividing Parsing from Validating/Logging/Auditing). Concurrent regions work. The start-arrow ambiguity is the only flaw. |
| 20 | activity/04_fork_join.puml.png | PASS | Fork bar (thick horizontal bar) and join bar both render correctly. Three parallel branches (Subtask A/A2, Subtask B, Subtask C) correctly splayed. "Merge Results" and terminal state correct. One minor note: Subtask C's convergence arrow goes diagonally to the join bar — slightly off-orthogonal but readable. Previously passing, still passing. |
| 21 | activity/07_partition.puml.png | WARN | Partition swimlanes (Worker, Backend, Frontend) render with column headers. Bug: start node (filled circle) overlaps the "Worker" column header — the dot is positioned on top of the column label text. The flow then routes correctly through the swimlanes. Cross-lane arrows (Worker→Backend, Backend→Frontend, Frontend→Worker for terminal) all draw correctly. Previously filed as #588 (cross-lane arrow overlap) — the header overlap is related but distinct. |
| 22 | math/02_complex.puml.png | PASS | Gaussian integral formula renders correctly: integral sign, e^(-x²) term, dx, equals sign, sqrt(pi)/2 fraction all correct. Previously passing; still passing. |
| 23 | gantt/01_basic.puml.png | PASS | Project Timeline: Design/Build/Test bars with correct date-based widths. Kickoff milestone diamond correctly placed at 2026-01-01. Dashed dependency arrows connecting bars. Date header row correct. Previously passing; still passing. |
| 24 | mindmap/02_multi_level.puml.png | PASS | Technology Stack mindmap: root (yellow), two primary branches (Frontend/Backend in blue, DevOps in blue), leaf nodes (green). Correct radial layout. All text legible. Previously passing; still passing. |
| 25 | chart/01_bar.puml.png | PASS | Bar Chart: Q1/Q2/Q3/Q4 bars with correct values (42/58/73/91). Y-axis gridlines, axis labels ("Value", "Category"), and value labels above bars all correct. Title "Bar Chart" present. Previously WARN (Wave 7); now fully passing. |
| 26 | ditaa/02_components.puml.png | BUG | Confirmed #523 still open: "Auth", "App", "DB" labels still render with spaces between every character ("A u t h", "A p p", "D B"). This is a character-spacing bug in the ditaa text renderer. Arrow between Auth→App renders correctly. App→DB connection line also correct. |

---

### FIXED candidates confirmed in this audit

| Issue # | Description | Evidence |
|---------|-------------|---------|
| #467 | Class inheritance: hollow triangle arrow | class/01_basic: hollow triangle confirmed |
| #468 | Class compartment divider between name and members | class/01_basic, 21_microservices: dividers present |
| #475 | Usecase: no actors (stick figures missing) | usecase/02_with_actors, 04_with_packages: actors present |
| #479 | Usecase: package/boundary rectangles not rendered | usecase/04_with_packages: boundary boxes present |
| #551 | Class stereotype rendered in members compartment | class/12_all_relations: «extends» appears above name not in body |
| #538 | Packages render as flat horizontal dashed strips | architecture-overview: package containers correct with tab labels |

**Note on #539/540:** Arrow-through-box routing is substantially improved in architecture-overview but not fully eliminated at the Pipeline Core title bar boundary. Label de-collision (#540) also improved but convergence zone at Frontends→Pipeline still has crowded labels. Neither issue is clean enough to close.

**Note on #578:** usecase/04_with_packages package labels now appear at top-left (not bottom). However the layout problem — everything in one horizontal strip — means this diagram is functionally unreadable regardless of label position. Issue remains open for layout.

---

### Still-present (confirmed in this audit)

| Issue # | Family | Observation |
|---------|--------|-------------|
| #523 | ditaa | Character spacing: "A u t h", "A p p", "D B" |
| #539 | arch/component | Arrow-through-box partially improved; not fully fixed |
| #540 | arch | Edge label overlap in convergence zone; substantially improved |
| #567 | component | 06_with_arrows: "uses"/"composed" labels overlap on A→C |
| #570 | class | 14_nested_packages: flat horizontal strip; all nodes same Y |
| #572 | class | 21_microservices: Gateway "authenticate(token)" text clipped |
| #574 | usecase | 02_with_actors: "leads to" label overlaps Admin actor body |
| #578 | usecase | 04_with_packages: layout unreadably flat/horizontal |
| #581 | arch/state | diagram-family-lifecycle: edge labels overlap state nodes |
| #571 | deployment | Node 3D cube shape absent (flat rectangles) |

---

### NEW issues filed

| Issue # | Priority | Family | Description |
|---------|----------|--------|-------------|
| #617 | P2 | class | 12_all_relations: "«extends»" and "association" labels overlap at arrow midpoint (same position, should be on opposite sides) |
| #618 | P2 | state | state/07_nested + state/03_concurrent: initial transition dot emits two arrows causing visual ambiguity with final/exit transition |
| #619 | P2 | activity | activity/07_partition: start dot overlaps partition column header text |

---

### Top systemic issues (post-Wave-15)

1. **Layout engine produces degenerate flat/horizontal arrangements** — Affects usecase/04_with_packages (everything same Y in one row), class/14_nested_packages (all nodes same Y). Root cause: left-right layout preference not balanced by vertical distribution. This makes diagrams with packages functionally unreadable even when individual element shapes are correct.
2. **State transition initial-arrow ambiguity** — Start dot emits multiple arrows; final/exit transitions route back through the start region. Affects state/07_nested and state/03_concurrent. Trackers: #618.
3. **Edge label crowding in convergence zones** — Still present in arch/architecture-overview (improved) and component/06_with_arrows. Label placement algorithm does not account for adjacent-arrow label proximity.
4. **Class member text clipping** — Gateway box in class/21_microservices clips "authenticate(token): boolean" at box right edge. Boxes not widened to fit longest member string.
5. **Ditaa character spacing** (#523) — persistent, minor but visible in all ditaa output.

---

### Overall grade trajectory

| Wave | Approximate grade | Key milestone |
|------|------------------|---------------|
| Pre-W12 | D / D+ | No actors, no compartment dividers, no package containers |
| Post-W12/W13 | C+ / B- | Actors, dividers, package containers, triangles fixed (P0 backlog cleared) |
| Post-W15 (this audit) | B- | Core UML shapes substantially correct; remaining gaps are layout quality, label clearance, and a handful of shape-specific bugs |

The trajectory is positive. The P0 backlog (missing actors, missing dividers, missing triangles, missing package containers) is cleared. What remains is a B-to-A gap: layout quality (flat arrangements, edge label crowding), a few shape-specific gaps (deployment 3D cubes, ditaa spacing, state initial-arrow ambiguity), and canvas sizing in edge cases.

---

### Recommended Wave 16 priorities

1. **Layout vertical distribution** — When a `left to right direction` is not specified, ensure nodes are distributed vertically (rank-based layout). Fix the degenerate single-row cases in usecase/04 and class/14. This is the biggest quality-of-life gap visible in the current corpus.
2. **Edge label clearance pass** — After placing edge labels, run a clearance check against all node bounding boxes and adjacent labels. Bump labels that intersect. Affects component/06, arch/diagram-family-lifecycle, usecase/02, class/12.
3. **Class box width = max(member text widths)** — Gateway box in class/21 clips member text. Auto-size box width to accommodate widest attribute/method string.
4. **State diagram exit-transition routing** — Ensure final/shutdown transitions originate from the state node boundary, not the start-dot region.
5. **Deployment 3D cube node shape** (#571) — Low complexity, high parity value. UML deployment nodes must render as 3D cubes.
