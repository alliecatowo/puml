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
///
/// Group frames that ENCLOSE the target position are never counted as
/// obstacles: the route must enter those frames to reach its destination, so
/// passing through them is intentional, not a pierce (#1472).
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
    // Check group/package frame obstacles.  Frames whose bounding box
    // contains the TARGET position are skipped: the route must enter those
    // frames, so they are not obstacles (#1325, #1472).
    let tgt_pos = check.positions.get(check.target_id).copied();
    check.group_bounds.values().any(|&(gx, gy, gw, gh)| {
        let frame = Rect::new(gx, gy, gw, gh);
        if !segment_crosses_rect(route, frame) {
            return false;
        }
        // Skip frames that enclose the target — the route legitimately enters
        // them to reach its destination.
        let encloses_tgt = tgt_pos.is_some_and(|(tx, ty)| frame.contains_point(Point::new(tx, ty)));
        !encloses_tgt
    })
}

/// Compute the x-coordinate for a detour vertical route that clears all
/// obstacles in the y-span [check.y1, check.y2].
///
/// Obstacles include:
/// - Leaf nodes (excluding source and target) whose y-ranges overlap the span.
/// - Group frames (excluding frames that enclose the target) whose y-ranges
///   overlap the span.
///
/// Two candidate detour lanes are computed:
///   • Right lane: iteratively push past obstacle right edges until clear.
///   • Left lane: iteratively push past obstacle left edges until clear.
/// The lane closest to the original check.x is returned.  Ties favour the
/// right lane.  This prevents detours from escaping the diagram boundary when
/// the right lane would push outside the cluster (#1472).
pub(super) fn detour_x_for_vertical_route(check: VerticalRouteCheck<'_>, clearance: f64) -> f64 {
    let tgt_pos = check.positions.get(check.target_id).copied();

    // Build a list of all obstacle rects in the y-span, excluding:
    //   • the source and target leaf nodes
    //   • group frames that enclose the target (entry frames, not obstacles)
    let obstacle_rects: Vec<Rect> = {
        let mut v: Vec<Rect> = Vec::new();
        for node in check.nodes {
            if node.id == check.source_id || node.id == check.target_id {
                continue;
            }
            if let Some(rect) = node_rect(node, check.positions) {
                if ranges_overlap(check.y1, check.y2, rect.min_y(), rect.max_y()) {
                    v.push(rect);
                }
            }
        }
        for &(gx, gy, gw, gh) in check.group_bounds.values() {
            let frame = Rect::new(gx, gy, gw, gh);
            let encloses_tgt =
                tgt_pos.is_some_and(|(tx, ty)| frame.contains_point(Point::new(tx, ty)));
            if encloses_tgt {
                continue;
            }
            if ranges_overlap(check.y1, check.y2, frame.min_y(), frame.max_y()) {
                v.push(frame);
            }
        }
        v
    };

    /// Push `x` rightward past every obstacle that contains it, until stable.
    fn push_right(x: f64, obstacles: &[Rect], clearance: f64) -> f64 {
        let mut cur = x;
        let max_iters = obstacles.len() + 2;
        for _ in 0..max_iters {
            let next = obstacles
                .iter()
                .filter(|r| cur > r.min_x() && cur < r.max_x())
                .map(|r| r.max_x() + clearance)
                .fold(cur, f64::max);
            if next <= cur + f64::EPSILON {
                break;
            }
            cur = next;
        }
        cur
    }

    /// Push `x` leftward past every obstacle that contains it, until stable.
    fn push_left(x: f64, obstacles: &[Rect], clearance: f64) -> f64 {
        let mut cur = x;
        let max_iters = obstacles.len() + 2;
        for _ in 0..max_iters {
            let next = obstacles
                .iter()
                .filter(|r| cur > r.min_x() && cur < r.max_x())
                .map(|r| r.min_x() - clearance)
                .fold(cur, f64::min);
            if next >= cur - f64::EPSILON {
                break;
            }
            cur = next;
        }
        cur
    }

    let right_x = push_right(check.x + clearance, &obstacle_rects, clearance);
    let left_x = push_left(check.x - clearance, &obstacle_rects, clearance);

    // Pick the lane that deviates least from check.x.
    let right_dist = (right_x - check.x).abs();
    let left_dist = (left_x - check.x).abs();
    if right_dist <= left_dist {
        right_x
    } else {
        left_x
    }
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
