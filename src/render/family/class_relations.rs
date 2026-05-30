use std::collections::BTreeMap;

use crate::render::geometry::{compute_edge_anchors_for_direction, pick_port};
use crate::render::relation::{
    arrow_style, normalize_relation_endpoints, render_lollipop_endpoint, usecase_dependency_label,
};
use crate::render::svg::escape_text;

use super::class_relation_labels::{relation_label_svg, resolve_relation_endpoint_key};
use super::class_routing::{
    class_box_anchor_toward_point, class_nudge_label_x, class_nudge_label_y,
    class_port_side_from_box_anchor, class_route_with_row_ports, qualified_row_anchor,
};
use super::class_types::{ClassEndpointAnchor, ClassNodeBox};

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
}

/// Render all edges (relations) for a class/object/usecase SVG diagram.
///
/// Emits `<line>` / `<polyline>` elements for edges, plus stereotype,
/// dependency, regular, cardinality, and role labels for each relation.
/// Nodes are rendered after this call so they visually cover edge endpoints.
pub(super) fn render_class_relations(out: &mut String, ctx: &ClassRelationCtx<'_>) {
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
        // the top-right corner instead — this is the PlantUML convention for
        // class self-association (ch 3 of the language reference).
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
            // Arc anchors: exit the top edge ~20px left of the right corner,
            // bulge up-and-right by `arc_w` × `arc_h`, and return into the
            // right edge ~20px below the top corner.  Two quadratic-bezier
            // corners give the classic looped self-pointer shape.
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
                out.push_str(&format!(
                    "<text class=\"uml-edge-label\" data-uml-label-role=\"edge\" x=\"{}\" y=\"{}\" text-anchor=\"start\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
                    label_x, label_y, ctx.class_style.member_color, escape_text(label)
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
        let (mut x1, mut y1, mut x2, mut y2) = if relation.direction.is_some() {
            compute_edge_anchors_for_direction(
                (from.x, from.y, from.w, from.h),
                (to.x, to.y, to.w, to.h),
                relation.direction.as_deref(),
            )
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
        let (x1, y1, x2, y2) = (x1 + off_x, y1 + off_y, x2 + off_x, y2 + off_y);
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
            if cn >= 3 && !from_anchor.is_row_port && !to_anchor.is_row_port {
                pts[1].0 = x1;
                if cn > 3 {
                    pts[cn - 2].0 = x2;
                }
            }
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
            // Use the longest non-degenerate segment as the label anchor.
            // We do NOT prefer the horizontal leg exclusively: in parallel-edge
            // routes the horizontal bend is often very short and near a node
            // boundary, so anchoring to it causes labels to land inside node
            // boxes and get nudged far off course (#1258).
            let longest_seg = pts
                .windows(2)
                .filter(|seg| seg[0] != seg[1])
                .max_by_key(|seg| {
                    let (ax, ay) = seg[0];
                    let (bx, by_) = seg[1];
                    (bx - ax).pow(2) + (by_ - ay).pow(2)
                });
            let (lmx, lmy) = match longest_seg {
                Some(seg) => ((seg[0].0 + seg[1].0) / 2, (seg[0].1 + seg[1].1) / 2 - 12),
                None => ((x1 + x2) / 2, (y1 + y2) / 2 - 12),
            };
            label_mx = lmx;
            label_my = lmy;
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
            out.push_str(&format!(
                "<text x=\"{x}\" y=\"{y}\" text-anchor=\"start\" font-family=\"monospace\" font-size=\"10\" fill=\"{member_color}\">{txt}</text>",
                x = x2 + 4,
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
            out.push_str(&relation_label_svg(
                lx,
                ly,
                label,
                11,
                &ctx.class_style.member_color,
            ));
        } else if let Some(label) = relation.label.as_deref() {
            // Regular relation label: place at edge midpoint.
            // `label_override` provides de-collided positions when multiple
            // labels would otherwise overlap (#1258, #1261).  Fall back to the
            // edge-path midpoint (`label_mx/label_my`) for single-edge cases.
            //
            // `label_on_vertical_edge` tracks whether this specific placement
            // came from a vertical segment (dy > dx).  In that case we apply an
            // additional x-nudge after the y-nudge so the label clears the
            // adjacent class boxes horizontally (#1258 clearance invariant).
            let (lx, ly, label_on_vertical_edge) =
                if let Some(&(ox, oy)) = ctx.label_override.get(&rel_idx) {
                    // De-collided override already accounts for box clearance.
                    (ox, oy, false)
                } else if ortho_pts.is_some() {
                    // No override: use label_mx/label_my from the longest-segment
                    // midpoint (already computed above from the actual edge path).
                    if edge_dy.abs() > edge_dx.abs() {
                        (label_mx + 14, label_my, true)
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
                            (raw_x + 14, raw_y - 6, true)
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
            // For vertical-edge labels only: push rightward out of any node box
            // so the label clears adjacent class boxes (#1258 clearance invariant).
            // This is intentionally NOT applied to fan-out overrides (those already
            // spread labels apart) or to horizontal-edge labels (those clear boxes
            // vertically via the y-nudge above).
            // Suppressed for object diagrams where labels are expected to stay
            // centred on the vertical edge, not pushed to the side.
            let lx = if label_on_vertical_edge && !ctx.is_object_diagram {
                class_nudge_label_x(lx, label_half_w, ctx.node_boxes)
            } else {
                lx
            };
            out.push_str(&relation_label_svg(
                lx,
                ly,
                label,
                11,
                &ctx.class_style.member_color,
            ));
        }
    }
}
