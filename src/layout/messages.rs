use std::collections::BTreeMap;

use crate::model::{VirtualEndpoint, VirtualEndpointSide};
use crate::scene::LayoutOptions;
use crate::theme::MessageAlign;

use super::text::normalize_label_lines;

pub(super) fn message_x_bounds(
    from: &str,
    to: &str,
    from_virtual: Option<VirtualEndpoint>,
    to_virtual: Option<VirtualEndpoint>,
    centers_by_id: &BTreeMap<String, i32>,
    options: &LayoutOptions,
) -> (i32, i32) {
    let default_center = options.margin + options.participant_width / 2;
    let from_center = centers_by_id.get(from).copied().unwrap_or(default_center);
    let to_center = centers_by_id.get(to).copied().unwrap_or(default_center);
    let side_offset = 56;
    let self_loop_width = 44;

    if from == to && from_virtual.is_none() && to_virtual.is_none() {
        return (from_center, from_center + self_loop_width);
    }
    if let (Some(from_meta), Some(to_meta)) = (from_virtual, to_virtual) {
        let x1 = if from_meta.side == VirtualEndpointSide::Left {
            default_center - side_offset
        } else {
            default_center + side_offset
        };
        let x2 = if to_meta.side == VirtualEndpointSide::Left {
            default_center - side_offset
        } else {
            default_center + side_offset
        };
        return (x1, x2);
    }
    if let Some(meta) = from_virtual {
        let target_center = to_center;
        let x1 = if meta.side == VirtualEndpointSide::Left {
            target_center - side_offset
        } else {
            target_center + side_offset
        };
        return (x1, target_center);
    }
    if let Some(meta) = to_virtual {
        let source_center = from_center;
        let x2 = if meta.side == VirtualEndpointSide::Left {
            source_center - side_offset
        } else {
            source_center + side_offset
        };
        return (source_center, x2);
    }
    (from_center, to_center)
}

pub(super) fn message_label_lines(
    label: Option<&str>,
    x1: i32,
    x2: i32,
    sequence_message_span: bool,
    options: &LayoutOptions,
) -> Vec<String> {
    let Some(label) = label else {
        return Vec::new();
    };
    let min_span = (options.participant_spacing - 20).max(56);
    let span_px = if sequence_message_span {
        (options.participant_spacing * 2).max((x2 - x1).abs())
    } else {
        (x2 - x1).abs().max(min_span) - 16
    };
    let tx = ((x1 + x2) / 2) + 2;
    let max_chars_by_span = (span_px / 7).max(1) as usize;
    let max_chars_by_left_edge = ((tx * 2) / 7).max(1) as usize;
    let mut max_chars = max_chars_by_span.min(max_chars_by_left_edge);
    if starts_with_autonumber_prefix(label) {
        max_chars = max_chars.saturating_add(4);
    }
    normalize_label_lines(label, max_chars, options.text_overflow_policy)
}

fn starts_with_autonumber_prefix(label: &str) -> bool {
    let Some(first) = label.split_whitespace().next() else {
        return false;
    };
    (first.contains('.')
        && first
            .split('.')
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit())))
        || (first.contains('-') && first.bytes().any(|b| b.is_ascii_digit()))
}

pub(super) fn message_label_bounds(
    x1: i32,
    x2: i32,
    text_width: i32,
    align: MessageAlign,
) -> (i32, i32) {
    let left = x1.min(x2);
    let right = x1.max(x2);
    match align {
        MessageAlign::Left => {
            let anchor = left + 8;
            (anchor, anchor + text_width)
        }
        MessageAlign::Center => {
            let anchor = ((x1 + x2) / 2) + 2;
            (anchor - (text_width / 2), anchor + ((text_width + 1) / 2))
        }
        MessageAlign::Right => {
            let anchor = right - 8;
            (anchor - text_width, anchor)
        }
    }
}
