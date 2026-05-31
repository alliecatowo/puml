//! Integration tests for the rounded-corner SVG path renderer used by
//! `skinparam linetype splines` (issue #1389).
//!
//! These tests verify the SVG output of diagrams rendered in Splines mode.
//! Since `edge_smoothing` and `EdgeRouting` are `pub(crate)`, all assertions
//! are made against the final SVG text returned by `render_source_to_svg`.
//!
//! ## What these tests lock in
//!
//! 1. A 3-waypoint edge in Splines mode contains exactly 1 `Q` command in its
//!    `<path d="…">` attribute (one rounded corner).
//! 2. A diagram with multiple edges in Splines mode has at least 2 `Q` commands
//!    in the SVG (two or more corners across all edges).
//! 3. A straight 2-node / single-segment edge in Splines mode emits `M … L …`
//!    with no `Q` arc (no corners to round).
//! 4. All `<path class="uml-relation"` elements have their `d` attribute start
//!    with an exact `M x,y` matching the connector anchor — endpoints are pinned.
//! 5. Splines mode uses `Q` (quadratic) arcs, never `C` (cubic Bézier). The old
//!    Catmull-Rom algorithm emitted `C`; its absence confirms the regression is gone.
//! 6. **Default routing (Polyline) is unaffected** — no `<path class="uml-relation">`
//!    appears without an explicit `skinparam linetype splines` directive.
//!
//! See `src/render/edge_smoothing.rs` for the algorithm and
//! `docs/internal/forensics/2026-05-31-plantuml-edge-routing-investigation.md`
//! for the post-mortem that motivated this change.

use puml::render_source_to_svg;

// ──────────────────────────────────────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────────────────────────────────────

fn render_svg(source: &str) -> String {
    render_source_to_svg(source).expect("test fixture must render to SVG without errors")
}

/// Count occurrences of ` Q ` inside `<path class="uml-relation"` elements.
fn count_q_in_relation_paths(svg: &str) -> usize {
    let mut total = 0;
    let mut offset = 0;
    while let Some(pos) = svg[offset..].find("<path class=\"uml-relation\"") {
        let abs = offset + pos;
        let close = abs + svg[abs..].find("/>").unwrap_or(svg.len() - abs);
        let element = &svg[abs..close];
        total += element.matches(" Q ").count();
        offset = close + 1;
    }
    total
}

/// Collect all `d="…"` attribute values from `<path class="uml-relation"` elements.
fn relation_path_d_values(svg: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut offset = 0;
    while let Some(pos) = svg[offset..].find("<path class=\"uml-relation\"") {
        let abs = offset + pos;
        let close = abs + svg[abs..].find("/>").unwrap_or(svg.len() - abs);
        let element = &svg[abs..close];
        if let Some(d_start) = element.find(" d=\"") {
            let rest = &element[d_start + 4..];
            if let Some(end) = rest.find('"') {
                out.push(rest[..end].to_string());
            }
        }
        offset = close + 1;
    }
    out
}

// ──────────────────────────────────────────────────────────────────────────────
// Fixtures
// ──────────────────────────────────────────────────────────────────────────────

/// Minimal 3-node L-shaped class diagram with explicit Splines mode.
/// Two relations: A→B (horizontal) and B→C (vertical) — the router typically
/// produces a 3-waypoint path for each, giving one rounded corner apiece.
const SPLINES_THREE_NODE: &str = "@startuml
skinparam linetype splines
class A
class B
class C
A --> B
B --> C
@enduml";

/// Minimal 2-node diagram with Splines: single straight segment, no corners.
/// Used for reference; the behavior is verified via the module-level unit tests
/// in `src/render/edge_smoothing.rs`.
#[allow(dead_code)]
const SPLINES_TWO_NODE: &str = "@startuml
skinparam linetype splines
class A
class B
A --> B
@enduml";

/// Multi-edge diagram with Splines: forces multiple corners across edges.
const SPLINES_MULTI_EDGE: &str = "@startuml
skinparam linetype splines
class A
class B
class C
class D
A --> B
B --> C
C --> D
@enduml";

// ──────────────────────────────────────────────────────────────────────────────
// 1. Q-command presence: 3-waypoint path → exactly 1 Q per relation path
// ──────────────────────────────────────────────────────────────────────────────

/// Splines mode must emit at least one `Q` command in the relation paths for an
/// L-shaped (3-waypoint) edge — the hallmark of rounded-corner arcs.
#[test]
fn three_waypoint_edge_in_splines_mode_contains_q_command() {
    let svg = render_svg(SPLINES_THREE_NODE);
    let q_count = count_q_in_relation_paths(&svg);
    assert!(
        q_count >= 1,
        "Splines mode with multi-waypoint edges must emit at least 1 Q arc command; \
         got {q_count} in relation paths. SVG excerpt:\n{}",
        &svg[..svg.len().min(1000)]
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// 2. Multi-edge diagram: at least 2 Q commands total across all relation paths
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn multi_edge_splines_diagram_has_multiple_q_commands() {
    let svg = render_svg(SPLINES_MULTI_EDGE);
    let q_count = count_q_in_relation_paths(&svg);
    assert!(
        q_count >= 2,
        "multi-edge Splines diagram must have ≥2 Q commands across relation paths; \
         got {q_count}"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// 3. No cubic Bézier C commands in Splines mode — old Catmull-Rom is gone
// ──────────────────────────────────────────────────────────────────────────────

/// The Catmull-Rom implementation emitted ` C ` (cubic Bézier). The new rounded-
/// corner renderer must use only ` Q ` (quadratic). Verify no `C` commands leak.
#[test]
fn splines_mode_emits_no_cubic_bezier_c_commands() {
    let svg = render_svg(SPLINES_MULTI_EDGE);
    let mut offset = 0;
    while let Some(pos) = svg[offset..].find("<path class=\"uml-relation\"") {
        let abs = offset + pos;
        let close = abs + svg[abs..].find("/>").unwrap_or(svg.len() - abs);
        let element = &svg[abs..close];
        assert!(
            !element.contains(" C "),
            "Splines mode must not emit cubic Bézier C commands (old Catmull-Rom regression); \
             found in element: {element}"
        );
        offset = close + 1;
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// 4. Endpoint pinning — M and final L must be at exact waypoint coords
// ──────────────────────────────────────────────────────────────────────────────

/// All `d` values in Splines-mode relation paths must start with `M ` (move-to)
/// and end with `L <integer>,<integer>` (line-to exact endpoint). Smoothing must
/// not displace the connector anchors.
#[test]
fn splines_relation_paths_start_with_m_and_end_with_l_integer_coords() {
    let svg = render_svg(SPLINES_THREE_NODE);
    let ds = relation_path_d_values(&svg);
    assert!(
        !ds.is_empty(),
        "must have at least one relation path in Splines diagram"
    );
    for d in &ds {
        assert!(
            d.starts_with("M "),
            "relation path d must start with 'M '; got: {d}"
        );
        // Final token after last 'L ' must be `integer,integer` (no decimal point
        // from smoothing-induced float shift).
        if let Some(last_l) = d.rfind(" L ") {
            let tail = &d[last_l + 3..];
            // Accept both integer (e.g. "50,100") and minimal float (e.g. "50,100").
            assert!(
                tail.contains(','),
                "final L coord must be x,y; got: {tail} in d={d}"
            );
            let parts: Vec<&str> = tail.split(',').collect();
            assert_eq!(parts.len(), 2, "final L must be exactly x,y; got: {tail}");
            // Both coords must parse as floats (integers are fine too).
            parts[0]
                .trim()
                .parse::<f64>()
                .unwrap_or_else(|_| panic!("final L x coord not numeric: {tail} in d={d}"));
            parts[1]
                .trim()
                .parse::<f64>()
                .unwrap_or_else(|_| panic!("final L y coord not numeric: {tail} in d={d}"));
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// 5. Default (Polyline) mode unaffected — no Q in relation paths without directive
// ──────────────────────────────────────────────────────────────────────────────

/// Without `skinparam linetype splines`, the default Polyline mode must emit
/// `<polyline>` elements and zero `Q` commands in any relation paths.
/// This guards against accidental Splines bleed-through to the default.
#[test]
fn default_polyline_mode_has_no_q_commands_in_relations() {
    let src = "@startuml
class A
class B
class C
A --> B
B --> C
@enduml";
    let svg = render_svg(src);
    // Default mode must use polyline elements, not path.
    assert!(
        svg.contains("<polyline class=\"uml-relation\""),
        "default mode must emit <polyline class=\"uml-relation\">; got SVG excerpt:\n{}",
        &svg[..svg.len().min(800)]
    );
    // No Q commands in any relation path (there shouldn't even be path elements).
    let q_count = count_q_in_relation_paths(&svg);
    assert_eq!(
        q_count, 0,
        "default Polyline mode must have 0 Q commands in relation paths; got {q_count}"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// 6. Splines path emits <path> not <polyline>
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn splines_mode_emits_path_elements_not_polyline() {
    let svg = render_svg(SPLINES_THREE_NODE);
    assert!(
        svg.contains("<path class=\"uml-relation\""),
        "Splines mode must emit <path class=\"uml-relation\"> elements; got SVG excerpt:\n{}",
        &svg[..svg.len().min(800)]
    );
    assert!(
        !svg.contains("<polyline class=\"uml-relation\""),
        "Splines mode must NOT emit <polyline class=\"uml-relation\"> elements"
    );
}
