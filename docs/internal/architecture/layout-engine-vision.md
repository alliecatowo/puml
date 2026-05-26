# PUML Layout Engine — Architectural Spike

> Goal: render diagrams that beat Mermaid and PlantUML at their own game. No JVM, no JavaScript runtime, no Graphviz — a single Rust binary that produces visually superior output to anything in the diagram-rendering ecosystem today.

## Where we are

PUML's renderers today are **single-pass grid layouts**. Each family in `src/render/<family>.rs` does its own ad-hoc positioning:

- **Sequence** (`src/render/sequence.rs`): lifelines positioned in declaration order; y-axis is message order. Works well — sequence diagrams are inherently 1-dimensional in both axes.
- **Family** (`src/render/family.rs` — class, object, usecase, component, deployment, c4): hardcoded 2–3 column grid. Wave 6-A added nested package containers. Wave 7-A added per-arrow L/Z-shape obstacle avoidance and reactive label de-collision.
- **State** (`src/render/state.rs`): Wave 5-B added composite enclosing boxes; Wave 7-B added BFS topological sort for top-level node ordering. Still single-column or 2-column grid for top-level.
- **Activity** (`src/render/activity.rs`): linear top-to-bottom with explicit fork/swimlane support added in waves 3-D and 8-C.
- **Specialized** (gantt, mindmap, wbs, chart, etc.): each is its own 1-D or tree layout — these mostly work because the domain is inherently 1-D.

The bottleneck is the **node-and-edge families** (component, class, state, deployment, c4, archimate, nwdiag). These produce real graphs — arbitrary node sets with arbitrary edges — and our grid + per-arrow routing can't compete with proper graph layout.

## The reference comparison: what Mermaid does

Mermaid flowcharts use **dagre.js**, a JS port of the Sugiyama hierarchical layout family. The pipeline:

1. **Rank assignment** (longest-path or network-simplex): assign each node to a horizontal rank/layer so all edges point downward.
2. **Crossing minimization** (barycenter heuristic, iterated): permute nodes within each rank to minimize edge crossings.
3. **Coordinate assignment** (Brandes-Köpf): set x-coordinates to minimize edge bends, with rank y-positions fixed.
4. **Edge routing**: route edges along the layered structure. Straight when possible, ortho-spline when not.
5. **Label placement**: position edge labels along their routed paths with de-collision.

Result: **edges flow predictably**, labels sit in space the layout reserved for them, crossings are minimized, the eye can follow the structure.

## The reference comparison: what Graphviz does

`dot` uses the same Sugiyama family but with more sophisticated edge routing (spline routing through node port positions). `neato`/`fdp` use force-directed layouts for non-hierarchical graphs. `osage` does packed cluster layouts.

## What we'd need to beat them

Beating the field means:

1. **Quality parity** on hierarchical diagrams (component, class, state machines).
2. **Determinism** — same input produces same output byte-for-byte. Graphviz/dagre have non-determinism from heuristic seeds; ours can be fully deterministic.
3. **Pixel-perfect output** — every label placed, no overflow, no clipping, ever. Real layout engines treat label placement as best-effort; we treat it as a hard constraint.
4. **Speed** — Rust native; should be 10–100× faster than dagre/Graphviz for typical diagrams.
5. **Themability** — every renderer-internal constant exposed as a skinparam.
6. **Embedding-first design** — SVG output is self-contained, no external font dependencies, copy-paste embed anywhere.

## Four-stage plan (#590)

### Stage 1 — Port assignment (~1 day, +40% crossing reduction)

For each arrow, pick which edge of the source/target box it attaches to (top/bottom/left/right) based on relative position. Currently we attach all arrows to box centers, producing crossed lines.

**Implementation**: in `src/render/family.rs` and `src/render/geometry.rs`, replace center-to-center attachment with edge-midpoint attachment based on `dx`/`dy` between centers (whichever is larger picks the orientation).

**Tickets**: file as #590 child.

### Stage 2 — Vendor hierarchical layout (~2–3 days, A-grade output)

Adopt a Rust hierarchical layout crate (or write a minimal one). Candidates:
- `layout-rs` — has Sugiyama-family layout, but maintenance status uncertain.
- `fdg` — force-directed graph, wrong family for hierarchical UML but good for arbitrary topology.
- Build minimal: rank assignment by longest-path topo-sort + barycenter ordering + simple coord assignment. ~500 LOC.

**Implementation**: extract a `src/layout/graph.rs` module with API:
```rust
pub struct GraphLayout {
    pub node_positions: HashMap<NodeId, Point>,
    pub edge_paths: HashMap<EdgeId, Vec<Point>>,
    pub canvas: Rect,
}

pub fn layout_hierarchical(
    nodes: &[(NodeId, Size)],
    edges: &[(EdgeId, NodeId, NodeId)],
    options: &LayoutOptions,
) -> GraphLayout;
```
Route component/class/state through this. Family renderers keep responsibility for shape drawing + label content but delegate positioning + routing.

**Tickets**: #590 children for each family migration (component → layout/graph, class → layout/graph, etc).

### Stage 3 — Orthogonal edge routing (~1 week, pixel-perfect)

Brandes-Köpf-style shared-channel orthogonal routing. Edges run along channels between ranks; multiple edges share channels with explicit spacing. Labels sit in the channels.

**Why this matters**: today's L/Z-shape avoidance is per-arrow; two arrows can still route through the same point. Channel routing reserves space.

**Implementation**: in `src/layout/routing.rs`. After Stage 2 places nodes in ranks, compute the channel grid between ranks, assign each edge a channel, route segments. Labels get an x-position along their channel.

### Stage 4 — Shared layout module + iteration (~2 weeks, surpass)

Extract `src/layout/graph.rs` as the canonical layout. All node-and-edge families (component, class, state, usecase, deployment, c4, archimate, nwdiag) delegate. Add an **iterative refinement** pass: layout → measure label collisions → adjust spacings → re-layout, capped at N iterations or stable fixed point.

**Why this matters**: real engines do single-pass and ship "good enough." Pixel-perfect requires iteration. Our determinism guarantee + Rust speed makes ≥100 iterations affordable.

## Aesthetic constraints (user's bar)

These become first-class invariants enforced in tests:

1. **No overlapping text, ever.** Two `<text>` elements never share canvas space. Enforce with a post-render bounding-box overlap check; treat failure as a render error.
2. **All arrows attach to box edges.** No arrow shaft passes through a box body. Enforce via a post-render obstacle-intersection check.
3. **All elements within canvas.** No clipping. Canvas auto-sizes after layout to fit + margin.
4. **Deterministic byte-stable output.** Same source = same SVG. Rendering is unconditionally deterministic (BTreeMap/sorted-key discipline).
5. **Aesthetic spacing.** Margins, gutters, line spacing — define a "design token" set in `src/theme/tokens.rs` and use everywhere.

## Files to read for context

- `src/render/family.rs` (2700 LOC) — the big one. Wave 7-A's L/Z routing is at the bottom.
- `src/render/state.rs` — best-shape example of family-specific composite handling.
- `src/render/sequence.rs` — best-shape example of single-axis layout.
- `src/layout.rs` — current "layout" module is mostly geometry helpers; needs to grow into a real layout module.
- `src/render/geometry.rs` — anchor/edge math helpers.
- `tests/visual_regression.rs` — golden PNG diff scaffolding.

## Next concrete steps

1. File breakdown tickets under #590 (Stage 1–4 each get an issue).
2. Implement Stage 1 (port assignment) in a focused wave.
3. Re-audit arch1, class/12_all_relations, state/05_fork_join_choice.
4. Begin Stage 2 prototype in a worktree.
5. Document deterministic-rendering invariants in `docs/internal/architecture/render-invariants.md` (separate doc).

## Inspiration / further reading

- Sugiyama, K., Tagawa, S., Toda, M. (1981). *Methods for visual understanding of hierarchical system structures*. The original paper.
- Gansner, E.R., et al. *A Technique for Drawing Directed Graphs*. The `dot` paper.
- Brandes, U., Köpf, B. (2002). *Fast and Simple Horizontal Coordinate Assignment*. The coord-assignment paper.
- The dagre.js source: https://github.com/dagrejs/dagre — modular pipeline worth reading.
- Graphviz source: https://gitlab.com/graphviz/graphviz — particularly `lib/dotgen/`.
- layout-rs: https://github.com/nadavrot/layout

## North star

If a contributor opens `docs/diagrams/architecture-overview.png` and says "huh, this is better than what Mermaid would produce" — and a developer reading the source agrees the code is clear — we've won.
