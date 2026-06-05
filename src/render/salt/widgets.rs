mod advanced;
mod controls;
mod table;

use super::model::SaltCellRender;
use super::style::SaltRenderStyle;

#[derive(Debug, Clone, Copy)]
pub(super) struct SaltCellBox {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) w: i32,
    pub(super) h: i32,
}

/// Render a single salt cell into SVG, appending to `out`.
pub(super) fn render_salt_cell_svg(
    out: &mut String,
    cell: &SaltCellRender,
    cell_box: SaltCellBox,
    colspan: usize,
    style: &SaltRenderStyle,
) {
    let SaltCellBox { w, .. } = cell_box;
    if colspan > 1 {
        out.push_str(&format!(
            "<g data-salt-widget=\"table-span\" data-salt-colspan=\"{}\" data-salt-span-width=\"{}\">",
            colspan, w
        ));
    }

    if !table::render_tableish(out, cell, cell_box, style) {
        render_control_or_advanced(out, cell, cell_box, style);
    }

    if colspan > 1 {
        out.push_str("</g>");
    }
}

fn render_control_or_advanced(
    out: &mut String,
    cell: &SaltCellRender,
    cell_box: SaltCellBox,
    style: &SaltRenderStyle,
) {
    match cell {
        SaltCellRender::Input(placeholder) => {
            controls::render_input(out, cell_box, placeholder, style);
        }
        SaltCellRender::Button(label) => controls::render_button(out, cell_box, label, style),
        SaltCellRender::Combo(label) => controls::render_combo(out, cell_box, label, style),
        SaltCellRender::CheckboxChecked(label) => {
            controls::render_checkbox(out, cell_box, label, true, style);
        }
        SaltCellRender::CheckboxUnchecked(label) => {
            controls::render_checkbox(out, cell_box, label, false, style);
        }
        SaltCellRender::RadioOn(label) => {
            controls::render_radio(out, cell_box, label, true, style);
        }
        SaltCellRender::RadioOff(label) => {
            controls::render_radio(out, cell_box, label, false, style);
        }
        SaltCellRender::TreeItem { depth, label } => {
            advanced::render_tree_item(out, cell_box, *depth, label, style);
        }
        SaltCellRender::TextAreaLine {
            text,
            scroll_vertical,
            scroll_horizontal,
        } => advanced::render_text_area(
            out,
            cell_box,
            text,
            *scroll_vertical,
            *scroll_horizontal,
            style,
        ),
        SaltCellRender::GroupBox(label) => {
            advanced::render_group_box(out, cell_box, label, style);
        }
        SaltCellRender::MenuBar(items) => {
            advanced::render_menu_bar(out, cell_box, items, style);
        }
        SaltCellRender::TabBar { tabs, active } => {
            advanced::render_tab_bar(out, cell_box, tabs, *active, style);
        }
        SaltCellRender::ScrollBar { vertical, percent } => {
            advanced::render_scroll_bar(out, cell_box, *vertical, *percent, style);
        }
        SaltCellRender::OpenCombo { label, items } => {
            advanced::render_open_combo(out, cell_box, label, items, style);
        }
        SaltCellRender::ProgressBar { fill_ratio } => {
            advanced::render_progress_bar(out, cell_box, *fill_ratio, style);
        }
        SaltCellRender::Password(label) => {
            advanced::render_password(out, cell_box, label, style);
        }
        SaltCellRender::Slider { min, max, value } => {
            advanced::render_slider(out, cell_box, *min, *max, *value, style);
        }
        SaltCellRender::Label(_)
        | SaltCellRender::Header(_)
        | SaltCellRender::TableEmpty
        | SaltCellRender::TableSpan
        | SaltCellRender::SpriteDef(_)
        | SaltCellRender::SpriteRef(_) => {}
    }
}
