use super::text::{estimate_salt_text_width, salt_text_line_count};

pub(super) enum SaltCellRender {
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
    /// Password input field — placeholder text is masked with `●` characters.
    /// The `label` is the visible hint text (shown masked). Used via `"*hint*"` syntax.
    Password(String),
    /// Horizontal range slider.
    /// `min`/`max` define the range; `value` is the current thumb position.
    /// Syntax: `{slider:min,max,value}` or `{slider:value}`.
    Slider {
        min: i32,
        max: i32,
        value: i32,
    },
}

impl SaltCellRender {
    pub(super) fn text(&self) -> &str {
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
            Self::Password(label) => label,
            Self::Slider { .. } => "",
        }
    }

    pub(super) fn intrinsic_width(&self) -> i32 {
        match self {
            Self::Input(text) => estimate_salt_text_width(text) + 29,
            Self::Button(text) => (estimate_salt_text_width(text) + 16).max(36),
            Self::Combo(text) => estimate_salt_text_width(text) + 23,
            Self::OpenCombo { label, items } => {
                let label_w = estimate_salt_text_width(label) + 23;
                let items_w = items
                    .iter()
                    .map(|i| estimate_salt_text_width(i) + 16)
                    .max()
                    .unwrap_or(0);
                label_w.max(items_w)
            }
            Self::ProgressBar { .. } => 100,
            Self::CheckboxChecked(text) | Self::CheckboxUnchecked(text) => {
                20 + estimate_salt_text_width(text)
            }
            Self::RadioOn(text) | Self::RadioOff(text) => 20 + estimate_salt_text_width(text),
            Self::MenuBar(items) => items
                .iter()
                .map(|item| estimate_salt_text_width(item) + 24)
                .sum(),
            Self::TabBar { tabs, .. } => tabs
                .iter()
                .map(|tab| estimate_salt_text_width(tab) + 24)
                .sum(),
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
            Self::Password(text) => estimate_salt_text_width(text) + 29,
            Self::Slider { .. } => 120,
            _ => estimate_salt_text_width(self.text()) + 20,
        }
    }

    pub(super) fn is_table_like(&self) -> bool {
        matches!(self, Self::Header(_) | Self::TableEmpty | Self::TableSpan)
    }

    /// Return the minimum row height required for this cell (normally 20px).
    pub(super) fn intrinsic_height(&self) -> i32 {
        match self {
            Self::OpenCombo { items, .. } => {
                // Header combo (19px) + padding (2+2) + items list (16px * n) + 4px margin.
                23 + (items.len() as i32) * 16 + 4
            }
            Self::Label(text)
            | Self::Header(text)
            | Self::Input(text)
            | Self::Button(text)
            | Self::Combo(text)
            | Self::CheckboxChecked(text)
            | Self::CheckboxUnchecked(text)
            | Self::RadioOn(text)
            | Self::RadioOff(text)
            | Self::TreeItem { label: text, .. }
            | Self::TextAreaLine { text, .. }
            | Self::GroupBox(text) => {
                let line_count = salt_text_line_count(text);
                if line_count <= 1 {
                    20
                } else {
                    8 + (line_count as i32 * 14)
                }
            }
            Self::MenuBar(items) if items.len() > 4 => 24 + ((items.len() - 1) as i32 * 16),
            // Slider needs extra room for the thumb (6px radius) + min/max labels below.
            Self::Slider { .. } => 36,
            _ => 20,
        }
    }
}
