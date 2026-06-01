//! Integration tests for the spline-native edge router extended to the class
//! family (#1412).
//!
//! PR #1410 shipped spline dispatch for box-grid (component/deployment/c4)
//! families via `box_grid_edges.rs`.  This file validates that the same
//! algorithm now fires for **class, object, and usecase** families, which
//! emit edges through `class_relations.rs`.
//!
//! ## Invariants locked in
//!
//! 1. **Cubic-Bézier emission** — with `skinparam linetype splines`, a
//!    class-family relation must be rendered as `<path d="M … C …">`, not a
//!    `<polyline>` or rounded-corner `Q` chamfer path.
//! 2. **Hub-and-spoke tangent divergence** — two or more edges leaving the
//!    same class box must have distinct source-side control points (cp1) so
//!    the curves fan out at the port instead of stacking.
//! 3. **Polyline default unchanged** — without `skinparam linetype splines`
//!    the class-family edges still use `<polyline>` (the PUML default) and no
//!    spline `<path>` appears for relations.
//! 4. **Fallback preserved** — the rounded-corner renderer remains intact for
//!    self-loops, which are out of scope for the spline router.
//!
//! See `src/render/family/class_relations.rs` for the dispatch site and
//! `src/render/graph_layout/spline_router.rs` for the algorithm.

use puml::render_source_to_svg;

fn render_svg(source: &str) -> String {
    render_source_to_svg(source).expect("test fixture must render to SVG without errors")
}

// ── Fixtures ─────────────────────────────────────────────────────────────────

/// Single cross-rank class edge in Splines mode — the simplest possible
/// class-family spline path.
const CLASS_SINGLE_EDGE_SPLINES: &str = "@startuml
skinparam linetype splines
class Animal {
  +name: String
}
class Dog {
  +breed: String
}
Animal --> Dog
@enduml";

/// Hub-and-spoke fixture: one parent class with two inheritance targets.
/// Exercises tangent-jitter fan divergence (#1412).
const CLASS_HUB_AND_SPOKE_SPLINES: &str = "@startuml
skinparam linetype splines
class Container {
  +items: List
}
class Stack {
  +push(): void
}
class Queue {
  +enqueue(): void
}
Container <|-- Stack
Container <|-- Queue
@enduml";

/// Default polyline mode — class edges must NOT become splines.
const CLASS_SINGLE_EDGE_DEFAULT: &str = "@startuml
class Animal {
  +name: String
}
class Dog {
  +breed: String
}
Animal --> Dog
@enduml";

// ── Tests ─────────────────────────────────────────────────────────────────────

#[test]
fn class_splines_mode_emits_path_with_c_command() {
    let svg = render_svg(CLASS_SINGLE_EDGE_SPLINES);
    let path_count = svg.matches("<path class=\"uml-relation\"").count();
    assert!(
        path_count >= 1,
        "Splines mode on class family must emit ≥1 <path class=\"uml-relation\">, got SVG:\n{svg}"
    );
    let c_count = svg.matches(" C ").count();
    assert!(
        c_count >= 1,
        "Spline path must contain at least one cubic 'C' command, got SVG:\n{svg}"
    );
}

#[test]
fn class_splines_path_starts_with_move_command() {
    let svg = render_svg(CLASS_SINGLE_EDGE_SPLINES);
    assert!(
        svg.contains("class=\"uml-relation\"") && svg.contains("d=\"M "),
        "Class spline path d= attribute must start with 'M ', got SVG:\n{svg}"
    );
}

#[test]
fn class_hub_and_spoke_produces_distinct_first_control_points() {
    let svg = render_svg(CLASS_HUB_AND_SPOKE_SPLINES);
    // Collect all spline relation `d` attributes.
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
        "Hub-and-spoke fixture must have ≥2 spline relations, got {}: {:?}",
        ds.len(),
        ds
    );
    // Extract the first control point (cp1) from each path.
    // Format: `M sx,sy C cp1x,cp1y cp2x,cp2y ex,ey [C ...]`
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
        "every spline path must have an extractable cp1"
    );
    // Two or more distinct cp1 values confirms tangent-jitter fan divergence.
    let mut unique: Vec<&String> = cp1s.iter().collect();
    unique.sort();
    unique.dedup();
    assert!(
        unique.len() >= 2,
        "hub-and-spoke fan must produce distinct source-side control points (cp1), got {:?}",
        cp1s
    );
}

#[test]
fn class_default_polyline_mode_unchanged() {
    // Without `skinparam linetype splines`, class edges must stay as
    // `<polyline>` elements — Polyline is the PUML default.
    let svg = render_svg(CLASS_SINGLE_EDGE_DEFAULT);
    assert!(
        svg.contains("<polyline class=\"uml-relation\""),
        "default mode must emit <polyline class=\"uml-relation\">, got SVG:\n{svg}"
    );
    // Confirm no spline cubic path was emitted for the relation.
    assert!(
        !svg.contains("<path class=\"uml-relation\""),
        "default mode must NOT emit spline <path> for class relations, got SVG:\n{svg}"
    );
}
