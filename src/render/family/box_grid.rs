use crate::model::{FamilyDocument, FamilyNodeKind, FamilyStyle, MetadataHAlign};
use crate::render::layout_constants::{
    COMPONENT_BOX_HEIGHT, COMPONENT_BOX_WIDTH, COMPONENT_CANVAS_MARGIN, PKG_INNER_GAP, PKG_PADDING,
    PKG_TAB_HEIGHT,
};
use crate::render::relation::{render_ie_marker_defs, render_relation_marker_defs};
use crate::render::svg::escape_text;
use crate::render::RenderArtifact;
use crate::render_core::text_metrics::estimate_text_width_f64;
use crate::render_core::Rect;
use crate::theme::ComponentStyle;

use super::box_grid_edges::render_box_grid_relations_and_labels;
use super::box_grid_frames::{
    render_box_grid_package_frames, render_box_grid_package_header_text, BoxGridPackageFrameInputs,
};
use super::box_grid_ports::apply_boundary_port_positions;
use super::node_shapes::{render_family_node_shape_styled, DeploymentShapeBounds};
use super::projections::render_family_projection_boxes;

pub(super) struct PackageLayout {
    #[allow(dead_code)]
    pub(super) group_idx: usize,
    pub(super) label: String,
    pub(super) scope: String,
    #[allow(dead_code)]
    pub(super) kind: String,
    pub(super) node_ids: Vec<String>,
    pub(super) abs_x: i32,
    pub(super) abs_y: i32,
    pub(super) frame_w: i32,
    pub(super) frame_h: i32,
    /// Optional fill color for the frame background (e.g. from `frame "X" #LightYellow`).
    pub(super) fill_color: Option<String>,
}

/// Backwards-compatible alias; delegates to the real timeline renderer.
pub fn render_component_svg(doc: &FamilyDocument) -> String {
    render_component_artifact(doc).svg
}

pub fn render_component_artifact(doc: &FamilyDocument) -> RenderArtifact {
    render_box_grid_artifact(doc, "component")
}

pub fn render_deployment_svg(doc: &FamilyDocument) -> String {
    render_deployment_artifact(doc).svg
}

pub fn render_deployment_artifact(doc: &FamilyDocument) -> RenderArtifact {
    render_box_grid_artifact(doc, "deployment")
}

fn render_box_grid_artifact(doc: &FamilyDocument, family: &str) -> RenderArtifact {
    let comp_style = match &doc.family_style {
        Some(FamilyStyle::Component(s)) => s.clone(),
        _ => ComponentStyle::default(),
    };

    let cell_w = COMPONENT_BOX_WIDTH;
    let cell_h = COMPONENT_BOX_HEIGHT;
    let inner_cols = 3i32;
    let inner_gap = PKG_INNER_GAP;
    let pkg_pad = PKG_PADDING;
    let pkg_tab = PKG_TAB_HEIGHT;
    let canvas_margin = COMPONENT_CANVAS_MARGIN;
    let pkg_gap = 32i32;
    let _outer_cols = 2i32;

    let mut node_to_group: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();

    let pkg_groups: Vec<&crate::model::FamilyGroup> = doc.groups.iter().collect();

    for (g_idx, group) in pkg_groups.iter().enumerate() {
        for member_id in &group.member_ids {
            node_to_group.entry(member_id.clone()).or_insert(g_idx);
        }
    }

    use crate::render::graph_layout::{
        layout_hierarchical, EdgeSpec as GlEdgeSpec, LayoutOptions as GlOptions,
        NodeSize as GlNodeSize,
    };

    let title_lines = doc
        .title
        .as_deref()
        .map(|v| v.lines().count() as i32)
        .unwrap_or(0);
    // Extra space for `header` text above the title (if any).
    let header_block_h = super::class_metadata::family_metadata_label_height(doc.header.as_deref());
    let header_h = if title_lines > 0 {
        16 + title_lines * 22 + header_block_h
    } else {
        header_block_h
    };

    let group_scope_by_idx: Vec<String> = {
        let mut scopes: Vec<String> = Vec::new();
        for (g_idx, group) in pkg_groups.iter().enumerate() {
            let raw_label = group.label.clone().unwrap_or_default();
            let scope = if raw_label.is_empty() {
                group.kind.clone()
            } else {
                raw_label
            };
            // Ensure unique scope strings (append index if needed)
            scopes.push(if scopes.contains(&scope) {
                format!("{scope}_{g_idx}")
            } else {
                scope
            });
        }
        scopes
    };
    let group_scope_map: std::collections::BTreeMap<usize, &str> = group_scope_by_idx
        .iter()
        .enumerate()
        .map(|(i, s)| (i, s.as_str()))
        .collect();

    // Deepest parent scope from `A::B::C` — grows ancestor bboxes in compute_group_bounds (#1287).
    let deepest = |q: &str| -> Option<String> {
        let p: Vec<&str> = q.split("::").filter(|s| !s.is_empty()).collect();
        (p.len() >= 2).then(|| p[..p.len() - 1].join("::"))
    };
    let gl_nodes: Vec<GlNodeSize> = doc
        .nodes
        .iter()
        .map(|n| {
            let key = n.alias.clone().unwrap_or_else(|| n.name.clone());
            let group_scope = node_to_group
                .get(&key)
                .or_else(|| node_to_group.get(&n.name))
                .and_then(|g_idx| group_scope_map.get(g_idx))
                .map(|s| s.to_string());
            let parent = deepest(&n.name).or_else(|| deepest(&key)).or(group_scope);
            GlNodeSize {
                id: key,
                width: cell_w as f64,
                height: cell_h as f64,
                parent,
            }
        })
        .collect();

    let mut gl_name_to_id: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();
    for node in &doc.nodes {
        let key = node.alias.clone().unwrap_or_else(|| node.name.clone());
        let mut insert_alias = |name: String| {
            gl_name_to_id.entry(name).or_insert_with(|| key.clone());
        };
        insert_alias(key.clone());
        insert_alias(node.name.clone());
        if let Some(alias) = &node.alias {
            insert_alias(alias.clone());
        }
        if let Some(unscoped) = node.name.rsplit("::").next() {
            insert_alias(unscoped.to_string());
        }
        if let Some(unscoped) = key.rsplit("::").next() {
            insert_alias(unscoped.to_string());
        }
    }
    let resolve_gl_endpoint = |endpoint: &str| -> String {
        gl_name_to_id
            .get(endpoint)
            .cloned()
            .unwrap_or_else(|| endpoint.to_string())
    };

    let gl_edges: Vec<GlEdgeSpec> = doc
        .relations
        .iter()
        .enumerate()
        // Explicit arrow directions define edge geometry, not Sugiyama rank
        // constraints. Hidden relations are rendered for metadata/parity, but
        // should not override visible directional ordering in the box layout.
        .filter(|(_, rel)| rel.direction.is_none() && !rel.hidden)
        .map(|(i, rel)| GlEdgeSpec {
            id: format!("r{i}"),
            from: resolve_gl_endpoint(&rel.from),
            to: resolve_gl_endpoint(&rel.to),
            label: rel.label.clone(),
        })
        .collect();

    let group_top_overhead = (pkg_pad + pkg_tab) as f64;
    let rank_sep = (cell_h + inner_gap) as f64 + 2.0 * pkg_pad as f64 + 40.0;
    let node_sep = 2 * pkg_pad + inner_gap;
    let has_lollipop_endpoint = doc
        .relations
        .iter()
        .any(|rel| rel.left_lollipop || rel.right_lollipop);
    // Component lollipop fixtures may model interfaces as concrete circle nodes
    // instead of relation endpoint flags; they need the same package stacking.
    let interface_layout_ids: std::collections::BTreeSet<String> = doc
        .nodes
        .iter()
        .filter(|node| matches!(node.kind, FamilyNodeKind::Interface))
        .map(|node| node.alias.clone().unwrap_or_else(|| node.name.clone()))
        .collect();
    let has_interface_endpoint = doc.relations.iter().any(|rel| {
        let from = resolve_gl_endpoint(&rel.from);
        let to = resolve_gl_endpoint(&rel.to);
        interface_layout_ids.contains(&from) || interface_layout_ids.contains(&to)
    });
    let gl_options = GlOptions {
        rank_separation: rank_sep,
        node_separation: node_sep as f64,
        group_padding: pkg_pad as f64,
        direction: crate::render::graph_layout::Direction::TopDown,
        canvas_margin: canvas_margin as f64 + header_h as f64 + group_top_overhead,
        // canvas_margin absorbs title + package-label tab height for vertical
        // positioning; the right-side gutter only needs canvas_margin (40px).
        canvas_right_margin: Some(canvas_margin as f64),
        stack_staggered_group_collisions: family == "component"
            && (has_lollipop_endpoint || has_interface_endpoint),
        skip_group_collision_resolution: false,
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
        let key = node.alias.clone().unwrap_or_else(|| node.name.clone());
        let pos = positions
            .get(key.as_str())
            .or_else(|| positions.get(node.name.as_str()))
            .copied();
        if let Some(pos) = pos {
            if let Some(unscoped) = node.name.rsplit("::").next() {
                positions.entry(unscoped.to_string()).or_insert(pos);
            }
            if let Some(unscoped) = key.rsplit("::").next() {
                positions.entry(unscoped.to_string()).or_insert(pos);
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

        // Ensure the frame is at least wide enough to contain the header text so
        // it is never clipped. The header renders at font-size 11 (monospace) with
        // 8px left padding; add 16px total horizontal padding for breathing room.
        let min_label_w = (estimate_text_width_f64(&label, 11.0) + 24.0).ceil() as i32;
        let fw = fw.max(min_label_w);

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
            fill_color: group.fill_color.clone(),
        });
    }

    // derive pkg_frame_widths/heights for compat
    let pkg_frame_widths: Vec<i32> = pkg_layouts.iter().map(|p| p.frame_w).collect();
    let pkg_frame_heights: Vec<i32> = pkg_layouts.iter().map(|p| p.frame_h).collect();

    apply_boundary_port_positions(
        doc,
        &mut positions,
        &pkg_layouts,
        &pkg_frame_widths,
        &pkg_frame_heights,
        pkg_tab,
    );

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

    let canvas =
        super::box_grid_canvas::compute_canvas_bounds(super::box_grid_canvas::CanvasBoundsInput {
            doc,
            family,
            pkg_layouts: &pkg_layouts,
            pkg_frame_widths: &pkg_frame_widths,
            pkg_frame_heights: &pkg_frame_heights,
            positions: &positions,
            ungrouped_len: ungrouped.len(),
            gl_canvas_width: gl_result.canvas_width,
            gl_canvas_height: gl_result.canvas_height,
            inner_cols,
            cell_w,
            cell_h,
            inner_gap,
            canvas_margin,
            pkg_bottom,
            header_h,
        });
    let super::box_grid_canvas::BoxGridCanvasBounds {
        svg_width,
        svg_height,
        all_pkg_bottom,
        gl_canvas_bottom,
        projection_extra_height,
        caption_block_h,
        ungrouped_bottom,
        ..
    } = canvas;

    // ─────────────────────────────────────────────────────────────────────────
    // Start SVG output
    // ─────────────────────────────────────────────────────────────────────────
    let mut out = String::new();
    let sepia_attr = if doc.style.sepia {
        " style=\"filter:sepia(1)\""
    } else {
        ""
    };
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\"{sepia_attr}>",
        w = svg_width, h = svg_height,
    ));
    out.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>",
        escape_text(&comp_style.background_color)
    ));
    render_relation_marker_defs(&mut out, &comp_style.arrow_color);
    render_ie_marker_defs(&mut out, &comp_style.arrow_color); // IE crow's-foot glyphs (#1293)
                                                              // `skinparam shadowing true` — drop-shadow filter referenced via
                                                              // filter="url(#shadow)" from component-node rects.
    if comp_style.shadowing {
        out.push_str("<defs><filter id=\"shadow\" x=\"-10%\" y=\"-10%\" width=\"130%\" height=\"130%\"><feDropShadow dx=\"3\" dy=\"3\" stdDeviation=\"2\" flood-color=\"#00000040\"/></filter></defs>");
    }

    // Header — rendered at the top before title and nodes.
    if let Some(header_text) = &doc.header {
        super::class_metadata::render_family_metadata_label(
            &mut out,
            header_text,
            "header",
            doc.header_align,
            16,
            svg_width,
            "fill=\"#333333\"",
        );
    }

    // Title
    if let Some(title) = &doc.title {
        // Title sits below the header block.
        let mut ty = canvas_margin;
        for line in title.lines() {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\">{}</text>",
                canvas_margin, ty, escape_text(line)
            ));
            ty += 22;
        }
    }

    render_box_grid_package_frames(
        &mut out,
        BoxGridPackageFrameInputs {
            doc,
            pkg_layouts: &pkg_layouts,
            pkg_frame_widths: &pkg_frame_widths,
            pkg_frame_heights: &pkg_frame_heights,
            pkg_tab,
            comp_style: &comp_style,
            positions: &positions,
        },
    );
    for node in &doc.nodes {
        let key = node.alias.clone().unwrap_or_else(|| node.name.clone());
        let Some(&(nx, ny, nw, nh)) = positions.get(&key) else {
            continue;
        };
        render_family_node_shape_styled(
            &mut out,
            node,
            DeploymentShapeBounds {
                x: nx,
                y: ny,
                w: nw,
                h: nh,
            },
            &comp_style,
            doc.hide_options.contains("stereotype"),
        );
    }

    // Collect obstacle boxes for relation collision detection.
    let all_boxes: Vec<(i32, i32, i32, i32)> = positions.values().copied().collect();
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
    // Re-paint header label text (text only, NOT the dark band) AFTER edge-label
    // backgrounds so the white label text stays readable even when a white edge-label
    // bg rect overlaps the header zone. The dark band itself is NOT repainted here
    // to avoid covering cross-package edge lines that pass through it (#1424/#1374).
    render_box_grid_package_header_text(
        &mut out,
        &pkg_layouts,
        &pkg_frame_widths,
        pkg_tab,
        &comp_style.border_color,
    );
    if !doc.json_projections.is_empty() {
        let proj_y = all_pkg_bottom.max(ungrouped_bottom) + 16;
        render_family_projection_boxes(&mut out, &doc.json_projections, canvas_margin, proj_y, 340);
    }

    if let Some(text) = &doc.legend {
        super::class_metadata::render_family_legend_box(
            &mut out,
            text,
            svg_width,
            svg_height,
            doc.legend_halign,
            doc.legend_valign,
        );
    }

    // Caption — rendered in italic below the diagram, before footer.
    let base_bottom = all_pkg_bottom.max(ungrouped_bottom).max(gl_canvas_bottom)
        + canvas_margin
        + projection_extra_height;
    let caption_y = base_bottom + 14;
    if let Some(caption_text) = &doc.caption {
        super::class_metadata::render_family_metadata_label(
            &mut out,
            caption_text,
            "caption",
            MetadataHAlign::Left,
            caption_y,
            svg_width,
            "fill=\"#555555\" font-style=\"italic\"",
        );
    }
    // Footer — rendered at the very bottom of the SVG.
    let footer_y = caption_y + caption_block_h + 14;
    if let Some(footer_text) = &doc.footer {
        super::class_metadata::render_family_metadata_label(
            &mut out,
            footer_text,
            "footer",
            doc.footer_align,
            footer_y,
            svg_width,
            "fill=\"#333333\"",
        );
    }

    out.push_str("</svg>");
    crate::output::append_optional_mainframe_svg(&mut out, doc.mainframe.as_deref());
    let mut scene = gl_result.scene.clone();
    scene.viewport = Rect::new(0.0, 0.0, svg_width as f64, svg_height as f64);
    RenderArtifact::with_scene(out, scene).with_common_command_parts(
        doc.scale.clone(),
        doc.mainframe.clone(),
        doc.mainframe.is_some(),
    )
}
