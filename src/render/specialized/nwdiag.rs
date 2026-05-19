use super::*;

/// A rendered node bounding box (may appear in multiple network rows).
struct NodeRect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

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
            let w = node
                .width
                .and_then(|w| i32::try_from(w).ok())
                .unwrap_or(140)
                .clamp(120, 240);
            column_widths
                .entry(node.name.clone())
                .and_modify(|current: &mut i32| *current = (*current).max(w))
                .or_insert(w);
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
    let network_height = if document.networks.is_empty() {
        24
    } else {
        net_rows * 102
    };
    // Groups now overlay the topology — no extra rows needed below.
    let height = 92 + network_height;
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
        // ── Pass 1: collect node rects so we can compute group overlays ──────────
        // Map: node_name → list of NodeRect (one per network row where it appears)
        let mut node_rects: BTreeMap<String, Vec<NodeRect>> = BTreeMap::new();
        {
            let mut scan_y = y;
            for net in &document.networks {
                let bar_y = scan_y + 24;
                let node_y = bar_y + 30;
                for node in &net.nodes {
                    let node_width = node
                        .width
                        .and_then(|w| i32::try_from(w).ok())
                        .unwrap_or(140)
                        .clamp(120, 240);
                    let x = column_x.get(&node.name).copied().unwrap_or(56);
                    node_rects
                        .entry(node.name.clone())
                        .or_default()
                        .push(NodeRect {
                            x,
                            y: node_y,
                            w: node_width,
                            h: 28,
                        });
                }
                scan_y = node_y + 52;
            }
        }

        // ── Compute group overlay bounding boxes ─────────────────────────────────
        // Bounding box = union over all rects of all member nodes, with padding.
        let group_pad = 8i32;
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
        for group in &document.groups {
            let mut min_x = i32::MAX;
            let mut min_y = i32::MAX;
            let mut max_x = i32::MIN;
            let mut max_y = i32::MIN;
            for member in &group.nodes {
                if let Some(rects) = node_rects.get(member) {
                    for r in rects {
                        min_x = min_x.min(r.x);
                        min_y = min_y.min(r.y);
                        max_x = max_x.max(r.x + r.w);
                        max_y = max_y.max(r.y + r.h);
                    }
                }
            }
            if min_x == i32::MAX {
                // No known member positions — skip overlay for this group.
                continue;
            }
            let ox = min_x - group_pad;
            let oy = min_y - group_pad;
            let ow = (max_x - min_x) + group_pad * 2;
            let oh = (max_y - min_y) + group_pad * 2;
            overlays.push(GroupOverlay {
                x: ox,
                y: oy,
                w: ow,
                h: oh,
                color: group.color.clone().unwrap_or_else(|| "#fef3c7".to_string()),
                style: group.style.clone().unwrap_or_else(|| "solid".to_string()),
                label: group.label.clone().unwrap_or_else(|| group.name.clone()),
                shape: group.shape.clone().unwrap_or_else(|| "box".to_string()),
            });
        }

        // ── Emit group overlays BEFORE nodes so they sit behind them ─────────────
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
            // Group label: float at bottom-left inside the overlay so it
            // doesn't overlap network bar headers that sit above the nodes.
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" font-weight=\"600\" fill=\"#92400e\">group {}</text>",
                overlay.x + 4,
                overlay.y + overlay.h - 4,
                escape_text(&overlay.label)
            ));
        }

        // ── Pass 2: emit networks and nodes on top of overlays ───────────────────
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
    }
    out.push_str("</svg>");
    out
}
