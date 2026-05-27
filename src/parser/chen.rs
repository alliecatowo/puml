use super::*;
pub(crate) fn parse_chen_declaration(
    lines: &[(&str, Span)],
    start: usize,
    line: &str,
) -> Result<Option<(StatementKind, usize)>, Diagnostic> {
    for (keyword, kind) in [
        ("entity", ChenDeclKind::Entity),
        ("relationship", ChenDeclKind::Relationship),
    ] {
        let Some((name, alias, stereotypes, has_block)) = parse_chen_decl_head(line, keyword)
        else {
            continue;
        };
        let end_idx = if has_block {
            find_chen_block_end(lines, start).ok_or_else(|| {
                Diagnostic::error(format!(
                    "[E_CHEN_DECL_BLOCK_UNCLOSED] unclosed {keyword} declaration block for `{name}`: missing `}}`"
                ))
                .with_span(lines[start].1)
            })?
        } else {
            start
        };
        let attributes = if has_block {
            parse_chen_attributes(&lines[start + 1..end_idx])?
        } else {
            Vec::new()
        };
        return Ok(Some((
            StatementKind::ChenDecl(ChenDecl {
                kind,
                name,
                alias,
                stereotypes,
                attributes,
            }),
            end_idx,
        )));
    }
    Ok(None)
}

pub(crate) fn parse_chen_decl_head(
    line: &str,
    keyword: &str,
) -> Option<(String, Option<String>, Vec<String>, bool)> {
    let lower = line.to_ascii_lowercase();
    if !lower.starts_with(keyword) {
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
    let head = if has_block {
        rest.trim_end_matches('{').trim()
    } else {
        rest
    };
    let (head, stereotypes) = strip_declaration_stereotypes(head);
    let (name_raw, alias_raw) = split_chen_alias(&head);
    let name = clean_ident(name_raw);
    if name.is_empty() {
        return None;
    }
    let alias = alias_raw.map(clean_ident).filter(|value| !value.is_empty());
    Some((name, alias, stereotypes, has_block))
}

pub(crate) fn split_chen_alias(input: &str) -> (&str, Option<&str>) {
    if let Some(idx) = find_top_level_keyword(input, " as ") {
        let lhs = input[..idx].trim();
        let rhs = input[idx + 4..].trim();
        return (lhs, Some(rhs));
    }
    (input.trim(), None)
}

pub(crate) fn find_chen_block_end(lines: &[(&str, Span)], start: usize) -> Option<usize> {
    let mut depth = 0i32;
    for (idx, (raw, _)) in lines.iter().enumerate().skip(start) {
        let line = strip_inline_plantuml_comment(raw);
        depth += line.chars().filter(|ch| *ch == '{').count() as i32;
        depth -= line.chars().filter(|ch| *ch == '}').count() as i32;
        if idx > start && depth <= 0 {
            return Some(idx);
        }
    }
    None
}

pub(crate) fn parse_chen_attributes(
    lines: &[(&str, Span)],
) -> Result<Vec<ChenAttribute>, Diagnostic> {
    let mut attrs = Vec::new();
    let mut idx = 0usize;
    while idx < lines.len() {
        let (raw, span) = lines[idx];
        let line = strip_inline_plantuml_comment(raw).trim();
        if line.is_empty() {
            idx += 1;
            continue;
        }
        if line == "}" {
            return Err(Diagnostic::error(
                "[E_CHEN_ATTR_BLOCK_UNMATCHED] unmatched `}` in Chen attributes",
            )
            .with_span(span));
        }
        let has_children = line.ends_with('{');
        let head = if has_children {
            line.trim_end_matches('{').trim()
        } else {
            line
        };
        let mut attr = parse_chen_attribute_head(head);
        if attr.name.is_empty() {
            return Err(Diagnostic::error(format!(
                "[E_CHEN_ATTR_INVALID] invalid Chen attribute syntax: `{line}`"
            ))
            .with_span(span));
        }
        if has_children {
            let end_idx = find_chen_attribute_child_end(lines, idx).ok_or_else(|| {
                Diagnostic::error(format!(
                    "[E_CHEN_ATTR_BLOCK_UNCLOSED] unclosed Chen attribute block for `{}`",
                    attr.name
                ))
                .with_span(span)
            })?;
            attr.children = parse_chen_attributes(&lines[idx + 1..end_idx])?;
            idx = end_idx + 1;
        } else {
            idx += 1;
        }
        attrs.push(attr);
    }
    Ok(attrs)
}

pub(crate) fn find_chen_attribute_child_end(lines: &[(&str, Span)], start: usize) -> Option<usize> {
    let mut depth = 0i32;
    for (idx, (raw, _)) in lines.iter().enumerate().skip(start) {
        let line = strip_inline_plantuml_comment(raw);
        depth += line.chars().filter(|ch| *ch == '{').count() as i32;
        depth -= line.chars().filter(|ch| *ch == '}').count() as i32;
        if idx > start && depth <= 0 {
            return Some(idx);
        }
    }
    None
}

pub(crate) fn parse_chen_attribute_head(input: &str) -> ChenAttribute {
    let (without_stereotypes, stereotypes) = strip_declaration_stereotypes(input);
    let (name_part, data_type) = without_stereotypes
        .split_once(':')
        .map(|(name, ty)| (name.trim(), Some(ty.trim().to_string())))
        .unwrap_or((without_stereotypes.trim(), None));
    let (name_raw, alias_raw) = split_chen_alias(name_part);
    ChenAttribute {
        name: clean_ident(name_raw),
        alias: alias_raw.map(clean_ident).filter(|value| !value.is_empty()),
        data_type: data_type.filter(|value| !value.is_empty()),
        stereotypes,
        children: Vec::new(),
    }
}

pub(crate) fn parse_chen_relation(line: &str) -> Option<StatementKind> {
    let parts = line.split_whitespace().collect::<Vec<_>>();
    if parts.len() < 3 {
        return None;
    }
    let arrow_idx = parts
        .iter()
        .position(|part| is_chen_cardinality_token(part))?;
    if arrow_idx == 0 || arrow_idx + 1 >= parts.len() {
        return None;
    }
    let from = clean_ident(&parts[..arrow_idx].join(" "));
    let to = clean_ident(&parts[arrow_idx + 1..].join(" "));
    if from.is_empty() || to.is_empty() {
        return None;
    }
    let token = parts[arrow_idx];
    let total_participation = token.starts_with('=') && token.ends_with('=');
    let cardinality = token.trim_matches('-').trim_matches('=').trim().to_string();
    if cardinality.is_empty() {
        return None;
    }
    Some(StatementKind::ChenRelation(ChenRelation {
        from,
        to,
        cardinality,
        total_participation,
    }))
}

pub(crate) fn is_chen_cardinality_token(token: &str) -> bool {
    if token.len() < 3 {
        return false;
    }
    let first = token.as_bytes().first().copied();
    let last = token.as_bytes().last().copied();
    matches!(
        (first, last),
        (Some(b'-'), Some(b'-')) | (Some(b'='), Some(b'='))
    ) && !matches!(token, "->-" | "-<-" | "=>=")
}

pub(crate) fn parse_chen_inheritance(line: &str) -> Option<StatementKind> {
    let (parent, connector, discriminator, children) =
        if let Some((lhs, rhs)) = line.split_once("=>=") {
            parse_chen_set_connector(lhs, "=>=", rhs)?
        } else if let Some((lhs, rhs)) = line.split_once("->-") {
            parse_chen_set_connector(lhs, "->-", rhs).or_else(|| {
                Some((
                    clean_ident(rhs),
                    "->-".to_string(),
                    None,
                    vec![clean_ident(lhs)],
                ))
            })?
        } else if let Some((lhs, rhs)) = line.split_once("-<-") {
            Some((
                clean_ident(lhs),
                "-<-".to_string(),
                None,
                vec![clean_ident(rhs)],
            ))?
        } else {
            return None;
        };
    if parent.is_empty() || children.iter().any(|child| child.is_empty()) {
        return None;
    }
    Some(StatementKind::ChenInheritance(ChenInheritance {
        parent,
        connector,
        discriminator,
        children,
    }))
}

pub(crate) fn parse_chen_set_connector(
    lhs: &str,
    connector: &str,
    rhs: &str,
) -> Option<(String, String, Option<String>, Vec<String>)> {
    let open = rhs.find('{')?;
    let close = rhs.rfind('}')?;
    if close <= open {
        return None;
    }
    let discriminator = clean_ident(rhs[..open].trim());
    let children = rhs[open + 1..close]
        .split(',')
        .map(clean_ident)
        .filter(|child| !child.is_empty())
        .collect::<Vec<_>>();
    if children.is_empty() {
        return None;
    }
    Some((
        clean_ident(lhs),
        connector.to_string(),
        (!discriminator.is_empty()).then_some(discriminator),
        children,
    ))
}
