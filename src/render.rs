use crate::ast::{DiagramKind, MemberModifier, NoteKind};
use crate::creole::{render_creole_to_svg_tspans, tokenize_creole};
use crate::model::{
    ArchimateDocument, ChartDocument, ChartLabelMode, ChartSubtype, DitaaDocument, EbnfDocument,
    EbnfToken, FamilyDocument, FamilyGroup, FamilyNode, FamilyNodeKind, FamilyOrientation,
    FamilyStyle, JsonDocument, LegendHAlign, LegendVAlign, MathDocument, MindMapSide,
    NwdiagDocument, ParticipantRole, RegexDocument, RegexToken, RepeatKind, ScaleSpec, SdlDocument,
    SdlStateKind, StateDocument, StateNode, StateNodeKind, TimelineChronologyEvent,
    TimelineDocument, TimelineMilestone, TimelineTask, VirtualEndpointKind, WbsCheckbox,
    YamlDocument,
};
use crate::scene::{LifecycleMarkerKind, ParticipantBox, Scene, StructureKind};
use crate::theme::css3_color_to_hex;
use crate::theme::{ActivityStyle, ClassStyle, ComponentStyle, MessageAlign};
use std::collections::BTreeMap;

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

    if let Some(header) = &scene.header {
        render_sequence_metadata_label(
            &mut out,
            header,
            "sequence-header",
            "font-family=\"monospace\" font-size=\"12\"",
            "#333",
            16,
        );
    }

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

    for a in &scene.activations {
        let offset = (a.depth as i32) * 6;
        let x = a.x + offset - 5;
        let y = a.y1.min(a.y2);
        let height = (a.y2 - a.y1).abs().max(12);
        out.push_str(&format!(
            "<rect class=\"sequence-activation\" data-participant=\"{}\" x=\"{}\" y=\"{}\" width=\"10\" height=\"{}\" fill=\"#ffffff\" stroke=\"{}\" stroke-width=\"1\"/>",
            escape_text(&a.participant_id),
            x,
            y,
            height,
            scene.style.lifeline_border_color
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
    let mut parallel_label_lanes: BTreeMap<i32, i32> = BTreeMap::new();
    for m in &scene.messages {
        let stroke_color = m
            .style
            .color
            .as_deref()
            .and_then(normalize_message_color)
            .unwrap_or(message_line_color);
        let arrow_fill = m
            .style
            .color
            .as_deref()
            .and_then(normalize_message_color)
            .unwrap_or(scene.style.arrow_color.as_str());
        let stroke_dash = if m.style.dotted {
            " stroke-dasharray=\"2 4\""
        } else if m.style.dashed || m.arrow.contains("--") {
            " stroke-dasharray=\"6 4\""
        } else {
            ""
        };
        let hidden = if m.style.hidden {
            " visibility=\"hidden\""
        } else {
            ""
        };
        let stroke_width = m
            .style
            .thickness
            .map(f32::from)
            .unwrap_or(1.5)
            .clamp(1.0, 8.0);
        if m.x1 == m.x2 {
            let loop_w = 46;
            let loop_h = 26;
            let dir = if m.arrow.starts_with('<') { -1 } else { 1 };
            let x2 = m.x1 + dir * loop_w;
            out.push_str(&format!(
                "<path d=\"M {} {} C {} {}, {} {}, {} {} S {} {}, {} {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}/>",
                m.x1,
                m.y,
                x2,
                m.y,
                x2,
                m.y + loop_h,
                m.x1,
                m.y + loop_h,
                x2,
                m.y + loop_h * 2,
                m.x1,
                m.y + loop_h * 2,
                stroke_color,
                stroke_width,
                stroke_dash,
                hidden
            ));
        } else {
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}{}/>",
                m.x1, m.y, m.x2, m.y, stroke_color, stroke_width, stroke_dash, hidden
            ));
        }
        render_sequence_arrow_heads(&mut out, m, stroke_color, arrow_fill, stroke_width, hidden);

        if let Some(virtual_ep) = m.from_virtual {
            render_virtual_endpoint_marker(&mut out, m.x1, m.y, virtual_ep.kind);
        }
        if let Some(virtual_ep) = m.to_virtual {
            render_virtual_endpoint_marker(&mut out, m.x2, m.y, virtual_ep.kind);
        }

        if !m.label_lines.is_empty() {
            let (tx, anchor) = sequence_message_label_anchor(m.x1, m.x2, scene.style.message_align);
            let below = scene.style.response_message_below_arrow && m.arrow.starts_with('<');
            let lane_offset = if m.style.parallel || below {
                let lane = parallel_label_lanes.entry(m.y).or_insert(0);
                let offset = *lane * MESSAGE_LABEL_LINE_GAP;
                *lane += (m.label_lines.len() as i32).max(1);
                offset
            } else {
                0
            };
            let start_y = if m.style.parallel || below {
                m.y + 16 + lane_offset
            } else {
                m.y - 8 - (((m.label_lines.len() as i32) - 1) * MESSAGE_LABEL_LINE_GAP)
            };
            for (idx, line) in m.label_lines.iter().enumerate() {
                out.push_str(&creole_text(
                    tx,
                    start_y + (idx as i32 * MESSAGE_LABEL_LINE_GAP),
                    &format!("text-anchor=\"{anchor}\" font-family=\"monospace\" font-size=\"12\""),
                    line,
                    "black",
                ));
            }
        } else if let Some(label) = &m.label {
            let (tx, anchor) = sequence_message_label_anchor(m.x1, m.x2, scene.style.message_align);
            let ty = if scene.style.response_message_below_arrow && m.arrow.starts_with('<') {
                m.y + 16
            } else {
                m.y - 8
            };
            out.push_str(&creole_text(
                tx,
                ty,
                &format!("text-anchor=\"{anchor}\" font-family=\"monospace\" font-size=\"12\""),
                label,
                "black",
            ));
        }
    }

    for n in &scene.notes {
        render_sequence_note_shape(&mut out, n.kind, n.x, n.y, n.width, n.height, scene);

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

    for marker in &scene.lifecycle_markers {
        match marker.kind {
            LifecycleMarkerKind::Create => {
                out.push_str(&format!(
                    "<circle class=\"sequence-create\" data-participant=\"{}\" cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"#dcfce7\" stroke=\"#15803d\" stroke-width=\"1.5\"/>",
                    escape_text(&marker.participant_id),
                    marker.x,
                    marker.y
                ));
            }
            LifecycleMarkerKind::Destroy => {
                out.push_str(&format!(
                    "<g class=\"sequence-destroy\" data-participant=\"{}\" stroke=\"#b91c1c\" stroke-width=\"2\"><line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"/><line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"/></g>",
                    escape_text(&marker.participant_id),
                    marker.x - 6,
                    marker.y - 6,
                    marker.x + 6,
                    marker.y + 6,
                    marker.x - 6,
                    marker.y + 6,
                    marker.x + 6,
                    marker.y - 6
                ));
            }
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

    if let Some(caption) = &scene.caption {
        render_sequence_metadata_label(
            &mut out,
            caption,
            "sequence-caption",
            "font-family=\"monospace\" font-size=\"12\" font-style=\"italic\"",
            "#333",
            16,
        );
    }

    if let Some(footer) = &scene.footer {
        render_sequence_metadata_label(
            &mut out,
            footer,
            "sequence-footer",
            "font-family=\"monospace\" font-size=\"12\"",
            "#333",
            16,
        );
    }

    // Render legend if present.
    if let Some(legend_text) = &scene.legend_text {
        render_legend(&mut out, legend_text, scene);
    }

    out.push_str("</svg>");
    out
}

fn render_sequence_metadata_label(
    out: &mut String,
    label: &crate::scene::Label,
    class_name: &str,
    attrs: &str,
    color: &str,
    line_gap: i32,
) {
    out.push_str(&format!("<g class=\"{}\">", escape_text(class_name)));
    for (idx, line) in label.lines.iter().enumerate() {
        out.push_str(&creole_text(
            label.x,
            label.y + (idx as i32 * line_gap),
            attrs,
            line,
            color,
        ));
    }
    out.push_str("</g>");
}

fn sequence_message_label_anchor(x1: i32, x2: i32, align: MessageAlign) -> (i32, &'static str) {
    let left = x1.min(x2);
    let right = x1.max(x2);
    match align {
        MessageAlign::Left => (left + 8, "start"),
        MessageAlign::Center => (((x1 + x2) / 2) + 2, "middle"),
        MessageAlign::Right => (right - 8, "end"),
    }
}

fn render_sequence_arrow_heads(
    out: &mut String,
    m: &crate::scene::MessageLine,
    stroke_color: &str,
    fill_color: &str,
    stroke_width: f32,
    hidden: &str,
) {
    let head_stroke_width = m
        .style
        .thickness
        .map(f32::from)
        .unwrap_or(1.0)
        .clamp(1.0, 8.0);
    let raw_arrow = m.arrow.as_str();
    let arrow = raw_arrow.replace(['/', '\\'], "");
    let left_marker = arrow.chars().next().filter(|c| matches!(c, 'o' | 'x'));
    let right_marker = arrow.chars().last().filter(|c| matches!(c, 'o' | 'x'));
    let left_arrow = arrow.starts_with('<') || arrow.starts_with("<<");
    let left_slant = sequence_arrow_head_slant(raw_arrow, true);
    let right_slant = sequence_arrow_head_slant(raw_arrow, false);
    let right_arrow =
        (arrow.contains('>') || right_slant.is_some()) && !matches!(right_marker, Some('o' | 'x'));
    let open_head = arrow.contains(">>") || arrow.contains("<<");

    if left_arrow {
        render_arrow_head(
            out,
            ArrowHeadRender {
                point: (m.x1, m.y),
                from_to_x: (m.x2, m.x1),
                open: open_head,
                slant: left_slant,
                colors: (stroke_color, fill_color),
                stroke_width: head_stroke_width,
                hidden,
            },
        );
    }
    if right_arrow {
        render_arrow_head(
            out,
            ArrowHeadRender {
                point: (m.x2, m.y),
                from_to_x: (m.x1, m.x2),
                open: open_head,
                slant: right_slant,
                colors: (stroke_color, fill_color),
                stroke_width: head_stroke_width,
                hidden,
            },
        );
    }
    if let Some(marker) = left_marker {
        render_arrow_endpoint_marker(out, m.x1, m.y, marker, stroke_color, stroke_width, hidden);
    }
    if let Some(marker) = right_marker {
        render_arrow_endpoint_marker(out, m.x2, m.y, marker, stroke_color, stroke_width, hidden);
    }
}

struct ArrowHeadRender<'a> {
    point: (i32, i32),
    from_to_x: (i32, i32),
    open: bool,
    slant: Option<char>,
    colors: (&'a str, &'a str),
    stroke_width: f32,
    hidden: &'a str,
}

fn render_arrow_head(out: &mut String, head: ArrowHeadRender<'_>) {
    let (x, y) = head.point;
    let (from_x, to_x) = head.from_to_x;
    let (stroke_color, fill_color) = head.colors;
    let dir = if to_x >= from_x { 1 } else { -1 };
    let back = x - (dir * 8);
    if let Some(slant) = head.slant {
        let back_y = match slant {
            '/' => y + (dir * 5),
            '\\' => y - (dir * 5),
            _ => y,
        };
        let slant_name = if slant == '/' { "slash" } else { "backslash" };
        out.push_str(&format!(
            "<line class=\"sequence-arrow-head sequence-arrow-head-{}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
            slant_name,
            back,
            back_y,
            x,
            y,
            stroke_color,
            head.stroke_width,
            head.hidden
        ));
    } else if head.open {
        out.push_str(&format!(
            "<polyline points=\"{},{} {},{} {},{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
            back,
            y - 5,
            x,
            y,
            back,
            y + 5,
            stroke_color,
            head.stroke_width,
            head.hidden
        ));
    } else {
        out.push_str(&format!(
            "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
            x,
            y,
            back,
            y - 5,
            back,
            y + 5,
            fill_color,
            stroke_color,
            head.stroke_width,
            head.hidden
        ));
    }
}

fn sequence_arrow_head_slant(raw_arrow: &str, left: bool) -> Option<char> {
    let mut marker = None;
    let mut saw_head = false;
    for ch in raw_arrow.chars() {
        if matches!(ch, '/' | '\\') {
            marker = Some(ch);
            continue;
        }
        if left && ch == '<' {
            return marker;
        }
        if !left && ch == '>' {
            saw_head = true;
        }
        if saw_head && matches!(ch, '/' | '\\') {
            return Some(ch);
        }
    }
    if left {
        None
    } else {
        marker
    }
}

fn render_arrow_endpoint_marker(
    out: &mut String,
    x: i32,
    y: i32,
    marker: char,
    stroke_color: &str,
    stroke_width: f32,
    hidden: &str,
) {
    match marker {
        'o' => out.push_str(&format!(
            "<circle class=\"sequence-arrow-end sequence-arrow-end-circle\" data-sequence-arrow-end=\"circle\" cx=\"{}\" cy=\"{}\" r=\"4\" fill=\"white\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
            x, y, stroke_color, stroke_width, hidden
        )),
        'x' => out.push_str(&format!(
            "<g class=\"sequence-arrow-end sequence-arrow-end-cross\" data-sequence-arrow-end=\"cross\" stroke=\"{}\" stroke-width=\"{}\"{}><line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"/><line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"/></g>",
            stroke_color,
            stroke_width,
            hidden,
            x - 4,
            y - 4,
            x + 4,
            y + 4,
            x - 4,
            y + 4,
            x + 4,
            y - 4
        )),
        _ => {}
    }
}

fn normalize_message_color(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.starts_with('#') {
        return Some(value);
    }
    css3_color_to_hex(value).or(Some(value))
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
    let group_frames = collect_render_group_frames(&document.groups);
    let max_group_depth = group_frames
        .iter()
        .map(|frame| frame.depth)
        .max()
        .unwrap_or(0);
    let group_top_reserve = if group_frames.is_empty() {
        0
    } else {
        ((max_group_depth as i32) + 1) * 24
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
        let body_h = if node.kind == FamilyNodeKind::Note {
            let lines = node
                .label
                .as_deref()
                .unwrap_or(&node.name)
                .lines()
                .count()
                .max(1) as i32;
            lines * 16 + 20
        } else if node.members.is_empty() {
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
        let mut y = margin_top + title_block_height + group_top_reserve;
        for (i, h) in row_heights.iter().enumerate() {
            row_y_offsets[i] = y;
            y += h + row_gap;
        }
    }

    for (idx, node) in document.nodes.iter().enumerate() {
        let col = (idx as i32) % col_count;
        let row = (idx as i32) / col_count;
        let body_h = if node.kind == FamilyNodeKind::Note {
            let lines = node
                .label
                .as_deref()
                .unwrap_or(&node.name)
                .lines()
                .count()
                .max(1) as i32;
            lines * 16 + 20
        } else if node.members.is_empty() {
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
        let usecase_dependency = usecase_dependency_label(relation.label.as_deref())
            .or_else(|| usecase_dependency_label(relation.stereotype.as_deref()));
        if usecase_dependency.is_some() {
            style.dashed = true;
            if style.end_marker.is_none() {
                style.end_marker = Some("arrow-open");
            }
        }
        let (x1, y1, x2, y2) = compute_edge_anchors_for_direction(
            (from.x, from.y, from.w, from.h),
            (to.x, to.y, to.w, to.h),
            relation.direction.as_deref(),
        );
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
        let pad = 12 + depth_outset;
        let label_header = 20 + depth_outset; // extra space at top for the group label
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

fn compute_edge_anchors_for_direction(
    from: (i32, i32, i32, i32),
    to: (i32, i32, i32, i32),
    direction: Option<&str>,
) -> (i32, i32, i32, i32) {
    let (fx, fy, fw, fh) = from;
    let (tx, ty, tw, th) = to;
    match direction {
        Some("left") => (fx, fy + fh / 2, tx + tw, ty + th / 2),
        Some("right") => (fx + fw, fy + fh / 2, tx, ty + th / 2),
        Some("up") => (fx + fw / 2, fy, tx + tw / 2, ty + th),
        Some("down") => (fx + fw / 2, fy + fh, tx + tw / 2, ty),
        _ => compute_edge_anchors_tuple(from, to),
    }
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
        let lines = extract_yaml_kv_lines(body);
        if !lines.is_empty() {
            return lines;
        }
    }
    extract_json_kv_lines(body)
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
pub fn render_salt_svg(document: &FamilyDocument) -> String {
    const CELL_H: i32 = 28;
    const CELL_PAD_X: i32 = 10;
    const MARGIN: i32 = 24;
    const MIN_CELL_W: i32 = 80;

    // Parse rows from the encoded node names.
    let mut rows: Vec<Vec<SaltCellRender>> = Vec::new();
    let mut salt_state = SaltTransformState::default();
    let mut style = SaltRenderStyle::default();
    for node in &document.nodes {
        if let Some(rest) = node.name.strip_prefix("SALT_ROW\x1f") {
            let cells: Vec<SaltCellRender> = rest.split('\x1e').map(decode_salt_cell).collect();
            if let Some(cells) = transform_salt_row(cells, &mut salt_state, &mut style) {
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
        "<rect data-salt-style=\"canvas\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
        total_w, svg_h, style.canvas_fill
    ));

    // Outer border
    out.push_str(&format!(
        "<rect data-salt-style=\"panel\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        MARGIN,
        MARGIN + title_h,
        total_w - MARGIN * 2,
        total_h - MARGIN * 2,
        style.panel_fill,
        style.border_color
    ));

    if let Some(title) = &document.title {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"{}\" font-size=\"13\" font-weight=\"600\" fill=\"{}\">{}</text>",
            MARGIN,
            MARGIN - 6,
            style.font_family,
            style.text_color,
            escape_text(title)
        ));
    }

    // Draw rows and cells.
    for (row_idx, cells) in rows.iter().enumerate() {
        let row_y = MARGIN + title_h + (row_idx as i32) * CELL_H;
        if is_salt_separator_row(cells) {
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                MARGIN + 4,
                row_y + CELL_H / 2,
                total_w - MARGIN - 4,
                row_y + CELL_H / 2,
                style.border_color
            ));
            continue;
        }
        let mut col_x = MARGIN;

        for (col_idx, cell) in cells.iter().enumerate() {
            let cell_w = col_widths.get(col_idx).copied().unwrap_or(MIN_CELL_W);
            render_salt_cell_svg(&mut out, cell, col_x, row_y, cell_w, CELL_H, &style);
            col_x += cell_w;
        }

        // Row separator line (skip the last row)
        if row_idx + 1 < rows.len() {
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
                MARGIN,
                row_y + CELL_H,
                total_w - MARGIN,
                row_y + CELL_H,
                style.grid_color
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
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
                    col_x,
                    MARGIN + title_h,
                    col_x,
                    MARGIN + title_h + total_h - MARGIN * 2,
                    style.grid_color
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

struct SaltRenderStyle {
    canvas_fill: String,
    panel_fill: String,
    header_fill: String,
    input_fill: String,
    button_fill: String,
    menu_fill: String,
    tab_fill: String,
    scroll_fill: String,
    checkbox_fill: String,
    radio_fill: String,
    accent_fill: String,
    border_color: String,
    grid_color: String,
    text_color: String,
    header_text_color: String,
    input_text_color: String,
    button_text_color: String,
    muted_text_color: String,
    font_family: &'static str,
}

impl Default for SaltRenderStyle {
    fn default() -> Self {
        Self {
            canvas_fill: "#f5f5f5".to_string(),
            panel_fill: "white".to_string(),
            header_fill: "#e2e8f0".to_string(),
            input_fill: "white".to_string(),
            button_fill: "#e8e8e8".to_string(),
            menu_fill: "#eef2ff".to_string(),
            tab_fill: "#eef2ff".to_string(),
            scroll_fill: "#eef2ff".to_string(),
            checkbox_fill: "white".to_string(),
            radio_fill: "white".to_string(),
            accent_fill: "#eef2ff".to_string(),
            border_color: "#555".to_string(),
            grid_color: "#ccc".to_string(),
            text_color: "#222".to_string(),
            header_text_color: "#222".to_string(),
            input_text_color: "#222".to_string(),
            button_text_color: "#222".to_string(),
            muted_text_color: "#aaa".to_string(),
            font_family: "monospace",
        }
    }
}

impl SaltRenderStyle {
    fn set(&mut self, key: &str, value: &str) -> bool {
        let value = normalize_salt_color(value).unwrap_or_else(|| value.trim().to_string());
        match key.to_ascii_lowercase().as_str() {
            "backgroundcolor" | "saltbackgroundcolor" | "canvascolor" => {
                self.canvas_fill = value;
                true
            }
            "saltpanelcolor" | "panelcolor" | "saltfillcolor" => {
                self.panel_fill = value;
                true
            }
            "saltheadercolor" | "headercolor" | "tableheadercolor" => {
                self.header_fill = value;
                true
            }
            "saltinputcolor" | "saltinputbackgroundcolor" | "inputbackgroundcolor" => {
                self.input_fill = value;
                true
            }
            "saltbuttoncolor" | "saltbuttonbackgroundcolor" | "buttonbackgroundcolor" => {
                self.button_fill = value;
                true
            }
            "saltmenucolor" | "saltmenubackgroundcolor" | "menubackgroundcolor" => {
                self.menu_fill = value;
                true
            }
            "salttabcolor" | "salttabbackgroundcolor" | "tabbackgroundcolor" => {
                self.tab_fill = value;
                true
            }
            "saltscrollbarcolor" | "scrollbarcolor" | "scrollbarbackgroundcolor" => {
                self.scroll_fill = value;
                true
            }
            "saltcheckboxcolor" | "checkboxbackgroundcolor" => {
                self.checkbox_fill = value;
                true
            }
            "saltradiocolor" | "radiobackgroundcolor" => {
                self.radio_fill = value;
                true
            }
            "saltaccentcolor" | "accentcolor" => {
                self.accent_fill = value;
                true
            }
            "bordercolor" | "saltbordercolor" => {
                self.border_color = value;
                true
            }
            "saltgridcolor" | "gridcolor" => {
                self.grid_color = value;
                true
            }
            "fontcolor" | "saltfontcolor" => {
                self.text_color = value;
                true
            }
            "saltheaderfontcolor" | "headerfontcolor" => {
                self.header_text_color = value;
                true
            }
            "saltinputfontcolor" | "inputfontcolor" => {
                self.input_text_color = value;
                true
            }
            "saltbuttonfontcolor" | "buttonfontcolor" => {
                self.button_text_color = value;
                true
            }
            "saltmutedfontcolor" | "mutedfontcolor" => {
                self.muted_text_color = value;
                true
            }
            "handwritten" if value.eq_ignore_ascii_case("true") => {
                self.font_family = "Comic Sans MS, cursive";
                true
            }
            _ => false,
        }
    }

    fn set_scoped(&mut self, scope: Option<&str>, key: &str, value: &str) -> bool {
        let Some(scope) = scope else {
            return self.set(key, value);
        };
        let scope = scope
            .trim()
            .trim_matches('{')
            .split_whitespace()
            .last()
            .unwrap_or(scope)
            .to_ascii_lowercase();
        let key = key.trim();
        let lower_key = key.to_ascii_lowercase();
        let mapped = match (scope.as_str(), lower_key.as_str()) {
            ("saltdiagram" | "salt", _) => key.to_string(),
            ("button", "backgroundcolor") => "saltButtonBackgroundColor".to_string(),
            ("button", "fontcolor") => "saltButtonFontColor".to_string(),
            ("input" | "textfield" | "textarea", "backgroundcolor") => {
                "saltInputBackgroundColor".to_string()
            }
            ("input" | "textfield" | "textarea", "fontcolor") => "saltInputFontColor".to_string(),
            ("header", "backgroundcolor") => "saltHeaderColor".to_string(),
            ("header", "fontcolor") => "saltHeaderFontColor".to_string(),
            ("menu", "backgroundcolor") => "saltMenuBackgroundColor".to_string(),
            ("tab", "backgroundcolor") => "saltTabBackgroundColor".to_string(),
            ("scrollbar", "backgroundcolor") => "saltScrollbarColor".to_string(),
            ("checkbox", "backgroundcolor") => "saltCheckboxColor".to_string(),
            ("radio", "backgroundcolor") => "saltRadioColor".to_string(),
            (_, "linecolor" | "bordercolor") => "saltBorderColor".to_string(),
            _ => key.to_string(),
        };
        self.set(&mapped, value)
    }
}

fn normalize_salt_color(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_matches('"');
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with('#') {
        Some(trimmed.to_string())
    } else {
        Some(css3_color_to_hex(trimmed).unwrap_or(trimmed).to_string())
    }
}

fn apply_salt_style_directive(line: &str, style: &mut SaltRenderStyle) -> bool {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("!option ") {
        let original_rest = trimmed[trimmed.len() - rest.len()..].trim();
        if let Some((key, value)) = original_rest.split_once(char::is_whitespace) {
            return style.set(key.trim(), value.trim());
        }
    }
    if let Some(rest) = lower
        .strip_prefix("skinparam salt")
        .or_else(|| lower.strip_prefix("skinparam "))
    {
        let offset = trimmed.len() - rest.len();
        let original_rest = trimmed[offset..].trim();
        if let Some((key, value)) = original_rest.split_once(char::is_whitespace) {
            return style.set(key.trim(), value.trim());
        }
    }
    if let Some(rest) = trimmed.strip_prefix("saltstyle ") {
        if let Some((key, value)) = rest.split_once('=') {
            return style.set(key.trim(), value.trim());
        }
        if let Some((key, value)) = rest.split_once(char::is_whitespace) {
            return style.set(key.trim(), value.trim());
        }
    }
    false
}

/// A decoded salt cell ready for rendering.
enum SaltCellRender {
    Label(String),
    Header(String),
    TableEmpty,
    TableSpan,
    SpriteDef(String),
    SpriteRef(String),
    Input(String),
    Button(String),
    Combo(String),
    CheckboxChecked(String),
    CheckboxUnchecked(String),
    RadioOn(String),
    RadioOff(String),
    TreeItem {
        depth: usize,
        label: String,
    },
    TextAreaLine {
        text: String,
        scroll_vertical: bool,
        scroll_horizontal: bool,
    },
    GroupBox(String),
    MenuBar(Vec<String>),
    TabBar {
        tabs: Vec<String>,
        active: usize,
    },
    ScrollBar {
        vertical: bool,
        percent: u8,
    },
}

impl SaltCellRender {
    fn text(&self) -> &str {
        match self {
            Self::Label(t)
            | Self::Header(t)
            | Self::Input(t)
            | Self::Button(t)
            | Self::Combo(t)
            | Self::CheckboxChecked(t)
            | Self::CheckboxUnchecked(t)
            | Self::RadioOn(t)
            | Self::RadioOff(t) => t,
            Self::TableEmpty => "",
            Self::TableSpan => "span",
            Self::SpriteDef(name) | Self::SpriteRef(name) => name,
            Self::TreeItem { label, .. } => label,
            Self::TextAreaLine { text, .. } => text,
            Self::GroupBox(label) => label,
            Self::MenuBar(items) => items.first().map(String::as_str).unwrap_or("menu"),
            Self::TabBar { tabs, .. } => tabs.first().map(String::as_str).unwrap_or("tab"),
            Self::ScrollBar { .. } => "scrollbar",
        }
    }
}

#[derive(Default)]
struct SaltTransformState {
    in_tree: bool,
    in_text_area: bool,
    in_style: bool,
    style_scope: Option<String>,
    table_header_pending: bool,
}

fn transform_salt_row(
    cells: Vec<SaltCellRender>,
    state: &mut SaltTransformState,
    style: &mut SaltRenderStyle,
) -> Option<Vec<SaltCellRender>> {
    if cells.len() != 1 {
        return Some(transform_salt_grid_cells(cells, state));
    }

    let SaltCellRender::Label(text) = &cells[0] else {
        return Some(transform_salt_grid_cells(cells, state));
    };
    let trimmed = text.trim();
    let lower = trimmed.to_ascii_lowercase();

    if lower == "<style>" {
        state.in_style = true;
        return None;
    }
    if lower == "</style>" {
        state.in_style = false;
        state.style_scope = None;
        return None;
    }
    if state.in_style {
        if trimmed.ends_with('{') {
            state.style_scope = Some(trimmed.trim_end_matches('{').trim().to_string());
            return None;
        }
        if trimmed == "}" {
            state.style_scope = None;
            return None;
        }
        if let Some((key, value)) = trimmed.split_once(char::is_whitespace) {
            style.set_scoped(state.style_scope.as_deref(), key.trim(), value.trim());
        }
        return None;
    }

    if apply_salt_style_directive(trimmed, style) {
        return None;
    }

    if matches!(trimmed, "{" | "}") {
        if trimmed == "}" {
            state.in_tree = false;
            state.in_text_area = false;
            state.table_header_pending = false;
        }
        return None;
    }

    if lower.starts_with("{#") || lower.starts_with("{!") {
        state.table_header_pending = true;
        state.in_text_area = false;
        return None;
    }

    if let Some(name) = parse_salt_sprite_def(trimmed) {
        return Some(vec![SaltCellRender::SpriteDef(name)]);
    }

    if lower.starts_with("{+") {
        state.in_text_area = true;
        state.in_tree = false;
        return Some(vec![SaltCellRender::TextAreaLine {
            text: String::new(),
            scroll_vertical: false,
            scroll_horizontal: false,
        }]);
    }

    if lower.starts_with("{^") {
        state.in_text_area = false;
        let label = trimmed
            .trim_start_matches("{^")
            .trim_matches('}')
            .trim()
            .to_string();
        return Some(vec![SaltCellRender::GroupBox(label)]);
    }

    if state.in_text_area {
        let text = if trimmed == "." { "" } else { trimmed };
        return Some(vec![SaltCellRender::TextAreaLine {
            text: text.to_string(),
            scroll_vertical: false,
            scroll_horizontal: false,
        }]);
    }

    if lower.starts_with("{t") || lower == "tree" || lower.starts_with("tree ") {
        state.in_tree = true;
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

    if let Some(scroll) = parse_salt_scroll_container(trimmed) {
        state.in_text_area = true;
        return Some(vec![SaltCellRender::TextAreaLine {
            text: String::new(),
            scroll_vertical: scroll.0,
            scroll_horizontal: scroll.1,
        }]);
    }

    if let Some((vertical, percent)) = parse_salt_scrollbar(trimmed) {
        return Some(vec![SaltCellRender::ScrollBar { vertical, percent }]);
    }

    if state.in_tree {
        state.in_tree = false;
    }

    Some(transform_salt_grid_cells(cells, state))
}

fn transform_salt_grid_cells(
    cells: Vec<SaltCellRender>,
    state: &mut SaltTransformState,
) -> Vec<SaltCellRender> {
    let header_row = state.table_header_pending;
    state.table_header_pending = false;
    cells
        .into_iter()
        .map(|cell| transform_salt_table_cell(cell, header_row))
        .collect()
}

fn transform_salt_table_cell(cell: SaltCellRender, header_row: bool) -> SaltCellRender {
    match cell {
        SaltCellRender::Label(text) => {
            let trimmed = text.trim();
            if trimmed == "." {
                SaltCellRender::TableEmpty
            } else if trimmed == "*" {
                SaltCellRender::TableSpan
            } else if let Some(name) = parse_salt_sprite_ref(trimmed) {
                SaltCellRender::SpriteRef(name)
            } else if header_row {
                SaltCellRender::Header(trimmed.trim_start_matches('=').trim().to_string())
            } else {
                promote_salt_header_cell(SaltCellRender::Label(text))
            }
        }
        other => other,
    }
}

fn promote_salt_header_cell(cell: SaltCellRender) -> SaltCellRender {
    match cell {
        SaltCellRender::Label(text) => {
            let trimmed = text.trim();
            if let Some(rest) = trimmed.strip_prefix('=') {
                SaltCellRender::Header(rest.trim().to_string())
            } else if let Some(name) = parse_salt_sprite_ref(trimmed) {
                SaltCellRender::SpriteRef(name)
            } else {
                SaltCellRender::Label(text)
            }
        }
        other => other,
    }
}

fn parse_salt_sprite_def(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let inner = trimmed.strip_prefix("<<")?;
    let name = inner
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_end_matches(">>")
        .trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn parse_salt_sprite_ref(text: &str) -> Option<String> {
    let trimmed = text.trim();
    let inner = trimmed.strip_prefix("<<")?.strip_suffix(">>")?.trim();
    if inner.is_empty() || inner.contains(char::is_whitespace) {
        None
    } else {
        Some(inner.to_string())
    }
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

fn parse_salt_scroll_container(line: &str) -> Option<(bool, bool)> {
    let lower = line.to_ascii_lowercase();
    if !lower.starts_with("{s") || lower.starts_with("{*") {
        return None;
    }
    let marker = lower.trim_matches('{').trim_matches('}').trim();
    if marker.starts_with("si") {
        Some((true, false))
    } else if marker.starts_with("s-") {
        Some((false, true))
    } else {
        Some((true, true))
    }
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

fn salt_text(out: &mut String, x: i32, y: i32, attrs: &str, text: &str, color: &str) {
    let icon_names = extract_salt_icon_names(text);
    let mut extra_attrs = attrs.to_string();
    if salt_text_has_creole(text) {
        extra_attrs.push_str(" data-salt-creole=\"true\"");
    }
    if !icon_names.is_empty() {
        extra_attrs.push_str(&format!(
            " data-salt-icons=\"{}\"",
            escape_text(&icon_names.join(","))
        ));
    }
    out.push_str(&creole_text(x, y, &extra_attrs, text, color));
}

fn salt_text_has_creole(text: &str) -> bool {
    text.contains("**")
        || text.contains("//")
        || text.contains("\"\"")
        || text.contains("__")
        || text.contains("--")
        || text.contains("[[")
        || text.contains("<color")
        || text.contains("<size")
        || text.contains("<b>")
        || text.contains("<B>")
        || text.contains("<i>")
        || text.contains("<I>")
        || text.contains("<u>")
        || text.contains("<U>")
        || text.contains("<&")
}

fn extract_salt_icon_names(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("<&") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find('>') else {
            break;
        };
        let name = rest[..end].trim();
        if !name.is_empty() {
            names.push(name.to_string());
        }
        rest = &rest[end + 1..];
    }
    names
}

/// Render a single salt cell into SVG, appending to `out`.
fn render_salt_cell_svg(
    out: &mut String,
    cell: &SaltCellRender,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    style: &SaltRenderStyle,
) {
    let pad = 8;
    let text_y = y + h / 2 + 4;
    match cell {
        SaltCellRender::Label(text) => {
            salt_text(
                out,
                x + pad,
                text_y,
                &format!(
                    "font-family=\"{}\" font-size=\"12\" fill=\"{}\"",
                    style.font_family, style.text_color
                ),
                text,
                &style.text_color,
            );
        }
        SaltCellRender::Header(text) => {
            out.push_str(&format!(
                "<rect data-salt-widget=\"header\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                x + 1,
                y + 1,
                w - 2,
                h - 2,
                style.header_fill,
                style.grid_color
            ));
            salt_text(
                out,
                x + pad,
                text_y,
                &format!(
                    "font-family=\"{}\" font-size=\"12\" font-weight=\"700\" fill=\"{}\"",
                    style.font_family, style.header_text_color
                ),
                text,
                &style.header_text_color,
            );
        }
        SaltCellRender::TableEmpty => {
            out.push_str(&format!(
                "<rect data-salt-widget=\"table-empty\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
                x + 1,
                y + 1,
                w - 2,
                h - 2,
                style.panel_fill,
                style.grid_color
            ));
        }
        SaltCellRender::TableSpan => {
            out.push_str(&format!(
                "<rect data-salt-widget=\"table-span\" data-salt-colspan=\"left\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"0.5\" stroke-dasharray=\"4 3\"/>",
                x + 1,
                y + 1,
                w - 2,
                h - 2,
                style.panel_fill,
                style.grid_color
            ));
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"11\" fill=\"{}\">span</text>",
                x + w / 2,
                text_y,
                style.font_family,
                style.muted_text_color
            ));
        }
        SaltCellRender::SpriteDef(name) => {
            out.push_str(&format!(
                "<g data-salt-widget=\"sprite\" data-salt-sprite=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-dasharray=\"3 2\"/><text x=\"{}\" y=\"{}\" font-family=\"{}\" font-size=\"11\" fill=\"{}\">sprite:{}</text></g>",
                escape_text(name),
                x + 4,
                y + 4,
                w - 8,
                h - 8,
                style.accent_fill,
                style.border_color,
                x + pad,
                text_y,
                style.font_family,
                style.muted_text_color,
                escape_text(name)
            ));
        }
        SaltCellRender::SpriteRef(name) => {
            out.push_str(&format!(
                "<g data-salt-widget=\"sprite-ref\" data-salt-sprite-ref=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"18\" height=\"18\" fill=\"{}\" stroke=\"{}\"/><text x=\"{}\" y=\"{}\" font-family=\"{}\" font-size=\"11\" fill=\"{}\">{}</text></g>",
                escape_text(name),
                x + pad,
                y + 5,
                style.accent_fill,
                style.border_color,
                x + pad + 24,
                text_y,
                style.font_family,
                style.text_color,
                escape_text(name)
            ));
        }
        SaltCellRender::Input(placeholder) => {
            // Bordered rectangle with gray placeholder text
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\" rx=\"2\" ry=\"2\"/>",
                x + pad,
                y + 4,
                w - pad * 2,
                h - 8,
                style.input_fill,
                style.border_color
            ));
            salt_text(
                out,
                x + pad + 4,
                text_y,
                &format!(
                    "font-family=\"{}\" font-size=\"11\" fill=\"{}\"",
                    style.font_family, style.input_text_color
                ),
                placeholder,
                &style.input_text_color,
            );
        }
        SaltCellRender::Button(label) => {
            // Rounded rectangle with bold text
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\" rx=\"4\" ry=\"4\"/>",
                x + pad,
                y + 4,
                w - pad * 2,
                h - 8,
                style.button_fill,
                style.border_color
            ));
            salt_text(
                out,
                x + w / 2,
                text_y,
                &format!(
                    "text-anchor=\"middle\" font-family=\"{}\" font-size=\"12\" font-weight=\"bold\" fill=\"{}\"",
                    style.font_family, style.button_text_color
                ),
                label,
                &style.button_text_color,
            );
        }
        SaltCellRender::Combo(label) => {
            // Rectangle with down-arrow indicator
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\" rx=\"2\" ry=\"2\"/>",
                x + pad,
                y + 4,
                w - pad * 2,
                h - 8,
                style.input_fill,
                style.border_color
            ));
            salt_text(
                out,
                x + pad + 4,
                text_y,
                &format!(
                    "font-family=\"{}\" font-size=\"11\" fill=\"{}\"",
                    style.font_family, style.input_text_color
                ),
                label,
                &style.input_text_color,
            );
            // Down arrow triangle
            let ax = x + w - pad - 10;
            let ay = y + h / 2;
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\"/>",
                ax,
                ay - 3,
                ax + 8,
                ay - 3,
                ax + 4,
                ay + 3,
                style.border_color
            ));
        }
        SaltCellRender::CheckboxChecked(label) => {
            let bx = x + pad;
            let by = y + h / 2 - 6;
            // Box
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"12\" height=\"12\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                bx, by, style.checkbox_fill, style.border_color
            ));
            // Checkmark (×)
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                bx + 2, by + 2, bx + 10, by + 10, style.text_color
            ));
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                bx + 10, by + 2, bx + 2, by + 10, style.text_color
            ));
            if !label.is_empty() {
                salt_text(
                    out,
                    bx + 16,
                    text_y,
                    &format!(
                        "font-family=\"{}\" font-size=\"12\" fill=\"{}\"",
                        style.font_family, style.text_color
                    ),
                    label,
                    &style.text_color,
                );
            }
        }
        SaltCellRender::CheckboxUnchecked(label) => {
            let bx = x + pad;
            let by = y + h / 2 - 6;
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"12\" height=\"12\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                bx, by, style.checkbox_fill, style.border_color
            ));
            if !label.is_empty() {
                salt_text(
                    out,
                    bx + 16,
                    text_y,
                    &format!(
                        "font-family=\"{}\" font-size=\"12\" fill=\"{}\"",
                        style.font_family, style.text_color
                    ),
                    label,
                    &style.text_color,
                );
            }
        }
        SaltCellRender::RadioOn(label) => {
            let cx = x + pad + 6;
            let cy = y + h / 2;
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                cx, cy, style.radio_fill, style.border_color
            ));
            // Filled dot
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"3\" fill=\"{}\"/>",
                cx, cy, style.text_color
            ));
            if !label.is_empty() {
                salt_text(
                    out,
                    cx + 10,
                    text_y,
                    &format!(
                        "font-family=\"{}\" font-size=\"12\" fill=\"{}\"",
                        style.font_family, style.text_color
                    ),
                    label,
                    &style.text_color,
                );
            }
        }
        SaltCellRender::RadioOff(label) => {
            let cx = x + pad + 6;
            let cy = y + h / 2;
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                cx, cy, style.radio_fill, style.border_color
            ));
            if !label.is_empty() {
                salt_text(
                    out,
                    cx + 10,
                    text_y,
                    &format!(
                        "font-family=\"{}\" font-size=\"12\" fill=\"{}\"",
                        style.font_family, style.text_color
                    ),
                    label,
                    &style.text_color,
                );
            }
        }
        SaltCellRender::TreeItem { depth, label } => {
            let indent = (*depth as i32) * 16;
            let branch_x = x + pad + indent;
            let cy = y + h / 2;
            out.push_str(&format!(
                "<g data-salt-widget=\"tree\" data-salt-tree-depth=\"{}\"><line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/><line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/><circle cx=\"{}\" cy=\"{}\" r=\"3\" fill=\"{}\"/>",
                depth,
                branch_x,
                y + 4,
                branch_x,
                y + h - 4,
                style.grid_color,
                branch_x,
                cy,
                branch_x + 10,
                cy,
                style.grid_color,
                branch_x + 10,
                cy,
                style.border_color
            ));
            salt_text(
                out,
                branch_x + 18,
                text_y,
                &format!(
                    "font-family=\"{}\" font-size=\"12\" fill=\"{}\"",
                    style.font_family, style.text_color
                ),
                label,
                &style.text_color,
            );
            out.push_str("</g>");
        }
        SaltCellRender::TextAreaLine {
            text,
            scroll_vertical,
            scroll_horizontal,
        } => {
            out.push_str(&format!(
                "<rect data-salt-widget=\"textarea\" data-salt-scroll-vertical=\"{}\" data-salt-scroll-horizontal=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\" rx=\"3\" ry=\"3\"/>",
                scroll_vertical,
                scroll_horizontal,
                x + 4,
                y + 3,
                w - 8,
                h - 6,
                style.input_fill,
                style.border_color
            ));
            if !text.is_empty() {
                salt_text(
                    out,
                    x + pad,
                    text_y,
                    &format!(
                        "font-family=\"{}\" font-size=\"12\" fill=\"{}\"",
                        style.font_family, style.input_text_color
                    ),
                    text,
                    &style.input_text_color,
                );
            }
            if *scroll_vertical {
                let track_x = x + w - pad - 10;
                out.push_str(&format!(
                    "<rect data-salt-widget=\"scrollbar\" x=\"{}\" y=\"{}\" width=\"8\" height=\"{}\" rx=\"4\" ry=\"4\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                    track_x,
                    y + 6,
                    h - 12,
                    style.scroll_fill,
                    style.border_color
                ));
            }
            if *scroll_horizontal {
                out.push_str(&format!(
                    "<rect data-salt-widget=\"scrollbar\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"8\" rx=\"4\" ry=\"4\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                    x + pad,
                    y + h - 10,
                    w - pad * 2,
                    style.scroll_fill,
                    style.border_color
                ));
            }
        }
        SaltCellRender::GroupBox(label) => {
            out.push_str(&format!(
                "<rect data-salt-widget=\"groupbox\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1\" rx=\"4\" ry=\"4\"/>",
                x + 2,
                y + 6,
                w - 4,
                h - 8,
                style.border_color
            ));
            if !label.is_empty() {
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"12\" fill=\"{}\"/>",
                    x + pad,
                    y + 1,
                    estimate_text_width(label) + 8,
                    style.panel_fill
                ));
                salt_text(
                    out,
                    x + pad + 4,
                    y + 11,
                    &format!(
                        "font-family=\"{}\" font-size=\"11\" fill=\"{}\"",
                        style.font_family, style.text_color
                    ),
                    label,
                    &style.text_color,
                );
            }
        }
        SaltCellRender::MenuBar(items) => {
            out.push_str(&format!(
                "<rect data-salt-widget=\"menu\" data-salt-open=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                items.len() > 4,
                x + 1,
                y + 2,
                w - 2,
                h - 4,
                style.menu_fill,
                style.border_color
            ));
            let mut item_x = x + pad;
            for item in items {
                salt_text(
                    out,
                    item_x,
                    text_y,
                    &format!(
                        "font-family=\"{}\" font-size=\"12\" fill=\"{}\"",
                        style.font_family, style.text_color
                    ),
                    item,
                    &style.text_color,
                );
                item_x += estimate_text_width(item) + 24;
            }
        }
        SaltCellRender::TabBar { tabs, active } => {
            let mut tab_x = x + pad;
            for (idx, tab) in tabs.iter().enumerate() {
                let tab_w = estimate_text_width(tab) + 24;
                let active_tab = idx == *active;
                let fill = if active_tab {
                    style.panel_fill.as_str()
                } else {
                    style.tab_fill.as_str()
                };
                let stroke = style.border_color.as_str();
                out.push_str(&format!(
                    "<rect data-salt-widget=\"tab\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"5\" ry=\"5\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                    tab_x,
                    y + 4,
                    tab_w,
                    h - 5,
                    fill,
                    stroke
                ));
                salt_text(
                    out,
                    tab_x + 12,
                    text_y,
                    &format!(
                        "font-family=\"{}\" font-size=\"12\" fill=\"{}\"",
                        style.font_family, style.text_color
                    ),
                    tab,
                    &style.text_color,
                );
                tab_x += tab_w - 1;
            }
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                x + pad,
                y + h - 1,
                x + w - pad,
                y + h - 1,
                style.border_color
            ));
        }
        SaltCellRender::ScrollBar { vertical, percent } => {
            let track_x = if *vertical { x + w - pad - 12 } else { x + pad };
            let track_y = if *vertical { y + 5 } else { y + h - 13 };
            let track_w = if *vertical { 12 } else { w - pad * 2 };
            let track_h = if *vertical { h - 10 } else { 12 };
            out.push_str(&format!(
                "<rect data-salt-widget=\"scrollbar\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" ry=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                track_x, track_y, track_w, track_h, style.scroll_fill, style.border_color
            ));
            if *vertical {
                let thumb_h = ((track_h as f32) * (*percent as f32 / 100.0)).round() as i32;
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"5\" ry=\"5\" fill=\"{}\"/>",
                    track_x + 2,
                    track_y + 2,
                    track_w - 4,
                    thumb_h.max(8).min(track_h - 4),
                    style.border_color
                ));
            } else {
                let thumb_w = ((track_w as f32) * (*percent as f32 / 100.0)).round() as i32;
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"5\" ry=\"5\" fill=\"{}\"/>",
                    track_x + 2,
                    track_y + 2,
                    thumb_w.max(12).min(track_w - 4),
                    track_h - 4,
                    style.border_color
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
            "<text x=\"{cx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"{}\" font-weight=\"600\" fill=\"{}\">{name}</text>",
            escape_text(font_family),
            title_font_size,
            escape_text(&class_style.font_color),
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
        FamilyNodeKind::Class => "class ",
        FamilyNodeKind::Object => "",
        FamilyNodeKind::UseCase => "",
        _ => "",
    };
    let display_name = namespace_separator
        .filter(|sep| !sep.is_empty())
        .map(|sep| node.name.replace("::", sep))
        .unwrap_or_else(|| node.name.clone());
    let header_text = if let Some(alias) = &node.alias {
        format!("{kind_prefix}{} (as {})", display_name, alias)
    } else {
        format!("{kind_prefix}{}", display_name)
    };
    // Underline for objects (PlantUML convention)
    let text_decoration = if matches!(node.kind, FamilyNodeKind::Object) {
        " text-decoration=\"underline\""
    } else {
        ""
    };
    out.push_str(&format!(
        "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"{}\" font-weight=\"600\" fill=\"{}\"{td}>{txt}</text>",
        escape_text(font_family),
        title_font_size,
        escape_text(&class_style.font_color),
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
            "<text class=\"uml-member\"{visibility_attr}{modifier_attr} x=\"{tx}\" y=\"{my}\" font-family=\"{}\" font-size=\"{}\" fill=\"{vc}\"{sa}>{m}</text>",
            escape_text(font_family),
            member_font_size,
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
    if matches!(compact.as_str(), "<<include>>" | "include" | "includes")
        || compact.contains("include")
    {
        Some("<<include>>")
    } else if matches!(compact.as_str(), "<<extend>>" | "extend" | "extends")
        || compact.contains("extend")
    {
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
    let has_calendar_notes = !document.closed_weekdays.is_empty()
        || !document.closed_ranges.is_empty()
        || !document.open_ranges.is_empty();
    let calendar_h = if !has_calendar_notes { 0 } else { 18 };
    let scale_h = if document.scale.is_some() { 18 } else { 0 };

    let row_count =
        (document.tasks.len() + document.milestones.len() + document.separators.len()) as i32;
    let chart_top = 40 + title_h + calendar_h + scale_h + header_h;
    let chart_h = (row_count.max(1)) * (bar_height + row_gap) + 20;
    let total_h = chart_top + chart_h + 40;

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">",
        w = width,
        h = total_h
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    if let Some(scale) = &document.scale {
        out.push_str(&format!(
            "<metadata data-gantt-scale=\"{}\"/>",
            escape_text(scale)
        ));
    }
    let resource_count = document
        .tasks
        .iter()
        .flat_map(|task| {
            task.resource_allocations
                .iter()
                .map(|allocation| allocation.name.as_str())
        })
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    out.push_str(&format!(
        "<metadata data-gantt-resource-count=\"{resource_count}\" data-gantt-separator-count=\"{}\"/>",
        document.separators.len()
    ));

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
    if has_calendar_notes {
        let mut labels = Vec::new();
        if !document.closed_weekdays.is_empty() {
            labels.push(
                document
                    .closed_weekdays
                    .iter()
                    .map(|day| title_case_ascii(day))
                    .collect::<Vec<_>>()
                    .join(", "),
            );
        }
        labels.extend(document.closed_ranges.iter().map(|range| {
            if range.start_date == range.end_date {
                range.start_date.clone()
            } else {
                format!("{} to {}", range.start_date, range.end_date)
            }
        }));
        let mut label = if labels.is_empty() {
            String::new()
        } else {
            format!("closed {}", labels.join("; "))
        };
        if !document.open_ranges.is_empty() {
            let open_label = document
                .open_ranges
                .iter()
                .map(|range| {
                    if range.start_date == range.end_date {
                        range.start_date.clone()
                    } else {
                        format!("{} to {}", range.start_date, range.end_date)
                    }
                })
                .collect::<Vec<_>>()
                .join("; ");
            if label.is_empty() {
                label = format!("open {open_label}");
            } else {
                label.push_str(&format!("; open {open_label}"));
            }
        }
        out.push_str(&format!(
            "<text class=\"gantt-calendar\" x=\"{x}\" y=\"{y}\" font-family=\"monospace\" font-size=\"11\" fill=\"#92400e\">Calendar: {label}</text>",
            x = margin_x,
            y = 42 + title_h,
            label = escape_text(&label)
        ));
    }
    if let Some(scale) = &document.scale {
        out.push_str(&format!(
            "<text class=\"gantt-scale\" x=\"{x}\" y=\"{y}\" font-family=\"monospace\" font-size=\"11\" fill=\"#334155\">Scale: {scale}</text>",
            x = margin_x,
            y = 42 + title_h + calendar_h,
            scale = escape_text(scale)
        ));
    }

    let has_resource_lanes = document.tasks.iter().any(|t| !t.resources.is_empty());
    let mut ordered_tasks: Vec<&TimelineTask> = document.tasks.iter().collect();
    if has_resource_lanes {
        ordered_tasks.sort_by(|a, b| {
            resource_lane_label(a)
                .cmp(&resource_lane_label(b))
                .then_with(|| a.name.cmp(&b.name))
        });
    }

    // Build row index for tasks + milestones
    let mut row_index: std::collections::BTreeMap<String, i32> = std::collections::BTreeMap::new();
    let mut row_counter: i32 = 0;
    for task in &ordered_tasks {
        row_index.insert(task.name.clone(), row_counter);
        row_counter += 1;
    }
    let task_count = document.tasks.len() as i32;
    for milestone in &document.milestones {
        row_index.insert(milestone.name.clone(), row_counter);
        row_counter += 1;
    }
    for separator in &document.separators {
        row_index.insert(format!("__separator::{}", separator.label), row_counter);
        row_counter += 1;
    }

    let task_bounds: std::collections::BTreeMap<&str, (u32, u32)> = document
        .tasks
        .iter()
        .map(|t| {
            (
                t.name.as_str(),
                (
                    t.start_day,
                    t.start_day.saturating_add(t.duration_days.max(1)),
                ),
            )
        })
        .collect();
    let preliminary_min_day = document
        .project_start_day
        .into_iter()
        .chain(document.tasks.iter().map(|t| t.start_day))
        .min()
        .unwrap_or(0);
    let milestone_anchor = document.project_start_day.unwrap_or(preliminary_min_day);
    let mut milestone_day: std::collections::BTreeMap<&str, u32> =
        std::collections::BTreeMap::new();
    for ms in &document.milestones {
        if let Some(day) = ms
            .happens_on
            .as_deref()
            .and_then(|target| resolve_gantt_milestone_day(target, milestone_anchor, &task_bounds))
        {
            milestone_day.insert(ms.name.as_str(), day);
            continue;
        }
        for c in &document.constraints {
            if c.subject != ms.name {
                continue;
            }
            if let Some(day) =
                resolve_gantt_milestone_day(&c.target, milestone_anchor, &task_bounds)
            {
                milestone_day.insert(ms.name.as_str(), day);
                break;
            }
        }
    }

    let min_day = document
        .project_start_day
        .into_iter()
        .chain(document.tasks.iter().map(|t| t.start_day))
        .chain(milestone_day.values().copied())
        .min()
        .unwrap_or(0);
    let project_end_day = document
        .constraints
        .iter()
        .find(|c| {
            c.subject.eq_ignore_ascii_case("Project")
                && c.kind.eq_ignore_ascii_case("ends")
                && parse_iso_date_day_number(&c.target).is_some()
        })
        .and_then(|c| parse_iso_date_day_number(&c.target));
    let max_day_exclusive = document
        .project_start_day
        .map(|d| d.saturating_add(1))
        .into_iter()
        .chain(project_end_day.map(|d| d.saturating_add(1)))
        .chain(
            document
                .tasks
                .iter()
                .map(|t| t.start_day.saturating_add(t.duration_days.max(1))),
        )
        .chain(milestone_day.values().map(|d| d.saturating_add(1)))
        .chain(document.separators.iter().filter_map(|separator| {
            separator
                .target
                .as_deref()
                .and_then(|target| {
                    resolve_gantt_milestone_day(target, milestone_anchor, &task_bounds)
                })
                .map(|day| day.saturating_add(1))
        }))
        .max()
        .unwrap_or(min_day.saturating_add(1));
    let total_days = max_day_exclusive.saturating_sub(min_day).max(1);
    let date_axis = document.project_start_day.is_some() || min_day > 366;
    let tick_offsets = gantt_tick_offsets(total_days, document.scale.as_deref());

    // Axis header bar
    out.push_str(&format!(
        "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"#f1f5f9\" stroke=\"#cbd5e1\" stroke-width=\"1\"/>",
        x = chart_left,
        y = chart_top - header_h,
        w = chart_w,
        h = header_h
    ));
    for day_offset in tick_offsets {
        let x = chart_left + ((chart_w as u32 * day_offset) / total_days) as i32;
        out.push_str(&format!(
            "<line x1=\"{x}\" y1=\"{y1}\" x2=\"{x}\" y2=\"{y2}\" stroke=\"#e2e8f0\" stroke-width=\"1\"/>",
            y1 = chart_top - header_h,
            y2 = chart_top + chart_h
        ));
        out.push_str(&format!(
            "<text class=\"gantt-scale-tick\" data-gantt-tick-day=\"{day}\" x=\"{tx}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"11\" fill=\"#475569\">{label}</text>",
            day = escape_text(&format_gantt_axis_label(
                min_day.saturating_add(day_offset),
                min_day,
                true
            )),
            tx = x + 6,
            ty = chart_top - 10,
            label = escape_text(&format_gantt_scale_axis_label(
                min_day.saturating_add(day_offset),
                min_day,
                date_axis,
                document.scale.as_deref()
            ))
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
    for range in &document.closed_ranges {
        if range.end_day < min_day || range.start_day > max_day_exclusive {
            continue;
        }
        let start = range.start_day.max(min_day);
        let end = range.end_day.saturating_add(1).min(max_day_exclusive);
        let x = day_to_x(start);
        let w = (day_to_x(end) - x).max(2);
        out.push_str(&format!(
            "<rect class=\"gantt-closed-range\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"#fef3c7\" opacity=\"0.7\"/>",
            y = chart_top,
            h = chart_h
        ));
    }
    for range in &document.open_ranges {
        if range.end_day < min_day || range.start_day > max_day_exclusive {
            continue;
        }
        let start = range.start_day.max(min_day);
        let end = range.end_day.saturating_add(1).min(max_day_exclusive);
        let x = day_to_x(start);
        let w = (day_to_x(end) - x).max(2);
        out.push_str(&format!(
            "<rect class=\"gantt-open-range\" data-gantt-open=\"{}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"#dcfce7\" opacity=\"0.62\"/>",
            escape_text(&format!(
                "{} to {}",
                format_gantt_axis_label(start, min_day, true),
                format_gantt_axis_label(end.saturating_sub(1), min_day, true)
            )),
            y = chart_top,
            h = chart_h
        ));
    }
    if !document.closed_weekdays.is_empty() {
        let mut day = min_day;
        while day < max_day_exclusive {
            if is_gantt_closed_weekday_number(day, &document.closed_weekdays) {
                let x = day_to_x(day);
                let w = (day_to_x(day.saturating_add(1)) - x).max(2);
                out.push_str(&format!(
                    "<rect class=\"gantt-closed-weekday\" data-gantt-day=\"{}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"#f8fafc\" opacity=\"0.82\"/>",
                    escape_text(&format_gantt_axis_label(day, min_day, date_axis)),
                    y = chart_top,
                    h = chart_h
                ));
            }
            day = day.saturating_add(1);
        }
    }
    if let Some(day) = project_end_day {
        if (min_day..=max_day_exclusive).contains(&day) {
            let x = day_to_x(day);
            out.push_str(&format!(
                "<line class=\"gantt-project-end\" x1=\"{x}\" y1=\"{y1}\" x2=\"{x}\" y2=\"{y2}\" stroke=\"#dc2626\" stroke-width=\"1.5\" stroke-dasharray=\"5 3\"/>",
                y1 = chart_top - header_h,
                y2 = chart_top + chart_h
            ));
            out.push_str(&format!(
                "<text class=\"gantt-project-end-label\" x=\"{x}\" y=\"{y}\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"10\" fill=\"#991b1b\">Project ends {label}</text>",
                x = x - 4,
                y = chart_top - header_h - 4,
                label = escape_text(&format_gantt_axis_label(day, min_day, true))
            ));
        }
    }
    if has_resource_lanes {
        let mut lane_start = 0usize;
        while lane_start < ordered_tasks.len() {
            let lane = resource_lane_label(ordered_tasks[lane_start]);
            let mut lane_end = lane_start + 1;
            while lane_end < ordered_tasks.len()
                && resource_lane_label(ordered_tasks[lane_end]) == lane
            {
                lane_end += 1;
            }
            let y = chart_top + lane_start as i32 * (bar_height + row_gap);
            let h = (lane_end - lane_start) as i32 * (bar_height + row_gap);
            out.push_str(&format!(
                "<rect class=\"resource-lane\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"#eff6ff\" stroke=\"#bfdbfe\" stroke-width=\"1\" opacity=\"0.72\"/>",
                x = chart_left,
                w = chart_w
            ));
            out.push_str(&format!(
                "<text x=\"{x}\" y=\"{y}\" font-family=\"monospace\" font-size=\"11\" fill=\"#1d4ed8\">{label}</text>",
                x = chart_left + 6,
                y = y + 14,
                label = escape_text(&lane)
            ));
            lane_start = lane_end;
        }
    }

    // Render tasks as horizontal bars
    for (i, task) in ordered_tasks.iter().enumerate() {
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
        if let (Some(base_start), Some(base_duration)) =
            (task.baseline_start_day, task.baseline_duration_days)
        {
            let base_offset = base_start.saturating_sub(min_day);
            let base_x = chart_left + ((chart_w as u32 * base_offset) / total_days) as i32;
            let base_w = (((chart_w as u32) * base_duration.max(1)) / total_days).max(8) as i32;
            out.push_str(&format!(
                "<rect class=\"gantt-baseline\" data-gantt-baseline-start=\"{}\" data-gantt-baseline-duration=\"{}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"4\" rx=\"2\" ry=\"2\" fill=\"#64748b\" opacity=\"0.88\"/>",
                escape_text(&format_gantt_axis_label(base_start, min_day, true)),
                base_duration,
                x = base_x,
                y = y + bar_height + 3,
                w = base_w
            ));
        }
        let resource_load = format_resource_load_metadata(task);
        let critical_class = if task.is_critical {
            " gantt-critical"
        } else {
            ""
        };
        let fill = if task.is_critical {
            "#ef4444"
        } else {
            "#3b82f6"
        };
        let stroke = if task.is_critical {
            "#991b1b"
        } else {
            "#1e40af"
        };
        out.push_str(&format!(
            "<rect class=\"gantt-task{critical_class}\" data-gantt-start=\"{}\" data-gantt-workload=\"{}\" data-gantt-duration=\"{}\" data-gantt-resources=\"{}\" data-gantt-load=\"{}\" x=\"{bx}\" y=\"{y}\" width=\"{bw}\" height=\"{bh}\" rx=\"3\" ry=\"3\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
            escape_text(&format_gantt_axis_label(task.start_day, min_day, date_axis)),
            task.workload_days,
            task.duration_days,
            escape_text(&task.resources.join(", ")),
            escape_text(&resource_load),
            bh = bar_height
        ));
        if !task.resources.is_empty() {
            let resource_label = task.resources.join(", ");
            let pill_w = ((resource_label.len() as i32) * 7 + 14).min((bw - 6).max(0));
            if pill_w > 26 {
                out.push_str(&format!(
                    "<rect class=\"gantt-resource-pill\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"14\" rx=\"7\" ry=\"7\" fill=\"#dbeafe\" stroke=\"#93c5fd\" stroke-width=\"1\"/>",
                    x = bx + 4,
                    y = y + 3,
                    w = pill_w
                ));
                out.push_str(&format!(
                    "<text class=\"gantt-resource\" data-gantt-load=\"{load}\" x=\"{x}\" y=\"{y}\" font-family=\"monospace\" font-size=\"9\" fill=\"#1e40af\">{txt}</text>",
                    load = escape_text(&resource_load),
                    x = bx + 10,
                    y = y + 14,
                    txt = escape_text(&resource_label)
                ));
            }
            out.push_str(&format!(
                "<text x=\"{x}\" y=\"{y}\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"10\" fill=\"#1e40af\">{txt}</text>",
                x = chart_right - 6,
                y = y + bar_height - 6,
                txt = escape_text(&resource_label)
            ));
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
            "<polygon class=\"gantt-milestone{}\" points=\"{x1},{y1} {x2},{y2} {x3},{y3} {x4},{y4}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            if milestone.is_critical { " gantt-critical" } else { "" },
            if milestone.is_critical { "#fb7185" } else { "#facc15" },
            if milestone.is_critical { "#9f1239" } else { "#854d0e" },
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

    for (i, separator) in document.separators.iter().enumerate() {
        let row = task_count + document.milestones.len() as i32 + i as i32;
        let y = chart_top + row * (bar_height + row_gap) + row_gap / 2 + bar_height / 2;
        let x = separator
            .target
            .as_deref()
            .and_then(|target| resolve_gantt_milestone_day(target, milestone_anchor, &task_bounds))
            .map(day_to_x)
            .unwrap_or(chart_left);
        out.push_str(&format!(
            "<line class=\"gantt-separator\" data-gantt-separator=\"{}\" x1=\"{x}\" y1=\"{}\" x2=\"{x}\" y2=\"{}\" stroke=\"#7c3aed\" stroke-width=\"1.4\" stroke-dasharray=\"6 4\"/>",
            escape_text(&separator.label),
            chart_top - header_h,
            chart_top + chart_h
        ));
        out.push_str(&format!(
            "<text class=\"gantt-separator-label\" x=\"{}\" y=\"{y}\" font-family=\"monospace\" font-size=\"11\" fill=\"#5b21b6\">{}</text>",
            (x + 6).min(chart_right - 80),
            escape_text(&separator.label)
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
        let Some((normalized_target, target_endpoint)) =
            parse_gantt_render_reference(&constraint.target)
        else {
            continue;
        };
        let to_row = row_index.get(&normalized_target).copied();
        if let Some(to_row) = to_row {
            let subject_endpoint = match constraint.kind.to_ascii_lowercase().as_str() {
                "ends" => "end",
                _ => "start",
            };
            let subject_y =
                chart_top + from_row * (bar_height + row_gap) + row_gap / 2 + bar_height / 2;
            let target_y =
                chart_top + to_row * (bar_height + row_gap) + row_gap / 2 + bar_height / 2;
            let from_task = document.tasks.iter().find(|t| t.name == constraint.subject);
            let to_task = document.tasks.iter().find(|t| t.name == normalized_target);
            let x2 = timeline_entity_x(
                from_task,
                document
                    .milestones
                    .iter()
                    .find(|milestone| milestone.name == constraint.subject),
                &milestone_day,
                subject_endpoint,
                &bar_geom,
                &day_to_x,
                chart_left,
            );
            let x1 = timeline_entity_x(
                to_task,
                document
                    .milestones
                    .iter()
                    .find(|milestone| milestone.name == normalized_target),
                &milestone_day,
                target_endpoint,
                &bar_geom,
                &day_to_x,
                chart_left + chart_w / 2,
            );
            let y1 = target_y;
            let y2 = subject_y;
            out.push_str(&format!(
                "<line class=\"gantt-dependency gantt-dependency-{}\" data-gantt-from=\"{}\" data-gantt-to=\"{}\" x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\" stroke=\"#64748b\" stroke-width=\"1.25\" stroke-dasharray=\"4 3\" marker-end=\"url(#gantt-arrow)\"/>",
                escape_text(&constraint.kind),
                escape_text(&normalized_target),
                escape_text(&constraint.subject)
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
        if row_index.contains_key(&constraint.target)
            || extract_bracketed_name(&constraint.target)
                .as_deref()
                .is_some_and(|target| row_index.contains_key(target))
        {
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

fn resource_lane_label(task: &TimelineTask) -> String {
    if task.resources.is_empty() {
        "Unassigned".to_string()
    } else {
        task.resources.join(", ")
    }
}

fn format_resource_load_metadata(task: &TimelineTask) -> String {
    if task.resource_allocations.is_empty() {
        return String::new();
    }
    task.resource_allocations
        .iter()
        .map(|allocation| match allocation.load_percent {
            Some(load) => format!("{}:{load}%", allocation.name),
            None => allocation.name.clone(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn title_case_ascii(raw: &str) -> String {
    let mut chars = raw.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut out = String::new();
    out.push(first.to_ascii_uppercase());
    out.push_str(chars.as_str());
    out
}

fn parse_relative_day(raw: &str) -> Option<u32> {
    let t = raw.trim();
    let rest = t.strip_prefix("D+").or_else(|| t.strip_prefix("d+"))?;
    rest.trim().parse::<u32>().ok()
}

fn resolve_gantt_milestone_day(
    target: &str,
    anchor_day: u32,
    task_bounds: &std::collections::BTreeMap<&str, (u32, u32)>,
) -> Option<u32> {
    if let Some((task_name, endpoint)) = parse_gantt_render_reference(target) {
        if let Some((start, end)) = task_bounds.get(task_name.as_str()) {
            return Some(if endpoint == "start" { *start } else { *end });
        }
    }
    if let Some(day) = parse_relative_day(target) {
        return Some(anchor_day.saturating_add(day));
    }
    parse_iso_date_day_number(target)
}

fn parse_gantt_render_reference(target: &str) -> Option<(String, &'static str)> {
    let name = extract_bracketed_name(target)?;
    let lower = target.to_ascii_lowercase();
    let endpoint = if lower.contains("'s start") || lower.contains(" start") {
        "start"
    } else {
        "end"
    };
    Some((name, endpoint))
}

fn timeline_entity_x(
    task: Option<&TimelineTask>,
    milestone: Option<&TimelineMilestone>,
    milestone_day: &std::collections::BTreeMap<&str, u32>,
    endpoint: &str,
    bar_geom: &impl Fn(&TimelineTask) -> (i32, i32),
    day_to_x: &impl Fn(u32) -> i32,
    fallback: i32,
) -> i32 {
    if let Some(task) = task {
        let (x, w) = bar_geom(task);
        return if endpoint == "start" { x } else { x + w };
    }
    if let Some(milestone) = milestone {
        if let Some(day) = milestone_day.get(milestone.name.as_str()) {
            return day_to_x(*day);
        }
    }
    fallback
}

fn gantt_tick_offsets(total_days: u32, scale: Option<&str>) -> Vec<u32> {
    let step = match scale {
        Some("weekly") => 7,
        Some("monthly") => 30,
        Some("quarterly") => 90,
        Some("yearly") => 365,
        _ => 1,
    };
    let mut offsets = Vec::new();
    let mut offset = 0u32;
    while offset < total_days {
        offsets.push(offset);
        offset = offset.saturating_add(step);
        if offsets.len() >= 8 && offset < total_days {
            let remaining = total_days.saturating_sub(offset).max(1);
            offset = offset.saturating_add(remaining.div_ceil(8));
        }
    }
    if offsets.last().copied() != Some(total_days) {
        offsets.push(total_days);
    }
    offsets
}

fn format_gantt_axis_label(day: u32, min_day: u32, date_axis: bool) -> String {
    if date_axis {
        day_number_to_iso(day).unwrap_or_else(|| format!("D+{}", day.saturating_sub(min_day)))
    } else {
        format!("D+{}", day.saturating_sub(min_day))
    }
}

fn format_gantt_scale_axis_label(
    day: u32,
    min_day: u32,
    date_axis: bool,
    scale: Option<&str>,
) -> String {
    if !date_axis {
        return format_gantt_axis_label(day, min_day, false);
    }
    let Some(iso) = day_number_to_iso(day) else {
        return format_gantt_axis_label(day, min_day, true);
    };
    match scale {
        Some("weekly") => format!("Wk {iso}"),
        Some("monthly") => iso
            .get(0..7)
            .map(format_month_label)
            .unwrap_or_else(|| iso.clone()),
        Some("quarterly") => format_quarter_label(&iso).unwrap_or_else(|| iso.clone()),
        Some("yearly") => iso.get(0..4).unwrap_or(&iso).to_string(),
        _ => iso,
    }
}

fn format_month_label(year_month: &str) -> String {
    let Some((year, month)) = year_month.split_once('-') else {
        return year_month.to_string();
    };
    let month = match month {
        "01" => "Jan",
        "02" => "Feb",
        "03" => "Mar",
        "04" => "Apr",
        "05" => "May",
        "06" => "Jun",
        "07" => "Jul",
        "08" => "Aug",
        "09" => "Sep",
        "10" => "Oct",
        "11" => "Nov",
        "12" => "Dec",
        _ => return year_month.to_string(),
    };
    format!("{month} {year}")
}

fn format_quarter_label(iso: &str) -> Option<String> {
    let year = iso.get(0..4)?;
    let month = iso.get(5..7)?.parse::<u32>().ok()?;
    let quarter = month.saturating_sub(1) / 3 + 1;
    Some(format!("Q{quarter} {year}"))
}

fn is_gantt_closed_weekday_number(day: u32, closed_weekdays: &[String]) -> bool {
    let weekday = match (day + 3) % 7 {
        0 => "monday",
        1 => "tuesday",
        2 => "wednesday",
        3 => "thursday",
        4 => "friday",
        5 => "saturday",
        _ => "sunday",
    };
    closed_weekdays.iter().any(|closed| closed == weekday)
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
    let y = i64::from(y);
    let m = i64::from(m);
    let d = i64::from(d);
    let y_adj = y - if m <= 2 { 1 } else { 0 };
    let era = if y_adj >= 0 { y_adj } else { y_adj - 399 } / 400;
    let yoe = y_adj - era * 400;
    let mp = m + if m > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    if days < 0 {
        return None;
    }
    u32::try_from(days).ok()
}

fn day_number_to_iso(day: u32) -> Option<String> {
    let z = i64::from(day) + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    y += if m <= 2 { 1 } else { 0 };
    Some(format!("{y:04}-{m:02}-{d:02}"))
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
    let group_frames = collect_render_group_frames(&doc.groups);
    let max_group_depth = group_frames
        .iter()
        .map(|frame| frame.depth)
        .max()
        .unwrap_or(0);
    let title_lines = doc
        .title
        .as_deref()
        .map(|v| v.lines().count() as i32)
        .unwrap_or(0);
    let header_h = 40 + title_lines * 22;
    let width = (margin_x * 2) + (cols * cell_w) + ((cols - 1).max(0) * gap);
    let projection_extra_height = family_projection_extra_height(&doc.json_projections);
    let height = header_h
        + margin_y
        + (rows.max(1) * cell_h)
        + ((rows - 1).max(0) * gap)
        + 60
        + (doc.groups.len() as i32 * 12)
        + ((max_group_depth as i32) * 24)
        + projection_extra_height;

    // Position lookup by name and alias.
    let mut positions: std::collections::BTreeMap<String, (i32, i32, i32, i32)> =
        std::collections::BTreeMap::new();

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        width, height, width, height
    ));
    out.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>",
        escape_text(&comp_style.background_color)
    ));
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

    for group in &group_frames {
        let mut gx_min = i32::MAX;
        let mut gy_min = i32::MAX;
        let mut gx_max = i32::MIN;
        let mut gy_max = i32::MIN;
        let mut found_any = false;
        for member_id in &group.member_ids {
            if let Some((x, y, w, h)) = positions.get(member_id.as_str()) {
                gx_min = gx_min.min(*x);
                gy_min = gy_min.min(*y);
                gx_max = gx_max.max(*x + *w);
                gy_max = gy_max.max(*y + *h);
                found_any = true;
            }
        }
        if !found_any {
            continue;
        }
        let depth_outset = (max_group_depth.saturating_sub(group.depth) as i32) * 18;
        let pad = 14 + depth_outset;
        let label_h = 20 + depth_outset;
        let fx = gx_min - pad;
        let fy = gy_min - pad - label_h;
        let fw = gx_max - gx_min + pad * 2;
        let fh = gy_max - gy_min + pad * 2 + label_h;
        let label = group.display_label();
        out.push_str(&format!(
            "<rect class=\"uml-group-frame\" data-uml-group=\"{}\" x=\"{fx}\" y=\"{fy}\" width=\"{fw}\" height=\"{fh}\" rx=\"8\" ry=\"8\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\" stroke-dasharray=\"6 4\"/>",
            escape_text(&group.scope),
            comp_style.border_color
        ));
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" font-weight=\"600\" fill=\"{}\">{}</text>",
            fx + 8,
            fy + 14,
            comp_style.border_color,
            escape_text(&label)
        ));
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
        let (x1, y1, x2, y2) = if rel.direction.is_some() {
            compute_edge_anchors_for_direction(
                (fx, fy, fw, fh),
                (tx, ty, tw, th),
                rel.direction.as_deref(),
            )
        } else {
            let cx1 = fx + fw / 2;
            let cy1 = fy + fh / 2;
            let cx2 = tx + tw / 2;
            let cy2 = ty + th / 2;
            let (x1, y1) = clip_to_box_edge((cx1, cy1), (cx2, cy2), (fx, fy, fw, fh));
            let (x2, y2) = clip_to_box_edge((cx2, cy2), (cx1, cy1), (tx, ty, tw, th));
            (x1, y1, x2, y2)
        };
        let style = arrow_style(&normalized_arrow);
        let relation_color = rel.line_color.as_deref().unwrap_or(&comp_style.arrow_color);
        let stroke_width = rel.thickness.unwrap_or(2).clamp(1, 8);
        let dash = if style.dashed || rel.dashed {
            " stroke-dasharray=\"5 3\""
        } else {
            ""
        };
        let visibility = if rel.hidden {
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
        let direction_attr = rel
            .direction
            .as_deref()
            .map(|direction| format!(" data-uml-direction=\"{}\"", escape_text(direction)))
            .unwrap_or_default();
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}{}{}{} />",
            x1, y1, x2, y2, relation_color, stroke_width, dash, visibility, direction_attr, markers
        ));
        if rel.left_lollipop {
            render_lollipop_endpoint(&mut out, x1, y1, relation_color);
        }
        if rel.right_lollipop {
            render_lollipop_endpoint(&mut out, x2, y2, relation_color);
        }
        if let Some(stereotype) = &rel.stereotype {
            let mx = (x1 + x2) / 2;
            let my = (y1 + y2) / 2 - if rel.label.is_some() { 20 } else { 6 };
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"#475569\">&lt;&lt;{}&gt;&gt;</text>",
                mx,
                my,
                escape_text(stereotype)
            ));
        }
        if let Some(label) = &rel.label {
            let label = usecase_dependency_label(Some(label)).unwrap_or(label);
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

    if !doc.json_projections.is_empty() {
        let proj_y = header_h
            + margin_y
            + (rows.max(1) * cell_h)
            + ((rows - 1).max(0) * gap)
            + 24
            + (doc.groups.len() as i32 * 12)
            + ((max_group_depth as i32) * 24);
        render_family_projection_boxes(&mut out, &doc.json_projections, margin_x, proj_y, 340);
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
    out.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"#475569\">{}</text>",
        cx,
        kind_tag_y,
        kind_label
    ));
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

fn render_note_card(out: &mut String, x: i32, y: i32, w: i32, h: i32, text: &str) {
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

fn render_lollipop_endpoint(out: &mut String, x: i32, y: i32, stroke: &str) {
    out.push_str(&format!(
        "<circle cx=\"{}\" cy=\"{}\" r=\"6\" fill=\"white\" stroke=\"{}\" stroke-width=\"1.5\" class=\"uml-lollipop\"/>",
        x,
        y,
        stroke
    ));
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
            Some(label) if !label.is_empty() => format!("{} {}", self.kind, label),
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
        | FamilyNodeKind::Artifact
        | FamilyNodeKind::Cloud
        | FamilyNodeKind::Frame
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
    out.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"{}\">{}</text>",
        cx, kind_tag_y, escape_text(&comp_style.font_color), kind_label
    ));
    render_node_stereotype_rows(out, node, cx, kind_tag_y + 13);
}

pub fn render_activity_svg(doc: &FamilyDocument) -> String {
    // Extract activity style (use defaults if not present)
    let act_style = match &doc.family_style {
        Some(FamilyStyle::Activity(s)) => s.clone(),
        _ => ActivityStyle::default(),
    };

    let step_h = 60i32;
    let title_lines = doc
        .title
        .as_deref()
        .map(|v| v.lines().count() as i32)
        .unwrap_or(0);
    let header_h = 40 + title_lines * 22;

    // ---------------------------------------------------------------------------
    // Pass 0: parse node metadata (step_kind, lane_name) for all nodes
    // ---------------------------------------------------------------------------
    struct NodeMeta {
        step_kind: String,
        lane_name: String,
        fork_branch: usize,
    }
    let metas: Vec<NodeMeta> = doc
        .nodes
        .iter()
        .map(|node| {
            let mut step_kind = String::new();
            let mut lane_name = "default".to_string();
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
            NodeMeta {
                step_kind,
                lane_name,
                fork_branch,
            }
        })
        .collect();

    // ---------------------------------------------------------------------------
    // Collect swim-lanes
    // ---------------------------------------------------------------------------
    let mut lanes: Vec<String> = Vec::new();
    for meta in &metas {
        if meta.lane_name != "default" && !lanes.iter().any(|l| l == &meta.lane_name) {
            lanes.push(meta.lane_name.clone());
        }
    }
    if lanes.is_empty() {
        lanes.push("default".to_string());
    }

    // Compute the base width from the number of lanes; we may widen for if/else.
    // Count max nesting depth of if/else to estimate extra width needed.
    let mut max_if_depth: i32 = 0;
    {
        let mut depth: i32 = 0;
        for meta in &metas {
            if meta.step_kind == "IfStart" {
                depth += 1;
                max_if_depth = max_if_depth.max(depth);
            } else if meta.step_kind == "EndIf" {
                depth = depth.saturating_sub(1);
            }
        }
    }
    // Branch horizontal offset: each nesting level adds 160px to either side
    let branch_x_offset = 160i32;
    // Total extra width: 2 * branch_x_offset * max_if_depth (left + right of center)
    let extra_branch_width = 2 * branch_x_offset * max_if_depth;

    let lane_area_x = 32i32;
    let base_lane_area_w = 416i32; // 480 - 64
    let lane_area_w = base_lane_area_w + extra_branch_width;
    let width = lane_area_w + 64;
    let lane_w = (lane_area_w / (lanes.len() as i32)).max(120);
    let lane_index = |name: &str| -> i32 {
        lanes
            .iter()
            .position(|l| l == name)
            .map(|i| i as i32)
            .unwrap_or(0)
    };
    let lane_center_x = |lane_name: &str| -> i32 {
        let idx = lane_index(lane_name);
        lane_area_x + idx * lane_w + lane_w / 2
    };

    // ---------------------------------------------------------------------------
    // Pass 1: compute layout positions for every node using a branch-aware
    // algorithm.
    //
    // For each node:
    //   slot_y      — top of the slot (y passed to shape renderers)
    //   arrow_out_y — where the outgoing arrow starts (slot_y + ARROW_OUT)
    //   next_slot_y — where the next node's slot begins (slot_y + step_h)
    //
    // current_slot_y tracks where the next node goes.
    // if_stack handles nested if/else.
    // ---------------------------------------------------------------------------

    const ARROW_OUT: i32 = 42; // visual bottom of a node within its slot

    struct IfFrame {
        diamond_cx: i32,
        diamond_arrow_out: i32, // arrow exit y of diamond
        diamond_next_slot: i32, // first slot_y inside the branches
        // then-branch: accumulated while in_else==false
        then_cx: i32,
        then_end_next_slot: i32, // current_slot_y saved at "Else" time
        // else-branch: accumulated while in_else==true
        in_else: bool,
        else_cx: i32,
        else_start_slot: i32, // slot_y of the Else marker (= diamond_next_slot)
    }

    // Per-node layout
    struct NodeLayout {
        cx: i32,
        slot_y: i32,
        arrow_out_y: i32,
        next_slot_y: i32,
    }

    let mut node_layouts: Vec<NodeLayout> = Vec::with_capacity(doc.nodes.len());
    // Extra arrows: (x1,y1, x2,y2) drawn in addition to prev→cur arrows
    let mut extra_arrows: Vec<(i32, i32, i32, i32)> = Vec::new();
    // Indices of nodes for which we suppress the standard prev→cur arrow
    let mut suppress_prev_arrow: std::collections::HashSet<usize> = Default::default();

    let mut current_slot_y = header_h;
    let mut if_stack: Vec<IfFrame> = Vec::new();

    for (i, meta) in metas.iter().enumerate() {
        let base_cx = lane_center_x(&meta.lane_name);
        let in_else_branch = if_stack.last().map(|f| f.in_else).unwrap_or(false);
        let cx = if in_else_branch {
            if_stack.last().map(|f| f.else_cx).unwrap_or(base_cx)
        } else {
            base_cx
        };

        match meta.step_kind.as_str() {
            "IfStart" => {
                let slot_y = current_slot_y;
                let arrow_out_y = slot_y + ARROW_OUT;
                let next_slot_y = slot_y + step_h;
                node_layouts.push(NodeLayout { cx, slot_y, arrow_out_y, next_slot_y });
                let else_cx = cx + branch_x_offset;
                if_stack.push(IfFrame {
                    diamond_cx: cx,
                    diamond_arrow_out: arrow_out_y,
                    diamond_next_slot: next_slot_y,
                    then_cx: cx,
                    then_end_next_slot: next_slot_y, // updated at "Else"
                    in_else: false,
                    else_cx,
                    else_start_slot: next_slot_y, // updated at "Else"
                });
                current_slot_y = next_slot_y;
            }
            "Else" => {
                // Save then-branch endpoint
                let then_end_next_slot = current_slot_y;
                let frame = if_stack.last_mut().expect("else without if");
                frame.then_cx = cx; // cx at end of then-branch (same lane)
                frame.then_end_next_slot = then_end_next_slot;
                // Else marker is placed at diamond_next_slot y, else_cx x
                let else_cx = frame.else_cx;
                let slot_y = frame.diamond_next_slot;
                let arrow_out_y = slot_y + ARROW_OUT;
                let next_slot_y = slot_y + step_h;
                frame.else_start_slot = slot_y;
                frame.in_else = true;
                // Suppress standard prev→cur; add diamond→Else arrow
                suppress_prev_arrow.insert(i);
                extra_arrows.push((frame.diamond_cx, frame.diamond_arrow_out, else_cx, slot_y));
                node_layouts.push(NodeLayout { cx: else_cx, slot_y, arrow_out_y, next_slot_y });
                current_slot_y = next_slot_y;
            }
            "EndIf" => {
                let frame = if_stack.pop().expect("endif without if");
                // then-branch end: (then_cx, arrow_out at end of then)
                let then_arrow_out_y = frame.then_end_next_slot - step_h + ARROW_OUT;
                let then_cx = frame.then_cx;
                // else-branch end: current_slot_y is past the last else node
                let else_arrow_out_y = current_slot_y - step_h + ARROW_OUT;
                let else_cx = frame.else_cx;
                // EndIf goes below the deeper branch
                let slot_y = frame.then_end_next_slot.max(current_slot_y);
                let arrow_out_y = slot_y + ARROW_OUT;
                let next_slot_y = slot_y + step_h;
                suppress_prev_arrow.insert(i);
                // Both branches converge on the EndIf node (at diamond_cx x)
                extra_arrows.push((then_cx, then_arrow_out_y, frame.diamond_cx, slot_y));
                extra_arrows.push((else_cx, else_arrow_out_y, frame.diamond_cx, slot_y));
                node_layouts.push(NodeLayout {
                    cx: frame.diamond_cx,
                    slot_y,
                    arrow_out_y,
                    next_slot_y,
                });
                current_slot_y = next_slot_y;
            }
            _ => {
                let slot_y = current_slot_y;
                let arrow_out_y = slot_y + ARROW_OUT;
                let next_slot_y = slot_y + step_h;
                node_layouts.push(NodeLayout { cx, slot_y, arrow_out_y, next_slot_y });
                current_slot_y = next_slot_y;
            }
        }
    }

    // Total height needed
    let height = node_layouts
        .iter()
        .map(|l| l.next_slot_y)
        .max()
        .unwrap_or(header_h + step_h)
        + 60;

    // ---------------------------------------------------------------------------
    // Emit SVG
    // ---------------------------------------------------------------------------
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        width, height, width, height
    ));
    out.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>",
        escape_text(&act_style.background_color)
    ));

    let mut y_cursor = 28;
    if let Some(title) = &doc.title {
        for line in title.lines() {
            out.push_str(&format!(
                "<text x=\"32\" y=\"{}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\" fill=\"{}\">{}</text>",
                y_cursor,
                escape_text(&act_style.font_color),
                escape_text(line)
            ));
            y_cursor += 22;
        }
    }
    out.push_str(&format!(
        "<text x=\"32\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"{}\">activity diagram</text>",
        y_cursor + 2,
        escape_text(&act_style.font_color)
    ));

    let lane_left = |idx: i32| -> i32 { lane_area_x + idx * lane_w };

    for (idx, lane) in lanes.iter().enumerate() {
        let lx = lane_left(idx as i32);
        let bg = if idx % 2 == 0 {
            act_style.background_color.as_str()
        } else {
            "#f1f5f9"
        };
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
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
                lx + lane_w / 2,
                header_h + 10,
                escape_text(&act_style.font_color),
                escape_text(lane)
            ));
        }
    }

    let box_w = (lane_w - 24).clamp(120, 220);
    let mut fork_anchor: Option<(i32, i32)> = None;

    // ---------------------------------------------------------------------------
    // Pass 2: render nodes and arrows using pre-computed positions
    // ---------------------------------------------------------------------------
    for (i, (node, meta)) in doc.nodes.iter().zip(metas.iter()).enumerate() {
        let layout = &node_layouts[i];
        let cx = layout.cx;
        let y = layout.slot_y;
        let label = node.label.clone().unwrap_or_default();
        let step_kind = &meta.step_kind;
        let fork_branch = meta.fork_branch;

        out.push_str(&format!(
            "<metadata data-activity-kind=\"{}\" data-activity-lane=\"{}\" data-activity-branch=\"{}\"/>",
            escape_text(step_kind),
            escape_text(&meta.lane_name),
            fork_branch
        ));
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
                if !label.is_empty() {
                    out.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"{}\">{}</text>",
                        cx,
                        y + 44,
                        escape_text(&act_style.font_color),
                        escape_text(&label)
                    ));
                }
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
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\" fill=\"{}\">{}</text>",
                    cx,
                    y + 27,
                    escape_text(&act_style.font_color),
                    escape_text(&label)
                ));
            }
            FamilyNodeKind::Note => {
                render_note_card(&mut out, cx - box_w / 2, y + 2, box_w, 44, &label);
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
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
                    cx,
                    y + 2 + dy + 4,
                    escape_text(&act_style.font_color),
                    escape_text(&label)
                ));
                if step_kind.contains("WhileStart") {
                    out.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"{}\">while</text>",
                        cx,
                        y + 54,
                        escape_text(&act_style.font_color)
                    ));
                }
                if step_kind.contains("RepeatWhile") {
                    out.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"{}\">repeat while</text>",
                        cx,
                        y + 54,
                        escape_text(&act_style.font_color)
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
                    let branch_label = if label.is_empty() {
                        format!("branch {}", fork_branch + 1)
                    } else {
                        format!("branch {} / {}", fork_branch + 1, label)
                    };
                    out.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"{}\">{}</text>",
                        cx,
                        y + 20,
                        escape_text(&act_style.font_color),
                        escape_text(&branch_label)
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
                    if label.is_empty() {
                        "(else)".to_string()
                    } else {
                        format!("(else) {}", label)
                    }
                } else if step_kind.contains("EndIf") {
                    "(endif)".to_string()
                } else if step_kind.contains("EndWhile") {
                    if label.is_empty() {
                        "(endwhile)".to_string()
                    } else {
                        format!("({label})")
                    }
                } else if step_kind.contains("RepeatStart") {
                    "(repeat)".to_string()
                } else {
                    format!("(merge) {}", label)
                };
                if !merge_label.is_empty() {
                    out.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
                        cx,
                        y + 28,
                        escape_text(&act_style.font_color),
                        escape_text(&merge_label)
                    ));
                }
            }
            FamilyNodeKind::ActivityPartition => {
                out.push_str(&format!(
                    "<rect x=\"24\" y=\"{}\" width=\"{}\" height=\"36\" rx=\"4\" ry=\"4\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"4 3\"/>",
                    y + 4,
                    width - 48,
                    escape_text(&act_style.background_color),
                    escape_text(&act_style.border_color)
                ));
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\" font-weight=\"600\" fill=\"{}\">{}</text>",
                    cx,
                    y + 27,
                    escape_text(&act_style.font_color),
                    escape_text(&format!("partition: {}", label))
                ));
            }
            _ => {
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\" fill=\"{}\">{}</text>",
                    cx,
                    y + 28,
                    escape_text(&act_style.font_color),
                    escape_text(&label)
                ));
            }
        }

        // Arrow from previous node (suppressed for branch-control nodes)
        if i > 0 && !suppress_prev_arrow.contains(&i) {
            let prev = &node_layouts[i - 1];
            let (px, py) = (prev.cx, prev.arrow_out_y);
            emit_activity_arrow(&mut out, px, py, cx, y, &act_style.arrow_color);
        }

        // Extra arrows for if-branching (diamond→else, branch-end→endif)
        for (x1, y1, x2, y2) in extra_arrows.iter().filter(|a| a.2 == cx && a.3 == y) {
            emit_activity_arrow(&mut out, *x1, *y1, *x2, *y2, &act_style.arrow_color);
        }

        // Fork branch arrows
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
    }

    out.push_str("</svg>");
    out
}

/// Emit a straight arrow from (x1,y1) to (x2,y2) with an arrowhead at (x2,y2).
fn emit_activity_arrow(out: &mut String, x1: i32, y1: i32, x2: i32, y2: i32, color: &str) {
    out.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        x1, y1, x2, y2, color
    ));
    // Arrowhead: small triangle pointing in the direction of travel
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len = ((dx * dx + dy * dy) as f64).sqrt().max(1.0);
    let ux = dx as f64 / len;
    let uy = dy as f64 / len;
    // Perpendicular
    let px = -uy;
    let py = ux;
    let tip_x = x2 as f64;
    let tip_y = y2 as f64;
    let base_x = tip_x - ux * 8.0;
    let base_y = tip_y - uy * 8.0;
    let l_x = (base_x + px * 4.0).round() as i32;
    let l_y = (base_y + py * 4.0).round() as i32;
    let r_x = (base_x - px * 4.0).round() as i32;
    let r_y = (base_y - py * 4.0).round() as i32;
    out.push_str(&format!(
        "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\"/>",
        x2, y2, l_x, l_y, r_x, r_y, color
    ));
}

fn render_sequence_note_shape(
    out: &mut String,
    kind: NoteKind,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    scene: &Scene,
) {
    let fill = &scene.style.note_background_color;
    let stroke = &scene.style.note_border_color;
    match kind {
        NoteKind::Folded => {
            let fold = 14.min(width / 4).min(height / 3).max(8);
            out.push_str(&format!(
                "<path d=\"M{x},{y} H{} L{} {} V{} H{x} Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                x + width - fold,
                x + width,
                y + fold,
                y + height,
                fill,
                stroke
            ));
            out.push_str(&format!(
                "<path d=\"M{} {y} V{} H{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1\"/>",
                x + width - fold,
                y + fold,
                x + width,
                stroke
            ));
        }
        NoteKind::Hexagonal => {
            let cut = 16.min(width / 5).max(8);
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                x + cut,
                y,
                x + width - cut,
                y,
                x + width,
                y + height / 2,
                x + width - cut,
                y + height,
                x + cut,
                y + height,
                x,
                y + height / 2,
                fill,
                stroke
            ));
        }
        NoteKind::Rectangle => {
            out.push_str(&format!(
                "<rect x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" rx=\"0\" ry=\"0\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                fill,
                stroke
            ));
        }
    }
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
            if parse_timing_range_note(&txt).is_some() {
                return None;
            }
            if txt.is_empty() {
                None
            } else {
                Some((t, txt))
            }
        })
        .collect();
    let timing_ranges: Vec<(i64, i64, String)> = events
        .iter()
        .filter_map(|e| {
            if e.alias.is_some() {
                return None;
            }
            let start = e.name.parse::<i64>().ok()?;
            let txt = e
                .label
                .clone()
                .or_else(|| e.members.first().map(|m| m.text.clone()))
                .unwrap_or_default();
            let (end, label) = parse_timing_range_note(&txt)?;
            Some((start, end, label))
        })
        .collect();

    // ── Parse time positions (@N) ─────────────────────────────────────────────
    // Collect unique numeric time values, sort them.
    let mut time_vals: Vec<i64> = events
        .iter()
        .filter_map(|e| e.name.parse::<i64>().ok())
        .collect();
    time_vals.extend(timing_ranges.iter().map(|(_, end, _)| *end));
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
    for (start, end, label) in &timing_ranges {
        let x1 = time_to_x((*start).min(*end));
        let x2 = time_to_x((*start).max(*end));
        let w = (x2 - x1).max(2);
        out.push_str(&format!(
            "<rect class=\"timing-range\" x=\"{x1}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"#fde68a\" opacity=\"0.45\" stroke=\"#f59e0b\" stroke-width=\"1\"/>",
            y = axis_top,
            h = axis_h + rows_h
        ));
        out.push_str(&format!(
            "<text class=\"timing-range-label\" x=\"{x}\" y=\"{y}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"#92400e\">{}</text>",
            escape_text(label),
            x = x1 + w / 2,
            y = axis_top + axis_h - 14
        ));
    }
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
        let signal_label = signal.label.as_deref().unwrap_or(&signal.name);
        out.push_str(&format!(
            "<text x=\"{x}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"12\" font-weight=\"600\" fill=\"{}\" text-anchor=\"end\">{name}</text>",
            escape_text(&style.font_color),
            x = left_pad - 8,
            ty = wave_mid + 4,
            name = escape_text(signal_label)
        ));
        // Signal kind tag
        out.push_str(&format!(
            "<text x=\"{x}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"9\" fill=\"#94a3b8\" text-anchor=\"end\">{kind}</text>",
            x = left_pad - 8,
            ty = wave_mid + 16,
            kind = family_node_label(signal.kind)
        ));
        if !signal.members.is_empty() {
            let controls = signal
                .members
                .iter()
                .map(|m| m.text.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!(
                "<text x=\"{x}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"9\" fill=\"#64748b\" text-anchor=\"end\">{controls}</text>",
                x = left_pad - 8,
                ty = wave_mid + 28,
                controls = escape_text(&controls)
            ));
        }

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
                    path.replace("M ", "").replace("L ", ""),
                    escape_text(&style.signal_border_color)
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
                let controlled_period = timing_control_i64(signal, "period");
                let controlled_pulse = timing_control_i64(signal, "pulse");
                let controlled_offset = timing_control_i64(signal, "offset").unwrap_or(0);
                let period = if let Some(period) = controlled_period {
                    period.max(1)
                } else if sig_events.len() >= 2 {
                    (sig_events[1].0 - sig_events[0].0).max(1)
                } else if time_vals.len() >= 2 {
                    (time_vals[1] - time_vals[0]).max(1)
                } else {
                    t_span / 4
                };
                let half = controlled_pulse
                    .unwrap_or_else(|| (period / 2).max(1))
                    .clamp(1, period.max(1));
                let t_end = t_max + period;

                let mut path_pts = String::new();
                let mut cur_t = t_min.saturating_add(controlled_offset);
                while cur_t > t_min {
                    cur_t = cur_t.saturating_sub(period);
                }
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
                    "<polyline data-timing-period=\"{period}\" data-timing-pulse=\"{half}\" data-timing-offset=\"{controlled_offset}\" points=\"{path_pts}\" fill=\"none\" stroke=\"{}\" stroke-width=\"2\"/>",
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

fn timing_control_i64(signal: &FamilyNode, key: &str) -> Option<i64> {
    for member in &signal.members {
        let mut parts = member.text.split_whitespace();
        while let Some(part) = parts.next() {
            if part.eq_ignore_ascii_case(key) {
                if let Some(value) = parts.next().and_then(|v| v.parse::<i64>().ok()) {
                    return Some(value);
                }
            }
        }
    }
    None
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
    render_relation_marker_defs(&mut out, "#475569");
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
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\" text-anchor=\"middle\" fill=\"{}\">{}</text>",
            width / 2,
            y_header,
            escape_text(&state_style.font_color),
            escape_text(title)
        ));
        y_header += 20;
    }
    out.push_str(&format!(
        "<text x=\"40\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"{}\">state diagram</text>",
        y_header,
        escape_text(&state_style.font_color)
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
                    "<path class=\"state-transition\" data-state-from=\"{}\" data-state-to=\"{}\" d=\"M {x1} {y1} Q {cpx} {cpy} {x2} {y2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}{} marker-end=\"url(#arrow)\"/>",
                    escape_text(&t.from),
                    escape_text(&t.to),
                    escape_text(t.line_color.as_deref().unwrap_or(&state_style.arrow_color)),
                    t.thickness.unwrap_or(2).clamp(1, 8),
                    state_dash_attr(t.dashed),
                    state_hidden_attr(t.hidden),
                    state_direction_attr(t.direction.as_deref())
                ));
            } else {
                out.push_str(&format!(
                    "<line class=\"state-transition\" data-state-from=\"{}\" data-state-to=\"{}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}{}{} marker-end=\"url(#arrow)\"/>",
                    escape_text(&t.from),
                    escape_text(&t.to),
                    x1, y1, x2, y2,
                    escape_text(t.line_color.as_deref().unwrap_or(&state_style.arrow_color)),
                    t.thickness.unwrap_or(2).clamp(1, 8),
                    state_dash_attr(t.dashed),
                    state_hidden_attr(t.hidden),
                    state_direction_attr(t.direction.as_deref())
                ));
            }
            if let Some(label) = &t.label {
                let mx = (x1 + x2) / 2;
                let my = (y1 + y2) / 2 - 6;
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\" text-anchor=\"middle\">{}</text>",
                    mx, my, escape_text(&state_style.font_color), escape_text(label)
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
    render_relation_marker_defs(&mut out, "#475569");
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

fn parse_timing_range_note(note: &str) -> Option<(i64, String)> {
    let rest = note.strip_prefix("range:")?;
    let (end, label) = rest.split_once(':').unwrap_or((rest, ""));
    let end = end.trim().trim_start_matches('@').parse::<i64>().ok()?;
    let label = if label.trim().is_empty() {
        "range".to_string()
    } else {
        label.trim().to_string()
    };
    Some((end, label))
}

pub fn render_nwdiag_svg(document: &NwdiagDocument) -> String {
    let width = 760;
    let net_rows: i32 = document
        .networks
        .iter()
        .map(|n| 1 + n.nodes.len() as i32)
        .sum();
    let group_rows: i32 = document
        .groups
        .iter()
        .map(|g| 1 + g.nodes.len() as i32)
        .sum();
    let height = 80
        + (net_rows + group_rows).max(1) * 24
        + ((document.networks.len() + document.groups.len()) as i32) * 14;
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
            let net_fill = net.color.as_deref().unwrap_or("#e0f2fe");
            let net_style = net.style.as_deref().unwrap_or("solid");
            let net_dash = if net_style.eq_ignore_ascii_case("dashed") {
                " stroke-dasharray=\"5 3\""
            } else {
                ""
            };
            out.push_str(&format!(
                "<rect class=\"nwdiag-network\" data-nwdiag-style=\"{}\" data-nwdiag-shape=\"{}\" x=\"24\" y=\"{}\" width=\"712\" height=\"22\" fill=\"{}\" stroke=\"#0284c7\" stroke-width=\"1\"{} />",
                escape_text(net_style),
                escape_text(net.shape.as_deref().unwrap_or("swimlane")),
                y,
                escape_text(net_fill),
                net_dash
            ));
            let net_name = net.label.as_deref().unwrap_or(&net.name);
            let label = match &net.address {
                Some(a) => format!("network {} ({})", net_name, a),
                None => format!("network {}", net_name),
            };
            out.push_str(&format!(
                "<text x=\"32\" y=\"{}\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" fill=\"#0c4a6e\">{}</text>",
                y + 16,
                escape_text(&label)
            ));
            y += 26;
            for node in &net.nodes {
                let node_fill = node.color.as_deref().unwrap_or("white");
                let shape = node.shape.as_deref().unwrap_or("box");
                let style = node.style.as_deref().unwrap_or("solid");
                let dashed = if style.eq_ignore_ascii_case("dashed") {
                    " stroke-dasharray=\"5 3\""
                } else {
                    ""
                };
                let node_width = node
                    .width
                    .and_then(|w| i32::try_from(w).ok())
                    .unwrap_or(680)
                    .clamp(120, 680);
                let radius = if shape.eq_ignore_ascii_case("roundedbox")
                    || shape.eq_ignore_ascii_case("cloud")
                {
                    10
                } else {
                    3
                };
                out.push_str(&format!(
                    "<rect class=\"nwdiag-node\" data-nwdiag-name=\"{}\" data-nwdiag-addresses=\"{}\" data-nwdiag-shape=\"{}\" data-nwdiag-style=\"{}\" x=\"56\" y=\"{}\" width=\"{}\" height=\"20\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"#0284c7\" stroke-width=\"1\"{}/>",
                    escape_text(&node.name),
                    escape_text(&node.addresses.join(", ")),
                    escape_text(shape),
                    escape_text(style),
                    y,
                    node_width,
                    radius,
                    radius,
                    escape_text(node_fill),
                    dashed
                ));
                let display = node.label.as_deref().unwrap_or(&node.name);
                let lbl = if node.addresses.is_empty() {
                    display.to_string()
                } else {
                    format!("{} [{}]", display, node.addresses.join(", "))
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
        for group in &document.groups {
            let fill = group.color.as_deref().unwrap_or("#fef3c7");
            let style = group.style.as_deref().unwrap_or("solid");
            let dashed = if style.eq_ignore_ascii_case("dashed") {
                " stroke-dasharray=\"5 3\""
            } else {
                ""
            };
            out.push_str(&format!(
                "<rect class=\"nwdiag-group\" data-nwdiag-style=\"{}\" data-nwdiag-shape=\"{}\" x=\"24\" y=\"{}\" width=\"712\" height=\"22\" fill=\"{}\" stroke=\"#d97706\" stroke-width=\"1\"{} />",
                escape_text(style),
                escape_text(group.shape.as_deref().unwrap_or("box")),
                y,
                escape_text(fill),
                dashed
            ));
            let group_label = group.label.as_deref().unwrap_or(&group.name);
            out.push_str(&format!(
                "<text x=\"32\" y=\"{}\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" fill=\"#78350f\">group {}</text>",
                y + 16,
                escape_text(group_label)
            ));
            y += 26;
            for node in &group.nodes {
                out.push_str(&format!(
                    "<rect class=\"nwdiag-group-member\" x=\"56\" y=\"{}\" width=\"680\" height=\"20\" rx=\"3\" ry=\"3\" fill=\"#fff7ed\" stroke=\"#f59e0b\" stroke-width=\"1\"/>",
                    y
                ));
                out.push_str(&format!(
                    "<text x=\"66\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#0f172a\">{}</text>",
                    y + 14,
                    escape_text(node)
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
        "junction",
    ];
    let lane_height = 80;
    let height = 80 + (layers.len() as i32) * lane_height + (document.relations.len() as i32) * 18;
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        width, height, width, height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    render_relation_marker_defs(&mut out, "#475569");
    let mut y = 28;
    out.push_str(&format!(
        "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\">{}</text>",
        y,
        escape_text(document.title.as_deref().unwrap_or("Archimate"))
    ));
    y += 16;
    let mut element_positions: BTreeMap<String, (i32, i32)> = BTreeMap::new();
    for layer in layers.iter() {
        let layer_y = y;
        let bg = match *layer {
            "strategy" => "#fee2e2",
            "business" => "#fef3c7",
            "application" => "#dbeafe",
            "technology" => "#dcfce7",
            "motivation" => "#ede9fe",
            "junction" => "#f1f5f9",
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
            let fill = elem.fill.as_deref().unwrap_or("white");
            let stroke = elem.stroke.as_deref().unwrap_or("#334155");
            out.push_str(&format!(
                "<rect class=\"archimate-element\" data-archimate-layer=\"{}\" data-archimate-alias=\"{}\" x=\"{}\" y=\"{}\" width=\"140\" height=\"40\" rx=\"4\" ry=\"4\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                escape_text(&elem.layer),
                escape_text(elem.alias.as_deref().unwrap_or("")),
                x,
                layer_y + 22,
                escape_text(fill),
                escape_text(stroke)
            ));
            element_positions.insert(elem.name.clone(), (x + 70, layer_y + 42));
            if let Some(alias) = &elem.alias {
                element_positions.insert(alias.clone(), (x + 70, layer_y + 42));
            }
            if elem.layer == "junction" {
                out.push_str(&format!(
                    "<circle class=\"archimate-junction\" cx=\"{}\" cy=\"{}\" r=\"8\" fill=\"#334155\"/>",
                    x + 122,
                    layer_y + 34
                ));
            }
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
    for rel in &document.relations {
        let Some(&(x1, y1)) = element_positions.get(&rel.from) else {
            continue;
        };
        let Some(&(x2, y2)) = element_positions.get(&rel.to) else {
            continue;
        };
        let color = rel
            .style
            .as_deref()
            .filter(|style| style.starts_with('#') || style.starts_with('$'))
            .unwrap_or("#475569");
        let dashed = rel
            .style
            .as_deref()
            .is_some_and(|style| style.to_ascii_lowercase().contains("dashed"));
        let width = if rel
            .style
            .as_deref()
            .is_some_and(|style| style.to_ascii_lowercase().contains("bold"))
        {
            2.5
        } else {
            1.5
        };
        out.push_str(&format!(
            "<line class=\"archimate-relation-edge\" data-archimate-kind=\"{}\" data-archimate-direction=\"{}\" data-archimate-style=\"{}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{} marker-end=\"url(#arrow-open)\"/>",
            escape_text(&rel.kind),
            escape_text(rel.direction.as_deref().unwrap_or("")),
            escape_text(rel.style.as_deref().unwrap_or("")),
            x1,
            y1,
            x2,
            y2,
            escape_text(color),
            width,
            if dashed { " stroke-dasharray=\"5 3\"" } else { "" }
        ));
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
            let direction = rel
                .direction
                .as_deref()
                .map(|d| format!(" direction={d}"))
                .unwrap_or_default();
            let style = rel
                .style
                .as_deref()
                .map(|s| format!(" style={s}"))
                .unwrap_or_default();
            out.push_str(&format!(
                "<text class=\"archimate-relation\" data-archimate-kind=\"{}\" data-archimate-direction=\"{}\" data-archimate-style=\"{}\" x=\"40\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#1e293b\">{} -[{}{}{}]-&gt; {}{}</text>",
                escape_text(&rel.kind),
                escape_text(rel.direction.as_deref().unwrap_or("")),
                escape_text(rel.style.as_deref().unwrap_or("")),
                y,
                escape_text(&rel.from),
                escape_text(&rel.kind),
                escape_text(&direction),
                escape_text(&style),
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
        let (class_name, fill, stroke) = regex_label_style(label);
        out.push_str(&format!(
            "<rect class=\"regex-token {class_name}\" x=\"{x}\" y=\"{ry}\" width=\"{w}\" height=\"22\" rx=\"4\" ry=\"4\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
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

fn regex_label_style(label: &str) -> (&'static str, &'static str, &'static str) {
    if label.contains("alt(") {
        ("regex-alt", "#fef3c7", "#d97706")
    } else if label.contains('{')
        || label.ends_with('?')
        || label.ends_with('*')
        || label.ends_with('+')
    {
        ("regex-repeat", "#dcfce7", "#16a34a")
    } else if label.starts_with('[') {
        ("regex-class", "#ede9fe", "#7c3aed")
    } else if label == "^" || label == "$" {
        ("regex-anchor", "#fee2e2", "#dc2626")
    } else {
        ("regex-literal", "#e0f2fe", "#0284c7")
    }
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
                RepeatKind::Exact(n) => return format!("{}{{{}}}", regex_token_label(inner), n),
                RepeatKind::Range { min, max } => {
                    return format!(
                        "{}{{{},{}}}",
                        regex_token_label(inner),
                        min.map(|n| n.to_string()).unwrap_or_default(),
                        max.map(|n| n.to_string()).unwrap_or_default()
                    );
                }
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
                let (class_name, fill, stroke) = ebnf_label_style(label);
                out.push_str(&format!(
                    "<rect class=\"ebnf-token {class_name}\" x=\"{x}\" y=\"{ry}\" width=\"{w}\" height=\"22\" rx=\"4\" ry=\"4\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
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

fn ebnf_label_style(label: &str) -> (&'static str, &'static str, &'static str) {
    if label.starts_with('"') || label.starts_with('\'') {
        ("ebnf-terminal", "#fef3c7", "#d97706")
    } else if label.starts_with('[') {
        ("ebnf-optional", "#dcfce7", "#16a34a")
    } else if label.starts_with('{') {
        ("ebnf-repetition", "#ede9fe", "#7c3aed")
    } else if label.contains(" | ") {
        ("ebnf-alt", "#fee2e2", "#dc2626")
    } else if label.contains('{')
        || label.ends_with('?')
        || label.ends_with('*')
        || label.ends_with('+')
    {
        ("ebnf-repeat", "#e0f2fe", "#0284c7")
    } else {
        ("ebnf-nonterminal", "#e0e7ff", "#4f46e5")
    }
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
                RepeatKind::Exact(n) => return format!("{}{{{}}}", ebnf_token_label(inner), n),
                RepeatKind::Range { min, max } => {
                    return format!(
                        "{}{{{},{}}}",
                        ebnf_token_label(inner),
                        min.map(|n| n.to_string()).unwrap_or_default(),
                        max.map(|n| n.to_string()).unwrap_or_default()
                    );
                }
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
    let series = effective_chart_series(document);
    let categories = effective_chart_categories(document, &series);
    let type_name = chart_subtype_name(document.subtype);
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\" data-chart-type=\"{type_name}\" data-chart-horizontal=\"{}\" data-chart-stacked=\"{}\">",
        document.horizontal,
        document.stacked,
        w = width,
        h = height
    ));
    out.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>",
        escape_text(&style.background_color)
    ));
    out.push_str(&format!(
        "<metadata data-chart-style=\"{} {} {} {} {} {} {} {}\"/>",
        escape_text(&style.background_color),
        escape_text(&style.axis_color),
        escape_text(&style.grid_color),
        escape_text(&style.series_color),
        escape_text(&style.bar_color),
        escape_text(&style.line_color),
        escape_text(&style.pie_border_color),
        escape_text(&style.font_color)
    ));
    let mut y = 28;
    if let Some(title) = &document.title {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\">{}</text>",
            escape_text(title)
        ));
        y += 22;
    }
    out.push_str(&format!(
        "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">{}</text>",
        type_name
    ));
    if !document.palette.is_empty() {
        out.push_str(&format!(
            "<metadata data-chart-palette=\"{}\"/>",
            escape_text(&document.palette.join(" "))
        ));
    }
    if !series.is_empty() {
        let names = series
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>()
            .join("|");
        out.push_str(&format!(
            "<metadata data-chart-series=\"{}\"/>",
            escape_text(&names)
        ));
    }
    out.push_str(&format!(
        "<metadata data-chart-label-mode=\"{}\"/>",
        chart_label_mode_name(document.label_mode)
    ));
    let legend_visible = chart_legend_visible(document, &series);
    let legend_left = legend_visible && document.legend.h_align == crate::model::LegendHAlign::Left;
    let legend_right =
        legend_visible && document.legend.h_align == crate::model::LegendHAlign::Right;
    let legend_bottom =
        legend_visible && document.legend.v_align == crate::model::LegendVAlign::Bottom;
    let plot_top =
        y + if legend_visible && document.legend.v_align == crate::model::LegendVAlign::Top {
            54
        } else {
            16
        };
    let plot_bottom = height - if legend_bottom { 122 } else { 74 };
    let plot_left = if legend_left { 218 } else { 78 };
    let plot_right = width - if legend_right { 178 } else { 40 };
    let plot = ChartPlotArea {
        left: plot_left,
        top: plot_top,
        right: plot_right,
        bottom: plot_bottom,
    };
    match document.subtype {
        ChartSubtype::Bar if document.horizontal => {
            render_chart_horizontal_bars(&mut out, document, &series, &categories, plot, style)
        }
        ChartSubtype::Bar => {
            render_chart_bars(&mut out, document, &series, &categories, plot, style)
        }
        ChartSubtype::Line => {
            render_chart_line(&mut out, document, &series, &categories, plot, style)
        }
        ChartSubtype::Pie => {
            let points = effective_chart_points(document, &series, &categories);
            render_chart_pie(
                document,
                &mut out,
                &points,
                width / 2,
                (plot_top + plot_bottom) / 2,
                style,
            )
        }
    }
    render_chart_annotations(&mut out, document, plot);
    render_chart_caption(&mut out, document, width, height);
    render_chart_legend(&mut out, document, &series, plot);
    out.push_str("</svg>");
    out
}

const CHART_PALETTE: &[&str] = &[
    "#1d4ed8", "#16a34a", "#d97706", "#7c3aed", "#0891b2", "#dc2626", "#0f172a", "#facc15",
];

#[derive(Clone, Copy)]
struct ChartPlotArea {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

fn render_chart_bars(
    out: &mut String,
    document: &ChartDocument,
    series: &[crate::model::ChartSeries],
    categories: &[String],
    plot: ChartPlotArea,
    style: &crate::theme::ChartStyle,
) {
    render_chart_axes(out, document, categories, plot, style);
    if series.is_empty() || categories.is_empty() {
        return;
    }
    let (min_value, max_value) = chart_value_range(document, series);
    let count = categories.len() as i32;
    let avail = (plot.right - plot.left).max(20);
    let band = (avail / count).max(10);
    let group_count = if document.stacked {
        1
    } else {
        series.len().max(1) as i32
    };
    let bar_w = ((band - 8) / group_count).max(4);
    for (cat_idx, category) in categories.iter().enumerate() {
        let band_x = plot.left + (cat_idx as i32) * band;
        let mut stack_pos = 0.0_f64;
        let mut stack_neg = 0.0_f64;
        for (series_idx, item) in series.iter().enumerate() {
            let value = item.values.get(cat_idx).copied().unwrap_or(0.0);
            let bx = band_x
                + 4
                + if document.stacked {
                    0
                } else {
                    (series_idx as i32) * bar_w
                };
            let (from, to) = if document.stacked {
                if value >= 0.0 {
                    let from = stack_pos;
                    stack_pos += value;
                    (from, stack_pos)
                } else {
                    let from = stack_neg;
                    stack_neg += value;
                    (from, stack_neg)
                }
            } else {
                (0.0, value)
            };
            let y1 = chart_y_for_value(from, min_value, max_value, plot);
            let y2 = chart_y_for_value(to, min_value, max_value, plot);
            let by = y1.min(y2);
            let bh = (y1 - y2).abs().max(1);
            let color = chart_series_color(document, item, series_idx, style.bar_color.as_str());
            out.push_str(&format!(
                "<rect x=\"{bx}\" y=\"{by}\" width=\"{bw}\" height=\"{bh}\" fill=\"{color}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
                escape_text(&style.axis_color),
                bx = bx,
                by = by,
                bw = bar_w,
                bh = bh,
                color = escape_text(&color)
            ));
            out.push_str(&format!(
                "<text class=\"chart-value-label\" data-chart-label-mode=\"{}\" x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"#334155\">{}</text>",
                chart_label_mode_name(document.label_mode),
                format_chart_value(value),
                tx = bx + bar_w / 2,
                ty = if value >= 0.0 { by - 4 } else { by + bh + 12 }
            ));
        }
        out.push_str(&format!(
            "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
            escape_text(&style.font_color),
            escape_text(category),
            tx = band_x + band / 2,
            ty = plot.bottom + 16
        ));
    }
}

fn render_chart_line(
    out: &mut String,
    document: &ChartDocument,
    series: &[crate::model::ChartSeries],
    categories: &[String],
    plot: ChartPlotArea,
    style: &crate::theme::ChartStyle,
) {
    render_chart_axes(out, document, categories, plot, style);
    if series.is_empty() || categories.is_empty() {
        return;
    }
    let (min_value, max_value) = chart_value_range(document, series);
    let count = categories.len() as i32;
    let step = ((plot.right - plot.left) as f64) / ((count.max(2) - 1) as f64).max(1.0);
    for (series_idx, item) in series.iter().enumerate() {
        let color = chart_series_color(document, item, series_idx, style.line_color.as_str());
        let mut points = String::new();
        for (idx, category) in categories.iter().enumerate() {
            let value = item.values.get(idx).copied().unwrap_or(0.0);
            let px = plot.left + ((idx as f64) * step) as i32;
            let py = chart_y_for_value(value, min_value, max_value, plot);
            if !points.is_empty() {
                points.push(' ');
            }
            points.push_str(&format!("{px},{py}"));
            out.push_str(&format!(
                "<circle class=\"chart-point\" data-chart-value=\"{}\" cx=\"{px}\" cy=\"{py}\" r=\"3\" fill=\"{}\"/>",
                format_chart_value(value),
                escape_text(&color)
            ));
            if !matches!(document.label_mode, ChartLabelMode::None) {
                out.push_str(&format!(
                    "<text class=\"chart-value-label\" data-chart-label-mode=\"{}\" x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"#334155\">{}</text>",
                    chart_label_mode_name(document.label_mode),
                    format_chart_value(value),
                    tx = px,
                    ty = py - 7
                ));
            }
            if series_idx == 0 {
                out.push_str(&format!(
                    "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
                    escape_text(&style.font_color),
                    escape_text(category),
                    tx = px,
                    ty = plot.bottom + 16
                ));
            }
        }
        out.push_str(&format!(
            "<polyline points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            points,
            escape_text(&color)
        ));
    }
}

fn render_chart_horizontal_bars(
    out: &mut String,
    document: &ChartDocument,
    series: &[crate::model::ChartSeries],
    categories: &[String],
    plot: ChartPlotArea,
    style: &crate::theme::ChartStyle,
) {
    render_chart_axes(out, document, categories, plot, style);
    if series.is_empty() || categories.is_empty() {
        return;
    }
    let (min_value, max_value) = chart_value_range(document, series);
    let count = categories.len() as i32;
    let avail = (plot.bottom - plot.top).max(20);
    let band = (avail / count).max(10);
    let group_count = if document.stacked {
        1
    } else {
        series.len().max(1) as i32
    };
    let bar_h = ((band - 8) / group_count).max(4);
    for (cat_idx, category) in categories.iter().enumerate() {
        let band_y = plot.top + (cat_idx as i32) * band;
        let mut stack_pos = 0.0_f64;
        let mut stack_neg = 0.0_f64;
        for (series_idx, item) in series.iter().enumerate() {
            let value = item.values.get(cat_idx).copied().unwrap_or(0.0);
            let (from, to) = if document.stacked {
                if value >= 0.0 {
                    let from = stack_pos;
                    stack_pos += value;
                    (from, stack_pos)
                } else {
                    let from = stack_neg;
                    stack_neg += value;
                    (from, stack_neg)
                }
            } else {
                (0.0, value)
            };
            let x1 = chart_x_for_value(from, min_value, max_value, plot);
            let x2 = chart_x_for_value(to, min_value, max_value, plot);
            let bx = x1.min(x2);
            let bw = (x1 - x2).abs().max(1);
            let by = band_y
                + 4
                + if document.stacked {
                    0
                } else {
                    (series_idx as i32) * bar_h
                };
            let color = chart_series_color(document, item, series_idx, style.bar_color.as_str());
            out.push_str(&format!(
                "<rect x=\"{bx}\" y=\"{by}\" width=\"{bw}\" height=\"{bh}\" fill=\"{color}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
                escape_text(&style.axis_color),
                bx = bx,
                by = by,
                bw = bw,
                bh = bar_h,
                color = escape_text(&color)
            ));
        }
        out.push_str(&format!(
            "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
            escape_text(&style.font_color),
            escape_text(category),
            tx = plot.left - 8,
            ty = band_y + band / 2 + 4
        ));
    }
}

fn render_chart_pie(
    document: &ChartDocument,
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
        let color = chart_slice_color(document, point, idx, style.series_color.as_str());
        out.push_str(&format!(
            "<path class=\"chart-pie-slice\" data-chart-slice=\"{}\" data-chart-value=\"{}\" data-chart-percent=\"{}\" d=\"M {cx} {cy} L {x1:.2} {y1:.2} A {r} {r} 0 {large} 1 {x2:.2} {y2:.2} Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
            escape_text(&point.label),
            format_chart_value(point.value),
            format_chart_percent(v, total),
            escape_text(&color),
            escape_text(&style.pie_border_color),
            cx = cx,
            cy = cy,
            r = radius,
            x1 = x1,
            y1 = y1,
            x2 = x2,
            y2 = y2,
            large = large
        ));
        let mid = (start + end) / 2.0;
        if !matches!(document.label_mode, ChartLabelMode::None) {
            let label_radius = if matches!(document.label_mode, ChartLabelMode::Outside) {
                1.23
            } else {
                0.6
            };
            let lx = cx as f64 + ((radius as f64) * label_radius) * mid.cos();
            let ly = cy as f64 + ((radius as f64) * label_radius) * mid.sin();
            let label_text = chart_pie_label_text(document.label_mode, point, v, total);
            if matches!(document.label_mode, ChartLabelMode::Outside) {
                let c1x = cx as f64 + ((radius as f64) * 0.82) * mid.cos();
                let c1y = cy as f64 + ((radius as f64) * 0.82) * mid.sin();
                let c2x = cx as f64 + ((radius as f64) * 1.08) * mid.cos();
                let c2y = cy as f64 + ((radius as f64) * 1.08) * mid.sin();
                out.push_str(&format!(
                    "<line class=\"chart-pie-callout\" data-chart-slice-callout=\"{}\" x1=\"{c1x:.0}\" y1=\"{c1y:.0}\" x2=\"{c2x:.0}\" y2=\"{c2y:.0}\" stroke=\"{}\" stroke-width=\"0.75\"/>",
                    escape_text(&point.label),
                    escape_text(&style.axis_color),
                ));
            }
            out.push_str(&format!(
                "<text class=\"chart-pie-label\" data-chart-label-mode=\"{}\" data-chart-slice-label=\"{}\" x=\"{lx:.0}\" y=\"{ly:.0}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
                chart_label_mode_name(document.label_mode),
                escape_text(&point.label),
                escape_text(&style.font_color),
                escape_text(&label_text),
                lx = lx,
                ly = ly
            ));
        }
    }
}

fn render_chart_axes(
    out: &mut String,
    document: &ChartDocument,
    categories: &[String],
    plot: ChartPlotArea,
    style: &crate::theme::ChartStyle,
) {
    let h_axis = document.h_axis.as_ref();
    let v_axis = document.v_axis.as_ref();
    let h_axis_color = chart_axis_color(h_axis, &style.axis_color);
    let v_axis_color = chart_axis_color(v_axis, &style.axis_color);
    let h_label_color = chart_axis_label_color(h_axis, &style.font_color);
    let v_label_color = chart_axis_label_color(v_axis, &style.font_color);
    let h_grid_color = chart_axis_grid_color(h_axis, &style.grid_color);
    let v_grid_color = chart_axis_grid_color(v_axis, &style.grid_color);
    out.push_str(&format!(
        "<line x1=\"{l}\" y1=\"{b}\" x2=\"{r}\" y2=\"{b}\" stroke=\"{}\" stroke-width=\"1\"/>",
        escape_text(h_axis_color),
        l = plot.left,
        r = plot.right,
        b = plot.bottom
    ));
    out.push_str(&format!(
        "<line x1=\"{l}\" y1=\"{t}\" x2=\"{l}\" y2=\"{b}\" stroke=\"{}\" stroke-width=\"1\"/>",
        escape_text(v_axis_color),
        l = plot.left,
        t = plot.top,
        b = plot.bottom
    ));
    let series = effective_chart_series(document);
    let (min_value, max_value) = chart_value_range(document, &series);
    let ticks = chart_axis_ticks(document, min_value, max_value);
    for value in ticks {
        let y = chart_y_for_value(value, min_value, max_value, plot);
        out.push_str(&format!(
            "<line class=\"chart-axis-grid chart-axis-grid-v\" x1=\"{l}\" y1=\"{y}\" x2=\"{r}\" y2=\"{y}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
            escape_text(v_grid_color),
            l = plot.left,
            r = plot.right
        ));
        out.push_str(&format!(
            "<text class=\"chart-axis-tick chart-axis-tick-v\" data-chart-axis-tick=\"{}\" x=\"{x}\" y=\"{ty}\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"10\" fill=\"{}\">{}</text>",
            format_chart_value(value),
            escape_text(v_label_color),
            format_chart_value(value),
            x = plot.left - 8,
            ty = y + 4
        ));
    }
    out.push_str(&format!(
        "<metadata data-chart-axis-v-range=\"{}..{}\"/>",
        format_chart_value(min_value),
        format_chart_value(max_value)
    ));
    render_chart_axis_metadata(out, "h", h_axis);
    render_chart_axis_metadata(out, "v", v_axis);
    if min_value <= 0.0 && max_value >= 0.0 {
        if document.horizontal {
            let x = chart_x_for_value(0.0, min_value, max_value, plot);
            out.push_str(&format!(
                "<line class=\"chart-zero-axis\" x1=\"{x}\" y1=\"{t}\" x2=\"{x}\" y2=\"{b}\" stroke=\"{}\" stroke-width=\"1.25\"/>",
                escape_text(v_axis_color),
                t = plot.top,
                b = plot.bottom
            ));
        } else {
            let y = chart_y_for_value(0.0, min_value, max_value, plot);
            out.push_str(&format!(
                "<line class=\"chart-zero-axis\" x1=\"{l}\" y1=\"{y}\" x2=\"{r}\" y2=\"{y}\" stroke=\"{}\" stroke-width=\"1.25\"/>",
                escape_text(v_axis_color),
                l = plot.left,
                r = plot.right
            ));
        }
    }
    if let Some(axis) = &document.h_axis {
        if let Some(label) = &axis.label {
            out.push_str(&format!(
                "<text x=\"{x}\" y=\"{y}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\" fill=\"{}\">{}</text>",
                escape_text(h_label_color),
                escape_text(label),
                x = (plot.left + plot.right) / 2,
                y = plot.bottom + 42
            ));
        }
    }
    if let Some(axis) = &document.v_axis {
        if let Some(label) = &axis.label {
            out.push_str(&format!(
                "<text x=\"18\" y=\"{y}\" transform=\"rotate(-90 18 {y})\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\" fill=\"{}\">{}</text>",
                escape_text(v_label_color),
                escape_text(label),
                y = (plot.top + plot.bottom) / 2
            ));
        }
    }
    if document.horizontal {
        return;
    }
    if categories.len() > 1 {
        let step =
            ((plot.right - plot.left) as f64) / ((categories.len() as i32 - 1).max(1) as f64);
        for idx in 0..categories.len() {
            let x = plot.left + ((idx as f64) * step) as i32;
            out.push_str(&format!(
                "<line class=\"chart-axis-grid chart-axis-grid-h\" x1=\"{x}\" y1=\"{t}\" x2=\"{x}\" y2=\"{b}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
                escape_text(h_grid_color),
                t = plot.top,
                b = plot.bottom
            ));
        }
    }
}

fn render_chart_legend(
    out: &mut String,
    document: &ChartDocument,
    series: &[crate::model::ChartSeries],
    plot: ChartPlotArea,
) {
    if !chart_legend_visible(document, series) {
        return;
    }
    let pie_points;
    let legend_items: Vec<ChartLegendItem<'_>> = if document.subtype == ChartSubtype::Pie {
        let categories = effective_chart_categories(document, series);
        pie_points = effective_chart_points(document, series, &categories);
        pie_points
            .iter()
            .enumerate()
            .map(|(idx, point)| ChartLegendItem {
                name: point.label.as_str(),
                color: chart_slice_color(document, point, idx, "#1d4ed8"),
            })
            .collect()
    } else {
        series
            .iter()
            .enumerate()
            .map(|(idx, item)| ChartLegendItem {
                name: item.name.as_str(),
                color: chart_series_color(document, item, idx, "#1d4ed8"),
            })
            .collect()
    };
    if legend_items.is_empty() {
        return;
    }
    let x = match document.legend.h_align {
        crate::model::LegendHAlign::Left => 24,
        crate::model::LegendHAlign::Center => ((plot.left + plot.right) / 2) - 66,
        crate::model::LegendHAlign::Right => plot.right + 20,
    };
    let y = match document.legend.v_align {
        crate::model::LegendVAlign::Top => (plot.top - 44).max(44),
        crate::model::LegendVAlign::Bottom => plot.bottom + 46,
    };
    let width = 132;
    let height = 18 + (legend_items.len() as i32) * 18;
    let background = document
        .legend
        .background_color
        .as_deref()
        .unwrap_or("#ffffff");
    let border = document.legend.border_color.as_deref().unwrap_or("#cbd5e1");
    let text_color = document.legend.text_color.as_deref().unwrap_or("#0f172a");
    out.push_str(&format!(
        "<g class=\"chart-legend\" data-chart-legend=\"{}\" data-chart-legend-h=\"{}\" data-chart-legend-v=\"{}\"><rect x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" rx=\"4\" fill=\"{}\" stroke=\"{}\"/>",
        chart_legend_position(document),
        chart_legend_h_name(document.legend.h_align),
        chart_legend_v_name(document.legend.v_align),
        escape_text(background),
        escape_text(border)
    ));
    for (idx, item) in legend_items.iter().enumerate() {
        let cy = y + 18 + (idx as i32) * 18;
        out.push_str(&format!(
            "<rect class=\"chart-legend-swatch\" x=\"{x1}\" y=\"{y1}\" width=\"10\" height=\"10\" fill=\"{}\"/><text class=\"chart-legend-label\" x=\"{tx}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
            escape_text(&item.color),
            escape_text(text_color),
            escape_text(item.name),
            x1 = x + 8,
            y1 = cy - 9,
            tx = x + 24,
            ty = cy
        ));
    }
    out.push_str("</g>");
}

struct ChartLegendItem<'a> {
    name: &'a str,
    color: String,
}

fn render_chart_annotations(out: &mut String, document: &ChartDocument, plot: ChartPlotArea) {
    if document.annotations.is_empty() {
        return;
    }
    let mut y = plot.top + 8;
    for annotation in &document.annotations {
        out.push_str(&format!(
            "<g data-chart-annotation=\"{}\"><rect x=\"{x}\" y=\"{y}\" width=\"190\" height=\"24\" rx=\"5\" ry=\"5\" fill=\"#fff7ed\" stroke=\"#f97316\" stroke-width=\"1\"/><text x=\"{tx}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"10\" fill=\"#7c2d12\">{}: {}</text></g>",
            escape_text(&annotation.target),
            escape_text(&annotation.target),
            escape_text(&annotation.text),
            x = plot.right - 196,
            y = y,
            tx = plot.right - 186,
            ty = y + 16
        ));
        y += 30;
    }
}

fn render_chart_caption(out: &mut String, document: &ChartDocument, width: i32, height: i32) {
    if let Some(caption) = &document.caption {
        out.push_str(&format!(
            "<text data-chart-caption=\"true\" x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#475569\">{}</text>",
            width / 2,
            height - 18,
            escape_text(caption)
        ));
    }
}

fn effective_chart_series(document: &ChartDocument) -> Vec<crate::model::ChartSeries> {
    if !document.series.is_empty() {
        return document.series.clone();
    }
    if document.data.is_empty() {
        return Vec::new();
    }
    vec![crate::model::ChartSeries {
        name: "Value".to_string(),
        values: document.data.iter().map(|p| p.value).collect(),
        color: None,
    }]
}

fn effective_chart_categories(
    document: &ChartDocument,
    series: &[crate::model::ChartSeries],
) -> Vec<String> {
    if let Some(axis) = &document.h_axis {
        if !axis.categories.is_empty() {
            return axis.categories.clone();
        }
    }
    if !document.data.is_empty() {
        return document.data.iter().map(|p| p.label.clone()).collect();
    }
    let count = series.iter().map(|s| s.values.len()).max().unwrap_or(0);
    (1..=count).map(|idx| idx.to_string()).collect()
}

fn effective_chart_points(
    document: &ChartDocument,
    series: &[crate::model::ChartSeries],
    categories: &[String],
) -> Vec<crate::model::ChartPoint> {
    if !document.data.is_empty() {
        return document.data.clone();
    }
    let Some(first_series) = series.first() else {
        return Vec::new();
    };
    first_series
        .values
        .iter()
        .enumerate()
        .map(|(idx, value)| crate::model::ChartPoint {
            label: categories
                .get(idx)
                .cloned()
                .unwrap_or_else(|| (idx + 1).to_string()),
            value: *value,
            color: first_series.color.clone(),
        })
        .collect()
}

fn chart_value_range(document: &ChartDocument, series: &[crate::model::ChartSeries]) -> (f64, f64) {
    let axis_min = document.v_axis.as_ref().and_then(|axis| axis.min);
    let axis_max = document.v_axis.as_ref().and_then(|axis| axis.max);
    let (computed_min, computed_max) = if document.stacked {
        let categories = series.iter().map(|s| s.values.len()).max().unwrap_or(0);
        let mut min_value = 0.0_f64;
        let mut max_value = 0.0_f64;
        for idx in 0..categories {
            let mut positive = 0.0_f64;
            let mut negative = 0.0_f64;
            for value in series
                .iter()
                .map(|s| s.values.get(idx).copied().unwrap_or(0.0))
            {
                if value >= 0.0 {
                    positive += value;
                } else {
                    negative += value;
                }
            }
            min_value = min_value.min(negative);
            max_value = max_value.max(positive);
        }
        (min_value, max_value)
    } else {
        let mut values = series
            .iter()
            .flat_map(|s| s.values.iter().copied())
            .peekable();
        if values.peek().is_none() {
            (0.0, 1.0)
        } else {
            values.fold((0.0_f64, 0.0_f64), |(min_value, max_value), value| {
                (min_value.min(value), max_value.max(value))
            })
        }
    };
    let min = axis_min.unwrap_or(computed_min.min(0.0));
    let max = axis_max
        .unwrap_or(computed_max.max(0.0).max(1.0))
        .max(min + 1.0);
    (min, max)
}

fn chart_y_for_value(value: f64, min_value: f64, max_value: f64, plot: ChartPlotArea) -> i32 {
    let value = value.clamp(min_value, max_value);
    let ratio = (value - min_value) / (max_value - min_value);
    plot.bottom - (ratio * ((plot.bottom - plot.top) as f64)) as i32
}

fn chart_x_for_value(value: f64, min_value: f64, max_value: f64, plot: ChartPlotArea) -> i32 {
    let value = value.clamp(min_value, max_value);
    let ratio = (value - min_value) / (max_value - min_value);
    plot.left + (ratio * ((plot.right - plot.left) as f64)) as i32
}

fn chart_axis_ticks(document: &ChartDocument, min_value: f64, max_value: f64) -> Vec<f64> {
    let Some(step) = document
        .v_axis
        .as_ref()
        .and_then(|axis| axis.tick_step)
        .filter(|step| *step > 0.0)
    else {
        return (0..=4)
            .map(|tick| min_value + ((max_value - min_value) * (tick as f64) / 4.0))
            .collect();
    };
    let mut ticks = Vec::new();
    let mut value = (min_value / step).ceil() * step;
    while value <= max_value + 1e-9 && ticks.len() < 64 {
        ticks.push(value);
        value += step;
    }
    if ticks
        .first()
        .is_none_or(|first| (*first - min_value).abs() > 1e-9)
    {
        ticks.insert(0, min_value);
    }
    if ticks
        .last()
        .is_none_or(|last| (*last - max_value).abs() > 1e-9)
    {
        ticks.push(max_value);
    }
    ticks
}

fn chart_axis_color<'a>(axis: Option<&'a crate::model::ChartAxis>, fallback: &'a str) -> &'a str {
    axis.and_then(|axis| axis.color.as_deref())
        .unwrap_or(fallback)
}

fn chart_axis_label_color<'a>(
    axis: Option<&'a crate::model::ChartAxis>,
    fallback: &'a str,
) -> &'a str {
    axis.and_then(|axis| axis.label_color.as_deref())
        .unwrap_or(fallback)
}

fn chart_axis_grid_color<'a>(
    axis: Option<&'a crate::model::ChartAxis>,
    fallback: &'a str,
) -> &'a str {
    axis.and_then(|axis| axis.grid_color.as_deref())
        .unwrap_or(fallback)
}

fn render_chart_axis_metadata(
    out: &mut String,
    name: &str,
    axis: Option<&crate::model::ChartAxis>,
) {
    let Some(axis) = axis else {
        return;
    };
    if let Some(color) = &axis.color {
        out.push_str(&format!(
            "<metadata data-chart-axis-{name}-color=\"{}\"/>",
            escape_text(color)
        ));
    }
    if let Some(color) = &axis.label_color {
        out.push_str(&format!(
            "<metadata data-chart-axis-{name}-text=\"{}\"/>",
            escape_text(color)
        ));
    }
    if let Some(color) = &axis.grid_color {
        out.push_str(&format!(
            "<metadata data-chart-axis-{name}-grid=\"{}\"/>",
            escape_text(color)
        ));
    }
}

fn chart_series_color(
    document: &ChartDocument,
    series: &crate::model::ChartSeries,
    idx: usize,
    first_fallback: &str,
) -> String {
    series.color.clone().unwrap_or_else(|| {
        if let Some(color) = document.palette.get(idx) {
            return color.clone();
        }
        if idx == 0 {
            first_fallback.to_string()
        } else {
            CHART_PALETTE[idx % CHART_PALETTE.len()].to_string()
        }
    })
}

fn chart_slice_color(
    document: &ChartDocument,
    point: &crate::model::ChartPoint,
    idx: usize,
    first_fallback: &str,
) -> String {
    point.color.clone().unwrap_or_else(|| {
        document.palette.get(idx).cloned().unwrap_or_else(|| {
            if idx == 0 {
                first_fallback.to_string()
            } else {
                CHART_PALETTE[idx % CHART_PALETTE.len()].to_string()
            }
        })
    })
}

fn chart_legend_visible(document: &ChartDocument, series: &[crate::model::ChartSeries]) -> bool {
    document.legend.visible
        || (!document.legend.explicit
            && (series.len() > 1
                || (document.subtype == ChartSubtype::Pie && document.data.len() > 1)))
}

fn chart_legend_position(document: &ChartDocument) -> &'static str {
    match (document.legend.v_align, document.legend.h_align) {
        (crate::model::LegendVAlign::Top, crate::model::LegendHAlign::Left) => "top-left",
        (crate::model::LegendVAlign::Top, crate::model::LegendHAlign::Center) => "top",
        (crate::model::LegendVAlign::Top, crate::model::LegendHAlign::Right) => "right",
        (crate::model::LegendVAlign::Bottom, crate::model::LegendHAlign::Left) => "bottom-left",
        (crate::model::LegendVAlign::Bottom, crate::model::LegendHAlign::Center) => "bottom",
        (crate::model::LegendVAlign::Bottom, crate::model::LegendHAlign::Right) => "bottom-right",
    }
}

fn chart_subtype_name(subtype: ChartSubtype) -> &'static str {
    match subtype {
        ChartSubtype::Bar => "bar",
        ChartSubtype::Line => "line",
        ChartSubtype::Pie => "pie",
    }
}

fn format_chart_value(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{}", v as i64)
    } else {
        format!("{:.2}", v)
    }
}

fn format_chart_percent(value: f64, total: f64) -> String {
    if total <= 0.0 {
        return "0%".to_string();
    }
    let pct = value.max(0.0) / total * 100.0;
    if (pct - pct.round()).abs() < 1e-9 {
        format!("{}%", pct as i64)
    } else {
        format!("{pct:.1}%")
    }
}

fn chart_label_mode_name(mode: ChartLabelMode) -> &'static str {
    match mode {
        ChartLabelMode::Auto => "auto",
        ChartLabelMode::Inside => "inside",
        ChartLabelMode::Outside => "outside",
        ChartLabelMode::None => "none",
        ChartLabelMode::Value => "value",
        ChartLabelMode::Percent => "percent",
    }
}

fn chart_pie_label_text(
    mode: ChartLabelMode,
    point: &crate::model::ChartPoint,
    value: f64,
    total: f64,
) -> String {
    match mode {
        ChartLabelMode::Value => format!("{} {}", point.label, format_chart_value(point.value)),
        ChartLabelMode::Percent => {
            format!("{} {}", point.label, format_chart_percent(value, total))
        }
        ChartLabelMode::None => String::new(),
        ChartLabelMode::Auto | ChartLabelMode::Inside | ChartLabelMode::Outside => {
            format!("{} {}", point.label, format_chart_percent(value, total))
        }
    }
}

fn chart_legend_h_name(value: LegendHAlign) -> &'static str {
    match value {
        LegendHAlign::Left => "left",
        LegendHAlign::Center => "center",
        LegendHAlign::Right => "right",
    }
}

fn chart_legend_v_name(value: LegendVAlign) -> &'static str {
    match value {
        LegendVAlign::Top => "top",
        LegendVAlign::Bottom => "bottom",
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

fn state_node_kind_name(kind: &StateNodeKind) -> &'static str {
    match kind {
        StateNodeKind::Normal => "normal",
        StateNodeKind::StartEnd => "start-end",
        StateNodeKind::HistoryShallow => "history-shallow",
        StateNodeKind::HistoryDeep => "history-deep",
        StateNodeKind::Fork => "fork",
        StateNodeKind::Join => "join",
        StateNodeKind::Choice => "choice",
        StateNodeKind::End => "end",
    }
}

fn state_dash_attr(dashed: bool) -> &'static str {
    if dashed {
        " stroke-dasharray=\"5 3\""
    } else {
        ""
    }
}

fn state_hidden_attr(hidden: bool) -> &'static str {
    if hidden {
        " visibility=\"hidden\""
    } else {
        ""
    }
}

fn state_direction_attr(direction: Option<&str>) -> String {
    direction
        .map(|direction| format!(" data-state-direction=\"{}\"", escape_text(direction)))
        .unwrap_or_default()
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
    out.push_str(&format!(
        "<metadata data-state-node=\"{}\" data-state-kind=\"{}\"{} />",
        escape_text(&node.name),
        state_node_kind_name(&node.kind),
        node.stereotype
            .as_deref()
            .map(|stereotype| format!(" data-state-stereotype=\"{}\"", escape_text(stereotype)))
            .unwrap_or_default()
    ));

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
                cx, cy, state_style.font_color, escape_text(label)
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
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\" text-anchor=\"middle\">{}</text>",
                x + w / 2, y + base_h / 2 + 18, state_style.font_color, escape_text(label)
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
                    "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" font-weight=\"600\" text-anchor=\"middle\" fill=\"{}\">{}</text>",
                    x + total_w / 2, y + 16, state_style.font_color, escape_text(display)
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
                    "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" text-anchor=\"middle\" fill=\"{}\">{}</text>",
                    x + w / 2, y + 24, state_style.font_color, escape_text(display)
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
                            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" font-style=\"italic\" fill=\"{}\">{}</text>",
                            x + 6, ay + 10, state_style.font_color, escape_text(&text)
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

fn family_node_fill<'a>(node: &'a crate::model::FamilyNode, fallback: &'a str) -> &'a str {
    node.fill_color.as_deref().unwrap_or(fallback)
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
    let mindmap_leaves = (0..n)
        .filter(|&idx| family_tree_child_indices(nodes, idx).is_empty())
        .count();
    let root_cx = MARGIN + left_w + root_w / 2;
    let root_cy = canvas_h / 2;

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\" data-mindmap-orientation=\"{orientation}\" data-mindmap-node-count=\"{node_count}\" data-mindmap-leaf-count=\"{leaf_count}\" data-mindmap-max-depth=\"{max_depth}\">",
        w = canvas_w,
        h = canvas_h,
        orientation = wbs_orientation_attr(doc.orientation),
        node_count = n,
        leaf_count = mindmap_leaves,
        max_depth = max_right_depth.max(max_left_depth)
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
        "<rect class=\"mindmap-node mindmap-root mindmap-branch\" data-mindmap-depth=\"0\" data-mindmap-child-count=\"{child_count}\" data-mindmap-fill=\"{fill}\" x=\"{rx}\" y=\"{ry}\" width=\"{rw}\" height=\"{h}\" rx=\"17\" ry=\"17\" fill=\"{fill}\" stroke=\"#92400e\" stroke-width=\"1.5\"/>",
        rx = rx, ry = ry, rw = root_w, h = NODE_H,
        child_count = family_tree_child_indices(nodes, 0).len(),
        fill = escape_text(family_node_fill(&nodes[0], mindmap_node_fill(0)))
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

fn family_tree_child_indices(nodes: &[crate::model::FamilyNode], idx: usize) -> Vec<usize> {
    let depth = nodes[idx].depth;
    (idx + 1..nodes.len())
        .take_while(|&j| nodes[j].depth > depth)
        .filter(|&j| nodes[j].depth == depth + 1)
        .collect()
}

fn node_sibling_index(nodes: &[crate::model::FamilyNode], idx: usize) -> usize {
    if idx == 0 {
        return 0;
    }
    let depth = nodes[idx].depth;
    let mut count = 0usize;
    for prev in (0..idx).rev() {
        if nodes[prev].depth < depth {
            break;
        }
        if nodes[prev].depth == depth {
            count += 1;
        }
    }
    count
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
        "<line class=\"mindmap-edge\" data-mindmap-side=\"{side}\" x1=\"{px}\" y1=\"{py}\" x2=\"{ax}\" y2=\"{ny}\" stroke=\"#94a3b8\" stroke-width=\"1.5\"/>",
        side = if is_left { "left" } else { "right" },
        px = parent_attach_x,
        py = parent_attach_y,
        ax = node_attach_x,
        ny = ny
    ));

    let children = family_tree_child_indices(nodes, idx);
    let child_count = children.len();
    let sibling_index = node_sibling_index(nodes, idx);
    let branch_class = if child_count == 0 {
        "mindmap-leaf"
    } else {
        "mindmap-branch"
    };

    // Node rectangle (rounded, pastel by depth)
    out.push_str(&format!(
        "<rect class=\"mindmap-node mindmap-depth-{depth} {branch_class}\" data-mindmap-depth=\"{depth}\" data-mindmap-side=\"{side}\" data-mindmap-child-count=\"{child_count}\" data-mindmap-sibling-index=\"{sibling_index}\" data-mindmap-fill=\"{fill}\" x=\"{nx}\" y=\"{ny_top}\" width=\"{nw}\" height=\"{nh}\" rx=\"14\" ry=\"14\" fill=\"{fill}\" stroke=\"#64748b\" stroke-width=\"1\"/>",
        depth = node.depth,
        branch_class = branch_class,
        side = if is_left { "left" } else { "right" },
        child_count = child_count,
        sibling_index = sibling_index,
        nx = nx, ny_top = ny_top, nw = nw, nh = node_h,
        fill = escape_text(family_node_fill(node, mindmap_node_fill(node.depth)))
    ));
    out.push_str(&format!(
        "<text x=\"{cx}\" y=\"{cy}\" text-anchor=\"middle\" dominant-baseline=\"middle\" font-family=\"monospace\" font-size=\"12\">{}</text>",
        escape_text(&node.name),
        cx = nx + nw / 2,
        cy = ny
    ));

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
            out.push_str(&format!(
                "<line class=\"wbs-edge\" data-wbs-edge-depth=\"{depth}\" x1=\"{px}\" y1=\"{py}\" x2=\"{cx}\" y2=\"{cy}\" stroke=\"#94a3b8\" stroke-width=\"1.5\"/>",
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
            fill = escape_text(fill),
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

fn wbs_orientation_attr(orientation: FamilyOrientation) -> &'static str {
    match orientation {
        FamilyOrientation::TopToBottom => "top-to-bottom",
        FamilyOrientation::LeftToRight => "left-to-right",
        FamilyOrientation::BottomToTop => "bottom-to-top",
        FamilyOrientation::RightToLeft => "right-to-left",
    }
}
