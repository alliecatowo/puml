use crate::scene::TextOverflowPolicy;

#[derive(Debug, Clone)]
pub struct Theme {
    pub skinparams: Vec<(String, String)>,
    pub footbox_visible: bool,
    pub text_overflow_policy: TextOverflowPolicy,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            skinparams: Vec::new(),
            footbox_visible: false,
            text_overflow_policy: TextOverflowPolicy::WrapAndGrow,
        }
    }
}

impl Theme {
    pub fn new() -> Self {
        Self {
            skinparams: Vec::new(),
            footbox_visible: true,
            text_overflow_policy: TextOverflowPolicy::WrapAndGrow,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceStyle {
    pub arrow_color: String,
    pub lifeline_border_color: String,
    pub participant_background_color: String,
    pub participant_border_color: String,
    pub note_background_color: String,
    pub note_border_color: String,
    pub group_background_color: String,
    pub group_border_color: String,
}

impl Default for SequenceStyle {
    fn default() -> Self {
        Self {
            arrow_color: "#111".to_string(),
            lifeline_border_color: "#555".to_string(),
            participant_background_color: "#f6f6f6".to_string(),
            participant_border_color: "#111".to_string(),
            note_background_color: "#fff8c4".to_string(),
            note_border_color: "#111".to_string(),
            group_background_color: "#fafafa".to_string(),
            group_border_color: "#666".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceThemePreset {
    pub name: &'static str,
    pub style: SequenceStyle,
}

pub const LOCAL_SEQUENCE_THEME_CATALOG: &[&str] = &[
    "aws-orange",
    "blueprint",
    "cerulean",
    "cerulean-outline",
    "crt-amber",
    "crt-green",
    "cyborg",
    "hacker",
    "mars",
    "materia",
    "metal",
    "mimeograph",
    "minty",
    "plain",
    "reddress-darkblue",
    "sandstone",
    "silver",
    "sketchy",
    "sketchy-outline",
    "spacelab",
    "superhero",
    "united",
];

pub fn resolve_sequence_theme_preset(spec: &str) -> Result<SequenceThemePreset, String> {
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        return Err("[E_THEME_INVALID] malformed !theme syntax: missing theme name".to_string());
    }

    let tokens: Vec<&str> = trimmed.split_whitespace().collect();
    if tokens.len() != 1 {
        if tokens.len() >= 3 && tokens[1].eq_ignore_ascii_case("from") {
            return Err(format!(
                "[E_THEME_SOURCE_UNSUPPORTED] unsupported !theme source `{}`; only built-in local themes are supported",
                tokens[2..].join(" ")
            ));
        }
        return Err(format!(
            "[E_THEME_INVALID] malformed !theme syntax: expected `!theme <name>`, got `!theme {}`",
            trimmed
        ));
    }

    let name = tokens[0].to_ascii_lowercase();
    match theme_style_by_name(name.as_str()) {
        Some((preset_name, style)) => Ok(SequenceThemePreset {
            name: preset_name,
            style,
        }),
        None => Err(format!(
            "[E_THEME_UNKNOWN] unknown theme `{}`; available local themes: {}",
            tokens[0],
            LOCAL_SEQUENCE_THEME_CATALOG.join(", ")
        )),
    }
}

fn theme_style_by_name(name: &str) -> Option<(&'static str, SequenceStyle)> {
    let themed = match name {
        "plain" => ("plain", SequenceStyle::default()),
        "spacelab" => (
            "spacelab",
            SequenceStyle {
                arrow_color: "#2f4f6f".to_string(),
                lifeline_border_color: "#6d7f91".to_string(),
                participant_background_color: "#edf3f8".to_string(),
                participant_border_color: "#2f4f6f".to_string(),
                note_background_color: "#fff7d8".to_string(),
                note_border_color: "#5f7388".to_string(),
                group_background_color: "#f4f8fc".to_string(),
                group_border_color: "#7b8da0".to_string(),
            },
        ),
        "aws-orange" => (
            "aws-orange",
            SequenceStyle {
                arrow_color: "#ff9900".to_string(),
                lifeline_border_color: "#7a5a2f".to_string(),
                participant_background_color: "#fff4e5".to_string(),
                participant_border_color: "#ff9900".to_string(),
                note_background_color: "#fff0d9".to_string(),
                note_border_color: "#c17d00".to_string(),
                group_background_color: "#fff9f1".to_string(),
                group_border_color: "#d89229".to_string(),
            },
        ),
        "blueprint" => (
            "blueprint",
            blue_tint("#1d4e89", "#365f8f", "#e9f1fb", "#5a7ca3"),
        ),
        "cerulean" => (
            "cerulean",
            blue_tint("#2a74b5", "#4d86b8", "#ecf5fc", "#6f97bd"),
        ),
        "cerulean-outline" => (
            "cerulean-outline",
            blue_tint("#2a74b5", "#4d86b8", "#f8fbff", "#7ca3c8"),
        ),
        "crt-amber" => (
            "crt-amber",
            warm_dark("#ffbf00", "#d8a000", "#231a00", "#6e5200"),
        ),
        "crt-green" => (
            "crt-green",
            cool_dark("#3cff8f", "#2bcc74", "#0a1f12", "#1f6b44"),
        ),
        "cyborg" => (
            "cyborg",
            cool_dark("#5bc0de", "#4f6b73", "#1f2528", "#3e4a50"),
        ),
        "hacker" => (
            "hacker",
            cool_dark("#00ff66", "#12a54f", "#08150d", "#1f6e3f"),
        ),
        "mars" => (
            "mars",
            warm_dark("#d1495b", "#9b3a46", "#2a1e20", "#5f3f43"),
        ),
        "materia" => (
            "materia",
            neutral_light("#3f51b5", "#5a66b9", "#f3f4fa", "#7f87c4"),
        ),
        "metal" => (
            "metal",
            neutral_light("#586069", "#6f7881", "#edf0f2", "#89929a"),
        ),
        "mimeograph" => (
            "mimeograph",
            neutral_light("#5f6875", "#7a8290", "#f5f5f0", "#9aa1ad"),
        ),
        "minty" => (
            "minty",
            neutral_light("#3fb27f", "#5cbf93", "#edf9f4", "#7ccbad"),
        ),
        "reddress-darkblue" => (
            "reddress-darkblue",
            cool_dark("#d94848", "#8c2c52", "#172337", "#3a4f74"),
        ),
        "sandstone" => (
            "sandstone",
            neutral_light("#8f6f47", "#a08159", "#f7f1e8", "#b09572"),
        ),
        "silver" => (
            "silver",
            neutral_light("#6d7582", "#838a95", "#f4f5f7", "#9ca2ab"),
        ),
        "sketchy" => (
            "sketchy",
            neutral_light("#202020", "#404040", "#fffef8", "#707070"),
        ),
        "sketchy-outline" => (
            "sketchy-outline",
            neutral_light("#303030", "#545454", "#ffffff", "#7d7d7d"),
        ),
        "superhero" => (
            "superhero",
            cool_dark("#df691a", "#a24f18", "#1d2733", "#4f6278"),
        ),
        "united" => (
            "united",
            warm_light("#e95420", "#c2461a", "#fff6f2", "#d46640"),
        ),
        _ => return None,
    };
    Some(themed)
}

fn blue_tint(arrow: &str, border: &str, fill: &str, group: &str) -> SequenceStyle {
    SequenceStyle {
        arrow_color: arrow.to_string(),
        lifeline_border_color: border.to_string(),
        participant_background_color: fill.to_string(),
        participant_border_color: arrow.to_string(),
        note_background_color: "#fff9e8".to_string(),
        note_border_color: border.to_string(),
        group_background_color: fill.to_string(),
        group_border_color: group.to_string(),
    }
}

fn neutral_light(arrow: &str, border: &str, fill: &str, group: &str) -> SequenceStyle {
    SequenceStyle {
        arrow_color: arrow.to_string(),
        lifeline_border_color: border.to_string(),
        participant_background_color: fill.to_string(),
        participant_border_color: arrow.to_string(),
        note_background_color: "#fff9e8".to_string(),
        note_border_color: border.to_string(),
        group_background_color: fill.to_string(),
        group_border_color: group.to_string(),
    }
}

fn warm_light(arrow: &str, border: &str, fill: &str, group: &str) -> SequenceStyle {
    SequenceStyle {
        arrow_color: arrow.to_string(),
        lifeline_border_color: border.to_string(),
        participant_background_color: fill.to_string(),
        participant_border_color: arrow.to_string(),
        note_background_color: "#fff4de".to_string(),
        note_border_color: border.to_string(),
        group_background_color: fill.to_string(),
        group_border_color: group.to_string(),
    }
}

fn cool_dark(arrow: &str, border: &str, fill: &str, group: &str) -> SequenceStyle {
    SequenceStyle {
        arrow_color: arrow.to_string(),
        lifeline_border_color: border.to_string(),
        participant_background_color: fill.to_string(),
        participant_border_color: arrow.to_string(),
        note_background_color: "#2a3238".to_string(),
        note_border_color: border.to_string(),
        group_background_color: "#25303a".to_string(),
        group_border_color: group.to_string(),
    }
}

fn warm_dark(arrow: &str, border: &str, fill: &str, group: &str) -> SequenceStyle {
    SequenceStyle {
        arrow_color: arrow.to_string(),
        lifeline_border_color: border.to_string(),
        participant_background_color: fill.to_string(),
        participant_border_color: arrow.to_string(),
        note_background_color: "#3a2f10".to_string(),
        note_border_color: border.to_string(),
        group_background_color: "#2f260d".to_string(),
        group_border_color: group.to_string(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SequenceSkinParamValue {
    FootboxVisible(bool),
    ArrowColor(String),
    LifelineBorderColor(String),
    ParticipantBackgroundColor(String),
    ParticipantBorderColor(String),
    NoteBackgroundColor(String),
    NoteBorderColor(String),
    GroupBackgroundColor(String),
    GroupBorderColor(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SequenceSkinParamSupport {
    SupportedNoop,
    SupportedWithValue(SequenceSkinParamValue),
    UnsupportedKey,
    UnsupportedValue,
}

pub fn classify_sequence_skinparam(key: &str, value: &str) -> SequenceSkinParamSupport {
    let normalized_key = key.trim().to_ascii_lowercase();
    match normalized_key.as_str() {
        "maxmessagesize" => SequenceSkinParamSupport::SupportedNoop,
        "footbox" | "sequencefootbox" => parse_footbox_value(value)
            .map(SequenceSkinParamSupport::SupportedWithValue)
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "arrowcolor" | "sequencearrowcolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::ArrowColor(
                    color,
                ))
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "lifelinebordercolor" | "sequencelifelinebordercolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::LifelineBorderColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "participantbackgroundcolor" | "sequenceparticipantbackgroundcolor" => {
            parse_color_value(value)
                .map(|color| {
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ParticipantBackgroundColor(color),
                    )
                })
                .unwrap_or(SequenceSkinParamSupport::UnsupportedValue)
        }
        "participantbordercolor" | "sequenceparticipantbordercolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::ParticipantBorderColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "notebackgroundcolor" | "sequencenotebackgroundcolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::NoteBackgroundColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "notebordercolor" | "sequencenotebordercolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::NoteBorderColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "groupbackgroundcolor" | "sequencegroupbackgroundcolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::GroupBackgroundColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "groupbordercolor" | "sequencegroupbordercolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::GroupBorderColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        _ => SequenceSkinParamSupport::UnsupportedKey,
    }
}

fn parse_footbox_value(value: &str) -> Option<SequenceSkinParamValue> {
    let normalized = value.trim().to_ascii_lowercase();
    let visible = match normalized.as_str() {
        "show" | "true" | "yes" | "on" => true,
        "hide" | "false" | "no" | "off" => false,
        _ => return None,
    };
    Some(SequenceSkinParamValue::FootboxVisible(visible))
}

fn parse_color_value(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(hex) = trimmed.strip_prefix('#') {
        let valid_len = matches!(hex.len(), 3 | 4 | 6 | 8);
        if valid_len && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Some(format!("#{}", hex.to_ascii_lowercase()));
        }
        return None;
    }
    if trimmed.bytes().all(|b| b.is_ascii_alphabetic()) {
        return Some(trimmed.to_ascii_lowercase());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{resolve_sequence_theme_preset, LOCAL_SEQUENCE_THEME_CATALOG};

    #[test]
    fn local_theme_catalog_entries_resolve_to_deterministic_styles() {
        for name in LOCAL_SEQUENCE_THEME_CATALOG {
            let preset = resolve_sequence_theme_preset(name)
                .unwrap_or_else(|e| panic!("theme `{name}` should resolve: {e}"));
            assert_eq!(preset.name, *name);
            assert!(!preset.style.arrow_color.is_empty());
            assert!(!preset.style.lifeline_border_color.is_empty());
            assert!(!preset.style.participant_background_color.is_empty());
            assert!(!preset.style.participant_border_color.is_empty());
            assert!(!preset.style.note_background_color.is_empty());
            assert!(!preset.style.note_border_color.is_empty());
            assert!(!preset.style.group_background_color.is_empty());
            assert!(!preset.style.group_border_color.is_empty());
        }
    }

    #[test]
    fn unknown_theme_error_lists_catalog() {
        let err = resolve_sequence_theme_preset("coffee").expect_err("unknown theme should fail");
        assert!(err.contains("E_THEME_UNKNOWN"));
        assert!(err.contains("available local themes: "));
        assert!(err.contains("plain"));
        assert!(err.contains("spacelab"));
    }
}
