use super::layout_constants::{MESSAGE_LABEL_LINE_GAP, REF_BODY_BASELINE_Y, REF_HEADER_HEIGHT};
use super::svg::{creole_text, escape_text};
use crate::output::RenderArtifact;
use crate::scene::Scene;
use std::collections::BTreeMap;

mod dimensions;
mod lifecycle;
mod messages;
mod metadata;
mod notes;
mod participants;
mod scene;
mod structures;

use dimensions::compute_svg_dimensions;
use lifecycle::render_lifecycle_markers;
use messages::{
    is_response_message_arrow, normalize_message_color, render_sequence_arrow_heads,
    render_virtual_endpoint_marker, sequence_message_label_anchor,
    sequence_message_line_style_attrs,
};
use metadata::{render_legend, render_mainframe, render_sequence_metadata_label};
use notes::render_sequence_note_shape;
use participants::{render_participant_box, render_participant_group_box};
use scene::build_render_scene;
use structures::render_sequence_structures;

pub fn render_artifact(scene: &Scene) -> RenderArtifact {
    let render_scene = build_render_scene(scene);
    let mut artifact = RenderArtifact::with_scene(render_svg(scene), render_scene);
    artifact.validate_svg(super::validate::AutoCorrect::EmitDiagnostic);
    artifact
}

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

    let sepia_attr = if scene.style.sepia {
        " style=\"filter:sepia(1)\""
    } else {
        ""
    };
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"{v}\"{sepia}>",
        w = svg_width,
        h = svg_height,
        v = viewbox,
        sepia = sepia_attr,
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
            out.push_str(&creole_text(
                title.x,
                title.y + (idx as i32 * 24),
                "font-family=\"monospace\" font-size=\"18\" font-weight=\"600\"",
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
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\" stroke-dasharray=\"6 4\"/>",
            l.x, l.y1, l.x, l.y2, scene.style.lifeline_border_color, lifeline_stroke_width
        ));
    }
    if scene.style.hand_drawn {
        out.push_str("</g>");
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
                    &format!("font-family=\"monospace\" font-size=\"11\" {header_font_weight}{header_font_style_attr}"),
                    "ref",
                    header_font_color,
                ));
                if let Some(label) = &g.label {
                    let mut y = g.y + REF_BODY_BASELINE_Y;
                    for line in label.lines() {
                        // The first line of a `ref over A, B : body` label is
                        // the participant spec (`over A, B`).  PlantUML renders
                        // only the body inside the ref box — the participant
                        // names are never shown as text content.
                        let line_lower = line.trim().to_ascii_lowercase();
                        if line_lower.starts_with("over ") || line_lower == "over" {
                            continue;
                        }
                        out.push_str(&creole_text(
                            g.x + 8,
                            y,
                            "font-family=\"monospace\" font-size=\"12\"",
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
                let notch_w =
                    (crate::render_core::text_metrics::estimate_text_width_default(&header_text)
                        + 16)
                        .clamp(40, g.width - 4);
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
                let notch_w = (crate::render::text_metrics::monospace_width(&g.kind, 7) + 16)
                    .clamp(40, g.width - 4);
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

    // Activation bars are drawn after group frames so they appear on top of the
    // combined-fragment border rather than being covered by it.
    // `lifelineStrategy nosolid` suppresses activation boxes on the lifeline.
    if !scene.style.lifeline_nosolid {
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
            // Curved self-message arc (#1319): emit a smooth right-hand loop
            // using two rounded quadratic-bezier corners instead of the old
            // hard 90° polyline.  The lifeline-side anchor is `m.x1`; the loop
            // bulges out to `loop_x2` and returns to the arrowhead just below
            // the start.  PlantUML renders self-messages with rounded corners
            // — `q`-style cubics keep that feel while still ending the arrow
            // exactly on the lifeline so the head tip lands cleanly.
            let corner_r: i32 = 8;
            let top_y = line_y;
            let bot_y = line_y + loop_h;
            let radius = corner_r.min(loop_h / 2).max(2);
            // Path segments:
            //   M x1, top_y                       — start at lifeline
            //   L (loop_x2 - r), top_y            — horizontal out
            //   Q loop_x2, top_y  loop_x2, top_y + r  — top-right rounded corner
            //   L loop_x2, bot_y - r              — vertical down on the right
            //   Q loop_x2, bot_y  (loop_x2 - r), bot_y — bottom-right corner
            //   L head_base_x, bot_y              — horizontal back toward lifeline
            // The result is a clean "D"-shaped arc emerging from the lifeline.
            out.push_str(&format!(
                "<path{} d=\"M {} {} L {} {} Q {} {} {} {} L {} {} Q {} {} {} {} L {} {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}/>",
                style_attrs,
                m.x1, top_y,
                loop_x2 - radius, top_y,
                loop_x2, top_y, loop_x2, top_y + radius,
                loop_x2, bot_y - radius,
                loop_x2, bot_y, loop_x2 - radius, bot_y,
                head_base_x, bot_y,
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
                out.push_str(&creole_text(
                    tx,
                    start_y + (idx as i32 * MESSAGE_LABEL_LINE_GAP),
                    &format!("text-anchor=\"{anchor}\" font-family=\"monospace\" font-size=\"12\""),
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
            out.push_str(&creole_text(
                tx,
                ty,
                &format!("text-anchor=\"{anchor}\" font-family=\"monospace\" font-size=\"12\""),
                label,
                &scene.style.arrow_color,
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
                &scene.style.arrow_color,
            ));
            text_y += 16;
        }
    }

    render_lifecycle_markers(&mut out, scene);

    render_sequence_structures(&mut out, scene);

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

    // Render mainframe border last so it overlays everything (feature 1.43).
    if let Some(mainframe_title) = &scene.mainframe {
        render_mainframe(&mut out, mainframe_title, scene);
    }

    out.push_str("</svg>");
    out
}
