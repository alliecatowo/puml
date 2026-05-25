use crate::diagnostic::Diagnostic;
use crate::model::FamilyStyle;
use crate::source::Span;

use super::{
    apply_monochrome_to_class_style, apply_monochrome_to_component_style,
    class_style_from_sequence_theme, classify_class_skinparam, classify_component_skinparam,
    classify_sequence_skinparam, component_style_from_sequence_theme,
    resolve_sequence_theme_preset, ClassSkinParamValue, ClassStyle, ComponentSkinParamValue,
    ComponentStyle, MonochromeMode, SequenceSkinParamSupport, SequenceSkinParamValue,
    SkinParamSupport,
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
