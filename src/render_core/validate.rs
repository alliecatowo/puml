//! Typed pre-SVG geometry validation for [`RenderScene`].
//!
//! The SVG validator in `render::validate` remains as a compatibility backstop
//! for unmigrated renderers. This module validates renderer-neutral scene
//! geometry before backend serialization.

use super::{Anchor, GeometryIssue, Point, Rect, RenderScene, SceneEdge, SceneNode, Segment};

const ENDPOINT_TOLERANCE: f64 = 0.5;
const PORT_TOLERANCE: f64 = 0.5;
const EDGE_LABEL_MAX_DISTANCE: f64 = 96.0;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SceneValidationReport {
    pub issues: Vec<GeometryIssue>,
    pub metrics: Vec<GeometryMetric>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GeometryMetric {
    EmptyGutter {
        left: f64,
        right: f64,
        top: f64,
        bottom: f64,
        viewport: Rect,
        content_bounds: Rect,
    },
    Compactness {
        viewport_area: f64,
        content_area: f64,
        fill_ratio: f64,
        aspect_ratio: f64,
    },
    RouteChannels {
        count: usize,
        total_area: f64,
    },
}

pub fn validate_scene(scene: &RenderScene) -> SceneValidationReport {
    let mut report = SceneValidationReport::default();
    validate_viewport_containment(scene, &mut report.issues);
    validate_edge_routes(scene, &mut report.issues);
    collect_quality_metrics(scene, &mut report.metrics);
    report
}

fn validate_viewport_containment(scene: &RenderScene, issues: &mut Vec<GeometryIssue>) {
    for node in scene.nodes.values() {
        if !scene.viewport.contains_rect(node.node_box.bounds) {
            issues.push(GeometryIssue::NodeOutsideViewport {
                node_id: node.id.clone(),
                bounds: node.node_box.bounds,
                viewport: scene.viewport,
            });
        }
    }
    for group in scene.groups.values() {
        if !scene.viewport.contains_rect(group.frame.bounds) {
            issues.push(GeometryIssue::GroupOutsideViewport {
                group_id: group.id.clone(),
                bounds: group.frame.bounds,
                viewport: scene.viewport,
            });
        }
    }
    for lane in scene.lanes.values() {
        if !scene.viewport.contains_rect(lane.bounds) {
            issues.push(GeometryIssue::LaneOutsideViewport {
                lane_id: lane.id.clone(),
                bounds: lane.bounds,
                viewport: scene.viewport,
            });
        }
    }
    for label in scene.labels.values() {
        if !scene.viewport.contains_rect(label.label_box.bounds) {
            issues.push(GeometryIssue::LabelOutsideViewport {
                label_id: label.id.clone(),
                bounds: label.label_box.bounds,
                viewport: scene.viewport,
            });
        }
    }
}

fn validate_edge_routes(scene: &RenderScene, issues: &mut Vec<GeometryIssue>) {
    for edge in scene.edges.values() {
        validate_edge_endpoints(scene, edge, issues);
        validate_edge_node_clearance(scene, edge, issues);
        validate_edge_group_header_clearance(scene, edge, issues);
        validate_edge_label_proximity(edge, issues);
    }
}

fn validate_edge_endpoints(scene: &RenderScene, edge: &SceneEdge, issues: &mut Vec<GeometryIssue>) {
    let Some(first) = edge.route.first() else {
        issues.push(GeometryIssue::EdgeMissingRoute {
            edge_id: edge.id.clone(),
        });
        return;
    };
    let Some(last) = edge.route.last() else {
        issues.push(GeometryIssue::EdgeMissingRoute {
            edge_id: edge.id.clone(),
        });
        return;
    };

    validate_anchor_endpoint(scene, edge, &edge.source_anchor, &edge.from, first, issues);
    validate_anchor_endpoint(scene, edge, &edge.target_anchor, &edge.to, last, issues);
}

fn validate_anchor_endpoint(
    scene: &RenderScene,
    edge: &SceneEdge,
    anchor: &Anchor,
    expected_node_id: &str,
    route_point: Point,
    issues: &mut Vec<GeometryIssue>,
) {
    if route_point.distance_to(anchor.position) > ENDPOINT_TOLERANCE {
        issues.push(GeometryIssue::EdgeEndpointDetached {
            edge_id: edge.id.clone(),
            anchor_id: anchor.id.clone(),
            expected: anchor.position,
            actual: route_point,
        });
    }

    if anchor.owner_id != expected_node_id {
        issues.push(GeometryIssue::EdgeAnchorOwnerMismatch {
            edge_id: edge.id.clone(),
            anchor_id: anchor.id.clone(),
            expected_node_id: expected_node_id.to_string(),
            actual_owner_id: anchor.owner_id.clone(),
        });
    }

    let Some(node) = scene.nodes.get(expected_node_id) else {
        return;
    };
    if node.node_box.ports.is_empty() {
        return;
    }

    let matching_declared_port = node
        .node_box
        .ports
        .iter()
        .any(|port| port.position.distance_to(anchor.position) <= PORT_TOLERANCE);
    if !matching_declared_port {
        issues.push(GeometryIssue::EdgeEndpointMissingDeclaredPort {
            edge_id: edge.id.clone(),
            anchor_id: anchor.id.clone(),
            node_id: expected_node_id.to_string(),
            position: anchor.position,
        });
    }

    if let Some(port) = &anchor.port {
        let declared = node
            .node_box
            .ports
            .iter()
            .find(|candidate| candidate.id == port.id);
        if let Some(declared) = declared {
            if declared.position.distance_to(anchor.position) > PORT_TOLERANCE {
                issues.push(GeometryIssue::EdgeAnchorPortMismatch {
                    edge_id: edge.id.clone(),
                    anchor_id: anchor.id.clone(),
                    port_id: port.id.clone(),
                    expected: declared.position,
                    actual: anchor.position,
                });
            }
        } else {
            issues.push(GeometryIssue::EdgeEndpointMissingDeclaredPort {
                edge_id: edge.id.clone(),
                anchor_id: anchor.id.clone(),
                node_id: expected_node_id.to_string(),
                position: anchor.position,
            });
        }
    }
}

fn validate_edge_node_clearance(
    scene: &RenderScene,
    edge: &SceneEdge,
    issues: &mut Vec<GeometryIssue>,
) {
    for segment in edge.route.segments() {
        for node in scene.nodes.values() {
            if is_endpoint_node(edge, node) {
                continue;
            }
            if segment_crosses_rect(segment, node.node_box.bounds) {
                issues.push(GeometryIssue::EdgeCrossesNode {
                    edge_id: edge.id.clone(),
                    node_id: node.id.clone(),
                    segment,
                    node_bounds: node.node_box.bounds,
                });
            }
        }
    }
}

fn validate_edge_group_header_clearance(
    scene: &RenderScene,
    edge: &SceneEdge,
    issues: &mut Vec<GeometryIssue>,
) {
    for segment in edge.route.segments() {
        for group in scene.groups.values() {
            let Some(header) = group.frame.header else {
                continue;
            };
            if segment_crosses_rect_interior(segment, header) {
                issues.push(GeometryIssue::EdgeCrossesGroupHeader {
                    edge_id: edge.id.clone(),
                    group_id: group.id.clone(),
                    segment,
                    header_bounds: header,
                });
            }
        }
    }
}

fn validate_edge_label_proximity(edge: &SceneEdge, issues: &mut Vec<GeometryIssue>) {
    if edge.route.points.len() < 2 {
        return;
    }
    for label in &edge.labels {
        let center = label.bounds.center();
        let min_distance = edge
            .route
            .segments()
            .into_iter()
            .map(|segment| distance_point_to_segment(center, segment))
            .fold(f64::INFINITY, f64::min);
        if min_distance.is_finite() && min_distance > EDGE_LABEL_MAX_DISTANCE {
            issues.push(GeometryIssue::EdgeLabelDetached {
                edge_id: edge.id.clone(),
                label_id: label.id.clone(),
                bounds: label.bounds,
                min_distance,
                max_distance: EDGE_LABEL_MAX_DISTANCE,
            });
        }
    }
}

fn is_endpoint_node(edge: &SceneEdge, node: &SceneNode) -> bool {
    node.id == edge.from || node.id == edge.to
}

fn segment_crosses_rect(segment: Segment, rect: Rect) -> bool {
    if point_strictly_inside_rect(segment.start, rect)
        || point_strictly_inside_rect(segment.end, rect)
    {
        return true;
    }

    let top_left = Point::new(rect.min_x(), rect.min_y());
    let top_right = Point::new(rect.max_x(), rect.min_y());
    let bottom_right = Point::new(rect.max_x(), rect.max_y());
    let bottom_left = Point::new(rect.min_x(), rect.max_y());

    segments_intersect(segment, Segment::new(top_left, top_right))
        || segments_intersect(segment, Segment::new(top_right, bottom_right))
        || segments_intersect(segment, Segment::new(bottom_right, bottom_left))
        || segments_intersect(segment, Segment::new(bottom_left, top_left))
}

fn segment_crosses_rect_interior(segment: Segment, rect: Rect) -> bool {
    if !segment.bounds().intersects(rect) {
        return false;
    }
    if point_strictly_inside_rect(segment.start, rect)
        || point_strictly_inside_rect(segment.end, rect)
    {
        return true;
    }
    if segment.is_vertical() {
        return segment.start.x > rect.min_x()
            && segment.start.x < rect.max_x()
            && ranges_overlap_strict(segment.start.y, segment.end.y, rect.min_y(), rect.max_y());
    }
    if segment.is_horizontal() {
        return segment.start.y > rect.min_y()
            && segment.start.y < rect.max_y()
            && ranges_overlap_strict(segment.start.x, segment.end.x, rect.min_x(), rect.max_x());
    }
    segment_crosses_rect(segment, rect)
}

fn point_strictly_inside_rect(point: Point, rect: Rect) -> bool {
    point.x > rect.min_x()
        && point.x < rect.max_x()
        && point.y > rect.min_y()
        && point.y < rect.max_y()
}

fn ranges_overlap_strict(a: f64, b: f64, c: f64, d: f64) -> bool {
    let a_min = a.min(b);
    let a_max = a.max(b);
    a_min < d && c < a_max
}

fn distance_point_to_segment(point: Point, segment: Segment) -> f64 {
    let dx = segment.end.x - segment.start.x;
    let dy = segment.end.y - segment.start.y;
    let len_sq = dx * dx + dy * dy;
    if len_sq <= f64::EPSILON {
        return point.distance_to(segment.start);
    }
    let t = (((point.x - segment.start.x) * dx + (point.y - segment.start.y) * dy) / len_sq)
        .clamp(0.0, 1.0);
    let projection = Point::new(segment.start.x + t * dx, segment.start.y + t * dy);
    point.distance_to(projection)
}

fn segments_intersect(a: Segment, b: Segment) -> bool {
    let d1 = orientation(a.start, a.end, b.start);
    let d2 = orientation(a.start, a.end, b.end);
    let d3 = orientation(b.start, b.end, a.start);
    let d4 = orientation(b.start, b.end, a.end);

    if d1 * d2 < 0.0 && d3 * d4 < 0.0 {
        return true;
    }

    (d1.abs() <= f64::EPSILON && point_on_segment(b.start, a))
        || (d2.abs() <= f64::EPSILON && point_on_segment(b.end, a))
        || (d3.abs() <= f64::EPSILON && point_on_segment(a.start, b))
        || (d4.abs() <= f64::EPSILON && point_on_segment(a.end, b))
}

fn orientation(a: Point, b: Point, c: Point) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

fn point_on_segment(point: Point, segment: Segment) -> bool {
    point.x >= segment.start.x.min(segment.end.x) - f64::EPSILON
        && point.x <= segment.start.x.max(segment.end.x) + f64::EPSILON
        && point.y >= segment.start.y.min(segment.end.y) - f64::EPSILON
        && point.y <= segment.start.y.max(segment.end.y) + f64::EPSILON
}

fn collect_quality_metrics(scene: &RenderScene, metrics: &mut Vec<GeometryMetric>) {
    let Some(content_bounds) = scene_content_bounds(scene) else {
        return;
    };

    let left = (content_bounds.min_x() - scene.viewport.min_x()).max(0.0);
    let right = (scene.viewport.max_x() - content_bounds.max_x()).max(0.0);
    let top = (content_bounds.min_y() - scene.viewport.min_y()).max(0.0);
    let bottom = (scene.viewport.max_y() - content_bounds.max_y()).max(0.0);
    metrics.push(GeometryMetric::EmptyGutter {
        left,
        right,
        top,
        bottom,
        viewport: scene.viewport,
        content_bounds,
    });

    let viewport_area = scene.viewport.size.width.max(0.0) * scene.viewport.size.height.max(0.0);
    let content_area = content_bounds.size.width.max(0.0) * content_bounds.size.height.max(0.0);
    let fill_ratio = if viewport_area > 0.0 {
        (content_area / viewport_area).min(1.0)
    } else {
        0.0
    };
    let aspect_ratio = if scene.viewport.size.height > 0.0 {
        scene.viewport.size.width / scene.viewport.size.height
    } else {
        0.0
    };
    metrics.push(GeometryMetric::Compactness {
        viewport_area,
        content_area,
        fill_ratio,
        aspect_ratio,
    });
    if !scene.route_channels.is_empty() {
        let total_area = scene
            .route_channels
            .values()
            .map(|channel| channel.bounds.size.width.max(0.0) * channel.bounds.size.height.max(0.0))
            .sum();
        metrics.push(GeometryMetric::RouteChannels {
            count: scene.route_channels.len(),
            total_area,
        });
    }
}

fn scene_content_bounds(scene: &RenderScene) -> Option<Rect> {
    let mut bounds = None;
    for node in scene.nodes.values() {
        union_optional(&mut bounds, node.node_box.bounds);
    }
    for edge in scene.edges.values() {
        if let Some(route_bounds) = edge.route.bounds() {
            union_optional(&mut bounds, route_bounds);
        }
    }
    for group in scene.groups.values() {
        union_optional(&mut bounds, group.frame.bounds);
        if let Some(header) = group.frame.header {
            union_optional(&mut bounds, header);
        }
    }
    for lane in scene.lanes.values() {
        union_optional(&mut bounds, lane.bounds);
        if let Some(header) = lane.header {
            union_optional(&mut bounds, header);
        }
    }
    for label in scene.labels.values() {
        union_optional(&mut bounds, label.label_box.bounds);
    }
    for channel in scene.route_channels.values() {
        union_optional(&mut bounds, channel.bounds);
    }
    bounds
}

fn union_optional(bounds: &mut Option<Rect>, rect: Rect) {
    *bounds = Some(match bounds {
        Some(existing) => existing.union(rect),
        None => rect,
    });
}
