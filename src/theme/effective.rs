use crate::model::{FamilyNode, FamilyNodeKind};

use super::{ClassStyle, ComponentStyle, ComponentStyleTarget, EffectiveStyleValue, StyleSource};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FamilyNodeInlineStyle {
    pub border_color: Option<String>,
    pub text_color: Option<String>,
    pub border_dashed: bool,
    pub border_thickness: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EffectiveClassNodeStyle {
    pub fill: EffectiveStyleValue<super::StyleColor>,
    pub stroke: EffectiveStyleValue<super::StyleColor>,
    pub font_color: EffectiveStyleValue<super::StyleColor>,
    pub member_color: EffectiveStyleValue<super::StyleColor>,
    pub header_color: EffectiveStyleValue<super::StyleColor>,
    pub border_dashed: bool,
    pub stroke_width: f32,
    pub font_family: String,
    pub title_font_size: u32,
    pub member_font_size: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EffectiveComponentNodeStyle {
    pub fill: EffectiveStyleValue<super::StyleColor>,
    pub stroke: EffectiveStyleValue<super::StyleColor>,
    pub font_color: EffectiveStyleValue<super::StyleColor>,
    pub border_dashed: bool,
    pub stroke_width: f32,
}

pub fn family_node_inline_style(node: &FamilyNode) -> FamilyNodeInlineStyle {
    let mut style = FamilyNodeInlineStyle::default();
    for member in &node.members {
        let text = member.text.trim();
        if let Some(color) = text.strip_prefix("\x1fstyle:border:") {
            style.border_color = Some(color.trim().to_string());
        } else if let Some(color) = text.strip_prefix("\x1fstyle:text:") {
            style.text_color = Some(color.trim().to_string());
        } else if text == "\x1fstyle:border-dashed" {
            style.border_dashed = true;
        } else if let Some(width) = text.strip_prefix("\x1fstyle:border-thickness:") {
            if let Ok(width) = width.trim().parse::<f32>() {
                style.border_thickness = Some(width.clamp(1.0, 8.0));
            }
        }
    }
    style
}

pub fn family_node_stereotype_key(node: &FamilyNode) -> Option<String> {
    node.members.iter().find_map(|member| {
        let text = member.text.trim();
        is_user_stereotype_marker(text).then(|| {
            text.trim_start_matches("<<")
                .trim_end_matches(">>")
                .trim()
                .to_ascii_lowercase()
        })
    })
}

pub fn component_style_target_for_node(kind: FamilyNodeKind) -> Option<ComponentStyleTarget> {
    match kind {
        FamilyNodeKind::Actor | FamilyNodeKind::Person => Some(ComponentStyleTarget::Actor),
        FamilyNodeKind::Artifact => Some(ComponentStyleTarget::Artifact),
        FamilyNodeKind::Boundary => Some(ComponentStyleTarget::Boundary),
        FamilyNodeKind::Cloud => Some(ComponentStyleTarget::Cloud),
        FamilyNodeKind::Component => Some(ComponentStyleTarget::Component),
        FamilyNodeKind::Control => Some(ComponentStyleTarget::Control),
        FamilyNodeKind::Database => Some(ComponentStyleTarget::Database),
        FamilyNodeKind::Entity => Some(ComponentStyleTarget::Entity),
        FamilyNodeKind::File => Some(ComponentStyleTarget::File),
        FamilyNodeKind::Folder => Some(ComponentStyleTarget::Folder),
        FamilyNodeKind::Frame => Some(ComponentStyleTarget::Frame),
        FamilyNodeKind::Interface => Some(ComponentStyleTarget::Interface),
        FamilyNodeKind::Node => Some(ComponentStyleTarget::Node),
        FamilyNodeKind::Package => Some(ComponentStyleTarget::Package),
        FamilyNodeKind::Port => Some(ComponentStyleTarget::Port),
        FamilyNodeKind::Queue => Some(ComponentStyleTarget::Queue),
        FamilyNodeKind::Storage => Some(ComponentStyleTarget::Storage),
        FamilyNodeKind::UseCaseDeployment => Some(ComponentStyleTarget::UseCase),
        _ => None,
    }
}

pub fn effective_component_node_style(
    component_style: &ComponentStyle,
    node: &FamilyNode,
) -> EffectiveComponentNodeStyle {
    let target_style = component_style_target_for_node(node.kind)
        .and_then(|target| component_style.target_styles.get(&target));
    let stereotype_style = family_node_stereotype_key(node)
        .and_then(|key| component_style.stereotype_styles.get(&key));
    let inline_style = family_node_inline_style(node);
    let (default_fill, default_fill_source) =
        if matches!(node.kind, FamilyNodeKind::Interface | FamilyNodeKind::Port) {
            (
                component_style.interface_color.as_str(),
                component_style.sources.interface_color,
            )
        } else {
            (
                component_style.background_color.as_str(),
                component_style.sources.background_color,
            )
        };

    let fill = node
        .fill_color
        .as_deref()
        .map(|value| (value, StyleSource::Inline))
        .or_else(|| {
            stereotype_style
                .and_then(|style| style.background_color.as_deref())
                .map(|value| (value, StyleSource::Stereotype))
        })
        .or_else(|| {
            target_style.and_then(|style| {
                style
                    .background_color
                    .as_deref()
                    .map(|value| (value, style.sources.background_color))
            })
        })
        .unwrap_or((default_fill, default_fill_source));
    let stroke = inline_style
        .border_color
        .as_deref()
        .map(|value| (value, StyleSource::Inline))
        .or_else(|| {
            stereotype_style
                .and_then(|style| style.border_color.as_deref())
                .map(|value| (value, StyleSource::Stereotype))
        })
        .or_else(|| {
            target_style.and_then(|style| {
                style
                    .border_color
                    .as_deref()
                    .map(|value| (value, style.sources.border_color))
            })
        })
        .unwrap_or((
            component_style.border_color.as_str(),
            component_style.sources.border_color,
        ));
    let font_color = inline_style
        .text_color
        .as_deref()
        .map(|value| (value, StyleSource::Inline))
        .or_else(|| {
            stereotype_style
                .and_then(|style| style.font_color.as_deref())
                .map(|value| (value, StyleSource::Stereotype))
        })
        .or_else(|| {
            target_style.and_then(|style| {
                style
                    .font_color
                    .as_deref()
                    .map(|value| (value, style.sources.font_color))
            })
        })
        .unwrap_or((
            component_style.font_color.as_str(),
            component_style.sources.font_color,
        ));

    EffectiveComponentNodeStyle {
        fill: EffectiveStyleValue::color(fill.0, fill.1),
        stroke: EffectiveStyleValue::color(stroke.0, stroke.1),
        font_color: EffectiveStyleValue::color(font_color.0, font_color.1),
        border_dashed: inline_style.border_dashed,
        stroke_width: inline_style.border_thickness.unwrap_or(1.5),
    }
}

pub fn effective_class_node_style(
    class_style: &ClassStyle,
    node: &FamilyNode,
) -> EffectiveClassNodeStyle {
    let scoped_style =
        family_node_stereotype_key(node).and_then(|key| class_style.stereotype_styles.get(&key));
    let inline_style = family_node_inline_style(node);
    let scoped_font_color = scoped_style
        .and_then(|style| style.font_color.as_deref())
        .filter(|color| !color.is_empty());
    let title_font_size = class_style.font_size.unwrap_or(13);

    let fill = node
        .fill_color
        .as_deref()
        .map(|value| (value, StyleSource::Inline))
        .or_else(|| {
            scoped_style
                .and_then(|style| style.background_color.as_deref())
                .map(|value| (value, StyleSource::Stereotype))
        })
        .unwrap_or((
            class_style.background_color.as_str(),
            class_style.sources.background_color,
        ));
    let stroke = inline_style
        .border_color
        .as_deref()
        .map(|value| (value, StyleSource::Inline))
        .or_else(|| {
            scoped_style
                .and_then(|style| style.border_color.as_deref())
                .map(|value| (value, StyleSource::Stereotype))
        })
        .unwrap_or((
            class_style.border_color.as_str(),
            class_style.sources.border_color,
        ));
    let font_color = inline_style
        .text_color
        .as_deref()
        .map(|value| (value, StyleSource::Inline))
        .or_else(|| scoped_font_color.map(|value| (value, StyleSource::Stereotype)))
        .unwrap_or((
            class_style.font_color.as_str(),
            class_style.sources.font_color,
        ));
    let member_color = inline_style
        .text_color
        .as_deref()
        .map(|value| (value, StyleSource::Inline))
        .or_else(|| scoped_font_color.map(|value| (value, StyleSource::Stereotype)))
        .unwrap_or((
            class_style.member_color.as_str(),
            class_style.sources.member_color,
        ));
    let header_color = scoped_style
        .and_then(|style| style.header_color.as_deref())
        .map(|value| (value, StyleSource::Stereotype))
        .unwrap_or((
            class_style.header_color.as_str(),
            class_style.sources.header_color,
        ));

    EffectiveClassNodeStyle {
        fill: EffectiveStyleValue::color(fill.0, fill.1),
        stroke: EffectiveStyleValue::color(stroke.0, stroke.1),
        font_color: EffectiveStyleValue::color(font_color.0, font_color.1),
        member_color: EffectiveStyleValue::color(member_color.0, member_color.1),
        header_color: EffectiveStyleValue::color(header_color.0, header_color.1),
        border_dashed: inline_style.border_dashed,
        stroke_width: inline_style.border_thickness.unwrap_or(1.5),
        font_family: class_style
            .font_name
            .clone()
            .unwrap_or_else(|| "monospace".to_string()),
        title_font_size,
        member_font_size: title_font_size.saturating_sub(2).max(9),
    }
}

fn is_user_stereotype_marker(text: &str) -> bool {
    text.starts_with("<<") && text.ends_with(">>") && !is_builtin_type_stereotype_marker(text)
}

fn is_builtin_type_stereotype_marker(text: &str) -> bool {
    matches!(
        text,
        "<<enum>>"
            | "<<interface>>"
            | "<<abstract>>"
            | "<<abstract class>>"
            | "<<annotation>>"
            | "<<protocol>>"
            | "<<struct>>"
    )
}
