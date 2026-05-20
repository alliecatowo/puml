use crate::render::scene_graph::{estimate_text_bbox, Rect as SceneRect};
use crate::render::svg::escape_text;
use crate::render::{puml_label_attrs, puml_node_attrs};
use crate::theme::ActivityStyle;

fn swimlane_id(idx: usize, lane: &str) -> String {
    format!("activity-swimlane-{idx}-{lane}")
}

fn swimlane_bbox(x: i32, y: i32, w: i32, h: i32) -> SceneRect {
    SceneRect::new(x as f64, y as f64, w as f64, h as f64)
}

/// Emit lane background rectangles and header labels.
///
/// When `sequential_partition_lanes` is true the lane boxes are drawn only over
/// the span of nodes that belong to that lane (for sequential partition style).
/// Otherwise the boxes stretch the full diagram height.
#[allow(clippy::too_many_arguments)]
pub(super) fn emit_lanes(
    out: &mut String,
    lanes: &[String],
    lane_spans: &[Option<(i32, i32)>],
    sequential_partition_lanes: bool,
    lane_area_x: i32,
    lane_w: i32,
    header_h: i32,
    lane_header_h: i32,
    height: i32,
    act_style: &ActivityStyle,
) {
    let lane_left = |idx: i32| -> i32 { lane_area_x + idx * lane_w };

    for (idx, lane) in lanes.iter().enumerate() {
        let lx = lane_left(idx as i32);
        let bg = if idx % 2 == 0 {
            act_style.background_color.as_str()
        } else {
            "#f1f5f9"
        };
        let header_fill = if idx % 2 == 0 { "#e2e8f0" } else { "#dde5ef" };

        if sequential_partition_lanes {
            let Some((span_top, span_bottom)) = lane_spans[idx] else {
                continue;
            };
            let body_y = span_top + lane_header_h;
            let body_h = (span_bottom - body_y).max(24);
            let lane_id = swimlane_id(idx, lane);
            let body_attrs = puml_node_attrs(
                &lane_id,
                "activity",
                "swimlane",
                swimlane_bbox(lx, body_y, lane_w, body_h),
            );
            out.push_str(&format!(
                "<rect class=\"activity-swimlane puml-node\" {} x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"#cbd5e1\" stroke-width=\"1\" stroke-dasharray=\"4 3\"/>",
                body_attrs, lx, body_y, lane_w, body_h, bg
            ));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"#94a3b8\" stroke-width=\"1\"/>",
                lx, span_top, lane_w, lane_header_h, header_fill
            ));
            out.push_str(&format!(
                "<text class=\"activity-swimlane-label puml-label\" {} x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" font-weight=\"600\" fill=\"{}\">{}</text>",
                puml_label_attrs(
                    &lane_id,
                    "swimlane-label",
                    estimate_text_bbox(
                        (lx + lane_w / 2) as f64,
                        (span_top + lane_header_h / 2 + 4) as f64,
                        lane,
                        11.0,
                        true,
                    ),
                ),
                lx + lane_w / 2,
                span_top + lane_header_h / 2 + 4,
                escape_text(&act_style.font_color),
                escape_text(lane)
            ));
            continue;
        }

        // Lane body (below header)
        let lane_id = swimlane_id(idx, lane);
        let body_y = header_h + lane_header_h;
        let body_h = height - header_h - lane_header_h - 20;
        let body_attrs = puml_node_attrs(
            &lane_id,
            "activity",
            "swimlane",
            swimlane_bbox(lx, body_y, lane_w, body_h),
        );
        out.push_str(&format!(
            "<rect class=\"activity-swimlane puml-node\" {} x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"#cbd5e1\" stroke-width=\"1\" stroke-dasharray=\"4 3\"/>",
            body_attrs, lx, body_y, lane_w, body_h, bg
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
                "<text class=\"activity-swimlane-label puml-label\" {} x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" font-weight=\"600\" fill=\"{}\">{}</text>",
                puml_label_attrs(
                    &lane_id,
                    "swimlane-label",
                    estimate_text_bbox(
                        (lx + lane_w / 2) as f64,
                        (header_h + lane_header_h / 2 + 4) as f64,
                        lane,
                        11.0,
                        true,
                    ),
                ),
                lx + lane_w / 2,
                header_h + lane_header_h / 2 + 4,
                escape_text(&act_style.font_color),
                escape_text(lane)
            ));
        }
    }
}

/// Compute sequential lane spans (the y-range each lane actually occupies).
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
) -> Vec<Option<(i32, i32)>> {
    use crate::model::FamilyNodeKind;
    let mut lane_spans: Vec<Option<(i32, i32)>> = vec![None; lanes.len()];

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
    }

    lane_spans
}
