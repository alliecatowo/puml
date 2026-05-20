use super::*;
use crate::model::SequenceParticipantGroup;

mod autonumber;
mod groups;
mod message;
mod options;
mod pagination;
mod participants;

pub(super) use self::options::strip_legend_pos_prefix;
pub(super) use self::pagination::paginate;

use autonumber::*;
use groups::*;
use message::*;
use options::*;
use participants::*;

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
            | StatementKind::SaltGridRow { .. } => {
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
