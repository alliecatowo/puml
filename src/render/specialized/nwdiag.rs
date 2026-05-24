use std::collections::BTreeSet;

use super::*;
use crate::creole::tokenize_creole;
use crate::model::{NwdiagNetwork, NwdiagNode};
use crate::render::text_metrics::rounded_proportional_monospace_width;

/// A rendered node bounding box (may appear in multiple network rows).
struct NodeRect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

struct BoxSpan {
    x: i32,
    w: i32,
}

struct SharedNodeSpan {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    addresses: Vec<String>,
    label: String,
    color: Option<String>,
    shape: Option<String>,
    style: Option<String>,
}

pub fn render_nwdiag_svg(document: &NwdiagDocument) -> String {
    let mut node_columns = Vec::new();
    for net in &document.networks {
        for node in &net.nodes {
            if !node_columns.iter().any(|name| name == &node.name) {
                node_columns.push(node.name.clone());
            }
        }
    }
    for node in &document.top_level_nodes {
        if !node_columns.iter().any(|name| name == &node.name) {
            node_columns.push(node.name.clone());
        }
    }
    for (from, to) in &document.peer_links {
        if !node_columns.iter().any(|name| name == from) {
            node_columns.push(from.clone());
        }
        if !node_columns.iter().any(|name| name == to) {
            node_columns.push(to.clone());
        }
    }
    let mut column_widths = BTreeMap::new();
    for net in &document.networks {
        for node in &net.nodes {
            let w = node_width(node);
            column_widths
                .entry(node.name.clone())
                .and_modify(|current: &mut i32| *current = (*current).max(w))
                .or_insert(w);
        }
    }
    for node in &document.top_level_nodes {
        let w = node_width(node);
        column_widths
            .entry(node.name.clone())
            .and_modify(|current: &mut i32| *current = (*current).max(w))
            .or_insert(w);
    }
    let gap = 24;
    let topology_width: i32 = node_columns
        .iter()
        .map(|name| column_widths.get(name).copied().unwrap_or(140))
        .sum::<i32>()
        + gap * node_columns.len().saturating_sub(1) as i32;
    let width = (topology_width + 48).max(760);
    let inner_width = width - 48;
    let topology_x = 24 + ((inner_width - topology_width).max(0) / 2);
    let mut column_x = BTreeMap::new();
    let mut next_x = topology_x;
    for name in &node_columns {
        column_x.insert(name.clone(), next_x);
        next_x += column_widths.get(name).copied().unwrap_or(140) + gap;
    }
    let mut rendered_top_level_names = Vec::new();
    for node in &document.top_level_nodes {
        rendered_top_level_names.push(node.name.clone());
    }
    for (from, to) in &document.peer_links {
        for name in [from, to] {
            if !node_is_in_network(document, name)
                && !rendered_top_level_names
                    .iter()
                    .any(|existing| existing == name)
            {
                rendered_top_level_names.push(name.clone());
            }
        }
    }
    let network_height = if document.networks.is_empty() {
        24
    } else {
        document.networks.iter().map(network_row_step).sum()
    };
    let top_level_row_height = if rendered_top_level_names.is_empty() {
        0
    } else {
        52
    };
    let height = 92 + network_height + top_level_row_height;
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
    let shared_node_spans = shared_node_spans(document, &column_x, &column_widths, y);

    // ── Pass 1: collect node rects so we can compute group overlays ──────────
    let mut node_rects: BTreeMap<String, Vec<NodeRect>> = BTreeMap::new();
    let top_level_node_y = {
        let mut scan_y = y;
        for net in &document.networks {
            let bar_y = scan_y + 24;
            let node_y = bar_y + 30;
            for node in &net.nodes {
                if shared_node_spans.contains_key(&node.name) {
                    continue;
                }
                let x = column_x.get(&node.name).copied().unwrap_or(56);
                let h = node_height(node);
                node_rects
                    .entry(node.name.clone())
                    .or_default()
                    .push(NodeRect {
                        x,
                        y: node_y,
                        w: node_width(node),
                        h,
                    });
            }
            scan_y = node_y + network_after_node_gap(net);
        }
        let top_level_node_y = scan_y + 8;
        for name in &rendered_top_level_names {
            let x = column_x.get(name).copied().unwrap_or(56);
            let width = document
                .top_level_nodes
                .iter()
                .find(|node| &node.name == name)
                .map(node_width)
                .or_else(|| column_widths.get(name).copied())
                .unwrap_or(140);
            let height = document
                .top_level_nodes
                .iter()
                .find(|node| &node.name == name)
                .map(node_height)
                .unwrap_or(28);
            node_rects.entry(name.clone()).or_default().push(NodeRect {
                x,
                y: top_level_node_y,
                w: width,
                h: height,
            });
        }
        top_level_node_y
    };
    for (name, span) in &shared_node_spans {
        node_rects.entry(name.clone()).or_default().push(NodeRect {
            x: span.x,
            y: span.y,
            w: span.w,
            h: span.h,
        });
    }

    // ── Compute group overlay bounding boxes ─────────────────────────────────
    let group_pad_x = 8i32;
    let group_pad_bottom = 8i32;
    let group_header_height = 30i32;
    struct GroupOverlay {
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        color: String,
        style: String,
        label: String,
        shape: String,
        connector_xs: Vec<i32>,
    }
    let mut overlays: Vec<GroupOverlay> = Vec::new();
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
        let label = group.label.clone().unwrap_or_else(|| group.name.clone());
        let label_width = text_width(&format!("group {label}"), 10) + 12;
        let base_x = min_x - group_pad_x;
        let base_width = (max_x - min_x) + group_pad_x * 2;
        let span = expand_span_to_fit(base_x, base_width, label_width, 24, inner_width);
        let mut connector_xs = BTreeSet::new();
        for member in &group.nodes {
            if let Some(rects) = node_rects.get(member) {
                for rect in rects {
                    connector_xs.insert(rect.x + (rect.w / 2));
                }
            }
        }
        overlays.push(GroupOverlay {
            x: span.x,
            y: min_y - group_header_height,
            w: span.w,
            h: (max_y - min_y) + group_header_height + group_pad_bottom,
            color: group.color.clone().unwrap_or_else(|| "#fef3c7".to_string()),
            style: group.style.clone().unwrap_or_else(|| "solid".to_string()),
            label,
            shape: group.shape.clone().unwrap_or_else(|| "box".to_string()),
            connector_xs: connector_xs.into_iter().collect(),
        });
    }

    for overlay in &overlays {
        let dash = nwdiag_stroke_dash(&overlay.style);
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
            dash
        ));
    }

    if document.networks.is_empty() && rendered_top_level_names.is_empty() {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">(no networks)</text>",
            y
        ));
    }

    for net in &document.networks {
        let net_fill = net.color.as_deref().unwrap_or("#e0f2fe");
        let net_style = net.style.as_deref().unwrap_or("solid");
        let net_dash = nwdiag_stroke_dash(net_style);
        let (network_x, network_width) = network_geometry(net, &column_x, inner_width);
        out.push_str(&format!(
            "<rect class=\"nwdiag-network\" data-nwdiag-style=\"{}\" data-nwdiag-shape=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"22\" fill=\"{}\" stroke=\"#0284c7\" stroke-width=\"1\"{} />",
            escape_text(net_style),
            escape_text(net.shape.as_deref().unwrap_or("swimlane")),
            network_x,
            y,
            network_width,
            escape_text(net_fill),
            net_dash
        ));
        let label = network_label(net);
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" fill=\"#0c4a6e\">{}</text>",
            network_x + 8,
            y + 15,
            escape_text(&label)
        ));
        let bar_y = y + 24;
        out.push_str(&format!(
            "<rect class=\"nwdiag-network\" data-nwdiag-style=\"{}\" data-nwdiag-shape=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"12\" rx=\"6\" ry=\"6\" fill=\"{}\" stroke=\"#0284c7\" stroke-width=\"1\"{} />",
            escape_text(net_style),
            escape_text(net.shape.as_deref().unwrap_or("swimlane")),
            network_x,
            bar_y,
            network_width,
            escape_text(net_fill),
            net_dash
        ));
        let node_y = bar_y + 30;
        for node in &net.nodes {
            let shared_span = shared_node_spans.get(&node.name);
            let node_fill = shared_span
                .and_then(|span| span.color.as_deref())
                .or(node.color.as_deref())
                .unwrap_or("white");
            let shape = shared_span
                .and_then(|span| span.shape.as_deref())
                .or(node.shape.as_deref())
                .unwrap_or("box");
            let style = shared_span
                .and_then(|span| span.style.as_deref())
                .or(node.style.as_deref())
                .unwrap_or("solid");
            let dash = nwdiag_stroke_dash(style);
            let node_width = shared_span
                .map(|span| span.w)
                .unwrap_or_else(|| node_width(node));
            let radius = if shape.eq_ignore_ascii_case("roundedbox")
                || shape.eq_ignore_ascii_case("cloud")
            {
                10
            } else {
                3
            };
            let x = shared_span
                .map(|span| span.x)
                .unwrap_or_else(|| column_x.get(&node.name).copied().unwrap_or(56));
            let connector_x = x + (node_width / 2);
            let connector_end_y = match shared_span {
                Some(span) if node_y != span.y => bar_y + 12,
                _ => node_y,
            };
            out.push_str(&format!(
                "<line class=\"nwdiag-connector\" data-nwdiag-network=\"{}\" data-nwdiag-node=\"{}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#0284c7\" stroke-width=\"2\"{} />",
                escape_text(&net.name),
                escape_text(&node.name),
                connector_x,
                bar_y + 12,
                connector_x,
                connector_end_y,
                dash
            ));
            if !node.addresses.is_empty() {
                out.push_str(&format!(
                    "<text class=\"nwdiag-address\" x=\"{}\" y=\"{}\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"11\" fill=\"#334155\">{}</text>",
                    connector_x - 6,
                    node_y - 8,
                    escape_text(&node.addresses.join(", "))
                ));
            }
            if let Some(span) = shared_span {
                if node_y != span.y {
                    continue;
                }
                if span.h > 28 {
                    out.push_str(&format!(
                        "<line class=\"nwdiag-jump-line\" data-nwdiag-node=\"{}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#0284c7\" stroke-width=\"2\" stroke-dasharray=\"3 3\" />",
                        escape_text(&node.name),
                        connector_x,
                        node_y + node_height(node),
                        connector_x,
                        span.y + span.h
                    ));
                }
            }
            let data_addresses = shared_span
                .map(|span| span.addresses.join(", "))
                .unwrap_or_else(|| node.addresses.join(", "));
            let shared_class = if shared_span.is_some() {
                " nwdiag-shared-node"
            } else {
                ""
            };
            let render_height = shared_span
                .map(|span| span.h)
                .unwrap_or_else(|| node_height(node));
            out.push_str(&format!(
                "<rect class=\"nwdiag-node{}\" data-nwdiag-name=\"{}\" data-nwdiag-addresses=\"{}\" data-nwdiag-shape=\"{}\" data-nwdiag-style=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"#0284c7\" stroke-width=\"1.5\"{}/>",
                shared_class,
                escape_text(&node.name),
                escape_text(&data_addresses),
                escape_text(shape),
                escape_text(style),
                x,
                node_y,
                node_width,
                render_height,
                radius,
                radius,
                escape_text(node_fill),
                dash
            ));
            let label = node_render_label(node, shared_span);
            render_nwdiag_node_label(&mut out, x, node_y, node_width, &label);
        }
        y = node_y + network_after_node_gap(net);
    }

    let mut rendered_stub_names = BTreeSet::new();
    for node in &document.top_level_nodes {
        let node_fill = node.color.as_deref().unwrap_or("#f1f5f9");
        let shape = node.shape.as_deref().unwrap_or("box");
        let style = node.style.as_deref().unwrap_or("solid");
        let dash = nwdiag_stroke_dash(style);
        let node_width = node_width(node);
        let node_height = node_height(node);
        let radius =
            if shape.eq_ignore_ascii_case("roundedbox") || shape.eq_ignore_ascii_case("cloud") {
                10
            } else {
                3
            };
        let x = column_x.get(&node.name).copied().unwrap_or(56);
        out.push_str(&format!(
            "<rect class=\"nwdiag-node nwdiag-toplevel\" data-nwdiag-name=\"{}\" data-nwdiag-addresses=\"{}\" data-nwdiag-shape=\"{}\" data-nwdiag-style=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"#475569\" stroke-width=\"1.5\"{}/>",
            escape_text(&node.name),
            escape_text(&node.addresses.join(", ")),
            escape_text(shape),
            escape_text(style),
            x,
            top_level_node_y,
            node_width,
            node_height,
            radius,
            radius,
            escape_text(node_fill),
            dash
        ));
        let label = node_render_label(node, None);
        render_nwdiag_node_label(&mut out, x, top_level_node_y, node_width, &label);
        rendered_stub_names.insert(node.name.clone());
    }
    for name in &rendered_top_level_names {
        if rendered_stub_names.contains(name) {
            continue;
        }
        let x = column_x.get(name).copied().unwrap_or(56);
        let width = column_widths.get(name).copied().unwrap_or(140);
        out.push_str(&format!(
            "<rect class=\"nwdiag-node nwdiag-toplevel\" data-nwdiag-name=\"{}\" data-nwdiag-addresses=\"\" data-nwdiag-shape=\"box\" data-nwdiag-style=\"solid\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"28\" rx=\"3\" ry=\"3\" fill=\"#f1f5f9\" stroke=\"#475569\" stroke-width=\"1.5\"/>",
            escape_text(name),
            x,
            top_level_node_y,
            width
        ));
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\" fill=\"#0f172a\">{}</text>",
            x + (width / 2),
            top_level_node_y + 18,
            escape_text(name)
        ));
    }

    for (from, to) in &document.peer_links {
        let Some(from_rect) = node_rects.get(from).and_then(|rects| rects.first()) else {
            continue;
        };
        let Some(to_rect) = node_rects.get(to).and_then(|rects| rects.first()) else {
            continue;
        };
        let from_x = from_rect.x + (from_rect.w / 2);
        let to_x = to_rect.x + (to_rect.w / 2);
        let from_y = from_rect.y + from_rect.h;
        let to_y = to_rect.y + to_rect.h;
        let link_y = from_y.max(to_y) + 18;
        out.push_str(&format!(
            "<path class=\"nwdiag-peer-link\" data-nwdiag-peer-a=\"{}\" data-nwdiag-peer-b=\"{}\" d=\"M {} {} L {} {} L {} {} L {} {}\" fill=\"none\" stroke=\"#475569\" stroke-width=\"2\" stroke-dasharray=\"4 2\" />",
            escape_text(from),
            escape_text(to),
            from_x,
            from_y,
            from_x,
            link_y,
            to_x,
            link_y,
            to_x,
            to_y
        ));
    }
    for overlay in &overlays {
        let label_text = format!("group {}", overlay.label);
        let label_width = text_width(&label_text, 10);
        let label_x = label_chip_x(overlay.x, overlay.w, label_width, &overlay.connector_xs);
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" font-weight=\"600\" fill=\"#92400e\">group {}</text>",
            label_x,
            overlay.y + 18,
            escape_text(&overlay.label)
        ));
    }
    out.push_str("</svg>");
    out
}

fn node_width(node: &NwdiagNode) -> i32 {
    if let Some(width) = node.width.and_then(|width| i32::try_from(width).ok()) {
        return width.clamp(120, 240);
    }

    let label = node_render_label(node, None);
    let label_width = normalized_label_lines(&label)
        .into_iter()
        .map(|line| {
            let sprite_padding = if label_contains_inline_sprite(&line) {
                22
            } else {
                0
            };
            text_width(&line, 12) + sprite_padding + 24
        })
        .max()
        .unwrap_or(140);
    label_width.clamp(120, 240)
}

fn node_height(node: &NwdiagNode) -> i32 {
    let label = node_render_label(node, None);
    let lines = normalized_label_lines(&label).len().max(1) as i32;
    (lines * 16 + 12).max(28)
}

fn network_row_step(network: &NwdiagNetwork) -> i32 {
    24 + 30 + network_max_node_height(network) + 24
}

fn network_after_node_gap(network: &NwdiagNetwork) -> i32 {
    network_max_node_height(network) + 24
}

fn network_max_node_height(network: &NwdiagNetwork) -> i32 {
    network.nodes.iter().map(node_height).max().unwrap_or(28)
}

fn node_render_label(node: &NwdiagNode, shared_span: Option<&SharedNodeSpan>) -> String {
    let display = shared_span
        .map(|span| span.label.as_str())
        .or(node.label.as_deref())
        .unwrap_or(&node.name);
    if node.addresses.is_empty() || shared_span.is_some() || label_needs_rich_layout(display) {
        display.to_string()
    } else {
        format!("{} [{}]", display, node.addresses.join(", "))
    }
}

fn render_nwdiag_node_label(out: &mut String, x: i32, y: i32, width: i32, label: &str) {
    let extra_attrs =
        "text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\" fill=\"#0f172a\"";
    if label_contains_inline_sprite(label) {
        out.push_str(&super::super::svg::creole_text(
            x + 10,
            y + 18,
            "font-family=\"monospace\" font-size=\"12\" fill=\"#0f172a\"",
            label,
            "#0f172a",
        ));
    } else {
        out.push_str(&super::super::svg::creole_text(
            x + (width / 2),
            y + 18,
            extra_attrs,
            label,
            "#0f172a",
        ));
    }
}

fn normalized_label_lines(label: &str) -> Vec<String> {
    tokenize_creole(label)
        .into_iter()
        .map(|line| line.into_iter().map(|span| span.text).collect::<String>())
        .collect()
}

fn label_contains_inline_sprite(label: &str) -> bool {
    label.contains("<$") || label.contains("<&") || label.contains('&')
}

fn label_needs_rich_layout(label: &str) -> bool {
    normalized_label_lines(label).len() > 1 || label_contains_inline_sprite(label)
}

fn shared_node_spans(
    document: &NwdiagDocument,
    column_x: &BTreeMap<String, i32>,
    column_widths: &BTreeMap<String, i32>,
    start_y: i32,
) -> BTreeMap<String, SharedNodeSpan> {
    let shared_names = shared_network_node_names(document);
    let mut spans: BTreeMap<String, SharedNodeSpan> = BTreeMap::new();
    let mut scan_y = start_y;
    for net in &document.networks {
        let bar_y = scan_y + 24;
        let node_y = bar_y + 30;
        for node in &net.nodes {
            if !shared_names.contains(&node.name) {
                continue;
            }
            let x = column_x.get(&node.name).copied().unwrap_or(56);
            let width = column_widths
                .get(&node.name)
                .copied()
                .unwrap_or_else(|| node_width(node));
            spans
                .entry(node.name.clone())
                .and_modify(|span| {
                    span.h = ((bar_y + 12) - span.y).max(span.h).max(node_height(node));
                    append_unique_addresses(&mut span.addresses, &node.addresses);
                    if span.color.is_none() {
                        span.color = node.color.clone();
                    }
                    if span.shape.is_none() {
                        span.shape = node.shape.clone();
                    }
                    if span.style.is_none() {
                        span.style = node.style.clone();
                    }
                })
                .or_insert_with(|| SharedNodeSpan {
                    x,
                    y: node_y,
                    w: width,
                    h: node_height(node),
                    addresses: node.addresses.clone(),
                    label: node.label.clone().unwrap_or_else(|| node.name.clone()),
                    color: node.color.clone(),
                    shape: node.shape.clone(),
                    style: node.style.clone(),
                });
        }
        scan_y = node_y + network_after_node_gap(net);
    }
    spans
}

fn shared_network_node_names(document: &NwdiagDocument) -> BTreeSet<String> {
    let mut network_counts: BTreeMap<String, usize> = BTreeMap::new();
    for net in &document.networks {
        let mut names_in_network = BTreeSet::new();
        for node in &net.nodes {
            names_in_network.insert(node.name.clone());
        }
        for name in names_in_network {
            *network_counts.entry(name).or_default() += 1;
        }
    }
    network_counts
        .into_iter()
        .filter_map(|(name, count)| (count > 1).then_some(name))
        .collect()
}

fn append_unique_addresses(target: &mut Vec<String>, addresses: &[String]) {
    for address in addresses {
        if !target.iter().any(|existing| existing == address) {
            target.push(address.clone());
        }
    }
}

fn node_is_in_network(document: &NwdiagDocument, name: &str) -> bool {
    document
        .networks
        .iter()
        .any(|network| network.nodes.iter().any(|node| node.name == name))
}

fn network_geometry(
    network: &NwdiagNetwork,
    column_x: &BTreeMap<String, i32>,
    inner_width: i32,
) -> (i32, i32) {
    if network.width_full || network.nodes.is_empty() {
        return (24, inner_width);
    }
    let mut min_x = i32::MAX;
    let mut max_x = i32::MIN;
    for node in &network.nodes {
        let Some(x) = column_x.get(&node.name).copied() else {
            continue;
        };
        min_x = min_x.min(x);
        max_x = max_x.max(x + node_width(node));
    }
    if min_x == i32::MAX {
        return (24, inner_width);
    }
    let padded_x = (min_x - 24).max(24);
    let padded_right = (max_x + 24).min(24 + inner_width);
    let base_width = (padded_right - padded_x).max(120);
    let label_width = text_width(&network_label(network), 13) + 16;
    let span = expand_span_to_fit(padded_x, base_width, label_width, 24, inner_width);
    (span.x, span.w)
}

fn network_label(network: &NwdiagNetwork) -> String {
    let name = network.label.as_deref().unwrap_or(&network.name).trim();
    match (name.is_empty(), network.address.as_deref()) {
        (true, Some(address)) => format!("network ({address})"),
        (true, None) => "network".to_string(),
        (false, Some(address)) => format!("network {name} ({address})"),
        (false, None) => format!("network {name}"),
    }
}

fn expand_span_to_fit(
    base_x: i32,
    base_width: i32,
    min_width: i32,
    frame_left: i32,
    inner_width: i32,
) -> BoxSpan {
    let max_right = frame_left + inner_width;
    let target_width = min_width.max(base_width);
    if target_width <= base_width {
        return BoxSpan {
            x: base_x,
            w: base_width,
        };
    }

    let extra = target_width - base_width;
    let mut x = base_x - (extra / 2);
    x = x.max(frame_left).min(max_right - target_width);
    let right = (x + target_width).min(max_right);
    BoxSpan {
        x,
        w: (right - x).max(base_width),
    }
}

fn text_width(text: &str, font_size: i32) -> i32 {
    rounded_proportional_monospace_width(text, font_size)
}

fn label_chip_x(overlay_x: i32, overlay_width: i32, chip_width: i32, connector_xs: &[i32]) -> i32 {
    let left = overlay_x + 4;
    let right = overlay_x + overlay_width - 4;
    if connector_xs.is_empty() {
        return left;
    }

    let mut gaps = Vec::new();
    let mut cursor = left;
    for &connector_x in connector_xs {
        let gap_right = connector_x - 6;
        if gap_right - cursor >= chip_width {
            gaps.push((cursor, gap_right));
        }
        cursor = (connector_x + 6).max(cursor);
    }
    if right - cursor >= chip_width {
        gaps.push((cursor, right));
    }
    if let Some((gap_left, gap_right)) = gaps
        .into_iter()
        .max_by_key(|(gap_left, gap_right)| (gap_right - gap_left, -(*gap_left - left).abs()))
    {
        let centered = gap_left + ((gap_right - gap_left - chip_width) / 2);
        return centered.clamp(left, right - chip_width);
    }

    left
}

fn nwdiag_stroke_dash(style: &str) -> &'static str {
    if style
        .split([',', ' '])
        .any(|part| part.eq_ignore_ascii_case("dotted"))
    {
        " stroke-dasharray=\"1 3\""
    } else if style
        .split([',', ' '])
        .any(|part| part.eq_ignore_ascii_case("dashed"))
    {
        " stroke-dasharray=\"5 3\""
    } else {
        ""
    }
}
