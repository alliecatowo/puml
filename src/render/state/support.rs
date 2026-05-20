use super::*;

/// SVG style attributes bundled together for the orthogonal-path emitter.
pub(super) struct StateEdgeStyle<'a> {
    pub(super) stroke: &'a str,
    pub(super) sw: u32,
    pub(super) dash: &'a str,
    pub(super) hidden: &'a str,
    pub(super) dir: &'a str,
}

/// Emit an SVG `<path>` element that routes a state transition orthogonally
/// (L-shaped / Z-shaped elbow) rather than as a straight diagonal.
///
/// Routing rules (same logic as the activity renderer):
/// - Same X or same Y: emit a straight line segment.
/// - Otherwise: route via a symmetric mid-point bend
///   `(x1,y1) → (x1,mid_y) → (x2,mid_y) → (x2,y2)`.
///
/// The path carries the same SVG attributes (stroke, stroke-width, dash, hidden,
/// direction, data-* labels, marker-end) as the old `<line>` element.
// Style attrs are already grouped into `StateEdgeStyle`; the remaining args are
// the mandatory out-buffer, two name strings, and four coordinate scalars — there
// is no meaningful grouping that would reduce the count further without obfuscating
// the call sites.
#[allow(clippy::too_many_arguments)]
pub(super) fn emit_state_orthogonal_path(
    out: &mut String,
    from_name: &str,
    to_name: &str,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    style: &StateEdgeStyle<'_>,
) {
    let d = if x1 == x2 || y1 == y2 {
        format!("M {x1} {y1} L {x2} {y2}")
    } else {
        let mid_y = y1 + (y2 - y1) / 2;
        format!("M {x1} {y1} L {x1} {mid_y} L {x2} {mid_y} L {x2} {y2}")
    };
    out.push_str(&format!(
        "<path class=\"state-transition puml-edge\" {} data-state-from=\"{}\" data-state-to=\"{}\" d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}{} marker-end=\"url(#arrow)\"/>",
        puml_state_edge_attrs(from_name, to_name),
        escape_text(from_name),
        escape_text(to_name),
        d,
        style.stroke,
        style.sw,
        style.dash,
        style.hidden,
        style.dir
    ));
}

/// Offset a line segment by `d` pixels perpendicular to its direction (to the right).
/// Used to separate bidirectional parallel edges so both arrows are visible.
pub(super) fn offset_parallel_edge(
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    d: i32,
) -> (i32, i32, i32, i32) {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len_sq = dx * dx + dy * dy;
    if len_sq == 0 {
        return (x1, y1, x2, y2);
    }
    // Perpendicular unit vector (rotated 90° clockwise): (dy, -dx) / |len|
    let len = (len_sq as f64).sqrt();
    let ox = ((dy as f64 / len) * d as f64).round() as i32;
    let oy = ((-dx as f64 / len) * d as f64).round() as i32;
    (x1 + ox, y1 + oy, x2 + ox, y2 + oy)
}

/// Compute the edge anchor points between two placed nodes.
pub(super) fn edge_anchors(from: &PlacedNode, to: &PlacedNode) -> (i32, i32, i32, i32) {
    let fcx = from.x + from.w / 2;
    let fcy = from.y + from.h / 2;
    let tcx = to.x + to.w / 2;
    let tcy = to.y + to.h / 2;

    let dx = tcx - fcx;
    let dy = tcy - fcy;

    // Use half-sizes for boundary computation
    let fhw = from.w / 2;
    let fhh = from.h / 2;
    let thw = to.w / 2;
    let thh = to.h / 2;

    if dx == 0 && dy == 0 {
        return (fcx, fcy, tcx, tcy);
    }

    // Determine exit/entry side based on dominant direction
    if dx.abs() >= dy.abs() {
        if dx >= 0 {
            (fcx + fhw, fcy, tcx - thw, tcy)
        } else {
            (fcx - fhw, fcy, tcx + thw, tcy)
        }
    } else if dy >= 0 {
        (fcx, fcy + fhh, tcx, tcy - thh)
    } else {
        (fcx, fcy - fhh, tcx, tcy + thh)
    }
}

pub(super) fn wrap_state_label(label: &str, max_cols: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();

    for word in label.split_whitespace() {
        if word.len() > max_cols {
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
            }
            let mut start = 0usize;
            while start < word.len() {
                let end = (start + max_cols).min(word.len());
                lines.push(word[start..end].to_string());
                start = end;
            }
            continue;
        }

        let next_len = if current.is_empty() {
            word.len()
        } else {
            current.len() + 1 + word.len()
        };
        if next_len > max_cols && !current.is_empty() {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }

    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

pub(super) fn measure_state_label(lines: &[String]) -> (i32, i32) {
    let max_cols = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0) as i32;
    let width = (max_cols * STATE_LABEL_CHAR_W).max(24);
    let height = (lines.len() as i32 * STATE_LABEL_LINE_H).max(STATE_LABEL_LINE_H);
    (width, height)
}

pub(super) fn place_state_transition_label(
    label: &str,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    placed: &std::collections::BTreeMap<String, PlacedNode>,
    occupied: &[LabelBounds],
) -> StateLabelLayout {
    let lines = wrap_state_label(label, STATE_LABEL_WRAP_COLS);
    let (w, h) = measure_state_label(&lines);
    let mx = (x1 + x2) as f64 / 2.0;
    let my = (y1 + y2) as f64 / 2.0;
    let dx = (x2 - x1) as f64;
    let dy = (y2 - y1) as f64;
    let len = (dx * dx + dy * dy).sqrt();
    let (tx, ty, nx, ny) = if len <= f64::EPSILON {
        (1.0, 0.0, 0.0, -1.0)
    } else {
        let tx = dx / len;
        let ty = dy / len;
        (tx, ty, -ty, tx)
    };

    let mut best = label_bounds_from_center(mx.round() as i32, (my - 18.0).round() as i32, w, h);
    let t_positions = [0.3, 0.4, 0.5, 0.6, 0.7];
    let along_offsets = [
        0.0, -18.0, 18.0, -36.0, 36.0, -56.0, 56.0, -76.0, 76.0, -96.0, 96.0, -120.0, 120.0,
    ];
    let normal_offsets = [18.0, 30.0, 42.0, 56.0, 72.0, 92.0, 116.0, 140.0, 168.0];

    for t in t_positions {
        let base_x = x1 as f64 + dx * t;
        let base_y = y1 as f64 + dy * t;
        for normal_sign in [1.0, -1.0] {
            for normal in normal_offsets {
                for along in along_offsets {
                    let cx = base_x + nx * normal * normal_sign + tx * along;
                    let cy = base_y + ny * normal * normal_sign + ty * along;
                    let candidate =
                        label_bounds_from_center(cx.round() as i32, cy.round() as i32, w, h);
                    if !state_label_hits_node(candidate, placed)
                        && !state_label_hits_other_label(candidate, occupied)
                    {
                        return StateLabelLayout {
                            cx: candidate.x + candidate.w / 2,
                            top: candidate.y,
                            lines,
                            bounds: candidate,
                        };
                    }
                    if state_label_candidate_score(candidate, placed, occupied)
                        > state_label_candidate_score(best, placed, occupied)
                    {
                        best = candidate;
                    }
                }
            }
        }
    }

    StateLabelLayout {
        cx: best.x + best.w / 2,
        top: best.y,
        lines,
        bounds: best,
    }
}

pub(super) fn label_bounds_from_center(cx: i32, cy: i32, w: i32, h: i32) -> LabelBounds {
    LabelBounds {
        x: cx - w / 2,
        y: cy - h / 2,
        w,
        h,
    }
}

pub(super) fn state_label_hits_node(
    label: LabelBounds,
    placed: &std::collections::BTreeMap<String, PlacedNode>,
) -> bool {
    placed
        .values()
        .any(|node| bounds_overlap(label, node_bounds(node), STATE_LABEL_NODE_CLEARANCE))
}

pub(super) fn state_label_hits_other_label(label: LabelBounds, occupied: &[LabelBounds]) -> bool {
    occupied
        .iter()
        .copied()
        .any(|other| bounds_overlap(label, other, STATE_LABEL_LABEL_CLEARANCE))
}

pub(super) fn state_label_candidate_score(
    label: LabelBounds,
    placed: &std::collections::BTreeMap<String, PlacedNode>,
    occupied: &[LabelBounds],
) -> i32 {
    let node_hits = placed
        .values()
        .filter(|node| bounds_overlap(label, node_bounds(node), STATE_LABEL_NODE_CLEARANCE))
        .count() as i32;
    let label_hits = occupied
        .iter()
        .filter(|other| bounds_overlap(label, **other, STATE_LABEL_LABEL_CLEARANCE))
        .count() as i32;
    -(node_hits * 100 + label_hits * 150)
}

pub(super) fn node_bounds(node: &PlacedNode) -> LabelBounds {
    LabelBounds {
        x: node.x,
        y: node.y,
        w: node.w,
        h: node.h,
    }
}

pub(super) fn bounds_overlap(a: LabelBounds, b: LabelBounds, padding: i32) -> bool {
    let ax1 = a.x - padding;
    let ay1 = a.y - padding;
    let ax2 = a.x + a.w + padding;
    let ay2 = a.y + a.h + padding;
    let bx1 = b.x;
    let by1 = b.y;
    let bx2 = b.x + b.w;
    let by2 = b.y + b.h;
    ax1 < bx2 && ax2 > bx1 && ay1 < by2 && ay2 > by1
}

pub(super) fn render_state_transition_label(
    out: &mut String,
    layout: &StateLabelLayout,
    owner: &str,
    original_label: &str,
    font_color: &str,
) {
    out.push_str(&format!(
        "<text class=\"puml-label\" {} x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\" text-anchor=\"middle\" data-state-label=\"{}\">",
        puml_state_label_attrs(owner, "edge-label", layout.bounds),
        layout.cx,
        layout.top + 11,
        escape_text(font_color),
        escape_text(original_label)
    ));
    for (idx, line) in layout.lines.iter().enumerate() {
        out.push_str(&format!(
            "<tspan x=\"{}\" y=\"{}\">{}</tspan>",
            layout.cx,
            layout.top + 11 + idx as i32 * STATE_LABEL_LINE_H,
            escape_text(line)
        ));
    }
    out.push_str("</text>");
}

pub(super) fn edge_anchors_for_kinds(
    from_kind: Option<&StateNodeKind>,
    from: &PlacedNode,
    to_kind: Option<&StateNodeKind>,
    to: &PlacedNode,
) -> (i32, i32, i32, i32) {
    let mut anchors = edge_anchors(from, to);
    let from_center_x = from.x + from.w / 2;
    let from_center_y = from.y + from.h / 2;
    let to_center_x = to.x + to.w / 2;
    let to_center_y = to.y + to.h / 2;

    if matches!(
        from_kind,
        Some(&StateNodeKind::Fork) | Some(&StateNodeKind::Join)
    ) {
        let target_below = to_center_y >= from_center_y;
        anchors.0 = to_center_x.clamp(from.x, from.x + from.w);
        anchors.1 = if target_below {
            from.y + from.h
        } else {
            from.y
        };
        anchors.2 = to_center_x;
        anchors.3 = if target_below { to.y } else { to.y + to.h };
    }

    if matches!(
        to_kind,
        Some(&StateNodeKind::Fork) | Some(&StateNodeKind::Join)
    ) {
        let source_above = from_center_y <= to_center_y;
        anchors.0 = from_center_x;
        anchors.1 = if source_above {
            from.y + from.h
        } else {
            from.y
        };
        anchors.2 = from_center_x.clamp(to.x, to.x + to.w);
        anchors.3 = if source_above { to.y } else { to.y + to.h };
    }

    if matches!(from_kind, Some(&StateNodeKind::Choice)) {
        (anchors.0, anchors.1) = diamond_anchor(from, to_center_x, to_center_y);
    }

    if matches!(to_kind, Some(&StateNodeKind::Choice)) {
        (anchors.2, anchors.3) = diamond_anchor(to, from_center_x, from_center_y);
    }

    anchors
}

pub(super) fn diamond_anchor(node: &PlacedNode, toward_x: i32, toward_y: i32) -> (i32, i32) {
    let cx = node.x + node.w / 2;
    let cy = node.y + node.h / 2;
    let dx = toward_x - cx;
    let dy = toward_y - cy;
    if dx == 0 && dy == 0 {
        return (cx, cy);
    }

    let half_w = (node.w / 2).max(1) as f64;
    let half_h = (node.h / 2).max(1) as f64;
    let scale =
        1.0 / (((dx.abs() as f64) / half_w) + ((dy.abs() as f64) / half_h)).max(f64::EPSILON);
    (
        cx + ((dx as f64) * scale).round() as i32,
        cy + ((dy as f64) * scale).round() as i32,
    )
}

pub(super) fn state_node_kind_name(kind: &StateNodeKind) -> &'static str {
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

pub(super) fn state_dash_attr(dashed: bool) -> &'static str {
    if dashed {
        " stroke-dasharray=\"5 3\""
    } else {
        ""
    }
}

pub(super) fn state_hidden_attr(hidden: bool) -> &'static str {
    if hidden {
        " visibility=\"hidden\""
    } else {
        ""
    }
}

pub(super) fn state_direction_attr(direction: Option<&str>) -> String {
    direction
        .map(|d| format!(" data-state-direction=\"{}\"", escape_text(d)))
        .unwrap_or_default()
}

/// Render a single state node (and its children recursively).
#[allow(clippy::too_many_arguments)]
pub(super) fn render_node<'a>(
    out: &mut String,
    node: &'a StateNode,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    state_style: &crate::theme::StateStyle,
    incoming_count: usize,
    outgoing_count: usize,
    placed: &std::collections::BTreeMap<String, PlacedNode>,
    incoming: &std::collections::BTreeMap<&str, usize>,
    outgoing: &std::collections::BTreeMap<&str, usize>,
    all_transitions: &'a [StateTransition],
    edge_set: &std::collections::BTreeSet<(&'a str, &'a str)>,
    node_kinds: &std::collections::BTreeMap<&'a str, &'a StateNodeKind>,
    occupied_label_bounds: &mut Vec<LabelBounds>,
) {
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
            if incoming_count > 0 && outgoing_count == 0 {
                // End variant: outer ring + inner dot
                out.push_str(&format!(
                    "<circle class=\"puml-node\" {} cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                    puml_state_node_attrs(node, x, y, w, h),
                    cx, cy, r, state_style.background_color, state_style.border_color
                ));
                out.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"7\" fill=\"{}\"/>",
                    cx, cy, state_style.start_color
                ));
            } else {
                // Start variant: filled circle
                out.push_str(&format!(
                    "<circle class=\"puml-node\" {} cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\"/>",
                    puml_state_node_attrs(node, x, y, w, h),
                    cx,
                    cy,
                    r,
                    state_style.start_color
                ));
            }
        }

        StateNodeKind::End => {
            let cx = x + w / 2;
            let cy = y + h / 2;
            out.push_str(&format!(
                "<circle class=\"puml-node\" {} cx=\"{}\" cy=\"{}\" r=\"12\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                puml_state_node_attrs(node, x, y, w, h),
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
                "<circle class=\"puml-node\" {} cx=\"{}\" cy=\"{}\" r=\"16\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                puml_state_node_attrs(node, x, y, w, h),
                cx, cy, state_style.background_color, state_style.border_color
            ));
            let label_bounds = state_label_bounds_centered(cx, cy, label, 13);
            out.push_str(&format!(
                "<text class=\"puml-label\" {} x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" text-anchor=\"middle\" dominant-baseline=\"middle\" fill=\"{}\">{}</text>",
                puml_state_label_attrs(&node.name, "node-label", label_bounds),
                cx, cy, state_style.font_color, escape_text(label)
            ));
        }

        StateNodeKind::Fork | StateNodeKind::Join => {
            // UML spec: thick horizontal bar; no text label
            let bar_h = 8i32;
            out.push_str(&format!(
                "<rect class=\"puml-node\" {} x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
                puml_state_node_attrs(node, x, y, w, h),
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
                "<polygon class=\"puml-node\" {} points=\"{},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                puml_state_node_attrs(node, x, y, w, h),
                cx, cy - r,
                cx + r, cy,
                cx, cy + r,
                cx - r, cy,
                state_style.background_color, state_style.border_color
            ));
        }

        StateNodeKind::Normal => {
            let has_children = node.regions.iter().any(|r| !r.is_empty());
            let display = node.display.as_deref().unwrap_or(&node.name);

            if has_children {
                // ── Composite state ──────────────────────────────────────────
                // Draw the enclosing rounded-rect box
                out.push_str(&format!(
                    "<rect class=\"puml-node\" {} x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" ry=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                    puml_state_node_attrs(node, x, y, w, h),
                    x, y, w, h, state_style.background_color, state_style.border_color
                ));
                // Composite name label at top-center
                let title_y = y + 20;
                let title_bounds = state_label_bounds_centered(x + w / 2, title_y, display, 13);
                out.push_str(&format!(
                    "<text class=\"puml-label\" {} x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" text-anchor=\"middle\" fill=\"{}\">{}</text>",
                    puml_state_label_attrs(&node.name, "node-label", title_bounds),
                    x + w / 2, title_y, state_style.font_color, escape_text(display)
                ));

                // Draw concurrent region dividers (dashed vertical lines)
                if node.regions.len() > 1 {
                    for ri in 0..node.regions.len() - 1 {
                        let prev_right = node.regions[ri]
                            .iter()
                            .filter_map(|child| placed.get(&child.name))
                            .map(|child| child.x + child.w)
                            .max();
                        let next_left = node.regions[ri + 1]
                            .iter()
                            .filter_map(|child| placed.get(&child.name))
                            .map(|child| child.x)
                            .min();
                        if let (Some(prev_right), Some(next_left)) = (prev_right, next_left) {
                            let div_x = (prev_right + next_left) / 2;
                            let div_top = y + COMPOSITE_PAD_Y - 8;
                            let div_bot = y + h - COMPOSITE_PAD_BOT + 4;
                            if div_top < div_bot {
                                out.push_str(&format!(
                                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"5 3\"/>",
                                    div_x, div_top, div_x, div_bot, state_style.border_color
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
                let mut inner_placed: std::collections::BTreeMap<String, PlacedNode> = placed
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
                for t in all_transitions {
                    if !child_names.contains(t.from.as_str())
                        || !child_names.contains(t.to.as_str())
                    {
                        continue;
                    }
                    let from_p = placed.get(&t.from);
                    let to_p = placed.get(&t.to);
                    if let (Some(fp), Some(tp)) = (from_p, to_p) {
                        let has_reverse =
                            t.from != t.to && edge_set.contains(&(t.to.as_str(), t.from.as_str()));
                        let (x1, y1, x2, y2) = edge_anchors_for_kinds(
                            node_kinds.get(t.from.as_str()).copied(),
                            fp,
                            node_kinds.get(t.to.as_str()).copied(),
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
                            let cpx = x1 + 18;
                            let cpy = y1 - 14;
                            out.push_str(&format!(
                                "<path class=\"state-transition puml-edge\" {} data-state-from=\"{}\" data-state-to=\"{}\" d=\"M {x1} {y1} Q {cpx} {cpy} {x2} {y2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}{} marker-end=\"url(#arrow)\"/>",
                                puml_state_edge_attrs(&t.from, &t.to),
                                escape_text(&t.from), escape_text(&t.to), stroke, sw, dash, hidden, dir
                            ));
                        } else if has_reverse {
                            let (ox1, oy1, ox2, oy2) = offset_parallel_edge(x1, y1, x2, y2, 10);
                            let cpx = (ox1 + ox2) / 2;
                            let cpy = (oy1 + oy2) / 2;
                            out.push_str(&format!(
                                "<path class=\"state-transition puml-edge\" {} data-state-from=\"{}\" data-state-to=\"{}\" d=\"M {} {} Q {} {} {} {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}{} marker-end=\"url(#arrow)\"/>",
                                puml_state_edge_attrs(&t.from, &t.to),
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
                                    &state_edge_id(&t.from, &t.to),
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
                            );
                        }
                        if let Some(label) = &t.label {
                            let layout = place_state_transition_label(
                                label,
                                x1,
                                y1,
                                x2,
                                y2,
                                &inner_placed,
                                occupied_label_bounds,
                            );
                            render_state_transition_label(
                                out,
                                &layout,
                                &state_edge_id(&t.from, &t.to),
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
                        if let Some(cp) = placed.get(&child.name) {
                            let c_inc = *incoming.get(child.name.as_str()).unwrap_or(&0);
                            let c_out = *outgoing.get(child.name.as_str()).unwrap_or(&0);
                            render_node(
                                out,
                                child,
                                cp.x,
                                cp.y,
                                cp.w,
                                cp.h,
                                state_style,
                                c_inc,
                                c_out,
                                placed,
                                incoming,
                                outgoing,
                                all_transitions,
                                edge_set,
                                node_kinds,
                                occupied_label_bounds,
                            );
                        }
                    }
                }
            } else {
                // ── Simple state box ─────────────────────────────────────────
                out.push_str(&format!(
                    "<rect class=\"puml-node\" {} x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" ry=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                    puml_state_node_attrs(node, x, y, w, h),
                    x, y, w, h, state_style.background_color, state_style.border_color
                ));
                let label_y = y + 24;
                let label_bounds = state_label_bounds_centered(x + w / 2, label_y, display, 13);
                out.push_str(&format!(
                    "<text class=\"puml-label\" {} x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" text-anchor=\"middle\" fill=\"{}\">{}</text>",
                    puml_state_label_attrs(&node.name, "node-label", label_bounds),
                    x + w / 2, label_y, state_style.font_color, escape_text(display)
                ));
                // Internal actions
                if !node.internal_actions.is_empty() {
                    out.push_str(&format!(
                        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                        x, y + STATE_NODE_H - 4, x + w, y + STATE_NODE_H - 4, state_style.border_color
                    ));
                    for (ai, action) in node.internal_actions.iter().enumerate() {
                        let ay = y + STATE_NODE_H + ai as i32 * 14;
                        let text = if action.action.is_empty() {
                            action.kind.clone()
                        } else {
                            format!("{} / {}", action.kind, action.action)
                        };
                        let action_x = x + 6;
                        let action_y = ay + 10;
                        let action_bounds = state_label_bounds_left(action_x, action_y, &text, 10);
                        out.push_str(&format!(
                            "<text class=\"puml-label\" {} x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" font-style=\"italic\" fill=\"{}\">{}</text>",
                            puml_state_label_attrs(&node.name, "state-action", action_bounds),
                            action_x, action_y, state_style.font_color, escape_text(&text)
                        ));
                    }
                }
            }
        }
    }
}
