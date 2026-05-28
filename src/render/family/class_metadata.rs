use crate::model::MetadataHAlign;
use crate::render::svg::escape_text;

/// Render a `header`, `footer`, or `caption` label for class/family diagrams.
///
/// `y_top` is the baseline of the first line of text.  `svg_width` is used to
/// compute the x position for centered / right-aligned variants.  Each text line
/// is spaced 16px apart (12px monospace font).
///
/// PlantUML renders these labels in a small italic font outside the diagram area.
/// We emit them as `<g class="uml-{role}">` blocks so CSS or tests can target them.
pub(super) fn render_family_metadata_label(
    out: &mut String,
    text: &str,
    role: &str,
    align: MetadataHAlign,
    y_top: i32,
    svg_width: i32,
    extra_attrs: &str,
) {
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        return;
    }
    out.push_str(&format!("<g class=\"uml-{role}\">"));
    for (idx, line) in lines.iter().enumerate() {
        let line_width = crate::render::text_metrics::monospace_width(line, 7).max(1);
        let x = match align {
            MetadataHAlign::Left => 8,
            MetadataHAlign::Center => ((svg_width - line_width) / 2).max(8),
            MetadataHAlign::Right => (svg_width - line_width - 8).max(8),
        };
        let y = y_top + (idx as i32) * 16;
        out.push_str(&format!(
            "<text x=\"{x}\" y=\"{y}\" font-family=\"monospace\" font-size=\"12\" {extra}>{text}</text>",
            extra = extra_attrs,
            text = escape_text(line)
        ));
    }
    out.push_str("</g>");
}

/// Compute the height (in pixels) consumed by a multi-line metadata label block.
/// Returns `0` if `text` is `None`.
pub(super) fn family_metadata_label_height(text: Option<&str>) -> i32 {
    match text {
        None => 0,
        Some(t) => {
            let count = t.lines().count().max(1) as i32;
            count * 16 + 8
        }
    }
}

/// Render a `legend ... end legend` block at the corner indicated by
/// `halign`/`valign`. Mirrors the sequence diagram's legend rendering so
/// PlantUML `legend left|right|top|bottom` placement is honored for class,
/// object, and usecase diagrams. Determinism: output is a function of
/// (text, dimensions, alignment) only — no hash / map iteration.
pub(super) fn render_family_legend_box(
    out: &mut String,
    text: &str,
    svg_width: i32,
    svg_height: i32,
    halign: crate::model::LegendHAlign,
    valign: crate::model::LegendVAlign,
) {
    use crate::model::{LegendHAlign, LegendVAlign};
    let lines: Vec<&str> = text.lines().collect();
    let line_count = lines.len().max(1) as i32;
    let max_line_width = lines
        .iter()
        .map(|line| crate::render::text_metrics::monospace_width(line, 7))
        .max()
        .unwrap_or(0);
    let box_width = (max_line_width + 16).max(120);
    let box_height = 18 + line_count * 16;
    let margin: i32 = 10;

    let x = match halign {
        LegendHAlign::Left => margin,
        LegendHAlign::Center => (svg_width - box_width) / 2,
        // PlantUML default placement is bottom-right when no halign is given,
        // and `right` here means the same.
        LegendHAlign::Right => svg_width - box_width - margin,
    };
    let y = match valign {
        LegendVAlign::Top => margin,
        // Default valign (Bottom) places the legend near the bottom edge.
        LegendVAlign::Bottom => svg_height - box_height - margin,
    };

    out.push_str(&format!(
        "<rect class=\"uml-legend\" x=\"{x}\" y=\"{y}\" width=\"{box_width}\" height=\"{box_height}\" rx=\"4\" ry=\"4\" fill=\"#fffff0\" stroke=\"#aaaaaa\" stroke-width=\"1\" opacity=\"0.9\"/>",
    ));

    let mut ty = y + 14;
    for line in &lines {
        out.push_str(&format!(
            "<text x=\"{tx}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"11\" fill=\"#333333\">{text}</text>",
            tx = x + 8,
            text = escape_text(line)
        ));
        ty += 16;
    }
}
