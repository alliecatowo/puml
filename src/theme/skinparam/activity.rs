use super::SkinParamSupport;
use crate::theme::color::parse_color_value;

// ─── Activity-family skinparam support ───────────────────────────────────────

/// Style overrides for activity diagrams.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityStyle {
    pub background_color: String,
    pub border_color: String,
    pub diamond_color: String,
    pub fork_color: String,
    pub font_color: String,
    pub arrow_color: String,
}

impl Default for ActivityStyle {
    fn default() -> Self {
        Self {
            background_color: "#ecfdf5".to_string(),
            border_color: "#047857".to_string(),
            diamond_color: "#fef9c3".to_string(),
            fork_color: "#0f172a".to_string(),
            font_color: "#0f172a".to_string(),
            arrow_color: "#0f172a".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivitySkinParamValue {
    BackgroundColor(String),
    BorderColor(String),
    DiamondBackgroundColor(String),
    BarColor(String),
    FontColor(String),
    ArrowColor(String),
}

pub fn classify_activity_skinparam(
    key: &str,
    value: &str,
) -> SkinParamSupport<ActivitySkinParamValue> {
    let normalized = key.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "backgroundcolor" | "activitybackgroundcolor" | "activitypartitionbackgroundcolor" => {
            parse_color_value(value)
                .map(|c| {
                    SkinParamSupport::SupportedWithValue(ActivitySkinParamValue::BackgroundColor(c))
                })
                .unwrap_or(SkinParamSupport::UnsupportedValue)
        }
        "bordercolor"
        | "activitybordercolor"
        | "activitypartitionbordercolor"
        | "swimlanebordercolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(ActivitySkinParamValue::BorderColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "activitydiamondbackgroundcolor" | "activitydiamondcolor" => parse_color_value(value)
            .map(|c| {
                SkinParamSupport::SupportedWithValue(
                    ActivitySkinParamValue::DiamondBackgroundColor(c),
                )
            })
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "activitybarcolor" | "activitystartcolor" | "activityendcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(ActivitySkinParamValue::BarColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontcolor" | "activityfontcolor" | "swimlanefontcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(ActivitySkinParamValue::FontColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "arrowcolor" | "activityarrowcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(ActivitySkinParamValue::ArrowColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "activityfontsize"
        | "activityfontname"
        | "activityborderthickness"
        | "activitypartitionfontcolor"
        | "activitypartitionfontsize"
        | "swimlanefontsize" => SkinParamSupport::SupportedNoop,
        _ => SkinParamSupport::UnsupportedKey,
    }
}
