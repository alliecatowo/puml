use super::*;

/// Backwards-compatible alias; delegates to the real timeline renderer.
pub fn render_component_svg(doc: &FamilyDocument) -> String {
    render_box_grid_svg(doc, "component")
}

pub fn render_deployment_svg(doc: &FamilyDocument) -> String {
    render_box_grid_svg(doc, "deployment")
}

pub(super) fn render_box_grid_svg(doc: &FamilyDocument, family: &str) -> String {
    // Do NOT emit a visible "component/deployment diagram" label — it was leaking
    // as unwanted canvas text (fix #490, #494).

    // Extract component style (use defaults if not present)
    let comp_style = match &doc.family_style {
        Some(FamilyStyle::Component(s)) => s.clone(),
        _ => ComponentStyle::default(),
    };

    // ─────────────────────────────────────────────────────────────────────────
    // Layout constants
    // ─────────────────────────────────────────────────────────────────────────
    let cell_w = 200i32; // component box width
    let cell_h = 80i32; // component box height
    let inner_cols = 3i32; // columns inside a package
    let inner_gap = 40i32; // gap between nodes inside a package
    let pkg_pad = 24i32; // padding inside package frame
    let pkg_tab = 40i32; // height of the package label tab at top (was 28; bumped to clear first-child node)
    let canvas_margin = 40i32;
    let pkg_gap = 32i32; // gap between packages on the canvas
                         // outer_cols was used by the old 2-column grid layout; now superseded by hierarchical layout.
    let _outer_cols = 2i32;

    // ─────────────────────────────────────────────────────────────────────────
    // Phase 1: Build group membership maps
    // ─────────────────────────────────────────────────────────────────────────
    // Build a map: node_id -> first group label/scope that contains it
    let mut node_to_group: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();

    // Collect groups from FamilyDocument directly (not via collect_render_group_frames
    // which deduplicates; we want direct per-group ordering).
    // Use doc.groups as the authoritative list; filter to depth-0 package groups.
    let pkg_groups: Vec<&crate::model::FamilyGroup> = doc.groups.iter().collect();

    for (g_idx, group) in pkg_groups.iter().enumerate() {
        for member_id in &group.member_ids {
            node_to_group.entry(member_id.clone()).or_insert(g_idx);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Phase 1b–1d: Hierarchical graph layout (Stage 2, #592)
    //
    // Build NodeSize + EdgeSpec lists, run layout_hierarchical, then extract
    // the resulting positions back into the pkg_layouts / positions structures
    // that the rendering code below expects.
    // ─────────────────────────────────────────────────────────────────────────
    use crate::render::graph_layout::{
        layout_hierarchical, EdgeSpec as GlEdgeSpec, LayoutOptions as GlOptions,
        NodeSize as GlNodeSize,
    };

    let title_lines = doc
        .title
        .as_deref()
        .map(|v| v.lines().count() as i32)
        .unwrap_or(0);
    let header_h = if title_lines > 0 {
        16 + title_lines * 22
    } else {
        0
    };

    // Build the group-id lookup: group scope string → group index
    // We use the first group's scope key as parent id for layout.
    // group_scope_by_idx[g_idx] → scope string used as parent id.
    let group_scope_by_idx: Vec<String> = {
        let mut scopes: Vec<String> = Vec::new();
        let mut seen: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
        for (g_idx, group) in pkg_groups.iter().enumerate() {
            if seen.contains(&g_idx) {
                continue;
            }
            seen.insert(g_idx);
            let raw_label = group.label.clone().unwrap_or_default();
            let scope = if raw_label.is_empty() {
                group.kind.clone()
            } else {
                raw_label.clone()
            };
            // Ensure unique scope strings (append index if needed)
            let unique_scope = if scopes.contains(&scope) {
                format!("{scope}_{g_idx}")
            } else {
                scope
            };
            scopes.push(unique_scope);
        }
        scopes
    };
    // Map: group_idx → scope (for node parent assignment)
    let group_scope_map: std::collections::BTreeMap<usize, &str> = group_scope_by_idx
        .iter()
        .enumerate()
        .map(|(i, s)| (i, s.as_str()))
        .collect();

    // Build NodeSize list
    let gl_nodes: Vec<GlNodeSize> = doc
        .nodes
        .iter()
        .map(|n| {
            let key = n.alias.clone().unwrap_or_else(|| n.name.clone());
            let parent = node_to_group
                .get(&key)
                .or_else(|| node_to_group.get(&n.name))
                .and_then(|g_idx| group_scope_map.get(g_idx))
                .map(|s| s.to_string());
            GlNodeSize {
                id: key,
                width: cell_w as f64,
                height: cell_h as f64,
                parent,
            }
        })
        .collect();

    // Build EdgeSpec list from doc.relations
    let gl_edges: Vec<GlEdgeSpec> = doc
        .relations
        .iter()
        .enumerate()
        .map(|(i, rel)| GlEdgeSpec {
            id: format!("r{i}"),
            from: rel.from.clone(),
            to: rel.to.clone(),
        })
        .collect();

    // Layout options derived from the existing constants.
    // Add (pkg_pad + pkg_tab) to canvas_margin so that the group label tab
    // above the top-rank nodes stays on canvas (the group bounds computation
    // subtracts group_padding + label_reserve above the minimum node y).
    let group_top_overhead = (pkg_pad + pkg_tab) as f64; // 24 + 40 = 64px (pkg_tab bumped)
                                                         // rank_separation: gap between the bottom of nodes in rank R and the top of
                                                         // nodes in rank R+1.  Package frames extend pkg_pad below/above their nodes
                                                         // and add label_reserve (40px) above the first node.  To keep visible
                                                         // whitespace between consecutive package frames we need:
                                                         //   rank_separation > 2 * pkg_pad + label_reserve  (= 88px)
                                                         // Use cell_h + inner_gap + 2*pkg_pad + label_reserve as a reasonable default
                                                         // so that adjacent package frames have at least inner_gap (40px) of breathing
                                                         // room between them.
    let rank_sep = (cell_h + inner_gap) as f64 + 2.0 * pkg_pad as f64 + 40.0; // ~188px
                                                                              // node_separation: horizontal gap between NODES in the same rank.  Package
                                                                              // frames extend pkg_pad (24px) on each side, so a node_separation of 2*pkg_pad
                                                                              // gives frames that just touch.  Add a visible inter-frame gutter on top.
    let node_sep = 2 * pkg_pad + inner_gap; // 48 + 40 = 88px → ~40px gap between frames
    let gl_options = GlOptions {
        rank_separation: rank_sep,
        node_separation: node_sep as f64,
        group_padding: pkg_pad as f64,
        direction: crate::render::graph_layout::Direction::TopDown,
        canvas_margin: canvas_margin as f64 + header_h as f64 + group_top_overhead,
        // canvas_margin absorbs title + package-label tab height for vertical
        // positioning; the right-side gutter only needs canvas_margin (40px).
        canvas_right_margin: Some(canvas_margin as f64),
    };

    // Run hierarchical layout
    let gl_result = layout_hierarchical(&gl_nodes, &gl_edges, &gl_options);

    // Convert f64 positions to i32 for the rest of the renderer
    let mut positions: std::collections::BTreeMap<String, (i32, i32, i32, i32)> =
        std::collections::BTreeMap::new();
    for (id, &(x, y)) in &gl_result.node_positions {
        positions.insert(id.clone(), (x as i32, y as i32, cell_w, cell_h));
    }

    // Also register name→position for nodes with aliases
    for node in &doc.nodes {
        if let Some(alias) = &node.alias {
            if let Some(&pos) = positions.get(alias.as_str()) {
                positions.entry(node.name.clone()).or_insert(pos);
            }
        }
    }
    let mut interface_nodes: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for node in &doc.nodes {
        if matches!(node.kind, FamilyNodeKind::Interface) {
            interface_nodes.insert(node.name.clone());
            if let Some(alias) = &node.alias {
                interface_nodes.insert(alias.clone());
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Phase 1b (compat): Build PackageLayout list from group_bounds
    //
    // The rendering code below (Phase 1e) iterates pkg_layouts to draw package
    // frames. We populate it from the hierarchical layout's group_bounds.
    // ─────────────────────────────────────────────────────────────────────────
    struct PackageLayout {
        #[allow(dead_code)]
        group_idx: usize,
        label: String,
        scope: String,
        #[allow(dead_code)]
        kind: String,
        node_ids: Vec<String>,
        // absolute canvas position of the package frame top-left
        abs_x: i32,
        abs_y: i32,
        // frame total size (including label tab)
        frame_w: i32,
        frame_h: i32,
    }

    let mut pkg_layouts: Vec<PackageLayout> = Vec::new();
    let mut seen_groups2: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();

    for (g_idx, group) in pkg_groups.iter().enumerate() {
        if seen_groups2.contains(&g_idx) {
            continue;
        }
        seen_groups2.insert(g_idx);

        // Get this group's scope string (pkg_layouts.len() == index before push)
        let scope_idx = pkg_layouts.len();
        let scope = group_scope_by_idx
            .get(scope_idx)
            .cloned()
            .unwrap_or_else(|| group.kind.clone());

        // Collect node IDs for this group (for package frame member-id list)
        let mut node_ids_in_group: Vec<String> = Vec::new();
        for node in &doc.nodes {
            let key = node.alias.clone().unwrap_or_else(|| node.name.clone());
            if (node_to_group.get(&key) == Some(&g_idx)
                || node_to_group.get(&node.name) == Some(&g_idx))
                && !node_ids_in_group.contains(&key)
            {
                node_ids_in_group.push(key);
            }
        }
        if node_ids_in_group.is_empty() {
            for mid in &group.member_ids {
                if !node_ids_in_group.contains(mid) {
                    node_ids_in_group.push(mid.clone());
                }
            }
        }

        // Get frame bounds from hierarchical layout result, or fall back
        let (fx, fy, fw, fh) = gl_result
            .group_bounds
            .get(&scope)
            .copied()
            .map(|(x, y, w, h)| (x as i32, y as i32, w as i32, h as i32))
            .unwrap_or_else(|| {
                // Fallback: bounding box of member nodes + padding
                let mut min_x = i32::MAX;
                let mut min_y = i32::MAX;
                let mut max_x = i32::MIN;
                let mut max_y = i32::MIN;
                let mut found = false;
                for nid in &node_ids_in_group {
                    if let Some(&(nx, ny, nw, nh)) = positions.get(nid.as_str()) {
                        min_x = min_x.min(nx);
                        min_y = min_y.min(ny);
                        max_x = max_x.max(nx + nw);
                        max_y = max_y.max(ny + nh);
                        found = true;
                    }
                }
                if found {
                    let pad = pkg_pad;
                    (
                        min_x - pad,
                        min_y - pad - pkg_tab,
                        (max_x - min_x) + pad * 2,
                        (max_y - min_y) + pad * 2 + pkg_tab,
                    )
                } else {
                    (
                        canvas_margin,
                        canvas_margin + header_h,
                        200,
                        80 + pkg_tab + pkg_pad * 2,
                    )
                }
            });

        let raw_label = group.label.clone().unwrap_or_default();
        let label = if raw_label.is_empty() {
            group.kind.clone()
        } else if group.kind == "rectangle" {
            raw_label.clone()
        } else {
            format!("{} {}", group.kind, raw_label)
        };

        pkg_layouts.push(PackageLayout {
            group_idx: g_idx,
            label,
            scope,
            kind: group.kind.clone(),
            node_ids: node_ids_in_group,
            abs_x: fx,
            abs_y: fy,
            frame_w: fw,
            frame_h: fh,
        });
    }

    // derive pkg_frame_widths/heights for compat
    let pkg_frame_widths: Vec<i32> = pkg_layouts.iter().map(|p| p.frame_w).collect();
    let pkg_frame_heights: Vec<i32> = pkg_layouts.iter().map(|p| p.frame_h).collect();

    // Ungrouped nodes (not placed by layout — place them below the canvas)
    let ungrouped: Vec<&crate::model::FamilyNode> = doc
        .nodes
        .iter()
        .filter(|n| {
            let key = n.alias.clone().unwrap_or_else(|| n.name.clone());
            !positions.contains_key(&key) && !positions.contains_key(&n.name)
        })
        .collect();

    // Find a safe Y below everything placed
    let pkg_bottom = pkg_layouts
        .iter()
        .enumerate()
        .map(|(i, p)| p.abs_y + pkg_frame_heights[i])
        .max()
        .unwrap_or(canvas_margin + header_h)
        + pkg_gap;

    for (idx, node) in ungrouped.iter().enumerate() {
        let col = (idx as i32) % inner_cols;
        let row = (idx as i32) / inner_cols;
        let x = canvas_margin + col * (cell_w + inner_gap);
        let y = pkg_bottom + row * (cell_h + inner_gap);
        let key = node.alias.clone().unwrap_or_else(|| node.name.clone());
        positions.insert(key, (x, y, cell_w, cell_h));
        if let Some(alias) = &node.alias {
            positions.insert(alias.clone(), (x, y, cell_w, cell_h));
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Compute SVG canvas size from hierarchical layout result
    // ─────────────────────────────────────────────────────────────────────────

    // 3D cube offset: Node and Frame kinds render a back-right face that extends
    // `cube_offset` pixels to the right (and up) of the layout bounding box.
    // We must add this to all right-edge estimates so the cube clears the canvas
    // right margin (fix #565 #569).
    const CUBE_OFFSET: i32 = 12;
    let has_3d_node = doc
        .nodes
        .iter()
        .any(|n| matches!(n.kind, FamilyNodeKind::Node | FamilyNodeKind::Frame));
    let shape_right_extra = if has_3d_node { CUBE_OFFSET } else { 0 };

    let all_pkg_right = pkg_layouts
        .iter()
        .enumerate()
        .map(|(i, pkg)| pkg.abs_x + pkg_frame_widths[i])
        .max()
        .unwrap_or(canvas_margin);
    let all_pkg_bottom = pkg_layouts
        .iter()
        .enumerate()
        .map(|(i, pkg)| pkg.abs_y + pkg_frame_heights[i])
        .max()
        .unwrap_or(canvas_margin + header_h);

    // Rightmost drawn position across all placed nodes, including any 3D cube
    // back-face extension.  This is the source-of-truth for the right canvas edge
    // when the graph-layout estimate (gl_canvas_right) falls short.
    let max_node_drawn_right = positions
        .values()
        .map(|&(nx, _, nw, _)| nx + nw + shape_right_extra)
        .max()
        .unwrap_or(canvas_margin);

    // Ungrouped nodes are placed by a fallback grid that is independent of the
    // graph-layout pass; their rightmost column must also contribute to svg_width.
    let ungrouped_right = if ungrouped.is_empty() {
        0
    } else {
        // The last occupied column index among all ungrouped rows.
        let last_col = ((ungrouped.len() as i32) - 1) % inner_cols;
        canvas_margin + last_col * (cell_w + inner_gap) + cell_w + shape_right_extra
    };
    let ungrouped_bottom = if ungrouped.is_empty() {
        0
    } else {
        let ungrouped_rows = (ungrouped.len() as i32 + inner_cols - 1) / inner_cols;
        pkg_bottom + ungrouped_rows * (cell_h + inner_gap)
    };

    // Also use gl_result canvas size as a floor
    let gl_canvas_right = gl_result.canvas_width as i32;
    let gl_canvas_bottom = gl_result.canvas_height as i32;

    let projection_extra_height = family_projection_extra_height(&doc.json_projections);
    let relation_label_half_width = doc
        .relations
        .iter()
        .filter_map(|rel| rel.label.as_ref())
        .map(|label| ((label.chars().count() as i32) * 7 + 12) / 2)
        .max()
        .unwrap_or(0);
    let right_gutter = if family == "deployment" {
        canvas_margin.max(12 + relation_label_half_width)
    } else {
        canvas_margin
    };
    // svg_width: the dominant right-edge estimate is max_node_drawn_right (which
    // already includes shape_right_extra); we also floor on gl_canvas_right and
    // all_pkg_right for backwards compatibility.  Deployment diagrams reserve an
    // extra right gutter for relation labels so rightmost nodes and labels do not
    // clip at the canvas boundary (#569).
    let svg_width = all_pkg_right
        .max(gl_canvas_right)
        .max(max_node_drawn_right)
        .max(ungrouped_right)
        .max(canvas_margin)
        + right_gutter;
    let svg_width = svg_width.max(400);
    let svg_height = all_pkg_bottom.max(ungrouped_bottom).max(gl_canvas_bottom)
        + canvas_margin
        + projection_extra_height;

    // ─────────────────────────────────────────────────────────────────────────
    // Start SVG output
    // ─────────────────────────────────────────────────────────────────────────
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        svg_width, svg_height, svg_width, svg_height
    ));
    out.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>",
        escape_text(&comp_style.background_color)
    ));
    render_relation_marker_defs(&mut out, &comp_style.arrow_color);

    // Title
    if let Some(title) = &doc.title {
        let mut ty = canvas_margin;
        for line in title.lines() {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\">{}</text>",
                canvas_margin, ty, escape_text(line)
            ));
            ty += 22;
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Phase 1e: Render package frames (BEFORE nodes, so nodes sit on top)
    // ─────────────────────────────────────────────────────────────────────────
    for (i, pkg) in pkg_layouts.iter().enumerate() {
        let fw = pkg_frame_widths[i];
        let fh = pkg_frame_heights[i];
        let fx = pkg.abs_x;
        let fy = pkg.abs_y;
        let container_attrs = crate::render::puml_container_attrs(
            &pkg.scope,
            family,
            "package",
            geometry_bbox(fx, fy, fw, fh),
        );

        // Draw the outer frame first (light fill, dark border, rounded corners)
        out.push_str(&format!(
            "<rect class=\"uml-group-frame puml-container\" data-uml-group=\"{}\" {} x=\"{fx}\" y=\"{fy}\" width=\"{fw}\" height=\"{fh}\" rx=\"8\" ry=\"8\" fill=\"#f8faff\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            escape_text(&pkg.scope),
            container_attrs,
            comp_style.border_color
        ));
        // Full-width header band inset into the frame top.  The band uses
        // rounded corners at the top-left and top-right (matching the outer
        // frame), then flat square corners at the bottom via a cover rect.
        // This makes the dark band look like an inset header, not a floating tab.
        out.push_str(&format!(
            "<rect x=\"{fx}\" y=\"{fy}\" width=\"{fw}\" height=\"{}\" rx=\"8\" ry=\"8\" fill=\"{}\" stroke=\"none\"/>",
            pkg_tab,
            comp_style.border_color,
        ));
        // Flatten only the bottom 8px of the header band (square the bottom corners)
        out.push_str(&format!(
            "<rect x=\"{fx}\" y=\"{}\" width=\"{fw}\" height=\"8\" fill=\"{}\" stroke=\"none\"/>",
            fy + pkg_tab - 8,
            comp_style.border_color
        ));
        // Package label text in the header band (left-aligned, vertically centred)
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" font-weight=\"600\" fill=\"#ffffff\">{}</text>",
            fx + 8,
            fy + pkg_tab - 8,
            escape_text(&pkg.label)
        ));
        // Horizontal separator line between header band and content area
        out.push_str(&format!(
            "<line x1=\"{fx}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            fy + pkg_tab,
            fx + fw,
            fy + pkg_tab,
            comp_style.border_color
        ));
    }

    // ── Nested sub-group frames (from collect_render_group_frames, depth > 0) ──
    // These handle nested packages like `node Rack { ... }` inside `package Edge { ... }`.
    // We draw them after top-level packages (so they appear inside), before nodes.
    {
        let all_group_frames = collect_render_group_frames(&doc.groups);
        let max_group_depth = all_group_frames.iter().map(|f| f.depth).max().unwrap_or(0);
        for frame in &all_group_frames {
            if frame.depth == 0 {
                // Top-level frames are already drawn above
                continue;
            }
            // Compute bounding box of all member nodes in this sub-frame
            let mut gx_min = i32::MAX;
            let mut gy_min = i32::MAX;
            let mut gx_max = i32::MIN;
            let mut gy_max = i32::MIN;
            let mut found_any = false;
            for mid in &frame.member_ids {
                // Try direct lookup, or strip namespace prefix
                let lookup_key = mid.rsplit("::").next().unwrap_or(mid.as_str()).to_string();
                let found = positions
                    .get(mid.as_str())
                    .or_else(|| positions.get(lookup_key.as_str()));
                if let Some(&(bx, by, bw, bh)) = found {
                    gx_min = gx_min.min(bx);
                    gy_min = gy_min.min(by);
                    gx_max = gx_max.max(bx + bw);
                    gy_max = gy_max.max(by + bh);
                    found_any = true;
                }
            }
            if !found_any {
                continue;
            }
            let depth_outset = (max_group_depth.saturating_sub(frame.depth) as i32) * 8;
            let pad = 10 + depth_outset;
            let label_h = 20 + depth_outset;
            let fx = gx_min - pad;
            let fy = gy_min - pad - label_h;
            let fw = gx_max - gx_min + pad * 2;
            let fh = gy_max - gy_min + pad * 2 + label_h;
            let sub_label = frame.display_label();
            let container_attrs = crate::render::puml_container_attrs(
                &frame.scope,
                family,
                &frame.kind,
                geometry_bbox(fx, fy, fw, fh),
            );
            out.push_str(&format!(
                "<rect class=\"uml-group-frame puml-container\" data-uml-group=\"{}\" {} x=\"{fx}\" y=\"{fy}\" width=\"{fw}\" height=\"{fh}\" rx=\"6\" ry=\"6\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\" stroke-dasharray=\"4 3\"/>",
                escape_text(&frame.scope),
                container_attrs,
                comp_style.border_color
            ));
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" font-weight=\"600\" fill=\"{}\">{}</text>",
                fx + 6,
                fy + 13,
                comp_style.border_color,
                escape_text(&sub_label)
            ));
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Phase 1f: Render nodes
    // ─────────────────────────────────────────────────────────────────────────
    for node in &doc.nodes {
        let key = node.alias.clone().unwrap_or_else(|| node.name.clone());
        let Some(&(nx, ny, nw, nh)) = positions.get(&key) else {
            continue;
        };
        out.push_str(&semantic_node_rect(
            &key,
            family,
            family_node_label(node.kind),
            nx,
            ny,
            nw,
            nh,
        ));
        render_family_node_shape_styled(&mut out, node, nx, ny, nw, nh, &comp_style);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Collect all obstacle boxes for collision detection.
    // `all_boxes` holds individual node boxes (used for all arrows).
    // `pkg_boxes` holds package frames with a list of member node IDs, so we
    // can exclude a package from blocking an arrow that starts or ends inside it.
    // ─────────────────────────────────────────────────────────────────────────
    let all_boxes: Vec<(i32, i32, i32, i32)> = positions.values().copied().collect();
    // Package frames: (rect, member_node_ids)
    type PkgFrameBox<'a> = ((i32, i32, i32, i32), &'a [String]);
    let pkg_frame_boxes: Vec<PkgFrameBox> = pkg_layouts
        .iter()
        .enumerate()
        .map(|(i, pkg)| {
            let fw = pkg_frame_widths[i];
            let fh = pkg_frame_heights[i];
            ((pkg.abs_x, pkg.abs_y, fw, fh), pkg.node_ids.as_slice())
        })
        .collect();

    // ─────────────────────────────────────────────────────────────────────────
    // Phase 2+3: Relation routing and label de-collision (extracted helper)
    // ─────────────────────────────────────────────────────────────────────────
    render_box_grid_relations_and_labels(
        &mut out,
        doc,
        family,
        &positions,
        &interface_nodes,
        &all_boxes,
        &pkg_frame_boxes,
        &gl_result.edge_paths,
        &comp_style,
    );
    // JSON projections
    if !doc.json_projections.is_empty() {
        let proj_y = all_pkg_bottom.max(ungrouped_bottom) + 16;
        render_family_projection_boxes(&mut out, &doc.json_projections, canvas_margin, proj_y, 340);
    }

    out.push_str("</svg>");
    out
}

/// Check if a line segment (x1,y1)→(x2,y2) intersects a rectangle (bx,by,bw,bh).
/// Uses the parametric Liang–Barsky / separating axis test.
pub(super) fn segment_intersects_rect(
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    (bx, by, bw, bh): (i32, i32, i32, i32),
) -> bool {
    // Grow the box by 4px to give a small margin
    let margin = 4;
    let rx0 = bx - margin;
    let ry0 = by - margin;
    let rx1 = bx + bw + margin;
    let ry1 = by + bh + margin;

    // Parametric: P(t) = (x1 + t*dx, y1 + t*dy)
    let dx = (x2 - x1) as f64;
    let dy = (y2 - y1) as f64;
    let mut tmin = 0.0f64;
    let mut tmax = 1.0f64;

    // Clip against left/right
    if dx.abs() < 1e-9 {
        // Vertical segment
        if (x1 < rx0) || (x1 > rx1) {
            return false;
        }
    } else {
        let t1 = (rx0 as f64 - x1 as f64) / dx;
        let t2 = (rx1 as f64 - x1 as f64) / dx;
        let (t_lo, t_hi) = if t1 < t2 { (t1, t2) } else { (t2, t1) };
        tmin = tmin.max(t_lo);
        tmax = tmax.min(t_hi);
        if tmin > tmax {
            return false;
        }
    }

    // Clip against top/bottom
    if dy.abs() < 1e-9 {
        if (y1 < ry0) || (y1 > ry1) {
            return false;
        }
    } else {
        let t1 = (ry0 as f64 - y1 as f64) / dy;
        let t2 = (ry1 as f64 - y1 as f64) / dy;
        let (t_lo, t_hi) = if t1 < t2 { (t1, t2) } else { (t2, t1) };
        tmin = tmin.max(t_lo);
        tmax = tmax.min(t_hi);
        if tmin > tmax {
            return false;
        }
    }

    // The intersection is within the segment if tmin ≤ tmax and tmin < 1 and tmax > 0
    tmin < tmax && tmax > 0.01 && tmin < 0.99
}

/// Count how many obstacles (excluding src/tgt) are intersected by a polyline.
pub(super) fn count_polyline_collisions(
    pts: &[(i32, i32)],
    all_boxes: &[(i32, i32, i32, i32)],
    src: (i32, i32, i32, i32),
    tgt: (i32, i32, i32, i32),
) -> usize {
    let mut count = 0;
    for seg in pts.windows(2) {
        let (ax, ay) = seg[0];
        let (bx_, by_) = seg[1];
        for &(obx, oby, obw, obh) in all_boxes {
            if (obx, oby, obw, obh) == src || (obx, oby, obw, obh) == tgt {
                continue;
            }
            if segment_intersects_rect(ax, ay, bx_, by_, (obx, oby, obw, obh)) {
                count += 1;
            }
        }
    }
    count
}
