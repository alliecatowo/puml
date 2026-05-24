use super::SkinParamSupport;
use crate::theme::color::parse_color_value;

/// Style overrides for state diagrams.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateStyle {
    pub background_color: String,
    pub border_color: String,
    pub arrow_color: String,
    pub start_color: String,
    pub font_color: String,
    pub font_size: Option<u32>,
}

impl Default for StateStyle {
    fn default() -> Self {
        Self {
            background_color: "#f6f6f6".to_string(),
            border_color: "#1e293b".to_string(),
            arrow_color: "#1e293b".to_string(),
            start_color: "#0f172a".to_string(),
            font_color: "#0f172a".to_string(),
            font_size: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateSkinParamValue {
    BackgroundColor(String),
    BorderColor(String),
    ArrowColor(String),
    StartColor(String),
    FontColor(String),
    FontSize(u32),
}

pub fn classify_state_skinparam(key: &str, value: &str) -> SkinParamSupport<StateSkinParamValue> {
    let normalized = key.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "backgroundcolor" | "statebackgroundcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(StateSkinParamValue::BackgroundColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "bordercolor" | "statebordercolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(StateSkinParamValue::BorderColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "arrowcolor" | "statearrowcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(StateSkinParamValue::ArrowColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "statestartcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(StateSkinParamValue::StartColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontcolor" | "statefontcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(StateSkinParamValue::FontColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontsize" | "statefontsize" => {
            if let Ok(n) = value.trim().parse::<u32>() {
                SkinParamSupport::SupportedWithValue(StateSkinParamValue::FontSize(n))
            } else {
                SkinParamSupport::UnsupportedValue
            }
        }
        "statefontname"
        | "statestereotypefontcolor"
        | "statestereotypefontsize"
        | "statestereotypefontname"
        | "stateattributefontcolor"
        | "stateattributefontsize" => SkinParamSupport::SupportedNoop,
        _ => SkinParamSupport::UnsupportedKey,
    }
}
