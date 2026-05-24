use puml::render_core::{
    validate::GeometryMetric, Anchor, GeometryIssue, LabelBox, LabelRole, NodeBox, Point, Polyline,
    Port, PortSide, Rect, RenderScene, SceneEdge, SceneNode,
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
fn typed_scene_reports_edge_crossing_non_endpoint_node() {
    let mut scene = RenderScene::new(Rect::new(0.0, 0.0, 220.0, 120.0));
    scene.add_node(node("A", Rect::new(10.0, 40.0, 30.0, 30.0), Vec::new()));
    scene.add_node(node("B", Rect::new(170.0, 40.0, 30.0, 30.0), Vec::new()));
    scene.add_node(node("C", Rect::new(85.0, 35.0, 40.0, 40.0), Vec::new()));
    scene.add_edge(SceneEdge {
        id: "e1".to_string(),
        from: "A".to_string(),
        to: "B".to_string(),
        route: Polyline::new(vec![Point::new(40.0, 55.0), Point::new(170.0, 55.0)]),
        source_anchor: anchor("e1:source", "A", 40.0, 55.0),
        target_anchor: anchor("e1:target", "B", 170.0, 55.0),
        labels: Vec::new(),
    });

    let issues = scene.validate_geometry();
    assert!(
        matches!(
            issues.as_slice(),
            [GeometryIssue::EdgeCrossesNode { edge_id, node_id, .. }]
                if edge_id == "e1" && node_id == "C"
        ),
        "expected edge/node crossing issue, got {issues:?}"
    );
}

#[test]
fn typed_scene_reports_endpoint_not_attached_to_declared_port() {
    let mut scene = RenderScene::new(Rect::new(0.0, 0.0, 220.0, 120.0));
    scene.add_node(node(
        "A",
        Rect::new(10.0, 40.0, 30.0, 30.0),
        vec![port("A:left", "A", PortSide::Left, 10.0, 55.0)],
    ));
    scene.add_node(node(
        "B",
        Rect::new(170.0, 40.0, 30.0, 30.0),
        vec![port("B:left", "B", PortSide::Left, 170.0, 55.0)],
    ));
    scene.add_edge(SceneEdge {
        id: "e1".to_string(),
        from: "A".to_string(),
        to: "B".to_string(),
        route: Polyline::new(vec![Point::new(45.0, 55.0), Point::new(170.0, 55.0)]),
        source_anchor: anchor("e1:source", "A", 45.0, 55.0),
        target_anchor: Anchor {
            port: Some(port("B:left", "B", PortSide::Left, 170.0, 55.0)),
            ..anchor("e1:target", "B", 170.0, 55.0)
        },
        labels: Vec::new(),
    });

    let issues = scene.validate_geometry();
    assert!(
        matches!(
            issues.as_slice(),
            [GeometryIssue::EdgeEndpointMissingDeclaredPort { edge_id, anchor_id, node_id, .. }]
                if edge_id == "e1" && anchor_id == "e1:source" && node_id == "A"
        ),
        "expected missing declared port issue, got {issues:?}"
    );
}

#[test]
fn typed_scene_reports_non_fatal_compactness_and_gutter_metrics() {
    let mut scene = RenderScene::new(Rect::new(0.0, 0.0, 200.0, 100.0));
    scene.add_node(node("A", Rect::new(25.0, 10.0, 50.0, 40.0), Vec::new()));

    let report = scene.validate_scene();
    assert!(report.issues.is_empty(), "unexpected issues: {report:?}");
    assert!(
        report.metrics.iter().any(|metric| {
            matches!(
                metric,
                GeometryMetric::EmptyGutter {
                    left,
                    right,
                    top,
                    bottom,
                    ..
                } if *left == 25.0 && *right == 125.0 && *top == 10.0 && *bottom == 50.0
            )
        }),
        "expected empty-gutter metric, got {:?}",
        report.metrics
    );
    assert!(
        report.metrics.iter().any(|metric| matches!(
            metric,
            GeometryMetric::Compactness {
                viewport_area,
                content_area,
                fill_ratio,
                aspect_ratio,
            } if *viewport_area == 20_000.0
                && *content_area == 2_000.0
                && (*fill_ratio - 0.1).abs() < f64::EPSILON
                && *aspect_ratio == 2.0
        )),
        "expected compactness metric, got {:?}",
        report.metrics
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

fn node(id: &str, bounds: Rect, ports: Vec<Port>) -> SceneNode {
    SceneNode {
        id: id.to_string(),
        node_box: NodeBox {
            id: id.to_string(),
            bounds,
            ports,
            labels: Vec::new(),
        },
    }
}

fn anchor(id: &str, owner_id: &str, x: f64, y: f64) -> Anchor {
    Anchor {
        id: id.to_string(),
        owner_id: owner_id.to_string(),
        position: Point::new(x, y),
        port: None,
    }
}

fn port(id: &str, node_id: &str, side: PortSide, x: f64, y: f64) -> Port {
    Port {
        id: id.to_string(),
        node_id: node_id.to_string(),
        side,
        position: Point::new(x, y),
    }
}
