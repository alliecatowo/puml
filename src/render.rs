use crate::ast::DiagramKind;
use crate::creole::{render_creole_to_svg_tspans, tokenize_creole};
use crate::model::{
    ArchimateDocument, ChartDocument, ChartSubtype, DitaaDocument, EbnfDocument, EbnfToken,
    FamilyDocument, FamilyNode, FamilyNodeKind, FamilyOrientation, JsonDocument, LegendHAlign,
    LegendVAlign, MathDocument, NwdiagDocument, ParticipantRole, RegexDocument, RegexToken,
    RepeatKind, ScaleSpec, SdlDocument, SdlStateKind, TimelineDocument, VirtualEndpointKind,
    YamlDocument,
};
use crate::scene::{ParticipantBox, Scene, StructureKind};

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

    let bg_fill = scene
        .style
        .background_color
        .as_deref()
        .unwrap_or("white");
    out.push_str(&format!("<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>", escape_text(bg_fill)));

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

    for l in &scene.lifelines {
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"6 4\"/>",
            l.x, l.y1, l.x, l.y2, scene.style.lifeline_border_color
        ));
    }

    for g in &scene.groups {
        let grx = (scene.style.round_corner / 2).max(2);
        out.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            g.x,
            g.y,
            g.width,
            g.height,
            grx,
            grx,
            if g.kind.eq_ignore_ascii_case("ref") {
                "#eef6ff"
            } else {
                scene.style.group_background_color.as_str()
            },
            scene.style.group_border_color
        ));

        if let Some(label) = &g.label {
            let header = label.lines().next().unwrap_or("");
            let header_full = format!("{} {}", g.kind, header);
            let header_trimmed = header_full.trim();
            out.push_str(&creole_text(
                g.x + 8,
                g.y + 16,
                "font-family=\"monospace\" font-size=\"12\" font-weight=\"600\"",
                header_trimmed,
                "black",
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

    for m in &scene.messages {
        let stroke_dash = if m.arrow.contains("--") {
            " stroke-dasharray=\"6 4\""
        } else {
            ""
        };
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"{}/>",
            m.x1, m.y, m.x2, m.y, scene.style.arrow_color, stroke_dash
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
        let h = header_height + body_h;
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
        let h = header_height + body_h;
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

    // Compute width / height of the SVG
    let svg_width = margin_x * 2 + col_count * node_width + (col_count - 1) * col_gap;
    let svg_height = nodes_bottom + 40;

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">",
        w = svg_width,
        h = svg_height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");

    // Arrowhead/diamond marker defs
    out.push_str("<defs>");
    out.push_str(
        "<marker id=\"arrow-open\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" \
         markerWidth=\"10\" markerHeight=\"10\" orient=\"auto-start-reverse\">\
         <path d=\"M0,0 L10,5 L0,10\" fill=\"none\" stroke=\"#1e293b\" stroke-width=\"1.5\"/>\
         </marker>",
    );
    out.push_str(
        "<marker id=\"arrow-triangle\" viewBox=\"0 0 12 12\" refX=\"11\" refY=\"6\" \
         markerWidth=\"12\" markerHeight=\"12\" orient=\"auto-start-reverse\">\
         <path d=\"M0,0 L12,6 L0,12 z\" fill=\"white\" stroke=\"#1e293b\" stroke-width=\"1.5\"/>\
         </marker>",
    );
    out.push_str(
        "<marker id=\"arrow-diamond-filled\" viewBox=\"0 0 14 10\" refX=\"13\" refY=\"5\" \
         markerWidth=\"14\" markerHeight=\"10\" orient=\"auto-start-reverse\">\
         <path d=\"M0,5 L7,0 L14,5 L7,10 z\" fill=\"#1e293b\" stroke=\"#1e293b\" stroke-width=\"1\"/>\
         </marker>",
    );
    out.push_str(
        "<marker id=\"arrow-diamond-open\" viewBox=\"0 0 14 10\" refX=\"13\" refY=\"5\" \
         markerWidth=\"14\" markerHeight=\"10\" orient=\"auto-start-reverse\">\
         <path d=\"M0,5 L7,0 L14,5 L7,10 z\" fill=\"white\" stroke=\"#1e293b\" stroke-width=\"1\"/>\
         </marker>",
    );
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
        let (from_name, to_name, normalized_arrow) = normalize_relation_endpoints(
            &relation.from,
            &relation.to,
            &relation.arrow,
        );
        let from = node_boxes.get(&from_name);
        let to = node_boxes.get(&to_name);
        let (Some(from), Some(to)) = (from, to) else {
            continue;
        };
        let style = arrow_style(&normalized_arrow);
        let (x1, y1, x2, y2) = compute_edge_anchors_tuple(
            (from.x, from.y, from.w, from.h),
            (to.x, to.y, to.w, to.h),
        );
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
            "<line x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\" stroke=\"#1e293b\" stroke-width=\"1.5\"{dash}{markers}/>",
            dash = stroke_dash
        ));
        if let Some(label) = &relation.label {
            let lx = (x1 + x2) / 2;
            let ly = (y1 + y2) / 2 - 4;
            out.push_str(&format!(
                "<text x=\"{lx}\" y=\"{ly}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#334155\">{txt}</text>",
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
        render_class_node(&mut out, node, bx.x, bx.y, bx.w, bx.h, bx.header_h);
    }

    out.push_str("</svg>");
    out
}

pub fn render_family_tree_svg(document: &FamilyDocument) -> String {
    const MARGIN: i32 = 24;
    const CHAR_WIDTH: i32 = 7;
    const TOPLINE_FONT_SIZE: i32 = 13;
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
        let raw_label = node
            .alias
            .as_ref()
            .map_or_else(|| node.name.clone(), |alias| format!("{} as {}", node.name, alias));
        let lines = wrap_text(raw_label, MAX_LINE_CHARS, document.text_overflow_policy);
        let width_chars = lines
            .iter()
            .map(|line| line.chars().count() as i32)
            .max()
            .unwrap_or(1);
        let width = (width_chars * CHAR_WIDTH + (NODE_PADDING_X * 2)).clamp(NODE_MIN_WIDTH, NODE_MAX_WIDTH);
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
        // Render members with visibility markers
        let show_members = !hide_empty_members || !node.members.is_empty();
        if show_members {
            let member_y_base = layout.y + NODE_PADDING_Y + (layout.label_lines.len() as i32 * 18) + 4;
            for (midx, member) in node.members.iter().enumerate() {
                let my = member_y_base + (midx as i32 * 16);
                let (symbol, color, member_text) = parse_visibility_member(member);
                if let Some(sym) = symbol {
                    out.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
                        layout.x + NODE_PADDING_X,
                        my,
                        color,
                        escape_text(sym)
                    ));
                }
                let (extra_style, clean_text) = parse_member_modifiers(member_text);
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
    if t.starts_with("{abstract}") {
        (" font-style=\"italic\"", t["{abstract}".len()..].trim_start())
    } else if t.starts_with("{static}") {
        (" text-decoration=\"underline\"", t["{static}".len()..].trim_start())
    } else {
        ("", t)
    }
}

fn family_kind_label(kind: DiagramKind) -> &'static str {
    match kind {
        DiagramKind::Class => "class",
        DiagramKind::Object => "object",
        DiagramKind::UseCase => "usecase",
        DiagramKind::Gantt => "gantt",
        DiagramKind::Chronology => "chronology",
        DiagramKind::Salt => "salt",
        DiagramKind::Component => "component",
        DiagramKind::Deployment => "deployment",
        DiagramKind::State => "state",
        DiagramKind::Activity => "activity",
        DiagramKind::Timing => "timing",
        DiagramKind::MindMap => "mindmap",
        DiagramKind::Wbs => "wbs",
        DiagramKind::Sequence => "sequence",
        DiagramKind::Json => "json",
        DiagramKind::Yaml => "yaml",
        DiagramKind::Nwdiag => "nwdiag",
        DiagramKind::Archimate => "archimate",
        DiagramKind::Regex => "regex",
        DiagramKind::Ebnf => "ebnf",
        DiagramKind::Math => "math",
        DiagramKind::Sdl => "sdl",
        DiagramKind::Ditaa => "ditaa",
        DiagramKind::Chart => "chart",
        DiagramKind::Unknown => "unknown",
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
    }
}

fn render_class_node(
    out: &mut String,
    node: &crate::model::FamilyNode,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    header_h: i32,
) {
    let (fill, stroke, header_fill) = match node.kind {
        FamilyNodeKind::Class => ("#ffffff", "#1e293b", "#dbeafe"),
        FamilyNodeKind::Object => ("#ffffff", "#1e293b", "#fef3c7"),
        FamilyNodeKind::UseCase => ("#ffffff", "#1e293b", "#dcfce7"),
        _ => ("#ffffff", "#1e293b", "#f1f5f9"),
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
                "<text x=\"{tx}\" y=\"{my}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#334155\">{m}</text>",
                tx = x + w / 2,
                m = escape_text(member)
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
        let (vis_sym, vis_color, rest_after_vis) = parse_visibility_member(member);
        let (style_attrs, text_after_mod) = parse_member_modifiers(rest_after_vis);
        // Reconstruct display text: keep visibility prefix + remaining text
        let display_text = if vis_sym.is_some() {
            format!("{}{}", vis_sym.unwrap_or(""), text_after_mod)
        } else {
            text_after_mod.to_string()
        };
        out.push_str(&format!(
            "<text x=\"{tx}\" y=\"{my}\" font-family=\"monospace\" font-size=\"11\" fill=\"{vc}\"{sa}>{m}</text>",
            tx = x + 10,
            vc = vis_color,
            sa = style_attrs,
            m = escape_text(&display_text)
        ));
        my += 16;
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
                return (stripped.trim_end().to_string(), match m {
                    "*" => "*",
                    "o" => "o",
                    "<" => "<",
                    "+" => "+",
                    _ => "",
                });
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
                return (stripped.trim_start().to_string(), match m {
                    "*" => "*",
                    "o" => "o",
                    ">" => ">",
                    "+" => "+",
                    _ => "",
                });
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

    // Time axis: use slot indices since baseline data has no explicit durations.
    // Allocate equal-width slots: max(rows, 4)
    let slot_count = (row_count).max(4);
    let slot_w = chart_w / slot_count;

    // Axis header bar
    out.push_str(&format!(
        "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"#f1f5f9\" stroke=\"#cbd5e1\" stroke-width=\"1\"/>",
        x = chart_left,
        y = chart_top - header_h,
        w = chart_w,
        h = header_h
    ));
    for i in 0..slot_count {
        let x = chart_left + i * slot_w;
        out.push_str(&format!(
            "<line x1=\"{x}\" y1=\"{y1}\" x2=\"{x}\" y2=\"{y2}\" stroke=\"#e2e8f0\" stroke-width=\"1\"/>",
            y1 = chart_top - header_h,
            y2 = chart_top + chart_h
        ));
        out.push_str(&format!(
            "<text x=\"{tx}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"11\" fill=\"#475569\">T{n}</text>",
            tx = x + 6,
            ty = chart_top - 10,
            n = i + 1
        ));
    }

    // Helper to compute bar geometry from name (slot = row_index)
    let bar_geom = |name: &str| -> (i32, i32, i32) {
        let idx = *row_index.get(name).unwrap_or(&0);
        let bx = chart_left + idx * slot_w;
        let bw = slot_w.max(40);
        (bx, bw, idx)
    };

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
        let (bx, bw, _) = bar_geom(&task.name);
        out.push_str(&format!(
            "<rect x=\"{bx}\" y=\"{y}\" width=\"{bw}\" height=\"{bh}\" rx=\"3\" ry=\"3\" fill=\"#3b82f6\" stroke=\"#1e40af\" stroke-width=\"1\"/>",
            bh = bar_height
        ));
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
        let (bx, _bw, _) = bar_geom(&milestone.name);
        let cx = bx + slot_w / 2;
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
            let (fx, fw, _) = bar_geom(&constraint.subject);
            let (tx, _tw, _) = bar_geom(&normalized_target);
            let x1 = fx + fw / 2;
            let x2 = tx + slot_w / 2;
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

    // Events
    for (i, event) in document.chronology_events.iter().enumerate() {
        let cy = line_top + (i as i32) * event_gap + event_gap / 2;
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
            x = line_x + 14,
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

pub fn render_state_svg(doc: &FamilyDocument) -> String {
    render_box_grid_svg(doc, "state")
}

fn render_box_grid_svg(doc: &FamilyDocument, family: &str) -> String {
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
        render_family_node_shape(&mut out, node, x, y, cell_w, cell_h);
        let id_name = node.name.clone();
        let id_alias = node.alias.clone();
        positions.insert(id_name, (x, y, cell_w, cell_h));
        if let Some(alias) = id_alias {
            positions.insert(alias, (x, y, cell_w, cell_h));
        }
    }

    // Draw relations as straight arrows between node center boundaries.
    for rel in &doc.relations {
        let from_box = positions.get(&rel.from);
        let to_box = positions.get(&rel.to);
        let (Some(&(fx, fy, fw, fh)), Some(&(tx, ty, tw, th))) = (from_box, to_box) else {
            continue;
        };
        let cx1 = fx + fw / 2;
        let cy1 = fy + fh / 2;
        let cx2 = tx + tw / 2;
        let cy2 = ty + th / 2;
        let (x1, y1) = clip_to_box_edge(cx1, cy1, cx2, cy2, fx, fy, fw, fh);
        let (x2, y2) = clip_to_box_edge(cx2, cy2, cx1, cy1, tx, ty, tw, th);
        let dashed = rel.arrow.contains("..") || rel.arrow.contains("--");
        let dash = if dashed && rel.arrow.contains("..") {
            " stroke-dasharray=\"4 4\""
        } else {
            ""
        };
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#334155\" stroke-width=\"1.5\"{}/>",
            x1, y1, x2, y2, dash
        ));
        // arrowhead
        out.push_str(&arrowhead_svg(x1, y1, x2, y2, "#334155"));
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

fn wrap_text(text: String, max_chars: usize, policy: crate::scene::TextOverflowPolicy) -> Vec<String> {
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

    let mut chars = text.chars();
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
    cx: i32,
    cy: i32,
    tx: i32,
    ty: i32,
    bx: i32,
    by: i32,
    bw: i32,
    bh: i32,
) -> (i32, i32) {
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

fn arrowhead_svg(_x1: i32, _y1: i32, x2: i32, y2: i32, color: &str) -> String {
    // small triangle arrowhead pointing in the direction from (x1,y1) -> (x2,y2)
    let dx = (x2 - _x1) as f64;
    let dy = (y2 - _y1) as f64;
    let len = (dx * dx + dy * dy).sqrt().max(1.0);
    let ux = dx / len;
    let uy = dy / len;
    let size = 8.0;
    let bx = (x2 as f64) - ux * size;
    let by = (y2 as f64) - uy * size;
    let px = -uy;
    let py = ux;
    let lx = bx + px * (size / 2.0);
    let ly = by + py * (size / 2.0);
    let rx = bx - px * (size / 2.0);
    let ry = by - py * (size / 2.0);
    format!(
        "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\"/>",
        x2, y2, lx as i32, ly as i32, rx as i32, ry as i32, color
    )
}

fn render_family_node_shape(out: &mut String, node: &FamilyNode, x: i32, y: i32, w: i32, h: i32) {
    let cx = x + w / 2;
    let cy = y + h / 2;
    let display = node
        .label
        .clone()
        .unwrap_or_else(|| node.name.clone());
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

pub fn render_activity_svg(doc: &FamilyDocument) -> String {
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

    let cx = width / 2;
    let box_w = 220;
    let mut last_y: Option<i32> = None;

    for (idx, node) in doc.nodes.iter().enumerate() {
        let y = header_h + (idx as i32) * step_h;
        let label = node.label.clone().unwrap_or_default();
        match node.kind {
            FamilyNodeKind::ActivityStart => {
                out.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"12\" fill=\"#0f172a\"/>",
                    cx,
                    y + 20
                ));
            }
            FamilyNodeKind::ActivityStop => {
                out.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"14\" fill=\"white\" stroke=\"#0f172a\" stroke-width=\"1.5\"/>",
                    cx,
                    y + 20
                ));
                out.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"7\" fill=\"#0f172a\"/>",
                    cx,
                    y + 20
                ));
            }
            FamilyNodeKind::ActivityAction => {
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"36\" rx=\"18\" ry=\"18\" fill=\"#ecfdf5\" stroke=\"#047857\" stroke-width=\"1.5\"/>",
                    cx - box_w / 2,
                    y + 4,
                    box_w
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
                    "<polygon points=\"{},{} {},{} {},{} {},{}\" fill=\"#fef9c3\" stroke=\"#a16207\" stroke-width=\"1.5\"/>",
                    cx,
                    y + 2,
                    cx + dx,
                    y + 2 + dy,
                    cx,
                    y + 2 + (dy * 2),
                    cx - dx,
                    y + 2 + dy
                ));
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\">{}</text>",
                    cx,
                    y + 2 + dy + 4,
                    escape_text(&label)
                ));
            }
            FamilyNodeKind::ActivityFork | FamilyNodeKind::ActivityForkEnd => {
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"8\" fill=\"#0f172a\"/>",
                    cx - box_w / 2,
                    y + 24,
                    box_w
                ));
            }
            FamilyNodeKind::ActivityMerge => {
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#475569\">{}</text>",
                    cx,
                    y + 28,
                    escape_text(&format!("(merge) {}", label))
                ));
            }
            FamilyNodeKind::ActivityPartition => {
                out.push_str(&format!(
                    "<rect x=\"24\" y=\"{}\" width=\"{}\" height=\"36\" rx=\"4\" ry=\"4\" fill=\"#f1f5f9\" stroke=\"#475569\" stroke-width=\"1\" stroke-dasharray=\"4 3\"/>",
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
        if let Some(prev_y) = last_y {
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#0f172a\" stroke-width=\"1.5\"/>",
                cx,
                prev_y + 42,
                cx,
                y
            ));
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{}\" fill=\"#0f172a\"/>",
                cx,
                y,
                cx - 4,
                y - 6,
                cx + 4,
                y - 6
            ));
        }
        last_y = Some(y);
    }

    out.push_str("</svg>");
    out
}

pub fn render_timing_svg(doc: &FamilyDocument) -> String {
    // Group nodes: signals (TimingConcise/Robust/Clock/Binary) form rows; TimingEvent are points.
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

    let mut times: Vec<&str> = events.iter().map(|e| e.name.as_str()).collect();
    times.sort();
    times.dedup();
    let n_times = times.len().max(2) as i32;
    let row_h = 56i32;
    let header_h = 60i32;
    let left_pad = 120i32;
    let right_pad = 40i32;
    let col_w = 80i32;
    let width = left_pad + right_pad + n_times * col_w;
    let height = header_h + (signals.len().max(1) as i32) * row_h + 60;

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
                "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\">{}</text>",
                y_cursor,
                escape_text(line)
            ));
            y_cursor += 22;
        }
    }
    out.push_str(&format!(
        "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">timing diagram</text>",
        y_cursor + 2
    ));

    // Time axis labels
    for (i, t) in times.iter().enumerate() {
        let x = left_pad + (i as i32) * col_w;
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#cbd5e1\" stroke-width=\"1\" stroke-dasharray=\"3 3\"/>",
            x,
            header_h - 6,
            x,
            header_h + (signals.len().max(1) as i32) * row_h
        ));
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#1e293b\">@{}</text>",
            x,
            header_h - 10,
            escape_text(t)
        ));
    }

    for (row_idx, signal) in signals.iter().enumerate() {
        let y = header_h + (row_idx as i32) * row_h;
        // Row label
        out.push_str(&format!(
            "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" font-weight=\"600\" fill=\"#0f172a\">{}</text>",
            y + row_h / 2,
            escape_text(&signal.name)
        ));
        out.push_str(&format!(
            "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" fill=\"#475569\">{}</text>",
            y + row_h / 2 + 14,
            family_node_label(signal.kind)
        ));
        // Baseline
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#94a3b8\" stroke-width=\"1\"/>",
            left_pad,
            y + row_h - 12,
            width - right_pad,
            y + row_h - 12
        ));

        // Transitions for this signal
        let mut last_x: Option<i32> = None;
        let mut last_state: Option<String> = None;
        for ev in events.iter().filter(|e| e.alias.as_deref() == Some(signal.name.as_str())) {
            let t_idx = times.iter().position(|t| *t == ev.name.as_str()).unwrap_or(0);
            let x = left_pad + (t_idx as i32) * col_w;
            let state = ev.members.first().cloned().unwrap_or_default();
            // vertical transition
            if let (Some(lx), Some(_ls)) = (last_x, last_state.as_ref()) {
                out.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#0f172a\" stroke-width=\"1.5\"/>",
                    lx,
                    y + row_h - 28,
                    x,
                    y + row_h - 28
                ));
            }
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#0f172a\" stroke-width=\"1.5\"/>",
                x,
                y + row_h - 12,
                x,
                y + row_h - 28
            ));
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#1e293b\">{}</text>",
                x,
                y + row_h - 32,
                escape_text(&state)
            ));
            last_x = Some(x);
            last_state = Some(state);
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
    let layers = ["strategy", "business", "application", "technology", "motivation"];
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
        "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">math (LaTeX-like, deterministic stub)</text>"
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
        "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">ditaa (ASCII art frame, deterministic stub)</text>"
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
        "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">SDL state machine (deterministic stub)</text>"
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
        ),
        ChartSubtype::Line => render_chart_line(
            &mut out,
            &document.data,
            plot_left,
            plot_top,
            plot_right,
            plot_bottom,
        ),
        ChartSubtype::Pie => {
            render_chart_pie(&mut out, &document.data, width / 2, (plot_top + plot_bottom) / 2)
        }
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
) {
    out.push_str(&format!(
        "<line x1=\"{l}\" y1=\"{b}\" x2=\"{r}\" y2=\"{b}\" stroke=\"#0f172a\" stroke-width=\"1\"/>",
        l = left,
        r = right,
        b = bottom
    ));
    out.push_str(&format!(
        "<line x1=\"{l}\" y1=\"{t}\" x2=\"{l}\" y2=\"{b}\" stroke=\"#0f172a\" stroke-width=\"1\"/>",
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
        let color = CHART_PALETTE[idx % CHART_PALETTE.len()];
        out.push_str(&format!(
            "<rect x=\"{bx}\" y=\"{by}\" width=\"{bw}\" height=\"{bh}\" fill=\"{color}\" stroke=\"#0f172a\" stroke-width=\"0.5\"/>",
            bx = bx,
            by = by,
            bw = bar_w,
            bh = bh
        ));
        out.push_str(&format!(
            "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#0f172a\">{}</text>",
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
) {
    out.push_str(&format!(
        "<line x1=\"{l}\" y1=\"{b}\" x2=\"{r}\" y2=\"{b}\" stroke=\"#0f172a\" stroke-width=\"1\"/>",
        l = left,
        r = right,
        b = bottom
    ));
    out.push_str(&format!(
        "<line x1=\"{l}\" y1=\"{t}\" x2=\"{l}\" y2=\"{b}\" stroke=\"#0f172a\" stroke-width=\"1\"/>",
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
            "<circle cx=\"{px}\" cy=\"{py}\" r=\"3\" fill=\"#1d4ed8\"/>"
        ));
        out.push_str(&format!(
            "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#0f172a\">{}</text>",
            escape_text(&point.label),
            tx = px,
            ty = bottom + 16
        ));
    }
    out.push_str(&format!(
        "<polyline points=\"{}\" fill=\"none\" stroke=\"#1d4ed8\" stroke-width=\"1.5\"/>",
        points
    ));
}

fn render_chart_pie(out: &mut String, data: &[crate::model::ChartPoint], cx: i32, cy: i32) {
    let radius = 120_i32;
    let total: f64 = data.iter().map(|p| p.value.max(0.0)).sum();
    if total <= 0.0 {
        out.push_str(&format!(
            "<circle cx=\"{cx}\" cy=\"{cy}\" r=\"{r}\" fill=\"#e2e8f0\" stroke=\"#0f172a\" stroke-width=\"1\"/>",
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
        let color = CHART_PALETTE[idx % CHART_PALETTE.len()];
        out.push_str(&format!(
            "<path d=\"M {cx} {cy} L {x1:.2} {y1:.2} A {r} {r} 0 {large} 1 {x2:.2} {y2:.2} Z\" fill=\"{color}\" stroke=\"#0f172a\" stroke-width=\"0.5\"/>",
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
            "<text x=\"{lx:.0}\" y=\"{ly:.0}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#fff\">{}</text>",
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
fn creole_text(
    x: i32,
    y: i32,
    extra_attrs: &str,
    label: &str,
    base_color: &str,
) -> String {
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
            let shadow_attr = if scene.style.shadowing { " filter=\"url(#shadow)\"" } else { "" };
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
