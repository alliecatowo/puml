use crate::model::{FamilyDocument, FamilyNodeKind};
use crate::render::svg::escape_text;
use crate::render::text_metrics::wrap_line_by_chars;
use crate::theme::ActivityStyle;

use super::arrows::{emit_activity_arrow, ActivityArrowStyle, NodeBbox};
use super::layout::{previous_activity_flow_node, NodeLayout, NodeMeta};

// ---------------------------------------------------------------------------
// Pass 2: render nodes (SVG shapes) for one node index
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub(super) fn render_node(
    out: &mut String,
    doc: &FamilyDocument,
    i: usize,
    node_layouts: &[NodeLayout],
    metas: &[NodeMeta],
    hidden_nodes: &std::collections::BTreeSet<usize>,
    fork_bar_half_widths: &std::collections::BTreeMap<usize, i32>,
    act_style: &ActivityStyle,
    box_w: i32,
) {
    let layout = &node_layouts[i];
    let cx = layout.cx;
    let y = layout.slot_y;
    let node = &doc.nodes[i];
    let label = node.label.clone().unwrap_or_default();
    let step_kind = &metas[i].step_kind;
    let fork_branch = metas[i].fork_branch;

    out.push_str(&format!(
        "<metadata data-activity-kind=\"{}\" data-activity-lane=\"{}\" data-activity-branch=\"{}\"/>",
        escape_text(step_kind),
        escape_text(&metas[i].lane_name),
        fork_branch
    ));

    if hidden_nodes.contains(&i) {
        return;
    }

    match node.kind {
        FamilyNodeKind::ActivityStart => {
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"12\" fill=\"{}\"/>",
                cx,
                y + 20,
                act_style.fork_color
            ));
        }
        FamilyNodeKind::ActivityStop => {
            match step_kind.as_str() {
                "Kill" => {
                    // Kill: circle with an X inside (PlantUML termination node)
                    let r = 12i32;
                    let cy = y + 20;
                    let d = (r as f64 * 0.65).round() as i32;
                    out.push_str(&format!(
                        "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"white\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                        cx, cy, r, escape_text(&act_style.fork_color)
                    ));
                    out.push_str(&format!(
                        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                        cx - d, cy - d, cx + d, cy + d, escape_text(&act_style.fork_color)
                    ));
                    out.push_str(&format!(
                        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                        cx + d, cy - d, cx - d, cy + d, escape_text(&act_style.fork_color)
                    ));
                }
                "Detach" => {
                    // Detach: a short horizontal bar (detach = silent end, no outgoing arrow)
                    let cy = y + 20;
                    out.push_str(&format!(
                        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"3\"/>",
                        cx - 12, cy, cx + 12, cy, escape_text(&act_style.fork_color)
                    ));
                }
                _ => {
                    // Stop / End: standard double-circle (bull's-eye)
                    out.push_str(&format!(
                        "<circle cx=\"{}\" cy=\"{}\" r=\"14\" fill=\"white\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                        cx,
                        y + 20,
                        act_style.fork_color
                    ));
                    out.push_str(&format!(
                        "<circle cx=\"{}\" cy=\"{}\" r=\"7\" fill=\"{}\"/>",
                        cx,
                        y + 20,
                        act_style.fork_color
                    ));
                    if !label.is_empty() {
                        out.push_str(&format!(
                            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"{}\">{}</text>",
                            cx,
                            y + 44,
                            escape_text(&act_style.font_color),
                            escape_text(&label)
                        ));
                    }
                }
            }
        }
        FamilyNodeKind::ActivityAction => {
            let fill = node
                .fill_color
                .as_deref()
                .unwrap_or(&act_style.background_color);
            if step_kind == "Connector" {
                out.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"16\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                    cx,
                    y + 22,
                    escape_text(fill),
                    escape_text(&act_style.border_color)
                ));
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
                    cx,
                    y + 26,
                    escape_text(&act_style.font_color),
                    escape_text(&label)
                ));
            } else {
                let sdl_shape = metas[i].sdl_shape.as_deref();
                emit_activity_action_box(
                    out,
                    cx,
                    y,
                    box_w,
                    &label,
                    fill,
                    &act_style.border_color,
                    &act_style.font_color,
                    sdl_shape,
                );
            }
        }
        FamilyNodeKind::Note => {
            if !metas[i].note_floating {
                render_activity_note_connector(out, doc, i, node_layouts, metas, box_w);
            }
            crate::render::family::render_note_card(
                out,
                cx - box_w / 2,
                y + 2,
                box_w,
                activity_note_card_height(&label),
                &label,
            );
        }
        FamilyNodeKind::ActivityDecision => {
            let condition_text = if let Some(idx) = label.find(" / ") {
                &label[..idx]
            } else {
                label.as_str()
            };
            let dx = 100;
            let dy = 22;
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx,
                y + 2,
                cx + dx,
                y + 2 + dy,
                cx,
                y + 2 + (dy * 2),
                cx - dx,
                y + 2 + dy,
                act_style.diamond_color,
                act_style.border_color
            ));
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
                cx,
                y + 2 + dy + 4,
                escape_text(&act_style.font_color),
                escape_text(condition_text)
            ));
        }
        FamilyNodeKind::ActivityFork | FamilyNodeKind::ActivityForkEnd => {
            if step_kind.contains("ForkAgain") {
                // ForkAgain nodes are layout bookmarks only; render nothing.
            } else {
                let bar_half = fork_bar_half_widths.get(&i).copied().unwrap_or(box_w / 2);
                if bar_half <= 0 {
                    return;
                }
                let bar_w = (bar_half * 2).max(box_w);
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"8\" fill=\"{}\"/>",
                    cx - bar_w / 2,
                    y + 24,
                    bar_w,
                    act_style.fork_color
                ));
            }
        }
        FamilyNodeKind::ActivityMerge => {
            if !(step_kind.contains("Else")
                || step_kind.contains("EndIf")
                || step_kind.contains("EndWhile")
                || step_kind.contains("RepeatStart"))
            {
                let merge_label = format!("(merge) {}", label);
                if !merge_label.is_empty() {
                    out.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
                        cx,
                        y + 28,
                        escape_text(&act_style.font_color),
                        escape_text(&merge_label)
                    ));
                }
            }
        }
        FamilyNodeKind::ActivityPartition => {}
        _ => {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\" fill=\"{}\">{}</text>",
                cx,
                y + 28,
                escape_text(&act_style.font_color),
                escape_text(&label)
            ));
        }
    }
}

pub(in crate::render::activity) fn activity_note_card_height(label: &str) -> i32 {
    let line_count = label.lines().count().max(1) as i32;
    (28 + line_count * 15).max(44)
}

// ---------------------------------------------------------------------------
// Predecessor-arrow emission for one node
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub(super) fn emit_predecessor_arrow(
    out: &mut String,
    doc: &FamilyDocument,
    i: usize,
    node_layouts: &[NodeLayout],
    metas: &[NodeMeta],
    suppress_prev_arrow: &std::collections::BTreeSet<usize>,
    act_style: &ActivityStyle,
    bboxes: &[NodeBbox],
) {
    if i == 0 {
        return;
    }
    if suppress_prev_arrow.contains(&i) {
        return;
    }
    if matches!(
        metas[i - 1].step_kind.as_str(),
        "Else" | "EndIf" | "EndWhile"
    ) {
        return;
    }

    let layout = &node_layouts[i];
    let cx = layout.cx;
    let y = layout.slot_y;

    // Walk back past zero-height partition markers to find the real predecessor
    let mut prev_idx = i - 1;
    while prev_idx > 0 {
        let is_invisible_control =
            super::layout::is_activity_flow_neutral_node(doc, metas, prev_idx);
        if !is_invisible_control {
            break;
        }
        prev_idx -= 1;
    }

    let current_is_note = matches!(doc.nodes[i].kind, FamilyNodeKind::Note);
    if !current_is_note
        && matches!(
            metas[prev_idx].step_kind.as_str(),
            "Stop" | "End" | "Kill" | "Detach"
        )
    {
        return;
    }

    let arrow_style = (prev_idx + 1..i)
        .rev()
        .find_map(|idx| metas[idx].arrow_style.as_ref());
    let branch_guard_style;
    // #1447: the predecessor arrow from an IfStart to the then-branch first node
    // should carry the *then* guard (e.g. "yes"), not the else guard.
    // `then_guard_for_if` extracts it from the IfStart node's label.
    let arrow_style = if arrow_style.is_none() && metas[prev_idx].step_kind == "IfStart" {
        branch_guard_style = then_guard_for_if(doc, prev_idx).map(|label| ActivityArrowStyle {
            label: Some(label.to_string()),
            ..ActivityArrowStyle::default()
        });
        branch_guard_style.as_ref()
    } else {
        arrow_style
    };
    let prev = &node_layouts[prev_idx];
    let (from_x, from_y) = if metas[prev_idx].step_kind == "IfStart" && prev.cx != cx {
        let side_x = if cx < prev.cx {
            prev.cx - 100
        } else {
            prev.cx + 100
        };
        (side_x, prev.slot_y + 24)
    } else {
        (prev.cx, prev.arrow_out_y)
    };
    // Skip zero-length arrows (same src and dst)
    if from_x != cx || from_y != y {
        if let Some(style) = arrow_style {
            super::arrows::emit_activity_arrow_with_style(
                out,
                from_x,
                from_y,
                cx,
                y,
                &act_style.arrow_color,
                style,
                bboxes,
            );
        } else {
            emit_activity_arrow(out, from_x, from_y, cx, y, &act_style.arrow_color, bboxes);
        }
    }
}

/// Routing-aware variant of [`emit_predecessor_arrow`].
///
/// Delegates to `emit_predecessor_arrow` for `Ortho` (legacy `<line>` segments);
/// uses `emit_activity_arrow_with_style_routed` for `Splines` and `Polyline`.
#[allow(clippy::too_many_arguments)]
pub(super) fn emit_predecessor_arrow_routed(
    out: &mut String,
    doc: &FamilyDocument,
    i: usize,
    node_layouts: &[NodeLayout],
    metas: &[NodeMeta],
    suppress_prev_arrow: &std::collections::BTreeSet<usize>,
    act_style: &ActivityStyle,
    bboxes: &[NodeBbox],
    routing: crate::render::graph_layout::EdgeRouting,
) {
    use crate::render::graph_layout::EdgeRouting;
    match routing {
        EdgeRouting::Ortho => {
            emit_predecessor_arrow(
                out,
                doc,
                i,
                node_layouts,
                metas,
                suppress_prev_arrow,
                act_style,
                bboxes,
            );
        }
        _ => {
            // Reproduce the same skip logic as emit_predecessor_arrow.
            if i == 0 {
                return;
            }
            if suppress_prev_arrow.contains(&i) {
                return;
            }
            if matches!(
                metas[i - 1].step_kind.as_str(),
                "Else" | "EndIf" | "EndWhile"
            ) {
                return;
            }

            let layout = &node_layouts[i];
            let cx = layout.cx;
            let y = layout.slot_y;

            let mut prev_idx = i - 1;
            while prev_idx > 0 {
                let is_invisible_control =
                    super::layout::is_activity_flow_neutral_node(doc, metas, prev_idx);
                if !is_invisible_control {
                    break;
                }
                prev_idx -= 1;
            }

            let current_is_note = matches!(doc.nodes[i].kind, FamilyNodeKind::Note);
            if !current_is_note
                && matches!(
                    metas[prev_idx].step_kind.as_str(),
                    "Stop" | "End" | "Kill" | "Detach"
                )
            {
                return;
            }

            let arrow_style = (prev_idx + 1..i)
                .rev()
                .find_map(|idx| metas[idx].arrow_style.as_ref());
            let branch_guard_style;
            // #1447: use the then-guard from the IfStart node for the then-branch
            // predecessor arrow, not the else guard.
            let arrow_style = if arrow_style.is_none() && metas[prev_idx].step_kind == "IfStart" {
                branch_guard_style =
                    then_guard_for_if(doc, prev_idx).map(|label| ActivityArrowStyle {
                        label: Some(label.to_string()),
                        ..ActivityArrowStyle::default()
                    });
                branch_guard_style.as_ref()
            } else {
                arrow_style
            };
            let prev = &node_layouts[prev_idx];
            let (from_x, from_y) = if metas[prev_idx].step_kind == "IfStart" && prev.cx != cx {
                let side_x = if cx < prev.cx {
                    prev.cx - 100
                } else {
                    prev.cx + 100
                };
                (side_x, prev.slot_y + 24)
            } else {
                (prev.cx, prev.arrow_out_y)
            };
            if from_x != cx || from_y != y {
                let eff_style = arrow_style
                    .cloned()
                    .unwrap_or_else(ActivityArrowStyle::default);
                super::arrows::emit_activity_arrow_with_style_routed(
                    out,
                    from_x,
                    from_y,
                    cx,
                    y,
                    &act_style.arrow_color,
                    &eff_style,
                    bboxes,
                    routing,
                );
            }
        }
    }
}

fn render_activity_note_connector(
    out: &mut String,
    doc: &FamilyDocument,
    note_idx: usize,
    node_layouts: &[NodeLayout],
    metas: &[NodeMeta],
    box_w: i32,
) {
    let Some(anchor_idx) = previous_activity_flow_node(doc, metas, note_idx) else {
        return;
    };
    let note = &node_layouts[note_idx];
    let anchor = &node_layouts[anchor_idx];
    let note_h =
        activity_note_card_height(doc.nodes[note_idx].label.as_deref().unwrap_or_default());
    let note_left = note.cx - box_w / 2;
    let note_right = note.cx + box_w / 2;
    let note_mid_y = note.slot_y + 24;
    let anchor_mid_y = anchor.slot_y + 22;
    let (x1, y1, x2, y2) = match metas[note_idx].note_side.as_deref() {
        Some("top") => (
            anchor.cx,
            anchor.slot_y + 4,
            note.cx,
            note.slot_y + 2 + note_h,
        ),
        Some("bottom") => (anchor.cx, anchor.slot_y + 40, note.cx, note.slot_y + 2),
        _ if note.cx < anchor.cx => (anchor.cx - box_w / 2, anchor_mid_y, note_right, note_mid_y),
        _ => (anchor.cx + box_w / 2, anchor_mid_y, note_left, note_mid_y),
    };
    out.push_str(&format!(
        "<line class=\"activity-note-connector\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#8a6d1d\" stroke-width=\"1\" stroke-dasharray=\"4 3\"/>",
        x1, y1, x2, y2
    ));
}

/// Return the *then*-branch guard label for an `IfStart` node.
///
/// `if (Cond?) then (yes)` is stored in the IfStart label as `"Cond? / yes"`.
/// `activity_decision_guard` splits on " / " and returns the part after the
/// slash.  We replicate that logic inline to avoid a cross-module import.
///
/// Used by the predecessor-arrow pass to label the direct flow from the diamond
/// to the then-branch first node.
fn then_guard_for_if(doc: &FamilyDocument, if_idx: usize) -> Option<&str> {
    // The IfStart label is "<condition> / <then_guard>" when a `then (guard)` is
    // present.  Split on " / " and take the part after the slash.
    let label = doc.nodes[if_idx].label.as_deref()?;
    label.split_once(" / ").map(|(_, guard)| guard.trim())
}

// ---------------------------------------------------------------------------
// SDL action box shape rendering
// ---------------------------------------------------------------------------

/// Emit an activity action box with optional SDL terminator shape.
///
/// SDL shape variants:
///   `None` / default — standard rounded rectangle (`;` terminator)
///   `"send"`         — right-pointing chevron (`>` terminator, send in SDL)
///   `"receive"`      — left-pointing chevron (`<` terminator, receive in SDL)
///   `"input"`        — parallelogram slanting right (`/` terminator)
///   `"output"`       — parallelogram slanting left (`\` terminator)
///   `"bar"`          — rectangle with no rounded corners (`|` terminator)
///   `"bracket"`      — flat-capped rectangle (`]` terminator)
///   `"brace"`        — stadium/hexagon shape (`}` terminator)
#[allow(clippy::too_many_arguments)]
pub(super) fn emit_activity_action_box(
    out: &mut String,
    cx: i32,
    y: i32,
    box_w: i32,
    label: &str,
    fill: &str,
    border_color: &str,
    font_color: &str,
    sdl_shape: Option<&str>,
) {
    // #1550: wrap label text to fit the box width.
    // Monospace font-size 12 ≈ 7.3 px/char; allow a 4px margin on each side.
    let char_w_px = 7i32;
    let inner_w = (box_w - 8).max(1);
    let max_chars = ((inner_w / char_w_px) as usize).max(1);
    let wrapped_lines = wrap_line_by_chars(label, max_chars);
    let n_lines = wrapped_lines.len() as i32;

    // Box height is always 36px so the layout slot (step_h=44) is never
    // violated.  Tighter line-height (12px) keeps 2–3 lines within 36px.
    let h = 36i32;
    let line_h = 12i32;

    let x = cx - box_w / 2;
    let top = y + 4;
    let bottom = top + h;
    // Vertically centre the text block inside the box.
    let text_block_h = n_lines * line_h;
    let first_line_y = top + (h - text_block_h) / 2 + line_h - 2;

    match sdl_shape {
        None => {
            // Standard rounded rectangle
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"18\" ry=\"18\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                x, top, box_w, h, escape_text(fill), escape_text(border_color)
            ));
        }
        Some("send") => {
            // Right-pointing chevron: rectangle with a right-side point
            let tip_x = cx + box_w / 2 + 12;
            let mid_y = top + h / 2;
            out.push_str(&format!(
                "<polygon points=\"{x},{top} {rx},{top} {tip},{mid} {rx},{bot} {x},{bot}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                escape_text(fill), escape_text(border_color),
                rx = cx + box_w / 2, tip = tip_x, mid = mid_y, bot = bottom
            ));
        }
        Some("receive") => {
            // Left-pointing chevron: rectangle with a left-side notch
            let notch_x = cx - box_w / 2 + 12;
            let mid_y = top + h / 2;
            out.push_str(&format!(
                "<polygon points=\"{lx},{top} {rx},{top} {rx},{bot} {lx},{bot} {notch},{mid}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                escape_text(fill), escape_text(border_color),
                lx = x, rx = cx + box_w / 2, notch = notch_x, mid = mid_y, bot = bottom
            ));
        }
        Some("input") => {
            // Parallelogram slanting right (input: left side offset up)
            let offset = 10i32;
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                x + offset, top,
                cx + box_w / 2 + offset, top,
                cx + box_w / 2, bottom,
                x, bottom,
                escape_text(fill), escape_text(border_color)
            ));
        }
        Some("output") => {
            // Parallelogram slanting left (output: right side offset up)
            let offset = 10i32;
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                x, top,
                cx + box_w / 2, top,
                cx + box_w / 2 - offset, bottom,
                x - offset, bottom,
                escape_text(fill), escape_text(border_color)
            ));
        }
        Some("bar") => {
            // Simple rectangle (no rounding) with vertical bars at sides
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"0\" ry=\"0\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                x, top, box_w, h, escape_text(fill), escape_text(border_color)
            ));
        }
        Some("bracket") => {
            // Rectangle with flat (squared) caps — same as bar but different semantic
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" ry=\"4\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                x, top, box_w, h, escape_text(fill), escape_text(border_color)
            ));
        }
        Some("brace") => {
            // Hexagon / stadium shape: cut corners on left side
            let cut = 10i32;
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{} {},{} {},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                x + cut, top,
                cx + box_w / 2 - cut, top,
                cx + box_w / 2, top + cut,
                cx + box_w / 2, bottom - cut,
                cx + box_w / 2 - cut, bottom,
                x + cut, bottom,
                x, bottom - cut,
                x, top + cut,
                escape_text(fill), escape_text(border_color)
            ));
        }
        Some(_) => {
            // Unknown shape: fall back to rounded rectangle
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"18\" ry=\"18\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                x, top, box_w, h, escape_text(fill), escape_text(border_color)
            ));
        }
    }

    if n_lines == 1 {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\" fill=\"{}\">{}</text>",
            cx, first_line_y, escape_text(font_color), escape_text(&wrapped_lines[0])
        ));
    } else {
        out.push_str(&format!(
            "<text text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\" fill=\"{}\">",
            escape_text(font_color)
        ));
        for (idx, line) in wrapped_lines.iter().enumerate() {
            let ty = first_line_y + idx as i32 * line_h;
            out.push_str(&format!(
                "<tspan x=\"{}\" y=\"{}\">{}</tspan>",
                cx,
                ty,
                escape_text(line)
            ));
        }
        out.push_str("</text>");
    }
}
