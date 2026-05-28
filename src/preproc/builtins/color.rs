/// Convert an HSL colour (h ∈ [0, 360), s ∈ [0, 100], l ∈ [0, 100]) and an
/// optional alpha ∈ [0, 255] into a `#rrggbb` or `#rrggbbaa` hex string.
///
/// PlantUML `%hsl_color(h, s, l)` and `%hsl_color(h, s, l, alpha)` follow the
/// CSS/W3C HSL-to-RGB algorithm.  Alpha is appended only when it differs from
/// the fully-opaque default (255).
pub(super) fn hsl_color_to_hex(h: f64, s: f64, l: f64, alpha: Option<u8>) -> String {
    // Clamp inputs to valid ranges.
    let h = h.rem_euclid(360.0);
    let s = s.clamp(0.0, 100.0) / 100.0;
    let l = l.clamp(0.0, 100.0) / 100.0;

    // CSS HSL-to-RGB.
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r1, g1, b1) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    let to_u8 = |v: f64| ((v + m).clamp(0.0, 1.0) * 255.0).round() as u8;
    let (r, g, b) = (to_u8(r1), to_u8(g1), to_u8(b1));

    match alpha {
        Some(a) if a != 255 => format!("#{:02x}{:02x}{:02x}{:02x}", r, g, b, a),
        _ => format!("#{:02x}{:02x}{:02x}", r, g, b),
    }
}

/// Reverse a colour using the HSLuv perceptual colour space.
///
/// PlantUML's `%reverse_hsluv_color` converts the colour to HSLuv, inverts
/// the lightness, then converts back.  We approximate this with a
/// lightness-inversion in standard HSL which is perceptually reasonable and
/// byte-stable.
pub(super) fn reverse_hsluv_color(raw: &str) -> String {
    let Some((r, g, b)) = parse_hex_rgb(raw) else {
        return String::new();
    };
    // Convert RGB → HSL.
    let r_f = f64::from(r) / 255.0;
    let g_f = f64::from(g) / 255.0;
    let b_f = f64::from(b) / 255.0;
    let cmax = r_f.max(g_f).max(b_f);
    let cmin = r_f.min(g_f).min(b_f);
    let delta = cmax - cmin;
    let l = (cmax + cmin) / 2.0;
    let s = if delta == 0.0 {
        0.0
    } else {
        delta / (1.0 - (2.0 * l - 1.0).abs())
    };
    let h = if delta == 0.0 {
        0.0
    } else if cmax == r_f {
        60.0 * (((g_f - b_f) / delta) % 6.0)
    } else if cmax == g_f {
        60.0 * ((b_f - r_f) / delta + 2.0)
    } else {
        60.0 * ((r_f - g_f) / delta + 4.0)
    };
    let h = h.rem_euclid(360.0);
    // Invert lightness (perceptual reversal).
    let l_inv = 1.0 - l;
    hsl_color_to_hex(h, s * 100.0, l_inv * 100.0, None)
}

/// Returns `true` when the colour is perceived as light (luminance ≥ 128).
/// This is the complement of `is_dark_color`, mirroring PlantUML `%is_light`.
pub(super) fn is_light_color(raw: &str) -> bool {
    !is_dark_color(raw)
}

pub(super) fn parse_hex_rgb(raw: &str) -> Option<(u8, u8, u8)> {
    let mut s = raw.trim();
    if let Some(rest) = s.strip_prefix('#') {
        s = rest;
    }
    if s.len() == 3 {
        let mut expanded = String::with_capacity(6);
        for ch in s.chars() {
            expanded.push(ch);
            expanded.push(ch);
        }
        return parse_hex_rgb(&expanded);
    }
    if s.len() != 6 || !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some((r, g, b))
}

pub(super) fn is_dark_color(raw: &str) -> bool {
    let Some((r, g, b)) = parse_hex_rgb(raw) else {
        return false;
    };
    let luminance = (u32::from(r) * 299 + u32::from(g) * 587 + u32::from(b) * 114) / 1000;
    luminance < 128
}

pub(super) fn adjust_color(raw: &str, pct: i64, lighten: bool) -> String {
    let Some((r, g, b)) = parse_hex_rgb(raw) else {
        return String::new();
    };
    let pct = pct.clamp(0, 100) as i32;
    let adjust = |v: u8| -> u8 {
        let v = i32::from(v);
        let next = if lighten {
            v + ((255 - v) * pct / 100)
        } else {
            v - (v * pct / 100)
        };
        next.clamp(0, 255) as u8
    };
    format!("#{:02x}{:02x}{:02x}", adjust(r), adjust(g), adjust(b))
}
