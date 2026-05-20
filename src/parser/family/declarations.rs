use super::*;

pub(super) fn parse_family_declaration(
    lines: &[(&str, Span)],
    start: usize,
    line: &str,
) -> Result<Option<(StatementKind, usize)>, Diagnostic> {
    for (keyword, marker) in [
        ("abstract class", Some("<<abstract class>>")),
        ("interface", Some("<<interface>>")),
        ("enum", Some("<<enum>>")),
        ("annotation", Some("<<annotation>>")),
        ("protocol", Some("<<protocol>>")),
        ("struct", Some("<<struct>>")),
        ("abstract", Some("<<abstract>>")),
        ("class", None),
    ] {
        let Some(decl) = parse_named_family_decl(line, keyword) else {
            continue;
        };
        let FamilyDeclParts {
            name,
            alias,
            has_block,
            stereotypes,
            fill_color,
        } = decl;
        let mut members = if has_block {
            let mut members = parse_family_decl_members(lines, start, keyword, &name)?;
            if let Some(marker) = marker {
                members.insert(
                    0,
                    ClassMember {
                        text: marker.to_string(),
                        modifier: None,
                    },
                );
            }
            for stereotype in stereotypes.iter().rev() {
                members.insert(
                    0,
                    ClassMember {
                        text: format!("<<{stereotype}>>"),
                        modifier: None,
                    },
                );
            }
            members
        } else {
            declaration_marker_members(marker, stereotypes)
        };
        append_inline_fill_member(&mut members, fill_color);
        return Ok(Some((
            StatementKind::ClassDecl(ClassDecl {
                name,
                alias,
                members,
            }),
            if has_block {
                find_family_decl_end(lines, start)
            } else {
                start
            },
        )));
    }

    for (keyword, marker) in [("map", Some("<<map>>")), ("object", None)] {
        let Some(decl) = parse_named_family_decl(line, keyword) else {
            continue;
        };
        let FamilyDeclParts {
            name,
            alias,
            has_block,
            stereotypes,
            fill_color,
        } = decl;
        let mut members = if has_block {
            let mut members = parse_family_decl_members(lines, start, keyword, &name)?;
            if let Some(marker) = marker {
                members.insert(
                    0,
                    ClassMember {
                        text: marker.to_string(),
                        modifier: None,
                    },
                );
            }
            for stereotype in stereotypes.iter().rev() {
                members.insert(
                    0,
                    ClassMember {
                        text: format!("<<{stereotype}>>"),
                        modifier: None,
                    },
                );
            }
            members
        } else {
            declaration_marker_members(marker, stereotypes)
        };
        append_inline_fill_member(&mut members, fill_color);
        return Ok(Some((
            StatementKind::ObjectDecl(ObjectDecl {
                name,
                alias,
                members,
            }),
            if has_block {
                find_family_decl_end(lines, start)
            } else {
                start
            },
        )));
    }

    if let Some(decl) = parse_parenthesized_usecase_decl(line) {
        let FamilyDeclParts {
            name,
            alias,
            has_block,
            fill_color,
            ..
        } = decl;
        let mut members = Vec::new();
        append_inline_fill_member(&mut members, fill_color);
        return Ok(Some((
            StatementKind::UseCaseDecl(UseCaseDecl {
                name,
                alias,
                members,
            }),
            if has_block {
                find_family_decl_end(lines, start)
            } else {
                start
            },
        )));
    }

    for (keyword, marker) in [("actor", Some("<<actor>>")), ("usecase", None)] {
        let Some(decl) = parse_named_family_decl(line, keyword) else {
            continue;
        };
        let FamilyDeclParts {
            name,
            alias,
            has_block,
            stereotypes,
            fill_color,
        } = decl;
        let mut members = if has_block {
            let mut members = parse_family_decl_members(lines, start, keyword, &name)?;
            if let Some(marker) = marker {
                members.insert(
                    0,
                    ClassMember {
                        text: marker.to_string(),
                        modifier: None,
                    },
                );
            }
            for stereotype in stereotypes.iter().rev() {
                members.insert(
                    0,
                    ClassMember {
                        text: format!("<<{stereotype}>>"),
                        modifier: None,
                    },
                );
            }
            members
        } else {
            declaration_marker_members(marker, stereotypes)
        };
        append_inline_fill_member(&mut members, fill_color);
        return Ok(Some((
            StatementKind::UseCaseDecl(UseCaseDecl {
                name,
                alias,
                members,
            }),
            if has_block {
                find_family_decl_end(lines, start)
            } else {
                start
            },
        )));
    }
    Ok(None)
}

pub(super) fn later_lines_contain_class_family_declaration(
    lines: &[(&str, Span)],
    start: usize,
) -> bool {
    lines.iter().skip(start + 1).any(|(raw, _)| {
        let line = raw.trim();
        line.starts_with("abstract class ")
            || line.starts_with("abstract ")
            || line.starts_with("annotation ")
            || line.starts_with("class ")
            || line.starts_with("enum ")
            || line.starts_with("protocol ")
            || line.starts_with("struct ")
    })
}

pub(super) fn later_lines_contain_usecase_family_declaration(
    lines: &[(&str, Span)],
    start: usize,
) -> bool {
    lines.iter().skip(start + 1).any(|(raw, _)| {
        let line = raw.trim();
        line.starts_with("usecase ") || line.starts_with("usecase(")
    })
}

/// Returns `true` if any subsequent line is an unambiguous sequence-diagram keyword.
/// Used to suppress the component-family heuristic when `actor` appears in a context
/// that is clearly a sequence diagram (fixes #776).
pub(super) fn later_lines_contain_sequence_family_keywords(
    lines: &[(&str, Span)],
    start: usize,
) -> bool {
    lines.iter().skip(start + 1).any(|(raw, _)| {
        let line = raw.trim();
        let lower = line.to_ascii_lowercase();
        // Sequence arrows: ->, ->>, -->, -->>, <-, <<-, <--, etc.
        let has_sequence_arrow = line.contains("->") || line.contains("<-");
        // Unambiguous sequence keywords (not shared with component/class)
        let is_sequence_keyword = lower.starts_with("activate ")
            || lower == "activate"
            || lower.starts_with("deactivate ")
            || lower == "deactivate"
            || lower.starts_with("destroy ")
            || lower == "destroy"
            || lower.starts_with("autonumber")
            || lower.starts_with("participant ")
            || lower.starts_with("boundary ")
            || lower.starts_with("control ")
            || lower.starts_with("entity ")
            || lower.starts_with("collections ")
            || lower.starts_with("queue ")
            || lower.starts_with("alt ")
            || lower == "alt"
            || lower.starts_with("opt ")
            || lower == "opt"
            || lower.starts_with("loop ")
            || lower == "loop"
            || lower.starts_with("par ")
            || lower == "par"
            || lower.starts_with("also ")
            || lower == "also"
            || lower.starts_with("critical ")
            || lower == "critical"
            || lower.starts_with("ref over ")
            || lower.starts_with("ref over\t");
        has_sequence_arrow || is_sequence_keyword
    })
}

#[derive(Debug, Clone)]
pub(super) struct FamilyDeclParts {
    pub(super) name: String,
    pub(super) alias: Option<String>,
    pub(super) has_block: bool,
    pub(super) stereotypes: Vec<String>,
    pub(super) fill_color: Option<String>,
}

pub(super) fn parse_named_family_decl(line: &str, keyword: &str) -> Option<FamilyDeclParts> {
    if !line.starts_with(keyword) {
        return None;
    }
    if line.len() > keyword.len()
        && !line[keyword.len()..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
    {
        return None;
    }
    let rest = line[keyword.len()..].trim();
    if rest.is_empty() {
        return None;
    }

    let has_block = rest.ends_with('{');
    let trimmed = if has_block {
        rest.trim_end_matches('{').trim()
    } else {
        rest
    };
    let (trimmed, fill_color) = split_declaration_inline_fill(trimmed);
    let trimmed = trimmed.trim();

    let (name_raw, alias_raw) = if let Some((lhs, rhs)) = trimmed.split_once(" as ") {
        (lhs.trim(), Some(rhs.trim()))
    } else {
        (trimmed, None)
    };

    let (name_without_stereotypes, stereotypes) = strip_declaration_stereotypes(name_raw);
    let name = clean_ident(&name_without_stereotypes);
    if name.is_empty() {
        return None;
    }
    let alias = alias_raw.map(clean_ident).filter(|v| !v.is_empty());
    Some(FamilyDeclParts {
        name,
        alias,
        has_block,
        stereotypes,
        fill_color,
    })
}

pub(super) fn append_inline_fill_member(
    members: &mut Vec<ClassMember>,
    fill_color: Option<String>,
) {
    if let Some(color) = fill_color {
        members.push(ClassMember {
            text: format!("\x1fstyle:fill:{color}"),
            modifier: None,
        });
    }
}

pub(super) fn split_declaration_inline_fill(input: &str) -> (String, Option<String>) {
    let trimmed = input.trim();
    let mut in_quote = false;
    let mut last_hash: Option<usize> = None;
    for (idx, ch) in trimmed.char_indices() {
        if ch == '"' {
            in_quote = !in_quote;
            continue;
        }
        if !in_quote && ch == '#' {
            last_hash = Some(idx);
        }
    }
    let Some(hash_idx) = last_hash else {
        return (trimmed.to_string(), None);
    };
    if hash_idx > 0
        && !trimmed[..hash_idx]
            .chars()
            .last()
            .is_some_and(char::is_whitespace)
    {
        return (trimmed.to_string(), None);
    }
    let after = &trimmed[hash_idx..];
    let token_len = after
        .char_indices()
        .take_while(|(_, ch)| ch.is_ascii_alphanumeric() || matches!(ch, '#' | '_' | '-' | ':'))
        .map(|(idx, ch)| idx + ch.len_utf8())
        .last()
        .unwrap_or(0);
    if token_len == 0 {
        return (trimmed.to_string(), None);
    }
    let token = &after[..token_len];
    let Some(color) = parse_relation_color_token(token) else {
        return (trimmed.to_string(), None);
    };
    let before = trimmed[..hash_idx].trim_end();
    let suffix = after[token_len..].trim_start();
    let mut cleaned = before.to_string();
    if !suffix.is_empty() {
        if !cleaned.is_empty() {
            cleaned.push(' ');
        }
        cleaned.push_str(suffix);
    }
    (cleaned, Some(color))
}

pub(super) fn declaration_marker_members(
    marker: Option<&str>,
    stereotypes: Vec<String>,
) -> Vec<ClassMember> {
    let mut members = Vec::new();
    if let Some(marker) = marker {
        members.push(ClassMember {
            text: marker.to_string(),
            modifier: None,
        });
    }
    for stereotype in stereotypes {
        members.push(ClassMember {
            text: format!("<<{stereotype}>>"),
            modifier: None,
        });
    }
    members
}

pub(super) fn strip_declaration_stereotypes(input: &str) -> (String, Vec<String>) {
    let mut remaining = input.trim().to_string();
    let mut stereotypes = Vec::new();
    while let Some(start) = remaining.find("<<") {
        let Some(end_rel) = remaining[start + 2..].find(">>") else {
            break;
        };
        let end = start + 2 + end_rel;
        let value = remaining[start + 2..end].trim();
        if !value.is_empty() {
            stereotypes.push(value.to_string());
        }
        remaining.replace_range(start..end + 2, "");
    }
    (remaining.trim().to_string(), stereotypes)
}

pub(super) fn parse_parenthesized_usecase_decl(line: &str) -> Option<FamilyDeclParts> {
    let trimmed = line.trim();
    let trimmed = trimmed.strip_prefix("usecase ").unwrap_or(trimmed).trim();
    if !trimmed.starts_with('(') {
        return None;
    }
    let close = trimmed.find(')')?;
    let name_raw = trimmed[1..close].trim();
    if name_raw.is_empty() {
        return None;
    }
    let rest = trimmed[close + 1..].trim();
    let has_block = rest.ends_with('{');
    let rest = if has_block {
        rest.trim_end_matches('{').trim()
    } else {
        rest
    };
    let (rest, fill_color) = split_declaration_inline_fill(rest);
    let rest = rest.trim();
    let alias = rest
        .strip_prefix("as ")
        .map(str::trim)
        .map(clean_ident)
        .filter(|v| !v.is_empty());
    Some(FamilyDeclParts {
        name: clean_ident(name_raw),
        alias,
        has_block,
        stereotypes: Vec::new(),
        fill_color,
    })
}

pub(super) fn parse_family_decl_members(
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
pub(super) fn parse_class_member(raw: &str) -> ClassMember {
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

fn leading_brace_word(s: &str) -> &str {
    // returns the content between the first { and }
    if let Some(rest) = s.strip_prefix('{') {
        if let Some(end) = rest.find('}') {
            return rest[..end].trim();
        }
    }
    ""
}

fn try_strip_leading_brace_modifier(s: &str) -> Option<&str> {
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

fn try_strip_trailing_brace_modifier(s: &str) -> Option<(&str, &str)> {
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

fn try_strip_leading_stereotype_modifier(s: &str) -> Option<(MemberModifier, &str)> {
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

fn try_strip_trailing_stereotype_modifier(s: &str) -> Option<(&str, MemberModifier)> {
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

fn is_member_modifier_word(word: &str) -> bool {
    matches!(
        word.to_ascii_lowercase().as_str(),
        "field" | "method" | "abstract" | "static" | "class"
    )
}

fn parse_brace_modifier_word(word: &str) -> Option<MemberModifier> {
    match word.to_ascii_lowercase().as_str() {
        "field" => Some(MemberModifier::Field),
        "method" => Some(MemberModifier::Method),
        "abstract" => Some(MemberModifier::Abstract),
        "static" | "class" => Some(MemberModifier::Static),
        _ => None,
    }
}

pub(super) fn find_family_decl_end(lines: &[(&str, Span)], start: usize) -> usize {
    for (idx, (raw, _)) in lines.iter().enumerate().skip(start + 1) {
        if raw.trim() == "}" {
            return idx;
        }
    }
    start
}
