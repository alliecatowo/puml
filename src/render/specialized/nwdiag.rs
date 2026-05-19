use super::*;

pub fn render_nwdiag_svg(document: &NwdiagDocument) -> String {
    let width = 760;
    let mut node_columns = Vec::new();
    for net in &document.networks {
        for node in &net.nodes {
            if !node_columns.iter().any(|name| name == &node.name) {
                node_columns.push(node.name.clone());
            }
        }
    }
    let mut column_widths = BTreeMap::new();
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
    let gap = 24;
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
    let net_rows: i32 = document.networks.len() as i32;
    let group_rows: i32 = document
        .groups
        .iter()
        .map(|g| 1 + g.nodes.len() as i32)
        .sum();
    let network_height = if document.networks.is_empty() {
        24
    } else {
        net_rows * 102
    };
    let height = 92 + network_height + group_rows.max(1) * 24 + (document.groups.len() as i32) * 36;
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
    if document.networks.is_empty() {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">(no networks)</text>",
            y
        ));
    } else {
        for net in &document.networks {
            let net_fill = net.color.as_deref().unwrap_or("#e0f2fe");
            let net_style = net.style.as_deref().unwrap_or("solid");
            let net_dash = if net_style.eq_ignore_ascii_case("dashed") {
                " stroke-dasharray=\"5 3\""
            } else {
                ""
            };
            out.push_str(&format!(
                "<rect class=\"nwdiag-network\" data-nwdiag-style=\"{}\" data-nwdiag-shape=\"{}\" x=\"24\" y=\"{}\" width=\"712\" height=\"22\" fill=\"{}\" stroke=\"#0284c7\" stroke-width=\"1\"{} />",
                escape_text(net_style),
                escape_text(net.shape.as_deref().unwrap_or("swimlane")),
                y,
                escape_text(net_fill),
                net_dash
            ));
            let net_name = net.label.as_deref().unwrap_or(&net.name);
            let label = match &net.address {
                Some(a) => format!("network {} ({})", net_name, a),
                None => format!("network {}", net_name),
            };
            out.push_str(&format!(
                "<text x=\"32\" y=\"{}\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" fill=\"#0c4a6e\">{}</text>",
                y + 15,
                escape_text(&label)
            ));
            let bar_y = y + 24;
            out.push_str(&format!(
                "<rect class=\"nwdiag-network\" data-nwdiag-style=\"{}\" data-nwdiag-shape=\"{}\" x=\"24\" y=\"{}\" width=\"712\" height=\"12\" rx=\"6\" ry=\"6\" fill=\"{}\" stroke=\"#0284c7\" stroke-width=\"1\"{} />",
                escape_text(net_style),
                escape_text(net.shape.as_deref().unwrap_or("swimlane")),
                bar_y,
                escape_text(net_fill),
                net_dash
            ));
            let node_y = bar_y + 30;
            for node in &net.nodes {
                let node_fill = node.color.as_deref().unwrap_or("white");
                let shape = node.shape.as_deref().unwrap_or("box");
                let style = node.style.as_deref().unwrap_or("solid");
                let dashed = if style.eq_ignore_ascii_case("dashed") {
                    " stroke-dasharray=\"5 3\""
                } else {
                    ""
                };
                let node_width = node
                    .width
                    .and_then(|w| i32::try_from(w).ok())
                    .unwrap_or(140)
                    .clamp(120, 240);
                let radius = if shape.eq_ignore_ascii_case("roundedbox")
                    || shape.eq_ignore_ascii_case("cloud")
                {
                    10
                } else {
                    3
                };
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
                    dashed
                ));
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
                let lbl = if node.addresses.is_empty() {
                    display.to_string()
                } else {
                    format!("{} [{}]", display, node.addresses.join(", "))
                };
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\" fill=\"#0f172a\">{}</text>",
                    x + (node_width / 2),
                    node_y + 18,
                    escape_text(&lbl)
                ));
            }
            y = node_y + 52;
        }
        for group in &document.groups {
            let fill = group.color.as_deref().unwrap_or("#fef3c7");
            let style = group.style.as_deref().unwrap_or("solid");
            let dashed = if style.eq_ignore_ascii_case("dashed") {
                " stroke-dasharray=\"5 3\""
            } else {
                ""
            };
            out.push_str(&format!(
                "<rect class=\"nwdiag-group\" data-nwdiag-style=\"{}\" data-nwdiag-shape=\"{}\" x=\"24\" y=\"{}\" width=\"712\" height=\"22\" fill=\"{}\" stroke=\"#d97706\" stroke-width=\"1\"{} />",
                escape_text(style),
                escape_text(group.shape.as_deref().unwrap_or("box")),
                y,
                escape_text(fill),
                dashed
            ));
            let group_label = group.label.as_deref().unwrap_or(&group.name);
            out.push_str(&format!(
                "<text x=\"32\" y=\"{}\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" fill=\"#78350f\">group {}</text>",
                y + 16,
                escape_text(group_label)
            ));
            y += 26;
            for node in &group.nodes {
                out.push_str(&format!(
                    "<rect class=\"nwdiag-group-member\" x=\"56\" y=\"{}\" width=\"680\" height=\"20\" rx=\"3\" ry=\"3\" fill=\"#fff7ed\" stroke=\"#f59e0b\" stroke-width=\"1\"/>",
                    y
                ));
                out.push_str(&format!(
                    "<text x=\"66\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#0f172a\">{}</text>",
                    y + 14,
                    escape_text(node)
                ));
                y += 24;
            }
            y += 10;
        }
    }
    out.push_str("</svg>");
    out
}
