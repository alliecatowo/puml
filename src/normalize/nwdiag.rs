use super::*;

pub(super) fn normalize_nwdiag_document(document: Document) -> Result<NwdiagDocument, Diagnostic> {
    let (raw, title) = collect_raw_block(&document);
    let mut networks: Vec<NwdiagNetwork> = Vec::new();
    let mut groups: Vec<NwdiagGroup> = Vec::new();
    let mut peer_links: Vec<NwdiagPeerLink> = Vec::new();
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
                network: current.as_ref().map(|network| network.name.clone()),
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
                if let Some(group) = current_group.take() {
                    groups.push(group);
                    current = Some(net);
                } else {
                    networks.push(net);
                }
            } else if let Some(group) = current_group.take() {
                groups.push(group);
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
            let scope = group.network.clone();
            if let Some(links) = parse_nwdiag_peer_links(trimmed, scope.clone()) {
                for link in links {
                    add_group_member(group, &link.from);
                    add_group_member(group, &link.to);
                    if let Some(net) = current.as_mut() {
                        upsert_nwdiag_node(&mut net.nodes, NwdiagNode::named(link.from.clone()));
                        upsert_nwdiag_node(&mut net.nodes, NwdiagNode::named(link.to.clone()));
                    }
                    peer_links.push(link);
                }
                continue;
            }
            for entry in split_nwdiag_entries(trimmed) {
                if let Some(node) = parse_nwdiag_node_entry(&entry) {
                    add_group_member(group, &node.name);
                    if let Some(net) = current.as_mut() {
                        upsert_nwdiag_node(&mut net.nodes, node);
                    }
                }
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
            if let Some(links) = parse_nwdiag_peer_links(trimmed, Some(net.name.clone())) {
                for link in links {
                    upsert_nwdiag_node(&mut net.nodes, NwdiagNode::named(link.from.clone()));
                    upsert_nwdiag_node(&mut net.nodes, NwdiagNode::named(link.to.clone()));
                    peer_links.push(link);
                }
                continue;
            }
            for entry in split_nwdiag_entries(trimmed) {
                if let Some(node) = parse_nwdiag_node_entry(&entry) {
                    upsert_nwdiag_node(&mut net.nodes, node);
                }
            }
            continue;
        }

        let stmt = trimmed.trim_end_matches(';').trim();
        if stmt.is_empty() || stmt.starts_with('}') {
            continue;
        }

        if let Some(links) = parse_nwdiag_peer_links(stmt, None) {
            peer_links.extend(links);
            continue;
        }

        if let Some(node) = parse_nwdiag_node_entry(stmt) {
            if !top_level_nodes
                .iter()
                .any(|existing| existing.name == node.name)
            {
                top_level_nodes.push(node);
            }
        }
    }
    if let Some(net) = current.take() {
        networks.push(net);
    }
    if let Some(group) = current_group.take() {
        groups.push(group);
    }
    top_level_nodes.retain(|node| {
        !networks
            .iter()
            .any(|network| network.nodes.iter().any(|member| member.name == node.name))
    });
    Ok(NwdiagDocument {
        networks,
        groups,
        peer_links,
        top_level_nodes,
        title,
        warnings: Vec::new(),
    })
}

impl NwdiagNode {
    fn named(name: String) -> Self {
        Self {
            name,
            address: None,
            addresses: Vec::new(),
            label: None,
            color: None,
            shape: None,
            style: None,
            width: None,
        }
    }
}

fn upsert_nwdiag_node(nodes: &mut Vec<NwdiagNode>, node: NwdiagNode) {
    if let Some(existing) = nodes.iter_mut().find(|existing| existing.name == node.name) {
        merge_nwdiag_node(existing, node);
    } else {
        nodes.push(node);
    }
}

fn merge_nwdiag_node(existing: &mut NwdiagNode, incoming: NwdiagNode) {
    if incoming.address.is_some() {
        existing.address = incoming.address;
    }
    for address in incoming.addresses {
        if !existing.addresses.iter().any(|current| current == &address) {
            existing.addresses.push(address);
        }
    }
    if incoming.label.is_some() {
        existing.label = incoming.label;
    }
    if incoming.color.is_some() {
        existing.color = incoming.color;
    }
    if incoming.shape.is_some() {
        existing.shape = incoming.shape;
    }
    if incoming.style.is_some() {
        existing.style = incoming.style;
    }
    if incoming.width.is_some() {
        existing.width = incoming.width;
    }
}

fn add_group_member(group: &mut NwdiagGroup, name: &str) {
    if !name.is_empty() && !group.nodes.iter().any(|member| member == name) {
        group.nodes.push(name.to_string());
    }
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

fn parse_nwdiag_peer_links(line: &str, network: Option<String>) -> Option<Vec<NwdiagPeerLink>> {
    let parts = split_nwdiag_peer_chain(line.trim_end_matches(';').trim());
    if parts.len() < 2 {
        return None;
    }
    let mut links = Vec::new();
    for pair in parts.windows(2) {
        let from = peer_link_name(&pair[0]);
        let to = peer_link_name(&pair[1]);
        if !from.is_empty() && !to.is_empty() {
            links.push(NwdiagPeerLink {
                from: from.to_string(),
                to: to.to_string(),
                network: network.clone(),
            });
        }
    }
    (!links.is_empty()).then_some(links)
}

fn split_nwdiag_peer_chain(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut bracket_depth = 0usize;
    let mut ix = 0usize;
    while ix < line.len() {
        let rest = &line[ix..];
        let Some(ch) = rest.chars().next() else {
            break;
        };
        match ch {
            '"' => {
                in_quotes = !in_quotes;
                cur.push(ch);
                ix += ch.len_utf8();
            }
            '[' if !in_quotes => {
                bracket_depth += 1;
                cur.push(ch);
                ix += ch.len_utf8();
            }
            ']' if !in_quotes => {
                bracket_depth = bracket_depth.saturating_sub(1);
                cur.push(ch);
                ix += ch.len_utf8();
            }
            _ if !in_quotes && bracket_depth == 0 => {
                if let Some(token_len) = nwdiag_peer_arrow_len(rest) {
                    let item = cur.trim();
                    if !item.is_empty() {
                        out.push(item.to_string());
                    }
                    cur.clear();
                    ix += token_len;
                } else {
                    cur.push(ch);
                    ix += ch.len_utf8();
                }
            }
            _ => {
                cur.push(ch);
                ix += ch.len_utf8();
            }
        }
    }
    let item = cur.trim();
    if !item.is_empty() {
        out.push(item.to_string());
    }
    out
}

fn nwdiag_peer_arrow_len(s: &str) -> Option<usize> {
    [
        "<-->", "<->", "-->", "<--", "--", "<..>", "..>", "<..", "..",
    ]
    .into_iter()
    .find_map(|token| s.starts_with(token).then_some(token.len()))
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

fn peer_link_name(segment: &str) -> &str {
    segment
        .split_once('[')
        .map(|(name, _)| name)
        .unwrap_or(segment)
        .trim()
        .trim_matches('"')
}
