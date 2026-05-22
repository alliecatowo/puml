use std::collections::BTreeMap;

use crate::model::{SequenceDocument, SequenceEventKind, SequencePage};
use crate::model::{VirtualEndpoint, VirtualEndpointSide};
use crate::normalize;
use crate::scene::{
    ActivationBox, GroupBox, GroupSeparator, Label, LayoutOptions, LifecycleMarker,
    LifecycleMarkerKind, Lifeline, MessageLine, NoteBox, ParticipantBox, Scene, StructureKind,
    StructureLine, TextOverflowPolicy,
};
use crate::theme::MessageAlign;

const TEXT_LINE_HEIGHT: i32 = 16;
const GROUP_TEXT_INSET_X: i32 = 8;
const GROUP_HEADER_BASELINE_Y: i32 = 16;
const GROUP_REF_BODY_BASELINE_Y: i32 = 32;
const GROUP_BOTTOM_PADDING: i32 = 8;
const NOTE_TEXT_WIDTH_GUARD_PX: i32 = 8;
const METADATA_LINE_HEIGHT: i32 = 16;
const METADATA_BLOCK_PADDING: i32 = 8;
const TEOZ_ROUTE_LANE_HEIGHT: i32 = 14;
/// Height of the rendered self-loop U-shape below the message's `y` coordinate.
/// Must match `loop_h` in `render/sequence.rs`.
const SELF_LOOP_DROP: i32 = 32;

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
                aligned,
            } => {
                let (content_width, text_lines) = multiline_metrics(text);
                let width_from_text =
                    content_width + (options.note_padding * 2) + NOTE_TEXT_WIDTH_GUARD_PX;
                let width = options.note_width.max(width_from_text);
                let height = (text_lines * TEXT_LINE_HEIGHT) + (options.note_padding * 2);
                // For `/ note` (aligned), reuse the y of the most-recently placed note
                // so that the two notes appear side-by-side at the same vertical level.
                let base_y: i32 = if *aligned {
                    notes
                        .last()
                        .map(|last_note: &NoteBox| last_note.y)
                        .unwrap_or_else(|| events_top + (event_rows * options.message_row_height))
                } else {
                    events_top + (event_rows * options.message_row_height)
                };
                let y = note_vertical_position_y(position, base_y, height, events_top);
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
                // Aligned notes don't advance the row counter.
                if !aligned {
                    event_rows += row_units_for_height(height, options.message_row_height);
                }
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
        mainframe: document.mainframe.clone(),
    }
}

fn metadata_label_block_height(label: Option<&Label>) -> i32 {
    label
        .map(|label| metadata_lines_block_height(Some(&label.lines)))
        .unwrap_or(0)
}

fn metadata_block_right_edge(label: &Option<Label>, margin: i32) -> i32 {
    label
        .as_ref()
        .map(|label| {
            label
                .lines
                .iter()
                .map(|line| label.x + estimate_text_px_width(line) + margin)
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0)
}

fn metadata_lines_right_edge(lines: Option<&Vec<String>>, margin: i32) -> i32 {
    lines
        .map(|lines| {
            lines
                .iter()
                .map(|line| margin + estimate_text_px_width(line) + margin)
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0)
}

fn metadata_lines_block_height(lines: Option<&Vec<String>>) -> i32 {
    lines
        .map(|lines| (lines.len() as i32 * METADATA_LINE_HEIGHT) + METADATA_BLOCK_PADDING)
        .unwrap_or(0)
}

#[derive(Debug, Clone)]
struct OpenActivation {
    participant_id: String,
    y1: i32,
    depth: usize,
}

fn normalize_label_lines(text: &str, max_chars: usize, policy: TextOverflowPolicy) -> Vec<String> {
    match policy {
        TextOverflowPolicy::EllipsisSingleLine => {
            let one_line = text.replace('\n', " ");
            vec![ellipsize(&one_line, max_chars)]
        }
        TextOverflowPolicy::WrapAndGrow => text
            .lines()
            .flat_map(|line| wrap_line(line, max_chars))
            .collect::<Vec<_>>(),
    }
}

/// Count the *visual* (display) characters in a word, stripping creole/HTML
/// markup tags so that `<color:red>`, `</color>`, `<size:18>`, `</size>`,
/// `<b>`, `</b>`, `<i>`, `</i>`, `<u>`, `</u>`, `<&icon>`, etc. do not
/// inflate the character count used for line-wrapping decisions.
fn visual_char_count(word: &str) -> usize {
    let chars: Vec<char> = word.chars().collect();
    let len = chars.len();
    let mut count = 0;
    let mut i = 0;
    while i < len {
        if chars[i] == '<' {
            // Try to skip a markup tag: collect up to '>'.
            let mut j = i + 1;
            // Allow longer sprite references such as `<$name{scale=2,color=#2563eb}>`
            // to stay atomic during wrapping; splitting inside them prevents render-time
            // sprite substitution.
            let tag_limit = if i + 1 < len && chars[i + 1] == '$' {
                96
            } else {
                32
            };
            // Allow at most tag_limit chars inside the tag to avoid consuming large
            // non-tag `<...` sequences (e.g. math operators).
            while j < len && j - i <= tag_limit && chars[j] != '>' {
                j += 1;
            }
            if j < len && chars[j] == '>' {
                if i + 1 < len && chars[i + 1] == '$' {
                    count += 2;
                }
                // Consumed a tag — skip it entirely (or as compact sprite width).
                i = j + 1;
                continue;
            }
            // No closing '>' found within limit — treat '<' as a visual char.
            count += 1;
            i += 1;
        } else {
            count += 1;
            i += 1;
        }
    }
    count
}

fn wrap_line(line: &str, max_chars: usize) -> Vec<String> {
    if line.is_empty() {
        return vec![String::new()];
    }
    let words = line.split_whitespace().collect::<Vec<_>>();
    if words.is_empty() {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    // Track the *visual* length of `current` separately from its raw length.
    let mut current_visual: usize = 0;
    for word in words {
        let word_visual = visual_char_count(word);
        if current.is_empty() {
            if word_visual <= max_chars {
                current.push_str(word);
                current_visual = word_visual;
            } else {
                // Word is visually longer than max_chars.  If it contains
                // markup (visual_len < raw len) keep it whole rather than
                // splitting mid-tag; otherwise chunk it the old way.
                let word_raw = word.chars().count();
                if word_visual < word_raw {
                    // Contains markup — don't chunk it.
                    current.push_str(word);
                    current_visual = word_visual;
                } else {
                    for chunk in chunk_text(word, max_chars) {
                        lines.push(chunk);
                    }
                }
            }
            continue;
        }

        let next_visual = current_visual + 1 + word_visual;
        if next_visual <= max_chars {
            current.push(' ');
            current.push_str(word);
            current_visual = next_visual;
        } else {
            lines.push(current);
            let word_raw = word.chars().count();
            if word_visual <= max_chars {
                current = word.to_string();
                current_visual = word_visual;
            } else if word_visual < word_raw {
                // Contains markup — keep whole.
                current = word.to_string();
                current_visual = word_visual;
            } else {
                let mut chunks = chunk_text(word, max_chars);
                let tail = chunks.pop().unwrap_or_default();
                lines.extend(chunks);
                current_visual = visual_char_count(&tail);
                current = tail;
            }
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    debug_assert!(!lines.is_empty());
    lines
}

fn chunk_text(text: &str, max_chars: usize) -> Vec<String> {
    if max_chars == 0 {
        return vec![text.to_string()];
    }
    let mut out = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        current.push(ch);
        if current.chars().count() >= max_chars {
            out.push(current);
            current = String::new();
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    if out.is_empty() {
        vec![String::new()]
    } else {
        out
    }
}

fn ellipsize(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    if max_chars == 0 {
        return String::new();
    }
    if max_chars == 1 {
        return "…".to_string();
    }
    let mut out = String::new();
    for ch in text.chars().take(max_chars - 1) {
        out.push(ch);
    }
    out.push('…');
    out
}

fn structure_bounds(centers_by_id: &BTreeMap<String, i32>, options: &LayoutOptions) -> (i32, i32) {
    let x1 = options.margin;
    let width = (centers_by_id.len() as i32 * options.participant_spacing)
        .max(options.participant_width + 64);
    (x1, x1 + width)
}

fn default_center(options: &LayoutOptions) -> i32 {
    options.margin + options.participant_width / 2
}

fn parse_target_ids(spec: &str) -> Vec<String> {
    spec.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn note_target_centers(
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

fn note_target_bounds(
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

fn note_horizontal_bounds(
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

fn note_vertical_position_y(position: &str, row_y: i32, height: i32, events_top: i32) -> i32 {
    if position.eq_ignore_ascii_case("top") {
        return (row_y - height - 8).max(events_top - height - 8);
    }
    if position.eq_ignore_ascii_case("bottom") {
        return row_y + 8;
    }
    row_y
}

struct SceneGeometryRefs<'a> {
    participants: &'a [ParticipantBox],
    footboxes: &'a [ParticipantBox],
    lifelines: &'a [Lifeline],
    messages: &'a [MessageLine],
    activations: &'a [ActivationBox],
    lifecycle_markers: &'a [LifecycleMarker],
    notes: &'a [NoteBox],
    groups: &'a [GroupBox],
    structures: &'a [StructureLine],
}

struct SceneGeometryMut<'a> {
    participants: &'a mut [ParticipantBox],
    footboxes: &'a mut [ParticipantBox],
    lifelines: &'a mut [Lifeline],
    messages: &'a mut [MessageLine],
    activations: &'a mut [ActivationBox],
    lifecycle_markers: &'a mut [LifecycleMarker],
    notes: &'a mut [NoteBox],
    groups: &'a mut [GroupBox],
    structures: &'a mut [StructureLine],
}

fn scene_leftmost_geometry_x(geometry: SceneGeometryRefs<'_>) -> i32 {
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

fn shift_scene_geometry_x(delta: i32, geometry: SceneGeometryMut<'_>) {
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

fn group_horizontal_bounds(
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
    let width = (bounds_by_id.len() as i32 * options.participant_spacing)
        .max(options.participant_width + 64)
        .max(min_content_width);
    (options.margin, width)
}

fn multiline_metrics(text: &str) -> (i32, i32) {
    let mut max_width = 0;
    let mut lines = 0;
    for line in text.split('\n') {
        max_width = max_width.max(estimate_text_px_width(line));
        lines += 1;
    }
    (max_width, lines)
}

fn group_content_min_size(kind: &str, label: Option<&str>) -> (i32, i32) {
    if kind.eq_ignore_ascii_case("box") {
        let min_width = label
            .map(|label| estimate_text_px_width(label) + (GROUP_TEXT_INSET_X * 2))
            .unwrap_or(0);
        return (min_width, 0);
    }
    let Some(label) = label else {
        return (0, 0);
    };
    let mut lines = label.split('\n');
    let header = lines.next().unwrap_or("");
    let header_text = format!("{kind} {header}");
    let mut max_width = estimate_text_px_width(header_text.trim());
    let mut height = GROUP_HEADER_BASELINE_Y + GROUP_BOTTOM_PADDING;

    if kind.eq_ignore_ascii_case("ref") {
        // For ref boxes all label lines (including the first "over ..." line)
        // appear in the body.  Count the header line too.
        let mut body_lines = 1; // the first line already consumed above
        for line in lines {
            max_width = max_width.max(estimate_text_px_width(line));
            body_lines += 1;
        }
        height = GROUP_REF_BODY_BASELINE_Y
            + ((body_lines - 1) * TEXT_LINE_HEIGHT)
            + GROUP_BOTTOM_PADDING;
    }

    (max_width + (GROUP_TEXT_INSET_X * 2), height)
}

fn else_separator_label(label: Option<&str>) -> String {
    match label.map(str::trim).filter(|label| !label.is_empty()) {
        Some(label) => format!("else {label}"),
        None => "else".to_string(),
    }
}

fn estimate_text_px_width(line: &str) -> i32 {
    (line.chars().count() as i32) * 7
}

fn message_label_bounds(x1: i32, x2: i32, text_width: i32, align: MessageAlign) -> (i32, i32) {
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

fn legend_box_size(text: &str) -> (i32, i32) {
    let lines = text.lines().collect::<Vec<_>>();
    let line_count = lines.len().max(1) as i32;
    let max_line_width = lines
        .iter()
        .map(|line| estimate_text_px_width(line))
        .max()
        .unwrap_or(0);
    let width = (max_line_width + 16).max(200);
    let height = 24 + (line_count * 16);
    (width, height)
}

fn message_x_bounds(
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

fn message_label_lines(
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

fn row_units_for_height(height: i32, row_height: i32) -> i32 {
    if row_height <= 0 {
        return 1;
    }
    ((height + row_height - 1) / row_height).max(1)
}

#[derive(Debug, Default)]
struct AutonumberState {
    enabled: bool,
    next: AutonumberCounter,
    step: u64,
    format: Option<String>,
}

impl AutonumberState {
    fn update(&mut self, raw: Option<&str>) {
        let value = raw.map(str::trim).unwrap_or("");
        if value.eq_ignore_ascii_case("stop") || value.eq_ignore_ascii_case("off") {
            self.enabled = false;
            return;
        }

        if value.is_empty() {
            if self.next.is_zero() {
                self.next = AutonumberCounter::from_number(1);
            }
            if self.step == 0 {
                self.step = 1;
            }
            self.enabled = true;
            return;
        }

        let parsed = parse_autonumber_command(value);
        if let Some(level) = parsed.increment_level {
            self.next.increment_level(level, self.step.max(1));
            self.enabled = true;
            return;
        }
        if parsed.resume_only {
            if self.next.is_zero() {
                self.next = AutonumberCounter::from_number(1);
            }
        } else {
            self.next = parsed
                .start
                .unwrap_or_else(|| AutonumberCounter::from_number(1));
        }
        if let Some(step) = parsed.step {
            self.step = step.max(1);
        } else if self.step == 0 {
            self.step = 1;
        }
        if let Some(fmt) = parsed.format {
            self.format = Some(fmt);
        }
        self.enabled = true;
    }

    fn apply(&mut self, label: Option<String>) -> Option<String> {
        if !self.enabled {
            return label;
        }
        if self.next.is_zero() {
            self.next = AutonumberCounter::from_number(1);
        }
        if self.step == 0 {
            self.step = 1;
        }

        let number = format_autonumber(&self.next, self.format.as_deref());
        self.next.advance(self.step);
        match label {
            Some(text) if text.contains("%autonumber%") => {
                Some(text.replace("%autonumber%", &number))
            }
            Some(text) if !text.is_empty() => Some(format!("{number} {text}")),
            _ => Some(number.to_string()),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct AutonumberCounter {
    prefix: Vec<String>,
    separators: Vec<char>,
    current: u64,
    width: usize,
}

impl AutonumberCounter {
    fn from_number(value: u64) -> Self {
        Self {
            prefix: Vec::new(),
            separators: Vec::new(),
            current: value,
            width: 0,
        }
    }

    fn from_token(token: &str) -> Option<Self> {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            return None;
        }
        let mut parts = Vec::new();
        let mut separators = Vec::new();
        let mut current_part = String::new();
        for ch in trimmed.chars() {
            if matches!(ch, '.' | ';' | ',' | ':') {
                if current_part.is_empty() {
                    return None;
                }
                parts.push(current_part);
                separators.push(ch);
                current_part = String::new();
            } else if ch.is_ascii_digit() {
                current_part.push(ch);
            } else {
                return None;
            }
        }
        if !current_part.is_empty() {
            parts.push(current_part);
        }
        if parts.is_empty()
            || parts
                .iter()
                .any(|part| part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()))
        {
            return None;
        }
        let last = parts.last()?;
        let current = last.parse::<u64>().ok()?;
        let width = if last.len() > 1 { last.len() } else { 0 };
        Some(Self {
            prefix: parts[..parts.len().saturating_sub(1)]
                .iter()
                .map(|part| (*part).to_string())
                .collect(),
            separators,
            current,
            width,
        })
    }

    fn is_zero(&self) -> bool {
        self.prefix.is_empty() && self.current == 0
    }

    fn advance(&mut self, step: u64) {
        self.current = self.current.saturating_add(step.max(1));
    }

    fn increment_level(&mut self, level: usize, step: u64) {
        if level == 0 {
            return;
        }
        if level <= self.prefix.len() {
            if let Some(part) = self.prefix.get_mut(level - 1) {
                let width = part.len();
                let next = part.parse::<u64>().unwrap_or(0).saturating_add(step.max(1));
                *part = if width > 1 {
                    format!("{:0width$}", next, width = width)
                } else {
                    next.to_string()
                };
            }
        } else {
            self.advance(step);
        }
    }

    fn render(&self) -> String {
        let tail = if self.width > 0 {
            format!("{:0width$}", self.current, width = self.width)
        } else {
            self.current.to_string()
        };
        if self.prefix.is_empty() {
            tail
        } else {
            let mut out = String::new();
            for (idx, part) in self.prefix.iter().enumerate() {
                out.push_str(part);
                out.push(*self.separators.get(idx).unwrap_or(&'.'));
            }
            out.push_str(&tail);
            out
        }
    }
}

#[derive(Debug, Default)]
struct ParsedAutonumber {
    resume_only: bool,
    start: Option<AutonumberCounter>,
    step: Option<u64>,
    format: Option<String>,
    increment_level: Option<usize>,
}

fn parse_autonumber_command(raw: &str) -> ParsedAutonumber {
    let mut parsed = ParsedAutonumber::default();
    let mut rest = raw.trim();

    if rest.eq_ignore_ascii_case("resume") {
        parsed.resume_only = true;
        return parsed;
    }

    if rest
        .get(..4)
        .is_some_and(|head| head.eq_ignore_ascii_case("inc "))
    {
        let level = &rest[4..];
        parsed.increment_level = autonumber_increment_level(level.trim());
        return parsed;
    }

    if let Some(tail) = rest.strip_prefix("resume ") {
        parsed.resume_only = true;
        rest = tail.trim_start();
    }

    if let Some((format, before)) = trailing_quoted_format(rest) {
        parsed.format = Some(format);
        rest = before.trim_end();
    }

    let tokens: Vec<&str> = rest.split_whitespace().collect();
    let mut idx = 0usize;
    if parsed.resume_only {
        if let Some(token) = tokens.get(idx) {
            if let Ok(step) = token.parse::<u64>() {
                parsed.step = Some(step);
                idx += 1;
            }
        }
    } else {
        if let Some(token) = tokens.get(idx) {
            if let Some(counter) = AutonumberCounter::from_token(token) {
                parsed.start = Some(counter);
                idx += 1;
            }
        }
        if let Some(token) = tokens.get(idx) {
            if let Ok(step) = token.parse::<u64>() {
                parsed.step = Some(step);
                idx += 1;
            }
        }
    }

    if parsed.format.is_none() {
        parsed.format = tokens.get(idx).map(|part| (*part).to_string());
    }

    parsed
}

fn autonumber_increment_level(raw: &str) -> Option<usize> {
    let ch = raw.trim().chars().next()?;
    if !ch.is_ascii_alphabetic() {
        return None;
    }
    Some((ch.to_ascii_uppercase() as u8 - b'A' + 1) as usize)
}

fn trailing_quoted_format(raw: &str) -> Option<(String, &str)> {
    let trimmed = raw.trim_end();
    let end = trimmed.strip_suffix('"')?;
    let start = end.rfind('"')?;
    let format = end[start + 1..].to_string();
    let prefix = &end[..start];
    Some((format, prefix))
}

fn format_autonumber(counter: &AutonumberCounter, format: Option<&str>) -> String {
    let Some(format) = format else {
        return counter.render();
    };
    let fmt = format.trim();
    if fmt.is_empty() {
        return counter.render();
    }

    if fmt.contains('#') {
        return replace_hash_runs(fmt, counter.current);
    }

    let mut longest_zero_run = 0usize;
    let mut run_start = 0usize;
    let bytes = fmt.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'0' {
            let start = i;
            while i < bytes.len() && bytes[i] == b'0' {
                i += 1;
            }
            let len = i - start;
            if len > longest_zero_run {
                longest_zero_run = len;
                run_start = start;
            }
            continue;
        }
        i += 1;
    }

    if longest_zero_run == 0 {
        return format!("{fmt}{}", counter.current);
    }

    let padded = format!("{:0width$}", counter.current, width = longest_zero_run);
    let prefix = &fmt[..run_start];
    let suffix = &fmt[run_start + longest_zero_run..];
    format!("{prefix}{padded}{suffix}")
}

fn replace_hash_runs(format: &str, value: u64) -> String {
    let mut out = String::with_capacity(format.len() + 8);
    let bytes = format.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'#' {
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && bytes[i] == b'#' {
            i += 1;
        }
        let width = i - start;
        if width > 1 {
            out.push_str(&format!("{:0width$}", value, width = width));
        } else {
            out.push_str(&value.to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        Participant, ParticipantRole, SequenceDocument, SequenceEvent, SequenceEventKind,
    };
    use crate::source::Span;

    #[test]
    fn return_event_with_ids_is_laid_out_with_default_centers_for_unknown_participants() {
        let doc = SequenceDocument {
            participants: vec![Participant {
                id: "A".to_string(),
                display: "A".to_string(),
                role: ParticipantRole::Participant,
                explicit: true,
            }],
            events: vec![SequenceEvent {
                span: Span { start: 0, end: 0 },
                kind: SequenceEventKind::Return {
                    label: Some("back".to_string()),
                    from: Some("missing-from".to_string()),
                    to: Some("missing-to".to_string()),
                },
            }],
            ..SequenceDocument::default()
        };
        let scene = layout(&doc, LayoutOptions::default());
        assert_eq!(scene.messages.len(), 1);
        assert_eq!(scene.messages[0].arrow, "-->>");
    }

    #[test]
    fn text_helpers_cover_empty_whitespace_and_extreme_limits() {
        assert_eq!(wrap_line("", 8), vec![String::new()]);
        assert_eq!(wrap_line("   ", 8), vec![String::new()]);
        assert_eq!(
            wrap_line("seed abcdefghijklmnop", 4),
            vec!["seed", "abcd", "efgh", "ijkl", "mnop"]
        );
        assert_eq!(chunk_text("abc", 0), vec!["abc".to_string()]);
        assert_eq!(chunk_text("", 3), vec![String::new()]);
        assert_eq!(ellipsize("abc", 8), "abc");
        assert_eq!(ellipsize("abc", 0), "");
        assert_eq!(ellipsize("abc", 1), "…");
    }

    #[test]
    fn geometry_and_autonumber_edge_branches_are_deterministic() {
        let options = LayoutOptions::default();
        let mut centers = BTreeMap::new();
        let mut bounds = BTreeMap::new();
        let center = options.margin + options.participant_width / 2;
        bounds.insert(
            "A".to_string(),
            (options.margin, options.margin + options.participant_width),
        );
        centers.insert("A".to_string(), center);

        let (x, _) = note_horizontal_bounds("right", None, &centers, &bounds, 300, 120, &options);
        assert_eq!(x, options.margin);
        let (x_mid, _) =
            note_horizontal_bounds("over", Some("A"), &centers, &bounds, 300, 120, &options);
        assert_eq!(x_mid, center - 60);

        let (gx, gw) = group_horizontal_bounds("group", Some("over   "), &bounds, &options);
        assert_eq!(gx, options.margin);
        assert!(gw >= options.participant_width + 64);
        assert_eq!(group_content_min_size("group", None), (0, 0));

        assert_eq!(row_units_for_height(40, 0), 1);
        assert_eq!(
            message_x_bounds(
                "x",
                "y",
                Some(VirtualEndpoint {
                    side: VirtualEndpointSide::Right,
                    kind: crate::model::VirtualEndpointKind::Filled,
                }),
                Some(VirtualEndpoint {
                    side: VirtualEndpointSide::Left,
                    kind: crate::model::VirtualEndpointKind::Filled,
                }),
                &centers,
                &options,
            ),
            (center + 56, center - 56)
        );

        let parsed = parse_autonumber_command("resume");
        assert!(parsed.resume_only);
        let parsed_fmt = parse_autonumber_command("resume fmt");
        assert_eq!(parsed_fmt.format.as_deref(), Some("fmt"));
        let parsed_dotted = parse_autonumber_command("1.02.003 4");
        assert_eq!(
            parsed_dotted.start.as_ref().map(AutonumberCounter::render),
            Some("1.02.003".to_string())
        );
        assert_eq!(parsed_dotted.step, Some(4));
        let mut auton = AutonumberState::default();
        auton.update(None);
        assert_eq!(auton.apply(Some(String::new())).as_deref(), Some("1"));
        let counter = AutonumberCounter::from_number(7);
        assert_eq!(format_autonumber(&counter, Some("")), "7");
        assert_eq!(format_autonumber(&counter, Some("item")), "item7");
        assert_eq!(format_autonumber(&counter, Some("n=#")), "n=7");
        assert_eq!(format_autonumber(&counter, Some("n=###")), "n=007");
    }

    #[test]
    fn autonumber_resume_and_zero_state_fallbacks_are_covered() {
        let mut state = AutonumberState::default();
        state.update(Some("resume"));
        assert_eq!(state.next.render(), "1");

        let mut state = AutonumberState {
            enabled: true,
            next: AutonumberCounter::default(),
            step: 0,
            format: None,
        };
        assert_eq!(state.apply(None).as_deref(), Some("1"));

        let mut state = AutonumberState {
            enabled: false,
            next: AutonumberCounter::from_number(8),
            step: 0,
            format: None,
        };
        state.update(Some("resume"));
        assert_eq!(state.step, 1);

        let mut state = AutonumberState::default();
        state.update(Some("1.02.003"));
        assert_eq!(
            state.apply(Some("dotted".to_string())).as_deref(),
            Some("1.02.003 dotted")
        );
        assert_eq!(
            state.apply(Some("next".to_string())).as_deref(),
            Some("1.02.004 next")
        );

        let bounds: BTreeMap<String, (i32, i32)> = BTreeMap::new();
        let (_gx, gw) = group_horizontal_bounds("group", None, &bounds, &LayoutOptions::default());
        assert!(gw >= LayoutOptions::default().participant_width + 64);
    }
}
