//! Regression tests for Wave-14 usecase visual fixes.
//!
//! #1291 — actor-generalization hollow triangle floated above parent actor head
//!          due to lateral ortho-path routing.  Fix: actor-to-actor generalization
//!          edges are forced to straight vertical lines (no ortho waypoints).
//!
//! #1292 — usecase system-boundary frames were decoration only: edges from
//!          external actors cut through top frame-borders.  Fix: ortho paths that
//!          cross a frame's top boundary are snapped to terminate at the border.

use puml::{
    normalize_family, parse_with_pipeline_options, render_artifact_pages_from_model,
    ParsePipelineOptions,
};

fn render(src: &str) -> String {
    let opts = ParsePipelineOptions::default();
    let document = parse_with_pipeline_options(src, &opts).expect("source should parse");
    let model = normalize_family(document).expect("source should normalize");
    let artifacts = render_artifact_pages_from_model(&model);
    artifacts
        .into_iter()
        .next()
        .map(|a| a.svg)
        .unwrap_or_default()
}

// ── #1291: actor-generalization uses straight lines (no ortho lateral routing) ─

/// `U <|-- RU` in a pure actor-only diagram must produce a `<line>` element,
/// not a `<polyline>`.  If a `<polyline>` appears it means the pre-computed
/// ortho path was NOT discarded and the edge still has lateral waypoints.
#[test]
fn actor_generalization_produces_straight_line() {
    let src = r#"
@startuml
actor "User" as U
actor "Registered User" as RU
U <|-- RU
@enduml
"#;
    let svg = render(src);
    // Must have the edge element.
    assert!(
        svg.contains("data-uml-from=\"U\" data-uml-to=\"RU\""),
        "missing edge element for U -> RU"
    );
    // The generalization edge must NOT be a polyline.
    assert!(
        !svg.contains("<polyline class=\"uml-relation\" data-uml-from=\"U\" data-uml-to=\"RU\""),
        "generalization edge must be <line>, not <polyline> (ortho path not discarded)"
    );
    // Must be a <line> element.
    assert!(
        svg.contains("<line class=\"uml-relation\" data-uml-from=\"U\" data-uml-to=\"RU\""),
        "generalization edge must be rendered as a <line>"
    );
}

/// When actors are in a vertical stack (the common case), the generalization
/// line must have equal x1 and x2 (within a small tolerance) — i.e. it must
/// be truly vertical.
#[test]
fn actor_generalization_vertical_same_x() {
    let src = r#"
@startuml
actor "Parent" as P
actor "Child" as C
P <|-- C
@enduml
"#;
    let svg = render(src);
    // Find the <line> element.
    let line_prefix = "<line class=\"uml-relation\" data-uml-from=\"P\" data-uml-to=\"C\"";
    assert!(
        svg.contains(line_prefix),
        "Expected <line> for P->C, got none"
    );
    // Extract x1 and x2 values from the line element.
    let line_start = svg.find(line_prefix).expect("line found");
    let line_end = svg[line_start..].find("/>").unwrap_or(200) + line_start;
    let line_elem = &svg[line_start..line_end];
    // Parse x1 and x2 from the element attributes.
    let x1 = parse_attr_i32(line_elem, "x1");
    let x2 = parse_attr_i32(line_elem, "x2");
    assert!(
        (x1 - x2).abs() <= 2,
        "generalization line should be vertical: x1={x1} x2={x2}"
    );
}

fn parse_attr_i32(html: &str, attr: &str) -> i32 {
    let needle = format!(" {attr}=\"");
    let start = html.find(&needle).map(|p| p + needle.len()).unwrap_or(0);
    let end = html[start..].find('"').unwrap_or(0) + start;
    html[start..end].parse().unwrap_or(0)
}

// ── #1292: system-boundary frame rendering ────────────────────────────────────

/// A usecase diagram with a system-boundary rectangle must render the frame.
#[test]
fn system_boundary_frame_rendered() {
    let src = r#"
@startuml
actor "Customer" as C
rectangle "My System" {
  usecase "Login" as UC1
}
C --> UC1
@enduml
"#;
    let svg = render(src);
    // The frame rect must be present.
    assert!(
        svg.contains("class=\"uml-group-frame\""),
        "system-boundary frame not rendered in usecase diagram"
    );
    // The edge must exist.
    assert!(
        svg.contains("data-uml-from=\"C\""),
        "edge from external actor not found"
    );
}

/// With frames present, a usecase diagram must still detect is_usecase_layout=true.
/// Verified indirectly: actor-to-actor generalization edges must be `<line>`,
/// not `<polyline>`, even when the diagram also has system-boundary frames.
#[test]
fn usecase_with_frames_detects_usecase_layout() {
    let src = r#"
@startuml
actor "User" as U
actor "Admin" as A
U <|-- A
rectangle "System" {
  usecase "Login" as UC1
}
U --> UC1
@enduml
"#;
    let svg = render(src);
    // If is_usecase_layout=true, U<|--A must be a <line>.
    assert!(
        !svg.contains("<polyline class=\"uml-relation\" data-uml-from=\"U\" data-uml-to=\"A\""),
        "<polyline> found for U<|--A; expected <line> (is_usecase_layout must be true even with frames)"
    );
    assert!(
        svg.contains("<line class=\"uml-relation\" data-uml-from=\"U\" data-uml-to=\"A\""),
        "<line> not found for U<|--A generalization"
    );
}
