use super::*;
pub(crate) fn parse_family_decl_members(
    lines: &[(&str, Span)],
    start: usize,
    keyword: &str,
    name: &str,
) -> Result<Vec<ClassMember>, Diagnostic> {
    let end_idx = find_family_decl_end(lines, start);
    if end_idx == start {
        return Err(Diagnostic::error(format!(
            "[E_FAMILY_DECL_BLOCK_UNCLOSED] unclosed {keyword} declaration block for `{name}`: missing `}}`",
        ))
        .with_span(lines[start].1));
    }
    let mut members = Vec::new();
    for (raw, _) in lines.iter().take(end_idx).skip(start + 1) {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            members.push(parse_class_member(trimmed));
        }
    }
    Ok(members)
}

/// Parse a single member line, extracting any `{field}`, `{method}`, `{abstract}`,
/// `{static}`, or `{class}` modifier token (trailing or leading), as well as
/// `<<abstract>>` and `<<static>>` stereotype tokens.
pub(crate) fn parse_class_member(raw: &str) -> ClassMember {
    // Check for leading brace modifier: `{field} +id: UUID`
    if let Some(rest) = try_strip_leading_brace_modifier(raw) {
        let modifier = parse_brace_modifier_word(leading_brace_word(raw));
        return ClassMember {
            text: rest.trim().to_string(),
            modifier,
        };
    }

    // Check for trailing brace modifier: `+id: UUID {field}`
    if let Some((text_part, mod_word)) = try_strip_trailing_brace_modifier(raw) {
        let modifier = parse_brace_modifier_word(mod_word);
        return ClassMember {
            text: text_part.trim().to_string(),
            modifier,
        };
    }

    // Check for leading `<<abstract>>` or `<<static>>` stereotype
    if let Some((modifier, rest)) = try_strip_leading_stereotype_modifier(raw) {
        return ClassMember {
            text: rest.trim().to_string(),
            modifier: Some(modifier),
        };
    }

    // Check for trailing `<<abstract>>` or `<<static>>` stereotype
    if let Some((text_part, modifier)) = try_strip_trailing_stereotype_modifier(raw) {
        return ClassMember {
            text: text_part.trim().to_string(),
            modifier: Some(modifier),
        };
    }

    ClassMember {
        text: raw.to_string(),
        modifier: None,
    }
}

pub(crate) fn leading_brace_word(s: &str) -> &str {
    // returns the content between the first { and }
    if let Some(rest) = s.strip_prefix('{') {
        if let Some(end) = rest.find('}') {
            return rest[..end].trim();
        }
    }
    ""
}

pub(crate) fn try_strip_leading_brace_modifier(s: &str) -> Option<&str> {
    let s = s.trim_start();
    if !s.starts_with('{') {
        return None;
    }
    let rest = &s[1..];
    let end = rest.find('}')?;
    let word = rest[..end].trim();
    if is_member_modifier_word(word) {
        Some(rest[end + 1..].trim())
    } else {
        None
    }
}

pub(crate) fn try_strip_trailing_brace_modifier(s: &str) -> Option<(&str, &str)> {
    let s = s.trim_end();
    if !s.ends_with('}') {
        return None;
    }
    let start = s.rfind('{')?;
    let word = s[start + 1..s.len() - 1].trim();
    if is_member_modifier_word(word) {
        Some((&s[..start], word))
    } else {
        None
    }
}

pub(crate) fn try_strip_leading_stereotype_modifier(s: &str) -> Option<(MemberModifier, &str)> {
    let s = s.trim_start();
    if !s.starts_with("<<") {
        return None;
    }
    let rest = &s[2..];
    let end = rest.find(">>")?;
    let word = rest[..end].trim();
    let modifier = match word.to_ascii_lowercase().as_str() {
        "abstract" => MemberModifier::Abstract,
        "static" => MemberModifier::Static,
        _ => return None,
    };
    Some((modifier, rest[end + 2..].trim()))
}

pub(crate) fn try_strip_trailing_stereotype_modifier(s: &str) -> Option<(&str, MemberModifier)> {
    let s = s.trim_end();
    if !s.ends_with(">>") {
        return None;
    }
    let start = s.rfind("<<")?;
    let word = s[start + 2..s.len() - 2].trim();
    let modifier = match word.to_ascii_lowercase().as_str() {
        "abstract" => MemberModifier::Abstract,
        "static" => MemberModifier::Static,
        _ => return None,
    };
    Some((&s[..start], modifier))
}

pub(crate) fn is_member_modifier_word(word: &str) -> bool {
    matches!(
        word.to_ascii_lowercase().as_str(),
        "field" | "method" | "abstract" | "static" | "class"
    )
}

pub(crate) fn parse_brace_modifier_word(word: &str) -> Option<MemberModifier> {
    match word.to_ascii_lowercase().as_str() {
        "field" => Some(MemberModifier::Field),
        "method" => Some(MemberModifier::Method),
        "abstract" => Some(MemberModifier::Abstract),
        "static" | "class" => Some(MemberModifier::Static),
        _ => None,
    }
}

pub(crate) fn find_family_decl_end(lines: &[(&str, Span)], start: usize) -> usize {
    for (idx, (raw, _)) in lines.iter().enumerate().skip(start + 1) {
        if raw.trim() == "}" {
            return idx;
        }
    }
    start
}
