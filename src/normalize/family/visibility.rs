use super::*;

pub(super) fn apply_class_visibility_controls(
    nodes: &mut Vec<FamilyNode>,
    relations: &mut Vec<ModelFamilyRelation>,
    groups: &mut Vec<FamilyGroup>,
    hide_options: &std::collections::BTreeSet<String>,
) {
    let node_tags: Vec<std::collections::BTreeSet<String>> =
        nodes.iter_mut().map(extract_family_node_tags).collect();

    if hide_options.is_empty() {
        return;
    }

    let mut removed = collect_filtered_node_names(nodes, relations, hide_options);
    apply_family_tag_visibility_controls(nodes, &node_tags, hide_options, &mut removed);
    if !removed.is_empty() {
        nodes.retain(|node| !node_matches_any_filter(node, &removed));
        relations.retain(|rel| {
            !name_matches_any_filter(&relation_node_endpoint(&rel.from), &removed)
                && !name_matches_any_filter(&relation_node_endpoint(&rel.to), &removed)
        });
        for group in groups {
            group
                .member_ids
                .retain(|member_id| !name_matches_any_filter(member_id, &removed));
        }
    }

    for node in nodes {
        let node_key = node
            .alias
            .as_deref()
            .unwrap_or(&node.name)
            .to_ascii_lowercase();
        node.members
            .retain(|member| class_member_visible_for_node(member, &node_key, hide_options));
    }
}

pub(super) fn extract_family_node_tags(
    node: &mut FamilyNode,
) -> std::collections::BTreeSet<String> {
    let mut tags = std::collections::BTreeSet::new();
    node.members.retain(|member| {
        let Some(tag) = member.text.strip_prefix("\x1ffamily:tag:") else {
            return true;
        };
        tags.insert(tag.to_ascii_lowercase());
        false
    });
    tags
}

pub(super) fn apply_family_tag_visibility_controls(
    nodes: &[FamilyNode],
    node_tags: &[std::collections::BTreeSet<String>],
    hide_options: &std::collections::BTreeSet<String>,
    removed: &mut std::collections::BTreeSet<String>,
) {
    if hide_options.contains("hide node *") || hide_options.contains("remove node *") {
        for node in nodes {
            removed.extend(family_node_match_keys(node));
        }
    }

    for (node, tags) in nodes.iter().zip(node_tags.iter()) {
        for tag in tags {
            if hide_options.contains(&format!("hide node {tag}"))
                || hide_options.contains(&format!("remove node {tag}"))
            {
                removed.extend(family_node_match_keys(node));
                break;
            }
        }
    }

    if hide_options.contains("restore node *") {
        removed.clear();
    }

    for (node, tags) in nodes.iter().zip(node_tags.iter()) {
        let restore_tagged = tags
            .iter()
            .any(|tag| hide_options.contains(&format!("restore node {tag}")));
        let restore_named = family_node_match_keys(node)
            .iter()
            .any(|key| hide_options.contains(&format!("restore node {key}")));
        if restore_tagged || restore_named {
            for key in family_node_match_keys(node) {
                removed.remove(&key);
            }
        }
    }
}

pub(super) fn family_node_match_keys(node: &FamilyNode) -> std::collections::BTreeSet<String> {
    let mut keys = std::collections::BTreeSet::from([clean_filter_name(&node.name)]);
    if let Some(alias) = &node.alias {
        keys.insert(clean_filter_name(alias));
    }
    keys
}

pub(super) fn collect_filtered_node_names(
    nodes: &[FamilyNode],
    relations: &[ModelFamilyRelation],
    hide_options: &std::collections::BTreeSet<String>,
) -> std::collections::BTreeSet<String> {
    let mut removed = std::collections::BTreeSet::new();
    for opt in hide_options {
        if let Some(name) = opt
            .strip_prefix("hide node ")
            .or_else(|| opt.strip_prefix("remove node "))
        {
            if name == "*" || name.starts_with('$') {
                continue;
            }
            removed.insert(clean_filter_name(name));
        }
    }
    if hide_options.contains("hide @unlinked") {
        let mut linked = std::collections::BTreeSet::new();
        for rel in relations {
            linked.insert(relation_node_endpoint(&rel.from).to_ascii_lowercase());
            linked.insert(relation_node_endpoint(&rel.to).to_ascii_lowercase());
        }
        for node in nodes {
            let name = node.name.to_ascii_lowercase();
            let alias = node.alias.as_deref().map(str::to_ascii_lowercase);
            if !linked.contains(&name) && alias.as_ref().is_none_or(|a| !linked.contains(a)) {
                removed.insert(name);
            }
        }
    }
    for opt in hide_options {
        if let Some(name) = opt.strip_prefix("restore node ") {
            if name == "*" || name.starts_with('$') {
                continue;
            }
            removed.remove(&clean_filter_name(name));
        }
    }
    removed
}

pub(super) fn class_member_visible_for_node(
    member: &ClassMember,
    node_key: &str,
    hide_options: &std::collections::BTreeSet<String>,
) -> bool {
    let text = member.text.trim();
    if hide_options.contains("stereotype") && text.starts_with("<<") && text.ends_with(">>") {
        return false;
    }
    if hide_options.contains("circle") && text == "()" {
        return false;
    }
    let visibility = member_visibility(text);
    let kind = member_kind(member);
    let show_key = format!("show {node_key} {kind}");
    let show_members_key = format!("show {node_key} members");
    if hide_options.contains(&show_key) || hide_options.contains(&show_members_key) {
        return true;
    }
    if hide_options.contains("members") || hide_options.contains(&format!("{visibility} members")) {
        return false;
    }
    if hide_options.contains(kind) || hide_options.contains(&format!("{visibility} {kind}")) {
        return false;
    }
    true
}

pub(super) fn member_visibility(text: &str) -> &'static str {
    match text.trim_start().chars().next() {
        Some('+') => "public",
        Some('-') => "private",
        Some('#') => "protected",
        Some('~') => "package",
        _ => "public",
    }
}

pub(super) fn member_kind(member: &ClassMember) -> &'static str {
    match member.modifier {
        Some(crate::ast::MemberModifier::Method) => "methods",
        Some(crate::ast::MemberModifier::Field) => "fields",
        _ => {
            let text = member
                .text
                .trim_start_matches(['+', '-', '#', '~'])
                .trim_start();
            if text.contains('(') {
                "methods"
            } else {
                "fields"
            }
        }
    }
}

pub(super) fn node_matches_any_filter(
    node: &FamilyNode,
    filters: &std::collections::BTreeSet<String>,
) -> bool {
    name_matches_any_filter(&node.name, filters)
        || node
            .alias
            .as_deref()
            .is_some_and(|alias| name_matches_any_filter(alias, filters))
}

pub(super) fn name_matches_any_filter(
    name: &str,
    filters: &std::collections::BTreeSet<String>,
) -> bool {
    filters.contains(&clean_filter_name(name))
}

pub(super) fn clean_filter_name(name: &str) -> String {
    name.trim().trim_matches('"').to_ascii_lowercase()
}

pub(super) fn apply_component_visibility_controls(
    nodes: &mut Vec<FamilyNode>,
    relations: &mut Vec<ModelFamilyRelation>,
    hide_options: &std::collections::BTreeSet<String>,
) {
    let node_tags: Vec<std::collections::BTreeSet<String>> =
        nodes.iter_mut().map(extract_component_node_tags).collect();

    if hide_options.is_empty() {
        return;
    }

    let hidden_nodes: std::collections::BTreeSet<String> = nodes
        .iter()
        .zip(node_tags.iter())
        .filter(|(node, tags)| component_node_hidden(node, tags, hide_options))
        .flat_map(|(node, _)| component_node_match_keys(node))
        .collect();
    if !hidden_nodes.is_empty() {
        nodes.retain(|node| !component_node_matches_any(node, &hidden_nodes));
        relations.retain(|rel| {
            !hidden_nodes.contains(&rel.from.to_ascii_lowercase())
                && !hidden_nodes.contains(&rel.to.to_ascii_lowercase())
        });
    }

    // Apply hide/remove @unlinked for component and deployment diagrams.
    // A node is "unlinked" if neither its name nor alias appears in any relation endpoint.
    if hide_options.contains("hide @unlinked") || hide_options.contains("remove @unlinked") {
        let mut linked: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for rel in relations.iter() {
            linked.insert(rel.from.to_ascii_lowercase());
            linked.insert(rel.to.to_ascii_lowercase());
        }
        nodes.retain(|node| {
            let name_lc = node.name.to_ascii_lowercase();
            let alias_lc = node.alias.as_deref().map(str::to_ascii_lowercase);
            linked.contains(&name_lc) || alias_lc.as_ref().is_some_and(|a| linked.contains(a))
        });
    }
}

pub(super) fn extract_component_node_tags(
    node: &mut FamilyNode,
) -> std::collections::BTreeSet<String> {
    let mut tags = std::collections::BTreeSet::new();
    node.members.retain(|member| {
        let Some(tag) = member.text.strip_prefix("\x1fcomponent:tag:") else {
            return true;
        };
        tags.insert(tag.to_ascii_lowercase());
        false
    });
    tags
}

pub(super) fn component_node_hidden(
    node: &FamilyNode,
    tags: &std::collections::BTreeSet<String>,
    hide_options: &std::collections::BTreeSet<String>,
) -> bool {
    let mut hidden = hide_options.contains("hide node *") || hide_options.contains("remove node *");

    for tag in tags {
        let hide_tag = format!("hide node {tag}");
        let remove_tag = format!("remove node {tag}");
        if hide_options.contains(&hide_tag) || hide_options.contains(&remove_tag) {
            hidden = true;
        }
    }

    for key in component_node_match_keys(node) {
        if key.starts_with('$') {
            continue;
        }
        let hide_node = format!("hide node {key}");
        let remove_node = format!("remove node {key}");
        if hide_options.contains(&hide_node) || hide_options.contains(&remove_node) {
            hidden = true;
        }
    }

    if hide_options.contains("restore node *") {
        hidden = false;
    }
    for tag in tags {
        if hide_options.contains(&format!("restore node {tag}")) {
            hidden = false;
        }
    }
    for key in component_node_match_keys(node) {
        if key.starts_with('$') {
            continue;
        }
        if hide_options.contains(&format!("restore node {key}")) {
            hidden = false;
        }
    }

    hidden
}

pub(super) fn component_node_match_keys(node: &FamilyNode) -> std::collections::BTreeSet<String> {
    let mut keys = std::collections::BTreeSet::from([node.name.to_ascii_lowercase()]);
    if let Some(alias) = &node.alias {
        keys.insert(alias.to_ascii_lowercase());
    }
    keys
}

pub(super) fn component_node_matches_any(
    node: &FamilyNode,
    keys: &std::collections::BTreeSet<String>,
) -> bool {
    component_node_match_keys(node)
        .iter()
        .any(|key| keys.contains(key))
}
