use std::fmt;

use super::color::parse_color_value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StyleSource {
    #[default]
    Default,
    ThemePreset,
    SkinParam,
    StyleBlock,
    Stereotype,
    Inline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleColor(String);

impl StyleColor {
    pub fn parse(value: &str) -> Option<Self> {
        parse_color_value(value).map(Self)
    }

    pub fn trusted(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for StyleColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveStyleValue<T> {
    value: T,
    source: StyleSource,
}

impl<T> EffectiveStyleValue<T> {
    pub fn new(value: T, source: StyleSource) -> Self {
        Self { value, source }
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn source(&self) -> StyleSource {
        self.source
    }
}

impl EffectiveStyleValue<StyleColor> {
    pub fn color(value: impl Into<String>, source: StyleSource) -> Self {
        Self::new(StyleColor::trusted(value), source)
    }

    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }
}
