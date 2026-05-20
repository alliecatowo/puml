use super::*;

/// Per-relation label gathered during edge rendering, de-collided in Phase 3.
pub(super) struct BoxGridPendingLabel {
    x: i32,
    y: i32,
    text: String,
    color: String,
    edge_id: String,
    from_name: String,
    to_name: String,
}

/// Render all edges (Phase 2) and de-collide their labels (Phase 3) for
/// `render_box_grid_svg` (component and deployment diagrams).
///
/// Uses the hierarchical orthogonal paths from the layout engine when available,
/// falling back to an L/Z-shape collision-avoidance router.  Labels are
/// gathered into a pending list and de-collided before emission.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)] // pkg_frame_boxes pairs a rect tuple with its member names; a named struct would be overkill for an internal helper
pub(super) fn render_box_grid_relations_and_labels(
    out: &mut String,
    doc: &FamilyDocument,
    family: &str,
    positions: &std::collections::BTreeMap<String, (i32, i32, i32, i32)>,
    interface_nodes: &std::collections::BTreeSet<String>,
    all_boxes: &[(i32, i32, i32, i32)],
    pkg_frame_boxes: &[((i32, i32, i32, i32), &[String])],
    edge_paths: &std::collections::BTreeMap<String, Vec<(f64, f64)>>,
    comp_style: &ComponentStyle,
) {
    use crate::render::geometry::{compute_edge_anchors_for_direction, pick_port};
    use crate::render::relation::{
        arrow_style, normalize_relation_endpoints, render_lollipop_endpoint,
        render_relation_marker_defs_with_prefix, usecase_dependency_label,
    };

    let mut pending_labels: Vec<BoxGridPendingLabel> = Vec::new();

    // Helper: adjust port anchor for interface circle nodes.
    let adjust_interface_anchor =
        |node_box: (i32, i32, i32, i32), other_box: (i32, i32, i32, i32)| {
            const INTERFACE_RADIUS: i32 = 18;
            let (nx, ny, nw, nh) = node_box;
            let (ox, oy, ow, oh) = other_box;
            let cx = nx + nw / 2;
            let cy = ny + nh / 2;
            let other_cx = ox + ow / 2;
            let other_cy = oy + oh / 2;
            let dx = other_cx - cx;
            let dy = other_cy - cy;
            if dx.abs() >= dy.abs() {
                (
                    cx + if dx >= 0 {
                        INTERFACE_RADIUS
                    } else {
                        -INTERFACE_RADIUS
                    },
                    cy,
                )
            } else {
                (
                    cx,
                    cy + if dy >= 0 {
                        INTERFACE_RADIUS
                    } else {
                        -INTERFACE_RADIUS
                    },
                )
            }
        };

    // ── Phase 2: Draw relations ──────────────────────────────────────────────
    for (rel_idx, rel) in doc.relations.iter().enumerate() {
        let (from_name, to_name, normalized_arrow) =
            normalize_relation_endpoints(&rel.from, &rel.to, &rel.arrow);
        let from_box = positions.get(&from_name);
        let to_box = positions.get(&to_name);
        let (Some(&(fx, fy, fw, fh)), Some(&(tx, ty, tw, th))) = (from_box, to_box) else {
            continue;
        };
        let (mut x1, mut y1, mut x2, mut y2) = if rel.direction.is_some() {
            compute_edge_anchors_for_direction(
                (fx, fy, fw, fh),
                (tx, ty, tw, th),
                rel.direction.as_deref(),
            )
        } else {
            pick_port((fx, fy, fw, fh), (tx, ty, tw, th))
        };
        if family == "component" {
            if interface_nodes.contains(&from_name) {
                (x1, y1) = adjust_interface_anchor((fx, fy, fw, fh), (tx, ty, tw, th));
            }
            if interface_nodes.contains(&to_name) {
                (x2, y2) = adjust_interface_anchor((tx, ty, tw, th), (fx, fy, fw, fh));
            }
        }

        let style = arrow_style(&normalized_arrow);
        let relation_color = rel.line_color.as_deref().unwrap_or(&comp_style.arrow_color);
        let marker_prefix = if rel.line_color.is_some() && relation_color != comp_style.arrow_color
        {
            let prefix = format!("uml-rel-{rel_idx}-");
            render_relation_marker_defs_with_prefix(out, relation_color, &prefix);
            prefix
        } else {
            String::new()
        };
        let stroke_width = rel.thickness.unwrap_or(2).clamp(1, 8);
        let dash_attr = if style.dashed || rel.dashed {
            " stroke-dasharray=\"5 3\""
        } else {
            ""
        };
        let visibility_attr = if rel.hidden {
            " visibility=\"hidden\""
        } else {
            ""
        };
        let mut markers = String::new();
        if let Some(end) = style.end_marker {
            markers.push_str(&format!(" marker-end=\"url(#{marker_prefix}{end})\""));
        }
        if let Some(start) = style.start_marker {
            markers.push_str(&format!(" marker-start=\"url(#{marker_prefix}{start})\""));
        }
        let direction_attr = rel
            .direction
            .as_deref()
            .map(|d| format!(" data-uml-direction=\"{}\"", escape_text(d)))
            .unwrap_or_default();
        let edge_id = format!("relation:{rel_idx}");
        let puml_edge_attrs =
            crate::render::puml_edge_attrs(&edge_id, family, "relation", &from_name, &to_name);
        let style_attr = {
            let mut tokens: Vec<String> = Vec::new();
            if rel.line_color.is_some() {
                tokens.push(format!("color:{relation_color}"));
            }
            if style.dashed || rel.dashed {
                tokens.push("dashed".to_string());
            }
            if rel.hidden {
                tokens.push("hidden".to_string());
            }
            if rel.thickness.is_some() {
                tokens.push(format!("thickness:{stroke_width}"));
            }
            if tokens.is_empty() {
                String::new()
            } else {
                format!(
                    " data-uml-relation-style=\"{}\"",
                    escape_text(&tokens.join(" "))
                )
            }
        };

        let label_color = "#1e293b";
        let (label_mx, label_my);

        // Prefer orthogonal path from layout engine; fall back to L/Z router.
        let ortho_path: Option<Vec<(i32, i32)>> = if rel.direction.is_none() && !rel.hidden {
            edge_paths
                .get(&format!("r{rel_idx}"))
                .filter(|p| p.len() >= 2)
                .map(|p| p.iter().map(|&(px, py)| (px as i32, py as i32)).collect())
        } else {
            None
        };

        if let Some(mut orth_pts) = ortho_path {
            // Snap the path endpoints to the actual pick_port anchors.
            // This ensures arrows attach to the correct box edge.
            // Also snap the adjacent intermediate waypoints' x-coordinate so
            // the first and last path segments remain vertical (orthogonal),
            // preventing diagonal segments when the anchor x differs from
            // the graph_layout-computed port x.
            //
            // For a downward path with n≥3 points:
            //   [0] → snap to (x1, y1); [1].x → x1 (vertical exit from src)
            //   [n-1] → snap to (x2, y2); [n-2].x → x2 (vertical entry to tgt)
            if let Some(first) = orth_pts.first_mut() {
                *first = (x1, y1);
            }
            if let Some(last) = orth_pts.last_mut() {
                *last = (x2, y2);
            }
            let n = orth_pts.len();
            if n >= 3 {
                // Snap the second point's x to x1 so the exit segment from
                // the source is vertical (orthogonal from the snapped endpoint).
                orth_pts[1].0 = x1;
                // Snap the penultimate point's x to x2 so the entry segment
                // into the target is also vertical.  For 3-point paths this is
                // the same element as index 1, so only update when n > 3 to
                // avoid overwriting the x1-snap above.
                if n > 3 {
                    orth_pts[n - 2].0 = x2;
                }
            }
            let pts_str: String = orth_pts
                .iter()
                .map(|(px, py)| format!("{px},{py}"))
                .collect::<Vec<_>>()
                .join(" ");
            out.push_str(&format!(
                "<polyline class=\"uml-relation puml-edge\" data-uml-from=\"{}\" data-uml-to=\"{}\" {} data-uml-arrow=\"{}\" points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}{}{}{} />",
                escape_text(&from_name), escape_text(&to_name), puml_edge_attrs,
                escape_text(&normalized_arrow), pts_str, relation_color, stroke_width,
                dash_attr, visibility_attr, direction_attr, style_attr, markers
            ));
            let longest_horiz = orth_pts
                .windows(2)
                .filter(|seg| seg[0].1 == seg[1].1)
                .max_by_key(|seg| (seg[1].0 - seg[0].0).abs());
            let (lmx, lmy) = match longest_horiz {
                Some(seg) => ((seg[0].0 + seg[1].0) / 2, seg[0].1 - 12),
                None => ((x1 + x2) / 2, (y1 + y2) / 2 - 12),
            };
            label_mx = lmx;
            label_my = lmy;
        } else {
            // ── Legacy L/Z-shape routing ─────────────────────────────────────
            let rel_obstacles: Vec<(i32, i32, i32, i32)> = {
                let mut obs: Vec<(i32, i32, i32, i32)> = all_boxes.to_vec();
                for &(rect, members) in pkg_frame_boxes {
                    let src_inside = members.iter().any(|m| m == &from_name);
                    let tgt_inside = members.iter().any(|m| m == &to_name);
                    if !src_inside && !tgt_inside {
                        obs.push(rect);
                    }
                }
                obs
            };
            let line_collides = if rel.direction.is_some() || rel.hidden {
                false
            } else {
                all_boxes.iter().any(|&(bx, by, bw, bh)| {
                    if (bx, by, bw, bh) == (fx, fy, fw, fh) || (bx, by, bw, bh) == (tx, ty, tw, th)
                    {
                        return false;
                    }
                    segment_intersects_rect(x1, y1, x2, y2, (bx, by, bw, bh))
                })
            };
            if !line_collides {
                out.push_str(&format!(
                    "<line class=\"uml-relation puml-edge\" data-uml-from=\"{}\" data-uml-to=\"{}\" {} data-uml-arrow=\"{}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}{}{}{}{} />",
                    escape_text(&from_name), escape_text(&to_name), puml_edge_attrs,
                    escape_text(&normalized_arrow), x1, y1, x2, y2, relation_color, stroke_width,
                    dash_attr, visibility_attr, direction_attr, style_attr, markers
                ));
                label_mx = (x1 + x2) / 2;
                label_my = (y1 + y2) / 2 - 12;
            } else {
                // L/Z-shape routing: try L first; escalate to Z if still collides.
                let src_cx = fx + fw / 2;
                let tgt_cx = tx + tw / 2;
                let src_cy = fy + fh / 2;
                let tgt_cy = ty + th / 2;
                let dx_abs = (tgt_cx - src_cx).abs();
                let dy_abs = (tgt_cy - src_cy).abs();
                let hv_mid_x = (x1 + x2) / 2;
                let hv_pts = [(x1, y1), (hv_mid_x, y1), (hv_mid_x, y2), (x2, y2)];
                let vh_mid_y = (y1 + y2) / 2;
                let vh_pts = [(x1, y1), (x1, vh_mid_y), (x2, vh_mid_y), (x2, y2)];
                let hv_col = count_polyline_collisions(
                    &hv_pts,
                    &rel_obstacles,
                    (fx, fy, fw, fh),
                    (tx, ty, tw, th),
                );
                let vh_col = count_polyline_collisions(
                    &vh_pts,
                    &rel_obstacles,
                    (fx, fy, fw, fh),
                    (tx, ty, tw, th),
                );
                let (l_pts, l_col) = if dx_abs >= dy_abs {
                    if vh_col <= hv_col {
                        (&vh_pts[..], vh_col)
                    } else {
                        (&hv_pts[..], hv_col)
                    }
                } else {
                    if hv_col <= vh_col {
                        (&hv_pts[..], hv_col)
                    } else {
                        (&vh_pts[..], vh_col)
                    }
                };
                let blocking: Vec<(i32, i32, i32, i32)> = if l_col > 0 {
                    rel_obstacles
                        .iter()
                        .copied()
                        .filter(|&b| {
                            b != (fx, fy, fw, fh)
                                && b != (tx, ty, tw, th)
                                && l_pts.windows(2).any(|seg| {
                                    segment_intersects_rect(
                                        seg[0].0, seg[0].1, seg[1].0, seg[1].1, b,
                                    )
                                })
                        })
                        .collect()
                } else {
                    Vec::new()
                };
                let best_pts: Vec<(i32, i32)> = if l_col == 0 || blocking.is_empty() {
                    l_pts.to_vec()
                } else {
                    let gap = 12i32;
                    let mut best: Option<Vec<(i32, i32)>> = None;
                    let mut best_col = l_col;
                    let mut waypoint_candidates: Vec<(i32, i32)> = Vec::new();
                    for &(bx, by, bw, bh) in &blocking {
                        waypoint_candidates.push((bx + bw / 2, by - gap));
                        waypoint_candidates.push((bx + bw / 2, by + bh + gap));
                        waypoint_candidates.push((bx - gap, by + bh / 2));
                        waypoint_candidates.push((bx + bw + gap, by + bh / 2));
                        waypoint_candidates.push((bx - gap, by - gap));
                        waypoint_candidates.push((bx + bw + gap, by - gap));
                        waypoint_candidates.push((bx - gap, by + bh + gap));
                        waypoint_candidates.push((bx + bw + gap, by + bh + gap));
                    }
                    'waypoint_loop: for &(wx, wy) in &waypoint_candidates {
                        let z1: Vec<(i32, i32)> = vec![(x1, y1), (wx, y1), (wx, y2), (x2, y2)];
                        let z2: Vec<(i32, i32)> = vec![(x1, y1), (x1, wy), (x2, wy), (x2, y2)];
                        let z3: Vec<(i32, i32)> =
                            vec![(x1, y1), (wx, y1), (wx, wy), (x2, wy), (x2, y2)];
                        let z4: Vec<(i32, i32)> =
                            vec![(x1, y1), (x1, wy), (wx, wy), (wx, y2), (x2, y2)];
                        for cand in [&z1, &z2, &z3, &z4] {
                            if cand.len() > 5 {
                                continue;
                            }
                            let c = count_polyline_collisions(
                                cand,
                                &rel_obstacles,
                                (fx, fy, fw, fh),
                                (tx, ty, tw, th),
                            );
                            if c < best_col {
                                best_col = c;
                                best = Some(cand.clone());
                                if c == 0 {
                                    break 'waypoint_loop;
                                }
                            }
                        }
                    }
                    best.unwrap_or_else(|| l_pts.to_vec())
                };

                let pts = best_pts.as_slice();
                let pts_str: String = pts
                    .iter()
                    .map(|(px, py)| format!("{},{}", px, py))
                    .collect::<Vec<_>>()
                    .join(" ");
                out.push_str(&format!(
                    "<polyline class=\"uml-relation puml-edge\" data-uml-from=\"{}\" data-uml-to=\"{}\" {} data-uml-arrow=\"{}\" points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}{}{} />",
                    escape_text(&from_name), escape_text(&to_name), puml_edge_attrs,
                    escape_text(&normalized_arrow), pts_str, relation_color, stroke_width,
                    dash_attr, visibility_attr, direction_attr, markers
                ));

                let target_is_deployment_data_store = family == "deployment"
                    && doc.nodes.iter().any(|node| {
                        node.name == to_name
                            && matches!(
                                node.kind,
                                FamilyNodeKind::Database | FamilyNodeKind::Storage
                            )
                    });
                let mut label_segments: Vec<&[(i32, i32)]> = pts.windows(2).collect();
                if target_is_deployment_data_store && label_segments.len() > 1 {
                    label_segments.pop();
                }
                let max_sq_len = label_segments
                    .iter()
                    .map(|seg| {
                        let (ax, ay) = seg[0];
                        let (bx, by_) = seg[1];
                        (bx - ax).pow(2) + (by_ - ay).pow(2)
                    })
                    .max()
                    .unwrap_or(0);
                let uniquely_longest = label_segments
                    .iter()
                    .filter(|seg| {
                        let (ax, ay) = seg[0];
                        let (bx, by_) = seg[1];
                        (bx - ax).pow(2) + (by_ - ay).pow(2) == max_sq_len
                    })
                    .count()
                    == 1;
                let (lmx, lmy) = if uniquely_longest {
                    let seg = label_segments
                        .into_iter()
                        .find(|seg| {
                            let (ax, ay) = seg[0];
                            let (bx, by_) = seg[1];
                            (bx - ax).pow(2) + (by_ - ay).pow(2) == max_sq_len
                        })
                        .unwrap();
                    ((seg[0].0 + seg[1].0) / 2, (seg[0].1 + seg[1].1) / 2 - 12)
                } else {
                    ((x1 + x2) / 2, (y1 + y2) / 2 - 12)
                };
                label_mx = lmx;
                label_my = lmy;
            }
        }

        if rel.left_lollipop {
            render_lollipop_endpoint(out, x1, y1, relation_color);
        }
        if rel.right_lollipop {
            render_lollipop_endpoint(out, x2, y2, relation_color);
        }
        if let Some(stereotype) = &rel.stereotype {
            let sx = (x1 + x2) / 2;
            let sy = (y1 + y2) / 2 - if rel.label.is_some() { 20 } else { 6 };
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"#475569\">&lt;&lt;{}&gt;&gt;</text>",
                sx, sy, escape_text(stereotype)
            ));
        }
        if let Some(label) = &rel.label {
            let label_text = usecase_dependency_label(Some(label)).unwrap_or(label);
            pending_labels.push(BoxGridPendingLabel {
                x: label_mx,
                y: label_my,
                text: label_text.to_string(),
                color: label_color.to_string(),
                edge_id: edge_id.clone(),
                from_name: from_name.clone(),
                to_name: to_name.clone(),
            });
        }
        if let Some(left) = &rel.left_cardinality {
            out.push_str(&format!(
                "<text class=\"puml-label\" {} x=\"{}\" y=\"{}\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"10\" fill=\"#334155\">{}</text>",
                text_semantic_attrs(&edge_id, "cardinality", x1 - 4, y1 - 6, left, 10, false),
                x1 - 4, y1 - 6, escape_text(left)
            ));
        }
        if let Some(right) = &rel.right_cardinality {
            out.push_str(&format!(
                "<text class=\"puml-label\" {} x=\"{}\" y=\"{}\" text-anchor=\"start\" font-family=\"monospace\" font-size=\"10\" fill=\"#334155\">{}</text>",
                text_semantic_attrs(&edge_id, "cardinality", x2 + 4, y2 - 6, right, 10, false),
                x2 + 4, y2 - 6, escape_text(right)
            ));
        }
        if let Some(left_role) = &rel.left_role {
            out.push_str(&format!(
                "<text class=\"puml-label\" {} x=\"{}\" y=\"{}\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"10\" fill=\"#334155\">{}</text>",
                text_semantic_attrs(&edge_id, "role", x1 - 4, y1 + 12, left_role, 10, false),
                x1 - 4, y1 + 12, escape_text(left_role)
            ));
        }
        if let Some(right_role) = &rel.right_role {
            out.push_str(&format!(
                "<text class=\"puml-label\" {} x=\"{}\" y=\"{}\" text-anchor=\"start\" font-family=\"monospace\" font-size=\"10\" fill=\"#334155\">{}</text>",
                text_semantic_attrs(&edge_id, "role", x2 + 4, y2 + 12, right_role, 10, false),
                x2 + 4, y2 + 12, escape_text(right_role)
            ));
        }
    }

    // ── Phase 3: De-collide edge labels ──────────────────────────────────────
    const LABEL_FAN_H_GAP: i32 = 85;
    const LABEL_CLUSTER_BAND: i32 = 18;
    const LABEL_CLEARANCE_X: i32 = 10;
    const LABEL_CLEARANCE_Y: i32 = 10;
    const LABEL_TEXT_HALF_HEIGHT: i32 = 8;

    let mut by_target: std::collections::BTreeMap<String, Vec<usize>> =
        std::collections::BTreeMap::new();
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

    // Fan remaining labels in the same y-band horizontally.
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
                let left_half_w = ((left.text.chars().count() as i32) * 7 + 2) / 2;
                let right_half_w = ((right.text.chars().count() as i32) * 7 + 2) / 2;
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

    // Final obstacle-clearance pass: push labels out of any node box they overlap.
    let label_overlaps_box =
        |lx: i32, ly: i32, text: &str, (bx, by, bw, bh): (i32, i32, i32, i32)| {
            let half_w = ((text.chars().count() as i32) * 7 + 2) / 2;
            lx + half_w + LABEL_CLEARANCE_X >= bx
                && lx - half_w - LABEL_CLEARANCE_X <= bx + bw
                && ly + 4 + LABEL_CLEARANCE_Y >= by
                && ly - LABEL_TEXT_HALF_HEIGHT - LABEL_CLEARANCE_Y <= by + bh
        };
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
            let half_w = ((text.chars().count() as i32) * 7 + 2) / 2;
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

    // Fill any None slots with the original position and emit all labels.
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
    for (idx, entry) in adjusted_labels.into_iter().enumerate() {
        let Some(entry) = entry else {
            continue;
        };
        let (lx, ly, text, color) = entry;
        let owner = &pending_labels[idx].edge_id;
        out.push_str(&format!(
            "<text class=\"puml-label\" {} x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
            text_semantic_attrs(owner, "edge-label", lx, ly, &text, 11, true),
            lx, ly, escape_text(&color), escape_text(&text)
        ));
    }
}
