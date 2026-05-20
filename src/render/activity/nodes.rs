use crate::model::{FamilyDocument, FamilyNodeKind};
use crate::render::scene_graph::{estimate_text_bbox, Rect as SceneRect};
use crate::render::svg::escape_text;
use crate::render::{puml_label_attrs, puml_node_attrs};
use crate::theme::ActivityStyle;

use super::arrows::{emit_activity_arrow, ArrowGeometry, EdgeSemantic, NodeBbox};
use super::layout::{NodeLayout, NodeMeta};

fn activity_node_id(i: usize) -> String {
    format!("activity-node-{i}")
}

fn activity_bbox(left: i32, top: i32, width: i32, height: i32) -> SceneRect {
    SceneRect::new(left as f64, top as f64, width as f64, height as f64)
}

fn activity_node_attrs(i: usize, kind: &str, bbox: SceneRect) -> String {
    puml_node_attrs(&activity_node_id(i), "activity", kind, bbox)
}

fn activity_label_attrs(
    i: usize,
    kind: &str,
    x: i32,
    y: i32,
    text: &str,
    font_size: f64,
) -> String {
    puml_label_attrs(
        &activity_node_id(i),
        kind,
        estimate_text_bbox(x as f64, y as f64, text, font_size, true),
    )
}

fn activity_start_label_attrs(
    i: usize,
    kind: &str,
    x: i32,
    y: i32,
    text: &str,
    font_size: f64,
) -> String {
    puml_label_attrs(
        &activity_node_id(i),
        kind,
        estimate_text_bbox(x as f64, y as f64, text, font_size, false),
    )
}

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
            let attrs = activity_node_attrs(i, "start", activity_bbox(cx - 12, y + 8, 24, 24));
            out.push_str(&format!(
                "<circle class=\"activity-start puml-node\" {} cx=\"{}\" cy=\"{}\" r=\"12\" fill=\"{}\"/>",
                attrs,
                cx,
                y + 20,
                act_style.fork_color
            ));
        }
        FamilyNodeKind::ActivityStop => {
            let attrs = activity_node_attrs(i, "end", activity_bbox(cx - 14, y + 6, 28, 28));
            out.push_str(&format!(
                "<circle class=\"activity-end puml-node\" {} cx=\"{}\" cy=\"{}\" r=\"14\" fill=\"white\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                attrs,
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
                    "<text class=\"activity-label puml-label\" {} x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"{}\">{}</text>",
                    activity_label_attrs(i, "node-label", cx, y + 44, &label, 10.0),
                    cx,
                    y + 44,
                    escape_text(&act_style.font_color),
                    escape_text(&label)
                ));
            }
        }
        FamilyNodeKind::ActivityAction => {
            let attrs =
                activity_node_attrs(i, "action", activity_bbox(cx - box_w / 2, y + 4, box_w, 36));
            out.push_str(&format!(
                "<rect class=\"activity-action puml-node\" {} x=\"{}\" y=\"{}\" width=\"{}\" height=\"36\" rx=\"18\" ry=\"18\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                attrs,
                cx - box_w / 2,
                y + 4,
                box_w,
                act_style.background_color,
                act_style.border_color
            ));
            out.push_str(&format!(
                "<text class=\"activity-label puml-label\" {} x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\" fill=\"{}\">{}</text>",
                activity_label_attrs(i, "node-label", cx, y + 27, &label, 12.0),
                cx,
                y + 27,
                escape_text(&act_style.font_color),
                escape_text(&label)
            ));
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
            let attrs =
                activity_node_attrs(i, "decision", activity_bbox(cx - dx, y + 2, dx * 2, dy * 2));
            out.push_str(&format!(
                "<polygon class=\"activity-decision puml-node\" {} points=\"{},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                attrs,
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
                "<text class=\"activity-label puml-label\" {} x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
                activity_label_attrs(i, "condition", cx, y + 2 + dy + 4, condition_text, 11.0),
                cx,
                y + 2 + dy + 4,
                escape_text(&act_style.font_color),
                escape_text(condition_text)
            ));
            if let Some(guard) = then_guard {
                out.push_str(&format!(
                    "<text class=\"activity-label puml-label\" {} x=\"{}\" y=\"{}\" text-anchor=\"start\" font-family=\"monospace\" font-size=\"10\" fill=\"{}\">{}</text>",
                    activity_start_label_attrs(i, "guard", cx + dx + 4, y + 2 + dy + 4, guard, 10.0),
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
                        "<text class=\"activity-label puml-label\" {} x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
                        activity_label_attrs(i, "node-label", cx, y + 28, &merge_label, 11.0),
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
                "<text class=\"activity-label puml-label\" {} x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\" fill=\"{}\">{}</text>",
                activity_label_attrs(i, "node-label", cx, y + 28, &label, 12.0),
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
                || prev_step == "OldStyle"))
            || prev_step == "RepeatStart";
        if !is_invisible_control {
            break;
        }
        prev_idx -= 1;
    }

    let prev = &node_layouts[prev_idx];
    // Skip zero-length arrows (same src and dst)
    if prev.cx != cx || prev.arrow_out_y != y {
        let semantic = EdgeSemantic {
            id: format!("activity-edge-{prev_idx}-{i}"),
            kind: "control-flow",
            from: activity_node_id(prev_idx),
            to: activity_node_id(i),
        };
        emit_activity_arrow(
            out,
            &semantic,
            ArrowGeometry {
                x1: prev.cx,
                y1: prev.arrow_out_y,
                x2: cx,
                y2: y,
            },
            &act_style.arrow_color,
            bboxes,
        );
    }
}
