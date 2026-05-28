use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn finish_sequence_scene(
    document: &SequencePage,
    options: LayoutOptions,
    header: Option<Label>,
    title: Option<Label>,
    mut participants: Vec<ParticipantBox>,
    mut messages: Vec<MessageLine>,
    mut activations: Vec<ActivationBox>,
    mut lifecycle_markers: Vec<LifecycleMarker>,
    mut notes: Vec<NoteBox>,
    mut groups: Vec<GroupBox>,
    mut open_groups: Vec<usize>,
    mut structures: Vec<StructureLine>,
    activation_stack: Vec<OpenActivation>,
    centers_by_id: &BTreeMap<String, i32>,
    bounds_by_id: &BTreeMap<String, (i32, i32)>,
    max_participant_right: i32,
    participant_top: i32,
    participant_height: i32,
    events_top: i32,
    event_rows: i32,
    title_max_chars: usize,
) -> Scene {
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
        .map(|p| {
            // For mid-flow created participants the box is rendered at creation time
            // (p.y > participant_top), so the lifeline starts from the bottom of that
            // inline box rather than from the standard top header band.
            let y1 = if p.y > participant_top {
                p.y + p.height
            } else {
                lifeline_start
            };
            Lifeline {
                participant_id: p.id.clone(),
                x: p.x + p.width / 2,
                y1,
                y2: lifeline_end,
            }
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

    let mut header = header;
    if let Some(label) = &mut header {
        label.x = metadata_label_x(label.align, width, options.margin);
    }

    let mut title = title;
    if let Some(label) = &mut title {
        label.x = metadata_label_x(label.align, width, options.margin);
    }

    let mut lower_metadata_y = min_bottom + METADATA_LINE_HEIGHT;
    let caption = caption_lines.map(|lines| {
        let label = Label {
            x: options.margin,
            y: lower_metadata_y,
            lines,
            align: Default::default(),
        };
        lower_metadata_y += metadata_label_block_height(Some(&label));
        label
    });
    let footer = footer_lines.map(|lines| Label {
        x: metadata_label_x(document.footer_align, width, options.margin),
        y: lower_metadata_y,
        lines,
        align: document.footer_align,
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
