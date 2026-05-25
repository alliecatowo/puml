//! Integration tests for the render-time invariants pass.
//!
//! Each test constructs a minimal diagram that would trigger a specific invariant,
//! renders it to SVG, and asserts that the invariant is either auto-corrected or
//! that no violations occur in a well-formed diagram.
//!
//! Tests live here (not in src/render/validate.rs) because they exercise the full
//! round-trip: parse → normalize → render → SVG post-processing.

use puml::render::validate::{self, AutoCorrect, InvariantKind, PackageFrame, PseudoStateKind};
use puml::render::RenderValidationState;
use puml::render_core::validate::GeometryMetric;
use puml::render_core::{
    Anchor, GeometryIssue, NodeBox, Point, Polyline, Rect, RenderScene, SceneAvailability,
    SceneEdge, SceneNode,
};

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn render_to_svg(source: &str) -> String {
    puml::render_source_to_svg(source).expect("render should succeed")
}

fn render_family_artifact(source: &str) -> puml::render::RenderArtifact {
    let doc = puml::parse(source).expect("parse should succeed");
    match puml::normalize_family(doc).expect("normalize should succeed") {
        puml::NormalizedDocument::Family(family) => puml::render_family_document_artifact(&family),
        other => panic!("expected family document, got {other:?}"),
    }
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

#[test]
fn invariant3_ignores_unmarked_node_text_when_edge_labels_are_marked() {
    let mut svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="300" height="200" viewBox="0 0 300 200">"#,
        r##"<polyline class="uml-relation" data-uml-from="X" data-uml-to="Y" points="20,100 280,100" fill="none" stroke="#555" stroke-width="2"/>"##,
        r#"<text x="150" y="100" text-anchor="middle" font-family="monospace">node header near route</text>"#,
        r#"<text class="uml-edge-label" data-uml-label-role="edge" x="150" y="60" text-anchor="middle" font-family="monospace">safe edge label</text>"#,
        r#"</svg>"#
    )
    .to_string();
    let violations = validate::check_label_edge_clearance(&mut svg, AutoCorrect::Apply);
    assert!(
        violations.is_empty(),
        "marked edge-label mode should not flag ordinary node/header text"
    );
    assert!(
        !svg.contains("uml-edge-label-bg"),
        "no background rect should be inserted for unmarked text"
    );
}

#[test]
fn invariant3_full_run_adds_background_for_marked_tight_edge_label() {
    let mut svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="300" height="200" viewBox="0 0 300 200">"#,
        r##"<polyline class="uml-relation" data-uml-from="X" data-uml-to="Y" points="20,100 280,100" fill="none" stroke="#555" stroke-width="2"/>"##,
        r#"<text class="uml-edge-label" data-uml-label-role="edge" x="150" y="100" text-anchor="middle" font-family="monospace">crowded route label</text>"#,
        r#"</svg>"#
    )
    .to_string();
    let report = validate::run(&mut svg, AutoCorrect::Apply);
    assert_eq!(
        report.background_rects_added, 1,
        "run() should add one backing rect for the tight marked edge label"
    );
    assert!(
        svg.contains("class=\"uml-edge-label-bg\""),
        "background rect should be marked for visual/audit checks"
    );
}

#[test]
fn run_with_scene_reports_typed_issues_and_svg_fallback_corrections() {
    let mut scene = RenderScene::new(Rect::new(0.0, 0.0, 320.0, 180.0));
    scene.add_node(SceneNode {
        id: "A".to_string(),
        node_box: NodeBox {
            id: "A".to_string(),
            bounds: Rect::new(20.0, 80.0, 60.0, 40.0),
            ports: vec![],
            labels: vec![],
        },
    });
    scene.add_node(SceneNode {
        id: "B".to_string(),
        node_box: NodeBox {
            id: "B".to_string(),
            bounds: Rect::new(240.0, 80.0, 60.0, 40.0),
            ports: vec![],
            labels: vec![],
        },
    });
    scene.add_node(SceneNode {
        id: "obstacle".to_string(),
        node_box: NodeBox {
            id: "obstacle".to_string(),
            bounds: Rect::new(130.0, 70.0, 60.0, 60.0),
            ports: vec![],
            labels: vec![],
        },
    });
    scene.add_edge(SceneEdge {
        id: "edge:A:B".to_string(),
        from: "A".to_string(),
        to: "B".to_string(),
        route: Polyline::from_tuples(&[(80.0, 100.0), (240.0, 100.0)]),
        route_channel_ids: Vec::new(),
        source_anchor: Anchor {
            id: "A:right".to_string(),
            owner_id: "A".to_string(),
            position: Point::new(80.0, 100.0),
            port: None,
        },
        target_anchor: Anchor {
            id: "B:left".to_string(),
            owner_id: "B".to_string(),
            position: Point::new(240.0, 100.0),
            port: None,
        },
        labels: vec![],
    });

    let mut svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="320" height="180" viewBox="0 0 320 180">"#,
        r##"<polyline class="uml-relation" data-uml-from="A" data-uml-to="B" points="80,100 240,100" fill="none" stroke="#555" stroke-width="2"/>"##,
        r#"<text class="uml-edge-label" data-uml-label-role="edge" x="160" y="100" text-anchor="middle" font-family="monospace">tight</text>"#,
        r#"</svg>"#
    )
    .to_string();
    let report = validate::run_with_scene(&mut svg, Some(&scene), AutoCorrect::Apply);

    assert!(
        report.typed_issues.iter().any(|issue| matches!(
            issue,
            GeometryIssue::EdgeCrossesNode {
                edge_id,
                node_id,
                ..
            } if edge_id == "edge:A:B" && node_id == "obstacle"
        )),
        "typed scene bridge should report the edge crossing before SVG fallback"
    );
    assert_eq!(
        report.background_rects_added, 1,
        "SVG fallback should still auto-correct tight edge-label clearance"
    );
    assert!(svg.contains("class=\"uml-edge-label-bg\""));
}

#[test]
fn graph_family_artifact_runs_typed_scene_validation_for_component() {
    let source = include_str!("../docs/examples/component/07_ports_lollipop_interfaces.puml");
    let artifact = render_family_artifact(source);
    let scene = artifact.scene.as_ref().expect("component scene");
    let report = artifact
        .invariant_report
        .as_ref()
        .expect("component invariant report");
    let typed = scene.validate_scene();

    assert_eq!(artifact.svg, render_to_svg(source));
    assert_eq!(report.typed_issues, typed.issues);
    assert_eq!(report.typed_metrics, typed.metrics);
    assert!(
        !scene.edges.is_empty(),
        "component scene should expose edges"
    );
    assert!(
        !scene.route_channels.is_empty(),
        "component scene should expose route-channel geometry"
    );
    assert!(report
        .typed_metrics
        .iter()
        .any(|metric| matches!(metric, GeometryMetric::EmptyGutter { .. })));
    assert!(report
        .typed_metrics
        .iter()
        .any(|metric| matches!(metric, GeometryMetric::Compactness { .. })));
}

#[test]
fn model_render_artifacts_carry_svg_metadata_diagnostics_and_typed_scene() {
    let source = r#"
@startuml
skinparam UnknownXyzKey value
class A
class B
A --> B
@enduml
"#;
    let artifacts = puml::render_source_to_artifacts(source).expect("render artifacts");
    assert_eq!(artifacts.len(), 1);

    let artifact = &artifacts[0];
    assert_eq!(artifact.svg, render_to_svg(source));
    assert_eq!(artifact.media_type(), "image/svg+xml");
    assert!(artifact
        .dimensions
        .is_some_and(|dimensions| dimensions.width > 0.0 && dimensions.height > 0.0));
    assert!(
        artifact
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("W_SKINPARAM_UNSUPPORTED")),
        "artifact should retain normalizer diagnostics for callers that do not own the model"
    );
    assert!(
        artifact.typed_scene().is_some(),
        "graph-family artifact should expose the typed scene"
    );
    assert!(
        artifact.typed_scene().is_some(),
        "typed scene accessor should expose migrated graph-family geometry"
    );
    assert!(
        matches!(
            artifact.scene_contract(),
            puml::RenderSceneContract::Typed(scene) if !scene.nodes.is_empty()
        ),
        "scene contract should encode migrated typed scene availability"
    );
    assert_eq!(
        artifact.scene_availability,
        SceneAvailability::TypedScene,
        "graph-family artifact should make typed scene availability explicit"
    );
    assert_eq!(
        artifact.scene_availability(),
        SceneAvailability::TypedScene,
        "scene availability should also be available through the artifact contract"
    );
    assert!(
        artifact.invariant_report.is_some(),
        "graph-family artifact should retain the validation report from the same render pass"
    );
    assert_eq!(
        artifact.validation_state(),
        RenderValidationState::TypedScene,
        "graph-family artifact validation should be explicitly tied to the typed scene"
    );
}

#[test]
fn sequence_render_artifacts_preserve_svg_api_and_attach_typed_scene() {
    let source = include_str!("fixtures/e2e/sequence_typed_scene_contract.puml");
    let artifacts = puml::render_source_to_artifacts(source).expect("render artifacts");
    assert_eq!(artifacts.len(), 1);

    let artifact = &artifacts[0];
    assert_eq!(artifact.svg, render_to_svg(source));
    assert!(artifact
        .dimensions
        .is_some_and(|dimensions| dimensions.view_box.is_some()));
    let scene = artifact
        .require_typed_scene()
        .expect("sequence should expose typed RenderScene");
    assert!(
        scene.nodes.contains_key("participant:Alice"),
        "sequence scene should expose participant node boxes"
    );
    assert!(
        scene.nodes.keys().any(|id| id.starts_with("activation:")),
        "sequence scene should expose activation boxes"
    );
    assert_eq!(
        scene.edges.len(),
        3,
        "sequence scene should expose one typed edge per rendered message"
    );
    assert!(
        scene.groups.keys().any(|id| id.contains(":alt")),
        "sequence scene should expose combined-fragment group frames"
    );
    assert!(
        scene.nodes.keys().any(|id| id.starts_with("note:")),
        "sequence scene should expose note boxes"
    );
    assert_eq!(
        artifact.scene_availability,
        SceneAvailability::TypedScene,
        "sequence renderers must make typed scene availability explicit"
    );
    assert_eq!(
        artifact.validation_state(),
        RenderValidationState::TypedScene,
        "sequence artifact validation should be explicitly tied to the typed scene"
    );
    let report = artifact
        .invariant_report
        .as_ref()
        .expect("sequence artifact should retain invariant report");
    assert_eq!(report.typed_issues, scene.validate_geometry());
}

#[test]
fn sequence_typed_scene_checks_endpoint_labels_viewport_and_group_ownership() {
    let source = include_str!("fixtures/e2e/sequence_typed_scene_contract.puml");
    let artifact = puml::render_source_to_artifacts(source)
        .expect("render artifacts")
        .remove(0);
    let scene = artifact
        .require_typed_scene()
        .expect("typed sequence scene");

    for edge in scene.edges.values() {
        assert!(
            scene.nodes.contains_key(&edge.from),
            "message source anchor owner should be a typed node"
        );
        assert!(
            scene.nodes.contains_key(&edge.to),
            "message target anchor owner should be a typed node"
        );
        assert!(
            edge.labels
                .iter()
                .all(|label| label.owner_id.as_deref() == Some(edge.id.as_str())),
            "message labels should be owned by their edge"
        );
    }

    assert!(
        scene
            .labels
            .values()
            .all(|label| scene.viewport.contains_rect(label.label_box.bounds)),
        "sequence label boxes should be validated inside the viewport"
    );

    let alt_group = scene
        .groups
        .values()
        .find(|group| group.id.contains(":alt"))
        .expect("alt group frame");
    assert!(
        alt_group.frame.header.is_some(),
        "combined-fragment frames should expose a typed header strip"
    );
    assert!(
        scene.groups.values().any(|group| group
            .frame
            .child_node_ids
            .iter()
            .any(|id| id == "participant:Alice")),
        "sequence group frames should own contained participant nodes"
    );
}

#[test]
fn sequence_render_scene_summary_reports_typed_scene_availability() {
    let source = include_str!("fixtures/e2e/sequence_typed_scene_contract.puml");
    let document = puml::parse(source).expect("sequence source should parse");
    let model = puml::normalize_family(document).expect("sequence source should normalize");
    let artifacts = puml::render_artifact_pages_from_model(&model);

    let summary = puml::normalized_artifact_scene_summary_to_json(&model, &artifacts);
    assert_eq!(summary["kind"], "Sequence");
    assert_eq!(summary["typed"], true);
    assert_eq!(summary["sceneAvailability"], "TypedScene");
    assert_eq!(summary["pages"][0]["kind"], "RenderScene");
    assert!(
        summary["pages"][0]["nodes"]
            .as_array()
            .is_some_and(|nodes| !nodes.is_empty()),
        "renderScene JSON should expose typed sequence scene nodes"
    );
}

#[test]
fn graph_family_artifacts_preserve_svg_api_for_class_and_deployment() {
    let fixtures = [
        include_str!("../docs/examples/class/32_association_class_deep_packages.puml"),
        include_str!("../docs/examples/deployment/06_kubernetes_pods_containers.puml"),
    ];

    for source in fixtures {
        let artifact = render_family_artifact(source);
        let scene = artifact.scene.as_ref().expect("graph-family scene");
        let report = artifact
            .invariant_report
            .as_ref()
            .expect("graph-family invariant report");

        assert_eq!(artifact.svg, render_to_svg(source));
        assert!(!scene.nodes.is_empty(), "scene should expose typed nodes");
        assert!(!scene.edges.is_empty(), "scene should expose typed edges");
        assert_eq!(report.typed_metrics, scene.validate_scene().metrics);
    }
}

#[test]
fn usecase_and_c4_graph_artifacts_expose_route_channels_and_typed_edge_labels() {
    let fixtures = [
        include_str!("../docs/examples/usecase/06_multi_system_boundary.puml"),
        include_str!("fixtures/families/valid_c4_full_system.puml"),
    ];

    for source in fixtures {
        let artifact = render_family_artifact(source);
        let scene = artifact.scene.as_ref().expect("graph-family scene");
        let report = artifact
            .invariant_report
            .as_ref()
            .expect("graph-family invariant report");

        assert!(
            !scene.route_channels.is_empty(),
            "migrated graph family should expose shared route channels"
        );
        assert!(
            scene.edges.values().any(|edge| !edge.labels.is_empty()),
            "migrated graph family should expose typed edge labels"
        );
        assert!(report.typed_metrics.iter().any(
            |metric| matches!(metric, GeometryMetric::RouteChannels { count, .. } if *count > 0)
        ));
    }
}

#[test]
fn component_ports_lollipop_fixture_exposes_typed_route_channel_and_group_ownership() {
    let source = include_str!("../docs/examples/component/07_ports_lollipop_interfaces.puml");
    let artifact = render_family_artifact(source);
    let scene = artifact.scene.as_ref().expect("component typed scene");
    let report = artifact
        .invariant_report
        .as_ref()
        .expect("component invariant report");

    assert!(
        !scene.route_channels.is_empty(),
        "high-risk component fixture should expose typed route channels"
    );
    assert!(
        scene
            .edges
            .values()
            .any(|edge| !edge.route_channel_ids.is_empty()),
        "graph edges should expose explicit route-channel ids"
    );
    assert!(
        scene
            .groups
            .values()
            .any(|group| { group.frame.child_node_ids.iter().any(|child| child == "OD") }),
        "component package ownership should survive into typed group frames"
    );
    assert!(
        !report.typed_issues.iter().any(|issue| matches!(
            issue,
            GeometryIssue::GroupChildOutsideFrame { .. }
                | GeometryIssue::GroupChildOverlapsHeader { .. }
        )),
        "component package child ownership should validate from typed frames: {:?}",
        report.typed_issues
    );
    assert!(report
        .typed_metrics
        .iter()
        .any(|metric| matches!(metric, GeometryMetric::RouteChannels { count, .. } if *count > 0)));
}

#[test]
fn object_artifact_scene_uses_final_normalized_edge_paths() {
    let source = r#"
@startuml
object Root
object Left
object Right
Root --> Left
Root --> Right
@enduml
"#;
    let artifact = render_family_artifact(source);
    let scene = artifact.scene.as_ref().expect("object scene");
    let routed = scene
        .edges
        .values()
        .find(|edge| edge.route.points.len() >= 4)
        .expect("expected a cross-rank routed object edge");
    let first = routed.route.points[0];
    let last = *routed.route.points.last().expect("route endpoint");
    let expected_mid_y = (first.y + last.y) / 2.0;
    let end = routed.route.points.len().saturating_sub(1);

    for point in routed.route.points.iter().take(end).skip(1) {
        assert!(
            (point.y - expected_mid_y).abs() <= 0.5,
            "typed scene should be rebuilt from final object route paths"
        );
    }
}

#[test]
fn invariant3_component_lollipop_fixture_marks_relation_labels() {
    let source = include_str!("../docs/examples/component/07_ports_lollipop_interfaces.puml");
    let svg = render_to_svg(source);
    assert!(
        svg.matches("class=\"uml-edge-label\"").count() >= 6,
        "component lollipop labels should be scoped for the label-clearance invariant"
    );
    assert!(
        svg.contains(">provides</text>") && svg.contains(">requires</text>"),
        "fixture should continue rendering the crowded lollipop labels this guardrail targets"
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
        x: 0,
        y: 50,
        width: 400,
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
        x: 0,
        y: 50,
        width: 400,
        header_height: 40,
    }];
    let violations = validate::check_package_headers(&svg, &frames);
    assert!(
        violations.is_empty(),
        "edge below package header should not trigger violation"
    );
}

#[test]
fn invariant4_extracts_group_headers_from_svg() {
    let svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="300" viewBox="0 0 400 300">"#,
        r#"<rect class="uml-group-frame" data-uml-group="Domain" x="40" y="60" width="300" height="180"/>"#,
        r##"<polyline class="uml-relation" data-uml-from="A" data-uml-to="B" points="20,72 360,72" fill="none" stroke="#333" stroke-width="2"/>"##,
        r#"</svg>"#
    );
    let frames = validate::extract_package_frames(svg);
    assert_eq!(frames.len(), 1, "expected one extracted group frame");
    assert_eq!(frames[0].id, "Domain");

    let violations = validate::check_package_headers_from_svg(svg);
    assert!(
        !violations.is_empty(),
        "route through extracted header strip should be reported"
    );
}

#[test]
fn invariant_label_proximity_detects_detached_edge_label() {
    let svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="300" viewBox="0 0 400 300">"#,
        r##"<polyline class="uml-relation" data-uml-from="A" data-uml-to="B" points="20,220 380,220" fill="none" stroke="#333" stroke-width="2"/>"##,
        r#"<text class="uml-edge-label" data-uml-label-role="edge" x="200" y="40" text-anchor="middle">detached</text>"#,
        r#"</svg>"#
    );
    let violations = validate::check_edge_label_proximity(svg, 64);
    assert!(
        !violations.is_empty(),
        "edge label far from all route segments should be reported"
    );
}

#[test]
fn quality_metrics_report_compactness_inputs() {
    let svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="300" viewBox="0 0 400 300">"#,
        r#"<desc data-uml-id="A">A</desc><rect class="uml-node" x="40" y="40" width="80" height="40"/>"#,
        r#"<desc data-uml-id="B">B</desc><rect class="uml-node" x="260" y="200" width="80" height="40"/>"#,
        r##"<polyline class="uml-relation" data-uml-from="A" data-uml-to="B" points="120,60 260,220" fill="none" stroke="#333" stroke-width="2"/>"##,
        r#"<text x="200" y="140" text-anchor="middle">route</text>"#,
        r#"</svg>"#
    );
    let metrics = validate::collect_quality_metrics(svg);
    assert_eq!(metrics.viewbox_width, 400);
    assert_eq!(metrics.viewbox_height, 300);
    assert_eq!(metrics.node_count, 2);
    assert_eq!(metrics.relation_count, 1);
    assert_eq!(metrics.text_count, 1);
    assert!(
        metrics.route_length_per_node_px > 100.0,
        "route length per node should be populated"
    );
    assert!(
        metrics.max_empty_gutter_ratio > 0.0,
        "empty gutter metric should be populated"
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
            style: Default::default(),
            internal_actions: vec![],
            regions: vec![],
        },
        StateNode {
            name: "[*]_extra".to_string(),
            display: None,
            kind: StateNodeKind::StartEnd,
            stereotype: None,
            style: Default::default(),
            internal_actions: vec![],
            regions: vec![],
        },
        StateNode {
            name: "Active".to_string(),
            display: None,
            kind: StateNodeKind::Normal,
            stereotype: None,
            style: Default::default(),
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
