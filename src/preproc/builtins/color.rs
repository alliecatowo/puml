pub(in crate::preproc) fn parse_hex_rgb(raw: &str) -> Option<(u8, u8, u8)> {
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

pub(in crate::preproc) fn is_dark_color(raw: &str) -> bool {
    let Some((r, g, b)) = parse_hex_rgb(raw) else {
        return false;
    };
    let luminance = (u32::from(r) * 299 + u32::from(g) * 587 + u32::from(b) * 114) / 1000;
    luminance < 128
}

pub(in crate::preproc) fn adjust_color(raw: &str, pct: i64, lighten: bool) -> String {
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

/// PlantUML-ish truthiness for `%boolval`/`%not`.
pub(in crate::preproc) fn boolval(s: &str) -> bool {
    let t = s.trim();
    if t.is_empty() {
        return false;
    }
    let lower = t.to_ascii_lowercase();
    !matches!(lower.as_str(), "0" | "false" | "no" | "off")
}
