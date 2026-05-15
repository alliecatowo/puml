use std::collections::BTreeMap;

use crate::model::{SequenceDocument, SequenceEventKind};
use crate::scene::{Label, LayoutOptions, Lifeline, MessageLine, NoteBox, ParticipantBox, Scene};

pub fn layout(document: &SequenceDocument, options: LayoutOptions) -> Scene {
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
    let mut event_rows: i32 = 0;

    for event in &document.events {
        match &event.kind {
            SequenceEventKind::Message {
                from, to, label, ..
            } => {
                let y = events_top + (event_rows * options.message_row_height);
                let x1 = centers_by_id
                    .get(from)
                    .copied()
                    .unwrap_or(options.margin + options.participant_width / 2);
                let x2 = centers_by_id
                    .get(to)
                    .copied()
                    .unwrap_or(options.margin + options.participant_width / 2);
                messages.push(MessageLine {
                    from_id: from.clone(),
                    to_id: to.clone(),
                    x1,
                    y,
                    x2,
                    label: label.clone(),
                });
                event_rows += 1;
            }
            SequenceEventKind::Note {
                target,
                text,
                position,
            } => {
                let y = events_top + (event_rows * options.message_row_height);
                let text_lines = text.lines().count().max(1) as i32;
                let height = (text_lines * 16) + (options.note_padding * 2);

                let x = if let Some(target_id) = target {
                    let center = centers_by_id
                        .get(target_id)
                        .copied()
                        .unwrap_or(options.margin + options.participant_width / 2);
                    if position.eq_ignore_ascii_case("left") {
                        center - options.note_width - 12
                    } else if position.eq_ignore_ascii_case("right") {
                        center + 12
                    } else {
                        center - (options.note_width / 2)
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
            _ => {}
        }
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
    }
}
