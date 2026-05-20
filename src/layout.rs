use std::collections::BTreeMap;

use crate::model::{SequenceDocument, SequenceEventKind, SequencePage};
use crate::normalize;
use crate::scene::{
    ActivationBox, GroupBox, GroupSeparator, Label, LayoutOptions, LifecycleMarker,
    LifecycleMarkerKind, Lifeline, MessageLine, NoteBox, ParticipantBox, Scene, StructureKind,
    StructureLine,
};

mod autonumber;
mod geometry;
mod groups;
mod messages;
mod metrics;
mod notes;
mod text;

use autonumber::AutonumberState;
use geometry::{
    scene_leftmost_geometry_x, shift_scene_geometry_x, SceneGeometryMut, SceneGeometryRefs,
};
use groups::{else_separator_label, group_content_min_size, group_horizontal_bounds};
use messages::{message_label_bounds, message_label_lines, message_x_bounds};
use metrics::{
    default_center, estimate_text_px_width, legend_box_size, metadata_block_right_edge,
    metadata_label_block_height, metadata_lines_block_height, metadata_lines_right_edge,
    multiline_metrics, row_units_for_height, structure_bounds, METADATA_LINE_HEIGHT,
    NOTE_TEXT_WIDTH_GUARD_PX, SELF_LOOP_DROP, TEOZ_ROUTE_LANE_HEIGHT, TEXT_LINE_HEIGHT,
};
use notes::{note_horizontal_bounds, note_vertical_position_y};
use text::normalize_label_lines;

#[cfg(test)]
mod tests;

pub fn layout(document: &SequenceDocument, options: LayoutOptions) -> Scene {
    let mut pages = layout_pages(document, options);
    debug_assert!(!pages.is_empty());
    pages.remove(0)
}

pub fn layout_pages(document: &SequenceDocument, options: LayoutOptions) -> Vec<Scene> {
    normalize::paginate(document)
        .into_iter()
        .map(|page| layout_page(&page, options))
        .collect()
}

fn layout_page(document: &SequencePage, options: LayoutOptions) -> Scene {
    let mut participants = Vec::with_capacity(document.participants.len());
    let mut centers_by_id = BTreeMap::new();
    let mut bounds_by_id = BTreeMap::new();

    let mut max_participant_right = options.margin;
    let participant_text_max_width = (options.participant_width - 16).max(8);
    let participant_max_chars = (participant_text_max_width / 7).max(1) as usize;
    let mut participant_lines_by_id = BTreeMap::new();
    let mut max_participant_line_count = 1_i32;
    for participant in &document.participants {
        let lines = normalize_label_lines(
            &participant.display,
            participant_max_chars,
            options.text_overflow_policy,
        );
        max_participant_line_count = max_participant_line_count.max(lines.len() as i32);
        participant_lines_by_id.insert(participant.id.clone(), lines);
    }
    let participant_height = (max_participant_line_count * 16) + 12;
    let participant_height = participant_height.max(options.participant_height);

    for (idx, participant) in document.participants.iter().enumerate() {
        let x = options.margin + (idx as i32 * options.participant_spacing);
        let center_x = x + options.participant_width / 2;
        max_participant_right = max_participant_right.max(x + options.participant_width);
        centers_by_id.insert(participant.id.clone(), center_x);
        bounds_by_id.insert(participant.id.clone(), (x, x + options.participant_width));

        participants.push(ParticipantBox {
            id: participant.id.clone(),
            display_lines: participant_lines_by_id
                .remove(&participant.id)
                .unwrap_or_else(|| vec![participant.display.clone()]),
            role: participant.role,
            x,
            y: options.margin,
            width: options.participant_width,
            height: participant_height,
        });
    }

    let title_max_width = (max_participant_right - (options.margin * 2)).max(200);
    let title_max_chars = (title_max_width / 9).max(1) as usize;
    let header = document.header.as_ref().map(|text| Label {
        x: options.margin,
        y: options.margin,
        lines: normalize_label_lines(text, title_max_chars, options.text_overflow_policy),
    });
    let header_block_height = metadata_label_block_height(header.as_ref());
    let title = document.title.as_ref().map(|text| Label {
        x: options.margin,
        y: options.margin + header_block_height,
        lines: normalize_label_lines(text, title_max_chars, options.text_overflow_policy),
    });

    let title_block_height = if let Some(label) = &title {
        (label.lines.len() as i32 * 24).max(options.title_height)
    } else {
        0
    };

    let participant_top = options.margin + header_block_height + title_block_height;
    for p in &mut participants {
        p.y = participant_top;
    }

    let events_top = participant_top + participant_height + 24;
    let mut messages: Vec<MessageLine> = Vec::new();
    let mut activations = Vec::new();
    let mut lifecycle_markers = Vec::new();
    let mut activation_stack: Vec<OpenActivation> = Vec::new();
    let mut notes = Vec::new();
    let mut groups: Vec<GroupBox> = Vec::new();
    let mut structures = Vec::new();
    let mut open_groups: Vec<usize> = Vec::new();
    let mut event_rows: i32 = 0;
    let mut autonumber = AutonumberState::default();
    let mut teoz_route_lanes_by_row: BTreeMap<i32, i32> = BTreeMap::new();
    // Track the y-coordinate of the last message that arrived at each participant
    // so that explicit `activate X` can pin the bar start to the arriving message row.
    let mut last_arrival_y: BTreeMap<String, i32> = BTreeMap::new();

    for event in &document.events {
        match &event.kind {
            SequenceEventKind::Message {
                from,
                to,
                arrow,
                label,
                style,
                from_virtual,
                to_virtual,
            } => {
                let is_parallel = style.parallel && !messages.is_empty();
                let y = if is_parallel {
                    messages
                        .last()
                        .map(|message| message.y)
                        .unwrap_or(events_top + (event_rows * options.message_row_height))
                } else {
                    events_top + (event_rows * options.message_row_height)
                };
                let (x1, x2) = message_x_bounds(
                    from,
                    to,
                    *from_virtual,
                    *to_virtual,
                    &centers_by_id,
                    &options,
                );
                let route_lane = if document.teoz && is_parallel {
                    let lane = teoz_route_lanes_by_row.entry(y).or_insert(0);
                    *lane += 1;
                    *lane
                } else {
                    0
                };
                let route_y = y + (route_lane * TEOZ_ROUTE_LANE_HEIGHT);
                let label = autonumber.apply(label.clone());
                let label_lines = message_label_lines(
                    label.as_deref(),
                    x1,
                    x2,
                    document.style.sequence_message_span,
                    &options,
                );
                let has_label_lines = !label_lines.is_empty();
                let is_self_loop = from == to && from_virtual.is_none() && to_virtual.is_none();
                // Self-loop messages render a U-shape that drops SELF_LOOP_DROP px below
                // the message's `y` coordinate.  Allocate at least 2 rows so the loop
                // bottom does not overlap the label of the immediately following message.
                let row_units = {
                    let base = (label_lines.len() as i32).max(1);
                    if is_self_loop {
                        base.max(row_units_for_height(
                            SELF_LOOP_DROP + options.message_row_height / 2,
                            options.message_row_height,
                        ))
                    } else {
                        base
                    }
                };
                // Record arrival y for the recipient so that an immediately
                // following explicit `activate` can pin its bar to this row.
                if to_virtual.is_none() && !to.is_empty() {
                    last_arrival_y.insert(to.clone(), y);
                }
                messages.push(MessageLine {
                    from_id: from.clone(),
                    to_id: to.clone(),
                    x1,
                    y,
                    route_y,
                    x2,
                    arrow: arrow.clone(),
                    label,
                    label_lines,
                    style: style.clone(),
                    from_virtual: *from_virtual,
                    to_virtual: *to_virtual,
                });
                if !is_parallel {
                    event_rows += row_units;
                } else if has_label_lines || document.teoz {
                    let route_units = row_units_for_height(
                        (route_y - y) + TEXT_LINE_HEIGHT,
                        options.message_row_height,
                    );
                    event_rows += row_units.max(route_units);
                }
            }
            SequenceEventKind::Return { label, from, to } => {
                if let (Some(from_id), Some(to_id)) = (from.as_ref(), to.as_ref()) {
                    let y = events_top + (event_rows * options.message_row_height);
                    let x1 = centers_by_id
                        .get(from_id)
                        .copied()
                        .unwrap_or(options.margin + options.participant_width / 2);
                    let x2 = centers_by_id
                        .get(to_id)
                        .copied()
                        .unwrap_or(options.margin + options.participant_width / 2);
                    let label = autonumber.apply(label.clone());
                    let label_lines = message_label_lines(
                        label.as_deref(),
                        x1,
                        x2,
                        document.style.sequence_message_span,
                        &options,
                    );
                    let row_units = (label_lines.len() as i32).max(1);
                    messages.push(MessageLine {
                        from_id: from_id.clone(),
                        to_id: to_id.clone(),
                        x1,
                        y,
                        route_y: y,
                        x2,
                        // `return` shorthand is a dashed reply with an open
                        // thin arrowhead — equivalent to PlantUML's `A -->> B`
                        // (or `A <-- B` written left-to-right).  Using "-->"
                        // produces a filled solid head which is wrong.
                        arrow: "-->>".to_string(),
                        label,
                        label_lines,
                        style: Default::default(),
                        from_virtual: None,
                        to_virtual: None,
                    });
                    event_rows += row_units;
                }
            }
            SequenceEventKind::Autonumber(raw) => {
                autonumber.update(raw.as_deref());
            }
            SequenceEventKind::Activate(id) => {
                let current_y = events_top + (event_rows * options.message_row_height);
                // Pin the activation bar to the y-coordinate of the most recent
                // message that arrived at this participant (i.e. the arrow head
                // position), so the bar visually starts at the incoming call
                // rather than at the next event row.
                let y1 = last_arrival_y
                    .get(id.as_str())
                    .copied()
                    .unwrap_or(current_y);
                let depth = activation_stack
                    .iter()
                    .filter(|open| open.participant_id == *id)
                    .count();
                activation_stack.push(OpenActivation {
                    participant_id: id.clone(),
                    y1,
                    depth,
                });
            }
            SequenceEventKind::Deactivate(id) => {
                let y = events_top + (event_rows * options.message_row_height);
                if let Some(pos) = activation_stack
                    .iter()
                    .rposition(|open| open.participant_id == *id)
                {
                    let open = activation_stack.remove(pos);
                    let x = centers_by_id
                        .get(id)
                        .copied()
                        .unwrap_or_else(|| default_center(&options));
                    activations.push(ActivationBox {
                        participant_id: id.clone(),
                        x,
                        y1: open.y1,
                        y2: y.max(open.y1 + 12),
                        depth: open.depth,
                    });
                }
            }
            SequenceEventKind::Create(id) => {
                let y = events_top + (event_rows * options.message_row_height);
                let x = centers_by_id
                    .get(id)
                    .copied()
                    .unwrap_or_else(|| default_center(&options));
                lifecycle_markers.push(LifecycleMarker {
                    participant_id: id.clone(),
                    x,
                    y,
                    kind: LifecycleMarkerKind::Create,
                });
            }
            SequenceEventKind::Destroy(id) => {
                let y = events_top + (event_rows * options.message_row_height);
                let x = centers_by_id
                    .get(id)
                    .copied()
                    .unwrap_or_else(|| default_center(&options));
                lifecycle_markers.push(LifecycleMarker {
                    participant_id: id.clone(),
                    x,
                    y,
                    kind: LifecycleMarkerKind::Destroy,
                });
                event_rows += 1;
            }
            SequenceEventKind::Note {
                kind,
                target,
                text,
                position,
            } => {
                let (content_width, text_lines) = multiline_metrics(text);
                let width_from_text =
                    content_width + (options.note_padding * 2) + NOTE_TEXT_WIDTH_GUARD_PX;
                let width = options.note_width.max(width_from_text);
                let height = (text_lines * TEXT_LINE_HEIGHT) + (options.note_padding * 2);
                let y = note_vertical_position_y(
                    position,
                    events_top + (event_rows * options.message_row_height),
                    height,
                    events_top,
                );
                let (x, width) = note_horizontal_bounds(
                    position,
                    target.as_deref(),
                    &centers_by_id,
                    &bounds_by_id,
                    max_participant_right,
                    width,
                    &options,
                );

                notes.push(NoteBox {
                    target_id: target.clone(),
                    kind: *kind,
                    x,
                    y,
                    width,
                    height,
                    text: text.clone(),
                });
                event_rows += row_units_for_height(height, options.message_row_height);
            }
            SequenceEventKind::GroupStart { kind, label } => {
                let y = events_top + (event_rows * options.message_row_height);
                if kind.eq_ignore_ascii_case("else") {
                    if let Some(ix) = open_groups.last().copied() {
                        let separator_follows_self_loop = messages.last().is_some_and(|message| {
                            message.from_id == message.to_id
                                && message.from_virtual.is_none()
                                && message.to_virtual.is_none()
                        });
                        let separator_y = if separator_follows_self_loop {
                            // Self-call rows already consume most of the available
                            // vertical lane, so reserve extra clearance for the
                            // `else` divider label before the next branch begins.
                            event_rows += 1;
                            y + (options.message_row_height / 2)
                        } else {
                            y
                        };
                        groups[ix].separators.push(GroupSeparator {
                            y: separator_y,
                            label: Some(else_separator_label(label.as_deref())),
                        });
                    }
                } else {
                    let (x, width) =
                        group_horizontal_bounds(kind, label.as_deref(), &bounds_by_id, &options);
                    if kind.eq_ignore_ascii_case("ref") {
                        let (_, min_height) = group_content_min_size(kind, label.as_deref());
                        let height = options.message_row_height.max(min_height);
                        groups.push(GroupBox {
                            kind: kind.clone(),
                            label: label.clone(),
                            color: None,
                            x,
                            y,
                            width,
                            height,
                            separators: Vec::new(),
                        });
                        // Add a clearance buffer (one label height) so the
                        // label on the first message after the ref box does not
                        // clip into the box's lower border.  The label is
                        // drawn at line_y - 8, so we need the next row to be
                        // at least (box_height + 16) px below the ref start.
                        event_rows += row_units_for_height(height + 16, options.message_row_height);
                        continue;
                    } else {
                        groups.push(GroupBox {
                            kind: kind.clone(),
                            label: label.clone(),
                            color: None,
                            x,
                            y,
                            width,
                            height: options.message_row_height,
                            separators: Vec::new(),
                        });
                        open_groups.push(groups.len() - 1);
                    }
                }
                event_rows += 1;
            }
            SequenceEventKind::GroupEnd => {
                let y = events_top + (event_rows * options.message_row_height);
                if let Some(ix) = open_groups.pop() {
                    groups[ix].height = (y - groups[ix].y) + options.message_row_height;
                    let (_, min_height) =
                        group_content_min_size(&groups[ix].kind, groups[ix].label.as_deref());
                    groups[ix].height = groups[ix].height.max(min_height);
                }
                event_rows += 1;
            }
            SequenceEventKind::Delay(label) => {
                let y = events_top + (event_rows * options.message_row_height);
                let (x1, x2) = structure_bounds(&centers_by_id, &options);
                structures.push(StructureLine {
                    kind: StructureKind::Delay,
                    y,
                    x1,
                    x2,
                    label: label.clone(),
                });
                event_rows += 1;
            }
            SequenceEventKind::Divider(label) => {
                let y = events_top + (event_rows * options.message_row_height);
                let (x1, x2) = structure_bounds(&centers_by_id, &options);
                structures.push(StructureLine {
                    kind: StructureKind::Divider,
                    y,
                    x1,
                    x2,
                    label: label.clone(),
                });
                event_rows += 1;
            }
            SequenceEventKind::Separator(label) => {
                let y = events_top + (event_rows * options.message_row_height);
                let (x1, x2) = structure_bounds(&centers_by_id, &options);
                structures.push(StructureLine {
                    kind: StructureKind::Separator,
                    y,
                    x1,
                    x2,
                    label: label.clone(),
                });
                event_rows += 1;
            }
            SequenceEventKind::Spacer(pixels) => {
                let y = events_top + (event_rows * options.message_row_height);
                let (x1, x2) = structure_bounds(&centers_by_id, &options);
                structures.push(StructureLine {
                    kind: StructureKind::Spacer,
                    y,
                    x1,
                    x2,
                    label: None,
                });
                let pixels = pixels.unwrap_or(options.message_row_height).max(1);
                event_rows += row_units_for_height(pixels, options.message_row_height);
            }
            _ => {}
        }
    }

    let end_y = events_top + (event_rows * options.message_row_height);
    while let Some(ix) = open_groups.pop() {
        groups[ix].height = (end_y - groups[ix].y).max(options.message_row_height);
        let (_, min_height) = group_content_min_size(&groups[ix].kind, groups[ix].label.as_deref());
        groups[ix].height = groups[ix].height.max(min_height);
    }
    let fallback_activation_end = end_y.max(events_top + options.message_row_height);
    for open in activation_stack {
        let x = centers_by_id
            .get(&open.participant_id)
            .copied()
            .unwrap_or_else(|| default_center(&options));
        activations.push(ActivationBox {
            participant_id: open.participant_id,
            x,
            y1: open.y1,
            y2: fallback_activation_end.max(open.y1 + 12),
            depth: open.depth,
        });
    }

    let events_height = if event_rows > 0 {
        (event_rows - 1) * options.message_row_height
    } else {
        0
    };

    // Self-loop messages extend SELF_LOOP_DROP px below their `y` coordinate in the
    // SVG renderer.  The standard `events_height` formula only tracks row *starts*, so
    // a self-loop that is the last (or near-last) event can overflow the computed
    // content boundary and cause footboxes to overlap it.  Clamp the content end to
    // include the full rendered drop of any self-loop message.
    let self_loop_max_bottom = messages
        .iter()
        .filter(|m| m.from_id == m.to_id && m.from_virtual.is_none() && m.to_virtual.is_none())
        .map(|m| m.y + SELF_LOOP_DROP)
        .max()
        .unwrap_or(0);

    let lifeline_start = participant_top + participant_height;
    let row_based_content_end = (events_top + events_height).max(self_loop_max_bottom);
    let max_box_bottom = groups
        .iter()
        .map(|g| g.y + g.height)
        .chain(notes.iter().map(|n| n.y + n.height))
        .max()
        .unwrap_or(row_based_content_end);
    let content_end = row_based_content_end.max(max_box_bottom);
    let lifeline_end = if document.footbox_visible {
        content_end + options.footer_height
    } else {
        content_end
    };
    let mut footboxes = if document.footbox_visible {
        participants
            .iter()
            .map(|p| ParticipantBox {
                id: p.id.clone(),
                display_lines: p.display_lines.clone(),
                role: p.role,
                x: p.x,
                y: lifeline_end,
                width: p.width,
                height: p.height,
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let mut lifelines = participants
        .iter()
        .map(|p| Lifeline {
            participant_id: p.id.clone(),
            x: p.x + p.width / 2,
            y1: lifeline_start,
            y2: lifeline_end,
        })
        .collect::<Vec<_>>();

    let participant_group_padding = document.style.box_padding.unwrap_or(12).max(0);
    let participant_group_bottom = if footboxes.is_empty() {
        lifeline_end + participant_group_padding
    } else {
        footboxes
            .iter()
            .map(|footbox| footbox.y + footbox.height + participant_group_padding)
            .max()
            .unwrap_or(lifeline_end + participant_group_padding)
    };
    for participant_group in &document.participant_groups {
        let member_bounds = participant_group
            .participant_ids
            .iter()
            .filter_map(|id| bounds_by_id.get(id))
            .copied()
            .collect::<Vec<_>>();
        if member_bounds.is_empty() {
            continue;
        }
        let min_left = member_bounds
            .iter()
            .map(|(left, _)| *left)
            .min()
            .unwrap_or(options.margin);
        let max_right = member_bounds
            .iter()
            .map(|(_, right)| *right)
            .max()
            .unwrap_or(options.margin + options.participant_width);
        let header_band = if participant_group.label.is_some() {
            22
        } else {
            10
        };
        let y = participant_top - header_band;
        groups.push(GroupBox {
            kind: "box".to_string(),
            label: participant_group.label.clone(),
            color: participant_group.color.clone(),
            x: min_left - participant_group_padding,
            y,
            width: (max_right - min_left) + (participant_group_padding * 2),
            height: (participant_group_bottom - y).max(participant_height + header_band),
            separators: Vec::new(),
        });
    }

    let leftmost_geometry_x = scene_leftmost_geometry_x(SceneGeometryRefs {
        participants: &participants,
        footboxes: &footboxes,
        lifelines: &lifelines,
        messages: &messages,
        activations: &activations,
        lifecycle_markers: &lifecycle_markers,
        notes: &notes,
        groups: &groups,
        structures: &structures,
    });
    if leftmost_geometry_x < options.margin {
        shift_scene_geometry_x(
            options.margin - leftmost_geometry_x,
            SceneGeometryMut {
                participants: &mut participants,
                footboxes: &mut footboxes,
                lifelines: &mut lifelines,
                messages: &mut messages,
                activations: &mut activations,
                lifecycle_markers: &mut lifecycle_markers,
                notes: &mut notes,
                groups: &mut groups,
                structures: &mut structures,
            },
        );
    }

    let mut width = (max_participant_right + options.margin).max(options.margin * 2 + 200);
    for n in &notes {
        width = width.max(n.x + n.width + options.margin);
    }
    for g in &groups {
        width = width.max(g.x + g.width + options.margin);
    }
    for s in &structures {
        width = width.max(s.x2 + options.margin);
    }
    for m in &messages {
        let message_left = m.x1.min(m.x2) - 8;
        let message_right = m.x1.max(m.x2) + 8;
        width = width.max((message_right + options.margin).max(message_left + options.margin));
        if m.label_lines.is_empty() {
            continue;
        }
        for line in &m.label_lines {
            let text_width = estimate_text_px_width(line);
            let (_, right) =
                message_label_bounds(m.x1, m.x2, text_width, document.style.message_align);
            width = width.max(right + options.margin);
        }
    }
    width = width.max(metadata_block_right_edge(&header, options.margin));
    width = width.max(metadata_block_right_edge(&title, options.margin));

    let lower_metadata_max_chars = title_max_chars;
    let caption_lines = document.caption.as_ref().map(|text| {
        normalize_label_lines(text, lower_metadata_max_chars, options.text_overflow_policy)
    });
    let footer_lines = document.footer.as_ref().map(|text| {
        normalize_label_lines(text, lower_metadata_max_chars, options.text_overflow_policy)
    });
    let lower_metadata_height = metadata_lines_block_height(caption_lines.as_ref())
        + metadata_lines_block_height(footer_lines.as_ref());
    width = width.max(metadata_lines_right_edge(
        caption_lines.as_ref(),
        options.margin,
    ));
    width = width.max(metadata_lines_right_edge(
        footer_lines.as_ref(),
        options.margin,
    ));
    if let Some(legend_text) = document.legend.as_deref() {
        let (legend_width, _) = legend_box_size(legend_text);
        width = width.max(legend_width + (options.margin * 2));
    }

    let min_bottom = if footboxes.is_empty() {
        lifeline_end + options.footer_height
    } else {
        lifeline_end + participant_height
    };
    let height = (min_bottom + lower_metadata_height + options.margin)
        .max(participant_top + participant_height + 80);

    let mut lower_metadata_y = min_bottom + METADATA_LINE_HEIGHT;
    let caption = caption_lines.map(|lines| {
        let label = Label {
            x: options.margin,
            y: lower_metadata_y,
            lines,
        };
        lower_metadata_y += metadata_label_block_height(Some(&label));
        label
    });
    let footer = footer_lines.map(|lines| Label {
        x: options.margin,
        y: lower_metadata_y,
        lines,
    });

    Scene {
        width,
        height,
        header,
        title,
        caption,
        footer,
        participants,
        footboxes,
        lifelines,
        messages,
        activations,
        lifecycle_markers,
        notes,
        groups,
        structures,
        style: document.style.clone(),
        scale: document.scale.clone(),
        legend_text: document.legend.clone(),
        legend_halign: document.legend_halign,
        legend_valign: document.legend_valign,
    }
}

#[derive(Debug, Clone)]
struct OpenActivation {
    participant_id: String,
    y1: i32,
    depth: usize,
}
