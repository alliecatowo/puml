mod support;

use super::scene_graph::{estimate_text_bbox, Rect as SceneRect};
use super::svg::{creole_text, escape_text, render_actor_stick_figure};
use crate::ast::NoteKind;
use crate::model::{LegendHAlign, LegendVAlign, ParticipantRole, ScaleSpec, VirtualEndpointKind};
use crate::scene::{LifecycleMarkerKind, NoteBox, ParticipantBox, Scene, StructureKind};
use crate::theme::{css3_color_to_hex, MessageAlign};
use std::collections::BTreeMap;
use support::*;

const MESSAGE_LABEL_LINE_GAP: i32 = 16;
const REF_HEADER_HEIGHT: i32 = 20;
const REF_BODY_BASELINE_Y: i32 = 32;

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

    // Embed SVG filter definitions.
    let need_defs = scene.style.shadowing || scene.style.hand_drawn;
    if need_defs {
        out.push_str("<defs>");
        if scene.style.shadowing {
            out.push_str(
                "<filter id=\"shadow\" x=\"-10%\" y=\"-10%\" width=\"130%\" height=\"130%\">\
                 <feDropShadow dx=\"3\" dy=\"3\" stdDeviation=\"2\" flood-color=\"#00000040\"/>\
                 </filter>",
            );
        }
        if scene.style.hand_drawn {
            // feTurbulence generates organic noise; feDisplacementMap uses it to
            // wobble the geometry so straight lines appear hand-drawn.  The scale
            // value (2.5) is intentionally subtle — enough to read as "sketchy"
            // at normal sizes without making arrowheads unreadable.
            out.push_str(
                "<filter id=\"sketch\" x=\"-5%\" y=\"-5%\" width=\"110%\" height=\"110%\">\
                 <feTurbulence type=\"fractalNoise\" baseFrequency=\"0.04\" numOctaves=\"3\" seed=\"2\" result=\"noise\"/>\
                 <feDisplacementMap in=\"SourceGraphic\" in2=\"noise\" scale=\"2.5\" xChannelSelector=\"R\" yChannelSelector=\"G\"/>\
                 </filter>",
            );
        }
        out.push_str("</defs>");
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
            let attrs = sequence_label_attrs(
                "diagram",
                "title",
                title.x,
                title.y + (idx as i32 * 24),
                line,
                18,
                false,
            );
            out.push_str(&creole_text(
                title.x,
                title.y + (idx as i32 * 24),
                &format!(
                    "class=\"puml-label\" {attrs} font-family=\"monospace\" font-size=\"18\" font-weight=\"600\""
                ),
                line,
                &scene.style.arrow_color,
            ));
        }
    }

    for g in &scene.groups {
        if g.kind.eq_ignore_ascii_case("box") {
            render_participant_group_box(&mut out, g, scene);
        }
    }

    for p in &scene.participants {
        render_participant_box(&mut out, p, scene);
    }

    // Wrap lifelines in the sketch filter group when hand-drawn theme is active.
    if scene.style.hand_drawn {
        out.push_str("<g filter=\"url(#sketch)\">");
    }
    let lifeline_stroke_width = scene.style.lifeline_thickness.unwrap_or(1);
    for l in &scene.lifelines {
        let edge_id = format!("lifeline:{}", l.participant_id);
        let puml_attrs = super::puml_edge_attrs(
            &edge_id,
            "sequence",
            "lifeline",
            &l.participant_id,
            &l.participant_id,
        );
        out.push_str(&format!(
            "<line class=\"sequence-lifeline puml-edge\" data-participant=\"{}\" {} x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\" stroke-dasharray=\"6 4\"/>",
            escape_text(&l.participant_id),
            puml_attrs,
            l.x,
            l.y1,
            l.x,
            l.y2,
            scene.style.lifeline_border_color,
            lifeline_stroke_width
        ));
    }
    if scene.style.hand_drawn {
        out.push_str("</g>");
    }

    for a in &scene.activations {
        let offset = (a.depth as i32) * 6;
        let x = a.x + offset - 5;
        let y = a.y1.min(a.y2);
        let height = (a.y2 - a.y1).abs().max(12);
        let node_id = format!("activation:{}:{}:{}", a.participant_id, y, a.depth);
        let puml_attrs = super::puml_node_attrs(
            &node_id,
            "sequence",
            "activation",
            geometry_bbox(x, y, 10, height),
        );
        out.push_str(&format!(
            "<rect class=\"sequence-activation\" data-participant=\"{}\" x=\"{}\" y=\"{}\" width=\"10\" height=\"{}\" fill=\"#ffffff\" stroke=\"{}\" stroke-width=\"1\"/>",
            escape_text(&a.participant_id),
            x,
            y,
            height,
            scene.style.lifeline_border_color
        ));
        out.push_str(&format!(
            "<rect class=\"puml-node\" data-participant=\"{}\" {} x=\"{}\" y=\"{}\" width=\"10\" height=\"{}\" fill=\"none\" stroke=\"none\" pointer-events=\"none\"/>",
            escape_text(&a.participant_id),
            puml_attrs,
            x,
            y,
            height
        ));
    }

    for g in &scene.groups {
        if g.kind.eq_ignore_ascii_case("box") {
            continue;
        }
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
                .unwrap_or(scene.style.arrow_color.as_str());
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
                let notch_w = 32_i32.min(g.width.saturating_sub(4)).max(24);
                let cut = 6_i32.min(REF_HEADER_HEIGHT.saturating_sub(2));
                out.push_str(&format!(
                    "<polygon points=\"{},{} {},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                    g.x, g.y,
                    g.x + notch_w, g.y,
                    g.x + notch_w, g.y + REF_HEADER_HEIGHT - cut,
                    g.x + notch_w - cut, g.y + REF_HEADER_HEIGHT,
                    g.x, g.y + REF_HEADER_HEIGHT,
                    group_fill, group_border
                ));
                out.push_str(&creole_text(
                    g.x + 6,
                    g.y + 14,
                    &format!(
                        "class=\"puml-label\" {} font-family=\"monospace\" font-size=\"11\" {header_font_weight}{header_font_style_attr}",
                        sequence_label_attrs("group:ref", "group-header", g.x + 6, g.y + 14, "ref", 11, false)
                    ),
                    "ref",
                    header_font_color,
                ));
                if let Some(label) = &g.label {
                    let mut y = g.y + REF_BODY_BASELINE_Y;
                    for line in label.lines() {
                        let attrs = sequence_label_attrs(
                            "group:ref",
                            "group-label",
                            g.x + 8,
                            y,
                            line,
                            12,
                            false,
                        );
                        out.push_str(&creole_text(
                            g.x + 8,
                            y,
                            &format!("class=\"puml-label\" {attrs} font-family=\"monospace\" font-size=\"12\""),
                            line,
                            &scene.style.arrow_color,
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
                    &format!(
                        "class=\"puml-label\" {} font-family=\"monospace\" font-size=\"12\" {header_font_weight}{header_font_style_attr}",
                        sequence_label_attrs("group", "group-header", g.x + 8, g.y + 14, &header_text, 12, false)
                    ),
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
                    &format!(
                        "class=\"puml-label\" {} font-family=\"monospace\" font-size=\"12\" {header_font_weight}{header_font_style_attr}",
                        sequence_label_attrs("group", "group-header", g.x + 8, g.y + 14, &g.kind, 12, false)
                    ),
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
                let attrs = sequence_label_attrs(
                    "group-separator",
                    "separator-label",
                    g.x + 8,
                    sep.y - 14,
                    label,
                    11,
                    false,
                );
                out.push_str(&creole_text(
                    g.x + 8,
                    sep.y - 14,
                    &format!("class=\"puml-label\" {attrs} font-family=\"monospace\" font-size=\"11\" fill=\"#333\""),
                    label,
                    "#333",
                ));
            }
        }
    }

    // When the hand-drawn theme is active, apply the sketch displacement filter
    // to arrow shafts and heads (but not to label text).
    let sketch_attr = if scene.style.hand_drawn {
        " filter=\"url(#sketch)\""
    } else {
        ""
    };

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
        // Open sketch group around shaft + arrowheads (not labels) when hand-drawn.
        if !sketch_attr.is_empty() {
            out.push_str(&format!("<g{}>", sketch_attr));
        }
        if is_self_loop {
            // Use m.x2 as the right extent set by the layout.
            let loop_x2 = m.x2;
            // Use a tall-enough loop height so the shape is clearly a self-loop
            // and not a squished rectangle.  32 px gives ample visual depth
            // while still fitting inside the allocated message row.
            let loop_h = 32;
            let head_base_x = m.x1 + 8;
            // Three-segment UML self-call: right → down → back-left to lifeline.
            // Stop the bottom segment at the arrowhead base so the head remains
            // visually distinct instead of collapsing into an open rectangle.
            out.push_str(&format!(
                "<path{} d=\"M {} {} L {} {} L {} {} L {} {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}/>",
                style_attrs,
                m.x1, line_y,
                loop_x2, line_y,
                loop_x2, line_y + loop_h,
                head_base_x, line_y + loop_h,
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
            if open_head {
                out.push_str(&format!(
                    "<polyline points=\"{},{} {},{} {},{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
                    head_base_x, tip_y - 5, tip_x, tip_y, head_base_x, tip_y + 5,
                    stroke_color, head_stroke_width, hidden
                ));
            } else {
                out.push_str(&format!(
                    "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
                    tip_x, tip_y, head_base_x, tip_y - 5, head_base_x, tip_y + 5,
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
        render_sequence_message_edge_hook(&mut out, m, is_self_loop);
        // Close sketch group (shaft + arrowheads only; labels are outside).
        if !sketch_attr.is_empty() {
            out.push_str("</g>");
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
                let label_y = start_y + (idx as i32 * MESSAGE_LABEL_LINE_GAP);
                let attrs = sequence_label_attrs(
                    &sequence_message_id(m),
                    "message-label",
                    tx,
                    label_y,
                    line,
                    12,
                    anchor == "middle",
                );
                out.push_str(&creole_text(
                    tx,
                    label_y,
                    &format!(
                        "class=\"puml-label\" {attrs} text-anchor=\"{anchor}\" font-family=\"monospace\" font-size=\"12\""
                    ),
                    line,
                    &scene.style.arrow_color,
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
            let attrs = sequence_label_attrs(
                &sequence_message_id(m),
                "message-label",
                tx,
                ty,
                label,
                12,
                anchor == "middle",
            );
            out.push_str(&creole_text(
                tx,
                ty,
                &format!(
                    "class=\"puml-label\" {attrs} text-anchor=\"{anchor}\" font-family=\"monospace\" font-size=\"12\""
                ),
                label,
                &scene.style.arrow_color,
            ));
        }
    }

    for (idx, n) in scene.notes.iter().enumerate() {
        let note_id = sequence_note_id(idx, n);
        render_sequence_note_shape(&mut out, n, scene, &note_id);

        let mut text_y = n.y + 20;
        for line in n.text.lines() {
            let attrs =
                sequence_label_attrs(&note_id, "note-label", n.x + 8, text_y, line, 12, false);
            out.push_str(&creole_text(
                n.x + 8,
                text_y,
                &format!("class=\"puml-label\" {attrs} font-family=\"monospace\" font-size=\"12\""),
                line,
                &scene.style.arrow_color,
            ));
            text_y += 16;
        }
    }

    for marker in &scene.lifecycle_markers {
        match marker.kind {
            LifecycleMarkerKind::Create => {
                let marker_id = format!("create:{}", marker.participant_id);
                let puml_attrs = super::puml_node_attrs(
                    &marker_id,
                    "sequence",
                    "lifecycle-create",
                    geometry_bbox(marker.x - 5, marker.y - 5, 10, 10),
                );
                out.push_str(&format!(
                    "<circle class=\"sequence-create\" data-participant=\"{}\" cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"#dcfce7\" stroke=\"#15803d\" stroke-width=\"1.5\"/>",
                    escape_text(&marker.participant_id),
                    marker.x,
                    marker.y
                ));
                out.push_str(&format!(
                    "<circle class=\"puml-node\" data-participant=\"{}\" {} cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"none\" stroke=\"none\" pointer-events=\"none\"/>",
                    escape_text(&marker.participant_id),
                    puml_attrs,
                    marker.x,
                    marker.y
                ));
            }
            LifecycleMarkerKind::Destroy => {
                let marker_id = format!("destroy:{}", marker.participant_id);
                let puml_attrs = super::puml_node_attrs(
                    &marker_id,
                    "sequence",
                    "lifecycle-destroy",
                    geometry_bbox(marker.x - 6, marker.y - 6, 12, 12),
                );
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
                out.push_str(&format!(
                    "<rect class=\"puml-node\" data-participant=\"{}\" {} x=\"{}\" y=\"{}\" width=\"12\" height=\"12\" fill=\"none\" stroke=\"none\" pointer-events=\"none\"/>",
                    escape_text(&marker.participant_id),
                    puml_attrs,
                    marker.x - 6,
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
                    let tx = (s.x1 + s.x2) / 2;
                    let attrs = sequence_label_attrs(
                        "delay",
                        "structure-label",
                        tx,
                        s.y - 6,
                        label,
                        11,
                        true,
                    );
                    out.push_str(&creole_text(
                        tx,
                        s.y - 6,
                        &format!(
                            "class=\"puml-label\" {attrs} text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#444\""
                        ),
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
                    let tx = (s.x1 + s.x2) / 2;
                    let attrs = sequence_label_attrs(
                        "divider",
                        "structure-label",
                        tx,
                        s.y - 6,
                        label,
                        11,
                        true,
                    );
                    out.push_str(&creole_text(
                        tx,
                        s.y - 6,
                        &format!(
                            "class=\"puml-label\" {attrs} text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#333\""
                        ),
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
                let tx = (s.x1 + s.x2) / 2;
                let attrs = sequence_label_attrs(
                    "separator",
                    "structure-label",
                    tx,
                    s.y - 6,
                    &label,
                    11,
                    true,
                );
                out.push_str(&creole_text(
                    tx,
                    s.y - 6,
                    &format!(
                        "class=\"puml-label\" {attrs} text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" font-weight=\"600\" fill=\"#222\""
                    ),
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
