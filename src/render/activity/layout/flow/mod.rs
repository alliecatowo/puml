mod frames;
mod helpers;

use frames::{branch_is_live, ForkBranch, ForkFrame, IfFrame, RepeatFrame, WhileFrame};
use helpers::{activity_decision_guard, layout_note_node, unpack_layout_params};
pub(in crate::render::activity) use helpers::{
    is_activity_flow_neutral_node, is_activity_terminal_step, previous_activity_flow_node,
};

use super::super::super::layout_constants::ACTIVITY_ARROW_OUT_OFFSET;
use super::super::arrows::fork_branch_cx;
use super::{ActivityRoute, LayoutParams, LayoutResult, NodeLayout, NodeMeta};
use crate::model::FamilyDocument;
use crate::model::FamilyNodeKind;

pub(in crate::render::activity) fn compute_layout(
    doc: &FamilyDocument,
    metas: &[NodeMeta],
    params: &LayoutParams<'_>,
) -> LayoutResult {
    let lane_center_x = params.lane_center_x;
    let (
        header_h,
        lane_header_h,
        step_h,
        branch_x_offset,
        fork_col_w,
        lane_w,
        min_fork_col_w,
        lane_area_x,
    ) = unpack_layout_params(params);
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
    let mut while_stack: Vec<WhileFrame> = Vec::new();
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
                // #1447: the else-branch arrow should carry the else guard
                // (from `else (no)`) not the then guard.
                // `doc.nodes[i]` is the Else node whose label holds the guard
                // text extracted from `else (no)` / `else(guard)`.
                let else_guard = doc.nodes[i].label.clone();
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
                        .with_label(else_guard),
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

                // effective_col_w: wide enough that node boxes never overlap,
                // but no wider than fork_col_w (the pre-allocated column budget).
                let effective_col_w = (lane_w / n_branches as i32)
                    .max(min_fork_col_w)
                    .min(fork_col_w.max(min_fork_col_w));
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

                // Compute bar half-width spanning the branch column centers.
                //
                // The bar must visibly connect the leftmost and rightmost branch
                // columns, but should NOT stretch a full `effective_col_w` past
                // each end — that produces fork bars that visually punch outside
                // the enclosing partition/lane (#1299).  We pad by a small
                // visual margin (~1/3 of a branch column, capped at the rough
                // node-box half-width derived from `min_fork_col_w`) which keeps
                // the bar reading as a single unit without engulfing the
                // surrounding frame.
                //
                // After the pad is applied, additionally clamp so that the bar
                // stays ≥24 px inside the enclosing partition frame (#1299).
                //
                // The partition's `effective_lx` (computed later in swimlanes.rs)
                // extends leftward to cover branch node boxes when they push past
                // `lane_area_x`.  To avoid computing that here, we approximate the
                // effective left edge as the leftmost branch node's left boundary
                // (leftmost column center − half box width − 16 px gap), clamped at
                // 0 (the SVG canvas edge).  The bar must clear this boundary by ≥24.
                let box_half_w_est = ((min_fork_col_w - 24) / 2).max(60);
                let bar_pad = (effective_col_w / 3).min(box_half_w_est).max(24);
                let leftmost_col_cx_for_clamp =
                    fork_branch_cx(fork_cx, 0, n_branches, effective_col_w);
                let effective_left_edge = (leftmost_col_cx_for_clamp - box_half_w_est - 16).max(0);
                // Lane right boundary: lane_area_x + lane_w (partition width never
                // expands rightward, only leftward via effective_lx).
                let lane_right = lane_area_x + lane_w;
                // Maximum half-width that keeps both left and right bar edges ≥24 px
                // inside the partition boundary (#1299 regression guard):
                //   left:  fork_cx - max_bar_half >= effective_left_edge + 24
                //   right: fork_cx + max_bar_half <= lane_right              - 24
                let max_bar_half = (fork_cx - effective_left_edge - 24)
                    .min(lane_right - fork_cx - 24)
                    .max(0);
                let bar_span_half = if n_branches > 1 {
                    let leftmost_cx = fork_branch_cx(fork_cx, 0, n_branches, effective_col_w);
                    let rightmost_cx =
                        fork_branch_cx(fork_cx, n_branches - 1, n_branches, effective_col_w);
                    ((rightmost_cx - leftmost_cx) / 2 + bar_pad).min(max_bar_half)
                } else {
                    (lane_w - 24).clamp(60, 110)
                };
                let live_bar_span_half = match live_branch_indices.as_slice() {
                    [] => 0,
                    [_] => (lane_w - 24).clamp(60, 110),
                    indices => {
                        let first = *indices.first().unwrap();
                        let last = *indices.last().unwrap();
                        let leftmost_cx =
                            fork_branch_cx(fork_cx, first, n_branches, effective_col_w);
                        let rightmost_cx =
                            fork_branch_cx(fork_cx, last, n_branches, effective_col_w);
                        ((rightmost_cx - leftmost_cx) / 2 + bar_pad).min(max_bar_half)
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
            "WhileStart" => {
                // Render the while condition diamond.  Body actions flow straight
                // down from the diamond on the "yes" (is) path.  On `endwhile` we
                // emit a back-edge from the last body node back to the top of the
                // diamond body and mark the exit path with the "no" guard label.
                let slot_y = current_slot_y;
                let arrow_out_y = slot_y + ARROW_OUT;
                let next_slot_y = slot_y + step_h;
                // Extract the "is (yes)" guard label for the back-loop arrow.
                let yes_guard = doc.nodes[i]
                    .label
                    .as_deref()
                    .and_then(activity_decision_guard);
                node_layouts.push(NodeLayout {
                    cx,
                    slot_y,
                    arrow_out_y,
                    next_slot_y,
                });
                while_stack.push(WhileFrame {
                    diamond_idx: i,
                    diamond_cx: cx,
                    yes_guard,
                });
                current_slot_y = next_slot_y;
            }
            "EndWhile" => {
                // Pop the matching WhileFrame.
                // Emit a back-edge arrow from the last body node's arrow_out_y
                // back to the diamond's slot_y (which IS the slot_y of the
                // first body node, since the diamond advances the slot by step_h).
                let slot_y = current_slot_y;
                let arrow_out_y = slot_y + ARROW_OUT;
                let next_slot_y = slot_y + step_h;
                // The EndWhile placeholder gets zero height (it's a control node).
                suppress_prev_arrow.insert(i);
                node_layouts.push(NodeLayout {
                    cx,
                    slot_y,
                    arrow_out_y,
                    next_slot_y,
                });

                if let Some(while_frame) = while_stack.pop() {
                    // Back-edge: from the node before EndWhile back to the diamond.
                    // The source is the arrow_out_y of the last body action (the
                    // node just before EndWhile).
                    let back_src_y = if i > 0 {
                        node_layouts[i - 1].arrow_out_y
                    } else {
                        slot_y
                    };
                    let diamond_slot_y = node_layouts[while_frame.diamond_idx].slot_y;

                    // The exit label from `endwhile (no)` goes on the exit arrow.
                    let exit_guard = doc.nodes[i]
                        .label
                        .as_deref()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string);

                    // Back-edge: last body node → while diamond (upward back-loop).
                    // Destination is the diamond node's slot_y so that
                    // emit_extra_arrows matches it when rendering the WhileStart node.
                    // Marked skip_in_scene: the SVG renderer detours this arrow
                    // around obstacle nodes; a straight-line approximation in the
                    // typed scene would produce spurious EdgeCrossesNode violations.
                    extra_arrows.push(
                        ActivityRoute::new(
                            while_frame.diamond_cx,
                            back_src_y,
                            while_frame.diamond_cx,
                            diamond_slot_y,
                        )
                        .with_label(while_frame.yes_guard)
                        .skip_in_scene(),
                    );
                    // Exit path arrow from the diamond's right side, routing around
                    // the loop body to land on the node below EndWhile.
                    // Destination slot_y: we store the exit guard and diamond info
                    // for the deferred arrow — the next node isn't laid out yet,
                    // so we mark the destination as (cx, next_slot_y) which is the
                    // EndWhile node's own next_slot_y; the successor node will
                    // share that slot_y (suppress_prev_arrow ensures it lands there).
                    // Also skip_in_scene: the diagonal route crosses body nodes.
                    let diamond_layout = &node_layouts[while_frame.diamond_idx];
                    let exit_src_x = diamond_layout.cx + 100; // right side of diamond
                    let exit_src_y = diamond_layout.slot_y + 24; // diamond mid-height
                    extra_arrows.push(
                        ActivityRoute::new(exit_src_x, exit_src_y, cx, next_slot_y)
                            .with_label(exit_guard)
                            .skip_in_scene(),
                    );
                }
                current_slot_y = next_slot_y;
            }
            _ => {
                // Partition/swimlane markers: zero-height layout nodes
                let is_partition_marker = meta.step_kind == "PartitionStart"
                    || meta.step_kind == "PartitionEnd"
                    || meta.step_kind == "Arrow"
                    || meta.step_kind == "OldStyle";
                if meta.step_kind == "Note" {
                    let (note_layout, next_slot_y) = layout_note_node(
                        doc,
                        metas,
                        &node_layouts,
                        i,
                        meta,
                        cx,
                        current_slot_y,
                        lane_w,
                        header_h,
                        step_h,
                        ARROW_OUT,
                    );
                    node_layouts.push(note_layout);
                    suppress_prev_arrow.insert(i);
                    current_slot_y = next_slot_y;
                } else if is_partition_marker
                    && matches!(doc.nodes[i].kind, FamilyNodeKind::ActivityPartition)
                {
                    let slot_y = current_slot_y;
                    let reserves_partition_space = has_partition_blocks
                        && (meta.step_kind == "PartitionStart" || meta.step_kind == "PartitionEnd");
                    // PartitionStart adds `lane_header_h` for the visible header band
                    // PLUS an extra 8 px so that the incoming arrow from the previous
                    // section clears the header before reaching the first node.
                    const PARTITION_ENTRY_CLEARANCE: i32 = 8;
                    let next_slot_y = if reserves_partition_space {
                        slot_y
                            + lane_header_h
                            + if meta.step_kind == "PartitionStart" {
                                PARTITION_ENTRY_CLEARANCE
                            } else {
                                0
                            }
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
