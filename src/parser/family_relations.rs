fn parse_family_relation(line: &str, family: Option<DiagramKind>) -> Option<Vec<StatementKind>> {
    // When family is still unknown, only C4 legacy relation macros are accepted.
    // This supports valid C4 inputs where relations appear before declarations.
    if family.is_none() {
        return parse_c4_legacy_family_relation(line, None);
    }

    match family {
        Some(DiagramKind::Class)
        | Some(DiagramKind::Object)
        | Some(DiagramKind::UseCase)
        | Some(DiagramKind::Salt)
        | Some(DiagramKind::MindMap)
        | Some(DiagramKind::Wbs)
        | Some(DiagramKind::Component)
        | Some(DiagramKind::Deployment) => {}
        _ => return None,
    }

    if let Some(kinds) = parse_c4_legacy_family_relation(line, family) {
        return Some(kinds);
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
    // Component/Deployment diagrams use `[Name]` bracket syntax for nodes, so
    // `looks_like_virtual_endpoint_syntax` (which rejects any `[`/`]`) must be
    // skipped for those families.
    let is_bracket_family = matches!(
        family,
        Some(DiagramKind::Component) | Some(DiagramKind::Deployment)
    );
    if normalize_virtual_endpoint(&lhs_core).is_some()
        || normalize_virtual_endpoint(&rhs_core).is_some()
        || (!is_bracket_family && looks_like_virtual_endpoint_syntax(&lhs_core))
        || (!is_bracket_family && looks_like_virtual_endpoint_syntax(&rhs_core))
    {
        return None;
    }
    let from = clean_bracketed_ident(&lhs_core);
    let to = clean_bracketed_ident(&rhs_core);
    if from.is_empty() || to.is_empty() {
        return None;
    }
    Some(vec![StatementKind::FamilyRelation(FamilyRelation {
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
    })])
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

fn association_class_family_relations(
    left: String,
    right: String,
    association: String,
    arrow: String,
) -> Vec<FamilyRelation> {
    vec![
        FamilyRelation {
            from: left.clone(),
            to: right.clone(),
            arrow,
            label: None,
            stereotype: None,
            left_cardinality: None,
            right_cardinality: None,
            left_role: None,
            right_role: None,
            line_color: None,
            dashed: false,
            hidden: false,
            thickness: None,
            direction: None,
            left_lollipop: false,
            right_lollipop: false,
        },
        FamilyRelation {
            from: association.clone(),
            to: left,
            arrow: "..".to_string(),
            label: None,
            stereotype: None,
            left_cardinality: None,
            right_cardinality: None,
            left_role: None,
            right_role: None,
            line_color: None,
            dashed: false,
            hidden: false,
            thickness: None,
            direction: None,
            left_lollipop: false,
            right_lollipop: false,
        },
        FamilyRelation {
            from: association,
            to: right,
            arrow: "..".to_string(),
            label: None,
            stereotype: None,
            left_cardinality: None,
            right_cardinality: None,
            left_role: None,
            right_role: None,
            line_color: None,
            dashed: false,
            hidden: false,
            thickness: None,
            direction: None,
            left_lollipop: false,
            right_lollipop: false,
        },
    ]
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
    let lower = line.to_ascii_lowercase();
    // `hide @unlinked` and `remove @unlinked` are component/deployment-specific
    // but may appear before the diagram family is detected (family == None).
    // Handle them before the family gate so they are not misinterpreted.
    if lower == "hide @unlinked" || lower == "remove @unlinked" {
        let is_component_family = matches!(
            family,
            None | Some(DiagramKind::Component | DiagramKind::Deployment)
        );
        if is_component_family {
            let keyword = if lower.starts_with("hide") {
                "hide @unlinked"
            } else {
                "remove @unlinked"
            };
            return Some(StatementKind::HideOption(keyword.to_string()));
        }
    }
    if lower == "hide empty description" && matches!(family, None | Some(DiagramKind::State)) {
        return Some(StatementKind::HideOption(
            "empty description".to_string(),
        ));
    }
    if family.is_none() {
        for keyword in ["hide", "remove", "restore"] {
            let Some(rest) = lower.strip_prefix(&format!("{keyword} ")) else {
                continue;
            };
            let rest = rest.trim();
            if rest == "*" || rest.starts_with('$') {
                return Some(StatementKind::HideOption(format!("{keyword} node {rest}")));
            }
        }
    }
    if !matches!(
        family,
        Some(
            DiagramKind::Class
                | DiagramKind::Object
                | DiagramKind::UseCase
                | DiagramKind::Component
                | DiagramKind::Deployment
        )
    ) {
        return None;
    }
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
        if rest.eq_ignore_ascii_case("@unlinked") {
            // `remove @unlinked` is synonymous with `hide @unlinked` — both drop
            // all nodes that have no relation edges.
            return Some(StatementKind::HideOption("hide @unlinked".to_string()));
        }
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
