use super::*;

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
            let edge_id = tree_edge_id("wbs", p, i);
            let edge_attrs = puml_edge_attrs(
                &edge_id,
                "wbs",
                "parent-child",
                &tree_node_id("wbs", p),
                &tree_node_id("wbs", i),
            );
            out.push_str(&format!(
                "<line class=\"wbs-edge puml-edge\" {edge_attrs} data-wbs-edge-depth=\"{depth}\" x1=\"{px}\" y1=\"{py}\" x2=\"{cx}\" y2=\"{cy}\" stroke=\"#94a3b8\" stroke-width=\"1.5\"/>",
                edge_attrs = edge_attrs,
                depth = nodes[i].depth,
                px = px, py = py, cx = cx, cy = cy
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
        let node_id = tree_node_id("wbs", i);
        let node_attrs = puml_node_attrs(
            &node_id,
            "wbs",
            "tree-node",
            rect_from_i32(nx, ny, nw, NODE_H),
        );
        out.push_str(&format!(
            "<rect class=\"wbs-node wbs-depth-{depth}{checkbox_class}{branch_class} puml-node\" {node_attrs} data-wbs-depth=\"{depth}\" data-wbs-child-count=\"{child_count}\" data-wbs-sibling-index=\"{sibling_index}\" data-wbs-fill=\"{fill}\"{checkbox_attr} x=\"{nx}\" y=\"{ny}\" width=\"{nw}\" height=\"{nh}\" rx=\"4\" ry=\"4\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
            depth = node.depth,
            checkbox_class = checkbox_class,
            branch_class = branch_class,
            node_attrs = node_attrs,
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
                let label_attrs = puml_label_attrs(
                    &node_id,
                    "node-label",
                    label_bbox(cx + 8, cy, &node.name, 12),
                );
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
                    "<text class=\"wbs-label puml-label\" {label_attrs} x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" dominant-baseline=\"middle\" font-family=\"monospace\" font-size=\"12\">{}</text>",
                    escape_text(&node.name), label_attrs = label_attrs, tx = cx + 8, ty = cy
                ));
            }
            Some(WbsCheckbox::Unchecked) => {
                let label_attrs = puml_label_attrs(
                    &node_id,
                    "node-label",
                    label_bbox(cx + 8, cy, &node.name, 12),
                );
                out.push_str(&format!(
                    "<rect class=\"wbs-checkbox-box\" data-wbs-annotation-style=\"unchecked\" x=\"{bx}\" y=\"{by}\" width=\"12\" height=\"12\" rx=\"2\" ry=\"2\" fill=\"#fff\" stroke=\"#64748b\" stroke-width=\"1\"/>",
                    bx = nx + NODE_PAD, by = cy - 6
                ));
                out.push_str(&format!(
                    "<text class=\"wbs-label puml-label\" {label_attrs} x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" dominant-baseline=\"middle\" font-family=\"monospace\" font-size=\"12\">{}</text>",
                    escape_text(&node.name), label_attrs = label_attrs, tx = cx + 8, ty = cy
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
                let label = format!("{} [{}%]", node.name, pct);
                let label_attrs =
                    puml_label_attrs(&node_id, "node-label", label_bbox(cx, cy - 2, &label, 12));
                out.push_str(&format!(
                    "<text class=\"wbs-label puml-label\" {label_attrs} x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" dominant-baseline=\"middle\" font-family=\"monospace\" font-size=\"12\">{} [{}%]</text>",
                    escape_text(&node.name), pct, label_attrs = label_attrs, tx = cx, ty = cy - 2
                ));
            }
            None => {
                let label_attrs =
                    puml_label_attrs(&node_id, "node-label", label_bbox(cx, cy, &node.name, 12));
                out.push_str(&format!(
                    "<text class=\"wbs-label puml-label\" {label_attrs} x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" dominant-baseline=\"middle\" font-family=\"monospace\" font-size=\"12\">{}</text>",
                    escape_text(&node.name), label_attrs = label_attrs, tx = cx, ty = cy
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

pub(super) fn wbs_orientation_attr(orientation: FamilyOrientation) -> &'static str {
    match orientation {
        FamilyOrientation::TopToBottom => "top-to-bottom",
        FamilyOrientation::LeftToRight => "left-to-right",
        FamilyOrientation::BottomToTop => "bottom-to-top",
        FamilyOrientation::RightToLeft => "right-to-left",
    }
}
