use super::*;
use crate::model::SequenceParticipantGroup;

pub(super) fn paginate(document: &SequenceDocument) -> Vec<SequencePage> {
    let mut pages = Vec::new();
    let mut page_events = Vec::new();
    let mut current_title = document.title.clone();

    for event in &document.events {
        if let SequenceEventKind::NewPage(next_title) = &event.kind {
            pages.push(page_from(document, &page_events, current_title.clone()));
            page_events.clear();
            current_title = cleaned_title(next_title).or_else(|| document.title.clone());
            continue;
        }
        page_events.push(event.clone());
    }

    pages.push(page_from(document, &page_events, current_title));
    pages
}

fn page_from(
    document: &SequenceDocument,
    events: &[SequenceEvent],
    title: Option<String>,
) -> SequencePage {
    SequencePage {
        participants: document.participants.clone(),
        participant_groups: document.participant_groups.clone(),
        events: events.to_vec(),
        teoz: document.teoz,
        title,
        header: document.header.clone(),
        footer: document.footer.clone(),
        caption: document.caption.clone(),
        legend: document.legend.clone(),
        skinparams: document.skinparams.clone(),
        style: document.style.clone(),
        footbox_visible: document.footbox_visible,
        scale: document.scale.clone(),
        legend_halign: document.legend_halign,
        legend_valign: document.legend_valign,
        warnings: document.warnings.clone(),
        hide_unlinked: document.hide_unlinked,
        hidden_participants: document.hidden_participants.clone(),
    }
}

fn cleaned_title(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned)
}

fn parse_teoz_pragma(lower: &str) -> Option<bool> {
    let mut parts = lower.split_whitespace();
    if parts.next()? != "teoz" {
        return None;
    }
    match parts.next() {
        None => Some(true),
        Some("true" | "on" | "yes") => Some(true),
        Some("false" | "off" | "no") => Some(false),
        Some(_) => Some(true),
    }
}

pub(super) fn normalize_with_options(
    document: Document,
    _options: &NormalizeOptions,
) -> Result<SequenceDocument, Diagnostic> {
    if document.kind != DiagramKind::Sequence {
        return Err(unsupported_family_diagnostic(document.kind));
    }

    let mut participants: Vec<Participant> = Vec::new();
    let mut participant_ix: BTreeMap<String, usize> = BTreeMap::new();
    let mut participant_order: BTreeMap<String, i32> = BTreeMap::new();
    let mut events = Vec::new();

    let mut title = None;
    let mut header = None;
    let mut footer = None;
    let mut caption = None;
    let mut legend = None;
    let mut skinparams = Vec::new();
    let mut footbox_visible = true;
    let mut style = SequenceStyle::default();
    let mut scale: Option<ScaleSpec> = None;
    let mut legend_halign = LegendHAlign::default();
    let mut legend_valign = LegendVAlign::default();
    let mut warnings: Vec<Diagnostic> = Vec::new();
    let mut teoz = false;
    let mut alive_by_id: BTreeMap<String, bool> = BTreeMap::new();
    let mut activation_stack: Vec<ActivationFrame> = Vec::new();
    let mut group_stack: Vec<GroupFrame> = Vec::new();
    let mut participant_group_stack: Vec<SequenceParticipantGroup> = Vec::new();
    let mut participant_groups = Vec::new();
    let mut last_message: Option<(String, String)> = None;
    let mut ignore_newpage = false;
    let mut hide_unlinked = false;

    for stmt in document.statements {
        match stmt.kind {
            StatementKind::HideUnlinked => {
                hide_unlinked = true;
            }
            StatementKind::Participant(p) => {
                mark_group_content(&mut group_stack);
                let id = p.alias.unwrap_or_else(|| p.name.clone());
                let display = p.display.unwrap_or_else(|| p.name.clone());
                if let Some(order) = p.order {
                    participant_order.insert(id.clone(), order);
                }
                upsert_participant(
                    &mut participants,
                    &mut participant_ix,
                    id.clone(),
                    display,
                    map_role(p.role),
                    true,
                )
                .map_err(|e| Diagnostic::error(e).with_span(stmt.span))?;
                for group in &mut participant_group_stack {
                    if !group.participant_ids.iter().any(|member| member == &id) {
                        group.participant_ids.push(id.clone());
                    }
                }
            }
            StatementKind::Message(m) => {
                mark_group_content(&mut group_stack);
                let parsed_arrow = parse_message_arrow(&m.arrow).ok_or_else(|| {
                    Diagnostic::error(format!(
                        "[E_ARROW_INVALID] malformed sequence arrow syntax: `{}`",
                        m.arrow
                    ))
                    .with_span(stmt.span)
                })?;
                // Determine the canonical (sender, receiver) pair for the event.
                // • Bidirectional arrows (<->) keep from/to as written; the
                //   render arrow already carries heads on both sides.
                // • Reversed left-facing arrows (e.g. `Bob <- Alice`) swap
                //   from/to so that x1 always belongs to the sender.
                let (event_from, event_to) = if parsed_arrow.reversed {
                    (m.to.clone(), m.from.clone())
                } else {
                    (m.from.clone(), m.to.clone())
                };
                let directions = vec![(event_from, event_to)];

                for (from, to) in directions {
                    let from_virtual = virtual_endpoint(from.as_str(), true);
                    let to_virtual = virtual_endpoint(to.as_str(), false);
                    validate_virtual_endpoint_combination(
                        stmt.span,
                        &from,
                        &to,
                        from_virtual,
                        to_virtual,
                    )?;
                    validate_and_touch_message_lifecycle(
                        stmt.span,
                        &from,
                        &to,
                        &mut participants,
                        &mut participant_ix,
                        &mut alive_by_id,
                    )?;
                    if !is_virtual_endpoint(&from) && !is_virtual_endpoint(&to) {
                        last_message = Some((from.clone(), to.clone()));
                    }
                    events.push(SequenceEvent {
                        span: stmt.span,
                        kind: SequenceEventKind::Message {
                            from: from.clone(),
                            to: to.clone(),
                            arrow: parsed_arrow.render_arrow.clone(),
                            label: m.label.clone(),
                            style: SequenceMessageStyle {
                                color: m.style.color.clone(),
                                hidden: m.style.hidden,
                                dashed: m.style.dashed,
                                dotted: m.style.dotted,
                                thickness: m.style.thickness,
                                parallel: m.style.parallel,
                            },
                            from_virtual,
                            to_virtual,
                        },
                    });
                }
                apply_lifecycle_shortcuts(
                    stmt.span,
                    &m.from,
                    &m.to,
                    &parsed_arrow,
                    &mut participants,
                    &mut participant_ix,
                    &mut alive_by_id,
                    &mut activation_stack,
                    &mut events,
                )?;
            }
            StatementKind::Note(n) => {
                mark_group_content(&mut group_stack);
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: SequenceEventKind::Note {
                        kind: n.kind,
                        position: n.position,
                        target: n.target,
                        text: n.text,
                    },
                });
            }
            StatementKind::Group(g) => {
                if g.kind.eq_ignore_ascii_case("box") {
                    let (label, color) = parse_participant_group_label(g.label.as_deref());
                    participant_group_stack.push(SequenceParticipantGroup {
                        label,
                        color,
                        participant_ids: Vec::new(),
                    });
                    continue;
                }
                if g.kind == "end" {
                    if g.label.as_deref() == Some("box") {
                        let Some(group) = participant_group_stack.pop() else {
                            return Err(Diagnostic::error(
                                "[E_BOX_END_UNMATCHED] `end box` without an open box block",
                            )
                            .with_span(stmt.span));
                        };
                        if !group.participant_ids.is_empty() {
                            participant_groups.push(group);
                        }
                        continue;
                    }
                    let Some(open) = group_stack.pop() else {
                        return Err(Diagnostic::error(
                            "[E_GROUP_END_UNMATCHED] `end` without an open group block",
                        )
                        .with_span(stmt.span));
                    };
                    if let Some(expected) = g.label.as_deref() {
                        if expected != open.kind {
                            return Err(Diagnostic::error(format!(
                                "[E_GROUP_END_KIND] `end {}` does not match open `{}` block",
                                expected, open.kind
                            ))
                            .with_span(stmt.span));
                        }
                    }
                    if rejects_empty_group(open.kind.as_str()) && !open.branch_has_content {
                        return Err(Diagnostic::error(format!(
                            "[E_GROUP_EMPTY] `{}` block must not be empty",
                            open.kind
                        ))
                        .with_span(stmt.span));
                    }
                    events.push(SequenceEvent {
                        span: stmt.span,
                        kind: SequenceEventKind::GroupEnd,
                    });
                } else if g.kind == "else" || g.kind == "also" {
                    // `also` is the parallel-branch continuation keyword for `par`
                    // blocks; it behaves like `else` inside `alt` (fixes #780).
                    let Some(top) = group_stack.last_mut() else {
                        let keyword = &g.kind;
                        return Err(Diagnostic::error(format!(
                            "[E_GROUP_ELSE_UNMATCHED] `{keyword}` without an open group block",
                        ))
                        .with_span(stmt.span));
                    };
                    if !allows_branch_separator(top.kind.as_str(), g.kind.as_str()) {
                        return Err(Diagnostic::error(format!(
                            "[E_GROUP_ELSE_KIND] `{}` is not valid inside `{}`",
                            g.kind, top.kind
                        ))
                        .with_span(stmt.span));
                    }
                    if rejects_empty_group(top.kind.as_str()) && !top.branch_has_content {
                        return Err(Diagnostic::error(format!(
                            "[E_GROUP_EMPTY_BRANCH] `{}` block contains an empty branch before `{}`",
                            top.kind, g.kind
                        ))
                        .with_span(stmt.span));
                    }
                    top.branch_has_content = false;
                    events.push(SequenceEvent {
                        span: stmt.span,
                        kind: SequenceEventKind::GroupStart {
                            kind: g.kind,
                            label: g.label,
                        },
                    });
                } else {
                    mark_group_content(&mut group_stack);
                    if g.kind != "ref" {
                        group_stack.push(GroupFrame {
                            kind: g.kind.clone(),
                            span: stmt.span,
                            branch_has_content: false,
                        });
                    } else {
                        // For `ref over A, B, C` auto-create any participants
                        // that haven't been declared yet so the ref box can
                        // span their lifelines.
                        if let Some(lbl) = &g.label {
                            let first_line = lbl.lines().next().unwrap_or("");
                            if let Some(over_spec) = first_line.strip_prefix("over ") {
                                for id in over_spec
                                    .split(',')
                                    .map(str::trim)
                                    .filter(|s| !s.is_empty())
                                {
                                    ensure_implicit(&mut participants, &mut participant_ix, id);
                                }
                            }
                        }
                    }
                    events.push(SequenceEvent {
                        span: stmt.span,
                        kind: SequenceEventKind::GroupStart {
                            kind: g.kind,
                            label: g.label,
                        },
                    });
                }
            }
            StatementKind::Title(v) => title = Some(v),
            StatementKind::Header(v) => header = Some(v),
            StatementKind::Footer(v) => footer = Some(v),
            StatementKind::Caption(v) => caption = Some(v),
            StatementKind::Legend(v) => {
                // Parse packed "LEGEND_POS:<pos>\n<text>" format emitted by the parser
                // when a multiline legend block has positioning qualifiers.
                if let Some(rest) = v.strip_prefix("LEGEND_POS:") {
                    if let Some(newline_idx) = rest.find('\n') {
                        let pos = &rest[..newline_idx];
                        let text = &rest[newline_idx + 1..];
                        legend = Some(text.to_string());
                        let lower_pos = pos.to_ascii_lowercase();
                        for token in lower_pos.split_whitespace() {
                            match token {
                                "left" => legend_halign = LegendHAlign::Left,
                                "right" => legend_halign = LegendHAlign::Right,
                                "center" => legend_halign = LegendHAlign::Center,
                                "top" => legend_valign = LegendVAlign::Top,
                                "bottom" => legend_valign = LegendVAlign::Bottom,
                                _ => {}
                            }
                        }
                    } else {
                        // Just position, no text
                        let lower_pos = rest.to_ascii_lowercase();
                        for token in lower_pos.split_whitespace() {
                            match token {
                                "left" => legend_halign = LegendHAlign::Left,
                                "right" => legend_halign = LegendHAlign::Right,
                                "center" => legend_halign = LegendHAlign::Center,
                                "top" => legend_valign = LegendVAlign::Top,
                                "bottom" => legend_valign = LegendVAlign::Bottom,
                                _ => {}
                            }
                        }
                    }
                } else {
                    legend = Some(v);
                }
            }
            StatementKind::SkinParam { key, value } => {
                mark_group_content(&mut group_stack);
                skinparams.push((key.clone(), value.clone()));
                match classify_sequence_skinparam(&key, &value) {
                    SequenceSkinParamSupport::SupportedNoop => {}
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::FootboxVisible(visible),
                    ) => {
                        footbox_visible = visible;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ArrowColor(color),
                    ) => style.arrow_color = color,
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::LifelineBorderColor(color),
                    ) => style.lifeline_border_color = color,
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ParticipantBackgroundColor(color),
                    ) => style.participant_background_color = color,
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ParticipantBorderColor(color),
                    ) => style.participant_border_color = color,
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::NoteBackgroundColor(color),
                    ) => style.note_background_color = color,
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::NoteBorderColor(color),
                    ) => style.note_border_color = color,
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::GroupBackgroundColor(color),
                    ) => style.group_background_color = color,
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::GroupBorderColor(color),
                    ) => style.group_border_color = color,
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::RoundCorner(n),
                    ) => style.round_corner = n,
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::Shadowing(enabled),
                    ) => style.shadowing = enabled,
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::DefaultFontName(name),
                    ) => style.default_font_name = Some(name),
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::DefaultFontSize(sz),
                    ) => style.default_font_size = Some(sz),
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::BackgroundColor(color),
                    ) => style.background_color = Some(color),
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::DefaultTextAlignment(align),
                    ) => style.text_alignment = align,
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ParticipantPadding(n),
                    ) => style.participant_padding = Some(n),
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::BoxPadding(n),
                    ) => style.box_padding = Some(n),
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::MessageAlign(align),
                    ) => style.message_align = align,
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ResponseMessageBelowArrow(enabled),
                    ) => style.response_message_below_arrow = enabled,
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::LifelineThickness(n),
                    ) => style.lifeline_thickness = Some(n),
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::MessageLineColor(color),
                    ) => style.message_line_color = Some(color),
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ReferenceBackgroundColor(color),
                    ) => style.reference_background_color = Some(color),
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ReferenceBorderColor(color),
                    ) => style.reference_border_color = Some(color),
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::GroupHeaderFontColor(color),
                    ) => style.group_header_font_color = Some(color),
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::GroupHeaderFontStyle(fs),
                    ) => style.group_header_font_style = fs,
                    SequenceSkinParamSupport::UnsupportedValue => {
                        warnings.push(
                            Diagnostic::warning(format!(
                                "[W_SKINPARAM_UNSUPPORTED_VALUE] unsupported value `{}` for skinparam `{}`",
                                value, key
                            ))
                            .with_span(stmt.span),
                        );
                    }
                    SequenceSkinParamSupport::UnsupportedKey => {
                        warnings.push(
                            Diagnostic::warning(format!(
                                "[W_SKINPARAM_UNSUPPORTED] unsupported skinparam `{}`",
                                key
                            ))
                            .with_span(stmt.span),
                        );
                    }
                }
            }
            StatementKind::Theme(name) => {
                mark_group_content(&mut group_stack);
                let preset = resolve_sequence_theme_preset(&name)
                    .map_err(|msg| Diagnostic::error(msg).with_span(stmt.span))?;
                style = preset.style;
            }
            StatementKind::Pragma(value) => {
                mark_group_content(&mut group_stack);
                let trimmed = value.trim();
                let lower = trimmed.to_ascii_lowercase();
                if lower.starts_with("teoz ") || lower == "teoz" {
                    teoz = parse_teoz_pragma(&lower).unwrap_or(true);
                } else if lower == "sequencemessagespan true"
                    || lower == "sequence message span true"
                {
                    style.sequence_message_span = true;
                } else if lower == "sequencemessagespan false"
                    || lower == "sequence message span false"
                {
                    style.sequence_message_span = false;
                } else {
                    warnings.push(
                        Diagnostic::warning(format!(
                            "[W_PRAGMA_UNSUPPORTED] unsupported pragma `{}`",
                            trimmed
                        ))
                        .with_span(stmt.span),
                    );
                }
            }
            StatementKind::Footbox(v) => {
                mark_group_content(&mut group_stack);
                footbox_visible = v
            }
            StatementKind::Delay(v) => {
                mark_group_content(&mut group_stack);
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: SequenceEventKind::Delay(v),
                })
            }
            StatementKind::Divider(v) => {
                mark_group_content(&mut group_stack);
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: SequenceEventKind::Divider(v),
                })
            }
            StatementKind::Separator(v) => {
                mark_group_content(&mut group_stack);
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: SequenceEventKind::Separator(v),
                })
            }
            StatementKind::Spacer(pixels) => {
                mark_group_content(&mut group_stack);
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: SequenceEventKind::Spacer(pixels),
                })
            }
            StatementKind::NewPage(v) => {
                mark_group_content(&mut group_stack);
                if !ignore_newpage {
                    events.push(SequenceEvent {
                        span: stmt.span,
                        kind: SequenceEventKind::NewPage(v),
                    });
                }
            }
            StatementKind::IgnoreNewPage => {
                mark_group_content(&mut group_stack);
                ignore_newpage = true;
            }
            StatementKind::Autonumber(v) => {
                mark_group_content(&mut group_stack);
                if let Some(raw) = v.as_deref() {
                    validate_autonumber_raw(raw).map_err(|reason| {
                        Diagnostic::error(format!("[E_AUTONUMBER_FORMAT_UNSUPPORTED] {reason}"))
                            .with_span(stmt.span)
                    })?;
                }
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: SequenceEventKind::Autonumber(
                        v.as_deref().and_then(canonicalize_autonumber_raw),
                    ),
                })
            }
            StatementKind::Activate(id) => {
                mark_group_content(&mut group_stack);
                ensure_implicit(&mut participants, &mut participant_ix, &id);
                if !is_alive(&alive_by_id, &id) {
                    return Err(Diagnostic::error(format!(
                        "[E_LIFECYCLE_ACTIVATE_DESTROYED] cannot activate destroyed participant `{}`",
                        id
                    ))
                    .with_span(stmt.span));
                }
                alive_by_id.insert(id.clone(), true);
                let caller = match &last_message {
                    Some((from, to)) if to == &id => Some(from.clone()),
                    _ => activation_stack.last().map(|f| f.participant.clone()),
                };
                activation_stack.push(ActivationFrame {
                    participant: id.clone(),
                    caller,
                });
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: SequenceEventKind::Activate(id),
                });
            }
            StatementKind::Deactivate(id) => {
                mark_group_content(&mut group_stack);
                ensure_implicit(&mut participants, &mut participant_ix, &id);
                if !is_alive(&alive_by_id, &id) {
                    return Err(Diagnostic::error(format!(
                        "[E_LIFECYCLE_DEACTIVATE_DESTROYED] cannot deactivate destroyed participant `{}`",
                        id
                    ))
                    .with_span(stmt.span));
                }
                alive_by_id.insert(id.clone(), true);
                match activation_stack.last() {
                    Some(frame) if frame.participant == id => {
                        activation_stack.pop();
                    }
                    Some(frame) => {
                        return Err(Diagnostic::error(format!(
                            "[E_LIFECYCLE_DEACTIVATE_ORDER] deactivate `{}` does not match current activation `{}`",
                            id, frame.participant
                        ))
                        .with_span(stmt.span));
                    }
                    None => {
                        return Err(Diagnostic::error(format!(
                            "[E_LIFECYCLE_DEACTIVATE_EMPTY] cannot deactivate `{}` without an active activation",
                            id
                        ))
                        .with_span(stmt.span));
                    }
                }
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: SequenceEventKind::Deactivate(id),
                });
            }
            StatementKind::Destroy(id) => {
                mark_group_content(&mut group_stack);
                ensure_implicit(&mut participants, &mut participant_ix, &id);
                if !is_alive(&alive_by_id, &id) {
                    return Err(Diagnostic::error(format!(
                        "[E_LIFECYCLE_DESTROY_TWICE] participant `{}` is already destroyed",
                        id
                    ))
                    .with_span(stmt.span));
                }
                if activation_stack.iter().any(|f| f.participant == id) {
                    return Err(Diagnostic::error(format!(
                        "[E_LIFECYCLE_DESTROY_ACTIVE] cannot destroy active participant `{}`",
                        id
                    ))
                    .with_span(stmt.span));
                }
                alive_by_id.insert(id.clone(), false);
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: SequenceEventKind::Destroy(id),
                });
            }
            StatementKind::Create(id) => {
                mark_group_content(&mut group_stack);
                ensure_implicit(&mut participants, &mut participant_ix, &id);
                if alive_by_id.get(&id).copied() == Some(true) {
                    return Err(Diagnostic::error(format!(
                        "[E_LIFECYCLE_CREATE_EXISTING] participant `{}` already exists; destroy before create",
                        id
                    ))
                    .with_span(stmt.span));
                }
                alive_by_id.insert(id.clone(), true);
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: SequenceEventKind::Create(id),
                });
            }
            StatementKind::Return(v) => {
                mark_group_content(&mut group_stack);
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: infer_return_event(stmt.span, v, &mut activation_stack, &last_message)?,
                })
            }
            StatementKind::Include(_) | StatementKind::Define { .. } | StatementKind::Undef(_) => {
                // Preprocessor directives should be expanded before normalization.
            }
            StatementKind::RawBlockContent(_) => {
                // Raw block content is only meaningful in dedicated raw-body families
                // (json/yaml/nwdiag/archimate); ignore in sequence normalization.
            }
            StatementKind::Scale(body) => {
                mark_group_content(&mut group_stack);
                scale = parse_scale_spec(&body).or(scale);
            }
            StatementKind::LegendPos(pos) => {
                mark_group_content(&mut group_stack);
                let lower = pos.to_ascii_lowercase();
                for token in lower.split_whitespace() {
                    match token {
                        "left" => legend_halign = LegendHAlign::Left,
                        "right" => legend_halign = LegendHAlign::Right,
                        "center" => legend_halign = LegendHAlign::Center,
                        "top" => legend_valign = LegendVAlign::Top,
                        "bottom" => legend_valign = LegendVAlign::Bottom,
                        _ => {}
                    }
                }
            }
            StatementKind::ClassDecl(_)
            | StatementKind::ObjectDecl(_)
            | StatementKind::UseCaseDecl(_)
            | StatementKind::FamilyRelation(_)
            | StatementKind::StateDecl(_)
            | StatementKind::StateTransition(_)
            | StatementKind::StateInternalAction(_)
            | StatementKind::StateRegionDivider
            | StatementKind::StateHistory { .. }
            | StatementKind::GanttTaskDecl { .. }
            | StatementKind::GanttMilestoneDecl { .. }
            | StatementKind::GanttConstraint { .. }
            | StatementKind::GanttCalendarClosed { .. }
            | StatementKind::GanttCalendarOpen { .. }
            | StatementKind::GanttCalendarClosedDateRange { .. }
            | StatementKind::GanttCalendarOpenDateRange { .. }
            | StatementKind::ChronologyHappensOn { .. }
            | StatementKind::ComponentDecl { .. }
            | StatementKind::ActivityStep(_)
            | StatementKind::TimingDecl { .. }
            | StatementKind::TimingEvent { .. }
            | StatementKind::RawBody(_)
            | StatementKind::ClassGroup { .. }
            | StatementKind::JsonProjection { .. }
            | StatementKind::YamlProjection { .. }
            | StatementKind::SaltGridRow { .. }
            | StatementKind::ChenEntityDecl(_)
            | StatementKind::ChenRelationshipDecl(_) => {
                return Err(Diagnostic::error(
                    "[E_FAMILY_MIXED] mixed diagram families are not supported in one document",
                )
                .with_span(stmt.span));
            }
            // Class-family-only options: silently ignored in sequence context
            StatementKind::SetOption { .. } | StatementKind::HideOption(_) => {}
            StatementKind::Unknown(line) => {
                if line.trim() == "---" {
                    continue;
                }
                return Err(Diagnostic::error(format!(
                    "[E_PARSE_UNKNOWN] unsupported syntax: `{}`",
                    line
                ))
                .with_span(stmt.span));
            }
        }
    }

    if let Some(open) = group_stack.pop() {
        return Err(Diagnostic::error(format!(
            "[E_GROUP_UNCLOSED] missing `end` for open `{}` block",
            open.kind
        ))
        .with_span(open.span));
    }

    let mut hidden_participants = Vec::new();

    // Apply `hide unlinked` filter: collect all participant IDs that appear in
    // sequence links or participant-scoped events, then drop explicit declarations
    // that never participate in the rendered interaction.
    if hide_unlinked {
        let mut referenced: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for ev in &events {
            match &ev.kind {
                SequenceEventKind::Message { from, to, .. } => {
                    // Only count real (non-virtual) endpoints.
                    if !is_virtual_endpoint(from) {
                        referenced.insert(from.clone());
                    }
                    if !is_virtual_endpoint(to) {
                        referenced.insert(to.clone());
                    }
                }
                SequenceEventKind::Note {
                    target: Some(t), ..
                } => {
                    // target may be comma-separated for `note over A,B`
                    for part in t.split(',') {
                        let id = part.trim();
                        if !id.is_empty() && !is_virtual_endpoint(id) {
                            referenced.insert(id.to_string());
                        }
                    }
                }
                SequenceEventKind::Note { target: None, .. } => {}
                SequenceEventKind::Activate(id)
                | SequenceEventKind::Deactivate(id)
                | SequenceEventKind::Destroy(id)
                | SequenceEventKind::Create(id) => {
                    referenced.insert(id.clone());
                }
                SequenceEventKind::Return { from, to, .. } => {
                    if let Some(f) = from {
                        referenced.insert(f.clone());
                    }
                    if let Some(t) = to {
                        referenced.insert(t.clone());
                    }
                }
                _ => {}
            }
        }
        participants.retain(|p| {
            let keep = !p.explicit || referenced.contains(&p.id);
            if !keep {
                hidden_participants.push(p.id.clone());
            }
            keep
        });
        if !hidden_participants.is_empty() {
            // Rebuild the participant index map.
            participant_ix.clear();
            for (idx, p) in participants.iter().enumerate() {
                participant_ix.insert(p.id.clone(), idx);
            }
        }
        if !participant_groups.is_empty() {
            participant_groups = participant_groups
                .into_iter()
                .filter_map(|mut group| {
                    group
                        .participant_ids
                        .retain(|id| !hidden_participants.contains(id));
                    (!group.participant_ids.is_empty()).then_some(group)
                })
                .collect();
        }
    }

    while let Some(group) = participant_group_stack.pop() {
        if !group.participant_ids.is_empty() {
            participant_groups.push(group);
        }
    }

    if !participant_order.is_empty() {
        let original_ix: BTreeMap<String, usize> = participants
            .iter()
            .enumerate()
            .map(|(idx, p)| (p.id.clone(), idx))
            .collect();
        participants.sort_by(|a, b| {
            let a_key = (
                participant_order.get(&a.id).copied().unwrap_or(i32::MAX),
                original_ix.get(&a.id).copied().unwrap_or(usize::MAX),
            );
            let b_key = (
                participant_order.get(&b.id).copied().unwrap_or(i32::MAX),
                original_ix.get(&b.id).copied().unwrap_or(usize::MAX),
            );
            a_key.cmp(&b_key)
        });
        participant_ix.clear();
        for (idx, p) in participants.iter().enumerate() {
            participant_ix.insert(p.id.clone(), idx);
        }
    }

    warnings.sort_by(|a, b| {
        let sa = a.span.map(|s| s.start).unwrap_or_default();
        let sb = b.span.map(|s| s.start).unwrap_or_default();
        (a.message.as_str(), sa).cmp(&(b.message.as_str(), sb))
    });

    Ok(SequenceDocument {
        participants,
        participant_groups,
        events,
        teoz,
        title,
        header,
        footer,
        caption,
        legend,
        skinparams,
        style,
        footbox_visible,
        scale,
        legend_halign,
        legend_valign,
        warnings,
        hide_unlinked,
        hidden_participants,
    })
}

fn parse_participant_group_label(raw: Option<&str>) -> (Option<String>, Option<String>) {
    let Some(raw) = raw.map(str::trim).filter(|raw| !raw.is_empty()) else {
        return (None, None);
    };

    let mut label = raw;
    let mut color = None;
    if let Some(last) = raw.split_whitespace().last() {
        if last.starts_with('#') && last.len() > 1 {
            color = Some(last.to_string());
            label = raw[..raw.len() - last.len()].trim_end();
        }
    }

    let label = label
        .trim()
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(label)
        .trim();

    ((!label.is_empty()).then_some(label.to_string()), color)
}

/// Strip the LEGEND_POS prefix from a packed legend value, returning just the text.
pub(super) fn strip_legend_pos_prefix(v: &str) -> String {
    if let Some(rest) = v.strip_prefix("LEGEND_POS:") {
        if let Some(nl) = rest.find('\n') {
            return rest[nl + 1..].to_string();
        }
        return String::new();
    }
    v.to_string()
}

/// Parse a scale body (everything after "scale ").
/// Supports:
///   "1.5"          → Factor(1.5)
///   "800*600"      → Fixed { width: 800, height: 600 }
///   "max 800"      → Max(800)
fn parse_scale_spec(body: &str) -> Option<ScaleSpec> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("max ") {
        let n: u32 = rest.trim().parse().ok()?;
        return Some(ScaleSpec::Max(n));
    }
    if let Some(idx) = trimmed.find('*') {
        let w: u32 = trimmed[..idx].trim().parse().ok()?;
        let h: u32 = trimmed[idx + 1..].trim().parse().ok()?;
        return Some(ScaleSpec::Fixed {
            width: w,
            height: h,
        });
    }
    let f: f64 = trimmed.parse().ok()?;
    if f > 0.0 {
        Some(ScaleSpec::Factor(f))
    } else {
        None
    }
}

fn unsupported_family_diagnostic(kind: DiagramKind) -> Diagnostic {
    let (code, family) = match kind {
        DiagramKind::Component => ("E_FAMILY_COMPONENT_UNSUPPORTED", "component"),
        DiagramKind::Deployment => ("E_FAMILY_DEPLOYMENT_UNSUPPORTED", "deployment"),
        DiagramKind::State => ("E_FAMILY_STATE_UNSUPPORTED", "state"),
        DiagramKind::Activity => ("E_FAMILY_ACTIVITY_UNSUPPORTED", "activity"),
        DiagramKind::Timing => ("E_FAMILY_TIMING_UNSUPPORTED", "timing"),
        DiagramKind::Gantt => ("E_FAMILY_GANTT_UNSUPPORTED", "gantt"),
        DiagramKind::Chronology => ("E_FAMILY_CHRONOLOGY_UNSUPPORTED", "chronology"),
        DiagramKind::Salt => ("E_FAMILY_SALT_UNSUPPORTED", "salt"),
        _ => ("E_FAMILY_UNSUPPORTED", "unknown"),
    };

    Diagnostic::error_code(
        code,
        format!(
            "diagram family `{family}` is not implemented yet; sequence is currently supported"
        ),
    )
}

fn is_alive(alive_by_id: &BTreeMap<String, bool>, id: &str) -> bool {
    alive_by_id.get(id).copied().unwrap_or(true)
}

#[derive(Debug, Clone)]
struct ActivationFrame {
    participant: String,
    caller: Option<String>,
}

#[derive(Debug, Clone)]
struct GroupFrame {
    kind: String,
    span: crate::source::Span,
    branch_has_content: bool,
}

fn mark_group_content(group_stack: &mut [GroupFrame]) {
    for frame in group_stack {
        frame.branch_has_content = true;
    }
}

fn allows_else(kind: &str) -> bool {
    matches!(kind, "alt" | "par" | "critical")
}

/// Returns `true` if `separator` is a valid branch-separator keyword inside a
/// group of type `group_kind`.  `else` works in `alt`, `par`, and `critical`.
/// `also` is the `par`-specific parallel-branch continuation (PlantUML parity,
/// fixes #780).
fn allows_branch_separator(group_kind: &str, separator: &str) -> bool {
    match separator {
        "also" => matches!(group_kind, "par"),
        _ => allows_else(group_kind),
    }
}

fn rejects_empty_group(kind: &str) -> bool {
    matches!(kind, "alt" | "par" | "critical")
}

fn infer_return_event(
    span: crate::source::Span,
    label: Option<String>,
    activation_stack: &mut Vec<ActivationFrame>,
    last_message: &Option<(String, String)>,
) -> Result<SequenceEventKind, Diagnostic> {
    if activation_stack.is_empty() {
        if let Some((from, to)) = last_message {
            return Ok(SequenceEventKind::Return {
                label,
                from: Some(to.clone()),
                to: Some(from.clone()),
            });
        }
    }
    let Some(frame) = activation_stack.pop() else {
        return Err(Diagnostic::error(
            "[E_RETURN_INFER_EMPTY] cannot infer `return` sender/target without an active activation",
        )
        .with_span(span));
    };

    let Some(caller) = frame.caller else {
        return Err(Diagnostic::error(format!(
            "[E_RETURN_INFER_CALLER] cannot infer `return` target for `{}`; use an explicit return message instead",
            frame.participant
        ))
        .with_span(span));
    };

    Ok(SequenceEventKind::Return {
        label,
        from: Some(frame.participant),
        to: Some(caller),
    })
}

fn ensure_implicit(
    participants: &mut Vec<Participant>,
    index: &mut BTreeMap<String, usize>,
    id: &str,
) {
    if index.contains_key(id) {
        return;
    }
    let pos = participants.len();
    participants.push(Participant {
        id: id.to_string(),
        display: id.to_string(),
        role: ParticipantRole::Participant,
        explicit: false,
    });
    index.insert(id.to_string(), pos);
}

fn upsert_participant(
    participants: &mut Vec<Participant>,
    index: &mut BTreeMap<String, usize>,
    id: String,
    display: String,
    role: ParticipantRole,
    explicit: bool,
) -> Result<(), String> {
    if let Some(ix) = index.get(&id).copied() {
        if explicit && participants[ix].explicit {
            return Err(format!(
                "[E_PARTICIPANT_DUPLICATE] duplicate participant id/alias `{}`",
                id
            ));
        }
        participants[ix].display = display;
        participants[ix].role = role;
        participants[ix].explicit = explicit;
        return Ok(());
    }

    let pos = participants.len();
    participants.push(Participant {
        id: id.clone(),
        display,
        role,
        explicit,
    });
    index.insert(id, pos);
    Ok(())
}

fn map_role(role: AstRole) -> ParticipantRole {
    match role {
        AstRole::Participant => ParticipantRole::Participant,
        AstRole::Actor => ParticipantRole::Actor,
        AstRole::Boundary => ParticipantRole::Boundary,
        AstRole::Control => ParticipantRole::Control,
        AstRole::Entity => ParticipantRole::Entity,
        AstRole::Database => ParticipantRole::Database,
        AstRole::Collections => ParticipantRole::Collections,
        AstRole::Queue => ParticipantRole::Queue,
    }
}

fn is_virtual_endpoint(id: &str) -> bool {
    matches!(id, "[*]" | "[" | "]" | "[o" | "o]" | "[x" | "x]")
}

fn virtual_endpoint(id: &str, is_from: bool) -> Option<VirtualEndpoint> {
    let (side, kind) = match id {
        "[" => (VirtualEndpointSide::Left, VirtualEndpointKind::Plain),
        "]" => (VirtualEndpointSide::Right, VirtualEndpointKind::Plain),
        "[o" => (VirtualEndpointSide::Left, VirtualEndpointKind::Circle),
        "o]" => (VirtualEndpointSide::Right, VirtualEndpointKind::Circle),
        "[x" => (VirtualEndpointSide::Left, VirtualEndpointKind::Cross),
        "x]" => (VirtualEndpointSide::Right, VirtualEndpointKind::Cross),
        "[*]" => (
            if is_from {
                VirtualEndpointSide::Left
            } else {
                VirtualEndpointSide::Right
            },
            VirtualEndpointKind::Filled,
        ),
        _ => return None,
    };
    Some(VirtualEndpoint { side, kind })
}

fn validate_virtual_endpoint_combination(
    span: crate::source::Span,
    from: &str,
    to: &str,
    from_virtual: Option<VirtualEndpoint>,
    to_virtual: Option<VirtualEndpoint>,
) -> Result<(), Diagnostic> {
    if from_virtual.is_some() && to_virtual.is_some() {
        return Err(Diagnostic::error(format!(
            "[E_ENDPOINT_COMBINATION] virtual endpoint messages must include at least one concrete participant: `{}` -> `{}`",
            from, to
        ))
        .with_span(span));
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct ParsedMessageArrow {
    render_arrow: String,
    /// True when the original arrow was left-facing (e.g. `<-`).  The caller
    /// must swap from/to before storing the event so that `from` always
    /// represents the semantic sender and x1 is the sender's centre.
    reversed: bool,
    left_modifier: Option<String>,
    right_modifier: Option<String>,
}

fn parse_message_arrow(raw: &str) -> Option<ParsedMessageArrow> {
    let (base, left_modifier, right_modifier) = decode_arrow_modifiers(raw)?;
    let canonical_base = base.replace(['/', '\\'], "");
    if canonical_base.is_empty()
        || !canonical_base
            .chars()
            .all(|c| matches!(c, '-' | '<' | '>' | 'o' | 'x'))
    {
        return None;
    }
    let stripped_left = canonical_base
        .strip_prefix('o')
        .or_else(|| canonical_base.strip_prefix('x'))
        .unwrap_or(&canonical_base);
    let stripped = stripped_left
        .strip_suffix('o')
        .or_else(|| stripped_left.strip_suffix('x'))
        .unwrap_or(stripped_left);
    let bidirectional = matches!(stripped, "<->" | "<-->" | "<<->>" | "<<-->>");

    // Detect left-facing arrows (the LHS of the syntax is the receiver, not
    // the sender).  Bidirectional arrows are not reversed because they have
    // heads on both ends anyway.
    let reversed = !bidirectional
        && (stripped.starts_with("<<-") || stripped.starts_with("<-"))
        && !stripped.contains('>');

    let render_arrow = if bidirectional {
        // Emit a single render arrow that carries heads on both ends.
        if stripped.contains("--") {
            "<-->".to_string()
        } else {
            "<->".to_string()
        }
    } else if reversed {
        // Mirror the arrow to its right-facing equivalent so that the render
        // engine always sees x1 < x2 with a right-pointing head.
        mirror_arrow(&base)
    } else {
        base
    };
    Some(ParsedMessageArrow {
        render_arrow,
        reversed,
        left_modifier,
        right_modifier,
    })
}

/// Flip a left-facing arrow string to its right-facing mirror.
///
/// `<-` → `->`, `<--` → `-->`, `<<-` → `->>`, `<<--` → `-->>`
/// Endpoint markers (o/x) swap sides as well.
fn mirror_arrow(base: &str) -> String {
    let canonical = base.replace(['/', '\\'], "");
    // Strip endpoint markers.
    let left_marker = canonical.chars().next().filter(|c| matches!(c, 'o' | 'x'));
    let right_marker = canonical.chars().last().filter(|c| matches!(c, 'o' | 'x'));
    let inner = canonical
        .strip_prefix(|c| matches!(c, 'o' | 'x'))
        .unwrap_or(&canonical);
    let inner = inner
        .strip_suffix(|c| matches!(c, 'o' | 'x'))
        .unwrap_or(inner);

    // Map the dash-only core (<-, <--, <<-, <<--) to its mirror.
    let mirrored_core = match inner {
        "<-" => "->",
        "<--" => "-->",
        "<<-" => "->>",
        "<<--" => "-->>",
        // Fallback: just return the original.
        _ => return base.to_string(),
    };

    // Re-attach markers (swapped).
    let mut out = String::new();
    if let Some(m) = right_marker {
        out.push(m);
    }
    out.push_str(mirrored_core);
    if let Some(m) = left_marker {
        out.push(m);
    }
    out
}

fn decode_arrow_modifiers(raw: &str) -> Option<(String, Option<String>, Option<String>)> {
    let mut rest = raw;
    let mut left_modifier = None;
    let mut right_modifier = None;
    while let Some(ix) = rest.find("@L").or_else(|| rest.find("@R")) {
        let side = &rest[ix..ix + 2];
        let token = rest.get(ix + 2..ix + 4)?;
        if !matches!(token, "++" | "--" | "**" | "!!") {
            return None;
        }
        if side == "@L" {
            left_modifier = Some(token.to_string());
        } else {
            right_modifier = Some(token.to_string());
        }
        rest = &rest[..ix];
    }
    Some((rest.to_string(), left_modifier, right_modifier))
}

fn validate_and_touch_message_lifecycle(
    span: crate::source::Span,
    from: &str,
    to: &str,
    participants: &mut Vec<Participant>,
    participant_ix: &mut BTreeMap<String, usize>,
    alive_by_id: &mut BTreeMap<String, bool>,
) -> Result<(), Diagnostic> {
    let from_virtual = is_virtual_endpoint(from);
    let to_virtual = is_virtual_endpoint(to);
    if !from_virtual {
        ensure_implicit(participants, participant_ix, from);
    }
    if !to_virtual {
        ensure_implicit(participants, participant_ix, to);
    }
    if !from_virtual && !is_alive(alive_by_id, from) {
        return Err(Diagnostic::error(format!(
            "[E_LIFECYCLE_DESTROYED_SENDER] message sender `{}` is destroyed",
            from
        ))
        .with_span(span));
    }
    if !to_virtual && !is_alive(alive_by_id, to) {
        return Err(Diagnostic::error(format!(
            "[E_LIFECYCLE_DESTROYED_TARGET] message target `{}` is destroyed (recreate it before sending messages to it)",
            to
        ))
        .with_span(span));
    }
    if !from_virtual {
        alive_by_id.insert(from.to_string(), true);
    }
    if !to_virtual {
        alive_by_id.insert(to.to_string(), true);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn apply_lifecycle_shortcuts(
    span: crate::source::Span,
    from: &str,
    to: &str,
    parsed_arrow: &ParsedMessageArrow,
    participants: &mut Vec<Participant>,
    participant_ix: &mut BTreeMap<String, usize>,
    alive_by_id: &mut BTreeMap<String, bool>,
    activation_stack: &mut Vec<ActivationFrame>,
    events: &mut Vec<SequenceEvent>,
) -> Result<(), Diagnostic> {
    if let Some(token) = &parsed_arrow.left_modifier {
        let caller = shortcut_caller(from, to);
        apply_one_lifecycle_shortcut(
            span,
            from,
            token,
            caller,
            participants,
            participant_ix,
            alive_by_id,
            activation_stack,
            events,
        )?;
    }
    if let Some(token) = &parsed_arrow.right_modifier {
        let id = if token == "--" { from } else { to };
        let caller = shortcut_caller(id, if id == from { to } else { from });
        apply_one_lifecycle_shortcut(
            span,
            id,
            token,
            caller,
            participants,
            participant_ix,
            alive_by_id,
            activation_stack,
            events,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn apply_one_lifecycle_shortcut(
    span: crate::source::Span,
    id: &str,
    token: &str,
    caller: Option<String>,
    participants: &mut Vec<Participant>,
    participant_ix: &mut BTreeMap<String, usize>,
    alive_by_id: &mut BTreeMap<String, bool>,
    activation_stack: &mut Vec<ActivationFrame>,
    events: &mut Vec<SequenceEvent>,
) -> Result<(), Diagnostic> {
    if is_virtual_endpoint(id) {
        return Err(Diagnostic::error(format!(
            "[E_LIFECYCLE_SHORTCUT_VIRTUAL] cannot apply lifecycle shortcut `{}` to virtual endpoint",
            token
        ))
        .with_span(span));
    }
    ensure_implicit(participants, participant_ix, id);
    match token {
        "++" => {
            if !is_alive(alive_by_id, id) {
                return Err(Diagnostic::error(format!(
                    "[E_LIFECYCLE_ACTIVATE_DESTROYED] cannot activate destroyed participant `{}`",
                    id
                ))
                .with_span(span));
            }
            alive_by_id.insert(id.to_string(), true);
            activation_stack.push(ActivationFrame {
                participant: id.to_string(),
                caller,
            });
            events.push(SequenceEvent {
                span,
                kind: SequenceEventKind::Activate(id.to_string()),
            });
        }
        "--" => {
            if !is_alive(alive_by_id, id) {
                return Err(Diagnostic::error(format!(
                    "[E_LIFECYCLE_DEACTIVATE_DESTROYED] cannot deactivate destroyed participant `{}`",
                    id
                ))
                .with_span(span));
            }
            alive_by_id.insert(id.to_string(), true);
            match activation_stack.last() {
                Some(frame) if frame.participant == id => {
                    activation_stack.pop();
                }
                Some(frame) => {
                    return Err(Diagnostic::error(format!(
                        "[E_LIFECYCLE_DEACTIVATE_ORDER] deactivate `{}` does not match current activation `{}`",
                        id, frame.participant
                    ))
                    .with_span(span));
                }
                None => {
                    return Err(Diagnostic::error(format!(
                        "[E_LIFECYCLE_DEACTIVATE_EMPTY] cannot deactivate `{}` without an active activation",
                        id
                    ))
                    .with_span(span));
                }
            }
            events.push(SequenceEvent {
                span,
                kind: SequenceEventKind::Deactivate(id.to_string()),
            });
        }
        "**" => {
            alive_by_id.insert(id.to_string(), true);
            events.push(SequenceEvent {
                span,
                kind: SequenceEventKind::Create(id.to_string()),
            });
        }
        "!!" => {
            if !is_alive(alive_by_id, id) {
                return Err(Diagnostic::error(format!(
                    "[E_LIFECYCLE_DESTROY_TWICE] participant `{}` is already destroyed",
                    id
                ))
                .with_span(span));
            }
            if activation_stack.iter().any(|f| f.participant == id) {
                return Err(Diagnostic::error(format!(
                    "[E_LIFECYCLE_DESTROY_ACTIVE] cannot destroy active participant `{}`",
                    id
                ))
                .with_span(span));
            }
            alive_by_id.insert(id.to_string(), false);
            events.push(SequenceEvent {
                span,
                kind: SequenceEventKind::Destroy(id.to_string()),
            });
        }
        _ => {
            return Err(Diagnostic::error(format!(
                "[E_LIFECYCLE_SHORTCUT_INVALID] unknown lifecycle shortcut `{}`",
                token
            ))
            .with_span(span));
        }
    }
    Ok(())
}

fn shortcut_caller(active: &str, other: &str) -> Option<String> {
    if is_virtual_endpoint(active) || is_virtual_endpoint(other) {
        None
    } else {
        Some(other.to_string())
    }
}

fn canonicalize_autonumber_raw(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut out = String::with_capacity(trimmed.len());
    let mut in_quotes = false;
    let mut prev_space = false;
    for ch in trimmed.chars() {
        if ch == '"' {
            in_quotes = !in_quotes;
            prev_space = false;
            out.push(ch);
            continue;
        }
        if ch.is_whitespace() && !in_quotes {
            if !prev_space {
                out.push(' ');
            }
            prev_space = true;
            continue;
        }
        prev_space = false;
        out.push(ch);
    }
    Some(out.trim().to_string())
}

fn validate_autonumber_raw(raw: &str) -> Result<(), String> {
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("stop")
        || trimmed.eq_ignore_ascii_case("off")
        || trimmed.eq_ignore_ascii_case("resume")
    {
        return Ok(());
    }

    let (format, body) = if trimmed.contains('"') {
        let Some((format, before)) = trailing_quoted_format(trimmed) else {
            return Err("malformed quoted autonumber format; quote-delimited format must be the final token".to_string());
        };
        (Some(format), before.trim_end())
    } else {
        (None, trimmed)
    };

    let mut tokens: Vec<&str> = body.split_whitespace().collect();
    let mut resume = false;
    if tokens.len() == 2
        && tokens[0].eq_ignore_ascii_case("inc")
        && is_autonumber_increment_level(tokens[1])
    {
        return Ok(());
    }
    if matches!(tokens.first(), Some(token) if token.eq_ignore_ascii_case("resume")) {
        resume = true;
        tokens.remove(0);
    }

    let mut idx = 0usize;
    if resume {
        if idx < tokens.len() && tokens[idx].parse::<u64>().is_ok() {
            idx += 1;
        }
    } else if idx < tokens.len() {
        if is_autonumber_counter_token(tokens[idx]) {
            idx += 1;
            if idx < tokens.len() && tokens[idx].parse::<u64>().is_ok() {
                idx += 1;
            } else if idx < tokens.len() && looks_like_autonumber_counter_token(tokens[idx]) {
                return Err(
                    "unsupported autonumber syntax; increment must be an unsigned integer"
                        .to_string(),
                );
            }
        } else if looks_like_autonumber_counter_token(tokens[idx]) {
            return Err(
                "malformed dotted autonumber start; expected dot-separated unsigned integers"
                    .to_string(),
            );
        }
    }

    let unquoted_format = if idx < tokens.len() {
        let fmt = tokens[idx];
        idx += 1;
        Some(fmt)
    } else {
        None
    };

    if idx < tokens.len() {
        return Err(
            "unsupported autonumber syntax; expected `autonumber [start] [increment] [format]` or `autonumber resume [increment] [format]`".to_string(),
        );
    }

    if let Some(fmt) = format.or(unquoted_format.map(str::to_string)) {
        validate_autonumber_format(&fmt)?;
    }

    Ok(())
}

fn is_autonumber_counter_token(token: &str) -> bool {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return false;
    }
    trimmed
        .split(['.', ';', ',', ':'])
        .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
}

fn looks_like_autonumber_counter_token(token: &str) -> bool {
    let trimmed = token.trim();
    trimmed
        .bytes()
        .any(|b| matches!(b, b'.' | b';' | b',' | b':'))
        && trimmed
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'.' | b';' | b',' | b':'))
        && trimmed.bytes().any(|b| b.is_ascii_digit())
}

fn is_autonumber_increment_level(token: &str) -> bool {
    token.len() == 1 && token.bytes().all(|b| b.is_ascii_alphabetic())
}

fn trailing_quoted_format(raw: &str) -> Option<(String, &str)> {
    let trimmed = raw.trim_end();
    let end = trimmed.strip_suffix('"')?;
    let start = end.rfind('"')?;
    let format = end[start + 1..].to_string();
    let prefix = &end[..start];
    Some((format, prefix))
}

fn validate_autonumber_format(format: &str) -> Result<(), String> {
    let fmt = format.trim();
    if fmt.is_empty() {
        return Err("autonumber format must not be empty".to_string());
    }
    if fmt.contains('<') || fmt.contains('>') {
        return Err(
            "autonumber format does not support HTML tags in this deterministic subset".to_string(),
        );
    }
    if fmt.contains('"') {
        return Err("autonumber format must not contain an embedded quote".to_string());
    }
    Ok(())
}

/// Extract `<<stereotype>>` from an alias string like `"myAlias <<person>>"`.
/// Returns `(clean_alias, Option<FamilyNodeKind>)` where `clean_alias` has the
/// stereotype stripped.  When the stereotype is not a recognised C4 marker the
/// kind is `None` and the caller keeps `FamilyNodeKind::Object`.
pub(crate) fn extract_c4_stereotype(
    alias: Option<String>,
) -> (Option<String>, Option<FamilyNodeKind>) {
    use crate::model::FamilyNodeKind;
    let Some(raw) = alias else {
        return (None, None);
    };
    // Find `<<...>>`
    if let Some(start) = raw.find("<<") {
        if let Some(end) = raw[start..].find(">>") {
            let stereotype = raw[start + 2..start + end].trim().to_ascii_lowercase();
            let clean_alias = raw[..start].trim().to_string();
            let kind = match stereotype.as_str() {
                "person" => Some(FamilyNodeKind::C4Person),
                "external-person" => Some(FamilyNodeKind::C4PersonExt),
                "system" => Some(FamilyNodeKind::C4System),
                "external-system" => Some(FamilyNodeKind::C4SystemExt),
                "system-db" | "systemdb" => Some(FamilyNodeKind::C4SystemDb),
                "system-queue" | "systemqueue" => Some(FamilyNodeKind::C4SystemQueue),
                "container" => Some(FamilyNodeKind::C4Container),
                "external-container" => Some(FamilyNodeKind::C4ContainerExt),
                "container-db" | "containerdb" => Some(FamilyNodeKind::C4ContainerDb),
                "container-queue" | "containerqueue" => Some(FamilyNodeKind::C4ContainerQueue),
                "c4-component" | "component" => Some(FamilyNodeKind::C4Component),
                "external-c4-component" | "external-component" => {
                    Some(FamilyNodeKind::C4ComponentExt)
                }
                "component-db" | "componentdb" => Some(FamilyNodeKind::C4ComponentDb),
                "component-queue" | "componentqueue" => Some(FamilyNodeKind::C4ComponentQueue),
                "boundary" | "enterprise-boundary" | "system-boundary" | "container-boundary" => {
                    Some(FamilyNodeKind::C4Boundary)
                }
                _ => None,
            };
            let clean = if clean_alias.is_empty() {
                None
            } else {
                Some(clean_alias)
            };
            return (clean, kind);
        }
    }
    (Some(raw), None)
}
