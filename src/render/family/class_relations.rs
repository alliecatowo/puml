use std::collections::BTreeMap;

use crate::model::FamilyRelationEndpointMarker;
use crate::render::geometry::{compute_edge_anchors_for_direction, pick_port};
use crate::render::graph_layout::spline_router::{
    generate_spline_path, tangent_from_bbox_side, SplinePathInput,
};
use crate::render::relation::{
    arrow_style, normalize_relation_endpoints, render_lollipop_endpoint, usecase_dependency_label,
};
use crate::render::svg::escape_text;

use super::class_relation_labels::{relation_label_svg, resolve_relation_endpoint_key};
use super::class_routing::{
    class_box_anchor_toward_point, class_nudge_label_x, class_nudge_label_y,
    class_port_side_from_box_anchor, class_route_with_row_ports, qualified_row_anchor,
};
use super::class_types::{ClassEndpointAnchor, ClassNodeBox, ClassPortSide};
use super::group_frames::ClassGroupFrameRect;

/// Context passed to `render_class_relations` — groups the many read-only
/// inputs that the relation-rendering loop needs from `render_class_svg`.
pub(super) struct ClassRelationCtx<'a> {
    pub(super) relations: &'a [crate::model::FamilyRelation],
    pub(super) nodes: &'a [crate::model::FamilyNode],
    pub(super) node_boxes: &'a BTreeMap<String, ClassNodeBox>,
    pub(super) edge_paths: &'a BTreeMap<String, Vec<(f64, f64)>>,
    pub(super) label_override: &'a BTreeMap<usize, (i32, i32)>,
    pub(super) parallel_offset: &'a BTreeMap<usize, i32>,
    pub(super) relation_pair_label_lanes: &'a BTreeMap<usize, i32>,
    pub(super) relation_source_label_lanes: &'a BTreeMap<usize, i32>,
    pub(super) class_style: &'a crate::theme::ClassStyle,
    pub(super) arrow_stroke: &'a str,
    pub(super) margin_x: i32,
    pub(super) margin_top: i32,
    /// Top y-coordinate where diagram content (nodes) begins.
    /// Equal to margin_top + title_block_height + group_top_reserve.
    /// Used to clamp edge labels so they don't escape into the title band.
    pub(super) canvas_margin_y: i32,
    pub(super) svg_width: i32,
    /// True when every node is an `Object` (object diagram).  In object diagrams
    /// relation labels are expected to stay near the edge midpoint (centred on
    /// the vertical line), so the box-clearance x-nudge is suppressed.
    pub(super) is_object_diagram: bool,
    /// Global edge-routing mode (mirrors PlantUML's `skinparam linetype`).
    /// Selects between cubic-Bézier `<path>` emission ([`EdgeRouting::Splines`])
    /// and straight-segment `<polyline>` emission
    /// ([`EdgeRouting::Polyline`] / [`EdgeRouting::Ortho`]).
    pub(super) edge_routing: crate::render::graph_layout::EdgeRouting,
    /// True for usecase diagrams -- enables actor-specific port overrides and
    /// frame-boundary edge snapping (#1291, #1292).
    pub(super) is_usecase_layout: bool,
    /// Computed group-frame rectangles (system-boundary `rectangle` groups in
    /// usecase diagrams).  Used for frame-boundary entry/exit snapping (#1292).
    pub(super) group_frame_rects: Vec<ClassGroupFrameRect>,
}

// #1291: actor-generalization port override

/// Returns true when the given node kind is a usecase actor shape (stick
/// figure).  Used to apply actor-specific port overrides (#1291).
fn is_actor_kind(kind: crate::model::FamilyNodeKind) -> bool {
    matches!(
        kind,
        crate::model::FamilyNodeKind::Actor | crate::model::FamilyNodeKind::BusinessActor
    )
}

/// For usecase diagrams: when both endpoints are Actor nodes AND the relation
/// carries a hollow-triangle (generalization) marker, override the port
/// selection to use vertical ports only (bottom of parent to top of child).
///
/// Actor stick figures look wrong when generalization edges exit from the side
/// because the connection appears to pierce the stickman's body; vertical
/// routing (feet to head or head to feet) is always cleaner (#1291).
fn actor_generalization_pick_port(
    from: &ClassNodeBox,
    to: &ClassNodeBox,
    normalized_arrow: &crate::model::FamilyRelationArrow,
    nodes: &[crate::model::FamilyNode],
    from_name: &str,
    to_name: &str,
) -> Option<(i32, i32, i32, i32)> {
    // Only override for generalization (hollow triangle) relations.
    let has_triangle = matches!(
        normalized_arrow.start_marker(),
        Some(FamilyRelationEndpointMarker::Triangle)
    ) || matches!(
        normalized_arrow.end_marker(),
        Some(FamilyRelationEndpointMarker::Triangle)
    );
    if !has_triangle {
        return None;
    }
    // Both endpoints must be Actor/BusinessActor.
    let from_node_kind = nodes
        .iter()
        .find(|n| n.alias.as_deref().unwrap_or(&n.name) == from_name || n.name == from_name)
        .map(|n| n.kind);
    let to_node_kind = nodes
        .iter()
        .find(|n| n.alias.as_deref().unwrap_or(&n.name) == to_name || n.name == to_name)
        .map(|n| n.kind);
    if !from_node_kind.map(is_actor_kind).unwrap_or(false)
        || !to_node_kind.map(is_actor_kind).unwrap_or(false)
    {
        return None;
    }
    // Use vertical ports: marker-start is at FROM (the hollow-triangle end).
    // If TO is below FROM: exit FROM's bottom, enter TO's top.
    // If TO is above FROM: exit FROM's top, enter TO's bottom.
    let from_cx = from.x + from.w / 2;
    let to_cx = to.x + to.w / 2;
    if to.y >= from.y {
        // TO is below or same level: bottom of FROM to top of TO
        Some((from_cx, from.y + from.h, to_cx, to.y))
    } else {
        // TO is above: top of FROM to bottom of TO
        Some((from_cx, from.y, to_cx, to.y + to.h))
    }
}

// #1292: system-boundary frame entry/exit snap

/// For usecase diagrams: when an ortho path segment runs through a group-frame
/// header band, nudge it below the header so the visible edge line does not
/// cross the frame label text (#1292, #1446).
///
/// Only horizontal segments are adjusted: when a horizontal segment's y falls
/// inside the frame's header band (frame.y .. frame.y + label_header), push
/// it below the header so the segment routes in the frame content area, not
/// over the label text.
///
/// NOTE: A former Case 1 (downward-vertical segment crossing a frame top) was
/// removed because it truncated paths targeting a node inside a child frame at
/// the frame border, producing bare arrowhead glyphs with no connecting line
/// body (#1541, #1546).  The frame boundary is not a valid edge endpoint for
/// edges that pass through it on the way to an enclosed node.
fn snap_path_to_frame_boundaries(pts: &mut [(i32, i32)], frame_rects: &[ClassGroupFrameRect]) {
    if frame_rects.is_empty() || pts.len() < 3 {
        return;
    }
    let n = pts.len();
    for i in 0..n.saturating_sub(1) {
        let (ax, ay) = pts[i];
        let (bx, by) = pts[i + 1];

        // Only adjust horizontal segments (ay == by).
        if ay != by {
            continue;
        }
        let seg_y = ay;
        let seg_x_min = ax.min(bx);
        let seg_x_max = ax.max(bx);
        for frame in frame_rects {
            let header_top = frame.y;
            let header_bot = frame.y + frame.label_header;
            // The segment must pass through the frame's x-span and be inside
            // the header band vertically.
            let overlaps_x = seg_x_min < frame.x + frame.w && seg_x_max > frame.x;
            if overlaps_x && seg_y >= header_top && seg_y < header_bot {
                // Push segment below the header. Adjust both endpoints (they
                // share the same y for a horizontal segment) and propagate the
                // new y to adjacent segment endpoints to keep the polyline
                // connected.
                pts[i].1 = header_bot;
                pts[i + 1].1 = header_bot;
                if i > 0 {
                    pts[i - 1].1 = header_bot;
                }
                if i + 2 < n {
                    pts[i + 2].1 = header_bot;
                }
                break;
            }
        }
    }
}

/// Render all edges (relations) for a class/object/usecase SVG diagram.
///
/// Emits `<line>` / `<polyline>` / `<path>` elements for edges, plus
/// stereotype, dependency, regular, cardinality, and role labels for each
/// relation.  In `EdgeRouting::Splines` mode, each edge is a cubic Bézier
/// `<path d="M … C …">` produced by the spline-native router (#1412);
/// the rounded-corner fallback is preserved for cases where the router
/// declines.  Nodes are rendered after this call so they visually cover
/// edge endpoints.
pub(super) fn render_class_relations(out: &mut String, ctx: &ClassRelationCtx<'_>) {
    // ── Spline fan-group pre-pass (#1412) ────────────────────────────────────
    // Pre-compute hub-and-spoke (source-fan) and converge (target-fan) groups
    // so each edge in a fan receives a distinct tangent-angle jitter and the
    // curves diverge at the port instead of stacking.
    //
    // Only populated when EdgeRouting::Splines is active; the maps are
    // trivially unused in Polyline/Ortho mode and add no overhead beyond a
    // couple of empty BTreeMap iterations.
    const SPLINE_FAN_STEP_RAD: f64 = 0.22; // ≈ 12.5° per lane
    const SPLINE_FAN_MAX_RAD: f64 = 0.70; // ≈ 40° absolute cap

    // Map from_name → vec of rel_idx (used for source-side jitter).
    let mut src_fan_groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    // Map to_name → vec of rel_idx (used for target-side jitter).
    let mut tgt_fan_groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();

    if matches!(
        ctx.edge_routing,
        crate::render::graph_layout::EdgeRouting::Splines
    ) {
        for (rel_idx, relation) in ctx.relations.iter().enumerate() {
            if relation.direction.is_some() || relation.hidden {
                continue;
            }
            let (from_name, to_name, _arrow) =
                normalize_relation_endpoints(&relation.from, &relation.to, &relation.arrow);
            if from_name == to_name {
                continue;
            }
            let render_from = resolve_relation_endpoint_key(&from_name, ctx.node_boxes);
            let render_to = resolve_relation_endpoint_key(&to_name, ctx.node_boxes);
            if !ctx.node_boxes.contains_key(&render_from)
                || !ctx.node_boxes.contains_key(&render_to)
            {
                continue;
            }
            src_fan_groups.entry(render_from).or_default().push(rel_idx);
            tgt_fan_groups.entry(render_to).or_default().push(rel_idx);
        }
    }

    // rel_idx → source-side jitter (radians).
    let mut spline_src_jitter: BTreeMap<usize, f64> = BTreeMap::new();
    // rel_idx → target-side jitter (radians).
    let mut spline_tgt_jitter: BTreeMap<usize, f64> = BTreeMap::new();
    for group in src_fan_groups.values() {
        if group.len() <= 1 {
            continue;
        }
        let n = group.len() as f64;
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
            // Reverse sign at the target end so converging curves mirror the
            // diverging-source pattern symmetrically.
            let theta =
                -(lane * SPLINE_FAN_STEP_RAD).clamp(-SPLINE_FAN_MAX_RAD, SPLINE_FAN_MAX_RAD);
            spline_tgt_jitter.insert(rel_idx, theta);
        }
    }
    // ─────────────────────────────────────────────────────────────────────────

    // Flattened obstacle list (all node bboxes) — built once, reused per edge.
    // The spline router filters out src/tgt internally so we can pass all boxes.
    let all_node_bboxes: Vec<(f64, f64, f64, f64)> = ctx
        .node_boxes
        .values()
        .map(|nb| (nb.x as f64, nb.y as f64, nb.w as f64, nb.h as f64))
        .collect();

    for (rel_idx, relation) in ctx.relations.iter().enumerate() {
        let (from_name, to_name, normalized_arrow) =
            normalize_relation_endpoints(&relation.from, &relation.to, &relation.arrow);
        let render_from_name = resolve_relation_endpoint_key(&from_name, ctx.node_boxes);
        let render_to_name = resolve_relation_endpoint_key(&to_name, ctx.node_boxes);
        let from = ctx.node_boxes.get(&render_from_name);
        let to = ctx.node_boxes.get(&render_to_name);
        let (Some(from), Some(to)) = (from, to) else {
            continue;
        };
        // Self-association curve (#1319): when the relation refers to the
        // same class box on both ends, the orthogonal router produces a
        // degenerate zero-length line.  Emit a small "C"-shaped arc hugging
        // the top-right corner instead.
        if render_from_name == render_to_name {
            let style = arrow_style(&normalized_arrow);
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
            let arc_w: i32 = 28;
            let arc_h: i32 = 28;
            let exit_x = from.x + from.w - 20;
            let exit_y = from.y;
            let return_x = from.x + from.w;
            let return_y = from.y + 20;
            let top_x = from.x + from.w + arc_w / 2;
            let top_y = from.y - arc_h;
            let right_x = from.x + from.w + arc_w;
            let right_y = from.y + arc_h / 2;
            let label_x = top_x + arc_w / 2;
            let label_y = top_y + 4;
            out.push_str(&format!(
                "<path class=\"uml-relation uml-self-association\" data-uml-from=\"{}\" data-uml-to=\"{}\" data-uml-arrow=\"{}\" d=\"M {} {} Q {} {} {} {} Q {} {} {} {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}{} />",
                escape_text(&relation.from),
                escape_text(&relation.to),
                escape_text(normalized_arrow.as_str()),
                exit_x, exit_y,
                top_x, top_y, right_x, right_y,
                right_x, return_y, return_x, return_y,
                relation_color, stroke_width,
                stroke_dash, visibility, markers
            ));
            if let Some(label) = relation.label.as_deref() {
                let lbl_fill = relation
                    .label_color
                    .as_ref()
                    .map(|c| c.as_str())
                    .unwrap_or(&ctx.class_style.member_color);
                out.push_str(&format!(
                    "<text class=\"uml-edge-label\" data-uml-label-role=\"edge\" x=\"{}\" y=\"{}\" text-anchor=\"start\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
                    label_x, label_y, lbl_fill, escape_text(label)
                ));
            }
            continue;
        }
        let mut style = arrow_style(&normalized_arrow);
        let usecase_dependency = usecase_dependency_label(relation.label.as_deref())
            .or_else(|| usecase_dependency_label(relation.stereotype.as_deref()));
        if usecase_dependency.is_some() {
            style.dashed = true;
            if style.end_marker.is_none() {
                style.end_marker = Some("arrow-open");
            }
        }
        // #1291: Track whether the actor-generalization port override fired so
        // we can later discard the pre-computed ortho path.
        let actor_gen_override = ctx.is_usecase_layout
            && actor_generalization_pick_port(
                from,
                to,
                &normalized_arrow,
                ctx.nodes,
                &render_from_name,
                &render_to_name,
            )
            .is_some();
        let (mut x1, mut y1, mut x2, mut y2) = if relation.direction.is_some() {
            compute_edge_anchors_for_direction(
                (from.x, from.y, from.w, from.h),
                (to.x, to.y, to.w, to.h),
                relation.direction.as_deref(),
            )
        } else if ctx.is_usecase_layout {
            // #1291: For actor-to-actor generalization edges in usecase diagrams,
            // override port selection to use vertical (top/bottom) ports.
            actor_generalization_pick_port(
                from,
                to,
                &normalized_arrow,
                ctx.nodes,
                &render_from_name,
                &render_to_name,
            )
            .unwrap_or_else(|| {
                pick_port((from.x, from.y, from.w, from.h), (to.x, to.y, to.w, to.h))
            })
        } else {
            pick_port((from.x, from.y, from.w, from.h), (to.x, to.y, to.w, to.h))
        };
        let mut from_anchor = ClassEndpointAnchor {
            x: x1,
            y: y1,
            side: class_port_side_from_box_anchor(x1, y1, from),
            is_row_port: false,
        };
        let mut to_anchor = ClassEndpointAnchor {
            x: x2,
            y: y2,
            side: class_port_side_from_box_anchor(x2, y2, to),
            is_row_port: false,
        };
        if let Some(anchor) = qualified_row_anchor(&from_name, ctx.nodes, ctx.node_boxes, to) {
            from_anchor = anchor;
            (x1, y1) = anchor.point();
        }
        if let Some(anchor) = qualified_row_anchor(&to_name, ctx.nodes, ctx.node_boxes, from) {
            to_anchor = anchor;
            (x2, y2) = anchor.point();
        }
        if from_anchor.is_row_port && !to_anchor.is_row_port {
            to_anchor = class_box_anchor_toward_point(to, from_anchor.point());
            (x2, y2) = to_anchor.point();
        } else if to_anchor.is_row_port && !from_anchor.is_row_port {
            from_anchor = class_box_anchor_toward_point(from, to_anchor.point());
            (x1, y1) = from_anchor.point();
        }

        let lat_offset = ctx.parallel_offset.get(&rel_idx).copied().unwrap_or(0);
        let edge_dx_raw = x2 - x1;
        let edge_dy_raw = y2 - y1;
        let (off_x, off_y) = if edge_dx_raw.abs() >= edge_dy_raw.abs() {
            (0, lat_offset)
        } else {
            (lat_offset, 0)
        };
        // #1461/#1481: clamp the fanned exit point so it stays within the
        // source node's bounding box.  Actor fan-out can push the start x
        // outside the actor bbox, creating an orphaned corner segment that
        // looks like a stray rectangle artifact in the PNG render.
        let raw_x1 = x1 + off_x;
        let raw_y1 = y1 + off_y;
        let clamped_x1 = raw_x1.clamp(from.x, from.x + from.w);
        let clamped_y1 = raw_y1.clamp(from.y, from.y + from.h);
        let (x1, y1, x2, y2) = (clamped_x1, clamped_y1, x2 + off_x, y2 + off_y);
        from_anchor.x = x1;
        from_anchor.y = y1;
        to_anchor.x = x2;
        to_anchor.y = y2;
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
        if let Some(row_port_pts) =
            class_route_with_row_ports(from_anchor, to_anchor, ortho_pts.as_deref())
        {
            ortho_pts = Some(row_port_pts);
        }
        // #1291: If the actor-generalization port override fired, discard the
        // pre-computed ortho path.  The layout engine routed the edge using the
        // old lateral ports, so its waypoints are wrong for the new vertical
        // head/feet routing.  Dropping ortho_pts causes the fallback <line>
        // branch below to be used, drawing a clean straight vertical line.
        if actor_gen_override {
            ortho_pts = None;
        }

        let (label_mx, label_my);

        if let Some(ref mut pts) = ortho_pts {
            // Snap path endpoints to the actual rendered node ports.
            if let Some(first) = pts.first_mut() {
                *first = (x1, y1);
            }
            if let Some(last) = pts.last_mut() {
                *last = (x2, y2);
            }
            // Perpendicular stub enforcement.  After snapping the endpoint
            // anchors above, the first segment (pts[0]→pts[1]) must be
            // perpendicular to the source side and the last segment
            // (pts[cn-2]→pts[cn-1]) must be perpendicular to the target
            // side.  When the snap shifts an endpoint laterally, the
            // router-emitted penultimate waypoint still sits at the
            // un-shifted axis, leaving the stub diagonal — visually the
            // arrowhead "grazes" the box border instead of pointing into the
            // side at 90°.  Align the off-axis coordinate of the
            // second/penultimate waypoint to the endpoint:
            //
            //   Top/Bottom side → outward normal is vertical → segment must
            //     be vertical → match x to endpoint x.
            //   Left/Right side → outward normal is horizontal → segment
            //     must be horizontal → match y to endpoint y.
            //
            // For cn == 3 the single interior waypoint must satisfy both the
            // source-side and target-side stub.  When the two sides are on
            // the same axis (both vertical or both horizontal), inject an
            // extra waypoint so we get a U-shaped bend.  When the two sides
            // are perpendicular, the single interior point becomes the
            // natural L-corner.
            let cn = pts.len();
            if cn >= 3 && !from_anchor.is_row_port && !to_anchor.is_row_port {
                let src_axis_vertical =
                    matches!(from_anchor.side, ClassPortSide::Top | ClassPortSide::Bottom);
                let tgt_axis_vertical =
                    matches!(to_anchor.side, ClassPortSide::Top | ClassPortSide::Bottom);
                if cn == 3 {
                    let (p0x, p0y) = pts[0];
                    let (p2x, p2y) = pts[2];
                    match (src_axis_vertical, tgt_axis_vertical) {
                        (true, true) => {
                            if p0x == p2x {
                                pts[1].0 = p0x;
                            } else {
                                let bend_y = pts[1].1;
                                pts[1] = (p0x, bend_y);
                                pts.insert(2, (p2x, bend_y));
                            }
                        }
                        (false, false) => {
                            if p0y == p2y {
                                pts[1].1 = p0y;
                            } else {
                                let bend_x = pts[1].0;
                                pts[1] = (bend_x, p0y);
                                pts.insert(2, (bend_x, p2y));
                            }
                        }
                        (true, false) => {
                            pts[1] = (p0x, p2y);
                        }
                        (false, true) => {
                            pts[1] = (p2x, p0y);
                        }
                    }
                } else {
                    let (p0x, p0y) = pts[0];
                    let cn_now = pts.len();
                    let (pnx, pny) = pts[cn_now - 1];
                    if src_axis_vertical {
                        pts[1].0 = p0x;
                    } else {
                        pts[1].1 = p0y;
                    }
                    if tgt_axis_vertical {
                        pts[cn_now - 2].0 = pnx;
                    } else {
                        pts[cn_now - 2].1 = pny;
                    }
                }
            }
            // #1292: For usecase diagrams, snap edge paths that cross system-boundary
            // frame top borders.
            if ctx.is_usecase_layout {
                snap_path_to_frame_boundaries(pts, &ctx.group_frame_rects);
            }
            // ── Splines-mode dispatch (#1412) ────────────────────────────────
            // When EdgeRouting::Splines is active, attempt the spline-native
            // generator first.  It produces a single sweeping cubic Bézier
            // M…C… from source-anchor to target-anchor with tangents
            // perpendicular to the source/target boundary — not an orthogonal
            // corner path with rounded chamfers.  If the topology is too
            // complex, the generator returns None and we fall through to the
            // existing rounded-corner renderer.
            let mut spline_label_point: Option<(i32, i32)> = None;
            let mut emitted_spline = false;
            if matches!(
                ctx.edge_routing,
                crate::render::graph_layout::EdgeRouting::Splines
            ) && pts.len() >= 2
            {
                let (sx, sy) = pts[0];
                let (tx_, ty_) = pts[pts.len() - 1];
                let src_anchor = (sx as f64, sy as f64);
                let tgt_anchor = (tx_ as f64, ty_ as f64);
                let src_bbox = (from.x as f64, from.y as f64, from.w as f64, from.h as f64);
                let tgt_bbox = (to.x as f64, to.y as f64, to.w as f64, to.h as f64);
                let src_tangent =
                    tangent_from_bbox_side(src_anchor, src_bbox, 4.0).unwrap_or_else(|| {
                        let (ax, ay) = pts[0];
                        let (bx, by) = pts[1];
                        let dx = (bx - ax) as f64;
                        let dy = (by - ay) as f64;
                        let len = dx.hypot(dy).max(1.0);
                        (dx / len, dy / len)
                    });
                let tgt_outward =
                    tangent_from_bbox_side(tgt_anchor, tgt_bbox, 4.0).unwrap_or_else(|| {
                        let n = pts.len();
                        let (ax, ay) = pts[n - 1];
                        let (bx, by) = pts[n - 2];
                        let dx = (ax - bx) as f64;
                        let dy = (ay - by) as f64;
                        let len = dx.hypot(dy).max(1.0);
                        (dx / len, dy / len)
                    });
                let tgt_tangent = (-tgt_outward.0, -tgt_outward.1);
                // Obstacles: all node bboxes except source/target (filtered
                // inside the spline router by anchor containment).
                let obstacles: Vec<(f64, f64, f64, f64)> = all_node_bboxes
                    .iter()
                    .filter(|&&b| {
                        b != (from.x as f64, from.y as f64, from.w as f64, from.h as f64)
                            && b != (to.x as f64, to.y as f64, to.w as f64, to.h as f64)
                    })
                    .copied()
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
                        escape_text(&relation.from),
                        escape_text(&relation.to),
                        escape_text(normalized_arrow.as_str()),
                        d, relation_color, stroke_width,
                        stroke_dash, visibility, direction_attr, markers
                    ));
                    let (mx, my) = spline.point_at(0.5);
                    spline_label_point = Some((mx as i32, my as i32 - 12));
                    emitted_spline = true;
                }
            }
            if !emitted_spline {
                let (tag, geom_attr) =
                    crate::render::edge_smoothing::edge_geometry_attr(ctx.edge_routing, pts);
                out.push_str(&format!(
                    "<{tag} class=\"uml-relation\" data-uml-from=\"{}\" data-uml-to=\"{}\" data-uml-arrow=\"{}\" {} fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}{}{} />",
                    escape_text(&relation.from),
                    escape_text(&relation.to),
                    escape_text(normalized_arrow.as_str()),
                    geom_attr,
                    relation_color, stroke_width,
                    stroke_dash, visibility, direction_attr, markers
                ));
            }
            let (lmx, lmy) = if let Some(p) = spline_label_point {
                p
            } else {
                let longest_seg = pts
                    .windows(2)
                    .filter(|seg| seg[0] != seg[1])
                    .max_by_key(|seg| {
                        let (ax, ay) = seg[0];
                        let (bx, by_) = seg[1];
                        (bx - ax).pow(2) + (by_ - ay).pow(2)
                    });
                match longest_seg {
                    Some(seg) => ((seg[0].0 + seg[1].0) / 2, (seg[0].1 + seg[1].1) / 2 - 12),
                    None => ((x1 + x2) / 2, (y1 + y2) / 2 - 12),
                }
            };
            label_mx = lmx;
            label_my = lmy;
        } else {
            // ── Splines-mode dispatch for the no-ortho-path branch (#1412) ──
            // When no ortho path was computed (actor-generalization override,
            // direction-forced edges that landed here, etc.), we still want
            // the spline-native cubic for Splines mode so the visual mode is
            // consistent across the diagram.  Fall back to <line> if the
            // router declines.
            let maybe_spline = if matches!(
                ctx.edge_routing,
                crate::render::graph_layout::EdgeRouting::Splines
            ) {
                let src_anchor = (x1 as f64, y1 as f64);
                let tgt_anchor = (x2 as f64, y2 as f64);
                let src_bbox = (from.x as f64, from.y as f64, from.w as f64, from.h as f64);
                let tgt_bbox = (to.x as f64, to.y as f64, to.w as f64, to.h as f64);
                let src_tangent =
                    tangent_from_bbox_side(src_anchor, src_bbox, 4.0).unwrap_or((0.0, 1.0));
                let tgt_outward =
                    tangent_from_bbox_side(tgt_anchor, tgt_bbox, 4.0).unwrap_or((0.0, -1.0));
                let tgt_tangent = (-tgt_outward.0, -tgt_outward.1);
                let obstacles: Vec<(f64, f64, f64, f64)> = all_node_bboxes
                    .iter()
                    .filter(|&&b| {
                        b != (from.x as f64, from.y as f64, from.w as f64, from.h as f64)
                            && b != (to.x as f64, to.y as f64, to.w as f64, to.h as f64)
                    })
                    .copied()
                    .collect();
                generate_spline_path(SplinePathInput {
                    src_anchor,
                    tgt_anchor,
                    src_tangent,
                    tgt_tangent,
                    obstacles,
                    src_tangent_jitter: spline_src_jitter.get(&rel_idx).copied().unwrap_or(0.0),
                    tgt_tangent_jitter: spline_tgt_jitter.get(&rel_idx).copied().unwrap_or(0.0),
                })
            } else {
                None
            };
            if let Some(spline) = maybe_spline {
                let d = spline.to_svg_d();
                out.push_str(&format!(
                    "<path class=\"uml-relation\" data-uml-from=\"{}\" data-uml-to=\"{}\" data-uml-arrow=\"{}\" d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}{}{} />",
                    escape_text(&relation.from),
                    escape_text(&relation.to),
                    escape_text(normalized_arrow.as_str()),
                    d, relation_color, stroke_width,
                    stroke_dash, visibility, direction_attr, markers
                ));
                let (mx, my) = spline.point_at(0.5);
                label_mx = mx as i32;
                label_my = my as i32 - 12;
            } else {
                out.push_str(&format!(
                    "<line class=\"uml-relation\" data-uml-from=\"{}\" data-uml-to=\"{}\" x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\" stroke=\"{relation_color}\" stroke-width=\"{stroke_width}\"{dash}{visibility}{direction_attr}{markers}/>",
                    escape_text(&relation.from),
                    escape_text(&relation.to),
                    dash = stroke_dash,
                ));
                label_mx = (x1 + x2) / 2;
                label_my = (y1 + y2) / 2 - 12;
            }
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
                "<text x=\"{x}\" y=\"{y}\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"10\" fill=\"{member_color}\">{txt}</text>",
                x = x1 - 4,
                y = y1 - 6,
                member_color = ctx.class_style.member_color,
                txt = escape_text(left)
            ));
        }
        if let Some(right) = &relation.right_cardinality {
            // #1454: anchor the right cardinality label at the END of the text
            // (i.e. the text extends leftward from x2-4) so it never bleeds into
            // the target node bbox when the edge enters from the left.
            out.push_str(&format!(
                "<text x=\"{x}\" y=\"{y}\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"10\" fill=\"{member_color}\">{txt}</text>",
                x = x2 - 4,
                y = y2 - 6,
                member_color = ctx.class_style.member_color,
                txt = escape_text(right)
            ));
        }
        if let Some(left_role) = &relation.left_role {
            out.push_str(&format!(
                "<text x=\"{x}\" y=\"{y}\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"10\" fill=\"{member_color}\">{txt}</text>",
                x = x1 - 4,
                y = y1 + 12,
                member_color = ctx.class_style.member_color,
                txt = escape_text(left_role)
            ));
        }
        if let Some(right_role) = &relation.right_role {
            out.push_str(&format!(
                "<text x=\"{x}\" y=\"{y}\" text-anchor=\"start\" font-family=\"monospace\" font-size=\"10\" fill=\"{member_color}\">{txt}</text>",
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
        let source_label_lane = ctx
            .relation_source_label_lanes
            .get(&rel_idx)
            .copied()
            .unwrap_or(0);
        let combined_label_lane = pair_label_lane + source_label_lane;
        if let Some(stereotype) = &relation.stereotype {
            if usecase_dependency.is_none() {
                let (sx, base_sy) = ctx
                    .label_override
                    .get(&rel_idx)
                    .copied()
                    .unwrap_or((label_mx, label_my));
                let sy =
                    base_sy - if relation.label.is_some() { 24 } else { 14 } + combined_label_lane;
                out.push_str(&format!(
                    "<text x=\"{sx}\" y=\"{sy}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"{member_color}\">&lt;&lt;{txt}&gt;&gt;</text>",
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
            let lx = lx.clamp(
                ctx.margin_x + 8 + label_half_w,
                ctx.svg_width - ctx.margin_x - 8 - label_half_w,
            );
            let ly = (ly + combined_label_lane).max(ctx.margin_top + 10);
            let (lx, ly) = class_nudge_label_y(lx, ly, label_half_w, ctx.node_boxes);
            // #1463/#1464: class_nudge_label_y can push a label above the
            // diagram content area (into the title band) when the source node
            // sits near the top of the canvas.  Re-clamp to canvas_margin_y
            // (= margin_top + title_block_height + group_top_reserve) so edge
            // labels are always visible within the content area.
            let ly = ly.max(ctx.canvas_margin_y + 4);
            let lbl_fill = relation
                .label_color
                .as_ref()
                .map(|c| c.as_str().to_owned())
                .unwrap_or_else(|| ctx.class_style.member_color.clone());
            out.push_str(&relation_label_svg(lx, ly, label, 11, &lbl_fill));
        } else if let Some(label) = relation.label.as_deref() {
            let (lx, ly, label_on_vertical_edge) =
                if let Some(&(ox, oy)) = ctx.label_override.get(&rel_idx) {
                    (ox, oy, false)
                } else if ortho_pts.is_some() {
                    if edge_dy.abs() > edge_dx.abs() {
                        // Vertical ortho edge: start at the arclength midpoint, centred
                        // on the edge. class_nudge_label_x will push right only if the
                        // label would actually overlap a node box.
                        (label_mx, label_my, true)
                    } else {
                        (label_mx, label_my - 14, false)
                    }
                } else {
                    let dx = x2 - x1;
                    let dy = y2 - y1;
                    let dx_abs = dx.abs();
                    let dy_abs = dy.abs();
                    let edge_len = ((dx_abs * dx_abs + dy_abs * dy_abs) as f64).sqrt() as i32;
                    if edge_len <= 2 {
                        ((x1 + x2) / 2, (y1 + y2) / 2 - 12, false)
                    } else {
                        let clearance = 30i32;
                        let t_num = (edge_len * 2 / 5).max(clearance).min(edge_len - clearance);
                        let raw_x = x1 + dx * t_num / edge_len;
                        let raw_y = y1 + dy * t_num / edge_len;
                        if dy_abs > dx_abs {
                            // Non-ortho vertical: same approach — default to midpoint,
                            // nudge only if collision is detected.
                            (raw_x, raw_y - 6, true)
                        } else {
                            (raw_x, raw_y - 14, false)
                        }
                    }
                };
            let label_half_w = ((label.chars().count() as i32) * 3).max(18);
            let lx = lx.clamp(
                ctx.margin_x + 8 + label_half_w,
                ctx.svg_width - ctx.margin_x - 8 - label_half_w,
            );
            let ly = (ly + combined_label_lane).max(ctx.margin_top + 10);
            let (lx, ly) = class_nudge_label_y(lx, ly, label_half_w, ctx.node_boxes);
            // #1464: re-clamp after nudge to prevent upward escape into title area.
            let ly = ly.max(ctx.canvas_margin_y + 4);
            let lx = if label_on_vertical_edge && !ctx.is_object_diagram {
                class_nudge_label_x(lx, ly, label_half_w, ctx.node_boxes)
            } else {
                lx
            };
            let lbl_fill = relation
                .label_color
                .as_ref()
                .map(|c| c.as_str().to_owned())
                .unwrap_or_else(|| ctx.class_style.member_color.clone());
            out.push_str(&relation_label_svg(lx, ly, label, 11, &lbl_fill));
        }
    }
}
