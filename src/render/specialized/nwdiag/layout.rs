use std::collections::BTreeMap;

use super::*;
use crate::model::{NwdiagNetwork, NwdiagNode};
use crate::render::text_metrics::rounded_proportional_monospace_width;

// ── Packed-grid density constants (#1466) ────────────────────────────────────
// Reduced from the original loose layout to match PlantUML's compact rendering.
// Target: ≤1.5× area ratio vs PlantUML for the 02_multi_network fixture.
const NWDIAG_NODE_WIDTH_MIN: i32 = 80;
const NWDIAG_NODE_WIDTH_MAX: i32 = 180;
const NWDIAG_NODE_LABEL_HPAD: i32 = 16; // horizontal padding added to label text width
const NWDIAG_NODE_LINES_VPAD: i32 = 8; // vertical padding per-node (added to line height sum)
const NWDIAG_NODE_LINE_HEIGHT: i32 = 14; // px per text line inside a node box
const NWDIAG_NODE_HEIGHT_MIN: i32 = 20; // minimum node box height
const NWDIAG_LANE_TOP_PAD: i32 = 12; // space above the network header band
pub(super) const NWDIAG_BUS_DROP: i32 = 18; // distance from bus bar bottom to node box top
const NWDIAG_LANE_BOTTOM_PAD: i32 = 12; // space below node boxes to next lane
pub(super) const NWDIAG_COL_GAP: i32 = 16; // horizontal gap between node columns
pub(super) const NWDIAG_CANVAS_MIN_WIDTH: i32 = 520; // minimum canvas width (was 760)

pub(super) fn node_width(node: &NwdiagNode) -> i32 {
    if let Some(width) = node.width.and_then(|width| i32::try_from(width).ok()) {
        return width.clamp(NWDIAG_NODE_WIDTH_MIN, NWDIAG_NODE_WIDTH_MAX);
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
            text_width(&line, 12) + sprite_padding + NWDIAG_NODE_LABEL_HPAD
        })
        .max()
        .unwrap_or(100);
    label_width.clamp(NWDIAG_NODE_WIDTH_MIN, NWDIAG_NODE_WIDTH_MAX)
}

pub(super) fn node_height(node: &NwdiagNode) -> i32 {
    let label = node_render_label(node, None);
    let lines = normalized_label_lines(&label).len().max(1) as i32;
    (lines * NWDIAG_NODE_LINE_HEIGHT + NWDIAG_NODE_LINES_VPAD).max(NWDIAG_NODE_HEIGHT_MIN)
}

pub(super) fn network_row_step(network: &NwdiagNetwork) -> i32 {
    NWDIAG_LANE_TOP_PAD
        + NWDIAG_BUS_DROP
        + network_max_node_height(network)
        + NWDIAG_LANE_BOTTOM_PAD
}

pub(super) fn network_after_node_gap(network: &NwdiagNetwork) -> i32 {
    network_max_node_height(network) + NWDIAG_BUS_DROP
}

pub(super) fn network_max_node_height(network: &NwdiagNetwork) -> i32 {
    network.nodes.iter().map(node_height).max().unwrap_or(28)
}

pub(super) fn network_geometry(
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

pub(super) fn network_label(network: &NwdiagNetwork) -> String {
    // Kind-tag suppression (#1372): omit the "network " prefix from segment
    // titles.  PlantUML renders only the segment name (plus CIDR on the bar),
    // not the "network" keyword.
    let name = network.label.as_deref().unwrap_or(&network.name).trim();
    match (name.is_empty(), network.address.as_deref()) {
        (true, Some(address)) => format!("({address})"),
        (true, None) => String::new(),
        (false, Some(address)) => format!("{name} ({address})"),
        (false, None) => name.to_string(),
    }
}

pub(super) fn expand_span_to_fit(
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

pub(super) fn text_width(text: &str, font_size: i32) -> i32 {
    rounded_proportional_monospace_width(text, font_size)
}

pub(super) fn label_chip_x(
    overlay_x: i32,
    overlay_width: i32,
    chip_width: i32,
    connector_xs: &[i32],
) -> i32 {
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
