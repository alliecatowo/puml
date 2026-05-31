# Edge routing — the three modes

Status: implemented (Stage 2 shipped in #1331; default reverted to Polyline in #1343;
Splines mode replaced with rounded-corner renderer in #1394; chen-ie ortho support
added in #1395).

PUML supports three global edge-routing modes, selected by
`skinparam linetype <value>`. The mode is per-diagram (not per-edge) and applies only
to Graphviz-routed families — class, object, usecase, component, deployment, ArchiMate,
C4. Bespoke families (sequence, activity new-syntax, state, timing, mindmap, WBS, salt,
nwdiag, gantt, json, yaml) draw their own geometry and are unaffected by this knob.

---

## Section 1 — What each mode does

### `Polyline` (the PUML default)

All three modes share the same waypoint set produced by the orthogonal channel router
in `src/render/graph_layout/router.rs`. In `Polyline` mode those waypoints are emitted
verbatim as a `<polyline points="…"/>` element — straight orthogonal line segments
through each router waypoint, no smoothing.

This is the default because it is the correct, stable representation of the channel
router's output. The default was `Splines` briefly after Stage 2 (#1331), but that
caused the catastrophic #1334 regression (wandering Béziers) and was reverted in #1343.
See the postmortem in `docs/internal/forensics/2026-05-30-splines-default-postmortem.md`.

### `Splines` (opt-in via `skinparam linetype splines`)

Uses the same waypoints as `Polyline`, but corners get **quarter-arc (quadratic Bézier)
chamfering** instead of sharp right-angle turns.

Algorithm (implemented in `src/render/edge_smoothing.rs:rounded_corner_path_d`):

1. `M p0`
2. For each interior waypoint `pᵢ` (i = 1 … N-2):
   - Compute the unit vectors coming in (`dir_in`) and going out (`dir_out`).
   - Set radius `r = CORNER_RADIUS` (8 px), capped to half the shorter adjacent
     segment to prevent overshoot.
   - Emit `L (pᵢ − r·dir_in)` — straight up to corner-minus-r.
   - Emit `Q pᵢ, (pᵢ + r·dir_out)` — quadratic arc through the corner point.
3. `L pN`

Endpoints are pinned to the routed source/target ports so arrowheads anchor correctly.
Two-point paths emit a plain `M … L …`. Straight-through collinear points produce no
visible arc (the two tangents are the same direction; the L/Q reduce to the straight
line).

**What this is NOT:** This is not a Graphviz B-spline. Graphviz's `splines=true` runs
a spline-channel algorithm that generates 3-5 cubic Bézier control points aware of
node bounding-box obstacles. Our Splines mode is a rounded-chamfer post-processor over
the channel router's rectilinear waypoints. The two produce different SVG but are
visually similar at typical diagram densities. We consciously chose the chamfer
approach after the #1334 investigation; see §5 (Non-goals) and the investigation doc
for full rationale.

The former Catmull-Rom / cubic Bézier implementation that shipped in #1334 has been
replaced. A backwards-compatible alias `cubic_bezier_path_d` still compiles but
delegates to `rounded_corner_path_d`.

### `Ortho`

For general Graphviz-routed families, `Ortho` behaves identically to `Polyline`: same
waypoints, same `<polyline>` element, same visual output. The distinction may diverge
in a future release if a diagonal-segment router variant is ever added.

For the **chen-ie (Entity-Relationship / IE notation) family**, `Ortho` switches angled
crow's-feet to orthogonal right-angle elbows, per the documented use case in
PlantUML §20.3. This is the _only_ family where `Ortho` visually differs from `Polyline`
today.

---

## Section 2 — PlantUML compatibility

### What is documented

The PlantUML 1.2025 Language Reference Guide mentions `skinparam linetype` exactly
**twice**, both in §20.3 ("Information Engineering Diagrams — Complete Example"):

> Currently the crows feet do not look very good when the relationship is drawn at
> an angle to the entity. This can be avoided by using the linetype ortho skinparam.

**`ortho` is the only value documented to users.** The guide contains no mention of
`splines` or `polyline` as `linetype` values. (Verified by full-text search of the
spec PDF — see investigation §Appendix B.)

### Undocumented values we support for upstream compatibility

The upstream Java parser (`DotStringFactory`) recognizes `polyline` and `splines` as
linetype tokens and maps them to Graphviz's `splines=polyline` and `splines=true`
directives. We support them for compatibility with diagrams written against the Java
implementation, but they are **undocumented features of upstream PlantUML**, not part
of the user contract.

| `skinparam linetype` token | Accepted aliases | PUML mode | Documented in spec? |
|---|---|---|---|
| `ortho`, `orthogonal` | — | `Ortho` | Yes — §20.3 |
| `polyline`, `poly`, `straight` | — | `Polyline` | No |
| `splines`, `spline`, `curve`, `curved` | — | `Splines` | No |
| *(anything else)* | — | *(no change; silent no-op)* | n/a |

Unknown values are a **silent no-op** — the current routing mode is unchanged. This
matches upstream PlantUML's fallback behavior (`getDotSplines()` returns the current
value for unrecognized tokens).

### Default linetype

PlantUML's (Graphviz) default is `splines=true` (smooth B-splines). PUML's default
is `Polyline`. These are intentionally different. We do not chase Graphviz B-spline
pixel parity; see §5 (Non-goals).

---

## Section 3 — Per-family routing matrix

| Family | Layout backend | `EdgeRouting` honored? | Notes |
|---|---|---|---|
| class | PUML channel router | Yes | `Polyline`/`Splines`/`Ortho` all active |
| object | PUML channel router | Yes | Same renderer path as class |
| usecase | PUML channel router | Yes | `Ortho` changes edges to rectilinear only |
| component | PUML channel router | Yes | Package frames don't shift between modes |
| deployment | PUML channel router | Yes | Nested packages identical across modes |
| C4 (built on component) | PUML channel router | Yes | Inherited |
| ArchiMate | PUML channel router | Yes | Same pipeline as component |
| chen-ie (ER / IE notation) | PUML channel router | Yes | **`Ortho` documented in PlantUML §20.3** — switches angled crow's-feet to right-angle elbows |
| state | Bespoke (`state/edges.rs`) | No | Edges are smooth curves drawn by state's own layout; `linetype` is ignored |
| activity (new-syntax) | Bespoke (`activity/arrows.rs`) | No | Orthogonal arrows are placed by activity's own grid layout; `linetype` is ignored |
| activity (legacy `:foo;` syntax) | PUML channel router | Yes (inherited) | Rarely used |
| sequence | Bespoke (slot-positioned) | No | Lifelines/arrows are message-slot-positioned; no routing |
| timing | Bespoke (time-axis grid) | No | Grid-based; no routing |
| gantt | Bespoke (calendar grid) | No | Calendar-positioned |
| mindmap | Bespoke (radial/horizontal tree) | No | Curved L-connectors are hand-emitted |
| WBS | Bespoke (top-down tree) | No | Tree connectors |
| nwdiag | Bespoke (network bus rows) | No | Network bus syntax |
| json / yaml | Bespoke (table) | No | Renders data as table |
| salt | Bespoke (UI mockup) | No | Form mock renderer |

Empirical confirmation across PlantUML 1.2026.5: for all bespoke families, node
positions are byte-identical across `default`/`polyline`/`ortho` modes, confirming
that `EdgeRouting` does not affect these renderers at all.

The callsites in our codebase that actually dispatch on `EdgeRouting` are:

- `src/render/family/box_grid_edges.rs` — component / deployment / usecase / ArchiMate / C4
- `src/render/family/class_relations.rs` — class / object

Bespoke family renderers never call `edge_geometry_attr` and are entirely independent
of the `EdgeRouting` enum.

---

## Section 4 — Reference docs

| Document | What it covers |
|---|---|
| `docs/internal/forensics/2026-05-30-splines-default-postmortem.md` | Root-cause analysis of the #1334 catastrophic regression; the Catmull-Rom algorithm mismatch |
| `docs/internal/forensics/2026-05-30-plantuml-parity-deep-audit.md` | Wave-2 parity deep audit |
| `docs/internal/forensics/2026-05-30-plantuml-parity-wave3-status.md` | Wave-3 parity status |
| `docs/internal/forensics/2026-05-31-plantuml-edge-routing-investigation.md` | Empirical investigation of per-family routing in PlantUML 1.2026.5; Allie's hypothesis refutation; algorithm comparison |
| `docs/internal/architecture/edge-curve-research-2026-05-29.md` | Earlier upstream Java references and the per-family verdict that motivated Stage 2 |

---

## Section 5 — Non-goals

These are explicit non-goals, not omissions:

**Pixel parity with Graphviz `splines=true` B-spline output is not a goal.**
Graphviz's spline router generates 3-5 cubic Bézier control points from a
spline-channel algorithm that is aware of node bounding-box obstacles. Our channel
router outputs orthogonal waypoints. Smoothing orthogonal waypoints with a curve-fitting
algorithm that ignores node obstacles is what caused the #1334 wander disaster. The
correct path to B-spline parity is a parallel spline router, which is a multi-PR sprint
(see investigation §4 Option B / Appendix C). We have intentionally deferred that work
because:

1. `splines` is not a documented `linetype` value in the PlantUML 1.2025 spec.
2. The rounded-corner approach delivers the "smooth arrows" feel without wander.
3. The cost/benefit of a real SplineRouter is too high relative to the PlantUML
   parity mandate (which only documents `ortho`).

**Splines-as-default is not being re-introduced.** The #1343 revert made `Polyline`
the default. That decision stands until a real SplineRouter exists and baselines are
vetted visually. The Stage 2 wander disaster is documented in the postmortem as the
reason.

**`EdgeRouting` mode does not affect node positions.** PlantUML (via Graphviz) performs
layout in phases A/B/C (rank, order, coord) independently of edge routing (phase D).
We follow the same architecture: the channel router produces waypoints from
already-fixed positions; `EdgeRouting` only affects SVG emission. Allie's hypothesis
that "different modes could produce different box positions" was empirically refuted
against PlantUML 1.2026.5 — verified byte-identical across class, usecase, deployment,
component, and state families.

**We do not implement layout-per-routing-mode.** Per
`memory/puml-mode-vs-plantuml-mode-principle.md`, LAYOUT must be identical across
modes for both PUML-mode and plantuml-mode rendering. Mode-conditional layout is out
of scope under current product policy.

---

## Where the code lives

- `src/render/graph_layout/router/contract.rs` — `EdgeRouting` enum and
  `parse_linetype` value parser.
- `src/render/graph_layout/router.rs` — re-exports `EdgeRouting` to the rest of
  the renderer.
- `src/render/edge_smoothing.rs` — `rounded_corner_path_d` (the post-#1394 Splines
  implementation), `polyline_points_attr`, and the `edge_geometry_attr` dispatch
  helper. Also contains the `cubic_bezier_path_d` backwards-compat alias.
- `src/model/family.rs` — `FamilyDocument::edge_routing` field carries the
  per-document mode.
- `src/normalize/family/directives.rs` — intercepts `skinparam linetype` during family
  normalization.
- `src/render/family/class_relations.rs` and
  `src/render/family/box_grid_edges.rs` — consult the mode and dispatch SVG emission.
- `tests/edge_routing_modes.rs` — end-to-end tests, one per mode plus case-insensitivity
  and unknown-value fallback.
- `tests/edge_routing_default_polyline_w14.rs` — regression tests confirming
  Polyline-default behavior.

---

## See also

- #1331 — Stage 2 EdgeRouting tracking issue (closed).
- #1334 — The catastrophic-default PR (closed; caused the regression).
- #1343 — Polyline-revert + safe re-lands (closed; made Polyline the default).
- #1394 — Replaced Catmull-Rom smoothing with rounded-corner quarter-arc renderer
  (closed; current Splines implementation).
- #1395 — chen-ie `linetype ortho` orthogonal crow's-feet support.
- #1390 — This doc rewrite issue.
- #590 — Renderer architecture and layout epic.
