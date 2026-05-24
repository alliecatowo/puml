use super::SaltCellBox;
use crate::render::salt::style::SaltRenderStyle;
use crate::render::salt::text::{salt_button_width, salt_combo_width, salt_input_width, salt_text};

pub(super) fn render_input(
    out: &mut String,
    cell_box: SaltCellBox,
    placeholder: &str,
    style: &SaltRenderStyle,
) {
    let SaltCellBox { x, y, w, h } = cell_box;
    let pad = 8;
    let text_y = y + h / 2 + 4;
    let control_w = salt_input_width(placeholder).min(w - pad * 2).max(24);
    let line_y = y + h - 4;
    out.push_str(&format!(
        "<g data-salt-widget=\"input\" fill=\"{}\">",
        style.input_fill
    ));
    salt_text(
        out,
        x + pad,
        text_y,
        &format!(
            "font-family=\"{}\" font-size=\"12\" fill=\"{}\"",
            style.font_family, style.input_text_color
        ),
        placeholder,
        &style.input_text_color,
    );
    out.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/><line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/><line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/></g>",
        x + pad - 2,
        line_y,
        x + pad + control_w,
        line_y,
        style.border_color,
        x + pad - 2,
        line_y - 3,
        x + pad - 2,
        line_y,
        style.border_color,
        x + pad + control_w,
        line_y - 3,
        x + pad + control_w,
        line_y,
        style.border_color
    ));
}

pub(super) fn render_button(
    out: &mut String,
    cell_box: SaltCellBox,
    label: &str,
    style: &SaltRenderStyle,
) {
    let SaltCellBox { x, y, w, h } = cell_box;
    let pad = 8;
    let button_w = salt_button_width(label).min(w - pad * 2).max(24);
    let button_h = 20;
    let button_y = y + ((h - button_h) / 2).max(0);
    out.push_str(&format!(
        "<rect data-salt-widget=\"button\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2.5\" rx=\"5\" ry=\"5\"/>",
        x + pad,
        button_y,
        button_w,
        button_h,
        style.button_fill,
        style.border_color
    ));
    salt_text(
        out,
        x + pad + button_w / 2,
        button_y + 15,
        &format!(
            "text-anchor=\"middle\" font-family=\"{}\" font-size=\"12\" font-weight=\"bold\" fill=\"{}\"",
            style.font_family, style.button_text_color
        ),
        label,
        &style.button_text_color,
    );
}

pub(super) fn render_combo(
    out: &mut String,
    cell_box: SaltCellBox,
    label: &str,
    style: &SaltRenderStyle,
) {
    let SaltCellBox { x, y, w, h } = cell_box;
    let pad = 8;
    let combo_w = salt_combo_width(label).min(w - pad * 2).max(28);
    let combo_h = 19;
    let combo_y = y + ((h - combo_h) / 2).max(0);
    out.push_str(&format!(
        "<rect data-salt-widget=\"combo\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
        x + pad,
        combo_y,
        combo_w,
        combo_h,
        style.input_fill,
        style.border_color
    ));
    salt_text(
        out,
        x + pad + 2,
        combo_y + 14,
        &format!(
            "font-family=\"{}\" font-size=\"12\" fill=\"{}\"",
            style.font_family, style.input_text_color
        ),
        label,
        &style.input_text_color,
    );
    let divider_x = x + pad + combo_w - 11;
    out.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
        divider_x,
        combo_y,
        divider_x,
        combo_y + combo_h,
        style.border_color
    ));
    out.push_str(&format!(
        "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
        divider_x + 3,
        combo_y + 6,
        divider_x + 9,
        combo_y + 6,
        divider_x + 6,
        combo_y + combo_h - 5,
        style.border_color,
        style.border_color
    ));
}

pub(super) fn render_checkbox(
    out: &mut String,
    cell_box: SaltCellBox,
    label: &str,
    checked: bool,
    style: &SaltRenderStyle,
) {
    let SaltCellBox { x, y, h, .. } = cell_box;
    let pad = 8;
    let bx = x + pad;
    let by = y + h / 2 - 5;
    out.push_str(&format!(
        "<rect data-salt-widget=\"checkbox\" x=\"{}\" y=\"{}\" width=\"10\" height=\"10\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        bx, by, style.checkbox_fill, style.border_color
    ));
    if checked {
        out.push_str(&format!(
            "<polygon points=\"{},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            bx + 1,
            by + 4,
            bx + 4,
            by + 7,
            bx + 11,
            by - 2,
            bx + 4,
            by + 5,
            style.text_color,
            style.text_color
        ));
    }
    if !label.is_empty() {
        salt_text(
            out,
            bx + 18,
            y + h / 2 + 4,
            &format!(
                "font-family=\"{}\" font-size=\"12\" fill=\"{}\"",
                style.font_family, style.text_color
            ),
            label,
            &style.text_color,
        );
    }
}

pub(super) fn render_radio(
    out: &mut String,
    cell_box: SaltCellBox,
    label: &str,
    selected: bool,
    style: &SaltRenderStyle,
) {
    let SaltCellBox { x, y, h, .. } = cell_box;
    let pad = 8;
    let cx = x + pad + 5;
    let cy = y + h / 2;
    out.push_str(&format!(
        "<circle data-salt-widget=\"radio\" cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        cx, cy, style.radio_fill, style.border_color
    ));
    if selected {
        out.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"2\" fill=\"{}\"/>",
            cx, cy, style.text_color
        ));
    }
    if !label.is_empty() {
        salt_text(
            out,
            cx + 10,
            y + h / 2 + 4,
            &format!(
                "font-family=\"{}\" font-size=\"12\" fill=\"{}\"",
                style.font_family, style.text_color
            ),
            label,
            &style.text_color,
        );
    }
}
