use super::super::super::layout_constants::ACTIVITY_ARROW_OUT_OFFSET;
use super::super::arrows::fork_branch_cx;
use super::{ActivityRoute, LayoutParams, LayoutResult, NodeLayout, NodeMeta};
use crate::model::{FamilyDocument, FamilyNodeKind};

struct IfFrame {
    diamond_cx: i32,
    diamond_arrow_out: i32,
    diamond_next_slot: i32,
    then_guard: Option<String>,
    then_cx: i32,
    then_rightmost_cx: i32,
    then_end_next_slot: i32,
    in_else: bool,
    else_cx: i32,
    else_start_slot: i32,
}

struct ForkFrame {
    fork_node_idx: usize,
    fork_cx: i32,
    fork_slot_y: i32,
    branch_start_y: i32,
    is_split: bool,
    branches: Vec<ForkBranch>,
    current_branch: usize,
    fork_again_indices: Vec<usize>,
}

struct ForkBranch {
    start_node_idx: usize,
    end_next_slot: i32,
    end_node_idx: Option<usize>,
}

fn branch_is_live(branch: &ForkBranch, metas: &[NodeMeta]) -> bool {
    !branch
        .end_node_idx
        .is_some_and(|idx| is_activity_terminal_step(&metas[idx].step_kind))
}

struct RepeatFrame {
    body_start_idx: usize,
}

// ---------------------------------------------------------------------------

// Pass 1: compute layout positions for every node
// ---------------------------------------------------------------------------

pub(in crate::render::activity) fn compute_layout(
    doc: &FamilyDocument,
    metas: &[NodeMeta],
    params: &LayoutParams<'_>,
) -> LayoutResult {
    let LayoutParams {
        header_h,
        lane_header_h,
        step_h,
        branch_x_offset,
        fork_col_w,
        lane_w,
        lane_center_x,
    } = params;
    let (header_h, lane_header_h, step_h, branch_x_offset, fork_col_w, lane_w) = (
        *header_h,
        *lane_header_h,
        *step_h,
        *branch_x_offset,
        *fork_col_w,
        *lane_w,
    );
    const ARROW_OUT: i32 = ACTIVITY_ARROW_OUT_OFFSET;

    let mut node_layouts: Vec<NodeLayout> = Vec::with_capacity(doc.nodes.len());
    let mut fork_bar_half_widths: std::collections::BTreeMap<usize, i32> = Default::default();
    let mut extra_arrows: Vec<ActivityRoute> = Vec::new();
    let mut direct_arrows: Vec<ActivityRoute> = Vec::new();
    let mut suppress_prev_arrow: std::collections::BTreeSet<usize> = Default::default();

    let mut current_slot_y = header_h + lane_header_h;
    let mut if_stack: Vec<IfFrame> = Vec::new();
    let mut fork_stack: Vec<ForkFrame> = Vec::new();
    let mut repeat_stack: Vec<RepeatFrame> = Vec::new();
    let has_partition_blocks = metas.iter().any(|meta| meta.step_kind == "PartitionEnd");

    for (i, meta) in metas.iter().enumerate() {
        let base_cx = lane_center_x(&meta.lane_name);
        let cx = if_stack
            .last()
            .map(|f| if f.in_else { f.else_cx } else { f.then_cx })
            .unwrap_or(base_cx);

        // Inside a fork: use branch column cx.
        let cx = if let Some(frame) = fork_stack.last() {
            let n_branches = frame.branches.len();
            let branch_idx = frame.current_branch;
            fork_branch_cx(frame.fork_cx, branch_idx, n_branches, fork_col_w)
        } else {
            cx
        };

        match meta.step_kind.as_str() {
            "IfStart" => {
                let slot_y = current_slot_y;
                let arrow_out_y = slot_y + ARROW_OUT;
                let next_slot_y = slot_y + step_h;
                node_layouts.push(NodeLayout {
                    cx,
                    slot_y,
                    arrow_out_y,
                    next_slot_y,
                });
                let then_cx = cx - branch_x_offset;
                let else_cx = cx + branch_x_offset;
                if_stack.push(IfFrame {
                    diamond_cx: cx,
                    diamond_arrow_out: arrow_out_y,
                    diamond_next_slot: next_slot_y,
                    then_guard: doc.nodes[i]
                        .label
                        .as_deref()
                        .and_then(activity_decision_guard),
                    then_cx,
                    then_rightmost_cx: then_cx,
                    then_end_next_slot: next_slot_y,
                    in_else: false,
                    else_cx,
                    else_start_slot: next_slot_y,
                });
                for frame in &mut if_stack {
                    if !frame.in_else {
                        frame.then_rightmost_cx = frame.then_rightmost_cx.max(then_cx);
                    }
                }
                current_slot_y = next_slot_y;
            }
            "Else" => {
                let then_end_next_slot = current_slot_y;
                let frame = if_stack.last_mut().expect("else without if");
                frame.then_cx = cx;
                frame.then_end_next_slot = then_end_next_slot;
                let else_cx = (frame.diamond_cx + branch_x_offset)
                    .max(frame.then_rightmost_cx + branch_x_offset);
                frame.else_cx = else_cx;
                let diamond_cx = frame.diamond_cx;
                let diamond_arrow_out = frame.diamond_arrow_out;
                let slot_y = frame.diamond_next_slot;
                let arrow_out_y = slot_y + ARROW_OUT;
                let next_slot_y = slot_y + step_h;
                let then_guard = frame.then_guard.clone();
                frame.else_start_slot = slot_y;
                frame.in_else = true;
                for frame in &mut if_stack {
                    if !frame.in_else {
                        frame.then_rightmost_cx = frame.then_rightmost_cx.max(else_cx);
                    }
                }
                suppress_prev_arrow.insert(i);
                extra_arrows.push(
                    ActivityRoute::new(diamond_cx, diamond_arrow_out, else_cx, slot_y)
                        .with_label(then_guard),
                );
                node_layouts.push(NodeLayout {
                    cx: else_cx,
                    slot_y,
                    arrow_out_y,
                    next_slot_y,
                });
                current_slot_y = next_slot_y;
            }
            "EndIf" => {
                let frame = if_stack.pop().expect("endif without if");
                let then_arrow_out_y = frame.then_end_next_slot - step_h + ARROW_OUT;
                let then_cx = frame.then_cx;
                let else_arrow_out_y = current_slot_y - step_h + ARROW_OUT;
                let else_cx = frame.else_cx;
                let slot_y = frame.then_end_next_slot.max(current_slot_y);
                let arrow_out_y = slot_y + ARROW_OUT;
                let next_slot_y = slot_y + step_h;
                suppress_prev_arrow.insert(i);
                extra_arrows.push(ActivityRoute::new(
                    then_cx,
                    then_arrow_out_y,
                    frame.diamond_cx,
                    slot_y,
                ));
                extra_arrows.push(ActivityRoute::new(
                    else_cx,
                    else_arrow_out_y,
                    frame.diamond_cx,
                    slot_y,
                ));
                node_layouts.push(NodeLayout {
                    cx: frame.diamond_cx,
                    slot_y,
                    arrow_out_y,
                    next_slot_y,
                });
                for parent in &mut if_stack {
                    if !parent.in_else {
                        parent.then_rightmost_cx =
                            parent.then_rightmost_cx.max(frame.then_rightmost_cx);
                    }
                }
                current_slot_y = next_slot_y;
            }
            "Fork" => {
                let slot_y = current_slot_y;
                let arrow_out_y = slot_y + ARROW_OUT;
                let next_slot_y = slot_y + step_h;
                node_layouts.push(NodeLayout {
                    cx,
                    slot_y,
                    arrow_out_y,
                    next_slot_y,
                });
                fork_stack.push(ForkFrame {
                    fork_node_idx: i,
                    fork_cx: cx,
                    fork_slot_y: slot_y,
                    branch_start_y: next_slot_y,
                    is_split: doc.nodes[i]
                        .label
                        .as_deref()
                        .is_some_and(|label| label.eq_ignore_ascii_case("split")),
                    branches: vec![ForkBranch {
                        start_node_idx: i + 1,
                        end_next_slot: next_slot_y,
                        end_node_idx: None,
                    }],
                    current_branch: 0,
                    fork_again_indices: Vec::new(),
                });
                current_slot_y = next_slot_y;
            }
            "ForkAgain" => {
                let frame = fork_stack.last_mut().expect("fork again without fork");
                let branch_idx = frame.current_branch;
                frame.branches[branch_idx].end_next_slot = current_slot_y;
                frame.fork_again_indices.push(i);
                frame.branches.push(ForkBranch {
                    start_node_idx: i + 1,
                    end_next_slot: frame.branch_start_y,
                    end_node_idx: None,
                });
                frame.current_branch += 1;
                let n_branches = frame.branches.len();
                let new_branch_idx = frame.current_branch;
                let fork_cx = frame.fork_cx;
                suppress_prev_arrow.insert(i);
                let slot_y = frame.fork_slot_y;
                let branch_col_cx = fork_branch_cx(fork_cx, new_branch_idx, n_branches, fork_col_w);
                node_layouts.push(NodeLayout {
                    cx: branch_col_cx,
                    slot_y,
                    arrow_out_y: slot_y + ARROW_OUT,
                    next_slot_y: slot_y + step_h,
                });
                current_slot_y = frame.branch_start_y;
            }
            "RepeatStart" => {
                let slot_y = current_slot_y;
                node_layouts.push(NodeLayout {
                    cx,
                    slot_y,
                    arrow_out_y: slot_y,
                    next_slot_y: slot_y,
                });
                suppress_prev_arrow.insert(i);
                repeat_stack.push(RepeatFrame {
                    body_start_idx: i + 1,
                });
            }
            "RepeatWhile" => {
                let slot_y = current_slot_y;
                let arrow_out_y = slot_y + ARROW_OUT;
                let next_slot_y = slot_y + step_h;
                node_layouts.push(NodeLayout {
                    cx,
                    slot_y,
                    arrow_out_y,
                    next_slot_y,
                });
                if let Some(repeat_frame) = repeat_stack.pop() {
                    if let Some(body_layout) = node_layouts.get(repeat_frame.body_start_idx) {
                        let guard_label = doc.nodes[i]
                            .label
                            .as_deref()
                            .and_then(activity_decision_guard);
                        extra_arrows.push(
                            ActivityRoute::new(cx, arrow_out_y, body_layout.cx, body_layout.slot_y)
                                .with_label(guard_label),
                        );
                    }
                }
                current_slot_y = next_slot_y;
            }
            "EndFork" => {
                let mut frame = fork_stack.pop().expect("endfork without fork");
                let last_branch = frame.current_branch;
                frame.branches[last_branch].end_next_slot = current_slot_y;

                let n_branches = frame.branches.len();
                let fork_cx = frame.fork_cx;
                let branch_start_y = frame.branch_start_y;

                let max_end = frame
                    .branches
                    .iter()
                    .map(|b| b.end_next_slot)
                    .max()
                    .unwrap_or(current_slot_y);
                let slot_y = max_end;
                let arrow_out_y = slot_y + ARROW_OUT;
                let next_slot_y = slot_y + step_h;
                let live_branch_count = frame
                    .branches
                    .iter()
                    .filter(|branch| branch_is_live(branch, metas))
                    .count();
                let split_all_terminal = frame.is_split && live_branch_count == 0;

                suppress_prev_arrow.insert(i);
                if split_all_terminal && i + 1 < metas.len() {
                    suppress_prev_arrow.insert(i + 1);
                }
                node_layouts.push(NodeLayout {
                    cx: fork_cx,
                    slot_y,
                    arrow_out_y,
                    next_slot_y,
                });

                let effective_col_w = (lane_w / n_branches as i32).max(120).min(fork_col_w);
                let live_branch_indices = frame
                    .branches
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, branch)| branch_is_live(branch, metas).then_some(idx))
                    .collect::<Vec<_>>();
                let join_cx = if live_branch_indices.is_empty() {
                    fork_cx
                } else {
                    let sum = live_branch_indices
                        .iter()
                        .map(|&idx| fork_branch_cx(fork_cx, idx, n_branches, effective_col_w))
                        .sum::<i32>();
                    sum / live_branch_indices.len() as i32
                };
                if let Some(layout) = node_layouts.get_mut(i) {
                    layout.cx = join_cx;
                }

                // Fix up cx positions for all nodes inside branches
                for (branch_idx, branch) in frame.branches.iter().enumerate() {
                    let col_cx = fork_branch_cx(fork_cx, branch_idx, n_branches, effective_col_w);
                    let branch_end_idx = if branch_idx + 1 < n_branches {
                        frame.fork_again_indices[branch_idx]
                    } else {
                        i
                    };
                    for node_idx in branch.start_node_idx..branch_end_idx {
                        if let Some(layout) = node_layouts.get_mut(node_idx) {
                            layout.cx = col_cx;
                        }
                    }
                    if branch_idx + 1 < n_branches {
                        let fa_idx = frame.fork_again_indices[branch_idx];
                        if let Some(layout) = node_layouts.get_mut(fa_idx) {
                            let next_col_cx = fork_branch_cx(
                                fork_cx,
                                branch_idx + 1,
                                n_branches,
                                effective_col_w,
                            );
                            layout.cx = next_col_cx;
                        }
                    }
                    // Straight-down arrow from branch last node to the join bar
                    if branch_is_live(branch, metas) {
                        let branch_arrow_out_y = branch.end_next_slot - step_h + ARROW_OUT;
                        let join_bar_top_y = slot_y + 24;
                        let (y1, y2) = if branch_arrow_out_y <= join_bar_top_y {
                            (branch_arrow_out_y, join_bar_top_y)
                        } else {
                            (join_bar_top_y, branch_arrow_out_y)
                        };
                        direct_arrows.push(ActivityRoute::new(col_cx, y1, col_cx, y2));
                    }
                }

                // Straight-down arrows from the fork bar bottom to each branch column
                let fork_bar_bottom_y = frame.fork_slot_y + 32;
                for (branch_idx, branch) in frame.branches.iter().enumerate() {
                    let col_cx = fork_branch_cx(fork_cx, branch_idx, n_branches, effective_col_w);
                    let (y1, y2) = if fork_bar_bottom_y <= branch_start_y {
                        (fork_bar_bottom_y, branch_start_y)
                    } else {
                        (branch_start_y, fork_bar_bottom_y)
                    };
                    direct_arrows.push(ActivityRoute::new(col_cx, y1, col_cx, y2));
                    suppress_prev_arrow.insert(branch.start_node_idx);
                }

                // Compute bar half-width spanning all branch columns
                let bar_span_half = if n_branches > 1 {
                    let leftmost = fork_branch_cx(fork_cx, 0, n_branches, effective_col_w)
                        - effective_col_w / 2;
                    let rightmost =
                        fork_branch_cx(fork_cx, n_branches - 1, n_branches, effective_col_w)
                            + effective_col_w / 2;
                    (rightmost - leftmost) / 2
                } else {
                    (lane_w - 24).clamp(60, 110)
                };
                let live_bar_span_half = match live_branch_indices.as_slice() {
                    [] => 0,
                    [_] => (lane_w - 24).clamp(60, 110),
                    indices => {
                        let first = *indices.first().unwrap();
                        let last = *indices.last().unwrap();
                        let leftmost = fork_branch_cx(fork_cx, first, n_branches, effective_col_w)
                            - effective_col_w / 2;
                        let rightmost = fork_branch_cx(fork_cx, last, n_branches, effective_col_w)
                            + effective_col_w / 2;
                        (rightmost - leftmost) / 2
                    }
                };
                fork_bar_half_widths.insert(frame.fork_node_idx, bar_span_half);
                fork_bar_half_widths.insert(
                    i,
                    if split_all_terminal {
                        0
                    } else {
                        live_bar_span_half
                    },
                );

                current_slot_y = next_slot_y;
            }
            _ => {
                // Partition/swimlane markers: zero-height layout nodes
                let is_partition_marker = meta.step_kind == "PartitionStart"
                    || meta.step_kind == "PartitionEnd"
                    || meta.step_kind == "Arrow"
                    || meta.step_kind == "OldStyle";
                if meta.step_kind == "Note" {
                    let anchor_idx = previous_activity_flow_node(doc, metas, i);
                    let (slot_y, anchor_cx, anchor_arrow_out_y) = anchor_idx
                        .and_then(|idx| {
                            node_layouts
                                .get(idx)
                                .map(|layout| (layout.slot_y, layout.cx, layout.arrow_out_y))
                        })
                        .unwrap_or((current_slot_y, cx, current_slot_y + ARROW_OUT));
                    let note_h = super::super::nodes::activity_note_card_height(
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
                        Some("right") | None => (anchor_cx + note_offset, slot_y, current_slot_y),
                        Some(_) => (anchor_cx + note_offset, slot_y, current_slot_y),
                    };
                    node_layouts.push(NodeLayout {
                        cx: note_cx,
                        slot_y: note_slot_y,
                        arrow_out_y: anchor_arrow_out_y,
                        next_slot_y,
                    });
                    suppress_prev_arrow.insert(i);
                    current_slot_y = next_slot_y;
                } else if is_partition_marker
                    && matches!(doc.nodes[i].kind, FamilyNodeKind::ActivityPartition)
                {
                    let slot_y = current_slot_y;
                    let reserves_partition_space = has_partition_blocks
                        && (meta.step_kind == "PartitionStart" || meta.step_kind == "PartitionEnd");
                    let next_slot_y = if reserves_partition_space {
                        slot_y + lane_header_h
                    } else {
                        slot_y
                    };
                    node_layouts.push(NodeLayout {
                        cx,
                        slot_y,
                        arrow_out_y: slot_y,
                        next_slot_y,
                    });
                    suppress_prev_arrow.insert(i);
                    current_slot_y = next_slot_y;
                } else {
                    let slot_y = current_slot_y;
                    let arrow_out_y = slot_y + ARROW_OUT;
                    let next_slot_y = slot_y + step_h;
                    node_layouts.push(NodeLayout {
                        cx,
                        slot_y,
                        arrow_out_y,
                        next_slot_y,
                    });
                    for frame in &mut if_stack {
                        if !frame.in_else {
                            frame.then_rightmost_cx = frame.then_rightmost_cx.max(cx);
                        }
                    }
                    if let Some(fork_frame) = fork_stack.last_mut() {
                        let bi = fork_frame.current_branch;
                        fork_frame.branches[bi].end_next_slot = next_slot_y;
                        if !matches!(
                            meta.step_kind.as_str(),
                            "Note" | "Arrow" | "PartitionStart" | "PartitionEnd" | "OldStyle"
                        ) {
                            fork_frame.branches[bi].end_node_idx = Some(i);
                        }
                    }
                    current_slot_y = next_slot_y;
                }
            }
        }
    }

    LayoutResult {
        node_layouts,
        fork_bar_half_widths,
        extra_arrows,
        direct_arrows,
        suppress_prev_arrow,
    }
}

fn activity_decision_guard(label: &str) -> Option<String> {
    label
        .split_once(" / ")
        .map(|(_, guard)| guard.trim().to_string())
}

pub(in crate::render::activity) fn is_activity_terminal_step(step_kind: &str) -> bool {
    matches!(step_kind, "Stop" | "End" | "Kill" | "Detach")
}

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

pub(in crate::render::activity) fn previous_activity_flow_node(
    doc: &FamilyDocument,
    metas: &[NodeMeta],
    idx: usize,
) -> Option<usize> {
    (0..idx)
        .rev()
        .find(|&prev_idx| !is_activity_flow_neutral_node(doc, metas, prev_idx))
}
