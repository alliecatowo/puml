# Visual Audit — 2026-05-28 — Specialized Diagrams Bucket

**Reviewer:** Opus 4.7 multimodal audit
**Bucket:** specialized — gantt, nwdiag, json, yaml, mindmap, wbs, salt, creole, chronology, wire, bytefield
**Corpus root:** `target/audit_corpus/png/`
**Method:** Read-tool raster vision on PNG output (no source inspection, no SVG inspection)

## Bucket inventory

| Family | Has examples | Has fixture renders | Status |
|---|---|---|---|
| gantt | yes (10) | 1 | covered |
| nwdiag | yes (7) | 0 | covered |
| json | yes (4) | 1 | covered |
| yaml | yes (3) | 1 | covered |
| mindmap | yes (7) | 3 | covered |
| wbs | yes (7) | 3 | covered |
| salt | yes (6) | 6 | covered |
| creole | yes (5) | 0 | covered |
| chronology | yes (5) | 0 | covered |
| wire | yes (4) | 0 | covered |
| bytefield | NONE | NONE | family not implemented / no fixtures present in corpus |

Bytefield is mentioned in the bucket scope but no PUML fixtures or rendered PNGs exist anywhere in the audit corpus or `tests/fixtures/`. Either the family is out of scope for v1 or its examples were dropped before corpus regeneration. Flagging for orchestrator triage only — no per-family issues filed.

---

## P0 — Blocking visual defects

### P0-1 — Salt: `widget submit_button` keyword renders as literal text
**Fixture:** `target/audit_corpus/png/tests/fixtures/families/valid_salt_bootstrap.puml.png`
**Symptom:** Entire diagram renders nothing but the plain text string `widget submit_button` at the top-left. No widget chrome, no button shape, no surrounding mockup frame. Whatever the `widget` keyword maps to in the salt grammar is being treated as a generic label, not as a widget directive.
**Evidence:** PNG is ~80px tall — there is no rendered widget at all, only an unstyled text run.
**Impact:** The "salt bootstrap" feature is non-functional. Anyone using `widget <name>` salt syntax gets garbage.

### P0-2 — Salt: top-level menu bar pipe separators missing across every salt fixture
**Fixtures:**
- `docs/examples/salt/05_settings_dialog_showcase.puml.png`
- `docs/examples/salt/06_style_widget_depth.puml.png`
- `tests/fixtures/families/valid_salt_settings_dialog_showcase.puml.png`
- `tests/fixtures/families/valid_salt_layout_depth.puml.png`
- `tests/fixtures/families/valid_salt_style_widget_depth.puml.png`

**Symptom:** PlantUML salt menu syntax `File | Edit | View | Tools | Help` should render with vertical bar separators between menu titles. Our output shows the titles separated by whitespace only — no `|` glyphs, no implied dividers. Same issue affects tab strips: tabs render as adjacent rounded rectangles but the `|`-delimited tab name list collapses to space-separated titles inside one combined background.
**Impact:** Salt UI mockups intentionally evoke desktop menu chrome — losing the pipe separators makes the menu bar look like a paragraph of words. Parity failure with upstream PlantUML.

### P0-3 — Salt: combobox / dropdown arrow glyph wrong (▲ instead of ▼)
**Fixtures:**
- `docs/examples/salt/05_settings_dialog_showcase.puml.png` — "Workspace" combobox in Theme row
- `docs/examples/salt/06_style_widget_depth.puml.png` — "Admin" combobox in Role row
- `tests/fixtures/families/valid_salt_style_widget_depth.puml.png` — same "Admin" combobox

**Symptom:** Combobox widgets render with an upward-pointing triangle `▲` in the arrow well. Comboboxes universally use a downward-pointing triangle `▼` (collapsed dropdown). The current glyph implies "expand upward / scroll up" which no real combobox does.
**Impact:** Mockups read wrong; users seeing a real UI mocked this way will think the widget is a spinner, not a dropdown.

### P0-4 — Salt: `..` and `==` separators render as literal punctuation
**Fixture:** `docs/examples/salt/03_separator.puml.png`
**Symptom:** Salt's horizontal-rule separators (`..` for dotted, `==` for thick) should render as horizontal lines spanning the widget column. Output shows the literal two-character strings `..` and `==` as text on their own row, with no horizontal rule.
**Impact:** Layout separators in salt mockups are invisible; rows above/below the separator now butt against each other with no visual divider.

### P0-5 — Creole: large fraction of inline / block markup falls through as raw text
**Fixture:** `docs/examples/creole/05_ch22_parity.puml.png`
**Symptom:** In the "Creole Blocks" note, multiple creole constructs render literally:
- `**literal code**` shows raw asterisks — should render as inline code span
- `[[not a link]]` shows raw double-brackets — should render as creole link text
- `\`- root` and `\`- child` show backtick-dash prefix — should render as tree-style indent guides
- `H2` `x2` `wave` appear with subscript / strikethrough glyphs that hint at partial support but inconsistent
- The `========== section ==========` divider renders as literal equals signs around the word

The associated sequence message above the note shows `???????` placeholder characters instead of the formatted message text — looks like creole rendering inside a sequence message label substitutes question marks when it can't render the original.

**Impact:** Creole parity is a chapter-22 acceptance criterion; a fixture explicitly named `05_ch22_parity` has at least 6 visible markup-rendering bugs in a single image.

### P0-6 — Mindmap: deep / multi-line nodes overlap into a single illegible stack
**Fixture:** `docs/examples/mindmap/06_multiline_node_labels.puml.png`
**Symptom:** Left-side branches "Sprint 1 Core Engine", "Sprint 2 API Layer", "Sprint 3 UI Integration", "Sprint 4 Testing & Hardening" stack vertically with massive Y-axis overlap — text from "Sprint 2" reads through "Sprint 3" because the node boxes are vertically positioned 8-12px apart while each box is ~40px tall. The right-side branches (Discovery Phase, Design Phase) lay out cleanly with proper spacing, so the layout failure is specific to multi-line node labels on the left side.
**Impact:** Mindmap layout engine miscomputes per-node vertical extent when the label has a newline, so it allocates one-line slot height for multi-line content.

### P0-7 — Wire: edge labels and port labels collide for every wire fixture
**Fixtures:**
- `docs/examples/wire/01_basic_components.puml.png` — "POWER" port label and "POWER" edge label render on top of each other, creating apparent strikethrough; same for "DATA"
- `docs/examples/wire/02_columns_spacing.puml.png` — port labels I1/I2/I3/USB sit directly on the arrowheads/edge midpoints
- `docs/examples/wire/03_variables_print.puml.png` — port labels MAIN/AUX overlap edge labels MAIN/AUX with crossing-out effect
- `docs/examples/wire/04_goto_move_ports.puml.png` — SDA/SCL labels on three components all overlap their adjacent edge segments

**Symptom:** Wire renderer places port labels just outside the component bounding box, but routes the edge through the same horizontal band. When edge label == port label (common — they identify the same signal), text overlaps and reads garbled.
**Impact:** Wire diagrams are unreadable in their current form. The 4/4 fixture defect rate makes this the highest-locality bug in the bucket.

### P0-8 — Gantt: duplicate milestone rows on critical-path fixture
**Fixture:** `docs/examples/gantt/08_milestones_critical_path.puml.png`
**Symptom:** "Alpha Release" and "Beta Release" each appear twice as separate task rows in the left-hand task list. Two diamond milestones for "Alpha Release" render at slightly different x-positions (column boundary vs. mid-column), confirming the milestone is parsed into the model twice with different date binding.
**Impact:** Critical-path output is corrupted. Likely a parser issue where `[Alpha Release] happens at ...` lines both register the milestone and re-register it on the predecessor link.

---

## P1 — High-impact but not blocking

### P1-1 — Chronology: vertical axis date labels only show extremes
**Fixtures:**
- `docs/examples/chronology/01_events.puml.png` — only "2026-01-01" and "2026-05-01" appear on axis; Feb/Mar/Apr dates absent even though events are evenly spaced
- `docs/examples/chronology/02_timeline.puml.png` — Phase 2 (2026-02-20) absent from axis
- `docs/examples/chronology/03_release_history.puml.png` — 5 releases, only first and last dates appear on axis

**Symptom:** Vertical date guide on the left side of the timeline renders the start date and end date of the project but skips intermediate event dates entirely. Either the axis is rendering "min and max only" or the intermediate labels are being clipped to a left margin too narrow to hold them.

### P1-2 — Chronology: ungrouped events crowd into the same Y band
**Fixture:** `docs/examples/chronology/05_calendar_depth.puml.png`
**Symptom:** The three events at the end of the timeline (2024-01-15 01:08:12, 2024-01-15 13:08:12, 2026-02 to 2026-04) all stack near 2026-04-30 with the connectors radiating from one node. The intra-day timestamps should plausibly cluster, but the "Research" span (2026-02 to 2026-04) should sit visibly above 2026-04-30, not at the same position.

### P1-3 — Nwdiag: group frame swallows network bus labels
**Fixture:** `docs/examples/nwdiag/04_icons_multiline.puml.png`
**Symptom:** The orange "group services" frame fully encloses the dark-blue "network public" header bar, partially occluding the IP-range subtitle and the "203.0.113.10" address label. The Z-order is wrong: group frames should render under the network bus chrome, not over it.

### P1-4 — Nwdiag: 02_multi_network shows three stacked network buses with redundant per-network chrome
**Fixture:** `docs/examples/nwdiag/02_multi_network.puml.png`
**Symptom:** Each network bus renders two parallel rounded-rectangle strips (header + body) instead of a single labeled bus line. The duplicate bus shape adds vertical clutter and an empty whitespace band between header and body that doesn't appear in upstream PlantUML output.

### P1-5 — Nwdiag: address-only labels render as `host [address]` where they should split
**Fixture:** `docs/examples/nwdiag/03_address_ranges.puml.png`
**Symptom:** Hosts `router [203.0.113.1]` and `db [10.0.0.20]` render their address in square brackets inline with the host name. The duplicate address (already on the bus drop line) makes the host label cluttered.

### P1-6 — JSON: large array-of-objects has no row striping or visible separation between siblings
**Fixture:** `docs/examples/json/04_deep_nesting_arrays_of_objects.puml.png`
**Symptom:** Three nested object rows for `data[0]` and continuation into `data[1]` are visually identical; nothing demarcates the boundary between sibling objects except the tree connector. At this depth (3+ levels of nesting) the eye loses track of which `shipping` belongs to which order.

### P1-7 — JSON: tree connector goes corner-to-corner only, no continuation line through wide rows
**Fixtures:**
- `docs/examples/json/04_deep_nesting_arrays_of_objects.puml.png`
- `docs/examples/json/03_nested.puml.png`

**Symptom:** Tree connector lines are drawn between adjacent rows but break entirely for the value column (right half). When the diagram is wide, the eye crosses an unstriped gap with no guide back to the tree. Adding a continuation row separator (or zebra striping) would fix readability.

### P1-8 — Salt: frame title detaches from frame chrome
**Fixture:** `docs/examples/salt/02_frame.puml.png`
**Symptom:** "Login Form" renders as its own rounded-rectangle widget below the empty outer frame, instead of as the title bar of the frame itself. Compare to upstream PlantUML where the frame title sits at the top-left of the frame border.

### P1-9 — Salt: scroll-bar handle / sprite-folder label leaks outside its widget
**Fixtures:**
- `tests/fixtures/families/valid_salt_layout_depth.puml.png` — "sprite folder" text appears clipped above the data table
- `tests/fixtures/families/valid_salt_style_widget_depth.puml.png` — "Scope shield" text appears clipped above the data table

**Symptom:** A sprite reference (`<&shield>`, `<&folder>`) intended to be an icon embedded in a cell is rendering its text label outside the widget bounds in a position that overlaps the next row's separator line.

### P1-10 — Gantt: weekend shading (calendar mark) misregistered against day grid
**Fixture:** `docs/examples/gantt/09_ch16_parity.puml.png`
**Symptom:** The weekly-scale gantt shows three color bands (red, light-yellow, light-green) that the title bar says correspond to "closed Sat/Sun", "named Prep window 09-21..09-22", and "Bob off 09-25; open 09-24". The bands are placed near the right edge of the first week column but the underlying weekly grid only shows 3 column boundaries, so it's unclear which day each colored band actually covers. The relationship between band X-positions and date labels is unverifiable from the rendered output — bands should be tied to day cells, but here they appear in a generic "first half of week 1" zone.

### P1-11 — Gantt: critical-path dependency arrow crosses through milestone diamond
**Fixture:** `docs/examples/gantt/08_milestones_critical_path.puml.png`
**Symptom:** Dependency arrow from "Docs" task back to "GA Release" milestone routes through the Beta Release diamond and the duplicate Alpha Release row, instead of going around. Together with the duplicate-rows defect (P0-8) this makes the critical-path render a mess.

### P1-12 — WBS: cross-arrow fixture renders only one extra arrow, not "cross arrows"
**Fixture:** `docs/examples/wbs/06_alias_cross_arrows.puml.png`
**Symptom:** The fixture name implies multiple cross-edges between WBS nodes (e.g., Platform↔Docs, API↔Release). Output shows only a single Platform→Launch arrow added on top of the standard WBS tree. Either the parser is dropping the additional alias-relation lines or the renderer is silently ignoring them.

---

## P2 — Polish / minor

### P2-1 — YAML: empty-root rendering shows literal `{...}` placeholder in first row
**Fixtures:** `docs/examples/yaml/01_mapping.puml.png`, `docs/examples/yaml/02_sequence.puml.png`, `docs/examples/yaml/03_nested.puml.png`
**Symptom:** Every YAML projection begins with a header row whose key column is empty and whose value column reads `{...}`. The placeholder is intended to mean "root collection here" but it leaks into the rendered view. JSON has the same pattern.

### P2-2 — Creole: monospace backticks render literally
**Fixture:** `docs/examples/creole/04_monospace.puml.png`
**Symptom:** Sequence message `send \`code snippet\`` retains the literal backticks around `code snippet` rather than rendering it in monospace. The accompanying note "use monospace for code" is also not rendered in monospace despite being the entire point of the fixture.

### P2-3 — Creole: `<size:N>` color/size formatting partially honored
**Fixture:** `docs/examples/creole/02_color_size.puml.png`
**Symptom:** "red text" renders red, confirming color tag works. "bigger" should render at a larger size per the `<size:14>` tag — visually it reads at the same size as surrounding text. Either the size tag is dropped or the size value is being clamped to the default.

### P2-4 — Chronology: phase connector lines have inconsistent length
**Fixture:** `docs/examples/chronology/02_timeline.puml.png`
**Symptom:** Phase 1 and Phase 3 connectors (axis dot → card) are long horizontal lines. Phase 2's connector is dramatically shorter — appears the card is rendered closer to the axis without re-aligning the X position of the connector endpoint.

### P2-5 — Mindmap: 07 theme vibrant renders only 3 nodes (no preview of full theme)
**Fixture:** `docs/examples/mindmap/07_theme_vibrant.puml.png`
**Symptom:** The theme preview shows root → "Branch" → "Leaf" — three nodes in a row. For a "vibrant theme" showcase this is much less coverage than `wbs/07_theme_vibrant.puml.png`, which shows 7 nodes with two depth levels. Either the source is intentionally minimal or the fixture should be expanded.

### P2-6 — Salt: tabs use color but lose tab-strip baseline
**Fixture:** `docs/examples/salt/04_tabs.puml.png`
**Symptom:** Tab1 / Tab2 / Tab3 render as three boxes with no underline separating active tab from inactive content area. Tab1 is bold (active) but the active-tab visual emphasis is otherwise identical to the inactive tabs.

---

## Family summary table

| Family | P0 count | P1 count | P2 count | Net status |
|---|---|---|---|---|
| gantt | 1 | 2 | 0 | needs fix (P0-8 dupe rows is real bug) |
| nwdiag | 0 | 3 | 0 | polish only |
| json | 0 | 2 | 1 | polish only |
| yaml | 0 | 0 | 1 | clean |
| mindmap | 1 | 0 | 1 | needs fix (P0-6 multiline layout) |
| wbs | 0 | 1 | 0 | needs investigation (P1-12 alias arrows) |
| salt | 4 | 2 | 0 | major work needed |
| creole | 1 | 0 | 2 | needs fix (P0-5 markup fallthrough) |
| chronology | 0 | 2 | 1 | needs fix (P1-1 axis labels) |
| wire | 1 | 0 | 0 | needs fix (P0-7 label collision) |
| bytefield | n/a | n/a | n/a | not present in corpus |

---

## Recommended issue filings

P0 candidates that warrant immediate GitHub issues (one per family-distinct defect):

1. P0-1 + P0-2 + P0-3 + P0-4 — file as `salt: widget keyword, menu separators, combobox glyph, separator rendering`
2. P0-5 — file as `creole: chapter-22 parity — multiple inline/block constructs render as raw text`
3. P0-6 — file as `mindmap: multi-line node labels stack with insufficient vertical spacing`
4. P0-7 — file as `wire: port labels collide with edge labels for every fixture`
5. P0-8 — file as `gantt: critical-path milestones rendered as duplicate task rows`

P1 candidates worth filing as separate visual-audit tickets:
- P1-1 — `chronology: vertical axis only renders extreme dates`
- P1-3/P1-4 — `nwdiag: network bus chrome doubled and group-frame z-order`
- P1-12 — `wbs: alias cross-arrow fixture only emits one extra edge`

The remaining P1/P2 items are polish-grade and can be batched into a single "specialized polish" tracking issue.
