use super::{LayoutOptions, NodeSize};
use crate::render::layout_constants::{
    COMPONENT_BOX_WIDTH, GROUP_COLLISION_MAX_PASSES, GROUP_COLLISION_MIN_GAP, PKG_TAB_HEIGHT,
};
use std::collections::BTreeMap;

pub(super) fn assign_coordinates(
    nodes: &[NodeSize],
    ranks: &BTreeMap<String, usize>,
    rank_order: &BTreeMap<usize, Vec<String>>,
    options: &LayoutOptions,
) -> (BTreeMap<String, (f64, f64)>, f64, f64) {
    let node_by_id: BTreeMap<&str, &NodeSize> = nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    let max_rank = ranks.values().copied().max().unwrap_or(0);

    let rank_width = |ids: &[String]| -> f64 {
        let n_nodes = ids.len() as f64;
        let total_node_w: f64 = ids
            .iter()
            .filter_map(|id| node_by_id.get(id.as_str()))
            .map(|n| n.width)
            .sum();
        total_node_w + (n_nodes - 1.0).max(0.0) * options.node_separation
    };

    let has_groups = nodes.iter().any(|n| n.parent.is_some());
    let wrap_rank_width = 2200.0;
    let min_wrapped_rank_nodes = 8;
    let mut visual_rows: Vec<Vec<String>> = Vec::new();
    for r in 0..=max_rank {
        let Some(ids) = rank_order.get(&r) else {
            continue;
        };
        if has_groups && ids.len() >= min_wrapped_rank_nodes && rank_width(ids) > wrap_rank_width {
            let mut row: Vec<String> = Vec::new();
            for id in ids {
                let mut candidate = row.clone();
                candidate.push(id.clone());
                if !row.is_empty() && rank_width(&candidate) > wrap_rank_width {
                    visual_rows.push(std::mem::take(&mut row));
                }
                row.push(id.clone());
            }
            if !row.is_empty() {
                visual_rows.push(row);
            }
        } else {
            visual_rows.push(ids.clone());
        }
    }

    // Compute per-visual-row y positions.  Very wide grouped ranks are split
    // into wrapped rows, giving the router real vertical channels instead of
    // one pathological all-in-one-row canvas.
    let mut row_y: Vec<f64> = Vec::with_capacity(visual_rows.len());
    {
        let mut y = options.canvas_margin;
        for ids in &visual_rows {
            row_y.push(y);
            let max_h = ids
                .iter()
                .filter_map(|id| node_by_id.get(id.as_str()))
                .map(|n| n.height)
                .fold(0.0_f64, f64::max);
            y += max_h + options.rank_separation;
        }
    }

    let row_widths: Vec<f64> = visual_rows.iter().map(|ids| rank_width(ids)).collect();
    let max_rank_width = row_widths.iter().cloned().fold(0.0_f64, f64::max);
    // Note: canvas_content_width is recomputed at the bottom after the
    // post-layout group-collision shift, which may extend the canvas right.

    let mut positions: BTreeMap<String, (f64, f64)> = BTreeMap::new();

    for (row_idx, ids) in visual_rows.iter().enumerate() {
        let rw = row_widths[row_idx];
        // Centre this visual row horizontally
        let rank_start_x = options.canvas_margin + (max_rank_width - rw) / 2.0;
        let ry = row_y[row_idx];

        let mut x = rank_start_x;
        for id in ids {
            let w = node_by_id
                .get(id.as_str())
                .map(|n| n.width)
                .unwrap_or(COMPONENT_BOX_WIDTH as f64);
            positions.insert(id.clone(), (x, ry));
            x += w + options.node_separation;
        }
    }

    // ── Post-layout: resolve group-bounds collisions ──────────────────────────
    // When a group has members spanning multiple ranks (e.g. Shared Services
    // with LangSvc at rank N and RenderSupport at rank N+1), its bbox can
    // extend into another group's column at the overlapping rank.  Detect
    // such overlaps and shift the right-side group's members rightward to
    // clear the collision.
    {
        let pad = options.group_padding;
        let label_reserve = PKG_TAB_HEIGHT as f64;
        // Compute current group bboxes from positions.
        let compute_bounds =
            |positions: &BTreeMap<String, (f64, f64)>| -> BTreeMap<String, (f64, f64, f64, f64)> {
                let mut bb: BTreeMap<String, (f64, f64, f64, f64)> = BTreeMap::new();
                let mut by_group: BTreeMap<String, Vec<(f64, f64, f64, f64)>> = BTreeMap::new();
                for n in nodes {
                    if let Some(parent) = &n.parent {
                        if let Some(&(x, y)) = positions.get(n.id.as_str()) {
                            by_group
                                .entry(parent.clone())
                                .or_default()
                                .push((x, y, n.width, n.height));
                        }
                    }
                }
                for (g, members) in by_group {
                    let mut min_x = f64::MAX;
                    let mut min_y = f64::MAX;
                    let mut max_x = f64::MIN;
                    let mut max_y = f64::MIN;
                    for (x, y, w, h) in members {
                        min_x = min_x.min(x);
                        min_y = min_y.min(y);
                        max_x = max_x.max(x + w);
                        max_y = max_y.max(y + h);
                    }
                    if min_x != f64::MAX {
                        bb.insert(
                            g,
                            (
                                min_x - pad,
                                min_y - pad - label_reserve,
                                (max_x - min_x) + pad * 2.0,
                                (max_y - min_y) + pad * 2.0 + label_reserve,
                            ),
                        );
                    }
                }
                bb
            };

        let min_gap = GROUP_COLLISION_MIN_GAP;
        // Iterate up to GROUP_COLLISION_MAX_PASSES; in practice 1–2 are enough.
        for _ in 0..GROUP_COLLISION_MAX_PASSES {
            let bb = compute_bounds(&positions);
            let grouped_span_width = {
                let min_x = bb.values().map(|(x, _, _, _)| *x).fold(f64::MAX, f64::min);
                let max_x = bb
                    .values()
                    .map(|(x, _, w, _)| x + w)
                    .fold(f64::MIN, f64::max);
                if min_x == f64::MAX {
                    0.0
                } else {
                    max_x - min_x
                }
            };
            let prefer_vertical_shift = grouped_span_width > wrap_rank_width;
            // Find first overlapping pair (sorted by group id for determinism).
            let mut overlap: Option<(String, f64, bool)> = None;
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
                        let staggered_tops = (ay - by).abs() > min_gap / 2.0;
                        let (shift_target, shift_amount, shift_down) = if prefer_vertical_shift
                            || (options.stack_staggered_group_collisions && staggered_tops)
                        {
                            // Wide grouped diagrams are more readable when
                            // colliding frames stack into rows instead of
                            // stretching into one extremely long row.
                            //
                            // Also stack lower-starting package groups
                            // vertically. Otherwise a small downstream
                            // package can sit inside a taller sibling group's
                            // inter-rank channel, collapsing routed
                            // lollipop/interface lanes onto that package's
                            // header-avoidance line.
                            if ay <= by {
                                (gb.to_string(), a_bottom - by + min_gap, true)
                            } else {
                                (ga.to_string(), b_bottom - ay + min_gap, true)
                            }
                        } else if ax <= bx {
                            // Shift the right-side group rightward.
                            (gb.to_string(), a_right - bx + min_gap, false)
                        } else {
                            (ga.to_string(), b_right - ax + min_gap, false)
                        };
                        overlap = Some((shift_target, shift_amount, shift_down));
                        break 'outer;
                    }
                }
            }
            match overlap {
                None => break,
                Some((g, delta, shift_down)) => {
                    // Shift all members of group `g` along the selected axis.
                    for n in nodes {
                        if n.parent.as_deref() == Some(g.as_str()) {
                            if let Some(p) = positions.get_mut(n.id.as_str()) {
                                if shift_down {
                                    p.1 += delta;
                                } else {
                                    p.0 += delta;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Canvas size — recompute to include any post-shift rightward extension.
    // Use canvas_right_margin when set; this decouples the large top/left
    // canvas_margin (which absorbs titles and package-label tabs) from the
    // right-side gutter, preventing inflated canvases on grouped diagrams.
    let canvas_content_width = {
        let max_right = positions
            .iter()
            .map(|(id, &(x, _))| {
                let w = node_by_id
                    .get(id.as_str())
                    .map(|n| n.width)
                    .unwrap_or(COMPONENT_BOX_WIDTH as f64);
                x + w
            })
            .fold(0.0_f64, f64::max);
        let right_margin = options.canvas_right_margin.unwrap_or(options.canvas_margin);
        max_right + right_margin
    };
    let canvas_height = {
        let bottom = visual_rows
            .last()
            .zip(row_y.last())
            .map(|(ids, y)| {
                y + ids
                    .iter()
                    .filter_map(|id| node_by_id.get(id.as_str()))
                    .map(|n| n.height)
                    .fold(0.0_f64, f64::max)
            })
            .unwrap_or(options.canvas_margin);
        bottom + options.canvas_margin
    };

    (positions, canvas_content_width, canvas_height)
}

// ─────────────────────────────────────────────────────────────────────────────
