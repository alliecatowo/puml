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
    // Map source name → list of relation indices that leave from it. Used by
    // the spline-native router (#1391) to assign tangent-angle jitter so that
    // hub-and-spoke fans diverge at the source port instead of sharing a
    // common stub.
    let mut src_fan_groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    // Map target name → list of relation indices that arrive at it. Used by
    // the spline-native router for many-to-one converge tangent jitter.
    let mut tgt_fan_groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
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
            .entry((to_name.clone(), x2, y2))
            .or_default()
            .push(rel_idx);
        src_fan_groups
            .entry(from_name.clone())
            .or_default()
            .push(rel_idx);
        tgt_fan_groups.entry(to_name).or_default().push(rel_idx);
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

    // ── Spline-native tangent jitter (#1391) ─────────────────────────────────
    // For each relation that leaves from a node with ≥2 outgoing edges, assign
    // a small angular jitter (radians) so the spline curves fan out at the
    // source port instead of overlapping. Mirror for converge fans at target.
    //
    // The jitter is symmetric around 0, spaced ~`SPLINE_FAN_STEP_RAD` per lane,
    // capped at ±`SPLINE_FAN_MAX_RAD` so the tangent never rotates past the
    // adjacent boundary side (~π/3 ≈ 60°).
    const SPLINE_FAN_STEP_RAD: f64 = 0.22; // ≈ 12.5° per lane
    const SPLINE_FAN_MAX_RAD: f64 = 0.7; // ≈ 40° absolute cap
    let mut spline_src_jitter: BTreeMap<usize, f64> = BTreeMap::new();
    let mut spline_tgt_jitter: BTreeMap<usize, f64> = BTreeMap::new();
    for group in src_fan_groups.values() {
        if group.len() <= 1 {
            continue;
        }
        let n = group.len() as f64;
        // Sort by rel_idx already (BTreeMap insertion order from sequential loop).
        for (slot, &rel_idx) in group.iter().enumerate() {
            let lane = slot as f64 - (n - 1.0) / 2.0;
            let theta = (lane * SPLINE_FAN_STEP_RAD).clamp(-SPLINE_FAN_MAX_RAD, SPLINE_FAN_MAX_RAD);
            spline_src_jitter.insert(rel_idx, theta);
        }
    }
    for group in tgt_fan_groups.values() {
        if group.len() <= 1 {
            continue;
        }
        let n = group.len() as f64;
        for (slot, &rel_idx) in group.iter().enumerate() {
            let lane = slot as f64 - (n - 1.0) / 2.0;
            // Reverse sign at the target end so the converging curves mirror
            // the diverging-source pattern symmetrically.
            let theta =
                -(lane * SPLINE_FAN_STEP_RAD).clamp(-SPLINE_FAN_MAX_RAD, SPLINE_FAN_MAX_RAD);
            spline_tgt_jitter.insert(rel_idx, theta);
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
        let fan_offset: (i32, i32) = if rel.direction.is_none() && !rel.hidden {
            port_fan_offset.get(&rel_idx).copied().unwrap_or((0, 0))
        } else {
            (0, 0)
        };
        x2 += fan_offset.0;
        y2 += fan_offset.1;

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
                // For interface circle targets, the endpoint was already adjusted by
                // adjust_interface_anchor (and re-set in the interface-alignment block
                // above). Never apply fan offset to interface circles — it would move
                // the endpoint off the circle edge and break precision assertions.
                let is_interface_tgt = interface_nodes.contains(&to_name);
                let snapped_x = if tgt_keep_routed_x && !is_interface_tgt {
                    // Top/bottom entry into a regular component: apply fan offset along x
                    // so parallel edges with the same target port get distinct x.
                    (last.0 + fan_offset.0).clamp(tgt_x_min, tgt_x_max)
                } else if tgt_keep_routed_x {
                    // Interface circle: use router x, no fan offset.
                    last.0.clamp(tgt_x_min, tgt_x_max)
                } else {
                    x2 // x2 already has fan_offset.0 applied
                };
                // When entering top/bottom keep the router's y (the true bbox edge);
                // fall back to pick_port's y only for left/right horizontal entry.
                // Exception: interface circle nodes use an anchor adjusted to the
                // circle edge via adjust_interface_anchor; always honour that y2.
                let snapped_y = if tgt_keep_routed_x && !is_interface_tgt {
                    // Top/bottom entry into regular component: keep the router's y
                    // (the true bbox edge). The fan offset along y is only valid when
                    // pick_port chose a side port; if the router arrived at top/bottom
                    // instead, applying fan_offset.1 would push the endpoint off the
                    // bbox edge and break precision assertions.
                    last.1
                } else {
                    y2 // y2 already has fan_offset.1 applied (or is interface-adjusted)
                };
                *last = (snapped_x, snapped_y);
            }
            // Perpendicular stub enforcement.  After snapping the endpoint
            // anchors above (including any port-fan lateral shift), the
            // first segment must be perpendicular to the source side and
            // the last must be perpendicular to the target side.  Align
            // the off-axis coordinate of the second/penultimate waypoint
            // to the (possibly fan-shifted) endpoint:
            //
            //   top/bottom side (src/tgt_keep_routed_x = true): outward
            //     normal is vertical → stub must be vertical → match x.
            //   left/right side (src/tgt_keep_routed_x = false): outward
            //     normal is horizontal → stub must be horizontal → match y.
            //
            // Without this, fan-shifting an endpoint laterally leaves the
            // router-emitted penultimate at the un-shifted axis, producing
            // a diagonal stub where the arrowhead "grazes" the box border
            // instead of pointing into the side at 90°.
            //
            // For n == 3 the single interior waypoint must satisfy both the
            // source-side and target-side stub.  When the two endpoints sit
            // on the same axis (both vertical exit/entry with different x, or
            // both horizontal with different y), one corner cannot satisfy
            // both constraints — insert an extra waypoint so we get a
            // U-shaped bend.  When the two sides are perpendicular axes, the
            // single interior point becomes the natural L-corner.
            let n = orth_pts.len();
            if n == 3 {
                let (p0x, p0y) = orth_pts[0];
                let (p2x, p2y) = orth_pts[2];
                match (src_keep_routed_x, tgt_keep_routed_x) {
                    // Both vertical stubs: require pts[1].x = p0.x AND
                    // pts[2-1=1].x = p2.x.  If endpoint xs differ, inject a
                    // mid-y bend at the existing pts[1].y.
                    (true, true) => {
                        if p0x == p2x {
                            orth_pts[1].0 = p0x;
                        } else {
                            let bend_y = orth_pts[1].1;
                            orth_pts[1] = (p0x, bend_y);
                            orth_pts.insert(2, (p2x, bend_y));
                        }
                    }
                    // Both horizontal stubs.
                    (false, false) => {
                        if p0y == p2y {
                            orth_pts[1].1 = p0y;
                        } else {
                            let bend_x = orth_pts[1].0;
                            orth_pts[1] = (bend_x, p0y);
                            orth_pts.insert(2, (bend_x, p2y));
                        }
                    }
                    // Source vertical + target horizontal: L-corner at
                    // (p0.x, p2.y).
                    (true, false) => {
                        orth_pts[1] = (p0x, p2y);
                    }
                    // Source horizontal + target vertical: L-corner at
                    // (p2.x, p0.y).
                    (false, true) => {
                        orth_pts[1] = (p2x, p0y);
                    }
                }
            } else if n > 3 {
                let (p0x, p0y) = orth_pts[0];
                let (pnx, pny) = orth_pts[n - 1];
                if src_keep_routed_x {
                    orth_pts[1].0 = p0x;
                } else {
                    orth_pts[1].1 = p0y;
                }
                if tgt_keep_routed_x {
                    orth_pts[n - 2].0 = pnx;
                } else {
                    orth_pts[n - 2].1 = pny;
                }
            }
            // ── Splines-mode dispatch (#1391) ────────────────────────────────
            // For `EdgeRouting::Splines`, attempt the spline-native generator
            // first. It produces a fundamentally different waypoint set: a
            // single sweeping cubic Bézier from source-anchor to target-anchor
            // with tangents perpendicular to the source/target boundary, NOT
            // an orthogonal-corner path with rounded chamfers. If the topology
            // is too complex (obstacle clutter, etc.), the generator returns
            // None and we fall back to the rounded-corner renderer.
            let mut emitted_spline = false;
            let mut spline_label_point: Option<(i32, i32)> = None;
            if matches!(
                doc.edge_routing,
                crate::render::graph_layout::EdgeRouting::Splines
            ) && orth_pts.len() >= 2
            {
                use crate::render::graph_layout::spline_router::{
                    generate_spline_path, tangent_from_bbox_side, SplinePathInput,
                };
                let (sx, sy) = orth_pts[0];
                let (tx_, ty_) = orth_pts[orth_pts.len() - 1];
                let src_anchor = (sx as f64, sy as f64);
                let tgt_anchor = (tx_ as f64, ty_ as f64);
                let src_bbox = (fx as f64, fy as f64, fw as f64, fh as f64);
                let tgt_bbox = (tx as f64, ty as f64, tw as f64, th as f64);
                // Tangent direction: prefer bbox-side normal (clean outward
                // direction); fall back to the orthogonal path's first
                // segment direction when the anchor doesn't sit cleanly on a
                // side (e.g. interface circles).
                let src_tangent =
                    tangent_from_bbox_side(src_anchor, src_bbox, 4.0).unwrap_or_else(|| {
                        let (ax, ay) = orth_pts[0];
                        let (bx, by) = orth_pts[1];
                        let dx = (bx - ax) as f64;
                        let dy = (by - ay) as f64;
                        let len = dx.hypot(dy).max(1.0);
                        (dx / len, dy / len)
                    });
                // Target tangent points INTO the target (the curve enters the
                // target along this direction). Outward normal at the target
                // anchor, then negate, is equivalent to inward normal.
                let tgt_outward =
                    tangent_from_bbox_side(tgt_anchor, tgt_bbox, 4.0).unwrap_or_else(|| {
                        let n = orth_pts.len();
                        let (ax, ay) = orth_pts[n - 1];
                        let (bx, by) = orth_pts[n - 2];
                        let dx = (ax - bx) as f64;
                        let dy = (ay - by) as f64;
                        let len = dx.hypot(dy).max(1.0);
                        (dx / len, dy / len)
                    });
                let tgt_tangent = (-tgt_outward.0, -tgt_outward.1);
                // Obstacles: all sibling node bboxes except source/target.
                let obstacles: Vec<(f64, f64, f64, f64)> = all_boxes
                    .iter()
                    .filter(|&&b| b != (fx, fy, fw, fh) && b != (tx, ty, tw, th))
                    .map(|&(bx, by_, bw, bh)| (bx as f64, by_ as f64, bw as f64, bh as f64))
                    .collect();
                let input = SplinePathInput {
                    src_anchor,
                    tgt_anchor,
                    src_tangent,
                    tgt_tangent,
                    obstacles,
                    src_tangent_jitter: spline_src_jitter.get(&rel_idx).copied().unwrap_or(0.0),
                    tgt_tangent_jitter: spline_tgt_jitter.get(&rel_idx).copied().unwrap_or(0.0),
                };
                if let Some(spline) = generate_spline_path(input) {
                    let d = spline.to_svg_d();
                    out.push_str(&format!(
                        "<path class=\"uml-relation\" data-uml-from=\"{}\" data-uml-to=\"{}\" data-uml-arrow=\"{}\" d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}{}{}{} />",
                        escape_text(&from_name), escape_text(&to_name), escape_text(normalized_arrow.as_str()),
                        d, relation_color, stroke_width,
                        dash_attr, visibility_attr, direction_attr, style_attr, markers
                    ));
                    // Label position: arclength midpoint of the spline (pin
                    // label to the path's actual visual midpoint, not the
                    // orthogonal-waypoint midpoint, per #1352/#1366/#1387).
                    let (mx, my) = spline.point_at(0.5);
                    spline_label_point = Some((mx as i32, my as i32 - 12));
                    emitted_spline = true;
                }
            }

            if !emitted_spline {
                let (tag, geom_attr) =
                    crate::render::edge_smoothing::edge_geometry_attr(doc.edge_routing, &orth_pts);
                out.push_str(&format!(
                    "<{tag} class=\"uml-relation\" data-uml-from=\"{}\" data-uml-to=\"{}\" data-uml-arrow=\"{}\" {} fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}{}{}{} />",
                    escape_text(&from_name), escape_text(&to_name), escape_text(normalized_arrow.as_str()),
                    geom_attr, relation_color, stroke_width,
                    dash_attr, visibility_attr, direction_attr, style_attr, markers
                ));
            }

            let (lmx, lmy) = if let Some(p) = spline_label_point {
                p
            } else {
                let longest_seg = orth_pts.windows(2).max_by_key(|seg| {
                    let dx = (seg[1].0 - seg[0].0).abs();
                    let dy = (seg[1].1 - seg[0].1).abs();
                    dx.max(dy)
                });
                match longest_seg {
                    Some(seg) => {
                        let is_horiz = (seg[1].1 - seg[0].1).abs() <= (seg[1].0 - seg[0].0).abs();
                        let mx = (seg[0].0 + seg[1].0) / 2;
                        let my = (seg[0].1 + seg[1].1) / 2;
                        if is_horiz {
                            (mx, my - 12)
                        } else {
                            (mx + 12, my)
                        }
                    }
                    None => ((x1 + x2) / 2, (y1 + y2) / 2 - 12),
                }
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
                // Spline-mode dispatch (#1391): even an obstacle-free
                // straight line gets a curved spline emission so the visual
                // mode is consistent across the diagram. Otherwise: emit a
                // plain <line>.
                let mut spline_label: Option<(i32, i32)> = None;
                if matches!(
                    doc.edge_routing,
                    crate::render::graph_layout::EdgeRouting::Splines
                ) {
                    use crate::render::graph_layout::spline_router::{
                        generate_spline_path, tangent_from_bbox_side, SplinePathInput,
                    };
                    let src_anchor = (x1 as f64, y1 as f64);
                    let tgt_anchor = (x2 as f64, y2 as f64);
                    let src_bbox = (fx as f64, fy as f64, fw as f64, fh as f64);
                    let tgt_bbox = (tx as f64, ty as f64, tw as f64, th as f64);
                    let src_tangent =
                        tangent_from_bbox_side(src_anchor, src_bbox, 4.0).unwrap_or((0.0, 1.0));
                    let tgt_outward =
                        tangent_from_bbox_side(tgt_anchor, tgt_bbox, 4.0).unwrap_or((0.0, -1.0));
                    let tgt_tangent = (-tgt_outward.0, -tgt_outward.1);
                    let obstacles: Vec<(f64, f64, f64, f64)> = all_boxes
                        .iter()
                        .filter(|&&b| b != (fx, fy, fw, fh) && b != (tx, ty, tw, th))
                        .map(|&(bx, by_, bw, bh)| (bx as f64, by_ as f64, bw as f64, bh as f64))
                        .collect();
                    let input = SplinePathInput {
                        src_anchor,
                        tgt_anchor,
                        src_tangent,
                        tgt_tangent,
                        obstacles,
                        src_tangent_jitter: spline_src_jitter.get(&rel_idx).copied().unwrap_or(0.0),
                        tgt_tangent_jitter: spline_tgt_jitter.get(&rel_idx).copied().unwrap_or(0.0),
                    };
                    if let Some(spline) = generate_spline_path(input) {
                        let d = spline.to_svg_d();
                        out.push_str(&format!(
                            "<path class=\"uml-relation\" data-uml-from=\"{}\" data-uml-to=\"{}\" data-uml-arrow=\"{}\" d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}{}{}{} />",
                            escape_text(&from_name), escape_text(&to_name), escape_text(normalized_arrow.as_str()),
                            d, relation_color, stroke_width,
                            dash_attr, visibility_attr, direction_attr, style_attr, markers
                        ));
                        let (mx, my) = spline.point_at(0.5);
                        spline_label = Some((mx as i32, my as i32 - 12));
                    }
                }
                if let Some((lmx, lmy)) = spline_label {
                    label_mx = lmx;
                    label_my = lmy;
                } else {
                    out.push_str(&format!(
                        "<line class=\"uml-relation\" data-uml-from=\"{}\" data-uml-to=\"{}\" data-uml-arrow=\"{}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}{}{}{}{} />",
                        escape_text(&from_name), escape_text(&to_name), escape_text(normalized_arrow.as_str()),
                        x1, y1, x2, y2, relation_color, stroke_width,
                        dash_attr, visibility_attr, direction_attr, style_attr, markers
                    ));
                    label_mx = (x1 + x2) / 2;
                    label_my = (y1 + y2) / 2 - 12;
                }
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

                // Spline-mode dispatch (#1391) for the L/Z fallback branch.
                let mut emitted_spline_lz = false;
                if matches!(
                    doc.edge_routing,
                    crate::render::graph_layout::EdgeRouting::Splines
                ) && pts.len() >= 2
                {
                    use crate::render::graph_layout::spline_router::{
                        generate_spline_path, tangent_from_bbox_side, SplinePathInput,
                    };
                    let (sx, sy) = pts[0];
                    let (tx_, ty_) = pts[pts.len() - 1];
                    let src_anchor = (sx as f64, sy as f64);
                    let tgt_anchor = (tx_ as f64, ty_ as f64);
                    let src_bbox = (fx as f64, fy as f64, fw as f64, fh as f64);
                    let tgt_bbox = (tx as f64, ty as f64, tw as f64, th as f64);
                    let src_tangent = tangent_from_bbox_side(src_anchor, src_bbox, 4.0)
                        .unwrap_or_else(|| {
                            let (ax, ay) = pts[0];
                            let (bx, by) = pts[1];
                            let dx = (bx - ax) as f64;
                            let dy = (by - ay) as f64;
                            let len = dx.hypot(dy).max(1.0);
                            (dx / len, dy / len)
                        });
                    let tgt_outward = tangent_from_bbox_side(tgt_anchor, tgt_bbox, 4.0)
                        .unwrap_or_else(|| {
                            let n = pts.len();
                            let (ax, ay) = pts[n - 1];
                            let (bx, by) = pts[n - 2];
                            let dx = (ax - bx) as f64;
                            let dy = (ay - by) as f64;
                            let len = dx.hypot(dy).max(1.0);
                            (dx / len, dy / len)
                        });
                    let tgt_tangent = (-tgt_outward.0, -tgt_outward.1);
                    let obstacles: Vec<(f64, f64, f64, f64)> = rel_obstacles
                        .iter()
                        .filter(|&&b| b != (fx, fy, fw, fh) && b != (tx, ty, tw, th))
                        .map(|&(bx, by_, bw, bh)| (bx as f64, by_ as f64, bw as f64, bh as f64))
                        .collect();
                    let input = SplinePathInput {
                        src_anchor,
                        tgt_anchor,
                        src_tangent,
                        tgt_tangent,
                        obstacles,
                        src_tangent_jitter: spline_src_jitter.get(&rel_idx).copied().unwrap_or(0.0),
                        tgt_tangent_jitter: spline_tgt_jitter.get(&rel_idx).copied().unwrap_or(0.0),
                    };
                    if let Some(spline) = generate_spline_path(input) {
                        let d = spline.to_svg_d();
                        out.push_str(&format!(
                            "<path class=\"uml-relation\" data-uml-from=\"{}\" data-uml-to=\"{}\" data-uml-arrow=\"{}\" d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}{}{} />",
                            escape_text(&from_name), escape_text(&to_name), escape_text(normalized_arrow.as_str()),
                            d, relation_color, stroke_width,
                            dash_attr, visibility_attr, direction_attr, markers
                        ));
                        emitted_spline_lz = true;
                    }
                }
                if !emitted_spline_lz {
                    let (tag, geom_attr) =
                        crate::render::edge_smoothing::edge_geometry_attr(doc.edge_routing, pts);
                    out.push_str(&format!(
                        "<{tag} class=\"uml-relation\" data-uml-from=\"{}\" data-uml-to=\"{}\" data-uml-arrow=\"{}\" {} fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}{}{} />",
                        escape_text(&from_name), escape_text(&to_name), escape_text(normalized_arrow.as_str()),
                        geom_attr, relation_color, stroke_width,
                        dash_attr, visibility_attr, direction_attr, markers
                    ));
                }

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
