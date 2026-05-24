use super::SkinParamSupport;
use crate::theme::color::parse_color_value;

// ─── Timing-family skinparam support ────────────────────────────────────────

/// Style overrides for timing diagrams.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimingStyle {
    pub background_color: String,
    pub axis_color: String,
    pub grid_color: String,
    pub signal_background_color: String,
    pub signal_border_color: String,
    pub arrow_color: String,
    pub font_color: String,
}

impl Default for TimingStyle {
    fn default() -> Self {
        Self {
            background_color: "#ffffff".to_string(),
            axis_color: "#64748b".to_string(),
            grid_color: "#cbd5e1".to_string(),
            signal_background_color: "#f8fafc".to_string(),
            signal_border_color: "#0f172a".to_string(),
            arrow_color: "#0ea5e9".to_string(),
            font_color: "#0f172a".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimingSkinParamValue {
    BackgroundColor(String),
    AxisColor(String),
    GridColor(String),
    SignalBackgroundColor(String),
    SignalBorderColor(String),
    ArrowColor(String),
    FontColor(String),
}

pub fn classify_timing_skinparam(key: &str, value: &str) -> SkinParamSupport<TimingSkinParamValue> {
    let normalized = key.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "backgroundcolor" | "timingbackgroundcolor" | "timingdiagrambackgroundcolor" => {
            parse_color_value(value)
                .map(|c| {
                    SkinParamSupport::SupportedWithValue(TimingSkinParamValue::BackgroundColor(c))
                })
                .unwrap_or(SkinParamSupport::UnsupportedValue)
        }
        "timingaxiscolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(TimingSkinParamValue::AxisColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "timinggridcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(TimingSkinParamValue::GridColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "timingsignalbackgroundcolor" | "timingparticipantbackgroundcolor" => {
            parse_color_value(value)
                .map(|c| {
                    SkinParamSupport::SupportedWithValue(
                        TimingSkinParamValue::SignalBackgroundColor(c),
                    )
                })
                .unwrap_or(SkinParamSupport::UnsupportedValue)
        }
        "timingsignalbordercolor" | "timingparticipantbordercolor" => parse_color_value(value)
            .map(|c| {
                SkinParamSupport::SupportedWithValue(TimingSkinParamValue::SignalBorderColor(c))
            })
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "arrowcolor" | "timingarrowcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(TimingSkinParamValue::ArrowColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontcolor" | "timingfontcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(TimingSkinParamValue::FontColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "timingfontsize" | "timingfontname" => SkinParamSupport::SupportedNoop,
        _ => SkinParamSupport::UnsupportedKey,
    }
}
