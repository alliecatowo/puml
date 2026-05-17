use crate::ast::{DiagramKind, MemberModifier};
use crate::creole::{render_creole_to_svg_tspans, tokenize_creole};
use crate::model::{
    ArchimateDocument, ChartDocument, ChartSubtype, DitaaDocument, EbnfDocument, EbnfToken,
    FamilyDocument, FamilyNode, FamilyNodeKind, FamilyOrientation, FamilyStyle, JsonDocument,
    LegendHAlign, LegendVAlign, MathDocument, MindMapSide, NwdiagDocument, ParticipantRole,
    RegexDocument, RegexToken, RepeatKind, ScaleSpec, SdlDocument, SdlStateKind, StateDocument,
    StateNode, StateNodeKind, TimelineChronologyEvent, TimelineDocument, TimelineTask,
    VirtualEndpointKind, WbsCheckbox, YamlDocument,
};
use crate::scene::{ParticipantBox, Scene, StructureKind};
use crate::theme::{ActivityStyle, ClassStyle, ComponentStyle};

const MESSAGE_LABEL_LINE_GAP: i32 = 16;

pub fn render_svg(scene: &Scene) -> String {
    let mut out = String::new();

    // Compute output dimensions based on scale spec.
    let (svg_width, svg_height, viewbox) = compute_svg_dimensions(scene);

    // Determine if a drop-shadow filter is needed.
    let shadow_filter = if scene.style.shadowing {
        " filter=\"url(#shadow)\""
    } else {
        ""
    };
    let _ = shadow_filter; // used below per-element

    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"{}\">",
        svg_width, svg_height, viewbox
    ));

    // Embed drop-shadow filter when shadowing is enabled.
    if scene.style.shadowing {
        out.push_str(
            "<defs><filter id=\"shadow\" x=\"-10%\" y=\"-10%\" width=\"130%\" height=\"130%\">\
             <feDropShadow dx=\"3\" dy=\"3\" stdDeviation=\"2\" flood-color=\"#00000040\"/>\
             </filter></defs>",
        );
    }

    let bg_fill = scene.style.background_color.as_deref().unwrap_or("white");
    out.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>",
        escape_text(bg_fill)
    ));

    // Determine font family and size from style.
    let font_family = scene
        .style
        .default_font_name
        .as_deref()
        .unwrap_or("monospace");
    let font_size_px = scene.style.default_font_size.unwrap_or(12);
    let _ = (font_family, font_size_px); // used inline below

    if let Some(title) = &scene.title {
        for (idx, line) in title.lines.iter().enumerate() {
            out.push_str(&creole_text(
                title.x,
                title.y + (idx as i32 * 24),
                "font-family=\"monospace\" font-size=\"18\" font-weight=\"600\"",
                line,
                "black",
            ));
        }
    }

    for p in &scene.participants {
        render_participant_box(&mut out, p, scene);
    }

    let lifeline_stroke_width = scene.style.lifeline_thickness.unwrap_or(1);
    for l in &scene.lifelines {
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\" stroke-dasharray=\"6 4\"/>",
            l.x, l.y1, l.x, l.y2, scene.style.lifeline_border_color, lifeline_stroke_width
        ));
    }

    for g in &scene.groups {
        let grx = (scene.style.round_corner / 2).max(2);
        let is_ref = g.kind.eq_ignore_ascii_case("ref");
        let group_fill = if is_ref {
            scene
                .style
                .reference_background_color
                .as_deref()
                .unwrap_or("#eef6ff")
        } else {
            scene.style.group_background_color.as_str()
        };
        let group_border = if is_ref {
            scene
                .style
                .reference_border_color
                .as_deref()
                .unwrap_or(scene.style.group_border_color.as_str())
        } else {
            scene.style.group_border_color.as_str()
        };
        out.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            g.x,
            g.y,
            g.width,
            g.height,
            grx,
            grx,
            group_fill,
            group_border
        ));

        if let Some(label) = &g.label {
            let header = label.lines().next().unwrap_or("");
            let header_full = format!("{} {}", g.kind, header);
            let header_trimmed = header_full.trim();
            let header_font_color = scene
                .style
                .group_header_font_color
                .as_deref()
                .unwrap_or("black");
            use crate::theme::GroupHeaderFontStyle;
            let header_font_weight = match scene.style.group_header_font_style {
                GroupHeaderFontStyle::Bold => "font-weight=\"bold\"",
                _ => "font-weight=\"600\"",
            };
            let header_font_style_attr = match scene.style.group_header_font_style {
                GroupHeaderFontStyle::Italic => " font-style=\"italic\"",
                _ => "",
            };
            out.push_str(&creole_text(
                g.x + 8,
                g.y + 16,
                &format!(
                    "font-family=\"monospace\" font-size=\"12\" {header_font_weight}{header_font_style_attr}"
                ),
                header_trimmed,
                header_font_color,
            ));
            if g.kind.eq_ignore_ascii_case("ref") {
                let mut y = g.y + 32;
                for line in label.lines().skip(1) {
                    out.push_str(&creole_text(
                        g.x + 8,
                        y,
                        "font-family=\"monospace\" font-size=\"12\"",
                        line,
                        "black",
                    ));
                    y += 16;
                }
            }
        }

        for sep in &g.separators {
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"5 4\"/>",
                g.x,
                sep.y,
                g.x + g.width,
                sep.y,
                scene.style.group_border_color
            ));
            if let Some(label) = &sep.label {
                out.push_str(&creole_text(
                    g.x + 8,
                    sep.y - 6,
                    "font-family=\"monospace\" font-size=\"11\" fill=\"#333\"",
                    label,
                    "#333",
                ));
            }
        }
    }

    let message_line_color = scene
        .style
        .message_line_color
        .as_deref()
        .unwrap_or(scene.style.arrow_color.as_str());
    for m in &scene.messages {
        let stroke_dash = if m.arrow.contains("--") {
            " stroke-dasharray=\"6 4\""
        } else {
            ""
        };
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"{}/>",
            m.x1, m.y, m.x2, m.y, message_line_color, stroke_dash
        ));
        let arrow_size = 6;
        if m.x2 >= m.x1 {
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\"/>",
                m.x2,
                m.y,
                m.x2 - arrow_size,
                m.y - 4,
                m.x2 - arrow_size,
                m.y + 4,
                scene.style.arrow_color
            ));
        } else {
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\"/>",
                m.x2,
                m.y,
                m.x2 + arrow_size,
                m.y - 4,
                m.x2 + arrow_size,
                m.y + 4,
                scene.style.arrow_color
            ));
        }

        if let Some(virtual_ep) = m.from_virtual {
            render_virtual_endpoint_marker(&mut out, m.x1, m.y, virtual_ep.kind);
        }
        if let Some(virtual_ep) = m.to_virtual {
            render_virtual_endpoint_marker(&mut out, m.x2, m.y, virtual_ep.kind);
        }

        if !m.label_lines.is_empty() {
            let tx = ((m.x1 + m.x2) / 2) + 2;
            let start_y = m.y - 8 - (((m.label_lines.len() as i32) - 1) * MESSAGE_LABEL_LINE_GAP);
            for (idx, line) in m.label_lines.iter().enumerate() {
                out.push_str(&creole_text(
                    tx,
                    start_y + (idx as i32 * MESSAGE_LABEL_LINE_GAP),
                    "text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\"",
                    line,
                    "black",
                ));
            }
        } else if let Some(label) = &m.label {
            let tx = ((m.x1 + m.x2) / 2) + 2;
            let ty = m.y - 8;
            out.push_str(&creole_text(
                tx,
                ty,
                "text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\"",
                label,
                "black",
            ));
        }
    }

    for n in &scene.notes {
        let nrx = (scene.style.round_corner / 2).max(2);
        out.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            n.x, n.y, n.width, n.height, nrx, nrx, scene.style.note_background_color, scene.style.note_border_color
        ));

        let mut text_y = n.y + 20;
        for line in n.text.lines() {
            out.push_str(&creole_text(
                n.x + 8,
                text_y,
                "font-family=\"monospace\" font-size=\"12\"",
                line,
                "black",
            ));
            text_y += 16;
        }
    }

    for s in &scene.structures {
        match s.kind {
            StructureKind::Delay => {
                out.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#777\" stroke-width=\"1\" stroke-dasharray=\"3 7\"/>",
                    s.x1, s.y, s.x2, s.y
                ));
                if let Some(label) = &s.label {
                    out.push_str(&creole_text(
                        (s.x1 + s.x2) / 2,
                        s.y - 6,
                        "text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#444\"",
                        label,
                        "#444",
                    ));
                }
            }
            StructureKind::Divider => {
                out.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#666\" stroke-width=\"1\" stroke-dasharray=\"8 5\"/>",
                    s.x1, s.y, s.x2, s.y
                ));
                if let Some(label) = &s.label {
                    out.push_str(&creole_text(
                        (s.x1 + s.x2) / 2,
                        s.y - 6,
                        "text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#333\"",
                        label,
                        "#333",
                    ));
                }
            }
            StructureKind::Separator => {
                out.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#222\" stroke-width=\"1.5\"/>",
                    s.x1, s.y, s.x2, s.y
                ));
                let label = if let Some(label) = &s.label {
                    format!("== {} ==", label)
                } else {
                    "== ==".to_string()
                };
                out.push_str(&creole_text(
                    (s.x1 + s.x2) / 2,
                    s.y - 6,
                    "text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" font-weight=\"600\" fill=\"#222\"",
                    &label,
                    "#222",
                ));
            }
            StructureKind::Spacer => {}
        }
    }

    for p in &scene.footboxes {
        render_participant_box(&mut out, p, scene);
    }

    // Render legend if present.
    if let Some(legend_text) = &scene.legend_text {
        render_legend(&mut out, legend_text, scene);
    }

    out.push_str("</svg>");
    out
}

fn compute_svg_dimensions(scene: &Scene) -> (String, String, String) {
    let w = scene.width;
    let h = scene.height;
    let viewbox = format!("0 0 {} {}", w, h);
    match &scene.scale {
        None => (w.to_string(), h.to_string(), viewbox),
        Some(ScaleSpec::Factor(f)) => {
            let sw = (w as f64 * f).round() as i32;
            let sh = (h as f64 * f).round() as i32;
            (sw.to_string(), sh.to_string(), viewbox)
        }
        Some(ScaleSpec::Fixed {
            width: fw,
            height: fh,
        }) => (fw.to_string(), fh.to_string(), viewbox),
        Some(ScaleSpec::Max(max)) => {
            let max = *max as f64;
            let larger = (w.max(h)) as f64;
            if larger <= max {
                (w.to_string(), h.to_string(), viewbox)
            } else {
                let factor = max / larger;
                let sw = (w as f64 * factor).round() as i32;
                let sh = (h as f64 * factor).round() as i32;
                (sw.to_string(), sh.to_string(), viewbox)
            }
        }
    }
}

fn render_legend(out: &mut String, text: &str, scene: &Scene) {
    let lines: Vec<&str> = text.lines().collect();
    let line_count = lines.len() as i32;
    let box_width = 200_i32;
    let box_height = 24 + line_count * 16;
    let margin = 10_i32;

    let x = match scene.legend_halign {
        LegendHAlign::Left => margin,
        LegendHAlign::Center => (scene.width - box_width) / 2,
        LegendHAlign::Right => scene.width - box_width - margin,
    };
    let y = match scene.legend_valign {
        LegendVAlign::Top => margin,
        LegendVAlign::Bottom => scene.height - box_height - margin,
    };

    out.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" ry=\"4\" fill=\"#fffff0\" stroke=\"#aaa\" stroke-width=\"1\" opacity=\"0.9\"/>",
        x, y, box_width, box_height
    ));

    let mut ty = y + 16;
    for line in &lines {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#333\">{}</text>",
            x + 8,
            ty,
            escape_text(line)
        ));
        ty += 16;
    }
}

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
    let node_width: i32 = 200;
    let col_gap: i32 = 48;
    let row_gap: i32 = 64;
    let header_height: i32 = 30;
    let member_line_height: i32 = 16;
    let member_padding: i32 = 8;
    let empty_member_pad: i32 = 8;

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
        let body_h = if node.members.is_empty() {
            empty_member_pad
        } else {
            (node.members.len() as i32) * member_line_height + 2 * member_padding
        };
        let h = c4_node_height(node.kind, header_height + body_h);
        let _ = col;
        if (row as usize) < row_heights.len() && h > row_heights[row as usize] {
            row_heights[row as usize] = h;
        }
    }

    // Second pass: assign coordinates
    let mut row_y_offsets: Vec<i32> = vec![0; row_heights.len()];
    {
        let mut y = margin_top + title_block_height;
        for (i, h) in row_heights.iter().enumerate() {
            row_y_offsets[i] = y;
            y += h + row_gap;
        }
    }

    for (idx, node) in document.nodes.iter().enumerate() {
        let col = (idx as i32) % col_count;
        let row = (idx as i32) / col_count;
        let body_h = if node.members.is_empty() {
            empty_member_pad
        } else {
            (node.members.len() as i32) * member_line_height + 2 * member_padding
        };
        let h = c4_node_height(node.kind, header_height + body_h);
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
        let kv_count = extract_json_kv_lines(&proj.body).len() as i32;
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
    out.push_str("<defs>");
    out.push_str(&format!(
        "<marker id=\"arrow-open\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" \
         markerWidth=\"10\" markerHeight=\"10\" orient=\"auto-start-reverse\">\
         <path d=\"M0,0 L10,5 L0,10\" fill=\"none\" stroke=\"{arrow_stroke}\" stroke-width=\"1.5\"/>\
         </marker>",
    ));
    out.push_str(&format!(
        "<marker id=\"arrow-triangle\" viewBox=\"0 0 12 12\" refX=\"11\" refY=\"6\" \
         markerWidth=\"12\" markerHeight=\"12\" orient=\"auto-start-reverse\">\
         <path d=\"M0,0 L12,6 L0,12 z\" fill=\"white\" stroke=\"{arrow_stroke}\" stroke-width=\"1.5\"/>\
         </marker>",
    ));
    out.push_str(&format!(
        "<marker id=\"arrow-diamond-filled\" viewBox=\"0 0 14 10\" refX=\"13\" refY=\"5\" \
         markerWidth=\"14\" markerHeight=\"10\" orient=\"auto-start-reverse\">\
         <path d=\"M0,5 L7,0 L14,5 L7,10 z\" fill=\"{arrow_stroke}\" stroke=\"{arrow_stroke}\" stroke-width=\"1\"/>\
         </marker>",
    ));
    out.push_str(&format!(
        "<marker id=\"arrow-diamond-open\" viewBox=\"0 0 14 10\" refX=\"13\" refY=\"5\" \
         markerWidth=\"14\" markerHeight=\"10\" orient=\"auto-start-reverse\">\
         <path d=\"M0,5 L7,0 L14,5 L7,10 z\" fill=\"white\" stroke=\"{arrow_stroke}\" stroke-width=\"1\"/>\
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
        let usecase_dependency = matches!(document.kind, crate::ast::DiagramKind::UseCase)
            .then(|| usecase_dependency_label(relation.label.as_deref()))
            .flatten();
        if usecase_dependency.is_some() {
            style.dashed = true;
            if style.end_marker.is_none() {
                style.end_marker = Some("arrow-open");
            }
        }
        let (x1, y1, x2, y2) =
            compute_edge_anchors_tuple((from.x, from.y, from.w, from.h), (to.x, to.y, to.w, to.h));
        let stroke_dash = if style.dashed {
            " stroke-dasharray=\"5 3\""
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
        out.push_str(&format!(
            "<line x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\" stroke=\"{arrow_stroke}\" stroke-width=\"1.5\"{dash}{markers}/>",
            dash = stroke_dash
        ));
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
        let rendered_label = usecase_dependency.or(relation.label.as_deref());
        if let Some(label) = rendered_label {
            let lx = (x1 + x2) / 2;
            let ly = (y1 + y2) / 2 - 4;
            out.push_str(&format!(
                "<text x=\"{lx}\" y=\"{ly}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{member_color}\">{txt}</text>",
                member_color = class_style.member_color,
                txt = escape_text(label)
            ));
        }
    }

    // Render groups (together/package/namespace) as labeled frames BEFORE nodes
    // so node rectangles visually sit on top of the frame borders.
    for group in &document.groups {
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
        let pad = 12;
        let label_header = 20; // extra space at top for the group label
        let fx = gx_min - pad;
        let fy = gy_min - pad - label_header;
        let fw = (gx_max - gx_min) + pad * 2;
        let fh = (gy_max - gy_min) + pad * 2 + label_header;

        let group_label = match group.label.as_deref() {
            Some(lbl) => format!("{} {}", group.kind, lbl),
            None => group.kind.clone(),
        };

        // Frame rectangle
        out.push_str(&format!(
            "<rect x=\"{fx}\" y=\"{fy}\" width=\"{fw}\" height=\"{fh}\" rx=\"6\" ry=\"6\" fill=\"none\" stroke=\"#6366f1\" stroke-width=\"1.5\" stroke-dasharray=\"5 3\"/>",
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
            let kv_lines = extract_json_kv_lines(&proj.body);
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
                "<text x=\"{tx}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"12\" font-weight=\"600\" fill=\"#78350f\">{alias}</text>",
                tx = proj_margin_left + 8,
                ty = proj_y + 15,
                alias = escape_text(&proj.alias),
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

/// Extract `key: value` display lines from a JSON body string.
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
                let label_lines = wrap_text(label.clone(), 18, document.text_overflow_policy);
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
pub fn render_salt_svg(document: &FamilyDocument) -> String {
    const CELL_H: i32 = 28;
    const CELL_PAD_X: i32 = 10;
    const MARGIN: i32 = 24;
    const MIN_CELL_W: i32 = 80;

    // Parse rows from the encoded node names.
    let mut rows: Vec<Vec<SaltCellRender>> = Vec::new();
    let mut in_tree = false;
    for node in &document.nodes {
        if let Some(rest) = node.name.strip_prefix("SALT_ROW\x1f") {
            let cells: Vec<SaltCellRender> = rest.split('\x1e').map(decode_salt_cell).collect();
            if let Some(cells) = transform_salt_row(cells, &mut in_tree) {
                rows.push(cells);
            }
        }
    }

    if rows.is_empty() {
        // Fallback: render a minimal empty wireframe
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"120\" height=\"60\"><rect width=\"120\" height=\"60\" fill=\"white\"/><text x=\"10\" y=\"30\" font-family=\"monospace\" font-size=\"11\" fill=\"#666\">[salt]</text></svg>".to_string();
    }

    // Compute number of columns from the max row width.
    let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(1);

    // First pass: compute per-column minimum widths based on text content.
    let mut col_widths: Vec<i32> = vec![MIN_CELL_W; col_count];
    for row in &rows {
        for (col_idx, cell) in row.iter().enumerate() {
            let text_w = estimate_text_width(cell.text()) + CELL_PAD_X * 2 + 20;
            if text_w > col_widths[col_idx] {
                col_widths[col_idx] = text_w;
            }
        }
    }

    let total_w = col_widths.iter().sum::<i32>() + MARGIN * 2;
    let total_h = (rows.len() as i32) * CELL_H + MARGIN * 2;

    // Title height
    let title_h = document.title.as_deref().map(|_| 28i32).unwrap_or(0);
    let svg_h = total_h + title_h;

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\">",
        total_w, svg_h
    ));
    out.push_str(&format!(
        "<rect width=\"{}\" height=\"{}\" fill=\"#f5f5f5\"/>",
        total_w, svg_h
    ));

    // Outer border
    out.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"white\" stroke=\"#555\" stroke-width=\"1.5\"/>",
        MARGIN,
        MARGIN + title_h,
        total_w - MARGIN * 2,
        total_h - MARGIN * 2
    ));

    if let Some(title) = &document.title {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" fill=\"#222\">{}</text>",
            MARGIN,
            MARGIN - 6,
            escape_text(title)
        ));
    }

    // Draw rows and cells.
    for (row_idx, cells) in rows.iter().enumerate() {
        let row_y = MARGIN + title_h + (row_idx as i32) * CELL_H;
        if is_salt_separator_row(cells) {
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#777\" stroke-width=\"1.5\"/>",
                MARGIN + 4,
                row_y + CELL_H / 2,
                total_w - MARGIN - 4,
                row_y + CELL_H / 2
            ));
            continue;
        }
        let mut col_x = MARGIN;

        for (col_idx, cell) in cells.iter().enumerate() {
            let cell_w = col_widths.get(col_idx).copied().unwrap_or(MIN_CELL_W);
            render_salt_cell_svg(&mut out, cell, col_x, row_y, cell_w, CELL_H);
            col_x += cell_w;
        }

        // Row separator line (skip the last row)
        if row_idx + 1 < rows.len() {
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#ccc\" stroke-width=\"0.5\"/>",
                MARGIN,
                row_y + CELL_H,
                total_w - MARGIN,
                row_y + CELL_H
            ));
        }
    }

    // Column separator lines
    {
        let mut col_x = MARGIN;
        for (col_idx, w) in col_widths.iter().enumerate() {
            col_x += w;
            if col_idx + 1 < col_count {
                out.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#ccc\" stroke-width=\"0.5\"/>",
                    col_x,
                    MARGIN + title_h,
                    col_x,
                    MARGIN + title_h + total_h - MARGIN * 2
                ));
            }
        }
    }

    out.push_str("</svg>");
    out
}

fn is_salt_separator_row(cells: &[SaltCellRender]) -> bool {
    let mut saw_dash = false;
    for cell in cells {
        match cell {
            SaltCellRender::Label(text) => {
                let t = text.trim();
                if t.is_empty() {
                    continue;
                }
                if t.chars().all(|c| c == '-') {
                    saw_dash = true;
                    continue;
                }
                return false;
            }
            _ => return false,
        }
    }
    saw_dash
}

/// A decoded salt cell ready for rendering.
enum SaltCellRender {
    Label(String),
    Input(String),
    Button(String),
    Combo(String),
    CheckboxChecked(String),
    CheckboxUnchecked(String),
    RadioOn(String),
    RadioOff(String),
    TreeItem { depth: usize, label: String },
    MenuBar(Vec<String>),
    TabBar { tabs: Vec<String>, active: usize },
    ScrollBar { vertical: bool, percent: u8 },
}

impl SaltCellRender {
    fn text(&self) -> &str {
        match self {
            Self::Label(t)
            | Self::Input(t)
            | Self::Button(t)
            | Self::Combo(t)
            | Self::CheckboxChecked(t)
            | Self::CheckboxUnchecked(t)
            | Self::RadioOn(t)
            | Self::RadioOff(t) => t,
            Self::TreeItem { label, .. } => label,
            Self::MenuBar(items) => items.first().map(String::as_str).unwrap_or("menu"),
            Self::TabBar { tabs, .. } => tabs.first().map(String::as_str).unwrap_or("tab"),
            Self::ScrollBar { .. } => "scrollbar",
        }
    }
}

fn transform_salt_row(
    cells: Vec<SaltCellRender>,
    in_tree: &mut bool,
) -> Option<Vec<SaltCellRender>> {
    if cells.len() != 1 {
        return Some(cells);
    }

    let SaltCellRender::Label(text) = &cells[0] else {
        return Some(cells);
    };
    let trimmed = text.trim();
    let lower = trimmed.to_ascii_lowercase();

    if matches!(trimmed, "{" | "}") {
        if trimmed == "}" {
            *in_tree = false;
        }
        return None;
    }

    if lower.starts_with("{t") || lower == "tree" || lower.starts_with("tree ") {
        *in_tree = true;
        return None;
    }

    if let Some((depth, label)) = parse_salt_tree_line(trimmed) {
        return Some(vec![SaltCellRender::TreeItem { depth, label }]);
    }

    if let Some(items) = parse_salt_items(trimmed, &["{*", "menu"]) {
        return Some(vec![SaltCellRender::MenuBar(items)]);
    }

    if let Some(tabs) = parse_salt_items(trimmed, &["{/", "tab", "tabs"]) {
        return Some(vec![SaltCellRender::TabBar { tabs, active: 0 }]);
    }

    if let Some((vertical, percent)) = parse_salt_scrollbar(trimmed) {
        return Some(vec![SaltCellRender::ScrollBar { vertical, percent }]);
    }

    if *in_tree {
        *in_tree = false;
    }

    Some(cells)
}

fn parse_salt_tree_line(line: &str) -> Option<(usize, String)> {
    let depth = line.chars().take_while(|&ch| ch == '+').count();
    if depth == 0 {
        return None;
    }
    let label = line[depth..].trim().trim_matches('"').to_string();
    if label.is_empty() {
        None
    } else {
        Some((depth.saturating_sub(1), label))
    }
}

fn parse_salt_items(line: &str, prefixes: &[&str]) -> Option<Vec<String>> {
    let lower = line.to_ascii_lowercase();
    let mut rest = None;
    for prefix in prefixes {
        if lower.starts_with(prefix)
            && (prefix.starts_with('{')
                || lower.len() == prefix.len()
                || lower
                    .as_bytes()
                    .get(prefix.len())
                    .is_some_and(|ch| ch.is_ascii_whitespace()))
        {
            rest = Some(line[prefix.len()..].trim());
            break;
        }
    }
    let rest = rest?;
    let rest = rest.trim_matches('{').trim_matches('}').trim();
    let items: Vec<String> = rest
        .split(['|', ','])
        .map(|item| item.trim().trim_matches('"').to_string())
        .filter(|item| !item.is_empty())
        .collect();
    if items.is_empty() {
        None
    } else {
        Some(items)
    }
}

fn parse_salt_scrollbar(line: &str) -> Option<(bool, u8)> {
    let lower = line.to_ascii_lowercase();
    if !(lower.starts_with("{s") || lower.starts_with("scroll") || lower.contains("scrollbar")) {
        return None;
    }
    let vertical = !lower.contains("horizontal");
    let percent = lower
        .split(|ch: char| !ch.is_ascii_digit())
        .find_map(|part| part.parse::<u8>().ok())
        .unwrap_or(40)
        .min(100);
    Some((vertical, percent))
}

/// Decode a salt cell from the encoded string `"X:text"`.
fn decode_salt_cell(s: &str) -> SaltCellRender {
    if let Some(rest) = s.strip_prefix("I:") {
        SaltCellRender::Input(rest.to_string())
    } else if let Some(rest) = s.strip_prefix("B:") {
        SaltCellRender::Button(rest.to_string())
    } else if let Some(rest) = s.strip_prefix("C:") {
        SaltCellRender::Combo(rest.to_string())
    } else if let Some(rest) = s.strip_prefix("CX:") {
        SaltCellRender::CheckboxChecked(rest.to_string())
    } else if let Some(rest) = s.strip_prefix("CU:") {
        SaltCellRender::CheckboxUnchecked(rest.to_string())
    } else if let Some(rest) = s.strip_prefix("RO:") {
        SaltCellRender::RadioOn(rest.to_string())
    } else if let Some(rest) = s.strip_prefix("RF:") {
        SaltCellRender::RadioOff(rest.to_string())
    } else if let Some(rest) = s.strip_prefix("L:") {
        SaltCellRender::Label(rest.to_string())
    } else {
        SaltCellRender::Label(s.to_string())
    }
}

/// Estimate text width in monospace pixels (approx 7px per char at 12px font).
fn estimate_text_width(text: &str) -> i32 {
    (text.chars().count() as i32) * 7
}

/// Render a single salt cell into SVG, appending to `out`.
fn render_salt_cell_svg(out: &mut String, cell: &SaltCellRender, x: i32, y: i32, w: i32, h: i32) {
    let pad = 8;
    let text_y = y + h / 2 + 4;
    match cell {
        SaltCellRender::Label(text) => {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#222\">{}</text>",
                x + pad,
                text_y,
                escape_text(text)
            ));
        }
        SaltCellRender::Input(placeholder) => {
            // Bordered rectangle with gray placeholder text
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"white\" stroke=\"#888\" stroke-width=\"1\" rx=\"2\" ry=\"2\"/>",
                x + pad,
                y + 4,
                w - pad * 2,
                h - 8
            ));
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#aaa\">{}</text>",
                x + pad + 4,
                text_y,
                escape_text(placeholder)
            ));
        }
        SaltCellRender::Button(label) => {
            // Rounded rectangle with bold text
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#e8e8e8\" stroke=\"#555\" stroke-width=\"1\" rx=\"4\" ry=\"4\"/>",
                x + pad,
                y + 4,
                w - pad * 2,
                h - 8
            ));
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\" font-weight=\"bold\" fill=\"#111\">{}</text>",
                x + w / 2,
                text_y,
                escape_text(label)
            ));
        }
        SaltCellRender::Combo(label) => {
            // Rectangle with down-arrow indicator
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"white\" stroke=\"#888\" stroke-width=\"1\" rx=\"2\" ry=\"2\"/>",
                x + pad,
                y + 4,
                w - pad * 2,
                h - 8
            ));
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#222\">{}</text>",
                x + pad + 4,
                text_y,
                escape_text(label)
            ));
            // Down arrow triangle
            let ax = x + w - pad - 10;
            let ay = y + h / 2;
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{}\" fill=\"#555\"/>",
                ax,
                ay - 3,
                ax + 8,
                ay - 3,
                ax + 4,
                ay + 3
            ));
        }
        SaltCellRender::CheckboxChecked(label) => {
            let bx = x + pad;
            let by = y + h / 2 - 6;
            // Box
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"12\" height=\"12\" fill=\"white\" stroke=\"#555\" stroke-width=\"1\"/>",
                bx, by
            ));
            // Checkmark (×)
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#222\" stroke-width=\"1.5\"/>",
                bx + 2, by + 2, bx + 10, by + 10
            ));
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#222\" stroke-width=\"1.5\"/>",
                bx + 10, by + 2, bx + 2, by + 10
            ));
            if !label.is_empty() {
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#222\">{}</text>",
                    bx + 16,
                    text_y,
                    escape_text(label)
                ));
            }
        }
        SaltCellRender::CheckboxUnchecked(label) => {
            let bx = x + pad;
            let by = y + h / 2 - 6;
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"12\" height=\"12\" fill=\"white\" stroke=\"#555\" stroke-width=\"1\"/>",
                bx, by
            ));
            if !label.is_empty() {
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#222\">{}</text>",
                    bx + 16,
                    text_y,
                    escape_text(label)
                ));
            }
        }
        SaltCellRender::RadioOn(label) => {
            let cx = x + pad + 6;
            let cy = y + h / 2;
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"6\" fill=\"white\" stroke=\"#555\" stroke-width=\"1\"/>",
                cx, cy
            ));
            // Filled dot
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"3\" fill=\"#333\"/>",
                cx, cy
            ));
            if !label.is_empty() {
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#222\">{}</text>",
                    cx + 10,
                    text_y,
                    escape_text(label)
                ));
            }
        }
        SaltCellRender::RadioOff(label) => {
            let cx = x + pad + 6;
            let cy = y + h / 2;
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"6\" fill=\"white\" stroke=\"#555\" stroke-width=\"1\"/>",
                cx, cy
            ));
            if !label.is_empty() {
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#222\">{}</text>",
                    cx + 10,
                    text_y,
                    escape_text(label)
                ));
            }
        }
        SaltCellRender::TreeItem { depth, label } => {
            let indent = (*depth as i32) * 16;
            let branch_x = x + pad + indent;
            let cy = y + h / 2;
            out.push_str(&format!(
                "<g data-salt-widget=\"tree\"><line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#9ca3af\" stroke-width=\"1\"/><line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#9ca3af\" stroke-width=\"1\"/><circle cx=\"{}\" cy=\"{}\" r=\"3\" fill=\"#475569\"/><text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#1f2937\">{}</text></g>",
                branch_x,
                y + 4,
                branch_x,
                y + h - 4,
                branch_x,
                cy,
                branch_x + 10,
                cy,
                branch_x + 10,
                cy,
                branch_x + 18,
                text_y,
                escape_text(label)
            ));
        }
        SaltCellRender::MenuBar(items) => {
            out.push_str(&format!(
                "<rect data-salt-widget=\"menu\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#eef2ff\" stroke=\"#64748b\" stroke-width=\"1\"/>",
                x + 1,
                y + 2,
                w - 2,
                h - 4
            ));
            let mut item_x = x + pad;
            for item in items {
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#1e293b\">{}</text>",
                    item_x,
                    text_y,
                    escape_text(item)
                ));
                item_x += estimate_text_width(item) + 24;
            }
        }
        SaltCellRender::TabBar { tabs, active } => {
            let mut tab_x = x + pad;
            for (idx, tab) in tabs.iter().enumerate() {
                let tab_w = estimate_text_width(tab) + 24;
                let active_tab = idx == *active;
                let fill = if active_tab { "white" } else { "#e2e8f0" };
                let stroke = if active_tab { "#334155" } else { "#94a3b8" };
                out.push_str(&format!(
                    "<rect data-salt-widget=\"tab\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"5\" ry=\"5\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                    tab_x,
                    y + 4,
                    tab_w,
                    h - 5,
                    fill,
                    stroke
                ));
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#0f172a\">{}</text>",
                    tab_x + 12,
                    text_y,
                    escape_text(tab)
                ));
                tab_x += tab_w - 1;
            }
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#334155\" stroke-width=\"1\"/>",
                x + pad,
                y + h - 1,
                x + w - pad,
                y + h - 1
            ));
        }
        SaltCellRender::ScrollBar { vertical, percent } => {
            let track_x = if *vertical { x + w - pad - 12 } else { x + pad };
            let track_y = if *vertical { y + 5 } else { y + h - 13 };
            let track_w = if *vertical { 12 } else { w - pad * 2 };
            let track_h = if *vertical { h - 10 } else { 12 };
            out.push_str(&format!(
                "<rect data-salt-widget=\"scrollbar\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" ry=\"6\" fill=\"#e5e7eb\" stroke=\"#94a3b8\" stroke-width=\"1\"/>",
                track_x, track_y, track_w, track_h
            ));
            if *vertical {
                let thumb_h = ((track_h as f32) * (*percent as f32 / 100.0)).round() as i32;
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"5\" ry=\"5\" fill=\"#64748b\"/>",
                    track_x + 2,
                    track_y + 2,
                    track_w - 4,
                    thumb_h.max(8).min(track_h - 4)
                ));
            } else {
                let thumb_w = ((track_w as f32) * (*percent as f32 / 100.0)).round() as i32;
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"5\" ry=\"5\" fill=\"#64748b\"/>",
                    track_x + 2,
                    track_y + 2,
                    thumb_w.max(12).min(track_w - 4),
                    track_h - 4
                ));
            }
        }
    }
}

/// Parse visibility prefix from member line.
/// Returns (Option<symbol_str>, color, rest_of_text).
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

fn family_node_label(kind: FamilyNodeKind) -> &'static str {
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

fn render_class_node(
    out: &mut String,
    node: &crate::model::FamilyNode,
    geometry: ClassNodeGeometry,
    class_style: &ClassStyle,
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

    let fill = &class_style.background_color;
    let stroke = &class_style.border_color;
    let header_fill = match node.kind {
        FamilyNodeKind::Class => class_style.header_color.as_str(),
        FamilyNodeKind::Object => "#fef3c7",
        FamilyNodeKind::UseCase => "#dcfce7",
        _ => "#f1f5f9",
    };

    if matches!(node.kind, FamilyNodeKind::UseCase) {
        // Ellipse rendering for use cases
        let cx = x + w / 2;
        let cy = y + h / 2;
        let rx = w / 2;
        let ry = h / 2;
        out.push_str(&format!(
            "<ellipse cx=\"{cx}\" cy=\"{cy}\" rx=\"{rx}\" ry=\"{ry}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
        ));
        // Name centered
        out.push_str(&format!(
            "<text x=\"{cx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" fill=\"#0f172a\">{name}</text>",
            ty = cy + 4,
            name = escape_text(&node.name)
        ));
        if let Some(alias) = &node.alias {
            out.push_str(&format!(
                "<text x=\"{cx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"#475569\">as {alias}</text>",
                ty = cy + 20,
                alias = escape_text(alias)
            ));
        }
        // Members rendered below the ellipse (rare for usecases)
        let mut my = y + h + 14;
        for member in &node.members {
            out.push_str(&format!(
                "<text x=\"{tx}\" y=\"{my}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{mc}\">{m}</text>",
                tx = x + w / 2,
                mc = class_style.member_color,
                m = escape_text(&member.text)
            ));
            my += 14;
        }
        return;
    }

    // Outer rect
    out.push_str(&format!(
        "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"4\" ry=\"4\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
    ));
    // Header band
    out.push_str(&format!(
        "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{hh}\" rx=\"4\" ry=\"4\" fill=\"{header_fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
        hh = header_h
    ));
    // Header separator line
    out.push_str(&format!(
        "<line x1=\"{x}\" y1=\"{ly}\" x2=\"{x2}\" y2=\"{ly}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
        ly = y + header_h,
        x2 = x + w
    ));

    // Header text: for Object render "name : Class" style if present; for now just name
    let kind_prefix = match node.kind {
        FamilyNodeKind::Class => "",
        FamilyNodeKind::Object => "",
        FamilyNodeKind::UseCase => "",
        _ => "",
    };
    let header_text = if let Some(alias) = &node.alias {
        format!("{kind_prefix}{} (as {})", node.name, alias)
    } else {
        format!("{kind_prefix}{}", node.name)
    };
    // Underline for objects (PlantUML convention)
    let text_decoration = if matches!(node.kind, FamilyNodeKind::Object) {
        " text-decoration=\"underline\""
    } else {
        ""
    };
    out.push_str(&format!(
        "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" fill=\"#0f172a\"{td}>{txt}</text>",
        tx = x + w / 2,
        ty = y + header_h - 9,
        td = text_decoration,
        txt = escape_text(&header_text)
    ));

    // Members
    let mut my = y + header_h + 16;
    for member in &node.members {
        let (vis_sym, vis_color, rest_after_vis) = parse_visibility_member(&member.text);
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
        let effective_color = vis_color;
        let _ = &class_style.member_color; // Available if needed for override
                                           // Reconstruct display text: keep visibility prefix + remaining text
        let display_text = if vis_sym.is_some() {
            format!("{}{}", vis_sym.unwrap_or(""), text_after_mod)
        } else {
            text_after_mod.to_string()
        };
        out.push_str(&format!(
            "<text x=\"{tx}\" y=\"{my}\" font-family=\"monospace\" font-size=\"11\" fill=\"{vc}\"{sa}>{m}</text>",
            tx = x + 10,
            vc = effective_color,
            sa = style_attrs,
            m = escape_text(&display_text)
        ));
        my += 16;
    }
}

/// Ensure C4 nodes have enough minimum height to render their visual elements.
fn c4_node_height(kind: FamilyNodeKind, computed: i32) -> i32 {
    match kind {
        // Person nodes need space for stick figure (44px) + body rect (≥50px)
        FamilyNodeKind::C4Person | FamilyNodeKind::C4PersonExt => computed.max(94),
        // All other C4 nodes need at least 60px for the label + type label
        k if is_c4_kind(k) => computed.max(60),
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

/// Normalize relation endpoints when the parser stuffs arrow-head markers
/// (e.g. `<|`, `*`, `o`) into the trailing chars of `from` or the leading
/// chars of `to`. Returns (clean_from, clean_to, normalized_arrow).
fn normalize_relation_endpoints(from: &str, to: &str, arrow: &str) -> (String, String, String) {
    let (clean_from, head_marker) = split_trailing_marker(from);
    let (clean_to, tail_marker) = split_leading_marker(to);
    let mut a = String::new();
    a.push_str(head_marker);
    a.push_str(arrow);
    a.push_str(tail_marker);
    (clean_from, clean_to, a)
}

fn split_trailing_marker(s: &str) -> (String, &'static str) {
    let trimmed = s.trim_end();
    if let Some(stripped) = trimmed.strip_suffix("<|") {
        return (stripped.trim_end().to_string(), "<|");
    }
    for m in ["*", "o", "<", "+"] {
        if let Some(stripped) = trimmed.strip_suffix(m) {
            // Require space between name and marker to avoid clobbering names
            // that legitimately end with these characters.
            if stripped.ends_with(' ') {
                return (
                    stripped.trim_end().to_string(),
                    match m {
                        "*" => "*",
                        "o" => "o",
                        "<" => "<",
                        "+" => "+",
                        _ => "",
                    },
                );
            }
        }
    }
    (trimmed.to_string(), "")
}

fn split_leading_marker(s: &str) -> (String, &'static str) {
    let trimmed = s.trim_start();
    if let Some(stripped) = trimmed.strip_prefix("|>") {
        return (stripped.trim_start().to_string(), "|>");
    }
    for m in ["*", "o", ">", "+"] {
        if let Some(stripped) = trimmed.strip_prefix(m) {
            if stripped.starts_with(' ') {
                return (
                    stripped.trim_start().to_string(),
                    match m {
                        "*" => "*",
                        "o" => "o",
                        ">" => ">",
                        "+" => "+",
                        _ => "",
                    },
                );
            }
        }
    }
    (trimmed.to_string(), "")
}

struct ArrowStyle {
    end_marker: Option<&'static str>,
    start_marker: Option<&'static str>,
    dashed: bool,
}

fn arrow_style(arrow: &str) -> ArrowStyle {
    let trimmed = arrow.trim();
    let dashed = trimmed.contains("..");
    // Detect markers at each end
    let head = trimmed.chars().next().unwrap_or(' ');
    let tail = trimmed.chars().last().unwrap_or(' ');
    let start_marker = match head {
        '<' => {
            // inheritance reversed if starts with "<|"
            if trimmed.starts_with("<|") {
                Some("arrow-triangle")
            } else {
                Some("arrow-open")
            }
        }
        '*' => Some("arrow-diamond-filled"),
        'o' => Some("arrow-diamond-open"),
        _ => None,
    };
    let end_marker = match tail {
        '>' => {
            if trimmed.ends_with("|>") {
                Some("arrow-triangle")
            } else {
                Some("arrow-open")
            }
        }
        '*' => Some("arrow-diamond-filled"),
        'o' => Some("arrow-diamond-open"),
        _ => None,
    };
    ArrowStyle {
        end_marker,
        start_marker,
        dashed,
    }
}

fn usecase_dependency_label(label: Option<&str>) -> Option<&'static str> {
    let normalized = label?.trim().to_ascii_lowercase();
    let compact = normalized.split_whitespace().collect::<String>();
    if matches!(compact.as_str(), "<<include>>" | "include" | "includes") {
        Some("<<include>>")
    } else if matches!(compact.as_str(), "<<extend>>" | "extend" | "extends") {
        Some("<<extend>>")
    } else {
        None
    }
}

fn render_relation_marker_defs(out: &mut String, arrow_stroke: &str) {
    out.push_str("<defs>");
    out.push_str(&format!(
        "<marker id=\"arrow-open\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" \
         markerWidth=\"10\" markerHeight=\"10\" orient=\"auto-start-reverse\">\
         <path d=\"M0,0 L10,5 L0,10\" fill=\"none\" stroke=\"{arrow_stroke}\" stroke-width=\"1.5\"/>\
         </marker>",
    ));
    out.push_str(&format!(
        "<marker id=\"arrow-triangle\" viewBox=\"0 0 12 12\" refX=\"11\" refY=\"6\" \
         markerWidth=\"12\" markerHeight=\"12\" orient=\"auto-start-reverse\">\
         <path d=\"M0,0 L12,6 L0,12 z\" fill=\"white\" stroke=\"{arrow_stroke}\" stroke-width=\"1.5\"/>\
         </marker>",
    ));
    out.push_str(&format!(
        "<marker id=\"arrow-diamond-filled\" viewBox=\"0 0 14 10\" refX=\"13\" refY=\"5\" \
         markerWidth=\"14\" markerHeight=\"10\" orient=\"auto-start-reverse\">\
         <path d=\"M0,5 L7,0 L14,5 L7,10 z\" fill=\"{arrow_stroke}\" stroke=\"{arrow_stroke}\" stroke-width=\"1\"/>\
         </marker>",
    ));
    out.push_str(&format!(
        "<marker id=\"arrow-diamond-open\" viewBox=\"0 0 14 10\" refX=\"13\" refY=\"5\" \
         markerWidth=\"14\" markerHeight=\"10\" orient=\"auto-start-reverse\">\
         <path d=\"M0,5 L7,0 L14,5 L7,10 z\" fill=\"white\" stroke=\"{arrow_stroke}\" stroke-width=\"1\"/>\
         </marker>",
    ));
    out.push_str("</defs>");
}

fn compute_edge_anchors_tuple(
    from: (i32, i32, i32, i32),
    to: (i32, i32, i32, i32),
) -> (i32, i32, i32, i32) {
    let (fx, fy, fw, fh) = from;
    let (tx, ty, tw, th) = to;
    let fcx = fx + fw / 2;
    let fcy = fy + fh / 2;
    let tcx = tx + tw / 2;
    let tcy = ty + th / 2;
    let (x1, y1) = anchor_on_rect(fx, fy, fw, fh, tcx, tcy);
    let (x2, y2) = anchor_on_rect(tx, ty, tw, th, fcx, fcy);
    (x1, y1, x2, y2)
}

fn anchor_on_rect(x: i32, y: i32, w: i32, h: i32, tx: i32, ty: i32) -> (i32, i32) {
    let cx = x + w / 2;
    let cy = y + h / 2;
    let dx = tx - cx;
    let dy = ty - cy;
    if dx == 0 && dy == 0 {
        return (cx, cy);
    }
    // Determine which side to exit
    let half_w = (w as f64) / 2.0;
    let half_h = (h as f64) / 2.0;
    let abs_dx = (dx as f64).abs();
    let abs_dy = (dy as f64).abs();
    if abs_dx * half_h > abs_dy * half_w {
        // Exit via left or right edge
        if dx > 0 {
            (x + w, cy + ((half_w / abs_dx) * (dy as f64)) as i32)
        } else {
            (x, cy + ((half_w / abs_dx) * (dy as f64)) as i32)
        }
    } else if dy > 0 {
        (cx + ((half_h / abs_dy) * (dx as f64)) as i32, y + h)
    } else {
        (cx + ((half_h / abs_dy) * (dx as f64)) as i32, y)
    }
}

/// Backwards-compatible alias; delegates to the real timeline renderer.
pub fn render_timeline_stub_svg(document: &TimelineDocument) -> String {
    render_timeline_svg(document)
}

/// Render Gantt/Chronology timelines as real SVGs:
///   - Gantt: horizontal task bars on a date axis, milestone diamonds,
///     dashed arrows for `requires`/start/etc. constraints between bars.
///   - Chronology: vertical timeline with event bullets along a date axis.
pub fn render_timeline_svg(document: &TimelineDocument) -> String {
    match document.kind {
        DiagramKind::Chronology => render_chronology_svg(document),
        _ => render_gantt_svg(document),
    }
}

fn render_gantt_svg(document: &TimelineDocument) -> String {
    let width: i32 = 800;
    let margin_x: i32 = 32;
    let label_col_w: i32 = 160;
    let bar_height: i32 = 20;
    let row_gap: i32 = 14;
    let header_h: i32 = 28;
    let chart_left: i32 = margin_x + label_col_w + 12;
    let chart_right: i32 = width - margin_x;
    let chart_w: i32 = chart_right - chart_left;

    let title_h = document
        .title
        .as_deref()
        .map(|t| 8 + (t.lines().count() as i32) * 22)
        .unwrap_or(0);

    let row_count = (document.tasks.len() + document.milestones.len()) as i32;
    let chart_top = 40 + title_h + header_h;
    let chart_h = (row_count.max(1)) * (bar_height + row_gap) + 20;
    let total_h = chart_top + chart_h + 40;

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">",
        w = width,
        h = total_h
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");

    // Title
    if let Some(title) = &document.title {
        let mut ty = 28;
        for line in title.lines() {
            out.push_str(&format!(
                "<text x=\"{x}\" y=\"{y}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\" fill=\"#0f172a\">{txt}</text>",
                x = margin_x,
                y = ty,
                txt = escape_text(line)
            ));
            ty += 22;
        }
    } else {
        out.push_str(&format!(
            "<text x=\"{x}\" y=\"28\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\" fill=\"#0f172a\">Gantt</text>",
            x = margin_x
        ));
    }

    // Build column index for tasks + milestones
    let mut row_index: std::collections::BTreeMap<String, i32> = std::collections::BTreeMap::new();
    let mut row_counter: i32 = 0;
    for task in &document.tasks {
        row_index.insert(task.name.clone(), row_counter);
        row_counter += 1;
    }
    let task_count = document.tasks.len() as i32;
    for milestone in &document.milestones {
        row_index.insert(milestone.name.clone(), row_counter);
        row_counter += 1;
    }

    let min_day = document
        .tasks
        .iter()
        .map(|t| t.start_day)
        .min()
        .unwrap_or(0);
    let max_day_exclusive = document
        .tasks
        .iter()
        .map(|t| t.start_day.saturating_add(t.duration_days.max(1)))
        .max()
        .unwrap_or(min_day.saturating_add(1));
    let total_days = max_day_exclusive.saturating_sub(min_day).max(1);
    let tick_count = total_days.clamp(1, 8);

    // Axis header bar
    out.push_str(&format!(
        "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"#f1f5f9\" stroke=\"#cbd5e1\" stroke-width=\"1\"/>",
        x = chart_left,
        y = chart_top - header_h,
        w = chart_w,
        h = header_h
    ));
    for i in 0..=tick_count {
        let day_offset = i.saturating_mul(total_days) / tick_count;
        let x = chart_left + ((chart_w as u32 * day_offset) / total_days) as i32;
        out.push_str(&format!(
            "<line x1=\"{x}\" y1=\"{y1}\" x2=\"{x}\" y2=\"{y2}\" stroke=\"#e2e8f0\" stroke-width=\"1\"/>",
            y1 = chart_top - header_h,
            y2 = chart_top + chart_h
        ));
        out.push_str(&format!(
            "<text x=\"{tx}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"11\" fill=\"#475569\">D+{n}</text>",
            tx = x + 6,
            ty = chart_top - 10,
            n = day_offset
        ));
    }

    let bar_geom = |task: &TimelineTask| -> (i32, i32) {
        let start_offset = task.start_day.saturating_sub(min_day);
        let bx = chart_left + ((chart_w as u32 * start_offset) / total_days) as i32;
        let bw = (((chart_w as u32) * task.duration_days.max(1)) / total_days).max(8) as i32;
        (bx, bw)
    };
    let day_to_x = |day: u32| -> i32 {
        let start_offset = day.saturating_sub(min_day);
        chart_left + ((chart_w as u32 * start_offset) / total_days) as i32
    };
    let task_end_day: std::collections::BTreeMap<&str, u32> = document
        .tasks
        .iter()
        .map(|t| {
            (
                t.name.as_str(),
                t.start_day.saturating_add(t.duration_days.max(1)),
            )
        })
        .collect();

    // Render tasks as horizontal bars
    for (i, task) in document.tasks.iter().enumerate() {
        let row = i as i32;
        let y = chart_top + row * (bar_height + row_gap) + row_gap / 2;
        // Label
        out.push_str(&format!(
            "<text x=\"{x}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"12\" fill=\"#0f172a\">{txt}</text>",
            x = margin_x,
            ty = y + bar_height - 6,
            txt = escape_text(&task.name)
        ));
        let (bx, bw) = bar_geom(task);
        out.push_str(&format!(
            "<rect x=\"{bx}\" y=\"{y}\" width=\"{bw}\" height=\"{bh}\" rx=\"3\" ry=\"3\" fill=\"#3b82f6\" stroke=\"#1e40af\" stroke-width=\"1\"/>",
            bh = bar_height
        ));
    }

    // Render milestones as diamonds (position derived from constraints when possible)
    let mut milestone_day: std::collections::BTreeMap<&str, u32> =
        std::collections::BTreeMap::new();
    for ms in &document.milestones {
        for c in &document.constraints {
            if c.subject != ms.name {
                continue;
            }
            if let Some(task_name) = extract_bracketed_name(&c.target) {
                if let Some(day) = task_end_day.get(task_name.as_str()) {
                    milestone_day.insert(ms.name.as_str(), *day);
                    break;
                }
            }
            if let Some(day) = parse_relative_day(&c.target) {
                milestone_day.insert(ms.name.as_str(), min_day.saturating_add(day));
                break;
            }
            if let Some(abs_day) = parse_iso_date_day_number(&c.target) {
                milestone_day.insert(ms.name.as_str(), abs_day.max(min_day));
                break;
            }
        }
    }

    // Render milestones as diamonds
    for (i, milestone) in document.milestones.iter().enumerate() {
        let row = task_count + i as i32;
        let y = chart_top + row * (bar_height + row_gap) + row_gap / 2;
        let cy = y + bar_height / 2;
        out.push_str(&format!(
            "<text x=\"{x}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"12\" fill=\"#0f172a\">{txt}</text>",
            x = margin_x,
            ty = y + bar_height - 6,
            txt = escape_text(&milestone.name)
        ));
        let cx = milestone_day
            .get(milestone.name.as_str())
            .map(|d| day_to_x(*d))
            .unwrap_or(chart_left + chart_w / 2);
        let r = (bar_height / 2) - 2;
        out.push_str(&format!(
            "<polygon points=\"{x1},{y1} {x2},{y2} {x3},{y3} {x4},{y4}\" fill=\"#facc15\" stroke=\"#854d0e\" stroke-width=\"1.5\"/>",
            x1 = cx,
            y1 = cy - r,
            x2 = cx + r,
            y2 = cy,
            x3 = cx,
            y3 = cy + r,
            x4 = cx - r,
            y4 = cy
        ));
    }

    // Render constraints as arrows between rows
    for constraint in &document.constraints {
        // Only draw if both endpoints exist
        let Some(&from_row) = row_index.get(&constraint.subject) else {
            // Render textual annotation when target row is missing
            continue;
        };
        // Some constraints are "starts <date>" with target being a date string, not a row.
        // We render row-to-row arrows for `requires`-style constraints.
        // The parser includes the keyword in `target` (e.g. "requires [Design]");
        // try to extract a bracketed target name.
        let normalized_target = extract_bracketed_name(&constraint.target)
            .unwrap_or_else(|| constraint.target.trim().to_string());
        let to_row = row_index.get(&normalized_target).copied();
        if let Some(to_row) = to_row {
            let from_y = chart_top + from_row * (bar_height + row_gap) + row_gap / 2 + bar_height;
            let to_y = chart_top + to_row * (bar_height + row_gap) + row_gap / 2;
            let from_task = document.tasks.iter().find(|t| t.name == constraint.subject);
            let to_task = document.tasks.iter().find(|t| t.name == normalized_target);
            let (fx, fw) = from_task.map(bar_geom).unwrap_or((chart_left, 0));
            let (tx, tw) = to_task
                .map(bar_geom)
                .unwrap_or((chart_left + chart_w / 2, 0));
            let x1 = fx + fw / 2;
            let x2 = tx + tw / 2;
            let y1 = from_y;
            let y2 = to_y;
            out.push_str(&format!(
                "<line x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\" stroke=\"#64748b\" stroke-width=\"1.25\" stroke-dasharray=\"4 3\" marker-end=\"url(#gantt-arrow)\"/>",
            ));
        }
    }

    // Constraint arrow marker def
    out.push_str("<defs>");
    out.push_str(
        "<marker id=\"gantt-arrow\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" \
         markerWidth=\"8\" markerHeight=\"8\" orient=\"auto-start-reverse\">\
         <path d=\"M0,0 L10,5 L0,10 z\" fill=\"#64748b\"/>\
         </marker>",
    );
    out.push_str("</defs>");

    // Render textual constraint annotations beneath chart (start/requires with date strings)
    let mut note_y = chart_top + chart_h + 10;
    for constraint in &document.constraints {
        if row_index.contains_key(&constraint.target) {
            continue;
        }
        out.push_str(&format!(
            "<text x=\"{x}\" y=\"{y}\" font-family=\"monospace\" font-size=\"11\" fill=\"#475569\">{s} {k} {t}</text>",
            x = margin_x,
            y = note_y,
            s = escape_text(&constraint.subject),
            k = escape_text(&constraint.kind),
            t = escape_text(&constraint.target)
        ));
        note_y += 14;
    }

    out.push_str("</svg>");
    out
}

fn extract_bracketed_name(target: &str) -> Option<String> {
    let start = target.find('[')?;
    let end = target.rfind(']')?;
    if end <= start + 1 {
        return None;
    }
    Some(target[start + 1..end].trim().to_string())
}

fn parse_relative_day(raw: &str) -> Option<u32> {
    let t = raw.trim();
    let rest = t.strip_prefix("D+").or_else(|| t.strip_prefix("d+"))?;
    rest.trim().parse::<u32>().ok()
}

fn parse_iso_date_tuple(raw: &str) -> Option<(i32, i32, i32)> {
    let mut parts = raw.trim().split('-');
    let y = parts.next()?.parse::<i32>().ok()?;
    let m = parts.next()?.parse::<i32>().ok()?;
    let d = parts.next()?.parse::<i32>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((y, m, d))
}

fn parse_iso_date_day_number(raw: &str) -> Option<u32> {
    let (y, m, d) = parse_iso_date_tuple(raw)?;
    if y < 0 || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let mut days = 0u32;
    for year in 0..y {
        days = days.saturating_add(if is_leap_year(year) { 366 } else { 365 });
    }
    const MONTH: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for mm in 1..m {
        let idx = (mm - 1) as usize;
        days = days.saturating_add(if mm == 2 && is_leap_year(y) {
            29
        } else {
            MONTH[idx]
        });
    }
    days = days.saturating_add((d - 1) as u32);
    Some(days)
}

fn is_leap_year(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}

fn render_chronology_svg(document: &TimelineDocument) -> String {
    let width: i32 = 760;
    let margin_x: i32 = 32;
    let line_x: i32 = margin_x + 60;
    let event_gap: i32 = 56;
    let top_pad: i32 = 60;

    let title_h = document
        .title
        .as_deref()
        .map(|t| 8 + (t.lines().count() as i32) * 22)
        .unwrap_or(0);

    let total_events = document.chronology_events.len() as i32;
    let total_h = top_pad + title_h + total_events.max(1) * event_gap + 60;

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">",
        w = width,
        h = total_h
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");

    // Title
    let mut header_bottom = 28 + title_h;
    if let Some(title) = &document.title {
        let mut ty = 28;
        for line in title.lines() {
            out.push_str(&format!(
                "<text x=\"{x}\" y=\"{y}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\" fill=\"#0f172a\">{txt}</text>",
                x = margin_x,
                y = ty,
                txt = escape_text(line)
            ));
            ty += 22;
        }
    } else {
        out.push_str(&format!(
            "<text x=\"{x}\" y=\"28\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\" fill=\"#0f172a\">Chronology</text>",
            x = margin_x
        ));
        header_bottom = 36;
    }

    // Vertical line
    let line_top = header_bottom + 20;
    let line_bottom = line_top + total_events.max(1) * event_gap;
    out.push_str(&format!(
        "<line x1=\"{x}\" y1=\"{y1}\" x2=\"{x}\" y2=\"{y2}\" stroke=\"#94a3b8\" stroke-width=\"2\"/>",
        x = line_x,
        y1 = line_top,
        y2 = line_bottom
    ));

    // Events (sorted by ISO date when parsable)
    let mut events: Vec<&TimelineChronologyEvent> = document.chronology_events.iter().collect();
    events.sort_by_key(|e| parse_iso_date_tuple(&e.when).unwrap_or((i32::MAX, i32::MAX, i32::MAX)));
    for (i, event) in events.iter().enumerate() {
        let cy = line_top + (i as i32) * event_gap + event_gap / 2;
        let card_y = cy - 16;
        let card_x = line_x + 12;
        out.push_str(&format!(
            "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"28\" rx=\"4\" ry=\"4\" fill=\"{bg}\" stroke=\"#cbd5e1\" stroke-width=\"1\"/>",
            x = card_x,
            y = card_y,
            w = width - card_x - margin_x,
            bg = if i % 2 == 0 { "#ffffff" } else { "#f8fafc" }
        ));
        // Bullet circle
        out.push_str(&format!(
            "<circle cx=\"{cx}\" cy=\"{cy}\" r=\"6\" fill=\"#3b82f6\" stroke=\"#1e40af\" stroke-width=\"1.5\"/>",
            cx = line_x
        ));
        // Date on left
        out.push_str(&format!(
            "<text x=\"{x}\" y=\"{y}\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"11\" fill=\"#475569\">{txt}</text>",
            x = line_x - 14,
            y = cy + 4,
            txt = escape_text(&event.when)
        ));
        // Subject on right
        out.push_str(&format!(
            "<text x=\"{x}\" y=\"{y}\" font-family=\"monospace\" font-size=\"13\" fill=\"#0f172a\">{txt}</text>",
            x = line_x + 20,
            y = cy + 4,
            txt = escape_text(&event.subject)
        ));
    }

    out.push_str("</svg>");
    out
}

pub fn render_component_svg(doc: &FamilyDocument) -> String {
    render_box_grid_svg(doc, "component")
}

pub fn render_deployment_svg(doc: &FamilyDocument) -> String {
    render_box_grid_svg(doc, "deployment")
}

fn render_box_grid_svg(doc: &FamilyDocument, family: &str) -> String {
    // Extract component style (use defaults if not present)
    let comp_style = match &doc.family_style {
        Some(FamilyStyle::Component(s)) => s.clone(),
        _ => ComponentStyle::default(),
    };

    let cols = 3i32;
    let cell_w = 200i32;
    let cell_h = 80i32;
    let margin_x = 40i32;
    let margin_y = 60i32;
    let gap = 40i32;
    let n = doc.nodes.len() as i32;
    let rows = (n + cols - 1) / cols;
    let title_lines = doc
        .title
        .as_deref()
        .map(|v| v.lines().count() as i32)
        .unwrap_or(0);
    let header_h = 40 + title_lines * 22;
    let width = (margin_x * 2) + (cols * cell_w) + ((cols - 1).max(0) * gap);
    let height = header_h + margin_y + (rows.max(1) * cell_h) + ((rows - 1).max(0) * gap) + 60;

    // Position lookup by name and alias.
    let mut positions: std::collections::BTreeMap<String, (i32, i32, i32, i32)> =
        std::collections::BTreeMap::new();

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        width, height, width, height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    render_relation_marker_defs(&mut out, &comp_style.arrow_color);

    let mut y_cursor = 28;
    if let Some(title) = &doc.title {
        for line in title.lines() {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\">{}</text>",
                margin_x,
                y_cursor,
                escape_text(line)
            ));
            y_cursor += 22;
        }
    }
    out.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">{} diagram</text>",
        margin_x,
        y_cursor + 2,
        family
    ));

    for (idx, node) in doc.nodes.iter().enumerate() {
        let col = (idx as i32) % cols;
        let row = (idx as i32) / cols;
        let x = margin_x + col * (cell_w + gap);
        let y = header_h + margin_y + row * (cell_h + gap);
        render_family_node_shape_styled(&mut out, node, x, y, cell_w, cell_h, &comp_style);
        let id_name = node.name.clone();
        let id_alias = node.alias.clone();
        positions.insert(id_name, (x, y, cell_w, cell_h));
        if let Some(alias) = id_alias {
            positions.insert(alias, (x, y, cell_w, cell_h));
        }
    }

    // Draw relations with shared relation-style semantics.
    for rel in &doc.relations {
        let (from_name, to_name, normalized_arrow) =
            normalize_relation_endpoints(&rel.from, &rel.to, &rel.arrow);
        let from_box = positions.get(&from_name);
        let to_box = positions.get(&to_name);
        let (Some(&(fx, fy, fw, fh)), Some(&(tx, ty, tw, th))) = (from_box, to_box) else {
            continue;
        };
        let cx1 = fx + fw / 2;
        let cy1 = fy + fh / 2;
        let cx2 = tx + tw / 2;
        let cy2 = ty + th / 2;
        let (x1, y1) = clip_to_box_edge((cx1, cy1), (cx2, cy2), (fx, fy, fw, fh));
        let (x2, y2) = clip_to_box_edge((cx2, cy2), (cx1, cy1), (tx, ty, tw, th));
        let style = arrow_style(&normalized_arrow);
        let dash = if style.dashed {
            " stroke-dasharray=\"5 3\""
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
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"{}{} />",
            x1, y1, x2, y2, comp_style.arrow_color, dash, markers
        ));
        if let Some(label) = &rel.label {
            let mx = (x1 + x2) / 2;
            let my = (y1 + y2) / 2 - 6;
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#1e293b\">{}</text>",
                mx,
                my,
                escape_text(label)
            ));
        }
        if let Some(left) = &rel.left_cardinality {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"10\" fill=\"#334155\">{}</text>",
                x1 - 4,
                y1 - 6,
                escape_text(left)
            ));
        }
        if let Some(right) = &rel.right_cardinality {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"start\" font-family=\"monospace\" font-size=\"10\" fill=\"#334155\">{}</text>",
                x2 + 4,
                y2 - 6,
                escape_text(right)
            ));
        }
        if let Some(left_role) = &rel.left_role {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"10\" fill=\"#334155\">{}</text>",
                x1 - 4,
                y1 + 12,
                escape_text(left_role)
            ));
        }
        if let Some(right_role) = &rel.right_role {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"start\" font-family=\"monospace\" font-size=\"10\" fill=\"#334155\">{}</text>",
                x2 + 4,
                y2 + 12,
                escape_text(right_role)
            ));
        }
    }

    out.push_str("</svg>");
    out
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

fn clip_to_box_edge(
    center: (i32, i32),
    target: (i32, i32),
    rect: (i32, i32, i32, i32),
) -> (i32, i32) {
    let (cx, cy) = center;
    let (tx, ty) = target;
    let (bx, by, bw, bh) = rect;
    let dx = (tx - cx) as f64;
    let dy = (ty - cy) as f64;
    if dx.abs() < 1e-6 && dy.abs() < 1e-6 {
        return (cx, cy);
    }
    let half_w = (bw as f64) / 2.0;
    let half_h = (bh as f64) / 2.0;
    let scale_x = if dx.abs() > 1e-6 {
        half_w / dx.abs()
    } else {
        f64::INFINITY
    };
    let scale_y = if dy.abs() > 1e-6 {
        half_h / dy.abs()
    } else {
        f64::INFINITY
    };
    let s = scale_x.min(scale_y);
    let ex = (cx as f64) + dx * s;
    let ey = (cy as f64) + dy * s;
    // Keep within box bounds
    let ex = ex.clamp(bx as f64, (bx + bw) as f64);
    let ey = ey.clamp(by as f64, (by + bh) as f64);
    (ex as i32, ey as i32)
}

fn render_family_node_shape(out: &mut String, node: &FamilyNode, x: i32, y: i32, w: i32, h: i32) {
    let cx = x + w / 2;
    let cy = y + h / 2;
    let display = node.label.clone().unwrap_or_else(|| node.name.clone());
    let kind_label = family_node_label(node.kind);

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
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"60\" height=\"14\" fill=\"#fef3c7\" stroke=\"#92400e\" stroke-width=\"1\"/>",
                x, y
            ));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#fef3c7\" stroke=\"#92400e\" stroke-width=\"1\"/>",
                x,
                y + 14,
                w,
                h - 14
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
    out.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"#475569\">{}</text>",
        cx,
        kind_tag_y,
        kind_label
    ));
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

    match node.kind {
        FamilyNodeKind::Interface => {
            let r = 18;
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx, cy, r, comp_style.interface_color, comp_style.border_color
            ));
        }
        FamilyNodeKind::Port => {
            let pw = 24;
            let ph = 24;
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"2\" ry=\"2\" fill=\"#f8fafc\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx - pw / 2,
                cy - ph / 2,
                pw,
                ph,
                comp_style.border_color
            ));
        }
        FamilyNodeKind::Component => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" ry=\"4\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                x, y, w, h, comp_style.background_color, comp_style.border_color
            ));
            // component badges (two small rectangles on the left edge)
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"16\" height=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                x - 4, y + 12, comp_style.background_color, comp_style.border_color
            ));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"16\" height=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                x - 4, y + h - 20, comp_style.background_color, comp_style.border_color
            ));
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
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\">{}</text>",
        label_x, label_y, escape_text(&display)
    ));
    let kind_tag_y = match node.kind {
        FamilyNodeKind::Interface | FamilyNodeKind::Port => label_y + 14,
        _ => y + 14,
    };
    out.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"#475569\">{}</text>",
        cx, kind_tag_y, kind_label
    ));
}

pub fn render_activity_svg(doc: &FamilyDocument) -> String {
    // Extract activity style (use defaults if not present)
    let act_style = match &doc.family_style {
        Some(FamilyStyle::Activity(s)) => s.clone(),
        _ => ActivityStyle::default(),
    };

    let width = 480;
    let step_h = 60;
    let n = doc.nodes.len() as i32;
    let title_lines = doc
        .title
        .as_deref()
        .map(|v| v.lines().count() as i32)
        .unwrap_or(0);
    let header_h = 40 + title_lines * 22;
    let height = header_h + n.max(1) * step_h + 60;

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        width, height, width, height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");

    let mut y_cursor = 28;
    if let Some(title) = &doc.title {
        for line in title.lines() {
            out.push_str(&format!(
                "<text x=\"32\" y=\"{}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\">{}</text>",
                y_cursor,
                escape_text(line)
            ));
            y_cursor += 22;
        }
    }
    out.push_str(&format!(
        "<text x=\"32\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">activity diagram</text>",
        y_cursor + 2
    ));

    let mut lanes: Vec<String> = Vec::new();
    for node in &doc.nodes {
        if let Some(alias) = &node.alias {
            if let Some(meta) = alias.strip_prefix("activity::") {
                for part in meta.split('|') {
                    if let Some(lane) = part.strip_prefix("lane=") {
                        if lane != "default" && !lanes.iter().any(|l| l == lane) {
                            lanes.push(lane.to_string());
                        }
                    }
                }
            }
        }
    }
    if lanes.is_empty() {
        lanes.push("default".to_string());
    }

    let lane_area_x = 32i32;
    let lane_area_w = width - 64;
    let lane_w = (lane_area_w / (lanes.len() as i32)).max(120);
    let lane_index = |name: &str| -> i32 {
        lanes
            .iter()
            .position(|l| l == name)
            .map(|i| i as i32)
            .unwrap_or(0)
    };
    let lane_center = |idx: i32| -> i32 { lane_area_x + idx * lane_w + lane_w / 2 };
    let lane_left = |idx: i32| -> i32 { lane_area_x + idx * lane_w };

    for (idx, lane) in lanes.iter().enumerate() {
        let lx = lane_left(idx as i32);
        let bg = if idx % 2 == 0 { "#f8fafc" } else { "#f1f5f9" };
        out.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"#cbd5e1\" stroke-width=\"1\" stroke-dasharray=\"4 3\"/>",
            lx,
            header_h - 8,
            lane_w,
            height - header_h - 20,
            bg
        ));
        if lane != "default" {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#334155\">{}</text>",
                lx + lane_w / 2,
                header_h + 10,
                escape_text(lane)
            ));
        }
    }

    let box_w = (lane_w - 24).clamp(120, 220);
    let mut last_point: Option<(i32, i32)> = None;
    let mut fork_anchor: Option<(i32, i32)> = None;

    for (idx, node) in doc.nodes.iter().enumerate() {
        let y = header_h + (idx as i32) * step_h;
        let label = node.label.clone().unwrap_or_default();
        let mut lane_name = "default".to_string();
        let mut step_kind = String::new();
        let mut fork_branch = 0usize;
        if let Some(alias) = &node.alias {
            if let Some(meta) = alias.strip_prefix("activity::") {
                for (pi, part) in meta.split('|').enumerate() {
                    if pi == 0 {
                        step_kind = part.to_string();
                        continue;
                    }
                    if let Some(v) = part.strip_prefix("lane=") {
                        lane_name = v.to_string();
                    } else if let Some(v) = part.strip_prefix("fork_branch=") {
                        fork_branch = v.parse::<usize>().unwrap_or(0);
                    }
                }
            }
        }
        let cx = lane_center(lane_index(&lane_name));
        match node.kind {
            FamilyNodeKind::ActivityStart => {
                out.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"12\" fill=\"{}\"/>",
                    cx,
                    y + 20,
                    act_style.fork_color
                ));
            }
            FamilyNodeKind::ActivityStop => {
                out.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"14\" fill=\"white\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                    cx,
                    y + 20,
                    act_style.fork_color
                ));
                out.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"7\" fill=\"{}\"/>",
                    cx,
                    y + 20,
                    act_style.fork_color
                ));
            }
            FamilyNodeKind::ActivityAction => {
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"36\" rx=\"18\" ry=\"18\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                    cx - box_w / 2,
                    y + 4,
                    box_w,
                    act_style.background_color,
                    act_style.border_color
                ));
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\">{}</text>",
                    cx,
                    y + 27,
                    escape_text(&label)
                ));
            }
            FamilyNodeKind::ActivityDecision => {
                // diamond
                let dx = 100;
                let dy = 22;
                out.push_str(&format!(
                    "<polygon points=\"{},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                    cx,
                    y + 2,
                    cx + dx,
                    y + 2 + dy,
                    cx,
                    y + 2 + (dy * 2),
                    cx - dx,
                    y + 2 + dy,
                    act_style.diamond_color,
                    act_style.border_color
                ));
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\">{}</text>",
                    cx,
                    y + 2 + dy + 4,
                    escape_text(&label)
                ));
                if step_kind.contains("WhileStart") {
                    out.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"#475569\">while</text>",
                        cx,
                        y + 54
                    ));
                }
                if step_kind.contains("RepeatWhile") {
                    out.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"#475569\">repeat while</text>",
                        cx,
                        y + 54
                    ));
                }
            }
            FamilyNodeKind::ActivityFork | FamilyNodeKind::ActivityForkEnd => {
                let bar_w = box_w;
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"8\" fill=\"{}\"/>",
                    cx - bar_w / 2,
                    y + 24,
                    bar_w,
                    act_style.fork_color
                ));
                if step_kind.contains("ForkAgain") {
                    out.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"#475569\">branch {}</text>",
                        cx,
                        y + 20,
                        fork_branch + 1
                    ));
                }
                if step_kind.contains("Fork") && !step_kind.contains("ForkAgain") {
                    fork_anchor = Some((cx, y + 28));
                }
                if step_kind.contains("EndFork") {
                    fork_anchor = None;
                }
            }
            FamilyNodeKind::ActivityMerge => {
                let merge_label = if step_kind.contains("Else") {
                    format!("(else) {}", label)
                } else if step_kind.contains("EndIf") {
                    "(endif)".to_string()
                } else if step_kind.contains("EndWhile") {
                    "(endwhile)".to_string()
                } else if step_kind.contains("RepeatStart") {
                    "(repeat)".to_string()
                } else {
                    format!("(merge) {}", label)
                };
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#475569\">{}</text>",
                    cx,
                    y + 28,
                    escape_text(&merge_label)
                ));
            }
            FamilyNodeKind::ActivityPartition => {
                out.push_str(&format!(
                    "<rect x=\"24\" y=\"{}\" width=\"{}\" height=\"36\" rx=\"4\" ry=\"4\" fill=\"#e2e8f0\" stroke=\"#475569\" stroke-width=\"1\" stroke-dasharray=\"4 3\"/>",
                    y + 4,
                    width - 48
                ));
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\" font-weight=\"600\" fill=\"#1e293b\">{}</text>",
                    cx,
                    y + 27,
                    escape_text(&format!("partition: {}", label))
                ));
            }
            _ => {
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\">{}</text>",
                    cx,
                    y + 28,
                    escape_text(&label)
                ));
            }
        }
        // arrow from previous
        if let Some((px, py)) = last_point {
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                px,
                py,
                cx,
                y,
                act_style.arrow_color
            ));
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\"/>",
                cx,
                y,
                cx - 4,
                y - 6,
                cx + 4,
                y - 6,
                act_style.arrow_color
            ));
        }
        if let Some((fx, fy)) = fork_anchor {
            if step_kind.contains("ForkAgain") || fork_branch > 0 {
                out.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.25\" stroke-dasharray=\"4 2\"/>",
                    fx,
                    fy,
                    cx,
                    y,
                    act_style.arrow_color
                ));
            }
        }
        last_point = Some((cx, y + 42));
    }

    out.push_str("</svg>");
    out
}

fn timing_state_color(state: &str, idx: usize) -> &'static str {
    // Map well-known digital states first.
    let lower = state.to_ascii_lowercase();
    if lower == "high" || lower == "1" {
        return "#bbf7d0"; // green-100
    }
    if lower == "low" || lower == "0" {
        return "#fecaca"; // red-100
    }
    if lower == "undef" || lower == "x" || lower == "z" {
        return "#e2e8f0"; // slate-200
    }
    // Otherwise cycle through a palette.
    const PALETTE: &[&str] = &[
        "#bfdbfe", // blue-200
        "#ddd6fe", // violet-200
        "#fde68a", // amber-200
        "#a7f3d0", // emerald-200
        "#fca5a5", // red-300
        "#6ee7b7", // emerald-300
        "#93c5fd", // blue-300
        "#c4b5fd", // violet-300
    ];
    PALETTE[idx % PALETTE.len()]
}

pub fn render_timing_svg(doc: &FamilyDocument) -> String {
    let default_timing_style;
    let style = match &doc.family_style {
        Some(crate::model::FamilyStyle::Timing(style)) => style,
        _ => {
            default_timing_style = crate::theme::TimingStyle::default();
            &default_timing_style
        }
    };
    // ── Collect signals and events ────────────────────────────────────────────
    let signals: Vec<&FamilyNode> = doc
        .nodes
        .iter()
        .filter(|n| {
            matches!(
                n.kind,
                FamilyNodeKind::TimingConcise
                    | FamilyNodeKind::TimingRobust
                    | FamilyNodeKind::TimingClock
                    | FamilyNodeKind::TimingBinary
            )
        })
        .collect();
    let events: Vec<&FamilyNode> = doc
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, FamilyNodeKind::TimingEvent))
        .collect();
    let global_events: Vec<(i64, String)> = events
        .iter()
        .filter_map(|e| {
            if e.alias.is_some() {
                return None;
            }
            let t = e.name.parse::<i64>().ok()?;
            let txt = e
                .label
                .clone()
                .or_else(|| e.members.first().map(|m| m.text.clone()))
                .unwrap_or_default();
            if txt.is_empty() {
                None
            } else {
                Some((t, txt))
            }
        })
        .collect();

    // ── Parse time positions (@N) ─────────────────────────────────────────────
    // Collect unique numeric time values, sort them.
    let mut time_vals: Vec<i64> = events
        .iter()
        .filter_map(|e| e.name.parse::<i64>().ok())
        .collect();
    time_vals.sort();
    time_vals.dedup();
    if time_vals.is_empty() {
        time_vals = vec![0, 10];
    }

    let t_min = *time_vals.first().unwrap();
    let t_max = *time_vals.last().unwrap();
    let t_span = (t_max - t_min).max(1);

    // ── Layout constants ──────────────────────────────────────────────────────
    let left_pad: i32 = 130; // signal name column width
    let right_pad: i32 = 32;
    let row_h: i32 = 64;
    let wave_top_pad: i32 = 10; // space above wave line inside row
    let wave_bot_pad: i32 = 10; // space below wave line inside row
    let wave_h: i32 = row_h - wave_top_pad - wave_bot_pad; // usable wave height
    let axis_h: i32 = 48;
    let chart_w: i32 = 760;
    let width: i32 = left_pad + chart_w + right_pad;

    // 22px title lines + 14px subtitle + 10px padding
    let title_h: i32 = doc
        .title
        .as_deref()
        .map(|t| (t.lines().count() as i32) * 22 + 10)
        .unwrap_or(0)
        + 14; // subtitle line

    let n_signals = signals.len().max(1) as i32;
    let height: i32 = title_h + axis_h + n_signals * row_h + 32;

    // Map a time value to an x coordinate in the chart area.
    let time_to_x =
        |t: i64| -> i32 { left_pad + ((t - t_min) as f64 / t_span as f64 * chart_w as f64) as i32 };

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">"
    ));
    out.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>",
        escape_text(&style.background_color)
    ));
    out.push_str(&format!(
        "<metadata data-timing-style=\"{} {} {} {} {} {} {}\"/>",
        escape_text(&style.background_color),
        escape_text(&style.axis_color),
        escape_text(&style.grid_color),
        escape_text(&style.signal_background_color),
        escape_text(&style.signal_border_color),
        escape_text(&style.arrow_color),
        escape_text(&style.font_color)
    ));

    // ── Title ─────────────────────────────────────────────────────────────────
    let mut ty = 22i32;
    if let Some(title) = &doc.title {
        for line in title.lines() {
            out.push_str(&format!(
                "<text x=\"24\" y=\"{ty}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\" fill=\"{}\">{}</text>",
                escape_text(&style.font_color),
                escape_text(line)
            ));
            ty += 22;
        }
    }
    // Subtitle: always emit "timing diagram" so downstream checks/tests can rely on it.
    out.push_str(&format!(
        "<text x=\"24\" y=\"{ty}\" font-family=\"monospace\" font-size=\"11\" fill=\"#94a3b8\">timing diagram</text>",
    ));
    ty += 14;
    let axis_top = ty + 4;
    let signals_top = axis_top + axis_h;

    // ── Time axis ─────────────────────────────────────────────────────────────
    // Background strip for time axis
    out.push_str(&format!(
        "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
        escape_text(&style.signal_background_color),
        escape_text(&style.grid_color),
        x = left_pad,
        y = axis_top,
        w = chart_w,
        h = axis_h
    ));

    // Major ticks at each @N position
    let rows_h = n_signals * row_h;
    for &t in &time_vals {
        let tx = time_to_x(t);
        // Gridline through all signal rows
        out.push_str(&format!(
            "<line x1=\"{tx}\" y1=\"{y1}\" x2=\"{tx}\" y2=\"{y2}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"4 3\"/>",
            escape_text(&style.grid_color),
            y1 = signals_top,
            y2 = signals_top + rows_h
        ));
        // Tick mark on axis
        out.push_str(&format!(
            "<line x1=\"{tx}\" y1=\"{y1}\" x2=\"{tx}\" y2=\"{y2}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            escape_text(&style.axis_color),
            y1 = axis_top + axis_h - 8,
            y2 = axis_top + axis_h
        ));
        // Label
        out.push_str(&format!(
            "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">@{t}</text>",
            escape_text(&style.font_color),
            ty = axis_top + 20
        ));
    }

    for (t, note) in &global_events {
        let tx = time_to_x(*t);
        out.push_str(&format!(
            "<circle cx=\"{tx}\" cy=\"{cy}\" r=\"3\" fill=\"{}\"/>",
            escape_text(&style.arrow_color),
            cy = axis_top + 8
        ));
        out.push_str(&format!(
            "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"{}\">{}</text>",
            escape_text(&style.font_color),
            escape_text(note),
            ty = axis_top + 10
        ));
    }

    // Minor ticks at midpoints between adjacent time positions
    for w in time_vals.windows(2) {
        let mid = (w[0] + w[1]) / 2;
        let mx = time_to_x(mid);
        out.push_str(&format!(
            "<line x1=\"{mx}\" y1=\"{y1}\" x2=\"{mx}\" y2=\"{y2}\" stroke=\"{}\" stroke-width=\"0.75\"/>",
            escape_text(&style.axis_color),
            y1 = axis_top + axis_h - 4,
            y2 = axis_top + axis_h
        ));
    }

    // ── Signal rows ───────────────────────────────────────────────────────────
    for (row_idx, signal) in signals.iter().enumerate() {
        let row_y = signals_top + (row_idx as i32) * row_h;
        let wave_y_hi = row_y + wave_top_pad; // y for logical HIGH
        let wave_y_lo = row_y + wave_top_pad + wave_h; // y for logical LOW
        let wave_mid = (wave_y_hi + wave_y_lo) / 2;

        // Row background (alternating)
        let row_bg = if row_idx % 2 == 0 {
            "#ffffff"
        } else {
            "#f8fafc"
        };
        out.push_str(&format!(
            "<rect x=\"0\" y=\"{row_y}\" width=\"{width}\" height=\"{row_h}\" fill=\"{row_bg}\"/>",
        ));

        // Signal name label (left column)
        out.push_str(&format!(
            "<text x=\"{x}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"12\" font-weight=\"600\" fill=\"{}\" text-anchor=\"end\">{name}</text>",
            escape_text(&style.font_color),
            x = left_pad - 8,
            ty = wave_mid + 4,
            name = escape_text(&signal.name)
        ));
        // Signal kind tag
        out.push_str(&format!(
            "<text x=\"{x}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"9\" fill=\"#94a3b8\" text-anchor=\"end\">{kind}</text>",
            x = left_pad - 8,
            ty = wave_mid + 16,
            kind = family_node_label(signal.kind)
        ));

        // Collect events for this signal, sorted by time.
        let mut sig_events: Vec<(i64, String)> = events
            .iter()
            .filter(|e| e.alias.as_deref() == Some(signal.name.as_str()))
            .filter_map(|e| {
                let t = e.name.parse::<i64>().ok()?;
                let state = e
                    .members
                    .first()
                    .map(|m| m.text.clone())
                    .unwrap_or_default();
                Some((t, state))
            })
            .collect();
        sig_events.sort_by_key(|(t, _)| *t);

        // Row separator line at bottom
        out.push_str(&format!(
            "<line x1=\"0\" y1=\"{y}\" x2=\"{width}\" y2=\"{y}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
            escape_text(&style.grid_color),
            y = row_y + row_h
        ));

        match signal.kind {
            FamilyNodeKind::TimingBinary => {
                // Binary: flat baseline with vertical pulses at @N positions.
                // HIGH=1/high, LOW=0/low; default LOW if no state.
                let is_high = |s: &str| -> bool {
                    let l = s.to_ascii_lowercase();
                    matches!(l.as_str(), "1" | "high" | "on" | "true")
                };

                // Draw the waveform as segments between events.
                let mut segments: Vec<(i64, i64, bool)> = Vec::new();
                let end_t = t_max + (t_span as f64 * 0.05) as i64 + 1;
                if sig_events.is_empty() {
                    segments.push((t_min, end_t, false));
                } else {
                    // Before first event: assume low
                    segments.push((t_min, sig_events[0].0, false));
                    for i in 0..sig_events.len() {
                        let t_start = sig_events[i].0;
                        let t_end = sig_events.get(i + 1).map(|(t, _)| *t).unwrap_or(end_t);
                        segments.push((t_start, t_end, is_high(&sig_events[i].1)));
                    }
                }

                let mut path = String::from("M ");
                let mut first_seg = true;
                let mut cur_hi = false;
                for (ts, te, hi) in &segments {
                    let x1 = time_to_x(*ts);
                    let x2 = time_to_x(*te);
                    let cy = if *hi { wave_y_hi } else { wave_y_lo };
                    if first_seg {
                        path.push_str(&format!("{x1},{cy} "));
                        first_seg = false;
                        cur_hi = *hi;
                    } else if *hi != cur_hi {
                        // Vertical transition
                        path.push_str(&format!("L {x1},{cy} "));
                        cur_hi = *hi;
                    }
                    path.push_str(&format!("L {x2},{cy} "));
                }
                out.push_str(&format!(
                    "<polyline points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"2\"/>",
                    path.replace("M ", "").replace("L ", "")
                    , escape_text(&style.signal_border_color)
                ));

                // Pulse labels
                for (t, state) in &sig_events {
                    let lx = time_to_x(*t);
                    let label_ty = wave_y_hi - 4;
                    out.push_str(&format!(
                        "<text x=\"{lx}\" y=\"{label_ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"#475569\">{}</text>",
                        escape_text(state)
                    ));
                }
            }

            FamilyNodeKind::TimingClock => {
                // Clock: square wave. If edge events exist for this signal, use
                // their spacing as the period baseline; otherwise fallback.
                let period = if sig_events.len() >= 2 {
                    (sig_events[1].0 - sig_events[0].0).max(1)
                } else if time_vals.len() >= 2 {
                    (time_vals[1] - time_vals[0]).max(1)
                } else {
                    t_span / 4
                };
                let half = period / 2;
                let t_end = t_max + period;

                let mut path_pts = String::new();
                let mut cur_t = t_min;
                let mut cur_hi = sig_events
                    .first()
                    .map(|(_, s)| {
                        let l = s.to_ascii_lowercase();
                        matches!(l.as_str(), "high" | "1" | "on" | "true")
                    })
                    .unwrap_or(true);
                let x0 = time_to_x(cur_t);
                let y0 = if cur_hi { wave_y_hi } else { wave_y_lo };
                path_pts.push_str(&format!("{x0},{y0}"));
                while cur_t < t_end {
                    let next_t = cur_t + half;
                    let x1 = time_to_x(next_t);
                    // Horizontal segment
                    let cur_y = if cur_hi { wave_y_hi } else { wave_y_lo };
                    path_pts.push_str(&format!(" {x1},{cur_y}"));
                    // Vertical transition
                    cur_hi = !cur_hi;
                    let next_y = if cur_hi { wave_y_hi } else { wave_y_lo };
                    path_pts.push_str(&format!(" {x1},{next_y}"));
                    cur_t = next_t;
                }
                out.push_str(&format!(
                    "<polyline points=\"{path_pts}\" fill=\"none\" stroke=\"{}\" stroke-width=\"2\"/>",
                    escape_text(&style.signal_border_color),
                ));
                // Clock label
                out.push_str(&format!(
                    "<text x=\"{x}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"10\" fill=\"#64748b\">clk</text>",
                    x = time_to_x(t_min) + 4,
                    ty = wave_y_hi - 4
                ));
            }

            FamilyNodeKind::TimingRobust => {
                // Robust: same as concise but with coloured fills per unique state.
                // Build unique state → colour map.
                let mut state_order: Vec<String> = Vec::new();
                for (_, state) in &sig_events {
                    if !state_order.contains(state) {
                        state_order.push(state.clone());
                    }
                }
                let state_color_idx =
                    |s: &str| -> usize { state_order.iter().position(|x| x == s).unwrap_or(0) };

                let end_t = t_max + (t_span as f64 * 0.05) as i64 + 1;
                let transition_w = 6i32; // slant width in px

                if sig_events.is_empty() {
                    // Flat unknown line
                    out.push_str(&format!(
                        "<line x1=\"{x1}\" y1=\"{wave_mid}\" x2=\"{x2}\" y2=\"{wave_mid}\" stroke=\"#94a3b8\" stroke-width=\"1.5\"/>",
                        x1 = time_to_x(t_min),
                        x2 = time_to_x(end_t)
                    ));
                } else {
                    // Render coloured state boxes with slanted transitions.
                    for i in 0..sig_events.len() {
                        let (t_start, ref state) = sig_events[i];
                        let t_end = sig_events.get(i + 1).map(|(t, _)| *t).unwrap_or(end_t);
                        let x1 = time_to_x(t_start);
                        let x2 = time_to_x(t_end);
                        let cidx = state_color_idx(state);
                        let fill = timing_state_color(state, cidx);

                        // Filled parallelogram-ish box
                        let pts = format!(
                            "{},{} {},{} {},{} {},{}",
                            x1 + transition_w,
                            wave_y_hi,
                            x2,
                            wave_y_hi,
                            x2 - transition_w,
                            wave_y_lo,
                            x1,
                            wave_y_lo
                        );
                        out.push_str(&format!(
                            "<polygon points=\"{pts}\" fill=\"{fill}\" stroke=\"#475569\" stroke-width=\"1.5\"/>",
                        ));

                        // State label centred in box
                        let label_x = (x1 + x2) / 2;
                        let label_ty = wave_mid + 4;
                        out.push_str(&format!(
                            "<text x=\"{label_x}\" y=\"{label_ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#0f172a\" font-weight=\"600\">{}</text>",
                            escape_text(state)
                        ));
                    }
                }
            }

            // TimingConcise (default)
            _ => {
                // Concise: state-name boxes with sharp vertical transitions.
                let end_t = t_max + (t_span as f64 * 0.05) as i64 + 1;

                if sig_events.is_empty() {
                    out.push_str(&format!(
                        "<line x1=\"{x1}\" y1=\"{wave_mid}\" x2=\"{x2}\" y2=\"{wave_mid}\" stroke=\"#94a3b8\" stroke-width=\"1.5\" stroke-dasharray=\"4 3\"/>",
                        x1 = time_to_x(t_min),
                        x2 = time_to_x(end_t)
                    ));
                } else {
                    // Top and bottom border lines for each segment.
                    for i in 0..sig_events.len() {
                        let (t_start, ref state) = sig_events[i];
                        let t_end = sig_events.get(i + 1).map(|(t, _)| *t).unwrap_or(end_t);
                        let x1 = time_to_x(t_start);
                        let x2 = time_to_x(t_end);

                        // Top border
                        out.push_str(&format!(
                            "<line x1=\"{x1}\" y1=\"{wave_y_hi}\" x2=\"{x2}\" y2=\"{wave_y_hi}\" stroke=\"#0f172a\" stroke-width=\"1.5\"/>",
                        ));
                        // Bottom border
                        out.push_str(&format!(
                            "<line x1=\"{x1}\" y1=\"{wave_y_lo}\" x2=\"{x2}\" y2=\"{wave_y_lo}\" stroke=\"#0f172a\" stroke-width=\"1.5\"/>",
                        ));
                        // Left vertical edge (transition)
                        out.push_str(&format!(
                            "<line x1=\"{x1}\" y1=\"{wave_y_hi}\" x2=\"{x1}\" y2=\"{wave_y_lo}\" stroke=\"#0f172a\" stroke-width=\"1.5\"/>",
                        ));

                        // State label centred in box
                        let label_x = (x1 + x2) / 2;
                        let label_ty = wave_mid + 4;
                        out.push_str(&format!(
                            "<text x=\"{label_x}\" y=\"{label_ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#1e293b\">{}</text>",
                            escape_text(state)
                        ));
                    }
                    // Right closing edge
                    let last_x = time_to_x(end_t);
                    out.push_str(&format!(
                        "<line x1=\"{last_x}\" y1=\"{wave_y_hi}\" x2=\"{last_x}\" y2=\"{wave_y_lo}\" stroke=\"#0f172a\" stroke-width=\"1.5\"/>",
                    ));
                }
            }
        }
    }

    out.push_str("</svg>");
    out
}

pub fn render_json_svg(document: &JsonDocument) -> String {
    let width = 760;
    let height = 80 + (document.nodes.len().max(1) as i32) * 22;
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        width, height, width, height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    let mut y = 28;
    out.push_str(&format!(
        "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\">{}</text>",
        y,
        escape_text(document.title.as_deref().unwrap_or("JSON"))
    ));
    y += 28;
    if document.nodes.is_empty() {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">(empty)</text>",
            y
        ));
    } else {
        for node in &document.nodes {
            let x = 24 + (node.depth as i32) * 18;
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"18\" rx=\"3\" ry=\"3\" fill=\"#f1f5f9\" stroke=\"#94a3b8\" stroke-width=\"1\"/>",
                x,
                y - 12,
                (width - 48 - (node.depth as i32) * 18).max(80)
            ));
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#0f172a\">{}</text>",
                x + 6,
                y + 2,
                escape_text(&node.label)
            ));
            y += 22;
        }
    }
    out.push_str("</svg>");
    out
}

// ─── State diagram renderer ──────────────────────────────────────────────────

/// Layout constants for state SVG rendering.
const STATE_NODE_W: i32 = 140;
const STATE_NODE_H: i32 = 40;
const STATE_NODE_GAP_X: i32 = 60;
const STATE_NODE_GAP_Y: i32 = 70;
const STATE_MARGIN: i32 = 30;
const _STATE_ARROW_LEN: i32 = 40;

pub fn render_state_svg(document: &StateDocument) -> String {
    // Simple left-to-right column layout: all top-level nodes in one or two columns,
    // then draw transitions as arrows.
    let nodes = &document.nodes;
    let transitions = &document.transitions;
    let state_style = &document.state_style;

    // Assign coordinates to each node
    let mut node_coords: std::collections::BTreeMap<String, (i32, i32)> =
        std::collections::BTreeMap::new();
    let cols = 2i32;
    for (idx, node) in nodes.iter().enumerate() {
        let col = (idx as i32) % cols;
        let row = (idx as i32) / cols;
        let x = STATE_MARGIN + col * (STATE_NODE_W + STATE_NODE_GAP_X);
        let y = STATE_MARGIN + row * (STATE_NODE_H + STATE_NODE_GAP_Y) + 50;
        node_coords.insert(node.name.clone(), (x, y));
    }

    let node_count = nodes.len() as i32;
    let rows = (node_count + cols - 1) / cols;
    let width = STATE_MARGIN * 2 + cols * (STATE_NODE_W + STATE_NODE_GAP_X);
    let height = STATE_MARGIN * 2 + rows * (STATE_NODE_H + STATE_NODE_GAP_Y) + 80;

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        width, height, width, height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    out.push_str(&format!(
        "<defs><marker id=\"arrow\" markerWidth=\"8\" markerHeight=\"8\" refX=\"6\" refY=\"3\" orient=\"auto\"><path d=\"M0,0 L0,6 L8,3 z\" fill=\"{}\"/></marker></defs>",
        state_style.arrow_color
    ));

    // Title
    let mut y_header = 28i32;
    if let Some(title) = &document.title {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\" text-anchor=\"middle\">{}</text>",
            width / 2,
            y_header,
            escape_text(title)
        ));
        y_header += 20;
    }
    out.push_str(&format!(
        "<text x=\"40\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">state diagram</text>",
        y_header
    ));

    let mut incoming: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    let mut outgoing: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for t in transitions {
        *incoming.entry(t.to.as_str()).or_insert(0) += 1;
        *outgoing.entry(t.from.as_str()).or_insert(0) += 1;
    }

    // Draw transitions (arrows) first so nodes appear on top
    for t in transitions {
        let from_coord = node_coords.get(&t.from);
        let to_coord = node_coords.get(&t.to);
        let from_node = nodes.iter().find(|n| n.name == t.from);
        let to_node = nodes.iter().find(|n| n.name == t.to);
        if let (Some(&(fx, fy)), Some(&(tx, ty)), Some(from_node), Some(to_node)) =
            (from_coord, to_coord, from_node, to_node)
        {
            // Compute start/end points at node boundaries
            let (x1, y1, x2, y2) = transition_endpoints(from_node, fx, fy, to_node, tx, ty);
            if t.from == t.to {
                let loop_rx = 18;
                let loop_ry = 14;
                let cpx = x1 + loop_rx;
                let cpy = y1 - loop_ry;
                out.push_str(&format!(
                    "<path d=\"M {x1} {y1} Q {cpx} {cpy} {x2} {y2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\" marker-end=\"url(#arrow)\"/>",
                    state_style.arrow_color
                ));
            } else {
                out.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\" marker-end=\"url(#arrow)\"/>",
                    x1, y1, x2, y2, state_style.arrow_color
                ));
            }
            if let Some(label) = &t.label {
                let mx = (x1 + x2) / 2;
                let my = (y1 + y2) / 2 - 6;
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#555\" text-anchor=\"middle\">{}</text>",
                    mx, my, escape_text(label)
                ));
            }
        }
    }

    // Draw nodes — pass state_style for coloring
    for node in nodes {
        if let Some(&(x, y)) = node_coords.get(&node.name) {
            render_state_node_svg_styled(
                &mut out,
                node,
                x,
                y,
                state_style,
                *incoming.get(node.name.as_str()).unwrap_or(&0),
                *outgoing.get(node.name.as_str()).unwrap_or(&0),
            );
        }
    }

    out.push_str("</svg>");
    out
}

pub fn render_yaml_svg(document: &YamlDocument) -> String {
    let width = 760;
    let height = 80 + (document.nodes.len().max(1) as i32) * 22;
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        width, height, width, height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    let mut y = 28;
    out.push_str(&format!(
        "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\">{}</text>",
        y,
        escape_text(document.title.as_deref().unwrap_or("YAML"))
    ));
    y += 28;
    if document.nodes.is_empty() {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">(empty)</text>",
            y
        ));
    } else {
        for node in &document.nodes {
            let x = 24 + (node.depth as i32) * 18;
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"18\" rx=\"3\" ry=\"3\" fill=\"#fef9c3\" stroke=\"#ca8a04\" stroke-width=\"1\"/>",
                x,
                y - 12,
                (width - 48 - (node.depth as i32) * 18).max(80)
            ));
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#0f172a\">{}</text>",
                x + 6,
                y + 2,
                escape_text(&node.label)
            ));
            y += 22;
        }
    }
    out.push_str("</svg>");
    out
}

pub fn render_nwdiag_svg(document: &NwdiagDocument) -> String {
    let width = 760;
    let net_rows: i32 = document
        .networks
        .iter()
        .map(|n| 1 + n.nodes.len() as i32)
        .sum();
    let height = 80 + net_rows.max(1) * 24 + (document.networks.len() as i32) * 14;
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        width, height, width, height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    let mut y = 28;
    out.push_str(&format!(
        "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\">{}</text>",
        y,
        escape_text(document.title.as_deref().unwrap_or("Network diagram"))
    ));
    y += 24;
    if document.networks.is_empty() {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">(no networks)</text>",
            y
        ));
    } else {
        for net in &document.networks {
            // Swimlane header
            out.push_str(&format!(
                "<rect x=\"24\" y=\"{}\" width=\"712\" height=\"22\" fill=\"#e0f2fe\" stroke=\"#0284c7\" stroke-width=\"1\"/>",
                y
            ));
            let label = match &net.address {
                Some(a) => format!("network {} ({})", net.name, a),
                None => format!("network {}", net.name),
            };
            out.push_str(&format!(
                "<text x=\"32\" y=\"{}\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" fill=\"#0c4a6e\">{}</text>",
                y + 16,
                escape_text(&label)
            ));
            y += 26;
            for node in &net.nodes {
                out.push_str(&format!(
                    "<rect x=\"56\" y=\"{}\" width=\"680\" height=\"20\" rx=\"3\" ry=\"3\" fill=\"white\" stroke=\"#0284c7\" stroke-width=\"1\"/>",
                    y
                ));
                let lbl = match &node.address {
                    Some(a) => format!("{} [{}]", node.name, a),
                    None => node.name.clone(),
                };
                out.push_str(&format!(
                    "<text x=\"66\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#0f172a\">{}</text>",
                    y + 14,
                    escape_text(&lbl)
                ));
                y += 24;
            }
            y += 10;
        }
    }
    out.push_str("</svg>");
    out
}

pub fn render_archimate_svg(document: &ArchimateDocument) -> String {
    let width = 760;
    let layers = [
        "strategy",
        "business",
        "application",
        "technology",
        "motivation",
    ];
    let lane_height = 80;
    let height = 80 + (layers.len() as i32) * lane_height + (document.relations.len() as i32) * 18;
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        width, height, width, height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    let mut y = 28;
    out.push_str(&format!(
        "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\">{}</text>",
        y,
        escape_text(document.title.as_deref().unwrap_or("Archimate"))
    ));
    y += 16;
    for layer in layers.iter() {
        let layer_y = y;
        let bg = match *layer {
            "strategy" => "#fee2e2",
            "business" => "#fef3c7",
            "application" => "#dbeafe",
            "technology" => "#dcfce7",
            "motivation" => "#ede9fe",
            _ => "#f1f5f9",
        };
        out.push_str(&format!(
            "<rect x=\"24\" y=\"{}\" width=\"712\" height=\"{}\" fill=\"{}\" stroke=\"#94a3b8\" stroke-width=\"1\"/>",
            layer_y, lane_height, bg
        ));
        out.push_str(&format!(
            "<text x=\"32\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#475569\">{}</text>",
            layer_y + 14,
            escape_text(layer)
        ));
        let mut x = 100;
        for elem in document.elements.iter().filter(|e| e.layer == *layer) {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"140\" height=\"40\" rx=\"4\" ry=\"4\" fill=\"white\" stroke=\"#334155\" stroke-width=\"1\"/>",
                x,
                layer_y + 22
            ));
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#0f172a\">{}</text>",
                x + 8,
                layer_y + 46,
                escape_text(&elem.name)
            ));
            x += 150;
            if x + 140 > 736 {
                break;
            }
        }
        y += lane_height;
    }
    if !document.relations.is_empty() {
        y += 12;
        out.push_str(&format!(
            "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" font-weight=\"600\" fill=\"#334155\">Relations</text>",
            y
        ));
        y += 18;
        for rel in &document.relations {
            let label = rel
                .label
                .as_deref()
                .map(|l| format!(" : {l}"))
                .unwrap_or_default();
            out.push_str(&format!(
                "<text x=\"40\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#1e293b\">{} -[{}]-&gt; {}{}</text>",
                y,
                escape_text(&rel.from),
                escape_text(&rel.kind),
                escape_text(&rel.to),
                escape_text(&label)
            ));
            y += 18;
        }
    }
    out.push_str("</svg>");
    out
}

pub fn render_regex_svg(document: &RegexDocument) -> String {
    let width = 760;
    let row_height = 80;
    let height = 80 + (document.patterns.len().max(1) as i32) * row_height;
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">",
        w = width,
        h = height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    let mut y = 32;
    if let Some(title) = &document.title {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\">{}</text>",
            escape_text(title)
        ));
        y += 24;
    }
    out.push_str(&format!(
        "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">Railroad diagram (regex)</text>"
    ));
    y += 18;
    if document.patterns.is_empty() {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"12\" fill=\"#94a3b8\">(empty)</text>"
        ));
    } else {
        for pat in &document.patterns {
            render_regex_row(&mut out, &pat.source, &pat.tokens, y, width);
            y += row_height;
        }
    }
    out.push_str("</svg>");
    out
}

fn render_regex_row(out: &mut String, source: &str, tokens: &[RegexToken], y: i32, width: i32) {
    out.push_str(&format!(
        "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#334155\">/{}/</text>",
        y - 4,
        escape_text(source)
    ));
    let baseline = y + 26;
    out.push_str(&format!(
        "<line x1=\"24\" y1=\"{by}\" x2=\"{x2}\" y2=\"{by}\" stroke=\"#94a3b8\" stroke-width=\"1\"/>",
        by = baseline,
        x2 = width - 24
    ));
    let mut x = 40;
    out.push_str(&format!(
        "<circle cx=\"{x}\" cy=\"{by}\" r=\"5\" fill=\"#1e293b\"/>",
        x = x,
        by = baseline
    ));
    x += 18;
    let labels = regex_tokens_to_labels(tokens);
    for label in &labels {
        let box_w = (label.len().max(1) as i32) * 8 + 18;
        let box_w = box_w.min(width - x - 60);
        out.push_str(&format!(
            "<rect x=\"{x}\" y=\"{ry}\" width=\"{w}\" height=\"22\" rx=\"4\" ry=\"4\" fill=\"#e0f2fe\" stroke=\"#0284c7\" stroke-width=\"1\"/>",
            x = x,
            ry = baseline - 11,
            w = box_w
        ));
        out.push_str(&format!(
            "<text x=\"{tx}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"11\" fill=\"#0c4a6e\">{}</text>",
            escape_text(label),
            tx = x + 6,
            ty = baseline + 4
        ));
        x += box_w + 8;
        if x > width - 80 {
            break;
        }
    }
    out.push_str(&format!(
        "<circle cx=\"{x}\" cy=\"{by}\" r=\"5\" fill=\"#1e293b\"/>",
        x = (width - 36),
        by = baseline
    ));
}

fn regex_tokens_to_labels(tokens: &[RegexToken]) -> Vec<String> {
    let mut out = Vec::new();
    for t in tokens {
        out.push(regex_token_label(t));
    }
    out
}

fn regex_token_label(token: &RegexToken) -> String {
    match token {
        RegexToken::Literal(s) => format!("'{}'", s),
        RegexToken::CharClass(s) => format!("[{}]", s),
        RegexToken::Group(inner) => format!("({})", regex_tokens_to_labels(inner).join(" ")),
        RegexToken::Alt(branches) => {
            let parts: Vec<String> = branches
                .iter()
                .map(|b| regex_tokens_to_labels(b).join(" "))
                .collect();
            format!("alt({})", parts.join("|"))
        }
        RegexToken::Repeat { inner, kind } => {
            let suffix = match kind {
                RepeatKind::ZeroOrOne => "?",
                RepeatKind::ZeroOrMore => "*",
                RepeatKind::OneOrMore => "+",
            };
            format!("{}{}", regex_token_label(inner), suffix)
        }
        RegexToken::Escape(c) => format!("\\{}", c),
        RegexToken::AnyChar => ".".to_string(),
        RegexToken::Anchor(s) => s.clone(),
        RegexToken::Unsupported(s) => format!("?{}?", s),
    }
}

pub fn render_ebnf_svg(document: &EbnfDocument) -> String {
    let width = 820;
    let row_height = 90;
    let height = 80 + (document.rules.len().max(1) as i32) * row_height;
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">",
        w = width,
        h = height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    let mut y = 32;
    if let Some(title) = &document.title {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\">{}</text>",
            escape_text(title)
        ));
        y += 24;
    }
    out.push_str(&format!(
        "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">EBNF railroad diagrams</text>"
    ));
    y += 18;
    if document.rules.is_empty() {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"12\" fill=\"#94a3b8\">(empty)</text>"
        ));
    } else {
        for rule in &document.rules {
            out.push_str(&format!(
                "<text x=\"24\" y=\"{ty}\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" fill=\"#0f172a\">{} ::=</text>",
                escape_text(&rule.name),
                ty = y
            ));
            let baseline = y + 30;
            out.push_str(&format!(
                "<line x1=\"24\" y1=\"{by}\" x2=\"{x2}\" y2=\"{by}\" stroke=\"#94a3b8\" stroke-width=\"1\"/>",
                by = baseline,
                x2 = width - 24
            ));
            out.push_str(&format!(
                "<circle cx=\"40\" cy=\"{by}\" r=\"5\" fill=\"#1e293b\"/>",
                by = baseline
            ));
            let mut x = 60;
            let labels = ebnf_tokens_to_labels(&rule.tokens);
            for label in &labels {
                let box_w = ((label.len() as i32) * 8).clamp(36, width - x - 60);
                let fill = if label.starts_with('\'') || label.starts_with('"') {
                    "#fef3c7"
                } else {
                    "#e0e7ff"
                };
                let stroke = if label.starts_with('\'') || label.starts_with('"') {
                    "#d97706"
                } else {
                    "#4f46e5"
                };
                out.push_str(&format!(
                    "<rect x=\"{x}\" y=\"{ry}\" width=\"{w}\" height=\"22\" rx=\"4\" ry=\"4\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
                    x = x,
                    ry = baseline - 11,
                    w = box_w
                ));
                out.push_str(&format!(
                    "<text x=\"{tx}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"11\" fill=\"#1e293b\">{}</text>",
                    escape_text(label),
                    tx = x + 6,
                    ty = baseline + 4
                ));
                x += box_w + 8;
                if x > width - 80 {
                    break;
                }
            }
            out.push_str(&format!(
                "<circle cx=\"{x}\" cy=\"{by}\" r=\"5\" fill=\"#1e293b\"/>",
                x = (width - 36),
                by = baseline
            ));
            y += row_height;
        }
    }
    out.push_str("</svg>");
    out
}

fn ebnf_tokens_to_labels(tokens: &[EbnfToken]) -> Vec<String> {
    tokens.iter().map(ebnf_token_label).collect()
}

fn ebnf_token_label(token: &EbnfToken) -> String {
    match token {
        EbnfToken::Terminal(s) => format!("\"{}\"", s),
        EbnfToken::NonTerminal(s) => s.clone(),
        EbnfToken::Alt(branches) => {
            let parts: Vec<String> = branches
                .iter()
                .map(|b| ebnf_tokens_to_labels(b).join(" "))
                .collect();
            format!("({})", parts.join(" | "))
        }
        EbnfToken::Group(inner) => format!("({})", ebnf_tokens_to_labels(inner).join(" ")),
        EbnfToken::Optional(inner) => format!("[{}]", ebnf_tokens_to_labels(inner).join(" ")),
        EbnfToken::Repetition(inner) => format!("{{{}}}", ebnf_tokens_to_labels(inner).join(" ")),
        EbnfToken::Repeat { inner, kind } => {
            let suffix = match kind {
                RepeatKind::ZeroOrOne => "?",
                RepeatKind::ZeroOrMore => "*",
                RepeatKind::OneOrMore => "+",
            };
            format!("{}{}", ebnf_token_label(inner), suffix)
        }
        EbnfToken::Unsupported(s) => format!("?{}?", s),
    }
}

pub fn render_math_svg(document: &MathDocument) -> String {
    let start = document
        .title
        .as_ref()
        .map(|title| format!("@startmath \"{}\"", title.replace('"', "\\\"")))
        .unwrap_or_else(|| "@startmath".to_string());
    let source = format!("{start}\n{}\n@endmath", document.body);
    if let Some(Ok(svg)) = crate::specialized::try_render_specialized(&source) {
        return svg;
    }

    let width = 760;
    let lines: Vec<&str> = document.body.lines().collect();
    let line_count = lines.len().max(1) as i32;
    let height = 120 + line_count * 22;
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">",
        w = width,
        h = height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    let mut y = 32;
    if let Some(title) = &document.title {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\">{}</text>",
            escape_text(title)
        ));
        y += 24;
    }
    out.push_str(&format!(
        "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">math (LaTeX-like)</text>"
    ));
    y += 16;
    let box_y = y;
    let box_h = (line_count * 22) + 24;
    out.push_str(&format!(
        "<rect x=\"24\" y=\"{by}\" width=\"{bw}\" height=\"{bh}\" rx=\"6\" ry=\"6\" fill=\"#f8fafc\" stroke=\"#cbd5e1\" stroke-width=\"1\"/>",
        by = box_y,
        bw = width - 48,
        bh = box_h
    ));
    let mut ty = box_y + 24;
    for line in lines {
        out.push_str(&format!(
            "<text x=\"40\" y=\"{ty}\" font-family=\"monospace\" font-size=\"13\" fill=\"#0f172a\">{}</text>",
            escape_text(line)
        ));
        ty += 22;
    }
    out.push_str("</svg>");
    out
}

pub fn render_ditaa_svg(document: &DitaaDocument) -> String {
    let start = document
        .title
        .as_ref()
        .map(|title| format!("@startditaa \"{}\"", title.replace('"', "\\\"")))
        .unwrap_or_else(|| "@startditaa".to_string());
    let source = format!("{start}\n{}\n@endditaa", document.body);
    if let Some(Ok(svg)) = crate::specialized::try_render_specialized(&source) {
        return svg;
    }

    let width = 820;
    let lines: Vec<&str> = document.body.lines().collect();
    let line_count = lines.len().max(1) as i32;
    let height = 120 + line_count * 18;
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">",
        w = width,
        h = height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    let mut y = 32;
    if let Some(title) = &document.title {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\">{}</text>",
            escape_text(title)
        ));
        y += 24;
    }
    out.push_str(&format!(
        "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">ditaa (ASCII art frame)</text>"
    ));
    y += 16;
    let box_y = y;
    let box_h = (line_count * 18) + 24;
    out.push_str(&format!(
        "<rect x=\"24\" y=\"{by}\" width=\"{bw}\" height=\"{bh}\" rx=\"4\" ry=\"4\" fill=\"#fdf6e3\" stroke=\"#b58900\" stroke-width=\"1\"/>",
        by = box_y,
        bw = width - 48,
        bh = box_h
    ));
    let mut ty = box_y + 20;
    for line in lines {
        out.push_str(&format!(
            "<text x=\"36\" y=\"{ty}\" font-family=\"monospace\" font-size=\"12\" fill=\"#073642\" xml:space=\"preserve\">{}</text>",
            escape_text(line)
        ));
        ty += 18;
    }
    out.push_str("</svg>");
    out
}

pub fn render_sdl_svg(document: &SdlDocument) -> String {
    let width = 820;
    let col_w = 160;
    let row_h = 90;
    let state_count = document.states.len().max(1) as i32;
    let cols = ((width - 80) / col_w).max(1);
    let rows = (state_count + cols - 1) / cols;
    let height = 120 + rows * row_h + (document.transitions.len() as i32 * 18);
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">",
        w = width,
        h = height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    let mut y = 32;
    if let Some(title) = &document.title {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\">{}</text>",
            escape_text(title)
        ));
        y += 24;
    }
    out.push_str(&format!(
        "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">SDL state machine</text>"
    ));
    y += 16;
    let grid_top = y;
    for (idx, state) in document.states.iter().enumerate() {
        let col = (idx as i32) % cols;
        let row = (idx as i32) / cols;
        let sx = 40 + col * col_w;
        let sy = grid_top + row * row_h + 20;
        let (fill, stroke) = match state.kind {
            SdlStateKind::Start => ("#dcfce7", "#16a34a"),
            SdlStateKind::Stop => ("#fee2e2", "#dc2626"),
            SdlStateKind::State => ("#e0e7ff", "#4f46e5"),
        };
        out.push_str(&format!(
            "<rect x=\"{sx}\" y=\"{sy}\" width=\"{w}\" height=\"40\" rx=\"18\" ry=\"18\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
            sx = sx,
            sy = sy,
            w = col_w - 16
        ));
        out.push_str(&format!(
            "<text x=\"{tx}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"12\" fill=\"#0f172a\">{}: {}</text>",
            sdl_state_kind_label(state.kind),
            escape_text(&state.name),
            tx = sx + 8,
            ty = sy + 24
        ));
    }
    let mut ty = grid_top + rows * row_h + 16;
    out.push_str(&format!(
        "<text x=\"24\" y=\"{ty}\" font-family=\"monospace\" font-size=\"12\" font-weight=\"600\" fill=\"#334155\">Transitions</text>"
    ));
    ty += 18;
    for tr in &document.transitions {
        let sig = tr
            .signal
            .as_deref()
            .map(|s| format!(" : {s}"))
            .unwrap_or_default();
        out.push_str(&format!(
            "<text x=\"36\" y=\"{ty}\" font-family=\"monospace\" font-size=\"12\" fill=\"#1e293b\">{} -&gt; {}{}</text>",
            escape_text(&tr.from),
            escape_text(&tr.to),
            escape_text(&sig)
        ));
        ty += 18;
    }
    out.push_str("</svg>");
    out
}

fn sdl_state_kind_label(kind: SdlStateKind) -> &'static str {
    match kind {
        SdlStateKind::Start => "start",
        SdlStateKind::Stop => "stop",
        SdlStateKind::State => "state",
    }
}

pub fn render_chart_svg(document: &ChartDocument) -> String {
    let width = 780;
    let height = 420;
    let style = &document.style;
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">",
        w = width,
        h = height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    let mut y = 28;
    if let Some(title) = &document.title {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\">{}</text>",
            escape_text(title)
        ));
        y += 22;
    }
    let label = match document.subtype {
        ChartSubtype::Bar => "bar chart",
        ChartSubtype::Line => "line chart",
        ChartSubtype::Pie => "pie chart",
    };
    out.push_str(&format!(
        "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">{}</text>",
        label
    ));
    let plot_top = y + 16;
    let plot_bottom = height - 60;
    let plot_left = 60;
    let plot_right = width - 40;
    match document.subtype {
        ChartSubtype::Bar => render_chart_bars(
            &mut out,
            &document.data,
            plot_left,
            plot_top,
            plot_right,
            plot_bottom,
            style,
        ),
        ChartSubtype::Line => render_chart_line(
            &mut out,
            &document.data,
            plot_left,
            plot_top,
            plot_right,
            plot_bottom,
            style,
        ),
        ChartSubtype::Pie => render_chart_pie(
            &mut out,
            &document.data,
            width / 2,
            (plot_top + plot_bottom) / 2,
            style,
        ),
    }
    out.push_str("</svg>");
    out
}

const CHART_PALETTE: &[&str] = &[
    "#1d4ed8", "#16a34a", "#d97706", "#7c3aed", "#0891b2", "#dc2626", "#0f172a", "#facc15",
];

fn render_chart_bars(
    out: &mut String,
    data: &[crate::model::ChartPoint],
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    style: &crate::theme::ChartStyle,
) {
    out.push_str(&format!(
        "<line x1=\"{l}\" y1=\"{b}\" x2=\"{r}\" y2=\"{b}\" stroke=\"{}\" stroke-width=\"1\"/>",
        escape_text(&style.axis_color),
        l = left,
        r = right,
        b = bottom
    ));
    out.push_str(&format!(
        "<line x1=\"{l}\" y1=\"{t}\" x2=\"{l}\" y2=\"{b}\" stroke=\"{}\" stroke-width=\"1\"/>",
        escape_text(&style.axis_color),
        l = left,
        t = top,
        b = bottom
    ));
    if data.is_empty() {
        return;
    }
    let max_value = data
        .iter()
        .map(|p| p.value.max(0.0))
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let count = data.len() as i32;
    let avail = (right - left).max(20);
    let bar_w = (avail / count).max(4) - 6;
    for (idx, point) in data.iter().enumerate() {
        let bx = left + (idx as i32) * (avail / count) + 4;
        let bh = ((point.value.max(0.0) / max_value) * ((bottom - top) as f64)) as i32;
        let by = bottom - bh;
        let color = if idx == 0 {
            style.bar_color.as_str()
        } else {
            CHART_PALETTE[idx % CHART_PALETTE.len()]
        };
        out.push_str(&format!(
            "<rect x=\"{bx}\" y=\"{by}\" width=\"{bw}\" height=\"{bh}\" fill=\"{color}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
            escape_text(&style.axis_color),
            bx = bx,
            by = by,
            bw = bar_w,
            bh = bh
        ));
        out.push_str(&format!(
            "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
            escape_text(&style.font_color),
            escape_text(&point.label),
            tx = bx + bar_w / 2,
            ty = bottom + 16
        ));
        out.push_str(&format!(
            "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"#334155\">{}</text>",
            format_chart_value(point.value),
            tx = bx + bar_w / 2,
            ty = by - 4
        ));
    }
}

fn render_chart_line(
    out: &mut String,
    data: &[crate::model::ChartPoint],
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    style: &crate::theme::ChartStyle,
) {
    out.push_str(&format!(
        "<line x1=\"{l}\" y1=\"{b}\" x2=\"{r}\" y2=\"{b}\" stroke=\"{}\" stroke-width=\"1\"/>",
        style.axis_color,
        l = left,
        r = right,
        b = bottom
    ));
    out.push_str(&format!(
        "<line x1=\"{l}\" y1=\"{t}\" x2=\"{l}\" y2=\"{b}\" stroke=\"{}\" stroke-width=\"1\"/>",
        style.axis_color,
        l = left,
        t = top,
        b = bottom
    ));
    if data.is_empty() {
        return;
    }
    let max_value = data
        .iter()
        .map(|p| p.value.max(0.0))
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let count = data.len() as i32;
    let step = ((right - left) as f64) / ((count.max(2) - 1) as f64).max(1.0);
    let mut points = String::new();
    for (idx, point) in data.iter().enumerate() {
        let px = left + ((idx as f64) * step) as i32;
        let ph = ((point.value.max(0.0) / max_value) * ((bottom - top) as f64)) as i32;
        let py = bottom - ph;
        if !points.is_empty() {
            points.push(' ');
        }
        points.push_str(&format!("{px},{py}"));
        out.push_str(&format!(
            "<circle cx=\"{px}\" cy=\"{py}\" r=\"3\" fill=\"{}\"/>",
            style.line_color
        ));
        out.push_str(&format!(
            "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
            style.font_color,
            escape_text(&point.label),
            tx = px,
            ty = bottom + 16
        ));
    }
    out.push_str(&format!(
        "<polyline points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        points, style.line_color
    ));
}

fn render_chart_pie(
    out: &mut String,
    data: &[crate::model::ChartPoint],
    cx: i32,
    cy: i32,
    style: &crate::theme::ChartStyle,
) {
    let radius = 120_i32;
    let total: f64 = data.iter().map(|p| p.value.max(0.0)).sum();
    if total <= 0.0 {
        out.push_str(&format!(
            "<circle cx=\"{cx}\" cy=\"{cy}\" r=\"{r}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            style.grid_color,
            style.pie_border_color,
            cx = cx,
            cy = cy,
            r = radius
        ));
        return;
    }
    let mut acc = 0.0_f64;
    // Deterministic angle accumulation using f64.
    for (idx, point) in data.iter().enumerate() {
        let v = point.value.max(0.0);
        let start = acc / total * std::f64::consts::TAU;
        acc += v;
        let end = acc / total * std::f64::consts::TAU;
        let x1 = cx as f64 + (radius as f64) * start.cos();
        let y1 = cy as f64 + (radius as f64) * start.sin();
        let x2 = cx as f64 + (radius as f64) * end.cos();
        let y2 = cy as f64 + (radius as f64) * end.sin();
        let large = if (end - start) > std::f64::consts::PI {
            1
        } else {
            0
        };
        let color = if idx == 0 {
            style.series_color.as_str()
        } else {
            CHART_PALETTE[idx % CHART_PALETTE.len()]
        };
        out.push_str(&format!(
            "<path d=\"M {cx} {cy} L {x1:.2} {y1:.2} A {r} {r} 0 {large} 1 {x2:.2} {y2:.2} Z\" fill=\"{color}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
            style.pie_border_color,
            cx = cx,
            cy = cy,
            r = radius,
            x1 = x1,
            y1 = y1,
            x2 = x2,
            y2 = y2,
            large = large,
            color = color
        ));
        let mid = (start + end) / 2.0;
        let lx = cx as f64 + ((radius as f64) * 0.6) * mid.cos();
        let ly = cy as f64 + ((radius as f64) * 0.6) * mid.sin();
        out.push_str(&format!(
            "<text x=\"{lx:.0}\" y=\"{ly:.0}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
            style.font_color,
            escape_text(&point.label),
            lx = lx,
            ly = ly
        ));
    }
}

fn format_chart_value(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{}", v as i64)
    } else {
        format!("{:.2}", v)
    }
}

/// Compute start and end points for a transition arrow between two nodes.
fn state_node_bbox(node: &StateNode) -> (i32, i32) {
    match node.kind {
        StateNodeKind::Fork | StateNodeKind::Join => (STATE_NODE_W, 8),
        StateNodeKind::Choice => (60, 40),
        StateNodeKind::HistoryShallow | StateNodeKind::HistoryDeep => (40, 40),
        StateNodeKind::StartEnd | StateNodeKind::End => (40, 40),
        StateNodeKind::Normal => {
            let actions_h = (node.internal_actions.len() as i32) * 14;
            (STATE_NODE_W, STATE_NODE_H + actions_h)
        }
    }
}

fn transition_endpoints(
    from_node: &StateNode,
    fx: i32,
    fy: i32,
    to_node: &StateNode,
    tx: i32,
    ty: i32,
) -> (i32, i32, i32, i32) {
    let (fw_full, fh_full) = state_node_bbox(from_node);
    let (tw_full, th_full) = state_node_bbox(to_node);
    let fh = fh_full / 2;
    let fw = fw_full / 2;
    let th = th_full / 2;
    let tw = tw_full / 2;

    // Center of each node
    let fcx = fx + fw;
    let fcy = fy + fh;
    let tcx = tx + tw;
    let tcy = ty + th;

    // Simple: exit from right/left/bottom/top depending on relative position
    let dx = tcx - fcx;
    let dy = tcy - fcy;

    if dx.abs() >= dy.abs() {
        // Horizontal
        if dx >= 0 {
            (fcx + fw, fcy, tcx - tw, tcy)
        } else {
            (fcx - fw, fcy, tcx + tw, tcy)
        }
    } else {
        // Vertical
        if dy >= 0 {
            (fcx, fcy + fh, tcx, tcy - th)
        } else {
            (fcx, fcy - fh, tcx, tcy + th)
        }
    }
}

/// Render a single state node at (x, y) — delegates to the styled version with defaults.
fn render_state_node_svg_styled(
    out: &mut String,
    node: &StateNode,
    x: i32,
    y: i32,
    state_style: &crate::theme::StateStyle,
    incoming_count: usize,
    outgoing_count: usize,
) {
    let w = STATE_NODE_W;
    let base_h = STATE_NODE_H;
    let action_rows = node.internal_actions.len() as i32;
    let h = base_h + action_rows * 14;

    match node.kind {
        StateNodeKind::StartEnd => {
            let cx = x + w / 2;
            let cy = y + base_h / 2;
            if incoming_count > 0 && outgoing_count == 0 {
                out.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"14\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                    cx, cy, state_style.background_color, state_style.border_color
                ));
                out.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"8\" fill=\"{}\"/>",
                    cx, cy, state_style.start_color
                ));
            } else {
                out.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"12\" fill=\"{}\"/>",
                    cx, cy, state_style.start_color
                ));
            }
        }
        StateNodeKind::HistoryShallow | StateNodeKind::HistoryDeep => {
            let cx = x + w / 2;
            let cy = y + base_h / 2;
            let label = node.display.as_deref().unwrap_or("H");
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"16\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                cx, cy, state_style.background_color, state_style.border_color
            ));
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" text-anchor=\"middle\" dominant-baseline=\"middle\" fill=\"{}\">{}</text>",
                cx, cy, state_style.border_color, escape_text(label)
            ));
        }
        StateNodeKind::Fork | StateNodeKind::Join => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"8\" fill=\"{}\"/>",
                x,
                y + base_h / 2 - 4,
                w,
                state_style.start_color
            ));
            let label = if node.kind == StateNodeKind::Fork {
                "fork"
            } else {
                "join"
            };
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#475569\" text-anchor=\"middle\">{}</text>",
                x + w / 2, y + base_h / 2 + 18, escape_text(label)
            ));
        }
        StateNodeKind::Choice => {
            let cx = x + w / 2;
            let cy = y + base_h / 2;
            let r = 18i32;
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                cx, cy - r,
                cx + r, cy,
                cx, cy + r,
                cx - r, cy,
                state_style.background_color, state_style.border_color
            ));
        }
        StateNodeKind::End => {
            let cx = x + w / 2;
            let cy = y + base_h / 2;
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"14\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                cx, cy, state_style.background_color, state_style.border_color
            ));
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"9\" fill=\"{}\"/>",
                cx, cy, state_style.start_color
            ));
        }
        StateNodeKind::Normal => {
            let has_regions = node.regions.len() > 1
                || node.regions.first().map(|r| !r.is_empty()).unwrap_or(false);
            let display = node.display.as_deref().unwrap_or(&node.name);

            if has_regions && node.regions.len() > 1 {
                let total_w = w + (node.regions.len() as i32 - 1) * (STATE_NODE_W / 2 + 10);
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" ry=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                    x, y, total_w, h + 16, state_style.background_color, state_style.border_color
                ));
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" font-weight=\"600\" text-anchor=\"middle\" fill=\"#0c4a6e\">{}</text>",
                    x + total_w / 2, y + 16, escape_text(display)
                ));
                let region_w = total_w / node.regions.len() as i32;
                for ri in 1..node.regions.len() {
                    let div_x = x + ri as i32 * region_w;
                    out.push_str(&format!(
                        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"4 3\"/>",
                        div_x, y + 24, div_x, y + h + 16, state_style.border_color
                    ));
                }
                for (ri, region) in node.regions.iter().enumerate() {
                    let region_x = x + ri as i32 * region_w + 4;
                    let mut child_y = y + 28;
                    for child in region {
                        render_state_node_svg_styled(
                            out,
                            child,
                            region_x,
                            child_y,
                            state_style,
                            0,
                            0,
                        );
                        child_y += STATE_NODE_H + 12;
                    }
                }
            } else {
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" ry=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                    x, y, w, h, state_style.background_color, state_style.border_color
                ));
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" text-anchor=\"middle\" fill=\"#0f172a\">{}</text>",
                    x + w / 2, y + 24, escape_text(display)
                ));
                if !node.internal_actions.is_empty() {
                    out.push_str(&format!(
                        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                        x, y + base_h - 4, x + w, y + base_h - 4, state_style.border_color
                    ));
                    for (ai, action) in node.internal_actions.iter().enumerate() {
                        let ay = y + base_h + ai as i32 * 14;
                        let text = if action.action.is_empty() {
                            action.kind.clone()
                        } else {
                            format!("{} / {}", action.kind, action.action)
                        };
                        out.push_str(&format!(
                            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" font-style=\"italic\" fill=\"#334155\">{}</text>",
                            x + 6, ay + 10, escape_text(&text)
                        ));
                    }
                }
                if let Some(region) = node.regions.first() {
                    if !region.is_empty() {
                        let mut child_y = y + h + 4;
                        for child in region {
                            render_state_node_svg_styled(
                                out,
                                child,
                                x + 8,
                                child_y,
                                state_style,
                                0,
                                0,
                            );
                            child_y += STATE_NODE_H + 8;
                        }
                    }
                }
            }
        }
    }
}

fn render_virtual_endpoint_marker(out: &mut String, x: i32, y: i32, kind: VirtualEndpointKind) {
    match kind {
        VirtualEndpointKind::Plain => {
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#111\" stroke-width=\"1.5\"/>",
                x, y - 6, x, y + 6
            ));
        }
        VirtualEndpointKind::Circle => {
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"4\" fill=\"white\" stroke=\"#111\" stroke-width=\"1.5\"/>",
                x, y
            ));
        }
        VirtualEndpointKind::Cross => {
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#111\" stroke-width=\"1.5\"/>",
                x - 4,
                y - 4,
                x + 4,
                y + 4
            ));
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#111\" stroke-width=\"1.5\"/>",
                x - 4,
                y + 4,
                x + 4,
                y - 4
            ));
        }
        VirtualEndpointKind::Filled => {
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"4\" fill=\"#111\" stroke=\"#111\" stroke-width=\"1.5\"/>",
                x, y
            ));
        }
    }
}

/// Emit a `<text>` element with Creole-formatted content.
///
/// If the text has a single line and no Creole markup, this is equivalent to
/// the old `escape_text` path. Multi-line content uses `<tspan dy="1.2em">`.
fn creole_text(x: i32, y: i32, extra_attrs: &str, label: &str, base_color: &str) -> String {
    let lines = tokenize_creole(label);
    let has_markup = label.contains("**")
        || label.contains("//")
        || label.contains("\"\"")
        || label.contains("__")
        || label.contains("--")
        || label.contains("[[")
        || label.contains("<color")
        || label.contains("<size")
        || label.contains("<b>")
        || label.contains("<B>")
        || label.contains("<i>")
        || label.contains("<I>")
        || label.contains("<u>")
        || label.contains("<U>")
        || label.contains("<&");

    if !has_markup && lines.len() == 1 {
        // Fast path — no markup, no multi-line: keep old behavior.
        return format!(
            "<text x=\"{}\" y=\"{}\"{}>{}",
            x,
            y,
            if extra_attrs.is_empty() {
                String::new()
            } else {
                format!(" {}", extra_attrs)
            },
            escape_text(label)
        ) + "</text>";
    }

    let inner = render_creole_to_svg_tspans(&lines, x, base_color);
    format!(
        "<text x=\"{}\" y=\"{}\"{}>{}",
        x,
        y,
        if extra_attrs.is_empty() {
            String::new()
        } else {
            format!(" {}", extra_attrs)
        },
        inner
    ) + "</text>"
}

fn escape_text(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn render_participant_box(out: &mut String, participant: &ParticipantBox, scene: &Scene) {
    let x = participant.x;
    let y = participant.y;
    let width = participant.width;
    let height = participant.height;
    let display_lines = &participant.display_lines;
    let cx = x + (width / 2);

    match participant.role {
        ParticipantRole::Participant => {
            let rx = scene.style.round_corner;
            let shadow_attr = if scene.style.shadowing {
                " filter=\"url(#shadow)\""
            } else {
                ""
            };
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"{shadow_attr}/>",
                x,
                y,
                width,
                height,
                rx,
                rx,
                scene.style.participant_background_color,
                scene.style.participant_border_color,
                shadow_attr = shadow_attr
            ));
        }
        ParticipantRole::Actor => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" ry=\"8\" fill=\"#fff3e0\" stroke=\"#8a5a00\" stroke-width=\"1\"/>",
                x, y, width, height
            ));
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"4\" fill=\"none\" stroke=\"#8a5a00\" stroke-width=\"1\"/>",
                x + 12,
                y + 10
            ));
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#8a5a00\" stroke-width=\"1\"/>",
                x + 12,
                y + 14,
                x + 12,
                y + 22
            ));
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#8a5a00\" stroke-width=\"1\"/>",
                x + 8,
                y + 18,
                x + 16,
                y + 18
            ));
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#8a5a00\" stroke-width=\"1\"/>",
                x + 12,
                y + 22,
                x + 8,
                y + 28
            ));
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#8a5a00\" stroke-width=\"1\"/>",
                x + 12,
                y + 22,
                x + 16,
                y + 28
            ));
        }
        ParticipantRole::Boundary => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#eef8ff\" stroke=\"#1b5e8a\" stroke-width=\"1\" stroke-dasharray=\"5 3\"/>",
                x, y, width, height
            ));
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#1b5e8a\" stroke-width=\"1\"/>",
                x + 6,
                y + 4,
                x + 6,
                y + height - 4
            ));
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#1b5e8a\" stroke-width=\"1\"/>",
                x + width - 6,
                y + 4,
                x + width - 6,
                y + height - 4
            ));
        }
        ParticipantRole::Control => {
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{} {},{} {},{} {},{}\" fill=\"#edf7ed\" stroke=\"#2d6a2d\" stroke-width=\"1\"/>",
                x + 10,
                y,
                x + width - 10,
                y,
                x + width,
                y + height / 2,
                x + width - 10,
                y + height,
                x + 10,
                y + height,
                x,
                y + height / 2
            ));
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#2d6a2d\" stroke-width=\"1\"/>",
                x + 10,
                y + height / 2,
                x + width - 10,
                y + height / 2
            ));
        }
        ParticipantRole::Entity => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" ry=\"3\" fill=\"#f4f0ff\" stroke=\"#4e3a8f\" stroke-width=\"1\"/>",
                x, y, width, height
            ));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"2\" ry=\"2\" fill=\"none\" stroke=\"#4e3a8f\" stroke-width=\"1\"/>",
                x + 4,
                y + 4,
                width - 8,
                height - 8
            ));
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#4e3a8f\" stroke-width=\"1\"/>",
                x + 6,
                y + 12,
                x + width - 6,
                y + 12
            ));
        }
        ParticipantRole::Database => {
            out.push_str(&format!(
                "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"6\" fill=\"#e9f5ff\" stroke=\"#1b5e8a\" stroke-width=\"1\"/>",
                cx,
                y + 6,
                (width / 2) - 2
            ));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#e9f5ff\" stroke=\"#1b5e8a\" stroke-width=\"1\"/>",
                x + 2,
                y + 6,
                width - 4,
                height - 12
            ));
            out.push_str(&format!(
                "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"6\" fill=\"#e9f5ff\" stroke=\"#1b5e8a\" stroke-width=\"1\"/>",
                cx,
                y + height - 6,
                (width / 2) - 2
            ));
        }
        ParticipantRole::Collections => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" ry=\"3\" fill=\"#fff9e8\" stroke=\"#8a6d1b\" stroke-width=\"1\"/>",
                x, y + 4, width, height - 4
            ));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"2\" ry=\"2\" fill=\"#fff9e8\" stroke=\"#8a6d1b\" stroke-width=\"1\"/>",
                x + 8, y, 24, 8
            ));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"2\" ry=\"2\" fill=\"#fff9e8\" stroke=\"#8a6d1b\" stroke-width=\"1\"/>",
                x + 14, y + 2, 24, 8
            ));
        }
        ParticipantRole::Queue => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" ry=\"4\" fill=\"#fff0f0\" stroke=\"#8a3030\" stroke-width=\"1\"/>",
                x, y, width, height
            ));
            for i in [8, 14, 20] {
                out.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#8a3030\" stroke-width=\"1\"/>",
                    x + 8,
                    y + i,
                    x + width - 8,
                    y + i
                ));
            }
        }
    }

    for (idx, line) in display_lines.iter().enumerate() {
        out.push_str(&creole_text(
            cx,
            y + 21 + (idx as i32 * 16),
            "text-anchor=\"middle\" font-family=\"monospace\" font-size=\"13\"",
            line,
            "black",
        ));
    }
}

// ─── MindMap renderer ─────────────────────────────────────────────────────────

/// Pastel palette cycling by depth (6 colours, 0-indexed by depth).
const MINDMAP_PALETTE: &[&str] = &[
    "#fde68a", // depth 0 — root amber
    "#bfdbfe", // depth 1 — sky blue
    "#bbf7d0", // depth 2 — mint
    "#fecaca", // depth 3 — rose
    "#e9d5ff", // depth 4 — lavender
    "#fed7aa", // depth 5 — peach
];

fn mindmap_node_fill(depth: usize) -> &'static str {
    MINDMAP_PALETTE[depth % MINDMAP_PALETTE.len()]
}

/// Render a `@startmindmap` document as SVG.
///
/// Layout: horizontal tree — root centred; right-side branches extend right,
/// left-side branches extend left. Each level increments x by `X_STEP`. Y is
/// spread evenly per side.
pub fn render_mindmap_svg(doc: &FamilyDocument) -> String {
    const X_STEP: i32 = 180;
    const Y_STEP: i32 = 48;
    const NODE_H: i32 = 34;
    const MARGIN: i32 = 24;
    const NODE_PAD_X: i32 = 10;

    // Separate nodes into root, left-side, right-side subtrees.
    // Depth 0 = root. Depth 1+ inherit side from their nearest depth-1 ancestor.
    let nodes = &doc.nodes;
    if nodes.is_empty() {
        return mindmap_empty_svg(doc);
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

    // Collect left/right subtrees at depth 1+.
    let right_roots: Vec<usize> = (0..n)
        .filter(|&i| nodes[i].depth == 1 && side[i] == MindMapSide::Right)
        .collect();
    let left_roots: Vec<usize> = (0..n)
        .filter(|&i| nodes[i].depth == 1 && side[i] == MindMapSide::Left)
        .collect();

    // For each depth-1 subtree, compute total height = number of descendants + self.
    fn subtree_leaf_count(nodes: &[crate::model::FamilyNode], idx: usize) -> usize {
        let depth = nodes[idx].depth;
        let children_count: usize = (idx + 1..nodes.len())
            .take_while(|&j| nodes[j].depth > depth)
            .filter(|&j| nodes[j].depth == depth + 1)
            .count();
        if children_count == 0 {
            return 1;
        }
        let mut total = 0usize;
        let mut j = idx + 1;
        while j < nodes.len() && nodes[j].depth > depth {
            if nodes[j].depth == depth + 1 {
                total += subtree_leaf_count(nodes, j);
            }
            j += 1;
        }
        total
    }

    // Assign y positions for right-side depth-1 nodes.
    let total_right_leaves: usize = right_roots
        .iter()
        .map(|&i| subtree_leaf_count(nodes, i))
        .sum();
    let total_left_leaves: usize = left_roots
        .iter()
        .map(|&i| subtree_leaf_count(nodes, i))
        .sum();
    let max_leaves = total_right_leaves.max(total_left_leaves).max(1);
    let canvas_h = (max_leaves as i32) * Y_STEP + 2 * MARGIN + NODE_H;

    // Max text width for nodes — simple heuristic.
    fn node_width(name: &str) -> i32 {
        let chars = name.chars().count() as i32;
        (chars * 7 + 20).clamp(80, 220)
    }

    let root_w = node_width(&nodes[0].name);
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

    let right_w = (max_right_depth as i32) * X_STEP + 240;
    let left_w = (max_left_depth as i32) * X_STEP + 240;
    let canvas_w = left_w + root_w + right_w + 2 * MARGIN;
    let root_cx = MARGIN + left_w + root_w / 2;
    let root_cy = canvas_h / 2;

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">",
        w = canvas_w,
        h = canvas_h
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

    // Draw nodes recursively — track y-cursors per side.
    // We assign y by a preorder traversal respecting leaf count.
    let mut y_positions = vec![0i32; n];
    {
        // Right side
        let mut y_cursor = root_cy - (total_right_leaves as i32 * Y_STEP) / 2 + Y_STEP / 2;
        assign_y_positions(nodes, &right_roots, &mut y_positions, &mut y_cursor, Y_STEP);
        // Left side
        y_cursor = root_cy - (total_left_leaves as i32 * Y_STEP) / 2 + Y_STEP / 2;
        assign_y_positions(nodes, &left_roots, &mut y_positions, &mut y_cursor, Y_STEP);
    }

    // Draw root node
    let rx = root_cx - root_w / 2;
    let ry = root_cy - NODE_H / 2;
    out.push_str(&format!(
        "<rect x=\"{rx}\" y=\"{ry}\" width=\"{rw}\" height=\"{h}\" rx=\"17\" ry=\"17\" fill=\"{fill}\" stroke=\"#92400e\" stroke-width=\"1.5\"/>",
        rx = rx, ry = ry, rw = root_w, h = NODE_H,
        fill = mindmap_node_fill(0)
    ));
    out.push_str(&format!(
        "<text x=\"{cx}\" y=\"{cy}\" text-anchor=\"middle\" dominant-baseline=\"middle\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\">{}</text>",
        escape_text(&nodes[0].name),
        cx = root_cx, cy = root_cy
    ));

    // Draw right-side branches.
    for &i in &right_roots {
        draw_mindmap_subtree(
            &mut out,
            nodes,
            i,
            root_cx + root_w / 2,
            root_cy,
            root_cx + root_w / 2 + X_STEP - NODE_PAD_X,
            &y_positions,
            X_STEP,
            NODE_H,
            NODE_PAD_X,
            false, // left=false → right
        );
    }
    // Draw left-side branches.
    for &i in &left_roots {
        draw_mindmap_subtree(
            &mut out,
            nodes,
            i,
            root_cx - root_w / 2,
            root_cy,
            root_cx - root_w / 2 - X_STEP + NODE_PAD_X,
            &y_positions,
            X_STEP,
            NODE_H,
            NODE_PAD_X,
            true, // left=true
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
    out
}

fn assign_y_positions(
    nodes: &[crate::model::FamilyNode],
    roots: &[usize],
    y_positions: &mut [i32],
    y_cursor: &mut i32,
    y_step: i32,
) {
    for &idx in roots {
        let depth = nodes[idx].depth;
        // Count leaf descendants
        let leaves = subtree_leaf_count_render(nodes, idx);
        // Place this node at the center of its allocated leaf-slots
        y_positions[idx] = *y_cursor + (leaves as i32 - 1) * y_step / 2;
        // Recurse into children
        let children: Vec<usize> = (idx + 1..nodes.len())
            .take_while(|&j| nodes[j].depth > depth)
            .filter(|&j| nodes[j].depth == depth + 1)
            .collect();
        assign_y_positions(nodes, &children, y_positions, y_cursor, y_step);
        if children.is_empty() {
            *y_cursor += y_step;
        }
    }
}

fn subtree_leaf_count_render(nodes: &[crate::model::FamilyNode], idx: usize) -> usize {
    let depth = nodes[idx].depth;
    let children: Vec<usize> = (idx + 1..nodes.len())
        .take_while(|&j| nodes[j].depth > depth)
        .filter(|&j| nodes[j].depth == depth + 1)
        .collect();
    if children.is_empty() {
        return 1;
    }
    children
        .iter()
        .map(|&c| subtree_leaf_count_render(nodes, c))
        .sum()
}

#[allow(clippy::too_many_arguments)]
fn draw_mindmap_subtree(
    out: &mut String,
    nodes: &[crate::model::FamilyNode],
    idx: usize,
    parent_attach_x: i32,
    parent_attach_y: i32,
    node_x_center: i32,
    y_positions: &[i32],
    x_step: i32,
    node_h: i32,
    node_pad_x: i32,
    is_left: bool,
) {
    let node = &nodes[idx];
    let ny = y_positions[idx];
    let nw = (node.name.chars().count() as i32 * 7 + 20).clamp(70, 200);
    let nx = if is_left {
        node_x_center - nw
    } else {
        node_x_center
    };
    let ny_top = ny - node_h / 2;

    // Connection line from parent
    let node_attach_x = if is_left { nx + nw } else { nx };
    out.push_str(&format!(
        "<line x1=\"{px}\" y1=\"{py}\" x2=\"{ax}\" y2=\"{ny}\" stroke=\"#94a3b8\" stroke-width=\"1.5\"/>",
        px = parent_attach_x,
        py = parent_attach_y,
        ax = node_attach_x,
        ny = ny
    ));

    // Node rectangle (rounded, pastel by depth)
    out.push_str(&format!(
        "<rect x=\"{nx}\" y=\"{ny_top}\" width=\"{nw}\" height=\"{nh}\" rx=\"14\" ry=\"14\" fill=\"{fill}\" stroke=\"#64748b\" stroke-width=\"1\"/>",
        nx = nx, ny_top = ny_top, nw = nw, nh = node_h,
        fill = mindmap_node_fill(node.depth)
    ));
    out.push_str(&format!(
        "<text x=\"{cx}\" y=\"{cy}\" text-anchor=\"middle\" dominant-baseline=\"middle\" font-family=\"monospace\" font-size=\"12\">{}</text>",
        escape_text(&node.name),
        cx = nx + nw / 2,
        cy = ny
    ));

    // Recurse into children
    let depth = node.depth;
    let children: Vec<usize> = (idx + 1..nodes.len())
        .take_while(|&j| nodes[j].depth > depth)
        .filter(|&j| nodes[j].depth == depth + 1)
        .collect();
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
            child_idx,
            from_x,
            ny,
            next_x_center,
            y_positions,
            x_step,
            node_h,
            node_pad_x,
            is_left,
        );
    }
}

fn mindmap_empty_svg(doc: &FamilyDocument) -> String {
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

// ─── WBS renderer ─────────────────────────────────────────────────────────────

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
    let canvas_w = (total_leaves as i32) * X_STEP + 2 * MARGIN;
    let max_depth = nodes.iter().map(|n| n.depth).max().unwrap_or(0);
    let canvas_h = (max_depth as i32 + 1) * Y_STEP + 2 * MARGIN + NODE_H;

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
        x_positions: &mut [i32],
        y_positions: &mut [i32],
    ) {
        let depth = nodes[idx].depth;
        let leaves = wbs_leaf_count(nodes, idx);
        let cx = x_start + (leaves as i32 * x_step) / 2;
        x_positions[idx] = cx;
        y_positions[idx] = margin + (depth as i32) * y_step + node_h / 2;

        let children: Vec<usize> = (idx + 1..nodes.len())
            .take_while(|&j| nodes[j].depth > depth)
            .filter(|&j| nodes[j].depth == depth + 1)
            .collect();
        let mut child_x = x_start;
        for &c in &children {
            assign_wbs_positions(
                nodes,
                c,
                child_x,
                x_step,
                margin,
                node_h,
                y_step,
                x_positions,
                y_positions,
            );
            child_x += wbs_leaf_count(nodes, c) as i32 * x_step;
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
        &mut x_positions,
        &mut y_positions,
    );

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">",
        w = canvas_w, h = canvas_h
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
            let px = x_positions[p];
            let py = y_positions[p] + NODE_H / 2;
            let cx = x_positions[i];
            let cy = y_positions[i] - NODE_H / 2;
            out.push_str(&format!(
                "<line x1=\"{px}\" y1=\"{py}\" x2=\"{cx}\" y2=\"{cy}\" stroke=\"#94a3b8\" stroke-width=\"1.5\"/>",
                px = px, py = py, cx = cx, cy = cy
            ));
        }
    }

    // Draw nodes.
    for i in 0..n {
        let node = &nodes[i];
        let cx = x_positions[i];
        let cy = y_positions[i];
        let nw = (node.name.chars().count() as i32 * 7 + 24).clamp(80, 200);
        let nx = cx - nw / 2;
        let ny = cy - NODE_H / 2;
        let fill = if node.depth == 0 {
            "#fde68a"
        } else {
            "#f1f5f9"
        };
        let stroke = if node.depth == 0 {
            "#92400e"
        } else {
            "#64748b"
        };
        out.push_str(&format!(
            "<rect x=\"{nx}\" y=\"{ny}\" width=\"{nw}\" height=\"{nh}\" rx=\"4\" ry=\"4\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
            nx = nx, ny = ny, nw = nw, nh = NODE_H, fill = fill, stroke = stroke
        ));

        // Render checkbox annotation if present.
        match &node.wbs_checkbox {
            Some(WbsCheckbox::Checked) => {
                // Checked checkbox before label
                out.push_str(&format!(
                    "<rect x=\"{bx}\" y=\"{by}\" width=\"12\" height=\"12\" rx=\"2\" ry=\"2\" fill=\"#16a34a\" stroke=\"#166534\" stroke-width=\"1\"/>",
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
                    "<rect x=\"{bx}\" y=\"{by}\" width=\"12\" height=\"12\" rx=\"2\" ry=\"2\" fill=\"#fff\" stroke=\"#64748b\" stroke-width=\"1\"/>",
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
                    "<rect x=\"{bx}\" y=\"{by}\" width=\"{bar_w}\" height=\"7\" rx=\"3\" ry=\"3\" fill=\"#e2e8f0\" stroke=\"#94a3b8\" stroke-width=\"0.5\"/>",
                    bx = nx + NODE_PAD, by = cy + 9, bar_w = bar_w
                ));
                if fill_w > 0 {
                    out.push_str(&format!(
                        "<rect x=\"{bx}\" y=\"{by}\" width=\"{fill_w}\" height=\"7\" rx=\"3\" ry=\"3\" fill=\"#3b82f6\"/>",
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
