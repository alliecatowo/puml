# 2026-05-29 — Post-curves visual audit

After landing #1318 (endpoint anchoring) and #1319 (curved self-edges), the
corpus was regenerated (`python3 scripts/render_corpus.py --force`,
414 rendered, 0 failed) and the most complex fixture per family was
inspected.

## What improved

### Sequence — `docs/examples/sequence/17_all_groups.puml`

- "break on error" group: the `abort` self-message now renders as a
  rounded "D" arc (quadratic-bezier corners) instead of the sharp 3-
  segment polyline.
- `19_lifecycle.puml` self-messages (`Alice -> Alice : start process`,
  `Worker -> Worker : processing`) likewise render as rounded loops.
- All non-self arrows still terminate cleanly on lifelines; no visual
  regression introduced.

### Class — `docs/examples/class/12_all_relations.puml`

- Inheritance / association / composition / aggregation arrows still
  land on the correct edges; no regression.
- Standalone test fixture (Node --> Node : next) confirms the new
  `uml-self-association` arc emits in the top-right corner of the class
  with the label adjacent to it.

### Activity — `docs/examples/activity/18_repeat_while_nested_partition.puml`

- Both nested while-loops ("more rows?" and "records in queue?") now
  render with rounded corner arcs on the back-edge.  The vertical leg
  is still a `<line>` element so wave-8 detection tooling and
  `w8_while_loop_emits_upward_back_edge_arrow` continue to match.
- Forward arrows inside swimlanes ("Extract", "Transform", "Load")
  remain clean.
- Fork/join in the "Load" partition is correct.

### State — `docs/examples/state/09_three_level_composite.puml`

- Composite state nesting (`Device > On > Initializing > SelfTest`)
  renders correctly; no self-transitions appear in this fixture, but a
  hand-rolled test with `Working --> Working : retry` confirms the new
  cubic-bezier loop emerges from the right edge and re-enters the top
  edge of the node, with the `retry` label clear of the box.

### Component — `docs/examples/component/06_with_arrows.puml`

- **The headline #1318 fix:** `C --* A : composed` previously terminated
  with the filled diamond floating against C's top-left CORNER while
  the rest of the polyline routed around to A.  After the router-order
  flip, the polyline now starts at C's top-edge midpoint, routes
  through the inter-rank channel, and terminates with the diamond at
  A's bottom-edge midpoint.
- Other `06_with_arrows` arrows (`A --> B : calls`, `A ..> C : uses`,
  `B <|-- D : extends`) all anchor cleanly on bbox edges.

## What's still pending (file as separate tickets)

### `docs/diagrams/architecture-overview.puml`

When rendering the architecture overview (used in README and docs):

- Follow-up tickets filed:
  - #1326 — package header crossed by entering arrows
  - #1327 — package-framed node anchored on frame corner

- **Header-band crossing** (#1326): vertical arrows entering the "Frontends",
  "Shared Services", "Pipeline Core", and "Output Formats" packages
  cross the dark navy header bar that contains the package label text.
  The `package_header_clearance` push in
  `src/render/graph_layout/router.rs::soft_clamp_ch_y` aims to keep
  channel-y values below the header band, but it appears to misfire when
  the package frame top falls between two ranks that are themselves
  separated by less than `rank_separation + package_header_clearance`.
  **Suggested investigation:** detail-level instrumentation in
  `soft_clamp_ch_y` showing the package frame top, the chosen channel y,
  and the resulting vertical segment span — then expand
  `expand_canvas_for_transition_labels`-style logic into the package
  header check or push the channel below the bottom of the header band
  rather than only below `gy + clearance`.
- **Top-corner anchoring on package entry**: edges entering the
  Pipeline-Core / Output-Formats packages anchor at the top-left CORNER
  of the package frame instead of the closest top-edge midpoint of the
  child node.  This is the same family of bug as #1318 but for the
  package-frame case rather than the component-to-component case.
  Suggest a separate ticket: "Edges entering package-framed nodes
  should anchor on the inner node, not the package frame corner."

### `docs/examples/class/14_nested_packages.puml`

- The three packages (`repository`, `service`, `domain`) overlap each
  other.  The `domain` frame extends below `repository`'s bottom into
  empty space while `service::ProductService` extends past the SVG
  right edge.  Layout-level issue, not addressed by this PR.  Existing
  ticket coverage: epic #590 (renderer architecture and layout).

## Net assessment

The two issues this PR targets (#1318 marker anchoring, #1319 curved
self-edges) are visibly fixed in every complex fixture inspected.  No
new visual regressions introduced.  Two follow-up areas (package-header
crossing and package-frame corner anchoring) became more visible
because the other arrows are now clean — they should each become their
own focused ticket.
