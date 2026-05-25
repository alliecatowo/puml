use crate::model::{FamilyDocument, FamilyNodeKind, FamilyStyle};
use crate::render::relation::{
    has_ie_endpoint_marker, normalize_relation_endpoints, render_ie_marker_defs,
};
use crate::render::svg::escape_text;
use crate::render::RenderArtifact;
use crate::render_core::Rect;
use crate::theme::ClassStyle;

use super::c4_nodes::c4_node_height;
use super::class_layout::{
    class_compute_canvas, class_node_display_name, class_run_layout, is_real_usecase_layout,
};
use super::class_members::{count_header_stereotype_members, parse_map_row};
use super::class_node_render::render_class_node;
use super::class_relation_labels::{
    class_build_label_overrides, relation_pair_label_lane_map, relation_source_label_lane_map,
};
use super::class_relations::{render_class_relations, ClassRelationCtx};
use super::class_types::ClassNodeGeometry;
use super::group_frames::{
    collect_render_group_frames, render_class_group_frames, CLASS_GROUP_LABEL_GAP,
    CLASS_GROUP_TAB_HEIGHT,
};
use super::projections::render_family_projection_boxes;

pub fn render_family_stub_svg(document: &FamilyDocument) -> String {
    render_family_stub_artifact(document).svg
}

pub fn render_family_stub_artifact(document: &FamilyDocument) -> RenderArtifact {
    render_class_artifact(document)
}

pub fn render_class_svg(document: &FamilyDocument) -> String {
    render_class_artifact(document).svg
}

pub fn render_class_artifact(document: &FamilyDocument) -> RenderArtifact {
    // Extract class style (use defaults if not present)
    let class_style = match &document.family_style {
        Some(FamilyStyle::Class(s)) => s.clone(),
        _ => ClassStyle::default(),
    };

    // Layout constants
    let margin_x: i32 = 32;
    let margin_top: i32 = 32;
    let col_count: i32 = 3;
    let group_frames = collect_render_group_frames(&document.groups);
    let max_group_depth = group_frames
        .iter()
        .map(|frame| frame.depth)
        .max()
        .unwrap_or(0);
    // Auto-size node_width from longest member text / node name (fix #572).
    // char_width=7px (monospace), padding=24px (accounts for left+right insets).
    // Upper clamp raised to 600 so long member lines are never truncated.
    let node_width: i32 = {
        let name_px = document
            .nodes
            .iter()
            .map(|n| {
                class_node_display_name(n, document.namespace_separator.as_deref())
                    .chars()
                    .count() as i32
                    * 8
                    + 32
            })
            .max()
            .unwrap_or(200);
        let member_px = document
            .nodes
            .iter()
            .flat_map(|n| n.members.iter())
            .map(|m| m.text.chars().count() as i32 * 7 + 24)
            .max()
            .unwrap_or(0);
        name_px.max(member_px).clamp(160, 600)
    };
    // Reserve enough horizontal gap for the longest relation label so object
    // edge labels stay clear of adjacent box borders (#564, #484).
    let relation_label_gap = document
        .relations
        .iter()
        .map(|rel| {
            let label_w = rel
                .label
                .as_ref()
                .map(|label| (label.chars().count() as i32) * 7 + 24)
                .unwrap_or(0);
            let stereotype_w = rel
                .stereotype
                .as_ref()
                .map(|label| (label.chars().count() as i32) * 7 + 56)
                .unwrap_or(0);
            label_w.max(stereotype_w)
        })
        .max()
        .unwrap_or(0);
    let col_gap: i32 = 80.max(relation_label_gap);
    let is_usecase = is_real_usecase_layout(document);
    let row_gap: i32 = if is_usecase { 46 } else { 64 };
    let header_height: i32 = 30;
    let member_line_height: i32 = 16;
    let member_padding: i32 = 8;
    let empty_member_pad: i32 = 8;
    // Reserve enough top room for the deepest package frame header plus its
    // outer padding. The exact frame geometry is centralized in
    // `class_group_frame_rect`; this pre-layout value must cover the same stack.
    let group_top_reserve = if group_frames.is_empty() {
        0
    } else {
        ((max_group_depth as i32) + 1) * (CLASS_GROUP_TAB_HEIGHT + CLASS_GROUP_LABEL_GAP)
    };
    let relation_pair_label_lanes = relation_pair_label_lane_map(document);
    let relation_source_label_lanes = relation_source_label_lane_map(document);

    let title_block_height = document
        .title
        .as_deref()
        .map(|t| 12 + (t.lines().count() as i32) * 22)
        .unwrap_or(0);

    // Compute per-node heights (used both for layout and for node rendering).
    // Store as Vec<(key, h)> in declaration order.
    let node_heights: Vec<(String, i32)> = document
        .nodes
        .iter()
        .map(|node| {
            let key = node.alias.clone().unwrap_or_else(|| node.name.clone());
            let header_stereotype_count = count_header_stereotype_members(&node.members);
            let display_member_count = node.members.len().saturating_sub(header_stereotype_count);
            let stereotype_extra_h = (header_stereotype_count as i32) * 14;
            let body_h = if node.kind == FamilyNodeKind::Note {
                let lines = node
                    .label
                    .as_deref()
                    .unwrap_or(&node.name)
                    .lines()
                    .count()
                    .max(1) as i32;
                lines * 16 + 20
            } else if node.kind == FamilyNodeKind::Map {
                let rows = node
                    .members
                    .iter()
                    .filter(|member| parse_map_row(&member.text).is_some())
                    .count() as i32;
                if rows == 0 {
                    empty_member_pad
                } else {
                    rows * 18
                }
            } else if display_member_count == 0 {
                empty_member_pad
            } else {
                (display_member_count as i32) * member_line_height + 2 * member_padding
            };
            let h = c4_node_height(node.kind, header_height + stereotype_extra_h + body_h);
            (key, h)
        })
        .collect();

    // ── Hierarchical layout + node_boxes population ─────────────────────────────
    let (gl_result_class, node_boxes) = class_run_layout(
        document,
        &node_heights,
        node_width,
        col_count,
        col_gap,
        row_gap,
        margin_x,
        margin_top,
        title_block_height,
        group_top_reserve,
        header_height,
    );
    // ── Canvas dimensions from layout result ──────────────────────────────────
    let canvas = class_compute_canvas(
        &node_boxes,
        &group_frames,
        max_group_depth,
        &document.json_projections,
        &document.relations,
        gl_result_class.canvas_width,
        gl_result_class.canvas_height,
        margin_x,
        margin_top,
        title_block_height,
    );
    let svg_width = canvas.svg_width;
    let svg_height = canvas.svg_height;
    let nodes_bottom = canvas.nodes_bottom;
    // gl_canvas_right / gl_canvas_bottom consumed by class_compute_canvas
    let mut out = String::new();
    let sepia_attr = if document.style.sepia {
        " style=\"filter:sepia(1)\""
    } else {
        ""
    };
    let orientation_attr = format!(" data-orientation=\"{}\"", document.orientation.as_str());
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\"{orientation}{sepia}>",
        w = svg_width,
        h = svg_height,
        orientation = orientation_attr,
        sepia = sepia_attr,
    ));
    out.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>",
        escape_text(&class_style.background_color)
    ));

    // Arrowhead/diamond marker defs — use class_style.arrow_color for stroke
    let arrow_stroke = &class_style.arrow_color;
    // markerUnits="userSpaceOnUse" pins marker sizes in SVG user units so they
    // are NOT scaled by the parent element's stroke-width (fix #471 collision).
    // fill="#ffffff" instead of fill="white" avoids resvg keyword-inheritance
    // rendering the triangle filled in PNG output (fix #467).
    out.push_str("<defs>");
    out.push_str(&format!(
        "<marker id=\"arrow-open\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" \
         markerWidth=\"10\" markerHeight=\"10\" markerUnits=\"userSpaceOnUse\" orient=\"auto-start-reverse\">\
         <path d=\"M0,0 L10,5 L0,10\" fill=\"none\" stroke=\"{arrow_stroke}\" stroke-width=\"1.5\"/>\
         </marker>",
    ));
    // Larger 16×16 marker for a clearly visible hollow triangle (fix #467).
    // refX=15 places the triangle tip exactly at the line endpoint; the white fill
    // covers the line shaft so only the triangle border is visible.
    out.push_str(&format!(
        "<marker id=\"arrow-triangle\" viewBox=\"0 0 16 16\" refX=\"15\" refY=\"8\" \
         markerWidth=\"16\" markerHeight=\"16\" markerUnits=\"userSpaceOnUse\" orient=\"auto-start-reverse\">\
         <polygon points=\"0,1 14,8 0,15\" fill=\"#ffffff\" stroke=\"{arrow_stroke}\" stroke-width=\"1.5\" fill-rule=\"nonzero\"/>\
         </marker>",
    ));
    out.push_str(&format!(
        "<marker id=\"arrow-diamond-filled\" viewBox=\"0 0 14 10\" refX=\"13\" refY=\"5\" \
         markerWidth=\"14\" markerHeight=\"10\" markerUnits=\"userSpaceOnUse\" orient=\"auto-start-reverse\">\
         <path d=\"M0,5 L7,0 L14,5 L7,10 z\" fill=\"{arrow_stroke}\" stroke=\"{arrow_stroke}\" stroke-width=\"1\"/>\
         </marker>",
    ));
    out.push_str(&format!(
        "<marker id=\"arrow-diamond-open\" viewBox=\"0 0 14 10\" refX=\"13\" refY=\"5\" \
         markerWidth=\"14\" markerHeight=\"10\" markerUnits=\"userSpaceOnUse\" orient=\"auto-start-reverse\">\
         <path d=\"M0,5 L7,0 L14,5 L7,10 z\" fill=\"#ffffff\" stroke=\"{arrow_stroke}\" stroke-width=\"1\"/>\
         </marker>",
    ));
    if document.relations.iter().any(|relation| {
        let (_, _, normalized_arrow) =
            normalize_relation_endpoints(&relation.from, &relation.to, &relation.arrow);
        has_ie_endpoint_marker(&normalized_arrow)
    }) {
        render_ie_marker_defs(&mut out, arrow_stroke);
    }
    out.push_str("</defs>");

    // Title
    if let Some(title) = &document.title {
        let mut ty = margin_top;
        for line in title.lines() {
            out.push_str(&format!(
                "<text x=\"{x}\" y=\"{y}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\" fill=\"#0f172a\">{txt}</text>",
                x = margin_x,
                y = ty,
                txt = escape_text(line)
            ));
            ty += 22;
        }
    }

    // ── Label de-collision pre-pass ───────────────────────────────────────────
    // Delegate to helper: fans labels that cluster on the same target node or
    // share the same horizontal y-band so they don't overlap (#706, #749).
    let label_override = class_build_label_overrides(
        &document.relations,
        &node_boxes,
        &gl_result_class.edge_paths,
    );

    // Build lateral-offset map for parallel edges and render all relations.
    // Delegate to helper to keep this orchestrator function concise.
    const PARALLEL_EDGE_GAP: i32 = 12;
    let mut parallel_groups: std::collections::BTreeMap<(String, String), Vec<usize>> =
        std::collections::BTreeMap::new();
    for (i, rel) in document.relations.iter().enumerate() {
        let (fn_, tn_, _) = normalize_relation_endpoints(&rel.from, &rel.to, &rel.arrow);
        let key = if fn_ <= tn_ { (fn_, tn_) } else { (tn_, fn_) };
        parallel_groups.entry(key).or_default().push(i);
    }
    let mut parallel_offset: std::collections::BTreeMap<usize, i32> =
        std::collections::BTreeMap::new();
    for group in parallel_groups.values() {
        if group.len() < 2 {
            continue;
        }
        let n = group.len() as i32;
        for (slot, &idx) in group.iter().enumerate() {
            let lane = slot as i32 - n / 2;
            parallel_offset.insert(idx, lane * PARALLEL_EDGE_GAP);
        }
    }
    render_class_relations(
        &mut out,
        &ClassRelationCtx {
            relations: &document.relations,
            nodes: &document.nodes,
            node_boxes: &node_boxes,
            edge_paths: &gl_result_class.edge_paths,
            label_override: &label_override,
            parallel_offset: &parallel_offset,
            relation_pair_label_lanes: &relation_pair_label_lanes,
            relation_source_label_lanes: &relation_source_label_lanes,
            class_style: &class_style,
            arrow_stroke: arrow_stroke.as_str(),
            margin_x,
            margin_top,
            svg_width,
        },
    );

    // Render group frames (together/package/namespace) BEFORE nodes so node
    // rectangles visually sit on top of the frame borders.
    render_class_group_frames(&mut out, &group_frames, max_group_depth, &node_boxes);

    // Render nodes
    for node in &document.nodes {
        let key = node.alias.clone().unwrap_or_else(|| node.name.clone());
        let Some(bx) = node_boxes.get(&key) else {
            continue;
        };
        render_class_node(
            &mut out,
            node,
            ClassNodeGeometry {
                x: bx.x,
                y: bx.y,
                w: bx.w,
                h: bx.h,
                header_h: bx.header_h,
            },
            &class_style,
            document.namespace_separator.as_deref(),
            document.hide_options.contains("stereotype"),
        );
    }

    // Render inline JSON/YAML projections below the node area.
    if !document.json_projections.is_empty() {
        let proj_margin_left = margin_x;
        render_family_projection_boxes(
            &mut out,
            &document.json_projections,
            proj_margin_left,
            nodes_bottom + 16,
            300,
        );
    }

    out.push_str("</svg>");
    let mut scene = gl_result_class.scene.clone();
    scene.viewport = Rect::new(0.0, 0.0, svg_width as f64, svg_height as f64);
    RenderArtifact::with_scene(out, scene)
}
