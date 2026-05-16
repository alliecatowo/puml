use crate::scene::TextOverflowPolicy;

/// Return the canonical lowercase hex value (`#rrggbb`) for a CSS3 named color.
pub fn css3_color_to_hex(name: &str) -> Option<&'static str> {
    match name.to_ascii_lowercase().as_str() {
        "aliceblue" => Some("#f0f8ff"),
        "antiquewhite" => Some("#faebd7"),
        "aqua" => Some("#00ffff"),
        "aquamarine" => Some("#7fffd4"),
        "azure" => Some("#f0ffff"),
        "beige" => Some("#f5f5dc"),
        "bisque" => Some("#ffe4c4"),
        "black" => Some("#000000"),
        "blanchedalmond" => Some("#ffebcd"),
        "blue" => Some("#0000ff"),
        "blueviolet" => Some("#8a2be2"),
        "brown" => Some("#a52a2a"),
        "burlywood" => Some("#deb887"),
        "cadetblue" => Some("#5f9ea0"),
        "chartreuse" => Some("#7fff00"),
        "chocolate" => Some("#d2691e"),
        "coral" => Some("#ff7f50"),
        "cornflowerblue" => Some("#6495ed"),
        "cornsilk" => Some("#fff8dc"),
        "crimson" => Some("#dc143c"),
        "cyan" => Some("#00ffff"),
        "darkblue" => Some("#00008b"),
        "darkcyan" => Some("#008b8b"),
        "darkgoldenrod" => Some("#b8860b"),
        "darkgray" | "darkgrey" => Some("#a9a9a9"),
        "darkgreen" => Some("#006400"),
        "darkkhaki" => Some("#bdb76b"),
        "darkmagenta" => Some("#8b008b"),
        "darkolivegreen" => Some("#556b2f"),
        "darkorange" => Some("#ff8c00"),
        "darkorchid" => Some("#9932cc"),
        "darkred" => Some("#8b0000"),
        "darksalmon" => Some("#e9967a"),
        "darkseagreen" => Some("#8fbc8f"),
        "darkslateblue" => Some("#483d8b"),
        "darkslategray" | "darkslategrey" => Some("#2f4f4f"),
        "darkturquoise" => Some("#00ced1"),
        "darkviolet" => Some("#9400d3"),
        "deeppink" => Some("#ff1493"),
        "deepskyblue" => Some("#00bfff"),
        "dimgray" | "dimgrey" => Some("#696969"),
        "dodgerblue" => Some("#1e90ff"),
        "firebrick" => Some("#b22222"),
        "floralwhite" => Some("#fffaf0"),
        "forestgreen" => Some("#228b22"),
        "fuchsia" => Some("#ff00ff"),
        "gainsboro" => Some("#dcdcdc"),
        "ghostwhite" => Some("#f8f8ff"),
        "gold" => Some("#ffd700"),
        "goldenrod" => Some("#daa520"),
        "gray" | "grey" => Some("#808080"),
        "green" => Some("#008000"),
        "greenyellow" => Some("#adff2f"),
        "honeydew" => Some("#f0fff0"),
        "hotpink" => Some("#ff69b4"),
        "indianred" => Some("#cd5c5c"),
        "indigo" => Some("#4b0082"),
        "ivory" => Some("#fffff0"),
        "khaki" => Some("#f0e68c"),
        "lavender" => Some("#e6e6fa"),
        "lavenderblush" => Some("#fff0f5"),
        "lawngreen" => Some("#7cfc00"),
        "lemonchiffon" => Some("#fffacd"),
        "lightblue" => Some("#add8e6"),
        "lightcoral" => Some("#f08080"),
        "lightcyan" => Some("#e0ffff"),
        "lightgoldenrodyellow" => Some("#fafad2"),
        "lightgray" | "lightgrey" => Some("#d3d3d3"),
        "lightgreen" => Some("#90ee90"),
        "lightpink" => Some("#ffb6c1"),
        "lightsalmon" => Some("#ffa07a"),
        "lightseagreen" => Some("#20b2aa"),
        "lightskyblue" => Some("#87cefa"),
        "lightslategray" | "lightslategrey" => Some("#778899"),
        "lightsteelblue" => Some("#b0c4de"),
        "lightyellow" => Some("#ffffe0"),
        "lime" => Some("#00ff00"),
        "limegreen" => Some("#32cd32"),
        "linen" => Some("#faf0e6"),
        "magenta" => Some("#ff00ff"),
        "maroon" => Some("#800000"),
        "mediumaquamarine" => Some("#66cdaa"),
        "mediumblue" => Some("#0000cd"),
        "mediumorchid" => Some("#ba55d3"),
        "mediumpurple" => Some("#9370db"),
        "mediumseagreen" => Some("#3cb371"),
        "mediumslateblue" => Some("#7b68ee"),
        "mediumspringgreen" => Some("#00fa9a"),
        "mediumturquoise" => Some("#48d1cc"),
        "mediumvioletred" => Some("#c71585"),
        "midnightblue" => Some("#191970"),
        "mintcream" => Some("#f5fffa"),
        "mistyrose" => Some("#ffe4e1"),
        "moccasin" => Some("#ffe4b5"),
        "navajowhite" => Some("#ffdead"),
        "navy" => Some("#000080"),
        "oldlace" => Some("#fdf5e6"),
        "olive" => Some("#808000"),
        "olivedrab" => Some("#6b8e23"),
        "orange" => Some("#ffa500"),
        "orangered" => Some("#ff4500"),
        "orchid" => Some("#da70d6"),
        "palegoldenrod" => Some("#eee8aa"),
        "palegreen" => Some("#98fb98"),
        "paleturquoise" => Some("#afeeee"),
        "palevioletred" => Some("#db7093"),
        "papayawhip" => Some("#ffefd5"),
        "peachpuff" => Some("#ffdab9"),
        "peru" => Some("#cd853f"),
        "pink" => Some("#ffc0cb"),
        "plum" => Some("#dda0dd"),
        "powderblue" => Some("#b0e0e6"),
        "purple" => Some("#800080"),
        "rebeccapurple" => Some("#663399"),
        "red" => Some("#ff0000"),
        "rosybrown" => Some("#bc8f8f"),
        "royalblue" => Some("#4169e1"),
        "saddlebrown" => Some("#8b4513"),
        "salmon" => Some("#fa8072"),
        "sandybrown" => Some("#f4a460"),
        "seagreen" => Some("#2e8b57"),
        "seashell" => Some("#fff5ee"),
        "sienna" => Some("#a0522d"),
        "silver" => Some("#c0c0c0"),
        "skyblue" => Some("#87ceeb"),
        "slateblue" => Some("#6a5acd"),
        "slategray" | "slategrey" => Some("#708090"),
        "snow" => Some("#fffafa"),
        "springgreen" => Some("#00ff7f"),
        "steelblue" => Some("#4682b4"),
        "tan" => Some("#d2b48c"),
        "teal" => Some("#008080"),
        "thistle" => Some("#d8bfd8"),
        "tomato" => Some("#ff6347"),
        "turquoise" => Some("#40e0d0"),
        "violet" => Some("#ee82ee"),
        "wheat" => Some("#f5deb3"),
        "white" => Some("#ffffff"),
        "whitesmoke" => Some("#f5f5f5"),
        "yellow" => Some("#ffff00"),
        "yellowgreen" => Some("#9acd32"),
        _ => None,
    }
}

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
    pub round_corner: i32,
    pub shadowing: bool,
    pub default_font_name: Option<String>,
    pub default_font_size: Option<u32>,
    pub background_color: Option<String>,
    pub text_alignment: TextAlignment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlignment {
    #[default]
    Center,
    Left,
    Right,
}

impl TextAlignment {
    pub fn as_text_anchor(self) -> &'static str {
        match self {
            TextAlignment::Center => "middle",
            TextAlignment::Left => "start",
            TextAlignment::Right => "end",
        }
    }
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
            round_corner: 4,
            shadowing: false,
            default_font_name: None,
            default_font_size: None,
            background_color: None,
            text_alignment: TextAlignment::Center,
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
                ..SequenceStyle::default()
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
                ..SequenceStyle::default()
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
                ..SequenceStyle::default()
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
                ..SequenceStyle::default()
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
                ..SequenceStyle::default()
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
                ..SequenceStyle::default()
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
                ..SequenceStyle::default()
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
                ..SequenceStyle::default()
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
                ..SequenceStyle::default()
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
                ..SequenceStyle::default()
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
                ..SequenceStyle::default()
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
                ..SequenceStyle::default()
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
                ..SequenceStyle::default()
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
                ..SequenceStyle::default()
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
                ..SequenceStyle::default()
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
                ..SequenceStyle::default()
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
                ..SequenceStyle::default()
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
                ..SequenceStyle::default()
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
                ..SequenceStyle::default()
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
                ..SequenceStyle::default()
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
                ..SequenceStyle::default()
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
    RoundCorner(i32),
    Shadowing(bool),
    DefaultFontName(String),
    DefaultFontSize(u32),
    BackgroundColor(String),
    DefaultTextAlignment(TextAlignment),
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
        "roundcorner" => {
            if let Ok(n) = value.trim().parse::<i32>() {
                SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::RoundCorner(n))
            } else {
                SequenceSkinParamSupport::UnsupportedValue
            }
        }
        "shadowing" => {
            let lower = value.trim().to_ascii_lowercase();
            let enabled = match lower.as_str() {
                "true" | "yes" | "on" => true,
                "false" | "no" | "off" => false,
                _ => return SequenceSkinParamSupport::UnsupportedValue,
            };
            SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::Shadowing(enabled))
        }
        "defaultfontname" => {
            let name = value.trim();
            if name.is_empty() {
                SequenceSkinParamSupport::UnsupportedValue
            } else {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::DefaultFontName(name.to_string()),
                )
            }
        }
        "defaultfontsize" => {
            if let Ok(n) = value.trim().parse::<u32>() {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::DefaultFontSize(n),
                )
            } else {
                SequenceSkinParamSupport::UnsupportedValue
            }
        }
        "backgroundcolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::BackgroundColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "defaulttextalignment" => {
            let lower = value.trim().to_ascii_lowercase();
            let alignment = match lower.as_str() {
                "center" => TextAlignment::Center,
                "left" => TextAlignment::Left,
                "right" => TextAlignment::Right,
                _ => return SequenceSkinParamSupport::UnsupportedValue,
            };
            SequenceSkinParamSupport::SupportedWithValue(
                SequenceSkinParamValue::DefaultTextAlignment(alignment),
            )
        }
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
        let lower = trimmed.to_ascii_lowercase();
        // Resolve CSS3 named colors to their hex equivalent.
        if let Some(hex) = css3_color_to_hex(&lower) {
            return Some(hex.to_string());
        }
        // Return the lowercase name as-is for any other alphabetic token
        // (e.g. SVG built-in color names).
        return Some(lower);
    }
    None
}
