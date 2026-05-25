use super::helpers::split_stereotype_scope;
use super::SkinParamSupport;
use crate::theme::color::parse_color_value;
use std::collections::BTreeMap;

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
    pub target_styles: BTreeMap<ComponentStyleTarget, ComponentNodeStyle>,
    pub stereotype_styles: BTreeMap<String, ComponentNodeStyle>,
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
            target_styles: BTreeMap::new(),
            stereotype_styles: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ComponentStyleTarget {
    Actor,
    Artifact,
    Boundary,
    Cloud,
    Component,
    Control,
    Database,
    Entity,
    File,
    Folder,
    Frame,
    Interface,
    Node,
    Package,
    Port,
    Queue,
    Storage,
    UseCase,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ComponentNodeStyle {
    pub background_color: Option<String>,
    pub border_color: Option<String>,
    pub font_color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComponentSkinParamValue {
    BackgroundColor(String),
    BorderColor(String),
    InterfaceColor(String),
    FontColor(String),
    ArrowColor(String),
    StyleMode(ComponentStyleMode),
    TargetBackgroundColor(ComponentStyleTarget, String),
    TargetBorderColor(ComponentStyleTarget, String),
    TargetFontColor(ComponentStyleTarget, String),
    StereotypeBackgroundColor(String, String),
    StereotypeBorderColor(String, String),
    StereotypeFontColor(String, String),
}

pub fn classify_component_skinparam(
    key: &str,
    value: &str,
) -> SkinParamSupport<ComponentSkinParamValue> {
    let (normalized, stereotype_scope) = split_stereotype_scope(key);
    if let Some(stereotype) = stereotype_scope {
        return match component_scoped_property(&normalized) {
            Some(ComponentScopedProperty::Background) => parse_color_value(value)
                .map(|c| {
                    SkinParamSupport::SupportedWithValue(
                        ComponentSkinParamValue::StereotypeBackgroundColor(stereotype, c),
                    )
                })
                .unwrap_or(SkinParamSupport::UnsupportedValue),
            Some(ComponentScopedProperty::Border) => parse_color_value(value)
                .map(|c| {
                    SkinParamSupport::SupportedWithValue(
                        ComponentSkinParamValue::StereotypeBorderColor(stereotype, c),
                    )
                })
                .unwrap_or(SkinParamSupport::UnsupportedValue),
            Some(ComponentScopedProperty::Font) => parse_color_value(value)
                .map(|c| {
                    SkinParamSupport::SupportedWithValue(
                        ComponentSkinParamValue::StereotypeFontColor(stereotype, c),
                    )
                })
                .unwrap_or(SkinParamSupport::UnsupportedValue),
            None => SkinParamSupport::UnsupportedKey,
        };
    }
    if let Some((target, property)) = component_target_property(&normalized) {
        return match property {
            ComponentScopedProperty::Background => parse_color_value(value)
                .map(|c| {
                    SkinParamSupport::SupportedWithValue(
                        ComponentSkinParamValue::TargetBackgroundColor(target, c),
                    )
                })
                .unwrap_or(SkinParamSupport::UnsupportedValue),
            ComponentScopedProperty::Border => parse_color_value(value)
                .map(|c| {
                    SkinParamSupport::SupportedWithValue(
                        ComponentSkinParamValue::TargetBorderColor(target, c),
                    )
                })
                .unwrap_or(SkinParamSupport::UnsupportedValue),
            ComponentScopedProperty::Font => parse_color_value(value)
                .map(|c| {
                    SkinParamSupport::SupportedWithValue(ComponentSkinParamValue::TargetFontColor(
                        target, c,
                    ))
                })
                .unwrap_or(SkinParamSupport::UnsupportedValue),
        };
    }
    match normalized.as_str() {
        "backgroundcolor"
        | "componentbackgroundcolor"
        | "deploymentbackgroundcolor" => parse_color_value(value)
            .map(|c| {
                SkinParamSupport::SupportedWithValue(ComponentSkinParamValue::BackgroundColor(c))
            })
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "bordercolor"
        | "componentbordercolor"
        | "deploymentbordercolor" => parse_color_value(value)
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

#[derive(Debug, Clone, Copy)]
enum ComponentScopedProperty {
    Background,
    Border,
    Font,
}

fn component_scoped_property(key: &str) -> Option<ComponentScopedProperty> {
    match key {
        "backgroundcolor"
        | "componentbackgroundcolor"
        | "deploymentbackgroundcolor"
        | "nodebackgroundcolor"
        | "artifactbackgroundcolor"
        | "databasebackgroundcolor"
        | "cloudbackgroundcolor"
        | "queuebackgroundcolor"
        | "storagebackgroundcolor"
        | "framebackgroundcolor"
        | "folderbackgroundcolor"
        | "filebackgroundcolor"
        | "actorbackgroundcolor"
        | "usecasebackgroundcolor"
        | "boundarybackgroundcolor"
        | "controlbackgroundcolor"
        | "entitybackgroundcolor"
        | "interfacebackgroundcolor"
        | "portbackgroundcolor" => Some(ComponentScopedProperty::Background),
        "bordercolor"
        | "linecolor"
        | "componentbordercolor"
        | "deploymentbordercolor"
        | "nodebordercolor"
        | "artifactbordercolor"
        | "databasebordercolor"
        | "cloudbordercolor"
        | "queuebordercolor"
        | "storagebordercolor"
        | "framebordercolor"
        | "folderbordercolor"
        | "filebordercolor"
        | "actorbordercolor"
        | "usecasebordercolor"
        | "boundarybordercolor"
        | "controlbordercolor"
        | "entitybordercolor"
        | "interfacebordercolor"
        | "portbordercolor" => Some(ComponentScopedProperty::Border),
        "fontcolor"
        | "componentfontcolor"
        | "deploymentfontcolor"
        | "nodefontcolor"
        | "artifactfontcolor"
        | "databasefontcolor"
        | "cloudfontcolor"
        | "queuefontcolor"
        | "storagefontcolor"
        | "framefontcolor"
        | "folderfontcolor"
        | "filefontcolor"
        | "actorfontcolor"
        | "usecasefontcolor"
        | "boundaryfontcolor"
        | "controlfontcolor"
        | "entityfontcolor"
        | "interfacefontcolor"
        | "portfontcolor" => Some(ComponentScopedProperty::Font),
        _ => None,
    }
}

fn component_target_property(key: &str) -> Option<(ComponentStyleTarget, ComponentScopedProperty)> {
    const TARGETS: [(&str, ComponentStyleTarget); 17] = [
        ("node", ComponentStyleTarget::Node),
        ("artifact", ComponentStyleTarget::Artifact),
        ("database", ComponentStyleTarget::Database),
        ("cloud", ComponentStyleTarget::Cloud),
        ("queue", ComponentStyleTarget::Queue),
        ("storage", ComponentStyleTarget::Storage),
        ("frame", ComponentStyleTarget::Frame),
        ("folder", ComponentStyleTarget::Folder),
        ("file", ComponentStyleTarget::File),
        ("actor", ComponentStyleTarget::Actor),
        ("usecase", ComponentStyleTarget::UseCase),
        ("boundary", ComponentStyleTarget::Boundary),
        ("control", ComponentStyleTarget::Control),
        ("entity", ComponentStyleTarget::Entity),
        ("interface", ComponentStyleTarget::Interface),
        ("port", ComponentStyleTarget::Port),
        ("package", ComponentStyleTarget::Package),
    ];
    for (prefix, target) in TARGETS {
        if let Some(suffix) = key.strip_prefix(prefix) {
            let property = match suffix {
                "backgroundcolor" => ComponentScopedProperty::Background,
                "bordercolor" | "linecolor" => ComponentScopedProperty::Border,
                "fontcolor" => ComponentScopedProperty::Font,
                _ => continue,
            };
            return Some((target, property));
        }
    }
    None
}
