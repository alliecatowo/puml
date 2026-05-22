use super::*;

pub(super) fn normalize_nwdiag_document(document: Document) -> Result<NwdiagDocument, Diagnostic> {
    let (raw, title) = collect_raw_block(&document);
    let mut networks: Vec<NwdiagNetwork> = Vec::new();
    let mut groups: Vec<NwdiagGroup> = Vec::new();
    let mut peer_links: Vec<(String, String)> = Vec::new();
    let mut top_level_nodes: Vec<NwdiagNode> = Vec::new();
    let mut current: Option<NwdiagNetwork> = None;
    let mut current_group: Option<NwdiagGroup> = None;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed == "nwdiag {"
            || trimmed == "{"
            || trimmed.starts_with('#')
            || trimmed.starts_with("//")
        {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("network ") {
            // close any previous network without explicit `}` (lenient)
            if let Some(net) = current.take() {
                networks.push(net);
            }
            if let Some(group) = current_group.take() {
                groups.push(group);
            }
            let name = rest.trim_end_matches('{').trim().to_string();
            current = Some(NwdiagNetwork {
                name,
                address: None,
                label: None,
                color: None,
                shape: None,
                style: None,
                width_full: false,
                nodes: Vec::new(),
            });
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("group") {
            if let Some(net) = current.take() {
                networks.push(net);
            }
            if let Some(group) = current_group.take() {
                groups.push(group);
            }
            let name = rest
                .trim()
                .trim_end_matches('{')
                .trim()
                .trim_matches('"')
                .to_string();
            current_group = Some(NwdiagGroup {
                name: if name.is_empty() {
                    "group".to_string()
                } else {
                    name
                },
                label: None,
                color: None,
                shape: None,
                style: None,
                nodes: Vec::new(),
            });
            continue;
        }
        if trimmed == "}" {
            if let Some(net) = current.take() {
                networks.push(net);
            } else if let Some(group) = current_group.take() {
                groups.push(group);
            }
            continue;
        }
        if let Some(net) = current.as_mut() {
            // address = "..."
            if let Some(rest) = trimmed.strip_prefix("address") {
                let value = rest
                    .trim_start_matches([' ', '='])
                    .trim()
                    .trim_matches('"')
                    .to_string();
                net.address = Some(value);
                continue;
            }
            if let Some((key, value)) = parse_nwdiag_assignment(trimmed) {
                match key.as_str() {
                    "color" => net.color = Some(value),
                    "description" | "label" => net.label = Some(value),
                    "shape" => net.shape = Some(value),
                    "style" => net.style = Some(value),
                    "width" if value.eq_ignore_ascii_case("full") => net.width_full = true,
                    _ => {}
                }
                continue;
            }
            for entry in split_nwdiag_entries(trimmed) {
                if let Some(node) = parse_nwdiag_node_entry(&entry) {
                    net.nodes.push(node);
                }
            }
            continue;
        }
        if let Some(group) = current_group.as_mut() {
            if let Some((key, value)) = parse_nwdiag_assignment(trimmed) {
                match key.as_str() {
                    "color" => group.color = Some(value),
                    "description" | "label" => group.label = Some(value),
                    "shape" => group.shape = Some(value),
                    "style" => group.style = Some(value),
                    _ => {}
                }
                continue;
            }
            for entry in split_nwdiag_entries(trimmed) {
                let name = entry
                    .split_once('[')
                    .map(|(name, _)| name)
                    .unwrap_or(entry.as_str())
                    .trim()
                    .trim_matches('"');
                if !name.is_empty() && !group.nodes.iter().any(|n| n == name) {
                    group.nodes.push(name.to_string());
                }
            }
            continue;
        }
        // Top-level (outside network/group): parse peer links `A -- B -- C`
        // and top-level node declarations `name [shape = cloud]`.
        // Strip trailing semicolons first.
        let stmt = trimmed.trim_end_matches(';').trim();
        if stmt.contains(" -- ") {
            // Chain: A -- B -- C generates pairs (A,B), (B,C).
            // Also register any new node names from the chain as top-level nodes.
            let parts: Vec<&str> = stmt.split(" -- ").map(str::trim).collect();
            for pair in parts.windows(2) {
                let a = pair[0]
                    .split_once('[')
                    .map(|(n, _)| n)
                    .unwrap_or(pair[0])
                    .trim()
                    .trim_matches('"');
                let b = pair[1]
                    .split_once('[')
                    .map(|(n, _)| n)
                    .unwrap_or(pair[1])
                    .trim()
                    .trim_matches('"');
                if !a.is_empty() && !b.is_empty() {
                    peer_links.push((a.to_string(), b.to_string()));
                }
            }
        } else if !stmt.is_empty() && !stmt.starts_with('}') {
            // Top-level node declaration with optional attrs: `name [shape = cloud]`
            if let Some(node) = parse_nwdiag_node_entry(stmt) {
                // Only add if not already declared inside a network.
                let already_in_network = networks
                    .iter()
                    .any(|net| net.nodes.iter().any(|n| n.name == node.name));
                let already_top_level = top_level_nodes.iter().any(|n| n.name == node.name);
                if !already_in_network && !already_top_level {
                    top_level_nodes.push(node);
                }
            }
        }
    }
    if let Some(net) = current.take() {
        networks.push(net);
    }
    if let Some(group) = current_group.take() {
        groups.push(group);
    }
    Ok(NwdiagDocument {
        networks,
        groups,
        peer_links,
        top_level_nodes,
        title,
        warnings: Vec::new(),
    })
}

fn parse_nwdiag_assignment(line: &str) -> Option<(String, String)> {
    let (key, value) = line.split_once('=')?;
    let key = key.trim().to_ascii_lowercase();
    if key.is_empty() || key.contains(char::is_whitespace) {
        return None;
    }
    Some((key, clean_nwdiag_attr_value(value)))
}

fn clean_nwdiag_attr_value(value: &str) -> String {
    value
        .trim()
        .trim_end_matches(';')
        .trim()
        .trim_matches('"')
        .to_string()
}

fn split_nwdiag_entries(line: &str) -> Vec<String> {
    line.split(';')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(str::to_string)
        .collect()
}

fn parse_nwdiag_node_entry(entry: &str) -> Option<NwdiagNode> {
    let (name_part, attrs) = match entry.split_once('[') {
        Some((n, rest)) => (
            n.trim().trim_matches('"').to_string(),
            Some(rest.trim_end_matches(']')),
        ),
        None => (entry.trim().trim_matches('"').to_string(), None),
    };
    if name_part.is_empty() {
        return None;
    }
    let mut node_address: Option<String> = None;
    let mut label: Option<String> = None;
    let mut color: Option<String> = None;
    let mut shape: Option<String> = None;
    let mut style: Option<String> = None;
    let mut width: Option<u32> = None;
    if let Some(attrs) = attrs {
        for kv in split_nwdiag_attr_args(attrs) {
            if let Some((k, v)) = kv.split_once('=') {
                let key = k.trim().to_ascii_lowercase();
                let value = clean_nwdiag_attr_value(v);
                match key.as_str() {
                    "address" => node_address = Some(value),
                    "description" | "label" => label = Some(value),
                    "color" => color = Some(value),
                    "shape" => shape = Some(value),
                    "style" => style = Some(value),
                    "width" => width = value.parse::<u32>().ok(),
                    _ => {}
                }
            }
        }
    }
    let addresses = parse_nwdiag_addresses(node_address.as_deref());
    Some(NwdiagNode {
        name: name_part,
        address: node_address,
        addresses,
        label,
        color,
        shape,
        style,
        width,
    })
}

fn parse_nwdiag_addresses(address: Option<&str>) -> Vec<String> {
    let Some(address) = address else {
        return Vec::new();
    };
    let trimmed = address.trim();
    let body = trimmed
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(trimmed);
    archimate::split_csv_args(body)
        .into_iter()
        .map(|item| clean_nwdiag_attr_value(&item))
        .filter(|item| !item.is_empty())
        .collect()
}

fn split_nwdiag_attr_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut bracket_depth = 0usize;
    for ch in s.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
                cur.push(ch);
            }
            '[' if !in_quotes => {
                bracket_depth += 1;
                cur.push(ch);
            }
            ']' if !in_quotes => {
                bracket_depth = bracket_depth.saturating_sub(1);
                cur.push(ch);
            }
            ',' if !in_quotes && bracket_depth == 0 => {
                let item = cur.trim();
                if !item.is_empty() {
                    out.push(item.to_string());
                }
                cur.clear();
            }
            _ => cur.push(ch),
        }
    }
    let item = cur.trim();
    if !item.is_empty() {
        out.push(item.to_string());
    }
    out
}
