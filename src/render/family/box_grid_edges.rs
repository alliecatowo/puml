use std::collections::{BTreeMap, BTreeSet};

use crate::model::{FamilyDocument, FamilyNodeKind};
use crate::render::svg::escape_text;
use crate::theme::ComponentStyle;

use super::box_grid_labels::{render_box_grid_relation_labels, BoxGridPendingLabel};

#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)] // pkg_frame_boxes pairs a rect tuple with its member names; a named struct would be overkill for an internal helper
pub(super) fn render_box_grid_relations_and_labels(
    out: &mut String,
    doc: &FamilyDocument,
    family: &str,
    positions: &BTreeMap<String, (i32, i32, i32, i32)>,
    interface_nodes: &BTreeSet<String>,
    all_boxes: &[(i32, i32, i32, i32)],
    pkg_frame_boxes: &[((i32, i32, i32, i32), &[String])],
    edge_paths: &BTreeMap<String, Vec<(f64, f64)>>,
    comp_style: &ComponentStyle,
) {
    use crate::render::geometry::{compute_edge_anchors_for_direction, pick_port};
    use crate::render::relation::{
        arrow_style, normalize_relation_endpoints, render_lollipop_endpoint,
        render_relation_marker_defs_with_prefix, usecase_dependency_label,
    };

    let mut pending_labels: Vec<BoxGridPendingLabel> = Vec::new();

    // ── Parallel-edge port fan (#1374) ────────────────────────────────────────
    // When multiple relations share the same (to_name, port_x, port_y) arrival
    // point, they overlap and appear as a single edge. Pre-compute a small fan
    // offset per relation index so each edge arrives at a distinct port position.
    //
    // Fan direction: left/right ports (x = tx or tx+tw) → fan along y.
    //                top/bottom ports (y = ty or ty+th)  → fan along x.
    const PORT_FAN_SPACING: i32 = 10; // pixels between fan lanes
    const PORT_FAN_MAX: i32 = 20; // max shift from center port

    // Map (to_name, port_x, port_y) → list of relation indices with that port.
    let mut tgt_port_groups: BTreeMap<(String, i32, i32), Vec<usize>> = BTreeMap::new();
    for (rel_idx, rel) in doc.relations.iter().enumerate() {
        if rel.direction.is_some() || rel.hidden {
            continue;
        }
        let (from_name, to_name, _arrow) =
            normalize_relation_endpoints(&rel.from, &rel.to, &rel.arrow);
        if from_name == to_name {
            continue;
        }
        let Some(&(fx, fy, fw, fh)) = positions.get(&from_name) else {
            continue;
        };
        let Some(&(tx, ty, tw, th)) = positions.get(&to_name) else {
            continue;
        };
        let (_, _, x2, y2) = pick_port((fx, fy, fw, fh), (tx, ty, tw, th));
        tgt_port_groups
            .entry((to_name, x2, y2))
            .or_default()
            .push(rel_idx);
    }

    // rel_idx → (dx, dy) fan offset to apply to (x2, y2).
    let mut port_fan_offset: BTreeMap<usize, (i32, i32)> = BTreeMap::new();
    for ((to_name, port_x, _port_y), group) in &tgt_port_groups {
        if group.len() <= 1 {
            continue;
        }
        let n = group.len() as i32;
        // Determine fan axis from which side of the target the port is on.
        let Some(&(tgt_x, _tgt_y, tgt_w, tgt_h)) = positions.get(to_name) else {
            continue;
        };
        // Port is on the left or right edge → fan along y.
        // Port is on the top or bottom edge → fan along x.
        let port_on_side = *port_x == tgt_x || *port_x == tgt_x + tgt_w;
        for (slot, &rel_idx) in group.iter().enumerate() {
            let lane = slot as i32 - (n - 1) / 2;
            let shift = (lane * PORT_FAN_SPACING).clamp(-PORT_FAN_MAX, PORT_FAN_MAX);
            let (dx, dy) = if port_on_side {
                // Port is on left or right edge → fan along y.
                (0, shift)
            } else {
                // Port is on top or bottom edge → fan along x.
                (shift, 0)
            };
            // Clamp fan offset so the fanned point stays within the target side.
            let dy_clamped = dy.clamp(-(tgt_h / 2 - 4), tgt_h / 2 - 4);
            let dx_clamped = dx.clamp(-(tgt_w / 2 - 4), tgt_w / 2 - 4);
            port_fan_offset.insert(rel_idx, (dx_clamped, dy_clamped));
        }
    }
    // ─────────────────────────────────────────────────────────────────────────

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

        // Apply parallel-edge port fan offset so edges with the same target
        // port arrive at distinct positions and are each visually visible (#1374).
        if rel.direction.is_none() && !rel.hidden {
            if let Some(&(fdx, fdy)) = port_fan_offset.get(&rel_idx) {
                x2 += fdx;
                y2 += fdy;
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
        let mut label_edge_points: Vec<(i32, i32)> = vec![(x1, y1), (x2, y2)];

        let ortho_path: Option<Vec<(i32, i32)>> = if rel.direction.is_none() && !rel.hidden {
            edge_paths
                .get(&format!("r{rel_idx}"))
                .filter(|p| p.len() >= 2)
                .map(|p| p.iter().map(|&(px, py)| (px as i32, py as i32)).collect())
        } else {
            None
        };

        // If the orthogonal router leaves/enters on a different side than
        // `pick_port` chose, align anchors to the path's first/last segment.
        if !interface_nodes.is_empty() {
            if let Some(ref orth_pts) = ortho_path {
                if orth_pts.len() >= 2 {
                    let (p0x, p0y) = orth_pts[0];
                    let (p1x, p1y) = orth_pts[1];
                    if !interface_nodes.contains(&from_name) {
                        if p1x == p0x {
                            x1 = p1x.clamp(fx, fx + fw);
                            y1 = if p1y < p0y { fy } else { fy + fh };
                        } else if p1y == p0y {
                            y1 = p1y.clamp(fy, fy + fh);
                            x1 = if p1x < p0x { fx } else { fx + fw };
                        }
                    }
                    let n = orth_pts.len();
                    let (pnx, pny) = orth_pts[n - 1];
                    let (pmx, pmy) = orth_pts[n - 2];
                    if interface_nodes.contains(&to_name) {
                        // For interface circles, snap endpoint to the actual circle
                        // edge using the router path's entry direction (not the
                        // component-center vector, which may pick the wrong side when
                        // dx == dy).
                        const IR: i32 = 18;
                        let cx = tx + tw / 2;
                        let cy = ty + th / 2;
                        if pmx == pnx {
                            // Vertical entry: snap to top or bottom of circle.
                            x2 = cx;
                            y2 = if pmy < pny { cy - IR } else { cy + IR };
                        } else if pmy == pny {
                            // Horizontal entry: snap to left or right of circle.
                            x2 = if pmx < pnx { cx - IR } else { cx + IR };
                            y2 = cy;
                        }
                    } else if pmx == pnx {
                        x2 = pmx.clamp(tx, tx + tw);
                        y2 = if pmy < pny { ty } else { ty + th };
                    } else if pmy == pny {
                        y2 = pmy.clamp(ty, ty + th);
                        x2 = if pmx < pnx { tx } else { tx + tw };
                    }
                }
            }
        }

        if let Some(mut orth_pts) = ortho_path {
            // Endpoint anchors can land on visual top/bottom edges that differ
            // from bbox extents for 3D deployment shapes; use a tolerance.
            //
            // #1327: Use the router's own endpoint y to detect top/bottom entry/exit.
            // pick_port may return center-y for horizontal-dominant edges (when the
            // horizontal displacement between source and target centers is larger than
            // the vertical displacement).  In that case y1/y2 land at the vertical
            // midpoint of the node, which is never within 16 px of the top or bottom
            // edge, causing tgt_keep_routed_x = false and the snapping to replace the
            // router's correctly-computed component-midpoint x with pick_port's
            // left/right-edge x.  Reading the router's own first/last waypoint y
            // correctly identifies top/bottom entry regardless of pick_port direction.
            let first_routed_y = orth_pts.first().map(|&(_, y)| y).unwrap_or(y1);
            let last_routed_y = orth_pts.last().map(|&(_, y)| y).unwrap_or(y2);
            let src_keep_routed_x =
                (first_routed_y - fy).abs() <= 16 || (first_routed_y - (fy + fh)).abs() <= 16;
            let tgt_keep_routed_x =
                (last_routed_y - ty).abs() <= 16 || (last_routed_y - (ty + th)).abs() <= 16;
            let src_x_min = fx.min(fx + fw);
            let src_x_max = fx.max(fx + fw);
            let tgt_x_min = tx.min(tx + tw);
            let tgt_x_max = tx.max(tx + tw);

            if let Some(first) = orth_pts.first_mut() {
                let snapped_x = if src_keep_routed_x {
                    first.0.clamp(src_x_min, src_x_max)
                } else {
                    x1
                };
                // When entering top/bottom keep the router's y (the true bbox edge);
                // fall back to pick_port's y only for left/right horizontal entry.
                let snapped_y = if src_keep_routed_x { first.1 } else { y1 };
                *first = (snapped_x, snapped_y);
            }
            if let Some(last) = orth_pts.last_mut() {
                let snapped_x = if tgt_keep_routed_x {
                    last.0.clamp(tgt_x_min, tgt_x_max)
                } else {
                    x2
                };
                // When entering top/bottom keep the router's y (the true bbox edge);
                // fall back to pick_port's y only for left/right horizontal entry.
                // Exception: interface circle nodes use an anchor adjusted to the
                // circle edge via adjust_interface_anchor; always honour that y2.
                let snapped_y = if tgt_keep_routed_x && !interface_nodes.contains(&to_name) {
                    last.1
                } else {
                    y2
                };
                *last = (snapped_x, snapped_y);
            }
            let n = orth_pts.len();
            if n >= 3 {
                if !src_keep_routed_x {
                    orth_pts[1].0 = x1;
                }
                if n > 3 && !tgt_keep_routed_x {
                    orth_pts[n - 2].0 = x2;
                }
            }
            let (tag, geom_attr) =
                crate::render::edge_smoothing::edge_geometry_attr(doc.edge_routing, &orth_pts);
            out.push_str(&format!(
                "<{tag} class=\"uml-relation\" data-uml-from=\"{}\" data-uml-to=\"{}\" data-uml-arrow=\"{}\" {} fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}{}{}{} />",
                escape_text(&from_name), escape_text(&to_name), escape_text(normalized_arrow.as_str()),
                geom_attr, relation_color, stroke_width,
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
            label_edge_points = orth_pts;
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
                    "<line class=\"uml-relation\" data-uml-from=\"{}\" data-uml-to=\"{}\" data-uml-arrow=\"{}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}{}{}{}{} />",
                    escape_text(&from_name), escape_text(&to_name), escape_text(normalized_arrow.as_str()),
                    x1, y1, x2, y2, relation_color, stroke_width,
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
                let (tag, geom_attr) =
                    crate::render::edge_smoothing::edge_geometry_attr(doc.edge_routing, pts);
                out.push_str(&format!(
                    "<{tag} class=\"uml-relation\" data-uml-from=\"{}\" data-uml-to=\"{}\" data-uml-arrow=\"{}\" {} fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}{}{} />",
                    escape_text(&from_name), escape_text(&to_name), escape_text(normalized_arrow.as_str()),
                    geom_attr, relation_color, stroke_width,
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
                label_edge_points = best_pts;
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
                from_name: from_name.clone(),
                to_name: to_name.clone(),
                edge_points: label_edge_points,
            });
        }
        if let Some(left) = &rel.left_cardinality {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"10\" fill=\"#334155\">{}</text>",
                x1 - 4, y1 - 6, escape_text(left)
            ));
        }
        if let Some(right) = &rel.right_cardinality {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"start\" font-family=\"monospace\" font-size=\"10\" fill=\"#334155\">{}</text>",
                x2 + 4, y2 - 6, escape_text(right)
            ));
        }
        if let Some(left_role) = &rel.left_role {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"10\" fill=\"#334155\">{}</text>",
                x1 - 4, y1 + 12, escape_text(left_role)
            ));
        }
        if let Some(right_role) = &rel.right_role {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"start\" font-family=\"monospace\" font-size=\"10\" fill=\"#334155\">{}</text>",
                x2 + 4, y2 + 12, escape_text(right_role)
            ));
        }
    }

    render_box_grid_relation_labels(out, positions, pending_labels);
}

/// Check if a line segment (x1,y1)→(x2,y2) intersects a rectangle (bx,by,bw,bh).
/// Uses the parametric Liang–Barsky / separating axis test.
fn segment_intersects_rect(
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    (bx, by, bw, bh): (i32, i32, i32, i32),
) -> bool {
    // Grow the box by 4px to give a small margin
    let margin = 4;
    let rx0 = bx - margin;
    let ry0 = by - margin;
    let rx1 = bx + bw + margin;
    let ry1 = by + bh + margin;

    // Parametric: P(t) = (x1 + t*dx, y1 + t*dy)
    let dx = (x2 - x1) as f64;
    let dy = (y2 - y1) as f64;
    let mut tmin = 0.0f64;
    let mut tmax = 1.0f64;

    // Clip against left/right
    if dx.abs() < 1e-9 {
        // Vertical segment
        if (x1 < rx0) || (x1 > rx1) {
            return false;
        }
    } else {
        let t1 = (rx0 as f64 - x1 as f64) / dx;
        let t2 = (rx1 as f64 - x1 as f64) / dx;
        let (t_lo, t_hi) = if t1 < t2 { (t1, t2) } else { (t2, t1) };
        tmin = tmin.max(t_lo);
        tmax = tmax.min(t_hi);
        if tmin > tmax {
            return false;
        }
    }

    // Clip against top/bottom
    if dy.abs() < 1e-9 {
        if (y1 < ry0) || (y1 > ry1) {
            return false;
        }
    } else {
        let t1 = (ry0 as f64 - y1 as f64) / dy;
        let t2 = (ry1 as f64 - y1 as f64) / dy;
        let (t_lo, t_hi) = if t1 < t2 { (t1, t2) } else { (t2, t1) };
        tmin = tmin.max(t_lo);
        tmax = tmax.min(t_hi);
        if tmin > tmax {
            return false;
        }
    }

    // The intersection is within the segment if tmin ≤ tmax and tmin < 1 and tmax > 0
    tmin < tmax && tmax > 0.01 && tmin < 0.99
}

/// Count how many obstacles (excluding src/tgt) are intersected by a polyline.
fn count_polyline_collisions(
    pts: &[(i32, i32)],
    all_boxes: &[(i32, i32, i32, i32)],
    src: (i32, i32, i32, i32),
    tgt: (i32, i32, i32, i32),
) -> usize {
    let mut count = 0;
    for seg in pts.windows(2) {
        let (ax, ay) = seg[0];
        let (bx_, by_) = seg[1];
        for &(obx, oby, obw, obh) in all_boxes {
            if (obx, oby, obw, obh) == src || (obx, oby, obw, obh) == tgt {
                continue;
            }
            if segment_intersects_rect(ax, ay, bx_, by_, (obx, oby, obw, obh)) {
                count += 1;
            }
        }
    }
    count
}
