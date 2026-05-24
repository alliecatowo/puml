use super::layout_constants::{
    ACTIVITY_BASE_LANE_WIDTH, ACTIVITY_BRANCH_X_OFFSET, ACTIVITY_LANE_AREA_X, ACTIVITY_STEP_HEIGHT,
};
use super::svg::escape_text;
use crate::model::{FamilyDocument, FamilyNodeKind, FamilyStyle};
use crate::theme::ActivityStyle;

mod arrows;
mod branches;
mod layout;
mod nodes;
mod swimlanes;

pub fn render_activity_svg(doc: &FamilyDocument) -> String {
    // -----------------------------------------------------------------------
    // 1. Style + global metrics
    // -----------------------------------------------------------------------
    let act_style = match &doc.family_style {
        Some(FamilyStyle::Activity(s)) => s.clone(),
        _ => ActivityStyle::default(),
    };

    let step_h = ACTIVITY_STEP_HEIGHT;
    let title_lines = doc
        .title
        .as_deref()
        .map(|v| v.lines().count() as i32)
        .unwrap_or(0);
    let header_h = 40 + title_lines * 22;

    // -----------------------------------------------------------------------
    // 2. Pass 0 — parse node metadata
    // -----------------------------------------------------------------------
    let metas = layout::parse_node_metas(doc);

    // -----------------------------------------------------------------------
    // 3. Collect swim-lanes
    // -----------------------------------------------------------------------
    let mut lanes: Vec<String> = Vec::new();
    for meta in &metas {
        if meta.lane_name != "default" && !lanes.iter().any(|l| l == &meta.lane_name) {
            lanes.push(meta.lane_name.clone());
        }
    }
    if lanes.is_empty() {
        lanes.push("default".to_string());
    }
    let mut lane_fills = std::collections::BTreeMap::new();
    for (node, meta) in doc.nodes.iter().zip(metas.iter()) {
        if meta.step_kind == "PartitionStart" && meta.lane_name != "default" {
            if let Some(fill) = &node.fill_color {
                lane_fills
                    .entry(meta.lane_name.clone())
                    .or_insert(fill.clone());
            }
        }
    }

    // -----------------------------------------------------------------------
    // 4. Canvas sizing
    // -----------------------------------------------------------------------
    // Count max nesting depth of if/else to estimate extra width needed.
    let mut max_if_depth: i32 = 0;
    {
        let mut depth: i32 = 0;
        for meta in &metas {
            if meta.step_kind == "IfStart" {
                depth += 1;
                max_if_depth = max_if_depth.max(depth);
            } else if meta.step_kind == "EndIf" {
                depth = depth.saturating_sub(1);
            }
        }
    }
    // Count max fork branch count to size canvas for parallel columns.
    let mut max_fork_branches: i32 = 0;
    {
        let mut count: i32 = 0;
        for meta in &metas {
            if meta.step_kind == "Fork" {
                count = 1;
            } else if meta.step_kind == "ForkAgain" {
                count += 1;
                max_fork_branches = max_fork_branches.max(count);
            } else if meta.step_kind == "EndFork" {
                count = 0;
            }
        }
    }
    let branch_x_offset = ACTIVITY_BRANCH_X_OFFSET;
    let extra_branch_width = 2 * branch_x_offset * max_if_depth;
    let extra_fork_width = (max_fork_branches * ACTIVITY_BRANCH_X_OFFSET).max(0);

    let has_left_notes = metas
        .iter()
        .any(|meta| meta.step_kind == "Note" && meta.note_side.as_deref() == Some("left"));
    let has_right_notes = metas
        .iter()
        .any(|meta| meta.step_kind == "Note" && meta.note_side.as_deref() != Some("left"));
    let side_note_margin = 260;
    let lane_area_x = ACTIVITY_LANE_AREA_X + if has_left_notes { side_note_margin } else { 0 };
    let base_lane_area_w = ACTIVITY_BASE_LANE_WIDTH;
    let lane_area_w = base_lane_area_w + extra_branch_width + extra_fork_width;
    let width = lane_area_x + lane_area_w + 32 + if has_right_notes { side_note_margin } else { 0 };
    let has_named_lanes = lanes.iter().any(|l| l != "default");
    let has_partition_markers = metas.iter().any(|meta| meta.step_kind == "PartitionStart");
    let has_partition_blocks = metas.iter().any(|meta| meta.step_kind == "PartitionEnd");
    // `partition Name { ... }` is a stacked group, while open-ended `|Lane|`
    // markers keep their existing lane-column behavior.
    let stacked_partition_blocks = has_named_lanes && has_partition_blocks;
    let lane_w = if stacked_partition_blocks {
        lane_area_w
    } else {
        (lane_area_w / (lanes.len() as i32)).max(120)
    };

    let lane_index = |name: &str| -> i32 {
        lanes
            .iter()
            .position(|l| l == name)
            .map(|i| i as i32)
            .unwrap_or(0)
    };
    let lane_center_x = |lane_name: &str| -> i32 {
        if stacked_partition_blocks {
            return lane_area_x + lane_area_w / 2;
        }
        let idx = lane_index(lane_name);
        lane_area_x + idx * lane_w + lane_w / 2
    };

    let lane_header_h = if has_named_lanes { 24i32 } else { 0i32 };
    let sequential_partition_lanes = has_named_lanes && has_partition_markers;

    let fork_col_w = (lane_w / 2).max(160i32);
    let box_w = (lane_w - 24).clamp(120, 220);

    // -----------------------------------------------------------------------
    // 5. Pass 1 — layout
    // -----------------------------------------------------------------------
    let layout_result = layout::compute_layout(
        doc,
        &metas,
        &layout::LayoutParams {
            header_h,
            lane_header_h,
            step_h,
            branch_x_offset,
            fork_col_w,
            lane_w,
            lane_center_x: &lane_center_x,
        },
    );
    let layout::LayoutResult {
        mut node_layouts,
        fork_bar_half_widths,
        extra_arrows,
        direct_arrows,
        mut suppress_prev_arrow,
    } = layout_result;

    // -----------------------------------------------------------------------
    // 6. Hidden-node deduplication pass
    // -----------------------------------------------------------------------
    let hidden_nodes =
        branches::compute_hidden_nodes(doc, &metas, &mut node_layouts, &mut suppress_prev_arrow);

    // -----------------------------------------------------------------------
    // 7. Extra-arrow redirect pass
    // -----------------------------------------------------------------------
    let redirected_extra_arrows =
        branches::redirect_extra_arrows(doc, &metas, &node_layouts, extra_arrows, &hidden_nodes);

    // -----------------------------------------------------------------------
    // 8. Canvas height + lane spans
    // -----------------------------------------------------------------------
    let height = node_layouts
        .iter()
        .map(|l| l.next_slot_y)
        .max()
        .unwrap_or(header_h + step_h)
        + 60;

    let lane_spans = if sequential_partition_lanes {
        swimlanes::compute_lane_spans(
            doc,
            &metas,
            &node_layouts,
            &lanes,
            &lane_index,
            lane_header_h,
            header_h,
            height,
        )
    } else {
        vec![None; lanes.len()]
    };

    // -----------------------------------------------------------------------
    // 9. Build obstacle bboxes for arrow routing (#734).
    //
    // Collect the bounding boxes of every visible node so that
    // emit_activity_arrow can choose a mid_y that does not cross any node body.
    // -----------------------------------------------------------------------
    let node_bboxes: Vec<arrows::NodeBbox> = doc
        .nodes
        .iter()
        .zip(node_layouts.iter())
        .zip(metas.iter())
        .filter_map(|((node, layout), meta)| {
            let cx = layout.cx;
            let y = layout.slot_y;
            match node.kind {
                FamilyNodeKind::ActivityAction | FamilyNodeKind::Note => Some(arrows::NodeBbox {
                    left: cx - box_w / 2,
                    top: y + 4,
                    right: cx + box_w / 2,
                    bottom: y + 40,
                }),
                FamilyNodeKind::ActivityDecision => Some(arrows::NodeBbox {
                    left: cx - 100,
                    top: y + 2,
                    right: cx + 100,
                    bottom: y + 46,
                }),
                FamilyNodeKind::ActivityStart => Some(arrows::NodeBbox {
                    left: cx - 12,
                    top: y + 8,
                    right: cx + 12,
                    bottom: y + 32,
                }),
                FamilyNodeKind::ActivityStop => Some(arrows::NodeBbox {
                    left: cx - 14,
                    top: y + 6,
                    right: cx + 14,
                    bottom: y + 34,
                }),
                FamilyNodeKind::ActivityFork | FamilyNodeKind::ActivityForkEnd => {
                    if meta.step_kind.contains("ForkAgain") {
                        None
                    } else {
                        Some(arrows::NodeBbox {
                            left: cx - box_w / 2,
                            top: y + 24,
                            right: cx + box_w / 2,
                            bottom: y + 32,
                        })
                    }
                }
                _ => None,
            }
        })
        .collect();

    // -----------------------------------------------------------------------
    // 10. Emit SVG
    // -----------------------------------------------------------------------
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        width, height, width, height
    ));
    out.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>",
        escape_text(&act_style.background_color)
    ));

    // Title block
    let mut y_cursor = 28;
    if let Some(title) = &doc.title {
        for line in title.lines() {
            out.push_str(&format!(
                "<text x=\"32\" y=\"{}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\" fill=\"{}\">{}</text>",
                y_cursor,
                escape_text(&act_style.font_color),
                escape_text(line)
            ));
            y_cursor += 22;
        }
    }
    out.push_str(&format!(
        "<text x=\"32\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"{}\">activity diagram</text>",
        y_cursor + 2,
        escape_text(&act_style.font_color)
    ));

    // Swim-lane backgrounds + headers
    swimlanes::emit_lanes(
        &mut out,
        &lanes,
        &lane_spans,
        sequential_partition_lanes,
        lane_area_x,
        lane_w,
        stacked_partition_blocks,
        header_h,
        lane_header_h,
        height,
        &act_style,
        &lane_fills,
    );

    // Pass 2: nodes + arrows
    for i in 0..doc.nodes.len() {
        nodes::render_node(
            &mut out,
            doc,
            i,
            &node_layouts,
            &metas,
            &hidden_nodes,
            &fork_bar_half_widths,
            &act_style,
            box_w,
        );

        nodes::emit_predecessor_arrow(
            &mut out,
            doc,
            i,
            &node_layouts,
            &metas,
            &suppress_prev_arrow,
            &act_style,
            &node_bboxes,
        );

        // Extra arrows for if-branching that target this node
        let layout = &node_layouts[i];
        arrows::emit_extra_arrows(
            &mut out,
            &redirected_extra_arrows,
            layout.cx,
            layout.slot_y,
            &act_style.arrow_color,
            &node_bboxes,
        );
    }

    // Direct arrows: fork-bar→branch and branch→join-bar
    arrows::emit_direct_arrows(
        &mut out,
        &direct_arrows,
        &act_style.arrow_color,
        &node_bboxes,
    );

    out.push_str("</svg>");
    out
}
