use std::collections::BTreeMap;

use crate::model::{SequenceDocument, SequenceEventKind, SequencePage};
use crate::normalize;
use crate::scene::{
    GroupBox, GroupSeparator, Label, LayoutOptions, Lifeline, MessageLine, NoteBox, ParticipantBox,
    Scene,
};

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

    let mut max_participant_right = options.margin;
    for (idx, participant) in document.participants.iter().enumerate() {
        let x = options.margin + (idx as i32 * options.participant_spacing);
        let center_x = x + options.participant_width / 2;
        max_participant_right = max_participant_right.max(x + options.participant_width);
        centers_by_id.insert(participant.id.clone(), center_x);

        participants.push(ParticipantBox {
            id: participant.id.clone(),
            display: participant.display.clone(),
            x,
            y: options.margin,
            width: options.participant_width,
            height: options.participant_height,
        });
    }

    let title = document.title.as_ref().map(|text| Label {
        x: options.margin,
        y: options.margin,
        text: text.clone(),
    });

    let title_block_height = if title.is_some() {
        options.title_height
    } else {
        0
    };

    let participant_top = options.margin + title_block_height;
    for p in &mut participants {
        p.y = participant_top;
    }

    let events_top = participant_top + options.participant_height + 24;
    let mut messages = Vec::new();
    let mut notes = Vec::new();
    let mut groups: Vec<GroupBox> = Vec::new();
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
            } => {
                let y = events_top + (event_rows * options.message_row_height);
                let (x1, x2) = message_x_bounds(from, to, &centers_by_id, &options);
                messages.push(MessageLine {
                    from_id: from.clone(),
                    to_id: to.clone(),
                    x1,
                    y,
                    x2,
                    arrow: arrow.clone(),
                    label: autonumber.apply(label.clone()),
                });
                event_rows += 1;
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
                    messages.push(MessageLine {
                        from_id: from_id.clone(),
                        to_id: to_id.clone(),
                        x1,
                        y,
                        x2,
                        arrow: "-->".to_string(),
                        label: autonumber.apply(label.clone()),
                    });
                    event_rows += 1;
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
                let text_lines = text.lines().count().max(1) as i32;
                let height = (text_lines * 16) + (options.note_padding * 2);

                let x = if let Some(target_spec) = target {
                    let centers = note_target_centers(target_spec, &centers_by_id, &options);
                    let min_center = *centers.iter().min().unwrap_or(&default_center(&options));
                    let max_center = *centers.iter().max().unwrap_or(&default_center(&options));
                    let mid_center = (min_center + max_center) / 2;
                    if position.eq_ignore_ascii_case("left") {
                        min_center - options.note_width - 12
                    } else if position.eq_ignore_ascii_case("right") {
                        max_center + 12
                    } else {
                        mid_center - (options.note_width / 2)
                    }
                } else {
                    options.margin
                };

                notes.push(NoteBox {
                    target_id: target.clone(),
                    x,
                    y,
                    width: options.note_width,
                    height,
                    text: text.clone(),
                });
                event_rows += 1;
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
                        group_horizontal_bounds(label.as_deref(), &centers_by_id, &options);
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
                event_rows += 1;
            }
            SequenceEventKind::GroupEnd => {
                let y = events_top + (event_rows * options.message_row_height);
                if let Some(ix) = open_groups.pop() {
                    groups[ix].height = (y - groups[ix].y) + options.message_row_height;
                }
                event_rows += 1;
            }
            _ => {}
        }
    }

    let end_y = events_top + (event_rows * options.message_row_height);
    while let Some(ix) = open_groups.pop() {
        groups[ix].height = (end_y - groups[ix].y).max(options.message_row_height);
    }

    let events_height = if event_rows > 0 {
        (event_rows - 1) * options.message_row_height
    } else {
        0
    };

    let lifeline_start = participant_top + options.participant_height;
    let lifeline_end = events_top + events_height + options.footer_height;
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

    let height =
        (lifeline_end + options.margin).max(participant_top + options.participant_height + 80);

    Scene {
        width,
        height,
        title,
        participants,
        lifelines,
        messages,
        notes,
        groups,
    }
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

fn group_horizontal_bounds(
    label: Option<&str>,
    centers_by_id: &BTreeMap<String, i32>,
    options: &LayoutOptions,
) -> (i32, i32) {
    if let Some(raw) = label {
        if let Some(target_spec) = raw.strip_prefix("over ") {
            let centers = note_target_centers(target_spec.trim(), centers_by_id, options);
            if !centers.is_empty() {
                let min_center = *centers.iter().min().unwrap_or(&default_center(options));
                let max_center = *centers.iter().max().unwrap_or(&default_center(options));
                let x = min_center - (options.note_width / 2);
                let width = (max_center - min_center) + options.note_width;
                return (x, width.max(options.note_width));
            }
        }
    }
    (
        options.margin,
        (centers_by_id.len() as i32 * options.participant_spacing)
            .max(options.participant_width + 64),
    )
}

fn message_x_bounds(
    from: &str,
    to: &str,
    centers_by_id: &BTreeMap<String, i32>,
    options: &LayoutOptions,
) -> (i32, i32) {
    let default_center = options.margin + options.participant_width / 2;
    let from_center = centers_by_id.get(from).copied().unwrap_or(default_center);
    let to_center = centers_by_id.get(to).copied().unwrap_or(default_center);
    let virtual_endpoint = "[*]";
    let side_offset = 56;
    let self_loop_width = 44;

    if from == to && from != virtual_endpoint {
        return (from_center, from_center + self_loop_width);
    }
    if from == virtual_endpoint && to == virtual_endpoint {
        return (default_center - side_offset, default_center + side_offset);
    }
    if from == virtual_endpoint {
        return (to_center - side_offset, to_center);
    }
    if to == virtual_endpoint {
        return (from_center, from_center + side_offset);
    }
    (from_center, to_center)
}

#[derive(Debug, Default)]
struct AutonumberState {
    enabled: bool,
    next: u64,
}

impl AutonumberState {
    fn update(&mut self, raw: Option<&str>) {
        let value = raw.map(str::trim).unwrap_or("");
        if value.eq_ignore_ascii_case("stop") {
            self.enabled = false;
            return;
        }

        if value.is_empty() {
            if self.next == 0 {
                self.next = 1;
            }
            self.enabled = true;
            return;
        }

        let start = value
            .split_whitespace()
            .next()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(1);
        self.next = start;
        self.enabled = true;
    }

    fn apply(&mut self, label: Option<String>) -> Option<String> {
        if !self.enabled {
            return label;
        }
        if self.next == 0 {
            self.next = 1;
        }

        let number = self.next;
        self.next += 1;
        match label {
            Some(text) if !text.is_empty() => Some(format!("{number} {text}")),
            _ => Some(number.to_string()),
        }
    }
}
