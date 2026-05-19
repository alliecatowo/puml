use crate::model::{FamilyDocument, FamilyNodeKind};

use super::layout::{NodeLayout, NodeMeta};

// ---------------------------------------------------------------------------
// Hidden-node deduplication pass
//
// If two ActivityAction nodes with the same label appear on the same lane,
// separated only by layout-only control nodes, collapse the second into the
// first so they overlap visually and no redundant box is drawn.
// ---------------------------------------------------------------------------

fn is_layout_only_control(doc: &FamilyDocument, metas: &[NodeMeta], idx: usize) -> bool {
    let step_kind = metas[idx].step_kind.as_str();
    (matches!(doc.nodes[idx].kind, FamilyNodeKind::ActivityPartition)
        && (step_kind == "PartitionStart"
            || step_kind == "PartitionEnd"
            || step_kind == "OldStyle"))
        || step_kind == "Else"
        || step_kind == "EndIf"
        || step_kind == "EndWhile"
        || step_kind == "RepeatStart"
}

pub(super) fn compute_hidden_nodes(
    doc: &FamilyDocument,
    metas: &[NodeMeta],
    node_layouts: &mut [NodeLayout],
    suppress_prev_arrow: &mut std::collections::HashSet<usize>,
) -> std::collections::HashSet<usize> {
    let mut hidden_nodes: std::collections::HashSet<usize> = Default::default();

    for i in 0..doc.nodes.len() {
        if hidden_nodes.contains(&i) || !matches!(doc.nodes[i].kind, FamilyNodeKind::ActivityAction)
        {
            continue;
        }
        let label = doc.nodes[i].label.as_deref().map(str::trim).unwrap_or("");
        if label.is_empty() {
            continue;
        }
        let mut j = i + 1;
        let mut saw_control_gap = false;
        let mut nested_merge_idx = None;
        while j < doc.nodes.len() && is_layout_only_control(doc, metas, j) {
            saw_control_gap = true;
            if nested_merge_idx.is_none() && metas[j].step_kind == "EndIf" {
                nested_merge_idx = Some(j);
            }
            j += 1;
        }
        if !saw_control_gap || j >= doc.nodes.len() {
            continue;
        }
        if !matches!(doc.nodes[j].kind, FamilyNodeKind::ActivityAction) {
            continue;
        }
        let next_label = doc.nodes[j].label.as_deref().map(str::trim).unwrap_or("");
        if next_label != label || metas[j].lane_name != metas[i].lane_name {
            continue;
        }
        node_layouts[j] = NodeLayout {
            cx: node_layouts[i].cx,
            slot_y: node_layouts[i].slot_y,
            arrow_out_y: node_layouts[i].arrow_out_y,
            next_slot_y: node_layouts[i].next_slot_y,
        };
        hidden_nodes.insert(j);
        if let Some(merge_idx) = nested_merge_idx {
            suppress_prev_arrow.insert(merge_idx);
        }
    }

    hidden_nodes
}

// ---------------------------------------------------------------------------
// Hidden-control-node predicate (needed for arrow redirect pass)
// ---------------------------------------------------------------------------

pub(super) fn is_hidden_control_node(
    doc: &FamilyDocument,
    metas: &[NodeMeta],
    hidden_nodes: &std::collections::HashSet<usize>,
    idx: usize,
) -> bool {
    hidden_nodes.contains(&idx) || is_layout_only_control(doc, metas, idx)
}

// ---------------------------------------------------------------------------
// Extra-arrow redirect pass
//
// Arrows that land on hidden control nodes are redirected to the next visible
// node so they still terminate visually.
// ---------------------------------------------------------------------------

pub(super) fn redirect_extra_arrows(
    doc: &FamilyDocument,
    metas: &[NodeMeta],
    node_layouts: &[NodeLayout],
    extra_arrows: Vec<(i32, i32, i32, i32)>,
    hidden_nodes: &std::collections::HashSet<usize>,
) -> Vec<(i32, i32, i32, i32)> {
    let slot_index_by_position: std::collections::HashMap<(i32, i32), usize> = node_layouts
        .iter()
        .enumerate()
        .map(|(idx, layout)| ((layout.cx, layout.slot_y), idx))
        .collect();
    let arrow_out_index_by_position: std::collections::HashMap<(i32, i32), usize> = node_layouts
        .iter()
        .enumerate()
        .map(|(idx, layout)| ((layout.cx, layout.arrow_out_y), idx))
        .collect();

    let is_hidden = |idx: usize| is_hidden_control_node(doc, metas, hidden_nodes, idx);
    let next_visible = |idx: usize| -> Option<usize> {
        ((idx + 1)..doc.nodes.len()).find(|&next_idx| !is_hidden(next_idx))
    };

    extra_arrows
        .into_iter()
        .filter_map(|(x1, y1, mut x2, mut y2)| {
            if let Some(&src_idx) = arrow_out_index_by_position.get(&(x1, y1)) {
                if is_hidden(src_idx) {
                    return None;
                }
            }
            if let Some(&dst_idx) = slot_index_by_position.get(&(x2, y2)) {
                if is_hidden(dst_idx) {
                    let next_idx = next_visible(dst_idx)?;
                    let layout = &node_layouts[next_idx];
                    x2 = layout.cx;
                    y2 = layout.slot_y;
                }
            }
            Some((x1, y1, x2, y2))
        })
        .collect()
}
