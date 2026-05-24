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

/// Nudge a label's y-coordinate upward until it no longer overlaps any node box.
/// Used by `render_class_svg` for both the pre-pass and the inline placement.
pub(super) fn class_nudge_label_y(
    lx: i32,
    ly: i32,
    label_half_w: i32,
    node_boxes: &std::collections::BTreeMap<String, ClassNodeBox>,
) -> (i32, i32) {
    let mut adjusted_y = ly;
    for _ in 0..8 {
        let overlap = node_boxes.values().find(|bbox| {
            lx + label_half_w >= bbox.x - 8
                && lx - label_half_w <= bbox.x + bbox.w + 8
                && adjusted_y >= bbox.y - 14
                && adjusted_y <= bbox.y + bbox.h + 6
        });
        match overlap {
            Some(bbox) => adjusted_y = bbox.y - 18,
            None => break,
        }
    }
    (lx, adjusted_y)
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
    let name = clean_text
        .split(['(', ':', '='])
        .next()
        .unwrap_or(clean_text)
        .trim();
    name == member_key
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
