use std::collections::BTreeMap;

use super::{LayoutOptions, NodeSize};

pub(super) fn assign_coordinates(
    nodes: &[NodeSize],
    ranks: &BTreeMap<String, usize>,
    rank_order: &BTreeMap<usize, Vec<String>>,
    options: &LayoutOptions,
) -> (BTreeMap<String, (f64, f64)>, f64, f64) {
    let node_by_id: BTreeMap<&str, &NodeSize> = nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let max_rank = ranks.values().copied().max().unwrap_or(0);
    let rank_y = compute_rank_y(max_rank, rank_order, &node_by_id, options);
    let rank_widths = compute_rank_widths(max_rank, rank_order, &node_by_id, options);
    let max_rank_width = rank_widths.iter().copied().fold(0.0_f64, f64::max);

    let mut positions: BTreeMap<String, (f64, f64)> = BTreeMap::new();

    for r in 0..=max_rank {
        let Some(ids) = rank_order.get(&r) else {
            continue;
        };
        let rank_start_x = options.canvas_margin + (max_rank_width - rank_widths[r]) / 2.0;
        let mut x = rank_start_x;
        for id in ids {
            let w = node_by_id
                .get(id.as_str())
                .map(|n| n.width)
                .unwrap_or(200.0);
            positions.insert(id.clone(), (x, rank_y[r]));
            x += w + options.node_separation;
        }
    }

    resolve_group_collisions(nodes, &mut positions, options);

    let canvas_width = canvas_content_width(&positions, &node_by_id, options);
    let canvas_height = canvas_content_height(max_rank, &rank_y, rank_order, &node_by_id, options);

    (positions, canvas_width, canvas_height)
}

fn compute_rank_y(
    max_rank: usize,
    rank_order: &BTreeMap<usize, Vec<String>>,
    node_by_id: &BTreeMap<&str, &NodeSize>,
    options: &LayoutOptions,
) -> Vec<f64> {
    let mut rank_y: Vec<f64> = vec![0.0; max_rank + 1];
    let mut y = options.canvas_margin;
    for (r, ry) in rank_y.iter_mut().enumerate().take(max_rank + 1) {
        *ry = y;
        let max_h = rank_order
            .get(&r)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| node_by_id.get(id.as_str()))
                    .map(|n| n.height)
                    .fold(0.0_f64, f64::max)
            })
            .unwrap_or(0.0);
        y += max_h + options.rank_separation;
    }
    rank_y
}

fn compute_rank_widths(
    max_rank: usize,
    rank_order: &BTreeMap<usize, Vec<String>>,
    node_by_id: &BTreeMap<&str, &NodeSize>,
    options: &LayoutOptions,
) -> Vec<f64> {
    (0..=max_rank)
        .map(|r| {
            rank_order
                .get(&r)
                .map(|ids| {
                    let n_nodes = ids.len() as f64;
                    let total_node_w: f64 = ids
                        .iter()
                        .filter_map(|id| node_by_id.get(id.as_str()))
                        .map(|n| n.width)
                        .sum();
                    total_node_w + (n_nodes - 1.0).max(0.0) * options.node_separation
                })
                .unwrap_or(0.0)
        })
        .collect()
}

fn resolve_group_collisions(
    nodes: &[NodeSize],
    positions: &mut BTreeMap<String, (f64, f64)>,
    options: &LayoutOptions,
) {
    // When a group has members spanning multiple ranks, its bbox can extend
    // into another group's column at the overlapping rank. Shift the right-side
    // group's members rightward to clear the collision.
    let min_gap = 40.0;
    for _ in 0..4 {
        let bb = compute_group_bounds_from_positions(nodes, positions, options);
        let mut overlap: Option<(String, f64)> = None;
        #[allow(clippy::type_complexity)] // simple (id, bbox) pairs; tuple is fine here
        let groups: Vec<(&String, &(f64, f64, f64, f64))> = bb.iter().collect();
        'outer: for (i, (ga, &(ax, ay, aw, ah))) in groups.iter().enumerate() {
            for (gb, &(bx, by, bw, bh)) in &groups[i + 1..] {
                let a_right = ax + aw;
                let a_bottom = ay + ah;
                let b_right = bx + bw;
                let b_bottom = by + bh;
                let x_overlap = a_right > bx && b_right > ax;
                let y_overlap = a_bottom > by && b_bottom > ay;
                if x_overlap && y_overlap {
                    let (shift_target, shift_amount) = if ax <= bx {
                        (gb.to_string(), a_right - bx + min_gap)
                    } else {
                        (ga.to_string(), b_right - ax + min_gap)
                    };
                    overlap = Some((shift_target, shift_amount));
                    break 'outer;
                }
            }
        }
        match overlap {
            None => break,
            Some((g, dx)) => {
                for n in nodes {
                    if n.parent.as_deref() == Some(g.as_str()) {
                        if let Some(p) = positions.get_mut(n.id.as_str()) {
                            p.0 += dx;
                        }
                    }
                }
            }
        }
    }
}

fn canvas_content_width(
    positions: &BTreeMap<String, (f64, f64)>,
    node_by_id: &BTreeMap<&str, &NodeSize>,
    options: &LayoutOptions,
) -> f64 {
    let max_right = positions
        .iter()
        .map(|(id, &(x, _))| {
            let w = node_by_id
                .get(id.as_str())
                .map(|n| n.width)
                .unwrap_or(200.0);
            x + w
        })
        .fold(0.0_f64, f64::max);
    let right_margin = options.canvas_right_margin.unwrap_or(options.canvas_margin);
    max_right + right_margin
}

fn canvas_content_height(
    max_rank: usize,
    rank_y: &[f64],
    rank_order: &BTreeMap<usize, Vec<String>>,
    node_by_id: &BTreeMap<&str, &NodeSize>,
    options: &LayoutOptions,
) -> f64 {
    let bottom = rank_y[max_rank]
        + rank_order
            .get(&max_rank)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| node_by_id.get(id.as_str()))
                    .map(|n| n.height)
                    .fold(0.0_f64, f64::max)
            })
            .unwrap_or(0.0);
    bottom + options.canvas_margin
}

pub(super) fn compute_group_bounds(
    nodes: &[NodeSize],
    positions: &BTreeMap<String, (f64, f64)>,
    options: &LayoutOptions,
) -> BTreeMap<String, (f64, f64, f64, f64)> {
    compute_group_bounds_from_positions(nodes, positions, options)
}

fn compute_group_bounds_from_positions(
    nodes: &[NodeSize],
    positions: &BTreeMap<String, (f64, f64)>,
    options: &LayoutOptions,
) -> BTreeMap<String, (f64, f64, f64, f64)> {
    let mut children_by_group: BTreeMap<String, Vec<(&str, f64, f64)>> = BTreeMap::new();
    for n in nodes {
        if let Some(parent) = &n.parent {
            if let Some(&(x, y)) = positions.get(n.id.as_str()) {
                children_by_group
                    .entry(parent.clone())
                    .or_default()
                    .push((n.id.as_str(), x, y));
            }
        }
    }

    let node_by_id: BTreeMap<&str, &NodeSize> = nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let pad = options.group_padding;
    let label_reserve = 40.0;

    let mut bounds: BTreeMap<String, (f64, f64, f64, f64)> = BTreeMap::new();
    for (group_id, children) in &children_by_group {
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        for &(id, cx, cy) in children {
            let (nw, nh) = node_by_id
                .get(id)
                .map(|n| (n.width, n.height))
                .unwrap_or((200.0, 80.0));
            min_x = min_x.min(cx);
            min_y = min_y.min(cy);
            max_x = max_x.max(cx + nw);
            max_y = max_y.max(cy + nh);
        }
        if min_x == f64::MAX {
            continue;
        }
        let gx = min_x - pad;
        let gy = min_y - pad - label_reserve;
        let gw = (max_x - min_x) + pad * 2.0;
        let gh = (max_y - min_y) + pad * 2.0 + label_reserve;
        bounds.insert(group_id.clone(), (gx, gy, gw, gh));
    }
    bounds
}
