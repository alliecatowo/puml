use super::model::TimingLayout;
use super::*;

pub(super) fn render_timing_svg_header(
    doc: &FamilyDocument,
    style: &crate::theme::TimingStyle,
    layout: &TimingLayout,
) -> String {
    let mut out = String::new();
    let width = layout.width;
    let height = layout.height;
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

    let mut ty = 14i32;
    if let Some(header) = &doc.header {
        for line in header.lines() {
            out.push_str(&format!(
                "<text class=\"timing-header\" x=\"{x}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\" text-anchor=\"middle\">{}</text>",
                escape_text(&style.font_color),
                escape_text(line),
                x = width / 2
            ));
            ty += 16;
        }
        ty += 4;
    }

    ty += 8;
    if let Some(title) = &doc.title {
        // #1543: centre the title text so long titles don't clip at the right
        // canvas edge.  `x=width/2` + `text-anchor="middle"` mirrors what the
        // other diagram families do for their title elements.
        for line in title.lines() {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\" fill=\"{}\">{}</text>",
                layout.width / 2,
                escape_text(&style.font_color),
                escape_text(line)
            ));
            ty += 22;
        }
    }
    // Kind-tag suppression (#1372): PlantUML does not render "timing diagram"
    // as an auto-subtitle, so we suppress it.
    out
}

pub(super) fn render_timing_footer_caption(
    out: &mut String,
    doc: &FamilyDocument,
    style: &crate::theme::TimingStyle,
    layout: &TimingLayout,
) {
    let mut bottom_y = layout.signals_top + layout.rows_h() + 20;
    if let Some(caption) = &doc.caption {
        for line in caption.lines() {
            out.push_str(&format!(
                "<text class=\"timing-caption\" x=\"{x}\" y=\"{bottom_y}\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\" text-anchor=\"middle\" font-style=\"italic\">{}</text>",
                escape_text(&style.font_color),
                escape_text(line),
                x = layout.width / 2
            ));
            bottom_y += 16;
        }
        bottom_y += 4;
    }
    if let Some(footer) = &doc.footer {
        for line in footer.lines() {
            out.push_str(&format!(
                "<text class=\"timing-footer\" x=\"{x}\" y=\"{bottom_y}\" font-family=\"monospace\" font-size=\"11\" fill=\"#94a3b8\" text-anchor=\"middle\">{}</text>",
                escape_text(line),
                x = layout.width / 2
            ));
            bottom_y += 16;
        }
    }
}
