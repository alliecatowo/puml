use std::collections::BTreeMap;

use crate::model::{VirtualEndpoint, VirtualEndpointSide};
use crate::scene::{
    ActivationBox, GroupBox, LayoutOptions, LifecycleMarker, Lifeline, MessageLine, NoteBox,
    ParticipantBox, StructureLine,
};

use super::text::group_content_min_size;

#[derive(Debug, Clone)]
pub(super) struct OpenActivation {
    pub(super) participant_id: String,
    pub(super) y1: i32,
    pub(super) depth: usize,
}

pub(super) fn structure_bounds(
    centers_by_id: &BTreeMap<String, i32>,
    options: &LayoutOptions,
) -> (i32, i32) {
    let x1 = options.margin;
    let width = (centers_by_id.len() as i32 * options.participant_spacing)
        .max(options.participant_width + 64);
    (x1, x1 + width)
}

pub(super) fn default_center(options: &LayoutOptions) -> i32 {
    options.margin + options.participant_width / 2
}

pub(super) fn parse_target_ids(spec: &str) -> Vec<String> {
    spec.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub(super) fn note_target_centers(
    target_spec: &str,
    centers_by_id: &BTreeMap<String, i32>,
    options: &LayoutOptions,
) -> Vec<i32> {
    let default = default_center(options);
    parse_target_ids(target_spec)
        .into_iter()
        .map(|id| centers_by_id.get(&id).copied().unwrap_or(default))
        .collect::<Vec<_>>()
}

pub(super) fn note_target_bounds(
    target_spec: &str,
    bounds_by_id: &BTreeMap<String, (i32, i32)>,
    options: &LayoutOptions,
) -> Vec<(i32, i32)> {
    let default_center = default_center(options);
    let default_bounds = (
        default_center - (options.participant_width / 2),
        default_center + (options.participant_width / 2),
    );
    parse_target_ids(target_spec)
        .into_iter()
        .map(|id| bounds_by_id.get(&id).copied().unwrap_or(default_bounds))
        .collect::<Vec<_>>()
}

pub(super) fn note_horizontal_bounds(
    position: &str,
    target_spec: Option<&str>,
    centers_by_id: &BTreeMap<String, i32>,
    bounds_by_id: &BTreeMap<String, (i32, i32)>,
    max_participant_right: i32,
    width: i32,
    options: &LayoutOptions,
) -> (i32, i32) {
    if position.eq_ignore_ascii_case("across") {
        let span_width = (max_participant_right - options.margin).max(options.note_width);
        return (options.margin, span_width.max(width));
    }

    let x = if let Some(target_spec) = target_spec {
        let bounds = note_target_bounds(target_spec, bounds_by_id, options);
        let min_left = bounds
            .iter()
            .map(|(left, _)| *left)
            .min()
            .unwrap_or(options.margin);
        let max_right = bounds
            .iter()
            .map(|(_, right)| *right)
            .max()
            .unwrap_or(max_participant_right);
        let centers = note_target_centers(target_spec, centers_by_id, options);
        let min_center = *centers.iter().min().unwrap_or(&default_center(options));
        let max_center = *centers.iter().max().unwrap_or(&default_center(options));
        let mid_center = (min_center + max_center) / 2;
        if position.eq_ignore_ascii_case("left") {
            min_left - width - 12
        } else if position.eq_ignore_ascii_case("right") {
            max_right + 12
        } else if bounds.len() > 1 {
            return (
                min_left.max(options.margin),
                width.max(max_right - min_left),
            );
        } else {
            mid_center - (width / 2)
        }
    } else {
        options.margin
    };

    (x, width)
}

pub(super) fn note_vertical_position_y(
    position: &str,
    row_y: i32,
    height: i32,
    events_top: i32,
) -> i32 {
    if position.eq_ignore_ascii_case("top") {
        return (row_y - height - 8).max(events_top - height - 8);
    }
    if position.eq_ignore_ascii_case("bottom") {
        return row_y + 8;
    }
    row_y
}

pub(super) struct SceneGeometryRefs<'a> {
    pub(super) participants: &'a [ParticipantBox],
    pub(super) footboxes: &'a [ParticipantBox],
    pub(super) lifelines: &'a [Lifeline],
    pub(super) messages: &'a [MessageLine],
    pub(super) activations: &'a [ActivationBox],
    pub(super) lifecycle_markers: &'a [LifecycleMarker],
    pub(super) notes: &'a [NoteBox],
    pub(super) groups: &'a [GroupBox],
    pub(super) structures: &'a [StructureLine],
}

pub(super) struct SceneGeometryMut<'a> {
    pub(super) participants: &'a mut [ParticipantBox],
    pub(super) footboxes: &'a mut [ParticipantBox],
    pub(super) lifelines: &'a mut [Lifeline],
    pub(super) messages: &'a mut [MessageLine],
    pub(super) activations: &'a mut [ActivationBox],
    pub(super) lifecycle_markers: &'a mut [LifecycleMarker],
    pub(super) notes: &'a mut [NoteBox],
    pub(super) groups: &'a mut [GroupBox],
    pub(super) structures: &'a mut [StructureLine],
}

pub(super) fn scene_leftmost_geometry_x(geometry: SceneGeometryRefs<'_>) -> i32 {
    let participant_min = geometry
        .participants
        .iter()
        .map(|participant| participant.x)
        .min();
    let footbox_min = geometry.footboxes.iter().map(|footbox| footbox.x).min();
    let lifeline_min = geometry.lifelines.iter().map(|lifeline| lifeline.x).min();
    let message_min = geometry
        .messages
        .iter()
        .map(|message| message.x1.min(message.x2) - 8)
        .min();
    let activation_min = geometry
        .activations
        .iter()
        .map(|activation| activation.x - 5)
        .min();
    let lifecycle_min = geometry
        .lifecycle_markers
        .iter()
        .map(|marker| marker.x - 6)
        .min();
    let note_min = geometry.notes.iter().map(|note| note.x).min();
    let group_min = geometry.groups.iter().map(|group| group.x).min();
    let structure_min = geometry
        .structures
        .iter()
        .map(|structure| structure.x1.min(structure.x2))
        .min();

    participant_min
        .into_iter()
        .chain(footbox_min)
        .chain(lifeline_min)
        .chain(message_min)
        .chain(activation_min)
        .chain(lifecycle_min)
        .chain(note_min)
        .chain(group_min)
        .chain(structure_min)
        .min()
        .unwrap_or(0)
}

pub(super) fn shift_scene_geometry_x(delta: i32, geometry: SceneGeometryMut<'_>) {
    if delta <= 0 {
        return;
    }

    for participant in geometry.participants {
        participant.x += delta;
    }
    for footbox in geometry.footboxes {
        footbox.x += delta;
    }
    for lifeline in geometry.lifelines {
        lifeline.x += delta;
    }
    for message in geometry.messages {
        message.x1 += delta;
        message.x2 += delta;
    }
    for activation in geometry.activations {
        activation.x += delta;
    }
    for marker in geometry.lifecycle_markers {
        marker.x += delta;
    }
    for note in geometry.notes {
        note.x += delta;
    }
    for group in geometry.groups {
        group.x += delta;
    }
    for structure in geometry.structures {
        structure.x1 += delta;
        structure.x2 += delta;
    }
}

pub(super) fn group_horizontal_bounds(
    kind: &str,
    label: Option<&str>,
    bounds_by_id: &BTreeMap<String, (i32, i32)>,
    options: &LayoutOptions,
) -> (i32, i32) {
    let (min_content_width, _) = group_content_min_size(kind, label);
    if let Some(raw) = label {
        let header = raw.lines().next().unwrap_or(raw);
        if let Some(target_spec) = header.strip_prefix("over ") {
            let bounds = note_target_bounds(target_spec.trim(), bounds_by_id, options);
            if !bounds.is_empty() {
                let min_left = bounds
                    .iter()
                    .map(|(left, _)| *left)
                    .min()
                    .unwrap_or(options.margin);
                let max_right = bounds
                    .iter()
                    .map(|(_, right)| *right)
                    .max()
                    .unwrap_or(options.margin + options.participant_width);
                let target_width = (max_right - min_left).max(options.participant_width);
                let width = target_width.max(min_content_width);
                let x = (min_left - ((width - target_width) / 2)).max(options.margin);
                return (x, width);
            }
        }
    }
    let min_left = bounds_by_id
        .values()
        .map(|(left, _)| *left)
        .min()
        .unwrap_or(options.margin);
    let max_right = bounds_by_id
        .values()
        .map(|(_, right)| *right)
        .max()
        .unwrap_or(options.margin + options.participant_width);
    let participant_span_width = (max_right - min_left).max(options.participant_width);
    let width = participant_span_width.max(min_content_width);
    let x = (min_left - ((width - participant_span_width) / 2)).max(options.margin);
    (x, width)
}

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

#[allow(clippy::too_many_arguments)]
pub(super) fn activation_message_x_bounds(
    from: &str,
    to: &str,
    x1: i32,
    x2: i32,
    from_virtual: Option<VirtualEndpoint>,
    to_virtual: Option<VirtualEndpoint>,
    centers_by_id: &BTreeMap<String, i32>,
    activation_stack: &[OpenActivation],
    options: &LayoutOptions,
) -> (i32, i32) {
    let left_to_right = x2 >= x1;
    let adjusted_x1 = if from_virtual.is_none() {
        active_activation_edge(
            from,
            left_to_right,
            centers_by_id,
            activation_stack,
            options,
        )
        .unwrap_or(x1)
    } else {
        x1
    };
    let adjusted_x2 = if to_virtual.is_none() {
        active_activation_edge(to, !left_to_right, centers_by_id, activation_stack, options)
            .unwrap_or(x2)
    } else {
        x2
    };
    (adjusted_x1, adjusted_x2)
}

pub(super) fn active_activation_edge(
    id: &str,
    right_edge: bool,
    centers_by_id: &BTreeMap<String, i32>,
    activation_stack: &[OpenActivation],
    options: &LayoutOptions,
) -> Option<i32> {
    activation_stack
        .iter()
        .rfind(|open| open.participant_id == id)
        .map(|open| activation_edge_for_depth(id, open.depth, right_edge, centers_by_id, options))
}

pub(super) fn activation_edge_for_depth(
    id: &str,
    depth: usize,
    right_edge: bool,
    centers_by_id: &BTreeMap<String, i32>,
    options: &LayoutOptions,
) -> i32 {
    let center = centers_by_id
        .get(id)
        .copied()
        .unwrap_or_else(|| default_center(options));
    let offset = (depth as i32) * 6;
    center + offset + if right_edge { 5 } else { -5 }
}
