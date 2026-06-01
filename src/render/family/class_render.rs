use crate::model::{FamilyDocument, FamilyNodeKind, FamilyStyle, MetadataHAlign};
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
use super::class_metadata::{
    family_metadata_label_height, render_family_legend_box, render_family_metadata_label,
};
use super::class_node_render::render_class_node;
use super::class_relation_labels::{
    class_build_label_overrides, relation_pair_label_lane_map, relation_source_label_lane_map,
};
use super::class_relations::{render_class_relations, ClassRelationCtx};
use super::class_types::ClassNodeGeometry;
use super::group_frames::{
    class_group_frame_rect, collect_render_group_frames, render_class_group_frames,
    CLASS_GROUP_LABEL_GAP, CLASS_GROUP_TAB_HEIGHT,
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
    // Extra top margin for a `header` block above the title / nodes.
    let header_block_h = family_metadata_label_height(document.header.as_deref());
    let margin_top: i32 = 32 + header_block_h;
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
            .map(|m| crate::render::text_metrics::monospace_width(&m.text, 7) + 24)
            .max()
            .unwrap_or(0);
        let base_w = name_px.max(member_px).clamp(160, 600);
        // Retune for usecase density (#1359): ovals are much smaller than class
        // boxes; cap usecase node width at 120 to match PlantUML oval sizing.
        if is_real_usecase_layout(document) {
            base_w.min(120)
        } else {
            base_w
        }
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
                .map(|label| crate::render::text_metrics::monospace_width(label, 7) + 24)
                .unwrap_or(0);
            let stereotype_w = rel
                .stereotype
                .as_ref()
                .map(|label| crate::render::text_metrics::monospace_width(label, 7) + 56)
                .unwrap_or(0);
            label_w.max(stereotype_w)
        })
        .max()
        .unwrap_or(0);
    let is_usecase = is_real_usecase_layout(document);
    // Retune for PlantUML-equivalent density (#1359): usecase diagrams use
    // much tighter spacing than class/object.  col_gap reduced 80→30 and
    // row_gap reduced 46→20 to match PlantUML's ~30px node separation.
    // Relation labels in usecase diagrams don't drive horizontal node spacing.
    let col_gap: i32 = if is_usecase {
        30
    } else {
        80.max(relation_label_gap)
    };
    let row_gap: i32 = if is_usecase { 20 } else { 64 };
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
            // For UseCase/BusinessUseCase nodes, exclude internal `\x1fuc:` members from the
            // regular display-member count — they are rendered inside the oval and should not
            // inflate the class-box height calculation the same way.
            let is_usecase_node = matches!(
                node.kind,
                FamilyNodeKind::UseCase | FamilyNodeKind::BusinessUseCase
            );
            let display_member_count = if is_usecase_node {
                node.members[header_stereotype_count..]
                    .iter()
                    .filter(|m| !m.text.starts_with("\x1fuc:"))
                    .count()
            } else {
                node.members.len().saturating_sub(header_stereotype_count)
            };
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
            } else if is_usecase_node {
                // UseCase/BusinessUseCase: count ext-point names to size the oval taller.
                let ext_point_count = node
                    .members
                    .iter()
                    .filter(|m| m.text.starts_with("\x1fuc:ext-point:"))
                    .count() as i32;
                // Base height for the oval (fits the name label).
                // With extension points: add divider (≈14px) + 12px per point name.
                if ext_point_count > 0 {
                    (14 + ext_point_count * 12).max(empty_member_pad)
                } else if display_member_count == 0 {
                    empty_member_pad
                } else {
                    display_member_count as i32 * member_line_height + 2 * member_padding
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
    // Reserve space below nodes for caption and footer labels.
    let caption_block_h = family_metadata_label_height(document.caption.as_deref());
    let footer_block_h = family_metadata_label_height(document.footer.as_deref());
    let svg_height = canvas.svg_height + caption_block_h + footer_block_h;
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
    // `skinparam shadowing true` drops a subtle shadow under each class/object
    // node rect. Filter id is referenced from class_node_render via the
    // `class_style.shadowing` flag.
    if class_style.shadowing {
        out.push_str(
            "<filter id=\"shadow\" x=\"-10%\" y=\"-10%\" width=\"130%\" height=\"130%\">\
             <feDropShadow dx=\"3\" dy=\"3\" stdDeviation=\"2\" flood-color=\"#00000040\"/>\
             </filter>",
        );
    }
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
    // Extended arrowhead marker defs used by exotic arrowheads (#--, x--, +--, ^--, }--)
    // and shared renderers. A second <defs> block is valid SVG.
    out.push_str("<defs>");
    out.push_str(&format!(
        "<marker id=\"arrow-triangle-filled\" viewBox=\"0 0 12 12\" refX=\"11\" refY=\"6\" \
         markerWidth=\"12\" markerHeight=\"12\" markerUnits=\"userSpaceOnUse\" orient=\"auto-start-reverse\">\
         <polygon points=\"0,0 12,6 0,12\" fill=\"{arrow_stroke}\" stroke=\"{arrow_stroke}\" stroke-width=\"1.5\"/>\
         </marker>",
    ));
    out.push_str(&format!(
        "<marker id=\"arrow-box-filled\" viewBox=\"0 0 12 12\" refX=\"11\" refY=\"6\" \
         markerWidth=\"12\" markerHeight=\"12\" markerUnits=\"userSpaceOnUse\" orient=\"auto-start-reverse\">\
         <rect x=\"2\" y=\"2\" width=\"8\" height=\"8\" fill=\"{arrow_stroke}\" stroke=\"{arrow_stroke}\" stroke-width=\"1.5\"/>\
         </marker>",
    ));
    out.push_str(&format!(
        "<marker id=\"arrow-plus\" viewBox=\"0 0 12 12\" refX=\"11\" refY=\"6\" \
         markerWidth=\"12\" markerHeight=\"12\" markerUnits=\"userSpaceOnUse\" orient=\"auto-start-reverse\">\
         <path d=\"M6,1 L6,11 M1,6 L11,6\" fill=\"none\" stroke=\"{arrow_stroke}\" stroke-width=\"1.8\" stroke-linecap=\"round\"/>\
         </marker>",
    ));
    out.push_str(&format!(
        "<marker id=\"arrow-cross\" viewBox=\"0 0 12 12\" refX=\"11\" refY=\"6\" \
         markerWidth=\"12\" markerHeight=\"12\" markerUnits=\"userSpaceOnUse\" orient=\"auto-start-reverse\">\
         <path d=\"M2,2 L10,10 M10,2 L2,10\" fill=\"none\" stroke=\"{arrow_stroke}\" stroke-width=\"1.8\" stroke-linecap=\"round\"/>\
         </marker>",
    ));
    out.push_str(&format!(
        "<marker id=\"arrow-bracket-open\" viewBox=\"0 0 12 14\" refX=\"11\" refY=\"7\" \
         markerWidth=\"12\" markerHeight=\"14\" markerUnits=\"userSpaceOnUse\" orient=\"auto-start-reverse\">\
         <path d=\"M10,1 L4,1 L4,13 L10,13\" fill=\"none\" stroke=\"{arrow_stroke}\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>\
         </marker>",
    ));
    out.push_str(&format!(
        "<marker id=\"arrow-circle-open\" viewBox=\"0 0 12 12\" refX=\"11\" refY=\"6\" \
         markerWidth=\"12\" markerHeight=\"12\" markerUnits=\"userSpaceOnUse\" orient=\"auto-start-reverse\">\
         <circle cx=\"6\" cy=\"6\" r=\"4\" fill=\"#ffffff\" stroke=\"{arrow_stroke}\" stroke-width=\"1.5\"/>\
         </marker>",
    ));
    out.push_str(&format!(
        "<marker id=\"arrow-circle-filled\" viewBox=\"0 0 12 12\" refX=\"11\" refY=\"6\" \
         markerWidth=\"12\" markerHeight=\"12\" markerUnits=\"userSpaceOnUse\" orient=\"auto-start-reverse\">\
         <circle cx=\"6\" cy=\"6\" r=\"4\" fill=\"{arrow_stroke}\" stroke=\"{arrow_stroke}\" stroke-width=\"1.5\"/>\
         </marker>",
    ));
    out.push_str(&format!(
        "<marker id=\"arrow-double-open\" viewBox=\"0 0 16 12\" refX=\"15\" refY=\"6\" \
         markerWidth=\"16\" markerHeight=\"12\" markerUnits=\"userSpaceOnUse\" orient=\"auto-start-reverse\">\
         <path d=\"M0,1 L8,6 L0,11 M7,1 L15,6 L7,11\" fill=\"none\" stroke=\"{arrow_stroke}\" stroke-width=\"1.5\" stroke-linejoin=\"round\"/>\
         </marker>",
    ));
    out.push_str("</defs>");

    // Header — rendered above the title at the very top of the canvas.
    // The header_block_h added to margin_top already reserves vertical space.
    if let Some(header_text) = &document.header {
        render_family_metadata_label(
            &mut out,
            header_text,
            "header",
            document.header_align,
            16,
            svg_width,
            "fill=\"#333333\"",
        );
    }

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
    const PARALLEL_EDGE_GAP: i32 = 20;
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

    // #1427: For usecase diagrams, fan edges from the same actor source
    // horizontally so that actor→usecase edges don't share a single vertical
    // stem.  Edges from an Actor/BusinessActor node to UseCase/BusinessUseCase
    // targets are grouped by source actor name and assigned lateral (x) offsets
    // spaced 20px apart, centred at zero.  The parallel_offset pass above only
    // separates same from/to pairs; this pass separates same-source fans.
    //
    // Only Actor→UseCase edges are fanned (not actor-to-actor generalizations)
    // so that the hollow-triangle generalization arrows are not disturbed.
    if is_usecase {
        // Build actor name lookup (alias or bare name, whichever is used in
        // relations).
        let actor_name_set: std::collections::BTreeSet<String> = document
            .nodes
            .iter()
            .filter(|n| {
                matches!(
                    n.kind,
                    FamilyNodeKind::Actor | FamilyNodeKind::BusinessActor
                )
            })
            .flat_map(|n| {
                let key = n.alias.clone().unwrap_or_else(|| n.name.clone());
                if let Some(alias) = &n.alias {
                    vec![alias.clone(), n.name.clone(), key]
                } else {
                    vec![n.name.clone(), key]
                }
            })
            .collect();

        // Build usecase target name lookup to exclude actor-to-actor edges.
        // Use cases inside group scopes are stored with scoped names like
        // "E-Commerce Platform::UC1"; also add the bare unscoped tail so that
        // relations using `U --> UC1` still match.
        let usecase_name_set: std::collections::BTreeSet<String> = document
            .nodes
            .iter()
            .filter(|n| {
                matches!(
                    n.kind,
                    FamilyNodeKind::UseCase | FamilyNodeKind::BusinessUseCase
                )
            })
            .flat_map(|n| {
                let key = n.alias.clone().unwrap_or_else(|| n.name.clone());
                let bare = key.rsplit("::").next().unwrap_or(&key).to_string();
                if let Some(alias) = &n.alias {
                    vec![alias.clone(), n.name.clone(), key, bare]
                } else {
                    let name_bare = n.name.rsplit("::").next().unwrap_or(&n.name).to_string();
                    vec![n.name.clone(), key, bare, name_bare]
                }
            })
            .collect();

        // Group actor→usecase edges by source actor name (normalised).
        let mut actor_fan_groups: std::collections::BTreeMap<String, Vec<usize>> =
            std::collections::BTreeMap::new();
        for (i, rel) in document.relations.iter().enumerate() {
            let (fn_, tn_, _) = normalize_relation_endpoints(&rel.from, &rel.to, &rel.arrow);
            if actor_name_set.contains(&fn_) && usecase_name_set.contains(&tn_) {
                actor_fan_groups.entry(fn_).or_default().push(i);
            }
        }
        // Assign x-offsets; skip groups of one (no tangle possible).
        for group in actor_fan_groups.values() {
            if group.len() < 2 {
                continue;
            }
            let n = group.len() as i32;
            for (slot, &idx) in group.iter().enumerate() {
                let lane = slot as i32 - n / 2;
                // Do not override offsets already assigned by the
                // parallel-pair pass (e.g. bidirectional edges).
                parallel_offset
                    .entry(idx)
                    .or_insert(lane * PARALLEL_EDGE_GAP);
            }
        }
    }

    let is_object_diagram = !document.nodes.is_empty()
        && document
            .nodes
            .iter()
            .all(|node| matches!(node.kind, FamilyNodeKind::Object));
    // #1292: Compute group-frame rects for usecase diagrams so the relation
    // renderer can snap edges to frame boundaries.
    let group_frame_rects: Vec<_> = if is_usecase {
        group_frames
            .iter()
            .filter_map(|gf| class_group_frame_rect(gf, max_group_depth, &node_boxes))
            .collect()
    } else {
        Vec::new()
    };
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
            is_object_diagram,
            edge_routing: document.edge_routing,
            is_usecase_layout: is_usecase,
            group_frame_rects,
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
            document.hide_options.contains("circle"),
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

    // Legend block. PlantUML supports `legend left|right|center|top|bottom`
    // followed by free-form body lines; we render a small bordered box at the
    // requested corner. Default placement matches PlantUML (bottom-right).
    if let Some(legend_text) = &document.legend {
        render_family_legend_box(
            &mut out,
            legend_text,
            svg_width,
            svg_height,
            document.legend_halign,
            document.legend_valign,
        );
    }

    // Caption — rendered in italic below the diagram nodes/legend (before footer).
    // `canvas.svg_height` is the bottom of the nodes+legend area (before our additions).
    let caption_y = canvas.svg_height + 14;
    if let Some(caption_text) = &document.caption {
        render_family_metadata_label(
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
    if let Some(footer_text) = &document.footer {
        render_family_metadata_label(
            &mut out,
            footer_text,
            "footer",
            document.footer_align,
            footer_y,
            svg_width,
            "fill=\"#333333\"",
        );
    }

    out.push_str("</svg>");
    crate::output::append_optional_mainframe_svg(&mut out, document.mainframe.as_deref());
    let mut scene = gl_result_class.scene.clone();
    scene.viewport = Rect::new(0.0, 0.0, svg_width as f64, svg_height as f64);
    RenderArtifact::with_scene(out, scene).with_common_command_parts(
        document.scale.clone(),
        document.mainframe.clone(),
        document.mainframe.is_some(),
    )
}
