use super::SaltCellBox;
use crate::render::salt::style::SaltRenderStyle;
use crate::render::salt::text::{estimate_text_width, salt_combo_width, salt_text};

pub(super) fn render_tree_item(
    out: &mut String,
    cell_box: SaltCellBox,
    depth: usize,
    label: &str,
    style: &SaltRenderStyle,
) {
    let SaltCellBox { x, y, h, .. } = cell_box;
    let pad = 8;
    let indent = (depth as i32) * 16;
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
        y + h / 2 + 4,
        &format!(
            "font-family=\"{}\" font-size=\"12\" fill=\"{}\"",
            style.font_family, style.text_color
        ),
        label,
        &style.text_color,
    );
    out.push_str("</g>");
}

pub(super) fn render_text_area(
    out: &mut String,
    cell_box: SaltCellBox,
    text: &str,
    scroll_vertical: bool,
    scroll_horizontal: bool,
    style: &SaltRenderStyle,
) {
    let SaltCellBox { x, y, w, h } = cell_box;
    let pad = 8;
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
            y + h / 2 + 4,
            &format!(
                "font-family=\"{}\" font-size=\"12\" fill=\"{}\"",
                style.font_family, style.input_text_color
            ),
            text,
            &style.input_text_color,
        );
    }
    if scroll_vertical {
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
    if scroll_horizontal {
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

pub(super) fn render_group_box(
    out: &mut String,
    cell_box: SaltCellBox,
    label: &str,
    style: &SaltRenderStyle,
) {
    let SaltCellBox { x, y, w, h } = cell_box;
    let pad = 8;
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

pub(super) fn render_menu_bar(
    out: &mut String,
    cell_box: SaltCellBox,
    items: &[String],
    style: &SaltRenderStyle,
) {
    let SaltCellBox { x, y, w, h } = cell_box;
    let pad = 8;
    let menu_h = 20;
    out.push_str(&format!(
        "<rect data-salt-widget=\"menu\" data-salt-open=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
        items.len() > 4,
        x + 1,
        y + 2,
        w - 2,
        menu_h,
        style.menu_fill,
        style.border_color
    ));
    let mut item_x = x + pad;
    for item in items {
        salt_text(
            out,
            item_x,
            y + 2 + menu_h / 2 + 4,
            &format!(
                "font-family=\"{}\" font-size=\"12\" fill=\"{}\"",
                style.font_family, style.text_color
            ),
            item,
            &style.text_color,
        );
        item_x += estimate_text_width(item) + 24;
    }
    if items.len() > 4 {
        let dropdown_y = y + 2 + menu_h;
        let item_h = 16;
        let dropdown_w = items
            .iter()
            .skip(1)
            .map(|item| estimate_text_width(item) + 24)
            .max()
            .unwrap_or(80)
            .max(80)
            .min(w - pad * 2);
        let dropdown_h = ((items.len() - 1) as i32 * item_h).min(h - menu_h - 4);
        out.push_str(&format!(
            "<rect data-salt-widget=\"menu-dropdown\" data-salt-open-index=\"0\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            x + pad,
            dropdown_y,
            dropdown_w,
            dropdown_h,
            style.menu_fill,
            style.border_color
        ));
        for (idx, item) in items.iter().skip(1).enumerate() {
            let item_y = dropdown_y + (idx as i32) * item_h;
            if item_y + item_h > dropdown_y + dropdown_h {
                break;
            }
            salt_text(
                out,
                x + pad + 6,
                item_y + 12,
                &format!(
                    "font-family=\"{}\" font-size=\"11\" fill=\"{}\"",
                    style.font_family, style.text_color
                ),
                item,
                &style.text_color,
            );
        }
    }
}

pub(super) fn render_tab_bar(
    out: &mut String,
    cell_box: SaltCellBox,
    tabs: &[String],
    active: usize,
    style: &SaltRenderStyle,
) {
    let SaltCellBox { x, y, w, h } = cell_box;
    let pad = 8;
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
    let tab_widths: Vec<i32> = tabs.iter().map(|t| estimate_text_width(t) + 24).collect();
    for (idx, tab) in tabs.iter().enumerate() {
        let tab_w = tab_widths[idx];
        let active_tab = idx == active;
        let tab_top = y + 3;
        let tab_h = if active_tab { h - 3 } else { h - 6 };
        let fill = if active_tab {
            &style.panel_fill
        } else {
            &style.tab_fill
        };
        out.push_str(&format!(
            "<rect data-salt-widget=\"tab\" data-salt-tab-active=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" ry=\"4\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            active_tab,
            tab_x,
            tab_top,
            tab_w,
            tab_h,
            fill,
            style.border_color
        ));
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
    let _ = (x + pad, y + h - 1, x + w - pad);
}

pub(super) fn render_scroll_bar(
    out: &mut String,
    cell_box: SaltCellBox,
    vertical: bool,
    percent: u8,
    style: &SaltRenderStyle,
) {
    let SaltCellBox { x, y, w, h } = cell_box;
    let pad = 8;
    let track_x = if vertical { x + w - pad - 12 } else { x + pad };
    let track_y = if vertical { y + 5 } else { y + h - 13 };
    let track_w = if vertical { 12 } else { w - pad * 2 };
    let track_h = if vertical { h - 10 } else { 12 };
    out.push_str(&format!(
        "<rect data-salt-widget=\"scrollbar\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" ry=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
        track_x, track_y, track_w, track_h, style.scroll_fill, style.border_color
    ));
    if vertical {
        let thumb_h = ((track_h as f32) * (percent as f32 / 100.0)).round() as i32;
        out.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"5\" ry=\"5\" fill=\"{}\"/>",
            track_x + 2,
            track_y + 2,
            track_w - 4,
            thumb_h.max(8).min(track_h - 4),
            style.border_color
        ));
    } else {
        let thumb_w = ((track_w as f32) * (percent as f32 / 100.0)).round() as i32;
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

pub(super) fn render_open_combo(
    out: &mut String,
    cell_box: SaltCellBox,
    label: &str,
    items: &[String],
    style: &SaltRenderStyle,
) {
    let SaltCellBox { x, y, w, .. } = cell_box;
    let pad = 8;
    let combo_w = salt_combo_width(label).min(w - pad * 2).max(28);
    let combo_h = 19;
    out.push_str(&format!(
        "<rect data-salt-widget=\"open-combo\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
        x + pad,
        y + 2,
        combo_w,
        combo_h,
        style.input_fill,
        style.border_color
    ));
    salt_text(
        out,
        x + pad + 2,
        y + 2 + 14,
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
        y + 2,
        divider_x,
        y + 2 + combo_h,
        style.border_color
    ));
    out.push_str(&format!(
        "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
        divider_x + 3,
        y + 2 + combo_h - 6,
        divider_x + 9,
        y + 2 + combo_h - 6,
        divider_x + 6,
        y + 2 + 5,
        style.border_color,
        style.border_color
    ));
    let item_h = 16i32;
    let list_y = y + 2 + combo_h;
    let list_h = (items.len() as i32) * item_h;
    if !items.is_empty() {
        out.push_str(&format!(
            "<rect data-salt-widget=\"open-combo-list\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            x + pad,
            list_y,
            combo_w,
            list_h,
            style.input_fill,
            style.border_color
        ));
        for (i, item) in items.iter().enumerate() {
            salt_text(
                out,
                x + pad + 4,
                list_y + (i as i32) * item_h + 12,
                &format!(
                    "font-family=\"{}\" font-size=\"11\" fill=\"{}\"",
                    style.font_family, style.text_color
                ),
                item,
                &style.text_color,
            );
        }
    }
}

pub(super) fn render_progress_bar(
    out: &mut String,
    cell_box: SaltCellBox,
    fill_ratio: f32,
    style: &SaltRenderStyle,
) {
    let SaltCellBox { x, y, w, h } = cell_box;
    let pad = 8;
    let bar_h = 10;
    let bar_y = y + (h - bar_h) / 2;
    let bar_w = (w - pad * 2).max(20);
    let filled_w = ((bar_w as f32) * fill_ratio.clamp(0.0, 1.0)).round() as i32;
    out.push_str(&format!(
        "<rect data-salt-widget=\"progress\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\" rx=\"3\" ry=\"3\"/>",
        x + pad, bar_y, bar_w, bar_h, style.panel_fill, style.border_color
    ));
    if filled_w > 0 {
        out.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" rx=\"3\" ry=\"3\"/>",
            x + pad,
            bar_y,
            filled_w,
            bar_h,
            style.accent_fill
        ));
    }
}
