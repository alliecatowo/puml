use super::*;
#[derive(Debug, Clone, Default)]
pub(crate) struct ParsedFamilyRelationStyle {
    pub(crate) line_color: Option<String>,
    pub(crate) dashed: bool,
    pub(crate) hidden: bool,
    pub(crate) thickness: Option<u8>,
    pub(crate) direction: Option<String>,
}

pub(crate) fn split_family_arrow(core: &str) -> Option<(&str, String, &str)> {
    split_family_arrow_styled(core).map(|(lhs, arrow, _, rhs)| (lhs, arrow, rhs))
}

pub(crate) fn split_family_arrow_styled(
    core: &str,
) -> Option<(&str, String, ParsedFamilyRelationStyle, &str)> {
    let mut in_quote = false;
    for (idx, ch) in core.char_indices() {
        if ch == '"' {
            in_quote = !in_quote;
            continue;
        }
        if in_quote {
            continue;
        }
        if !matches!(
            ch,
            '-' | '.'
                | '<'
                | '*'
                | 'o'
                | '+'
                | '|'
                | '{'
                | '}'
                | '>'
                | '0'
                | '('
                | ')'
                | '@'
                | '^'
                | '#'
                | '\\'
                | '/'
                | ':'
        ) {
            continue;
        }
        let rest = &core[idx..];
        let Some(len) = family_arrow_token_len(rest) else {
            continue;
        };
        if len == 1 {
            continue;
        }
        let mut arrow_start = idx;
        let mut arrow_len = len;
        let raw_candidate = &rest[..len];
        if raw_candidate.starts_with("()") && raw_candidate[2..].contains('-') {
            arrow_start += 2;
            arrow_len = arrow_len.saturating_sub(2);
        }
        if arrow_len > 2
            && core[arrow_start..arrow_start + arrow_len].ends_with("()")
            && core[arrow_start..arrow_start + arrow_len - 2].contains('-')
        {
            arrow_len -= 2;
        }
        if arrow_len == 1 {
            continue;
        }
        let lhs = core[..arrow_start].trim();
        let rhs = core[arrow_start + arrow_len..].trim();
        if lhs.is_empty() || rhs.is_empty() {
            continue;
        }
        let raw_arrow = &core[arrow_start..arrow_start + arrow_len];
        let arrow = normalize_family_arrow_token(raw_arrow);
        if arrow.is_empty() {
            continue;
        }
        let mut relation_style = parse_family_relation_style(raw_arrow);
        relation_style.direction = parse_family_relation_direction(raw_arrow);
        return Some((lhs, arrow, relation_style, rhs));
    }
    None
}

pub(crate) fn parse_family_relation_style(raw_arrow: &str) -> ParsedFamilyRelationStyle {
    let mut style = ParsedFamilyRelationStyle::default();
    let mut rest = raw_arrow;
    while let Some(open) = rest.find('[') {
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find(']') else {
            break;
        };
        let content = &after_open[..close];
        for part in content.split(|ch: char| ch == ',' || ch == ';' || ch.is_whitespace()) {
            let token = part.trim();
            if token.is_empty() {
                continue;
            }
            let lower_raw = token.to_ascii_lowercase();
            let lower = lower_raw
                .strip_prefix("line.")
                .or_else(|| lower_raw.strip_prefix("line:"))
                .unwrap_or(lower_raw.as_str());
            match lower {
                "dashed" | "dotted" | "dash" | "dot" => style.dashed = true,
                "hidden" => style.hidden = true,
                "bold" | "thick" => style.thickness = Some(style.thickness.unwrap_or(3).max(3)),
                "thin" => style.thickness = Some(1),
                _ => {
                    if let Some(value) = lower
                        .strip_prefix("thickness=")
                        .or_else(|| lower.strip_prefix("thickness:"))
                        .or_else(|| lower.strip_prefix("weight="))
                        .or_else(|| lower.strip_prefix("weight:"))
                    {
                        if let Ok(n) = value.trim().parse::<u8>() {
                            style.thickness = Some(n.clamp(1, 8));
                        }
                    } else if let Some(color) =
                        crate::theme::color::parse_relation_color_token(token)
                    {
                        style.line_color = Some(color);
                    }
                }
            }
        }
        rest = &after_open[close + 1..];
    }
    style
}

pub(crate) fn parse_family_relation_direction(raw_arrow: &str) -> Option<String> {
    let mut cleaned = String::new();
    let mut in_bracket = false;
    for ch in raw_arrow.chars() {
        match ch {
            '[' => in_bracket = true,
            ']' => in_bracket = false,
            _ if !in_bracket => cleaned.push(ch),
            _ => {}
        }
    }
    let lower = cleaned.to_ascii_lowercase();
    for (needle, direction) in [
        ("left", "left"),
        ("right", "right"),
        ("up", "up"),
        ("down", "down"),
        ("l", "left"),
        ("r", "right"),
        ("u", "up"),
        ("d", "down"),
    ] {
        if lower.contains(needle) {
            return Some(direction.to_string());
        }
    }
    None
}

pub(crate) fn family_arrow_token_len(s: &str) -> Option<usize> {
    if let Some(len) = directional_family_arrow_token_len(s) {
        return Some(len);
    }

    let len = s
        .char_indices()
        .take_while(|(_, ch)| {
            matches!(
                ch,
                '-' | '.'
                    | '<'
                    | '>'
                    | '|'
                    | '*'
                    | 'o'
                    | '+'
                    | '{'
                    | '}'
                    | '0'
                    | '('
                    | ')'
                    | '@'
                    | '^'
                    | '#'
                    | '\\'
                    | '/'
                    | ':'
            )
        })
        .map(|(idx, ch)| idx + ch.len_utf8())
        .last()?;
    let token = &s[..len];
    if is_family_arrow_token(token) {
        Some(len)
    } else {
        None
    }
}

pub(crate) fn directional_family_arrow_token_len(s: &str) -> Option<usize> {
    let dirs = ["left", "right", "up", "down", "l", "r", "u", "d"];
    for prefix_len in 1..=2 {
        let prefix = s.get(..prefix_len)?;
        if !prefix.chars().all(|ch| matches!(ch, '-' | '.')) {
            continue;
        }
        let after_prefix = &s[prefix_len..];
        if let Some(after_directive) = after_prefix.strip_prefix('[') {
            if let Some(close) = after_directive.find(']') {
                let after_with_optional_dir = &after_directive[close + 1..];
                let after = dirs
                    .iter()
                    .find_map(|dir| after_with_optional_dir.strip_prefix(dir))
                    .unwrap_or(after_with_optional_dir);
                let dir_len = after_with_optional_dir.len().saturating_sub(after.len());
                let suffix_len = after
                    .char_indices()
                    .take_while(|(_, ch)| {
                        matches!(
                            ch,
                            '-' | '.'
                                | '<'
                                | '>'
                                | '|'
                                | '{'
                                | '}'
                                | 'o'
                                | '0'
                                | '('
                                | ')'
                                | '@'
                                | '^'
                                | '#'
                                | '\\'
                                | '/'
                                | ':'
                        )
                    })
                    .map(|(idx, ch)| idx + ch.len_utf8())
                    .last()
                    .unwrap_or(0);
                if suffix_len > 0 {
                    return Some(prefix_len + close + 2 + dir_len + suffix_len);
                }
            }
        }
        for dir in dirs {
            if let Some(after_dir) = after_prefix.strip_prefix(dir) {
                let suffix_len = after_dir
                    .char_indices()
                    .take_while(|(_, ch)| {
                        matches!(
                            ch,
                            '-' | '.'
                                | '<'
                                | '>'
                                | '|'
                                | '{'
                                | '}'
                                | 'o'
                                | '0'
                                | '('
                                | ')'
                                | '@'
                                | '^'
                                | '#'
                                | '\\'
                                | '/'
                                | ':'
                        )
                    })
                    .map(|(idx, ch)| idx + ch.len_utf8())
                    .last()
                    .unwrap_or(0);
                if suffix_len > 0 {
                    return Some(prefix_len + dir.len() + suffix_len);
                }
            }
        }
    }
    None
}

pub(crate) fn is_family_arrow_token(token: &str) -> bool {
    token.contains('-') || token.contains('.')
}

pub(crate) fn normalize_family_arrow_token(token: &str) -> String {
    let mut out = String::new();
    let mut chars = token.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '[' {
            // Strip bracketed direction/color annotations like [left], [#red]
            for next in chars.by_ref() {
                if next == ']' {
                    break;
                }
            }
            continue;
        }
        // 'o' is a valid arrow-head marker char (aggregation / hollow diamond).
        // All other alphabetic runs are direction/color keywords — strip them.
        if ch.is_ascii_alphabetic() && ch != 'o' {
            continue;
        }
        out.push(ch);
    }
    out
}

pub(crate) fn clean_bracketed_ident(s: &str) -> String {
    let trimmed = s.trim();
    // Preserve special state markers like [*] verbatim.
    if trimmed == "[*]" || trimmed == "[H]" || trimmed == "[H*]" {
        return trimmed.to_string();
    }
    // Allow `[Name]` shorthand: strip the surrounding brackets if balanced and no interior bracket.
    if let Some(inner) = trimmed.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
        if !inner.contains('[') && !inner.contains(']') && !inner.is_empty() {
            return clean_ident(inner);
        }
    }
    // Strip `()` interface-style prefix `() Name`.
    if let Some(rest) = trimmed.strip_prefix("()") {
        return clean_ident(rest.trim());
    }
    clean_ident(trimmed)
}
