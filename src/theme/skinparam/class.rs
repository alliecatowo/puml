use super::helpers::{parse_bool_value, parse_monochrome_value, split_stereotype_scope};
use super::SkinParamSupport;
use crate::theme::color::parse_color_value;
use crate::theme::styles::*;
use crate::theme::StyleSource;
use std::collections::BTreeMap;

// ─── Class-family skinparam support ─────────────────────────────────────────

/// Chrome rendering mode carried through the class/object render chain.
///
/// Set via the CLI `--style puml|plantuml` flag. The default is `Puml`.
/// Only paint differs between modes — layout (positions, sizes, edge paths) is
/// always identical.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StyleMode {
    /// PUML-enhanced chrome: richer fills, type badges, UML 2.x glyphs.
    #[default]
    Puml,
    /// PlantUML-compatible neutral chrome: flat fills, no badges, ASCII visibility.
    Plantuml,
}

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
    pub actor_style: ActorStyle,
    pub attribute_icons: bool,
    /// Corner radius for class/object node rectangles in px. `None` keeps the
    /// renderer's built-in default. Set by `skinparam roundcorner <N>`.
    pub round_corner: Option<i32>,
    /// When true, class/object node rectangles render with a drop shadow.
    /// Controlled by `skinparam shadowing true|false`.
    pub shadowing: bool,
    pub stereotype_styles: BTreeMap<String, ClassStereotypeStyle>,
    pub sources: ClassStyleSources,
    /// Chrome rendering mode: `Puml` (default) keeps PUML-enhanced output;
    /// `Plantuml` renders a neutral look matching PlantUML's defaults.
    /// Only paint is affected — layout is always identical.
    pub style_mode: StyleMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClassStyleSources {
    pub background_color: StyleSource,
    pub border_color: StyleSource,
    pub header_color: StyleSource,
    pub member_color: StyleSource,
    pub font_color: StyleSource,
    pub arrow_color: StyleSource,
    pub font_size: StyleSource,
    pub font_name: StyleSource,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ActorStyle {
    #[default]
    Stick,
    Awesome,
    Hollow,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClassStereotypeStyle {
    pub background_color: Option<String>,
    pub border_color: Option<String>,
    pub header_color: Option<String>,
    pub font_color: Option<String>,
    pub sources: ClassStereotypeStyleSources,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClassStereotypeStyleSources {
    pub background_color: StyleSource,
    pub border_color: StyleSource,
    pub header_color: StyleSource,
    pub font_color: StyleSource,
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
            actor_style: ActorStyle::Stick,
            attribute_icons: true,
            round_corner: None,
            shadowing: false,
            stereotype_styles: BTreeMap::new(),
            sources: ClassStyleSources::default(),
            style_mode: StyleMode::default(),
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
    ActorStyle(ActorStyle),
    AttributeIcons(bool),
    /// `skinparam roundcorner <N>` — corner radius for class/object node rects.
    RoundCorner(i32),
    /// `skinparam shadowing true|false` — drop-shadow on class/object node rects.
    Shadowing(bool),
    Monochrome(MonochromeMode),
    StereotypeBackgroundColor(String, String),
    StereotypeBorderColor(String, String),
    StereotypeHeaderBackgroundColor(String, String),
    StereotypeFontColor(String, String),
}

pub fn classify_class_skinparam(key: &str, value: &str) -> SkinParamSupport<ClassSkinParamValue> {
    let (normalized, stereotype_scope) = split_stereotype_scope(key);
    if let Some(stereotype) = stereotype_scope {
        return match normalized.as_str() {
            "backgroundcolor"
            | "classbackgroundcolor"
            | "objectbackgroundcolor"
            | "usecasebackgroundcolor"
            | "actorbackgroundcolor" => parse_color_value(value)
                .map(|c| {
                    SkinParamSupport::SupportedWithValue(
                        ClassSkinParamValue::StereotypeBackgroundColor(stereotype, c),
                    )
                })
                .unwrap_or(SkinParamSupport::UnsupportedValue),
            "bordercolor" | "classbordercolor" | "objectbordercolor" | "usecasebordercolor"
            | "actorbordercolor" => parse_color_value(value)
                .map(|c| {
                    SkinParamSupport::SupportedWithValue(
                        ClassSkinParamValue::StereotypeBorderColor(stereotype, c),
                    )
                })
                .unwrap_or(SkinParamSupport::UnsupportedValue),
            "classheaderbackgroundcolor" => parse_color_value(value)
                .map(|c| {
                    SkinParamSupport::SupportedWithValue(
                        ClassSkinParamValue::StereotypeHeaderBackgroundColor(stereotype, c),
                    )
                })
                .unwrap_or(SkinParamSupport::UnsupportedValue),
            "fontcolor" | "classfontcolor" | "objectfontcolor" | "usecasefontcolor"
            | "actorfontcolor" => parse_color_value(value)
                .map(|c| {
                    SkinParamSupport::SupportedWithValue(ClassSkinParamValue::StereotypeFontColor(
                        stereotype, c,
                    ))
                })
                .unwrap_or(SkinParamSupport::UnsupportedValue),
            "shadowing" => {
                if parse_bool_value(value).is_some() {
                    SkinParamSupport::SupportedNoop
                } else {
                    SkinParamSupport::UnsupportedValue
                }
            }
            "fontsize"
            | "classfontsize"
            | "objectfontsize"
            | "usecasefontsize"
            | "actorfontsize"
            | "classfontname"
            | "objectfontname"
            | "usecasefontname"
            | "actorfontname"
            | "classstereotypefontcolor"
            | "classstereotypefontsize"
            | "classstereotypefontname" => SkinParamSupport::SupportedNoop,
            _ => SkinParamSupport::UnsupportedKey,
        };
    }
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
        "fontname" | "classfontname" | "objectfontname" | "usecasefontname" | "actorfontname" => {
            let name = value.trim();
            if name.is_empty() {
                SkinParamSupport::UnsupportedValue
            } else {
                SkinParamSupport::SupportedWithValue(ClassSkinParamValue::FontName(
                    name.to_string(),
                ))
            }
        }
        "actorstyle" => match value.trim().to_ascii_lowercase().as_str() {
            "awesome" => {
                SkinParamSupport::SupportedWithValue(ClassSkinParamValue::ActorStyle(
                    ActorStyle::Awesome,
                ))
            }
            "hollow" => {
                SkinParamSupport::SupportedWithValue(ClassSkinParamValue::ActorStyle(
                    ActorStyle::Hollow,
                ))
            }
            _ => SkinParamSupport::UnsupportedValue,
        },
        "classattributeiconsize" => match value.trim().parse::<i32>() {
            Ok(0) => SkinParamSupport::SupportedWithValue(ClassSkinParamValue::AttributeIcons(false)),
            Ok(_) => SkinParamSupport::SupportedWithValue(ClassSkinParamValue::AttributeIcons(true)),
            Err(_) => SkinParamSupport::UnsupportedValue,
        },
        "monochrome" => match parse_monochrome_value(value) {
            Some(Some(mode)) => {
                SkinParamSupport::SupportedWithValue(ClassSkinParamValue::Monochrome(mode))
            }
            Some(None) => SkinParamSupport::SupportedNoop,
            None => SkinParamSupport::UnsupportedValue,
        },
        "handwritten" => {
            if parse_bool_value(value).is_some() {
                SkinParamSupport::SupportedNoop
            } else {
                SkinParamSupport::UnsupportedValue
            }
        }
        "roundcorner" => {
            if let Ok(n) = value.trim().parse::<i32>() {
                if n >= 0 {
                    SkinParamSupport::SupportedWithValue(ClassSkinParamValue::RoundCorner(n))
                } else {
                    SkinParamSupport::UnsupportedValue
                }
            } else {
                SkinParamSupport::UnsupportedValue
            }
        }
        "shadowing" => match parse_bool_value(value) {
            Some(enabled) => {
                SkinParamSupport::SupportedWithValue(ClassSkinParamValue::Shadowing(enabled))
            }
            None => SkinParamSupport::UnsupportedValue,
        },
        "classstereotypefontcolor"
        | "classstereotypefontsize"
        | "classstereotypefontname"
        | "classattributefontsize"
        | "classmethodfontsize"
        | "objectstereotypefontcolor"
        | "usecasestereotypefontcolor"
        | "actorstereotypefontcolor"
        | "linetype"
        // Package visual style and layout skinparams — accepted as noop (shape not yet rendered).
        | "packagestyle"
        | "packagebackgroundcolor"
        | "packagebordercolor"
        | "packagefontcolor"
        | "packagefontsize"
        | "packagefontname"
        | "packagestereotypefontcolor"
        | "namespaceseparator"
        | "groupinheritance"
        // Generic skinparams used in class diagrams accepted as noop.
        | "genericdisplay" => SkinParamSupport::SupportedNoop,
        _ => SkinParamSupport::UnsupportedKey,
    }
}

// ─── State-family skinparam support ─────────────────────────────────────────
