pub(super) struct SaltRenderStyle {
    pub(super) canvas_fill: String,
    pub(super) panel_fill: String,
    pub(super) header_fill: String,
    pub(super) input_fill: String,
    pub(super) button_fill: String,
    pub(super) menu_fill: String,
    pub(super) tab_fill: String,
    pub(super) scroll_fill: String,
    pub(super) checkbox_fill: String,
    pub(super) radio_fill: String,
    pub(super) accent_fill: String,
    pub(super) border_color: String,
    pub(super) grid_color: String,
    pub(super) text_color: String,
    pub(super) header_text_color: String,
    pub(super) input_text_color: String,
    pub(super) button_text_color: String,
    pub(super) muted_text_color: String,
    pub(super) font_family: &'static str,
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

    pub(super) fn set_scoped(&mut self, scope: Option<&str>, key: &str, value: &str) -> bool {
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
    crate::theme::color::resolve_css3_color_or_original(value)
}

pub(super) fn apply_salt_style_directive(line: &str, style: &mut SaltRenderStyle) -> bool {
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
