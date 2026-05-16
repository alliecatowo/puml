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
    "plain",
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
    match name.as_str() {
        "plain" => Ok(SequenceThemePreset {
            name: "plain",
            style: SequenceStyle::default(),
        }),
        "aws-orange" => Ok(SequenceThemePreset {
            name: "aws-orange",
            style: SequenceStyle {
                arrow_color: "#232f3e".to_string(),
                lifeline_border_color: "#ff9900".to_string(),
                participant_background_color: "#ff9900".to_string(),
                participant_border_color: "#232f3e".to_string(),
                note_background_color: "#fff4d6".to_string(),
                note_border_color: "#ff9900".to_string(),
                group_background_color: "#fdf3e3".to_string(),
                group_border_color: "#cc7a00".to_string(),
            },
        }),
        "blueprint" => Ok(SequenceThemePreset {
            name: "blueprint",
            style: SequenceStyle {
                arrow_color: "#ffffff".to_string(),
                lifeline_border_color: "#7eb4d4".to_string(),
                participant_background_color: "#1a3a5c".to_string(),
                participant_border_color: "#ffffff".to_string(),
                note_background_color: "#0d2b4a".to_string(),
                note_border_color: "#7eb4d4".to_string(),
                group_background_color: "#0f2d4a".to_string(),
                group_border_color: "#7eb4d4".to_string(),
            },
        }),
        "cerulean" => Ok(SequenceThemePreset {
            name: "cerulean",
            style: SequenceStyle {
                arrow_color: "#2fa4e7".to_string(),
                lifeline_border_color: "#2fa4e7".to_string(),
                participant_background_color: "#d9edf7".to_string(),
                participant_border_color: "#2fa4e7".to_string(),
                note_background_color: "#fcf8e3".to_string(),
                note_border_color: "#2fa4e7".to_string(),
                group_background_color: "#ebf5fb".to_string(),
                group_border_color: "#5bc0de".to_string(),
            },
        }),
        "cerulean-outline" => Ok(SequenceThemePreset {
            name: "cerulean-outline",
            style: SequenceStyle {
                arrow_color: "#2fa4e7".to_string(),
                lifeline_border_color: "#2fa4e7".to_string(),
                participant_background_color: "#ffffff".to_string(),
                participant_border_color: "#2fa4e7".to_string(),
                note_background_color: "#ffffff".to_string(),
                note_border_color: "#2fa4e7".to_string(),
                group_background_color: "#ffffff".to_string(),
                group_border_color: "#2fa4e7".to_string(),
            },
        }),
        "crt-amber" => Ok(SequenceThemePreset {
            name: "crt-amber",
            style: SequenceStyle {
                arrow_color: "#ffb000".to_string(),
                lifeline_border_color: "#cc8800".to_string(),
                participant_background_color: "#1a0e00".to_string(),
                participant_border_color: "#ffb000".to_string(),
                note_background_color: "#0d0700".to_string(),
                note_border_color: "#ffb000".to_string(),
                group_background_color: "#110900".to_string(),
                group_border_color: "#cc8800".to_string(),
            },
        }),
        "crt-green" => Ok(SequenceThemePreset {
            name: "crt-green",
            style: SequenceStyle {
                arrow_color: "#00ff41".to_string(),
                lifeline_border_color: "#00cc33".to_string(),
                participant_background_color: "#001100".to_string(),
                participant_border_color: "#00ff41".to_string(),
                note_background_color: "#000d00".to_string(),
                note_border_color: "#00ff41".to_string(),
                group_background_color: "#000f00".to_string(),
                group_border_color: "#00cc33".to_string(),
            },
        }),
        "cyborg" => Ok(SequenceThemePreset {
            name: "cyborg",
            style: SequenceStyle {
                arrow_color: "#2a9fd6".to_string(),
                lifeline_border_color: "#2a9fd6".to_string(),
                participant_background_color: "#060606".to_string(),
                participant_border_color: "#2a9fd6".to_string(),
                note_background_color: "#0d0d0d".to_string(),
                note_border_color: "#2a9fd6".to_string(),
                group_background_color: "#080808".to_string(),
                group_border_color: "#555555".to_string(),
            },
        }),
        "hacker" => Ok(SequenceThemePreset {
            name: "hacker",
            style: SequenceStyle {
                arrow_color: "#00ff00".to_string(),
                lifeline_border_color: "#00cc00".to_string(),
                participant_background_color: "#0d0d0d".to_string(),
                participant_border_color: "#00ff00".to_string(),
                note_background_color: "#000000".to_string(),
                note_border_color: "#00ff00".to_string(),
                group_background_color: "#050505".to_string(),
                group_border_color: "#00aa00".to_string(),
            },
        }),
        "mars" => Ok(SequenceThemePreset {
            name: "mars",
            style: SequenceStyle {
                arrow_color: "#e03030".to_string(),
                lifeline_border_color: "#c02020".to_string(),
                participant_background_color: "#1a0000".to_string(),
                participant_border_color: "#e03030".to_string(),
                note_background_color: "#0d0000".to_string(),
                note_border_color: "#e03030".to_string(),
                group_background_color: "#100000".to_string(),
                group_border_color: "#aa1a1a".to_string(),
            },
        }),
        "materia" => Ok(SequenceThemePreset {
            name: "materia",
            style: SequenceStyle {
                arrow_color: "#2196f3".to_string(),
                lifeline_border_color: "#90caf9".to_string(),
                participant_background_color: "#e3f2fd".to_string(),
                participant_border_color: "#2196f3".to_string(),
                note_background_color: "#fff9c4".to_string(),
                note_border_color: "#f9a825".to_string(),
                group_background_color: "#f5f5f5".to_string(),
                group_border_color: "#bdbdbd".to_string(),
            },
        }),
        "metal" => Ok(SequenceThemePreset {
            name: "metal",
            style: SequenceStyle {
                arrow_color: "#555555".to_string(),
                lifeline_border_color: "#888888".to_string(),
                participant_background_color: "#d4d4d4".to_string(),
                participant_border_color: "#555555".to_string(),
                note_background_color: "#f0f0f0".to_string(),
                note_border_color: "#888888".to_string(),
                group_background_color: "#e8e8e8".to_string(),
                group_border_color: "#999999".to_string(),
            },
        }),
        "mimeograph" => Ok(SequenceThemePreset {
            name: "mimeograph",
            style: SequenceStyle {
                arrow_color: "#5b3a8e".to_string(),
                lifeline_border_color: "#7b5aa6".to_string(),
                participant_background_color: "#f5f0fa".to_string(),
                participant_border_color: "#5b3a8e".to_string(),
                note_background_color: "#fdf9ff".to_string(),
                note_border_color: "#7b5aa6".to_string(),
                group_background_color: "#f8f4fc".to_string(),
                group_border_color: "#9b7abc".to_string(),
            },
        }),
        "minty" => Ok(SequenceThemePreset {
            name: "minty",
            style: SequenceStyle {
                arrow_color: "#78c2ad".to_string(),
                lifeline_border_color: "#56b29f".to_string(),
                participant_background_color: "#e8f7f4".to_string(),
                participant_border_color: "#78c2ad".to_string(),
                note_background_color: "#f0faf8".to_string(),
                note_border_color: "#78c2ad".to_string(),
                group_background_color: "#edf8f5".to_string(),
                group_border_color: "#a3d9ce".to_string(),
            },
        }),
        "reddress-darkblue" => Ok(SequenceThemePreset {
            name: "reddress-darkblue",
            style: SequenceStyle {
                arrow_color: "#cc0000".to_string(),
                lifeline_border_color: "#cc0000".to_string(),
                participant_background_color: "#1b2a4a".to_string(),
                participant_border_color: "#cc0000".to_string(),
                note_background_color: "#0f1f38".to_string(),
                note_border_color: "#cc0000".to_string(),
                group_background_color: "#152240".to_string(),
                group_border_color: "#3d5a8a".to_string(),
            },
        }),
        "sandstone" => Ok(SequenceThemePreset {
            name: "sandstone",
            style: SequenceStyle {
                arrow_color: "#8e6b3e".to_string(),
                lifeline_border_color: "#b08c5a".to_string(),
                participant_background_color: "#f5ede0".to_string(),
                participant_border_color: "#8e6b3e".to_string(),
                note_background_color: "#fdf5e8".to_string(),
                note_border_color: "#b08c5a".to_string(),
                group_background_color: "#f9eedc".to_string(),
                group_border_color: "#c4a070".to_string(),
            },
        }),
        "silver" => Ok(SequenceThemePreset {
            name: "silver",
            style: SequenceStyle {
                arrow_color: "#7d7d7d".to_string(),
                lifeline_border_color: "#ababab".to_string(),
                participant_background_color: "#efefef".to_string(),
                participant_border_color: "#7d7d7d".to_string(),
                note_background_color: "#f8f8f8".to_string(),
                note_border_color: "#ababab".to_string(),
                group_background_color: "#f2f2f2".to_string(),
                group_border_color: "#c0c0c0".to_string(),
            },
        }),
        "sketchy" => Ok(SequenceThemePreset {
            name: "sketchy",
            style: SequenceStyle {
                arrow_color: "#333333".to_string(),
                lifeline_border_color: "#555555".to_string(),
                participant_background_color: "#fffde7".to_string(),
                participant_border_color: "#333333".to_string(),
                note_background_color: "#fff8e1".to_string(),
                note_border_color: "#555555".to_string(),
                group_background_color: "#fafafa".to_string(),
                group_border_color: "#777777".to_string(),
            },
        }),
        "sketchy-outline" => Ok(SequenceThemePreset {
            name: "sketchy-outline",
            style: SequenceStyle {
                arrow_color: "#333333".to_string(),
                lifeline_border_color: "#555555".to_string(),
                participant_background_color: "#ffffff".to_string(),
                participant_border_color: "#333333".to_string(),
                note_background_color: "#ffffff".to_string(),
                note_border_color: "#555555".to_string(),
                group_background_color: "#ffffff".to_string(),
                group_border_color: "#777777".to_string(),
            },
        }),
        "spacelab" => Ok(SequenceThemePreset {
            name: "spacelab",
            style: SequenceStyle {
                arrow_color: "#2f4f6f".to_string(),
                lifeline_border_color: "#6d7f91".to_string(),
                participant_background_color: "#edf3f8".to_string(),
                participant_border_color: "#2f4f6f".to_string(),
                note_background_color: "#fff7d8".to_string(),
                note_border_color: "#5f7388".to_string(),
                group_background_color: "#f4f8fc".to_string(),
                group_border_color: "#7b8da0".to_string(),
            },
        }),
        "superhero" => Ok(SequenceThemePreset {
            name: "superhero",
            style: SequenceStyle {
                arrow_color: "#df6919".to_string(),
                lifeline_border_color: "#df6919".to_string(),
                participant_background_color: "#1a1a2e".to_string(),
                participant_border_color: "#df6919".to_string(),
                note_background_color: "#10101e".to_string(),
                note_border_color: "#df6919".to_string(),
                group_background_color: "#16162a".to_string(),
                group_border_color: "#2a2a50".to_string(),
            },
        }),
        "united" => Ok(SequenceThemePreset {
            name: "united",
            style: SequenceStyle {
                arrow_color: "#e95420".to_string(),
                lifeline_border_color: "#c34113".to_string(),
                participant_background_color: "#f4e3d7".to_string(),
                participant_border_color: "#e95420".to_string(),
                note_background_color: "#fdf0e8".to_string(),
                note_border_color: "#e95420".to_string(),
                group_background_color: "#faeade".to_string(),
                group_border_color: "#c34113".to_string(),
            },
        }),
        _ => Err(format!(
            "[E_THEME_UNKNOWN] unknown theme `{}`; available local themes: {}",
            tokens[0],
            LOCAL_SEQUENCE_THEME_CATALOG.join(", ")
        )),
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
