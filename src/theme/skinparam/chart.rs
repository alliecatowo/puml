use super::SkinParamSupport;
use crate::theme::color::parse_color_value;

// ─── Chart-family skinparam support ─────────────────────────────────────────

/// Style overrides for chart diagrams.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChartStyle {
    pub background_color: String,
    pub axis_color: String,
    pub grid_color: String,
    pub series_color: String,
    pub bar_color: String,
    pub line_color: String,
    pub pie_border_color: String,
    pub font_color: String,
}

impl Default for ChartStyle {
    fn default() -> Self {
        Self {
            background_color: "#ffffff".to_string(),
            axis_color: "#0f172a".to_string(),
            grid_color: "#e2e8f0".to_string(),
            series_color: "#1d4ed8".to_string(),
            bar_color: "#1d4ed8".to_string(),
            line_color: "#1d4ed8".to_string(),
            pie_border_color: "#0f172a".to_string(),
            font_color: "#0f172a".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChartSkinParamValue {
    BackgroundColor(String),
    AxisColor(String),
    GridColor(String),
    SeriesColor(String),
    BarColor(String),
    LineColor(String),
    PieBorderColor(String),
    FontColor(String),
}

pub fn classify_chart_skinparam(key: &str, value: &str) -> SkinParamSupport<ChartSkinParamValue> {
    let normalized = key.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "backgroundcolor" | "chartbackgroundcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(ChartSkinParamValue::BackgroundColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "axiscolor" | "chartaxiscolor" | "chartaxislinecolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(ChartSkinParamValue::AxisColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "gridcolor" | "chartgridcolor" | "chartgridlinecolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(ChartSkinParamValue::GridColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "chartseriescolor" | "seriescolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(ChartSkinParamValue::SeriesColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "chartbarcolor" | "barcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(ChartSkinParamValue::BarColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "chartlinecolor" | "linecolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(ChartSkinParamValue::LineColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "chartpiebordercolor" | "piebordercolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(ChartSkinParamValue::PieBorderColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontcolor" | "chartfontcolor" | "chartlabelfontcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(ChartSkinParamValue::FontColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "chartfontsize" | "chartfontname" | "legendfontcolor" | "legendfontsize" => {
            SkinParamSupport::SupportedNoop
        }
        _ => SkinParamSupport::UnsupportedKey,
    }
}
