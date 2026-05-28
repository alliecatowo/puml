use super::*;

pub(crate) fn parse_deployment_usecase_decl(line: &str) -> Option<StatementKind> {
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

pub(crate) fn parse_component_multiline_decl(
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
    let mut port_members: Vec<ClassMember> = Vec::new();
    for (idx, (raw, _)) in lines.iter().enumerate().skip(start + 1) {
        let text = strip_inline_plantuml_comment(raw).trim();
        if text == "]" {
            if let StatementKind::ComponentDecl { label, members, .. } = &mut kind {
                let label_body = body.join("\n");
                if !label_body.is_empty() {
                    *label = Some(label_body);
                }
                members.extend(port_members);
            }
            return Ok(Some((kind, idx)));
        }
        // Detect `port NAME`, `portin NAME`, `portout NAME` lines inside the block.
        // Encode them as special members so the normalizer can emit separate Port nodes.
        let lower = text.to_ascii_lowercase();
        if let Some(port_name_raw) = lower
            .strip_prefix("portin ")
            .or_else(|| lower.strip_prefix("portout "))
            .or_else(|| lower.strip_prefix("port "))
        {
            let raw_offset = text.len() - port_name_raw.len();
            let port_name = clean_ident(text[raw_offset..].trim());
            if !port_name.is_empty() {
                let direction_hint = if lower.starts_with("portin ") {
                    "portin"
                } else if lower.starts_with("portout ") {
                    "portout"
                } else {
                    "port"
                };
                port_members.push(ClassMember {
                    text: format!("\x1fcomponent:port:{direction_hint}:{port_name}"),
                    modifier: None,
                });
                continue;
            }
        }
        body.push(text.to_string());
    }
    Err(Diagnostic::error(
        "[E_COMPONENT_DECL_UNCLOSED] unclosed component declaration body: missing `]`",
    )
    .with_span(lines[start].1))
}
