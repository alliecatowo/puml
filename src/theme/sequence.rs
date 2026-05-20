use super::color::{hex_color_is_dark, parse_color_value};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SequenceSkinParamValue {
    FootboxVisible(bool),
    ArrowColor(String),
    LifelineBorderColor(String),
    ParticipantBackgroundColor(String),
    ParticipantBorderColor(String),
    NoteBackgroundColor(String),
    NoteBorderColor(String),
    GroupBackgroundColor(String),
    GroupBorderColor(String),
    RoundCorner(i32),
    Shadowing(bool),
    DefaultFontName(String),
    DefaultFontSize(u32),
    BackgroundColor(String),
    DefaultTextAlignment(TextAlignment),
    // --- Extended skinparams (#182 wishlist) ---
    ParticipantPadding(i32),
    BoxPadding(i32),
    MessageAlign(MessageAlign),
    ResponseMessageBelowArrow(bool),
    LifelineThickness(i32),
    MessageLineColor(String),
    ReferenceBackgroundColor(String),
    ReferenceBorderColor(String),
    GroupHeaderFontColor(String),
    GroupHeaderFontStyle(GroupHeaderFontStyle),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SequenceSkinParamSupport {
    SupportedNoop,
    SupportedWithValue(SequenceSkinParamValue),
    UnsupportedKey,
    UnsupportedValue,
}

pub fn classify_sequence_skinparam(key: &str, value: &str) -> SequenceSkinParamSupport {
    let normalized_key = key.trim().to_ascii_lowercase();
    match normalized_key.as_str() {
        "maxmessagesize" => SequenceSkinParamSupport::SupportedNoop,
        "footbox" | "sequencefootbox" => parse_footbox_value(value)
            .map(SequenceSkinParamSupport::SupportedWithValue)
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "arrowcolor" | "sequencearrowcolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::ArrowColor(
                    color,
                ))
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "lifelinebordercolor" | "sequencelifelinebordercolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::LifelineBorderColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "participantbackgroundcolor" | "sequenceparticipantbackgroundcolor" => {
            parse_color_value(value)
                .map(|color| {
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ParticipantBackgroundColor(color),
                    )
                })
                .unwrap_or(SequenceSkinParamSupport::UnsupportedValue)
        }
        "participantbordercolor" | "sequenceparticipantbordercolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::ParticipantBorderColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "notebackgroundcolor" | "sequencenotebackgroundcolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::NoteBackgroundColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "notebordercolor" | "sequencenotebordercolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::NoteBorderColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "groupbackgroundcolor" | "sequencegroupbackgroundcolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::GroupBackgroundColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "groupbordercolor" | "sequencegroupbordercolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::GroupBorderColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "roundcorner" => {
            if let Ok(n) = value.trim().parse::<i32>() {
                SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::RoundCorner(n))
            } else {
                SequenceSkinParamSupport::UnsupportedValue
            }
        }
        "shadowing" => {
            let lower = value.trim().to_ascii_lowercase();
            let enabled = match lower.as_str() {
                "true" | "yes" | "on" => true,
                "false" | "no" | "off" => false,
                _ => return SequenceSkinParamSupport::UnsupportedValue,
            };
            SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::Shadowing(enabled))
        }
        "defaultfontname" => {
            let name = value.trim();
            if name.is_empty() {
                SequenceSkinParamSupport::UnsupportedValue
            } else {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::DefaultFontName(name.to_string()),
                )
            }
        }
        "defaultfontsize" => {
            if let Ok(n) = value.trim().parse::<u32>() {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::DefaultFontSize(n),
                )
            } else {
                SequenceSkinParamSupport::UnsupportedValue
            }
        }
        "backgroundcolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::BackgroundColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "defaulttextalignment" => {
            let lower = value.trim().to_ascii_lowercase();
            let alignment = match lower.as_str() {
                "center" => TextAlignment::Center,
                "left" => TextAlignment::Left,
                "right" => TextAlignment::Right,
                _ => return SequenceSkinParamSupport::UnsupportedValue,
            };
            SequenceSkinParamSupport::SupportedWithValue(
                SequenceSkinParamValue::DefaultTextAlignment(alignment),
            )
        }
        // --- Extended skinparams (#182 wishlist) ---
        "participantpadding" => {
            if let Ok(n) = value.trim().parse::<i32>() {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::ParticipantPadding(n),
                )
            } else {
                SequenceSkinParamSupport::UnsupportedValue
            }
        }
        "boxpadding" => {
            if let Ok(n) = value.trim().parse::<i32>() {
                SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::BoxPadding(n))
            } else {
                SequenceSkinParamSupport::UnsupportedValue
            }
        }
        "sequencemessagealign" => {
            let lower = value.trim().to_ascii_lowercase();
            let align = match lower.as_str() {
                "left" => MessageAlign::Left,
                "center" => MessageAlign::Center,
                "right" => MessageAlign::Right,
                _ => return SequenceSkinParamSupport::UnsupportedValue,
            };
            SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::MessageAlign(
                align,
            ))
        }
        "responsemessagebelowarrow" | "sequenceresponsemessagebelowarrow" => {
            let lower = value.trim().to_ascii_lowercase();
            let enabled = match lower.as_str() {
                "true" | "yes" | "on" => true,
                "false" | "no" | "off" => false,
                _ => return SequenceSkinParamSupport::UnsupportedValue,
            };
            SequenceSkinParamSupport::SupportedWithValue(
                SequenceSkinParamValue::ResponseMessageBelowArrow(enabled),
            )
        }
        "sequencelifelinethickness" => {
            if let Ok(n) = value.trim().parse::<i32>() {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::LifelineThickness(n),
                )
            } else {
                SequenceSkinParamSupport::UnsupportedValue
            }
        }
        "messagelinecolor" | "sequencemessagelinecolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::MessageLineColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "referencebackgroundcolor" | "sequencereferencebackgroundcolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::ReferenceBackgroundColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "referencebordercolor" | "sequencereferencebordercolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::ReferenceBorderColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "sequencegroupheaderfontcolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::GroupHeaderFontColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "sequencegroupheaderfontstyle" => {
            let lower = value.trim().to_ascii_lowercase();
            let style = match lower.as_str() {
                "normal" => GroupHeaderFontStyle::Normal,
                "bold" => GroupHeaderFontStyle::Bold,
                "italic" => GroupHeaderFontStyle::Italic,
                _ => return SequenceSkinParamSupport::UnsupportedValue,
            };
            SequenceSkinParamSupport::SupportedWithValue(
                SequenceSkinParamValue::GroupHeaderFontStyle(style),
            )
        }
        _ => SequenceSkinParamSupport::UnsupportedKey,
    }
}

fn parse_footbox_value(value: &str) -> Option<SequenceSkinParamValue> {
    let normalized = value.trim().to_ascii_lowercase();
    let visible = match normalized.as_str() {
        "show" | "true" | "yes" | "on" => true,
        "hide" | "false" | "no" | "off" => false,
        _ => return None,
    };
    Some(SequenceSkinParamValue::FootboxVisible(visible))
}
