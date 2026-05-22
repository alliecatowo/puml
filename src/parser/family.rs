fn parse_family_declaration(
    lines: &[(&str, Span)],
    start: usize,
    line: &str,
) -> Result<Option<(StatementKind, usize)>, Diagnostic> {
    for (keyword, marker) in [
        ("abstract class", Some("<<abstract class>>")),
        ("exception", Some("<<exception>>")),
        ("metaclass", Some("<<metaclass>>")),
        ("stereotype", Some("<<stereotype>>")),
        ("interface", Some("<<interface>>")),
        ("enum", Some("<<enum>>")),
        ("annotation", Some("<<annotation>>")),
        ("protocol", Some("<<protocol>>")),
        ("struct", Some("<<struct>>")),
        ("circle", Some("<<circle>>")),
        ("diamond", Some("<<diamond>>")),
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
            heritage,
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
        append_heritage_members(&mut members, heritage);
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

    if let Some(decl) = parse_named_family_decl(line, "entity") {
        let FamilyDeclParts {
            name,
            alias,
            has_block,
            stereotypes,
            fill_color,
            heritage,
        } = decl;
        if has_block || later_lines_contain_ie_family_context(lines, start) {
            let mut members = if has_block {
                let mut members = parse_family_decl_members(lines, start, "entity", &name)?;
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
                declaration_marker_members(None, stereotypes)
            };
            append_inline_fill_member(&mut members, fill_color);
            append_heritage_members(&mut members, heritage);
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
            ..
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

    if let Some(kind) = parse_association_class_relation(line) {
        return Ok(Some((kind, start)));
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
            ..
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

fn later_lines_contain_class_family_declaration(lines: &[(&str, Span)], start: usize) -> bool {
    lines.iter().skip(start + 1).any(|(raw, _)| {
        let line = raw.trim();
        line.starts_with("abstract class ")
            || line.starts_with("abstract ")
            || line.starts_with("annotation ")
            || line.starts_with("circle ")
            || line.starts_with("class ")
            || line.starts_with("diamond ")
            || line.starts_with("enum ")
            || line.starts_with("exception ")
            || line.starts_with("metaclass ")
            || line.starts_with("protocol ")
            || line.starts_with("stereotype ")
            || line.starts_with("struct ")
            || (line.starts_with("entity ") && line.ends_with('{'))
    })
}

fn later_lines_contain_ie_family_context(lines: &[(&str, Span)], start: usize) -> bool {
    lines.iter().skip(start + 1).any(|(raw, _)| {
        let line = raw.trim();
        line.starts_with("entity ") && line.ends_with('{') || line_contains_ie_relation_token(line)
    })
}

fn line_contains_ie_relation_token(line: &str) -> bool {
    [
        "||--", "||..", "|o--", "|o..", "}o--", "}o..", "}|--", "}|..", "--||", "..||", "--o|",
        "..o|", "--o{", "..o{", "--|{", "..|{",
    ]
    .iter()
    .any(|token| line.contains(token))
}

fn later_lines_contain_usecase_family_declaration(lines: &[(&str, Span)], start: usize) -> bool {
    lines.iter().skip(start + 1).any(|(raw, _)| {
        let line = raw.trim();
        line.starts_with("usecase ")
            || line.starts_with("usecase(")
            || line.starts_with('(')
            || line.starts_with("actor ")
    })
}

/// Returns `true` if any subsequent line is an unambiguous sequence-diagram keyword.
/// Used to suppress the component-family heuristic when `actor` appears in a context
/// that is clearly a sequence diagram (fixes #776).
fn later_lines_contain_sequence_family_keywords(lines: &[(&str, Span)], start: usize) -> bool {
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
            || lower.starts_with("ref over\t")
            || (lower.starts_with("==") && lower.ends_with("==") && lower.len() >= 4);
        has_sequence_arrow || is_sequence_keyword
    })
}

fn later_lines_contain_activity_context(lines: &[(&str, Span)], start: usize) -> bool {
    lines.iter().skip(start + 1).any(|(raw, _)| {
        let line = raw.trim();
        looks_like_old_activity_flow(line) || parse_activity_step(line).is_some()
    })
}

#[derive(Debug, Clone)]
struct FamilyDeclParts {
    name: String,
    alias: Option<String>,
    has_block: bool,
    stereotypes: Vec<String>,
    fill_color: Option<String>,
    heritage: Vec<FamilyHeritage>,
}

#[derive(Debug, Clone)]
struct FamilyHeritage {
    arrow: String,
    target: String,
}

fn parse_named_family_decl(line: &str, keyword: &str) -> Option<FamilyDeclParts> {
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

    let (name_raw, heritage) = split_declaration_heritage(name_raw);
    let (name_without_stereotypes, stereotypes) = strip_declaration_stereotypes(&name_raw);
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
        heritage,
    })
}

fn append_heritage_members(members: &mut Vec<ClassMember>, heritage: Vec<FamilyHeritage>) {
    for item in heritage {
        members.push(ClassMember {
            text: format!("\x1fheritage:{}:{}", item.arrow, item.target),
            modifier: None,
        });
    }
}

fn split_declaration_heritage(input: &str) -> (String, Vec<FamilyHeritage>) {
    let trimmed = input.trim();
    let Some((base, clause)) = split_at_first_top_level_heritage_keyword(trimmed) else {
        return (trimmed.to_string(), Vec::new());
    };

    let mut heritage = Vec::new();
    let mut rest = clause.trim();
    loop {
        let lower = rest.to_ascii_lowercase();
        if lower.starts_with("extends ") {
            rest = rest[8..].trim_start();
            let (targets, next) = take_heritage_targets(rest);
            for target in split_heritage_targets(targets) {
                heritage.push(FamilyHeritage {
                    arrow: "<|--".to_string(),
                    target,
                });
            }
            rest = next.trim_start();
        } else if lower.starts_with("implements ") {
            rest = rest[11..].trim_start();
            let (targets, next) = take_heritage_targets(rest);
            for target in split_heritage_targets(targets) {
                heritage.push(FamilyHeritage {
                    arrow: "<|..".to_string(),
                    target,
                });
            }
            rest = next.trim_start();
        } else {
            break;
        }
        if rest.is_empty() {
            break;
        }
    }

    (base.trim().to_string(), heritage)
}

fn split_at_first_top_level_heritage_keyword(input: &str) -> Option<(&str, &str)> {
    let extends = find_top_level_keyword(input, " extends ");
    let implements = find_top_level_keyword(input, " implements ");
    let idx = match (extends, implements) {
        (Some(a), Some(b)) => a.min(b),
        (Some(a), None) | (None, Some(a)) => a,
        (None, None) => return None,
    };
    Some((&input[..idx], input[idx + 1..].trim_start()))
}

fn take_heritage_targets(input: &str) -> (&str, &str) {
    let extends = find_top_level_keyword(input, " extends ");
    let implements = find_top_level_keyword(input, " implements ");
    let idx = match (extends, implements) {
        (Some(a), Some(b)) => a.min(b),
        (Some(a), None) | (None, Some(a)) => a,
        (None, None) => return (input, ""),
    };
    (&input[..idx], input[idx + 1..].trim_start())
}

fn split_heritage_targets(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut angle_depth = 0i32;
    let mut in_quote = false;
    for (idx, ch) in input.char_indices() {
        match ch {
            '"' => in_quote = !in_quote,
            '<' if !in_quote => angle_depth += 1,
            '>' if !in_quote => angle_depth = angle_depth.saturating_sub(1),
            ',' if !in_quote && angle_depth == 0 => {
                let target = clean_ident(&input[start..idx]);
                if !target.is_empty() {
                    out.push(target);
                }
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }
    let target = clean_ident(&input[start..]);
    if !target.is_empty() {
        out.push(target);
    }
    out
}

fn find_top_level_keyword(input: &str, keyword: &str) -> Option<usize> {
    let lower = input.to_ascii_lowercase();
    let needle = keyword.to_ascii_lowercase();
    let mut search_from = 0usize;
    while let Some(rel) = lower[search_from..].find(&needle) {
        let idx = search_from + rel;
        if is_top_level_span(input, idx) {
            return Some(idx);
        }
        search_from = idx + needle.len();
    }
    None
}

fn is_top_level_span(input: &str, byte_idx: usize) -> bool {
    let mut angle_depth = 0i32;
    let mut in_quote = false;
    for (idx, ch) in input.char_indices() {
        if idx >= byte_idx {
            break;
        }
        match ch {
            '"' => in_quote = !in_quote,
            '<' if !in_quote => angle_depth += 1,
            '>' if !in_quote => angle_depth = angle_depth.saturating_sub(1),
            _ => {}
        }
    }
    !in_quote && angle_depth == 0
}

fn append_inline_fill_member(members: &mut Vec<ClassMember>, fill_color: Option<String>) {
    if let Some(color) = fill_color {
        members.push(ClassMember {
            text: format!("\x1fstyle:fill:{color}"),
            modifier: None,
        });
    }
}

fn split_declaration_inline_fill(input: &str) -> (String, Option<String>) {
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
        .take_while(|(_, ch)| {
            ch.is_ascii_alphanumeric() || matches!(ch, '#' | '_' | '-' | ':' | ';' | '.')
        })
        .map(|(idx, ch)| idx + ch.len_utf8())
        .last()
        .unwrap_or(0);
    if token_len == 0 {
        return (trimmed.to_string(), None);
    }
    let token = &after[..token_len];
    let fill_token = token
        .split(';')
        .find(|part| !part.trim().is_empty())
        .unwrap_or(token)
        .trim();
    let Some(color) = parse_relation_color_token(fill_token) else {
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

fn declaration_marker_members(marker: Option<&str>, stereotypes: Vec<String>) -> Vec<ClassMember> {
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

fn strip_declaration_stereotypes(input: &str) -> (String, Vec<String>) {
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

fn parse_parenthesized_usecase_decl(line: &str) -> Option<FamilyDeclParts> {
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
        heritage: Vec::new(),
    })
}

fn parse_family_decl_members(
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
fn parse_class_member(raw: &str) -> ClassMember {
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

fn find_family_decl_end(lines: &[(&str, Span)], start: usize) -> usize {
    for (idx, (raw, _)) in lines.iter().enumerate().skip(start + 1) {
        if raw.trim() == "}" {
            return idx;
        }
    }
    start
}

fn parse_family_relation(line: &str, family: Option<DiagramKind>) -> Option<StatementKind> {
    match family {
        Some(DiagramKind::Class)
        | Some(DiagramKind::Object)
        | Some(DiagramKind::UseCase)
        | Some(DiagramKind::Salt)
        | Some(DiagramKind::Component)
        | Some(DiagramKind::Deployment) => {}
        _ => return None,
    }

    let (core, raw_label) = split_family_relation_label(line);
    let (lhs, arrow, relation_style, rhs) = split_family_arrow_styled(core)?;
    if !arrow.contains('-') && !arrow.contains('.') {
        return None;
    }
    let (rhs, trailing_stereotype) = split_relation_trailing_stereotype(rhs);
    let (label, label_stereotype) = split_relation_label_stereotype(raw_label);
    // Strip surrounding double-quotes from labels produced by preprocessor macro
    // expansion (e.g. C4 Rel() emits `from --> to : "Label"` with quotes intact).
    let label = label.map(|l| {
        let t = l.trim().to_string();
        if t.starts_with('"') && t.ends_with('"') && t.len() >= 2 {
            t[1..t.len() - 1].to_string()
        } else {
            t
        }
    });
    let (lhs_core, left_cardinality, left_role) = parse_relation_side_annotations(lhs, true);
    let (rhs_core, right_cardinality, right_role) = parse_relation_side_annotations(rhs, false);
    let (lhs_core, left_lollipop) = strip_lollipop_endpoint(&lhs_core);
    let (rhs_core, right_lollipop) = strip_lollipop_endpoint(&rhs_core);
    if normalize_virtual_endpoint(&lhs_core).is_some()
        || normalize_virtual_endpoint(&rhs_core).is_some()
        || looks_like_virtual_endpoint_syntax(&lhs_core)
        || looks_like_virtual_endpoint_syntax(&rhs_core)
    {
        return None;
    }
    let from = clean_bracketed_ident(&lhs_core);
    let to = clean_bracketed_ident(&rhs_core);
    if from.is_empty() || to.is_empty() {
        return None;
    }
    Some(StatementKind::FamilyRelation(FamilyRelation {
        from,
        to,
        arrow,
        label,
        stereotype: label_stereotype.or(trailing_stereotype),
        left_cardinality,
        right_cardinality,
        left_role,
        right_role,
        line_color: relation_style.line_color,
        dashed: relation_style.dashed,
        hidden: relation_style.hidden,
        thickness: relation_style.thickness,
        direction: relation_style.direction,
        left_lollipop,
        right_lollipop,
    }))
}

fn parse_association_class_relation(line: &str) -> Option<StatementKind> {
    let trimmed = line.trim();
    let after_open = trimmed.strip_prefix('(')?;
    let close = after_open.find(')')?;
    let pair = &after_open[..close];
    let (left_raw, right_raw) = pair.split_once(',')?;
    let left = clean_ident(left_raw);
    let right = clean_ident(right_raw);
    if left.is_empty() || right.is_empty() {
        return None;
    }
    let rest = after_open[close + 1..].trim();
    let arrow_len = family_arrow_token_len(rest)?;
    let arrow = normalize_family_arrow_token(&rest[..arrow_len]);
    let association_raw = rest[arrow_len..].trim();
    let association = clean_bracketed_ident(association_raw);
    if association.is_empty() {
        return None;
    }
    Some(StatementKind::AssociationClass {
        left,
        right,
        association,
        arrow,
    })
}

fn strip_lollipop_endpoint(side: &str) -> (String, bool) {
    let trimmed = side.trim();
    if let Some(rest) = trimmed.strip_prefix("()") {
        return (rest.trim_start().to_string(), true);
    }
    if let Some(rest) = trimmed.strip_suffix("()") {
        return (rest.trim_end().to_string(), true);
    }
    (trimmed.to_string(), false)
}

fn split_relation_label_stereotype(label: Option<String>) -> (Option<String>, Option<String>) {
    let Some(label) = label else {
        return (None, None);
    };
    let trimmed = label.trim();
    if let Some((stereotype, rest)) = parse_leading_stereotype(trimmed) {
        let label = rest.trim();
        return (
            (!label.is_empty()).then(|| label.to_string()),
            Some(stereotype),
        );
    }
    (Some(label), None)
}

fn split_relation_trailing_stereotype(side: &str) -> (&str, Option<String>) {
    let trimmed = side.trim();
    let Some(open) = trimmed.rfind("<<") else {
        return (side, None);
    };
    let before = trimmed[..open].trim_end();
    let tail = trimmed[open..].trim();
    if before.is_empty() {
        return (side, None);
    }
    if let Some((stereotype, rest)) = parse_leading_stereotype(tail) {
        if rest.trim().is_empty() {
            return (before, Some(stereotype));
        }
    }
    (side, None)
}

fn parse_leading_stereotype(s: &str) -> Option<(String, &str)> {
    let rest = s.trim_start().strip_prefix("<<")?;
    let close = rest.find(">>")?;
    let value = rest[..close].trim();
    if value.is_empty() {
        return None;
    }
    Some((value.to_string(), &rest[close + 2..]))
}

fn parse_family_member_row(line: &str, family: Option<DiagramKind>) -> Option<StatementKind> {
    let family = match family {
        Some(DiagramKind::Class | DiagramKind::Object | DiagramKind::UseCase) => family?,
        _ => return None,
    };
    if split_family_arrow(line).is_some() {
        return None;
    }
    let (owner, member) = line.split_once(':')?;
    if owner.contains("--") || owner.contains("..") || owner.contains("->") || owner.contains("<-")
    {
        return None;
    }
    let owner = clean_bracketed_ident(owner);
    let member = member.trim();
    if owner.is_empty() || member.is_empty() {
        return None;
    }
    let members = vec![parse_class_member(member)];
    Some(match family {
        DiagramKind::Object => StatementKind::ObjectDecl(ObjectDecl {
            name: owner,
            alias: None,
            members,
        }),
        DiagramKind::UseCase => StatementKind::UseCaseDecl(UseCaseDecl {
            name: owner,
            alias: None,
            members,
        }),
        _ => StatementKind::ClassDecl(ClassDecl {
            name: owner,
            alias: None,
            members,
        }),
    })
}

fn parse_family_visibility_control(
    line: &str,
    family: Option<DiagramKind>,
) -> Option<StatementKind> {
    if !matches!(family, Some(DiagramKind::Class | DiagramKind::Object | DiagramKind::UseCase)) {
        return None;
    }
    let lower = line.to_ascii_lowercase();
    if lower.starts_with("hide ") {
        let rest = line.strip_prefix("hide ").unwrap_or("").trim();
        if rest.eq_ignore_ascii_case("@unlinked") {
            return Some(StatementKind::HideOption("hide @unlinked".to_string()));
        }
        if !rest.is_empty() {
            return Some(StatementKind::HideOption(format!("hide node {rest}")));
        }
    }
    if lower.starts_with("remove ") {
        let rest = line.strip_prefix("remove ").unwrap_or("").trim();
        if !rest.is_empty() {
            return Some(StatementKind::HideOption(format!("remove node {rest}")));
        }
    }
    if lower.starts_with("restore ") {
        let rest = line.strip_prefix("restore ").unwrap_or("").trim();
        if !rest.is_empty() {
            return Some(StatementKind::HideOption(format!("restore node {rest}")));
        }
    }
    if lower.starts_with("show ") {
        let rest = line.strip_prefix("show ").unwrap_or("").trim();
        if !rest.is_empty() {
            return Some(StatementKind::HideOption(format!("show {rest}")));
        }
    }
    None
}

fn parse_relation_side_annotations(
    side: &str,
    is_left: bool,
) -> (String, Option<String>, Option<String>) {
    let trimmed = side.trim();
    if trimmed.is_empty() {
        return (String::new(), None, None);
    }

    let mut rem = trimmed.to_string();
    let mut cardinality: Option<String> = None;
    let mut role: Option<String> = None;

    if is_left {
        loop {
            let t = rem.trim_end();
            if t.ends_with(']') {
                if let Some(start_bracket) = t.rfind('[') {
                    let value = t[start_bracket + 1..t.len() - 1].trim();
                    let endpoint = t[..start_bracket].trim_end();
                    if !value.is_empty() && !endpoint.is_empty() {
                        if role.is_none() {
                            role = Some(value.to_string());
                        }
                        rem = endpoint.to_string();
                        continue;
                    }
                }
            }
            if let Some(stripped) = t.strip_suffix('"') {
                if let Some(start_quote) = stripped.rfind('"') {
                    let value = stripped[start_quote + 1..].trim();
                    let endpoint = t[..start_quote].trim_end();
                    if !value.is_empty() && !endpoint.is_empty() {
                        if cardinality.is_none() {
                            cardinality = Some(value.to_string());
                        }
                        rem = endpoint.to_string();
                        continue;
                    }
                }
            }
            if let Some(colon) = t.rfind(" :") {
                let value = t[colon + 2..].trim();
                let endpoint = t[..colon].trim_end();
                if !value.is_empty() && !endpoint.is_empty() {
                    if role.is_none() {
                        role = Some(value.to_string());
                    }
                    rem = endpoint.to_string();
                    continue;
                }
            }
            break;
        }
    } else {
        loop {
            let t = rem.trim_start();
            if let Some(rest) = t.strip_prefix('"') {
                if let Some(end_quote_rel) = rest.find('"') {
                    let value = rest[..end_quote_rel].trim();
                    let endpoint = rest[end_quote_rel + 1..].trim_start();
                    if !value.is_empty() && !endpoint.is_empty() {
                        if cardinality.is_none() {
                            cardinality = Some(value.to_string());
                        }
                        rem = endpoint.to_string();
                        continue;
                    }
                }
            }
            if let Some(rest) = t.strip_prefix('[') {
                if let Some(end_bracket_rel) = rest.find(']') {
                    let value = rest[..end_bracket_rel].trim();
                    let endpoint = rest[end_bracket_rel + 1..].trim_start();
                    if !value.is_empty() && !endpoint.is_empty() {
                        if role.is_none() {
                            role = Some(value.to_string());
                        }
                        rem = endpoint.to_string();
                        continue;
                    }
                }
            }
            if let Some(rest) = t.strip_prefix(':') {
                let value_len = rest
                    .char_indices()
                    .take_while(|(_, ch)| !ch.is_whitespace())
                    .map(|(idx, ch)| idx + ch.len_utf8())
                    .last()
                    .unwrap_or(0);
                if value_len > 0 {
                    let value = rest[..value_len].trim();
                    let endpoint = rest[value_len..].trim_start();
                    if !value.is_empty() && !endpoint.is_empty() {
                        if role.is_none() {
                            role = Some(value.to_string());
                        }
                        rem = endpoint.to_string();
                        continue;
                    }
                }
            }
            break;
        }
    }

    (rem.trim().to_string(), cardinality, role)
}

#[derive(Debug, Clone, Default)]
struct ParsedFamilyRelationStyle {
    line_color: Option<String>,
    dashed: bool,
    hidden: bool,
    thickness: Option<u8>,
    direction: Option<String>,
}

fn split_family_arrow(core: &str) -> Option<(&str, String, &str)> {
    split_family_arrow_styled(core).map(|(lhs, arrow, _, rhs)| (lhs, arrow, rhs))
}

fn split_family_arrow_styled(
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

fn parse_family_relation_style(raw_arrow: &str) -> ParsedFamilyRelationStyle {
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
                    } else if let Some(color) = parse_relation_color_token(token) {
                        style.line_color = Some(color);
                    }
                }
            }
        }
        rest = &after_open[close + 1..];
    }
    style
}

fn parse_family_relation_direction(raw_arrow: &str) -> Option<String> {
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

fn parse_relation_color_token(token: &str) -> Option<String> {
    let trimmed = token.trim();
    if trimmed.len() == 7
        && trimmed.starts_with('#')
        && trimmed[1..].chars().all(|ch| ch.is_ascii_hexdigit())
    {
        return Some(trimmed.to_ascii_lowercase());
    }
    crate::theme::css3_color_to_hex(trimmed.trim_start_matches('#')).map(str::to_string)
}

fn family_arrow_token_len(s: &str) -> Option<usize> {
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

fn directional_family_arrow_token_len(s: &str) -> Option<usize> {
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

fn is_family_arrow_token(token: &str) -> bool {
    token.contains('-') || token.contains('.')
}

fn normalize_family_arrow_token(token: &str) -> String {
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

fn clean_bracketed_ident(s: &str) -> String {
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

#[derive(Debug, Clone, Default)]
struct ScopedGroupContent {
    members: Vec<String>,
    relations: Vec<FamilyRelation>,
}

/// Parse `together { ... }`, `package "name" { ... }`, `namespace ns { ... }` blocks.
/// Returns (StatementKind, end_line_index) where end_line_index points to the closing `}`.
fn parse_class_scoping_block(
    lines: &[(&str, Span)],
    start: usize,
    line: &str,
) -> Result<Option<(StatementKind, usize)>, Diagnostic> {
    let lower = line.to_ascii_lowercase();

    // together { ... }
    if lower == "together {" || lower.starts_with("together {") {
        let end_idx = find_family_decl_end(lines, start);
        if end_idx == start {
            return Err(Diagnostic::error(
                "[E_CLASS_TOGETHER_UNCLOSED] unclosed `together` block: missing `}`",
            )
            .with_span(lines[start].1));
        }
        let members: Vec<String> = lines[start + 1..end_idx]
            .iter()
            .map(|(raw, _)| raw.trim())
            .filter(|s| !s.is_empty())
            .map(clean_ident)
            .filter(|s| !s.is_empty())
            .collect();
        return Ok(Some((
            StatementKind::ClassGroup {
                kind: "together".to_string(),
                label: None,
                members,
                relations: Vec::new(),
            },
            end_idx,
        )));
    }

    // package "label" { ... } or package label { ... }
    if lower.starts_with("package ") && line.trim_end().ends_with('{') {
        let rest = line.strip_prefix("package ").unwrap_or("").trim();
        let label_raw = rest.trim_end_matches('{').trim();
        let label = clean_ident(label_raw.trim_matches('"'));
        let end_idx = find_scoping_block_end(lines, start);
        if end_idx == start {
            return Err(Diagnostic::error(
                "[E_CLASS_PACKAGE_UNCLOSED] unclosed `package` block: missing `}`",
            )
            .with_span(lines[start].1));
        }
        if group_body_contains_component_family(lines, start, end_idx)
            && !group_body_contains_class_family(lines, start, end_idx)
            && !group_body_contains_object_family(lines, start, end_idx)
            && !group_body_contains_usecase_family(lines, start, end_idx)
        {
            return Ok(None);
        }
        let content =
            collect_scoped_class_group_content(lines, start, end_idx, std::slice::from_ref(&label));
        return Ok(Some((
            StatementKind::ClassGroup {
                kind: "package".to_string(),
                label: if label.is_empty() { None } else { Some(label) },
                members: content.members,
                relations: content.relations,
            },
            end_idx,
        )));
    }

    // namespace ns { ... }
    if lower.starts_with("namespace ") && line.trim_end().ends_with('{') {
        let rest = line.strip_prefix("namespace ").unwrap_or("").trim();
        let label_raw = rest.trim_end_matches('{').trim();
        let label = clean_ident(label_raw.trim_matches('"'));
        let end_idx = find_scoping_block_end(lines, start);
        if end_idx == start {
            return Err(Diagnostic::error(
                "[E_CLASS_NAMESPACE_UNCLOSED] unclosed `namespace` block: missing `}`",
            )
            .with_span(lines[start].1));
        }
        let content =
            collect_scoped_class_group_content(lines, start, end_idx, std::slice::from_ref(&label));
        return Ok(Some((
            StatementKind::ClassGroup {
                kind: "namespace".to_string(),
                label: if label.is_empty() { None } else { Some(label) },
                members: content.members,
                relations: content.relations,
            },
            end_idx,
        )));
    }

    // rectangle "Label" { ... } — used in usecase diagrams as system boundary frames (fix #479)
    if lower.starts_with("rectangle ") && line.trim_end().ends_with('{') {
        let rest = line.strip_prefix("rectangle ").unwrap_or("").trim();
        let label_raw = rest.trim_end_matches('{').trim();
        let label = clean_ident(label_raw.trim_matches('"'));
        let end_idx = find_scoping_block_end(lines, start);
        if end_idx == start {
            return Err(Diagnostic::error(
                "[E_CLASS_RECTANGLE_UNCLOSED] unclosed `rectangle` block: missing `}`",
            )
            .with_span(lines[start].1));
        }
        let content =
            collect_scoped_class_group_content(lines, start, end_idx, std::slice::from_ref(&label));
        return Ok(Some((
            StatementKind::ClassGroup {
                kind: "rectangle".to_string(),
                label: if label.is_empty() { None } else { Some(label) },
                members: content.members,
                relations: content.relations,
            },
            end_idx,
        )));
    }

    Ok(None)
}

fn collect_scoped_class_group_content(
    lines: &[(&str, Span)],
    start: usize,
    end_idx: usize,
    scope: &[String],
) -> ScopedGroupContent {
    let mut content = ScopedGroupContent::default();
    let mut idx = start + 1;
    while idx < end_idx {
        let line = lines[idx].0.trim();
        let lower = line.to_ascii_lowercase();
        if line.is_empty() || line == "}" {
            idx += 1;
            continue;
        }
        if let Some(StatementKind::FamilyRelation(rel)) =
            parse_family_relation(line, Some(DiagramKind::Class))
        {
            content.relations.push(qualify_scoped_relation(rel, scope));
            idx += 1;
            continue;
        }
        if (lower.starts_with("package ") || lower.starts_with("namespace "))
            && line.trim_end().ends_with('{')
        {
            let keyword = if lower.starts_with("package ") {
                "package"
            } else {
                "namespace"
            };
            let label = clean_ident(
                line[keyword.len()..]
                    .trim()
                    .trim_end_matches('{')
                    .trim()
                    .trim_matches('"'),
            );
            let nested_end = find_scoping_block_end(lines, idx);
            if nested_end > idx {
                let mut nested_scope = scope.to_vec();
                if !label.is_empty() {
                    nested_scope.push(label);
                }
                let nested =
                    collect_scoped_class_group_content(lines, idx, nested_end, &nested_scope);
                content.members.extend(nested.members);
                content.relations.extend(nested.relations);
                idx = nested_end + 1;
                continue;
            }
        }
        if let Some(decl) = parse_parenthesized_usecase_decl(line) {
            let FamilyDeclParts {
                name,
                alias,
                has_block,
                fill_color,
                ..
            } = decl;
            let has_alias = alias.is_some();
            let id = alias.unwrap_or_else(|| name.clone());
            let mut encoded = qualify_scoped_identifier(id, scope);
            if has_alias {
                encoded.push('\t');
                encoded.push_str(&name);
            }
            if let Some(fill_color) = fill_color {
                encoded.push('\t');
                encoded.push_str(&format!("\x1fstyle:fill:{fill_color}"));
            }
            content.members.push(encoded);
            if has_block {
                let nested_end = find_family_decl_end(lines, idx);
                if nested_end > idx {
                    idx = nested_end + 1;
                    continue;
                }
            }
            idx += 1;
            continue;
        }
        let declaration_keywords = [
            "abstract class",
            "annotation",
            "circle",
            "interface",
            "abstract",
            "diamond",
            "enum",
            "exception",
            "metaclass",
            "protocol",
            "stereotype",
            "struct",
            "class",
            "object",
            "map",
            "actor",
            "usecase",
        ];
        let mut handled_declaration = false;
        for keyword in declaration_keywords {
            if let Some(decl) = parse_named_family_decl(line, keyword) {
                let FamilyDeclParts {
                    name,
                    alias,
                    has_block,
                    fill_color,
                    ..
                } = decl;
                let has_alias = alias.is_some();
                let id = alias.unwrap_or_else(|| name.clone());
                let scoped_name = qualify_scoped_identifier(id, scope);
                let mut encoded = scoped_name.clone();
                if has_alias {
                    encoded.push('\t');
                    encoded.push_str(&name);
                }
                // Embed actor marker so the normalizer can promote to Actor kind.
                if keyword == "actor" {
                    encoded.push('\t');
                    encoded.push_str("<<actor>>");
                }
                if let Some(fill_color) = fill_color {
                    encoded.push('\t');
                    encoded.push_str(&format!("\x1fstyle:fill:{fill_color}"));
                }
                let nested_end = if has_block {
                    let nested_end = find_family_decl_end(lines, idx);
                    let members_text = parse_family_decl_members(lines, idx, keyword, &scoped_name)
                        .map(|members| {
                            members
                                .into_iter()
                                .map(|member| member.text)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    for member in members_text {
                        encoded.push('\t');
                        encoded.push_str(&member);
                    }
                    nested_end
                } else {
                    idx
                };
                content.members.push(encoded);
                idx = if nested_end > idx {
                    nested_end + 1
                } else {
                    idx + 1
                };
                handled_declaration = true;
                break;
            }
        }
        if handled_declaration {
            continue;
        }
        for keyword in [
            "abstract class",
            "annotation",
            "circle",
            "interface",
            "abstract",
            "diamond",
            "enum",
            "exception",
            "metaclass",
            "protocol",
            "stereotype",
            "struct",
            "class",
            "object",
            "map",
            "actor",
            "usecase",
        ] {
            if let Some(decl) = parse_named_family_decl(line, keyword).filter(|decl| decl.has_block)
            {
                let FamilyDeclParts {
                    name,
                    alias,
                    fill_color,
                    ..
                } = decl;
                let has_alias = alias.is_some();
                let id = alias.unwrap_or_else(|| name.clone());
                let scoped_name = qualify_scoped_identifier(id, scope);
                let nested_end = find_family_decl_end(lines, idx);
                let members_text = parse_family_decl_members(lines, idx, keyword, &scoped_name)
                    .map(|members| {
                        members
                            .into_iter()
                            .map(|member| member.text)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let mut encoded = scoped_name;
                if has_alias {
                    encoded.push('\t');
                    encoded.push_str(&name);
                }
                // Embed actor marker so the normalizer can promote to Actor kind
                // (mirrors the same logic in the top-level declaration loop above).
                if keyword == "actor" {
                    encoded.push('\t');
                    encoded.push_str("<<actor>>");
                }
                if let Some(fill_color) = fill_color {
                    encoded.push('\t');
                    encoded.push_str(&format!("\x1fstyle:fill:{fill_color}"));
                }
                for member in members_text {
                    encoded.push('\t');
                    encoded.push_str(&member);
                }
                content.members.push(encoded);
                if nested_end > idx {
                    idx = nested_end + 1;
                    continue;
                }
            } else if let Some(decl) =
                parse_named_family_decl(line, keyword).filter(|decl| !decl.has_block)
            {
                let FamilyDeclParts {
                    name,
                    alias,
                    fill_color,
                    ..
                } = decl;
                let has_alias = alias.is_some();
                let id = alias.unwrap_or_else(|| name.clone());
                let mut encoded = qualify_scoped_identifier(id, scope);
                if has_alias {
                    encoded.push('\t');
                    encoded.push_str(&name);
                }
                // Embed actor marker so the normalizer can promote to Actor kind
                // (mirrors the same logic in the top-level declaration loop above).
                if keyword == "actor" {
                    encoded.push('\t');
                    encoded.push_str("<<actor>>");
                }
                if let Some(fill_color) = fill_color {
                    encoded.push('\t');
                    encoded.push_str(&format!("\x1fstyle:fill:{fill_color}"));
                }
                content.members.push(encoded);
                idx += 1;
                continue;
            }
        }
        let name = extract_class_member_name(line);
        if !name.is_empty() {
            let scoped = qualify_scoped_identifier(name, scope);
            content.members.push(scoped);
        }
        if line.ends_with('{') {
            let nested_end = find_family_decl_end(lines, idx);
            if nested_end > idx {
                idx = nested_end + 1;
                continue;
            }
        }
        idx += 1;
    }
    content
}

fn scoped_prefix(scope: &[String]) -> String {
    scope
        .iter()
        .filter(|s| !s.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join("::")
}

fn qualify_scoped_identifier(name: String, scope: &[String]) -> String {
    let prefix = scoped_prefix(scope);
    if prefix.is_empty()
        || name.is_empty()
        || name.contains("::")
        || name == "[*]"
        || name == "[H]"
        || name == "[H*]"
    {
        name
    } else {
        format!("{prefix}::{name}")
    }
}

fn qualify_scoped_relation(mut rel: FamilyRelation, scope: &[String]) -> FamilyRelation {
    rel.from = qualify_scoped_identifier(rel.from, scope);
    rel.to = qualify_scoped_identifier(rel.to, scope);
    rel
}

fn group_body_contains_component_family(
    lines: &[(&str, Span)],
    start: usize,
    end_idx: usize,
) -> bool {
    lines[start + 1..end_idx].iter().any(|(raw, _)| {
        let line = strip_inline_plantuml_comment(raw).trim();
        component_decl_keyword(line).is_some()
    })
}

fn group_body_contains_class_family(lines: &[(&str, Span)], start: usize, end_idx: usize) -> bool {
    lines[start + 1..end_idx].iter().any(|(raw, _)| {
        let line = strip_inline_plantuml_comment(raw).trim();
        let lower = line.to_ascii_lowercase();
        lower.starts_with("abstract class ")
            || lower.starts_with("annotation ")
            || lower.starts_with("interface ")
            || lower.starts_with("abstract ")
            || lower.starts_with("enum ")
            || lower.starts_with("exception ")
            || lower.starts_with("metaclass ")
            || lower.starts_with("stereotype ")
            || lower.starts_with("circle ")
            || lower.starts_with("diamond ")
            || lower.starts_with("protocol ")
            || lower.starts_with("struct ")
            || lower.starts_with("class ")
            || (lower.starts_with("entity ") && lower.ends_with('{'))
    })
}

fn group_body_contains_object_family(lines: &[(&str, Span)], start: usize, end_idx: usize) -> bool {
    lines[start + 1..end_idx].iter().any(|(raw, _)| {
        let line = strip_inline_plantuml_comment(raw).trim();
        let lower = line.to_ascii_lowercase();
        lower.starts_with("object ") || lower.starts_with("map ")
    })
}

fn group_body_contains_usecase_family(
    lines: &[(&str, Span)],
    start: usize,
    end_idx: usize,
) -> bool {
    lines[start + 1..end_idx].iter().any(|(raw, _)| {
        let line = strip_inline_plantuml_comment(raw).trim();
        let lower = line.to_ascii_lowercase();
        lower.starts_with("usecase ")
            || lower.starts_with("usecase(")
            || lower.starts_with("actor ")
            || parse_parenthesized_usecase_decl(line).is_some()
    })
}

fn scoped_family_kind_for_block(
    lines: &[(&str, Span)],
    start: usize,
    end_idx: usize,
) -> DiagramKind {
    if group_body_contains_object_family(lines, start, end_idx) {
        DiagramKind::Object
    } else if group_body_contains_usecase_family(lines, start, end_idx) {
        DiagramKind::UseCase
    } else {
        DiagramKind::Class
    }
}
