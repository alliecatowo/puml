use crate::diagnostic::Diagnostic;
use crate::model::{FamilyNode, FamilyStyle};
use crate::source::Span;

use super::{
    apply_monochrome_to_class_style, apply_monochrome_to_component_style,
    class_style_from_sequence_theme, classify_class_skinparam, classify_component_skinparam,
    classify_sequence_skinparam, component_style_from_sequence_theme,
    resolve_sequence_theme_preset, ClassSkinParamValue, ClassStyle, ComponentSkinParamValue,
    ComponentStyle, ComponentStyleTarget, MonochromeMode, SequenceSkinParamSupport,
    SequenceSkinParamValue, SkinParamSupport,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphStyleFamily {
    Class,
    Object,
    UseCase,
    Component,
    Deployment,
}

impl GraphStyleFamily {
    pub const fn is_class_family(self) -> bool {
        matches!(self, Self::Class | Self::Object | Self::UseCase)
    }

    pub const fn is_component_family(self) -> bool {
        matches!(self, Self::Component | Self::Deployment)
    }
}

#[derive(Debug, Clone)]
pub struct GraphStyleCascade {
    family: GraphStyleFamily,
    class_style: ClassStyle,
    component_style: ComponentStyle,
    class_monochrome_mode: Option<MonochromeMode>,
    component_monochrome_mode: Option<MonochromeMode>,
    sepia: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FamilyNodeInlineStyle {
    pub border_color: Option<String>,
    pub text_color: Option<String>,
    pub border_dashed: bool,
    pub border_thickness: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EffectiveClassNodeStyle {
    pub fill: String,
    pub stroke: String,
    pub font_color: String,
    pub member_color: String,
    pub header_color: String,
    pub border_dashed: bool,
    pub stroke_width: f32,
    pub font_family: String,
    pub title_font_size: u32,
    pub member_font_size: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EffectiveComponentNodeStyle {
    pub fill: String,
    pub stroke: String,
    pub font_color: String,
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

pub fn component_style_target_for_node(
    kind: crate::model::FamilyNodeKind,
) -> Option<ComponentStyleTarget> {
    use crate::model::FamilyNodeKind;
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
    let default_fill = if matches!(
        node.kind,
        crate::model::FamilyNodeKind::Interface | crate::model::FamilyNodeKind::Port
    ) {
        &component_style.interface_color
    } else {
        &component_style.background_color
    };

    EffectiveComponentNodeStyle {
        fill: node
            .fill_color
            .as_deref()
            .or_else(|| stereotype_style.and_then(|style| style.background_color.as_deref()))
            .or_else(|| target_style.and_then(|style| style.background_color.as_deref()))
            .unwrap_or(default_fill)
            .to_string(),
        stroke: inline_style
            .border_color
            .as_deref()
            .or_else(|| stereotype_style.and_then(|style| style.border_color.as_deref()))
            .or_else(|| target_style.and_then(|style| style.border_color.as_deref()))
            .unwrap_or(&component_style.border_color)
            .to_string(),
        font_color: inline_style
            .text_color
            .as_deref()
            .or_else(|| stereotype_style.and_then(|style| style.font_color.as_deref()))
            .or_else(|| target_style.and_then(|style| style.font_color.as_deref()))
            .unwrap_or(&component_style.font_color)
            .to_string(),
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

    EffectiveClassNodeStyle {
        fill: node
            .fill_color
            .as_deref()
            .or_else(|| scoped_style.and_then(|style| style.background_color.as_deref()))
            .unwrap_or(&class_style.background_color)
            .to_string(),
        stroke: inline_style
            .border_color
            .as_deref()
            .or_else(|| scoped_style.and_then(|style| style.border_color.as_deref()))
            .unwrap_or(&class_style.border_color)
            .to_string(),
        font_color: inline_style
            .text_color
            .as_deref()
            .or(scoped_font_color)
            .unwrap_or(&class_style.font_color)
            .to_string(),
        member_color: inline_style
            .text_color
            .as_deref()
            .or(scoped_font_color)
            .unwrap_or(class_style.member_color.as_str())
            .to_string(),
        header_color: scoped_style
            .and_then(|style| style.header_color.as_deref())
            .unwrap_or(class_style.header_color.as_str())
            .to_string(),
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

impl GraphStyleCascade {
    pub fn new(family: GraphStyleFamily) -> Self {
        Self {
            family,
            class_style: ClassStyle::default(),
            component_style: ComponentStyle::default(),
            class_monochrome_mode: None,
            component_monochrome_mode: None,
            sepia: false,
        }
    }

    pub const fn sepia(&self) -> bool {
        self.sepia
    }

    pub fn apply_theme(&mut self, value: &str, span: Span) -> Result<(), Diagnostic> {
        let style = resolve_sequence_theme_preset(value)
            .map_err(|msg| Diagnostic::error(msg).with_span(span))?
            .style;
        if self.family.is_class_family() {
            self.class_style = class_style_from_sequence_theme(&style);
        } else if self.family.is_component_family() {
            self.component_style = component_style_from_sequence_theme(&style);
        }
        Ok(())
    }

    pub fn apply_skinparam(
        &mut self,
        key: &str,
        value: &str,
        span: Span,
        warnings: &mut Vec<Diagnostic>,
    ) {
        if self.family.is_class_family() {
            self.apply_class_skinparam(key, value, span, warnings);
        } else if self.family.is_component_family() {
            self.apply_component_skinparam(key, value, span, warnings);
        }
    }

    pub fn apply_style_param(
        &mut self,
        selector: Option<&str>,
        property: &str,
        key: Option<&str>,
        value: &str,
        span: Span,
        warnings: &mut Vec<Diagnostic>,
    ) {
        if let Some(key) = key {
            self.apply_skinparam(key, value, span, warnings);
        } else {
            warnings.push(unsupported_style_warning(selector, property, span));
        }
    }

    pub fn into_family_style(mut self) -> FamilyStyle {
        if self.family.is_class_family() {
            if let Some(mode) = self.class_monochrome_mode {
                apply_monochrome_to_class_style(&mut self.class_style, mode);
            }
            FamilyStyle::Class(self.class_style)
        } else {
            if let Some(mode) = self.component_monochrome_mode {
                apply_monochrome_to_component_style(&mut self.component_style, mode);
            }
            FamilyStyle::Component(self.component_style)
        }
    }

    fn apply_class_skinparam(
        &mut self,
        key: &str,
        value: &str,
        span: Span,
        warnings: &mut Vec<Diagnostic>,
    ) {
        match classify_class_skinparam(key, value) {
            SkinParamSupport::SupportedNoop => {}
            SkinParamSupport::SupportedWithValue(v) => match v {
                ClassSkinParamValue::BackgroundColor(c) => self.class_style.background_color = c,
                ClassSkinParamValue::BorderColor(c) => self.class_style.border_color = c,
                ClassSkinParamValue::HeaderBackgroundColor(c) => self.class_style.header_color = c,
                ClassSkinParamValue::MemberFontColor(c) => self.class_style.member_color = c,
                ClassSkinParamValue::FontColor(c) => self.class_style.font_color = c,
                ClassSkinParamValue::ArrowColor(c) => self.class_style.arrow_color = c,
                ClassSkinParamValue::FontSize(n) => self.class_style.font_size = Some(n),
                ClassSkinParamValue::FontName(n) => self.class_style.font_name = Some(n),
                ClassSkinParamValue::ActorStyle(style) => self.class_style.actor_style = style,
                ClassSkinParamValue::AttributeIcons(enabled) => {
                    self.class_style.attribute_icons = enabled;
                }
                ClassSkinParamValue::Monochrome(mode) => self.class_monochrome_mode = Some(mode),
                ClassSkinParamValue::StereotypeBackgroundColor(stereotype, c) => {
                    self.class_style
                        .stereotype_styles
                        .entry(stereotype)
                        .or_default()
                        .background_color = Some(c);
                }
                ClassSkinParamValue::StereotypeBorderColor(stereotype, c) => {
                    self.class_style
                        .stereotype_styles
                        .entry(stereotype)
                        .or_default()
                        .border_color = Some(c);
                }
                ClassSkinParamValue::StereotypeHeaderBackgroundColor(stereotype, c) => {
                    self.class_style
                        .stereotype_styles
                        .entry(stereotype)
                        .or_default()
                        .header_color = Some(c);
                }
                ClassSkinParamValue::StereotypeFontColor(stereotype, c) => {
                    self.class_style
                        .stereotype_styles
                        .entry(stereotype)
                        .or_default()
                        .font_color = Some(c);
                }
            },
            SkinParamSupport::UnsupportedKey => {
                if key.trim().eq_ignore_ascii_case("sepia") {
                    if let SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::Sepia(enabled),
                    ) = classify_sequence_skinparam(key, value)
                    {
                        self.sepia = enabled;
                    }
                } else if matches!(
                    classify_sequence_skinparam(key, value),
                    SequenceSkinParamSupport::UnsupportedKey
                ) {
                    warnings.push(unsupported_skinparam_warning(key, span));
                }
            }
            SkinParamSupport::UnsupportedValue => {
                warnings.push(unsupported_skinparam_value_warning(key, value, span));
            }
        }
    }

    fn apply_component_skinparam(
        &mut self,
        key: &str,
        value: &str,
        span: Span,
        warnings: &mut Vec<Diagnostic>,
    ) {
        let mut handled = false;
        if key.trim().eq_ignore_ascii_case("monochrome") {
            handled = true;
            match classify_sequence_skinparam(key, value) {
                SequenceSkinParamSupport::SupportedNoop => {}
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::Monochrome(mode),
                ) => self.component_monochrome_mode = Some(mode),
                _ => warnings.push(unsupported_skinparam_value_warning(key, value, span)),
            }
        } else if key.trim().eq_ignore_ascii_case("handwritten") {
            handled = true;
            match classify_sequence_skinparam(key, value) {
                SequenceSkinParamSupport::SupportedNoop
                | SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::Handwritten(_),
                ) => {}
                _ => warnings.push(unsupported_skinparam_value_warning(key, value, span)),
            }
        } else if key.trim().eq_ignore_ascii_case("sepia") {
            handled = true;
            if let SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::Sepia(
                enabled,
            )) = classify_sequence_skinparam(key, value)
            {
                self.sepia = enabled;
            } else {
                warnings.push(unsupported_skinparam_value_warning(key, value, span));
            }
        }

        match classify_component_skinparam(key, value) {
            SkinParamSupport::SupportedNoop => {
                handled = true;
            }
            SkinParamSupport::SupportedWithValue(v) => {
                handled = true;
                match v {
                    ComponentSkinParamValue::BackgroundColor(c) => {
                        self.component_style.background_color = c;
                    }
                    ComponentSkinParamValue::BorderColor(c) => {
                        self.component_style.border_color = c;
                    }
                    ComponentSkinParamValue::InterfaceColor(c) => {
                        self.component_style.interface_color = c;
                    }
                    ComponentSkinParamValue::FontColor(c) => {
                        self.component_style.font_color = c;
                    }
                    ComponentSkinParamValue::ArrowColor(c) => {
                        self.component_style.arrow_color = c;
                    }
                    ComponentSkinParamValue::StyleMode(mode) => {
                        self.component_style.component_style_mode = mode;
                    }
                    ComponentSkinParamValue::TargetBackgroundColor(target, c) => {
                        self.component_style
                            .target_styles
                            .entry(target)
                            .or_default()
                            .background_color = Some(c);
                    }
                    ComponentSkinParamValue::TargetBorderColor(target, c) => {
                        self.component_style
                            .target_styles
                            .entry(target)
                            .or_default()
                            .border_color = Some(c);
                    }
                    ComponentSkinParamValue::TargetFontColor(target, c) => {
                        self.component_style
                            .target_styles
                            .entry(target)
                            .or_default()
                            .font_color = Some(c);
                    }
                    ComponentSkinParamValue::StereotypeBackgroundColor(stereotype, c) => {
                        self.component_style
                            .stereotype_styles
                            .entry(stereotype)
                            .or_default()
                            .background_color = Some(c);
                    }
                    ComponentSkinParamValue::StereotypeBorderColor(stereotype, c) => {
                        self.component_style
                            .stereotype_styles
                            .entry(stereotype)
                            .or_default()
                            .border_color = Some(c);
                    }
                    ComponentSkinParamValue::StereotypeFontColor(stereotype, c) => {
                        self.component_style
                            .stereotype_styles
                            .entry(stereotype)
                            .or_default()
                            .font_color = Some(c);
                    }
                }
            }
            SkinParamSupport::UnsupportedKey => {}
            SkinParamSupport::UnsupportedValue => {
                handled = true;
                warnings.push(unsupported_skinparam_value_warning(key, value, span));
            }
        }
        if !handled {
            warnings.push(unsupported_skinparam_warning(key, span));
        }
    }
}

fn unsupported_skinparam_warning(key: &str, span: Span) -> Diagnostic {
    Diagnostic::warning(format!(
        "[W_SKINPARAM_UNSUPPORTED] unsupported skinparam `{}`",
        key
    ))
    .with_span(span)
}

fn unsupported_skinparam_value_warning(key: &str, value: &str, span: Span) -> Diagnostic {
    Diagnostic::warning(format!(
        "[W_SKINPARAM_UNSUPPORTED_VALUE] unsupported value `{}` for skinparam `{}`",
        value, key
    ))
    .with_span(span)
}

fn unsupported_style_warning(selector: Option<&str>, property: &str, span: Span) -> Diagnostic {
    let selector = selector.unwrap_or("<diagram>");
    Diagnostic::warning(format!(
        "[W_STYLE_UNSUPPORTED] unsupported style `{}` in selector `{}`",
        property, selector
    ))
    .with_span(span)
}
