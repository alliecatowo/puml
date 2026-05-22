fn parse_component_scoping_block(
    lines: &[(&str, Span)],
    start: usize,
    line: &str,
) -> Result<Option<(StatementKind, usize)>, Diagnostic> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    let Some((kind, label_raw)) = component_scoping_block_head(trimmed, &lower) else {
        return Ok(None);
    };
    if !trimmed.ends_with('{') {
        return Ok(None);
    }
    let end_idx = find_scoping_block_end(lines, start);
    if end_idx == start {
        return Err(Diagnostic::error(format!(
            "[E_COMPONENT_GROUP_UNCLOSED] unclosed `{kind}` block: missing `}}`",
        ))
        .with_span(lines[start].1));
    }
    if matches!(kind, "namespace" | "package") {
        let has_component_family = group_body_contains_component_family(lines, start, end_idx);
        if !has_component_family
            || group_body_contains_object_family(lines, start, end_idx)
            || group_body_contains_usecase_family(lines, start, end_idx)
        {
            return Ok(None);
        }
    }
    // rectangle blocks that contain usecase/actor content belong to usecase diagrams,
    // not component diagrams — let the class/usecase scoping-block parser handle them.
    if kind == "rectangle" && group_body_contains_usecase_family(lines, start, end_idx) {
        return Ok(None);
    }
    if kind == "namespace" && !group_body_contains_component_family(lines, start, end_idx) {
        return Ok(None);
    }
    let label = clean_ident(label_raw.trim_end_matches('{').trim().trim_matches('"'));
    let content =
        collect_scoped_component_group_content(lines, start, end_idx, std::slice::from_ref(&label));
    Ok(Some((
        StatementKind::ClassGroup {
            kind: kind.to_string(),
            label: if label.is_empty() { None } else { Some(label) },
            members: content.members,
            relations: content.relations,
        },
        end_idx,
    )))
}

fn collect_scoped_component_group_content(
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
            parse_family_relation(line, Some(DiagramKind::Component))
        {
            content.relations.push(qualify_scoped_relation(rel, scope));
            idx += 1;
            continue;
        }
        if line.trim_end().ends_with('{') {
            if let Some((keyword, _)) = component_scoping_block_head(line, &lower) {
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
                    collect_scoped_component_group_content(lines, idx, nested_end, &nested_scope);
                content.members.extend(nested.members);
                content.relations.extend(nested.relations);
                idx = nested_end + 1;
                continue;
            }
            }
        }
        if let Some(StatementKind::ComponentDecl {
            kind,
            name,
            alias,
            label,
            members,
            ..
        }) = parse_component_decl(line)
        {
            let fill_color = members.iter().find_map(|member| {
                member
                    .text
                    .strip_prefix("\x1fstyle:fill:")
                    .map(str::to_string)
            });
            let local_id = alias.clone().unwrap_or_else(|| name.clone());
            let scoped_id = qualify_scoped_identifier(local_id, scope);
            let display = label
                .or_else(|| alias.as_ref().map(|_| name.clone()))
                .or_else(|| (scoped_id != name).then(|| name.clone()))
                .filter(|value| value != &scoped_id);
            let display = append_component_declaration_metadata(display, &members);
            let mut encoded = scoped_id;
            if let Some(display) = display {
                encoded.push('\t');
                encoded.push_str(&display);
            }
            encoded.push('\t');
            encoded.push_str(component_decl_kind_name(kind));
            if let Some(fill_color) = fill_color {
                encoded.push('\t');
                encoded.push_str(&format!("\x1fstyle:fill:{fill_color}"));
            }
            content.members.push(encoded);
        } else {
            let name = extract_component_group_member_name(line);
            if !name.is_empty() {
                content.members.push(qualify_scoped_identifier(name, scope));
            }
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

fn component_decl_kind_name(kind: ComponentNodeKind) -> &'static str {
    match kind {
        ComponentNodeKind::Action => "action",
        ComponentNodeKind::Agent => "agent",
        ComponentNodeKind::Component => "component",
        ComponentNodeKind::Interface => "interface",
        ComponentNodeKind::Port => "port",
        ComponentNodeKind::Node => "node",
        ComponentNodeKind::Artifact => "artifact",
        ComponentNodeKind::Boundary => "boundary",
        ComponentNodeKind::Cloud => "cloud",
        ComponentNodeKind::Circle => "circle",
        ComponentNodeKind::Collections => "collections",
        ComponentNodeKind::Frame => "frame",
        ComponentNodeKind::Storage => "storage",
        ComponentNodeKind::Container => "container",
        ComponentNodeKind::Control => "control",
        ComponentNodeKind::Database => "database",
        ComponentNodeKind::Entity => "entity",
        ComponentNodeKind::Package => "package",
        ComponentNodeKind::Rectangle => "rectangle",
        ComponentNodeKind::Folder => "folder",
        ComponentNodeKind::File => "file",
        ComponentNodeKind::Card => "card",
        ComponentNodeKind::Actor => "actor",
        ComponentNodeKind::Hexagon => "hexagon",
        ComponentNodeKind::Label => "label",
        ComponentNodeKind::Person => "person",
        ComponentNodeKind::Process => "process",
        ComponentNodeKind::Queue => "queue",
        ComponentNodeKind::Stack => "stack",
        ComponentNodeKind::UseCase => "usecase",
    }
}

fn component_scoping_block_head<'a>(
    trimmed: &'a str,
    lower: &str,
) -> Option<(&'static str, &'a str)> {
    if lower.starts_with("namespace ") {
        return Some(("namespace", trimmed.strip_prefix("namespace ").unwrap_or("").trim()));
    }
    component_decl_keywords()
        .iter()
        .map(|(keyword, _)| *keyword)
        .filter(|keyword| is_component_container_keyword(keyword))
        .find_map(|keyword| {
            lower.starts_with(&format!("{keyword} ")).then(|| {
                (
                    keyword,
                    trimmed
                        .get(keyword.len()..)
                        .unwrap_or_default()
                        .trim_start(),
                )
            })
        })
}

fn append_component_declaration_metadata(
    display: Option<String>,
    members: &[ClassMember],
) -> Option<String> {
    let stereotypes = members
        .iter()
        .map(|member| member.text.trim())
        .filter(|text| text.starts_with("<<") && text.ends_with(">>"))
        .collect::<Vec<_>>();
    if stereotypes.is_empty() {
        return display;
    }
    let mut label = display.unwrap_or_default();
    if !label.is_empty() {
        label.push(' ');
    }
    label.push_str(&stereotypes.join(" "));
    Some(label)
}

fn find_scoping_block_end(lines: &[(&str, Span)], start: usize) -> usize {
    let mut depth = 0usize;
    for (idx, (raw, _)) in lines.iter().enumerate().skip(start) {
        let trimmed = strip_inline_plantuml_comment(raw).trim();
        if trimmed.ends_with('{') {
            depth += 1;
        }
        if trimmed == "}" {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return idx;
            }
        }
    }
    start
}
