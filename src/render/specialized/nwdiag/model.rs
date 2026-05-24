use std::collections::BTreeSet;

use super::*;
use crate::creole::tokenize_creole;
use crate::model::NwdiagNode;

pub(super) struct NodeRect {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) w: i32,
    pub(super) h: i32,
}

pub(super) struct BoxSpan {
    pub(super) x: i32,
    pub(super) w: i32,
}

pub(super) struct SharedNodeSpan {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) w: i32,
    pub(super) h: i32,
    pub(super) addresses: Vec<String>,
    pub(super) label: String,
    pub(super) color: Option<String>,
    pub(super) shape: Option<String>,
    pub(super) style: Option<String>,
}
pub(super) fn node_render_label(node: &NwdiagNode, shared_span: Option<&SharedNodeSpan>) -> String {
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

pub(super) fn normalized_label_lines(label: &str) -> Vec<String> {
    tokenize_creole(label)
        .into_iter()
        .map(|line| line.into_iter().map(|span| span.text).collect::<String>())
        .collect()
}

pub(super) fn label_contains_inline_sprite(label: &str) -> bool {
    label.contains("<$") || label.contains("<&") || label.contains('&')
}

pub(super) fn label_needs_rich_layout(label: &str) -> bool {
    normalized_label_lines(label).len() > 1 || label_contains_inline_sprite(label)
}

pub(super) fn shared_node_spans(
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

pub(super) fn shared_network_node_names(document: &NwdiagDocument) -> BTreeSet<String> {
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

pub(super) fn append_unique_addresses(target: &mut Vec<String>, addresses: &[String]) {
    for address in addresses {
        if !target.iter().any(|existing| existing == address) {
            target.push(address.clone());
        }
    }
}

pub(super) fn node_is_in_network(document: &NwdiagDocument, name: &str) -> bool {
    document
        .networks
        .iter()
        .any(|network| network.nodes.iter().any(|node| node.name == name))
}
