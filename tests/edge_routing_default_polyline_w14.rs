//! Regression tests for EdgeRouting::Polyline as the PUML default (#1341).
//!
//! ## Background
//!
//! PR #1334 (Stage-2 edge routing) introduced three routing modes and promoted
//! `EdgeRouting::Splines` to the default, matching PlantUML's upstream behavior.
//! Forensic audit #1341 established that splines override the channel router's
//! waypoint geometry — the cubic Bézier smoothing produces arcs that pierce
//! package frame headers and create visual noise incompatible with the orthogonal
//! layout engine. The fix reverts the default to `EdgeRouting::Polyline`.
//!
//! ## What these tests lock in
//!
//! 1. `EdgeRouting::default() == EdgeRouting::Polyline` at the type level.
//! 2. A diagram with no `skinparam linetype` directive renders `<polyline>`
//!    relations (not `<path … C …>` Bézier curves).
//! 3. `skinparam linetype splines` remains available as an explicit opt-in and
//!    does produce Bézier curves when requested.
//! 4. The architecture-overview fixture (a multi-package component diagram)
//!    renders with polyline relations by default and zero cubic-Bézier relation
//!    paths, confirming no spline bleed-through on real-world diagrams.

use puml::render_source_to_svg;

// ---------------------------------------------------------------------------
// 1. Behavioral default assertion (type is pub(crate); verified via SVG output)
// ---------------------------------------------------------------------------

#[test]
fn edge_routing_default_is_polyline() {
    // Verify that `EdgeRouting::default()` is `Polyline` by observing SVG output:
    // a diagram with no `skinparam linetype` directive must produce `<polyline>`
    // relations (straight segments) and zero `<path class="uml-relation">` Bézier
    // curves.  If `#[default]` is ever moved back to `Splines`, this test fails.
    let svg = render_svg(MINIMAL_CLASS);
    assert!(
        svg.contains("<polyline class=\"uml-relation\""),
        "EdgeRouting::default() must be Polyline — no-directive diagram must emit <polyline> relations; got:\n{svg}"
    );
    assert!(
        !svg.contains("<path class=\"uml-relation\""),
        "EdgeRouting::default() must be Polyline — no-directive diagram must NOT emit <path> (spline) relations; got:\n{svg}"
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn render_svg(source: &str) -> String {
    render_source_to_svg(source).expect("test fixture must render to SVG without errors")
}

/// A minimal multi-node diagram with no routing directive.
const MINIMAL_CLASS: &str = "@startuml
class A
class B
class C
A --> B
B --> C
A --> C
@enduml";

// ---------------------------------------------------------------------------
// 2. End-to-end SVG output: no-directive → polylines
// ---------------------------------------------------------------------------

#[test]
fn default_diagram_emits_polyline_relations_not_splines() {
    // With no `skinparam linetype` the SVG must use `<polyline>` elements for
    // relations, not `<path … d="… C …">` Bézier curves.
    let svg = render_svg(MINIMAL_CLASS);
    assert!(
        svg.contains("<polyline class=\"uml-relation\""),
        "default routing must emit <polyline class=\"uml-relation\"> elements; got:\n{svg}"
    );
    // The absence of uml-relation <path> elements confirms Splines is off.
    assert!(
        !svg.contains("<path class=\"uml-relation\""),
        "default routing must NOT emit <path class=\"uml-relation\"> (spline) elements; got:\n{svg}"
    );
}

#[test]
fn default_diagram_has_no_cubic_bezier_in_relation_paths() {
    // Belt-and-suspenders: even if a future refactor changes element names,
    // verify that no `C ` cubic-Bézier command appears inside any element
    // that also carries the `uml-relation` class.
    let svg = render_svg(MINIMAL_CLASS);
    // Collect all substrings that look like `<path class="uml-relation"…`.
    let mut offset = 0;
    while let Some(pos) = svg[offset..].find("<path class=\"uml-relation\"") {
        let abs = offset + pos;
        let close = abs + svg[abs..].find("/>").unwrap_or(svg.len() - abs);
        let element = &svg[abs..close];
        assert!(
            !element.contains(" C "),
            "uml-relation path must not contain cubic Bézier 'C' command in default mode:\n{element}"
        );
        offset = close + 1;
    }
}

// ---------------------------------------------------------------------------
// 3. Splines remains available as explicit opt-in
// ---------------------------------------------------------------------------

#[test]
fn skinparam_linetype_splines_still_emits_bezier_curves() {
    // Splines must remain functional as an explicit opt-in. Diagrams that
    // set `skinparam linetype splines` should still get curved paths.
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
        "explicit 'skinparam linetype splines' must emit <path> elements; got:\n{svg}"
    );
    assert!(
        svg.contains(" C "),
        "explicit 'skinparam linetype splines' must emit cubic Bézier curves; got:\n{svg}"
    );
    assert!(
        !svg.contains("<polyline class=\"uml-relation\""),
        "explicit splines mode must NOT emit <polyline> relations; got:\n{svg}"
    );
}

// ---------------------------------------------------------------------------
// 4. Architecture-overview structural check
// ---------------------------------------------------------------------------

#[test]
fn architecture_overview_default_has_polyline_relations_and_no_bezier() {
    // Render the canonical multi-package component diagram used as the
    // primary visual fixture in the #1341 forensic. In default (Polyline) mode:
    //   - at least one `<polyline class="uml-relation"` must be present
    //   - zero `<path class="uml-relation"` elements that carry a `C ` Bézier command
    let src = std::fs::read_to_string("docs/diagrams/architecture-overview.puml")
        .expect("architecture-overview.puml must exist");
    let svg = render_svg(&src);

    let polyline_count = svg.matches("<polyline class=\"uml-relation\"").count();
    assert!(
        polyline_count > 0,
        "architecture-overview must have >0 polyline relations in default mode, got 0"
    );

    // Verify no spline bleed-through: no uml-relation path with a C command.
    let mut offset = 0;
    let mut bezier_found = false;
    while let Some(pos) = svg[offset..].find("<path class=\"uml-relation\"") {
        let abs = offset + pos;
        let close = abs + svg[abs..].find("/>").unwrap_or(svg.len() - abs);
        let element = &svg[abs..close];
        if element.contains(" C ") {
            bezier_found = true;
            break;
        }
        offset = close + 1;
    }
    assert!(
        !bezier_found,
        "architecture-overview must have 0 cubic-Bézier uml-relation paths in default (Polyline) mode"
    );
}
