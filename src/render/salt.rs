use super::*;

pub fn render_salt_svg(document: &FamilyDocument) -> String {
    const DEFAULT_CELL_H: i32 = 20;
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

    // Per-row heights (support variable-height cells like open combos).
    let row_heights: Vec<i32> = rows
        .iter()
        .map(|row| {
            row.iter()
                .map(SaltCellRender::intrinsic_height)
                .max()
                .unwrap_or(DEFAULT_CELL_H)
        })
        .collect();

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
    let total_h = row_heights.iter().sum::<i32>() + MARGIN * 2;

    // Header/footer/title heights.
    let header_h = document.header.as_deref().map(|_| 20i32).unwrap_or(0);
    let title_h = document.title.as_deref().map(|_| 22i32).unwrap_or(0);
    let footer_h = document.footer.as_deref().map(|_| 20i32).unwrap_or(0);
    let caption_h = document.caption.as_deref().map(|_| 18i32).unwrap_or(0);
    let legend_h = document.legend.as_deref().map(|_| 18i32).unwrap_or(0);
    let top_extra = header_h + title_h;
    let bottom_extra = footer_h + caption_h + legend_h;
    let svg_h = total_h + top_extra + bottom_extra;

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
            MARGIN + top_extra,
            total_w - MARGIN * 2,
            total_h - MARGIN * 2,
            style.panel_fill,
            style.border_color
        ));
    }

    // Header (top of diagram, above title).
    if let Some(header) = &document.header {
        salt_text(
            &mut out,
            MARGIN,
            MARGIN + 14,
            &format!(
                "font-family=\"{}\" font-size=\"11\" fill=\"{}\"",
                style.font_family, style.muted_text_color
            ),
            header,
            &style.muted_text_color,
        );
    }

    // Title (below header, above content).
    if let Some(title) = &document.title {
        let ty = MARGIN + header_h + 16;
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"{}\" font-size=\"13\" font-weight=\"600\" fill=\"{}\" text-anchor=\"middle\">{}</text>",
            total_w / 2,
            ty,
            style.font_family,
            style.text_color,
            escape_text(title)
        ));
    }

    // Draw rows and cells.
    let mut current_y = MARGIN + top_extra;
    for (row_idx, cells) in rows.iter().enumerate() {
        let row_h = row_heights[row_idx];
        let row_y = current_y;
        current_y += row_h;

        if is_salt_separator_row(cells) {
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                MARGIN + 4,
                row_y + row_h / 2,
                total_w - MARGIN - 4,
                row_y + row_h / 2,
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
                    h: row_h,
                },
                cell.colspan,
                &style,
            );
            col_x += cell.width;
        }

        // Row separator line (skip the last row).
        if table_like && row_idx + 1 < rows.len() {
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
                MARGIN,
                current_y,
                total_w - MARGIN,
                current_y,
                style.grid_color
            ));
        }
    }

    // Column separator lines.
    if table_like {
        let mut col_x = MARGIN;
        for (col_idx, w) in col_widths.iter().enumerate() {
            col_x += w;
            if col_idx + 1 < col_count {
                out.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
                    col_x,
                    MARGIN + top_extra,
                    col_x,
                    MARGIN + top_extra + total_h - MARGIN * 2,
                    style.grid_color
                ));
            }
        }
    }

    // Footer (below content).
    let footer_y = MARGIN + top_extra + total_h;
    if let Some(footer) = &document.footer {
        salt_text(
            &mut out,
            MARGIN,
            footer_y + 14,
            &format!(
                "font-family=\"{}\" font-size=\"11\" fill=\"{}\"",
                style.font_family, style.muted_text_color
            ),
            footer,
            &style.muted_text_color,
        );
    }

    // Caption (below footer).
    if let Some(caption) = &document.caption {
        let cy = footer_y + footer_h + 14;
        salt_text(
            &mut out,
            total_w / 2,
            cy,
            &format!(
                "font-family=\"{}\" font-size=\"11\" fill=\"{}\" text-anchor=\"middle\"",
                style.font_family, style.muted_text_color
            ),
            caption,
            &style.muted_text_color,
        );
    }

    // Legend (below caption).
    if let Some(legend) = &document.legend {
        let ly = footer_y + footer_h + caption_h + 14;
        salt_text(
            &mut out,
            MARGIN,
            ly,
            &format!(
                "font-family=\"{}\" font-size=\"11\" font-style=\"italic\" fill=\"{}\"",
                style.font_family, style.muted_text_color
            ),
            legend,
            &style.muted_text_color,
        );
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
    /// Expanded combo/droplist showing a list of items below the field.
    /// `label` is the selected/header value; `items` are the dropdown entries.
    OpenCombo {
        label: String,
        items: Vec<String>,
    },
    /// Progress bar: `[=====   ]` style. `fill_ratio` in [0.0, 1.0].
    ProgressBar {
        fill_ratio: f32,
    },
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
            Self::OpenCombo { label, .. } => label,
            Self::ProgressBar { .. } => "",
        }
    }

    fn intrinsic_width(&self) -> i32 {
        match self {
            Self::Input(text) => estimate_text_width(text) + 29,
            Self::Button(text) => (estimate_text_width(text) + 16).max(36),
            Self::Combo(text) => estimate_text_width(text) + 23,
            Self::OpenCombo { label, items } => {
                let label_w = estimate_text_width(label) + 23;
                let items_w = items
                    .iter()
                    .map(|i| estimate_text_width(i) + 16)
                    .max()
                    .unwrap_or(0);
                label_w.max(items_w)
            }
            Self::ProgressBar { .. } => 100,
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

    /// Return the minimum row height required for this cell (normally 20px).
    fn intrinsic_height(&self) -> i32 {
        match self {
            Self::OpenCombo { items, .. } => {
                // Header combo (19px) + padding (2+2) + items list (16px * n) + 4px margin.
                23 + (items.len() as i32) * 16 + 4
            }
            _ => 20,
        }
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

/// Parse `^label^^item1^^item2^` (open / expanded droplist).
/// Returns `(label, items)` if the pattern matches a multi-item droplist,
/// or `None` for a plain closed `^label^` combo or a non-combo string.
fn parse_salt_open_combo(line: &str) -> Option<(String, Vec<String>)> {
    let trimmed = line.trim();
    if !trimmed.starts_with('^') {
        return None;
    }
    // Split on `^^` — gives `["", "item1", "item2", ...]` for `^label^^item1^^item2^`
    // but also gives `["label"]` for `^label^`.
    let parts: Vec<&str> = trimmed.split("^^").collect();
    if parts.len() < 2 {
        // Just `^label^` — plain combo handled elsewhere.
        return None;
    }
    let label = parts[0].trim_start_matches('^').to_string();
    let items: Vec<String> = parts[1..]
        .iter()
        .map(|p| p.trim_end_matches('^').to_string())
        .filter(|s| !s.is_empty())
        .collect();
    Some((label, items))
}

/// Parse a progress bar: `[=====   ]` or `[========]`.
/// Returns fill ratio [0.0, 1.0] if the cell looks like a progress bar.
fn parse_salt_progress_bar(line: &str) -> Option<f32> {
    let trimmed = line.trim();
    // Must be enclosed in `[...]`
    let inner = trimmed.strip_prefix('[')?.strip_suffix(']')?;
    if inner.is_empty() {
        return None;
    }
    // Content must consist only of `=` and space characters (at least one `=`).
    if !inner.chars().all(|c| c == '=' || c == ' ') {
        return None;
    }
    let filled = inner.chars().filter(|&c| c == '=').count();
    let total = inner.len();
    if total == 0 {
        return None;
    }
    Some(filled as f32 / total as f32)
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

#[derive(Debug, Clone, Copy)]
struct SaltCellBox {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

/// Render a single salt cell into SVG, appending to `out`.
fn render_salt_cell_svg(
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
        SaltCellRender::OpenCombo { label, items } => {
            // Draw the closed-combo header at the top, then list items below.
            let combo_w = salt_combo_width(label).min(w - pad * 2).max(28);
            let combo_h = 19;
            // Combo field at the top of the cell.
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
            // Arrow indicator (pointing up = open).
            let divider_x = x + pad + combo_w - 11;
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                divider_x, y + 2, divider_x, y + 2 + combo_h, style.border_color
            ));
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                divider_x + 3, y + 2 + combo_h - 6,
                divider_x + 9, y + 2 + combo_h - 6,
                divider_x + 6, y + 2 + 5,
                style.border_color, style.border_color
            ));
            // Drop-down item list below the combo field.
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
                    let iy = list_y + (i as i32) * item_h + 12;
                    salt_text(
                        out,
                        x + pad + 4,
                        iy,
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
        SaltCellRender::ProgressBar { fill_ratio } => {
            let bar_h = 10;
            let bar_y = y + (h - bar_h) / 2;
            let bar_w = (w - pad * 2).max(20);
            let filled_w = ((bar_w as f32) * fill_ratio.clamp(0.0, 1.0)).round() as i32;
            // Track (empty) background.
            out.push_str(&format!(
                "<rect data-salt-widget=\"progress\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\" rx=\"3\" ry=\"3\"/>",
                x + pad, bar_y, bar_w, bar_h, style.panel_fill, style.border_color
            ));
            // Filled portion.
            if filled_w > 0 {
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" rx=\"3\" ry=\"3\"/>",
                    x + pad, bar_y, filled_w, bar_h, style.accent_fill
                ));
            }
        }
    }
    if colspan > 1 {
        out.push_str("</g>");
    }
}
