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
    // --- Extended skinparams (#182 wishlist) ---
    /// Horizontal gap (px) between participant header boxes.
    pub participant_padding: Option<i32>,
    /// Padding (px) around `box ... end box` groups.
    pub box_padding: Option<i32>,
    /// Alignment of sequence message labels (left/center/right).
    pub message_align: MessageAlign,
    /// Whether to place the response message label below the arrow.
    pub response_message_below_arrow: bool,
    /// Stroke width (px) for lifeline dashed lines.
    pub lifeline_thickness: Option<i32>,
    /// Override color for sequence message arrow lines.
    pub message_line_color: Option<String>,
    /// Background color for `ref` group boxes.
    pub reference_background_color: Option<String>,
    /// Border color for `ref` group boxes.
    pub reference_border_color: Option<String>,
    /// Font color for group header labels.
    pub group_header_font_color: Option<String>,
    /// Font style for group header labels (normal/bold/italic).
    pub group_header_font_style: GroupHeaderFontStyle,
    /// Allow long message labels to span beyond the sender/receiver gap in teoz-style layouts.
    pub sequence_message_span: bool,
}

/// Alignment of sequence message labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MessageAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// Font style for group header labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GroupHeaderFontStyle {
    #[default]
    Normal,
    Bold,
    Italic,
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
            participant_padding: None,
            box_padding: None,
            message_align: MessageAlign::Left,
            response_message_below_arrow: false,
            lifeline_thickness: None,
            message_line_color: None,
            reference_background_color: None,
            reference_border_color: None,
            group_header_font_color: None,
            group_header_font_style: GroupHeaderFontStyle::Normal,
            sequence_message_span: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceThemePreset {
    pub name: &'static str,
    pub style: SequenceStyle,
}

pub fn class_style_from_sequence_theme(style: &SequenceStyle) -> ClassStyle {
    ClassStyle {
        background_color: style.participant_background_color.clone(),
        border_color: style.participant_border_color.clone(),
        header_color: style.group_background_color.clone(),
        member_color: style.arrow_color.clone(),
        font_color: style.arrow_color.clone(),
        arrow_color: style.arrow_color.clone(),
        font_size: style.default_font_size,
        font_name: style.default_font_name.clone(),
    }
}

pub fn state_style_from_sequence_theme(style: &SequenceStyle) -> StateStyle {
    StateStyle {
        background_color: style.participant_background_color.clone(),
        border_color: style.participant_border_color.clone(),
        arrow_color: style.arrow_color.clone(),
        start_color: style.arrow_color.clone(),
        font_color: style.arrow_color.clone(),
        font_size: style.default_font_size,
    }
}

pub fn component_style_from_sequence_theme(style: &SequenceStyle) -> ComponentStyle {
    ComponentStyle {
        background_color: style.participant_background_color.clone(),
        border_color: style.participant_border_color.clone(),
        interface_color: style.note_background_color.clone(),
        font_color: style.arrow_color.clone(),
        arrow_color: style.arrow_color.clone(),
    }
}

pub fn activity_style_from_sequence_theme(style: &SequenceStyle) -> ActivityStyle {
    ActivityStyle {
        background_color: style.participant_background_color.clone(),
        border_color: style.participant_border_color.clone(),
        diamond_color: style.note_background_color.clone(),
        fork_color: style.arrow_color.clone(),
        font_color: style.arrow_color.clone(),
        arrow_color: style.arrow_color.clone(),
    }
}

pub fn timing_style_from_sequence_theme(style: &SequenceStyle) -> TimingStyle {
    TimingStyle {
        background_color: style
            .background_color
            .clone()
            .unwrap_or_else(|| "#ffffff".to_string()),
        axis_color: style.arrow_color.clone(),
        grid_color: style.lifeline_border_color.clone(),
        signal_background_color: style.participant_background_color.clone(),
        signal_border_color: style.participant_border_color.clone(),
        arrow_color: style.arrow_color.clone(),
        font_color: style.arrow_color.clone(),
    }
}

pub fn chart_style_from_sequence_theme(style: &SequenceStyle) -> ChartStyle {
    ChartStyle {
        background_color: style
            .background_color
            .clone()
            .unwrap_or_else(|| "#ffffff".to_string()),
        axis_color: style.arrow_color.clone(),
        grid_color: style.lifeline_border_color.clone(),
        series_color: style.arrow_color.clone(),
        bar_color: style.participant_border_color.clone(),
        line_color: style.arrow_color.clone(),
        pie_border_color: style.group_border_color.clone(),
        font_color: style.arrow_color.clone(),
    }
}

pub const LOCAL_SEQUENCE_THEME_CATALOG: &[&str] = &[
    "plain",
    "_none_",
    "amiga",
    "aws-orange",
    "blueprint",
    "bluegray",
    "carbon-gray",
    "cerulean",
    "cerulean-outline",
    "crt-amber",
    "crt-green",
    "cyborg",
    "hacker",
    "mars",
    "materia",
    "materia-outline",
    "metal",
    "mimeograph",
    "minty",
    "mono",
    "nautilus",
    "not-so-funny",
    "reddress-darkblue",
    "reddress-darkgreen",
    "reddress-darkorange",
    "reddress-darkred",
    "reddress-lightblue",
    "reddress-lightgreen",
    "reddress-lightorange",
    "reddress-lightred",
    "sandstone",
    "silver",
    "sketchy",
    "sketchy-outline",
    "spacelab",
    "spacelab-white",
    "sunlust",
    "superhero",
    "toy",
    "united",
    "vibrant",
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
        // ── New themes added in parity expansion ────────────────────────────
        "_none_" => Ok(SequenceThemePreset {
            name: "_none_",
            style: SequenceStyle::default(),
        }),
        "amiga" => Ok(SequenceThemePreset {
            name: "amiga",
            style: SequenceStyle {
                arrow_color: "#0055aa".to_string(),
                lifeline_border_color: "#0055aa".to_string(),
                participant_background_color: "#ff6600".to_string(),
                participant_border_color: "#0055aa".to_string(),
                note_background_color: "#fff8cc".to_string(),
                note_border_color: "#ff6600".to_string(),
                group_background_color: "#ffe5cc".to_string(),
                group_border_color: "#0055aa".to_string(),
                ..SequenceStyle::default()
            },
        }),
        "bluegray" => Ok(SequenceThemePreset {
            name: "bluegray",
            style: SequenceStyle {
                arrow_color: "#546e7a".to_string(),
                lifeline_border_color: "#78909c".to_string(),
                participant_background_color: "#eceff1".to_string(),
                participant_border_color: "#546e7a".to_string(),
                note_background_color: "#f5f7f8".to_string(),
                note_border_color: "#78909c".to_string(),
                group_background_color: "#f0f4f5".to_string(),
                group_border_color: "#90a4ae".to_string(),
                ..SequenceStyle::default()
            },
        }),
        "carbon-gray" => Ok(SequenceThemePreset {
            name: "carbon-gray",
            style: SequenceStyle {
                arrow_color: "#e0e0e0".to_string(),
                lifeline_border_color: "#8d8d8d".to_string(),
                participant_background_color: "#393939".to_string(),
                participant_border_color: "#e0e0e0".to_string(),
                note_background_color: "#262626".to_string(),
                note_border_color: "#8d8d8d".to_string(),
                group_background_color: "#2e2e2e".to_string(),
                group_border_color: "#525252".to_string(),
                background_color: Some("#161616".to_string()),
                ..SequenceStyle::default()
            },
        }),
        "materia-outline" => Ok(SequenceThemePreset {
            name: "materia-outline",
            style: SequenceStyle {
                arrow_color: "#2196f3".to_string(),
                lifeline_border_color: "#2196f3".to_string(),
                participant_background_color: "#ffffff".to_string(),
                participant_border_color: "#2196f3".to_string(),
                note_background_color: "#ffffff".to_string(),
                note_border_color: "#f9a825".to_string(),
                group_background_color: "#ffffff".to_string(),
                group_border_color: "#bdbdbd".to_string(),
                ..SequenceStyle::default()
            },
        }),
        "mono" => Ok(SequenceThemePreset {
            name: "mono",
            style: SequenceStyle {
                arrow_color: "#222222".to_string(),
                lifeline_border_color: "#555555".to_string(),
                participant_background_color: "#e8e8e8".to_string(),
                participant_border_color: "#222222".to_string(),
                note_background_color: "#f5f5f5".to_string(),
                note_border_color: "#555555".to_string(),
                group_background_color: "#f0f0f0".to_string(),
                group_border_color: "#777777".to_string(),
                ..SequenceStyle::default()
            },
        }),
        "nautilus" => Ok(SequenceThemePreset {
            name: "nautilus",
            style: SequenceStyle {
                arrow_color: "#00bcd4".to_string(),
                lifeline_border_color: "#006064".to_string(),
                participant_background_color: "#0d2633".to_string(),
                participant_border_color: "#00bcd4".to_string(),
                note_background_color: "#071820".to_string(),
                note_border_color: "#00bcd4".to_string(),
                group_background_color: "#0a1e29".to_string(),
                group_border_color: "#00838f".to_string(),
                background_color: Some("#040f16".to_string()),
                ..SequenceStyle::default()
            },
        }),
        "not-so-funny" => Ok(SequenceThemePreset {
            name: "not-so-funny",
            style: SequenceStyle {
                arrow_color: "#000000".to_string(),
                lifeline_border_color: "#333333".to_string(),
                participant_background_color: "#ffffff".to_string(),
                participant_border_color: "#000000".to_string(),
                note_background_color: "#ffffe0".to_string(),
                note_border_color: "#000000".to_string(),
                group_background_color: "#f9f9f9".to_string(),
                group_border_color: "#333333".to_string(),
                ..SequenceStyle::default()
            },
        }),
        "reddress-darkgreen" => Ok(SequenceThemePreset {
            name: "reddress-darkgreen",
            style: SequenceStyle {
                arrow_color: "#cc0000".to_string(),
                lifeline_border_color: "#cc0000".to_string(),
                participant_background_color: "#1a3a20".to_string(),
                participant_border_color: "#cc0000".to_string(),
                note_background_color: "#0f2414".to_string(),
                note_border_color: "#cc0000".to_string(),
                group_background_color: "#152a1a".to_string(),
                group_border_color: "#2e6b3a".to_string(),
                ..SequenceStyle::default()
            },
        }),
        "reddress-darkorange" => Ok(SequenceThemePreset {
            name: "reddress-darkorange",
            style: SequenceStyle {
                arrow_color: "#cc0000".to_string(),
                lifeline_border_color: "#cc0000".to_string(),
                participant_background_color: "#3a200a".to_string(),
                participant_border_color: "#cc0000".to_string(),
                note_background_color: "#241306".to_string(),
                note_border_color: "#cc0000".to_string(),
                group_background_color: "#2a1800".to_string(),
                group_border_color: "#8a4800".to_string(),
                ..SequenceStyle::default()
            },
        }),
        "reddress-darkred" => Ok(SequenceThemePreset {
            name: "reddress-darkred",
            style: SequenceStyle {
                arrow_color: "#cc0000".to_string(),
                lifeline_border_color: "#cc0000".to_string(),
                participant_background_color: "#3a0a0a".to_string(),
                participant_border_color: "#cc0000".to_string(),
                note_background_color: "#240606".to_string(),
                note_border_color: "#cc0000".to_string(),
                group_background_color: "#2a0808".to_string(),
                group_border_color: "#8a2020".to_string(),
                ..SequenceStyle::default()
            },
        }),
        "reddress-lightblue" => Ok(SequenceThemePreset {
            name: "reddress-lightblue",
            style: SequenceStyle {
                arrow_color: "#cc0000".to_string(),
                lifeline_border_color: "#cc0000".to_string(),
                participant_background_color: "#d6e8f5".to_string(),
                participant_border_color: "#cc0000".to_string(),
                note_background_color: "#eef4fb".to_string(),
                note_border_color: "#cc0000".to_string(),
                group_background_color: "#e4eef8".to_string(),
                group_border_color: "#5b8dc4".to_string(),
                ..SequenceStyle::default()
            },
        }),
        "reddress-lightgreen" => Ok(SequenceThemePreset {
            name: "reddress-lightgreen",
            style: SequenceStyle {
                arrow_color: "#cc0000".to_string(),
                lifeline_border_color: "#cc0000".to_string(),
                participant_background_color: "#d6f0d6".to_string(),
                participant_border_color: "#cc0000".to_string(),
                note_background_color: "#edf8ed".to_string(),
                note_border_color: "#cc0000".to_string(),
                group_background_color: "#e4f5e4".to_string(),
                group_border_color: "#4aaa4a".to_string(),
                ..SequenceStyle::default()
            },
        }),
        "reddress-lightorange" => Ok(SequenceThemePreset {
            name: "reddress-lightorange",
            style: SequenceStyle {
                arrow_color: "#cc0000".to_string(),
                lifeline_border_color: "#cc0000".to_string(),
                participant_background_color: "#fde9cc".to_string(),
                participant_border_color: "#cc0000".to_string(),
                note_background_color: "#fef4e6".to_string(),
                note_border_color: "#cc0000".to_string(),
                group_background_color: "#fdeedd".to_string(),
                group_border_color: "#e08a20".to_string(),
                ..SequenceStyle::default()
            },
        }),
        "reddress-lightred" => Ok(SequenceThemePreset {
            name: "reddress-lightred",
            style: SequenceStyle {
                arrow_color: "#cc0000".to_string(),
                lifeline_border_color: "#cc0000".to_string(),
                participant_background_color: "#f8d6d6".to_string(),
                participant_border_color: "#cc0000".to_string(),
                note_background_color: "#fdeaea".to_string(),
                note_border_color: "#cc0000".to_string(),
                group_background_color: "#fae2e2".to_string(),
                group_border_color: "#cc5555".to_string(),
                ..SequenceStyle::default()
            },
        }),
        "spacelab-white" => Ok(SequenceThemePreset {
            name: "spacelab-white",
            style: SequenceStyle {
                arrow_color: "#2f4f6f".to_string(),
                lifeline_border_color: "#6d7f91".to_string(),
                participant_background_color: "#ffffff".to_string(),
                participant_border_color: "#2f4f6f".to_string(),
                note_background_color: "#ffffff".to_string(),
                note_border_color: "#5f7388".to_string(),
                group_background_color: "#ffffff".to_string(),
                group_border_color: "#7b8da0".to_string(),
                ..SequenceStyle::default()
            },
        }),
        "sunlust" => Ok(SequenceThemePreset {
            name: "sunlust",
            style: SequenceStyle {
                arrow_color: "#f57f17".to_string(),
                lifeline_border_color: "#e65100".to_string(),
                participant_background_color: "#fff8e1".to_string(),
                participant_border_color: "#f57f17".to_string(),
                note_background_color: "#fffde7".to_string(),
                note_border_color: "#f9a825".to_string(),
                group_background_color: "#fff9c4".to_string(),
                group_border_color: "#f9a825".to_string(),
                ..SequenceStyle::default()
            },
        }),
        "toy" => Ok(SequenceThemePreset {
            name: "toy",
            style: SequenceStyle {
                arrow_color: "#e53935".to_string(),
                lifeline_border_color: "#1e88e5".to_string(),
                participant_background_color: "#e3f2fd".to_string(),
                participant_border_color: "#e53935".to_string(),
                note_background_color: "#fff9c4".to_string(),
                note_border_color: "#43a047".to_string(),
                group_background_color: "#fce4ec".to_string(),
                group_border_color: "#e53935".to_string(),
                ..SequenceStyle::default()
            },
        }),
        "vibrant" => Ok(SequenceThemePreset {
            name: "vibrant",
            style: SequenceStyle {
                arrow_color: "#7c3aed".to_string(),
                lifeline_border_color: "#6d28d9".to_string(),
                participant_background_color: "#ede9fe".to_string(),
                participant_border_color: "#7c3aed".to_string(),
                note_background_color: "#fef3c7".to_string(),
                note_border_color: "#d97706".to_string(),
                group_background_color: "#f5f3ff".to_string(),
                group_border_color: "#8b5cf6".to_string(),
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
    // --- Extended skinparams (#182 wishlist) ---
    ParticipantPadding(i32),
    BoxPadding(i32),
    MessageAlign(MessageAlign),
    ResponseMessageBelowArrow(bool),
    LifelineThickness(i32),
    MessageLineColor(String),
    ReferenceBackgroundColor(String),
    ReferenceBorderColor(String),
    GroupHeaderFontColor(String),
    GroupHeaderFontStyle(GroupHeaderFontStyle),
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
        // --- Extended skinparams (#182 wishlist) ---
        "participantpadding" => {
            if let Ok(n) = value.trim().parse::<i32>() {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::ParticipantPadding(n),
                )
            } else {
                SequenceSkinParamSupport::UnsupportedValue
            }
        }
        "boxpadding" => {
            if let Ok(n) = value.trim().parse::<i32>() {
                SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::BoxPadding(n))
            } else {
                SequenceSkinParamSupport::UnsupportedValue
            }
        }
        "sequencemessagealign" => {
            let lower = value.trim().to_ascii_lowercase();
            let align = match lower.as_str() {
                "left" => MessageAlign::Left,
                "center" => MessageAlign::Center,
                "right" => MessageAlign::Right,
                _ => return SequenceSkinParamSupport::UnsupportedValue,
            };
            SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::MessageAlign(
                align,
            ))
        }
        "responsemessagebelowarrow" | "sequenceresponsemessagebelowarrow" => {
            let lower = value.trim().to_ascii_lowercase();
            let enabled = match lower.as_str() {
                "true" | "yes" | "on" => true,
                "false" | "no" | "off" => false,
                _ => return SequenceSkinParamSupport::UnsupportedValue,
            };
            SequenceSkinParamSupport::SupportedWithValue(
                SequenceSkinParamValue::ResponseMessageBelowArrow(enabled),
            )
        }
        "sequencelifelinethickness" => {
            if let Ok(n) = value.trim().parse::<i32>() {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::LifelineThickness(n),
                )
            } else {
                SequenceSkinParamSupport::UnsupportedValue
            }
        }
        "messagelinecolor" | "sequencemessagelinecolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::MessageLineColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "referencebackgroundcolor" | "sequencereferencebackgroundcolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::ReferenceBackgroundColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "referencebordercolor" | "sequencereferencebordercolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::ReferenceBorderColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "sequencegroupheaderfontcolor" => parse_color_value(value)
            .map(|color| {
                SequenceSkinParamSupport::SupportedWithValue(
                    SequenceSkinParamValue::GroupHeaderFontColor(color),
                )
            })
            .unwrap_or(SequenceSkinParamSupport::UnsupportedValue),
        "sequencegroupheaderfontstyle" => {
            let lower = value.trim().to_ascii_lowercase();
            let style = match lower.as_str() {
                "normal" => GroupHeaderFontStyle::Normal,
                "bold" => GroupHeaderFontStyle::Bold,
                "italic" => GroupHeaderFontStyle::Italic,
                _ => return SequenceSkinParamSupport::UnsupportedValue,
            };
            SequenceSkinParamSupport::SupportedWithValue(
                SequenceSkinParamValue::GroupHeaderFontStyle(style),
            )
        }
        _ => SequenceSkinParamSupport::UnsupportedKey,
    }
}

// ─── Class-family skinparam support ─────────────────────────────────────────

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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkinParamSupport<V> {
    SupportedNoop,
    SupportedWithValue(V),
    UnsupportedKey,
    UnsupportedValue,
}

pub fn classify_class_skinparam(key: &str, value: &str) -> SkinParamSupport<ClassSkinParamValue> {
    let normalized = key.trim().to_ascii_lowercase();
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
        "classfontname" | "objectfontname" | "usecasefontname" | "actorfontname" => {
            let name = value.trim();
            if name.is_empty() {
                SkinParamSupport::UnsupportedValue
            } else {
                SkinParamSupport::SupportedWithValue(ClassSkinParamValue::FontName(
                    name.to_string(),
                ))
            }
        }
        "classstereotypefontcolor"
        | "classstereotypefontsize"
        | "classstereotypefontname"
        | "classattributefontsize"
        | "classmethodfontsize"
        | "objectstereotypefontcolor"
        | "usecasestereotypefontcolor"
        | "actorstereotypefontcolor"
        | "roundcorner"
        | "shadowing" => SkinParamSupport::SupportedNoop,
        _ => SkinParamSupport::UnsupportedKey,
    }
}

// ─── State-family skinparam support ─────────────────────────────────────────

/// Style overrides for state diagrams.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateStyle {
    pub background_color: String,
    pub border_color: String,
    pub arrow_color: String,
    pub start_color: String,
    pub font_color: String,
    pub font_size: Option<u32>,
}

impl Default for StateStyle {
    fn default() -> Self {
        Self {
            background_color: "#f6f6f6".to_string(),
            border_color: "#1e293b".to_string(),
            arrow_color: "#1e293b".to_string(),
            start_color: "#0f172a".to_string(),
            font_color: "#0f172a".to_string(),
            font_size: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateSkinParamValue {
    BackgroundColor(String),
    BorderColor(String),
    ArrowColor(String),
    StartColor(String),
    FontColor(String),
    FontSize(u32),
}

pub fn classify_state_skinparam(key: &str, value: &str) -> SkinParamSupport<StateSkinParamValue> {
    let normalized = key.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "backgroundcolor" | "statebackgroundcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(StateSkinParamValue::BackgroundColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "bordercolor" | "statebordercolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(StateSkinParamValue::BorderColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "arrowcolor" | "statearrowcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(StateSkinParamValue::ArrowColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "statestartcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(StateSkinParamValue::StartColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontcolor" | "statefontcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(StateSkinParamValue::FontColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontsize" | "statefontsize" => {
            if let Ok(n) = value.trim().parse::<u32>() {
                SkinParamSupport::SupportedWithValue(StateSkinParamValue::FontSize(n))
            } else {
                SkinParamSupport::UnsupportedValue
            }
        }
        "statefontname"
        | "statestereotypefontcolor"
        | "statestereotypefontsize"
        | "statestereotypefontname"
        | "stateattributefontcolor"
        | "stateattributefontsize" => SkinParamSupport::SupportedNoop,
        _ => SkinParamSupport::UnsupportedKey,
    }
}

// ─── Component-family skinparam support ──────────────────────────────────────

/// Style overrides for component/deployment diagrams.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentStyle {
    pub background_color: String,
    pub border_color: String,
    pub interface_color: String,
    pub font_color: String,
    pub arrow_color: String,
}

impl Default for ComponentStyle {
    fn default() -> Self {
        Self {
            background_color: "#f0f4f8".to_string(),
            border_color: "#1e293b".to_string(),
            interface_color: "#e2e8f0".to_string(),
            font_color: "#0f172a".to_string(),
            arrow_color: "#1e293b".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComponentSkinParamValue {
    BackgroundColor(String),
    BorderColor(String),
    InterfaceColor(String),
    FontColor(String),
    ArrowColor(String),
}

pub fn classify_component_skinparam(
    key: &str,
    value: &str,
) -> SkinParamSupport<ComponentSkinParamValue> {
    let normalized = key.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "backgroundcolor"
        | "componentbackgroundcolor"
        | "deploymentbackgroundcolor"
        | "nodebackgroundcolor"
        | "artifactbackgroundcolor"
        | "databasebackgroundcolor" => parse_color_value(value)
            .map(|c| {
                SkinParamSupport::SupportedWithValue(ComponentSkinParamValue::BackgroundColor(c))
            })
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "bordercolor"
        | "componentbordercolor"
        | "deploymentbordercolor"
        | "nodebordercolor"
        | "artifactbordercolor"
        | "databasebordercolor" => parse_color_value(value)
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
        | "nodefontcolor"
        | "artifactfontcolor"
        | "databasefontcolor"
        | "portfontcolor"
        | "interfacefontcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(ComponentSkinParamValue::FontColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "arrowcolor" | "componentarrowcolor" | "deploymentarrowcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(ComponentSkinParamValue::ArrowColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
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
        | "componentstyle"
        | "componentstereotypefontcolor"
        | "componentstereotypefontsize"
        | "componentstereotypefontname"
        | "deploymentstereotypefontcolor"
        | "deploymentstereotypefontsize"
        | "deploymentstereotypefontname"
        | "portfontsize"
        | "portfontname" => SkinParamSupport::SupportedNoop,
        _ => SkinParamSupport::UnsupportedKey,
    }
}

// ─── Activity-family skinparam support ───────────────────────────────────────

/// Style overrides for activity diagrams.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityStyle {
    pub background_color: String,
    pub border_color: String,
    pub diamond_color: String,
    pub fork_color: String,
    pub font_color: String,
    pub arrow_color: String,
}

impl Default for ActivityStyle {
    fn default() -> Self {
        Self {
            background_color: "#ecfdf5".to_string(),
            border_color: "#047857".to_string(),
            diamond_color: "#fef9c3".to_string(),
            fork_color: "#0f172a".to_string(),
            font_color: "#0f172a".to_string(),
            arrow_color: "#0f172a".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivitySkinParamValue {
    BackgroundColor(String),
    BorderColor(String),
    DiamondBackgroundColor(String),
    BarColor(String),
    FontColor(String),
    ArrowColor(String),
}

pub fn classify_activity_skinparam(
    key: &str,
    value: &str,
) -> SkinParamSupport<ActivitySkinParamValue> {
    let normalized = key.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "backgroundcolor" | "activitybackgroundcolor" | "activitypartitionbackgroundcolor" => {
            parse_color_value(value)
                .map(|c| {
                    SkinParamSupport::SupportedWithValue(ActivitySkinParamValue::BackgroundColor(c))
                })
                .unwrap_or(SkinParamSupport::UnsupportedValue)
        }
        "bordercolor"
        | "activitybordercolor"
        | "activitypartitionbordercolor"
        | "swimlanebordercolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(ActivitySkinParamValue::BorderColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "activitydiamondbackgroundcolor" | "activitydiamondcolor" => parse_color_value(value)
            .map(|c| {
                SkinParamSupport::SupportedWithValue(
                    ActivitySkinParamValue::DiamondBackgroundColor(c),
                )
            })
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "activitybarcolor" | "activitystartcolor" | "activityendcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(ActivitySkinParamValue::BarColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontcolor" | "activityfontcolor" | "swimlanefontcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(ActivitySkinParamValue::FontColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "arrowcolor" | "activityarrowcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(ActivitySkinParamValue::ArrowColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "activityfontsize"
        | "activityfontname"
        | "activityborderthickness"
        | "activitypartitionfontcolor"
        | "activitypartitionfontsize"
        | "swimlanefontsize" => SkinParamSupport::SupportedNoop,
        _ => SkinParamSupport::UnsupportedKey,
    }
}

// ─── Timing-family skinparam support ────────────────────────────────────────

/// Style overrides for timing diagrams.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimingStyle {
    pub background_color: String,
    pub axis_color: String,
    pub grid_color: String,
    pub signal_background_color: String,
    pub signal_border_color: String,
    pub arrow_color: String,
    pub font_color: String,
}

impl Default for TimingStyle {
    fn default() -> Self {
        Self {
            background_color: "#ffffff".to_string(),
            axis_color: "#64748b".to_string(),
            grid_color: "#cbd5e1".to_string(),
            signal_background_color: "#f8fafc".to_string(),
            signal_border_color: "#0f172a".to_string(),
            arrow_color: "#0ea5e9".to_string(),
            font_color: "#0f172a".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimingSkinParamValue {
    BackgroundColor(String),
    AxisColor(String),
    GridColor(String),
    SignalBackgroundColor(String),
    SignalBorderColor(String),
    ArrowColor(String),
    FontColor(String),
}

pub fn classify_timing_skinparam(key: &str, value: &str) -> SkinParamSupport<TimingSkinParamValue> {
    let normalized = key.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "backgroundcolor" | "timingbackgroundcolor" | "timingdiagrambackgroundcolor" => {
            parse_color_value(value)
                .map(|c| {
                    SkinParamSupport::SupportedWithValue(TimingSkinParamValue::BackgroundColor(c))
                })
                .unwrap_or(SkinParamSupport::UnsupportedValue)
        }
        "timingaxiscolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(TimingSkinParamValue::AxisColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "timinggridcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(TimingSkinParamValue::GridColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "timingsignalbackgroundcolor" | "timingparticipantbackgroundcolor" => {
            parse_color_value(value)
                .map(|c| {
                    SkinParamSupport::SupportedWithValue(
                        TimingSkinParamValue::SignalBackgroundColor(c),
                    )
                })
                .unwrap_or(SkinParamSupport::UnsupportedValue)
        }
        "timingsignalbordercolor" | "timingparticipantbordercolor" => parse_color_value(value)
            .map(|c| {
                SkinParamSupport::SupportedWithValue(TimingSkinParamValue::SignalBorderColor(c))
            })
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "arrowcolor" | "timingarrowcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(TimingSkinParamValue::ArrowColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontcolor" | "timingfontcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(TimingSkinParamValue::FontColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "timingfontsize" | "timingfontname" => SkinParamSupport::SupportedNoop,
        _ => SkinParamSupport::UnsupportedKey,
    }
}

// ─── Chart-family skinparam support ─────────────────────────────────────────

/// Style overrides for chart diagrams.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChartStyle {
    pub background_color: String,
    pub axis_color: String,
    pub grid_color: String,
    pub series_color: String,
    pub bar_color: String,
    pub line_color: String,
    pub pie_border_color: String,
    pub font_color: String,
}

impl Default for ChartStyle {
    fn default() -> Self {
        Self {
            background_color: "#ffffff".to_string(),
            axis_color: "#0f172a".to_string(),
            grid_color: "#e2e8f0".to_string(),
            series_color: "#1d4ed8".to_string(),
            bar_color: "#1d4ed8".to_string(),
            line_color: "#1d4ed8".to_string(),
            pie_border_color: "#0f172a".to_string(),
            font_color: "#0f172a".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChartSkinParamValue {
    BackgroundColor(String),
    AxisColor(String),
    GridColor(String),
    SeriesColor(String),
    BarColor(String),
    LineColor(String),
    PieBorderColor(String),
    FontColor(String),
}

pub fn classify_chart_skinparam(key: &str, value: &str) -> SkinParamSupport<ChartSkinParamValue> {
    let normalized = key.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "backgroundcolor" | "chartbackgroundcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(ChartSkinParamValue::BackgroundColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "axiscolor" | "chartaxiscolor" | "chartaxislinecolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(ChartSkinParamValue::AxisColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "gridcolor" | "chartgridcolor" | "chartgridlinecolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(ChartSkinParamValue::GridColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "chartseriescolor" | "seriescolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(ChartSkinParamValue::SeriesColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "chartbarcolor" | "barcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(ChartSkinParamValue::BarColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "chartlinecolor" | "linecolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(ChartSkinParamValue::LineColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "chartpiebordercolor" | "piebordercolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(ChartSkinParamValue::PieBorderColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontcolor" | "chartfontcolor" | "chartlabelfontcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(ChartSkinParamValue::FontColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "chartfontsize" | "chartfontname" | "legendfontcolor" | "legendfontsize" => {
            SkinParamSupport::SupportedNoop
        }
        _ => SkinParamSupport::UnsupportedKey,
    }
}

// ─── Generic skinparam value type used by families that don't yet wire style ──

/// Shared value type for classify functions that recognize common skinparam
/// keys but haven't wired them to per-family style structs yet.
#[derive(Debug, Clone, PartialEq)]
pub enum GenericSkinParamValue {
    BackgroundColor(String),
    BorderColor(String),
    FontColor(String),
    FontSize(u32),
}

// ─── Gantt ────────────────────────────────────────────────────────────────────

/// Classify a skinparam key/value pair for Gantt diagrams.
pub fn classify_gantt_skinparam(
    key: &str,
    value: &str,
) -> SkinParamSupport<GenericSkinParamValue> {
    let normalized = key.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "backgroundcolor" | "ganttbackgroundcolor" | "ganttdiagrambackgroundcolor" => {
            parse_color_value(value)
                .map(|c| {
                    SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BackgroundColor(c))
                })
                .unwrap_or(SkinParamSupport::UnsupportedValue)
        }
        "bordercolor" | "ganttbordercolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BorderColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontcolor" | "ganttfontcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontsize" | "ganttfontsize" => value
            .trim()
            .parse::<u32>()
            .ok()
            .map(|n| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontSize(n)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "arrowcolor"
        | "ganttarrowcolor"
        | "ganttlinecolor"
        | "todaycolor"
        | "closedcolor"
        | "opencolor"
        | "milestonestyle"
        | "ganttdiagramarrowcolor"
        | "fontname"
        | "ganttfontname" => SkinParamSupport::SupportedNoop,
        _ => SkinParamSupport::UnsupportedKey,
    }
}

// ─── MindMap ──────────────────────────────────────────────────────────────────

/// Classify a skinparam key/value pair for MindMap diagrams.
pub fn classify_mindmap_skinparam(
    key: &str,
    value: &str,
) -> SkinParamSupport<GenericSkinParamValue> {
    let normalized = key.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "backgroundcolor" | "mindmapbackgroundcolor" | "nodebordercolor" => {
            parse_color_value(value)
                .map(|c| {
                    SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BackgroundColor(c))
                })
                .unwrap_or(SkinParamSupport::UnsupportedValue)
        }
        "bordercolor" | "mindmapbordercolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BorderColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontcolor" | "mindmapfontcolor" | "nodefontcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontsize" | "mindmapfontsize" | "nodefontsize" => value
            .trim()
            .parse::<u32>()
            .ok()
            .map(|n| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontSize(n)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "arrowcolor"
        | "mindmaparrowcolor"
        | "nodefontname"
        | "mindmapfontname"
        | "roundcorner"
        | "mindmaproundcorner" => SkinParamSupport::SupportedNoop,
        _ => SkinParamSupport::UnsupportedKey,
    }
}

// ─── WBS ──────────────────────────────────────────────────────────────────────

/// Classify a skinparam key/value pair for WBS (Work Breakdown Structure) diagrams.
pub fn classify_wbs_skinparam(key: &str, value: &str) -> SkinParamSupport<GenericSkinParamValue> {
    let normalized = key.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "backgroundcolor" | "wbsbackgroundcolor" => parse_color_value(value)
            .map(|c| {
                SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BackgroundColor(c))
            })
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "bordercolor" | "wbsbordercolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BorderColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontcolor" | "wbsfontcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontsize" | "wbsfontsize" => value
            .trim()
            .parse::<u32>()
            .ok()
            .map(|n| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontSize(n)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "arrowcolor" | "wbsarrowcolor" | "wbsfontname" | "fontname" | "roundcorner" => {
            SkinParamSupport::SupportedNoop
        }
        _ => SkinParamSupport::UnsupportedKey,
    }
}

// ─── Timeline (Chronology) ────────────────────────────────────────────────────

/// Classify a skinparam key/value pair for Timeline/Chronology diagrams.
pub fn classify_timeline_skinparam(
    key: &str,
    value: &str,
) -> SkinParamSupport<GenericSkinParamValue> {
    let normalized = key.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "backgroundcolor" | "timelinebackgroundcolor" => parse_color_value(value)
            .map(|c| {
                SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BackgroundColor(c))
            })
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "bordercolor" | "timelinebordercolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BorderColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontcolor" | "timelinefontcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontsize" | "timelinefontsize" => value
            .trim()
            .parse::<u32>()
            .ok()
            .map(|n| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontSize(n)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "arrowcolor" | "timelinearrowcolor" | "timelinefontname" | "fontname" => {
            SkinParamSupport::SupportedNoop
        }
        _ => SkinParamSupport::UnsupportedKey,
    }
}

// ─── NwDiag ───────────────────────────────────────────────────────────────────

/// Classify a skinparam key/value pair for NwDiag (network diagram) diagrams.
pub fn classify_nwdiag_skinparam(
    key: &str,
    value: &str,
) -> SkinParamSupport<GenericSkinParamValue> {
    let normalized = key.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "backgroundcolor" | "nwdiagbackgroundcolor" => parse_color_value(value)
            .map(|c| {
                SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BackgroundColor(c))
            })
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "bordercolor" | "nwdiagbordercolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BorderColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontcolor" | "nwdiagfontcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontsize" | "nwdiagfontsize" => value
            .trim()
            .parse::<u32>()
            .ok()
            .map(|n| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontSize(n)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "arrowcolor"
        | "nwdiagarrowcolor"
        | "nwdiagfontname"
        | "fontname"
        | "networkcolor"
        | "nwdiagnetworkcolor" => SkinParamSupport::SupportedNoop,
        _ => SkinParamSupport::UnsupportedKey,
    }
}

// ─── Archimate ────────────────────────────────────────────────────────────────

/// Classify a skinparam key/value pair for Archimate diagrams.
pub fn classify_archimate_skinparam(
    key: &str,
    value: &str,
) -> SkinParamSupport<GenericSkinParamValue> {
    let normalized = key.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "backgroundcolor" | "archimatebackgroundcolor" => parse_color_value(value)
            .map(|c| {
                SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BackgroundColor(c))
            })
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "bordercolor" | "archimatebordercolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BorderColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontcolor" | "archimatefontcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontsize" | "archimateontsize" => value
            .trim()
            .parse::<u32>()
            .ok()
            .map(|n| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontSize(n)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "arrowcolor"
        | "archimatearrowcolor"
        | "archimatefontname"
        | "fontname"
        | "roundcorner"
        | "archimatestyle" => SkinParamSupport::SupportedNoop,
        _ => SkinParamSupport::UnsupportedKey,
    }
}

// ─── SDL ──────────────────────────────────────────────────────────────────────

/// Classify a skinparam key/value pair for SDL (Specification and Description Language) diagrams.
pub fn classify_sdl_skinparam(key: &str, value: &str) -> SkinParamSupport<GenericSkinParamValue> {
    let normalized = key.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "backgroundcolor" | "sdlbackgroundcolor" => parse_color_value(value)
            .map(|c| {
                SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BackgroundColor(c))
            })
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "bordercolor" | "sdlbordercolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BorderColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontcolor" | "sdlfontcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontsize" | "sdlfontsize" => value
            .trim()
            .parse::<u32>()
            .ok()
            .map(|n| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontSize(n)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "arrowcolor" | "sdlarrowcolor" | "sdlfontname" | "fontname" => {
            SkinParamSupport::SupportedNoop
        }
        _ => SkinParamSupport::UnsupportedKey,
    }
}

// ─── Ditaa ────────────────────────────────────────────────────────────────────

/// Classify a skinparam key/value pair for Ditaa diagrams.
pub fn classify_ditaa_skinparam(
    key: &str,
    value: &str,
) -> SkinParamSupport<GenericSkinParamValue> {
    let normalized = key.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "backgroundcolor" | "ditaabackgroundcolor" => parse_color_value(value)
            .map(|c| {
                SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BackgroundColor(c))
            })
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "bordercolor" | "ditaabordercolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BorderColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontcolor" | "ditaafontcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontsize" | "ditaafontsize" => value
            .trim()
            .parse::<u32>()
            .ok()
            .map(|n| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontSize(n)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontname" | "ditaafontname" | "shadowing" | "ditaashadowing" => {
            SkinParamSupport::SupportedNoop
        }
        _ => SkinParamSupport::UnsupportedKey,
    }
}

// ─── Salt ─────────────────────────────────────────────────────────────────────

/// Classify a skinparam key/value pair for Salt (wireframe/UI mockup) diagrams.
pub fn classify_salt_skinparam(key: &str, value: &str) -> SkinParamSupport<GenericSkinParamValue> {
    let normalized = key.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "backgroundcolor" | "saltbackgroundcolor" => parse_color_value(value)
            .map(|c| {
                SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BackgroundColor(c))
            })
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "bordercolor" | "saltbordercolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BorderColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontcolor" | "saltfontcolor" => parse_color_value(value)
            .map(|c| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontColor(c)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontsize" | "saltfontsize" => value
            .trim()
            .parse::<u32>()
            .ok()
            .map(|n| SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontSize(n)))
            .unwrap_or(SkinParamSupport::UnsupportedValue),
        "fontname" | "saltfontname" | "roundcorner" | "saltroundcorner" => {
            SkinParamSupport::SupportedNoop
        }
        _ => SkinParamSupport::UnsupportedKey,
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
