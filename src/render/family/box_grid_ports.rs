use std::collections::BTreeMap;

use crate::model::{FamilyDocument, FamilyNode, FamilyNodeKind};

use super::box_grid::PackageLayout;

#[derive(Clone, Copy)]
enum BoundaryPortSide {
    Left,
    Right,
    Bottom,
}

#[derive(Clone, Copy)]
struct PackageFrame {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    tab: i32,
}

pub(super) fn apply_boundary_port_positions(
    doc: &FamilyDocument,
    positions: &mut BTreeMap<String, (i32, i32, i32, i32)>,
    pkg_layouts: &[PackageLayout],
    pkg_frame_widths: &[i32],
    pkg_frame_heights: &[i32],
    pkg_tab: i32,
) {
    let mut node_by_key: BTreeMap<&str, &FamilyNode> = BTreeMap::new();
    for node in &doc.nodes {
        node_by_key.entry(node.name.as_str()).or_insert(node);
        if let Some(alias) = &node.alias {
            node_by_key.entry(alias.as_str()).or_insert(node);
        }
        if let Some(unscoped) = node.name.rsplit("::").next() {
            node_by_key.entry(unscoped).or_insert(node);
        }
    }

    for (idx, pkg) in pkg_layouts.iter().enumerate() {
        let mut left = Vec::new();
        let mut right = Vec::new();
        let mut bottom = Vec::new();
        for member_id in &pkg.node_ids {
            let Some(node) = node_by_key
                .get(member_id.as_str())
                .or_else(|| {
                    member_id
                        .rsplit("::")
                        .next()
                        .and_then(|key| node_by_key.get(key))
                })
                .copied()
            else {
                continue;
            };
            if node.kind != FamilyNodeKind::Port {
                continue;
            }
            match boundary_port_side(node) {
                BoundaryPortSide::Left => left.push(node),
                BoundaryPortSide::Right => right.push(node),
                BoundaryPortSide::Bottom => bottom.push(node),
            }
        }

        let frame = PackageFrame {
            x: pkg.abs_x,
            y: pkg.abs_y,
            w: pkg_frame_widths[idx],
            h: pkg_frame_heights[idx],
            tab: pkg_tab,
        };
        place_boundary_port_side(positions, &left, BoundaryPortSide::Left, frame);
        place_boundary_port_side(positions, &right, BoundaryPortSide::Right, frame);
        place_boundary_port_side(positions, &bottom, BoundaryPortSide::Bottom, frame);
    }
}

fn boundary_port_side(node: &FamilyNode) -> BoundaryPortSide {
    if node
        .members
        .iter()
        .any(|member| member.text == "<<portin>>")
    {
        BoundaryPortSide::Left
    } else if node
        .members
        .iter()
        .any(|member| member.text == "<<portout>>")
    {
        BoundaryPortSide::Right
    } else {
        BoundaryPortSide::Bottom
    }
}

fn place_boundary_port_side(
    positions: &mut BTreeMap<String, (i32, i32, i32, i32)>,
    ports: &[&FamilyNode],
    side: BoundaryPortSide,
    frame: PackageFrame,
) {
    const PORT_SIZE: i32 = 24;
    if ports.is_empty() {
        return;
    }
    for (slot, node) in ports.iter().enumerate() {
        let slot = slot as i32;
        let count = ports.len() as i32;
        let (cx, cy) = match side {
            BoundaryPortSide::Left => {
                let usable_h = (frame.h - frame.tab - 40).max(1);
                let y = frame.y + frame.tab + 20 + usable_h * (slot + 1) / (count + 1);
                (frame.x, y)
            }
            BoundaryPortSide::Right => {
                let usable_h = (frame.h - frame.tab - 40).max(1);
                let y = frame.y + frame.tab + 20 + usable_h * (slot + 1) / (count + 1);
                (frame.x + frame.w, y)
            }
            BoundaryPortSide::Bottom => {
                let usable_w = (frame.w - 48).max(1);
                let x = frame.x + 24 + usable_w * (slot + 1) / (count + 1);
                (x, frame.y + frame.h)
            }
        };
        let pos = (cx - PORT_SIZE / 2, cy - PORT_SIZE / 2, PORT_SIZE, PORT_SIZE);
        positions.insert(node.name.clone(), pos);
        if let Some(alias) = &node.alias {
            positions.insert(alias.clone(), pos);
        }
        if let Some(unscoped) = node.name.rsplit("::").next() {
            positions.insert(unscoped.to_string(), pos);
        }
    }
}
