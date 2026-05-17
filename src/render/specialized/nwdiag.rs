use super::*;

pub fn render_nwdiag_svg(document: &NwdiagDocument) -> String {
    let width = 760;
    let net_rows: i32 = document
        .networks
        .iter()
        .map(|n| 1 + n.nodes.len() as i32)
        .sum();
    let group_rows: i32 = document
        .groups
        .iter()
        .map(|g| 1 + g.nodes.len() as i32)
        .sum();
    let height = 80
        + (net_rows + group_rows).max(1) * 24
        + ((document.networks.len() + document.groups.len()) as i32) * 14;
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
            // Swimlane header
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
                y + 16,
                escape_text(&label)
            ));
            y += 26;
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
                    .unwrap_or(680)
                    .clamp(120, 680);
                let radius = if shape.eq_ignore_ascii_case("roundedbox")
                    || shape.eq_ignore_ascii_case("cloud")
                {
                    10
                } else {
                    3
                };
                out.push_str(&format!(
                    "<rect class=\"nwdiag-node\" data-nwdiag-name=\"{}\" data-nwdiag-addresses=\"{}\" data-nwdiag-shape=\"{}\" data-nwdiag-style=\"{}\" x=\"56\" y=\"{}\" width=\"{}\" height=\"20\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"#0284c7\" stroke-width=\"1\"{}/>",
                    escape_text(&node.name),
                    escape_text(&node.addresses.join(", ")),
                    escape_text(shape),
                    escape_text(style),
                    y,
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
                    "<text x=\"66\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#0f172a\">{}</text>",
                    y + 14,
                    escape_text(&lbl)
                ));
                y += 24;
            }
            y += 10;
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
