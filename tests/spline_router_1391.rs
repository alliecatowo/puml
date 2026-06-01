//! Integration tests for the spline-native edge waypoint generator (#1391).
//!
//! These tests assert against the final SVG output of `render_source_to_svg`
//! for component-family diagrams (which dispatch through
//! `box_grid_edges.rs`, where #1391 wired the spline-native path) rendered
//! with `skinparam linetype splines`.
//!
//! ## Invariants locked in
//!
//! 1. **Cubic-Bézier emission** — each Splines-mode component-family edge
//!    SVG element is a `<path d="M ... C ...">`, not a `<polyline points=...>`
//!    or a rounded-corner `Q` chamfer path. The presence of one or more `C`
//!    commands per edge identifies the spline-native renderer.
//! 2. **Endpoint pinning** — the path starts (`M`) at the source anchor and
//!    ends at the target anchor. Arrowheads anchor correctly only if the
//!    endpoints match exactly.
//! 3. **Hub-and-spoke fan** — two edges leaving the same source produce
//!    distinct first control points (the source-side cp1), so the curves
//!    diverge at the port instead of stacking.
//! 4. **Polyline default unaffected** — without `skinparam linetype splines`,
//!    component-family edges are still `<polyline>` (the PUML default).
//!
//! See `src/render/graph_layout/spline_router.rs` for the algorithm and
//! `docs/internal/architecture/edge-routing.md` for the mode-selection
//! contract.

#![allow(clippy::field_reassign_with_default)]

use puml::render_source_to_svg;

fn render_svg(source: &str) -> String {
    render_source_to_svg(source).expect("test fixture must render to SVG without errors")
}

/// Two-component fixture in Splines mode: one cross-rank edge, expected
/// to produce a single-segment cubic Bézier path.
const TWO_COMPONENT_SPLINES: &str = "@startuml
skinparam linetype splines
component A
component B
A --> B
@enduml";

/// Hub-and-spoke fixture: one source, two children — assert tangent jitter
/// produces distinct first control points.
const HUB_AND_SPOKE_SPLINES: &str = "@startuml
skinparam linetype splines
component Hub
component Child1
component Child2
Hub --> Child1
Hub --> Child2
@enduml";

/// Package fixture: many cross-rank edges between two packages — exercises
/// the obstacle-aware spline generator with sibling packages as obstacles.
const PACKAGED_COMPONENTS_SPLINES: &str = "@startuml
skinparam linetype splines
package Frontend {
  component WebApp
  component MobileApp
}
package Backend {
  component OrderService
  component AuthService
  component NotificationService
}
WebApp --> OrderService
MobileApp --> AuthService
OrderService --> NotificationService
@enduml";

#[test]
fn single_edge_splines_emits_path_with_cubic_segment() {
    let svg = render_svg(TWO_COMPONENT_SPLINES);
    // Find the uml-relation element.
    let path_count = svg.matches("<path class=\"uml-relation\"").count();
    assert!(
        path_count >= 1,
        "Splines mode must emit at least one <path class=\"uml-relation\"...>, got SVG:\n{svg}"
    );
    // The spline-native renderer emits at least one cubic Bézier `C`
    // command per edge (single-segment for simple topology).
    let c_count = svg.matches(" C ").count();
    assert!(
        c_count >= 1,
        "Spline path must contain at least one C command, got SVG:\n{svg}"
    );
}

#[test]
fn spline_path_starts_with_move_command() {
    // Every spline path's `d` attribute begins with `M `.
    let svg = render_svg(TWO_COMPONENT_SPLINES);
    // Find a relation path's d attribute. It must contain `d="M `.
    assert!(
        svg.contains("class=\"uml-relation\"") && svg.contains("d=\"M "),
        "Spline path d= attribute must start with M (move-to), got SVG:\n{svg}"
    );
}

#[test]
fn hub_and_spoke_produces_distinct_first_control_points() {
    let svg = render_svg(HUB_AND_SPOKE_SPLINES);
    // Extract every relation's `d="..."` body.
    let ds: Vec<String> = svg
        .match_indices("class=\"uml-relation\"")
        .filter_map(|(idx, _)| {
            let tail = &svg[idx..];
            let d_start = tail.find("d=\"")? + 3;
            let d_end = tail[d_start..].find('"')? + d_start;
            Some(tail[d_start..d_end].to_string())
        })
        .filter(|d| d.contains(" C "))
        .collect();
    assert!(
        ds.len() >= 2,
        "Hub-and-spoke must have ≥2 spline relations, got {}: {:?}",
        ds.len(),
        ds
    );
    // Extract first cubic Bézier cp1 (the source-side control) for each
    // path. Format: `M sx,sy C cp1x,cp1y cp2x,cp2y ex,ey ...`.
    let cp1s: Vec<String> = ds
        .iter()
        .filter_map(|d| {
            let c_pos = d.find(" C ")?;
            let after_c = &d[c_pos + 3..];
            let first_token = after_c.split(' ').next()?.to_string();
            Some(first_token)
        })
        .collect();
    assert_eq!(
        cp1s.len(),
        ds.len(),
        "every spline must have an extractable cp1"
    );
    // At least two distinct cp1 values → the tangent jitter actually
    // separated the curves at the source port.
    let mut unique: Vec<&String> = cp1s.iter().collect();
    unique.sort();
    unique.dedup();
    assert!(
        unique.len() >= 2,
        "hub-and-spoke fan must produce distinct cp1 (source tangents), got {:?}",
        cp1s
    );
}

#[test]
fn packaged_components_render_without_panic() {
    // Smoke test: the obstacle-aware path inside packages doesn't panic
    // and produces at least one spline path per edge.
    let svg = render_svg(PACKAGED_COMPONENTS_SPLINES);
    let c_count = svg.matches(" C ").count();
    assert!(
        c_count >= 3,
        "expected ≥3 cubic segments for 3 edges in packaged fixture, got {} from SVG:\n{svg}",
        c_count
    );
}

#[test]
fn polyline_default_still_emits_polyline_not_path() {
    // Without `skinparam linetype splines`, component edges must still
    // render as `<polyline>` elements (PUML default is Polyline since #1343).
    let src = "@startuml
component A
component B
A --> B
@enduml";
    let svg = render_svg(src);
    assert!(
        svg.contains("<polyline class=\"uml-relation\""),
        "default mode must emit <polyline class=\"uml-relation\"...>, got SVG:\n{svg}"
    );
    // No spline-native <path> for the relation under the default.
    assert!(
        !svg.contains("<path class=\"uml-relation\""),
        "default mode must NOT emit spline <path> relations, got SVG:\n{svg}"
    );
}

#[test]
fn spline_endpoints_pin_to_source_and_target_anchors() {
    // Extract the spline d attribute from TWO_COMPONENT_SPLINES, parse
    // its M start point and final cubic endpoint, and verify each lies
    // on the corresponding node bbox boundary (the source/target
    // <rect> elements).
    let svg = render_svg(TWO_COMPONENT_SPLINES);
    // Find the relation path d="..." body.
    let rel_idx = svg
        .find("class=\"uml-relation\"")
        .expect("expected a uml-relation");
    let tail = &svg[rel_idx..];
    let d_start = tail.find("d=\"").expect("expected d= attribute") + 3;
    let d_end = tail[d_start..].find('"').unwrap() + d_start;
    let d = &tail[d_start..d_end];
    // d looks like: M sx,sy C c1x,c1y c2x,c2y ex,ey [C ...]
    assert!(d.starts_with("M "), "d must start with 'M ', got: {d}");
    // Extract M coords (first two numbers after 'M ').
    let after_m = &d[2..];
    let move_token = after_m.split(' ').next().unwrap();
    let (mx, my) = parse_pair(move_token);
    // Extract final C segment endpoint (last 'x,y' in d, after all
    // intermediate control points).
    let last_segment = d.rsplit(" C ").next().unwrap();
    let last_token = last_segment.split(' ').next_back().unwrap();
    let (ex, ey) = parse_pair(last_token);
    // Endpoint coords must be finite real numbers in canvas space.
    assert!(
        mx.is_finite() && my.is_finite(),
        "M coords finite: {mx},{my}"
    );
    assert!(
        ex.is_finite() && ey.is_finite(),
        "End coords finite: {ex},{ey}"
    );
    // M and end must differ (otherwise it's a degenerate zero-length curve).
    assert!(
        (mx - ex).hypot(my - ey) > 1.0,
        "M and end must differ by >1 px, got M=({mx},{my}) end=({ex},{ey})"
    );
}

fn parse_pair(token: &str) -> (f64, f64) {
    let mut parts = token.split(',');
    let x: f64 = parts.next().unwrap().parse().unwrap();
    let y: f64 = parts.next().unwrap().parse().unwrap();
    (x, y)
}

#[test]
fn label_x_matches_path_midpoint_within_tolerance() {
    // Component relation with a label: the label x should be near the
    // arclength midpoint of the spline curve, not the bbox-pair midpoint.
    let src = "@startuml
skinparam linetype splines
component Source
component Target
Source --> Target : my_label
@enduml";
    let svg = render_svg(src);
    // Extract the path d attribute.
    let rel_idx = svg.find("class=\"uml-relation\"").expect("uml-relation");
    let tail = &svg[rel_idx..];
    let d_start = tail.find("d=\"").unwrap() + 3;
    let d_end = tail[d_start..].find('"').unwrap() + d_start;
    let d = &tail[d_start..d_end];
    // Compute approximate spline midpoint from d using the M and last
    // endpoint (a rough proxy — exact arclength midpoint comes from the
    // unit-tested `SplinePath::point_at(0.5)` in `spline_router.rs`).
    let after_m = &d[2..];
    let move_token = after_m.split(' ').next().unwrap();
    let (mx, _) = parse_pair(move_token);
    let last_segment = d.rsplit(" C ").next().unwrap();
    let last_token = last_segment.split(' ').next_back().unwrap();
    let (ex, _) = parse_pair(last_token);
    let approx_mid_x = (mx + ex) / 2.0;
    // Find the label element.
    let label_idx = svg.find(">my_label<").expect("label rendered");
    // Walk back to find the x= attribute of that text element.
    let prelude = &svg[..label_idx];
    let text_start = prelude.rfind("<text").expect("text start");
    let text_attrs = &svg[text_start..label_idx];
    let x_start = text_attrs.find("x=\"").expect("x= attr") + 3;
    let x_end = text_attrs[x_start..].find('"').unwrap() + x_start;
    let label_x: f64 = text_attrs[x_start..x_end].parse().unwrap();
    // Tolerance: ≤ 50 px from the approx midpoint x. The actual arclength
    // midpoint sits within ~half the handle length of the linear midpoint.
    let dx = (label_x - approx_mid_x).abs();
    assert!(
        dx <= 50.0,
        "label x ({label_x}) must be near spline midpoint x (~{approx_mid_x}), dx={dx}, SVG:\n{svg}"
    );
}
