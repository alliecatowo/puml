use super::*;

/// Box geometry for a single class/node box used by `render_class_svg`.
#[derive(Clone, Copy)]
pub(super) struct ClassNodeBox {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) w: i32,
    pub(super) h: i32,
    pub(super) header_h: i32,
}

/// Render the group/package/namespace frames for a class diagram.
///
/// Draws labeled frame rectangles (with optional tab headers) behind all node
/// boxes so that node rectangles visually sit on top of the frame borders.
pub(super) fn render_class_group_frames(
    out: &mut String,
    group_frames: &[RenderGroupFrame],
    max_group_depth: usize,
    node_boxes: &std::collections::BTreeMap<String, ClassNodeBox>,
) {
    for group in group_frames {
        let mut gx_min = i32::MAX;
        let mut gy_min = i32::MAX;
        let mut gx_max = i32::MIN;
        let mut gy_max = i32::MIN;
        let mut found_any = false;
        for member_id in &group.member_ids {
            if let Some(bx) = node_boxes.get(member_id.as_str()) {
                gx_min = gx_min.min(bx.x);
                gy_min = gy_min.min(bx.y);
                gx_max = gx_max.max(bx.x + bx.w);
                gy_max = gy_max.max(bx.y + bx.h);
                found_any = true;
            }
        }
        if !found_any {
            continue;
        }
        let depth_outset = (max_group_depth.saturating_sub(group.depth) as i32) * 18;
        let pad = 20 + depth_outset;
        let tab_h = 24;
        let label_header = tab_h + 28 + depth_outset;
        let fx = gx_min - pad;
        let fy = gy_min - pad - label_header;
        let fw = (gx_max - gx_min) + pad * 2;
        let fh = (gy_max - gy_min) + pad * 2 + label_header;

        let group_label = group.display_label();
        let uses_tab_header = matches!(group.kind.as_str(), "rectangle" | "package");
        let container_attrs = crate::render::puml_container_attrs(
            &group.scope,
            "class",
            &group.kind,
            geometry_bbox(fx, fy, fw, fh),
        );

        out.push_str(&format!(
            "<rect class=\"uml-group-frame puml-container\" data-uml-group=\"{}\" {} x=\"{fx}\" y=\"{fy}\" width=\"{fw}\" height=\"{fh}\" rx=\"6\" ry=\"6\" fill=\"none\" stroke=\"#6366f1\" stroke-width=\"1.5\" stroke-dasharray=\"5 3\"/>",
            escape_text(&group.scope),
            container_attrs
        ));
        if uses_tab_header {
            let tab_w = ((group_label.len() as i32) * 8 + 16).max(60).min(fw);
            out.push_str(&format!(
                "<rect x=\"{fx}\" y=\"{fy}\" width=\"{tab_w}\" height=\"{tab_h}\" rx=\"6\" ry=\"6\" fill=\"#ffffff\" stroke=\"#6366f1\" stroke-width=\"1.5\"/>"
            ));
            out.push_str(&format!(
                "<rect x=\"{fx}\" y=\"{}\" width=\"{tab_w}\" height=\"8\" fill=\"#ffffff\" stroke=\"none\"/>",
                fy + tab_h - 8
            ));
            out.push_str(&format!(
                "<text x=\"{tx}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"11\" font-weight=\"600\" fill=\"#4338ca\">{label}</text>",
                tx = fx + 8,
                ty = fy + 16,
                label = escape_text(&group_label)
            ));
            out.push_str(&format!(
                "<line x1=\"{fx}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#6366f1\" stroke-width=\"1\"/>",
                fy + tab_h,
                fx + fw,
                fy + tab_h
            ));
        } else {
            out.push_str(&format!(
                "<text x=\"{tx}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"11\" font-weight=\"600\" fill=\"#4338ca\">{label}</text>",
                tx = fx + 8,
                ty = fy + 14,
                label = escape_text(&group_label)
            ));
        }
    }
}

/// Nudge a label's y-coordinate upward until it no longer overlaps any node box.
/// Used by `render_class_svg` for both the pre-pass and the inline placement.
pub(super) fn class_nudge_label_y(
    lx: i32,
    ly: i32,
    label_half_w: i32,
    node_boxes: &std::collections::BTreeMap<String, ClassNodeBox>,
) -> (i32, i32) {
    let mut adjusted_y = ly;
    for _ in 0..8 {
        let overlap = node_boxes.values().find(|bbox| {
            lx + label_half_w >= bbox.x - 8
                && lx - label_half_w <= bbox.x + bbox.w + 8
                && adjusted_y >= bbox.y - 14
                && adjusted_y <= bbox.y + bbox.h + 6
        });
        match overlap {
            Some(bbox) => adjusted_y = bbox.y - 18,
            None => break,
        }
    }
    (lx, adjusted_y)
}

/// Run the hierarchical layout engine and populate `node_boxes` for `render_class_svg`.
///
/// Builds `GlNodeSize` / `GlEdgeSpec` inputs for `layout_hierarchical`, runs the
/// layout, then converts the resulting `(f64, f64)` positions into `ClassNodeBox`
/// entries.  Falls back to a simple grid for any node the engine did not place.
/// Returns `(GraphLayout, BTreeMap<String, ClassNodeBox>)`.
#[allow(clippy::too_many_arguments)]
pub(super) fn class_run_layout(
    document: &FamilyDocument,
    node_heights: &[(String, i32)],
    node_width: i32,
    col_count: i32,
    col_gap: i32,
    row_gap: i32,
    margin_x: i32,
    margin_top: i32,
    title_block_height: i32,
    group_top_reserve: i32,
    header_height: i32,
) -> (
    crate::render::graph_layout::GraphLayout,
    std::collections::BTreeMap<String, ClassNodeBox>,
) {
    use crate::render::graph_layout::{
        layout_hierarchical, EdgeSpec as GlEdgeSpec, LayoutOptions as GlOptions,
        NodeSize as GlNodeSize,
    };

    // Build group membership lookup for parent assignment.
    let group_frames_for_gl = collect_render_group_frames(&document.groups);
    let mut node_to_gl_group: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();
    for frame in &group_frames_for_gl {
        for mid in &frame.member_ids {
            node_to_gl_group
                .entry(mid.clone())
                .or_insert_with(|| frame.scope.clone());
        }
    }

    let gl_nodes: Vec<GlNodeSize> = document
        .nodes
        .iter()
        .zip(node_heights.iter())
        .map(|(node, (_key, h))| {
            let key = node.alias.clone().unwrap_or_else(|| node.name.clone());
            let parent = node_to_gl_group
                .get(&key)
                .or_else(|| node_to_gl_group.get(&node.name))
                .cloned();
            GlNodeSize {
                id: key,
                width: node_width as f64,
                height: *h as f64,
                parent,
            }
        })
        .collect();

    // Build a resolver from unscoped/alias names to full node IDs so edges match.
    let mut gl_name_to_id: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();
    for n in &gl_nodes {
        gl_name_to_id
            .entry(n.id.clone())
            .or_insert_with(|| n.id.clone());
        if let Some(tail) = n.id.rsplit("::").next() {
            gl_name_to_id
                .entry(tail.to_string())
                .or_insert_with(|| n.id.clone());
        }
    }
    for node in &document.nodes {
        if let Some(alias) = &node.alias {
            let scoped = node.alias.clone().unwrap_or_else(|| node.name.clone());
            gl_name_to_id
                .entry(alias.clone())
                .or_insert_with(|| scoped.clone());
            gl_name_to_id
                .entry(node.name.clone())
                .or_insert_with(|| scoped);
        }
    }
    let resolve_gl = |name: &str| -> String {
        gl_name_to_id
            .get(name)
            .cloned()
            .unwrap_or_else(|| name.to_string())
    };

    let gl_edges: Vec<GlEdgeSpec> = document
        .relations
        .iter()
        .enumerate()
        .map(|(i, rel)| GlEdgeSpec {
            id: format!("r{i}"),
            from: resolve_gl(&rel.from),
            to: resolve_gl(&rel.to),
        })
        .collect();

    let gl_options = GlOptions {
        rank_separation: (row_gap + node_heights.iter().map(|(_, h)| *h).max().unwrap_or(60))
            as f64,
        node_separation: col_gap as f64,
        group_padding: 16.0,
        direction: crate::render::graph_layout::Direction::TopDown,
        canvas_margin: (margin_top + title_block_height + group_top_reserve) as f64,
        // Right-side gutter only needs margin_x (32px); canvas_margin also absorbs
        // title height and group-label tabs which are only needed vertically.
        canvas_right_margin: Some(margin_x as f64),
    };

    let gl_result = layout_hierarchical(&gl_nodes, &gl_edges, &gl_options);

    // Populate node_boxes: use layout positions when available, else grid fallback.
    let mut node_boxes: std::collections::BTreeMap<String, ClassNodeBox> =
        std::collections::BTreeMap::new();
    let total_nodes = document.nodes.len() as i32;
    let row_count = if total_nodes == 0 {
        0
    } else {
        (total_nodes + col_count - 1) / col_count
    };
    let max_row_height = node_heights.iter().map(|(_, h)| *h).max().unwrap_or(60);
    let mut fallback_row_y_offsets: Vec<i32> = Vec::new();
    {
        let mut y = margin_top + title_block_height + group_top_reserve;
        for _ in 0..row_count {
            fallback_row_y_offsets.push(y);
            y += max_row_height + row_gap;
        }
    }

    for (idx, (node, (_key, h))) in document.nodes.iter().zip(node_heights.iter()).enumerate() {
        let key = node.alias.clone().unwrap_or_else(|| node.name.clone());
        let (nx, ny) = if let Some(&(lx, ly)) = gl_result.node_positions.get(&key) {
            (lx as i32, ly as i32)
        } else {
            let col = (idx as i32) % col_count;
            let row = (idx as i32) / col_count;
            let fx = margin_x + col * (node_width + col_gap);
            let fy = fallback_row_y_offsets
                .get(row as usize)
                .copied()
                .unwrap_or(margin_top + title_block_height);
            (fx, fy)
        };
        let nb = ClassNodeBox {
            x: nx,
            y: ny,
            w: node_width,
            h: *h,
            header_h: header_height,
        };
        node_boxes.insert(key.clone(), nb);
        if node.alias.is_some() {
            node_boxes.entry(node.name.clone()).or_insert(nb);
        }
        // Register by unscoped name for relations that reference "Browse" not
        // "Online Store::Browse" (fixes rectangle group scoping in usecase diagrams).
        if key.contains("::") {
            if let Some(unscoped) = key.rsplit("::").next() {
                node_boxes.entry(unscoped.to_string()).or_insert(nb);
            }
        }
    }

    (gl_result, node_boxes)
}

/// Output of `class_compute_canvas` — the canvas dimensions and node extents
/// needed to build the SVG header and position projections/labels.
pub(super) struct ClassCanvasMetrics {
    pub(super) svg_width: i32,
    pub(super) svg_height: i32,
    pub(super) nodes_bottom: i32,
}

/// Compute SVG canvas size and related metrics for `render_class_svg`.
///
/// Derives the canvas width/height from the bounding boxes of laid-out nodes,
/// group frames, and the layout engine floor values.  Also computes the total
/// projection extra height so the SVG is tall enough to include them.
#[allow(clippy::too_many_arguments)] // 10 distinct canvas metrics; a struct would add churn without clarity
pub(super) fn class_compute_canvas(
    node_boxes: &std::collections::BTreeMap<String, ClassNodeBox>,
    group_frames: &[RenderGroupFrame],
    max_group_depth: usize,
    json_projections: &[crate::model::JsonProjection],
    relations: &[crate::model::FamilyRelation],
    gl_canvas_width: f64,
    gl_canvas_height: f64,
    margin_x: i32,
    margin_top: i32,
    title_block_height: i32,
) -> ClassCanvasMetrics {
    let nodes_right = node_boxes
        .values()
        .map(|b| b.x + b.w)
        .max()
        .unwrap_or(margin_x);
    let nodes_bottom = node_boxes
        .values()
        .map(|b| b.y + b.h)
        .max()
        .unwrap_or(margin_top + title_block_height);

    let mut groups_right = margin_x;
    let mut groups_bottom = margin_top + title_block_height;
    for group in group_frames {
        let mut gx_min = i32::MAX;
        let mut gy_min = i32::MAX;
        let mut gx_max = i32::MIN;
        let mut gy_max = i32::MIN;
        let mut found_any = false;
        for member_id in &group.member_ids {
            if let Some(bx) = node_boxes.get(member_id.as_str()) {
                gx_min = gx_min.min(bx.x);
                gy_min = gy_min.min(bx.y);
                gx_max = gx_max.max(bx.x + bx.w);
                gy_max = gy_max.max(bx.y + bx.h);
                found_any = true;
            }
        }
        if !found_any {
            continue;
        }
        let depth_outset = (max_group_depth.saturating_sub(group.depth) as i32) * 18;
        let pad = 16 + depth_outset;
        let label_header = 40 + depth_outset;
        let fx = gx_min - pad;
        let fy = gy_min - pad - label_header;
        let fw = (gx_max - gx_min) + pad * 2;
        let fh = (gy_max - gy_min) + pad * 2 + label_header;
        groups_right = groups_right.max(fx + fw);
        groups_bottom = groups_bottom.max(fy + fh);
    }

    let proj_extra_height: i32 = json_projections.iter().fold(0, |acc, proj| {
        let kv_count = extract_projection_tree_rows(&proj.body, &proj.format).len() as i32;
        acc + 22 + kv_count * 16 + 8 + 12
    });

    let gl_canvas_right = gl_canvas_width as i32;
    let gl_canvas_bottom = gl_canvas_height as i32;
    let max_label_half_w = relations
        .iter()
        .map(|rel| {
            rel.label
                .as_ref()
                .map(|l| ((l.chars().count() as i32) * 6 / 2).max(18))
                .unwrap_or(0)
        })
        .max()
        .unwrap_or(0);
    // label_right_pad: the canvas right edge must be far enough that when a
    // relation label is side-cleared to `nodes_right + 8 + label_half_w` it
    // still fits within the clamping range `[..., svg_width - margin_x - 8 -
    // label_half_w]`.  Solving: svg_width >= nodes_right + 16 + 2*label_half_w
    // + margin_x, so pad = 16 + 2*max_label_half_w + margin_x.
    let label_right_pad = 16 + 2 * max_label_half_w + margin_x;
    // Drop the old 3-column grid minimum (col_count*node_width) — it inflated
    // the canvas to 700+ px even for 2-node diagrams.
    let svg_width = gl_canvas_right
        .max(nodes_right + label_right_pad)
        .max(groups_right + margin_x);
    let svg_height =
        (nodes_bottom.max(groups_bottom) + 40 + proj_extra_height).max(gl_canvas_bottom + 40);

    ClassCanvasMetrics {
        svg_width,
        svg_height,
        nodes_bottom,
    }
}
