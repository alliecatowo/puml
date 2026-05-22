use crate::model::{FamilyDocument, FamilyNodeKind};
use crate::render::svg::escape_text;
use crate::theme::ActivityStyle;

use super::arrows::{emit_activity_arrow, NodeBbox};
use super::layout::{NodeLayout, NodeMeta};

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
    hidden_nodes: &std::collections::HashSet<usize>,
    fork_bar_half_widths: &std::collections::HashMap<usize, i32>,
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
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"36\" rx=\"18\" ry=\"18\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                    cx - box_w / 2,
                    y + 4,
                    box_w,
                    escape_text(fill),
                    escape_text(&act_style.border_color)
                ));
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\" fill=\"{}\">{}</text>",
                    cx,
                    y + 27,
                    escape_text(&act_style.font_color),
                    escape_text(&label)
                ));
            }
        }
        FamilyNodeKind::Note => {
            crate::render::family::render_note_card(out, cx - box_w / 2, y + 2, box_w, 44, &label);
        }
        FamilyNodeKind::ActivityDecision => {
            let (condition_text, then_guard) = if let Some(idx) = label.find(" / ") {
                (&label[..idx], Some(&label[idx + 3..]))
            } else {
                (label.as_str(), None)
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
            if let Some(guard) = then_guard {
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"start\" font-family=\"monospace\" font-size=\"10\" fill=\"{}\">{}</text>",
                    cx + dx + 4,
                    y + 2 + dy + 4,
                    escape_text(&act_style.font_color),
                    escape_text(guard)
                ));
            }
        }
        FamilyNodeKind::ActivityFork | FamilyNodeKind::ActivityForkEnd => {
            if step_kind.contains("ForkAgain") {
                // ForkAgain nodes are layout bookmarks only; render nothing.
            } else {
                let bar_half = fork_bar_half_widths.get(&i).copied().unwrap_or(box_w / 2);
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
    suppress_prev_arrow: &std::collections::HashSet<usize>,
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
        let prev_kind = &doc.nodes[prev_idx].kind;
        let prev_step = &metas[prev_idx].step_kind;
        let is_invisible_control = (matches!(prev_kind, FamilyNodeKind::ActivityPartition)
            && (prev_step == "PartitionStart"
                || prev_step == "PartitionEnd"
                || prev_step == "Arrow"
                || prev_step == "OldStyle"))
            || prev_step == "RepeatStart";
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
    let prev = &node_layouts[prev_idx];
    // Skip zero-length arrows (same src and dst)
    if prev.cx != cx || prev.arrow_out_y != y {
        if let Some(style) = arrow_style {
            super::arrows::emit_activity_arrow_with_style(
                out,
                prev.cx,
                prev.arrow_out_y,
                cx,
                y,
                &act_style.arrow_color,
                style,
                bboxes,
            );
        } else {
            emit_activity_arrow(
                out,
                prev.cx,
                prev.arrow_out_y,
                cx,
                y,
                &act_style.arrow_color,
                bboxes,
            );
        }
    }
}
