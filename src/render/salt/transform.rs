use super::model::SaltCellRender;
use super::parsing::{
    parse_salt_items, parse_salt_open_combo, parse_salt_open_combo_payload, parse_salt_password,
    parse_salt_progress_bar, parse_salt_scroll_container, parse_salt_scrollbar, parse_salt_slider,
    parse_salt_sprite_def, parse_salt_sprite_ref, parse_salt_tab_bar, parse_salt_tree_line,
};
use super::style::{apply_salt_style_directive, SaltRenderStyle};

#[derive(Default)]
pub(super) struct SaltTransformState {
    in_tree: bool,
    in_text_area: bool,
    in_style: bool,
    in_sprite_def: bool,
    style_scope: Option<String>,
    table_header_pending: bool,
}

pub(super) fn transform_salt_row(
    cells: Vec<SaltCellRender>,
    state: &mut SaltTransformState,
    style: &mut SaltRenderStyle,
) -> Option<Vec<SaltCellRender>> {
    if cells.len() != 1 {
        return Some(transform_salt_grid_cells(cells, state));
    }

    let SaltCellRender::Label(text) = &cells[0] else {
        return Some(transform_salt_grid_cells(cells, state));
    };
    let trimmed = text.trim();
    let lower = trimmed.to_ascii_lowercase();

    if lower == "<style>" {
        state.in_style = true;
        return None;
    }
    if lower == "</style>" {
        state.in_style = false;
        state.style_scope = None;
        return None;
    }
    if state.in_style {
        if trimmed.ends_with('{') {
            state.style_scope = Some(trimmed.trim_end_matches('{').trim().to_string());
            return None;
        }
        if trimmed == "}" {
            state.style_scope = None;
            return None;
        }
        if let Some((key, value)) = trimmed.split_once(char::is_whitespace) {
            style.set_scoped(state.style_scope.as_deref(), key.trim(), value.trim());
        }
        return None;
    }

    if apply_salt_style_directive(trimmed, style) {
        return None;
    }

    if state.in_sprite_def {
        if lower.starts_with(">>") {
            state.in_sprite_def = false;
        }
        return None;
    }

    if matches!(trimmed, "{" | "}") {
        if trimmed == "}" {
            state.in_tree = false;
            state.in_text_area = false;
            state.in_sprite_def = false;
            state.table_header_pending = false;
        }
        return None;
    }

    if lower.starts_with("{#") || lower.starts_with("{!") {
        state.table_header_pending = true;
        state.in_text_area = false;
        return None;
    }

    if let Some(name) = parse_salt_sprite_def(trimmed) {
        state.in_sprite_def = true;
        return Some(vec![SaltCellRender::SpriteDef(name)]);
    }

    if lower.starts_with("{+") {
        state.in_text_area = true;
        state.in_tree = false;
        return Some(vec![SaltCellRender::TextAreaLine {
            text: String::new(),
            scroll_vertical: false,
            scroll_horizontal: false,
        }]);
    }

    if lower.starts_with("{^") {
        state.in_text_area = false;
        let label = trimmed
            .trim_start_matches("{^")
            .trim_matches('}')
            .trim()
            .to_string();
        return Some(vec![SaltCellRender::GroupBox(label)]);
    }

    // Structural block widgets ({/ tabs, {* menu) must be recognised even when
    // in_text_area is active — they can appear inside a bordered {+ container.
    if let Some((tabs, active)) = parse_salt_tab_bar(trimmed) {
        state.in_text_area = false;
        return Some(vec![SaltCellRender::TabBar { tabs, active }]);
    }

    if let Some(items) = parse_salt_items(trimmed, &["{*", "menu"]) {
        state.in_text_area = false;
        return Some(vec![SaltCellRender::MenuBar(items)]);
    }

    if state.in_text_area {
        let text = if trimmed == "." { "" } else { trimmed };
        return Some(vec![SaltCellRender::TextAreaLine {
            text: text.to_string(),
            scroll_vertical: false,
            scroll_horizontal: false,
        }]);
    }

    // `{T ... }` and `{. ... }` are both tree/outline container blocks.
    // `{T}` uses `+`/`++` prefix syntax; `{.}` uses `**`/`***` prefix syntax.
    // Both activate `in_tree` so subsequent lines are parsed as tree items.
    if lower.starts_with("{t")
        || lower.starts_with("{.")
        || lower == "tree"
        || lower.starts_with("tree ")
    {
        state.in_tree = true;
        return None;
    }

    if let Some((depth, label)) = parse_salt_tree_line(trimmed) {
        return Some(vec![SaltCellRender::TreeItem { depth, label }]);
    }

    // Bare `tab`/`tabs` keyword tab-bar (outside text areas).
    if let Some(items) = parse_salt_items(trimmed, &["tab", "tabs"]) {
        if items.len() > 1 || trimmed.contains('|') {
            return Some(vec![SaltCellRender::TabBar {
                tabs: items,
                active: 0,
            }]);
        }
    }

    if let Some(scroll) = parse_salt_scroll_container(trimmed) {
        state.in_text_area = true;
        return Some(vec![SaltCellRender::TextAreaLine {
            text: String::new(),
            scroll_vertical: scroll.0,
            scroll_horizontal: scroll.1,
        }]);
    }

    if let Some((vertical, percent)) = parse_salt_scrollbar(trimmed) {
        return Some(vec![SaltCellRender::ScrollBar { vertical, percent }]);
    }

    // Standalone open droplist `^label^^item1^^item2^`.
    if let Some((label, items)) = parse_salt_open_combo(trimmed) {
        return Some(vec![SaltCellRender::OpenCombo { label, items }]);
    }

    // Standalone closed combo `^label^` (no `|`-delimited context).
    if trimmed.starts_with('^') && trimmed.ends_with('^') && trimmed.len() >= 3 {
        let inner = trimmed[1..trimmed.len() - 1].to_string();
        return Some(vec![SaltCellRender::Combo(inner)]);
    }

    // Progress bar `[=====   ]`.
    if let Some(fill_ratio) = parse_salt_progress_bar(trimmed) {
        return Some(vec![SaltCellRender::ProgressBar { fill_ratio }]);
    }

    // Slider `{slider:min,max,val}` / `{slider}`.
    if let Some((min, max, value)) = parse_salt_slider(trimmed) {
        return Some(vec![SaltCellRender::Slider { min, max, value }]);
    }

    // Password field `"****"` / `"*hint*"` — must come after other quoted-string
    // checks (Input, Combo) to avoid false-positive masking of plain inputs.
    if let Some(label) = parse_salt_password(trimmed) {
        return Some(vec![SaltCellRender::Password(label)]);
    }

    if state.in_tree {
        state.in_tree = false;
    }

    Some(transform_salt_grid_cells(cells, state))
}

fn transform_salt_grid_cells(
    cells: Vec<SaltCellRender>,
    state: &mut SaltTransformState,
) -> Vec<SaltCellRender> {
    let header_row = state.table_header_pending;
    state.table_header_pending = false;
    cells
        .into_iter()
        .map(|cell| transform_salt_table_cell(cell, header_row))
        .collect()
}

fn transform_salt_table_cell(cell: SaltCellRender, header_row: bool) -> SaltCellRender {
    match cell {
        SaltCellRender::Label(text) => {
            let trimmed = text.trim();
            if trimmed == "." {
                SaltCellRender::TableEmpty
            } else if trimmed == "*" {
                SaltCellRender::TableSpan
            } else if let Some(name) = parse_salt_sprite_ref(trimmed) {
                SaltCellRender::SpriteRef(name)
            } else if let Some(fill_ratio) = parse_salt_progress_bar(trimmed) {
                // Progress bars in table cells
                SaltCellRender::ProgressBar { fill_ratio }
            } else if let Some((min, max, value)) = parse_salt_slider(trimmed) {
                // Sliders in table cells
                SaltCellRender::Slider { min, max, value }
            } else if let Some(label) = parse_salt_password(trimmed) {
                // Password fields in table cells
                SaltCellRender::Password(label)
            } else if header_row {
                SaltCellRender::Header(trimmed.trim_start_matches('=').trim().to_string())
            } else {
                promote_salt_header_cell(SaltCellRender::Label(text))
            }
        }
        SaltCellRender::Combo(text) => {
            if let Some((label, items)) = parse_salt_open_combo_payload(&text) {
                SaltCellRender::OpenCombo { label, items }
            } else {
                SaltCellRender::Combo(text)
            }
        }
        other => other,
    }
}

fn promote_salt_header_cell(cell: SaltCellRender) -> SaltCellRender {
    match cell {
        SaltCellRender::Label(text) => {
            let trimmed = text.trim();
            if let Some(rest) = trimmed.strip_prefix('=') {
                SaltCellRender::Header(rest.trim().to_string())
            } else if let Some(name) = parse_salt_sprite_ref(trimmed) {
                SaltCellRender::SpriteRef(name)
            } else {
                SaltCellRender::Label(text)
            }
        }
        other => other,
    }
}
