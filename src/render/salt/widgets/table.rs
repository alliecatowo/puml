use super::SaltCellBox;
use crate::render::escape_text;
use crate::render::salt::model::SaltCellRender;
use crate::render::salt::style::SaltRenderStyle;
use crate::render::salt::text::salt_text;

pub(super) fn render_label(
    out: &mut String,
    cell_box: SaltCellBox,
    text: &str,
    style: &SaltRenderStyle,
) {
    let SaltCellBox { x, y, h, .. } = cell_box;
    let pad = 8;
    salt_text(
        out,
        x + pad,
        y + h / 2 + 4,
        &format!(
            "font-family=\"{}\" font-size=\"12\" fill=\"{}\"",
            style.font_family, style.text_color
        ),
        text,
        &style.text_color,
    );
}

pub(super) fn render_header(
    out: &mut String,
    cell_box: SaltCellBox,
    text: &str,
    style: &SaltRenderStyle,
) {
    let SaltCellBox { x, y, w, h } = cell_box;
    let pad = 8;
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
        y + h / 2 + 4,
        &format!(
            "font-family=\"{}\" font-size=\"12\" font-weight=\"700\" fill=\"{}\"",
            style.font_family, style.header_text_color
        ),
        text,
        &style.header_text_color,
    );
}

pub(super) fn render_table_empty(out: &mut String, cell_box: SaltCellBox, style: &SaltRenderStyle) {
    let SaltCellBox { x, y, w, h } = cell_box;
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

pub(super) fn render_table_span(out: &mut String, cell_box: SaltCellBox, style: &SaltRenderStyle) {
    let SaltCellBox { x, y, w, h } = cell_box;
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
        y + h / 2 + 4,
        style.font_family,
        style.muted_text_color
    ));
}

pub(super) fn render_sprite_def(
    out: &mut String,
    cell_box: SaltCellBox,
    name: &str,
    style: &SaltRenderStyle,
) {
    let SaltCellBox { x, y, w, h } = cell_box;
    let pad = 8;
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
        y + h / 2 + 4,
        style.font_family,
        style.muted_text_color,
        escape_text(name)
    ));
}

pub(super) fn render_sprite_ref(
    out: &mut String,
    cell_box: SaltCellBox,
    name: &str,
    style: &SaltRenderStyle,
) {
    let SaltCellBox { x, y, h, .. } = cell_box;
    let pad = 8;
    out.push_str(&format!(
        "<g data-salt-widget=\"sprite-ref\" data-salt-sprite-ref=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"18\" height=\"18\" fill=\"{}\" stroke=\"{}\"/><text x=\"{}\" y=\"{}\" font-family=\"{}\" font-size=\"11\" fill=\"{}\">{}</text></g>",
        escape_text(name),
        x + pad,
        y + 5,
        style.accent_fill,
        style.border_color,
        x + pad + 24,
        y + h / 2 + 4,
        style.font_family,
        style.text_color,
        escape_text(name)
    ));
}

pub(super) fn render_tableish(
    out: &mut String,
    cell: &SaltCellRender,
    cell_box: SaltCellBox,
    style: &SaltRenderStyle,
) -> bool {
    match cell {
        SaltCellRender::Label(text) => render_label(out, cell_box, text, style),
        SaltCellRender::Header(text) => render_header(out, cell_box, text, style),
        SaltCellRender::TableEmpty => render_table_empty(out, cell_box, style),
        SaltCellRender::TableSpan => render_table_span(out, cell_box, style),
        SaltCellRender::SpriteDef(name) => render_sprite_def(out, cell_box, name, style),
        SaltCellRender::SpriteRef(name) => render_sprite_ref(out, cell_box, name, style),
        _ => return false,
    }
    true
}
