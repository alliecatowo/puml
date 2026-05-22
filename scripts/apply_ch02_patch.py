#!/usr/bin/env python3
"""One-shot patch applier for Chapter 2 use case parity (issue #938)."""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def patch(path: str, old: str, new: str) -> None:
    p = ROOT / path
    text = p.read_text()
    if old not in text:
        raise SystemExit(f"patch miss in {path}: anchor not found")
    p.write_text(text.replace(old, new, 1))


def main() -> None:
    patch(
        "src/theme.rs",
        """// ─── Class-family skinparam support ─────────────────────────────────────────

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
    pub stereotype_styles: BTreeMap<String, ClassStereotypeStyle>,
}""",
        """// ─── Class-family skinparam support ─────────────────────────────────────────

/// Stick figure vs PlantUML `skinparam actorStyle` variants (Chapter 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActorStyleKind {
    #[default]
    Stick,
    Awesome,
    Hollow,
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
    pub actor_style: ActorStyleKind,
    pub stereotype_styles: BTreeMap<String, ClassStereotypeStyle>,
}""",
    )

    patch(
        "src/theme.rs",
        """            font_size: None,
            font_name: None,
            stereotype_styles: BTreeMap::new(),
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
    Monochrome(MonochromeMode),
    StereotypeBackgroundColor(String, String),
    StereotypeBorderColor(String, String),
    StereotypeHeaderBackgroundColor(String, String),
    StereotypeFontColor(String, String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkinParamSupport<V> {""",
        """            font_size: None,
            font_name: None,
            actor_style: ActorStyleKind::default(),
            stereotype_styles: BTreeMap::new(),
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
    Monochrome(MonochromeMode),
    ActorStyle(ActorStyleKind),
    StereotypeBackgroundColor(String, String),
    StereotypeBorderColor(String, String),
    StereotypeHeaderBackgroundColor(String, String),
    StereotypeFontColor(String, String),
}

fn parse_actor_style_value(value: &str) -> Option<ActorStyleKind> {
    match value.trim().to_ascii_lowercase().as_str() {
        "stick" | "stickman" | "default" => Some(ActorStyleKind::Stick),
        "awesome" => Some(ActorStyleKind::Awesome),
        "hollow" => Some(ActorStyleKind::Hollow),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkinParamSupport<V> {""",
    )

    patch(
        "src/theme.rs",
        """        font_size: style.default_font_size,
        font_name: style.default_font_name.clone(),
        stereotype_styles: BTreeMap::new(),
    }
}

pub fn state_style_from_sequence_theme""",
        """        font_size: style.default_font_size,
        font_name: style.default_font_name.clone(),
        actor_style: ActorStyleKind::default(),
        stereotype_styles: BTreeMap::new(),
    }
}

pub fn state_style_from_sequence_theme""",
    )

    patch(
        "src/theme.rs",
        """        "monochrome" => match parse_monochrome_value(value) {
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
        "classstereotypefontcolor""",
        """        "monochrome" => match parse_monochrome_value(value) {
            Some(Some(mode)) => {
                SkinParamSupport::SupportedWithValue(ClassSkinParamValue::Monochrome(mode))
            }
            Some(None) => SkinParamSupport::SupportedNoop,
            None => SkinParamSupport::UnsupportedValue,
        },
        "actorstyle" => parse_actor_style_value(value)
            .map(|style| {
                SkinParamSupport::SupportedWithValue(ClassSkinParamValue::ActorStyle(style))
            })
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "handwritten" => {
            if parse_bool_value(value).is_some() {
                SkinParamSupport::SupportedNoop
            } else {
                SkinParamSupport::UnsupportedValue
            }
        }
        "classstereotypefontcolor""",
    )

    print("ch02 patch applied")


if __name__ == "__main__":
    main()
