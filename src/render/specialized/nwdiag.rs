use super::*;

/// A rendered node bounding box (may appear in multiple network rows).
#[derive(Clone, Copy)]
struct NodeRect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

struct NetworkBar {
    left: i32,
    width: i32,
}

pub fn render_nwdiag_svg(document: &NwdiagDocument) -> String {
    let width = 760;
    let gap = 24;
    let node_height = 28;
    let mut node_columns = Vec::new();
    for node in &document.peer_nodes {
        if !node_columns.iter().any(|name| name == &node.name) {
            node_columns.push(node.name.clone());
        }
    }
    for net in &document.networks {
        for node in &net.nodes {
            if !node_columns.iter().any(|name| name == &node.name) {
                node_columns.push(node.name.clone());
            }
        }
    }

    let mut column_widths = BTreeMap::new();
    for node in &document.peer_nodes {
        let width = node
            .width
            .and_then(|w| i32::try_from(w).ok())
            .unwrap_or(140)
            .clamp(120, 240);
        column_widths
            .entry(node.name.clone())
            .and_modify(|current: &mut i32| *current = (*current).max(width))
            .or_insert(width);
    }
    for net in &document.networks {
        for node in &net.nodes {
            let width = node
                .width
                .and_then(|w| i32::try_from(w).ok())
                .unwrap_or(140)
                .clamp(120, 240);
            column_widths
                .entry(node.name.clone())
                .and_modify(|current: &mut i32| *current = (*current).max(width))
                .or_insert(width);
        }
    }

    let topology_width: i32 = node_columns
        .iter()
        .map(|name| column_widths.get(name).copied().unwrap_or(140))
        .sum::<i32>()
        + gap * node_columns.len().saturating_sub(1) as i32;
    let topology_x = 24 + ((712 - topology_width).max(0) / 2);
    let mut column_x = BTreeMap::new();
    let mut next_x = topology_x;
    for name in &node_columns {
        column_x.insert(name.clone(), next_x);
        next_x += column_widths.get(name).copied().unwrap_or(140) + gap;
    }

    let peer_section_height = if document.peer_nodes.is_empty() {
        0
    } else {
        72
    };
    let network_height = if document.networks.is_empty() {
        24
    } else {
        document.networks.len() as i32 * 102
    };
    let height = 92 + peer_section_height + network_height;
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        width, height, width, height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");

    let mut y = 28;
    out.push_str(&format!(
        "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\">{}</text>",
        y,
        escape_text(document.title.as_deref().unwrap_or("Network diagram"))
    ));
    y += 24;

    if document.networks.is_empty() && document.peer_nodes.is_empty() {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">(no networks)</text>",
            y
        ));
        out.push_str("</svg>");
        return out;
    }

    let mut node_rects: BTreeMap<String, Vec<NodeRect>> = BTreeMap::new();
    let mut peer_rects: BTreeMap<String, NodeRect> = BTreeMap::new();
    let mut network_bars = Vec::new();
    let mut scan_y = y;

    if !document.peer_nodes.is_empty() {
        let peer_y = scan_y + 8;
        for node in &document.peer_nodes {
            let node_width = column_widths.get(&node.name).copied().unwrap_or(140);
            let x = column_x.get(&node.name).copied().unwrap_or(56);
            let rect = NodeRect {
                x,
                y: peer_y,
                w: node_width,
                h: node_height,
            };
            node_rects
                .entry(node.name.clone())
                .or_default()
                .push(NodeRect { ..rect });
            peer_rects.insert(node.name.clone(), rect);
        }
        scan_y += peer_section_height;
    }

    for net in &document.networks {
        let bar = network_bar_bounds(&node_columns, &column_x, &column_widths, Some(net), false);
        network_bars.push(bar);
        let bar_y = scan_y + 24;
        let node_y = bar_y + 30;
        for node in &net.nodes {
            let node_width = column_widths.get(&node.name).copied().unwrap_or(140);
            let x = column_x.get(&node.name).copied().unwrap_or(56);
            node_rects
                .entry(node.name.clone())
                .or_default()
                .push(NodeRect {
                    x,
                    y: node_y,
                    w: node_width,
                    h: node_height,
                });
        }
        scan_y = node_y + 52;
    }

    struct GroupOverlay {
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        color: String,
        style: String,
        label: String,
        shape: String,
    }

    let mut overlays: Vec<GroupOverlay> = Vec::new();
    let group_pad = 8i32;
    for group in &document.groups {
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;
        for member in &group.nodes {
            if let Some(rects) = node_rects.get(member) {
                for rect in rects {
                    min_x = min_x.min(rect.x);
                    min_y = min_y.min(rect.y);
                    max_x = max_x.max(rect.x + rect.w);
                    max_y = max_y.max(rect.y + rect.h);
                }
            }
        }
        if min_x == i32::MAX {
            continue;
        }
        overlays.push(GroupOverlay {
            x: min_x - group_pad,
            y: min_y - group_pad,
            w: (max_x - min_x) + group_pad * 2,
            h: (max_y - min_y) + group_pad * 2,
            color: group.color.clone().unwrap_or_else(|| "#fef3c7".to_string()),
            style: group.style.clone().unwrap_or_else(|| "solid".to_string()),
            label: group.label.clone().unwrap_or_else(|| group.name.clone()),
            shape: group.shape.clone().unwrap_or_else(|| "box".to_string()),
        });
    }

    for overlay in &overlays {
        let dashed = if overlay.style.eq_ignore_ascii_case("dashed") {
            " stroke-dasharray=\"5 3\""
        } else {
            ""
        };
        let radius = if overlay.shape.eq_ignore_ascii_case("roundedbox") {
            12
        } else {
            6
        };
        out.push_str(&format!(
            "<rect class=\"nwdiag-group\" data-nwdiag-style=\"{}\" data-nwdiag-shape=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" fill-opacity=\"0.35\" stroke=\"#d97706\" stroke-width=\"1.5\"{} />",
            escape_text(&overlay.style),
            escape_text(&overlay.shape),
            overlay.x,
            overlay.y,
            overlay.w,
            overlay.h,
            radius,
            radius,
            escape_text(&overlay.color),
            dashed
        ));
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" font-weight=\"600\" fill=\"#92400e\">group {}</text>",
            overlay.x + 4,
            overlay.y + overlay.h - 4,
            escape_text(&overlay.label)
        ));
    }

    for link in &document.peer_links {
        let Some((from, to)) = resolve_peer_link_anchors(&node_rects, &peer_rects, link) else {
            continue;
        };
        out.push_str(&format!(
            "<line class=\"nwdiag-peer-link\" data-nwdiag-from=\"{}\" data-nwdiag-to=\"{}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#475569\" stroke-width=\"2\" />",
            escape_text(&link.from),
            escape_text(&link.to),
            from.0,
            from.1,
            to.0,
            to.1
        ));
    }

    if !document.peer_nodes.is_empty() {
        let peer_y = y + 8;
        for node in &document.peer_nodes {
            render_node(&mut out, node, peer_y, &column_x, &column_widths, false);
        }
        y += peer_section_height;
    }

    for (index, net) in document.networks.iter().enumerate() {
        let bar = &network_bars[index];
        let net_fill = net.color.as_deref().unwrap_or("#e0f2fe");
        let net_style = net.style.as_deref().unwrap_or("solid");
        let net_dash = if net_style.eq_ignore_ascii_case("dashed") {
            " stroke-dasharray=\"5 3\""
        } else {
            ""
        };
        out.push_str(&format!(
            "<rect class=\"nwdiag-network\" data-nwdiag-style=\"{}\" data-nwdiag-shape=\"{}\" data-nwdiag-width-mode=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"22\" fill=\"{}\" stroke=\"#0284c7\" stroke-width=\"1\"{} />",
            escape_text(net_style),
            escape_text(net.shape.as_deref().unwrap_or("swimlane")),
            if net.width_full { "full" } else { "auto" },
            bar.left,
            y,
            bar.width,
            escape_text(net_fill),
            net_dash
        ));
        let net_name = net.label.as_deref().unwrap_or(&net.name);
        let label = match &net.address {
            Some(address) => format!("network {} ({})", net_name, address),
            None => format!("network {}", net_name),
        };
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" fill=\"#0c4a6e\">{}</text>",
            bar.left + 8,
            y + 15,
            escape_text(&label)
        ));
        let bar_y = y + 24;
        out.push_str(&format!(
            "<rect class=\"nwdiag-network\" data-nwdiag-style=\"{}\" data-nwdiag-shape=\"{}\" data-nwdiag-width-mode=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"12\" rx=\"6\" ry=\"6\" fill=\"{}\" stroke=\"#0284c7\" stroke-width=\"1\"{} />",
            escape_text(net_style),
            escape_text(net.shape.as_deref().unwrap_or("swimlane")),
            if net.width_full { "full" } else { "auto" },
            bar.left,
            bar_y,
            bar.width,
            escape_text(net_fill),
            net_dash
        ));
        let node_y = bar_y + 30;
        for node in &net.nodes {
            let node_width = column_widths.get(&node.name).copied().unwrap_or(140);
            let x = column_x.get(&node.name).copied().unwrap_or(56);
            let connector_x = x + (node_width / 2);
            out.push_str(&format!(
                "<line class=\"nwdiag-connector\" data-nwdiag-network=\"{}\" data-nwdiag-node=\"{}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#0284c7\" stroke-width=\"2\"{} />",
                escape_text(&net.name),
                escape_text(&node.name),
                connector_x,
                bar_y + 12,
                connector_x,
                node_y,
                net_dash
            ));
            render_node(&mut out, node, node_y, &column_x, &column_widths, true);
        }
        y = node_y + 52;
    }

    out.push_str("</svg>");
    out
}

fn render_node(
    out: &mut String,
    node: &NwdiagNode,
    node_y: i32,
    column_x: &BTreeMap<String, i32>,
    column_widths: &BTreeMap<String, i32>,
    include_addresses_in_label: bool,
) {
    let node_fill = node.color.as_deref().unwrap_or("white");
    let shape = node.shape.as_deref().unwrap_or("box");
    let style = node.style.as_deref().unwrap_or("solid");
    let dashed = if style.eq_ignore_ascii_case("dashed") {
        " stroke-dasharray=\"5 3\""
    } else {
        ""
    };
    let node_width = column_widths.get(&node.name).copied().unwrap_or(140);
    let radius = if shape.eq_ignore_ascii_case("roundedbox") || shape.eq_ignore_ascii_case("cloud")
    {
        10
    } else {
        3
    };
    let x = column_x.get(&node.name).copied().unwrap_or(56);
    let connector_x = x + (node_width / 2);
    if !node.addresses.is_empty() {
        out.push_str(&format!(
            "<text class=\"nwdiag-address\" x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#334155\">{}</text>",
            connector_x,
            node_y - 8,
            escape_text(&node.addresses.join(", "))
        ));
    }
    out.push_str(&format!(
        "<rect class=\"nwdiag-node\" data-nwdiag-name=\"{}\" data-nwdiag-addresses=\"{}\" data-nwdiag-shape=\"{}\" data-nwdiag-style=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"28\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"#0284c7\" stroke-width=\"1.5\"{}/>",
        escape_text(&node.name),
        escape_text(&node.addresses.join(", ")),
        escape_text(shape),
        escape_text(style),
        x,
        node_y,
        node_width,
        radius,
        radius,
        escape_text(node_fill),
        dashed
    ));
    let display = node.label.as_deref().unwrap_or(&node.name);
    let label = if include_addresses_in_label && !node.addresses.is_empty() {
        format!("{} [{}]", display, node.addresses.join(", "))
    } else {
        display.to_string()
    };
    out.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\" fill=\"#0f172a\">{}</text>",
        x + (node_width / 2),
        node_y + 18,
        escape_text(&label)
    ));
}

fn network_bar_bounds(
    node_columns: &[String],
    column_x: &BTreeMap<String, i32>,
    column_widths: &BTreeMap<String, i32>,
    network: Option<&NwdiagNetwork>,
    fallback_full: bool,
) -> NetworkBar {
    if fallback_full || network.is_some_and(|network| network.width_full) {
        return span_for_nodes(node_columns, column_x, column_widths);
    }
    let names = network
        .map(|network| {
            network
                .nodes
                .iter()
                .map(|node| node.name.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if names.is_empty() {
        return span_for_nodes(node_columns, column_x, column_widths);
    }
    span_for_nodes(&names, column_x, column_widths)
}

fn span_for_nodes(
    names: &[String],
    column_x: &BTreeMap<String, i32>,
    column_widths: &BTreeMap<String, i32>,
) -> NetworkBar {
    let mut left = i32::MAX;
    let mut right = i32::MIN;
    for name in names {
        if let Some(x) = column_x.get(name) {
            let width = column_widths.get(name).copied().unwrap_or(140);
            left = left.min(*x);
            right = right.max(*x + width);
        }
    }
    if left == i32::MAX {
        return NetworkBar {
            left: 24,
            width: 712,
        };
    }
    let padded_left = (left - 12).max(24);
    let padded_right = (right + 12).min(736);
    NetworkBar {
        left: padded_left,
        width: (padded_right - padded_left).max(120),
    }
}

fn resolve_peer_link_anchors(
    node_rects: &BTreeMap<String, Vec<NodeRect>>,
    peer_rects: &BTreeMap<String, NodeRect>,
    link: &NwdiagPeerLink,
) -> Option<((i32, i32), (i32, i32))> {
    let from_rect = preferred_rect(node_rects, peer_rects, &link.from)?;
    let to_rect = preferred_rect(node_rects, peer_rects, &link.to)?;
    let from_center_x = from_rect.x + (from_rect.w / 2);
    let from_center_y = from_rect.y + (from_rect.h / 2);
    let to_center_x = to_rect.x + (to_rect.w / 2);
    let to_center_y = to_rect.y + (to_rect.h / 2);

    let from_x = if from_center_x <= to_center_x {
        from_rect.x + from_rect.w
    } else {
        from_rect.x
    };
    let to_x = if from_center_x <= to_center_x {
        to_rect.x
    } else {
        to_rect.x + to_rect.w
    };

    Some(((from_x, from_center_y), (to_x, to_center_y)))
}

fn preferred_rect<'a>(
    node_rects: &'a BTreeMap<String, Vec<NodeRect>>,
    peer_rects: &'a BTreeMap<String, NodeRect>,
    name: &str,
) -> Option<&'a NodeRect> {
    peer_rects
        .get(name)
        .or_else(|| node_rects.get(name).and_then(|rects| rects.first()))
}
