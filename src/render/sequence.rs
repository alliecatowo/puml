use super::svg::{creole_text, escape_text};
use crate::ast::NoteKind;
use crate::model::{LegendHAlign, LegendVAlign, ParticipantRole, ScaleSpec, VirtualEndpointKind};
use crate::scene::{LifecycleMarkerKind, ParticipantBox, Scene, StructureKind};
use crate::theme::{css3_color_to_hex, MessageAlign};
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

        {
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
            if is_ref {
                // ref box: "ref" appears alone in a small pentagon notch at the
                // top-left; the participant list and body text go in the body.
                let notch_w = 32_i32;
                let notch_h = 20_i32;
                let cut = 6_i32;
                out.push_str(&format!(
                    "<polygon points=\"{},{} {},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                    g.x, g.y,
                    g.x + notch_w, g.y,
                    g.x + notch_w, g.y + notch_h - cut,
                    g.x + notch_w - cut, g.y + notch_h,
                    g.x, g.y + notch_h,
                    group_fill, group_border
                ));
                out.push_str(&creole_text(
                    g.x + 6,
                    g.y + 14,
                    &format!("font-family=\"monospace\" font-size=\"11\" {header_font_weight}{header_font_style_attr}"),
                    "ref",
                    header_font_color,
                ));
                // Body: all label lines starting from the first one.
                if let Some(label) = &g.label {
                    let mut y = g.y + 32;
                    for line in label.lines() {
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
            } else if let Some(label) = &g.label {
                // Combined-fragment header: a small pentagon notch at the top-left
                // showing "<kind> <condition>".
                let first_line = label.lines().next().unwrap_or("");
                let header_text = format!("{} {}", g.kind, first_line).trim().to_string();
                // Estimate the notch width from the text content.
                let char_w = 7_i32;
                let notch_w =
                    (header_text.chars().count() as i32 * char_w + 16).clamp(40, g.width - 4);
                let notch_h = 20_i32;
                let cut = 6_i32;
                out.push_str(&format!(
                    "<polygon points=\"{},{} {},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                    g.x, g.y,
                    g.x + notch_w, g.y,
                    g.x + notch_w, g.y + notch_h - cut,
                    g.x + notch_w - cut, g.y + notch_h,
                    g.x, g.y + notch_h,
                    group_fill, group_border
                ));
                out.push_str(&creole_text(
                    g.x + 8,
                    g.y + 14,
                    &format!("font-family=\"monospace\" font-size=\"12\" {header_font_weight}{header_font_style_attr}"),
                    &header_text,
                    header_font_color,
                ));
            } else {
                // No label — kind only.
                let notch_w = (g.kind.chars().count() as i32 * 7 + 16).clamp(40, g.width - 4);
                let notch_h = 20_i32;
                let cut = 6_i32;
                out.push_str(&format!(
                    "<polygon points=\"{},{} {},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                    g.x, g.y,
                    g.x + notch_w, g.y,
                    g.x + notch_w, g.y + notch_h - cut,
                    g.x + notch_w - cut, g.y + notch_h,
                    g.x, g.y + notch_h,
                    group_fill, group_border
                ));
                out.push_str(&creole_text(
                    g.x + 8,
                    g.y + 14,
                    &format!("font-family=\"monospace\" font-size=\"12\" {header_font_weight}{header_font_style_attr}"),
                    &g.kind,
                    header_font_color,
                ));
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
                // Place the label above the dashed divider so it doesn't
                // overlap the rule.  A baseline of sep.y - 4 sits just above
                // the 1 px stroke and leaves a small gap.
                out.push_str(&creole_text(
                    g.x + 8,
                    sep.y - 14,
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
        let style_attrs = sequence_message_line_style_attrs(m);
        let line_y = m.route_y;
        // A self-loop is detected by from_id == to_id (x1 != x2 because the
        // layout gives the loop a non-zero width).
        let is_self_loop =
            m.from_id == m.to_id && m.from_virtual.is_none() && m.to_virtual.is_none();
        if is_self_loop {
            // Use m.x2 as the right extent set by the layout.
            let loop_x2 = m.x2;
            // Use a tall-enough loop height so the shape is clearly a self-loop
            // and not a squished rectangle.  32 px gives ample visual depth
            // while still fitting inside the allocated message row.
            let loop_h = 32;
            // Three-segment UML self-call: right → down → back-left to lifeline.
            out.push_str(&format!(
                "<path{} d=\"M {} {} L {} {} L {} {} L {} {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}/>",
                style_attrs,
                m.x1, line_y,
                loop_x2, line_y,
                loop_x2, line_y + loop_h,
                m.x1, line_y + loop_h,
                stroke_color,
                stroke_width,
                stroke_dash,
                hidden
            ));
            // Arrowhead at the bottom of the loop, pointing left (from the right).
            let head_stroke_width = m
                .style
                .thickness
                .map(f32::from)
                .unwrap_or(1.0)
                .clamp(1.0, 8.0);
            let open_head = m.arrow.contains(">>") || m.arrow.contains("<<");
            let tip_x = m.x1;
            let tip_y = line_y + loop_h;
            let back_x = tip_x + 8; // back is to the right
            if open_head {
                out.push_str(&format!(
                    "<polyline points=\"{},{} {},{} {},{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
                    back_x, tip_y - 5, tip_x, tip_y, back_x, tip_y + 5,
                    stroke_color, head_stroke_width, hidden
                ));
            } else {
                out.push_str(&format!(
                    "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
                    tip_x, tip_y, back_x, tip_y - 5, back_x, tip_y + 5,
                    arrow_fill, stroke_color, head_stroke_width, hidden
                ));
            }
        } else {
            out.push_str(&format!(
                "<line{} x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}{}/>",
                style_attrs,
                m.x1,
                line_y,
                m.x2,
                line_y,
                stroke_color,
                stroke_width,
                stroke_dash,
                hidden
            ));
            render_sequence_arrow_heads(
                &mut out,
                m,
                stroke_color,
                arrow_fill,
                stroke_width,
                hidden,
            );
        }

        if let Some(virtual_ep) = m.from_virtual {
            render_virtual_endpoint_marker(&mut out, m.x1, line_y, virtual_ep.kind);
        }
        if let Some(virtual_ep) = m.to_virtual {
            render_virtual_endpoint_marker(&mut out, m.x2, line_y, virtual_ep.kind);
        }

        if !m.label_lines.is_empty() {
            let (tx, anchor) = sequence_message_label_anchor(m.x1, m.x2, scene.style.message_align);
            let below =
                scene.style.response_message_below_arrow && is_response_message_arrow(&m.arrow);
            let lane_offset = if m.style.parallel || below {
                let lane = parallel_label_lanes.entry(m.y).or_insert(0);
                let offset = *lane * MESSAGE_LABEL_LINE_GAP;
                *lane += (m.label_lines.len() as i32).max(1);
                offset
            } else {
                0
            };
            let start_y = if m.style.parallel || below {
                line_y + 16 + lane_offset
            } else {
                line_y - 8 - (((m.label_lines.len() as i32) - 1) * MESSAGE_LABEL_LINE_GAP)
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
            let ty = if scene.style.response_message_below_arrow
                && is_response_message_arrow(&m.arrow)
            {
                line_y + 16
            } else {
                line_y - 8
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

fn sequence_message_line_style_attrs(m: &crate::scene::MessageLine) -> String {
    if m.style.color.is_none()
        && !m.style.hidden
        && !m.style.dashed
        && !m.style.dotted
        && m.style.thickness.is_none()
    {
        return String::new();
    }

    let mut classes = Vec::new();
    let mut styles = Vec::new();
    let renders_dashed = m.style.dashed || (!m.style.dotted && m.arrow.contains("--"));
    if m.style.color.is_some() {
        classes.push("sequence-message-line-colored");
        styles.push("color");
    }
    if m.style.dotted {
        classes.push("sequence-message-line-dotted");
        styles.push("dotted");
    } else if renders_dashed {
        classes.push("sequence-message-line-dashed");
        styles.push("dashed");
    }
    if m.style.hidden {
        classes.push("sequence-message-line-hidden");
        styles.push("hidden");
    }
    if m.style.thickness.is_some() {
        classes.push("sequence-message-line-thick");
        styles.push("thickness");
    }
    if classes.is_empty() {
        return String::new();
    }

    format!(
        " class=\"sequence-message-line {}\" data-sequence-message-style=\"{}\"",
        classes.join(" "),
        styles.join(" ")
    )
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

fn is_response_message_arrow(arrow: &str) -> bool {
    arrow.contains("--")
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
                point: (m.x1, m.route_y),
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
                point: (m.x2, m.route_y),
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
        render_arrow_endpoint_marker(
            out,
            m.x1,
            m.route_y,
            marker,
            stroke_color,
            stroke_width,
            hidden,
        );
    }
    if let Some(marker) = right_marker {
        render_arrow_endpoint_marker(
            out,
            m.x2,
            m.route_y,
            marker,
            stroke_color,
            stroke_width,
            hidden,
        );
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
    let line_count = lines.len().max(1) as i32;
    let max_line_width = lines
        .iter()
        .map(|line| (line.chars().count() as i32) * 7)
        .max()
        .unwrap_or(0);
    let box_width = (max_line_width + 16).max(200);
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

fn render_virtual_endpoint_marker(out: &mut String, x: i32, y: i32, kind: VirtualEndpointKind) {
    match kind {
        VirtualEndpointKind::Plain => {
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#111\" stroke-width=\"1.5\"/>",
                x,
                y - 6,
                x,
                y + 6
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
