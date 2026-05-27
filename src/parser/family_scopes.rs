use super::*;
#[derive(Debug, Clone, Default)]
pub(crate) struct ScopedGroupContent {
    pub(crate) members: Vec<String>,
    pub(crate) relations: Vec<FamilyRelation>,
}

/// Parse `together { ... }`, `package "name" { ... }`, `namespace ns { ... }` blocks.
/// Returns (StatementKind, end_line_index) where end_line_index points to the closing `}`.
pub(crate) fn parse_class_scoping_block(
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

pub(crate) fn collect_scoped_class_group_content(
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
        if let Some(StatementKind::AssociationClass {
            left,
            right,
            association,
            arrow,
        }) = parse_association_class_relation(line)
        {
            let scoped_association = qualify_scoped_identifier(association, scope);
            if !content.members.iter().any(|member| {
                member.split('\t').next().unwrap_or(member.as_str()) == scoped_association
            }) {
                content.members.push(scoped_association.clone());
            }
            content.relations.extend(association_class_family_relations(
                qualify_scoped_identifier(left, scope),
                qualify_scoped_identifier(right, scope),
                scoped_association,
                arrow,
            ));
            idx += 1;
            continue;
        }
        if let Some(kinds) = parse_family_relation(line, Some(DiagramKind::Class)) {
            for kind in kinds {
                if let StatementKind::FamilyRelation(rel) = kind {
                    content.relations.push(qualify_scoped_relation(rel, scope));
                }
            }
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
                style_members,
                business,
                ..
            } = decl;
            let has_alias = alias.is_some();
            let id = alias.unwrap_or_else(|| name.clone());
            let mut encoded = qualify_scoped_identifier(id, scope);
            if has_alias {
                encoded.push('\t');
                encoded.push_str(&name);
            }
            if business {
                encoded.push('\t');
                encoded.push_str("<<business>>");
            }
            if let Some(fill_color) = fill_color {
                encoded.push('\t');
                encoded.push_str(&format!("\x1fstyle:fill:{fill_color}"));
            }
            for style_member in style_members {
                encoded.push('\t');
                encoded.push_str(&style_member);
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
            "actor/",
            "usecase/",
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
                    style_members,
                    business,
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
                if keyword.starts_with("actor") {
                    encoded.push('\t');
                    encoded.push_str("<<actor>>");
                }
                if business || keyword.ends_with('/') {
                    encoded.push('\t');
                    encoded.push_str("<<business>>");
                }
                if let Some(fill_color) = fill_color {
                    encoded.push('\t');
                    encoded.push_str(&format!("\x1fstyle:fill:{fill_color}"));
                }
                for style_member in style_members {
                    encoded.push('\t');
                    encoded.push_str(&style_member);
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
            "actor/",
            "usecase/",
            "actor",
            "usecase",
        ] {
            if let Some(decl) = parse_named_family_decl(line, keyword).filter(|decl| decl.has_block)
            {
                let FamilyDeclParts {
                    name,
                    alias,
                    fill_color,
                    style_members,
                    business,
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
                if keyword.starts_with("actor") {
                    encoded.push('\t');
                    encoded.push_str("<<actor>>");
                }
                if business || keyword.ends_with('/') {
                    encoded.push('\t');
                    encoded.push_str("<<business>>");
                }
                if let Some(fill_color) = fill_color {
                    encoded.push('\t');
                    encoded.push_str(&format!("\x1fstyle:fill:{fill_color}"));
                }
                for style_member in style_members {
                    encoded.push('\t');
                    encoded.push_str(&style_member);
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
                    style_members,
                    business,
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
                if keyword.starts_with("actor") {
                    encoded.push('\t');
                    encoded.push_str("<<actor>>");
                }
                if business || keyword.ends_with('/') {
                    encoded.push('\t');
                    encoded.push_str("<<business>>");
                }
                if let Some(fill_color) = fill_color {
                    encoded.push('\t');
                    encoded.push_str(&format!("\x1fstyle:fill:{fill_color}"));
                }
                for style_member in style_members {
                    encoded.push('\t');
                    encoded.push_str(&style_member);
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

pub(crate) fn scoped_prefix(scope: &[String]) -> String {
    scope
        .iter()
        .filter(|s| !s.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join("::")
}

pub(crate) fn qualify_scoped_identifier(name: String, scope: &[String]) -> String {
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

pub(crate) fn qualify_scoped_relation(mut rel: FamilyRelation, scope: &[String]) -> FamilyRelation {
    rel.from = qualify_scoped_identifier(rel.from, scope);
    rel.to = qualify_scoped_identifier(rel.to, scope);
    rel
}
