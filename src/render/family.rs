use super::geometry::{compute_edge_anchors_for_direction, pick_port};
use super::relation::{
    arrow_style, normalize_relation_endpoints, render_lollipop_endpoint,
    render_relation_marker_defs, render_relation_marker_defs_with_prefix, usecase_dependency_label,
};
use super::svg::escape_text;
use crate::ast::MemberModifier;
use crate::model::{
    FamilyDocument, FamilyGroup, FamilyNode, FamilyNodeKind, FamilyOrientation, FamilyStyle,
};
use crate::theme::{ClassStyle, ComponentStyle};

/// Backwards-compatible alias for the family stub renderer. Now delegates to
/// the real renderer.
pub fn render_family_stub_svg(document: &FamilyDocument) -> String {
    render_class_svg(document)
}

/// Render Class/Object/UseCase documents as a real SVG with boxed nodes
/// (header + member compartment) laid out in a simple grid, plus arrows
/// for the document's relations.
pub fn render_class_svg(document: &FamilyDocument) -> String {
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
    // Auto-size node_width from longest member text / node name (fix #572)
    let node_width: i32 = {
        let name_px = document
            .nodes
            .iter()
            .map(|n| n.name.chars().count() as i32 * 8 + 32)
            .max()
            .unwrap_or(200);
        let member_px = document
            .nodes
            .iter()
            .flat_map(|n| n.members.iter())
            .map(|m| m.text.chars().count() as i32 * 7 + 28)
            .max()
            .unwrap_or(0);
        name_px.max(member_px).clamp(160, 320)
    };
    // col_gap wide enough for edge labels between adjacent nodes (fix #564, #575)
    let col_gap: i32 = 80;
    let row_gap: i32 = 64;
    let header_height: i32 = 30;
    let member_line_height: i32 = 16;
    let member_padding: i32 = 8;
    let empty_member_pad: i32 = 8;
    // group_top_reserve must match label_header+pad used in frame rendering loop
    let group_top_reserve = if group_frames.is_empty() {
        0
    } else {
        ((max_group_depth as i32) + 1) * 28
    };

    // Compute heights per node
    struct NodeBox {
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        header_h: i32,
    }
    let mut node_boxes: std::collections::BTreeMap<String, NodeBox> =
        std::collections::BTreeMap::new();
    // Stable iteration: keep declaration order
    let mut ordered_keys: Vec<String> = Vec::new();

    let title_block_height = document
        .title
        .as_deref()
        .map(|t| 12 + (t.lines().count() as i32) * 22)
        .unwrap_or(0);

    let total_nodes = document.nodes.len() as i32;
    let row_count = if total_nodes == 0 {
        0
    } else {
        (total_nodes + col_count - 1) / col_count
    };

    // First pass: compute heights
    let mut row_heights: Vec<i32> = vec![0; row_count.max(0) as usize];
    for (idx, node) in document.nodes.iter().enumerate() {
        let col = (idx as i32) % col_count;
        let row = (idx as i32) / col_count;
        // Count header stereotype members (built-in type marker + user-defined <<…>> markers).
        // These are rendered in the header, not as member rows (fix #470, #551).
        let header_stereotype_count = count_header_stereotype_members(&node.members);
        let display_member_count = node.members.len().saturating_sub(header_stereotype_count);
        // Extra header height: 14px per stereotype label shown above the class name.
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
        } else if display_member_count == 0 {
            empty_member_pad
        } else {
            (display_member_count as i32) * member_line_height + 2 * member_padding
        };
        let h = c4_node_height(node.kind, header_height + stereotype_extra_h + body_h);
        let _ = col;
        if (row as usize) < row_heights.len() && h > row_heights[row as usize] {
            row_heights[row as usize] = h;
        }
    }

    // Second pass: assign coordinates
    let mut row_y_offsets: Vec<i32> = vec![0; row_heights.len()];
    {
        let mut y = margin_top + title_block_height + group_top_reserve;
        for (i, h) in row_heights.iter().enumerate() {
            row_y_offsets[i] = y;
            y += h + row_gap;
        }
    }

    for (idx, node) in document.nodes.iter().enumerate() {
        let col = (idx as i32) % col_count;
        let row = (idx as i32) / col_count;
        let header_stereotype_count2 = count_header_stereotype_members(&node.members);
        let display_member_count2 = node.members.len().saturating_sub(header_stereotype_count2);
        let stereotype_extra_h2 = (header_stereotype_count2 as i32) * 14;
        let body_h = if node.kind == FamilyNodeKind::Note {
            let lines = node
                .label
                .as_deref()
                .unwrap_or(&node.name)
                .lines()
                .count()
                .max(1) as i32;
            lines * 16 + 20
        } else if display_member_count2 == 0 {
            empty_member_pad
        } else {
            (display_member_count2 as i32) * member_line_height + 2 * member_padding
        };
        let h = c4_node_height(node.kind, header_height + stereotype_extra_h2 + body_h);
        let x = margin_x + col * (node_width + col_gap);
        let y = row_y_offsets[row as usize];
        let key = node.alias.clone().unwrap_or_else(|| node.name.clone());
        ordered_keys.push(key.clone());
        node_boxes.insert(
            key,
            NodeBox {
                x,
                y,
                w: node_width,
                h,
                header_h: header_height,
            },
        );
        // Also accept name as a key if alias was used (for relations referring by name)
        if let Some(_alias) = &node.alias {
            node_boxes.insert(
                node.name.clone(),
                NodeBox {
                    x,
                    y,
                    w: node_width,
                    h,
                    header_h: header_height,
                },
            );
        }
    }

    let nodes_bottom = row_y_offsets
        .iter()
        .zip(row_heights.iter())
        .map(|(y, h)| y + h)
        .max()
        .unwrap_or(margin_top + title_block_height);

    // Compute width / height of the SVG; account for JSON projection height.
    let proj_extra_height: i32 = document.json_projections.iter().fold(0, |acc, proj| {
        let kv_count = extract_projection_kv_lines(&proj.body, &proj.format).len() as i32;
        acc + 22 + kv_count * 16 + 8 + 12
    });
    let svg_width = margin_x * 2 + col_count * node_width + (col_count - 1) * col_gap;
    let svg_height = nodes_bottom + 40 + proj_extra_height;

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">",
        w = svg_width,
        h = svg_height
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

    // Render relations first so node rectangles cover endpoints
    for relation in &document.relations {
        let (from_name, to_name, normalized_arrow) =
            normalize_relation_endpoints(&relation.from, &relation.to, &relation.arrow);
        let from = node_boxes.get(&from_name);
        let to = node_boxes.get(&to_name);
        let (Some(from), Some(to)) = (from, to) else {
            continue;
        };
        let mut style = arrow_style(&normalized_arrow);
        let usecase_dependency = usecase_dependency_label(relation.label.as_deref())
            .or_else(|| usecase_dependency_label(relation.stereotype.as_deref()));
        if usecase_dependency.is_some() {
            style.dashed = true;
            if style.end_marker.is_none() {
                style.end_marker = Some("arrow-open");
            }
        }
        let (x1, y1, x2, y2) = if relation.direction.is_some() {
            compute_edge_anchors_for_direction(
                (from.x, from.y, from.w, from.h),
                (to.x, to.y, to.w, to.h),
                relation.direction.as_deref(),
            )
        } else {
            // Port-based anchoring: attach to mid-point of the nearest box edge
            // (left/right for horizontal-dominant, top/bottom for vertical-dominant).
            // Part of the layout engine refactor (#591, #590 epic).
            pick_port((from.x, from.y, from.w, from.h), (to.x, to.y, to.w, to.h))
        };
        let relation_color = relation
            .line_color
            .as_deref()
            .unwrap_or(arrow_stroke.as_str());
        let stroke_width = relation.thickness.unwrap_or(2).clamp(1, 8);
        let stroke_dash = if style.dashed || relation.dashed {
            " stroke-dasharray=\"5 3\""
        } else {
            ""
        };
        let visibility = if relation.hidden {
            " visibility=\"hidden\""
        } else {
            ""
        };
        let mut markers = String::new();
        if let Some(end) = style.end_marker {
            markers.push_str(&format!(" marker-end=\"url(#{end})\""));
        }
        if let Some(start) = style.start_marker {
            markers.push_str(&format!(" marker-start=\"url(#{start})\""));
        }
        let direction_attr = relation
            .direction
            .as_deref()
            .map(|direction| format!(" data-uml-direction=\"{}\"", escape_text(direction)))
            .unwrap_or_default();
        out.push_str(&format!(
                "<line class=\"uml-relation\" data-uml-from=\"{}\" data-uml-to=\"{}\" x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\" stroke=\"{relation_color}\" stroke-width=\"{stroke_width}\"{dash}{visibility}{direction_attr}{markers}/>",
                escape_text(&relation.from),
                escape_text(&relation.to),
                dash = stroke_dash
            ));
        if relation.left_lollipop {
            render_lollipop_endpoint(&mut out, x1, y1, relation_color);
        }
        if relation.right_lollipop {
            render_lollipop_endpoint(&mut out, x2, y2, relation_color);
        }
        if let Some(left) = &relation.left_cardinality {
            out.push_str(&format!(
                "<text x=\"{x}\" y=\"{y}\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"10\" fill=\"{member_color}\">{txt}</text>",
                x = x1 - 4,
                y = y1 - 6,
                member_color = class_style.member_color,
                txt = escape_text(left)
            ));
        }
        if let Some(right) = &relation.right_cardinality {
            out.push_str(&format!(
                "<text x=\"{x}\" y=\"{y}\" text-anchor=\"start\" font-family=\"monospace\" font-size=\"10\" fill=\"{member_color}\">{txt}</text>",
                x = x2 + 4,
                y = y2 - 6,
                member_color = class_style.member_color,
                txt = escape_text(right)
            ));
        }
        if let Some(left_role) = &relation.left_role {
            out.push_str(&format!(
                "<text x=\"{x}\" y=\"{y}\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"10\" fill=\"{member_color}\">{txt}</text>",
                x = x1 - 4,
                y = y1 + 12,
                member_color = class_style.member_color,
                txt = escape_text(left_role)
            ));
        }
        if let Some(right_role) = &relation.right_role {
            out.push_str(&format!(
                "<text x=\"{x}\" y=\"{y}\" text-anchor=\"start\" font-family=\"monospace\" font-size=\"10\" fill=\"{member_color}\">{txt}</text>",
                x = x2 + 4,
                y = y2 + 12,
                member_color = class_style.member_color,
                txt = escape_text(right_role)
            ));
        }
        if let Some(stereotype) = &relation.stereotype {
            if usecase_dependency.is_none() {
                let sx = (x1 + x2) / 2;
                let sy = (y1 + y2) / 2 - if relation.label.is_some() { 20 } else { 6 };
                out.push_str(&format!(
                    "<text x=\"{sx}\" y=\"{sy}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"{member_color}\">&lt;&lt;{txt}&gt;&gt;</text>",
                    member_color = class_style.member_color,
                    txt = escape_text(stereotype)
                ));
            }
        }
        // UseCase dependency labels (<<extend>>, <<include>>) at midpoint above edge (fix #575).
        // Regular relation labels at 40% from source, clamped 30px from endpoints (fix #564).
        if let Some(label) = usecase_dependency {
            let dx_abs = (x2 - x1).abs();
            let dy_abs = (y2 - y1).abs();
            let (lx, ly) = if dy_abs > dx_abs {
                ((x1 + x2) / 2 + 14, (y1 + y2) / 2 - 6)
            } else {
                ((x1 + x2) / 2, (y1 + y2) / 2 - 14)
            };
            out.push_str(&format!(
                "<text x=\"{lx}\" y=\"{ly}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{member_color}\">{txt}</text>",
                member_color = class_style.member_color,
                txt = escape_text(label)
            ));
        } else if let Some(label) = relation.label.as_deref() {
            let dx = x2 - x1;
            let dy = y2 - y1;
            let dx_abs = dx.abs();
            let dy_abs = dy.abs();
            let edge_len = ((dx_abs * dx_abs + dy_abs * dy_abs) as f64).sqrt() as i32;
            let (lx, ly) = if edge_len <= 2 {
                ((x1 + x2) / 2, (y1 + y2) / 2 - 12)
            } else {
                let clearance = 30i32;
                let t_num = (edge_len * 2 / 5).max(clearance).min(edge_len - clearance);
                let raw_x = x1 + dx * t_num / edge_len;
                let raw_y = y1 + dy * t_num / edge_len;
                if dy_abs > dx_abs {
                    (raw_x + 14, raw_y - 6)
                } else {
                    (raw_x, raw_y - 14)
                }
            };
            out.push_str(&format!(
                "<text x=\"{lx}\" y=\"{ly}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{member_color}\">{txt}</text>",
                member_color = class_style.member_color,
                txt = escape_text(label)
            ));
        }
    }

    // Render groups (together/package/namespace) as labeled frames BEFORE nodes
    // so node rectangles visually sit on top of the frame borders.
    for group in &group_frames {
        // Compute bounding box around all member nodes in this group
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
        // Add padding around the member bounding box
        let depth_outset = (max_group_depth.saturating_sub(group.depth) as i32) * 18;
        let pad = 16 + depth_outset;
        let label_header = 28 + depth_outset; // extra space at top for the group label
        let fx = gx_min - pad;
        let fy = gy_min - pad - label_header;
        let fw = (gx_max - gx_min) + pad * 2;
        let fh = (gy_max - gy_min) + pad * 2 + label_header;

        let group_label = group.display_label();

        // Frame rectangle
        out.push_str(&format!(
            "<rect class=\"uml-group-frame\" data-uml-group=\"{}\" x=\"{fx}\" y=\"{fy}\" width=\"{fw}\" height=\"{fh}\" rx=\"6\" ry=\"6\" fill=\"none\" stroke=\"#6366f1\" stroke-width=\"1.5\" stroke-dasharray=\"5 3\"/>",
            escape_text(&group.scope)
        ));
        // Group label text
        out.push_str(&format!(
            "<text x=\"{tx}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"11\" font-weight=\"600\" fill=\"#4338ca\">{label}</text>",
            tx = fx + 8,
            ty = fy + 14,
            label = escape_text(&group_label)
        ));
    }

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
        );
    }

    // Render inline JSON projections (yellow labelled rectangles with key: value layout).
    if !document.json_projections.is_empty() {
        let proj_margin_left = margin_x;
        let mut proj_y = nodes_bottom + 16;
        let proj_width = 300_i32;
        let proj_line_h = 16_i32;
        let proj_header_h = 22_i32;
        let proj_padding = 8_i32;

        for proj in &document.json_projections {
            let kv_lines = extract_projection_kv_lines(&proj.body, &proj.format);
            let body_h = (kv_lines.len() as i32) * proj_line_h + proj_padding * 2;
            let proj_h = proj_header_h + body_h;

            // Yellow fill for the JSON projection box.
            out.push_str(&format!(
                "<rect x=\"{px}\" y=\"{py}\" width=\"{pw}\" height=\"{ph}\" rx=\"4\" ry=\"4\" fill=\"#fffde7\" stroke=\"#f59e0b\" stroke-width=\"1.5\"/>",
                px = proj_margin_left,
                py = proj_y,
                pw = proj_width,
                ph = proj_h,
            ));
            // Header: alias name.
            out.push_str(&format!(
                "<rect x=\"{px}\" y=\"{py}\" width=\"{pw}\" height=\"{hh}\" rx=\"4\" ry=\"4\" fill=\"#fef08a\" stroke=\"#f59e0b\" stroke-width=\"1.5\"/>",
                px = proj_margin_left,
                py = proj_y,
                pw = proj_width,
                hh = proj_header_h,
            ));
            out.push_str(&format!(
                "<text x=\"{tx}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"12\" font-weight=\"600\" fill=\"#78350f\">{alias} ({format})</text>",
                tx = proj_margin_left + 8,
                ty = proj_y + 15,
                alias = escape_text(&proj.alias),
                format = escape_text(&proj.format),
            ));
            // Separator line.
            out.push_str(&format!(
                "<line x1=\"{lx1}\" y1=\"{ly}\" x2=\"{lx2}\" y2=\"{ly}\" stroke=\"#f59e0b\" stroke-width=\"1\"/>",
                lx1 = proj_margin_left,
                ly = proj_y + proj_header_h,
                lx2 = proj_margin_left + proj_width,
            ));
            // Body: key: value lines.
            for (idx, kv) in kv_lines.iter().enumerate() {
                let text_y =
                    proj_y + proj_header_h + proj_padding + (idx as i32) * proj_line_h + 12;
                out.push_str(&format!(
                    "<text x=\"{tx}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"11\" fill=\"#1e293b\">{kv}</text>",
                    tx = proj_margin_left + 8,
                    ty = text_y,
                    kv = escape_text(kv),
                ));
            }

            proj_y += proj_h + 12;
        }
    }

    out.push_str("</svg>");
    out
}

/// Extract deterministic display lines from a JSON/YAML projection body.
fn extract_projection_kv_lines(body: &str, format: &str) -> Vec<String> {
    if format == "json" {
        if let Some(value) = parse_projection_json_value(body) {
            let mut lines = Vec::new();
            flatten_projection_json("", &value, &mut lines);
            if !lines.is_empty() {
                return lines;
            }
        }
    }
    if format == "yaml" {
        let lines = parse_projection_yaml_value(body)
            .map(|value| {
                let mut lines = Vec::new();
                flatten_projection_yaml("", &value, &mut lines);
                lines
            })
            .unwrap_or_else(|| extract_yaml_kv_lines(body));
        if !lines.is_empty() {
            return lines;
        }
    }
    extract_json_kv_lines(body)
}

fn parse_projection_yaml_value(body: &str) -> Option<yaml_rust2::Yaml> {
    yaml_rust2::YamlLoader::load_from_str(body.trim())
        .ok()
        .and_then(|docs| {
            docs.into_iter()
                .find(|doc| !matches!(doc, yaml_rust2::Yaml::BadValue))
        })
}

fn parse_projection_json_value(body: &str) -> Option<serde_json::Value> {
    let trimmed = body.trim();
    serde_json::from_str::<serde_json::Value>(trimmed)
        .ok()
        .or_else(|| serde_json::from_str::<serde_json::Value>(&format!("{{{trimmed}}}")).ok())
}

fn family_projection_extra_height(projections: &[crate::model::JsonProjection]) -> i32 {
    if projections.is_empty() {
        return 0;
    }
    projections.iter().fold(12, |acc, proj| {
        let line_count = extract_projection_kv_lines(&proj.body, &proj.format)
            .len()
            .max(1) as i32;
        acc + 22 + 16 + (line_count * 16) + 20
    })
}

fn render_family_projection_boxes(
    out: &mut String,
    projections: &[crate::model::JsonProjection],
    x: i32,
    mut y: i32,
    width: i32,
) {
    for proj in projections {
        let kv_lines = extract_projection_kv_lines(&proj.body, &proj.format);
        let lines = if kv_lines.is_empty() {
            vec!["(empty)".to_string()]
        } else {
            kv_lines
        };
        let header_h = 22;
        let line_h = 16;
        let body_h = (lines.len() as i32) * line_h + 16;
        let height = header_h + body_h;
        out.push_str(&format!(
            "<g class=\"uml-projection\" data-uml-projection=\"{}\" data-uml-projection-format=\"{}\" data-uml-projection-lines=\"{}\">",
            escape_text(&proj.alias),
            escape_text(&proj.format),
            lines.len()
        ));
        out.push_str(&format!(
            "<rect x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" rx=\"5\" ry=\"5\" fill=\"#fffde7\" stroke=\"#f59e0b\" stroke-width=\"1.5\"/>"
        ));
        out.push_str(&format!(
            "<rect x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{header_h}\" rx=\"5\" ry=\"5\" fill=\"#fef08a\" stroke=\"#f59e0b\" stroke-width=\"1.5\"/>"
        ));
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" font-weight=\"600\" fill=\"#78350f\">{} ({})</text>",
            x + 8,
            y + 15,
            escape_text(&proj.alias),
            escape_text(&proj.format)
        ));
        for (idx, line) in lines.iter().enumerate() {
            out.push_str(&format!(
                "<text class=\"uml-projection-row\" data-uml-projection-row=\"{}\" x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#1e293b\">{}</text>",
                idx,
                x + 8,
                y + header_h + 18 + (idx as i32 * line_h),
                escape_text(line)
            ));
        }
        out.push_str("</g>");
        y += height + 12;
    }
}

fn flatten_projection_json(prefix: &str, value: &serde_json::Value, lines: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(obj) => {
            for (key, value) in obj {
                let next = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten_projection_json(&next, value, lines);
            }
        }
        serde_json::Value::Array(items) => {
            for (idx, value) in items.iter().enumerate() {
                flatten_projection_json(&format!("{prefix}[{idx}]"), value, lines);
            }
        }
        serde_json::Value::String(s) => lines.push(format!("{prefix}: {s}")),
        serde_json::Value::Number(n) => lines.push(format!("{prefix}: {n}")),
        serde_json::Value::Bool(b) => lines.push(format!("{prefix}: {b}")),
        serde_json::Value::Null => lines.push(format!("{prefix}: null")),
    }
}

fn flatten_projection_yaml(prefix: &str, value: &yaml_rust2::Yaml, lines: &mut Vec<String>) {
    match value {
        yaml_rust2::Yaml::Hash(map) => {
            for (key, value) in map {
                let key = projection_yaml_label(key);
                let next = if prefix.is_empty() {
                    key
                } else {
                    format!("{prefix}.{key}")
                };
                flatten_projection_yaml(&next, value, lines);
            }
        }
        yaml_rust2::Yaml::Array(items) => {
            for (idx, value) in items.iter().enumerate() {
                flatten_projection_yaml(&format!("{prefix}[{idx}]"), value, lines);
            }
        }
        scalar => lines.push(format!("{prefix}: {}", projection_yaml_label(scalar))),
    }
}

fn projection_yaml_label(value: &yaml_rust2::Yaml) -> String {
    match value {
        yaml_rust2::Yaml::Real(s) | yaml_rust2::Yaml::String(s) => s.clone(),
        yaml_rust2::Yaml::Integer(n) => n.to_string(),
        yaml_rust2::Yaml::Boolean(b) => b.to_string(),
        yaml_rust2::Yaml::Alias(id) => format!("*{id}"),
        yaml_rust2::Yaml::Null => "null".to_string(),
        yaml_rust2::Yaml::BadValue => "(invalid)".to_string(),
        yaml_rust2::Yaml::Array(_) => "[...]".to_string(),
        yaml_rust2::Yaml::Hash(_) => "{...}".to_string(),
    }
}

fn extract_yaml_kv_lines(body: &str) -> Vec<String> {
    let mut path: Vec<String> = Vec::new();
    let mut lines = Vec::new();
    for raw in body.lines() {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = raw.chars().take_while(|c| *c == ' ').count() / 2;
        path.truncate(indent);
        let item = trimmed.strip_prefix("- ").unwrap_or(trimmed);
        let Some((key, value)) = item.split_once(':') else {
            continue;
        };
        let key = key.trim().trim_matches('"').trim_matches('\'').to_string();
        let value = value.trim().trim_matches('"').trim_matches('\'');
        if value.is_empty() {
            path.push(key);
        } else {
            let mut full = path.clone();
            full.push(key);
            lines.push(format!("{}: {}", full.join("."), value));
        }
    }
    lines
}

/// Extract `key: value` display lines from a JSON-ish body string.
/// Strips outer braces/brackets, parses simple string-keyed properties.
fn extract_json_kv_lines(body: &str) -> Vec<String> {
    let mut lines = Vec::new();
    // Simple line-by-line extraction: look for `"key": value` patterns.
    for raw in body.lines() {
        let trimmed = raw.trim().trim_end_matches(',');
        if trimmed.is_empty()
            || trimmed == "{"
            || trimmed == "}"
            || trimmed == "["
            || trimmed == "]"
        {
            continue;
        }
        // Try to extract key: value from `"key": value` form.
        if let Some(kv) = parse_json_kv_display(trimmed) {
            lines.push(kv);
        } else if !trimmed.is_empty() {
            // Just push the trimmed line if we can't parse it as k/v.
            lines.push(trimmed.to_string());
        }
    }
    // If body is a flat single-line JSON, try splitting on commas.
    if lines.is_empty() && !body.trim().is_empty() {
        let flat = body
            .trim()
            .trim_start_matches('{')
            .trim_end_matches('}')
            .trim();
        for segment in flat.split(',') {
            let seg = segment.trim().trim_end_matches(',');
            if !seg.is_empty() {
                if let Some(kv) = parse_json_kv_display(seg) {
                    lines.push(kv);
                }
            }
        }
    }
    lines
}

/// Parse a single JSON key-value segment like `"name": "Alice"` → `name: Alice`.
fn parse_json_kv_display(segment: &str) -> Option<String> {
    // Expect: optional quote, key chars, optional quote, `:`, value
    let (key_part, val_part) = segment.split_once(':')?;
    let key = key_part.trim().trim_matches('"');
    let val = val_part.trim().trim_matches('"');
    if key.is_empty() {
        return None;
    }
    Some(format!("{key}: {val}"))
}

pub fn render_family_tree_svg(document: &FamilyDocument) -> String {
    const MARGIN: i32 = 24;
    const CHAR_WIDTH: i32 = 7;
    const NODE_FONT_SIZE: i32 = 12;
    const NODE_MIN_WIDTH: i32 = 220;
    const NODE_MAX_WIDTH: i32 = 360;
    const NODE_PADDING_X: i32 = 12;
    const NODE_PADDING_Y: i32 = 12;
    const MIN_SPACING_X: i32 = 80;
    const MIN_SPACING_Y: i32 = 48;
    const MAX_LINE_CHARS: usize = 24;

    let mut out = String::new();
    let title_lines = document
        .title
        .as_deref()
        .map(|v| v.lines().collect::<Vec<_>>())
        .unwrap_or_default();

    let hide_empty_members = document.hide_options.contains("empty members")
        || document.hide_options.contains("empty methods")
        || document.hide_options.contains("empty fields");
    let hide_circle = document.hide_options.contains("circle");
    let hide_stereotype = document.hide_options.contains("stereotype");

    let mut layouts = Vec::with_capacity(document.nodes.len());
    for node in &document.nodes {
        let raw_label = node.alias.as_ref().map_or_else(
            || node.name.clone(),
            |alias| format!("{} as {}", node.name, alias),
        );
        let lines = wrap_text(raw_label, MAX_LINE_CHARS, document.text_overflow_policy);
        let width_chars = lines
            .iter()
            .map(|line| line.chars().count() as i32)
            .max()
            .unwrap_or(1);
        let width =
            (width_chars * CHAR_WIDTH + (NODE_PADDING_X * 2)).clamp(NODE_MIN_WIDTH, NODE_MAX_WIDTH);
        let member_count = if hide_empty_members && node.members.is_empty() {
            0
        } else {
            node.members.len() as i32
        };
        let height = (lines.len() as i32 * 18) + (NODE_PADDING_Y * 2) + (member_count * 16);
        layouts.push(NodeLayout {
            label_lines: lines,
            width,
            height,
            x: 0,
            y: 0,
        });
    }

    let mut levels = Vec::<Vec<usize>>::new();
    let mut max_depth = 0usize;
    for (idx, node) in document.nodes.iter().enumerate() {
        let depth = node.depth;
        if depth > max_depth {
            max_depth = depth;
        }
        if levels.len() <= depth {
            levels.resize_with(depth + 1, Vec::new);
        }
        levels[depth].push(idx);
    }

    let mut depth_slot = vec![0usize; document.nodes.len()];
    for level_nodes in &levels {
        for (slot, idx) in level_nodes.iter().copied().enumerate() {
            depth_slot[idx] = slot;
        }
    }

    let max_node_width = layouts
        .iter()
        .map(|layout| layout.width)
        .max()
        .unwrap_or(NODE_MIN_WIDTH);
    let max_node_height = layouts
        .iter()
        .map(|layout| layout.height)
        .max()
        .unwrap_or(58);

    let x_step = max_node_width + MIN_SPACING_X;
    let y_step = max_node_height + MIN_SPACING_Y;

    let mut y_offsets = vec![0i32; levels.len()];
    for i in 1..levels.len() {
        let prev = y_offsets[i - 1] + y_step;
        y_offsets[i] = prev;
    }

    let vertical = matches!(
        document.orientation,
        FamilyOrientation::TopToBottom | FamilyOrientation::BottomToTop
    );

    let mut height_offset = MARGIN;
    if !title_lines.is_empty() {
        height_offset += (title_lines.len() as i32) * 24;
        height_offset += 12;
    }
    // Extra space for groups
    height_offset += (document.groups.len() as i32) * 48;

    for (depth, level_nodes) in levels.iter().enumerate() {
        for &node_idx in level_nodes {
            let slot = depth_slot[node_idx] as i32;
            let display_depth = match document.orientation {
                FamilyOrientation::TopToBottom => depth,
                FamilyOrientation::BottomToTop => max_depth.saturating_sub(depth),
                FamilyOrientation::LeftToRight => depth,
                FamilyOrientation::RightToLeft => max_depth.saturating_sub(depth),
            };

            if vertical {
                layouts[node_idx].x = MARGIN + (slot * x_step);
                layouts[node_idx].y = height_offset + (display_depth as i32 * y_step);
            } else {
                layouts[node_idx].x = MARGIN + (display_depth as i32 * x_step);
                layouts[node_idx].y = MARGIN + (slot * y_step);
            }
        }
    }

    let mut max_x = MARGIN;
    let mut max_y = height_offset;
    for layout in &layouts {
        max_x = max_x.max(layout.x + layout.width);
        max_y = max_y.max(layout.y + layout.height);
    }
    if !title_lines.is_empty() {
        max_y = max_y.max(height_offset);
    }

    let width = (max_x + MARGIN).max(760);
    let height = (max_y + MARGIN).max(180);

    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        width, height, width, height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");

    let mut y_cursor = MARGIN;
    if !title_lines.is_empty() {
        for line in &title_lines {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\">{}</text>",
                MARGIN,
                y_cursor,
                escape_text(line)
            ));
            y_cursor += 24;
        }
        y_cursor += 12;
    }
    // Render groups (together/package/namespace) as labeled frames before class boxes
    for group in &document.groups {
        let group_label = match group.label.as_deref() {
            // `rectangle` is a visual-boundary keyword; show just the label (fix #553)
            Some(lbl) if group.kind == "rectangle" => lbl.to_string(),
            Some(lbl) => format!("{} {}", group.kind, lbl),
            None => group.kind.clone(),
        };
        let member_list = group.member_ids.join(", ");
        out.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"200\" height=\"40\" rx=\"6\" ry=\"6\" fill=\"#f0f4ff\" stroke=\"#6366f1\" stroke-width=\"1.5\"/>",
            MARGIN,
            y_cursor
        ));
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" font-weight=\"600\" fill=\"#4338ca\">{}</text>",
            MARGIN + 8,
            y_cursor + 14,
            escape_text(&group_label)
        ));
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" fill=\"#6366f1\">{}</text>",
            MARGIN + 8,
            y_cursor + 28,
            escape_text(&member_list)
        ));
        y_cursor += 48;
    }

    for (idx, layout) in layouts.iter().enumerate() {
        out.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" ry=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            layout.x,
            layout.y,
            layout.width,
            layout.height,
            document.style.participant_background_color,
            document.style.participant_border_color
        ));

        let node = &document.nodes[idx];
        // Render label lines (name, alias)
        for (line_idx, line) in layout.label_lines.iter().enumerate() {
            let tx = if !hide_circle && node.kind == crate::model::FamilyNodeKind::Class {
                layout.x + NODE_PADDING_X + 16
            } else {
                layout.x + NODE_PADDING_X
            };
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"{}\" fill=\"#0f172a\">{}</text>",
                tx,
                layout.y + NODE_PADDING_Y + (line_idx as i32 * 18),
                NODE_FONT_SIZE,
                escape_text(line)
            ));
        }
        // Class circle icon
        if !hide_circle && node.kind == crate::model::FamilyNodeKind::Class {
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"6\" fill=\"none\" stroke=\"#64748b\" stroke-width=\"1\"/>",
                layout.x + NODE_PADDING_X + 8,
                layout.y + NODE_PADDING_Y + 6
            ));
        }
        // Render members with visibility markers + modifier styling
        let show_members = !hide_empty_members || !node.members.is_empty();
        if show_members {
            let member_y_base =
                layout.y + NODE_PADDING_Y + (layout.label_lines.len() as i32 * 18) + 4;
            for (midx, member) in node.members.iter().enumerate() {
                let my = member_y_base + (midx as i32 * 16);
                let (symbol, color, member_text) = parse_visibility_member(&member.text);
                if let Some(sym) = symbol {
                    out.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
                        layout.x + NODE_PADDING_X,
                        my,
                        color,
                        escape_text(sym)
                    ));
                }
                let (base_style, clean_text) = parse_member_modifiers(member_text);
                let mut extra_style = String::from(base_style);
                match &member.modifier {
                    Some(MemberModifier::Abstract) | Some(MemberModifier::Field) => {
                        if !extra_style.contains("font-style") {
                            extra_style.push_str(" font-style=\"italic\"");
                        }
                    }
                    Some(MemberModifier::Static) => {
                        if !extra_style.contains("text-decoration") {
                            extra_style.push_str(" text-decoration=\"underline\"");
                        }
                    }
                    Some(MemberModifier::Method) | None => {}
                }
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#334155\"{}>{}</text>",
                    layout.x + NODE_PADDING_X + 12,
                    my,
                    extra_style,
                    escape_text(clean_text)
                ));
            }
        }
    }
    let _ = hide_stereotype; // used in branch version; suppress warning

    for relation in &document.relations {
        let from_idx = document
            .nodes
            .iter()
            .position(|node| node.name == relation.from)
            .or_else(|| {
                document
                    .nodes
                    .iter()
                    .position(|node| node.alias.as_deref() == Some(relation.from.as_str()))
            });
        let to_idx = document
            .nodes
            .iter()
            .position(|node| node.name == relation.to)
            .or_else(|| {
                document
                    .nodes
                    .iter()
                    .position(|node| node.alias.as_deref() == Some(relation.to.as_str()))
            });

        if let (Some(from), Some(to)) = (from_idx, to_idx) {
            let from_layout = &layouts[from];
            let to_layout = &layouts[to];
            let (x1, y1, x2, y2) = match document.orientation {
                FamilyOrientation::TopToBottom => (
                    from_layout.x + from_layout.width / 2,
                    from_layout.y + from_layout.height,
                    to_layout.x + to_layout.width / 2,
                    to_layout.y,
                ),
                FamilyOrientation::BottomToTop => (
                    from_layout.x + from_layout.width / 2,
                    from_layout.y,
                    to_layout.x + to_layout.width / 2,
                    to_layout.y + to_layout.height,
                ),
                FamilyOrientation::LeftToRight => (
                    from_layout.x + from_layout.width,
                    from_layout.y + from_layout.height / 2,
                    to_layout.x,
                    to_layout.y + to_layout.height / 2,
                ),
                FamilyOrientation::RightToLeft => (
                    from_layout.x,
                    from_layout.y + from_layout.height / 2,
                    to_layout.x + to_layout.width,
                    to_layout.y + to_layout.height / 2,
                ),
            };

            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                x1, y1, x2, y2, document.style.arrow_color
            ));
            render_tree_arrow(&mut out, x1, y1, x2, y2, &document.style.arrow_color);

            if let Some(label) = &relation.label {
                let label = usecase_dependency_label(Some(label)).unwrap_or(label);
                let label_lines = wrap_text(label.to_string(), 18, document.text_overflow_policy);
                let label_x = ((x1 + x2) / 2).max(4);
                let label_y = ((y1 + y2) / 2).min(height - 8);
                for (line_idx, line) in label_lines.iter().enumerate() {
                    out.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\">{}</text>",
                        label_x,
                        label_y + (line_idx as i32 * 12),
                        escape_text(line)
                    ));
                }
            }
        }
    }

    let relation_count = if document.relations.is_empty() {
        "relationships: 0".to_string()
    } else {
        format!("relationships: {}", document.relations.len())
    };
    out.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#475569\">{}</text>",
        MARGIN,
        height - 12,
        relation_count
    ));

    out.push_str("</svg>");
    out
}

/// Render a `@startsalt` wireframe grid as an SVG.
/// Nodes in the FamilyDocument whose `name` starts with `"SALT_ROW\x1f"` are
/// decoded back into cell lists and drawn as a proper wireframe table.
fn parse_visibility_member(member: &str) -> (Option<&'static str>, &'static str, &str) {
    let trimmed = member.trim();
    match trimmed.chars().next() {
        Some('+') => (Some("+"), "#16a34a", trimmed[1..].trim_start()),
        Some('-') => (Some("-"), "#dc2626", trimmed[1..].trim_start()),
        Some('#') => (Some("#"), "#d97706", trimmed[1..].trim_start()),
        Some('~') => (Some("~"), "#7c3aed", trimmed[1..].trim_start()),
        _ => (None, "#334155", trimmed),
    }
}

fn uml_visibility_name(symbol: &str) -> &'static str {
    match symbol {
        "+" => "public",
        "-" => "private",
        "#" => "protected",
        "~" => "package",
        _ => "unknown",
    }
}

fn member_modifier_name(modifier: Option<&MemberModifier>) -> Option<&'static str> {
    match modifier {
        Some(MemberModifier::Field) => Some("field"),
        Some(MemberModifier::Method) => Some("method"),
        Some(MemberModifier::Abstract) => Some("abstract"),
        Some(MemberModifier::Static) => Some("static"),
        None => None,
    }
}

/// Parse {abstract} / {static} modifiers from member text.
/// Returns (SVG style attrs string, cleaned text without modifiers).
fn parse_member_modifiers(text: &str) -> (&'static str, &str) {
    let t = text.trim();
    if let Some(rest) = t.strip_prefix("{abstract}") {
        (" font-style=\"italic\"", rest.trim_start())
    } else if let Some(rest) = t.strip_prefix("{static}") {
        (" text-decoration=\"underline\"", rest.trim_start())
    } else {
        ("", t)
    }
}

pub(crate) fn family_node_label(kind: FamilyNodeKind) -> &'static str {
    match kind {
        FamilyNodeKind::Class => "class",
        FamilyNodeKind::Object => "object",
        FamilyNodeKind::UseCase => "usecase",
        FamilyNodeKind::Salt => "widget",
        FamilyNodeKind::MindMap => "mindmap",
        FamilyNodeKind::Wbs => "wbs",
        FamilyNodeKind::Component => "component",
        FamilyNodeKind::Interface => "interface",
        FamilyNodeKind::Port => "port",
        FamilyNodeKind::Node => "node",
        FamilyNodeKind::Artifact => "artifact",
        FamilyNodeKind::Cloud => "cloud",
        FamilyNodeKind::Frame => "frame",
        FamilyNodeKind::Storage => "storage",
        FamilyNodeKind::Database => "database",
        FamilyNodeKind::Package => "package",
        FamilyNodeKind::Rectangle => "rectangle",
        FamilyNodeKind::Folder => "folder",
        FamilyNodeKind::File => "file",
        FamilyNodeKind::Card => "card",
        FamilyNodeKind::Actor => "actor",
        FamilyNodeKind::State => "state",
        FamilyNodeKind::StateInitial => "initial",
        FamilyNodeKind::StateFinal => "final",
        FamilyNodeKind::StateHistory => "history",
        FamilyNodeKind::ActivityStart => "start",
        FamilyNodeKind::ActivityStop => "stop",
        FamilyNodeKind::ActivityAction => "action",
        FamilyNodeKind::ActivityDecision => "decision",
        FamilyNodeKind::ActivityFork => "fork",
        FamilyNodeKind::ActivityForkEnd => "end fork",
        FamilyNodeKind::ActivityMerge => "merge",
        FamilyNodeKind::ActivityPartition => "partition",
        FamilyNodeKind::TimingConcise => "concise",
        FamilyNodeKind::TimingRobust => "robust",
        FamilyNodeKind::TimingClock => "clock",
        FamilyNodeKind::TimingBinary => "binary",
        FamilyNodeKind::TimingEvent => "event",
        FamilyNodeKind::Note => "note",
        // C4 family
        FamilyNodeKind::C4Person => "person",
        FamilyNodeKind::C4PersonExt => "person_ext",
        FamilyNodeKind::C4System => "system",
        FamilyNodeKind::C4SystemExt => "system_ext",
        FamilyNodeKind::C4SystemDb => "system_db",
        FamilyNodeKind::C4SystemQueue => "system_queue",
        FamilyNodeKind::C4Container => "container",
        FamilyNodeKind::C4ContainerExt => "container_ext",
        FamilyNodeKind::C4ContainerDb => "container_db",
        FamilyNodeKind::C4ContainerQueue => "container_queue",
        FamilyNodeKind::C4Component => "component",
        FamilyNodeKind::C4ComponentExt => "component_ext",
        FamilyNodeKind::C4ComponentDb => "component_db",
        FamilyNodeKind::C4ComponentQueue => "component_queue",
        FamilyNodeKind::C4Boundary => "boundary",
    }
}

struct ClassNodeGeometry {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    header_h: i32,
}

/// Return the recognised kind-stereotype label for a type-marker member
/// (e.g. `"<<enum>>"` → `Some("«enumeration»")`).  Only the built-in
/// keyword markers produced by the parser qualify; user-defined stereotypes
/// like `"<<controller>>"` are NOT covered here (they are handled separately).
fn builtin_type_stereotype_label(text: &str) -> Option<&'static str> {
    match text {
        "<<enum>>" => Some("\u{ab}enumeration\u{bb}"),
        "<<interface>>" => Some("\u{ab}interface\u{bb}"),
        "<<abstract>>" | "<<abstract class>>" => Some("\u{ab}abstract\u{bb}"),
        "<<annotation>>" => Some("\u{ab}annotation\u{bb}"),
        "<<protocol>>" => Some("\u{ab}protocol\u{bb}"),
        "<<struct>>" => Some("\u{ab}struct\u{bb}"),
        _ => None,
    }
}

/// Return true if `text` is an arbitrary user-defined stereotype marker
/// (any `<<…>>` value that is NOT one of the built-in type keywords).
fn is_user_stereotype(text: &str) -> bool {
    text.starts_with("<<") && text.ends_with(">>") && builtin_type_stereotype_label(text).is_none()
}

/// Count how many leading members of `members` are header stereotypes that
/// should be rendered in the class-box header rather than as member rows.
/// This includes the optional built-in type marker (first position) plus any
/// consecutive user-defined stereotype markers that immediately follow it.
fn count_header_stereotype_members(members: &[crate::ast::ClassMember]) -> usize {
    let mut skip = 0;
    // First member may be a built-in type marker (e.g. <<enum>>).
    if members
        .first()
        .is_some_and(|m| builtin_type_stereotype_label(&m.text).is_some())
    {
        skip += 1;
    }
    // Any consecutive user-defined <<…>> members directly after the type marker
    // (or at the start if there was no type marker) are also header stereotypes.
    while skip < members.len() && is_user_stereotype(&members[skip].text) {
        skip += 1;
    }
    skip
}

fn render_class_node(
    out: &mut String,
    node: &crate::model::FamilyNode,
    geometry: ClassNodeGeometry,
    class_style: &ClassStyle,
    namespace_separator: Option<&str>,
) {
    let ClassNodeGeometry {
        x,
        y,
        w,
        h,
        header_h,
    } = geometry;

    // ── C4 node rendering ─────────────────────────────────────────────────────
    if is_c4_kind(node.kind) {
        render_c4_node(out, node, x, y, w, h);
        return;
    }

    if node.kind == FamilyNodeKind::Note {
        render_note_card(out, x, y, w, h, node.label.as_deref().unwrap_or(&node.name));
        return;
    }

    let fill = node
        .fill_color
        .as_deref()
        .unwrap_or(&class_style.background_color);
    let stroke = &class_style.border_color;
    let font_family = class_style.font_name.as_deref().unwrap_or("monospace");
    let title_font_size = class_style.font_size.unwrap_or(13);
    let member_font_size = title_font_size.saturating_sub(2).max(9);
    let header_fill = match node.kind {
        FamilyNodeKind::Class => class_style.header_color.as_str(),
        FamilyNodeKind::Object => "#fef3c7",
        FamilyNodeKind::UseCase => "#dcfce7",
        _ => "#f1f5f9",
    };

    if matches!(node.kind, FamilyNodeKind::Actor) {
        // Stick-figure rendering for actors in usecase diagrams.
        let cx = x + w / 2;
        let fig_top = y + 2;
        // Head
        out.push_str(&format!(
            "<circle cx=\"{cx}\" cy=\"{head_cy}\" r=\"9\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
            head_cy = fig_top + 9
        ));
        // Body
        out.push_str(&format!(
            "<line x1=\"{cx}\" y1=\"{by}\" x2=\"{cx}\" y2=\"{ey}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
            by = fig_top + 18,
            ey = fig_top + 32
        ));
        // Arms
        out.push_str(&format!(
            "<line x1=\"{ax1}\" y1=\"{ay}\" x2=\"{ax2}\" y2=\"{ay}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
            ax1 = cx - 12,
            ay = fig_top + 24,
            ax2 = cx + 12
        ));
        // Left leg
        out.push_str(&format!(
            "<line x1=\"{cx}\" y1=\"{ly}\" x2=\"{lx2}\" y2=\"{ley}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
            ly = fig_top + 32,
            lx2 = cx - 10,
            ley = fig_top + 44
        ));
        // Right leg
        out.push_str(&format!(
            "<line x1=\"{cx}\" y1=\"{ly}\" x2=\"{lx2}\" y2=\"{ley}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
            ly = fig_top + 32,
            lx2 = cx + 10,
            ley = fig_top + 44
        ));
        // Name below the figure
        out.push_str(&format!(
            "<text x=\"{cx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"{}\" font-weight=\"600\" fill=\"{}\">{name}</text>",
            escape_text(font_family),
            title_font_size,
            escape_text(&class_style.font_color),
            ty = fig_top + 58,
            name = escape_text(&node.name)
        ));
        // Stereotype / extra members below name
        let mut member_y = fig_top + 72;
        for member in &node.members {
            let text = member.text.trim();
            if text.is_empty() {
                continue;
            }
            out.push_str(&format!(
                "<text x=\"{cx}\" y=\"{member_y}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"11\" fill=\"#334155\">{}</text>",
                escape_text(font_family),
                escape_text(text)
            ));
            member_y += 14;
        }
        return;
    }

    if matches!(node.kind, FamilyNodeKind::UseCase) {
        // Ellipse rendering for use cases
        let cx = x + w / 2;
        let cy = y + h / 2;
        let rx = w / 2;
        let ry = h / 2;
        out.push_str(&format!(
            "<ellipse cx=\"{cx}\" cy=\"{cy}\" rx=\"{rx}\" ry=\"{ry}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
        ));
        // Resolve display name: namespace-qualified nodes (e.g. "Package::MP") encode
        // the human-readable label as members[0] when the parser embeds `as DisplayName`
        // inside a group. Detect this by checking that members[0] is plain text (not a
        // UML modifier line) and use it as the displayed label (fix #578).
        let (uc_display_name, uc_member_skip): (&str, usize) = if node.name.contains("::") {
            let first_member_is_label = node.members.first().is_some_and(|m| {
                let t = m.text.trim();
                !t.is_empty()
                    && !t.starts_with("<<")
                    && !t.starts_with('+')
                    && !t.starts_with('-')
                    && !t.starts_with('#')
                    && !t.starts_with('~')
                    && !t.starts_with('{')
                    && !t.starts_with('\x1f')
                    && !t.contains(':')
                    && !t.contains('(')
            });
            if first_member_is_label {
                (node.members[0].text.trim(), 1)
            } else {
                let short = node.name.rsplit("::").next().unwrap_or(&node.name);
                (short, 0)
            }
        } else {
            (node.name.as_str(), 0)
        };
        // Name centered — the alias is the internal id only; do NOT display it (fix #478)
        out.push_str(&format!(
            "<text x=\"{cx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"{}\" font-weight=\"600\" fill=\"{}\">{name}</text>",
            escape_text(font_family),
            title_font_size,
            escape_text(&class_style.font_color),
            ty = cy + 4,
            name = escape_text(uc_display_name)
        ));
        // Members rendered below the ellipse (rare for usecases), skipping display-label slot
        let mut my = y + h + 14;
        for member in node.members.iter().skip(uc_member_skip) {
            out.push_str(&format!(
                "<text x=\"{tx}\" y=\"{my}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"{}\" fill=\"{mc}\">{m}</text>",
                escape_text(font_family),
                member_font_size,
                tx = x + w / 2,
                mc = class_style.member_color,
                m = escape_text(&member.text)
            ));
            my += 14;
        }
        return;
    }

    // Collect all leading header stereotype labels (built-in type markers + user-defined
    // <<…>> markers — fix #470 for built-in types, fix #551 for user stereotypes).
    // These are rendered as guillemet labels in the header, NOT as ordinary member rows.
    let header_skip = count_header_stereotype_members(&node.members);
    // Build the list of guillemet labels to show in the header (top → bottom).
    let mut header_stereotype_labels: Vec<String> = Vec::new();
    for m in &node.members[..header_skip] {
        if let Some(builtin) = builtin_type_stereotype_label(&m.text) {
            header_stereotype_labels.push(builtin.to_string());
        } else if is_user_stereotype(&m.text) {
            // Convert <<foo>> → «foo»
            let inner = m.text.trim_start_matches("<<").trim_end_matches(">>");
            header_stereotype_labels.push(format!("\u{ab}{inner}\u{bb}"));
        }
    }
    // Members to display: skip all header stereotype members
    let display_members = &node.members[header_skip..];

    // Outer rect
    out.push_str(&format!(
        "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"4\" ry=\"4\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
    ));
    // Header band — taller when we display stereotype labels (14px per label — fix #470, #551)
    let stereotype_extra = (header_stereotype_labels.len() as i32) * 14;
    let effective_header_h = header_h + stereotype_extra;
    out.push_str(&format!(
        "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{hh}\" rx=\"4\" ry=\"4\" fill=\"{header_fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
        hh = effective_header_h
    ));
    // Header separator line
    out.push_str(&format!(
        "<line x1=\"{x}\" y1=\"{ly}\" x2=\"{x2}\" y2=\"{ly}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
        ly = y + effective_header_h,
        x2 = x + w
    ));

    // Render each stereotype label above the class name in the header (fix #470, #551)
    for (i, label) in header_stereotype_labels.iter().enumerate() {
        out.push_str(&format!(
            "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"{ff}\" font-size=\"10\" fill=\"{fc}\">{lbl}</text>",
            tx = x + w / 2,
            ty = y + 13 + (i as i32) * 14,
            ff = escape_text(font_family),
            fc = escape_text(&class_style.font_color),
            lbl = escape_text(label)
        ));
    }

    // Header text: class name (fix #486 — Object shows `Name : Type` underlined)
    let display_name = namespace_separator
        .filter(|sep| !sep.is_empty())
        .map(|sep| node.name.replace("::", sep))
        .unwrap_or_else(|| node.name.clone());
    // For objects: if the name contains " : " it's already in `name : Type` form;
    // otherwise we show just the name.  Either way we underline per UML.
    let header_text = display_name.clone();
    // Underline for objects (PlantUML convention — fix #486)
    let text_decoration = if matches!(node.kind, FamilyNodeKind::Object) {
        " text-decoration=\"underline\""
    } else {
        ""
    };
    let name_ty = y + effective_header_h - 9;
    out.push_str(&format!(
        "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"{ff}\" font-size=\"{fs}\" font-weight=\"600\" fill=\"{fc}\"{td}>{txt}</text>",
        ff = escape_text(font_family),
        fs = title_font_size,
        fc = escape_text(&class_style.font_color),
        tx = x + w / 2,
        ty = name_ty,
        td = text_decoration,
        txt = escape_text(&header_text)
    ));

    // Members — split by `--` / `..` divider tokens to draw compartment lines (fix #468).
    // We also auto-insert a divider between the last attribute and the first operation
    // when there is no explicit divider in the source (fix #468 second compartment).
    //
    // Pre-scan: detect whether there are both attributes and operations in display_members
    // so we know to auto-insert a divider at the transition boundary.
    let has_explicit_divider = display_members
        .iter()
        .any(|m| m.text.trim() == "--" || m.text.trim() == "..");
    let auto_divider = if !has_explicit_divider {
        // Determine the index of the first operation (text containing '(') after at least one attribute.
        let mut first_op_idx: Option<usize> = None;
        let mut seen_attr = false;
        for (i, m) in display_members.iter().enumerate() {
            let t = m.text.trim();
            if t == "--" || t == ".." || t.is_empty() {
                continue;
            }
            // Strip visibility prefix before checking for '('
            let (_vis, _col, rest) = parse_visibility_member(t);
            if rest.contains('(') {
                if seen_attr {
                    first_op_idx = Some(i);
                }
                break;
            } else {
                seen_attr = true;
            }
        }
        first_op_idx
    } else {
        None
    };

    let mut my = y + effective_header_h + 16;
    let mut section_started = false; // tracks if we've seen at least one non-divider member
    for (midx, member) in display_members.iter().enumerate() {
        let raw_text = member.text.trim();
        // Auto-insert divider before the first operation when no explicit divider exists (fix #468)
        if auto_divider == Some(midx) {
            let div_y = my - 8;
            out.push_str(&format!(
                "<line x1=\"{x}\" y1=\"{div_y}\" x2=\"{x2}\" y2=\"{div_y}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
                x2 = x + w
            ));
            section_started = false;
        }
        // Detect explicit divider tokens (`--` or `..` compartment separator)
        if raw_text == "--" || raw_text == ".." {
            // Draw a horizontal divider line (fix #468)
            let div_y = my - 8;
            out.push_str(&format!(
                "<line x1=\"{x}\" y1=\"{div_y}\" x2=\"{x2}\" y2=\"{div_y}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
                x2 = x + w
            ));
            section_started = false;
            continue;
        }
        // Skip blank display lines
        if raw_text.is_empty() {
            my += 16;
            continue;
        }
        let _ = section_started;
        section_started = true;
        let (vis_sym, vis_color, rest_after_vis) = parse_visibility_member(raw_text);
        let (base_style, text_after_mod) = parse_member_modifiers(rest_after_vis);
        let mut style_attrs = String::from(base_style);
        match &member.modifier {
            Some(MemberModifier::Abstract) | Some(MemberModifier::Field) => {
                if !style_attrs.contains("font-style") {
                    style_attrs.push_str(" font-style=\"italic\"");
                }
            }
            Some(MemberModifier::Static) => {
                if !style_attrs.contains("text-decoration") {
                    style_attrs.push_str(" text-decoration=\"underline\"");
                }
            }
            Some(MemberModifier::Method) | None => {}
        }
        // If no explicit visibility color, fall back to member_color from style
        let effective_color = if vis_sym.is_some() {
            vis_color
        } else {
            class_style.member_color.as_str()
        };
        // Reconstruct display text: keep visibility prefix + remaining text
        let display_text = if vis_sym.is_some() {
            format!("{}{}", vis_sym.unwrap_or(""), text_after_mod)
        } else {
            text_after_mod.to_string()
        };
        let visibility_attr = vis_sym
            .map(uml_visibility_name)
            .map(|name| format!(" data-uml-visibility=\"{name}\""))
            .unwrap_or_default();
        let modifier_attr = member_modifier_name(member.modifier.as_ref())
            .map(|name| format!(" data-uml-modifier=\"{name}\""))
            .unwrap_or_default();
        out.push_str(&format!(
            "<text class=\"uml-member\"{visibility_attr}{modifier_attr} x=\"{tx}\" y=\"{my}\" font-family=\"{ff}\" font-size=\"{fs}\" fill=\"{vc}\"{sa}>{m}</text>",
            ff = escape_text(font_family),
            fs = member_font_size,
            tx = x + 10,
            vc = effective_color,
            sa = style_attrs,
            m = escape_text(&display_text)
        ));
        my += 16;
    }
}

/// Ensure C4 and Actor nodes have enough minimum height to render their visual elements.
fn c4_node_height(kind: FamilyNodeKind, computed: i32) -> i32 {
    match kind {
        // Person nodes need space for stick figure (44px) + body rect (≥50px)
        FamilyNodeKind::C4Person | FamilyNodeKind::C4PersonExt => computed.max(94),
        // All other C4 nodes need at least 60px for the label + type label
        k if is_c4_kind(k) => computed.max(60),
        // Usecase actor: stick figure (≈46px) + name label (≈18px) = 64px minimum
        FamilyNodeKind::Actor => computed.max(64),
        _ => computed,
    }
}

/// Returns true if the kind belongs to the C4 family.
fn is_c4_kind(kind: FamilyNodeKind) -> bool {
    matches!(
        kind,
        FamilyNodeKind::C4Person
            | FamilyNodeKind::C4PersonExt
            | FamilyNodeKind::C4System
            | FamilyNodeKind::C4SystemExt
            | FamilyNodeKind::C4SystemDb
            | FamilyNodeKind::C4SystemQueue
            | FamilyNodeKind::C4Container
            | FamilyNodeKind::C4ContainerExt
            | FamilyNodeKind::C4ContainerDb
            | FamilyNodeKind::C4ContainerQueue
            | FamilyNodeKind::C4Component
            | FamilyNodeKind::C4ComponentExt
            | FamilyNodeKind::C4ComponentDb
            | FamilyNodeKind::C4ComponentQueue
            | FamilyNodeKind::C4Boundary
    )
}

/// Render a C4 architecture node with proper visual style.
///
/// Color conventions (following C4-PlantUML):
///   Person / Person_Ext   — person shape (stick figure above rounded rect)
///   System / *Ext         — saturated blue / gray rounded rect
///   Container             — blue rect with `[Container]` sub-label
///   Component             — lighter blue
///   *Db                   — cylinder (database icon)
///   *Queue                — open-ended cylinder
///   Boundary              — dashed rounded border
fn render_c4_node(
    out: &mut String,
    node: &crate::model::FamilyNode,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) {
    let cx = x + w / 2;
    let is_person = matches!(
        node.kind,
        FamilyNodeKind::C4Person | FamilyNodeKind::C4PersonExt
    );
    let is_db = matches!(
        node.kind,
        FamilyNodeKind::C4SystemDb | FamilyNodeKind::C4ContainerDb | FamilyNodeKind::C4ComponentDb
    );
    let is_queue = matches!(
        node.kind,
        FamilyNodeKind::C4SystemQueue
            | FamilyNodeKind::C4ContainerQueue
            | FamilyNodeKind::C4ComponentQueue
    );
    let is_boundary = matches!(node.kind, FamilyNodeKind::C4Boundary);
    let is_ext = matches!(
        node.kind,
        FamilyNodeKind::C4PersonExt
            | FamilyNodeKind::C4SystemExt
            | FamilyNodeKind::C4ContainerExt
            | FamilyNodeKind::C4ComponentExt
    );

    // Color palette
    let (fill, stroke, text_color) = if is_boundary {
        ("none", "#444444", "#444444")
    } else if is_ext {
        ("#8a8a8a", "#6b6b6b", "#ffffff")
    } else if matches!(
        node.kind,
        FamilyNodeKind::C4Component
            | FamilyNodeKind::C4ComponentDb
            | FamilyNodeKind::C4ComponentQueue
    ) {
        ("#85bbf0", "#5d82a8", "#000000")
    } else if matches!(
        node.kind,
        FamilyNodeKind::C4Container
            | FamilyNodeKind::C4ContainerDb
            | FamilyNodeKind::C4ContainerQueue
    ) {
        ("#438dd5", "#2e6da0", "#ffffff")
    } else {
        // Person, System, SystemDb, SystemQueue
        ("#1168bd", "#0d4f8f", "#ffffff")
    };

    let body_y = if is_person { y + 44 } else { y };
    let body_h = if is_person { h - 44 } else { h };
    let _ = body_h;

    // Boundary: just a dashed rounded rect
    if is_boundary {
        out.push_str(&format!(
            "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"12\" ry=\"12\" \
             fill=\"none\" stroke=\"{stroke}\" stroke-width=\"2\" stroke-dasharray=\"8 4\"/>",
        ));
        out.push_str(&format!(
            "<text x=\"{cx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" \
             font-size=\"13\" font-weight=\"600\" fill=\"{stroke}\">{name}</text>",
            ty = y + 18,
            name = escape_text(&node.name)
        ));
        return;
    }

    // Person: stick figure above a rounded rect
    if is_person {
        // Draw figure above body
        let head_cx = cx;
        let head_cy = y + 10;
        // Head circle
        out.push_str(&format!(
            "<circle cx=\"{head_cx}\" cy=\"{head_cy}\" r=\"9\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
        ));
        // Body line
        out.push_str(&format!(
            "<line x1=\"{head_cx}\" y1=\"{by}\" x2=\"{head_cx}\" y2=\"{body_line_end}\" stroke=\"{stroke}\" stroke-width=\"2\"/>",
            by = head_cy + 9,
            body_line_end = head_cy + 22
        ));
        // Arms
        out.push_str(&format!(
            "<line x1=\"{ax1}\" y1=\"{ay}\" x2=\"{ax2}\" y2=\"{ay}\" stroke=\"{stroke}\" stroke-width=\"2\"/>",
            ax1 = head_cx - 12,
            ay = head_cy + 16,
            ax2 = head_cx + 12
        ));
        // Legs
        out.push_str(&format!(
            "<line x1=\"{head_cx}\" y1=\"{ly}\" x2=\"{lx2}\" y2=\"{ley}\" stroke=\"{stroke}\" stroke-width=\"2\"/>",
            ly = head_cy + 22,
            lx2 = head_cx - 10,
            ley = head_cy + 34
        ));
        out.push_str(&format!(
            "<line x1=\"{head_cx}\" y1=\"{ly}\" x2=\"{lx2}\" y2=\"{ley}\" stroke=\"{stroke}\" stroke-width=\"2\"/>",
            ly = head_cy + 22,
            lx2 = head_cx + 10,
            ley = head_cy + 34
        ));
    }

    // Database / cylinder shape
    if is_db {
        let ell_ry = 8i32;
        let rect_y = body_y + ell_ry;
        let rect_h = h - ell_ry * 2;
        // cylinder body
        out.push_str(&format!(
            "<rect x=\"{x}\" y=\"{rect_y}\" width=\"{w}\" height=\"{rect_h}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
        ));
        // top ellipse
        out.push_str(&format!(
            "<ellipse cx=\"{cx}\" cy=\"{rect_y}\" rx=\"{rx}\" ry=\"{ell_ry}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
            rx = w / 2
        ));
        // bottom ellipse
        out.push_str(&format!(
            "<ellipse cx=\"{cx}\" cy=\"{bot}\" rx=\"{rx}\" ry=\"{ell_ry}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
            bot = rect_y + rect_h,
            rx = w / 2
        ));
        // label
        out.push_str(&format!(
            "<text x=\"{cx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" \
             font-size=\"13\" font-weight=\"600\" fill=\"{text_color}\">{name}</text>",
            ty = rect_y + rect_h / 2 + 4,
            name = escape_text(&node.name)
        ));
        c4_sublabel(out, cx, rect_y + rect_h / 2 + 18, node, text_color);
        return;
    }

    // Queue: open-ended cylinder
    if is_queue {
        let ell_ry = 8i32;
        let rect_x = x + ell_ry;
        let rect_w = w - ell_ry * 2;
        let cy_mid = body_y + h / 2;
        // left open end (half-ellipse)
        out.push_str(&format!(
            "<path d=\"M{rect_x},{top} A{ell_ry},{ell_ry} 0 0 0 {rect_x},{bot}\" \
             fill=\"none\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
            top = body_y,
            bot = body_y + h
        ));
        // right closed end
        out.push_str(&format!(
            "<ellipse cx=\"{rx_cx}\" cy=\"{cy_mid}\" rx=\"{ell_ry}\" ry=\"{ry}\" \
             fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
            rx_cx = rect_x + rect_w,
            ry = h / 2
        ));
        // body rect
        out.push_str(&format!(
            "<rect x=\"{rect_x}\" y=\"{body_y}\" width=\"{rect_w}\" height=\"{h}\" \
             fill=\"{fill}\" stroke=\"none\"/>",
        ));
        // top/bottom lines
        out.push_str(&format!(
            "<line x1=\"{rect_x}\" y1=\"{top}\" x2=\"{rx_end}\" y2=\"{top}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
            top = body_y,
            rx_end = rect_x + rect_w
        ));
        out.push_str(&format!(
            "<line x1=\"{rect_x}\" y1=\"{bot}\" x2=\"{rx_end}\" y2=\"{bot}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
            bot = body_y + h,
            rx_end = rect_x + rect_w
        ));
        out.push_str(&format!(
            "<text x=\"{cx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" \
             font-size=\"13\" font-weight=\"600\" fill=\"{text_color}\">{name}</text>",
            ty = cy_mid + 4,
            name = escape_text(&node.name)
        ));
        c4_sublabel(out, cx, cy_mid + 18, node, text_color);
        return;
    }

    // Standard rounded rect (Person body, System, Container, Component)
    out.push_str(&format!(
        "<rect x=\"{x}\" y=\"{body_y}\" width=\"{w}\" height=\"{rect_h}\" rx=\"8\" ry=\"8\" \
         fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
        rect_h = h - (if is_person { 44 } else { 0 })
    ));

    // Type label line (e.g. "[Person]", "[System]", "[Container]")
    let type_label = c4_type_label(node.kind);
    let name_y = body_y + (if is_person { 24 } else { h / 2 - 4 });
    out.push_str(&format!(
        "<text x=\"{cx}\" y=\"{name_y}\" text-anchor=\"middle\" font-family=\"monospace\" \
         font-size=\"13\" font-weight=\"600\" fill=\"{text_color}\">{name}</text>",
        name = escape_text(&node.name)
    ));
    // Sub-label: [Type]
    out.push_str(&format!(
        "<text x=\"{cx}\" y=\"{sub_y}\" text-anchor=\"middle\" font-family=\"monospace\" \
         font-size=\"10\" fill=\"{text_color}\">{type_label}</text>",
        sub_y = name_y + 14
    ));
    // Description (from members[0] if any, shown as italic)
    if let Some(desc) = node.members.first() {
        out.push_str(&format!(
            "<text x=\"{cx}\" y=\"{desc_y}\" text-anchor=\"middle\" font-family=\"monospace\" \
             font-size=\"9\" font-style=\"italic\" fill=\"{text_color}\">{desc}</text>",
            desc_y = name_y + 26,
            desc = escape_text(&desc.text)
        ));
    }
}

/// Return the `[Type]` sub-label for a C4 kind.
fn c4_type_label(kind: FamilyNodeKind) -> &'static str {
    match kind {
        FamilyNodeKind::C4Person => "[Person]",
        FamilyNodeKind::C4PersonExt => "[Person, ext]",
        FamilyNodeKind::C4System => "[System]",
        FamilyNodeKind::C4SystemExt => "[System, ext]",
        FamilyNodeKind::C4SystemDb => "[Database]",
        FamilyNodeKind::C4SystemQueue => "[Queue]",
        FamilyNodeKind::C4Container => "[Container]",
        FamilyNodeKind::C4ContainerExt => "[Container, ext]",
        FamilyNodeKind::C4ContainerDb => "[Database]",
        FamilyNodeKind::C4ContainerQueue => "[Queue]",
        FamilyNodeKind::C4Component => "[Component]",
        FamilyNodeKind::C4ComponentExt => "[Component, ext]",
        FamilyNodeKind::C4ComponentDb => "[Database]",
        FamilyNodeKind::C4ComponentQueue => "[Queue]",
        FamilyNodeKind::C4Boundary => "[Boundary]",
        _ => "",
    }
}

/// Render a small italic sub-label beneath the main name for C4 nodes.
fn c4_sublabel(out: &mut String, cx: i32, y: i32, node: &crate::model::FamilyNode, color: &str) {
    let type_label = c4_type_label(node.kind);
    out.push_str(&format!(
        "<text x=\"{cx}\" y=\"{y}\" text-anchor=\"middle\" font-family=\"monospace\" \
         font-size=\"10\" fill=\"{color}\">{type_label}</text>",
    ));
    if let Some(desc) = node.members.first() {
        out.push_str(&format!(
            "<text x=\"{cx}\" y=\"{dy}\" text-anchor=\"middle\" font-family=\"monospace\" \
             font-size=\"9\" font-style=\"italic\" fill=\"{color}\">{desc}</text>",
            dy = y + 12,
            desc = escape_text(&desc.text)
        ));
    }
}

/// Backwards-compatible alias; delegates to the real timeline renderer.
pub fn render_component_svg(doc: &FamilyDocument) -> String {
    render_box_grid_svg(doc, "component")
}

pub fn render_deployment_svg(doc: &FamilyDocument) -> String {
    render_box_grid_svg(doc, "deployment")
}

fn render_box_grid_svg(doc: &FamilyDocument, family: &str) -> String {
    // Do NOT emit a visible "component/deployment diagram" label — it was leaking
    // as unwanted canvas text (fix #490, #494).
    let _ = family;

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
    let pkg_tab = 28i32; // height of the package label tab at top
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
    let group_top_overhead = (pkg_pad + pkg_tab) as f64; // 24 + 28 = 52px
    let gl_options = GlOptions {
        rank_separation: (cell_h + inner_gap) as f64,
        node_separation: inner_gap as f64,
        group_padding: pkg_pad as f64,
        direction: crate::render::graph_layout::Direction::TopDown,
        canvas_margin: canvas_margin as f64 + header_h as f64 + group_top_overhead,
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

    // Also account for ungrouped nodes
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
    let svg_width = all_pkg_right.max(gl_canvas_right).max(canvas_margin) + canvas_margin;
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

        // Tab rectangle (label background) — use scope for data-uml-group
        let tab_w = ((pkg.label.len() as i32) * 8 + 16).max(60).min(fw);
        out.push_str(&format!(
            "<rect class=\"uml-group-frame\" data-uml-group=\"{}\" x=\"{fx}\" y=\"{fy}\" width=\"{fw}\" height=\"{fh}\" rx=\"8\" ry=\"8\" fill=\"#f8faff\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            escape_text(&pkg.scope),
            comp_style.border_color
        ));
        // Tab label background (small header rectangle at top-left)
        out.push_str(&format!(
            "<rect x=\"{fx}\" y=\"{fy}\" width=\"{tab_w}\" height=\"{}\" rx=\"8\" ry=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            pkg_tab,
            comp_style.border_color,
            comp_style.border_color
        ));
        // Flatten bottom corners of tab (cover the rounded bottom)
        out.push_str(&format!(
            "<rect x=\"{fx}\" y=\"{}\" width=\"{tab_w}\" height=\"8\" fill=\"{}\" stroke=\"none\"/>",
            fy + pkg_tab - 8,
            comp_style.border_color
        ));
        // Package label text in the tab (display label includes kind prefix)
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" font-weight=\"600\" fill=\"#ffffff\">{}</text>",
            fx + 8,
            fy + pkg_tab - 8,
            escape_text(&pkg.label)
        ));
        // Horizontal separator line between tab and content area
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
            out.push_str(&format!(
                "<rect class=\"uml-group-frame\" data-uml-group=\"{}\" x=\"{fx}\" y=\"{fy}\" width=\"{fw}\" height=\"{fh}\" rx=\"6\" ry=\"6\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\" stroke-dasharray=\"4 3\"/>",
                escape_text(&frame.scope),
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
    // Phase 2: Draw relations with L-shape routing on collision
    // Phase 3: Collect edge label positions, then de-collide at the end
    // ─────────────────────────────────────────────────────────────────────────
    struct PendingLabel {
        x: i32,
        y: i32,
        text: String,
        color: String,
    }
    let mut pending_labels: Vec<PendingLabel> = Vec::new();

    for (rel_idx, rel) in doc.relations.iter().enumerate() {
        let (from_name, to_name, normalized_arrow) =
            normalize_relation_endpoints(&rel.from, &rel.to, &rel.arrow);
        let from_box = positions.get(&from_name);
        let to_box = positions.get(&to_name);
        let (Some(&(fx, fy, fw, fh)), Some(&(tx, ty, tw, th))) = (from_box, to_box) else {
            continue;
        };

        // Compute anchor points (edge of box, not center).
        // Port-based anchoring: attach to mid-point of the nearest box edge
        // (left/right for horizontal-dominant, top/bottom for vertical-dominant).
        // Part of the layout engine refactor (#591, #590 epic).
        let (x1, y1, x2, y2) = if rel.direction.is_some() {
            compute_edge_anchors_for_direction(
                (fx, fy, fw, fh),
                (tx, ty, tw, th),
                rel.direction.as_deref(),
            )
        } else {
            pick_port((fx, fy, fw, fh), (tx, ty, tw, th))
        };

        let style = arrow_style(&normalized_arrow);
        let relation_color = rel.line_color.as_deref().unwrap_or(&comp_style.arrow_color);
        let marker_prefix = if rel.line_color.is_some() && relation_color != comp_style.arrow_color
        {
            let prefix = format!("uml-rel-{rel_idx}-");
            render_relation_marker_defs_with_prefix(&mut out, relation_color, &prefix);
            prefix
        } else {
            String::new()
        };
        let stroke_width = rel.thickness.unwrap_or(2).clamp(1, 8);
        let dash_attr = if style.dashed || rel.dashed {
            " stroke-dasharray=\"5 3\""
        } else {
            ""
        };
        let visibility_attr = if rel.hidden {
            " visibility=\"hidden\""
        } else {
            ""
        };
        let mut markers = String::new();
        if let Some(end) = style.end_marker {
            markers.push_str(&format!(" marker-end=\"url(#{marker_prefix}{end})\""));
        }
        if let Some(start) = style.start_marker {
            markers.push_str(&format!(" marker-start=\"url(#{marker_prefix}{start})\""));
        }
        let direction_attr = rel
            .direction
            .as_deref()
            .map(|d| format!(" data-uml-direction=\"{}\"", escape_text(d)))
            .unwrap_or_default();

        // Build data-uml-relation-style metadata (same as old renderer)
        let style_attr = {
            let mut tokens: Vec<String> = Vec::new();
            if rel.line_color.is_some() {
                tokens.push(format!("color:{relation_color}"));
            }
            if style.dashed || rel.dashed {
                tokens.push("dashed".to_string());
            }
            if rel.hidden {
                tokens.push("hidden".to_string());
            }
            if rel.thickness.is_some() {
                tokens.push(format!("thickness:{stroke_width}"));
            }
            if tokens.is_empty() {
                String::new()
            } else {
                format!(
                    " data-uml-relation-style=\"{}\"",
                    escape_text(&tokens.join(" "))
                )
            }
        };

        // ── Phase 2: Collision check & L-shape routing ──────────────────────
        // Skip L-routing when a direction is explicitly specified (directed
        // relations must stay straight to preserve test-verified geometry).
        // Also skip when the relation is hidden (direction doesn't matter).

        // Build a combined obstacle list: individual node boxes + package frames
        // that do NOT contain the source or target node.
        let rel_obstacles: Vec<(i32, i32, i32, i32)> = {
            let mut obs: Vec<(i32, i32, i32, i32)> = all_boxes.clone();
            for &(rect, members) in &pkg_frame_boxes {
                // A package frame is not an obstacle for arrows that start/end inside it
                let src_inside = members.iter().any(|m| m == &from_name);
                let tgt_inside = members.iter().any(|m| m == &to_name);
                if !src_inside && !tgt_inside {
                    obs.push(rect);
                }
            }
            obs
        };

        let line_collides = if rel.direction.is_some() || rel.hidden {
            false
        } else {
            // Check if the direct line (x1,y1)→(x2,y2) passes through any individual
            // component box. Package frames are NOT used for the straight-line trigger
            // (they're only used to improve L/Z route quality when routing is needed).
            all_boxes.iter().any(|&(bx, by, bw, bh)| {
                if (bx, by, bw, bh) == (fx, fy, fw, fh) || (bx, by, bw, bh) == (tx, ty, tw, th) {
                    return false;
                }
                segment_intersects_rect(x1, y1, x2, y2, (bx, by, bw, bh))
            })
        };

        // Label midpoint
        let label_color = "#1e293b";
        let (label_mx, label_my);

        if !line_collides {
            // Straight line
            out.push_str(&format!(
                "<line class=\"uml-relation\" data-uml-from=\"{}\" data-uml-to=\"{}\" data-uml-arrow=\"{}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}{}{}{}{} />",
                escape_text(&from_name),
                escape_text(&to_name),
                escape_text(&normalized_arrow),
                x1, y1, x2, y2,
                relation_color, stroke_width,
                dash_attr, visibility_attr, direction_attr, style_attr, markers
            ));
            label_mx = (x1 + x2) / 2;
            label_my = (y1 + y2) / 2 - 12;
        } else {
            // L-shape / Z-shape routing
            // Strategy: try L first (2 segments); if still collides try Z-shapes;
            // cap at 5 segments, fall back to best L if nothing cleans up.
            let src_cx = fx + fw / 2;
            let tgt_cx = tx + tw / 2;
            let src_cy = fy + fh / 2;
            let tgt_cy = ty + th / 2;
            let dx_abs = (tgt_cx - src_cx).abs();
            let dy_abs = (tgt_cy - src_cy).abs();

            // ── Two L-shape candidates ────────────────────────────────────────
            // H→V: go horizontal to mid-x, then vertical
            let hv_mid_x = (x1 + x2) / 2;
            let hv_pts = [(x1, y1), (hv_mid_x, y1), (hv_mid_x, y2), (x2, y2)];
            // V→H: go vertical to mid-y, then horizontal
            let vh_mid_y = (y1 + y2) / 2;
            let vh_pts = [(x1, y1), (x1, vh_mid_y), (x2, vh_mid_y), (x2, y2)];

            let hv_col = count_polyline_collisions(
                &hv_pts,
                &rel_obstacles,
                (fx, fy, fw, fh),
                (tx, ty, tw, th),
            );
            let vh_col = count_polyline_collisions(
                &vh_pts,
                &rel_obstacles,
                (fx, fy, fw, fh),
                (tx, ty, tw, th),
            );

            // Pick the preferred L-shape
            let (l_pts, l_col) = if dx_abs >= dy_abs {
                if vh_col <= hv_col {
                    (&vh_pts[..], vh_col)
                } else {
                    (&hv_pts[..], hv_col)
                }
            } else {
                if hv_col <= vh_col {
                    (&hv_pts[..], hv_col)
                } else {
                    (&vh_pts[..], vh_col)
                }
            };

            // ── Z-shape escalation if L still collides ────────────────────────
            // Gather all blocking boxes from rel_obstacles (including package frames)
            let blocking: Vec<(i32, i32, i32, i32)> = if l_col > 0 {
                rel_obstacles
                    .iter()
                    .copied()
                    .filter(|&b| {
                        b != (fx, fy, fw, fh)
                            && b != (tx, ty, tw, th)
                            && l_pts.windows(2).any(|seg| {
                                segment_intersects_rect(
                                    seg[0].0,
                                    seg[0].1,
                                    seg[1].0,
                                    seg[1].1,
                                    (b.0, b.1, b.2, b.3),
                                )
                            })
                    })
                    .collect()
            } else {
                Vec::new()
            };

            let best_pts: Vec<(i32, i32)> = if l_col == 0 || blocking.is_empty() {
                // L is clean — use it
                l_pts.to_vec()
            } else {
                // Try Z-routes by generating waypoints around every blocking box.
                // Use clearance = 12px outside the box edge.
                let gap = 12i32;
                let mut best: Option<Vec<(i32, i32)>> = None;
                let mut best_col = l_col;

                // Generate candidate waypoints: edges of every blocking box
                let mut waypoint_candidates: Vec<(i32, i32)> = Vec::new();
                for &(bx, by, bw, bh) in &blocking {
                    waypoint_candidates.push((bx + bw / 2, by - gap)); // above
                    waypoint_candidates.push((bx + bw / 2, by + bh + gap)); // below
                    waypoint_candidates.push((bx - gap, by + bh / 2)); // left
                    waypoint_candidates.push((bx + bw + gap, by + bh / 2)); // right
                                                                            // Also try corners (useful for routing around package frames)
                    waypoint_candidates.push((bx - gap, by - gap));
                    waypoint_candidates.push((bx + bw + gap, by - gap));
                    waypoint_candidates.push((bx - gap, by + bh + gap));
                    waypoint_candidates.push((bx + bw + gap, by + bh + gap));
                }

                'waypoint_loop: for &(wx, wy) in &waypoint_candidates {
                    // Z1: H→V→H  (x1,y1)→(wx,y1)→(wx,y2)→(x2,y2)
                    let z1: Vec<(i32, i32)> = vec![(x1, y1), (wx, y1), (wx, y2), (x2, y2)];
                    // Z2: V→H→V  (x1,y1)→(x1,wy)→(x2,wy)→(x2,y2)
                    let z2: Vec<(i32, i32)> = vec![(x1, y1), (x1, wy), (x2, wy), (x2, y2)];
                    // Z3: 5-seg H→V→H with waypoint intermediate
                    let z3: Vec<(i32, i32)> =
                        vec![(x1, y1), (wx, y1), (wx, wy), (x2, wy), (x2, y2)];
                    // Z4: 5-seg V→H→V with waypoint intermediate
                    let z4: Vec<(i32, i32)> =
                        vec![(x1, y1), (x1, wy), (wx, wy), (wx, y2), (x2, y2)];

                    for cand in [&z1, &z2, &z3, &z4] {
                        if cand.len() > 5 {
                            continue;
                        }
                        let c = count_polyline_collisions(
                            cand,
                            &rel_obstacles,
                            (fx, fy, fw, fh),
                            (tx, ty, tw, th),
                        );
                        if c < best_col {
                            best_col = c;
                            best = Some(cand.clone());
                            if c == 0 {
                                break 'waypoint_loop;
                            }
                        }
                    }
                }

                best.unwrap_or_else(|| l_pts.to_vec())
            };

            let pts = best_pts.as_slice();
            let pts_str: String = pts
                .iter()
                .map(|(px, py)| format!("{},{}", px, py))
                .collect::<Vec<_>>()
                .join(" ");

            out.push_str(&format!(
                "<polyline class=\"uml-relation\" data-uml-from=\"{}\" data-uml-to=\"{}\" data-uml-arrow=\"{}\" points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}{}{} />",
                escape_text(&from_name),
                escape_text(&to_name),
                escape_text(&normalized_arrow),
                pts_str,
                relation_color, stroke_width,
                dash_attr, visibility_attr, direction_attr, markers
            ));

            // Label at longest segment midpoint
            let longest_seg = pts.windows(2).max_by_key(|seg| {
                let (ax, ay) = seg[0];
                let (bx, by_) = seg[1];
                (bx - ax).pow(2) + (by_ - ay).pow(2)
            });
            let (lmx, lmy) = match longest_seg {
                Some(seg) => ((seg[0].0 + seg[1].0) / 2, (seg[0].1 + seg[1].1) / 2 - 12),
                None => ((x1 + x2) / 2, (y1 + y2) / 2 - 12),
            };
            label_mx = lmx;
            label_my = lmy;
        }

        if rel.left_lollipop {
            render_lollipop_endpoint(&mut out, x1, y1, relation_color);
        }
        if rel.right_lollipop {
            render_lollipop_endpoint(&mut out, x2, y2, relation_color);
        }
        if let Some(stereotype) = &rel.stereotype {
            let sx = (x1 + x2) / 2;
            let sy = (y1 + y2) / 2 - if rel.label.is_some() { 20 } else { 6 };
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"#475569\">&lt;&lt;{}&gt;&gt;</text>",
                sx, sy, escape_text(stereotype)
            ));
        }
        if let Some(label) = &rel.label {
            let label_text = usecase_dependency_label(Some(label)).unwrap_or(label);
            pending_labels.push(PendingLabel {
                x: label_mx,
                y: label_my,
                text: label_text.to_string(),
                color: label_color.to_string(),
            });
        }
        if let Some(left) = &rel.left_cardinality {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"10\" fill=\"#334155\">{}</text>",
                x1 - 4, y1 - 6, escape_text(left)
            ));
        }
        if let Some(right) = &rel.right_cardinality {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"start\" font-family=\"monospace\" font-size=\"10\" fill=\"#334155\">{}</text>",
                x2 + 4, y2 - 6, escape_text(right)
            ));
        }
        if let Some(left_role) = &rel.left_role {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"10\" fill=\"#334155\">{}</text>",
                x1 - 4, y1 + 12, escape_text(left_role)
            ));
        }
        if let Some(right_role) = &rel.right_role {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"start\" font-family=\"monospace\" font-size=\"10\" fill=\"#334155\">{}</text>",
                x2 + 4, y2 + 12, escape_text(right_role)
            ));
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Phase 3: De-collide edge labels
    // ─────────────────────────────────────────────────────────────────────────
    // Group labels whose initial centers are within 24px Manhattan distance,
    // then fan them out symmetrically (vertically first, horizontal stagger
    // as a secondary de-overlap pass).
    let cluster_dist = 24i32;
    let v_stride = 18i32; // vertical fan step per label within a group
    let h_stagger = 20i32; // horizontal shift applied when still overlapping after v-fan

    // Sort pending labels by (y, x) so groups form deterministically.
    let mut sorted_labels = pending_labels;
    sorted_labels.sort_by_key(|l| (l.y, l.x));

    // Assign each label to a group (simple greedy union-find by proximity).
    // group_id[i] = index of the earliest label in the same cluster.
    let n = sorted_labels.len();
    let mut group_id: Vec<usize> = (0..n).collect();
    for i in 0..n {
        for j in 0..i {
            let dist = (sorted_labels[i].x - sorted_labels[j].x).abs()
                + (sorted_labels[i].y - sorted_labels[j].y).abs();
            if dist < cluster_dist {
                // Merge: point i to the root of j's group
                let root = group_id[j];
                for k in 0..=i {
                    if group_id[k] == group_id[i] {
                        group_id[k] = root;
                    }
                }
            }
        }
    }

    // Collect groups: map root → list of label indices
    let mut groups: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
    for (i, &root) in group_id.iter().enumerate() {
        groups.entry(root).or_default().push(i);
    }

    // Compute adjusted positions for each label
    let mut adjusted_labels: Vec<(i32, i32, String, String)> =
        vec![(0, 0, String::new(), String::new()); n];
    for indices in groups.values() {
        let count = indices.len() as i32;
        // Base position: centroid of the group
        let base_x: i32 = indices.iter().map(|&i| sorted_labels[i].x).sum::<i32>() / count;
        let base_y: i32 = indices.iter().map(|&i| sorted_labels[i].y).sum::<i32>() / count;

        for (rank, &idx) in indices.iter().enumerate() {
            let rank = rank as i32;
            // Symmetric vertical fan: rank 0 → -(N-1)/2 * stride, etc.
            let dy = v_stride * (rank - (count - 1) / 2);
            // Secondary horizontal stagger for even/odd when N ≥ 3
            let dx = if count >= 3 && (rank % 2 == 1) {
                h_stagger
            } else {
                0
            };
            adjusted_labels[idx] = (
                base_x + dx,
                base_y + dy,
                sorted_labels[idx].text.clone(),
                sorted_labels[idx].color.clone(),
            );
        }
    }

    // Emit the final labels
    for (lx, ly, text, color) in &adjusted_labels {
        if text.is_empty() {
            continue;
        }
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
            lx, ly, escape_text(color), escape_text(text)
        ));
    }

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
fn segment_intersects_rect(
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
fn count_polyline_collisions(
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

#[derive(Debug, Clone)]
struct NodeLayout {
    label_lines: Vec<String>,
    width: i32,
    height: i32,
    x: i32,
    y: i32,
}

fn wrap_text(
    text: String,
    max_chars: usize,
    policy: crate::scene::TextOverflowPolicy,
) -> Vec<String> {
    match policy {
        crate::scene::TextOverflowPolicy::EllipsisSingleLine => {
            let one_line = text.replace('\n', " ");
            vec![ellipsize(one_line, max_chars)]
        }
        crate::scene::TextOverflowPolicy::WrapAndGrow => text
            .lines()
            .flat_map(|line| wrap_line(line, max_chars))
            .collect::<Vec<_>>(),
    }
}

fn render_tree_arrow(out: &mut String, x1: i32, y1: i32, x2: i32, y2: i32, color: &str) {
    let size = 6;
    if x2 >= x1 && y1 == y2 {
        out.push_str(&format!(
            "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\"/>",
            x2,
            y2,
            x2 - size,
            y2 - size,
            x2 - size,
            y2 + size,
            color
        ));
        return;
    }

    if x1 == x2 && y2 >= y1 {
        out.push_str(&format!(
            "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\"/>",
            x2,
            y2,
            x2 - size,
            y2 - size,
            x2 + size,
            y2 - size,
            color
        ));
        return;
    }

    if x2 >= x1 {
        out.push_str(&format!(
            "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\"/>",
            x2,
            y2,
            x2 - size,
            y2 - size,
            x2 - size,
            y2 + size,
            color
        ));
        return;
    }

    if x1 > x2 {
        out.push_str(&format!(
            "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\"/>",
            x2,
            y2,
            x2 + size,
            y2 - size,
            x2 + size,
            y2 + size,
            color
        ));
    }
}

fn wrap_line(line: &str, max_chars: usize) -> Vec<String> {
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
                for chunk in chunk_text(word, max_chars) {
                    lines.push(chunk);
                }
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
                let mut chunks = chunk_text(word, max_chars);
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

fn chunk_text(text: &str, max_chars: usize) -> Vec<String> {
    if max_chars == 0 {
        return vec![text.to_string()];
    }
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

fn ellipsize(text: String, max_chars: usize) -> String {
    if max_chars == 0 {
        return "...".to_string();
    }

    let count = text.chars().count();
    if count <= max_chars {
        return text;
    }

    if max_chars <= 3 {
        return "...".to_string();
    }

    text.chars().take(max_chars - 3).collect::<String>() + "..."
}

fn render_family_node_shape(out: &mut String, node: &FamilyNode, x: i32, y: i32, w: i32, h: i32) {
    let cx = x + w / 2;
    let cy = y + h / 2;
    let display = node.label.clone().unwrap_or_else(|| node.name.clone());
    let kind_label = family_node_label(node.kind);
    out.push_str(&format!(
        "<desc data-uml-id=\"{}\">{}</desc>",
        escape_text(&node.name),
        escape_text(&node.name)
    ));

    match node.kind {
        FamilyNodeKind::Interface => {
            // small circle interface
            let r = 18;
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"#f1f5f9\" stroke=\"#0f172a\" stroke-width=\"1.5\"/>",
                cx, cy, r
            ));
        }
        FamilyNodeKind::Component => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" ry=\"4\" fill=\"#fefce8\" stroke=\"#a16207\" stroke-width=\"1\"/>",
                x, y, w, h
            ));
            // component badges (two small rectangles on the left edge)
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"16\" height=\"8\" fill=\"#fefce8\" stroke=\"#a16207\" stroke-width=\"1\"/>",
                x - 4,
                y + 12
            ));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"16\" height=\"8\" fill=\"#fefce8\" stroke=\"#a16207\" stroke-width=\"1\"/>",
                x - 4,
                y + h - 20
            ));
        }
        FamilyNodeKind::Node | FamilyNodeKind::Frame => {
            // 3D-ish prism: outer rect with offset shadow
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#eef2ff\" stroke=\"#3730a3\" stroke-width=\"1\"/>",
                x + 6,
                y + 6,
                w,
                h
            ));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#ffffff\" stroke=\"#3730a3\" stroke-width=\"1.5\"/>",
                x, y, w, h
            ));
        }
        FamilyNodeKind::Cloud => {
            // cloud-ish: rounded with several arcs
            out.push_str(&format!(
                "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"#f0f9ff\" stroke=\"#0369a1\" stroke-width=\"1.5\"/>",
                cx,
                cy,
                w / 2 - 4,
                h / 2 - 4
            ));
        }
        FamilyNodeKind::Database => {
            // database cylinder
            out.push_str(&format!(
                "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"8\" fill=\"#ecfeff\" stroke=\"#0e7490\" stroke-width=\"1.5\"/>",
                cx,
                y + 10,
                w / 2 - 6
            ));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#ecfeff\" stroke=\"#0e7490\" stroke-width=\"1.5\"/>",
                x + 6,
                y + 10,
                w - 12,
                h - 20
            ));
            out.push_str(&format!(
                "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"8\" fill=\"#ecfeff\" stroke=\"#0e7490\" stroke-width=\"1.5\"/>",
                cx,
                y + h - 10,
                w / 2 - 6
            ));
        }
        FamilyNodeKind::Artifact | FamilyNodeKind::File => {
            // folded-corner rectangle
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{} {},{} {},{}\" fill=\"#fff7ed\" stroke=\"#9a3412\" stroke-width=\"1.5\"/>",
                x,
                y,
                x + w - 18,
                y,
                x + w,
                y + 18,
                x + w,
                y + h,
                x,
                y + h
            ));
        }
        FamilyNodeKind::Folder | FamilyNodeKind::Package => {
            let fill = node.fill_color.as_deref().unwrap_or("#fef3c7");
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"60\" height=\"14\" fill=\"{}\" stroke=\"#92400e\" stroke-width=\"1\"/>",
                x, y, fill
            ));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"#92400e\" stroke-width=\"1\"/>",
                x,
                y + 14,
                w,
                h - 14,
                fill
            ));
        }
        FamilyNodeKind::Storage => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"16\" ry=\"16\" fill=\"#fff1f2\" stroke=\"#9f1239\" stroke-width=\"1.5\"/>",
                x, y, w, h
            ));
        }
        FamilyNodeKind::Rectangle
        | FamilyNodeKind::Card
        | FamilyNodeKind::Actor
        | FamilyNodeKind::Port => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" ry=\"6\" fill=\"#f8fafc\" stroke=\"#475569\" stroke-width=\"1\"/>",
                x, y, w, h
            ));
        }
        FamilyNodeKind::State => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"14\" ry=\"14\" fill=\"#ecfccb\" stroke=\"#3f6212\" stroke-width=\"1.5\"/>",
                x, y, w, h
            ));
        }
        FamilyNodeKind::StateInitial => {
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"10\" fill=\"#0f172a\"/>",
                cx, cy
            ));
        }
        FamilyNodeKind::StateFinal => {
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"12\" fill=\"#ffffff\" stroke=\"#0f172a\" stroke-width=\"1.5\"/>",
                cx, cy
            ));
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"6\" fill=\"#0f172a\"/>",
                cx, cy
            ));
        }
        FamilyNodeKind::StateHistory => {
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"14\" fill=\"#fef3c7\" stroke=\"#92400e\" stroke-width=\"1.5\"/>",
                cx, cy
            ));
        }
        FamilyNodeKind::Note => {
            render_note_card(out, x, y, w, h, &display);
            return;
        }
        FamilyNodeKind::Class | FamilyNodeKind::Object | FamilyNodeKind::UseCase => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" ry=\"6\" fill=\"#f1f5f9\" stroke=\"#64748b\" stroke-width=\"1\"/>",
                x, y, w, h
            ));
        }
        _ => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" ry=\"4\" fill=\"#f8fafc\" stroke=\"#64748b\" stroke-width=\"1\"/>",
                x, y, w, h
            ));
        }
    }

    // For interface/initial/final we render label below the marker.
    let (label_x, label_y) = match node.kind {
        FamilyNodeKind::Interface
        | FamilyNodeKind::StateInitial
        | FamilyNodeKind::StateFinal
        | FamilyNodeKind::StateHistory => (cx, cy + 28),
        _ => (cx, cy + 6),
    };
    out.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\">{}</text>",
        label_x,
        label_y,
        escape_text(&display)
    ));
    let kind_tag_y = match node.kind {
        FamilyNodeKind::Interface
        | FamilyNodeKind::StateInitial
        | FamilyNodeKind::StateFinal
        | FamilyNodeKind::StateHistory => label_y + 14,
        _ => y + 14,
    };
    // Suppress the kind-tag for package/rectangle/folder container nodes — they already
    // show their label in a visual header/tab (fix #549).
    let is_package_container = matches!(
        node.kind,
        FamilyNodeKind::Package | FamilyNodeKind::Rectangle | FamilyNodeKind::Folder
    );
    if !is_package_container {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"#475569\">{}</text>",
            cx,
            kind_tag_y,
            kind_label
        ));
    }
    render_node_stereotype_rows(out, node, cx, kind_tag_y + 13);
}

fn render_node_stereotype_rows(out: &mut String, node: &FamilyNode, cx: i32, start_y: i32) {
    for (idx, member) in node
        .members
        .iter()
        .filter(|member| {
            let text = member.text.trim();
            text.starts_with("<<") && text.ends_with(">>")
        })
        .take(4)
        .enumerate()
    {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"#64748b\">{}</text>",
            cx,
            start_y + idx as i32 * 12,
            escape_text(member.text.trim())
        ));
    }
}

pub(crate) fn render_note_card(out: &mut String, x: i32, y: i32, w: i32, h: i32, text: &str) {
    out.push_str(&format!(
        "<path d=\"M{x},{y} H{} L{} {} V{} H{x} Z\" fill=\"#fff8c4\" stroke=\"#8a6d00\" stroke-width=\"1.2\"/>",
        x + w - 16,
        x + w,
        y + 16,
        y + h
    ));
    out.push_str(&format!(
        "<path d=\"M{} {y} V{} H{}\" fill=\"none\" stroke=\"#8a6d00\" stroke-width=\"1\"/>",
        x + w - 16,
        y + 16,
        x + w
    ));
    let mut ty = y + 22;
    for line in text.lines().take(5) {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#3b2f00\">{}</text>",
            x + 10,
            ty,
            escape_text(line)
        ));
        ty += 15;
    }
}

#[derive(Debug, Clone)]
struct RenderGroupFrame {
    kind: String,
    label: Option<String>,
    scope: String,
    member_ids: Vec<String>,
    depth: usize,
}

impl RenderGroupFrame {
    fn display_label(&self) -> String {
        match self.label.as_deref() {
            Some(label) if !label.is_empty() => {
                // For boundary keywords like `rectangle` (used in usecase diagrams as
                // system-boundary frames, fix #553), the label alone is the display
                // name — the keyword is structural, not part of the visible text.
                if self.kind == "rectangle" {
                    label.to_string()
                } else {
                    format!("{} {}", self.kind, label)
                }
            }
            _ => self.kind.clone(),
        }
    }
}

fn collect_render_group_frames(groups: &[FamilyGroup]) -> Vec<RenderGroupFrame> {
    let mut frames: std::collections::BTreeMap<String, RenderGroupFrame> =
        std::collections::BTreeMap::new();

    for group in groups {
        let explicit_scope = group
            .label
            .as_deref()
            .filter(|label| !label.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| group.kind.clone());
        if !group.member_ids.is_empty() {
            let scope = explicit_scope;
            let depth = scope.split("::").filter(|part| !part.is_empty()).count();
            let key = format!("{}\x1f{}", group.kind, scope);
            let entry = frames.entry(key).or_insert_with(|| RenderGroupFrame {
                kind: group.kind.clone(),
                label: group.label.clone(),
                scope: scope.clone(),
                member_ids: Vec::new(),
                depth: depth.saturating_sub(1),
            });
            entry.member_ids.extend(group.member_ids.iter().cloned());
        }

        for member_id in &group.member_ids {
            let node_id = member_id
                .split('\t')
                .next()
                .unwrap_or(member_id.as_str())
                .trim();
            if node_id.is_empty() {
                continue;
            }
            let parts = node_id
                .split("::")
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>();
            if parts.len() < 2 {
                continue;
            }
            for prefix_len in 1..parts.len() {
                let scope = parts[..prefix_len].join("::");
                let key = format!("{}\x1f{}", group.kind, scope);
                let label = parts.get(prefix_len - 1).map(|value| (*value).to_string());
                let entry = frames.entry(key).or_insert_with(|| RenderGroupFrame {
                    kind: group.kind.clone(),
                    label,
                    scope: scope.clone(),
                    member_ids: Vec::new(),
                    depth: prefix_len.saturating_sub(1),
                });
                entry.member_ids.push(node_id.to_string());
            }
        }
    }

    let mut frames = frames.into_values().collect::<Vec<_>>();
    for frame in &mut frames {
        frame.member_ids.sort();
        frame.member_ids.dedup();
    }
    frames.sort_by(|a, b| {
        (a.depth, a.scope.as_str(), a.kind.as_str()).cmp(&(
            b.depth,
            b.scope.as_str(),
            b.kind.as_str(),
        ))
    });
    frames
}

/// Styled variant of `render_family_node_shape` that applies `comp_style` for
/// Component/Interface nodes and falls back to the default for others.
fn render_family_node_shape_styled(
    out: &mut String,
    node: &FamilyNode,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    comp_style: &ComponentStyle,
) {
    let cx = x + w / 2;
    let cy = y + h / 2;
    let display = node.label.clone().unwrap_or_else(|| node.name.clone());
    let kind_label = family_node_label(node.kind);
    out.push_str(&format!(
        "<desc data-uml-id=\"{}\">{}</desc>",
        escape_text(&node.name),
        escape_text(&node.name)
    ));

    match node.kind {
        FamilyNodeKind::Interface => {
            let r = 18;
            let fill = node
                .fill_color
                .as_deref()
                .unwrap_or(&comp_style.interface_color);
            out.push_str(&format!(
                "<circle class=\"uml-node uml-interface\" data-uml-kind=\"interface\" cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx, cy, r, fill, comp_style.border_color
            ));
        }
        FamilyNodeKind::Port => {
            let pw = 24;
            let ph = 24;
            let fill = node
                .fill_color
                .as_deref()
                .unwrap_or(&comp_style.interface_color);
            let port_dir = if node.members.iter().any(|m| m.text == "<<portin>>") {
                "in"
            } else if node.members.iter().any(|m| m.text == "<<portout>>") {
                "out"
            } else {
                ""
            };
            out.push_str(&format!(
                "<rect class=\"uml-node uml-port\" data-uml-kind=\"port\" data-uml-port-direction=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"2\" ry=\"2\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                escape_text(port_dir),
                cx - pw / 2,
                cy - ph / 2,
                pw,
                ph,
                fill,
                comp_style.border_color
            ));
        }
        FamilyNodeKind::Component => {
            let fill = node
                .fill_color
                .as_deref()
                .unwrap_or(&comp_style.background_color);
            out.push_str(&format!(
                "<rect class=\"uml-node uml-component\" data-uml-kind=\"component\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" ry=\"4\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                x, y, w, h, fill, comp_style.border_color
            ));
            // component badges (two small rectangles on the left edge)
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"16\" height=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                x - 4, y + 12, fill, comp_style.border_color
            ));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"16\" height=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                x - 4, y + h - 20, fill, comp_style.border_color
            ));
        }
        FamilyNodeKind::Node
        | FamilyNodeKind::Frame
        | FamilyNodeKind::Artifact
        | FamilyNodeKind::Cloud
        | FamilyNodeKind::Storage
        | FamilyNodeKind::Database
        | FamilyNodeKind::Package
        | FamilyNodeKind::Rectangle
        | FamilyNodeKind::Folder
        | FamilyNodeKind::File
        | FamilyNodeKind::Card
        | FamilyNodeKind::Actor => {
            let fill = node
                .fill_color
                .as_deref()
                .unwrap_or(&comp_style.background_color);
            match node.kind {
                // 3D cube for deployment nodes (fix #495)
                FamilyNodeKind::Node | FamilyNodeKind::Frame => {
                    let depth = 10i32; // 3D offset
                                       // Back face (top-right shadow)
                    out.push_str(&format!(
                        "<polygon class=\"uml-node uml-deployment-shape\" data-uml-kind=\"{}\" \
                         points=\"{},{} {},{} {},{} {},{}\" \
                         fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                        kind_label,
                        x + depth,
                        y, // top-left of back face
                        x + w + depth,
                        y, // top-right of back face
                        x + w + depth,
                        y + h, // bottom-right of back face
                        x + depth,
                        y + h, // bottom-left of back face (= right edge of front)
                        escape_text(fill),
                        comp_style.border_color
                    ));
                    // Top face (parallelogram)
                    out.push_str(&format!(
                        "<polygon points=\"{},{} {},{} {},{} {},{}\" \
                         fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                        x,
                        y, // front-top-left
                        x + depth,
                        y - depth + depth, // back-top-left (same y for simplicity, offset right)
                        x + w + depth,
                        y, // back-top-right
                        x + w,
                        y, // front-top-right
                        escape_text(fill),
                        comp_style.border_color
                    ));
                    // Right face (parallelogram)
                    out.push_str(&format!(
                        "<polygon points=\"{},{} {},{} {},{} {},{}\" \
                         fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                        x + w,
                        y, // front-top-right
                        x + w + depth,
                        y, // back-top-right
                        x + w + depth,
                        y + h, // back-bottom-right
                        x + w,
                        y + h, // front-bottom-right
                        escape_text(fill),
                        comp_style.border_color
                    ));
                    // Front face (main visible face)
                    out.push_str(&format!(
                        "<rect class=\"uml-node uml-deployment-shape\" data-uml-kind=\"{}\" \
                         x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" \
                         fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                        kind_label,
                        x,
                        y,
                        w,
                        h,
                        escape_text(fill),
                        comp_style.border_color
                    ));
                }
                FamilyNodeKind::Database | FamilyNodeKind::Storage => {
                    out.push_str(&format!(
                        "<path class=\"uml-node uml-deployment-shape\" data-uml-kind=\"{}\" d=\"M{x},{top} C{x},{top_minus} {right},{top_minus} {right},{top} L{right},{bottom} C{right},{bottom_plus} {x},{bottom_plus} {x},{bottom} Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                        kind_label,
                        escape_text(fill),
                        comp_style.border_color,
                        top = y + 10,
                        top_minus = y,
                        right = x + w,
                        bottom = y + h - 10,
                        bottom_plus = y + h
                    ));
                    out.push_str(&format!(
                        "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"10\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                        cx,
                        y + 10,
                        w / 2,
                        escape_text(fill),
                        comp_style.border_color
                    ));
                }
                FamilyNodeKind::Cloud => {
                    out.push_str(&format!(
                        "<path class=\"uml-node uml-deployment-shape\" data-uml-kind=\"cloud\" d=\"M{} {} C{} {}, {} {}, {} {} C{} {}, {} {}, {} {} L{} {} C{} {}, {} {}, {} {} Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                        x + 24, y + 56,
                        x + 4, y + 54, x + 4, y + 28, x + 30, y + 28,
                        x + 36, y + 8, x + 76, y + 8, x + 88, y + 26,
                        x + w - 22, y + 26,
                        x + w - 2, y + 28, x + w - 4, y + 56, x + w - 28, y + 56,
                        escape_text(fill),
                        comp_style.border_color
                    ));
                }
                FamilyNodeKind::Folder => {
                    out.push_str(&format!(
                        "<path class=\"uml-node uml-deployment-shape\" data-uml-kind=\"folder\" d=\"M{x},{y} H{} L{} {} H{} V{} H{x} Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                        x + 66,
                        x + 82,
                        y + 14,
                        x + w,
                        y + h,
                        escape_text(fill),
                        comp_style.border_color
                    ));
                }
                FamilyNodeKind::Artifact | FamilyNodeKind::File => {
                    out.push_str(&format!(
                        "<path class=\"uml-node uml-deployment-shape\" data-uml-kind=\"{}\" d=\"M{x},{y} H{} L{} {} V{} H{x} Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                        kind_label,
                        x + w - 18,
                        x + w,
                        y + 18,
                        y + h,
                        escape_text(fill),
                        comp_style.border_color
                    ));
                }
                _ => {
                    out.push_str(&format!(
                        "<rect class=\"uml-node uml-deployment-shape\" data-uml-kind=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" ry=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                        kind_label, x, y, w, h, fill, comp_style.border_color
                    ));
                }
            }
        }
        _ => {
            // Delegate to the non-styled version for all other shapes
            render_family_node_shape(out, node, x, y, w, h);
            return;
        }
    }

    // Label
    let (label_x, label_y) = match node.kind {
        FamilyNodeKind::Interface | FamilyNodeKind::Port => (cx, cy + 28),
        _ => (cx, cy + 6),
    };
    out.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" fill=\"{}\">{}</text>",
        label_x,
        label_y,
        escape_text(&comp_style.font_color),
        escape_text(&display)
    ));
    let kind_tag_y = match node.kind {
        FamilyNodeKind::Interface | FamilyNodeKind::Port => label_y + 14,
        _ => y + 14,
    };
    // For Component, show «component» guillemet stereotype instead of raw "component" (fix #525).
    // For Package and Rectangle container nodes, suppress the kind-tag entirely — these
    // shapes display their label in a tab/header already (fix #549).
    let is_package_container = matches!(
        node.kind,
        FamilyNodeKind::Package | FamilyNodeKind::Rectangle | FamilyNodeKind::Folder
    );
    if !is_package_container {
        let kind_tag_text: std::borrow::Cow<str> = match node.kind {
            FamilyNodeKind::Component => std::borrow::Cow::Borrowed("\u{ab}component\u{bb}"),
            FamilyNodeKind::Interface => std::borrow::Cow::Borrowed("\u{ab}interface\u{bb}"),
            _ => std::borrow::Cow::Borrowed(kind_label),
        };
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"{}\">{}</text>",
            cx, kind_tag_y, escape_text(&comp_style.font_color), escape_text(&kind_tag_text)
        ));
    }
    render_node_stereotype_rows(out, node, cx, kind_tag_y + 13);
}
