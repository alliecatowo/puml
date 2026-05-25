use std::collections::BTreeMap;

use crate::model::MetadataHAlign;
use crate::render::layout_constants::{MESSAGE_LABEL_LINE_GAP, REF_HEADER_HEIGHT};
use crate::render::text_metrics::default_monospace_width;
use crate::render_core::{LabelBox, LabelRole, Rect};
use crate::scene::{GroupBox, Label, MessageLine, NoteBox, ParticipantBox};
use crate::theme::MessageAlign;

const LABEL_HEIGHT: f64 = 16.0;
const PARTICIPANT_LABEL_TOP: f64 = 10.0;
const MESSAGE_LABEL_ASCENT: f64 = 12.0;
const NOTE_TEXT_TOP: f64 = 12.0;
const NOTE_TEXT_LEFT: f64 = 8.0;

pub(super) fn participant_label_boxes(
    node_id: &str,
    participant: &ParticipantBox,
) -> Vec<LabelBox> {
    let text = participant.display_lines.join("\n");
    let width = participant
        .display_lines
        .iter()
        .map(|line| default_monospace_width(line))
        .max()
        .unwrap_or(0)
        .min(participant.width)
        .max(1);
    vec![LabelBox {
        id: format!("{node_id}:label"),
        text,
        bounds: Rect::new(
            f64::from(participant.x + (participant.width - width) / 2),
            f64::from(participant.y) + PARTICIPANT_LABEL_TOP,
            f64::from(width),
            (participant.display_lines.len() as f64 * LABEL_HEIGHT).max(LABEL_HEIGHT),
        ),
        owner_id: Some(node_id.to_string()),
        role: LabelRole::Node,
    }]
}

pub(super) fn note_label_boxes(node_id: &str, note: &NoteBox) -> Vec<LabelBox> {
    let lines = note.text.lines().collect::<Vec<_>>();
    let width = lines
        .iter()
        .map(|line| default_monospace_width(line))
        .max()
        .unwrap_or(0)
        .min(note.width - 16)
        .max(1);
    vec![LabelBox {
        id: format!("{node_id}:label"),
        text: note.text.clone(),
        bounds: Rect::new(
            f64::from(note.x) + NOTE_TEXT_LEFT,
            f64::from(note.y) + NOTE_TEXT_TOP,
            f64::from(width),
            (lines.len() as f64 * LABEL_HEIGHT).max(LABEL_HEIGHT),
        ),
        owner_id: Some(node_id.to_string()),
        role: LabelRole::Node,
    }]
}

pub(super) fn group_header_bounds(group: &GroupBox) -> Option<Rect> {
    if group.kind.eq_ignore_ascii_case("box") {
        return group.label.as_ref().map(|label| {
            Rect::new(
                f64::from(group.x),
                f64::from(group.y),
                f64::from((default_monospace_width(label) + 16).clamp(40, group.width)),
                22.0,
            )
        });
    }

    let width = if group.kind.eq_ignore_ascii_case("ref") {
        32_i32.min(group.width.saturating_sub(4)).max(24)
    } else {
        let first_line = group
            .label
            .as_deref()
            .and_then(|label| label.lines().next())
            .unwrap_or("");
        let header_text = format!("{} {}", group.kind, first_line).trim().to_string();
        (default_monospace_width(&header_text) + 16)
            .clamp(40, group.width.saturating_sub(4).max(40))
    };
    Some(Rect::new(
        f64::from(group.x),
        f64::from(group.y),
        f64::from(width),
        f64::from(REF_HEADER_HEIGHT),
    ))
}

pub(super) fn group_label_boxes(group_id: &str, group: &GroupBox) -> Vec<LabelBox> {
    let Some(header) = group_header_bounds(group) else {
        return Vec::new();
    };
    let text = if group.kind.eq_ignore_ascii_case("ref") {
        group.label.clone().unwrap_or_else(|| "ref".to_string())
    } else if let Some(label) = &group.label {
        format!("{} {}", group.kind, label.lines().next().unwrap_or(""))
            .trim()
            .to_string()
    } else {
        group.kind.clone()
    };
    let width = default_monospace_width(text.lines().next().unwrap_or(&text)).max(1);
    vec![LabelBox {
        id: format!("{group_id}:label"),
        text,
        bounds: Rect::new(
            header.min_x() + 8.0,
            header.min_y() + 2.0,
            f64::from(width),
            LABEL_HEIGHT,
        ),
        owner_id: Some(group_id.to_string()),
        role: LabelRole::Group,
    }]
}

pub(super) fn metadata_label_boxes(role: &str, label: &Label, scene_width: i32) -> Vec<LabelBox> {
    label
        .lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            let width = default_monospace_width(line).max(1);
            let x = match label.align {
                MetadataHAlign::Left => label.x,
                MetadataHAlign::Center => (scene_width - width) / 2,
                MetadataHAlign::Right => scene_width - label.x - width,
            };
            LabelBox {
                id: format!("metadata:{role}:{index}"),
                text: line.clone(),
                bounds: Rect::new(
                    f64::from(x),
                    f64::from(label.y + (index as i32 * 16)) - MESSAGE_LABEL_ASCENT,
                    f64::from(width),
                    LABEL_HEIGHT,
                ),
                owner_id: None,
                role: LabelRole::Other,
            }
        })
        .collect()
}

pub(super) fn message_label_boxes(
    edge_id: &str,
    message: &MessageLine,
    align: MessageAlign,
    response_below_arrow: bool,
    parallel_label_lanes: &mut BTreeMap<i32, i32>,
) -> Vec<LabelBox> {
    let lines = if message.label_lines.is_empty() {
        message
            .label
            .as_ref()
            .map(|label| vec![label.clone()])
            .unwrap_or_default()
    } else {
        message.label_lines.clone()
    };
    if lines.is_empty() {
        return Vec::new();
    }

    let (text_x, anchor) = sequence_message_label_anchor(message.x1, message.x2, align);
    let below = response_below_arrow && is_response_message_arrow(&message.arrow);
    let lane_offset = if message.style.parallel || below {
        let lane = parallel_label_lanes.entry(message.y).or_insert(0);
        let offset = *lane * MESSAGE_LABEL_LINE_GAP;
        *lane += (lines.len() as i32).max(1);
        offset
    } else {
        0
    };
    let baseline_y = if message.style.parallel || below {
        message.route_y + 16 + lane_offset
    } else {
        message.route_y - 8 - (((lines.len() as i32) - 1) * MESSAGE_LABEL_LINE_GAP)
    };
    let width = lines
        .iter()
        .map(|line| default_monospace_width(line))
        .max()
        .unwrap_or(0)
        .max(1);
    let x = label_left_for_anchor(text_x, width, anchor);
    vec![LabelBox {
        id: format!("{edge_id}:label"),
        text: lines.join("\n"),
        bounds: Rect::new(
            f64::from(x),
            f64::from(baseline_y) - MESSAGE_LABEL_ASCENT,
            f64::from(width),
            (lines.len() as f64 * f64::from(MESSAGE_LABEL_LINE_GAP)).max(LABEL_HEIGHT),
        ),
        owner_id: Some(edge_id.to_string()),
        role: LabelRole::Edge,
    }]
}

fn sequence_message_label_anchor(x1: i32, x2: i32, align: MessageAlign) -> (i32, &'static str) {
    let left = x1.min(x2);
    let right = x1.max(x2);
    match align {
        MessageAlign::Left => (left + 8, "start"),
        MessageAlign::Center => (((x1 + x2) / 2) + 2, "middle"),
        MessageAlign::Right => (right - 8, "end"),
    }
}

fn label_left_for_anchor(text_x: i32, width: i32, anchor: &str) -> i32 {
    match anchor {
        "middle" => text_x - (width / 2),
        "end" => text_x - width,
        _ => text_x,
    }
}

fn is_response_message_arrow(arrow: &str) -> bool {
    arrow.contains("--")
}
