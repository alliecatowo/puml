use super::*;

pub(super) fn wrap_state_label(label: &str, max_cols: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();

    for explicit in split_state_label_explicit_lines(label) {
        let mut current = String::new();

        for word in explicit.split_whitespace() {
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
        } else if explicit.is_empty() {
            lines.push(String::new());
        }
    }

    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn split_state_label_explicit_lines(label: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut chars = label.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some('n' | 'r' | 'l') = chars.peek().copied() {
                chars.next();
                lines.push(std::mem::take(&mut current));
                continue;
            }
        }
        current.push(ch);
    }

    lines.push(current);
    lines
}

pub(super) fn measure_state_label(lines: &[String]) -> (i32, i32) {
    // Width uses the shared monospace metric (7 px/char) rather than the local
    // STATE_LABEL_CHAR_W constant copy — arithmetic is byte-identical because
    // DEFAULT_MONOSPACE_CHAR_WIDTH == STATE_LABEL_CHAR_W == 7.
    let max_line = lines
        .iter()
        .map(String::as_str)
        .max_by_key(|line| line.chars().count())
        .unwrap_or("");
    let width = crate::render::text_metrics::default_monospace_width(max_line).max(24);
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
    let label_half_extent_on_normal = nx.abs() * (w as f64 / 2.0) + ny.abs() * (h as f64 / 2.0);
    let min_normal_offset = (label_half_extent_on_normal + 8.0).max(18.0);
    let normal_offsets = [
        min_normal_offset,
        min_normal_offset + 12.0,
        min_normal_offset + 24.0,
        min_normal_offset + 38.0,
        min_normal_offset + 54.0,
        min_normal_offset + 74.0,
        min_normal_offset + 98.0,
        min_normal_offset + 122.0,
        min_normal_offset + 150.0,
    ];

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
    original_label: &str,
    font_color: &str,
) {
    out.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\" text-anchor=\"middle\" data-state-label=\"{}\">",
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
