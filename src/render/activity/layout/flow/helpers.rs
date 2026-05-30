use super::super::{LayoutParams, NodeLayout, NodeMeta};
use crate::model::{FamilyDocument, FamilyNodeKind};

/// Extract the guard label from an `if (cond) / (guard)` or `while (cond) is (guard)` label.
pub(super) fn activity_decision_guard(label: &str) -> Option<String> {
    label
        .split_once(" / ")
        .map(|(_, guard)| guard.trim().to_string())
}

/// Return `true` for step kinds that unconditionally terminate the current branch.
pub(in crate::render::activity) fn is_activity_terminal_step(step_kind: &str) -> bool {
    matches!(step_kind, "Stop" | "End" | "Kill" | "Detach")
}

/// Return `true` for nodes that do not advance `current_slot_y` and are skipped
/// when searching for the previous meaningful flow node (e.g. for note anchoring).
pub(in crate::render::activity) fn is_activity_flow_neutral_node(
    doc: &FamilyDocument,
    metas: &[NodeMeta],
    idx: usize,
) -> bool {
    let step_kind = metas[idx].step_kind.as_str();
    (matches!(doc.nodes[idx].kind, FamilyNodeKind::ActivityPartition)
        && (step_kind == "PartitionStart"
            || step_kind == "PartitionEnd"
            || step_kind == "Arrow"
            || step_kind == "OldStyle"))
        || step_kind == "RepeatStart"
        || step_kind == "Note"
}

/// Walk backwards from `idx` to find the nearest node that is not
/// [`is_activity_flow_neutral_node`].
pub(in crate::render::activity) fn previous_activity_flow_node(
    doc: &FamilyDocument,
    metas: &[NodeMeta],
    idx: usize,
) -> Option<usize> {
    (0..idx)
        .rev()
        .find(|&prev_idx| !is_activity_flow_neutral_node(doc, metas, prev_idx))
}

pub(super) fn unpack_layout_params(
    params: &LayoutParams<'_>,
) -> (i32, i32, i32, i32, i32, i32, i32, i32) {
    (
        params.header_h,
        params.lane_header_h,
        params.step_h,
        params.branch_x_offset,
        params.fork_col_w,
        params.lane_w,
        params.min_fork_col_w,
        params.lane_area_x,
    )
}

/// Compute the [`NodeLayout`] and updated `current_slot_y` for a `Note` node.
///
/// Notes are pinned to the side of their anchor node and do not advance the
/// main flow column — `next_slot_y` returned here is usually the same as
/// `current_slot_y` (for left/right/top side-notes) or slightly larger for
/// `bottom` notes.
#[allow(clippy::too_many_arguments)]
pub(super) fn layout_note_node(
    doc: &FamilyDocument,
    metas: &[NodeMeta],
    node_layouts: &[NodeLayout],
    i: usize,
    meta: &NodeMeta,
    cx: i32,
    current_slot_y: i32,
    lane_w: i32,
    header_h: i32,
    step_h: i32,
    arrow_out: i32,
) -> (NodeLayout, i32) {
    let anchor_idx = previous_activity_flow_node(doc, metas, i);
    let (slot_y, anchor_cx, anchor_arrow_out_y) = anchor_idx
        .and_then(|idx| {
            node_layouts
                .get(idx)
                .map(|layout| (layout.slot_y, layout.cx, layout.arrow_out_y))
        })
        .unwrap_or((current_slot_y, cx, current_slot_y + arrow_out));
    let note_h = crate::render::activity::nodes::activity_note_card_height(
        doc.nodes[i].label.as_deref().unwrap_or_default(),
    );
    let note_offset = (lane_w / 2).max(140) + 32;
    let vertical_note_offset = (lane_w / 4).clamp(160, 240);
    let vertical_note_cx = anchor_cx + vertical_note_offset;
    let (note_cx, note_slot_y, next_slot_y) = match meta.note_side.as_deref() {
        Some("left") => (anchor_cx - note_offset, slot_y, current_slot_y),
        Some("top") => (
            vertical_note_cx,
            (slot_y - note_h - 12).max(header_h),
            current_slot_y,
        ),
        Some("bottom") => {
            let y = slot_y + step_h;
            (vertical_note_cx, y, current_slot_y.max(y + note_h + 12))
        }
        Some("right") | None | Some(_) => (anchor_cx + note_offset, slot_y, current_slot_y),
    };
    (
        NodeLayout {
            cx: note_cx,
            slot_y: note_slot_y,
            arrow_out_y: anchor_arrow_out_y,
            next_slot_y,
        },
        next_slot_y,
    )
}
