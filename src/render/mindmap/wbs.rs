use super::super::{escape_text, FamilyDocument, FamilyOrientation, WbsCheckbox};
use super::style::{mindmap_node_border_color, mindmap_style, tree_node_fill_resolved};
use super::tree::{family_tree_child_indices, node_sibling_index};
use super::wbs_scene::{build_wbs_artifact, wbs_empty_svg};
use crate::output::RenderArtifact;
use std::collections::BTreeMap;

// ─── WBS renderer ─────────────────────────────────────────────────────────────

pub fn render_wbs_svg(doc: &FamilyDocument) -> String {
    render_wbs_artifact(doc).svg
}

/// Render a `@startwbs` document into a typed [`RenderArtifact`].
///
/// Layout: vertical tree, top-down, rectangular nodes. WBS annotations
/// (`[x]`, `[ ]`, `[%NN]`) are rendered inline in the node. SVG output is
/// byte-identical to the legacy `render_wbs_svg`; a [`RenderScene`] is attached
/// so the typed-geometry validation path can inspect the drawn positions.
pub fn render_wbs_artifact(doc: &FamilyDocument) -> RenderArtifact {
    const X_STEP: i32 = 200;
    const Y_STEP: i32 = 54;
    const NODE_H: i32 = 36;
    const MARGIN: i32 = 24;
    const NODE_PAD: i32 = 10;
    const SIBLING_GAP: i32 = 20;

    let nodes = &doc.nodes;
    if nodes.is_empty() {
        return RenderArtifact::svg_only(wbs_empty_svg(doc));
    }
    let style = mindmap_style(doc);

    let n = nodes.len();

    fn wbs_node_width(node: &super::super::FamilyNode) -> i32 {
        (crate::render::text_metrics::default_monospace_width(&node.name) + 24).clamp(80, 200)
    }

    // Count leaves in each subtree for horizontal distribution.
    fn wbs_leaf_count(nodes: &[super::super::FamilyNode], idx: usize) -> usize {
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

    // Build child adjacency once so depth and width passes stay in sync.
    let mut children_of = vec![Vec::<usize>::new(); n];
    {
        let mut stack: Vec<usize> = Vec::new();
        for (i, node) in nodes.iter().enumerate() {
            let depth = node.depth;
            while stack.len() > depth {
                stack.pop();
            }
            if let Some(&p) = stack.last() {
                children_of[p].push(i);
            }
            stack.push(i);
        }
    }

    let total_leaves = wbs_leaf_count(nodes, 0);
    let max_depth = nodes.iter().map(|n| n.depth).max().unwrap_or(0);

    // PlantUML parity (#1467): for the default TopToBottom WBS orientation,
    // upstream PlantUML uses a `Fork` at depth 0 + recursive `ITFComposed`
    // vertical-stack layout at depth ≥ 1. Each depth-1 branch sits in a
    // horizontal row below the root; under each branch, descendants stack
    // vertically with horizontal "+-" connectors. This matches PlantUML's
    // byte-identical layout and dramatically reduces canvas width compared
    // to the previous PUML horizontal-spread leaves convention.
    let use_plantuml_topdown_layout = matches!(doc.orientation, FamilyOrientation::TopToBottom);
    let layout_orientation = doc.orientation;
    let layout_vertical = matches!(
        layout_orientation,
        FamilyOrientation::TopToBottom | FamilyOrientation::BottomToTop
    );
    let mut subtree_span = vec![0i32; n];

    fn compute_wbs_subtree_span(
        idx: usize,
        children_of: &[Vec<usize>],
        nodes: &[super::super::FamilyNode],
        sibling_gap: i32,
        subtree_span: &mut [i32],
    ) -> i32 {
        let node_w = wbs_node_width(&nodes[idx]);
        let children = &children_of[idx];
        if children.is_empty() {
            subtree_span[idx] = node_w;
            return node_w;
        }
        let mut total_children = 0;
        for (k, child) in children.iter().enumerate() {
            total_children +=
                compute_wbs_subtree_span(*child, children_of, nodes, sibling_gap, subtree_span);
            if k > 0 {
                total_children += sibling_gap;
            }
        }
        let span = node_w.max(total_children);
        subtree_span[idx] = span;
        span
    }

    let _root_span =
        compute_wbs_subtree_span(0, &children_of, nodes, SIBLING_GAP, &mut subtree_span);

    // PlantUML-style "vertical-stack subtree" geometry. For each non-root node
    // we compute:
    //   - `subtree_block_h[idx]` : total vertical height the subtree (idx + all
    //     descendants) occupies when laid out as a vertical stack.
    //   - `subtree_block_w[idx]` : total horizontal width the subtree occupies
    //     (own width + max child block width + indent).
    // The root itself is placed at depth 0; its depth-1 children are arranged
    // horizontally with each child consuming `subtree_block_w[child]` width.
    const VSTACK_INDENT: i32 = 30; // horizontal indent per nesting level
    const VSTACK_ROW_GAP: i32 = 12; // vertical gap between stacked siblings
    let mut subtree_block_w = vec![0i32; n];
    let mut subtree_block_h = vec![0i32; n];
    if use_plantuml_topdown_layout {
        #[allow(clippy::too_many_arguments)]
        fn compute_vstack_block(
            idx: usize,
            children_of: &[Vec<usize>],
            nodes: &[super::super::FamilyNode],
            node_h: i32,
            row_gap: i32,
            indent: i32,
            subtree_block_w: &mut [i32],
            subtree_block_h: &mut [i32],
        ) {
            let own_w = wbs_node_width(&nodes[idx]);
            let children = &children_of[idx];
            if children.is_empty() {
                subtree_block_w[idx] = own_w;
                subtree_block_h[idx] = node_h;
                return;
            }
            let mut max_child_w = 0;
            let mut total_h = node_h;
            for &c in children {
                compute_vstack_block(
                    c,
                    children_of,
                    nodes,
                    node_h,
                    row_gap,
                    indent,
                    subtree_block_w,
                    subtree_block_h,
                );
                max_child_w = max_child_w.max(subtree_block_w[c]);
                total_h += row_gap + subtree_block_h[c];
            }
            subtree_block_w[idx] = own_w.max(indent + max_child_w);
            subtree_block_h[idx] = total_h;
        }
        // Compute per-depth-1-branch blocks (the root itself uses Fork below).
        for &c in &children_of[0] {
            compute_vstack_block(
                c,
                &children_of,
                nodes,
                NODE_H,
                VSTACK_ROW_GAP,
                VSTACK_INDENT,
                &mut subtree_block_w,
                &mut subtree_block_h,
            );
        }
    }

    let canvas_w = if use_plantuml_topdown_layout {
        // Sum of depth-1 child block widths + gaps + margins. Fall back to
        // root width if there are no children.
        let root_w = wbs_node_width(&nodes[0]);
        let mut sum_w = 0i32;
        for (k, &c) in children_of[0].iter().enumerate() {
            sum_w += subtree_block_w[c];
            if k > 0 {
                sum_w += SIBLING_GAP;
            }
        }
        sum_w.max(root_w) + 2 * MARGIN
    } else if layout_vertical {
        (total_leaves as i32) * X_STEP + 2 * MARGIN
    } else {
        (max_depth as i32 + 1) * X_STEP + 2 * MARGIN + 120
    };
    let canvas_h = if use_plantuml_topdown_layout {
        // Root row + fork gap + max depth-1 child block.
        let max_branch_h = children_of[0]
            .iter()
            .map(|&c| subtree_block_h[c])
            .max()
            .unwrap_or(0);
        NODE_H + Y_STEP + max_branch_h + 2 * MARGIN
    } else if layout_vertical {
        (max_depth as i32 + 1) * Y_STEP + 2 * MARGIN + NODE_H
    } else {
        (total_leaves as i32) * Y_STEP + 2 * MARGIN + NODE_H
    };

    let mut x_positions = vec![0i32; n];
    let mut y_positions = vec![0i32; n];

    // Assign x positions by leaf-count distribution, y by depth.
    #[allow(clippy::too_many_arguments)]
    fn assign_wbs_positions(
        nodes: &[super::super::FamilyNode],
        idx: usize,
        x_start: i32,
        x_step: i32,
        margin: i32,
        node_h: i32,
        y_step: i32,
        orientation: FamilyOrientation,
        max_depth: usize,
        use_compact_vertical_layout: bool,
        subtree_span: &[i32],
        children_of: &[Vec<usize>],
        sibling_gap: i32,
        x_positions: &mut [i32],
        y_positions: &mut [i32],
    ) {
        let depth = nodes[idx].depth;
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
            let cx = if use_compact_vertical_layout {
                let span = subtree_span[idx];
                x_start + span / 2
            } else {
                let leaves = wbs_leaf_count(nodes, idx);
                x_start + (leaves as i32 * x_step) / 2
            };
            x_positions[idx] = cx;
            y_positions[idx] = margin + (display_depth as i32) * y_step + node_h / 2;
        } else {
            let leaves = wbs_leaf_count(nodes, idx);
            let cy = x_start + (leaves as i32 * y_step) / 2;
            x_positions[idx] = margin + (display_depth as i32) * x_step + 80;
            y_positions[idx] = cy;
        }
        let children = &children_of[idx];
        let mut child_x = if vertical {
            if use_compact_vertical_layout {
                let total_children_span = children
                    .iter()
                    .enumerate()
                    .map(|(k, c)| subtree_span[*c] + if k == 0 { 0 } else { sibling_gap })
                    .sum::<i32>();
                x_start + (subtree_span[idx] - total_children_span) / 2
            } else {
                x_start
            }
        } else {
            x_start
        };
        let leaf_step = y_step;
        for &c in children {
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
                use_compact_vertical_layout,
                subtree_span,
                children_of,
                sibling_gap,
                x_positions,
                y_positions,
            );
            child_x += if vertical {
                if use_compact_vertical_layout {
                    subtree_span[c] + sibling_gap
                } else {
                    wbs_leaf_count(nodes, c) as i32 * x_step
                }
            } else {
                wbs_leaf_count(nodes, c) as i32 * leaf_step
            };
        }
    }

    if use_plantuml_topdown_layout {
        // Root sits at top center; depth-1 children fork horizontally below.
        let root_w = wbs_node_width(&nodes[0]);
        let mut sum_branch_w = 0i32;
        for (k, &c) in children_of[0].iter().enumerate() {
            sum_branch_w += subtree_block_w[c];
            if k > 0 {
                sum_branch_w += SIBLING_GAP;
            }
        }
        let total_block_w = sum_branch_w.max(root_w);
        let block_start_x = (canvas_w - total_block_w) / 2;
        x_positions[0] = block_start_x + total_block_w / 2;
        y_positions[0] = MARGIN + NODE_H / 2;

        // Layout each depth-1 branch as a vertical-stack subtree.
        #[allow(clippy::too_many_arguments)]
        fn assign_vstack_subtree(
            idx: usize,
            origin_x: i32, // left edge of this subtree's column
            origin_y: i32, // top of this subtree's own node
            node_h: i32,
            row_gap: i32,
            indent: i32,
            children_of: &[Vec<usize>],
            nodes: &[super::super::FamilyNode],
            subtree_block_h: &[i32],
            x_positions: &mut [i32],
            y_positions: &mut [i32],
        ) {
            let own_w = wbs_node_width(&nodes[idx]);
            // Place node so its left edge sits at the column origin; descendants
            // are indented right (PlantUML ITFComposed vertical-stack layout).
            x_positions[idx] = origin_x + own_w / 2;
            y_positions[idx] = origin_y + node_h / 2;
            let mut cur_y = origin_y + node_h + row_gap;
            for &c in &children_of[idx] {
                assign_vstack_subtree(
                    c,
                    origin_x + indent,
                    cur_y,
                    node_h,
                    row_gap,
                    indent,
                    children_of,
                    nodes,
                    subtree_block_h,
                    x_positions,
                    y_positions,
                );
                cur_y += subtree_block_h[c] + row_gap;
            }
        }

        let mut x_cursor = block_start_x;
        let branch_top_y = MARGIN + NODE_H + Y_STEP;
        for &c in &children_of[0] {
            let block_w = subtree_block_w[c];
            assign_vstack_subtree(
                c,
                x_cursor,
                branch_top_y,
                NODE_H,
                VSTACK_ROW_GAP,
                VSTACK_INDENT,
                &children_of,
                nodes,
                &subtree_block_h,
                &mut x_positions,
                &mut y_positions,
            );
            x_cursor += block_w + SIBLING_GAP;
        }
    } else {
        assign_wbs_positions(
            nodes,
            0,
            MARGIN,
            X_STEP,
            MARGIN,
            NODE_H,
            Y_STEP,
            layout_orientation,
            max_depth,
            false,
            &subtree_span,
            &children_of,
            SIBLING_GAP,
            &mut x_positions,
            &mut y_positions,
        );
    }

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
    for (p, children) in children_of.iter().enumerate() {
        for &c in children {
            parent_of[c] = Some(p);
        }
    }

    // Draw edges (parent → child).
    for i in 0..n {
        if let Some(p) = parent_of[i] {
            let parent_w = wbs_node_width(&nodes[p]);
            let child_w = wbs_node_width(&nodes[i]);
            // PlantUML-parity vertical-stack: at depth ≥ 1, draw an orthogonal
            // "+-" connector (vertical drop from parent's left edge, then
            // horizontal segment to child's left edge). At depth 0 (root → its
            // depth-1 children) keep the straight Fork-style edge.
            if use_plantuml_topdown_layout && nodes[i].depth >= 2 {
                let px = x_positions[p] - parent_w / 2;
                let py = y_positions[p] + NODE_H / 2;
                let cx = x_positions[i] - child_w / 2;
                let cy = y_positions[i];
                // Vertical drop
                out.push_str(&format!(
                    "<line class=\"wbs-edge\" data-wbs-edge-depth=\"{depth}\" x1=\"{px}\" y1=\"{py}\" x2=\"{px}\" y2=\"{cy}\" stroke=\"#94a3b8\" stroke-width=\"1.5\"/>",
                    depth = nodes[i].depth,
                    px = px, py = py, cy = cy
                ));
                // Horizontal segment
                out.push_str(&format!(
                    "<line class=\"wbs-edge\" data-wbs-edge-depth=\"{depth}\" x1=\"{px}\" y1=\"{cy}\" x2=\"{cx}\" y2=\"{cy}\" stroke=\"#94a3b8\" stroke-width=\"1.5\"/>",
                    depth = nodes[i].depth,
                    px = px, cx = cx, cy = cy
                ));
                continue;
            }
            let (px, py, cx, cy) = match layout_orientation {
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

    let mut id_to_idx: BTreeMap<String, usize> = BTreeMap::new();
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
        let (sx, sy, ex, ey) = match layout_orientation {
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
        let fill = tree_node_fill_resolved(node, style, default_fill);
        let default_stroke = if node.depth == 0 {
            "#92400e"
        } else {
            "#64748b"
        };
        let stroke = mindmap_node_border_color(node.depth, style, default_stroke);
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
            fill = escape_text(&fill),
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

    build_wbs_artifact(
        out,
        nodes,
        &x_positions,
        &y_positions,
        &children_of,
        &parent_of,
        canvas_w,
        canvas_h,
        NODE_H,
        use_plantuml_topdown_layout,
    )
}

pub(super) fn wbs_orientation_attr(orientation: FamilyOrientation) -> &'static str {
    match orientation {
        FamilyOrientation::TopToBottom => "top-to-bottom",
        FamilyOrientation::LeftToRight => "left-to-right",
        FamilyOrientation::BottomToTop => "bottom-to-top",
        FamilyOrientation::RightToLeft => "right-to-left",
    }
}
