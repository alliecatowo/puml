use super::*;

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-6,
        "expected {actual} to be close to {expected}"
    );
}

#[test]
fn rect_overlap_respects_clearance() {
    let a = Rect::new(10.0, 10.0, 20.0, 20.0);
    let b = Rect::new(32.0, 10.0, 20.0, 20.0);
    assert!(!a.overlaps(b, 1.0));
    assert!(a.overlaps(b, 3.0));
}

#[test]
fn rect_translation_inflation_union_and_intersection() {
    let a = Rect::new(10.0, 20.0, 30.0, 40.0);
    assert_eq!(a.translated(5.0, -10.0), Rect::new(15.0, 10.0, 30.0, 40.0));
    assert_eq!(a.inflated(2.0), Rect::new(8.0, 18.0, 34.0, 44.0));

    let b = Rect::new(30.0, 40.0, 20.0, 10.0);
    assert_eq!(a.union(b), Rect::new(10.0, 20.0, 40.0, 40.0));
    assert_eq!(a.intersection(b), Some(Rect::new(30.0, 40.0, 10.0, 10.0)));
}

#[test]
fn rect_clearance_reports_axis_and_diagonal_gaps() {
    let a = Rect::new(0.0, 0.0, 10.0, 10.0);
    let horizontal = Rect::new(15.0, 2.0, 4.0, 4.0);
    let diagonal = Rect::new(13.0, 14.0, 4.0, 4.0);
    assert_close(a.clearance_to(horizontal), 5.0);
    assert_close(a.clearance_to(diagonal), 5.0);
    assert!(a.has_clearance(horizontal, 5.0));
    assert!(!a.has_clearance(horizontal, 6.0));
}

#[test]
fn shape_anchors_follow_shape_boundaries() {
    let bbox = Rect::new(0.0, 0.0, 100.0, 50.0);
    assert_eq!(
        ShapeKind::Rect.anchor_towards(bbox, Point::new(200.0, 25.0)),
        Point::new(100.0, 25.0)
    );
    assert_eq!(
        ShapeKind::Diamond.anchor_towards(bbox, Point::new(200.0, 25.0)),
        Point::new(100.0, 25.0)
    );

    let ellipse_anchor = ShapeKind::Ellipse.anchor_towards(bbox, Point::new(50.0, 100.0));
    assert_close(ellipse_anchor.x, 50.0);
    assert_close(ellipse_anchor.y, 50.0);

    let diagonal_diamond = ShapeKind::Diamond.anchor_towards(bbox, Point::new(100.0, 50.0));
    assert_close(diagonal_diamond.x, 75.0);
    assert_close(diagonal_diamond.y, 37.5);
}

#[test]
fn circle_bounds_are_centered_inside_non_square_bbox() {
    let bbox = Rect::new(0.0, 0.0, 100.0, 40.0);
    assert_eq!(
        ShapeKind::Circle.bounds_for(bbox),
        Rect::new(30.0, 0.0, 40.0, 40.0)
    );
}

#[test]
fn obstacle_set_finds_collisions_and_clearance() {
    let obstacles = ObstacleSet::from_rects([Rect::new(0.0, 0.0, 10.0, 10.0)]);
    let candidate = Rect::new(15.0, 0.0, 5.0, 5.0);
    assert!(obstacles.is_clear(candidate, 5.0));
    assert!(!obstacles.is_clear(candidate, 6.0));
    assert_eq!(
        obstacles.first_collision(candidate, 6.0),
        Some(Rect::new(0.0, 0.0, 10.0, 10.0))
    );
    assert_eq!(obstacles.min_clearance(candidate), Some(5.0));
}

#[test]
fn render_scene_validation_catches_reference_and_geometry_issues() {
    let scene = RenderScene {
        family: "test".to_string(),
        viewbox: Rect::new(0.0, 0.0, 100.0, 100.0),
        nodes: vec![VisualNode {
            id: "n1".to_string(),
            family: "test".to_string(),
            kind: "node".to_string(),
            shape: ShapeKind::Rect,
            bbox: Rect::new(10.0, 10.0, 20.0, 20.0),
            label_ids: Vec::new(),
            parent_id: None,
        }],
        edges: vec![VisualEdge {
            id: "e1".to_string(),
            from: "n1".to_string(),
            to: "missing".to_string(),
            points: vec![Point::new(20.0, 20.0)],
            source_port: None,
            target_port: None,
            label_ids: Vec::new(),
            route_kind: "direct".to_string(),
        }],
        labels: Vec::new(),
        containers: Vec::new(),
        diagnostics: Vec::new(),
    };

    let issues = scene.validate();
    assert!(issues
        .iter()
        .any(|issue| issue.kind == SceneValidationKind::MissingReference));
    assert!(issues
        .iter()
        .any(|issue| issue.kind == SceneValidationKind::DegenerateGeometry));
    assert!(scene.into_validated().is_err());
}

#[test]
fn render_scene_visual_bounds_unions_all_visuals() {
    let mut scene = RenderScene::new("test", Rect::new(0.0, 0.0, 200.0, 200.0));
    scene.nodes.push(VisualNode {
        id: "n1".to_string(),
        family: "test".to_string(),
        kind: "node".to_string(),
        shape: ShapeKind::Rect,
        bbox: Rect::new(10.0, 10.0, 20.0, 20.0),
        label_ids: Vec::new(),
        parent_id: None,
    });
    scene.edges.push(VisualEdge {
        id: "e1".to_string(),
        from: "n1".to_string(),
        to: "n1".to_string(),
        points: vec![Point::new(150.0, 150.0), Point::new(160.0, 170.0)],
        source_port: None,
        target_port: None,
        label_ids: Vec::new(),
        route_kind: "loop".to_string(),
    });

    assert_eq!(
        scene.visual_bounds(),
        Some(Rect::new(10.0, 10.0, 150.0, 160.0))
    );
}

#[test]
fn render_scene_validation_produces_opaque_validated_scene() {
    let mut scene = RenderScene::new("test", Rect::new(0.0, 0.0, 200.0, 200.0));
    scene.nodes.push(VisualNode {
        id: "n1".to_string(),
        family: "test".to_string(),
        kind: "node".to_string(),
        shape: ShapeKind::Rect,
        bbox: Rect::new(10.0, 10.0, 40.0, 30.0),
        label_ids: vec!["l1".to_string()],
        parent_id: None,
    });
    scene.labels.push(VisualLabel {
        id: "l1".to_string(),
        owner_id: "n1".to_string(),
        kind: "node-label".to_string(),
        text: "Node".to_string(),
        anchor: Point::new(20.0, 25.0),
        estimated_bbox: Rect::new(12.0, 14.0, 24.0, 14.0),
    });

    let validated = scene.into_validated().expect("scene should validate");
    assert_eq!(validated.scene().nodes.len(), 1);
    assert_eq!(validated.warnings().count(), 0);
}

#[test]
fn text_bbox_is_centered_for_middle_anchor() {
    let bbox = estimate_text_bbox(100.0, 50.0, "abcd", 10.0, true);
    assert!(bbox.x < 100.0);
    assert!(bbox.right() > 100.0);
    assert!(bbox.y < 50.0);
}
