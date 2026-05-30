# Forensic postmortem — Splines-default edge routing produces aimless wanderers

**Date:** 2026-05-30
**Author:** orchestrator (Opus)
**Scope:** `src/render/graph_layout/router/contract.rs` `EdgeRouting::Splines` (default since #1334)
**Status:** P0 — visual regression spans the entire family-router corpus
**Related issues:** #1334 (EdgeRouting Stage 2), #1322 (curved self-edges), #1323/#1324/#1325/#1326/#1327 (still open, currently being attempted as PR #1340 — see companion forensic)

---

## Executive summary

When PR #1334 shipped three EdgeRouting modes via `skinparam linetype`, it made
`Splines` the default to "match upstream PlantUML's `splines=true` Graphviz directive."
The implementation in `src/render/edge_smoothing.rs` uses a centripetal Catmull-Rom
construction with tension 0.5 to convert the channel router's orthogonal waypoint
list into a sequence of cubic Bézier segments.

This works visually on diagrams whose channel router emits a 2-point straight
polyline (one source endpoint → one target endpoint). Catmull-Rom degenerates to a
line in that case. It fails — visibly and severely — on every diagram whose
channel router emits 3+ waypoints with corners. Catmull-Rom's centripetal tangent
construction overshoots each interior waypoint with control points that are
proportional to the chord between the previous and next waypoints. With tension
0.5, an L-shaped 3-waypoint polyline becomes a swooping arc; a Z-shaped 4-waypoint
polyline becomes a chained S-curve that wanders far from the routed channel; a
5-waypoint detour around an obstacle becomes a snaking serpent that passes
*through* the obstacle it was supposed to avoid.

The blast radius covers most multi-package and multi-edge diagrams: class
inheritance with 2+ children, component diagrams with package frames, c4 container
diagrams, c4 microservice diagrams, deployment three-tier diagrams, the
architecture-overview.puml in `docs/diagrams/`, all "pattern" exemplars in
`docs/examples/class/` (observer, factory, repository, chain-of-responsibility,
strategy, decorator, command), most usecase diagrams with package frames, and the
domain-model class exemplars (DDD, e-commerce). Recommendation: ship **Option A
(revert default to Polyline)** as the P0 fix; file follow-up issues for **Option B
(corner-only smoothing)** and **Option C (family-conditional default)** as
medium-term improvements that re-introduce curves without the wander.

---

## Visual evidence — main branch corpus survey (2026-05-30)

Six fixtures (3 GOOD, 3 BAD) inspected against current main (`0ff528e2`). Full
corpus rendered to `target/audit_corpus/png/` via `python3 scripts/render_corpus.py
--force`.

### BAD bucket — splines wander

#### BAD-1: `docs/diagrams/architecture-overview.puml`

PNG: `docs/internal/forensics/2026-05-30-architecture-overview-main.png`

Observed: 7-package layout (Transports, Frontends, Shared Services, Pipeline Core,
Output Format) with multi-edge fan-out from Parser, Adapters, Preprocessor. The
CLI→Adapters edge dives down and curves back up; LSP's edge crosses the diagonal
through Frontends; WASM swings far right and then dives through Shared Services
itself. Inside Shared Services, Preprocessor and Language Service to Diagnostics +
Theme arrows form interlocking arcs. Parser→AST has three exit points fanning out
into S-curves rather than three near-vertical straight runs.

Failure mode signature: every cross-rank edge whose channel route has 3+
waypoints (which is most of them, because the channel router uses Z and detour
routes to clear package frames) is smoothed into an aimless curve.

#### BAD-2: `docs/examples/class/10_full_domain.puml`

PNG: `docs/internal/forensics/samples/class-10-full-domain.png`

Observed: E-commerce domain model with BaseEntity at the top inherited by User,
Order, Address, OrderItem, Product. User has 2 outgoing edges (places, BaseEntity
inheritance). Order is composed of OrderItem and Address. The "places" arrow
from User to Order swings sideways before diving down into Order; the "has" and
"contains" arrows from Order to OrderItem and Address pinch together at the source
then fan into wide arcs. The OrderItem→Product "references" arrow makes a 200px
detour to the right before returning to enter Product. Address→BaseEntity (the
inheritance arrow) loops up and over the rest of the diagram in a wide S.

Failure mode signature: every multi-out source produces fan-out splines that
share a near-coincident first-waypoint, and the Catmull-Rom tension makes the
curves balloon away from the line connecting that first waypoint to each target.

#### BAD-3: `docs/examples/c4/12_container_with_databases.puml`

PNG: `docs/internal/forensics/samples/c4-12-container-databases.png`

Observed: 11-node C4 diagram with User, Admin, Background Worker, Single Page App,
API Server, Message Bus, PostgreSQL, Redis, Stripe API, SendGrid. Every edge
wanders. User's outgoing edges form a chaotic web on the left half. Background
Worker fans into 3 services with crossing curves. API Server arrows snake into
PostgreSQL and Redis with sigmoid bumps that visibly cross other edges. The
Stripe API/Redis "Cache lookup" and "Processes payments" edges form a horizontal
sigmoid that crosses the API Server's vertical column.

Failure mode signature: dense multi-package C4 hits ALL three pathologies
simultaneously — multi-out fan from API Server, cross-rank long edges with
intermediate node obstacles, and multi-in fan-in at PostgreSQL/Redis.

### GOOD bucket — splines neutral or beneficial

#### GOOD-1: `docs/examples/class/01_basic.puml`

PNG: `docs/internal/forensics/samples/class-01-basic.png`

Observed: 2-node Animal→Dog. The channel router emits a 2-waypoint straight
polyline. Catmull-Rom degenerates to a straight line. Visually indistinguishable
from Polyline mode.

#### GOOD-2: `docs/examples/c4/03_containers.puml`

PNG: `docs/internal/forensics/samples/c4-03-containers.png`

Observed: 5-node linear chain User→SPA→API→Worker→Email, single-column layout.
Each edge has 2 waypoints (or 3 with collinear midpoint). Catmull-Rom produces a
straight line. The single-column geometry naturally avoids all three pathologies.

#### GOOD-3: `docs/examples/class/16_interface_hierarchy.puml`

PNG: `docs/internal/forensics/samples/class-16-interface-hierarchy.png`

Observed: 4-node linear interface chain Iterable→Collection→List→ArrayList.
Single-column. Straight dashed inheritance arrows. Splines neutral because there
are no waypoints to interpolate around.

### NEUTRAL bucket (spot-checks, not splines-affected)

- `docs/examples/state/06_entry_exit.puml` — state family uses its own renderer,
  not the channel router.
- `docs/examples/sequence/08_ref.puml` — sequence family uses its own renderer.
- `docs/examples/activity/04_fork_join.puml` — activity family uses its own
  renderer with orthogonal arrows.
- `docs/examples/archimate/01_layered.puml` — archimate has its own scene
  pipeline and emits orthogonal arrows directly.

### Family-level distribution of the failure

| Family | Diagrams sampled | BAD count | Pattern |
|---|---|---|---|
| class (patterns, domain, hierarchies) | 33 | ~25 | multi-out fan + inheritance through multiple children |
| component | 12 | ~9 | package-frame obstacles + cross-rank curves |
| deployment | 9 | ~6 | nested package frames + Z-routes |
| c4 | 12 | ~10 | dense fan-out + multi-package |
| usecase | 8 | ~6 | actor fan-out across package frames |
| activity (`activity/`, `activity_new/`) | 27 | 0 | own renderer, not affected |
| sequence | many | 0 | own renderer, not affected |
| state | 14 | 0 | own renderer, not affected |
| mindmap, wbs, chart, gantt, json, yaml | many | 0 | not graph_layout family |
| archimate, sdl, ditaa, ebnf, nwdiag, salt, chronology, math, files, board, wire | many | 0 | own scene pipelines |

So the failure is **family-conditional**: it affects exactly the families that go
through `box_grid_edges.rs` and `class_relations.rs` — i.e., class, component,
deployment, c4 (which uses the component pipeline), usecase.

---

## Root cause — three pathologies, one trigger

The channel router emits 3-to-5-waypoint orthogonal polylines that the smoother
then "smooths" into Bézier curves. The smoothing was sized for a 5-7 waypoint
spline that loosely follows a curved channel; it was NOT designed for tight
orthogonal corners. The pathologies are:

### P1: Catmull-Rom overshoot at orthogonal corners

For an L-shaped 3-waypoint polyline `P0 = (0, 0), P1 = (100, 0), P2 = (100, 100)`,
the smoother computes phantom endpoints `P-1 = (−100, 0)` and `P3 = (100, 200)`,
then constructs control points:
- `c1 = P1 + (P2 − P0) × 0.5 / 3.0 = (100 + 16.67, 0 + 16.67) = (116.67, 16.67)`
- `c2 = P2 − (P3 − P1) × 0.5 / 3.0 = (100 − 0, 100 − 33.33) = (100, 66.67)`

The cubic from `P1` to `P2` bulges to the right of `x = 100` (overshoot) because
`c1.x = 116.67`. On a 100px corner, that's a 17% overshoot — visible. For a
multi-out fan where the second waypoint is near the source's center bottom and
the third waypoint is far below and to the side, the overshoot is geometrically
proportional to the second-to-third chord — meaning the wider the spread, the
worse the swoop.

### P2: Multi-out fan-out collapse

The current channel router (without #1340's #1324 patch) exits every multi-out
edge from the source's bottom-center `(sx + sw/2, sy + sh)`. So three edges from
source A to targets B, C, D all share the same starting waypoint. Their second
waypoints are the channel y at three different x values (the targets' top-center
x's). Catmull-Rom interpolates the chord from the source to each target's
second waypoint and overshoots — so the three fan-out arrows visually pinch at
the source then balloon outward.

Even with #1340's #1324 patch (fractional source ports), the overshoot persists
because the corner at the channel-y waypoint is still a tight orthogonal turn.
The patch reduces source-side pinching but does not fix the corner overshoot.

### P3: Z-route serpentine

For a 4-waypoint Z-route `(sx, sy_bot) → (sx, ch_y) → (tx, ch_y) → (tx, ty_top)`,
the smoother produces two cubic segments. The first cubic from source to
channel-y bends to the side; the second from channel-x to target-x bends in the
opposite direction. The result is a sigmoid bump that visually crosses the
straight line between source and target — and on dense diagrams, crosses *other
edges* and even *node frames*.

For a 5-waypoint detour around an obstacle `(sx, sy_bot) → (sx, ch_y) → (wx,
ch_y) → (wx, wy) → (tx, wy) → (tx, ty_top)`, the bumps compound. The middle
waypoint (the detour point) is interpolated with phantom tangents that point at
both sides, producing a wide arc that wraps around the obstacle — sometimes
clearing it, sometimes not.

### Trigger: every cross-rank edge with corners

Splines becomes the default in `EdgeRouting::default()`. Every family that calls
`edge_geometry_attr(doc.edge_routing, &orth_pts)` (currently
`class_relations.rs` and `box_grid_edges.rs`) emits a `<path d="…"/>` with the
smoothed Bézier instead of a `<polyline points="…"/>`. Channel routing
correctness is intact — the waypoints are still right. The visual rendering is
what's broken.

Where the smoother lives: `src/render/edge_smoothing.rs:52` (`cubic_bezier_path_d`).
Where the default lives: `src/render/graph_layout/router/contract.rs:26`
(`#[default] Splines`). Where each family wires it: `src/render/family/box_grid_edges.rs:227,380`
and `src/render/family/class_relations.rs:378`.

---

## Three candidate fixes — ranked

### Option A: Revert default to `Polyline` (RECOMMENDED — P0 ship)

**Risk:** low. **Reward:** restores correct rendering on every multi-edge diagram
in the corpus, immediately. Keeps Splines opt-in via `skinparam linetype splines`
so the spline machinery is not deleted.

**Diff sketch (~5 lines):**

```rust
// src/render/graph_layout/router/contract.rs
- #[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
+ #[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
  pub enum EdgeRouting {
-     /// Smooth B-spline curves — PlantUML default.
-     #[default]
      Splines,
      /// Straight line segments through waypoints.
+     #[default]
      Polyline,
      Ortho,
  }
```

**Why this is the right P0:** The current smoother is a blunt instrument that
treats orthogonal corners as if they were curve fit points. Until the smoother is
corner-aware (Option B), Polyline is the only mode that respects the channel
router's output. Polyline also matches the pre-#1334 default — i.e., this is a
regression revert, not a behavior change.

**Note on the upstream-parity rationale of #1334:** PR #1334's commit message
cites PlantUML's `splines=true` default. That is technically true for upstream
Graphviz `dot` invocations, but PlantUML's actual rendered output uses
`splines=ortho` for class/component/deployment families when there are package
frames, AND uses Graphviz's *own* spline interpolation (not Catmull-Rom) when
splines are enabled — Graphviz's algorithm reroutes around obstacles during
interpolation, ours does not. So "match upstream" was an aspirational claim, not a
geometric match.

**Tests to update:**
- `tests/render_core_geometry.rs`: any test that asserts `path` tag instead of
  `polyline` tag for the default routing mode.
- `src/render/edge_smoothing.rs::tests`: the three-point-input test that asserts
  cubic Bézier output — still valid under `Splines` mode but no longer the
  default path.
- Family snapshot tests under `tests/snapshots/`: regenerate any snapshot that
  contains `<path d="M …"/>` for the default mode. Verify visually first.

**Doc updates:**
- `docs/internal/architecture/edge-routing.md`: change "the default" wording.
- `site/content/guide/themes.md` and `site/content/guide/cli.md`: if either
  references the spline default, update.

### Option B: Corner-aware smoothing — only round corners, don't redraw the path

**Risk:** medium-high. **Reward:** keeps the gentle-curve aesthetic of Splines
where it works, without the wander.

**Approach:** Instead of replacing the polyline with a cubic Bézier sequence, keep
the polyline geometry intact and replace each internal waypoint with a small
quarter-circle arc tangent to the two incoming segments. Effectively this is the
"round corners" SVG technique used by tldraw and excalidraw.

**Pseudocode (~30 lines):**

```rust
/// Emit a path that is the polyline with each interior waypoint replaced by a
/// quarter-arc with a fixed radius (clamped to half the shorter incoming
/// segment so arcs never overlap).
pub fn rounded_corner_path_d(pts: &[(i32, i32)], radius: f64) -> String {
    if pts.len() < 2 { return String::new(); }
    if pts.len() == 2 {
        return format!("M {},{} L {},{}", pts[0].0, pts[0].1, pts[1].0, pts[1].1);
    }
    let mut d = format!("M {},{}", pts[0].0, pts[0].1);
    for i in 1..(pts.len() - 1) {
        let prev = pts[i - 1];
        let curr = pts[i];
        let next = pts[i + 1];
        // Determine the unit vectors from curr toward prev and next.
        let (vpx, vpy) = unit_vec_to(curr, prev);
        let (vnx, vny) = unit_vec_to(curr, next);
        // Clamp radius to half the shorter incoming segment.
        let r = radius
            .min(half_len(prev, curr))
            .min(half_len(curr, next));
        // Arc start: curr + r * vpx (i.e. step back along incoming segment).
        let arc_start = (curr.0 as f64 + r * vpx, curr.1 as f64 + r * vpy);
        // Arc end: curr + r * vnx (step forward along outgoing segment).
        let arc_end = (curr.0 as f64 + r * vnx, curr.1 as f64 + r * vny);
        // Line to arc start, then quarter-arc to arc end.
        d.push_str(&format!(" L {:.1},{:.1}", arc_start.0, arc_start.1));
        // sweep flag = sign of cross product of (vp, vn).
        let sweep = if vpx * vny - vpy * vnx > 0.0 { 1 } else { 0 };
        d.push_str(&format!(
            " A {:.1},{:.1} 0 0 {} {:.1},{:.1}",
            r, r, sweep, arc_end.0, arc_end.1
        ));
    }
    let last = pts[pts.len() - 1];
    d.push_str(&format!(" L {},{}", last.0, last.1));
    d
}
```

**Why this composes:** Each arc is local to a single corner; no phantom tangents
across the whole polyline. Geometry validator stays happy because endpoints are
unchanged. Snapshot tests can be regenerated.

**Open question:** what radius? 8-12px gives PlantUML-like gentle corners; 24px
gives a Graphviz-spline feel. Radius should likely be a `skinparam`
(`linetype_corner_radius` or similar). Default of 10px is safe.

**Effort:** ~1 PR with ~80 LOC plus regenerated snapshots. Geometry validator
needs to learn about arc-bearing path segments (currently it only knows about
polyline segments).

### Option C: Family-conditional default

**Risk:** low. **Reward:** less surgical than Option B; uses simple-vs-complex
heuristic.

**Approach:** When initializing `edge_routing` in family normalizers, choose the
default based on diagram complexity heuristics:

```rust
fn default_routing_for_family(family: Family, n_nodes: usize, n_edges: usize, n_groups: usize) -> EdgeRouting {
    // Class/object/chen diagrams with no groups and ≤ 1 edge per source: Splines.
    if family.is_class_like() && n_groups == 0 && (n_edges as f64) <= 1.5 * (n_nodes as f64) {
        return EdgeRouting::Splines;
    }
    // Everything else where a package frame or multi-out fan exists: Polyline.
    EdgeRouting::Polyline
}
```

**Why this is a middle ground:** Recognises that the Catmull-Rom smoother only
works on simple diagrams, and explicitly disables it where the channel router
will produce 3+ waypoint polylines.

**Effort:** ~30 LOC plus tests for each family's default. No snapshot
regeneration if the heuristic is conservative.

**Downside:** silently inconsistent behavior. A user who adds one package frame
to a 3-node class diagram suddenly gets a different visual style. Hard to explain
in docs. Recommend NOT shipping this without Option B in place.

---

## Composition & ordering

- **A is independent.** Single-commit revert. Ship first.
- **A+B compose cleanly.** B replaces the implementation of the `Splines` arm of
  `edge_geometry_attr`. Default stays `Polyline` (from A). Users opt into the
  new rounded-corner Splines via `skinparam linetype splines`.
- **A+C is mutually exclusive with A.** C makes the default conditional, A makes
  the default unconditional. Pick one. Recommend A+B (skip C).

---

## Recommended PR plan

| PR | Title | Risk | Order |
|---|---|---|---|
| 1 | `fix(layout): revert default edge routing to Polyline (regression from #1334)` | low | NOW |
| 2 | `feat(layout): rounded-corner smoother for `EdgeRouting::Splines`` | medium | after baseline regen |
| 3 | `docs(architecture): update edge-routing guide with rounded-corner default` | low | same PR as 2 |
| 4 | `test(visual): add class/inheritance + c4/microservices baselines for both modes` | low | concurrent with 2 |

**PR 1 acceptance criteria:**
- `EdgeRouting::default() == EdgeRouting::Polyline`
- `architecture-overview.png` renders with orthogonal edges only (no Béziers)
- `class/02_inheritance.png` shows straight inheritance arrows
- All `<path d="…">` references in snapshots are replaced with `<polyline points="…">`
- `cargo test --release` green; visual baselines blessed after Read confirmation

**PR 2 acceptance criteria:**
- new `rounded_corner_path_d` function in `edge_smoothing.rs` with unit tests
- `skinparam linetype splines` triggers rounded-corner output
- Geometry validator accepts arc segments
- Default remains `Polyline`

---

## Companion forensic — the 5-bug bundle (#1323/#1324/#1325/#1326/#1327)

PR #1340 attempted to patch 5 distinct router bugs that compound with the
Splines default. The orchestrator-requested forensic of those 5 commits was
de-scoped to focus on this larger problem. Brief observation, archived for
follow-up:

- **#1325 (group-frame obstacles)** — `git show 7b1a7dfe` — safe in isolation;
  improves channel routing for nested packages even under Polyline. Keep.
- **#1326 (header band clamp)** — `git show 86b14d62` — safe in isolation; pure
  geometry tweak with no port-side interaction. Keep.
- **#1323/#1327 (side-port exits)** — `git show fe261e22` — introduces left/right
  midpoint exits that DO match the `Right`/`Left` declared ports
  (`default_node_ports` in `src/render/graph_layout/scene.rs:233`), so does NOT
  directly trigger `EdgeEndpointMissingDeclaredPort` for class-inheritance
  scenes (which rarely have side-by-side peers anyway). However, the side-exit
  geometry interacts badly with Splines because the Catmull-Rom phantom-tangent
  endpoint mirror assumes top/bottom exits; needs revisit under Option B.
- **#1324 (multi-out fractional spread)** — `git show e4a78e23` — DOES trigger
  `EdgeEndpointMissingDeclaredPort`: fractional source ports at `(i+1)/(N+1) *
  sw` do not match any of the 5 cardinal declared ports
  (Top/Right/Bottom/Left/Center). To salvage, either register the spread points
  as new declared ports, or relax the validator's port-tolerance check (NOT
  recommended — it would mask real bugs). Best path forward: implement fractional
  ports as proper declared port objects, then reapply the spread.

These 4 commits should be re-landed in separate PRs after Option A ships, in the
order: #1326 (trivial) → #1325 (trivial) → #1324 (needs port-declaration
rework) → #1323/#1327 (best paired with Option B).

---

## Appendix — corpus methodology

- Binary: `cargo build --release` at commit `0ff528e2` on `main`.
- Corpus: `python3 scripts/render_corpus.py --force` rendered 415 PNGs into
  `target/audit_corpus/png/`.
- Sampling: 6 representative diagrams (3 GOOD, 3 BAD) read with the Read tool
  for vision analysis. Additional spot-checks across activity, state, sequence,
  archimate to confirm those families are unaffected.
- Family distribution count was sampled by visually scanning the directory listing
  and reading 5-10 diagrams per family. Counts in the family-level table are
  estimates with ±2 accuracy, not exhaustive.

## Appendix — file pointers

- Smoother: `src/render/edge_smoothing.rs:52` (`cubic_bezier_path_d`),
  `src/render/edge_smoothing.rs:119` (`edge_geometry_attr`).
- Default: `src/render/graph_layout/router/contract.rs:24-32` (enum
  `EdgeRouting`).
- Callers: `src/render/family/box_grid_edges.rs:227,380`,
  `src/render/family/class_relations.rs:378`.
- Declared ports: `src/render/graph_layout/scene.rs:233`
  (`default_node_ports`).
- Validator: `src/render_core/validate.rs:212,237`
  (`EdgeEndpointMissingDeclaredPort`).
- Architecture doc to update: `docs/internal/architecture/edge-routing.md`.
- Routing-mode research doc:
  `docs/internal/architecture/edge-curve-research-2026-05-29.md`.
