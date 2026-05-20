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

/// Returns `true` when the hex color string represents a dark color (WCAG luminance < 0.179).
pub fn hex_color_is_dark(hex: &str) -> bool {
    let hex = hex.trim_start_matches('#');
    let (r, g, b) = match hex.len() {
        3 => {
            let digits: Vec<u8> = hex
                .chars()
                .filter_map(|c| u8::from_str_radix(&c.to_string().repeat(2), 16).ok())
                .collect();
            if digits.len() != 3 {
                return false;
            }
            (digits[0], digits[1], digits[2])
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(128);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(128);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(128);
            (r, g, b)
        }
        _ => return false,
    };
    fn linearise(c: u8) -> f64 {
        let s = c as f64 / 255.0;
        if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055_f64).powf(2.4)
        }
    }
    let lum = 0.2126 * linearise(r) + 0.7152 * linearise(g) + 0.0722 * linearise(b);
    lum < 0.179
}

pub(super) fn parse_color_value(value: &str) -> Option<String> {
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
