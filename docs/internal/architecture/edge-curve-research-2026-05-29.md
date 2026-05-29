# Edge curve research — 2026-05-29

**Author:** research agent (Claude Opus 4.7, 1M context)
**Trigger:** Allie feedback on PR #1322 — "the circular arrows ... its a little fucked up.
its not what I was talking about at all."
**Reference issue:** #1331 — *Long-distance feedback / cancellation edges should render as
sweeping curved arcs*.
**Method:** Read the PlantUML reference PDF (`docs/internal/spec/PlantUML_Language_Reference_Guide_v1.2025.0.pdf`,
607 pages) cover-to-cover for any geometry mention; cross-checked stdlib (`stdlib/C4/`,
upstream stdlib via `gh search code`); read the upstream Java renderer
(`plantuml/plantuml@master`) for the authoritative definition of `skinparam linetype`.
**Hard constraint observed:** stay strictly within "what does PlantUML actually do".
No invented heuristics (e.g. ">300 px endpoints", "≥ 3 turns"). When PlantUML's behavior
is empirical (Graphviz output we don't control), this doc says so plainly.

---

## Executive summary

PlantUML delegates edge geometry to **Graphviz** for every diagram family that has a
visible graph layout (class, object, use-case, component, deployment, state,
activity-legacy, archimate, IE, C4, ArchiMate, etc.). The single user-facing knob is
`skinparam linetype`, which has three documented values mapping 1-to-1 onto Graphviz's
`splines=` attribute:

| `skinparam linetype` | Graphviz directive | What Graphviz emits | When you'd use it |
|---|---|---|---|
| (unset — default) | (no directive — Graphviz default is `splines=true`) | Smooth Bézier splines along edge channels | Default UML appearance; most diagrams |
| `polyline` | `splines=polyline;` | Straight line segments through computed waypoints (no curve through bends) | "I want straight, jagged-acceptable edges" |
| `ortho` | `splines=ortho; forcelabels=true;` | Pure right-angle orthogonal routing | Entity-relation diagrams with crows-feet, or anyone who wants grid-clean edges |

Source: `net.sourceforge.plantuml.skin.SkinParam.getDotSplines()` and
`net.sourceforge.plantuml.svek.DotStringFactory` in upstream PlantUML master.
Spec PDF §20.3 (p. 437–439) confirms `ortho` is the documented opt-in for ER diagrams.
**No other linetype values exist in upstream PlantUML.**

**This means PlantUML's "curved" rendering is not a per-edge heuristic** — it's the
*default Graphviz spline router applied to every edge in the graph*. Long feedback
edges look like sweeping arcs because Graphviz lays them out that way; short adjacent
edges look near-straight for the same reason. Users do not select "curve" per edge,
nor does PlantUML pick "curve here, ortho there" based on edge classification. It's
**one global mode**, set at the diagram level.

**Implication for our renderer.** Our channel router (in
`src/render/graph_layout/router.rs`) currently *always* produces orthogonal polylines.
We have **no equivalent of `splines=true` (the PlantUML default)** and **no equivalent
of `splines=polyline`**. The only mode we render is approximately equivalent to
`splines=ortho`. That is the root gap — both vs PlantUML and vs Allie's screenshots.

PR #1322 (currently in draft) tries to bolt curves onto specific *self-edges*
(sequence, class, state, activity back-edge). That's tangential to the main gap.
Sequence self-message D-shape and class self-association C-shape match upstream
PlantUML behavior; the state self-transition rewrite and activity back-edge corner-
rounding don't match any documented PlantUML behavior and should be re-examined or
narrowed.

**Recommended scope (verdict in one line):** add a global `EdgeRouting` mode to our
router contract (`Splines | Polyline | Ortho`), default it to `Splines`, route via
B-spline interpolation through the channel waypoints we already compute. Don't invent
edge-classification heuristics. Keep the `c2e271c7` anchoring fix and the two
self-edge fixes that match upstream PlantUML behavior (sequence D-shape, class C-shape);
roll back or narrow the state self-transition and activity back-edge changes.

---

## What the spec PDF actually says (chapter by chapter)

The PlantUML reference guide is **geometry-agnostic** for the entire 607-page
document, with exactly one geometry mention:

> §20.3 Complete Example, p. 437:
> ```
> ' avoid problems with angled crows feet
> skinparam linetype ortho
> ```
> p. 439: "Currently the crows feet do not look very good when the relationship is
> drawn at an angle to the entity. This can be avoided by using the **linetype
> ortho** skinparam."

That's the **only** time the spec mentions edge geometry. It implies:

1. PlantUML's default routing is something other than `ortho` (otherwise the
   `linetype ortho` knob would be pointless).
2. The default produces "angles" — i.e. it's not orthogonal. Graphviz `splines=true`
   (smooth curves) is exactly that.

Every other chapter only documents arrow **syntax** (`->`, `-->`, `<|--`, `..>`,
`*--`, `o--`, etc.), arrow **direction hints** (`-up->`, `-down->`, `-l-`, `-r-`),
and arrow **decoration** (`#color`, `bold`, `dashed`, `dotted`, arrowhead `o`, `x`).
None of these control routing geometry.

Spec sections that *could* mention edge geometry but don't:

- §1.1–1.45 Sequence — only describes message tokens and the "Itself arrow" §1.39.2.
- §3.2 Class relations — only the relation-symbol table.
- §6.6.2 / §6.9.2 Activity back-edges — only describes `backward` keyword; geometry
  is whatever the bespoke activity renderer produces.
- §7 Component — direction hints only; default Graphviz layout otherwise.
- §9.4 / §9.7 State — direction hints, composite/concurrent structure; no geometry.
- §27.5 C4 stdlib — `Rel(...)` macros expand to `--> :` (standard Graphviz arrow).

**Therefore**: PlantUML's per-family curve behavior is **empirically** whatever
Graphviz `splines=true` produces for the layout that PlantUML's `DotStringFactory`
builds. There is no PlantUML-side specification of "this kind of edge curves, that
kind doesn't".

---

## Authoritative source for `skinparam linetype` — upstream Java

`src/main/java/net/sourceforge/plantuml/skin/SkinParam.java`, method `getDotSplines()`:

```java
@Override
public DotSplines getDotSplines() {
    final String value = getValue("linetype");
    if ("polyline".equalsIgnoreCase(value))
        return DotSplines.POLYLINE;
    if ("ortho".equalsIgnoreCase(value))
        return DotSplines.ORTHO;
    return DotSplines.SPLINES;
}
```

`src/main/java/net/sourceforge/plantuml/dot/DotSplines.java`:

```java
public enum DotSplines {
    POLYLINE, ORTHO, SPLINES
}
```

`src/main/java/net/sourceforge/plantuml/svek/DotStringFactory.java`:

```java
final DotSplines dotSplines = skinParam.getDotSplines();
if (dotSplines == DotSplines.POLYLINE) {
    sb.append("splines=polyline;");
} else if (dotSplines == DotSplines.ORTHO) {
    sb.append("splines=ortho;");
    sb.append("forcelabels=true;");
}
// default SPLINES: no directive emitted — Graphviz default (splines=true / smooth)
```

That is **the entire surface area** PlantUML exposes for edge routing geometry.
Default = Graphviz spline (curved B-splines). `polyline` = straight segments through
waypoints. `ortho` = right-angle.

Verified absent from upstream:
- No `linetype curve` keyword (despite Allie's intuition).
- No `linetype straight` keyword (there is an upstream forum feature request to add
  one, but it has not been merged).
- No `skinparam EdgeRouting` or `skinparam ArrowStyle` knob controlling geometry.
- No per-edge classification (back-edge / cancellation / exception) used by PlantUML
  to choose routing — Graphviz decides.

This rules out the heuristic I proposed in an earlier draft (">300 px endpoints
trigger Bézier") — PlantUML doesn't do that, and Allie correctly stopped me from
inventing it.

---

## Per-family verdict (what PlantUML actually emits)

For each family this section says (a) which rendering backend handles edges
(Graphviz vs bespoke), (b) what edge shapes the user actually sees, and (c) whether
any of it is *specified* or merely *emergent from Graphviz*.

### 1 · Sequence (Spec Ch 1, pp. 1–47) — bespoke renderer

- Messages between distinct lifelines: **straight horizontal segments.** Specified
  by the sequence renderer; not Graphviz-routed.
- Self-messages (`A -> A`, §1.5 p. 5; §1.39.2 p. 43): **"itself arrow"** — vertical
  out, semicircular arc to the right, vertical back to the lifeline with arrowhead
  at the tip. This shape is *defined by the upstream sequence renderer* (file
  `net.sourceforge.plantuml.sequencediagram.graphic.ArrowComponentImpl`).
- Slanted arrows (`->(nn)`, §1.44 p. 45): straight at an angle. Specified.
- Parallel teoz (§1.45 p. 47): straight horizontal at same y. Specified.
- Lifeline (vertical bar): straight vertical. Specified.

**Verdict**: PR #1322's `44b25f89` (sequence D-shape self-message) matches upstream
exactly. **Keep.**

### 2 · Use case (Spec Ch 2, pp. 48–60) — Graphviz

All actor↔usecase and usecase↔usecase relations rendered by Graphviz with default
`splines=true`. Long arcs sweep, short arcs come out near-straight. Direction hints
(`-up->`) influence Graphviz layout but not routing style.

**Verdict**: Not touched by PR #1322. Our renderer currently produces orthogonal
polylines — different from upstream. Falls under the universal `EdgeRouting=Splines`
gap below.

### 3 · Class (Spec Ch 3, pp. 61–104) — Graphviz

All relations (`<|--`, `*--`, `o--`, `-->`, `..>`, `..|>`, etc.) routed by Graphviz
default splines. Self-association edges (`A *-- A`) are rendered by Graphviz as
small self-loops — Graphviz emits its standard self-edge geometry (C-shape on one
side of the node).

**Verdict**: PR #1322's `e987762f` (class self-association C-shape arc) is consistent
with Graphviz's self-loop rendering. **Keep.** All other class edges fall under the
universal gap.

### 4 · Object (Spec Ch 4, pp. 105–111) — Graphviz

Same as class.

### 5 · Activity legacy (Spec Ch 5, pp. 112–121) — Graphviz

Same — fully Graphviz-routed. Spline by default.

### 6 · Activity new syntax (Spec Ch 6, pp. 122–163) — bespoke renderer

Upstream says "No Dependency on Graphviz" (§6.0.1, p. 122). The new-syntax activity
renderer is fully bespoke. Edge shapes:

- Forward arrows: **straight vertical.** Specified.
- Back-edges from `while ... endwhile`, `repeat ... repeat while`, `backward:`:
  upstream draws these as **a loop on the right side of the loop body**, returning
  to the top of the loop guard. The exact shape (single Bézier vs polyline with
  rounded corners vs sharp 90°) is **implementation detail of the upstream Java
  renderer** — the spec does not specify it. Visually upstream produces a *gently
  rounded* polyline, not a single sweep; sharp 90° corners only on very short
  back-edges.

**Verdict**: PR #1322's `391da8bc` (activity back-edge rounded corners) is
**plausibly consistent** with upstream's appearance — but since the spec doesn't
pin down the exact shape, this is a visual-fidelity decision, not a correctness one.
Recommend keeping the rounded-corner stopgap **conditional** on a visual comparison
against upstream output for the same fixture. If upstream's back-edge is visually
indistinguishable from a straight polyline with rounded corners, then `391da8bc`
matches. If upstream sweeps smoothly, then `391da8bc` is wrong.

### 7 · Component (Spec Ch 7, pp. 164–181) — Graphviz

Default `splines=true`. Same universal gap as class.

### 8 · Deployment (Spec Ch 8, pp. 182–230) — Graphviz

Same.

### 9 · State (Spec Ch 9, pp. 231–255) — Graphviz

All state transitions Graphviz-routed. Self-transitions (`A --> A`, §9.4 p. 234):
Graphviz's standard self-loop on one side of the node.

**Verdict**: PR #1322's `95685c19` (state self-transition cubic-bezier exiting right
edge and returning to top edge) does **not** match Graphviz's standard self-loop
shape, which is a small loop on a single side (Graphviz centers the self-loop on
one corner). The PR introduces a custom shape that has no documented basis in either
the spec or upstream behavior. **Revert and reimplement** as a small single-side
loop, mirroring the class self-association arc emitted by `e987762f`.

All other state edges (cross-region, back-up, exit-to-final) fall under the
universal `EdgeRouting=Splines` gap.

### 10 · Timing / 11 · JSON / 12 · YAML / 13 · nwdiag / 14 · Salt / 16 · Gantt / 17 · Mindmap / 18 · WBS — bespoke

None of these use Graphviz for edges. Their edge shapes are fully specified by
their bespoke renderers:

- Timing: rectilinear by clock domain.
- JSON / YAML: straight connector lines in a tree layout.
- nwdiag: orthogonal grid.
- Salt: no edges (widget layout).
- Gantt: dependency arrows are straight or step-like depending on date offset; spec
  doesn't pin it.
- Mindmap / WBS: radial branches, not routed edges.

**Verdict**: no changes needed in any of these; not touched by PR #1322.

### 15 · ArchiMate (Spec pp. ~350) — Graphviz

Same as component.

### 19+ — n/a

Maths / IE (covered above for linetype) / Creole / sprites / preproc / unicode /
stdlib are not edge-rendering chapters. C4 is covered under §27.5 and uses
Graphviz via the standard `-->` macro expansion (`stdlib/C4/C4.puml` line ~48:
`!procedure Rel($from, $to, $label, $tech="") $from --> $to : $label`).

---

## Where Allie's screenshots fit

Allie's reference images in #1331 ("test failed", "in review", "review done",
"Cancelled", "Student Dropped") are all **state diagrams** rendered by upstream
PlantUML in its default mode — i.e. with Graphviz `splines=true`. The long
sweeping arcs he likes are exactly what Graphviz emits when:

1. Two nodes are placed at distant ranks (e.g. top and bottom of the diagram).
2. The intermediate route would have to bend around several other nodes.
3. The B-spline interpolator smooths the resulting polyline into a single arc.

So Allie's preferred look = PlantUML default = Graphviz `splines=true` =
`EdgeRouting::Splines` mode in our router. **Not a heuristic — a global mode.**

Importantly: in PlantUML's actual output, **short** state transitions in the same
diagram do *not* curve dramatically (Graphviz keeps them near-straight). So
"`Splines` mode" is not "curve everything aggressively" — it's "let the spline
interpolator decide", which naturally produces gentle straights for short edges
and big sweeps for long ones. That's the property Allie wants.

---

## Gap analysis (vs PR #1322 and current `main`)

### KEEP — matches upstream PlantUML

| Commit | What | Why keep |
|---|---|---|
| `c2e271c7` *fix(layout): re-orient router paths to model from→to direction* | Endpoint anchoring; composition diamond now lands on bottom-edge midpoint of A | Pure correctness bug, orthogonal to curves. Closes #1318. |
| `44b25f89` *feat(render): emit curved arc for sequence self-message loops* | D-shape self-message in sequence | Matches upstream sequence renderer's "itself arrow" (§1.39.2). |
| `e987762f` *feat(render): emit curved arc for class self-association edges* | C-shape self-association arc on class | Matches Graphviz default self-loop rendering for class diagrams. |
| `c43ba0a0` *test(render): cover wave-14 curves + endpoint anchoring* | Tests for the four invariants | Keep tests for the parts we keep; update the rest. |

### REVERT or NARROW — does not match upstream

| Commit | Issue | Recommendation |
|---|---|---|
| `95685c19` *feat(render): emit visible curved arc for state self-transitions* | Replaced with cubic-bezier `right-edge → top-edge` loop. Graphviz's actual self-loop on state diagrams is a small loop on a single side, not a corner-to-corner cubic. The shape is not specified anywhere in PlantUML. | **Revert and reimplement** using the same small-loop shape as the class self-association arc (`e987762f`). |
| `391da8bc` *feat(render): emit rounded curved corners for activity back-edges* | Polyline with 10 px rounded corners on the upward bypass route. Upstream activity new-syntax renderer's back-edge shape is implementation-defined (not in the spec). It *looks* gently rounded, so this PR is **plausible but not authoritative.** | **Keep as a stopgap**; mark with `// TODO(#1331): match upstream activity new-syntax back-edge geometry once visually verified` and confirm against an upstream-rendered fixture before declaring final. |

### ADD — the universal gap

Our router has **no equivalent of `splines=true` (PlantUML default)** and **no
equivalent of `splines=polyline`**. All edges in Graphviz-backed families render as
orthogonal polylines, equivalent to `splines=ortho`. That is a one-mode renderer
where upstream has three.

The fix is one cohesive piece of work (not a per-family bolt-on):

1. Add `EdgeRouting { Splines, Polyline, Ortho }` to the router contract,
   defaulting to `Splines` to match upstream.
2. Plumb `skinparam linetype` parsing in our parser to set the mode
   (`polyline` → `Polyline`, `ortho` → `Ortho`, unset → `Splines`).
3. Implement `Splines` mode by running a B-spline interpolation over the channel
   waypoints we already compute (same control polygon, smoothed). For self-edges,
   keep the bespoke arc emission (sequence D-shape, class C-shape, state same as
   class).
4. Implement `Polyline` mode by emitting the channel waypoints as a straight
   polyline (no interpolation) — this is what we currently do internally except
   we wrap it in extra orthogonal turns; needs a small refactor.
5. Keep `Ortho` mode = current behavior.

### Already correct — no change

- Sequence non-self messages, parallel teoz, lifelines.
- Activity new-syntax forward path (straight vertical).
- Timing, JSON, YAML, nwdiag, salt, mindmap, WBS, gantt — bespoke renderers
  that don't depend on the universal gap.

---

## Proposed implementation plan

### Stage 1 — Triage PR #1322 (today / next agent)

Action items, no new commits required to land #1322:

- Decide on `95685c19` (state self-transition): either revert in-PR, or merge as-is
  and immediately follow up with the C-shape rewrite. Recommendation: **revert
  in-PR** so we don't ship a documented-incorrect shape.
- Decide on `391da8bc` (activity back-edge corners): **keep**, mark TODO referencing
  #1331 for upstream-visual-fidelity follow-up.
- Update PR #1322 description to scope it to: "(A) anchoring fix #1318; (B) two
  upstream-correct self-edge fixes (sequence D-shape, class C-shape); (C) activity
  back-edge corner-rounding stopgap pending visual verification." Mark state
  self-transition as deferred to #1331 follow-up.

### Stage 2 — Universal edge routing (issue #1331 proper)

1. **`src/render/graph_layout/router/contract.rs`**: add
   ```rust
   #[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
   pub enum EdgeRouting {
       #[default] Splines,
       Polyline,
       Ortho,
   }
   ```
   Extend `RouteOptions` with `routing: EdgeRouting`.
2. **`src/render/graph_layout/router.rs`**: after the existing
   waypoint-channel-track computation, branch on `options.routing`:
   - `Splines`: interpolate a cubic B-spline through the waypoints
     (centripetal-Catmull-Rom or De Boor), emit as
     `<path d="M sx,sy C c1x,c1y c2x,c2y x,y ..."/>`.
   - `Polyline`: emit the waypoints as a straight `<polyline points="..."/>`
     (no orthogonal hooks).
   - `Ortho`: current behavior.
3. **`src/parser/skinparam.rs`** (or wherever we currently parse
   `skinparam`): add `linetype <value>` parsing, mapping to the enum above.
   Document in `site/content/guide/themes.md`.
4. **`src/normalize/...`**: no changes — the routing mode is global, not per-edge,
   matching upstream.
5. **Tests**: `tests/edge_routing.rs` with three fixtures (default → `<path>` with
   `C` commands; `linetype polyline` → `<polyline>`; `linetype ortho` → current
   orthogonal output). Add a class self-association test that confirms the
   bespoke C-shape arc still wins over the global mode (self-edges short-circuit
   the router).
6. **Default**: keep our current behavior (`Ortho`-equivalent) as the **rendered**
   default for now via a transition flag, then flip to `Splines` once visual
   baselines are blessed against upstream output for the docs/examples corpus.

### Stage 3 — Visual gate

Use `scripts/render_corpus.py --force` + multimodal Read-the-PNG audit against the
upstream PlantUML JAR output for the same source. Acceptance fixtures:

- `docs/examples/state/08_full_machine.puml` — has all the patterns Allie's
  screenshots show (back-up transition, cancellation arrow, cross-region).
- `docs/examples/class/*` — long inheritance chains.
- `docs/examples/component/06_with_arrows.svg` — already touched by PR #1322's
  anchoring fix; should sweep gently under `Splines` mode.
- Architecture-overview C4 fixture — Allie's "is it the c4 graphs?" hint.

### Stage 4 — Cleanup

- Remove the `TODO(#1331)` comment on `391da8bc` once activity back-edge
  geometry is verified against upstream.
- Update `docs/internal/architecture/renderer-refactor-roadmap.md` with the
  three-mode `EdgeRouting` contract.
- Add `skinparam linetype` to `site/content/guide/themes.md`.
- Add the `EdgeRouting` enum to `docs/internal/architecture/layout-engine-vision.md`.

---

## Open questions Allie should answer before Stage 2

1. **Default mode**: ship `EdgeRouting::Splines` as default (matches upstream
   PlantUML) or keep `EdgeRouting::Ortho` as default (matches our current corpus)?
   Recommended: `Splines`, with a wave to re-bless visual baselines.
2. **Activity back-edge** (`391da8bc`): keep as stopgap, or block on visual
   verification against upstream before merge?
3. **State self-transition** (`95685c19`): revert in-PR or merge-and-follow-up?
4. **Beyond-PlantUML extensions**: do we want a 4th value like
   `EdgeRouting::SmoothPerKind` that *does* classify back-edges/exceptions and
   curve them more aggressively? Allie's screenshots show such edges *as
   Graphviz-default splines*, not as a special class. So Stage 2 alone may be
   sufficient. Defer this question until after Stage 2 visual gate.

---

## References

- PR #1322: <https://github.com/alliecatowo/puml/pull/1322>
- Issue #1318 (endpoint anchoring): closed by `c2e271c7`. Keep.
- Issue #1319 (curved self-edges): partially correct (sequence + class match
  upstream; state + activity don't, per Stage 1).
- Issue #1331 (long-distance feedback): the master ticket for Stage 2 above.
- Spec §1.5 / §1.39.2 (sequence self-message), §3.2 (class relation tokens),
  §6.0.1 (activity new-syntax bespoke renderer), §6.6.2 / §6.9.2 (back-edges),
  §9.4 (state self-transition), §20.3 (linetype ortho — only documented
  geometry knob), §27.5 (C4 stdlib).
- Upstream PlantUML Java:
  - `src/main/java/net/sourceforge/plantuml/skin/SkinParam.java#getDotSplines()`
  - `src/main/java/net/sourceforge/plantuml/dot/DotSplines.java`
  - `src/main/java/net/sourceforge/plantuml/svek/DotStringFactory.java`
- Local stdlib: `stdlib/C4/C4.puml` (lines 48–66 — `Rel*` macros expand to `-->`).
