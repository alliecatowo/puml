use super::super::svg::{creole_text, escape_text};
use crate::model::{LegendHAlign, LegendVAlign};
use crate::scene::{Label, Scene};

pub(super) fn render_mainframe(out: &mut String, title: &str, scene: &Scene) {
    const INSET: i32 = 4;
    const NOTCH_H: i32 = 20;
    const NOTCH_CUT: i32 = 6;
    let char_w = 7_i32;
    let notch_w = (title.chars().count() as i32 * char_w + 16).clamp(32, scene.width - 2 * INSET);

    let x = INSET;
    let y = INSET;
    let w = scene.width - 2 * INSET;
    let h = scene.height - 2 * INSET;

    // Outer rectangle.
    out.push_str(&format!(
        "<rect class=\"uml-mainframe\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        x, y, w, h,
        scene.style.arrow_color
    ));
    // Title notch (pentagon at top-left).
    out.push_str(&format!(
        "<polygon points=\"{},{} {},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        x, y,
        x + notch_w, y,
        x + notch_w, y + NOTCH_H - NOTCH_CUT,
        x + notch_w - NOTCH_CUT, y + NOTCH_H,
        x, y + NOTCH_H,
        scene.style.participant_background_color,
        scene.style.arrow_color
    ));
    // Title text.
    out.push_str(&creole_text(
        x + 8,
        y + 14,
        "font-family=\"monospace\" font-size=\"12\" font-weight=\"600\"",
        title,
        &scene.style.arrow_color,
    ));
}

pub(super) fn render_sequence_metadata_label(
    out: &mut String,
    label: &Label,
    class_name: &str,
    attrs: &str,
    color: &str,
    line_gap: i32,
) {
    out.push_str(&format!("<g class=\"{}\">", escape_text(class_name)));
    let anchor = match label.align {
        crate::model::MetadataHAlign::Left => "start",
        crate::model::MetadataHAlign::Center => "middle",
        crate::model::MetadataHAlign::Right => "end",
    };
    let attrs = if attrs.contains("text-anchor=") {
        attrs.to_string()
    } else {
        format!("{attrs} text-anchor=\"{anchor}\"")
    };
    for (idx, line) in label.lines.iter().enumerate() {
        out.push_str(&creole_text(
            label.x,
            label.y + (idx as i32 * line_gap),
            &attrs,
            line,
            color,
        ));
    }
    out.push_str("</g>");
}

pub(super) fn render_legend(out: &mut String, text: &str, scene: &Scene) {
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
