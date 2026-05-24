use crate::scene::TextOverflowPolicy;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct Theme {
    pub skinparams: Vec<(String, String)>,
    pub footbox_visible: bool,
    pub text_overflow_policy: TextOverflowPolicy,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            skinparams: Vec::new(),
            footbox_visible: false,
            text_overflow_policy: TextOverflowPolicy::WrapAndGrow,
        }
    }
}

impl Theme {
    pub fn new() -> Self {
        Self {
            skinparams: Vec::new(),
            footbox_visible: true,
            text_overflow_policy: TextOverflowPolicy::WrapAndGrow,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceStyle {
    pub arrow_color: String,
    pub lifeline_border_color: String,
    pub participant_background_color: String,
    pub participant_border_color: String,
    /// Explicit font color for participant header text. `None` means auto-detect
    /// from the participant background (dark bg -> white, light bg -> black).
    pub participant_font_color: Option<String>,
    pub note_background_color: String,
    pub note_border_color: String,
    pub group_background_color: String,
    pub group_border_color: String,
    pub round_corner: i32,
    pub shadowing: bool,
    pub default_font_name: Option<String>,
    pub default_font_size: Option<u32>,
    pub background_color: Option<String>,
    pub text_alignment: TextAlignment,
    // --- Extended skinparams (#182 wishlist) ---
    /// Horizontal gap (px) between participant header boxes.
    pub participant_padding: Option<i32>,
    /// Padding (px) around `box ... end box` groups.
    pub box_padding: Option<i32>,
    /// Alignment of sequence message labels (left/center/right).
    pub message_align: MessageAlign,
    /// Whether to place the response message label below the arrow.
    pub response_message_below_arrow: bool,
    /// Stroke width (px) for lifeline dashed lines.
    pub lifeline_thickness: Option<i32>,
    /// Override color for sequence message arrow lines.
    pub message_line_color: Option<String>,
    /// Background color for `ref` group boxes.
    pub reference_background_color: Option<String>,
    /// Border color for `ref` group boxes.
    pub reference_border_color: Option<String>,
    /// Font color for group header labels.
    pub group_header_font_color: Option<String>,
    /// Font style for group header labels (normal/bold/italic).
    pub group_header_font_style: GroupHeaderFontStyle,
    /// Allow long message labels to span beyond the sender/receiver gap in teoz-style layouts.
    pub sequence_message_span: bool,
    /// When `true`, arrows and lifelines are rendered with an SVG hand-drawn
    /// (sketchy) filter so they appear wobbly/irregular instead of perfectly
    /// straight. Set automatically for the `sketchy` and `sketchy-outline`
    /// themes.
    pub hand_drawn: bool,
    /// When `true`, lifelines use `nosolid` strategy: the participant head box
    /// is shown but there is no solid activation box drawn on the lifeline.
    /// Corresponds to `skinparam lifelineStrategy nosolid` (feature 1.40.2).
    pub lifeline_nosolid: bool,
    /// When `true`, the entire diagram is rendered with a sepia CSS filter
    /// (`filter:sepia(1)`). Controlled by `skinparam sepia true/false`.
    pub sepia: bool,
}

/// Alignment of sequence message labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MessageAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// Font style for group header labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GroupHeaderFontStyle {
    #[default]
    Normal,
    Bold,
    Italic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonochromeMode {
    True,
    Reverse,
}

impl MonochromeMode {
    pub(crate) const fn ink(self) -> &'static str {
        match self {
            Self::True => "#000000",
            Self::Reverse => "#ffffff",
        }
    }

    pub(crate) const fn paper(self) -> &'static str {
        match self {
            Self::True => "#ffffff",
            Self::Reverse => "#000000",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlignment {
    #[default]
    Center,
    Left,
    Right,
}

impl TextAlignment {
    pub fn as_text_anchor(self) -> &'static str {
        match self {
            TextAlignment::Center => "middle",
            TextAlignment::Left => "start",
            TextAlignment::Right => "end",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MindMapStyle {
    pub depth_styles: BTreeMap<usize, MindMapDepthStyle>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MindMapDepthStyle {
    pub background_color: Option<String>,
    pub font_color: Option<String>,
    pub border_color: Option<String>,
}

impl Default for SequenceStyle {
    fn default() -> Self {
        Self {
            arrow_color: "#111".to_string(),
            lifeline_border_color: "#555".to_string(),
            participant_background_color: "#f6f6f6".to_string(),
            participant_border_color: "#111".to_string(),
            participant_font_color: None,
            note_background_color: "#fff8c4".to_string(),
            note_border_color: "#111".to_string(),
            group_background_color: "#fafafa".to_string(),
            group_border_color: "#666".to_string(),
            round_corner: 4,
            shadowing: false,
            default_font_name: None,
            default_font_size: None,
            background_color: None,
            text_alignment: TextAlignment::Center,
            participant_padding: None,
            box_padding: None,
            message_align: MessageAlign::Left,
            response_message_below_arrow: false,
            lifeline_thickness: None,
            message_line_color: None,
            reference_background_color: None,
            reference_border_color: None,
            group_header_font_color: None,
            group_header_font_style: GroupHeaderFontStyle::Normal,
            sequence_message_span: false,
            hand_drawn: false,
            lifeline_nosolid: false,
            sepia: false,
        }
    }
}

impl SequenceStyle {
    /// Return the font color for participant header text.
    /// Uses explicit `participant_font_color` if set; otherwise auto-detects from background luminance.
    pub fn participant_font_color_resolved(&self) -> &str {
        if let Some(ref c) = self.participant_font_color {
            return c.as_str();
        }
        if hex_color_is_dark(&self.participant_background_color) {
            "#ffffff"
        } else {
            "#111111"
        }
    }
}

/// Returns `true` when the hex color string represents a dark color (WCAG luminance < 0.179).
pub fn hex_color_is_dark(hex: &str) -> bool {
    let hex = hex.trim_start_matches('#');
    let (r, g, b) = match hex.len() {
        3 => {
            let digits: Vec<u8> = hex
                .chars()
                .filter_map(|c| u8::from_str_radix(&c.to_string().repeat(2), 16).ok())
                .collect();
            if digits.len() != 3 {
                return false;
            }
            (digits[0], digits[1], digits[2])
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(128);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(128);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(128);
            (r, g, b)
        }
        _ => return false,
    };
    fn linearise(c: u8) -> f64 {
        let s = c as f64 / 255.0;
        if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055_f64).powf(2.4)
        }
    }
    let lum = 0.2126 * linearise(r) + 0.7152 * linearise(g) + 0.0722 * linearise(b);
    lum < 0.179
}
