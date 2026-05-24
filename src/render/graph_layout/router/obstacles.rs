use crate::render::graph_layout::NodeSize;
use crate::render_core::{Point, Rect, Segment};
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
pub(super) struct VerticalRouteCheck<'a> {
    pub(super) x: f64,
    pub(super) y1: f64,
    pub(super) y2: f64,
    pub(super) source_id: &'a str,
    pub(super) target_id: &'a str,
    pub(super) nodes: &'a [NodeSize],
    pub(super) positions: &'a BTreeMap<String, (f64, f64)>,
}

pub(super) fn vertical_route_crosses_node(check: VerticalRouteCheck<'_>) -> bool {
    let route = Segment::new(Point::new(check.x, check.y1), Point::new(check.x, check.y2));
    check.nodes.iter().any(|node| {
        node.id != check.source_id
            && node.id != check.target_id
            && node_rect(node, check.positions)
                .is_some_and(|rect| segment_crosses_rect(route, rect))
    })
}

pub(super) fn detour_x_for_vertical_route(check: VerticalRouteCheck<'_>, clearance: f64) -> f64 {
    let route = Segment::new(Point::new(check.x, check.y1), Point::new(check.x, check.y2));
    check
        .nodes
        .iter()
        .filter(|node| node.id != check.source_id && node.id != check.target_id)
        .filter_map(|node| node_rect(node, check.positions))
        .filter(|rect| segment_crosses_rect(route, *rect))
        .map(|rect| rect.max_x() + clearance)
        .fold(check.x + clearance, f64::max)
}

fn node_rect(node: &NodeSize, positions: &BTreeMap<String, (f64, f64)>) -> Option<Rect> {
    let &(x, y) = positions.get(&node.id)?;
    Some(Rect::new(x, y, node.width, node.height))
}

fn segment_crosses_rect(segment: Segment, rect: Rect) -> bool {
    if !segment.bounds().intersects(rect) {
        return false;
    }
    let min_x = rect.min_x();
    let max_x = rect.max_x();
    let min_y = rect.min_y();
    let max_y = rect.max_y();
    if segment.is_vertical() {
        return segment.start.x > min_x
            && segment.start.x < max_x
            && ranges_overlap(segment.start.y, segment.end.y, min_y, max_y);
    }
    if segment.is_horizontal() {
        return segment.start.y > min_y
            && segment.start.y < max_y
            && ranges_overlap(segment.start.x, segment.end.x, min_x, max_x);
    }
    rect.contains_point(segment.start) || rect.contains_point(segment.end)
}

fn ranges_overlap(a: f64, b: f64, c: f64, d: f64) -> bool {
    let a_min = a.min(b);
    let a_max = a.max(b);
    let c_min = c.min(d);
    let c_max = c.max(d);
    a_min < c_max && c_min < a_max
}
