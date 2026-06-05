use crate::model::FamilyNodeKind;

use super::class_members::{
    count_header_stereotype_members, parse_map_row, parse_member_modifiers, parse_visibility_member,
};
use super::class_relation_labels::resolve_relation_endpoint_key;
use super::class_types::{ClassEndpointAnchor, ClassNodeBox, ClassPortSide};

pub(super) fn class_port_side_from_box_anchor(
    x: i32,
    y: i32,
    node_box: &ClassNodeBox,
) -> ClassPortSide {
    let distances = [
        (ClassPortSide::Left, (x - node_box.x).abs()),
        (ClassPortSide::Right, (x - (node_box.x + node_box.w)).abs()),
        (ClassPortSide::Top, (y - node_box.y).abs()),
        (ClassPortSide::Bottom, (y - (node_box.y + node_box.h)).abs()),
    ];
    distances
        .into_iter()
        .min_by_key(|(_, distance)| *distance)
        .map(|(side, _)| side)
        .unwrap_or(ClassPortSide::Bottom)
}

fn class_port_normal(side: ClassPortSide) -> (i32, i32) {
    match side {
        ClassPortSide::Left => (-1, 0),
        ClassPortSide::Right => (1, 0),
        ClassPortSide::Top => (0, -1),
        ClassPortSide::Bottom => (0, 1),
    }
}

pub(super) fn class_box_anchor_toward_point(
    node_box: &ClassNodeBox,
    point: (i32, i32),
) -> ClassEndpointAnchor {
    let cx = node_box.x + node_box.w / 2;
    let cy = node_box.y + node_box.h / 2;
    let (px, py) = point;
    let (x, y, side) = if py < node_box.y {
        (cx, node_box.y, ClassPortSide::Top)
    } else if py > node_box.y + node_box.h {
        (cx, node_box.y + node_box.h, ClassPortSide::Bottom)
    } else if px < cx {
        (node_box.x, cy, ClassPortSide::Left)
    } else {
        (node_box.x + node_box.w, cy, ClassPortSide::Right)
    };
    ClassEndpointAnchor {
        x,
        y,
        side,
        is_row_port: false,
    }
}

fn class_row_port_stub(
    anchor: ClassEndpointAnchor,
    original_adjacent: Option<(i32, i32)>,
) -> (i32, i32) {
    const ROW_PORT_STUB: i32 = 40;
    if anchor.is_row_port {
        let (nx, ny) = class_port_normal(anchor.side);
        return (anchor.x + nx * ROW_PORT_STUB, anchor.y + ny * ROW_PORT_STUB);
    }
    if let Some((ax, ay)) = original_adjacent {
        return match anchor.side {
            ClassPortSide::Left | ClassPortSide::Right => (ax, anchor.y),
            ClassPortSide::Top | ClassPortSide::Bottom => (anchor.x, ay),
        };
    }
    let (nx, ny) = class_port_normal(anchor.side);
    (anchor.x + nx * ROW_PORT_STUB, anchor.y + ny * ROW_PORT_STUB)
}

fn class_dedup_consecutive_points(points: Vec<(i32, i32)>) -> Vec<(i32, i32)> {
    let mut deduped = Vec::with_capacity(points.len());
    for point in points {
        if deduped.last().copied() != Some(point) {
            deduped.push(point);
        }
    }
    deduped
}

pub(super) fn class_route_with_row_ports(
    start: ClassEndpointAnchor,
    end: ClassEndpointAnchor,
    original_points: Option<&[(i32, i32)]>,
) -> Option<Vec<(i32, i32)>> {
    if !start.is_row_port && !end.is_row_port {
        return None;
    }
    let start_adjacent = original_points.and_then(|points| points.get(1).copied());
    let end_adjacent = original_points.and_then(|points| {
        points
            .len()
            .checked_sub(2)
            .and_then(|idx| points.get(idx).copied())
    });
    let start_stub = class_row_port_stub(start, start_adjacent);
    let end_stub = class_row_port_stub(end, end_adjacent);

    let mut points = vec![start.point(), start_stub];
    if start_stub.0 != end_stub.0 && start_stub.1 != end_stub.1 {
        let bend = match start.side {
            ClassPortSide::Left | ClassPortSide::Right => (start_stub.0, end_stub.1),
            ClassPortSide::Top | ClassPortSide::Bottom => (end_stub.0, start_stub.1),
        };
        points.push(bend);
    }
    points.push(end_stub);
    points.push(end.point());
    Some(class_dedup_consecutive_points(points))
}

/// Nudge a label's y-coordinate until it no longer visually overlaps any node box.
///
/// The label baseline is at `adjusted_y`; the visible text body extends upward
/// ~14px (one font em) from that baseline.  Two overlap cases are resolved:
///
/// 1. **Label baseline inside the box** (`adjusted_y > bbox.y + 2` AND
///    `adjusted_y <= bbox.y + bbox.h`): the entire text block is inside or
///    straddles the box — push the label UP to `bbox.y - 18`.
/// 2. **Label text top clips into box bottom** (baseline `adjusted_y` is below
///    `bbox.y + bbox.h` but within one text-height): push the label DOWN to
///    `bbox.y + bbox.h + 18` so the text clears the bottom edge (#1551).
///
/// The tight `adjusted_y > bbox.y + 2` guard on the upper bound prevents
/// spurious triggers when the label is merely near the box top and the 14px
/// look-ahead from the bottom of an adjacent box.  This avoids oscillation when
/// a label sits in the narrow gap between two vertically-adjacent class boxes.
///
/// Used by `render_class_svg` for both the pre-pass and the inline placement.
pub(super) fn class_nudge_label_y(
    lx: i32,
    ly: i32,
    label_half_w: i32,
    node_boxes: &std::collections::BTreeMap<String, ClassNodeBox>,
) -> (i32, i32) {
    // Height of the text block above the baseline (monospace 11px font).
    const LABEL_TEXT_H: i32 = 14;
    let mut adjusted_y = ly;
    for _ in 0..8 {
        let overlap = node_boxes.values().find(|bbox| {
            let box_bottom = bbox.y + bbox.h;
            lx + label_half_w >= bbox.x - 8
                && lx - label_half_w <= bbox.x + bbox.w + 8
                // Baseline must be clearly below box top (>2px) to trigger —
                // prevents false positives on labels approaching from above.
                && adjusted_y > bbox.y + 2
                // Baseline within text-height below box bottom catches both
                // internal labels (baseline <= box_bottom) and labels whose
                // text top clips the box bottom from below (#1551).
                && adjusted_y < box_bottom + LABEL_TEXT_H
        });
        match overlap {
            Some(bbox) => {
                let box_bottom = bbox.y + bbox.h;
                if adjusted_y > box_bottom {
                    // Baseline below box bottom — text top clips in: push DOWN.
                    adjusted_y = box_bottom + 18;
                } else {
                    // Baseline inside the box — push UP.
                    adjusted_y = bbox.y - 18;
                }
            }
            None => break,
        }
    }
    (lx, adjusted_y)
}

/// Nudge a label's x-coordinate rightward **only when it would actually collide**
/// with a node box.
///
/// This is only appropriate for labels on **vertical** edge segments whose x
/// coordinate coincides with the horizontal centre of the connected node boxes.
/// It should NOT be called from the fan-out pre-pass (`class_build_label_overrides`)
/// because those fans already spread labels apart in x; calling it there causes
/// double-nudging and breaks adjacent-label clearance invariants.
///
/// A collision is defined as the label bounding box (centred at `lx`, top at
/// `ly`, height 14px) overlapping a node bounding box in **both** x and y.
/// On sparse diagrams the label sits in the gap between two nodes (at the
/// arclength midpoint of a vertical edge) and therefore does not overlap any
/// node; the x-push must not fire in that case.
///
/// After nudging, the label's left edge clears the rightmost overlapping box's
/// right edge by 8 px.
pub(super) fn class_nudge_label_x(
    lx: i32,
    ly: i32,
    label_half_w: i32,
    node_boxes: &std::collections::BTreeMap<String, ClassNodeBox>,
) -> i32 {
    // The label bounding box: x in [lx - label_half_w, lx + label_half_w],
    //                         y in [ly - 14, ly + 2]  (14px text, 2px slack).
    const LABEL_H: i32 = 14;
    const LABEL_SLACK: i32 = 2;
    // Minimum depth of vertical overlap required before the x-push fires.
    // Without this guard, a label sitting at the arclength midpoint of a
    // tightly-packed vertical edge (CLASS_ROW_GAP ≈ 30px) may clip the bottom
    // edge of the source node by just a few pixels and trigger a large push
    // sideways — a false-positive collision.  6px ensures the label centre is
    // meaningfully inside the node rather than just grazing the boundary.
    const MIN_OVERLAP_Y: i32 = 6;
    let label_top = ly - LABEL_H;
    let label_bot = ly + LABEL_SLACK;
    let max_box_right = node_boxes
        .values()
        .filter(|bbox| {
            let box_bot = bbox.y + bbox.h;
            let box_top = bbox.y;
            let overlap_top = label_top.max(box_top);
            let overlap_bot = label_bot.min(box_bot);
            let overlap_depth = overlap_bot - overlap_top;
            // X: label centre must be inside the box horizontally (same check as before)
            lx > bbox.x
                && lx < bbox.x + bbox.w
                // Y: label must actually overlap the box vertically by at least
                // MIN_OVERLAP_Y pixels to warrant a push (prevents false-positive
                // pushes when a label just grazes a node edge at the endpoint of a
                // tight vertical edge with CLASS_ROW_GAP ~30px).
                && overlap_depth >= MIN_OVERLAP_Y
        })
        .map(|bbox| bbox.x + bbox.w)
        .max();
    match max_box_right {
        Some(right) => right + 8 + label_half_w,
        None => lx,
    }
}

// Run the hierarchical layout engine and populate `node_boxes` for `render_class_svg`.
// Builds `GlNodeSize` / `GlEdgeSpec` inputs for `layout_hierarchical`, runs the
// layout, then converts the resulting `(f64, f64)` positions into `ClassNodeBox`.
fn map_row_anchor_y(
    node: &crate::model::FamilyNode,
    key: &str,
    y: i32,
    header_h: i32,
) -> Option<i32> {
    let key = key.trim();
    if key.is_empty() {
        return None;
    }
    let mut row_idx = 0;
    for member in &node.members {
        let Some(row) = parse_map_row(&member.text) else {
            continue;
        };
        if row.key == key {
            return Some(y + header_h + 12 + row_idx * 18);
        }
        row_idx += 1;
    }
    None
}

fn qualified_member_anchor_y(
    node: &crate::model::FamilyNode,
    member_key: &str,
    y: i32,
    header_h: i32,
) -> Option<i32> {
    let member_key = member_key.trim();
    if member_key.is_empty() {
        return None;
    }
    if node.kind == FamilyNodeKind::Map {
        return map_row_anchor_y(node, member_key, y, header_h);
    }

    let header_skip = count_header_stereotype_members(&node.members);
    let mut row_idx = 0;
    for member in node.members.iter().skip(header_skip) {
        let text = member.text.trim();
        if text == "--" || text == ".." {
            continue;
        }
        if text.is_empty() {
            row_idx += 1;
            continue;
        }
        if member_anchor_matches(text, member_key) {
            return Some(y + header_h + 16 + row_idx * 16);
        }
        row_idx += 1;
    }
    None
}

fn member_anchor_matches(member_text: &str, member_key: &str) -> bool {
    let (_visibility, _color, after_visibility) = parse_visibility_member(member_text.trim());
    let (_style, clean_text) = parse_member_modifiers(after_visibility.trim());
    let clean_text = clean_text.trim();
    if clean_text == member_key {
        return true;
    }
    // Strip parameter list and type annotation to get the bare identifier.
    // e.g. "int value" → "value", "void increment()" → "increment"
    let name = clean_text
        .split(['(', ':', '='])
        .next()
        .unwrap_or(clean_text)
        .trim();
    if name == member_key {
        return true;
    }
    // Handle "Type name" pattern: the identifier is the last whitespace-delimited
    // token (after stripping type prefix).  e.g. "int value" → last token "value".
    // Closes #1403: `note right of Counter::value` where member text is "int value".
    let last_word = name.split_whitespace().last().unwrap_or(name);
    last_word == member_key
}

pub(super) fn qualified_row_anchor(
    endpoint: &str,
    nodes: &[crate::model::FamilyNode],
    node_boxes: &std::collections::BTreeMap<String, ClassNodeBox>,
    other: &ClassNodeBox,
) -> Option<ClassEndpointAnchor> {
    let (owner, row_key) = endpoint.rsplit_once("::")?;
    let owner_key = resolve_relation_endpoint_key(owner, node_boxes);
    let owner_box = node_boxes.get(&owner_key)?;
    let owner_node = nodes.iter().find(|node| {
        node.name == owner
            || node.alias.as_deref() == Some(owner)
            || node.name == owner_key
            || node.alias.as_deref() == Some(owner_key.as_str())
    })?;
    let y = qualified_member_anchor_y(owner_node, row_key, owner_box.y, owner_box.header_h)?;
    let owner_cx = owner_box.x + owner_box.w / 2;
    let other_cx = other.x + other.w / 2;
    let (x, side) = if other_cx < owner_cx {
        (owner_box.x, ClassPortSide::Left)
    } else {
        (owner_box.x + owner_box.w, ClassPortSide::Right)
    };
    Some(ClassEndpointAnchor {
        x,
        y,
        side,
        is_row_port: true,
    })
}
