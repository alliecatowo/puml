use crate::render::svg::escape_text;
use crate::theme::ActivityStyle;

/// Emit lane background rectangles and header labels.
///
/// When `sequential_partition_lanes` is true the lane boxes are drawn only over
/// the span of nodes that belong to that lane (for sequential partition style).
/// Otherwise the boxes stretch the full diagram height.
///
/// `min_node_cx` holds the leftmost actual node cx for each lane (when
/// `sequential_partition_lanes` is true).  When fork branches place a lane's
/// nodes in an adjacent column, the lane background is extended leftward to
/// include those nodes so the frame visually contains its children.
#[allow(clippy::too_many_arguments)]
pub(super) fn emit_lanes(
    out: &mut String,
    lanes: &[String],
    lane_spans: &[Option<(i32, i32)>],
    min_node_cx: &[Option<i32>],
    sequential_partition_lanes: bool,
    lane_area_x: i32,
    lane_w: i32,
    box_w: i32,
    stacked_partition_blocks: bool,
    header_h: i32,
    lane_header_h: i32,
    height: i32,
    act_style: &ActivityStyle,
    lane_fills: &std::collections::BTreeMap<String, String>,
) {
    let lane_left = |idx: i32| -> i32 { lane_area_x + idx * lane_w };

    for (idx, lane) in lanes.iter().enumerate() {
        let lx = if stacked_partition_blocks {
            lane_area_x
        } else {
            lane_left(idx as i32)
        };
        let bg = lane_fills
            .get(lane)
            .map(String::as_str)
            .unwrap_or(if idx % 2 == 0 {
                act_style.background_color.as_str()
            } else {
                "#f1f5f9"
            });
        let header_fill = lane_fills
            .get(lane)
            .map(String::as_str)
            .unwrap_or(if idx % 2 == 0 { "#e2e8f0" } else { "#dde5ef" });

        if sequential_partition_lanes {
            let Some((span_top, span_bottom)) = lane_spans[idx] else {
                continue;
            };
            // Extend the lane background leftward when fork branches have placed
            // this lane's nodes in an adjacent (left) column.  The extension
            // uses the leftmost observed node cx minus half the box width and a
            // small margin, capped at the diagram left edge.
            let node_half_w = box_w / 2;
            let effective_lx = if let Some(min_cx) = min_node_cx.get(idx).copied().flatten() {
                let node_left = min_cx - node_half_w - 16; // 16px margin
                lx.min(node_left).max(0)
            } else {
                lx
            };
            let effective_w = (lx + lane_w) - effective_lx;
            let body_y = span_top + lane_header_h;
            let body_h = (span_bottom - body_y).max(24);
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"#cbd5e1\" stroke-width=\"1\" stroke-dasharray=\"4 3\"/>",
                effective_lx, body_y, effective_w, body_h, bg
            ));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"#94a3b8\" stroke-width=\"1\"/>",
                effective_lx, span_top, effective_w, lane_header_h, header_fill
            ));
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" font-weight=\"600\" fill=\"{}\">{}</text>",
                effective_lx + effective_w / 2,
                span_top + lane_header_h / 2 + 4,
                escape_text(&act_style.font_color),
                escape_text(lane)
            ));
            continue;
        }

        // Lane body (below header)
        out.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"#cbd5e1\" stroke-width=\"1\" stroke-dasharray=\"4 3\"/>",
            lx,
            header_h + lane_header_h,
            lane_w,
            height - header_h - lane_header_h - 20,
            bg
        ));
        if lane != "default" {
            // Lane header box
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"#94a3b8\" stroke-width=\"1\"/>",
                lx,
                header_h,
                lane_w,
                lane_header_h,
                header_fill
            ));
            // Lane name centered in the header box
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" font-weight=\"600\" fill=\"{}\">{}</text>",
                lx + lane_w / 2,
                header_h + lane_header_h / 2 + 4,
                escape_text(&act_style.font_color),
                escape_text(lane)
            ));
        }
    }
}

/// Per-lane y-span: `(span_top, span_bottom)`.
type LaneSpan = Option<(i32, i32)>;

/// Compute sequential lane spans (the y-range each lane actually occupies),
/// plus the minimum actual node cx observed for each lane.
///
/// The min-cx value is used by `emit_lanes` to extend a lane's background
/// leftward when fork branches place that lane's nodes in an adjacent column.
#[allow(clippy::too_many_arguments)]
pub(super) fn compute_lane_spans(
    doc: &crate::model::FamilyDocument,
    metas: &[super::layout::NodeMeta],
    node_layouts: &[super::layout::NodeLayout],
    lanes: &[String],
    lane_index_fn: &dyn Fn(&str) -> i32,
    lane_header_h: i32,
    header_h: i32,
    height: i32,
) -> (Vec<LaneSpan>, Vec<Option<i32>>) {
    use crate::model::FamilyNodeKind;
    let mut lane_spans: Vec<LaneSpan> = vec![None; lanes.len()];
    // min_node_cx[i] = minimum cx seen for lane i (None if no node yet)
    let mut min_node_cx: Vec<Option<i32>> = vec![None; lanes.len()];

    for ((node, meta), layout) in doc.nodes.iter().zip(metas.iter()).zip(node_layouts.iter()) {
        let is_invisible_merge = matches!(node.kind, FamilyNodeKind::ActivityMerge)
            && (meta.step_kind.contains("Else")
                || meta.step_kind.contains("EndIf")
                || meta.step_kind.contains("EndWhile")
                || meta.step_kind.contains("RepeatStart"));
        let is_layout_only = matches!(node.kind, FamilyNodeKind::ActivityPartition)
            || meta.step_kind == "RepeatStart"
            || is_invisible_merge;
        if is_layout_only || meta.lane_name == "default" {
            continue;
        }
        let lane_idx = lane_index_fn(&meta.lane_name) as usize;
        let span_top = (layout.slot_y - lane_header_h).max(header_h);
        let span_bottom = (layout.next_slot_y + 20).min(height - 20);
        match &mut lane_spans[lane_idx] {
            Some((top, bottom)) => {
                *top = (*top).min(span_top);
                *bottom = (*bottom).max(span_bottom);
            }
            None => lane_spans[lane_idx] = Some((span_top, span_bottom)),
        }
        // Track the leftmost node cx for this lane so we can extend the lane
        // background leftward when nodes are in an adjacent fork column.
        let cx = layout.cx;
        match &mut min_node_cx[lane_idx] {
            Some(prev) => *prev = (*prev).min(cx),
            None => min_node_cx[lane_idx] = Some(cx),
        }
    }

    (lane_spans, min_node_cx)
}
