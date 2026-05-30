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
    /// Group/package bounding boxes included in obstacle detection (#1325).
    /// Every group frame is an opaque obstacle that a straight vertical route
    /// must not pierce — even when the route's endpoints are leaf nodes that
    /// live outside the group.
    pub(super) group_bounds: &'a BTreeMap<String, (f64, f64, f64, f64)>,
}

/// Returns `true` when a straight vertical route at `check.x` from `check.y1`
/// to `check.y2` would pierce any visible bbox — both leaf nodes AND group/
/// package frames at every nesting depth (#1325).
pub(super) fn vertical_route_crosses_node(check: VerticalRouteCheck<'_>) -> bool {
    let route = Segment::new(Point::new(check.x, check.y1), Point::new(check.x, check.y2));
    // Check leaf-node obstacles.
    let crosses_leaf = check.nodes.iter().any(|node| {
        node.id != check.source_id
            && node.id != check.target_id
            && node_rect(node, check.positions)
                .is_some_and(|rect| segment_crosses_rect(route, rect))
    });
    if crosses_leaf {
        return true;
    }
    // Check group/package frame obstacles: include every group frame whose
    // bounding box the vertical segment would cross.  Frames that fully
    // contain both endpoints are skipped — the route is *inside* that package,
    // which is not a pierce.
    check
        .group_bounds
        .values()
        .any(|&(gx, gy, gw, gh)| segment_crosses_rect(route, Rect::new(gx, gy, gw, gh)))
}

pub(super) fn detour_x_for_vertical_route(check: VerticalRouteCheck<'_>, clearance: f64) -> f64 {
    let route = Segment::new(Point::new(check.x, check.y1), Point::new(check.x, check.y2));
    // Collect right edges of all pierced leaf-node obstacles.
    let leaf_right = check
        .nodes
        .iter()
        .filter(|node| node.id != check.source_id && node.id != check.target_id)
        .filter_map(|node| node_rect(node, check.positions))
        .filter(|rect| segment_crosses_rect(route, *rect))
        .map(|rect| rect.max_x() + clearance);
    // Collect right edges of all pierced group/package frame obstacles.
    let group_right = check
        .group_bounds
        .values()
        .map(|&(gx, gy, gw, gh)| Rect::new(gx, gy, gw, gh))
        .filter(|rect| segment_crosses_rect(route, *rect))
        .map(|rect| rect.max_x() + clearance);
    leaf_right
        .chain(group_right)
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
