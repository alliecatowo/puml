use crate::render::graph_layout::EdgeRouting;

use super::*;

#[derive(Clone, Copy)]
pub(super) struct NodeFrame {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[derive(Clone, Copy)]
pub(super) struct NodeEdgeCounts {
    pub incoming: usize,
    pub outgoing: usize,
}

pub(super) struct RenderNodeContext<'a> {
    pub state_style: &'a crate::theme::StateStyle,
    pub placed: &'a std::collections::BTreeMap<String, PlacedNode>,
    pub incoming: &'a std::collections::BTreeMap<&'a str, usize>,
    pub outgoing: &'a std::collections::BTreeMap<&'a str, usize>,
    pub all_transitions: &'a [StateTransition],
    pub edge_set: &'a std::collections::BTreeSet<(&'a str, &'a str)>,
    pub node_kinds: &'a std::collections::BTreeMap<&'a str, &'a StateNodeKind>,
    pub edge_routing: EdgeRouting,
}

pub(super) fn render_node<'a>(
    out: &mut String,
    node: &'a StateNode,
    frame: NodeFrame,
    counts: NodeEdgeCounts,
    ctx: &RenderNodeContext<'a>,
    occupied_label_bounds: &mut Vec<LabelBounds>,
) {
    let NodeFrame { x, y, w, h } = frame;
    let state_style = ctx.state_style;
    out.push_str(&format!(
        "<metadata data-state-node=\"{}\" data-state-kind=\"{}\"{} />",
        escape_text(&node.name),
        state_node_kind_name(&node.kind),
        node.stereotype
            .as_deref()
            .map(|s| format!(" data-state-stereotype=\"{}\"", escape_text(s)))
            .unwrap_or_default()
    ));

    match node.kind {
        StateNodeKind::StartEnd => {
            let cx = x + w / 2;
            let cy = y + h / 2;
            let r = 12i32;
            if counts.incoming > 0 && counts.outgoing == 0 {
                // End variant: outer ring + inner dot
                out.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                    cx, cy, r, state_style.background_color, state_style.border_color
                ));
                out.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"7\" fill=\"{}\"/>",
                    cx, cy, state_style.start_color
                ));
            } else {
                // Start variant: filled circle
                out.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\"/>",
                    cx, cy, r, state_style.start_color
                ));
            }
        }

        StateNodeKind::End => {
            let cx = x + w / 2;
            let cy = y + h / 2;
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"12\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                cx, cy, state_style.background_color, state_style.border_color
            ));
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"8\" fill=\"{}\"/>",
                cx, cy, state_style.start_color
            ));
        }

        StateNodeKind::HistoryShallow | StateNodeKind::HistoryDeep => {
            let cx = x + w / 2;
            let cy = y + h / 2;
            let label = node.display.as_deref().unwrap_or("H");
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"16\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                cx, cy, state_style.background_color, state_style.border_color
            ));
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"{}\" font-weight=\"600\" text-anchor=\"middle\" dominant-baseline=\"middle\" fill=\"{}\">{}</text>",
                cx, cy, state_node_font_size(state_style), state_style.font_color, escape_text(label)
            ));
        }

        StateNodeKind::Fork | StateNodeKind::Join => {
            // UML spec: thick horizontal bar; no text label
            let bar_h = 8i32;
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
                x,
                y + h / 2 - bar_h / 2,
                w,
                bar_h,
                state_style.start_color
            ));
            // No "fork"/"join" text — UML spec shows only the bar
        }

        StateNodeKind::Choice => {
            let cx = x + w / 2;
            let cy = y + h / 2;
            let r = (w / 2).min(h / 2) - 2;
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                cx, cy - r,
                cx + r, cy,
                cx, cy + r,
                cx - r, cy,
                state_style.background_color, state_style.border_color
            ));
        }

        StateNodeKind::EntryPoint | StateNodeKind::ExitPoint => {
            let cx = x + w / 2;
            let cy = y + h / 2;
            let border = state_node_border(node, state_style);
            let fill = state_node_fill(node, state_style);
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"10\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                cx, cy, fill, border
            ));
            if node.kind == StateNodeKind::ExitPoint {
                out.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                    cx - 5, cy - 5, cx + 5, cy + 5, border
                ));
                out.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                    cx + 5, cy - 5, cx - 5, cy + 5, border
                ));
            }
            // Name label below the glyph (closes #1305)
            push_pseudostate_name_label(out, node, cx, cy, 16, state_style);
        }

        StateNodeKind::InputPin | StateNodeKind::OutputPin => {
            let fill = state_node_fill(node, state_style);
            let border = state_node_border(node, state_style);
            let sw = state_node_stroke_width(node, 1.5);
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{} />",
                x + 5,
                y + 5,
                w - 10,
                h - 10,
                fill,
                border,
                sw,
                state_node_border_dash(node)
            ));
            // Name label below the glyph (closes #1305)
            push_pseudostate_name_label(out, node, x + w / 2, y + h / 2, 18, state_style);
        }

        StateNodeKind::ExpansionInput | StateNodeKind::ExpansionOutput => {
            let fill = state_node_fill(node, state_style);
            let border = state_node_border(node, state_style);
            let sw = state_node_stroke_width(node, 1.5);
            let segment_w = (w - 8) / 3;
            for idx in 0..3 {
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{} />",
                    x + 4 + idx * segment_w,
                    y + 5,
                    segment_w,
                    h - 10,
                    fill,
                    border,
                    sw,
                    state_node_border_dash(node)
                ));
            }
            // Name label below the glyph (closes #1305)
            push_pseudostate_name_label(out, node, x + w / 2, y + h / 2, 16, state_style);
        }

        StateNodeKind::Terminate => {
            // UML terminate pseudostate: circle with an X drawn inside it.
            let cx = x + w / 2;
            let cy = y + h / 2;
            let r = 10i32;
            let border = state_node_border(node, state_style);
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                cx, cy, r, state_style.background_color, border
            ));
            // X cross lines
            let offset = (r as f32 * 0.6) as i32;
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                cx - offset,
                cy - offset,
                cx + offset,
                cy + offset,
                border
            ));
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                cx + offset,
                cy - offset,
                cx - offset,
                cy + offset,
                border
            ));
        }

        StateNodeKind::SdlReceive => {
            // SDL receive signal: rectangle with an arrow-head indent on the left side.
            let fill = state_node_fill(node, state_style);
            let border = state_node_border(node, state_style);
            let indent = (h / 4).max(6);
            let pts = format!(
                "{},{} {},{} {},{} {},{} {},{}",
                x + indent,
                y,
                x + w,
                y,
                x + w,
                y + h,
                x + indent,
                y + h,
                x,
                y + h / 2,
            );
            out.push_str(&format!(
                "<polygon points=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                pts, fill, border
            ));
            // Label: state name centered
            let display = node.display.as_deref().unwrap_or(&node.name);
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"{}\" font-weight=\"600\" text-anchor=\"middle\" fill=\"{}\">{}</text>",
                x + (w + indent) / 2,
                y + h / 2 + state_node_font_size(state_style) as i32 / 3,
                state_node_font_size(state_style),
                state_node_text(node, state_style),
                escape_text(display)
            ));
        }

        StateNodeKind::SdlSend => {
            // SDL send signal: rectangle with a convex arrow-point on the right side.
            let fill = state_node_fill(node, state_style);
            let border = state_node_border(node, state_style);
            let point = (h / 4).max(6);
            let pts = format!(
                "{},{} {},{} {},{} {},{} {},{}",
                x,
                y,
                x + w - point,
                y,
                x + w,
                y + h / 2,
                x + w - point,
                y + h,
                x,
                y + h,
            );
            out.push_str(&format!(
                "<polygon points=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                pts, fill, border
            ));
            // Label: state name centered
            let display = node.display.as_deref().unwrap_or(&node.name);
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"{}\" font-weight=\"600\" text-anchor=\"middle\" fill=\"{}\">{}</text>",
                x + (w - point) / 2,
                y + h / 2 + state_node_font_size(state_style) as i32 / 3,
                state_node_font_size(state_style),
                state_node_text(node, state_style),
                escape_text(display)
            ));
        }

        StateNodeKind::Note => {
            render_state_note(out, node, x, y, w, h);
        }

        StateNodeKind::JsonProjection => {
            render_state_json_projection(out, node, x, y, w, h, state_style);
        }

        StateNodeKind::Normal => {
            let has_children = node.regions.iter().any(|r| !r.is_empty());
            let display = node.display.as_deref().unwrap_or(&node.name);

            if has_children {
                // ── Composite state ──────────────────────────────────────────
                // Emit gradient def if needed, then draw the enclosing rounded-rect box
                out.push_str(&state_node_gradient_def(node));
                let fill = state_node_fill(node, state_style);
                let border = state_node_border(node, state_style);
                let text = state_node_text(node, state_style);
                let sw = state_node_stroke_width(node, 1.5);
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" ry=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
                    x, y, w, h, fill, border, sw, state_node_border_dash(node)
                ));
                // Composite name label at top-center
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"{}\" font-weight=\"600\" text-anchor=\"middle\" fill=\"{}\">{}</text>",
                    x + w / 2, y + 20, state_node_font_size(state_style), text, escape_text(display)
                ));
                // Internal actions (entry/exit/do) in composite header band (closes #1304)
                push_internal_actions(out, node, x, w, y + 28, state_style);

                // Draw concurrent region dividers (dashed horizontal lines).
                // Regions are stacked top-to-bottom; dividers sit between adjacent
                // regions, spanning the full inner width of the composite state.
                if node.regions.len() > 1 {
                    for ri in 0..node.regions.len() - 1 {
                        let prev_bot = node.regions[ri]
                            .iter()
                            .filter_map(|child| ctx.placed.get(&child.name))
                            .map(|child| child.y + child.h)
                            .max();
                        let next_top = node.regions[ri + 1]
                            .iter()
                            .filter_map(|child| ctx.placed.get(&child.name))
                            .map(|child| child.y)
                            .min();
                        if let (Some(prev_bot), Some(next_top)) = (prev_bot, next_top) {
                            let div_y = (prev_bot + next_top) / 2;
                            let div_left = x + COMPOSITE_PAD_X - 8;
                            let div_right = x + w - COMPOSITE_PAD_X + 8;
                            if div_left < div_right {
                                out.push_str(&format!(
                                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"5 3\"/>",
                                    div_left, div_y, div_right, div_y, state_style.border_color
                                ));
                            }
                        }
                    }
                }

                // Collect names of all direct children across all regions of this
                // composite, so we can draw intra-composite transitions above the
                // background rect but below child node boxes.
                let child_names: std::collections::BTreeSet<&str> = node
                    .regions
                    .iter()
                    .flat_map(|r| r.iter())
                    .map(|c| c.name.as_str())
                    .collect();

                // For label placement within the composite, replace the composite
                // parent's full bounding box with thin "wall" slabs along its
                // content edges.  The parent's full bounding box covers the entire
                // interior, so keeping it would force every intra-composite label
                // outside the composite box (#709).  The walls prevent labels from
                // drifting into the header / footer / side margins while still
                // allowing the algorithm to find positions in the interior gap.
                let composite_p = PlacedNode { x, y, w, h };
                let header_wall = PlacedNode {
                    x: composite_p.x,
                    y: composite_p.y,
                    w: composite_p.w,
                    h: COMPOSITE_PAD_Y, // covers the title bar
                };
                let footer_wall = PlacedNode {
                    x: composite_p.x,
                    y: composite_p.y + composite_p.h - COMPOSITE_PAD_BOT,
                    w: composite_p.w,
                    h: COMPOSITE_PAD_BOT,
                };
                let left_wall = PlacedNode {
                    x: composite_p.x,
                    y: composite_p.y,
                    w: COMPOSITE_PAD_X,
                    h: composite_p.h,
                };
                let right_wall = PlacedNode {
                    x: composite_p.x + composite_p.w - COMPOSITE_PAD_X,
                    y: composite_p.y,
                    w: COMPOSITE_PAD_X,
                    h: composite_p.h,
                };
                let mut inner_placed: std::collections::BTreeMap<String, PlacedNode> = ctx
                    .placed
                    .iter()
                    .filter(|(k, _)| k.as_str() != node.name.as_str())
                    .map(|(k, v)| (k.clone(), *v))
                    .collect();
                inner_placed.insert(format!("__wall_header_{}", node.name), header_wall);
                inner_placed.insert(format!("__wall_footer_{}", node.name), footer_wall);
                inner_placed.insert(format!("__wall_left_{}", node.name), left_wall);
                inner_placed.insert(format!("__wall_right_{}", node.name), right_wall);

                // Draw intra-composite transitions (both endpoints are direct children).
                // These were skipped in the outer transition loop so they appear above
                // the composite background rect rather than hidden behind it.
                for t in ctx.all_transitions {
                    if !child_names.contains(t.from.as_str())
                        || !child_names.contains(t.to.as_str())
                    {
                        continue;
                    }
                    let from_p = ctx.placed.get(&t.from);
                    let to_p = ctx.placed.get(&t.to);
                    if let (Some(fp), Some(tp)) = (from_p, to_p) {
                        let has_reverse = t.from != t.to
                            && ctx.edge_set.contains(&(t.to.as_str(), t.from.as_str()));
                        let (x1, y1, x2, y2) = edge_anchors_for_kinds(
                            ctx.node_kinds.get(t.from.as_str()).copied(),
                            fp,
                            ctx.node_kinds.get(t.to.as_str()).copied(),
                            tp,
                        );
                        let stroke = escape_text(
                            t.line_color.as_deref().unwrap_or(&state_style.arrow_color),
                        );
                        let sw = t.thickness.unwrap_or(2).clamp(1, 8);
                        let dash = state_dash_attr(t.dashed);
                        let hidden = state_hidden_attr(t.hidden);
                        let dir = state_direction_attr(t.direction.as_deref());

                        if t.from == t.to {
                            // Self-transition curve (#1319): emit a visible
                            // arc that exits the right edge and re-enters the
                            // top edge so the loop reads as a real curved
                            // back-pointer rather than a degenerate quadratic
                            // collapsed to a single point.
                            let (sx_box, sy_box, sw_box, sh_box) = (fp.x, fp.y, fp.w, fp.h);
                            let exit_x = sx_box + sw_box;
                            let exit_y = sy_box + sh_box / 3;
                            let enter_x = sx_box + sw_box - 20.max(sw_box / 4);
                            let enter_y = sy_box;
                            let arc_w = 28;
                            let arc_h = 28;
                            let c1x = exit_x + arc_w;
                            let c1y = exit_y;
                            let c2x = enter_x;
                            let c2y = enter_y - arc_h;
                            out.push_str(&format!(
                                "<path class=\"state-transition\" data-state-from=\"{}\" data-state-to=\"{}\" d=\"M {exit_x} {exit_y} C {c1x} {c1y} {c2x} {c2y} {enter_x} {enter_y}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}{} marker-end=\"url(#arrow)\"/>",
                                escape_text(&t.from), escape_text(&t.to), stroke, sw, dash, hidden, dir
                            ));
                            let _ = (x1, y1, x2, y2);
                            if let Some(label) = &t.label {
                                let apex_x = exit_x + arc_w + 2;
                                let apex_y = enter_y - arc_h / 2;
                                let layout = place_state_transition_label(
                                    label,
                                    apex_x,
                                    apex_y,
                                    apex_x,
                                    apex_y,
                                    &inner_placed,
                                    occupied_label_bounds,
                                );
                                render_state_transition_label(
                                    out,
                                    &layout,
                                    label,
                                    &state_style.font_color,
                                );
                                occupied_label_bounds.push(layout.bounds);
                            }
                            continue;
                        } else if has_reverse {
                            let (ox1, oy1, ox2, oy2) = offset_parallel_edge(x1, y1, x2, y2, 10);
                            let cpx = (ox1 + ox2) / 2;
                            let cpy = (oy1 + oy2) / 2;
                            out.push_str(&format!(
                                "<path class=\"state-transition\" data-state-from=\"{}\" data-state-to=\"{}\" d=\"M {} {} Q {} {} {} {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}{} marker-end=\"url(#arrow)\"/>",
                                escape_text(&t.from), escape_text(&t.to),
                                ox1, oy1, cpx, cpy, ox2, oy2,
                                stroke, sw, dash, hidden, dir
                            ));
                            if let Some(label) = &t.label {
                                let layout = place_state_transition_label(
                                    label,
                                    ox1,
                                    oy1,
                                    ox2,
                                    oy2,
                                    &inner_placed,
                                    occupied_label_bounds,
                                );
                                render_state_transition_label(
                                    out,
                                    &layout,
                                    label,
                                    &state_style.font_color,
                                );
                                occupied_label_bounds.push(layout.bounds);
                            }
                            continue;
                        } else {
                            emit_state_orthogonal_path(
                                out,
                                &t.from,
                                &t.to,
                                x1,
                                y1,
                                x2,
                                y2,
                                &StateEdgeStyle {
                                    stroke: &stroke,
                                    sw: sw as u32,
                                    dash,
                                    hidden,
                                    dir: &dir,
                                },
                                ctx.edge_routing,
                            );
                        }
                        if let Some(label) = &t.label {
                            let (lx1, ly1, lx2, ly2) =
                                state_orthogonal_label_segment(x1, y1, x2, y2);
                            let layout = place_state_transition_label(
                                label,
                                lx1,
                                ly1,
                                lx2,
                                ly2,
                                &inner_placed,
                                occupied_label_bounds,
                            );
                            render_state_transition_label(
                                out,
                                &layout,
                                label,
                                &state_style.font_color,
                            );
                            occupied_label_bounds.push(layout.bounds);
                        }
                    }
                }

                // Draw children recursively
                for region in &node.regions {
                    for child in region {
                        if let Some(cp) = ctx.placed.get(&child.name) {
                            let c_inc = *ctx.incoming.get(child.name.as_str()).unwrap_or(&0);
                            let c_out = *ctx.outgoing.get(child.name.as_str()).unwrap_or(&0);
                            render_node(
                                out,
                                child,
                                NodeFrame {
                                    x: cp.x,
                                    y: cp.y,
                                    w: cp.w,
                                    h: cp.h,
                                },
                                NodeEdgeCounts {
                                    incoming: c_inc,
                                    outgoing: c_out,
                                },
                                ctx,
                                occupied_label_bounds,
                            );
                        }
                    }
                }
            } else {
                // ── Simple state box ─────────────────────────────────────────
                // Emit gradient def if needed
                out.push_str(&state_node_gradient_def(node));
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" ry=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
                    x,
                    y,
                    w,
                    h,
                    state_node_fill(node, state_style),
                    state_node_border(node, state_style),
                    state_node_stroke_width(node, 1.5),
                    state_node_border_dash(node)
                ));
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"{}\" font-weight=\"600\" text-anchor=\"middle\" fill=\"{}\">{}</text>",
                    x + w / 2,
                    y + 24,
                    state_node_font_size(state_style),
                    state_node_text(node, state_style),
                    escape_text(display)
                ));
                // Internal actions
                push_internal_actions(out, node, x, w, y + STATE_NODE_H - 4, state_style);
            }
        }
    }
}
