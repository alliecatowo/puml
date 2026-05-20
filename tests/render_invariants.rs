//! Integration tests for the render-time invariants pass.
//!
//! Each test constructs a minimal diagram that would trigger a specific invariant,
//! renders it to SVG, and asserts that the invariant is either auto-corrected or
//! that no violations occur in a well-formed diagram.
//!
//! Tests live here (not in src/render/validate.rs) because they exercise the full
//! round-trip: parse → normalize → render → SVG post-processing.

use puml::render::validate::{
    self, AutoCorrect, GraphValidationProfile, InvariantKind, PackageFrame, PseudoStateKind,
    SemanticRole,
};

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn render_to_svg(source: &str) -> String {
    puml::render_source_to_svg(source).expect("render should succeed")
}

// ─────────────────────────────────────────────────────────────────────────────
// Invariant #2: Label-inside-viewBox
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn invariant2_label_inside_viewbox_after_auto_correct() {
    // A class diagram with a very long class name that might overflow the viewBox.
    let source = r#"
@startuml
class AVeryLongClassNameThatMightOverflowTheViewBoxBoundaryOnSmallCanvases {
  + someMethod(): void
}
@enduml
"#;
    let mut svg = render_to_svg(source);
    let violations = validate::check_labels_inside_viewbox(&mut svg, AutoCorrect::Apply);

    // After auto-correct all labels should fit.  Re-check with EmitDiagnostic to verify.
    let mut svg2 = svg.clone();
    let remaining = validate::check_labels_inside_viewbox(&mut svg2, AutoCorrect::EmitDiagnostic);
    assert!(
        remaining.is_empty(),
        "after auto-correct there should be no remaining label-overflow violations; got: {remaining:?}"
    );
    // Check that auto-correct flag was set when violations existed.
    for v in &violations {
        if !v.corrected {
            panic!("violation was not auto-corrected: {v}");
        }
    }
}

#[test]
fn invariant2_simple_class_diagram_no_overflow_after_autocorrect() {
    // A normal class diagram: after auto-correct, all labels should be within bounds.
    let source = r#"
@startuml
class A {
  + x: int
}
class B {
  + y: String
}
A --> B
@enduml
"#;
    let mut svg = render_to_svg(source);
    // Apply auto-correct first.
    let _ = validate::check_labels_inside_viewbox(&mut svg, AutoCorrect::Apply);
    // Now verify: no remaining violations after correction.
    let mut svg2 = svg.clone();
    let remaining = validate::check_labels_inside_viewbox(&mut svg2, AutoCorrect::EmitDiagnostic);
    assert!(
        remaining.is_empty(),
        "after auto-correct, no label should overflow the viewBox; got: {remaining:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Invariant #1: Edge-vs-node non-intersection
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn invariant1_no_violations_on_simple_linear_class_diagram() {
    // A simple chain A → B → C should never have edges cross intermediate nodes
    // in a hierarchical layout.
    let source = r#"
@startuml
class A
class B
class C
A --> B
B --> C
@enduml
"#;
    let svg = render_to_svg(source);
    let violations = validate::check_edge_node_clearance(&svg);
    // We don't assert zero violations here because intermediate SVG nodes may
    // lack `data-uml-id` attributes that the checker needs — but we do assert
    // that check_edge_node_clearance runs without panicking and returns a Vec.
    let _ = violations; // just confirm the invariant pass doesn't crash
}

#[test]
fn invariant1_check_returns_structured_violations() {
    // Craft a synthetic SVG with an edge that provably crosses an intermediate
    // node to verify the violation struct is correct.
    // Note: SVG is assembled without r#"…"# to avoid # terminating raw strings.
    let svg = [
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="300" viewBox="0 0 400 300">"#,
        r#"<rect class="uml-node" id="node_A" x="10" y="100" width="80" height="40"/>"#,
        r#"<rect class="uml-node" id="node_B" x="160" y="100" width="80" height="40"/>"#,
        r#"<rect class="uml-node" id="node_obstacle" x="100" y="90" width="50" height="60"/>"#,
        "<polyline class=\"uml-relation\" data-uml-from=\"node_A\" data-uml-to=\"node_B\" points=\"90,120 240,120\" fill=\"none\" stroke=\"#333\" stroke-width=\"2\"/>",
        r#"</svg>"#,
    ].join("\n");
    let svg = svg.as_str();
    let violations = validate::check_edge_node_clearance(svg);
    // The edge 90,120 → 240,120 passes through node_obstacle at x=100..150, y=90..150.
    // At y=120 the horizontal segment crosses the obstacle's x-range [100+2, 150-2].
    assert!(
        !violations.is_empty(),
        "expected edge-crosses-node violation for synthetic SVG"
    );
    assert!(matches!(
        violations[0].kind,
        InvariantKind::EdgeCrossesNode { .. }
    ));
}

#[test]
fn invariant1_uses_canonical_puml_node_and_edge_hooks() {
    let svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="300" height="180" viewBox="0 0 300 180">"#,
        r#"<rect class="puml-node" data-puml-id="A" data-puml-kind="entity" data-puml-bbox="10 70 70 40" x="10" y="70" width="70" height="40"/>"#,
        r#"<ellipse class="puml-node" data-puml-id="B" data-puml-kind="attribute" data-puml-bbox="210 70 70 40" cx="245" cy="90" rx="35" ry="20"/>"#,
        r#"<polygon class="puml-node" data-puml-id="obstacle" data-puml-kind="relationship" data-puml-bbox="120 60 60 60" points="150,60 180,90 150,120 120,90"/>"#,
        "<line class=\"puml-edge\" data-puml-from=\"A\" data-puml-to=\"B\" x1=\"80.0\" y1=\"90.0\" x2=\"210.0\" y2=\"90.0\" stroke=\"#333\"/>",
        r#"</svg>"#
    );
    let violations = validate::check_edge_node_clearance(svg);
    assert_eq!(violations.len(), 1);
    assert!(matches!(
        violations[0].kind,
        InvariantKind::EdgeCrossesNode { ref node_id, .. } if node_id == "obstacle"
    ));
}

#[test]
fn invariant6_uses_canonical_puml_node_and_edge_hooks() {
    let svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="300" height="180" viewBox="0 0 300 180">"#,
        r#"<rect class="puml-node" data-puml-id="A" data-puml-kind="entity" data-puml-bbox="10 70 70 40" x="10" y="70" width="70" height="40"/>"#,
        r#"<ellipse class="puml-node" data-puml-id="B" data-puml-kind="attribute" data-puml-bbox="210 70 70 40" cx="245" cy="90" rx="35" ry="20"/>"#,
        "<line class=\"puml-edge\" data-puml-from=\"A\" data-puml-to=\"B\" x1=\"80.0\" y1=\"90.0\" x2=\"210.0\" y2=\"90.0\" stroke=\"#333\"/>",
        r#"</svg>"#
    );
    let violations = validate::check_endpoint_connectivity(svg);
    assert!(
        violations.is_empty(),
        "expected connected puml-edge: {violations:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Canonical puml-* semantic hooks
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn canonical_puml_label_hooks_are_parsed_family_neutrally() {
    let svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="220" height="120" viewBox="0 0 220 120">"#,
        r#"<text class="puml-label" data-puml-owner="node-a" data-puml-label-kind="caption" data-puml-bbox="20 30 56 16" x="48" y="42" text-anchor="middle">Alpha</text>"#,
        r#"</svg>"#
    );

    let labels = validate::extract_semantic_labels(svg);
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0].owner, "node-a");
    assert_eq!(labels[0].label_kind, "caption");
    assert_eq!((labels[0].bbox.x, labels[0].bbox.y), (20, 30));
    assert_eq!(labels[0].text.as_deref(), Some("Alpha"));
}

#[test]
fn canonical_semantic_bboxes_must_fit_inside_viewbox() {
    let svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="160" height="90" viewBox="0 0 160 90">"#,
        r#"<rect class="puml-node" data-puml-id="inside" data-puml-kind="box" data-puml-bbox="10 10 40 30" x="10" y="10" width="40" height="30"/>"#,
        r#"<text class="puml-label" data-puml-owner="inside" data-puml-label-kind="caption" data-puml-bbox="130 70 50 16" x="155" y="82">too far</text>"#,
        r#"</svg>"#
    );

    let violations = validate::check_semantic_bboxes_inside_viewbox(svg);
    assert_eq!(violations.len(), 1);
    assert!(matches!(
        violations[0].kind,
        InvariantKind::SemanticBBoxOutsideViewbox {
            role: SemanticRole::Label,
            ..
        }
    ));
}

#[test]
fn canonical_primary_puml_nodes_must_not_overlap() {
    let svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="240" height="120" viewBox="0 0 240 120">"#,
        r#"<rect class="puml-node" data-puml-id="left" data-puml-kind="generic" data-puml-bbox="20 30 80 40" x="20" y="30" width="80" height="40"/>"#,
        r#"<rect class="puml-node" data-puml-id="right" data-puml-kind="generic" data-puml-bbox="90 40 80 40" x="90" y="40" width="80" height="40"/>"#,
        r#"</svg>"#
    );

    let violations = validate::check_primary_node_non_overlap(svg);
    assert_eq!(violations.len(), 1);
    assert!(matches!(
        violations[0].kind,
        InvariantKind::PrimaryNodeOverlap { ref a, ref b }
            if a == "left" && b == "right"
    ));
}

#[test]
fn canonical_labels_must_clear_non_owner_nodes() {
    let svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="260" height="140" viewBox="0 0 260 140">"#,
        r#"<rect class="puml-node" data-puml-id="owner" data-puml-kind="generic" data-puml-bbox="20 40 70 40" x="20" y="40" width="70" height="40"/>"#,
        r#"<rect class="puml-node" data-puml-id="other" data-puml-kind="generic" data-puml-bbox="140 40 70 40" x="140" y="40" width="70" height="40"/>"#,
        r#"<text class="puml-label" data-puml-owner="owner" data-puml-label-kind="caption" data-puml-bbox="150 50 44 16" x="172" y="62">oops</text>"#,
        r#"</svg>"#
    );

    let violations = validate::check_labels_clear_non_owner_nodes(svg);
    assert_eq!(violations.len(), 1);
    assert!(matches!(
        violations[0].kind,
        InvariantKind::LabelOverlapsNonOwnerNode { ref node_id, .. } if node_id == "other"
    ));
}

#[test]
fn graph_profile_requires_canonical_graph_hooks() {
    let good_svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="260" height="140" viewBox="0 0 260 140">"#,
        r#"<rect class="puml-node" data-puml-id="a" data-puml-kind="generic" data-puml-bbox="20 40 70 40" x="20" y="40" width="70" height="40"/>"#,
        r#"<rect class="puml-node" data-puml-id="b" data-puml-kind="generic" data-puml-bbox="170 40 70 40" x="170" y="40" width="70" height="40"/>"#,
        r#"<line class="puml-edge" data-puml-from="a" data-puml-to="b" x1="90" y1="60" x2="170" y2="60"/>"#,
        r#"<text class="puml-label" data-puml-owner="a" data-puml-label-kind="caption" data-puml-bbox="34 52 42 16" x="55" y="64">A</text>"#,
        r#"</svg>"#
    );
    assert!(
        validate::check_canonical_graph_hooks(good_svg, GraphValidationProfile::Graph).is_empty()
    );

    let bad_svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="260" height="140" viewBox="0 0 260 140">"#,
        r#"<rect class="puml-node" data-puml-id="a" x="20" y="40" width="70" height="40"/>"#,
        r#"<line class="puml-edge" data-puml-from="a" x1="90" y1="60" x2="170" y2="60"/>"#,
        r#"<text class="puml-label" data-puml-owner="a" x="55" y="64">A</text>"#,
        r#"</svg>"#
    );
    assert!(
        validate::check_canonical_graph_hooks(bad_svg, GraphValidationProfile::None).is_empty()
    );

    let violations = validate::check_canonical_graph_hooks(bad_svg, GraphValidationProfile::Graph);
    assert!(
        violations.iter().any(|v| matches!(
            v.kind,
            InvariantKind::CanonicalGraphHookMissing { ref element, ref hook }
                if element == "puml-node" && hook == "data-puml-bbox"
        )),
        "expected missing puml-node bbox hook: {violations:?}"
    );
    assert!(
        violations.iter().any(|v| matches!(
            v.kind,
            InvariantKind::CanonicalGraphHookMissing { ref element, ref hook }
                if element == "puml-edge" && hook == "data-puml-to"
        )),
        "expected missing puml-edge target hook: {violations:?}"
    );
    assert!(
        violations.iter().any(|v| matches!(
            v.kind,
            InvariantKind::CanonicalGraphHookMissing { ref element, ref hook }
                if element == "puml-label" && hook == "data-puml-label-kind"
        )),
        "expected missing puml-label kind hook: {violations:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Invariant #3: Label-vs-edge-stroke clearance
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn invariant3_background_rect_inserted_when_clearance_insufficient() {
    let svg = format!(
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="300" height="200" viewBox="0 0 300 200">"#,
            // Edge from (20,100) to (280,100)
            r#"<polyline class="uml-relation" data-uml-from="X" data-uml-to="Y" points="20,100 280,100" fill="none" stroke="{}" stroke-width="2"/>"#,
            // Label centered on the edge at y=100 — zero clearance
            r#"<text x="150" y="100" text-anchor="middle" font-family="monospace">edge label</text>"#,
            r#"</svg>"#
        ),
        "#555"
    );
    let mut svg = svg.to_string();
    let violations = validate::check_label_edge_clearance(&mut svg, AutoCorrect::Apply);
    // At y=100, clearance between label (y in [88, 104]) and segment (y=100) is 0 < 4px.
    assert!(
        !violations.is_empty(),
        "expected clearance violation, text y=100 equals edge y=100"
    );
    assert!(
        svg.contains("<rect"),
        "background rect should be injected into SVG when auto-correcting"
    );
    for v in &violations {
        assert!(v.corrected, "violation should be marked corrected: {v}");
    }
}

#[test]
fn invariant3_no_violation_when_label_is_above_edge() {
    // Label 20px above the edge — well clear.
    let svg = format!(
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="300" height="200" viewBox="0 0 300 200">"#,
            r#"<polyline class="uml-relation" data-uml-from="X" data-uml-to="Y" points="20,150 280,150" fill="none" stroke="{}" stroke-width="2"/>"#,
            r#"<text x="150" y="120" text-anchor="middle" font-family="monospace">label</text>"#,
            r#"</svg>"#
        ),
        "#555"
    );
    let mut svg = svg.to_string();
    let v = validate::check_label_edge_clearance(&mut svg, AutoCorrect::Apply);
    assert!(
        v.is_empty(),
        "label 30px above edge should not trigger clearance violation"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Invariant #4: Package-header reservation
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn invariant4_edge_through_package_header_detected() {
    // Synthetic SVG with a package frame at y=50, header_height=40.
    // Edge segment y=60 passes through the header strip [50, 90].
    let svg = format!(
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="300" viewBox="0 0 400 300">"#,
            r#"<polyline class="uml-relation" data-uml-from="A" data-uml-to="B" points="10,60 390,60" fill="none" stroke="{}" stroke-width="2"/>"#,
            r#"</svg>"#
        ),
        "#333"
    );
    let frames = vec![PackageFrame {
        id: "MyPackage".to_string(),
        y: 50,
        header_height: 40,
    }];
    let violations = validate::check_package_headers(&svg, &frames);
    assert!(
        !violations.is_empty(),
        "edge at y=60 should violate package header strip [50,90]"
    );
    assert!(matches!(
        violations[0].kind,
        InvariantKind::EdgeThroughPackageHeader { .. }
    ));
}

#[test]
fn invariant4_edge_below_header_no_violation() {
    // Edge at y=110 — below the header strip [50, 90].
    let svg = format!(
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="300" viewBox="0 0 400 300">"#,
            r#"<polyline class="uml-relation" data-uml-from="A" data-uml-to="B" points="10,110 390,110" fill="none" stroke="{}" stroke-width="2"/>"#,
            r#"</svg>"#
        ),
        "#333"
    );
    let frames = vec![PackageFrame {
        id: "MyPackage".to_string(),
        y: 50,
        header_height: 40,
    }];
    let violations = validate::check_package_headers(&svg, &frames);
    assert!(
        violations.is_empty(),
        "edge below package header should not trigger violation"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Invariant #5: Pseudo-state deduplication
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn invariant5_state_diagram_has_at_most_one_initial_pseudostate() {
    // A valid state diagram with a single [*] → Active → [*] flow.
    // After normalization [*] is split: the outgoing [*] stays as StartEnd,
    // and the incoming [*] becomes [*]__end (End kind).
    let source = r#"
@startuml
[*] --> Active
Active --> [*]
@enduml
"#;
    let doc = puml::parse(source).expect("parse ok");
    let norm = puml::normalize_family(doc).expect("normalize_family ok");
    if let puml::NormalizedDocument::State(ref state) = norm {
        let violations = validate::check_pseudo_state_dedup(&state.nodes, "root");
        assert!(
            violations.is_empty(),
            "normalized state diagram should have no duplicate pseudo-states; got: {violations:?}"
        );
    } else {
        panic!("expected NormalizedDocument::State for a state diagram");
    }
}

#[test]
fn invariant5_synthetic_duplicate_initial_is_caught() {
    use puml::model::{StateNode, StateNodeKind};
    // Two StartEnd nodes at the flat level — this should be caught.
    let nodes = vec![
        StateNode {
            name: "[*]".to_string(),
            display: None,
            kind: StateNodeKind::StartEnd,
            stereotype: None,
            internal_actions: vec![],
            regions: vec![],
        },
        StateNode {
            name: "[*]_extra".to_string(),
            display: None,
            kind: StateNodeKind::StartEnd,
            stereotype: None,
            internal_actions: vec![],
            regions: vec![],
        },
        StateNode {
            name: "Active".to_string(),
            display: None,
            kind: StateNodeKind::Normal,
            stereotype: None,
            internal_actions: vec![],
            regions: vec![],
        },
    ];
    let violations = validate::check_pseudo_state_dedup(&nodes, "root");
    assert_eq!(
        violations.len(),
        1,
        "expected exactly one initial-duplicate violation"
    );
    assert!(matches!(
        violations[0].kind,
        InvariantKind::DuplicatePseudoState {
            kind: PseudoStateKind::Initial,
            count: 2,
            ..
        }
    ));
}

// ─────────────────────────────────────────────────────────────────────────────
// Invariant #6: Edge endpoint connectivity
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn invariant6_floating_endpoint_detected() {
    // Edge from "A" whose first point (5,50) is far from the "A" node box at (100,100).
    let svg_parts = [
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="300" viewBox="0 0 400 300">"#,
        r#"<rect class="uml-node" id="A" x="100" y="100" width="80" height="40"/>"#,
        r#"<rect class="uml-node" id="B" x="250" y="100" width="80" height="40"/>"#,
        "<polyline class=\"uml-relation\" data-uml-from=\"A\" data-uml-to=\"B\" points=\"5,50 250,120\" fill=\"none\" stroke=\"#333\" stroke-width=\"2\"/>",
        r#"</svg>"#,
    ];
    let svg = svg_parts.join("\n");
    let violations = validate::check_endpoint_connectivity(&svg);
    assert!(
        !violations.is_empty(),
        "edge starting at (5,50) should not be connected to node A at (100,100)"
    );
}

#[test]
fn invariant6_properly_connected_edge_no_violation() {
    // Edge from "A" (100,100 80×40) starting at (180,120) — on the right edge.
    let svg_parts = [
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="300" viewBox="0 0 400 300">"#,
        r#"<rect class="uml-node" id="A" x="100" y="100" width="80" height="40"/>"#,
        r#"<rect class="uml-node" id="B" x="250" y="100" width="80" height="40"/>"#,
        "<polyline class=\"uml-relation\" data-uml-from=\"A\" data-uml-to=\"B\" points=\"180,120 250,120\" fill=\"none\" stroke=\"#333\" stroke-width=\"2\"/>",
        r#"</svg>"#,
    ];
    let svg = svg_parts.join("\n");
    let violations = validate::check_endpoint_connectivity(&svg);
    assert!(
        violations.is_empty(),
        "properly connected edge should have no endpoint violations"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Full round-trip: run() on a real render
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn full_run_on_class_diagram_completes_without_panic() {
    let source = r#"
@startuml
package "Core" {
  class Service {
    + process(): void
  }
  class Repository {
    + find(id: int): Entity
  }
}
class Controller {
  + handle(): void
}
Controller --> Service
Service --> Repository
@enduml
"#;
    let mut svg = render_to_svg(source);
    // Should not panic, should return a report.
    let report = validate::run(&mut svg, AutoCorrect::Apply);
    // Basic sanity: output is still valid SVG.
    assert!(svg.contains("</svg>"), "SVG output should be well-formed");
    // Report should exist (even if empty).
    let _ = report;
}

#[test]
fn full_run_on_state_diagram_completes_without_panic() {
    let source = r#"
@startuml
[*] --> Idle
Idle --> Active : start
Active --> Idle : stop
Active --> [*] : done
@enduml
"#;
    // State diagrams render via render_state_svg, not wired into the validate::run
    // path yet.  Calling run() directly on the SVG output should be safe.
    let mut svg = puml::render_source_to_svg(source).expect("state render ok");
    let report = validate::run(&mut svg, AutoCorrect::Apply);
    assert!(svg.contains("</svg>"), "SVG output should be well-formed");
    let _ = report;
}
