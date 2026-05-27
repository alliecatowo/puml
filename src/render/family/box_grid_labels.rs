use std::collections::BTreeMap;

use crate::render::svg::escape_text;

const LABEL_FAN_H_GAP: i32 = 85;
const LABEL_CLUSTER_BAND: i32 = 18;
const LABEL_CLEARANCE_X: i32 = 10;
const LABEL_CLEARANCE_Y: i32 = 10;
const LABEL_TEXT_HALF_HEIGHT: i32 = 8;

/// Per-relation label gathered during component/deployment edge rendering,
/// de-collided before final SVG emission.
pub(super) struct BoxGridPendingLabel {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) text: String,
    pub(super) color: String,
    pub(super) from_name: String,
    pub(super) to_name: String,
    pub(super) edge_points: Vec<(i32, i32)>,
}

pub(super) fn render_box_grid_relation_labels(
    out: &mut String,
    positions: &BTreeMap<String, (i32, i32, i32, i32)>,
    pending_labels: Vec<BoxGridPendingLabel>,
) {
    let mut by_target: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, pl) in pending_labels.iter().enumerate() {
        by_target.entry(pl.to_name.clone()).or_default().push(i);
    }
    let n = pending_labels.len();
    let mut adjusted_labels: Vec<Option<(i32, i32, String, String)>> = vec![None; n];

    // Fan labels that target the same node horizontally above that node.
    for (to_name, indices) in &by_target {
        let count = indices.len() as i32;
        if count < 2 {
            continue;
        }
        let (anchor_cx, anchor_y) = match positions.get(to_name.as_str()) {
            Some(&(tx, ty, tw, _)) => (tx + tw / 2, ty - 28),
            None => {
                let mx = indices.iter().map(|&i| pending_labels[i].x).sum::<i32>() / count;
                let my = indices.iter().map(|&i| pending_labels[i].y).sum::<i32>() / count;
                (mx, my)
            }
        };
        let mut sorted_idx = indices.clone();
        sorted_idx.sort_by_key(|&i| pending_labels[i].x);
        for (slot, &raw_idx) in sorted_idx.iter().enumerate() {
            let offset = (slot as i32) * LABEL_FAN_H_GAP - (count - 1) * LABEL_FAN_H_GAP / 2;
            adjusted_labels[raw_idx] = Some((
                anchor_cx + offset,
                anchor_y,
                pending_labels[raw_idx].text.clone(),
                pending_labels[raw_idx].color.clone(),
            ));
        }
    }

    let labels_overlap = |left: &BoxGridPendingLabel, right: &BoxGridPendingLabel| {
        let left_half_w = (crate::render::text_metrics::monospace_width(&left.text, 7) + 2) / 2;
        let right_half_w = (crate::render::text_metrics::monospace_width(&right.text, 7) + 2) / 2;
        left.x + left_half_w + LABEL_CLEARANCE_X >= right.x - right_half_w - LABEL_CLEARANCE_X
            && right.x + right_half_w + LABEL_CLEARANCE_X
                >= left.x - left_half_w - LABEL_CLEARANCE_X
            && left.y + 4 + LABEL_CLEARANCE_Y
                >= right.y - LABEL_TEXT_HALF_HEIGHT - LABEL_CLEARANCE_Y
            && right.y + 4 + LABEL_CLEARANCE_Y
                >= left.y - LABEL_TEXT_HALF_HEIGHT - LABEL_CLEARANCE_Y
    };

    let mut by_source: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, pl) in pending_labels.iter().enumerate() {
        if adjusted_labels[i].is_none() {
            by_source.entry(pl.from_name.clone()).or_default().push(i);
        }
    }
    for (_from_name, indices) in by_source {
        if indices.len() < 2 {
            continue;
        }
        let mut sorted = indices;
        sorted.sort_by_key(|&i| pending_labels[i].to_name.clone());
        let has_overlap = sorted.iter().enumerate().any(|(pos, &left_idx)| {
            sorted.iter().skip(pos + 1).any(|&right_idx| {
                labels_overlap(&pending_labels[left_idx], &pending_labels[right_idx])
            })
        });
        if !has_overlap {
            continue;
        }
        let count = sorted.len();
        let denom = (count - 1).max(1);
        for (slot, &raw_idx) in sorted.iter().enumerate() {
            let pl = &pending_labels[raw_idx];
            let frac = 0.3 + (slot as f64 / denom as f64) * 0.4;
            let (px, py, dx, dy) = point_on_polyline_at_fraction(&pl.edge_points, frac);
            let (lx, ly) = if dx.abs() < dy.abs() {
                (px + 14, py)
            } else {
                (px, py - 14)
            };
            adjusted_labels[raw_idx] = Some((lx, ly, pl.text.clone(), pl.color.clone()));
        }
    }

    let mut y_clusters: Vec<Vec<usize>> = Vec::new();
    for (i, label) in pending_labels.iter().enumerate() {
        if adjusted_labels[i].is_some() {
            continue;
        }
        let found = y_clusters.iter().position(|cluster| {
            let rep = pending_labels[*cluster.first().expect("non-empty cluster")].y;
            (label.y - rep).abs() <= LABEL_CLUSTER_BAND
        });
        match found {
            Some(ci) => y_clusters[ci].push(i),
            None => y_clusters.push(vec![i]),
        }
    }
    for cluster in y_clusters {
        if cluster.len() >= 2 {
            let count = cluster.len() as i32;
            let mut sorted_idx = cluster;
            sorted_idx.sort_by_key(|&i| pending_labels[i].x);
            let labels_overlap = sorted_idx.windows(2).any(|pair| {
                let left = &pending_labels[pair[0]];
                let right = &pending_labels[pair[1]];
                let left_half_w =
                    (crate::render::text_metrics::monospace_width(&left.text, 7) + 2) / 2;
                let right_half_w =
                    (crate::render::text_metrics::monospace_width(&right.text, 7) + 2) / 2;
                left.x + left_half_w + LABEL_CLEARANCE_X
                    >= right.x - right_half_w - LABEL_CLEARANCE_X
            });
            if labels_overlap {
                let mean_x = sorted_idx.iter().map(|&i| pending_labels[i].x).sum::<i32>() / count;
                for (slot, &raw_idx) in sorted_idx.iter().enumerate() {
                    let offset =
                        (slot as i32) * LABEL_FAN_H_GAP - (count - 1) * LABEL_FAN_H_GAP / 2;
                    adjusted_labels[raw_idx] = Some((
                        mean_x + offset,
                        pending_labels[raw_idx].y,
                        pending_labels[raw_idx].text.clone(),
                        pending_labels[raw_idx].color.clone(),
                    ));
                }
                continue;
            }
            for &raw_idx in &sorted_idx {
                adjusted_labels[raw_idx] = Some((
                    pending_labels[raw_idx].x,
                    pending_labels[raw_idx].y,
                    pending_labels[raw_idx].text.clone(),
                    pending_labels[raw_idx].color.clone(),
                ));
            }
        } else if let Some(&raw_idx) = cluster.first() {
            adjusted_labels[raw_idx] = Some((
                pending_labels[raw_idx].x,
                pending_labels[raw_idx].y,
                pending_labels[raw_idx].text.clone(),
                pending_labels[raw_idx].color.clone(),
            ));
        }
    }

    clear_labels_from_obstacles(&pending_labels, &mut adjusted_labels, positions);

    for (idx, entry) in adjusted_labels.iter_mut().enumerate() {
        if entry.is_none() {
            *entry = Some((
                pending_labels[idx].x,
                pending_labels[idx].y,
                pending_labels[idx].text.clone(),
                pending_labels[idx].color.clone(),
            ));
        }
    }
    for entry in adjusted_labels.into_iter().flatten() {
        let (lx, ly, text, color) = entry;
        out.push_str(&format!(
            "<text class=\"uml-edge-label\" data-uml-label-role=\"edge\" x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
            lx,
            ly,
            escape_text(&color),
            escape_text(&text)
        ));
    }
}

fn point_on_polyline_at_fraction(points: &[(i32, i32)], frac: f64) -> (i32, i32, i32, i32) {
    if points.is_empty() {
        return (0, 0, 0, 0);
    }
    if points.len() == 1 {
        let (x, y) = points[0];
        return (x, y, 0, 0);
    }

    let total_len: f64 = points
        .windows(2)
        .map(|seg| {
            let dx = (seg[1].0 - seg[0].0) as f64;
            let dy = (seg[1].1 - seg[0].1) as f64;
            (dx * dx + dy * dy).sqrt()
        })
        .sum();
    if total_len <= f64::EPSILON {
        let (x, y) = points[0];
        return (x, y, 0, 0);
    }

    let mut remaining = total_len * frac.clamp(0.0, 1.0);
    for seg in points.windows(2) {
        let (ax, ay) = seg[0];
        let (bx, by) = seg[1];
        let dx = (bx - ax) as f64;
        let dy = (by - ay) as f64;
        let seg_len = (dx * dx + dy * dy).sqrt();
        if seg_len <= f64::EPSILON {
            continue;
        }
        if remaining <= seg_len {
            let t = remaining / seg_len;
            return (
                (ax as f64 + dx * t).round() as i32,
                (ay as f64 + dy * t).round() as i32,
                bx - ax,
                by - ay,
            );
        }
        remaining -= seg_len;
    }

    let (ax, ay) = points[points.len() - 2];
    let (bx, by) = points[points.len() - 1];
    (bx, by, bx - ax, by - ay)
}

fn clear_labels_from_obstacles(
    pending_labels: &[BoxGridPendingLabel],
    adjusted_labels: &mut [Option<(i32, i32, String, String)>],
    positions: &BTreeMap<String, (i32, i32, i32, i32)>,
) {
    let obstacle_boxes: Vec<(i32, i32, i32, i32)> = positions.values().copied().collect();
    for (label_idx, entry) in adjusted_labels.iter_mut().enumerate() {
        let (lx, ly, text, _) = match entry.as_mut() {
            Some(e) => e,
            None => continue,
        };
        if text.is_empty() {
            continue;
        }
        let from_box = positions.get(&pending_labels[label_idx].from_name).copied();
        let to_box = positions.get(&pending_labels[label_idx].to_name).copied();
        let edge_obstacles: Vec<(i32, i32, i32, i32)> = obstacle_boxes
            .iter()
            .copied()
            .filter(|&b| Some(b) != from_box && Some(b) != to_box)
            .collect();
        let label_overlaps_any = |lx: i32, ly: i32, text: &str| {
            edge_obstacles
                .iter()
                .any(|&bbox| label_overlaps_box(lx, ly, text, bbox))
        };
        for _ in 0..edge_obstacles.len().max(1) {
            if !label_overlaps_any(*lx, *ly, text) {
                break;
            }
            let half_w = (crate::render::text_metrics::monospace_width(text, 7) + 2) / 2;
            let mut moved = false;
            for &(bx, by, bw, bh) in &edge_obstacles {
                if !label_overlaps_box(*lx, *ly, text, (bx, by, bw, bh)) {
                    continue;
                }
                let candidates = [
                    (*lx, by - 14),
                    (*lx, by + bh + 18),
                    (bx - half_w - 12, *ly),
                    (bx + bw + half_w + 12, *ly),
                ];
                if let Some((next_x, next_y)) = candidates
                    .into_iter()
                    .find(|&(cx, cy)| !label_overlaps_any(cx, cy, text))
                {
                    *lx = next_x;
                    *ly = next_y;
                } else {
                    *ly = by - 14;
                }
                moved = true;
                break;
            }
            if !moved {
                break;
            }
        }
    }
}

fn label_overlaps_box(
    lx: i32,
    ly: i32,
    text: &str,
    (bx, by, bw, bh): (i32, i32, i32, i32),
) -> bool {
    let half_w = (crate::render::text_metrics::monospace_width(text, 7) + 2) / 2;
    lx + half_w + LABEL_CLEARANCE_X >= bx
        && lx - half_w - LABEL_CLEARANCE_X <= bx + bw
        && ly + 4 + LABEL_CLEARANCE_Y >= by
        && ly - LABEL_TEXT_HALF_HEIGHT - LABEL_CLEARANCE_Y <= by + bh
}
