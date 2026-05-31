//! End-to-end tests for the three global edge-routing modes selected by
//! `skinparam linetype <value>`.
//!
//! PlantUML's upstream renderer exposes three values mapped 1-to-1 onto
//! Graphviz's `splines=` attribute (see
//! `src/main/java/net/sourceforge/plantuml/skin/SkinParam.java#getDotSplines()`):
//!
//! - default (no directive) → PUML default is `Polyline` (straight segments).
//!   PlantUML upstream uses splines, but we diverge intentionally (#1341):
//!   splines override the channel router's geometry and produce arcs that pierce
//!   package frames. Polyline is stable and correct with the channel layout engine.
//! - `polyline` → `splines=polyline` → straight segments through waypoints
//! - `ortho` → `splines=ortho` → orthogonal right-angle elbows
//! - `splines` (opt-in) → smooth B-spline curves, available but not the default
//!
//! Reference: `docs/internal/architecture/edge-routing.md` and
//! `docs/internal/architecture/edge-curve-research-2026-05-29.md`.

use puml::render_source_to_svg;

/// Render `source` to SVG. Panics if rendering produces hard errors —
/// these tests render minimal valid fixtures so any error indicates a
/// regression in the test harness itself, not the routing modes.
fn render_svg(source: &str) -> String {
    render_source_to_svg(source).expect("test fixture must render to SVG without errors")
}

const MINIMAL_CLASS_DIAGRAM: &str = "@startuml
class A
class B
class C
A --> B
B --> C
A --> C
@enduml";

#[test]
fn default_routing_is_polyline() {
    // No `skinparam linetype` is set → the renderer must emit straight
    // polyline segments (PUML default since #1341 revert). Each relation
    // must appear as a `<polyline … points="…"/>` element with no cubic
    // Bézier `C` command inside a uml-relation path.
    let svg = render_svg(MINIMAL_CLASS_DIAGRAM);
    assert!(
        svg.contains("<polyline class=\"uml-relation\""),
        "default mode must emit <polyline> elements for relations, got: {svg}"
    );
    // No spline-curved relations under the default mode.
    assert!(
        !svg.contains("<path class=\"uml-relation\""),
        "default mode must NOT emit <path> (spline) relations, got: {svg}"
    );
}

#[test]
fn skinparam_linetype_splines_emits_rounded_corner_paths() {
    // Explicit `skinparam linetype splines` must produce rounded-corner paths
    // using `Q` (quadratic arc) commands. The old Catmull-Rom cubic Bézier `C`
    // is replaced by the rounded-corner renderer (issue #1389).
    let src = "@startuml
skinparam linetype splines
class A
class B
class C
A --> B
B --> C
A --> C
@enduml";
    let svg = render_svg(src);
    assert!(
        svg.contains("<path class=\"uml-relation\""),
        "explicit splines mode must emit <path> elements, got: {svg}"
    );
    assert!(
        svg.contains(" Q "),
        "explicit splines mode must emit rounded-corner Q arc commands, got: {svg}"
    );
    assert!(
        !svg.contains("<polyline class=\"uml-relation\""),
        "explicit splines mode must NOT emit polyline relations, got: {svg}"
    );
}

#[test]
fn skinparam_linetype_polyline_emits_straight_segments() {
    // `skinparam linetype polyline` must emit `<polyline>` elements with
    // `points="…"` straight-segment geometry. No cubic Bézier `C`
    // commands should appear inside relation `d` attributes (markers
    // separately emit their own `path d="…"` decorations, so we narrow
    // the negative assertion to relation-shaped elements only).
    let src = "@startuml
skinparam linetype polyline
class A
class B
class C
A --> B
B --> C
A --> C
@enduml";
    let svg = render_svg(src);
    assert!(
        svg.contains("<polyline class=\"uml-relation\""),
        "polyline mode must emit <polyline> elements for relations, got: {svg}"
    );
    assert!(
        !svg.contains("<path class=\"uml-relation\""),
        "polyline mode must NOT emit <path> relations, got: {svg}"
    );
}

#[test]
fn skinparam_linetype_ortho_emits_right_angles() {
    // `skinparam linetype ortho` must emit `<polyline>` with right-angle
    // corner geometry. The orthogonal channel router introduces a
    // detour around the middle node B for the A→C edge, so the
    // resulting polyline must have ≥ 4 waypoints (start, two corners,
    // end) for that relation. We verify the corner pattern by checking
    // that one polyline has a repeated x or y coordinate between
    // adjacent points — a hallmark of orthogonal routing.
    let src = "@startuml
skinparam linetype ortho
class A
class B
class C
A --> B
B --> C
A --> C
@enduml";
    let svg = render_svg(src);
    assert!(
        svg.contains("<polyline class=\"uml-relation\""),
        "ortho mode must emit <polyline> relations, got: {svg}"
    );
    assert!(
        !svg.contains("<path class=\"uml-relation\""),
        "ortho mode must NOT emit <path> relations, got: {svg}"
    );
    // Find the A→C polyline (the longest one — it detours around B).
    // Slice the SVG starting at the <polyline class="uml-relation" tag for
    // the A→C edge so we don't accidentally grab marker definitions.
    let needle = "<polyline class=\"uml-relation\"";
    let mut search_pos = 0;
    let mut target_open = None;
    while let Some(rel_off) = svg[search_pos..].find(needle) {
        let abs = search_pos + rel_off;
        let close = abs + svg[abs..].find("/>").expect("polyline must close");
        let element = &svg[abs..close];
        if element.contains("data-uml-from=\"A\"") && element.contains("data-uml-to=\"C\"") {
            target_open = Some(element);
            break;
        }
        search_pos = close;
    }
    let target = target_open.expect("A→C polyline must be present");
    // Extract the points attribute body and check for a repeated-axis
    // pattern across at least one adjacent pair of waypoints.
    let points_start = target.find("points=\"").expect("points attr must exist") + 8;
    let points_end = target[points_start..]
        .find('"')
        .expect("points attr must close");
    let body = &target[points_start..points_start + points_end];
    let pts: Vec<(i32, i32)> = body
        .split_whitespace()
        .filter_map(|pair| {
            let (x, y) = pair.split_once(',')?;
            Some((x.parse().ok()?, y.parse().ok()?))
        })
        .collect();
    assert!(
        pts.len() >= 4,
        "ortho A→C must have at least 4 waypoints (detour around B), got {pts:?}"
    );
    let has_right_angle = pts
        .windows(2)
        .any(|seg| seg[0].0 == seg[1].0 || seg[0].1 == seg[1].1);
    assert!(
        has_right_angle,
        "ortho mode must emit axis-aligned segments (repeated x or y), got {pts:?}"
    );
}

#[test]
fn skinparam_linetype_case_insensitive() {
    // Upstream PlantUML accepts `splines`, `Splines`, `SPLINES`, etc.
    // Our parser must do the same. `Polyline` and `Ortho` capitalized
    // should resolve to the polyline mode.
    let src = "@startuml
skinparam linetype POLYLINE
class A
class B
A --> B
@enduml";
    let svg = render_svg(src);
    assert!(
        svg.contains("<polyline class=\"uml-relation\""),
        "case-insensitive POLYLINE must resolve to polyline mode, got: {svg}"
    );
    assert!(
        !svg.contains("<path class=\"uml-relation\""),
        "case-insensitive POLYLINE must NOT emit <path>, got: {svg}"
    );
}

#[test]
fn skinparam_linetype_unknown_value_falls_back_to_default() {
    // Unknown values are silently ignored — the default (Polyline) wins (#1341).
    let src = "@startuml
skinparam linetype bezier3
class A
class B
A --> B
@enduml";
    let svg = render_svg(src);
    assert!(
        svg.contains("<polyline class=\"uml-relation\""),
        "unknown linetype must fall back to polyline default, got: {svg}"
    );
    assert!(
        !svg.contains("<path class=\"uml-relation\""),
        "unknown linetype must NOT emit spline <path> relations, got: {svg}"
    );
}
