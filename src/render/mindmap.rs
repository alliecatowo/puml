mod wbs;

pub use wbs::render_wbs_svg;
use wbs::wbs_orientation_attr;

use super::scene_graph::Rect;
use super::*;

const MINDMAP_PALETTE: &[&str] = &[
    "#fde68a", // depth 0 — root amber
    "#bfdbfe", // depth 1 — sky blue
    "#bbf7d0", // depth 2 — mint
    "#fecaca", // depth 3 — rose
    "#e9d5ff", // depth 4 — lavender
    "#fed7aa", // depth 5 — peach
];

fn mindmap_node_fill(depth: usize) -> &'static str {
    MINDMAP_PALETTE[depth % MINDMAP_PALETTE.len()]
}

fn family_node_fill<'a>(node: &'a crate::model::FamilyNode, fallback: &'a str) -> &'a str {
    node.fill_color.as_deref().unwrap_or(fallback)
}

/// Emit a centered multi-line `<text>` element. PlantUML supports `\n` in node
/// labels for explicit line breaks (#560); the parser converts the escape to a
/// real newline and this helper paints each line as a `<tspan>` centered around
/// `y_center`.
#[allow(clippy::too_many_arguments)]
fn render_multiline_text(
    x: i32,
    y_center: i32,
    text: &str,
    font_size: i32,
    font_family: &str,
    font_weight: &str,
    class_attr: &str,
    attrs: &str,
) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    let attr_suffix = if attrs.is_empty() {
        String::new()
    } else {
        format!(" {attrs}")
    };
    if lines.len() <= 1 {
        return format!(
            "<text class=\"{class_attr}\"{attr_suffix} x=\"{x}\" y=\"{y_center}\" text-anchor=\"middle\" dominant-baseline=\"middle\" font-family=\"{ff}\" font-size=\"{fs}\" font-weight=\"{fw}\">{txt}</text>",
            x = x,
            y_center = y_center,
            ff = font_family,
            fs = font_size,
            fw = font_weight,
            txt = escape_text(text),
        );
    }
    let n = lines.len() as i32;
    let line_h = (font_size as f32 * 1.25) as i32;
    let total_h = line_h * (n - 1);
    let start_y = y_center - total_h / 2;
    let mut out = format!(
        "<text class=\"{class_attr}\"{attr_suffix} x=\"{x}\" y=\"{y_center}\" text-anchor=\"middle\" dominant-baseline=\"middle\" font-family=\"{ff}\" font-size=\"{fs}\" font-weight=\"{fw}\">",
        x = x,
        y_center = y_center,
        ff = font_family,
        fs = font_size,
        fw = font_weight,
    );
    for (i, line) in lines.iter().enumerate() {
        let y = start_y + (i as i32) * line_h;
        out.push_str(&format!(
            "<tspan x=\"{x}\" y=\"{y}\">{}</tspan>",
            escape_text(line),
            x = x,
            y = y,
        ));
    }
    out.push_str("</text>");
    out
}

fn tree_node_id(family: &str, idx: usize) -> String {
    format!("{family}-node-{idx}")
}

fn tree_edge_id(family: &str, parent_idx: usize, child_idx: usize) -> String {
    format!("{family}-edge-{parent_idx}-{child_idx}")
}

fn rect_from_i32(x: i32, y: i32, w: i32, h: i32) -> Rect {
    Rect::new(x as f64, y as f64, w as f64, h as f64)
}

fn label_bbox(x: i32, y_center: i32, text: &str, font_size: i32) -> Rect {
    let width = (multiline_char_width(text) * 7).max(1);
    let lines = multiline_line_count(text).max(1);
    let line_h = ((font_size as f32) * 1.25) as i32;
    let height = (line_h * lines).max(font_size);
    rect_from_i32(x - width / 2, y_center - height / 2, width, height)
}

/// Width of a multi-line label = the longest line, in monospace char units.
fn multiline_char_width(text: &str) -> i32 {
    text.split('\n')
        .map(|s| s.chars().count() as i32)
        .max()
        .unwrap_or(0)
}

/// Number of lines in a (possibly multi-line) label.
fn multiline_line_count(text: &str) -> i32 {
    text.split('\n').count() as i32
}

/// Render a `@startmindmap` document as SVG.
///
/// Layout: horizontal tree — root centred; right-side branches extend right,
/// left-side branches extend left. Each level increments x by `X_STEP`. Y is
/// spread evenly per side.
pub fn render_mindmap_svg(doc: &FamilyDocument) -> String {
    const X_STEP: i32 = 180;
    const Y_STEP: i32 = 48;
    const NODE_H: i32 = 34;
    const MARGIN: i32 = 24;
    const NODE_PAD_X: i32 = 10;

    // Separate nodes into root, left-side, right-side subtrees.
    // Depth 0 = root. Depth 1+ inherit side from their nearest depth-1 ancestor.
    let nodes = &doc.nodes;
    if nodes.is_empty() {
        return mindmap_empty_svg(doc);
    }

    // Build parent indices and side assignments.
    let n = nodes.len();
    let mut side = vec![MindMapSide::Right; n];
    let mut parent: Vec<Option<usize>> = vec![None; n];
    {
        let mut stack: Vec<usize> = Vec::new();
        for i in 0..n {
            let depth = nodes[i].depth;
            while stack.len() > depth {
                stack.pop();
            }
            if let Some(&p) = stack.last() {
                parent[i] = Some(p);
            }
            // Side: use the node's own side if depth >= 1
            if depth == 0 {
                side[i] = MindMapSide::Right; // root — not rendered as left/right
            } else if depth == 1 {
                side[i] = nodes[i].mindmap_side;
            } else if let Some(p) = parent[i] {
                side[i] = side[p];
            }
            stack.push(i);
        }
    }

    // Auto-balance: if all depth-1 nodes are Right (no explicit side markers),
    // distribute them evenly left/right for a balanced mindmap (#430/#532).
    // We assign the first half to Left (alternating from the last node upward so
    // the first child stays on the right per convention).
    let has_explicit_left =
        (0..n).any(|i| nodes[i].depth == 1 && nodes[i].mindmap_side == MindMapSide::Left);
    if !has_explicit_left {
        let depth1_indices: Vec<usize> = (0..n).filter(|&i| nodes[i].depth == 1).collect();
        let total = depth1_indices.len();
        if total > 1 {
            // Assign the bottom half (by index order) to Left, keeping the top half Right.
            let left_count = total / 2;
            for (rank, &node_idx) in depth1_indices.iter().enumerate() {
                if rank >= total - left_count {
                    // Bottom `left_count` children go left; propagate to their descendants.
                    let mut j = node_idx;
                    while j < n {
                        if j != node_idx && nodes[j].depth <= nodes[node_idx].depth {
                            break;
                        }
                        side[j] = MindMapSide::Left;
                        j += 1;
                    }
                }
            }
        }
    }

    // Collect left/right subtrees at depth 1+.
    let right_roots: Vec<usize> = (0..n)
        .filter(|&i| nodes[i].depth == 1 && side[i] == MindMapSide::Right)
        .collect();
    let left_roots: Vec<usize> = (0..n)
        .filter(|&i| nodes[i].depth == 1 && side[i] == MindMapSide::Left)
        .collect();

    // For each depth-1 subtree, compute total height = number of descendants + self.
    fn subtree_leaf_count(nodes: &[crate::model::FamilyNode], idx: usize) -> usize {
        let depth = nodes[idx].depth;
        let children_count: usize = (idx + 1..nodes.len())
            .take_while(|&j| nodes[j].depth > depth)
            .filter(|&j| nodes[j].depth == depth + 1)
            .count();
        if children_count == 0 {
            return 1;
        }
        let mut total = 0usize;
        let mut j = idx + 1;
        while j < nodes.len() && nodes[j].depth > depth {
            if nodes[j].depth == depth + 1 {
                total += subtree_leaf_count(nodes, j);
            }
            j += 1;
        }
        total
    }

    // Assign y positions for right-side depth-1 nodes.
    let total_right_leaves: usize = right_roots
        .iter()
        .map(|&i| subtree_leaf_count(nodes, i))
        .sum();
    let total_left_leaves: usize = left_roots
        .iter()
        .map(|&i| subtree_leaf_count(nodes, i))
        .sum();
    let max_leaves = total_right_leaves.max(total_left_leaves).max(1);
    let canvas_h = (max_leaves as i32) * Y_STEP + 2 * MARGIN + NODE_H;

    // Max text width for nodes — simple heuristic.
    fn node_width(name: &str) -> i32 {
        let chars = name.chars().count() as i32;
        (chars * 7 + 20).clamp(80, 220)
    }

    let root_w = node_width(&nodes[0].name);
    let max_right_depth = (0..n)
        .filter(|&i| side[i] == MindMapSide::Right && nodes[i].depth >= 1)
        .map(|i| nodes[i].depth)
        .max()
        .unwrap_or(0);
    let max_left_depth = (0..n)
        .filter(|&i| side[i] == MindMapSide::Left && nodes[i].depth >= 1)
        .map(|i| nodes[i].depth)
        .max()
        .unwrap_or(0);

    let root_cy = canvas_h / 2;

    // Draw nodes recursively — track y-cursors per side.
    // We assign y by a preorder traversal respecting leaf count.
    let mut y_positions = vec![0i32; n];
    {
        // Right side
        let mut y_cursor = root_cy - (total_right_leaves as i32 * Y_STEP) / 2 + Y_STEP / 2;
        assign_y_positions(nodes, &right_roots, &mut y_positions, &mut y_cursor, Y_STEP);
        // Left side
        y_cursor = root_cy - (total_left_leaves as i32 * Y_STEP) / 2 + Y_STEP / 2;
        assign_y_positions(nodes, &left_roots, &mut y_positions, &mut y_cursor, Y_STEP);
    }

    fn subtree_bounds(
        nodes: &[crate::model::FamilyNode],
        idx: usize,
        node_x_center: i32,
        x_step: i32,
        node_pad_x: i32,
        is_left: bool,
    ) -> (i32, i32) {
        let nw = (multiline_char_width(&nodes[idx].name) * 7 + 20).clamp(70, 220);
        let nx = if is_left {
            node_x_center - nw
        } else {
            node_x_center
        };
        let children = family_tree_child_indices(nodes, idx);
        let next_x_center = if is_left {
            node_x_center - x_step
        } else {
            node_x_center + x_step + nw - node_pad_x
        };
        children
            .iter()
            .fold((nx, nx + nw), |(acc_min, acc_max), &c| {
                let (child_min, child_max) =
                    subtree_bounds(nodes, c, next_x_center, x_step, node_pad_x, is_left);
                (acc_min.min(child_min), acc_max.max(child_max))
            })
    }

    let depth_bias = (max_left_depth as i32) * X_STEP + 240;
    let root_cx_prelim = MARGIN + depth_bias + root_w / 2;
    let root_min_x = root_cx_prelim - root_w / 2;
    let root_max_x = root_cx_prelim + root_w / 2;
    let actual_max_right = {
        let right_start = root_cx_prelim + root_w / 2 + X_STEP - NODE_PAD_X;
        right_roots
            .iter()
            .map(|&i| subtree_bounds(nodes, i, right_start, X_STEP, NODE_PAD_X, false).1)
            .max()
            .unwrap_or(root_max_x)
    };
    let actual_min_left = {
        let left_start = root_cx_prelim - root_w / 2 - X_STEP + NODE_PAD_X;
        left_roots
            .iter()
            .map(|&i| subtree_bounds(nodes, i, left_start, X_STEP, NODE_PAD_X, true).0)
            .min()
            .unwrap_or(root_min_x)
    };
    let actual_min_x = root_min_x.min(actual_min_left);
    let actual_max_x = root_max_x.max(actual_max_right);
    let extra_left = (MARGIN - actual_min_x).max(0);
    let canvas_w = actual_max_x + extra_left + MARGIN;
    let mindmap_leaves = (0..n)
        .filter(|&idx| family_tree_child_indices(nodes, idx).is_empty())
        .count();
    let root_cx = root_cx_prelim + extra_left;

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\" data-mindmap-orientation=\"{orientation}\" data-mindmap-node-count=\"{node_count}\" data-mindmap-leaf-count=\"{leaf_count}\" data-mindmap-max-depth=\"{max_depth}\">",
        w = canvas_w,
        h = canvas_h,
        orientation = wbs_orientation_attr(doc.orientation),
        node_count = n,
        leaf_count = mindmap_leaves,
        max_depth = max_right_depth.max(max_left_depth)
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");

    // Title
    let mut ty = MARGIN;
    if let Some(title) = &doc.title {
        for line in title.lines() {
            out.push_str(&format!(
                "<text x=\"{cx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\">{txt}</text>",
                cx = root_cx,
                ty = ty,
                txt = escape_text(line)
            ));
            ty += 20;
        }
    }

    // Draw root node
    let rx = root_cx - root_w / 2;
    let ry = root_cy - NODE_H / 2;
    let root_id = tree_node_id("mindmap", 0);
    let root_bbox = rect_from_i32(rx, ry, root_w, NODE_H);
    let root_attrs = puml_node_attrs(&root_id, "mindmap", "tree-node", root_bbox);
    out.push_str(&format!(
        "<rect class=\"mindmap-node mindmap-root mindmap-branch puml-node\" {root_attrs} data-mindmap-depth=\"0\" data-mindmap-child-count=\"{child_count}\" data-mindmap-fill=\"{fill}\" x=\"{rx}\" y=\"{ry}\" width=\"{rw}\" height=\"{h}\" rx=\"17\" ry=\"17\" fill=\"{fill}\" stroke=\"#92400e\" stroke-width=\"1.5\"/>",
        rx = rx, ry = ry, rw = root_w, h = NODE_H,
        root_attrs = root_attrs,
        child_count = family_tree_child_indices(nodes, 0).len(),
        fill = escape_text(family_node_fill(&nodes[0], mindmap_node_fill(0)))
    ));
    let root_label_attrs = puml_label_attrs(
        &root_id,
        "node-label",
        label_bbox(root_cx, root_cy, &nodes[0].name, 13),
    );
    out.push_str(&render_multiline_text(
        root_cx,
        root_cy,
        &nodes[0].name,
        13,
        "monospace",
        "600",
        "mindmap-label puml-label",
        &root_label_attrs,
    ));

    // Draw right-side branches.
    for &i in &right_roots {
        draw_mindmap_subtree(
            &mut out,
            nodes,
            i,
            root_cx + root_w / 2,
            root_cy,
            root_cx + root_w / 2 + X_STEP - NODE_PAD_X,
            &y_positions,
            X_STEP,
            NODE_H,
            NODE_PAD_X,
            false, // left=false → right
        );
    }
    // Draw left-side branches.
    for &i in &left_roots {
        draw_mindmap_subtree(
            &mut out,
            nodes,
            i,
            root_cx - root_w / 2,
            root_cy,
            root_cx - root_w / 2 - X_STEP + NODE_PAD_X,
            &y_positions,
            X_STEP,
            NODE_H,
            NODE_PAD_X,
            true, // left=true
        );
    }

    // Caption
    if let Some(caption) = &doc.caption {
        out.push_str(&format!(
            "<text x=\"{cx}\" y=\"{cy}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#475569\">{}</text>",
            escape_text(caption),
            cx = canvas_w / 2,
            cy = canvas_h - 8
        ));
    }
    // Legend
    if let Some(legend) = &doc.legend {
        let lx = canvas_w - 160;
        let ly = MARGIN + 10;
        out.push_str(&format!(
            "<rect x=\"{lx}\" y=\"{ly}\" width=\"140\" height=\"50\" rx=\"4\" ry=\"4\" fill=\"#f9fafb\" stroke=\"#94a3b8\" stroke-width=\"1\"/>",
            lx = lx, ly = ly
        ));
        out.push_str(&format!(
            "<text x=\"{tx}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"10\" fill=\"#475569\">{}</text>",
            escape_text(legend),
            tx = lx + 8,
            ty = ly + 18
        ));
    }

    out.push_str("</svg>");
    out
}

fn assign_y_positions(
    nodes: &[crate::model::FamilyNode],
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

fn subtree_leaf_count_render(nodes: &[crate::model::FamilyNode], idx: usize) -> usize {
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

fn family_tree_child_indices(nodes: &[crate::model::FamilyNode], idx: usize) -> Vec<usize> {
    let depth = nodes[idx].depth;
    (idx + 1..nodes.len())
        .take_while(|&j| nodes[j].depth > depth)
        .filter(|&j| nodes[j].depth == depth + 1)
        .collect()
}

fn node_sibling_index(nodes: &[crate::model::FamilyNode], idx: usize) -> usize {
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

fn parent_index(nodes: &[crate::model::FamilyNode], idx: usize) -> Option<usize> {
    if idx == 0 {
        return None;
    }
    let depth = nodes[idx].depth;
    (0..idx).rev().find(|&prev| nodes[prev].depth + 1 == depth)
}

#[allow(clippy::too_many_arguments)]
fn draw_mindmap_subtree(
    out: &mut String,
    nodes: &[crate::model::FamilyNode],
    idx: usize,
    parent_attach_x: i32,
    parent_attach_y: i32,
    node_x_center: i32,
    y_positions: &[i32],
    x_step: i32,
    node_h: i32,
    node_pad_x: i32,
    is_left: bool,
) {
    let node = &nodes[idx];
    let ny = y_positions[idx];
    let nw = (multiline_char_width(&node.name) * 7 + 20).clamp(70, 220);
    let lines = multiline_line_count(&node.name);
    let node_h = if lines > 1 {
        (node_h + (lines - 1) * 16).min(node_h * lines.max(1))
    } else {
        node_h
    };
    let nx = if is_left {
        node_x_center - nw
    } else {
        node_x_center
    };
    let ny_top = ny - node_h / 2;

    // Connection line from parent
    let node_attach_x = if is_left { nx + nw } else { nx };
    let parent_idx = parent_index(nodes, idx).unwrap_or(0);
    let edge_id = tree_edge_id("mindmap", parent_idx, idx);
    let edge_attrs = puml_edge_attrs(
        &edge_id,
        "mindmap",
        "parent-child",
        &tree_node_id("mindmap", parent_idx),
        &tree_node_id("mindmap", idx),
    );
    out.push_str(&format!(
        "<line class=\"mindmap-edge puml-edge\" {edge_attrs} data-mindmap-side=\"{side}\" x1=\"{px}\" y1=\"{py}\" x2=\"{ax}\" y2=\"{ny}\" stroke=\"#94a3b8\" stroke-width=\"1.5\"/>",
        edge_attrs = edge_attrs,
        side = if is_left { "left" } else { "right" },
        px = parent_attach_x,
        py = parent_attach_y,
        ax = node_attach_x,
        ny = ny
    ));

    let children = family_tree_child_indices(nodes, idx);
    let child_count = children.len();
    let sibling_index = node_sibling_index(nodes, idx);
    let branch_class = if child_count == 0 {
        "mindmap-leaf"
    } else {
        "mindmap-branch"
    };

    // Node rectangle (rounded, pastel by depth)
    let node_id = tree_node_id("mindmap", idx);
    let node_attrs = puml_node_attrs(
        &node_id,
        "mindmap",
        "tree-node",
        rect_from_i32(nx, ny_top, nw, node_h),
    );
    out.push_str(&format!(
        "<rect class=\"mindmap-node mindmap-depth-{depth} {branch_class} puml-node\" {node_attrs} data-mindmap-depth=\"{depth}\" data-mindmap-side=\"{side}\" data-mindmap-child-count=\"{child_count}\" data-mindmap-sibling-index=\"{sibling_index}\" data-mindmap-fill=\"{fill}\" x=\"{nx}\" y=\"{ny_top}\" width=\"{nw}\" height=\"{nh}\" rx=\"14\" ry=\"14\" fill=\"{fill}\" stroke=\"#64748b\" stroke-width=\"1\"/>",
        depth = node.depth,
        branch_class = branch_class,
        node_attrs = node_attrs,
        side = if is_left { "left" } else { "right" },
        child_count = child_count,
        sibling_index = sibling_index,
        nx = nx, ny_top = ny_top, nw = nw, nh = node_h,
        fill = escape_text(family_node_fill(node, mindmap_node_fill(node.depth)))
    ));
    let label_attrs = puml_label_attrs(
        &node_id,
        "node-label",
        label_bbox(nx + nw / 2, ny, &node.name, 12),
    );
    out.push_str(&render_multiline_text(
        nx + nw / 2,
        ny,
        &node.name,
        12,
        "monospace",
        "400",
        "mindmap-label puml-label",
        &label_attrs,
    ));

    let next_x_center = if is_left {
        node_x_center - x_step
    } else {
        node_x_center + x_step + nw - node_pad_x
    };
    let from_x = if is_left { nx } else { nx + nw };
    for &child_idx in &children {
        draw_mindmap_subtree(
            out,
            nodes,
            child_idx,
            from_x,
            ny,
            next_x_center,
            y_positions,
            x_step,
            node_h,
            node_pad_x,
            is_left,
        );
    }
}

fn mindmap_empty_svg(doc: &FamilyDocument) -> String {
    let mut out = String::new();
    out.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"300\" height=\"80\" viewBox=\"0 0 300 80\">");
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    if let Some(title) = &doc.title {
        out.push_str(&format!(
            "<text x=\"12\" y=\"28\" font-family=\"monospace\" font-size=\"14\" font-weight=\"600\">{}</text>",
            escape_text(title)
        ));
    }
    out.push_str("<text x=\"12\" y=\"52\" font-family=\"monospace\" font-size=\"12\" fill=\"#64748b\">(empty mindmap)</text>");
    out.push_str("</svg>");
    out
}
