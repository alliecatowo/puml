fn parse_component_decl(line: &str) -> Option<StatementKind> {
    use crate::ast::ComponentNodeKind as K;
    let keywords: &[(&str, K)] = &[
        ("component", K::Component),
        ("interface", K::Interface),
        ("portin", K::Port),
        ("portout", K::Port),
        ("port", K::Port),
        ("node", K::Node),
        ("database", K::Database),
        ("cloud", K::Cloud),
        ("frame", K::Frame),
        ("storage", K::Storage),
        ("package", K::Package),
        ("rectangle", K::Rectangle),
        ("folder", K::Folder),
        ("file", K::File),
        ("card", K::Card),
        ("artifact", K::Artifact),
        ("actor", K::Actor),
    ];
    for (kw, kind) in keywords.iter().copied() {
        let trimmed = line.trim();
        if !trimmed.starts_with(kw) {
            continue;
        }
        let rest_raw = trimmed[kw.len()..].trim();
        if rest_raw.is_empty() {
            return None;
        }
        if looks_like_family_relation_tail(rest_raw) {
            continue;
        }
        if rest_raw.starts_with('-') || rest_raw.starts_with('.') || rest_raw.starts_with('<') {
            return None;
        }
        // Must be followed by whitespace OR the rest is a non-identifier prefix; require space.
        if !line
            .as_bytes()
            .get(kw.len())
            .copied()
            .is_some_and(|b| b == b' ' || b == b'\t')
        {
            // For the very first char after kw, ensure it's whitespace.
            // (line is already trimmed by caller; recompute on trimmed)
            let bytes = trimmed.as_bytes();
            if let Some(&b) = bytes.get(kw.len()) {
                if !(b == b' ' || b == b'\t') {
                    continue;
                }
            }
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
        match kw {
            "portin" => members.push(ClassMember {
                text: "<<portin>>".to_string(),
                modifier: None,
            }),
            "portout" => members.push(ClassMember {
                text: "<<portout>>".to_string(),
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
    // Anonymous shorthand: `[Name]` declares a component, `() Name` declares an interface.
    let trimmed = line.trim();
    if let Some(rest) = trimmed.strip_prefix('[') {
        if let Some(end) = rest.find(']') {
            let inner = rest[..end].trim();
            let suffix = rest[end + 1..].trim();
            if !suffix.is_empty() && !suffix.starts_with("as ") {
                return None;
            }
            let bracketed_inner = format!("[{inner}]");
            if normalize_virtual_endpoint(&bracketed_inner).is_some()
                || matches!(inner, "*" | "H" | "H*")
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
                    members: Vec::new(),
                });
            }
        }
    }
    if let Some(rest) = trimmed.strip_prefix("()") {
        let rest = rest.trim();
        if !rest.is_empty() {
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
            if !name.is_empty() {
                return Some(StatementKind::ComponentDecl {
                    kind: ComponentNodeKind::Interface,
                    name,
                    alias: alias.filter(|v| !v.is_empty()),
                    label,
                    members: Vec::new(),
                });
            }
        }
    }
    if let Some(inner) = trimmed.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
        let bracketed_inner = format!("[{inner}]");
        if normalize_virtual_endpoint(&bracketed_inner).is_none()
            && !matches!(inner, "*" | "H" | "H*")
            && !inner.is_empty()
            && !inner.contains('[')
            && !inner.contains(']')
        {
            return Some(StatementKind::ComponentDecl {
                kind: ComponentNodeKind::Component,
                name: clean_ident(inner),
                alias: None,
                label: None,
                members: Vec::new(),
            });
        }
    }
    if let Some(rest) = trimmed.strip_prefix("()") {
        let rest = rest.trim();
        if !rest.is_empty() {
            return Some(StatementKind::ComponentDecl {
                kind: ComponentNodeKind::Interface,
                name: clean_ident(rest),
                alias: None,
                label: None,
                members: Vec::new(),
            });
        }
    }
    None
}

fn looks_like_family_relation_tail(rest: &str) -> bool {
    rest.contains("--")
        || rest.contains("..")
        || rest.contains("->")
        || rest.contains("<-")
        || rest.contains("-[")
        || rest.contains(".[")
}
