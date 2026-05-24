fn parse_component_decl(line: &str) -> Option<StatementKind> {
    for (kw, kind) in component_decl_keywords() {
        let trimmed = line.trim();
        if !trimmed.starts_with(kw) {
            continue;
        }
        let rest_raw = trimmed[kw.len()..].trim();
        if rest_raw.is_empty() {
            return None;
        }
        if rest_raw.ends_with('{') {
            continue;
        }
        if looks_like_family_relation_tail(rest_raw) {
            continue;
        }
        if rest_raw.starts_with('-') || rest_raw.starts_with('.') || rest_raw.starts_with('<') {
            return None;
        }
        if !trimmed
            .as_bytes()
            .get(kw.len())
            .copied()
            .is_some_and(|b| b == b' ' || b == b'\t')
        {
            continue;
        }
        let rest = rest_raw.trim_end_matches('{').trim();
        let (rest, fill_color) = split_declaration_inline_fill(rest);
        let rest = rest.trim();
        let (rest_without_stereotypes, stereotypes) = strip_declaration_stereotypes(rest);
        let rest = rest_without_stereotypes.trim();
        let (label, rest_after_label) = if rest.starts_with('"') {
            let stripped = rest.strip_prefix('"')?;
            let end = stripped.find('"')?;
            (
                Some(stripped[..end].to_string()),
                stripped[end + 1..].trim(),
            )
        } else if rest.starts_with('[') {
            let stripped = rest.strip_prefix('[')?;
            let end = stripped.find(']')?;
            (
                Some(stripped[..end].trim().to_string()),
                stripped[end + 1..].trim(),
            )
        } else {
            (None, rest)
        };
        let (rest_after_label, tags) = split_component_trailing_tags(rest_after_label);
        let rest_after_label = rest_after_label.as_str();
        let (name_raw, alias_raw) = if let Some(alias) = rest_after_label.strip_prefix("as ") {
            (label.as_deref().unwrap_or("").trim(), Some(alias.trim()))
        } else if let Some((lhs, rhs)) = rest_after_label.split_once(" as ") {
            (lhs.trim(), Some(rhs.trim()))
        } else if rest_after_label.is_empty() {
            (label.as_deref().unwrap_or("").trim(), None)
        } else {
            (rest_after_label, None)
        };
        let name = clean_bracketed_ident(name_raw);
        if name.is_empty() {
            return None;
        }
        let alias = alias_raw.map(clean_ident).filter(|v| !v.is_empty());
        let mut members = declaration_marker_members(None, stereotypes);
        append_component_tag_members(&mut members, tags);
        match kw {
            "portin" => members.push(ClassMember {
                text: "<<portin>>".to_string(),
                modifier: None,
            }),
            "portout" => members.push(ClassMember {
                text: "<<portout>>".to_string(),
                modifier: None,
            }),
            "actor/" => members.push(ClassMember {
                text: "<<actor/>>".to_string(),
                modifier: None,
            }),
            "usecase/" => members.push(ClassMember {
                text: "<<usecase/>>".to_string(),
                modifier: None,
            }),
            _ => {}
        }
        append_inline_fill_member(&mut members, fill_color);
        return Some(StatementKind::ComponentDecl {
            kind,
            name,
            alias,
            label,
            members,
        });
    }

    let trimmed = line.trim();
    if let Some(kind) = parse_component_bracketed_shorthand(trimmed) {
        return Some(kind);
    }
    if let Some(kind) = parse_actor_colon_shorthand(trimmed) {
        return Some(kind);
    }
    if let Some(kind) = parse_component_parenthesized_usecase_shorthand(trimmed) {
        return Some(kind);
    }
    if let Some(kind) = parse_component_interface_shorthand(trimmed) {
        return Some(kind);
    }
    None
}

fn component_decl_keywords() -> impl Iterator<Item = (&'static str, ComponentNodeKind)> + Clone {
    crate::registry::component_declaration_keywords()
}

fn component_decl_keyword(line: &str) -> Option<(&'static str, ComponentNodeKind)> {
    let trimmed = line.trim_start();
    component_decl_keywords().find(|(kw, _)| {
        trimmed.starts_with(kw)
            && trimmed
                .as_bytes()
                .get(kw.len())
                .copied()
                .is_some_and(|b| b == b' ' || b == b'\t')
    })
}

fn split_component_trailing_tags(input: &str) -> (String, Vec<String>) {
    let mut rest = input.trim_end();
    let mut tags = Vec::new();
    while let Some((start, token)) = last_component_token(rest) {
        if !is_component_tag_token(token) {
            break;
        }
        tags.push(token.to_string());
        rest = rest[..start].trim_end();
    }
    tags.reverse();
    (rest.trim().to_string(), tags)
}

fn last_component_token(input: &str) -> Option<(usize, &str)> {
    let trimmed = input.trim_end();
    if trimmed.is_empty() {
        return None;
    }
    let start = trimmed
        .char_indices()
        .rev()
        .find_map(|(idx, ch)| ch.is_whitespace().then_some(idx + ch.len_utf8()))
        .unwrap_or(0);
    Some((start, &trimmed[start..]))
}

fn is_component_tag_token(token: &str) -> bool {
    let Some(rest) = token.strip_prefix('$') else {
        return false;
    };
    !rest.is_empty()
        && rest
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
}

fn append_component_tag_members(members: &mut Vec<ClassMember>, tags: Vec<String>) {
    for tag in tags {
        members.push(ClassMember {
            text: format!("\x1fcomponent:tag:{tag}"),
            modifier: None,
        });
    }
}

fn is_component_container_keyword(keyword: &str) -> bool {
    matches!(
        keyword,
        "action"
            | "artifact"
            | "card"
            | "cloud"
            | "component"
            | "container"
            | "database"
            | "file"
            | "folder"
            | "frame"
            | "hexagon"
            | "node"
            | "package"
            | "process"
            | "queue"
            | "rectangle"
            | "stack"
            | "storage"
    )
}

fn is_ambiguous_sequence_participant_keyword(keyword: &str) -> bool {
    matches!(
        keyword,
        "actor" | "boundary" | "control" | "entity" | "collections" | "queue"
    )
}

fn is_ambiguous_activity_keyword(keyword: &str) -> bool {
    matches!(keyword, "action" | "label")
}

fn parse_component_bracketed_shorthand(trimmed: &str) -> Option<StatementKind> {
    let rest = trimmed.strip_prefix('[')?;
    let end = rest.find(']')?;
    let inner = rest[..end].trim();
    let suffix = rest[end + 1..].trim();
    let (suffix_without_tags, tags) = split_component_trailing_tags(suffix);
    let suffix = suffix_without_tags.trim();
    if !suffix.is_empty() && !suffix.starts_with("as ") {
        return None;
    }
    let bracketed_inner = format!("[{inner}]");
    if normalize_virtual_endpoint(&bracketed_inner).is_some() || matches!(inner, "*" | "H" | "H*")
    {
        return None;
    }
    let alias = suffix
        .strip_prefix("as ")
        .map(str::trim)
        .map(clean_ident)
        .filter(|v| !v.is_empty());
    if !inner.is_empty() && !inner.contains('[') && !inner.contains(']') {
        let name = alias.clone().unwrap_or_else(|| clean_ident(inner));
        let label = alias.as_ref().map(|_| inner.to_string());
        return Some(StatementKind::ComponentDecl {
            kind: ComponentNodeKind::Component,
            name,
            alias,
            label,
            members: {
                let mut members = Vec::new();
                append_component_tag_members(&mut members, tags);
                members
            },
        });
    }
    None
}

fn parse_actor_colon_shorthand(trimmed: &str) -> Option<StatementKind> {
    let inner = trimmed.strip_prefix(':')?.strip_suffix(':')?.trim();
    if inner.is_empty() || inner.contains(':') {
        return None;
    }
    Some(StatementKind::ComponentDecl {
        kind: ComponentNodeKind::Actor,
        name: clean_ident(inner),
        alias: None,
        label: Some(inner.to_string()),
        members: Vec::new(),
    })
}

fn parse_component_parenthesized_usecase_shorthand(trimmed: &str) -> Option<StatementKind> {
    let rest = trimmed.strip_prefix('(')?;
    let end = rest.find(')')?;
    let inner = rest[..end].trim();
    if inner.is_empty() || inner.contains('(') || inner.contains(')') {
        return None;
    }
    let suffix = rest[end + 1..].trim();
    if !suffix.is_empty() && !suffix.starts_with("as ") {
        return None;
    }
    let alias = suffix
        .strip_prefix("as ")
        .map(str::trim)
        .map(clean_ident)
        .filter(|value| !value.is_empty());
    Some(StatementKind::ComponentDecl {
        kind: ComponentNodeKind::UseCase,
        name: alias.clone().unwrap_or_else(|| clean_ident(inner)),
        alias,
        label: Some(inner.to_string()),
        members: Vec::new(),
    })
}

fn parse_component_interface_shorthand(trimmed: &str) -> Option<StatementKind> {
    let rest = trimmed.strip_prefix("()")?.trim();
    if rest.is_empty() {
        return None;
    }
    let (label, rest_after_label) = if rest.starts_with('"') {
        let stripped = rest.strip_prefix('"')?;
        let end = stripped.find('"')?;
        (
            Some(stripped[..end].to_string()),
            stripped[end + 1..].trim(),
        )
    } else {
        (None, rest)
    };
    let (name_raw, alias) = if let Some(alias) = rest_after_label.strip_prefix("as ") {
        (
            label.as_deref().unwrap_or("").trim(),
            Some(clean_ident(alias.trim())),
        )
    } else if let Some((lhs, rhs)) = rest_after_label.split_once(" as ") {
        (lhs.trim(), Some(clean_ident(rhs.trim())))
    } else {
        (rest_after_label, None)
    };
    let name = alias
        .clone()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| clean_ident(name_raw));
    if name.is_empty() {
        return None;
    }
    Some(StatementKind::ComponentDecl {
        kind: ComponentNodeKind::Interface,
        name,
        alias: alias.filter(|v| !v.is_empty()),
        label,
        members: Vec::new(),
    })
}

fn parse_deployment_usecase_decl(line: &str) -> Option<StatementKind> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix("usecase ")?.trim();
    if rest.is_empty() || rest.ends_with('{') || looks_like_family_relation_tail(rest) {
        return None;
    }
    let (rest, fill_color) = split_declaration_inline_fill(rest);
    let rest = rest.trim();
    let (rest_without_stereotypes, stereotypes) = strip_declaration_stereotypes(rest);
    let rest = rest_without_stereotypes.trim();
    let (label, rest_after_label) = if rest.starts_with('"') {
        let stripped = rest.strip_prefix('"')?;
        let end = stripped.find('"')?;
        (
            Some(stripped[..end].to_string()),
            stripped[end + 1..].trim(),
        )
    } else if rest.starts_with('(') {
        let stripped = rest.strip_prefix('(')?;
        let end = stripped.find(')')?;
        (
            Some(stripped[..end].to_string()),
            stripped[end + 1..].trim(),
        )
    } else {
        (None, rest)
    };
    let (rest_after_label, tags) = split_component_trailing_tags(rest_after_label);
    let rest_after_label = rest_after_label.as_str();
    let (name_raw, alias_raw) = if let Some(alias) = rest_after_label.strip_prefix("as ") {
        (label.as_deref().unwrap_or("").trim(), Some(alias.trim()))
    } else if let Some((lhs, rhs)) = rest_after_label.split_once(" as ") {
        (lhs.trim(), Some(rhs.trim()))
    } else if rest_after_label.is_empty() {
        (label.as_deref().unwrap_or("").trim(), None)
    } else {
        (rest_after_label, None)
    };
    let name = clean_bracketed_ident(name_raw);
    if name.is_empty() {
        return None;
    }
    let mut members = declaration_marker_members(None, stereotypes);
    append_component_tag_members(&mut members, tags);
    append_inline_fill_member(&mut members, fill_color);
    Some(StatementKind::ComponentDecl {
        kind: ComponentNodeKind::UseCase,
        name,
        alias: alias_raw.map(clean_ident).filter(|v| !v.is_empty()),
        label,
        members,
    })
}

fn parse_component_multiline_decl(
    lines: &[(&str, Span)],
    start: usize,
    line: &str,
) -> Result<Option<(StatementKind, usize)>, Diagnostic> {
    let trimmed = line.trim();
    let Some(_) = component_decl_keyword(trimmed) else {
        return Ok(None);
    };
    let Some(open_idx) = trimmed.rfind('[') else {
        return Ok(None);
    };
    if !trimmed[open_idx + 1..].trim().is_empty() {
        return Ok(None);
    }
    let prefix = trimmed[..open_idx].trim();
    if prefix.is_empty() {
        return Ok(None);
    }
    let Some(mut kind) = parse_component_decl(prefix) else {
        return Ok(None);
    };
    let mut body = Vec::new();
    for (idx, (raw, _)) in lines.iter().enumerate().skip(start + 1) {
        let text = strip_inline_plantuml_comment(raw).trim();
        if text == "]" {
            if let StatementKind::ComponentDecl { label, .. } = &mut kind {
                *label = Some(body.join("\n"));
            }
            return Ok(Some((kind, idx)));
        }
        body.push(text.to_string());
    }
    Err(Diagnostic::error(
        "[E_COMPONENT_DECL_UNCLOSED] unclosed component declaration body: missing `]`",
    )
    .with_span(lines[start].1))
}

fn looks_like_family_relation_tail(rest: &str) -> bool {
    rest.contains("--")
        || rest.contains("..")
        || rest.contains("->")
        || rest.contains("<-")
        || rest.contains("-[")
        || rest.contains(".[")
}
