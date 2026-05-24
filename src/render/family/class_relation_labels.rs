use std::collections::BTreeMap;

use crate::model::{FamilyDocument, FamilyNodeKind};
use crate::render::geometry::{compute_edge_anchors_for_direction, pick_port};
use crate::render::relation::normalize_relation_endpoints;
use crate::render::svg::escape_text;

use super::c4_nodes::is_c4_component_kind;
use super::class_routing::class_nudge_label_y;
use super::class_types::ClassNodeBox;

/// Emit a centered SVG `<text>` element for a relation label.
///
/// Labels may contain `\n` after normalization merges multiple Rel() calls on
/// the same source→target pair into a single coalesced label (#425).  Each
/// logical line is emitted as a `<tspan>` so they stack visually instead of
/// being run together as a single string of whitespace.
pub(super) fn relation_label_svg(
    x: i32,
    y: i32,
    label: &str,
    font_size: i32,
    fill: &str,
) -> String {
    let lines: Vec<&str> = label.split('\n').collect();
    if lines.len() <= 1 {
        // Fast path – no newline, emit plain text element.
        return format!(
            "<text class=\"uml-edge-label\" data-uml-label-role=\"edge\" x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"{}\" fill=\"{}\">{}</text>",
            x, y, font_size, escape_text(fill), escape_text(label)
        );
    }
    // Multiline: emit one <tspan> per logical line, each shifted down by
    // (font_size + 2) pixels so lines are clearly separated.
    let line_h = font_size + 2;
    let total_h = (lines.len() as i32 - 1) * line_h;
    // Start above the anchor so the block is centred on y.
    let start_y = y - total_h / 2;
    let mut buf = format!(
        "<text class=\"uml-edge-label\" data-uml-label-role=\"edge\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"{}\" fill=\"{}\">",
        font_size,
        escape_text(fill)
    );
    for (i, line) in lines.iter().enumerate() {
        let ty = start_y + (i as i32) * line_h;
        buf.push_str(&format!(
            "<tspan x=\"{}\" y=\"{}\">{}</tspan>",
            x,
            ty,
            escape_text(line)
        ));
    }
    buf.push_str("</text>");
    buf
}

pub(super) fn relation_pair_label_lane_map(document: &FamilyDocument) -> BTreeMap<usize, i32> {
    let mut pair_counts: BTreeMap<(String, String), i32> = BTreeMap::new();
    let mut pair_seen: BTreeMap<(String, String), i32> = BTreeMap::new();
    let mut lanes = BTreeMap::new();

    for relation in &document.relations {
        let key = if relation.from <= relation.to {
            (relation.from.clone(), relation.to.clone())
        } else {
            (relation.to.clone(), relation.from.clone())
        };
        *pair_counts.entry(key).or_insert(0) += 1;
    }

    for (idx, relation) in document.relations.iter().enumerate() {
        let key = if relation.from <= relation.to {
            (relation.from.clone(), relation.to.clone())
        } else {
            (relation.to.clone(), relation.from.clone())
        };
        let count = pair_counts.get(&key).copied().unwrap_or(1);
        let seen = pair_seen.entry(key).or_insert(0);
        let lane = if count <= 1 {
            0
        } else {
            (*seen * 2 - (count - 1)) * 14
        };
        *seen += 1;
        lanes.insert(idx, lane);
    }

    lanes
}

pub(super) fn relation_source_label_lane_map(document: &FamilyDocument) -> BTreeMap<usize, i32> {
    let mut kind_by_key: BTreeMap<String, FamilyNodeKind> = BTreeMap::new();
    for node in &document.nodes {
        kind_by_key.insert(node.name.clone(), node.kind);
        if let Some(alias) = &node.alias {
            kind_by_key.insert(alias.clone(), node.kind);
        }
    }
    let mut source_labeled_indices: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut lanes = BTreeMap::new();

    for (idx, relation) in document.relations.iter().enumerate() {
        let has_label = relation
            .label
            .as_deref()
            .is_some_and(|label| !label.trim().is_empty())
            || relation
                .stereotype
                .as_deref()
                .is_some_and(|label| !label.trim().is_empty());
        if has_label
            && kind_by_key
                .get(&relation.from)
                .copied()
                .is_some_and(is_c4_component_kind)
        {
            source_labeled_indices
                .entry(relation.from.clone())
                .or_default()
                .push(idx);
        }
    }

    for indices in source_labeled_indices.values() {
        if indices.len() <= 1 {
            continue;
        }
        for (slot, rel_idx) in indices.iter().enumerate() {
            lanes.insert(*rel_idx, (slot as i32) * 20);
        }
    }

    lanes
}

/// Build the `label_override` map for `render_class_svg`.
///
/// Performs the label de-collision pre-pass: clusters labels that land in the
/// same y-band or converge on the same target node, then fans them out so they
/// don't overlap.  Returns a map from `rel_idx` to the de-collided `(lx, ly)`.
pub(super) fn class_build_label_overrides(
    relations: &[crate::model::FamilyRelation],
    node_boxes: &BTreeMap<String, ClassNodeBox>,
    edge_paths: &BTreeMap<String, Vec<(f64, f64)>>,
) -> BTreeMap<usize, (i32, i32)> {
    const LABEL_FAN_GAP: i32 = 24;
    const LABEL_CLUSTER_BAND: i32 = 18;

    let mut label_override: BTreeMap<usize, (i32, i32)> = BTreeMap::new();

    struct RawLabel {
        rel_idx: usize,
        from_name: String,
        to_name: String,
        text: String,
        lx: i32,
        ly: i32,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
    }
    let mut raw_labels: Vec<RawLabel> = Vec::new();

    for (rel_idx, relation) in relations.iter().enumerate() {
        let label_text = relation.label.as_deref().or(relation.stereotype.as_deref());
        if label_text.is_none() {
            continue;
        }
        let (from_name, to_name, _arrow) =
            normalize_relation_endpoints(&relation.from, &relation.to, &relation.arrow);
        let from_key = resolve_relation_endpoint_key(&from_name, node_boxes);
        let to_key = resolve_relation_endpoint_key(&to_name, node_boxes);
        let from = match node_boxes.get(&from_key) {
            Some(b) => b,
            None => continue,
        };
        let to = match node_boxes.get(&to_key) {
            Some(b) => b,
            None => continue,
        };
        let (x1, y1, x2, y2) = if relation.direction.is_some() {
            compute_edge_anchors_for_direction(
                (from.x, from.y, from.w, from.h),
                (to.x, to.y, to.w, to.h),
                relation.direction.as_deref(),
            )
        } else {
            pick_port((from.x, from.y, from.w, from.h), (to.x, to.y, to.w, to.h))
        };
        let ortho_pts: Option<Vec<(i32, i32)>> = if relation.direction.is_none() && !relation.hidden
        {
            edge_paths
                .get(&format!("r{rel_idx}"))
                .filter(|p| p.len() >= 2)
                .map(|p| p.iter().map(|&(px, py)| (px as i32, py as i32)).collect())
        } else {
            None
        };
        let (lx, ly) = if let Some(ref pts) = ortho_pts {
            let longest_horiz = pts
                .windows(2)
                .filter(|seg| seg[0].1 == seg[1].1)
                .max_by_key(|seg| (seg[1].0 - seg[0].0).abs());
            match longest_horiz {
                Some(seg) => ((seg[0].0 + seg[1].0) / 2, seg[0].1 - 12),
                None => {
                    let longest_seg = pts.windows(2).max_by_key(|seg| {
                        let (ax, ay) = seg[0];
                        let (bx, by_) = seg[1];
                        (bx - ax).pow(2) + (by_ - ay).pow(2)
                    });
                    match longest_seg {
                        Some(seg) => ((seg[0].0 + seg[1].0) / 2, (seg[0].1 + seg[1].1) / 2 - 12),
                        None => ((x1 + x2) / 2, (y1 + y2) / 2 - 12),
                    }
                }
            }
        } else {
            ((x1 + x2) / 2, (y1 + y2) / 2 - 12)
        };
        raw_labels.push(RawLabel {
            rel_idx,
            from_name,
            to_name,
            text: label_text.unwrap_or_default().to_string(),
            lx,
            ly,
            x1,
            y1,
            x2,
            y2,
        });
    }

    // ── Target-based fan (≥ 2 labels → same target node) ─────────────────────
    let mut by_target: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, rl) in raw_labels.iter().enumerate() {
        by_target.entry(rl.to_name.clone()).or_default().push(i);
    }
    for (to_name, group) in &by_target {
        if group.len() < 2 {
            continue;
        }
        let target_box = match node_boxes.get(to_name.as_str()) {
            Some(b) => b,
            None => continue,
        };
        let anchor_y = target_box.y - 14;
        let anchor_cx = target_box.x + target_box.w / 2;
        let n = group.len() as i32;
        let mut sorted = group.clone();
        sorted.sort_unstable();
        let total_width = sorted
            .iter()
            .map(|&raw_idx| (((raw_labels[raw_idx].text.chars().count() as i32) * 3).max(18)) * 2)
            .sum::<i32>()
            + (n - 1) * LABEL_FAN_GAP;
        let mut cursor = -total_width / 2;
        for &raw_idx in &sorted {
            let label_half_w = ((raw_labels[raw_idx].text.chars().count() as i32) * 3).max(18);
            let center_offset = cursor + label_half_w;
            let anchor = class_nudge_label_y(
                anchor_cx + center_offset,
                anchor_y,
                label_half_w,
                node_boxes,
            );
            label_override.insert(raw_labels[raw_idx].rel_idx, anchor);
            cursor += label_half_w * 2 + LABEL_FAN_GAP;
        }
    }

    // ── Source-based fan (≥ 2 labelled edges share the same source node) ─────
    let mut by_source: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, rl) in raw_labels.iter().enumerate() {
        if !label_override.contains_key(&rl.rel_idx) {
            by_source.entry(rl.from_name.clone()).or_default().push(i);
        }
    }
    for group in by_source.values() {
        if group.len() < 2 {
            continue;
        }
        let mut sorted = group.clone();
        sorted.sort_unstable();
        let count = sorted.len();
        for (slot, &raw_idx) in sorted.iter().enumerate() {
            let rl = &raw_labels[raw_idx];
            let frac = 0.3 + (slot as f64 / count as f64) * 0.4;
            let dx = rl.x2 - rl.x1;
            let dy = rl.y2 - rl.y1;
            let lx = rl.x1 + (dx as f64 * frac) as i32;
            let ly = rl.y1 + (dy as f64 * frac) as i32 - 12;
            let (lx, ly) = if dy.abs() > dx.abs() {
                (lx + 14, ly)
            } else {
                (lx, ly - 2)
            };
            let label_half_w = ((rl.text.chars().count() as i32) * 3).max(18);
            let (lx, ly) = class_nudge_label_y(lx, ly, label_half_w, node_boxes);
            label_override.insert(rl.rel_idx, (lx, ly));
        }
    }

    // ── Same-y cluster fan (remaining labels in the same horizontal channel) ──
    let mut y_clusters: Vec<Vec<usize>> = Vec::new();
    for i in 0..raw_labels.len() {
        if label_override.contains_key(&raw_labels[i].rel_idx) {
            continue;
        }
        let ly_i = raw_labels[i].ly;
        let found = y_clusters.iter().position(|cluster| {
            let rep = cluster
                .first()
                .map(|&idx| raw_labels[idx].ly)
                .unwrap_or(ly_i);
            (ly_i - rep).abs() <= LABEL_CLUSTER_BAND
        });
        match found {
            Some(ci) => y_clusters[ci].push(i),
            None => y_clusters.push(vec![i]),
        }
    }
    for cluster in &y_clusters {
        if cluster.len() < 2 {
            continue;
        }
        let mean_x = cluster.iter().map(|&i| raw_labels[i].lx).sum::<i32>() / cluster.len() as i32;
        let mut sorted = cluster.clone();
        sorted.sort_by_key(|&i| raw_labels[i].lx);
        let n = sorted.len() as i32;
        let total_width = sorted
            .iter()
            .map(|&raw_idx| (((raw_labels[raw_idx].text.chars().count() as i32) * 3).max(18)) * 2)
            .sum::<i32>()
            + (n - 1) * LABEL_FAN_GAP;
        let mut cursor = -total_width / 2;
        for &raw_idx in &sorted {
            let label_half_w = ((raw_labels[raw_idx].text.chars().count() as i32) * 3).max(18);
            let center_offset = cursor + label_half_w;
            let anchor = class_nudge_label_y(
                mean_x + center_offset,
                raw_labels[raw_idx].ly,
                label_half_w,
                node_boxes,
            );
            label_override.insert(raw_labels[raw_idx].rel_idx, anchor);
            cursor += label_half_w * 2 + LABEL_FAN_GAP;
        }
    }

    label_override
}

pub(super) fn resolve_relation_endpoint_key(
    endpoint: &str,
    node_boxes: &BTreeMap<String, ClassNodeBox>,
) -> String {
    if node_boxes.contains_key(endpoint) {
        return endpoint.to_string();
    }
    if let Some((owner, _member)) = endpoint.rsplit_once("::") {
        if node_boxes.contains_key(owner) {
            return owner.to_string();
        }
    }
    endpoint.to_string()
}
