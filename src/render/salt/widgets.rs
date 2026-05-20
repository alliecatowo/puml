use super::*;

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
    let SaltCellBox { x, y, w, h } = cell_box;
    if colspan > 1 {
        out.push_str(&format!(
            "<g data-salt-widget=\"table-span\" data-salt-colspan=\"{}\" data-salt-span-width=\"{}\">",
            colspan, w
        ));
    }
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
        SaltCellRender::Button(label) => {
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
        SaltCellRender::Combo(label) => {
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
        SaltCellRender::CheckboxChecked(label) => {
            let bx = x + pad;
            let by = y + h / 2 - 5;
            out.push_str(&format!(
                "<rect data-salt-widget=\"checkbox\" x=\"{}\" y=\"{}\" width=\"10\" height=\"10\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                bx, by, style.checkbox_fill, style.border_color
            ));
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
            if !label.is_empty() {
                salt_text(
                    out,
                    bx + 18,
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
            let by = y + h / 2 - 5;
            out.push_str(&format!(
                "<rect data-salt-widget=\"checkbox\" x=\"{}\" y=\"{}\" width=\"10\" height=\"10\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                bx, by, style.checkbox_fill, style.border_color
            ));
            if !label.is_empty() {
                salt_text(
                    out,
                    bx + 18,
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
            let cx = x + pad + 5;
            let cy = y + h / 2;
            out.push_str(&format!(
                "<circle data-salt-widget=\"radio\" cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx, cy, style.radio_fill, style.border_color
            ));
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"2\" fill=\"{}\"/>",
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
            let cx = x + pad + 5;
            let cy = y + h / 2;
            out.push_str(&format!(
                "<circle data-salt-widget=\"radio\" cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
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
            // Draw the full-width underline first so active tab can overdraw it.
            let strip_y = y + h - 2;
            out.push_str(&format!(
                "<line data-salt-widget=\"tab-strip\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                x + pad,
                strip_y,
                x + w - pad,
                strip_y,
                style.border_color
            ));
            let mut tab_x = x + pad;
            // First pass: collect tab widths for the active-tab gap overdraw.
            let tab_widths: Vec<i32> = tabs.iter().map(|t| estimate_text_width(t) + 24).collect();
            for (idx, tab) in tabs.iter().enumerate() {
                let tab_w = tab_widths[idx];
                let active_tab = idx == *active;
                let tab_top = y + 3;
                // Active tab is taller (reaches the strip) and filled white;
                // inactive tabs are shorter and filled with tab_fill colour.
                let tab_h = if active_tab { h - 3 } else { h - 6 };
                let fill = if active_tab {
                    &style.panel_fill
                } else {
                    &style.tab_fill
                };
                let stroke = &style.border_color;
                // Rounded-top tab rectangle.
                out.push_str(&format!(
                    "<rect data-salt-widget=\"tab\" data-salt-tab-active=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" ry=\"4\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                    active_tab,
                    tab_x,
                    tab_top,
                    tab_w,
                    tab_h,
                    fill,
                    stroke
                ));
                // Overdraw the bottom border of the active tab with its fill
                // colour so the tab appears "connected" to the content below.
                if active_tab {
                    out.push_str(&format!(
                        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"3\"/>",
                        tab_x + 1,
                        strip_y,
                        tab_x + tab_w - 1,
                        strip_y,
                        fill
                    ));
                }
                let label_color = if active_tab {
                    &style.text_color
                } else {
                    &style.muted_text_color
                };
                salt_text(
                    out,
                    tab_x + 12,
                    tab_top + tab_h / 2 + 4,
                    &format!(
                        "font-family=\"{}\" font-size=\"12\"{}fill=\"{}\"",
                        style.font_family,
                        if active_tab {
                            " font-weight=\"bold\" "
                        } else {
                            " "
                        },
                        label_color
                    ),
                    tab,
                    label_color,
                );
                tab_x += tab_w - 1;
            }
            // Invisible anchor preserving the trailing baseline geometry.
            let _ = (x + pad, y + h - 1, x + w - pad);
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
    if colspan > 1 {
        out.push_str("</g>");
    }
}
