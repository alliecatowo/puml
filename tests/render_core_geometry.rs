use puml::render_core::{
    Anchor, GeometryIssue, LabelBox, LabelRole, NodeBox, Point, Polyline, Rect, RenderScene,
    SceneEdge, SceneNode,
};

#[test]
fn typed_scene_reports_labels_outside_viewport_without_svg_scraping() {
    let mut scene = RenderScene::new(Rect::new(0.0, 0.0, 100.0, 80.0));
    let label = LabelBox {
        id: "node:A:label".to_string(),
        text: "A".to_string(),
        bounds: Rect::new(92.0, 10.0, 16.0, 12.0),
        owner_id: Some("A".to_string()),
        role: LabelRole::Node,
    };
    scene.add_node(SceneNode {
        id: "A".to_string(),
        node_box: NodeBox {
            id: "A".to_string(),
            bounds: Rect::new(10.0, 10.0, 40.0, 30.0),
            ports: Vec::new(),
            labels: vec![label],
        },
    });

    let issues = scene.validate_geometry();
    assert!(
        matches!(
            issues.as_slice(),
            [GeometryIssue::LabelOutsideViewport { label_id, .. }] if label_id == "node:A:label"
        ),
        "expected one typed label viewport issue, got {issues:?}"
    );
}

#[test]
fn typed_scene_reports_detached_edge_endpoint() {
    let mut scene = RenderScene::new(Rect::new(0.0, 0.0, 200.0, 120.0));
    scene.add_edge(SceneEdge {
        id: "e1".to_string(),
        from: "A".to_string(),
        to: "B".to_string(),
        route: Polyline::new(vec![Point::new(20.0, 20.0), Point::new(160.0, 20.0)]),
        source_anchor: Anchor {
            id: "e1:source".to_string(),
            owner_id: "A".to_string(),
            position: Point::new(10.0, 20.0),
            port: None,
        },
        target_anchor: Anchor {
            id: "e1:target".to_string(),
            owner_id: "B".to_string(),
            position: Point::new(160.0, 20.0),
            port: None,
        },
        labels: Vec::new(),
    });

    let issues = scene.validate_geometry();
    assert!(
        matches!(
            issues.as_slice(),
            [GeometryIssue::EdgeEndpointDetached { edge_id, anchor_id, .. }]
                if edge_id == "e1" && anchor_id == "e1:source"
        ),
        "expected one detached endpoint issue, got {issues:?}"
    );
}

#[test]
fn component_render_smoke_still_runs_with_graph_layout_scene_population() {
    let source = r#"
@startuml
package "API" {
  component Gateway
}
package "Core" {
  component Service
}
Gateway --> Service : calls
@enduml
"#;

    let svg = puml::render_source_to_svg(source).expect("component render should succeed");
    assert!(svg.contains("<svg"));
    assert!(svg.contains("uml-relation"));
    assert!(svg.contains("Gateway"));
    assert!(svg.contains("Service"));
}
