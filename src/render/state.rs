use super::*;

const STATE_NODE_W: i32 = 140;
const STATE_NODE_H: i32 = 40;
const STATE_NODE_GAP_X: i32 = 60;
const STATE_NODE_GAP_Y: i32 = 70;
const STATE_MARGIN: i32 = 30;
const _STATE_ARROW_LEN: i32 = 40;

pub fn render_state_svg(document: &StateDocument) -> String {
    // Simple left-to-right column layout: all top-level nodes in one or two columns,
    // then draw transitions as arrows.
    let nodes = &document.nodes;
    let transitions = &document.transitions;
    let state_style = &document.state_style;

    // Assign coordinates to each node
    let mut node_coords: std::collections::BTreeMap<String, (i32, i32)> =
        std::collections::BTreeMap::new();
    let cols = 2i32;
    for (idx, node) in nodes.iter().enumerate() {
        let col = (idx as i32) % cols;
        let row = (idx as i32) / cols;
        let x = STATE_MARGIN + col * (STATE_NODE_W + STATE_NODE_GAP_X);
        let y = STATE_MARGIN + row * (STATE_NODE_H + STATE_NODE_GAP_Y) + 50;
        node_coords.insert(node.name.clone(), (x, y));
    }

    let node_count = nodes.len() as i32;
    let rows = (node_count + cols - 1) / cols;
    let width = STATE_MARGIN * 2 + cols * (STATE_NODE_W + STATE_NODE_GAP_X);
    let height = STATE_MARGIN * 2 + rows * (STATE_NODE_H + STATE_NODE_GAP_Y) + 80;

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        width, height, width, height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    out.push_str(&format!(
        "<defs><marker id=\"arrow\" markerWidth=\"8\" markerHeight=\"8\" refX=\"6\" refY=\"3\" orient=\"auto\"><path d=\"M0,0 L0,6 L8,3 z\" fill=\"{}\"/></marker></defs>",
        state_style.arrow_color
    ));

    // Title
    let mut y_header = 28i32;
    if let Some(title) = &document.title {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\" text-anchor=\"middle\" fill=\"{}\">{}</text>",
            width / 2,
            y_header,
            escape_text(&state_style.font_color),
            escape_text(title)
        ));
        y_header += 20;
    }
    out.push_str(&format!(
        "<text x=\"40\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"{}\">state diagram</text>",
        y_header,
        escape_text(&state_style.font_color)
    ));

    let mut incoming: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    let mut outgoing: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for t in transitions {
        *incoming.entry(t.to.as_str()).or_insert(0) += 1;
        *outgoing.entry(t.from.as_str()).or_insert(0) += 1;
    }

    // Draw transitions (arrows) first so nodes appear on top
    for t in transitions {
        let from_coord = node_coords.get(&t.from);
        let to_coord = node_coords.get(&t.to);
        let from_node = nodes.iter().find(|n| n.name == t.from);
        let to_node = nodes.iter().find(|n| n.name == t.to);
        if let (Some(&(fx, fy)), Some(&(tx, ty)), Some(from_node), Some(to_node)) =
            (from_coord, to_coord, from_node, to_node)
        {
            // Compute start/end points at node boundaries
            let (x1, y1, x2, y2) = transition_endpoints(from_node, fx, fy, to_node, tx, ty);
            if t.from == t.to {
                let loop_rx = 18;
                let loop_ry = 14;
                let cpx = x1 + loop_rx;
                let cpy = y1 - loop_ry;
                out.push_str(&format!(
                    "<path class=\"state-transition\" data-state-from=\"{}\" data-state-to=\"{}\" d=\"M {x1} {y1} Q {cpx} {cpy} {x2} {y2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}{} marker-end=\"url(#arrow)\"/>",
                    escape_text(&t.from),
                    escape_text(&t.to),
                    escape_text(t.line_color.as_deref().unwrap_or(&state_style.arrow_color)),
                    t.thickness.unwrap_or(2).clamp(1, 8),
                    state_dash_attr(t.dashed),
                    state_hidden_attr(t.hidden),
                    state_direction_attr(t.direction.as_deref())
                ));
            } else {
                out.push_str(&format!(
                    "<line class=\"state-transition\" data-state-from=\"{}\" data-state-to=\"{}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}{}{} marker-end=\"url(#arrow)\"/>",
                    escape_text(&t.from),
                    escape_text(&t.to),
                    x1, y1, x2, y2,
                    escape_text(t.line_color.as_deref().unwrap_or(&state_style.arrow_color)),
                    t.thickness.unwrap_or(2).clamp(1, 8),
                    state_dash_attr(t.dashed),
                    state_hidden_attr(t.hidden),
                    state_direction_attr(t.direction.as_deref())
                ));
            }
            if let Some(label) = &t.label {
                let mx = (x1 + x2) / 2;
                let my = (y1 + y2) / 2 - 6;
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\" text-anchor=\"middle\">{}</text>",
                    mx, my, escape_text(&state_style.font_color), escape_text(label)
                ));
            }
        }
    }

    // Draw nodes — pass state_style for coloring
    for node in nodes {
        if let Some(&(x, y)) = node_coords.get(&node.name) {
            render_state_node_svg_styled(
                &mut out,
                node,
                x,
                y,
                state_style,
                *incoming.get(node.name.as_str()).unwrap_or(&0),
                *outgoing.get(node.name.as_str()).unwrap_or(&0),
            );
        }
    }

    out.push_str("</svg>");
    out
}

fn state_node_bbox(node: &StateNode) -> (i32, i32) {
    match node.kind {
        StateNodeKind::Fork | StateNodeKind::Join => (STATE_NODE_W, 8),
        StateNodeKind::Choice => (60, 40),
        StateNodeKind::HistoryShallow | StateNodeKind::HistoryDeep => (40, 40),
        StateNodeKind::StartEnd | StateNodeKind::End => (40, 40),
        StateNodeKind::Normal => {
            let actions_h = (node.internal_actions.len() as i32) * 14;
            (STATE_NODE_W, STATE_NODE_H + actions_h)
        }
    }
}

fn state_node_kind_name(kind: &StateNodeKind) -> &'static str {
    match kind {
        StateNodeKind::Normal => "normal",
        StateNodeKind::StartEnd => "start-end",
        StateNodeKind::HistoryShallow => "history-shallow",
        StateNodeKind::HistoryDeep => "history-deep",
        StateNodeKind::Fork => "fork",
        StateNodeKind::Join => "join",
        StateNodeKind::Choice => "choice",
        StateNodeKind::End => "end",
    }
}

fn state_dash_attr(dashed: bool) -> &'static str {
    if dashed {
        " stroke-dasharray=\"5 3\""
    } else {
        ""
    }
}

fn state_hidden_attr(hidden: bool) -> &'static str {
    if hidden {
        " visibility=\"hidden\""
    } else {
        ""
    }
}

fn state_direction_attr(direction: Option<&str>) -> String {
    direction
        .map(|direction| format!(" data-state-direction=\"{}\"", escape_text(direction)))
        .unwrap_or_default()
}

fn transition_endpoints(
    from_node: &StateNode,
    fx: i32,
    fy: i32,
    to_node: &StateNode,
    tx: i32,
    ty: i32,
) -> (i32, i32, i32, i32) {
    let (fw_full, fh_full) = state_node_bbox(from_node);
    let (tw_full, th_full) = state_node_bbox(to_node);
    let fh = fh_full / 2;
    let fw = fw_full / 2;
    let th = th_full / 2;
    let tw = tw_full / 2;

    // Center of each node
    let fcx = fx + fw;
    let fcy = fy + fh;
    let tcx = tx + tw;
    let tcy = ty + th;

    // Simple: exit from right/left/bottom/top depending on relative position
    let dx = tcx - fcx;
    let dy = tcy - fcy;

    if dx.abs() >= dy.abs() {
        // Horizontal
        if dx >= 0 {
            (fcx + fw, fcy, tcx - tw, tcy)
        } else {
            (fcx - fw, fcy, tcx + tw, tcy)
        }
    } else {
        // Vertical
        if dy >= 0 {
            (fcx, fcy + fh, tcx, tcy - th)
        } else {
            (fcx, fcy - fh, tcx, tcy + th)
        }
    }
}

/// Render a single state node at (x, y) — delegates to the styled version with defaults.
fn render_state_node_svg_styled(
    out: &mut String,
    node: &StateNode,
    x: i32,
    y: i32,
    state_style: &crate::theme::StateStyle,
    incoming_count: usize,
    outgoing_count: usize,
) {
    let w = STATE_NODE_W;
    let base_h = STATE_NODE_H;
    let action_rows = node.internal_actions.len() as i32;
    let h = base_h + action_rows * 14;
    out.push_str(&format!(
        "<metadata data-state-node=\"{}\" data-state-kind=\"{}\"{} />",
        escape_text(&node.name),
        state_node_kind_name(&node.kind),
        node.stereotype
            .as_deref()
            .map(|stereotype| format!(" data-state-stereotype=\"{}\"", escape_text(stereotype)))
            .unwrap_or_default()
    ));

    match node.kind {
        StateNodeKind::StartEnd => {
            let cx = x + w / 2;
            let cy = y + base_h / 2;
            if incoming_count > 0 && outgoing_count == 0 {
                out.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"14\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                    cx, cy, state_style.background_color, state_style.border_color
                ));
                out.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"8\" fill=\"{}\"/>",
                    cx, cy, state_style.start_color
                ));
            } else {
                out.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"12\" fill=\"{}\"/>",
                    cx, cy, state_style.start_color
                ));
            }
        }
        StateNodeKind::HistoryShallow | StateNodeKind::HistoryDeep => {
            let cx = x + w / 2;
            let cy = y + base_h / 2;
            let label = node.display.as_deref().unwrap_or("H");
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"16\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                cx, cy, state_style.background_color, state_style.border_color
            ));
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" text-anchor=\"middle\" dominant-baseline=\"middle\" fill=\"{}\">{}</text>",
                cx, cy, state_style.font_color, escape_text(label)
            ));
        }
        StateNodeKind::Fork | StateNodeKind::Join => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"8\" fill=\"{}\"/>",
                x,
                y + base_h / 2 - 4,
                w,
                state_style.start_color
            ));
            let label = if node.kind == StateNodeKind::Fork {
                "fork"
            } else {
                "join"
            };
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\" text-anchor=\"middle\">{}</text>",
                x + w / 2, y + base_h / 2 + 18, state_style.font_color, escape_text(label)
            ));
        }
        StateNodeKind::Choice => {
            let cx = x + w / 2;
            let cy = y + base_h / 2;
            let r = 18i32;
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                cx, cy - r,
                cx + r, cy,
                cx, cy + r,
                cx - r, cy,
                state_style.background_color, state_style.border_color
            ));
        }
        StateNodeKind::End => {
            let cx = x + w / 2;
            let cy = y + base_h / 2;
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"14\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                cx, cy, state_style.background_color, state_style.border_color
            ));
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"9\" fill=\"{}\"/>",
                cx, cy, state_style.start_color
            ));
        }
        StateNodeKind::Normal => {
            let has_regions = node.regions.len() > 1
                || node.regions.first().map(|r| !r.is_empty()).unwrap_or(false);
            let display = node.display.as_deref().unwrap_or(&node.name);

            if has_regions && node.regions.len() > 1 {
                let total_w = w + (node.regions.len() as i32 - 1) * (STATE_NODE_W / 2 + 10);
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" ry=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                    x, y, total_w, h + 16, state_style.background_color, state_style.border_color
                ));
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" font-weight=\"600\" text-anchor=\"middle\" fill=\"{}\">{}</text>",
                    x + total_w / 2, y + 16, state_style.font_color, escape_text(display)
                ));
                let region_w = total_w / node.regions.len() as i32;
                for ri in 1..node.regions.len() {
                    let div_x = x + ri as i32 * region_w;
                    out.push_str(&format!(
                        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"4 3\"/>",
                        div_x, y + 24, div_x, y + h + 16, state_style.border_color
                    ));
                }
                for (ri, region) in node.regions.iter().enumerate() {
                    let region_x = x + ri as i32 * region_w + 4;
                    let mut child_y = y + 28;
                    for child in region {
                        render_state_node_svg_styled(
                            out,
                            child,
                            region_x,
                            child_y,
                            state_style,
                            0,
                            0,
                        );
                        child_y += STATE_NODE_H + 12;
                    }
                }
            } else {
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" ry=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                    x, y, w, h, state_style.background_color, state_style.border_color
                ));
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" text-anchor=\"middle\" fill=\"{}\">{}</text>",
                    x + w / 2, y + 24, state_style.font_color, escape_text(display)
                ));
                if !node.internal_actions.is_empty() {
                    out.push_str(&format!(
                        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                        x, y + base_h - 4, x + w, y + base_h - 4, state_style.border_color
                    ));
                    for (ai, action) in node.internal_actions.iter().enumerate() {
                        let ay = y + base_h + ai as i32 * 14;
                        let text = if action.action.is_empty() {
                            action.kind.clone()
                        } else {
                            format!("{} / {}", action.kind, action.action)
                        };
                        out.push_str(&format!(
                            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" font-style=\"italic\" fill=\"{}\">{}</text>",
                            x + 6, ay + 10, state_style.font_color, escape_text(&text)
                        ));
                    }
                }
                if let Some(region) = node.regions.first() {
                    if !region.is_empty() {
                        let mut child_y = y + h + 4;
                        for child in region {
                            render_state_node_svg_styled(
                                out,
                                child,
                                x + 8,
                                child_y,
                                state_style,
                                0,
                                0,
                            );
                            child_y += STATE_NODE_H + 8;
                        }
                    }
                }
            }
        }
    }
}
