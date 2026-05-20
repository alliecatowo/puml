use super::*;

/// Context passed to `render_class_relations` — groups the many read-only
/// inputs that the relation-rendering loop needs from `render_class_svg`.
pub(super) struct ClassRelationCtx<'a> {
    pub(super) relations: &'a [crate::model::FamilyRelation],
    pub(super) nodes: &'a [crate::model::FamilyNode],
    pub(super) node_boxes: &'a std::collections::BTreeMap<String, ClassNodeBox>,
    pub(super) edge_paths: &'a std::collections::BTreeMap<String, Vec<(f64, f64)>>,
    pub(super) label_override: &'a std::collections::BTreeMap<usize, (i32, i32)>,
    pub(super) parallel_offset: &'a std::collections::BTreeMap<usize, i32>,
    pub(super) relation_pair_label_lanes: &'a std::collections::BTreeMap<usize, i32>,
    pub(super) class_style: &'a crate::theme::ClassStyle,
    pub(super) family_id: &'a str,
    pub(super) arrow_stroke: &'a str,
    pub(super) margin_x: i32,
    pub(super) margin_top: i32,
    pub(super) svg_width: i32,
}

/// Render all edges (relations) for a class/object/usecase SVG diagram.
///
/// Emits `<line>` / `<polyline>` elements for edges, plus stereotype,
/// dependency, regular, cardinality, and role labels for each relation.
/// Nodes are rendered after this call so they visually cover edge endpoints.
pub(super) fn render_class_relations(out: &mut String, ctx: &ClassRelationCtx<'_>) {
    use crate::render::geometry::{compute_edge_anchors_for_direction, pick_port};
    use crate::render::relation::{
        arrow_style, normalize_relation_endpoints, render_lollipop_endpoint,
        usecase_dependency_label,
    };

    for (rel_idx, relation) in ctx.relations.iter().enumerate() {
        let (from_name, to_name, normalized_arrow) =
            normalize_relation_endpoints(&relation.from, &relation.to, &relation.arrow);
        let from = ctx.node_boxes.get(&from_name);
        let to = ctx.node_boxes.get(&to_name);
        let (Some(from), Some(to)) = (from, to) else {
            continue;
        };
        let mut style = arrow_style(&normalized_arrow);
        let usecase_dependency = usecase_dependency_label(relation.label.as_deref())
            .or_else(|| usecase_dependency_label(relation.stereotype.as_deref()));
        if usecase_dependency.is_some() {
            style.dashed = true;
            if style.end_marker.is_none() {
                style.end_marker = Some("arrow-open");
            }
        }
        let (x1, y1, x2, y2) = if relation.direction.is_some() {
            compute_edge_anchors_for_direction(
                (from.x, from.y, from.w, from.h),
                (to.x, to.y, to.w, to.h),
                relation.direction.as_deref(),
            )
        } else {
            pick_port((from.x, from.y, from.w, from.h), (to.x, to.y, to.w, to.h))
        };

        let lat_offset = ctx.parallel_offset.get(&rel_idx).copied().unwrap_or(0);
        let edge_dx_raw = x2 - x1;
        let edge_dy_raw = y2 - y1;
        let (off_x, off_y) = if edge_dx_raw.abs() >= edge_dy_raw.abs() {
            (0, lat_offset)
        } else {
            (lat_offset, 0)
        };
        let (x1, y1, x2, y2) = (x1 + off_x, y1 + off_y, x2 + off_x, y2 + off_y);
        let relation_color = relation.line_color.as_deref().unwrap_or(ctx.arrow_stroke);
        let stroke_width = relation.thickness.unwrap_or(2).clamp(1, 8);
        let stroke_dash = if style.dashed || relation.dashed {
            " stroke-dasharray=\"5 3\""
        } else {
            ""
        };
        let visibility = if relation.hidden {
            " visibility=\"hidden\""
        } else {
            ""
        };
        let mut markers = String::new();
        if let Some(end) = style.end_marker {
            markers.push_str(&format!(" marker-end=\"url(#{end})\""));
        }
        if let Some(start) = style.start_marker {
            markers.push_str(&format!(" marker-start=\"url(#{start})\""));
        }
        let direction_attr = relation
            .direction
            .as_deref()
            .map(|direction| format!(" data-uml-direction=\"{}\"", escape_text(direction)))
            .unwrap_or_default();
        let edge_id = format!("relation:{rel_idx}");
        let family_id = ctx.family_id;
        let puml_edge_attrs =
            crate::render::puml_edge_attrs(&edge_id, family_id, "relation", &from_name, &to_name);

        let mut ortho_pts: Option<Vec<(i32, i32)>> =
            if relation.direction.is_none() && !relation.hidden {
                ctx.edge_paths
                    .get(&format!("r{rel_idx}"))
                    .filter(|p| p.len() >= 2)
                    .map(|p| {
                        p.iter()
                            .map(|&(px, py)| (px as i32 + off_x, py as i32 + off_y))
                            .collect()
                    })
            } else {
                None
            };

        let (label_mx, label_my);

        if let Some(ref mut pts) = ortho_pts {
            // Snap path endpoints to the actual rendered node ports so arrows
            // attach correctly when layout coordinates differ from box anchors
            // (e.g. rounding, lateral offsets).  Also realign the adjacent
            // waypoints so the entry/exit segments stay axis-aligned.
            if let Some(first) = pts.first_mut() {
                *first = (x1, y1);
            }
            if let Some(last) = pts.last_mut() {
                *last = (x2, y2);
            }
            let cn = pts.len();
            if cn >= 3 {
                pts[1].0 = x1;
                if cn > 3 {
                    pts[cn - 2].0 = x2;
                }
            }
            let pts_str: String = pts
                .iter()
                .map(|(px, py)| format!("{px},{py}"))
                .collect::<Vec<_>>()
                .join(" ");
            out.push_str(&format!(
                "<polyline class=\"uml-relation puml-edge\" data-uml-from=\"{}\" data-uml-to=\"{}\" {} data-uml-arrow=\"{}\" points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}{}{} />",
                escape_text(&from_name),
                escape_text(&to_name),
                puml_edge_attrs,
                escape_text(&normalized_arrow),
                pts_str,
                relation_color, stroke_width,
                stroke_dash, visibility, direction_attr, markers
            ));
            let longest_horiz = pts
                .windows(2)
                .filter(|seg| seg[0].1 == seg[1].1)
                .max_by_key(|seg| (seg[1].0 - seg[0].0).abs());
            let (lmx, lmy) = match longest_horiz {
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
            };
            label_mx = lmx;
            label_my = lmy;
        } else {
            out.push_str(&format!(
                "<line class=\"uml-relation puml-edge\" data-uml-from=\"{}\" data-uml-to=\"{}\" {} x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\" stroke=\"{relation_color}\" stroke-width=\"{stroke_width}\"{dash}{visibility}{direction_attr}{markers}/>",
                escape_text(&relation.from),
                escape_text(&relation.to),
                puml_edge_attrs,
                dash = stroke_dash,
            ));
            label_mx = (x1 + x2) / 2;
            label_my = (y1 + y2) / 2 - 12;
        }
        let edge_dx = x2 - x1;
        let edge_dy = y2 - y1;

        if relation.left_lollipop {
            render_lollipop_endpoint(out, x1, y1, relation_color);
        }
        if relation.right_lollipop {
            render_lollipop_endpoint(out, x2, y2, relation_color);
        }
        if let Some(left) = &relation.left_cardinality {
            out.push_str(&format!(
                "<text class=\"puml-label\" {} x=\"{x}\" y=\"{y}\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"10\" fill=\"{member_color}\">{txt}</text>",
                text_semantic_attrs(&edge_id, "cardinality", x1 - 4, y1 - 6, left, 10, false),
                x = x1 - 4,
                y = y1 - 6,
                member_color = ctx.class_style.member_color,
                txt = escape_text(left)
            ));
        }
        if let Some(right) = &relation.right_cardinality {
            out.push_str(&format!(
                "<text class=\"puml-label\" {} x=\"{x}\" y=\"{y}\" text-anchor=\"start\" font-family=\"monospace\" font-size=\"10\" fill=\"{member_color}\">{txt}</text>",
                text_semantic_attrs(&edge_id, "cardinality", x2 + 4, y2 - 6, right, 10, false),
                x = x2 + 4,
                y = y2 - 6,
                member_color = ctx.class_style.member_color,
                txt = escape_text(right)
            ));
        }
        if let Some(left_role) = &relation.left_role {
            out.push_str(&format!(
                "<text class=\"puml-label\" {} x=\"{x}\" y=\"{y}\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"10\" fill=\"{member_color}\">{txt}</text>",
                text_semantic_attrs(&edge_id, "role", x1 - 4, y1 + 12, left_role, 10, false),
                x = x1 - 4,
                y = y1 + 12,
                member_color = ctx.class_style.member_color,
                txt = escape_text(left_role)
            ));
        }
        if let Some(right_role) = &relation.right_role {
            out.push_str(&format!(
                "<text class=\"puml-label\" {} x=\"{x}\" y=\"{y}\" text-anchor=\"start\" font-family=\"monospace\" font-size=\"10\" fill=\"{member_color}\">{txt}</text>",
                text_semantic_attrs(&edge_id, "role", x2 + 4, y2 + 12, right_role, 10, false),
                x = x2 + 4,
                y = y2 + 12,
                member_color = ctx.class_style.member_color,
                txt = escape_text(right_role)
            ));
        }
        let pair_label_lane = ctx
            .relation_pair_label_lanes
            .get(&rel_idx)
            .copied()
            .unwrap_or(0);
        let from_is_class = ctx
            .nodes
            .iter()
            .find(|node| node.name == from_name)
            .is_some_and(|node| matches!(node.kind, FamilyNodeKind::Class));
        let to_is_class = ctx
            .nodes
            .iter()
            .find(|node| node.name == to_name)
            .is_some_and(|node| matches!(node.kind, FamilyNodeKind::Class));
        let prefer_side_clearance = pair_label_lane != 0 || (from_is_class && to_is_class);
        if let Some(stereotype) = &relation.stereotype {
            if usecase_dependency.is_none() {
                let (sx, base_sy) = ctx
                    .label_override
                    .get(&rel_idx)
                    .copied()
                    .unwrap_or((label_mx, label_my));
                let sy = base_sy - if relation.label.is_some() { 24 } else { 14 } + pair_label_lane;
                out.push_str(&format!(
                    "<text class=\"puml-label\" {} x=\"{sx}\" y=\"{sy}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"{member_color}\">&lt;&lt;{txt}&gt;&gt;</text>",
                    text_semantic_attrs(&edge_id, "stereotype", sx, sy, stereotype, 10, true),
                    member_color = ctx.class_style.member_color,
                    txt = escape_text(stereotype)
                ));
            }
        }
        if let Some(label) = usecase_dependency {
            let (lx, ly) = if let Some(&(ox, oy)) = ctx.label_override.get(&rel_idx) {
                (ox, oy)
            } else if ortho_pts.is_some() {
                if edge_dy.abs() > edge_dx.abs() {
                    (label_mx + 14, label_my)
                } else {
                    (label_mx, label_my - 14)
                }
            } else {
                let dx = x2 - x1;
                let dy = y2 - y1;
                let dx_abs = dx.abs();
                let dy_abs = dy.abs();
                let edge_len = ((dx_abs * dx_abs + dy_abs * dy_abs) as f64).sqrt() as i32;
                if edge_len <= 2 {
                    ((x1 + x2) / 2, (y1 + y2) / 2 - 12)
                } else {
                    let clearance = 30i32;
                    let t_num = (edge_len * 2 / 5).max(clearance).min(edge_len - clearance);
                    let raw_x = x1 + dx * t_num / edge_len;
                    let raw_y = y1 + dy * t_num / edge_len;
                    if dy_abs > dx_abs {
                        (raw_x + 14, raw_y - 6)
                    } else {
                        (raw_x, raw_y - 14)
                    }
                }
            };
            let label_half_w = ((label.chars().count() as i32) * 3).max(18);
            let corridor_left = from.x.max(to.x);
            let corridor_right = (from.x + from.w).min(to.x + to.w);
            let lx = if prefer_side_clearance
                && edge_dy.abs() > edge_dx.abs()
                && corridor_left < corridor_right
                && lx > corridor_left - 8 - label_half_w
                && lx < corridor_right + 8 + label_half_w
            {
                if x2 >= x1 {
                    corridor_right + 8 + label_half_w
                } else {
                    corridor_left - 8 - label_half_w
                }
            } else {
                lx
            };
            let lx = lx.clamp(
                ctx.margin_x + 8 + label_half_w,
                ctx.svg_width - ctx.margin_x - 8 - label_half_w,
            );
            let ly = (ly + pair_label_lane).max(ctx.margin_top + 10);
            let (lx, ly) = class_nudge_label_y(lx, ly, label_half_w, ctx.node_boxes);
            out.push_str(&relation_label_svg(
                lx,
                ly,
                label,
                11,
                &ctx.class_style.member_color,
                &edge_id,
                "edge-label",
            ));
        } else if let Some(label) = relation.label.as_deref() {
            let (lx, ly) = if let Some(&(ox, oy)) = ctx.label_override.get(&rel_idx) {
                (ox, oy)
            } else if let Some(ref pts) = ortho_pts {
                let has_horiz = pts.windows(2).any(|seg| seg[0].1 == seg[1].1);
                if !has_horiz {
                    ((x1 + x2) / 2, (y1 + y2) / 2 - 12)
                } else if edge_dy.abs() > edge_dx.abs() {
                    (label_mx + 14, label_my)
                } else {
                    (label_mx, label_my - 14)
                }
            } else {
                let dx = x2 - x1;
                let dy = y2 - y1;
                let dx_abs = dx.abs();
                let dy_abs = dy.abs();
                let edge_len = ((dx_abs * dx_abs + dy_abs * dy_abs) as f64).sqrt() as i32;
                if edge_len <= 2 {
                    ((x1 + x2) / 2, (y1 + y2) / 2 - 12)
                } else {
                    let clearance = 30i32;
                    let t_num = (edge_len * 2 / 5).max(clearance).min(edge_len - clearance);
                    let raw_x = x1 + dx * t_num / edge_len;
                    let raw_y = y1 + dy * t_num / edge_len;
                    if dy_abs > dx_abs {
                        (raw_x + 14, raw_y - 6)
                    } else {
                        (raw_x, raw_y - 14)
                    }
                }
            };
            let label_half_w = ((label.chars().count() as i32) * 3).max(18);
            let corridor_left = from.x.max(to.x);
            let corridor_right = (from.x + from.w).min(to.x + to.w);
            let lx = if prefer_side_clearance
                && edge_dy.abs() > edge_dx.abs()
                && corridor_left < corridor_right
                && lx > corridor_left - 8 - label_half_w
                && lx < corridor_right + 8 + label_half_w
            {
                if x2 >= x1 {
                    corridor_right + 8 + label_half_w
                } else {
                    corridor_left - 8 - label_half_w
                }
            } else {
                lx
            };
            let lx = lx.clamp(
                ctx.margin_x + 8 + label_half_w,
                ctx.svg_width - ctx.margin_x - 8 - label_half_w,
            );
            let ly = (ly + pair_label_lane).max(ctx.margin_top + 10);
            let (lx, ly) = class_nudge_label_y(lx, ly, label_half_w, ctx.node_boxes);
            out.push_str(&relation_label_svg(
                lx,
                ly,
                label,
                11,
                &ctx.class_style.member_color,
                &edge_id,
                "edge-label",
            ));
        }
    }
}

/// Build the `label_override` map for `render_class_svg`.
///
/// Performs the label de-collision pre-pass: clusters labels that land in the
/// same y-band or converge on the same target node, then fans them out so they
/// don't overlap.  Returns a map from `rel_idx` to the de-collided `(lx, ly)`.
pub(super) fn class_build_label_overrides(
    relations: &[crate::model::FamilyRelation],
    node_boxes: &std::collections::BTreeMap<String, ClassNodeBox>,
    edge_paths: &std::collections::BTreeMap<String, Vec<(f64, f64)>>,
) -> std::collections::BTreeMap<usize, (i32, i32)> {
    const LABEL_FAN_GAP: i32 = 24;
    const LABEL_CLUSTER_BAND: i32 = 18;

    let mut label_override: std::collections::BTreeMap<usize, (i32, i32)> =
        std::collections::BTreeMap::new();

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
        let from = match node_boxes.get(&from_name) {
            Some(b) => b,
            None => continue,
        };
        let to = match node_boxes.get(&to_name) {
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
    let mut by_target: std::collections::BTreeMap<String, Vec<usize>> =
        std::collections::BTreeMap::new();
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
    let mut by_source: std::collections::BTreeMap<String, Vec<usize>> =
        std::collections::BTreeMap::new();
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
