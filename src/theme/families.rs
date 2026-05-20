use super::color::parse_color_value;

// ─── Class-family skinparam support ─────────────────────────────────────────

/// Style overrides for class/object/usecase diagrams.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassStyle {
    pub background_color: String,
    pub border_color: String,
    pub header_color: String,
    pub member_color: String,
    pub font_color: String,
    pub arrow_color: String,
    pub font_size: Option<u32>,
    pub font_name: Option<String>,
}

impl Default for ClassStyle {
    fn default() -> Self {
        Self {
            background_color: "#ffffff".to_string(),
            border_color: "#1e293b".to_string(),
            header_color: "#dbeafe".to_string(),
            member_color: "#334155".to_string(),
            font_color: "#0f172a".to_string(),
            arrow_color: "#1e293b".to_string(),
            font_size: None,
            font_name: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClassSkinParamValue {
    BackgroundColor(String),
    BorderColor(String),
    HeaderBackgroundColor(String),
    MemberFontColor(String),
    FontColor(String),
    ArrowColor(String),
    FontSize(u32),
    FontName(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkinParamSupport<V> {
    SupportedNoop,
    SupportedWithValue(V),
    UnsupportedKey,
    UnsupportedValue,
}

pub fn classify_class_skinparam(key: &str, value: &str) -> SkinParamSupport<ClassSkinParamValue> {
    let normalized = key.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "backgroundcolor"
        | "classbackgroundcolor"
        | "objectbackgroundcolor"
        | "usecasebackgroundcolor"
        | "actorbackgroundcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(ClassSkinParamValue::BackgroundColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "bordercolor" | "classbordercolor" | "objectbordercolor" | "usecasebordercolor"
        | "actorbordercolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(ClassSkinParamValue::BorderColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "classheaderbackgroundcolor" => parse_color_value(value)
            .map(|c| {
                SkinParamSupport::SupportedWithValue(ClassSkinParamValue::HeaderBackgroundColor(c))
            })
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "classmemberfontcolor" | "classattributefontcolor" | "classmethodfontcolor" => {
            parse_color_value(value)
                .map(|c| {
                    SkinParamSupport::SupportedWithValue(ClassSkinParamValue::MemberFontColor(c))
                })
                .unwrap_or(SkinParamSupport::UnsupportedValue)
        }
        "fontcolor" | "classfontcolor" | "objectfontcolor" | "usecasefontcolor"
        | "actorfontcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(ClassSkinParamValue::FontColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "arrowcolor" | "classarrowcolor" | "objectarrowcolor" | "usecasearrowcolor" => {
            parse_color_value(value)
                .map(|c| SkinParamSupport::SupportedWithValue(ClassSkinParamValue::ArrowColor(c)))
                .unwrap_or(SkinParamSupport::UnsupportedValue)
        }
        "fontsize" | "classfontsize" | "objectfontsize" | "usecasefontsize" | "actorfontsize" => {
            if let Ok(n) = value.trim().parse::<u32>() {
                SkinParamSupport::SupportedWithValue(ClassSkinParamValue::FontSize(n))
            } else {
                SkinParamSupport::UnsupportedValue
            }
        }
        "classfontname" | "objectfontname" | "usecasefontname" | "actorfontname" => {
            let name = value.trim();
            if name.is_empty() {
                SkinParamSupport::UnsupportedValue
            } else {
                SkinParamSupport::SupportedWithValue(ClassSkinParamValue::FontName(
                    name.to_string(),
                ))
            }
        }
        "classstereotypefontcolor"
        | "classstereotypefontsize"
        | "classstereotypefontname"
        | "classattributefontsize"
        | "classmethodfontsize"
        | "objectstereotypefontcolor"
        | "usecasestereotypefontcolor"
        | "actorstereotypefontcolor"
        | "roundcorner"
        | "shadowing" => SkinParamSupport::SupportedNoop,
        _ => SkinParamSupport::UnsupportedKey,
    }
}

// ─── State-family skinparam support ─────────────────────────────────────────

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

// ─── Component-family skinparam support ──────────────────────────────────────

/// Style overrides for component/deployment diagrams.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentStyle {
    pub background_color: String,
    pub border_color: String,
    pub interface_color: String,
    pub font_color: String,
    pub arrow_color: String,
}

impl Default for ComponentStyle {
    fn default() -> Self {
        Self {
            background_color: "#f0f4f8".to_string(),
            border_color: "#1e293b".to_string(),
            interface_color: "#e2e8f0".to_string(),
            font_color: "#0f172a".to_string(),
            arrow_color: "#1e293b".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComponentSkinParamValue {
    BackgroundColor(String),
    BorderColor(String),
    InterfaceColor(String),
    FontColor(String),
    ArrowColor(String),
}

pub fn classify_component_skinparam(
    key: &str,
    value: &str,
) -> SkinParamSupport<ComponentSkinParamValue> {
    let normalized = key.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "backgroundcolor"
        | "componentbackgroundcolor"
        | "deploymentbackgroundcolor"
        | "nodebackgroundcolor"
        | "artifactbackgroundcolor"
        | "databasebackgroundcolor" => parse_color_value(value)
            .map(|c| {
                SkinParamSupport::SupportedWithValue(ComponentSkinParamValue::BackgroundColor(c))
            })
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "bordercolor"
        | "componentbordercolor"
        | "deploymentbordercolor"
        | "nodebordercolor"
        | "artifactbordercolor"
        | "databasebordercolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(ComponentSkinParamValue::BorderColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "interfacebackgroundcolor" | "interfacecolor" | "interfacecirclebackgroundcolor" => {
            parse_color_value(value)
                .map(|c| {
                    SkinParamSupport::SupportedWithValue(ComponentSkinParamValue::InterfaceColor(c))
                })
                .unwrap_or(SkinParamSupport::UnsupportedValue)
        }
        "portbackgroundcolor" | "portcolor" => parse_color_value(value)
            .map(|c| {
                SkinParamSupport::SupportedWithValue(ComponentSkinParamValue::InterfaceColor(c))
            })
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontcolor"
        | "componentfontcolor"
        | "deploymentfontcolor"
        | "nodefontcolor"
        | "artifactfontcolor"
        | "databasefontcolor"
        | "portfontcolor"
        | "interfacefontcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(ComponentSkinParamValue::FontColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "arrowcolor" | "componentarrowcolor" | "deploymentarrowcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(ComponentSkinParamValue::ArrowColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "componentfontsize"
        | "deploymentfontsize"
        | "componentfontname"
        | "deploymentfontname"
        | "nodefontsize"
        | "nodefontname"
        | "artifactfontsize"
        | "artifactfontname"
        | "databasefontsize"
        | "databasefontname"
        | "componentstyle"
        | "componentstereotypefontcolor"
        | "componentstereotypefontsize"
        | "componentstereotypefontname"
        | "deploymentstereotypefontcolor"
        | "deploymentstereotypefontsize"
        | "deploymentstereotypefontname"
        | "portfontsize"
        | "portfontname"
        // Decorative layout hints — recognized as no-op (benign PlantUML compat)
        | "packagestyle"
        | "packagebordercolor"
        | "packagebackgroundcolor"
        | "packagefontcolor"
        | "packagefontsize"
        | "packagefontname" => SkinParamSupport::SupportedNoop,
        _ => SkinParamSupport::UnsupportedKey,
    }
}

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
