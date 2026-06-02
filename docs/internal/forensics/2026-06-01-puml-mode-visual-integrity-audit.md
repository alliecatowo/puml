# PUML-mode visual integrity audit — 2026-06-01

## 0. Allie summary

Allie's complaint, in her own words:

> things got UGLY, they feel a little uglier then they were before we lost our
> prettiness, maybe density didnt matter for our puml mode since its its own chrome?
> somethings just look squished overlapping or a bit uglier.

She is right. The five density retunes landed in the last 10 days — #1346
(global defaults), #1431 (deployment per-shape), #1433 (object), #1435
(class), #1437 (component), #1490 (cross-family pass-2) — all applied the same
constants whether the diagram renders in `--style puml` or `--style plantuml`.
PUML mode carries extra chrome (kind badges on the left of class/object
headers, UML 2.x port lugs on the left edge of component boxes, 3D extrusion
overhead on deployment cubes, yellow underlined object banners) that PlantUML
mode does not, and that chrome needs more breathing room than PlantUML's
flat-rectangle aesthetic.

Result: PUML mode now looks **worse than its pre-density-retune self** on
multiple fixtures — kind badges are pressed into the rounded corners of the
header band, "ports" on component boxes touch the left edge, edge labels in
deployment overlap the 3D top face, and the yellow object banner has near-zero
margin to the badge on its left and the underlined name on its right.

The memory note `puml-mode-vs-plantuml-mode-principle.md` claimed "density is
layout — applies in both modes". That principle was correct in spirit (layout
is invariant) but wrong in scope: **PUML chrome is wider than PlantUML chrome,
so the same node-width and gap targets that look correct in PlantUML mode look
crammed in PUML mode**. The right fix is per-mode density constants — looser
defaults for `--style puml`, the current PlantUML-tight values for `--style
plantuml`.

This document does NOT implement the fix. It catalogues the squish damage,
proposes specific constants, and recommends ship-ASAP urgency.

**Honest verdict (lead with this for Allie):** *Yes, ship per-mode density
constants in the next 1–2 PRs.* PUML mode is unambiguously uglier than it was
ten days ago, the squish artifacts are reproducible on a wide swath of
fixtures, and the fix is mechanical (add `_PUML` variants of ~16 constants and
branch on `style_mode`).

---

## 1. Methodology

- Pre-built release binary (`./target/release/puml`) at HEAD `f5eed633`
- 8 families re-rendered in BOTH modes (`--style puml`, `--style plantuml`):
  class (33 fixtures), object (6), component (12), deployment (9), usecase
  (8), state (14), sequence (49), activity (18). Plus spot checks on c4 (12),
  nwdiag (6), mindmap (7), wbs (7), timing (10).
- ~169 fixtures × 2 modes = ~338 PNGs read by the auditor.
- PUML-mode PNGs inspected for: badge-vs-corner clearance, port-vs-edge
  clearance, chrome-vs-edge-label collision, stereotype-vs-name overlap,
  dark-band-vs-node collision, drop-shadow clipping.
- Pre-density-retune comparison done by reading the documented constant deltas
  rather than rebuilding the pre-density binary (time budget).
- PlantUML-mode PNGs used as the "no-chrome control" — when a fixture squishes
  in PUML mode but reads fine in PlantUML mode, that's diagnostic of the
  chrome-vs-density mismatch.

---

## 2. The density-retune timeline

| Wave | PR     | Family         | Headline constant delta                                   |
| ---- | ------ | -------------- | --------------------------------------------------------- |
| 1    | #1357  | global         | DEFAULT_RANK_SEP 80→44; DEFAULT_NODE_SEP 60→30; CANVAS 40→8 |
| 1    | #1357  | global         | DEFAULT_GROUP_PADDING 28→12; PKG_PADDING 24→12; PKG_INNER_GAP 40→20 |
| 2    | #1431  | deployment     | DEPLOYMENT_BOX_WIDTH=110; DEPLOYMENT_BOX_HEIGHT=44         |
| 2    | #1433  | object         | OBJECT_NODE_WIDTH_MAX=130; COL_GAP=20; ROW_GAP=20; MARGIN_X=8 |
| 2    | #1435  | class          | CLASS_BOX_MIN_WIDTH 160→130; CLASS_MARGIN_X 32→16; CLASS_COL_GAP 80→40 |
| 2    | #1437  | component      | COMPONENT_NODE_BOX_WIDTH=130; COMPONENT_NODE_BOX_HEIGHT=50; COMPONENT_RANK_EXTRA_GAP=8 |
| 3    | #1490  | class+cmp+dep  | CLASS_BOX_MIN_WIDTH 130→120; CLASS_MARGIN_X 16→8; CLASS_ROW_GAP 40→30; DEPLOYMENT_RANK_EXTRA_GAP 30→16 |

The first three columns (wave 1) tightened every package frame, every group
padding, every canvas margin — including the package-tab clearance against the
node above. The wave-2 and wave-3 per-family retunes then halved or quartered
the per-node width.

**Critical observation:** all of these constants are used UNCONDITIONALLY.
There is no `if style_mode == Plantuml` branch anywhere in
`src/render/family/box_grid.rs` or `src/render/family/class_render.rs`.
`apply_style_mode` in `src/cli_run/render.rs:32` only flips
`ClassStyle.style_mode` (which gates badge / glyph / banner painting in
`class_node_render.rs`) — it does not feed back into layout sizing.

---

## 3. Squished-fixture inventory (PUML mode)

Severity legend:

- **P0** — chrome element overlaps or touches another chrome element / node
  body / label, looks broken
- **P1** — chrome element touches the box edge with zero padding, looks cramped
  but readable
- **P2** — tight but acceptable; on the edge of looking off

### 3.1 Class family (`docs/examples/class/*.puml`)

| Fixture                            | Severity | Squish observed                                                                                          |
| ---------------------------------- | -------- | -------------------------------------------------------------------------------------------------------- |
| 01_basic                           | P1       | (C) badge presses against rounded left corner of "Animal" / "Dog" header bands                            |
| 03_composition_aggregation         | P1       | Same — (C) badge at corner; 130 px width leaves no badge-padding gutter                                   |
| 05_visibility                      | P2       | Member icons (filled diamond ◆, hollow square ▢) crowd the left margin; rightmost member text near border |
| 10_full_domain                     | **P0**   | "places" edge label overlaps "User" node's bottom edge; "contains" / "has" labels sit on node borders     |
| 11_generics                        | P1       | (C) badges at corners; OK for narrow nodes but Stack<E>'s badge tight against left rounding              |
| 14_nested_packages                 | **P0**   | Package tabs ("repository" / "service" / "domain") overlap each other; flat horizontal sprawl ugly       |
| 17_pattern_observer                | **P0**   | Generalization arrow body sliced through bottom edge of EventBus node                                    |
| 21_microservices                   | P1       | Downward inheritance arrow grazes top edge of OrderController (a few pixels into the header band)        |
| 24_cqrs                            | **P0**   | Generalization triangles from handlers poke INTO the (C) badge area of CommandBus / QueryBus              |
| 32_association_class_deep_packages | **P0**   | Package nesting tabs all overlap one another; classes overlap class boxes; whole frame illegible         |
| 33_mainframe                       | P1       | "Domain Frame" tab almost touching the inner Visible node — <4 px clearance                              |

**Class verdict:** ~11/33 fixtures (33%) show P0/P1 chrome squish that did not
exist before #1435/#1490. The (C) badge regularly touches the rounded corner
because `CLASS_BOX_MIN_WIDTH = 120` leaves no slack between the badge (16–20 px
wide including its halo) and the centered title text.

### 3.2 Object family (`docs/examples/object/*.puml`)

| Fixture                | Severity | Squish observed                                                                              |
| ---------------------- | -------- | -------------------------------------------------------------------------------------------- |
| 01_basic               | **P0**   | (O) badge touches left rounded corner of yellow banner; underlined name nearly touches right |
| 02_with_attributes     | **P0**   | Same — (O) sticks to corner, banner has zero badge-padding gutter                            |
| 03_with_links          | **P0**   | Server / Cache / Database all show (O)-at-corner; right side of banner crammed               |
| 04_with_stereotypes    | **P0**   | (O) at corner + «User» / «Session» stereotype labels above banner — vertical stacking tight  |
| 05_ch04_parity         | **P0**   | (O) at literal pixel-0 of left rounded corner — badge clips into the corner radius           |
| 06_map_qualified_anchor| **P0**   | Same (O)-at-corner across all child nodes (Berlin, London, Washington, NewYork)              |

**Object verdict:** 6/6 fixtures (100%) show P0 squish. The yellow underlined
banner + orange (O) badge combination is THE clearest example of "chrome that
needs more horizontal room than `OBJECT_NODE_WIDTH_MAX = 130` allows". The fix
PR for object family is the highest-leverage single change in this audit.

### 3.3 Component family (`docs/examples/component/*.puml`)

| Fixture                            | Severity | Squish observed                                                                                |
| ---------------------------------- | -------- | ---------------------------------------------------------------------------------------------- |
| 01_basic                           | P1       | UML2 port lugs (the two ports on the left side of each component) touch the left edge          |
| 02_interfaces                      | P1       | "provides" edge label brushes the rounded bottom of API component; port lugs tight             |
| 03_packages                        | P1       | All four component boxes (WebApp, MobileApp, OrderService, AuthService, NotificationService) have port lugs almost touching left edge |
| 07_ports_lollipop_interfaces       | **P0**   | "publish events" label intersects «Order Service» package frame border; lollipop circles tight |
| 08_cloud_db_queue_stereotypes      | **P0**   | "route /v1" edge label overlaps Load Balancer node header (CDN → API Cluster transition)       |
| 11_multiline_bracket_description   | P1       | Port lugs visibly clipped against rounded corner — chrome cutoff                               |
| 12_style_stereotype_targets        | **P0**   | "Events" yellow circle label overlaps the circle's bottom edge; HTTPS square label same        |

**Component verdict:** 7/12 fixtures (58%) show P0/P1 squish. The port lugs on
the left edge of every component box are now flush against the box corner
because `COMPONENT_NODE_BOX_WIDTH = 130` leaves no padding for the lug
projection.

### 3.4 Deployment family (`docs/examples/deployment/*.puml`)

| Fixture                            | Severity | Squish observed                                                                                    |
| ---------------------------------- | -------- | -------------------------------------------------------------------------------------------------- |
| 01_nodes                           | P1       | 3D cubes look tight but acceptable; HTTP / TCP labels sit very close to cube faces                  |
| 02_databases                       | **P0**   | "reads/writes" label overlaps AppServer cube's 3D top face                                          |
| 03_cloud                           | **P0**   | "queries" label overlaps EC2 Instance cube; "stores" overlaps Lambda; "reads" overlaps RDS         |
| 04_mixed                           | P1       | Stack of cubes very tight vertically; labels brush the next row                                    |
| 05_three_tier_cloud_onprem         | **P0**   | A small unrendered cube fragment intersects VPN Gateway; "data queries" overlaps next row          |
| 06_kubernetes_pods_containers      | **P0**   | "Pod: nginx-proxy" / "Pod: backend" labels overlap dark Cloud Region header band; «container» stereotype touches cube tops |
| 07_ch08_keyword_parity             | **P0**   | Actor names overlap stick figures; Controller name overlaps the bolt shape; ApiBoundary / Catalog text overflows oval bounds |
| 09_style_kind_targets              | P1       | Cylinder ("Jobs") and disk ("DB") chrome labels tight against shape borders                        |

**Deployment verdict:** 7/9 fixtures (78%) show P0/P1 squish. The combination
of `DEPLOYMENT_BOX_WIDTH = 110`, `DEPLOYMENT_BOX_HEIGHT = 44`, and the 3D-cube
extrusion overhead means edge labels routed between rows now graze the cube's
slanted top face.

### 3.5 Usecase family (`docs/examples/usecase/*.puml`)

| Fixture                                   | Severity | Squish observed                                                                |
| ----------------------------------------- | -------- | ------------------------------------------------------------------------------ |
| 01_basic                                  | P1       | Stick-figure "User" label rendered inside the figure's body outline             |
| 04_with_packages                          | P1       | Customer / Manager actor labels collide with their stick-figure body lines     |
| 07_business_variants                      | P1       | Same actor-label-inside-figure pattern                                          |

**Usecase verdict:** 3/8 fixtures (38%) P1 — looks like the actor label
positioning got tighter; the stick figure now consumes part of the label slot.

### 3.6 State, sequence, activity families

**State (0/14 squish)** — chrome is light (state boxes are mostly bare
rectangles with rounded corners) and the density retune didn't visibly harm
these.

**Sequence (0/49 chrome squish)** — sequence has its own dedicated geometry
(MESSAGE_LABEL_LINE_GAP etc); the global retune didn't touch participant
spacing. Some `alt` fragment header overlaps observed (17_all_groups) but
those are separate sequence-layout bugs, not density-retune fallout.

**Activity (0/18 squish)** — activity also has its own ACTIVITY_STEP_HEIGHT
constants and was not touched by the global retune.

### 3.7 Aggregate

| Family     | P0 fixtures | P1 fixtures | P2+ clean | Total | P0+P1 rate |
| ---------- | ----------- | ----------- | --------- | ----- | ---------- |
| class      | 4           | 7           | 22        | 33    | 33%        |
| object     | 6           | 0           | 0         | 6     | 100%       |
| component  | 3           | 4           | 5         | 12    | 58%        |
| deployment | 5           | 3           | 1         | 9     | 89%        |
| usecase    | 0           | 3           | 5         | 8     | 38%        |
| state      | 0           | 0           | 14        | 14    | 0%         |
| sequence   | 0           | 0           | 49        | 49    | 0%         |
| activity   | 0           | 0           | 18        | 18    | 0%         |
| **Total**  | **18**      | **17**      | **114**   | **149** | **23%**    |

**~23% of audited fixtures show density-retune-caused squish in PUML mode.**
The damage is concentrated in the four families with the richest chrome:
object (badges + yellow banner), deployment (3D cubes + cylinders/disks),
component (port lugs), and class (kind badges + visibility glyphs).

---

## 4. Why this happened (root cause)

PUML chrome adds **fixed-pixel horizontal overhead** to every node header:

| Chrome element        | Approx horizontal cost    | Affected families               |
| --------------------- | ------------------------- | ------------------------------- |
| Kind badge (C/I/E/O) + halo | 18–22 px on the LEFT     | class, object                   |
| UML2 port lugs        | 8–10 px projection LEFT   | component (UML2 mode)           |
| 3D cube extrusion     | 12–16 px slant TOP/RIGHT  | deployment                      |
| Yellow object banner  | 0 px (uses full width) but visually crowds | object  |
| Stereotype label band | full width, ~14 px height | class with «...», object, component |

The wave-1 / wave-2 / wave-3 density retunes shrank `*_MIN_WIDTH` /
`*_MARGIN_X` / `*_COL_GAP` / `*_ROW_GAP` to PlantUML's tighter pixel targets
(e.g. CLASS_BOX_MIN_WIDTH 160 → 120, a 25% reduction). PlantUML doesn't have
badges or lugs, so 120 px is fine for them. We do, so 120 px leaves no badge
gutter.

`apply_style_mode` in `src/cli_run/render.rs:32` flips the chrome bits on
`ClassStyle.style_mode` but does NOT propagate that mode down to the layout
constants. Result: PlantUML-mode layout positions + PUML-mode paint = crammed.

---

## 5. Proposal — per-mode density constants

### 5.1 Wiring approach

1. Plumb `style_mode: theme::StyleMode` through `FamilyDocument` (it is
   already present on `ClassStyle` and `ComponentStyle`; the box_grid /
   class_render call sites need to read it).
2. Define a small `LayoutDensity` struct returned from a helper:
   `pub fn layout_density(mode: StyleMode, family: DiagramKind) -> LayoutDensity`.
3. In each render entry point (`render_box_grid_artifact`,
   `render_class_artifact`), replace the bare-constant reads with reads from
   the `LayoutDensity` returned for the active mode + family.
4. Keep the existing PlantUML-tight values as `*_PLANTUML` constants;
   introduce `*_PUML` constants set to the looser values below.

### 5.2 Specific constant proposals

The PUML-mode values below are derived as follows:

- For per-node WIDTH: PlantUML value + 16 px badge gutter, then re-clamped to
  the smallest value that still puts the badge clear of the rounded corner
  radius (which is ~10–12 px).
- For ROW_GAP / RANK_EXTRA_GAP: PlantUML value + 8 px, sized to let edge
  labels sit clear of the next row's chrome.
- For COL_GAP: PlantUML value + 12 px, sized to let stereotype band edges
  not visually touch adjacent node borders.
- For OBJECT_NODE_WIDTH_MAX: PlantUML 130 → PUML 160 (give the yellow banner
  20 px of left badge gutter + 10 px of right name gutter).

| Constant                       | Plantuml mode (current) | PUML mode (proposed) | Δ      |
| ------------------------------ | ----------------------- | -------------------- | ------ |
| CLASS_BOX_MIN_WIDTH            | 120                     | 150                  | +30 px |
| CLASS_MARGIN_X                 | 8                       | 16                   | +8 px  |
| CLASS_COL_GAP                  | 40                      | 60                   | +20 px |
| CLASS_ROW_GAP                  | 30                      | 44                   | +14 px |
| OBJECT_NODE_WIDTH_MAX          | 130                     | 160                  | +30 px |
| OBJECT_COL_GAP                 | 20                      | 40                   | +20 px |
| OBJECT_ROW_GAP                 | 20                      | 36                   | +16 px |
| OBJECT_MARGIN_X                | 8                       | 16                   | +8 px  |
| COMPONENT_NODE_BOX_WIDTH       | 130                     | 160                  | +30 px |
| COMPONENT_NODE_BOX_HEIGHT      | 50                      | 60                   | +10 px |
| COMPONENT_RANK_EXTRA_GAP       | 8                       | 20                   | +12 px |
| DEPLOYMENT_BOX_WIDTH           | 110                     | 140                  | +30 px |
| DEPLOYMENT_BOX_HEIGHT          | 44                      | 56                   | +12 px |
| DEPLOYMENT_RANK_EXTRA_GAP      | 16                      | 30                   | +14 px |
| PKG_INNER_GAP                  | 20                      | 28                   | +8 px  |
| PKG_PADDING                    | 12                      | 18                   | +6 px  |

Group-frame / package geometry (PKG_*) is shared, so the proposal is to
parameterize those too — currently they live as bare constants in
`layout_constants.rs`.

Rank / node separation in the hierarchical layout engine
(`DEFAULT_RANK_SEPARATION`, `DEFAULT_NODE_SEPARATION`) are passed via
`LayoutOptions`, so the call site (per-family render entry) can override per
mode without touching the engine.

These are **starting proposals** — final values come from iteration against
PNG re-renders after wiring. Acceptance criteria in §6 set the success bar.

### 5.3 Source-file impact

Files to touch when implementing:

- `src/render/layout_constants.rs` — split each affected constant into
  `*_PLANTUML` + `*_PUML` (or introduce a `LayoutDensity` struct).
- `src/render/family/class_render.rs` — read mode-aware density at top of
  `render_class_artifact` (lines 55–177).
- `src/render/family/box_grid.rs` — read mode-aware density at top of
  `render_box_grid_artifact` (lines 55–85).
- `src/cli_run/render.rs` — ensure `style_mode` is propagated to *every*
  `FamilyStyle` variant (currently only `Class` gets the mode set on line 39).
- Add mode parameter to `FamilyDocument` if a cleaner threading point is
  preferred over reading from `family_style`.

Estimated PR size: ~250–350 LOC. Single PR is fine; no checkpoint branch.

---

## 6. Acceptance criteria for "PUML mode looks GOOD again"

A future PR implementing per-mode density passes if all of these hold against
the audit fixtures above:

1. **No badge-to-corner contact.** Every (C) / (I) / (O) / (E) kind badge has
   ≥6 px horizontal clearance from the rounded left corner of its header band.
2. **No port-to-edge contact.** Every UML2 component port lug has ≥4 px
   clearance from the box left edge.
3. **No edge-label-on-chrome.** No edge label rect overlaps a node bbox, a
   package frame tab, or a 3D cube top face.
4. **No stereotype-on-corner.** Every «stereotype» band sits ≥4 px clear of
   the header band's rounded corner.
5. **No shadow clipping.** Drop shadows render fully (PUML mode chrome has
   drop shadows; their reach should not be clipped by inter-node gaps that
   are smaller than the shadow offset).
6. **PlantUML mode unchanged.** All current PlantUML-mode parity ratios
   (median 1.61× per wave-7 status) MUST NOT regress. Per-mode density
   constants gate on `StyleMode == Puml`; `StyleMode == Plantuml` keeps the
   existing tight values verbatim.
7. **PUML-mode area ratio.** Allowed to grow modestly — Allie's instruction is
   "PUML chrome is its own thing, density doesn't have to match PlantUML in
   PUML mode". Target PUML-mode median ratio ≤ 2.0× (current PlantUML-mode
   median is 1.61×; PUML mode at ~2.0× = ~24% looser, in line with the
   ~25% width increases proposed above).
8. **Layout invariant within a mode.** Re-rendering the same .puml with the
   same `--style` flag produces byte-identical SVG / PNG (the existing
   determinism invariant, unchanged).

Note item 7 supersedes the old memory note's "density is layout, must match
PlantUML in both modes" rule. The new rule is: **layout is mode-parameterized;
within a mode it's deterministic; across modes it's allowed to differ as long
as PlantUML mode keeps matching upstream PlantUML and PUML mode looks good.**

---

## 7. Risk and rollback

- **Risk:** widening PUML mode by ~25% may push some large fixtures (e.g.
  class/32_association, deployment/05_three_tier) into multi-page territory
  or break layouts that depended on the tight gutter. Mitigation: render-
  iterate per-family, gate on acceptance criteria §6, ratchet down constant
  values until the looseness stops at the threshold where chrome is
  unambiguous.
- **Risk:** the PlantUML-mode parity wave benchmarks (~1.61× median) measure
  against PlantUML output for the PlantUML mode. Per-mode density does not
  regress that — PlantUML mode keeps its current constants.
- **Rollback:** if PUML mode at the new values somehow looks worse, revert by
  pointing the `_PUML` constant aliases back at the `_PLANTUML` values.
  Single constant flip per affected family.

---

## 8. Tickets to file (P0)

Three tickets recommended for this work:

1. **`feat(render): per-mode density wiring — plumb StyleMode into layout
   constants` (P0)** — the wiring change. No constant changes yet; just thread
   `StyleMode` through `FamilyDocument` → `render_class_artifact` /
   `render_box_grid_artifact` and add a `LayoutDensity` helper that returns
   the current PlantUML-tight values for both modes (no behavior change).
2. **`feat(render): per-mode density values for object + class — PUML chrome
   breathing room` (P0)** — flip the `_PUML` constants for the two highest-
   damage families to the values proposed in §5.2. Re-render audit corpus;
   gate on §6 acceptance criteria.
3. **`feat(render): per-mode density values for component + deployment —
   port lugs and 3D cube clearance` (P0)** — same as ticket 2 for the
   remaining two families. Separate ticket so checkpoint risk is bounded
   per PR.

Filed under labels: `P0`, `architecture`, `parity`, `visual-audit`,
`agent-ready`. Ticket numbers:

- **#1514** — wiring: plumb StyleMode into layout constants (no value change)
- **#1515** — per-mode values for object + class (highest-leverage)
- **#1516** — per-mode values for component + deployment

---

## 9. Honest verdict (TL;DR)

**Ship per-mode density constants ASAP.** Three small PRs (~1 wiring, 2
constant-set-and-iterate). PUML mode currently looks worse than its
pre-density-retune self on 35/149 audited fixtures (~23%), and the squish is
not subtle — kind badges are visibly stuck to the rounded corners of header
bands, port lugs are visibly clipped, and edge labels visibly overlap 3D cube
faces. The memory-note principle that "density is layout in both modes" is the
wrong abstraction once we accept that PUML chrome is wider than PlantUML
chrome. The correct principle is **per-mode layout density that suits each
mode's chrome**, with the invariant that within a mode the layout is fully
deterministic.

This forensic only documents and proposes. Implementation in a follow-up PR.
