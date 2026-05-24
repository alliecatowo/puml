use super::super::FamilyNode;

pub(super) fn assign_y_positions(
    nodes: &[FamilyNode],
    roots: &[usize],
    y_positions: &mut [i32],
    y_cursor: &mut i32,
    y_step: i32,
) {
    for &idx in roots {
        let depth = nodes[idx].depth;
        // Count leaf descendants
        let leaves = subtree_leaf_count_render(nodes, idx);
        // Place this node at the center of its allocated leaf-slots
        y_positions[idx] = *y_cursor + (leaves as i32 - 1) * y_step / 2;
        // Recurse into children
        let children: Vec<usize> = (idx + 1..nodes.len())
            .take_while(|&j| nodes[j].depth > depth)
            .filter(|&j| nodes[j].depth == depth + 1)
            .collect();
        assign_y_positions(nodes, &children, y_positions, y_cursor, y_step);
        if children.is_empty() {
            *y_cursor += y_step;
        }
    }
}

pub(super) fn subtree_leaf_count_render(nodes: &[FamilyNode], idx: usize) -> usize {
    let depth = nodes[idx].depth;
    let children: Vec<usize> = (idx + 1..nodes.len())
        .take_while(|&j| nodes[j].depth > depth)
        .filter(|&j| nodes[j].depth == depth + 1)
        .collect();
    if children.is_empty() {
        return 1;
    }
    children
        .iter()
        .map(|&c| subtree_leaf_count_render(nodes, c))
        .sum()
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
