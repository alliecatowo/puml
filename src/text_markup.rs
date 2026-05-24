/// Shared text and SVG escaping helpers used by Creole and renderer output.
pub fn decode_unicode_escapes(text: &str) -> String {
    if !text.contains("&#")
        && !text.contains("<U+")
        && !text.contains("<u+")
        && !text.contains("<:")
        && !text.contains("<#")
    {
        return text.to_string();
    }

    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < text.len() {
        let rest = &text[i..];

        if let Some((decoded, consumed)) = decode_numeric_character_reference(rest) {
            out.push(decoded);
            i += consumed;
            continue;
        }

        if let Some((decoded, consumed)) = decode_codepoint_tag(rest) {
            out.push(decoded);
            i += consumed;
            continue;
        }

        if let Some((decoded, consumed)) = decode_emoji_tag(rest) {
            out.push_str(&decoded);
            i += consumed;
            continue;
        }

        if let Some((decoded, consumed)) = decode_colored_emoji_tag(rest) {
            out.push_str(&decoded);
            i += consumed;
            continue;
        }

        let ch = rest.chars().next().expect("non-empty rest");
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

pub(crate) fn escape_svg_text(input: &str) -> String {
    escape_svg_text_with_decoded(&decode_unicode_escapes(input))
}

pub(crate) fn escape_svg_attr(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '"' => out.push_str("&quot;"),
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(ch),
        }
    }
    out
}

fn escape_svg_text_with_decoded(decoded: &str) -> String {
    let mut escaped = String::with_capacity(decoded.len());
    for ch in decoded.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn decode_numeric_character_reference(s: &str) -> Option<(char, usize)> {
    if !s.starts_with("&#") {
        return None;
    }

    let after_prefix = &s[2..];
    let (radix, digits_start) = if after_prefix.starts_with('x') || after_prefix.starts_with('X') {
        (16, 3)
    } else {
        (10, 2)
    };
    let close = s[digits_start..].find(';')? + digits_start;
    let digits = &s[digits_start..close];
    if digits.is_empty() || !digits.chars().all(|ch| ch.is_digit(radix) && ch.is_ascii()) {
        return None;
    }

    let value = u32::from_str_radix(digits, radix).ok()?;
    let decoded = char::from_u32(value)?;
    Some((decoded, close + 1))
}

fn decode_codepoint_tag(s: &str) -> Option<(char, usize)> {
    if !s
        .get(..3)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("<u+"))
    {
        return None;
    }

    let close = s[3..].find('>')? + 3;
    let digits = &s[3..close];
    if !valid_codepoint_hex(digits) {
        return None;
    }

    let value = u32::from_str_radix(digits, 16).ok()?;
    let decoded = char::from_u32(value)?;
    Some((decoded, close + 1))
}

fn decode_emoji_tag(s: &str) -> Option<(String, usize)> {
    if !s.starts_with("<:") {
        return None;
    }

    let token_end = s[2..].find(":>")? + 2;
    let token = &s[2..token_end];
    if token.is_empty() {
        return None;
    }

    let decoded = decode_emoji_token(token)?;
    Some((decoded, token_end + 2))
}

fn decode_colored_emoji_tag(s: &str) -> Option<(String, usize)> {
    if !s.starts_with("<#") {
        return None;
    }

    let close = s.find(":>")?;
    let inner = &s[2..close];
    let token_start = inner.rfind(':')? + 1;
    let token = &inner[token_start..];
    if token.is_empty() {
        return None;
    }

    let decoded = decode_emoji_token(token)?;
    Some((decoded, close + 2))
}

fn decode_emoji_token(token: &str) -> Option<String> {
    if valid_codepoint_hex(token) {
        let value = u32::from_str_radix(token, 16).ok()?;
        return char::from_u32(value).map(|ch| ch.to_string());
    }

    let normalized = token.trim().to_ascii_lowercase().replace(['-', ' '], "_");
    let mapped = match normalized.as_str() {
        "calendar" => "📅",
        "check" | "white_check_mark" => "✅",
        "grin" | "grinning" | "smile" | "smiley" => "😀",
        "heart" | "red_heart" => "❤",
        "innocent" => "😇",
        "star" => "⭐",
        "sunglasses" => "😎",
        "sun" | "sunny" => "☀",
        "warning" => "⚠",
        _ if normalized
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '+' || ch == '-')
            && !normalized.is_empty() =>
        {
            return Some(format!(":{normalized}:"));
        }
        _ => return None,
    };
    Some(mapped.to_string())
}

fn valid_codepoint_hex(s: &str) -> bool {
    (1..=6).contains(&s.len()) && s.chars().all(|ch| ch.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_decimal_and_hex_numeric_character_references() {
        assert_eq!(
            decode_unicode_escapes("decimal &#8734; hex &#x221E; upper &#X1F600;"),
            "decimal ∞ hex ∞ upper 😀"
        );
    }

    #[test]
    fn decodes_u_plus_codepoint_tags() {
        assert_eq!(
            decode_unicode_escapes("This is <U+221E> and <u+1F527>"),
            "This is ∞ and 🔧"
        );
    }

    #[test]
    fn decodes_small_emoji_map_and_deterministic_fallback() {
        assert_eq!(
            decode_unicode_escapes("<:calendar:> <:1f600:> <:not_in_small_map:> <#green:sunny:>"),
            "📅 😀 :not_in_small_map: ☀"
        );
    }

    #[test]
    fn leaves_invalid_unicode_escapes_literal() {
        let text = "bad &#xZZ; missing &#9731 no-code <U+110000> no-end <U+221E emoji <::>";
        assert_eq!(decode_unicode_escapes(text), text);
    }

    #[test]
    fn svg_text_decodes_and_escapes() {
        assert_eq!(
            escape_svg_text("A & <B> &#9731; 'q'"),
            "A &amp; &lt;B&gt; ☃ &#39;q&#39;"
        );
    }
}
