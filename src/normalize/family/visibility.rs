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
    // Stereotype-based whole-node removal: `hide <<Foo>>` / `remove <<Foo>>`.
    apply_stereotype_node_removal(nodes, hide_options, &mut removed);
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
        // Collect this node's stereotypes for per-stereotype member filtering.
        let node_stereotypes = node_stereotype_labels(node);
        node.members.retain(|member| {
            class_member_visible_for_node(member, &node_key, &node_stereotypes, hide_options)
        });
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

/// Returns all `<<stereotype>>` labels carried by a node as lowercase strings
/// without the angle brackets (e.g. `"service"` for `<<Service>>`).
pub(super) fn node_stereotype_labels(node: &FamilyNode) -> Vec<String> {
    node.members
        .iter()
        .filter_map(|m| {
            let t = m.text.trim();
            if t.starts_with("<<") && t.ends_with(">>") {
                Some(t[2..t.len() - 2].trim().to_ascii_lowercase())
            } else {
                None
            }
        })
        .collect()
}

/// Detect `hide node <<Foo>>` and `remove node <<Foo>>` options (no member suffix)
/// and add matching nodes to `removed`.
pub(super) fn apply_stereotype_node_removal(
    nodes: &[FamilyNode],
    hide_options: &std::collections::BTreeSet<String>,
    removed: &mut std::collections::BTreeSet<String>,
) {
    // Collect all stereotype-only hide/remove options: "hide node <<Foo>>" or "remove node <<Foo>>"
    let stereotype_hides: Vec<String> = hide_options
        .iter()
        .filter_map(|opt| {
            let rest = opt
                .strip_prefix("hide node ")
                .or_else(|| opt.strip_prefix("remove node "))?;
            let rest = rest.trim();
            // Must be purely a stereotype token with no trailing qualifier
            if rest.starts_with("<<") && rest.ends_with(">>") {
                Some(rest[2..rest.len() - 2].trim().to_ascii_lowercase())
            } else {
                None
            }
        })
        .collect();

    if stereotype_hides.is_empty() {
        return;
    }

    for node in nodes {
        let node_stereos = node_stereotype_labels(node);
        let should_remove = node_stereos.iter().any(|s| stereotype_hides.contains(s));
        if should_remove {
            removed.extend(family_node_match_keys(node));
        }
    }
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
            // Skip stereotype-based hide options — handled by apply_stereotype_node_removal
            // and apply_stereotype_member_controls instead.
            let name_trimmed = name.trim();
            if name_trimmed.starts_with("<<") {
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

/// Check if any of this node's stereotypes triggers a per-stereotype member hide rule.
///
/// Handles:
/// - `hide <<Foo>> members` / `hide node <<Foo>> members`
/// - `hide <<Foo>> methods` / `hide node <<Foo>> methods`
/// - `hide <<Foo>> fields`  / `hide node <<Foo>> fields`
/// - `hide <<Foo>> circle`  / `hide node <<Foo>> circle`
/// - `show <<Foo>> members` / `show <<Foo>> methods` / `show <<Foo>> fields` (override)
fn stereotype_member_hidden(
    member: &ClassMember,
    node_stereotypes: &[String],
    hide_options: &std::collections::BTreeSet<String>,
) -> bool {
    if node_stereotypes.is_empty() {
        return false;
    }
    let text = member.text.trim();
    let kind = member_kind(member);
    let is_circle = text == "()";
    let is_stereotype_text = text.starts_with("<<") && text.ends_with(">>");

    for stereo in node_stereotypes {
        let stereo_token = format!("<<{stereo}>>");
        // `show <<Foo>> members` overrides any hide for normal members.
        let show_members_key = format!("show {stereo_token} members");
        let show_kind_key = format!("show {stereo_token} {kind}");
        if hide_options.contains(&show_members_key) || hide_options.contains(&show_kind_key) {
            return false;
        }
        // `hide <<Foo>> circle` hides the `()` circle marker.
        let hide_circle_key1 = format!("hide {stereo_token} circle");
        let hide_circle_key2 = format!("hide node {stereo_token} circle");
        if is_circle
            && (hide_options.contains(&hide_circle_key1)
                || hide_options.contains(&hide_circle_key2))
        {
            return true;
        }
        // Don't hide stereotype text members via the members/methods/fields rules.
        if is_stereotype_text {
            continue;
        }
        // `hide <<Foo>> members` / `hide node <<Foo>> members`
        let hide_members_key1 = format!("hide {stereo_token} members");
        let hide_members_key2 = format!("hide node {stereo_token} members");
        if hide_options.contains(&hide_members_key1) || hide_options.contains(&hide_members_key2) {
            return true;
        }
        // `hide <<Foo>> methods` / `hide node <<Foo>> methods`
        let hide_methods_key1 = format!("hide {stereo_token} methods");
        let hide_methods_key2 = format!("hide node {stereo_token} methods");
        if kind == "methods"
            && (hide_options.contains(&hide_methods_key1)
                || hide_options.contains(&hide_methods_key2))
        {
            return true;
        }
        // `hide <<Foo>> fields` / `hide node <<Foo>> fields`
        let hide_fields_key1 = format!("hide {stereo_token} fields");
        let hide_fields_key2 = format!("hide node {stereo_token} fields");
        if kind == "fields"
            && (hide_options.contains(&hide_fields_key1)
                || hide_options.contains(&hide_fields_key2))
        {
            return true;
        }
    }
    false
}

pub(super) fn class_member_visible_for_node(
    member: &ClassMember,
    node_key: &str,
    node_stereotypes: &[String],
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
    // Per-stereotype member visibility (e.g. `hide <<Service>> members`).
    if stereotype_member_hidden(member, node_stereotypes, hide_options) {
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
