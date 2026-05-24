use super::super::{escape_text, FamilyDocument, FamilyNode};
use super::labels::{multiline_char_width, multiline_line_count, render_mindmap_node_label};
use super::style::{
    mindmap_node_border_color, mindmap_node_fill_resolved, mindmap_node_font_color,
};
use super::tree::{family_tree_child_indices, node_sibling_index};
use super::{MINDMAP_CHAR_PX, MINDMAP_NODE_PAD_X};

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_mindmap_subtree(
    out: &mut String,
    nodes: &[FamilyNode],
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

pub(super) fn mindmap_empty_svg(doc: &FamilyDocument) -> String {
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
