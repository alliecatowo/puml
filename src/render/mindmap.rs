use super::*;
use crate::creole::tokenize_creole;

const MINDMAP_CHAR_PX: i32 = 7;
const MINDMAP_NODE_PAD_X: i32 = 20;

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

fn mindmap_style(doc: &FamilyDocument) -> Option<&crate::theme::MindMapStyle> {
    match &doc.family_style {
        Some(crate::model::FamilyStyle::MindMap(style)) => Some(style),
        _ => None,
    }
}

fn mindmap_node_fill_resolved(
    node: &crate::model::FamilyNode,
    style: Option<&crate::theme::MindMapStyle>,
) -> String {
    node.fill_color
        .clone()
        .or_else(|| {
            style
                .and_then(|s| s.depth_styles.get(&node.depth))
                .and_then(|s| s.background_color.clone())
        })
        .unwrap_or_else(|| mindmap_node_fill(node.depth).to_string())
}

fn mindmap_node_font_color<'a>(
    depth: usize,
    style: Option<&'a crate::theme::MindMapStyle>,
    fallback: &'a str,
) -> &'a str {
    style
        .and_then(|s| s.depth_styles.get(&depth))
        .and_then(|s| s.font_color.as_deref())
        .unwrap_or(fallback)
}

fn mindmap_node_border_color<'a>(
    depth: usize,
    style: Option<&'a crate::theme::MindMapStyle>,
    fallback: &'a str,
) -> &'a str {
    style
        .and_then(|s| s.depth_styles.get(&depth))
        .and_then(|s| s.border_color.as_deref())
        .unwrap_or(fallback)
}

fn mindmap_max_chars(maximum_width: Option<i32>) -> Option<usize> {
    let px = maximum_width.filter(|w| *w > 0)?;
    let inner = px.saturating_sub(MINDMAP_NODE_PAD_X);
    Some((inner / MINDMAP_CHAR_PX).max(1) as usize)
}

/// Word-wrap `text` at `max_chars` per line (monospace heuristic, 7px/char).
fn wrap_mindmap_label(text: &str, max_chars: usize) -> String {
    text.split('\n')
        .flat_map(|line| wrap_mindmap_line(line, max_chars))
        .collect::<Vec<_>>()
        .join("\n")
}

fn wrap_mindmap_line(line: &str, max_chars: usize) -> Vec<String> {
    if line.is_empty() {
        return vec![String::new()];
    }
    let words = line.split_whitespace().collect::<Vec<_>>();
    if words.is_empty() {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    for word in words {
        let word_len = word.chars().count();
        if current.is_empty() {
            if word_len <= max_chars {
                current.push_str(word);
            } else {
                lines.extend(chunk_mindmap_word(word, max_chars));
            }
            continue;
        }
        let next_len = current.chars().count() + 1 + word_len;
        if next_len <= max_chars {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            if word_len <= max_chars {
                current = word.to_string();
            } else {
                let mut chunks = chunk_mindmap_word(word, max_chars);
                let tail = chunks.pop().unwrap_or_default();
                lines.extend(chunks);
                current = tail;
            }
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn chunk_mindmap_word(text: &str, max_chars: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        current.push(ch);
        if current.chars().count() >= max_chars {
            out.push(current);
            current = String::new();
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    if out.is_empty() {
        vec![String::new()]
    } else {
        out
    }
}

fn prepare_mindmap_label(raw: &str, maximum_width: Option<i32>) -> String {
    match mindmap_max_chars(maximum_width) {
        Some(max_chars) => wrap_mindmap_label(raw, max_chars),
        None => raw.to_string(),
    }
}

fn mindmap_label_attrs(font_size: i32, font_family: &str, font_weight: &str) -> String {
    format!(
        "text-anchor=\"middle\" dominant-baseline=\"middle\" font-family=\"{font_family}\" font-size=\"{font_size}\" font-weight=\"{font_weight}\""
    )
}

/// Emit a centered multi-line `<text>` element with Creole markup support.
fn render_mindmap_node_label(
    x: i32,
    y_center: i32,
    text: &str,
    font_size: i32,
    font_family: &str,
    font_weight: &str,
    font_color: &str,
) -> String {
    let attrs = mindmap_label_attrs(font_size, font_family, font_weight);
    let lines: Vec<&str> = text.split('\n').collect();
    if lines.len() <= 1 {
        return creole_text(x, y_center, &attrs, text, font_color);
    }

    let creole_lines = tokenize_creole(text);
    let line_h = (font_size as f32 * 1.25) as i32;
    let n = creole_lines.len() as i32;
    let total_h = line_h * (n - 1);
    let start_y = y_center - total_h / 2;
    let mut out = format!("<text x=\"{x}\" y=\"{start_y}\" {attrs}>");
    for (i, line) in creole_lines.iter().enumerate() {
        let y = start_y + (i as i32) * line_h;
        let inner = render_creole_line_to_tspans_inline(line, font_color);
        out.push_str(&format!(
            "<tspan x=\"{x}\" y=\"{y}\">{inner}</tspan>",
            x = x,
            y = y
        ));
    }
    out.push_str("</text>");
    out
}

fn render_creole_line_to_tspans_inline(
    line: &crate::creole::CreoleLine,
    default_color: &str,
) -> String {
    use crate::creole::render_creole_line_to_tspans;
    render_creole_line_to_tspans(line, 0, default_color)
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

    let maximum_width = doc.maximum_width;
    let style = mindmap_style(doc);
    let display_names: Vec<String> = doc
        .nodes
        .iter()
        .map(|node| prepare_mindmap_label(&node.name, maximum_width))
        .collect();

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
    fn node_width(name: &str, maximum_width: Option<i32>) -> i32 {
        let chars = multiline_char_width(name);
        let heuristic = chars * MINDMAP_CHAR_PX + MINDMAP_NODE_PAD_X;
        match maximum_width.filter(|w| *w > 0) {
            Some(max_px) => heuristic.clamp(70, max_px),
            None => heuristic.clamp(70, 220),
        }
    }

    let root_w = node_width(&display_names[0], maximum_width);
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

    #[allow(clippy::too_many_arguments)]
    fn subtree_bounds(
        nodes: &[crate::model::FamilyNode],
        display_names: &[String],
        idx: usize,
        node_x_center: i32,
        x_step: i32,
        node_pad_x: i32,
        is_left: bool,
        maximum_width: Option<i32>,
    ) -> (i32, i32) {
        let nw = node_width(&display_names[idx], maximum_width);
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
                let (child_min, child_max) = subtree_bounds(
                    nodes,
                    display_names,
                    c,
                    next_x_center,
                    x_step,
                    node_pad_x,
                    is_left,
                    maximum_width,
                );
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
            .map(|&i| {
                subtree_bounds(
                    nodes,
                    &display_names,
                    i,
                    right_start,
                    X_STEP,
                    NODE_PAD_X,
                    false,
                    maximum_width,
                )
                .1
            })
            .max()
            .unwrap_or(root_max_x)
    };
    let actual_min_left = {
        let left_start = root_cx_prelim - root_w / 2 - X_STEP + NODE_PAD_X;
        left_roots
            .iter()
            .map(|&i| {
                subtree_bounds(
                    nodes,
                    &display_names,
                    i,
                    left_start,
                    X_STEP,
                    NODE_PAD_X,
                    true,
                    maximum_width,
                )
                .0
            })
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
    let root_fill = mindmap_node_fill_resolved(&nodes[0], style);
    let root_border = mindmap_node_border_color(0, style, "#92400e");
    let root_font_color = mindmap_node_font_color(0, style, "#111827");
    out.push_str(&format!(
        "<rect class=\"mindmap-node mindmap-root mindmap-branch\" data-mindmap-depth=\"0\" data-mindmap-child-count=\"{child_count}\" data-mindmap-fill=\"{fill}\" x=\"{rx}\" y=\"{ry}\" width=\"{rw}\" height=\"{h}\" rx=\"17\" ry=\"17\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
        rx = rx, ry = ry, rw = root_w, h = NODE_H,
        child_count = family_tree_child_indices(nodes, 0).len(),
        fill = escape_text(&root_fill),
        stroke = escape_text(root_border)
    ));
    out.push_str(&render_mindmap_node_label(
        root_cx,
        root_cy,
        &display_names[0],
        13,
        "monospace",
        "600",
        root_font_color,
    ));

    // Draw right-side branches.
    for &i in &right_roots {
        draw_mindmap_subtree(
            &mut out,
            nodes,
            &display_names,
            i,
            root_cx + root_w / 2,
            root_cy,
            root_cx + root_w / 2 + X_STEP - NODE_PAD_X,
            &y_positions,
            X_STEP,
            NODE_H,
            NODE_PAD_X,
            false, // left=false → right
            maximum_width,
            style,
        );
    }
    // Draw left-side branches.
    for &i in &left_roots {
        draw_mindmap_subtree(
            &mut out,
            nodes,
            &display_names,
            i,
            root_cx - root_w / 2,
            root_cy,
            root_cx - root_w / 2 - X_STEP + NODE_PAD_X,
            &y_positions,
            X_STEP,
            NODE_H,
            NODE_PAD_X,
            true, // left=true
            maximum_width,
            style,
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

#[allow(clippy::too_many_arguments)]
fn draw_mindmap_subtree(
    out: &mut String,
    nodes: &[crate::model::FamilyNode],
    display_names: &[String],
    idx: usize,
    parent_attach_x: i32,
    parent_attach_y: i32,
    node_x_center: i32,
    y_positions: &[i32],
    x_step: i32,
    node_h: i32,
    node_pad_x: i32,
    is_left: bool,
    maximum_width: Option<i32>,
    style: Option<&crate::theme::MindMapStyle>,
) {
    let node = &nodes[idx];
    let label = &display_names[idx];
    let ny = y_positions[idx];
    let chars = multiline_char_width(label);
    let heuristic = chars * MINDMAP_CHAR_PX + MINDMAP_NODE_PAD_X;
    let nw = match maximum_width.filter(|w| *w > 0) {
        Some(max_px) => heuristic.clamp(70, max_px),
        None => heuristic.clamp(70, 220),
    };
    let lines = multiline_line_count(label);
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
    out.push_str(&format!(
        "<line class=\"mindmap-edge\" data-mindmap-side=\"{side}\" x1=\"{px}\" y1=\"{py}\" x2=\"{ax}\" y2=\"{ny}\" stroke=\"#94a3b8\" stroke-width=\"1.5\"/>",
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

    let fill = mindmap_node_fill_resolved(node, style);
    let stroke = mindmap_node_border_color(node.depth, style, "#64748b");
    let font_color = mindmap_node_font_color(node.depth, style, "#111827");

    // Node rectangle (rounded, pastel by depth unless overridden by style)
    out.push_str(&format!(
        "<rect class=\"mindmap-node mindmap-depth-{depth} {branch_class}\" data-mindmap-depth=\"{depth}\" data-mindmap-side=\"{side}\" data-mindmap-child-count=\"{child_count}\" data-mindmap-sibling-index=\"{sibling_index}\" data-mindmap-fill=\"{fill}\" x=\"{nx}\" y=\"{ny_top}\" width=\"{nw}\" height=\"{nh}\" rx=\"14\" ry=\"14\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
        depth = node.depth,
        branch_class = branch_class,
        side = if is_left { "left" } else { "right" },
        child_count = child_count,
        sibling_index = sibling_index,
        nx = nx, ny_top = ny_top, nw = nw, nh = node_h,
        fill = escape_text(&fill),
        stroke = escape_text(stroke)
    ));
    out.push_str(&render_mindmap_node_label(
        nx + nw / 2,
        ny,
        label,
        12,
        "monospace",
        "400",
        font_color,
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
            display_names,
            child_idx,
            from_x,
            ny,
            next_x_center,
            y_positions,
            x_step,
            node_h,
            node_pad_x,
            is_left,
            maximum_width,
            style,
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

// ─── WBS renderer ─────────────────────────────────────────────────────────────

/// Render a `@startwbs` document as SVG.
///
/// Layout: vertical tree, top-down, rectangular nodes. WBS annotations
/// (`[x]`, `[ ]`, `[%NN]`) are rendered inline in the node.
pub fn render_wbs_svg(doc: &FamilyDocument) -> String {
    const X_STEP: i32 = 200;
    const Y_STEP: i32 = 54;
    const NODE_H: i32 = 36;
    const MARGIN: i32 = 24;
    const NODE_PAD: i32 = 10;

    let nodes = &doc.nodes;
    if nodes.is_empty() {
        return wbs_empty_svg(doc);
    }

    let n = nodes.len();

    fn wbs_node_width(node: &crate::model::FamilyNode) -> i32 {
        (node.name.chars().count() as i32 * 7 + 24).clamp(80, 200)
    }

    // Count leaves in each subtree for horizontal distribution.
    fn wbs_leaf_count(nodes: &[crate::model::FamilyNode], idx: usize) -> usize {
        let depth = nodes[idx].depth;
        let children: Vec<usize> = (idx + 1..nodes.len())
            .take_while(|&j| nodes[j].depth > depth)
            .filter(|&j| nodes[j].depth == depth + 1)
            .collect();
        if children.is_empty() {
            return 1;
        }
        children.iter().map(|&c| wbs_leaf_count(nodes, c)).sum()
    }

    let total_leaves = wbs_leaf_count(nodes, 0);
    let max_depth = nodes.iter().map(|n| n.depth).max().unwrap_or(0);
    let vertical = matches!(
        doc.orientation,
        FamilyOrientation::TopToBottom | FamilyOrientation::BottomToTop
    );
    let canvas_w = if vertical {
        (total_leaves as i32) * X_STEP + 2 * MARGIN
    } else {
        (max_depth as i32 + 1) * X_STEP + 2 * MARGIN + 120
    };
    let canvas_h = if vertical {
        (max_depth as i32 + 1) * Y_STEP + 2 * MARGIN + NODE_H
    } else {
        (total_leaves as i32) * Y_STEP + 2 * MARGIN + NODE_H
    };

    let mut x_positions = vec![0i32; n];
    let mut y_positions = vec![0i32; n];

    // Assign x positions by leaf-count distribution, y by depth.
    #[allow(clippy::too_many_arguments)]
    fn assign_wbs_positions(
        nodes: &[crate::model::FamilyNode],
        idx: usize,
        x_start: i32,
        x_step: i32,
        margin: i32,
        node_h: i32,
        y_step: i32,
        orientation: FamilyOrientation,
        max_depth: usize,
        x_positions: &mut [i32],
        y_positions: &mut [i32],
    ) {
        let depth = nodes[idx].depth;
        let leaves = wbs_leaf_count(nodes, idx);
        let vertical = matches!(
            orientation,
            FamilyOrientation::TopToBottom | FamilyOrientation::BottomToTop
        );
        let display_depth = match orientation {
            FamilyOrientation::TopToBottom | FamilyOrientation::LeftToRight => depth,
            FamilyOrientation::BottomToTop | FamilyOrientation::RightToLeft => {
                max_depth.saturating_sub(depth)
            }
        };
        if vertical {
            let cx = x_start + (leaves as i32 * x_step) / 2;
            x_positions[idx] = cx;
            y_positions[idx] = margin + (display_depth as i32) * y_step + node_h / 2;
        } else {
            let cy = x_start + (leaves as i32 * y_step) / 2;
            x_positions[idx] = margin + (display_depth as i32) * x_step + 80;
            y_positions[idx] = cy;
        }

        let children: Vec<usize> = (idx + 1..nodes.len())
            .take_while(|&j| nodes[j].depth > depth)
            .filter(|&j| nodes[j].depth == depth + 1)
            .collect();
        let mut child_x = x_start;
        let leaf_step = if vertical { x_step } else { y_step };
        for &c in &children {
            assign_wbs_positions(
                nodes,
                c,
                child_x,
                x_step,
                margin,
                node_h,
                y_step,
                orientation,
                max_depth,
                x_positions,
                y_positions,
            );
            child_x += wbs_leaf_count(nodes, c) as i32 * leaf_step;
        }
    }

    assign_wbs_positions(
        nodes,
        0,
        MARGIN,
        X_STEP,
        MARGIN,
        NODE_H,
        Y_STEP,
        doc.orientation,
        max_depth,
        &mut x_positions,
        &mut y_positions,
    );

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\" data-wbs-orientation=\"{orientation}\" data-wbs-node-count=\"{node_count}\" data-wbs-leaf-count=\"{leaf_count}\" data-wbs-max-depth=\"{max_depth}\">",
        w = canvas_w,
        h = canvas_h,
        orientation = wbs_orientation_attr(doc.orientation),
        node_count = n,
        leaf_count = total_leaves,
        max_depth = max_depth
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");

    // Title
    if let Some(title) = &doc.title {
        for (li, line) in title.lines().enumerate() {
            out.push_str(&format!(
                "<text x=\"{cx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\">{}</text>",
                escape_text(line),
                cx = canvas_w / 2,
                ty = 20 + li as i32 * 20
            ));
        }
    }

    // Build parent lookup.
    let mut parent_of = vec![None::<usize>; n];
    {
        let mut stack: Vec<usize> = Vec::new();
        for i in 0..n {
            let depth = nodes[i].depth;
            while stack.len() > depth {
                stack.pop();
            }
            if let Some(&p) = stack.last() {
                parent_of[i] = Some(p);
            }
            stack.push(i);
        }
    }

    // Draw edges (parent → child).
    for i in 0..n {
        if let Some(p) = parent_of[i] {
            let parent_w = wbs_node_width(&nodes[p]);
            let child_w = wbs_node_width(&nodes[i]);
            let (px, py, cx, cy) = match doc.orientation {
                FamilyOrientation::TopToBottom => (
                    x_positions[p],
                    y_positions[p] + NODE_H / 2,
                    x_positions[i],
                    y_positions[i] - NODE_H / 2,
                ),
                FamilyOrientation::BottomToTop => (
                    x_positions[p],
                    y_positions[p] - NODE_H / 2,
                    x_positions[i],
                    y_positions[i] + NODE_H / 2,
                ),
                FamilyOrientation::LeftToRight => (
                    x_positions[p] + parent_w / 2,
                    y_positions[p],
                    x_positions[i] - child_w / 2,
                    y_positions[i],
                ),
                FamilyOrientation::RightToLeft => (
                    x_positions[p] - parent_w / 2,
                    y_positions[p],
                    x_positions[i] + child_w / 2,
                    y_positions[i],
                ),
            };
            out.push_str(&format!(
                "<line class=\"wbs-edge\" data-wbs-edge-depth=\"{depth}\" x1=\"{px}\" y1=\"{py}\" x2=\"{cx}\" y2=\"{cy}\" stroke=\"#94a3b8\" stroke-width=\"1.5\"/>",
                depth = nodes[i].depth,
                px = px, py = py, cx = cx, cy = cy
            ));
        }
    }

    let mut id_to_idx: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for (idx, node) in nodes.iter().enumerate() {
        id_to_idx.entry(node.name.clone()).or_insert(idx);
        if let Some(alias) = &node.alias {
            id_to_idx.entry(alias.clone()).or_insert(idx);
        }
    }

    // Draw explicit relation arrows (cross-tree links), resolved by alias or name.
    // Tree parent→child relations are filtered out to avoid duplicate connectors.
    for rel in &doc.relations {
        let Some(&from_idx) = id_to_idx.get(&rel.from) else {
            continue;
        };
        let Some(&to_idx) = id_to_idx.get(&rel.to) else {
            continue;
        };
        if from_idx == to_idx {
            continue;
        }
        if parent_of[to_idx] == Some(from_idx) {
            continue;
        }
        let from_w = wbs_node_width(&nodes[from_idx]);
        let to_w = wbs_node_width(&nodes[to_idx]);
        let (sx, sy, ex, ey) = match doc.orientation {
            FamilyOrientation::TopToBottom | FamilyOrientation::BottomToTop => {
                if x_positions[from_idx] <= x_positions[to_idx] {
                    (
                        x_positions[from_idx] + from_w / 2,
                        y_positions[from_idx],
                        x_positions[to_idx] - to_w / 2,
                        y_positions[to_idx],
                    )
                } else {
                    (
                        x_positions[from_idx] - from_w / 2,
                        y_positions[from_idx],
                        x_positions[to_idx] + to_w / 2,
                        y_positions[to_idx],
                    )
                }
            }
            FamilyOrientation::LeftToRight | FamilyOrientation::RightToLeft => {
                if y_positions[from_idx] <= y_positions[to_idx] {
                    (
                        x_positions[from_idx],
                        y_positions[from_idx] + NODE_H / 2,
                        x_positions[to_idx],
                        y_positions[to_idx] - NODE_H / 2,
                    )
                } else {
                    (
                        x_positions[from_idx],
                        y_positions[from_idx] - NODE_H / 2,
                        x_positions[to_idx],
                        y_positions[to_idx] + NODE_H / 2,
                    )
                }
            }
        };

        out.push_str(&format!(
            "<line class=\"wbs-relation-edge\" data-wbs-relation-from=\"{from}\" data-wbs-relation-to=\"{to}\" x1=\"{sx}\" y1=\"{sy}\" x2=\"{ex}\" y2=\"{ey}\" stroke=\"#334155\" stroke-width=\"1.5\"/>",
            from = escape_text(&rel.from),
            to = escape_text(&rel.to),
            sx = sx,
            sy = sy,
            ex = ex,
            ey = ey
        ));
        // Arrowhead at relation destination.
        let dx = ex - sx;
        let dy = ey - sy;
        let len = ((dx * dx + dy * dy) as f64).sqrt();
        if len >= 1.0 {
            let ux = dx as f64 / len;
            let uy = dy as f64 / len;
            let head_len = 10.0_f64;
            let wing = 4.0_f64;
            let lx = ex as f64 - ux * head_len + uy * wing;
            let ly = ey as f64 - uy * head_len - ux * wing;
            let rx = ex as f64 - ux * head_len - uy * wing;
            let ry = ey as f64 - uy * head_len + ux * wing;
            out.push_str(&format!(
                "<path class=\"wbs-relation-arrowhead\" d=\"M {ex} {ey} L {lx:.2} {ly:.2} L {rx:.2} {ry:.2} Z\" fill=\"#334155\"/>",
                ex = ex,
                ey = ey,
                lx = lx,
                ly = ly,
                rx = rx,
                ry = ry
            ));
        }
    }

    // Draw nodes.
    for i in 0..n {
        let node = &nodes[i];
        let cx = x_positions[i];
        let cy = y_positions[i];
        let nw = wbs_node_width(node);
        let nx = cx - nw / 2;
        let ny = cy - NODE_H / 2;
        let default_fill = if node.depth == 0 {
            "#fde68a"
        } else {
            "#f1f5f9"
        };
        let fill = family_node_fill(node, default_fill);
        let stroke = if node.depth == 0 {
            "#92400e"
        } else {
            "#64748b"
        };
        let (checkbox_class, checkbox_attr) = match &node.wbs_checkbox {
            Some(WbsCheckbox::Checked) => {
                (" wbs-checked", " data-wbs-checkbox=\"checked\"".to_string())
            }
            Some(WbsCheckbox::Unchecked) => (
                " wbs-unchecked",
                " data-wbs-checkbox=\"unchecked\"".to_string(),
            ),
            Some(WbsCheckbox::Progress(pct)) => (
                " wbs-progress",
                format!(" data-wbs-checkbox=\"progress\" data-wbs-progress=\"{pct}\""),
            ),
            None => ("", String::new()),
        };
        let child_count = family_tree_child_indices(nodes, i).len();
        let branch_class = if child_count == 0 {
            " wbs-leaf"
        } else {
            " wbs-branch"
        };
        out.push_str(&format!(
            "<rect class=\"wbs-node wbs-depth-{depth}{checkbox_class}{branch_class}\" data-wbs-depth=\"{depth}\" data-wbs-child-count=\"{child_count}\" data-wbs-sibling-index=\"{sibling_index}\" data-wbs-fill=\"{fill}\"{checkbox_attr} x=\"{nx}\" y=\"{ny}\" width=\"{nw}\" height=\"{nh}\" rx=\"4\" ry=\"4\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
            depth = node.depth,
            checkbox_class = checkbox_class,
            branch_class = branch_class,
            child_count = child_count,
            sibling_index = node_sibling_index(nodes, i),
            checkbox_attr = checkbox_attr,
            nx = nx,
            ny = ny,
            nw = nw,
            nh = NODE_H,
            fill = escape_text(fill),
            stroke = stroke
        ));

        // Render checkbox annotation if present.
        match &node.wbs_checkbox {
            Some(WbsCheckbox::Checked) => {
                // Checked checkbox before label
                out.push_str(&format!(
                    "<rect class=\"wbs-checkbox-box\" data-wbs-annotation-style=\"checked\" x=\"{bx}\" y=\"{by}\" width=\"12\" height=\"12\" rx=\"2\" ry=\"2\" fill=\"#16a34a\" stroke=\"#166534\" stroke-width=\"1\"/>",
                    bx = nx + NODE_PAD, by = cy - 6
                ));
                out.push_str(&format!(
                    "<text x=\"{tx}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"10\" fill=\"white\" font-weight=\"600\">✓</text>",
                    tx = nx + NODE_PAD + 1, ty = cy + 4
                ));
                out.push_str(&format!(
                    "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" dominant-baseline=\"middle\" font-family=\"monospace\" font-size=\"12\">{}</text>",
                    escape_text(&node.name), tx = cx + 8, ty = cy
                ));
            }
            Some(WbsCheckbox::Unchecked) => {
                out.push_str(&format!(
                    "<rect class=\"wbs-checkbox-box\" data-wbs-annotation-style=\"unchecked\" x=\"{bx}\" y=\"{by}\" width=\"12\" height=\"12\" rx=\"2\" ry=\"2\" fill=\"#fff\" stroke=\"#64748b\" stroke-width=\"1\"/>",
                    bx = nx + NODE_PAD, by = cy - 6
                ));
                out.push_str(&format!(
                    "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" dominant-baseline=\"middle\" font-family=\"monospace\" font-size=\"12\">{}</text>",
                    escape_text(&node.name), tx = cx + 8, ty = cy
                ));
            }
            Some(WbsCheckbox::Progress(pct)) => {
                // Progress bar inline
                let bar_w = nw - 2 * NODE_PAD - 4;
                let fill_w = (bar_w as u32 * (*pct as u32) / 100) as i32;
                out.push_str(&format!(
                    "<rect class=\"wbs-progress-track\" data-wbs-annotation-style=\"progress\" x=\"{bx}\" y=\"{by}\" width=\"{bar_w}\" height=\"7\" rx=\"3\" ry=\"3\" fill=\"#e2e8f0\" stroke=\"#94a3b8\" stroke-width=\"0.5\"/>",
                    bx = nx + NODE_PAD, by = cy + 9, bar_w = bar_w
                ));
                if fill_w > 0 {
                    out.push_str(&format!(
                        "<rect class=\"wbs-progress-fill\" data-wbs-progress-fill=\"{pct}\" x=\"{bx}\" y=\"{by}\" width=\"{fill_w}\" height=\"7\" rx=\"3\" ry=\"3\" fill=\"#3b82f6\"/>",
                        bx = nx + NODE_PAD, by = cy + 9, fill_w = fill_w
                    ));
                }
                out.push_str(&format!(
                    "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" dominant-baseline=\"middle\" font-family=\"monospace\" font-size=\"12\">{} [{}%]</text>",
                    escape_text(&node.name), pct, tx = cx, ty = cy - 2
                ));
            }
            None => {
                out.push_str(&format!(
                    "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" dominant-baseline=\"middle\" font-family=\"monospace\" font-size=\"12\">{}</text>",
                    escape_text(&node.name), tx = cx, ty = cy
                ));
            }
        }
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

fn wbs_empty_svg(doc: &FamilyDocument) -> String {
    let mut out = String::new();
    out.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"300\" height=\"80\" viewBox=\"0 0 300 80\">");
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    if let Some(title) = &doc.title {
        out.push_str(&format!(
            "<text x=\"12\" y=\"28\" font-family=\"monospace\" font-size=\"14\" font-weight=\"600\">{}</text>",
            escape_text(title)
        ));
    }
    out.push_str("<text x=\"12\" y=\"52\" font-family=\"monospace\" font-size=\"12\" fill=\"#64748b\">(empty wbs)</text>");
    out.push_str("</svg>");
    out
}

fn wbs_orientation_attr(orientation: FamilyOrientation) -> &'static str {
    match orientation {
        FamilyOrientation::TopToBottom => "top-to-bottom",
        FamilyOrientation::LeftToRight => "left-to-right",
        FamilyOrientation::BottomToTop => "bottom-to-top",
        FamilyOrientation::RightToLeft => "right-to-left",
    }
}
