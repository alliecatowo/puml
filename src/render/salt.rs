mod widgets;

use super::*;
use widgets::*;

pub fn render_salt_svg(document: &FamilyDocument) -> String {
    const CELL_H: i32 = 20;
    const CELL_PAD_X: i32 = 10;
    const MARGIN: i32 = 6;
    const MIN_CELL_W: i32 = 80;

    // Parse rows from the encoded node names.
    let mut rows: Vec<Vec<SaltCellRender>> = Vec::new();
    let mut salt_state = SaltTransformState::default();
    let mut style = SaltRenderStyle::default();
    for node in &document.nodes {
        if let Some(rest) = node.name.strip_prefix("SALT_ROW\x1f") {
            let cells: Vec<SaltCellRender> = rest.split('\x1e').map(decode_salt_cell).collect();
            if let Some(cells) = transform_salt_row(cells, &mut salt_state, &mut style) {
                rows.push(cells);
            }
        }
    }

    if rows.is_empty() {
        // Fallback: render a minimal empty wireframe
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"120\" height=\"60\"><rect width=\"120\" height=\"60\" fill=\"white\"/><text x=\"10\" y=\"30\" font-family=\"monospace\" font-size=\"11\" fill=\"#666\">[salt]</text></svg>".to_string();
    }

    // Compute number of columns from the max row width.
    let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(1);
    let table_like = rows.iter().flatten().any(SaltCellRender::is_table_like);

    // First pass: compute per-column minimum widths based on text content.
    let mut col_widths: Vec<i32> = vec![MIN_CELL_W; col_count];
    for row in &rows {
        for (col_idx, cell) in row.iter().enumerate() {
            let text_w = cell
                .intrinsic_width()
                .max(estimate_text_width(cell.text()) + CELL_PAD_X);
            if text_w > col_widths[col_idx] {
                col_widths[col_idx] = text_w;
            }
        }
    }

    let total_w = col_widths.iter().sum::<i32>() + MARGIN * 2;
    let total_h = (rows.len() as i32) * CELL_H + MARGIN * 2;

    // Title height
    let title_h = document.title.as_deref().map(|_| 28i32).unwrap_or(0);
    let svg_h = total_h + title_h;

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\">",
        total_w, svg_h
    ));
    out.push_str(&format!(
        "<rect data-salt-style=\"canvas\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
        total_w, svg_h, style.canvas_fill
    ));

    let render_panel = table_like || style.panel_fill != SaltRenderStyle::default().panel_fill;
    if render_panel {
        out.push_str(&format!(
            "<rect data-salt-style=\"panel\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            MARGIN,
            MARGIN + title_h,
            total_w - MARGIN * 2,
            total_h - MARGIN * 2,
            style.panel_fill,
            style.border_color
        ));
    }

    if let Some(title) = &document.title {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"{}\" font-size=\"13\" font-weight=\"600\" fill=\"{}\">{}</text>",
            MARGIN,
            MARGIN - 6,
            style.font_family,
            style.text_color,
            escape_text(title)
        ));
    }

    // Draw rows and cells.
    for (row_idx, cells) in rows.iter().enumerate() {
        let row_y = MARGIN + title_h + (row_idx as i32) * CELL_H;
        if is_salt_separator_row(cells) {
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                MARGIN + 4,
                row_y + CELL_H / 2,
                total_w - MARGIN - 4,
                row_y + CELL_H / 2,
                style.border_color
            ));
            continue;
        }
        let mut col_x = MARGIN;
        let rendered_cells = salt_row_layout(cells, &col_widths, MIN_CELL_W);

        for cell in rendered_cells {
            render_salt_cell_svg(
                &mut out,
                cell.cell,
                SaltCellBox {
                    x: col_x,
                    y: row_y,
                    w: cell.width,
                    h: CELL_H,
                },
                cell.colspan,
                &style,
            );
            col_x += cell.width;
        }

        // Row separator line (skip the last row)
        if table_like && row_idx + 1 < rows.len() {
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
                MARGIN,
                row_y + CELL_H,
                total_w - MARGIN,
                row_y + CELL_H,
                style.grid_color
            ));
        }
    }

    // Column separator lines
    if table_like {
        let mut col_x = MARGIN;
        for (col_idx, w) in col_widths.iter().enumerate() {
            col_x += w;
            if col_idx + 1 < col_count {
                out.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
                    col_x,
                    MARGIN + title_h,
                    col_x,
                    MARGIN + title_h + total_h - MARGIN * 2,
                    style.grid_color
                ));
            }
        }
    }

    out.push_str("</svg>");
    out
}

struct SaltRenderedCell<'a> {
    cell: &'a SaltCellRender,
    width: i32,
    colspan: usize,
}

fn salt_row_layout<'a>(
    cells: &'a [SaltCellRender],
    col_widths: &[i32],
    min_cell_w: i32,
) -> Vec<SaltRenderedCell<'a>> {
    let mut rendered: Vec<SaltRenderedCell<'a>> = Vec::new();
    for (col_idx, cell) in cells.iter().enumerate() {
        let width = col_widths.get(col_idx).copied().unwrap_or(min_cell_w);
        if matches!(cell, SaltCellRender::TableSpan) {
            if let Some(previous) = rendered.last_mut() {
                previous.width += width;
                previous.colspan += 1;
            } else {
                rendered.push(SaltRenderedCell {
                    cell,
                    width,
                    colspan: 1,
                });
            }
        } else {
            rendered.push(SaltRenderedCell {
                cell,
                width,
                colspan: 1,
            });
        }
    }
    rendered
}

fn is_salt_separator_row(cells: &[SaltCellRender]) -> bool {
    let mut saw_dash = false;
    for cell in cells {
        match cell {
            SaltCellRender::Label(text) => {
                let t = text.trim();
                if t.is_empty() {
                    continue;
                }
                if t.chars().all(|c| c == '-') {
                    saw_dash = true;
                    continue;
                }
                return false;
            }
            _ => return false,
        }
    }
    saw_dash
}

struct SaltRenderStyle {
    canvas_fill: String,
    panel_fill: String,
    header_fill: String,
    input_fill: String,
    button_fill: String,
    menu_fill: String,
    tab_fill: String,
    scroll_fill: String,
    checkbox_fill: String,
    radio_fill: String,
    accent_fill: String,
    border_color: String,
    grid_color: String,
    text_color: String,
    header_text_color: String,
    input_text_color: String,
    button_text_color: String,
    muted_text_color: String,
    font_family: &'static str,
}

impl Default for SaltRenderStyle {
    fn default() -> Self {
        Self {
            canvas_fill: "white".to_string(),
            panel_fill: "white".to_string(),
            header_fill: "#e2e8f0".to_string(),
            input_fill: "white".to_string(),
            button_fill: "#e8e8e8".to_string(),
            menu_fill: "#eef2ff".to_string(),
            tab_fill: "#eef2ff".to_string(),
            scroll_fill: "#eef2ff".to_string(),
            checkbox_fill: "white".to_string(),
            radio_fill: "white".to_string(),
            accent_fill: "#eef2ff".to_string(),
            border_color: "#555".to_string(),
            grid_color: "#ccc".to_string(),
            text_color: "#222".to_string(),
            header_text_color: "#222".to_string(),
            input_text_color: "#222".to_string(),
            button_text_color: "#222".to_string(),
            muted_text_color: "#aaa".to_string(),
            font_family: "monospace",
        }
    }
}

impl SaltRenderStyle {
    fn set(&mut self, key: &str, value: &str) -> bool {
        let value = normalize_salt_color(value).unwrap_or_else(|| value.trim().to_string());
        match key.to_ascii_lowercase().as_str() {
            "backgroundcolor" | "saltbackgroundcolor" | "canvascolor" => {
                self.canvas_fill = value;
                true
            }
            "saltpanelcolor" | "panelcolor" | "saltfillcolor" => {
                self.panel_fill = value;
                true
            }
            "saltheadercolor" | "headercolor" | "tableheadercolor" => {
                self.header_fill = value;
                true
            }
            "saltinputcolor" | "saltinputbackgroundcolor" | "inputbackgroundcolor" => {
                self.input_fill = value;
                true
            }
            "saltbuttoncolor" | "saltbuttonbackgroundcolor" | "buttonbackgroundcolor" => {
                self.button_fill = value;
                true
            }
            "saltmenucolor" | "saltmenubackgroundcolor" | "menubackgroundcolor" => {
                self.menu_fill = value;
                true
            }
            "salttabcolor" | "salttabbackgroundcolor" | "tabbackgroundcolor" => {
                self.tab_fill = value;
                true
            }
            "saltscrollbarcolor" | "scrollbarcolor" | "scrollbarbackgroundcolor" => {
                self.scroll_fill = value;
                true
            }
            "saltcheckboxcolor" | "checkboxbackgroundcolor" => {
                self.checkbox_fill = value;
                true
            }
            "saltradiocolor" | "radiobackgroundcolor" => {
                self.radio_fill = value;
                true
            }
            "saltaccentcolor" | "accentcolor" => {
                self.accent_fill = value;
                true
            }
            "bordercolor" | "linecolor" | "saltbordercolor" | "saltlinecolor" => {
                self.border_color = value;
                true
            }
            "saltgridcolor" | "gridcolor" => {
                self.grid_color = value;
                true
            }
            "fontcolor" | "saltfontcolor" => {
                self.text_color = value;
                true
            }
            "saltheaderfontcolor" | "headerfontcolor" => {
                self.header_text_color = value;
                true
            }
            "saltinputfontcolor" | "inputfontcolor" => {
                self.input_text_color = value;
                true
            }
            "saltbuttonfontcolor" | "buttonfontcolor" => {
                self.button_text_color = value;
                true
            }
            "saltmutedfontcolor" | "mutedfontcolor" => {
                self.muted_text_color = value;
                true
            }
            "handwritten" if value.eq_ignore_ascii_case("true") => {
                self.font_family = "Comic Sans MS, cursive";
                true
            }
            _ => false,
        }
    }

    fn set_scoped(&mut self, scope: Option<&str>, key: &str, value: &str) -> bool {
        let Some(scope) = scope else {
            return self.set(key, value);
        };
        let scope = scope
            .trim()
            .trim_matches('{')
            .split_whitespace()
            .last()
            .unwrap_or(scope)
            .to_ascii_lowercase();
        let key = key.trim();
        let lower_key = key.to_ascii_lowercase();
        let mapped = match (scope.as_str(), lower_key.as_str()) {
            ("saltdiagram" | "salt", _) => key.to_string(),
            ("button", "backgroundcolor") => "saltButtonBackgroundColor".to_string(),
            ("button", "fontcolor") => "saltButtonFontColor".to_string(),
            ("input" | "textfield" | "textarea", "backgroundcolor") => {
                "saltInputBackgroundColor".to_string()
            }
            ("input" | "textfield" | "textarea", "fontcolor") => "saltInputFontColor".to_string(),
            ("header", "backgroundcolor") => "saltHeaderColor".to_string(),
            ("header", "fontcolor") => "saltHeaderFontColor".to_string(),
            ("menu", "backgroundcolor") => "saltMenuBackgroundColor".to_string(),
            ("tab", "backgroundcolor") => "saltTabBackgroundColor".to_string(),
            ("scrollbar", "backgroundcolor") => "saltScrollbarColor".to_string(),
            ("checkbox", "backgroundcolor") => "saltCheckboxColor".to_string(),
            ("radio", "backgroundcolor") => "saltRadioColor".to_string(),
            (_, "linecolor" | "bordercolor") => "saltBorderColor".to_string(),
            _ => key.to_string(),
        };
        self.set(&mapped, value)
    }
}

fn normalize_salt_color(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_matches('"');
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with('#') {
        Some(trimmed.to_string())
    } else {
        Some(css3_color_to_hex(trimmed).unwrap_or(trimmed).to_string())
    }
}

fn apply_salt_style_directive(line: &str, style: &mut SaltRenderStyle) -> bool {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("!option ") {
        let original_rest = trimmed[trimmed.len() - rest.len()..].trim();
        if let Some((key, value)) = original_rest.split_once(char::is_whitespace) {
            return style.set(key.trim(), value.trim());
        }
    }
    if let Some(rest) = lower
        .strip_prefix("skinparam salt")
        .or_else(|| lower.strip_prefix("skinparam "))
    {
        let offset = trimmed.len() - rest.len();
        let original_rest = trimmed[offset..].trim();
        if let Some((key, value)) = original_rest.split_once(char::is_whitespace) {
            return style.set(key.trim(), value.trim());
        }
    }
    if let Some(rest) = trimmed.strip_prefix("saltstyle ") {
        if let Some((key, value)) = rest.split_once('=') {
            return style.set(key.trim(), value.trim());
        }
        if let Some((key, value)) = rest.split_once(char::is_whitespace) {
            return style.set(key.trim(), value.trim());
        }
    }
    false
}

/// A decoded salt cell ready for rendering.
enum SaltCellRender {
    Label(String),
    Header(String),
    TableEmpty,
    TableSpan,
    SpriteDef(String),
    SpriteRef(String),
    Input(String),
    Button(String),
    Combo(String),
    CheckboxChecked(String),
    CheckboxUnchecked(String),
    RadioOn(String),
    RadioOff(String),
    TreeItem {
        depth: usize,
        label: String,
    },
    TextAreaLine {
        text: String,
        scroll_vertical: bool,
        scroll_horizontal: bool,
    },
    GroupBox(String),
    MenuBar(Vec<String>),
    TabBar {
        tabs: Vec<String>,
        active: usize,
    },
    ScrollBar {
        vertical: bool,
        percent: u8,
    },
}

impl SaltCellRender {
    fn text(&self) -> &str {
        match self {
            Self::Label(t)
            | Self::Header(t)
            | Self::Input(t)
            | Self::Button(t)
            | Self::Combo(t)
            | Self::CheckboxChecked(t)
            | Self::CheckboxUnchecked(t)
            | Self::RadioOn(t)
            | Self::RadioOff(t) => t,
            Self::TableEmpty => "",
            Self::TableSpan => "span",
            Self::SpriteDef(name) | Self::SpriteRef(name) => name,
            Self::TreeItem { label, .. } => label,
            Self::TextAreaLine { text, .. } => text,
            Self::GroupBox(label) => label,
            Self::MenuBar(items) => items.first().map(String::as_str).unwrap_or("menu"),
            Self::TabBar { tabs, .. } => tabs.first().map(String::as_str).unwrap_or("tab"),
            Self::ScrollBar { .. } => "scrollbar",
        }
    }

    fn intrinsic_width(&self) -> i32 {
        match self {
            Self::Input(text) => estimate_text_width(text) + 29,
            Self::Button(text) => (estimate_text_width(text) + 16).max(36),
            Self::Combo(text) => estimate_text_width(text) + 23,
            Self::CheckboxChecked(text) | Self::CheckboxUnchecked(text) => {
                20 + estimate_text_width(text)
            }
            Self::RadioOn(text) | Self::RadioOff(text) => 20 + estimate_text_width(text),
            Self::MenuBar(items) => items
                .iter()
                .map(|item| estimate_text_width(item) + 24)
                .sum(),
            Self::TabBar { tabs, .. } => tabs.iter().map(|tab| estimate_text_width(tab) + 24).sum(),
            Self::ScrollBar { vertical, .. } => {
                if *vertical {
                    24
                } else {
                    80
                }
            }
            Self::SpriteRef(_) => 48,
            Self::TableEmpty => 24,
            Self::TableSpan => 42,
            _ => estimate_text_width(self.text()) + 20,
        }
    }

    fn is_table_like(&self) -> bool {
        matches!(self, Self::Header(_) | Self::TableEmpty | Self::TableSpan)
    }
}

#[derive(Default)]
struct SaltTransformState {
    in_tree: bool,
    in_text_area: bool,
    in_style: bool,
    in_sprite_def: bool,
    style_scope: Option<String>,
    table_header_pending: bool,
}

fn transform_salt_row(
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

    if lower.starts_with("{t") || lower == "tree" || lower.starts_with("tree ") {
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
            } else if header_row {
                SaltCellRender::Header(trimmed.trim_start_matches('=').trim().to_string())
            } else {
                promote_salt_header_cell(SaltCellRender::Label(text))
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

fn parse_salt_sprite_def(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.ends_with(">>") {
        return None;
    }
    let inner = trimmed.strip_prefix("<<")?;
    let name = inner
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_end_matches(">>")
        .trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn parse_salt_sprite_ref(text: &str) -> Option<String> {
    let trimmed = text.trim();
    let inner = trimmed.strip_prefix("<<")?.strip_suffix(">>")?.trim();
    if inner.is_empty() || inner.contains(char::is_whitespace) {
        None
    } else {
        Some(inner.to_string())
    }
}

fn parse_salt_tree_line(line: &str) -> Option<(usize, String)> {
    let depth = line.chars().take_while(|&ch| ch == '+').count();
    if depth == 0 {
        return None;
    }
    let label = line[depth..].trim().trim_matches('"').to_string();
    if label.is_empty() {
        None
    } else {
        Some((depth.saturating_sub(1), label))
    }
}

fn parse_salt_items(line: &str, prefixes: &[&str]) -> Option<Vec<String>> {
    let lower = line.to_ascii_lowercase();
    let mut rest = None;
    for prefix in prefixes {
        if lower.starts_with(prefix)
            && (prefix.starts_with('{')
                || lower.len() == prefix.len()
                || lower
                    .as_bytes()
                    .get(prefix.len())
                    .is_some_and(|ch| ch.is_ascii_whitespace()))
        {
            rest = Some(line[prefix.len()..].trim());
            break;
        }
    }
    let rest = rest?;
    let rest = rest.trim_matches('{').trim_matches('}').trim();
    let items: Vec<String> = rest
        .split(['|', ','])
        .map(|item| item.trim().trim_matches('"').to_string())
        .filter(|item| !item.is_empty())
        .collect();
    if items.is_empty() {
        None
    } else {
        Some(items)
    }
}

/// Parse a `{/ Tab1 | **Tab2** | Tab3 }` tab-bar declaration.
/// Returns `(labels, active_index)` where active tab is detected from `**...**` markup.
/// Falls back to index 0 when no tab is marked.
fn parse_salt_tab_bar(line: &str) -> Option<(Vec<String>, usize)> {
    let lower = line.to_ascii_lowercase();
    // Must start with `{/` (PlantUML tab syntax).
    // Bare `tab`/`tabs` keywords are handled by `parse_salt_items` elsewhere;
    // we deliberately do not match them here to avoid false-positives on lines
    // like "Tab content here" which begin with the word "tab".
    let rest = if lower.starts_with("{/") {
        line[2..].trim()
    } else {
        return None;
    };
    // Strip surrounding braces left over after the prefix
    let rest = rest.trim_start_matches('{').trim_end_matches('}').trim();
    if rest.is_empty() {
        return None;
    }
    let mut active = 0usize;
    let tabs: Vec<String> = rest
        .split('|')
        .enumerate()
        .filter_map(|(idx, raw)| {
            let t = raw.trim();
            if t.is_empty() {
                return None;
            }
            // `**label**` → active tab; strip markup and record index.
            if t.starts_with("**") && t.ends_with("**") && t.len() > 4 {
                active = idx;
                Some(t[2..t.len() - 2].trim().to_string())
            } else {
                Some(t.trim_matches('"').to_string())
            }
        })
        .collect();
    if tabs.is_empty() {
        None
    } else {
        Some((tabs, active))
    }
}

fn parse_salt_scrollbar(line: &str) -> Option<(bool, u8)> {
    let lower = line.to_ascii_lowercase();
    if !(lower.starts_with("{s") || lower.starts_with("scroll") || lower.contains("scrollbar")) {
        return None;
    }
    let vertical = !lower.contains("horizontal");
    let percent = lower
        .split(|ch: char| !ch.is_ascii_digit())
        .find_map(|part| part.parse::<u8>().ok())
        .unwrap_or(40)
        .min(100);
    Some((vertical, percent))
}

fn parse_salt_scroll_container(line: &str) -> Option<(bool, bool)> {
    let lower = line.to_ascii_lowercase();
    if !lower.starts_with("{s") || lower.starts_with("{*") {
        return None;
    }
    let marker = lower.trim_matches('{').trim_matches('}').trim();
    if marker.starts_with("si") {
        Some((true, false))
    } else if marker.starts_with("s-") {
        Some((false, true))
    } else {
        Some((true, true))
    }
}

/// Decode a salt cell from the encoded string `"X:text"`.
fn decode_salt_cell(s: &str) -> SaltCellRender {
    if let Some(rest) = s.strip_prefix("I:") {
        SaltCellRender::Input(rest.to_string())
    } else if let Some(rest) = s.strip_prefix("B:") {
        SaltCellRender::Button(rest.to_string())
    } else if let Some(rest) = s.strip_prefix("C:") {
        SaltCellRender::Combo(rest.to_string())
    } else if let Some(rest) = s.strip_prefix("CX:") {
        SaltCellRender::CheckboxChecked(rest.to_string())
    } else if let Some(rest) = s.strip_prefix("CU:") {
        SaltCellRender::CheckboxUnchecked(rest.to_string())
    } else if let Some(rest) = s.strip_prefix("RO:") {
        SaltCellRender::RadioOn(rest.to_string())
    } else if let Some(rest) = s.strip_prefix("RF:") {
        SaltCellRender::RadioOff(rest.to_string())
    } else if let Some(rest) = s.strip_prefix("L:") {
        SaltCellRender::Label(rest.to_string())
    } else {
        SaltCellRender::Label(s.to_string())
    }
}

/// Estimate text width in monospace pixels (approx 7px per char at 12px font).
fn estimate_text_width(text: &str) -> i32 {
    (text.chars().count() as i32) * 7
}

fn salt_input_width(text: &str) -> i32 {
    estimate_text_width(text) + 29
}

fn salt_button_width(text: &str) -> i32 {
    (estimate_text_width(text) + 16).max(36)
}

fn salt_combo_width(text: &str) -> i32 {
    estimate_text_width(text) + 23
}

fn salt_text(out: &mut String, x: i32, y: i32, attrs: &str, text: &str, color: &str) {
    let icon_names = extract_salt_icon_names(text);
    let mut extra_attrs = attrs.to_string();
    if salt_text_has_creole(text) {
        extra_attrs.push_str(" data-salt-creole=\"true\"");
    }
    if !icon_names.is_empty() {
        extra_attrs.push_str(&format!(
            " data-salt-icons=\"{}\"",
            escape_text(&icon_names.join(","))
        ));
    }
    out.push_str(&creole_text(x, y, &extra_attrs, text, color));
}

fn salt_text_has_creole(text: &str) -> bool {
    text.contains("**")
        || text.contains("//")
        || text.contains("\"\"")
        || text.contains("__")
        || text.contains("--")
        || text.contains("[[")
        || text.contains("<color")
        || text.contains("<size")
        || text.contains("<b>")
        || text.contains("<B>")
        || text.contains("<i>")
        || text.contains("<I>")
        || text.contains("<u>")
        || text.contains("<U>")
        || text.contains("<&")
}

fn extract_salt_icon_names(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("<&") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find('>') else {
            break;
        };
        let name = rest[..end].trim();
        if !name.is_empty() {
            names.push(name.to_string());
        }
        rest = &rest[end + 1..];
    }
    names
}
