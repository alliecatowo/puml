use super::catalog::LOCAL_SEQUENCE_THEME_CATALOG;
use super::styles::SequenceStyle;

#[derive(Debug, Clone)]
pub struct SequenceThemePreset {
    pub name: &'static str,
    pub style: SequenceStyle,
}
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
                // Blueprint is a dark theme — set a matching canvas background so
                // the white (#ffffff) arrows are visible against the page (#0a1628).
                // Without this, white arrows are invisible on the default white canvas.
                background_color: Some("#0a1628".to_string()),
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
                participant_font_color: Some("#00ff00".to_string()),
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
                hand_drawn: true,
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
                hand_drawn: true,
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
