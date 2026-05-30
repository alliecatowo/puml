use super::super::FamilyNode;
use super::labels::multiline_line_count;

/// Compute the effective vertical slot size for a leaf node based on its
/// rendered height.  A single-line node fits in `base_y_step`; multi-line
/// nodes get extra room so they don't visually overlap their siblings.
///
/// The formula mirrors the height expansion in `draw_mindmap_subtree`:
///   `node_h = base_node_h + (lines - 1) * 16`
/// with `NODE_H = 34` and `16 px` per extra line.  We add 14 px of vertical
/// padding on top of the rendered height so adjacent boxes never touch.
pub(super) fn leaf_slot_height(display_name: &str, base_node_h: i32, base_y_step: i32) -> i32 {
    let lines = multiline_line_count(display_name);
    if lines <= 1 {
        return base_y_step;
    }
    // rendered height of the node box (mirrors nodes.rs logic)
    let rendered_h = (base_node_h + (lines - 1) * 16).min(base_node_h * lines.max(1));
    // slot = rendered height + per-side padding (14 px each → 28 px total)
    (rendered_h + 28).max(base_y_step)
}

/// Compute the total slot height consumed by a subtree's leaf nodes.
pub(super) fn subtree_slot_height(
    nodes: &[FamilyNode],
    display_names: &[String],
    idx: usize,
    base_node_h: i32,
    base_y_step: i32,
) -> i32 {
    let depth = nodes[idx].depth;
    let children: Vec<usize> = (idx + 1..nodes.len())
        .take_while(|&j| nodes[j].depth > depth)
        .filter(|&j| nodes[j].depth == depth + 1)
        .collect();
    if children.is_empty() {
        // Leaf: slot height derived from line count of THIS node's label.
        return leaf_slot_height(&display_names[idx], base_node_h, base_y_step);
    }
    children
        .iter()
        .map(|&c| subtree_slot_height(nodes, display_names, c, base_node_h, base_y_step))
        .sum()
}

pub(super) fn assign_y_positions(
    nodes: &[FamilyNode],
    display_names: &[String],
    roots: &[usize],
    y_positions: &mut [i32],
    y_cursor: &mut i32,
    base_node_h: i32,
    base_y_step: i32,
) {
    for &idx in roots {
        let depth = nodes[idx].depth;
        // Total slot height allocated to this subtree (accounts for multi-line labels)
        let total_slot = subtree_slot_height(nodes, display_names, idx, base_node_h, base_y_step);
        // Place this node at the center of its allocated slot
        y_positions[idx] = *y_cursor + total_slot / 2;
        // Recurse into children
        let children: Vec<usize> = (idx + 1..nodes.len())
            .take_while(|&j| nodes[j].depth > depth)
            .filter(|&j| nodes[j].depth == depth + 1)
            .collect();
        assign_y_positions(
            nodes,
            display_names,
            &children,
            y_positions,
            y_cursor,
            base_node_h,
            base_y_step,
        );
        if children.is_empty() {
            // Leaf: advance cursor by this node's effective slot height.
            *y_cursor += leaf_slot_height(&display_names[idx], base_node_h, base_y_step);
        }
    }
}

pub(super) fn family_tree_child_indices(nodes: &[FamilyNode], idx: usize) -> Vec<usize> {
    let depth = nodes[idx].depth;
    (idx + 1..nodes.len())
        .take_while(|&j| nodes[j].depth > depth)
        .filter(|&j| nodes[j].depth == depth + 1)
        .collect()
}

pub(super) fn node_sibling_index(nodes: &[FamilyNode], idx: usize) -> usize {
    if idx == 0 {
        return 0;
    }
    let depth = nodes[idx].depth;
    let mut count = 0usize;
    for prev in (0..idx).rev() {
        if nodes[prev].depth < depth {
            break;
        }
        if nodes[prev].depth == depth {
            count += 1;
        }
    }
    count
}
