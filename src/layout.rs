use std::collections::BTreeMap;

use crate::model::{SequenceDocument, SequenceEventKind, SequencePage};
use crate::model::{VirtualEndpoint, VirtualEndpointSide};
use crate::normalize;
use crate::scene::{
    GroupBox, GroupSeparator, Label, LayoutOptions, Lifeline, MessageLine, NoteBox, ParticipantBox,
    Scene, StructureKind, StructureLine, TextOverflowPolicy,
};

const TEXT_LINE_HEIGHT: i32 = 16;
const GROUP_TEXT_INSET_X: i32 = 8;
const GROUP_HEADER_BASELINE_Y: i32 = 16;
const GROUP_REF_BODY_BASELINE_Y: i32 = 32;
const GROUP_BOTTOM_PADDING: i32 = 8;
const NOTE_TEXT_WIDTH_GUARD_PX: i32 = 8;

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
    let title = document.title.as_ref().map(|text| Label {
        x: options.margin,
        y: options.margin,
        lines: normalize_label_lines(text, title_max_chars, options.text_overflow_policy),
    });

    let title_block_height = if let Some(label) = &title {
        (label.lines.len() as i32 * 24).max(options.title_height)
    } else {
        0
    };

    let participant_top = options.margin + title_block_height;
    for p in &mut participants {
        p.y = participant_top;
    }

    let events_top = participant_top + participant_height + 24;
    let mut messages = Vec::new();
    let mut notes = Vec::new();
    let mut groups: Vec<GroupBox> = Vec::new();
    let mut structures = Vec::new();
    let mut open_groups: Vec<usize> = Vec::new();
    let mut event_rows: i32 = 0;
    let mut autonumber = AutonumberState::default();

    for event in &document.events {
        match &event.kind {
            SequenceEventKind::Message {
                from,
                to,
                arrow,
                label,
                from_virtual,
                to_virtual,
            } => {
                let y = events_top + (event_rows * options.message_row_height);
                let (x1, x2) = message_x_bounds(
                    from,
                    to,
                    *from_virtual,
                    *to_virtual,
                    &centers_by_id,
                    &options,
                );
                let label = autonumber.apply(label.clone());
                let label_lines = message_label_lines(label.as_deref(), x1, x2, &options);
                let row_units = (label_lines.len() as i32).max(1);
                messages.push(MessageLine {
                    from_id: from.clone(),
                    to_id: to.clone(),
                    x1,
                    y,
                    x2,
                    arrow: arrow.clone(),
                    label,
                    label_lines,
                    from_virtual: *from_virtual,
                    to_virtual: *to_virtual,
                });
                event_rows += row_units;
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
                    let label_lines = message_label_lines(label.as_deref(), x1, x2, &options);
                    let row_units = (label_lines.len() as i32).max(1);
                    messages.push(MessageLine {
                        from_id: from_id.clone(),
                        to_id: to_id.clone(),
                        x1,
                        y,
                        x2,
                        arrow: "-->".to_string(),
                        label,
                        label_lines,
                        from_virtual: None,
                        to_virtual: None,
                    });
                    event_rows += row_units;
                }
            }
            SequenceEventKind::Autonumber(raw) => {
                autonumber.update(raw.as_deref());
            }
            SequenceEventKind::Note {
                target,
                text,
                position,
            } => {
                let y = events_top + (event_rows * options.message_row_height);
                let (content_width, text_lines) = multiline_metrics(text);
                let width_from_text =
                    content_width + (options.note_padding * 2) + NOTE_TEXT_WIDTH_GUARD_PX;
                let width = options.note_width.max(width_from_text);
                let height = (text_lines * TEXT_LINE_HEIGHT) + (options.note_padding * 2);
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
                        groups[ix].separators.push(GroupSeparator {
                            y,
                            label: label.clone(),
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
                            x,
                            y,
                            width,
                            height,
                            separators: Vec::new(),
                        });
                        event_rows += row_units_for_height(height, options.message_row_height);
                        continue;
                    } else {
                        groups.push(GroupBox {
                            kind: kind.clone(),
                            label: label.clone(),
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
            SequenceEventKind::Spacer => {
                let y = events_top + (event_rows * options.message_row_height);
                let (x1, x2) = structure_bounds(&centers_by_id, &options);
                structures.push(StructureLine {
                    kind: StructureKind::Spacer,
                    y,
                    x1,
                    x2,
                    label: None,
                });
                event_rows += 1;
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

    let events_height = if event_rows > 0 {
        (event_rows - 1) * options.message_row_height
    } else {
        0
    };

    let lifeline_start = participant_top + participant_height;
    let row_based_content_end = events_top + events_height;
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
    let footboxes = if document.footbox_visible {
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
    let lifelines = participants
        .iter()
        .map(|p| Lifeline {
            participant_id: p.id.clone(),
            x: p.x + p.width / 2,
            y1: lifeline_start,
            y2: lifeline_end,
        })
        .collect::<Vec<_>>();

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
        if m.label_lines.is_empty() {
            continue;
        }
        let tx = ((m.x1 + m.x2) / 2) + 2;
        for line in &m.label_lines {
            let text_width = estimate_text_px_width(line);
            let right = tx + (text_width / 2);
            width = width.max(right + options.margin);
        }
    }

    let min_bottom = if footboxes.is_empty() {
        lifeline_end + options.footer_height
    } else {
        lifeline_end + participant_height
    };
    let height = (min_bottom + options.margin).max(participant_top + participant_height + 80);

    Scene {
        width,
        height,
        title,
        participants,
        footboxes,
        lifelines,
        messages,
        notes,
        groups,
        structures,
        style: document.style.clone(),
    }
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
    for word in words {
        let word_len = word.chars().count();
        if current.is_empty() {
            if word_len <= max_chars {
                current.push_str(word);
            } else {
                for chunk in chunk_text(word, max_chars) {
                    lines.push(chunk);
                }
            }
            continue;
        }

        let next_len = current.chars().count() + 1 + word_len;
        if next_len <= max_chars {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            if word_len <= max_chars {
                current = word.to_string();
            } else {
                let mut chunks = chunk_text(word, max_chars);
                let tail = chunks.pop().unwrap_or_default();
                lines.extend(chunks);
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
            min_left
        } else {
            mid_center - (width / 2)
        }
    } else {
        options.margin
    };

    (x.max(options.margin), width)
}

fn group_horizontal_bounds(
    kind: &str,
    label: Option<&str>,
    bounds_by_id: &BTreeMap<String, (i32, i32)>,
    options: &LayoutOptions,
) -> (i32, i32) {
    let (min_content_width, _) = group_content_min_size(kind, label);
    if let Some(raw) = label {
        if let Some(target_spec) = raw.strip_prefix("over ") {
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
    let Some(label) = label else {
        return (0, 0);
    };
    let mut lines = label.split('\n');
    let header = lines.next().unwrap_or("");
    let header_text = format!("{kind} {header}");
    let mut max_width = estimate_text_px_width(header_text.trim());
    let mut height = GROUP_HEADER_BASELINE_Y + GROUP_BOTTOM_PADDING;

    if kind.eq_ignore_ascii_case("ref") {
        let mut body_lines = 0;
        for line in lines {
            max_width = max_width.max(estimate_text_px_width(line));
            body_lines += 1;
        }
        if body_lines > 0 {
            height = GROUP_REF_BODY_BASELINE_Y
                + ((body_lines - 1) * TEXT_LINE_HEIGHT)
                + GROUP_BOTTOM_PADDING;
        }
    }

    (max_width + (GROUP_TEXT_INSET_X * 2), height)
}

fn estimate_text_px_width(line: &str) -> i32 {
    (line.chars().count() as i32) * 7
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
    options: &LayoutOptions,
) -> Vec<String> {
    let Some(label) = label else {
        return Vec::new();
    };
    let min_span = (options.participant_spacing - 20).max(56);
    let span_px = (x2 - x1).abs().max(min_span) - 16;
    let tx = ((x1 + x2) / 2) + 2;
    let max_chars_by_span = (span_px / 7).max(1) as usize;
    let max_chars_by_left_edge = ((tx * 2) / 7).max(1) as usize;
    let max_chars = max_chars_by_span.min(max_chars_by_left_edge);
    normalize_label_lines(label, max_chars, options.text_overflow_policy)
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
    next: u64,
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
            if self.next == 0 {
                self.next = 1;
            }
            if self.step == 0 {
                self.step = 1;
            }
            self.enabled = true;
            return;
        }

        let parsed = parse_autonumber_command(value);
        if parsed.resume_only {
            if self.next == 0 {
                self.next = 1;
            }
        } else {
            self.next = parsed.start.unwrap_or(1);
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
        if self.next == 0 {
            self.next = 1;
        }
        if self.step == 0 {
            self.step = 1;
        }

        let number = self.next;
        self.next = self.next.saturating_add(self.step);
        let number = format_autonumber(number, self.format.as_deref());
        match label {
            Some(text) if !text.is_empty() => Some(format!("{number} {text}")),
            _ => Some(number.to_string()),
        }
    }
}

#[derive(Debug, Default)]
struct ParsedAutonumber {
    resume_only: bool,
    start: Option<u64>,
    step: Option<u64>,
    format: Option<String>,
}

fn parse_autonumber_command(raw: &str) -> ParsedAutonumber {
    let mut parsed = ParsedAutonumber::default();
    let mut rest = raw.trim();

    if rest.eq_ignore_ascii_case("resume") {
        parsed.resume_only = true;
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

    let nums: Vec<u64> = rest
        .split_whitespace()
        .filter_map(|v| v.parse::<u64>().ok())
        .collect();
    if parsed.resume_only {
        if let Some(step) = nums.first() {
            parsed.step = Some(*step);
        }
    } else {
        parsed.start = nums.first().copied();
        parsed.step = nums.get(1).copied();
    }

    if parsed.format.is_none() {
        parsed.format = rest.split_whitespace().find_map(|part| {
            if part.parse::<u64>().is_ok() {
                None
            } else {
                Some(part.to_string())
            }
        });
    }

    parsed
}

fn trailing_quoted_format(raw: &str) -> Option<(String, &str)> {
    let trimmed = raw.trim_end();
    let end = trimmed.strip_suffix('"')?;
    let start = end.rfind('"')?;
    let format = end[start + 1..].to_string();
    let prefix = &end[..start];
    Some((format, prefix))
}

fn format_autonumber(value: u64, format: Option<&str>) -> String {
    let Some(format) = format else {
        return value.to_string();
    };
    let fmt = format.trim();
    if fmt.is_empty() {
        return value.to_string();
    }

    if fmt.contains('#') {
        return fmt.replace('#', &value.to_string());
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
        return format!("{fmt}{value}");
    }

    let padded = format!("{:0width$}", value, width = longest_zero_run);
    let prefix = &fmt[..run_start];
    let suffix = &fmt[run_start + longest_zero_run..];
    format!("{prefix}{padded}{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        Participant, ParticipantRole, SequenceDocument, SequenceEvent, SequenceEventKind,
    };
    use crate::source::Span;
    use crate::theme::SequenceStyle;

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
            title: None,
            header: None,
            footer: None,
            caption: None,
            legend: None,
            skinparams: vec![],
            style: SequenceStyle::default(),
            footbox_visible: true,
            warnings: vec![],
        };
        let scene = layout(&doc, LayoutOptions::default());
        assert_eq!(scene.messages.len(), 1);
        assert_eq!(scene.messages[0].arrow, "-->");
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
        let mut auton = AutonumberState::default();
        auton.update(None);
        assert_eq!(auton.apply(Some(String::new())).as_deref(), Some("1"));
        assert_eq!(format_autonumber(7, Some("")), "7");
        assert_eq!(format_autonumber(7, Some("item")), "item7");
        assert_eq!(format_autonumber(7, Some("n=#")), "n=7");
    }

    #[test]
    fn autonumber_resume_and_zero_state_fallbacks_are_covered() {
        let mut state = AutonumberState::default();
        state.update(Some("resume"));
        assert_eq!(state.next, 1);

        let mut state = AutonumberState {
            enabled: true,
            next: 0,
            step: 0,
            format: None,
        };
        assert_eq!(state.apply(None).as_deref(), Some("1"));

        let mut state = AutonumberState {
            enabled: false,
            next: 8,
            step: 0,
            format: None,
        };
        state.update(Some("resume"));
        assert_eq!(state.step, 1);

        let bounds: BTreeMap<String, (i32, i32)> = BTreeMap::new();
        let (_gx, gw) = group_horizontal_bounds("group", None, &bounds, &LayoutOptions::default());
        assert!(gw >= LayoutOptions::default().participant_width + 64);
    }
}
