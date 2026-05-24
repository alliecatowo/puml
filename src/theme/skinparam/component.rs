use super::SkinParamSupport;
use crate::theme::color::parse_color_value;

// ─── Component-family skinparam support ──────────────────────────────────────

/// Controls component-node rendering style (set via `skinparam componentStyle`).
///
/// - `Uml2` (default): UML2 icon — two badge rectangles on the left edge.
/// - `Uml1`: UML1 style — badge icon in the top-right corner.
/// - `Rectangle`: bare rectangle with no component icon or stereotype.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ComponentStyleMode {
    #[default]
    Uml2,
    Uml1,
    Rectangle,
}

/// Style overrides for component/deployment diagrams.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentStyle {
    pub background_color: String,
    pub border_color: String,
    pub interface_color: String,
    pub font_color: String,
    pub arrow_color: String,
    /// Controls UML1 / UML2 / Rectangle rendering for component nodes.
    pub component_style_mode: ComponentStyleMode,
}

impl Default for ComponentStyle {
    fn default() -> Self {
        Self {
            background_color: "#f0f4f8".to_string(),
            border_color: "#1e293b".to_string(),
            interface_color: "#e2e8f0".to_string(),
            font_color: "#0f172a".to_string(),
            arrow_color: "#1e293b".to_string(),
            component_style_mode: ComponentStyleMode::Uml2,
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
    StyleMode(ComponentStyleMode),
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
        "componentstyle" => {
            let mode = match value.trim().to_ascii_lowercase().as_str() {
                "uml1" => ComponentStyleMode::Uml1,
                "rectangle" => ComponentStyleMode::Rectangle,
                "uml2" | "" => ComponentStyleMode::Uml2,
                _ => return SkinParamSupport::UnsupportedValue,
            };
            SkinParamSupport::SupportedWithValue(ComponentSkinParamValue::StyleMode(mode))
        }
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
