use super::{escape_text, FamilyDocument, MindMapSide};
use crate::output::RenderArtifact;

pub(super) const MINDMAP_CHAR_PX: i32 = 7;
pub(super) const MINDMAP_NODE_PAD_X: i32 = 20;

mod labels;
mod nodes;
mod scene;
mod style;
mod tree;
mod wbs;
mod wbs_scene;

pub use wbs::{render_wbs_artifact, render_wbs_svg};

use labels::{multiline_char_width, prepare_mindmap_label, render_mindmap_node_label};
use nodes::{draw_mindmap_subtree, mindmap_empty_svg};
use style::mindmap_node_fill_resolved;
use style::{mindmap_node_border_color, mindmap_node_font_color, mindmap_style};
use tree::{assign_y_positions, family_tree_child_indices, subtree_slot_height};
use wbs::wbs_orientation_attr;

pub fn render_mindmap_svg(doc: &FamilyDocument) -> String {
    render_mindmap_artifact(doc).svg
}

/// Render a `@startmindmap` document into a typed [`RenderArtifact`].
///
/// The SVG is emitted unchanged (byte-identical to the legacy `render_mindmap_svg`).
/// A [`RenderScene`] is built from the same laid-out geometry the SVG draws — each
/// node box at its computed `(x, y, w, h)`, and each parent→child connector along
/// the same line segment — so the scene and SVG stay in sync.
pub fn render_mindmap_artifact(doc: &FamilyDocument) -> RenderArtifact {
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
        return RenderArtifact::svg_only(mindmap_empty_svg(doc));
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

    // PlantUML parity (#1467): without explicit `left side` markers, all depth-1
    // branches stay on the right, matching upstream PlantUML's vertical-stack
    // mindmap convention. Users who want a symmetric splay can opt in by
    // tagging individual branches with `left side`. The auto-balance heuristic
    // that previously redistributed half the branches to the left was a PUML
    // chrome flourish that broke 1:1 layout parity (median +0.20×).

    // Collect left/right subtrees at depth 1+.
    let right_roots: Vec<usize> = (0..n)
        .filter(|&i| nodes[i].depth == 1 && side[i] == MindMapSide::Right)
        .collect();
    let left_roots: Vec<usize> = (0..n)
        .filter(|&i| nodes[i].depth == 1 && side[i] == MindMapSide::Left)
        .collect();

    // Compute the total vertical slot (in pixels) consumed by each side's
    // subtrees.  Unlike a simple leaf-count, this accounts for multi-line
    // labels which require taller slots to avoid node overlap.
    let total_right_slots: i32 = right_roots
        .iter()
        .map(|&i| subtree_slot_height(nodes, &display_names, i, NODE_H, Y_STEP))
        .sum();
    let total_left_slots: i32 = left_roots
        .iter()
        .map(|&i| subtree_slot_height(nodes, &display_names, i, NODE_H, Y_STEP))
        .sum();
    let max_slots = total_right_slots.max(total_left_slots).max(Y_STEP);
    let canvas_h = max_slots + 2 * MARGIN + NODE_H;

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

    let mindmap_leaf_count = (0..n)
        .filter(|&idx| family_tree_child_indices(nodes, idx).is_empty())
        .count();
    let max_mindmap_depth = max_right_depth.max(max_left_depth);
    let x_step = if max_mindmap_depth >= 4 && mindmap_leaf_count >= 12 {
        130
    } else {
        X_STEP
    };

    let root_cy = canvas_h / 2;

    // Draw nodes recursively — track y-cursors per side.
    // We assign y by a preorder traversal that uses per-node slot heights so
    // multi-line labels don't cause sibling overlap.
    let mut y_positions = vec![0i32; n];
    {
        // Right side: center the block of subtrees around root_cy.
        let right_start = root_cy - total_right_slots / 2;
        let mut y_cursor = right_start;
        assign_y_positions(
            nodes,
            &display_names,
            &right_roots,
            &mut y_positions,
            &mut y_cursor,
            NODE_H,
            Y_STEP,
        );
        // Left side: same centering logic.
        let left_start = root_cy - total_left_slots / 2;
        y_cursor = left_start;
        assign_y_positions(
            nodes,
            &display_names,
            &left_roots,
            &mut y_positions,
            &mut y_cursor,
            NODE_H,
            Y_STEP,
        );
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

    let depth_bias = (max_left_depth as i32) * x_step + 240;
    let root_cx_prelim = MARGIN + depth_bias + root_w / 2;
    let root_min_x = root_cx_prelim - root_w / 2;
    let root_max_x = root_cx_prelim + root_w / 2;
    let actual_max_right = {
        let right_start = root_cx_prelim + root_w / 2 + x_step - NODE_PAD_X;
        right_roots
            .iter()
            .map(|&i| {
                subtree_bounds(
                    nodes,
                    &display_names,
                    i,
                    right_start,
                    x_step,
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
        let left_start = root_cx_prelim - root_w / 2 - x_step + NODE_PAD_X;
        left_roots
            .iter()
            .map(|&i| {
                subtree_bounds(
                    nodes,
                    &display_names,
                    i,
                    left_start,
                    x_step,
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
    let root_cx = root_cx_prelim + extra_left;

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\" data-mindmap-orientation=\"{orientation}\" data-mindmap-node-count=\"{node_count}\" data-mindmap-leaf-count=\"{leaf_count}\" data-mindmap-max-depth=\"{max_depth}\">",
        w = canvas_w,
        h = canvas_h,
        orientation = wbs_orientation_attr(doc.orientation),
        node_count = n,
        leaf_count = mindmap_leaf_count,
        max_depth = max_mindmap_depth
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
            root_cx + root_w / 2 + x_step - NODE_PAD_X,
            &y_positions,
            x_step,
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
            root_cx - root_w / 2 - x_step + NODE_PAD_X,
            &y_positions,
            x_step,
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

    scene::build_mindmap_artifact(
        out,
        nodes,
        &display_names,
        &parent,
        &side,
        &y_positions,
        &right_roots,
        &left_roots,
        root_cx,
        root_cy,
        root_w,
        NODE_H,
        NODE_PAD_X,
        maximum_width,
        x_step,
        canvas_w,
        canvas_h,
    )
}
