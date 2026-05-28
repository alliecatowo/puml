use std::collections::BTreeMap;

mod autonumber;
mod create;
mod finalize;
mod geometry;
mod structure;
mod text;

use crate::model::{ParticipantRole, SequenceDocument, SequenceEventKind, SequencePage};
use crate::normalize;
use crate::scene::{
    ActivationBox, GroupBox, GroupSeparator, Label, LayoutOptions, LifecycleMarker, Lifeline,
    MessageLine, NoteBox, ParticipantBox, Scene, StructureKind, StructureLine,
};
use autonumber::AutonumberState;
use create::{handle_create_event, handle_destroy_event};
use geometry::{
    activation_edge_for_depth, activation_message_x_bounds, default_center,
    group_horizontal_bounds, message_x_bounds, note_horizontal_bounds, note_vertical_position_y,
    scene_leftmost_geometry_x, shift_scene_geometry_x, OpenActivation, SceneGeometryMut,
    SceneGeometryRefs,
};
use structure::push_structure_line;
use text::{
    else_separator_label, estimate_text_px_width, group_content_min_size, legend_box_size,
    message_label_bounds, message_label_lines, message_label_top_clearance,
    metadata_block_right_edge, metadata_label_block_height, metadata_label_x,
    metadata_lines_block_height, metadata_lines_right_edge, multiline_metrics,
    normalize_label_lines, row_units_for_height,
};

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

    // Compute x/center positions for ALL participants (mid-flow created ones included)
    // so routing has access to them.  Header boxes for mid-flow creates are deferred.
    for (idx, participant) in document.participants.iter().enumerate() {
        let x = options.margin + (idx as i32 * options.participant_spacing);
        let center_x = x + options.participant_width / 2;
        max_participant_right = max_participant_right.max(x + options.participant_width);
        centers_by_id.insert(participant.id.clone(), center_x);
        bounds_by_id.insert(participant.id.clone(), (x, x + options.participant_width));

        // Skip the header box for participants created mid-flow — their box will
        // be added when the `Create` event is processed below.
        if document.created_participants.contains(&participant.id) {
            // Still consume the display_lines entry to avoid leaking memory.
            participant_lines_by_id.remove(&participant.id);
            continue;
        }

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
        align: document.header_align,
    });
    let header_block_height = metadata_label_block_height(header.as_ref());
    let title = document.title.as_ref().map(|text| Label {
        x: options.margin,
        y: options.margin + header_block_height,
        lines: normalize_label_lines(text, title_max_chars, options.text_overflow_policy),
        align: Default::default(),
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

    // Pre-build display-lines for mid-flow created participants so they can be
    // used when the Create event is processed in the event loop below.
    let mut created_display_lines: BTreeMap<String, (Vec<String>, ParticipantRole)> =
        BTreeMap::new();
    for participant in &document.participants {
        if document.created_participants.contains(&participant.id) {
            let lines = normalize_label_lines(
                &participant.display,
                participant_max_chars,
                options.text_overflow_policy,
            );
            created_display_lines.insert(participant.id.clone(), (lines, participant.role));
        }
    }

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
    let mut last_arrival_message_ix: BTreeMap<String, usize> = BTreeMap::new();

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
                let base_y = if is_parallel {
                    messages
                        .last()
                        .map(|message| message.y)
                        .unwrap_or(events_top + (event_rows * options.message_row_height))
                } else {
                    events_top + (event_rows * options.message_row_height)
                };
                let (mut x1, mut x2) = message_x_bounds(
                    from,
                    to,
                    *from_virtual,
                    *to_virtual,
                    &centers_by_id,
                    &options,
                );
                if from != to || from_virtual.is_some() || to_virtual.is_some() {
                    (x1, x2) = activation_message_x_bounds(
                        from,
                        to,
                        x1,
                        x2,
                        *from_virtual,
                        *to_virtual,
                        &centers_by_id,
                        &activation_stack,
                        &options,
                    );
                }
                let label = autonumber.apply(label.clone());
                let label_lines = message_label_lines(
                    label.as_deref(),
                    x1,
                    x2,
                    document.style.sequence_message_span,
                    &options,
                );
                let label_clearance = message_label_top_clearance(
                    &label_lines,
                    style.parallel,
                    document.style.response_message_below_arrow,
                    arrow,
                );
                let y = base_y + label_clearance;
                let has_label_lines = !label_lines.is_empty();
                let route_lane = if document.teoz && is_parallel {
                    let lane = teoz_route_lanes_by_row.entry(y).or_insert(0);
                    *lane += 1;
                    *lane
                } else {
                    0
                };
                let route_y = y + (route_lane * TEOZ_ROUTE_LANE_HEIGHT);
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
                    last_arrival_message_ix.insert(to.clone(), messages.len());
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
                    let y = y + message_label_top_clearance(
                        &label_lines,
                        false,
                        document.style.response_message_below_arrow,
                        "-->>",
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
                if let Some(message_ix) = last_arrival_message_ix.get(id.as_str()).copied() {
                    if let Some(message) = messages.get_mut(message_ix) {
                        if message.to_id == *id && message.to_virtual.is_none() {
                            let left_to_right = message.x2 >= message.x1;
                            let target_edge_right = !left_to_right;
                            message.x2 = activation_edge_for_depth(
                                id,
                                depth,
                                target_edge_right,
                                &centers_by_id,
                                &options,
                            );
                        }
                    }
                }
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
                handle_create_event(
                    id,
                    &document.created_participants,
                    &created_display_lines,
                    &centers_by_id,
                    &options,
                    events_top,
                    &mut event_rows,
                    participant_height,
                    &mut participants,
                    &mut lifecycle_markers,
                );
            }
            SequenceEventKind::Destroy(id) => {
                handle_destroy_event(
                    id,
                    &centers_by_id,
                    &options,
                    events_top,
                    &mut event_rows,
                    &mut lifecycle_markers,
                );
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
            SequenceEventKind::Delay(label) => push_structure_line(
                StructureKind::Delay,
                label.clone(),
                None,
                &mut event_rows,
                events_top,
                &centers_by_id,
                &options,
                &mut structures,
            ),
            SequenceEventKind::Divider(label) => push_structure_line(
                StructureKind::Divider,
                label.clone(),
                None,
                &mut event_rows,
                events_top,
                &centers_by_id,
                &options,
                &mut structures,
            ),
            SequenceEventKind::Separator(label) => push_structure_line(
                StructureKind::Separator,
                label.clone(),
                None,
                &mut event_rows,
                events_top,
                &centers_by_id,
                &options,
                &mut structures,
            ),
            SequenceEventKind::Spacer(pixels) => push_structure_line(
                StructureKind::Spacer,
                None,
                Some(pixels.unwrap_or(options.message_row_height)),
                &mut event_rows,
                events_top,
                &centers_by_id,
                &options,
                &mut structures,
            ),
            _ => {}
        }
    }

    finalize::finish_sequence_scene(
        document,
        options,
        header,
        title,
        participants,
        messages,
        activations,
        lifecycle_markers,
        notes,
        groups,
        open_groups,
        structures,
        activation_stack,
        &centers_by_id,
        &bounds_by_id,
        max_participant_right,
        participant_top,
        participant_height,
        events_top,
        event_rows,
        title_max_chars,
    )
}

#[cfg(test)]
mod tests;
