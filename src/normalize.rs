use std::collections::BTreeMap;

use crate::ast::{DiagramKind, Document, ParticipantRole as AstRole, StatementKind};
use crate::diagnostic::Diagnostic;
use crate::model::{
    FamilyDocument, FamilyNode, FamilyNodeKind, FamilyRelation as ModelFamilyRelation,
    NormalizedDocument, Participant, ParticipantRole, SequenceDocument, SequenceEvent,
    SequenceEventKind, SequencePage, TimelineChronologyEvent, TimelineConstraint, TimelineDocument,
    TimelineMilestone, TimelineTask, VirtualEndpoint, VirtualEndpointKind, VirtualEndpointSide,
};
use crate::theme::{
    classify_sequence_skinparam, resolve_sequence_theme_preset, SequenceSkinParamSupport,
    SequenceSkinParamValue, SequenceStyle,
};

#[derive(Debug, Clone, Default)]
pub struct NormalizeOptions {
    pub include_root: Option<std::path::PathBuf>,
}

pub fn normalize(document: Document) -> Result<SequenceDocument, Diagnostic> {
    normalize_with_options(document, &NormalizeOptions::default())
}

pub fn normalize_family(document: Document) -> Result<NormalizedDocument, Diagnostic> {
    normalize_family_with_options(document, &NormalizeOptions::default())
}

pub fn normalize_family_with_options(
    document: Document,
    options: &NormalizeOptions,
) -> Result<NormalizedDocument, Diagnostic> {
    match document.kind {
        DiagramKind::Sequence => {
            normalize_with_options(document, options).map(NormalizedDocument::Sequence)
        }
        DiagramKind::Class | DiagramKind::Object | DiagramKind::UseCase => {
            normalize_stub_family(document).map(NormalizedDocument::Family)
        }
        DiagramKind::Gantt | DiagramKind::Chronology | DiagramKind::Activity => {
            normalize_timeline_baseline(document).map(NormalizedDocument::Timeline)
        }
        DiagramKind::MindMap
        | DiagramKind::Wbs
        | DiagramKind::Component
        | DiagramKind::Deployment
        | DiagramKind::State
        | DiagramKind::Timing => Err(unsupported_family_diagnostic(document.kind)),
        DiagramKind::Unknown => Err(Diagnostic::error(
            "[E_FAMILY_UNKNOWN] unable to detect supported diagram family; expected sequence/class/object/usecase/gantt/chronology syntax",
        )),
    }
}

fn normalize_timeline_baseline(document: Document) -> Result<TimelineDocument, Diagnostic> {
    let mut tasks = Vec::new();
    let mut milestones = Vec::new();
    let mut constraints = Vec::new();
    let mut chronology_events = Vec::new();
    let mut title = None;
    let mut header = None;
    let mut footer = None;
    let mut caption = None;
    let mut legend = None;

    for stmt in document.statements {
        match stmt.kind {
            StatementKind::GanttTaskDecl { name } => tasks.push(TimelineTask { name }),
            StatementKind::GanttMilestoneDecl { name } => {
                milestones.push(TimelineMilestone { name })
            }
            StatementKind::GanttConstraint {
                subject,
                kind,
                target,
            } => constraints.push(TimelineConstraint {
                subject,
                kind,
                target,
            }),
            StatementKind::ChronologyHappensOn { subject, when } => {
                chronology_events.push(TimelineChronologyEvent { subject, when })
            }
            StatementKind::Title(v) => title = Some(v),
            StatementKind::Header(v) => header = Some(v),
            StatementKind::Footer(v) => footer = Some(v),
            StatementKind::Caption(v) => caption = Some(v),
            StatementKind::Legend(v) => legend = Some(v),
            StatementKind::SkinParam { .. }
            | StatementKind::Theme(_)
            | StatementKind::Pragma(_) => {}
            StatementKind::Unknown(line) => {
                return Err(Diagnostic::error(line).with_span(stmt.span));
            }
            _ => {
                let family = family_kind_name(document.kind);
                return Err(Diagnostic::error(format!(
                    "[E_TIMELINE_BASELINE_UNSUPPORTED] unsupported {family} syntax in baseline slice"
                ))
                .with_span(stmt.span));
            }
        }
    }

    Ok(TimelineDocument {
        kind: document.kind,
        tasks,
        milestones,
        constraints,
        chronology_events,
        title,
        header,
        footer,
        caption,
        legend,
        warnings: Vec::new(),
    })
}

fn normalize_stub_family(document: Document) -> Result<FamilyDocument, Diagnostic> {
    let family_kind = document.kind;
    let node_kind = match family_kind {
        DiagramKind::Class => FamilyNodeKind::Class,
        DiagramKind::Object => FamilyNodeKind::Object,
        DiagramKind::UseCase => FamilyNodeKind::UseCase,
        _ => {
            return Err(Diagnostic::error(
                "[E_FAMILY_STUB_INTERNAL] invalid family for stub normalization",
            ));
        }
    };

    let mut nodes = Vec::new();
    let mut relations = Vec::new();
    let mut title = None;
    let mut header = None;
    let mut footer = None;
    let mut caption = None;
    let mut legend = None;

    for stmt in document.statements {
        match stmt.kind {
            StatementKind::ClassDecl(decl) => {
                if node_kind != FamilyNodeKind::Class {
                    return Err(Diagnostic::error(format!(
                        "[E_FAMILY_MIXED] mixed diagram families are not supported: found class declaration in {} diagram",
                        family_kind_name(family_kind)
                    ))
                    .with_span(stmt.span));
                }
                nodes.push(FamilyNode {
                    kind: FamilyNodeKind::Class,
                    name: decl.name,
                    alias: decl.alias,
                    members: decl.members,
                });
            }
            StatementKind::ObjectDecl(decl) => {
                if node_kind != FamilyNodeKind::Object {
                    return Err(Diagnostic::error(format!(
                        "[E_FAMILY_MIXED] mixed diagram families are not supported: found object declaration in {} diagram",
                        family_kind_name(family_kind)
                    ))
                    .with_span(stmt.span));
                }
                nodes.push(FamilyNode {
                    kind: FamilyNodeKind::Object,
                    name: decl.name,
                    alias: decl.alias,
                    members: decl.members,
                });
            }
            StatementKind::UseCaseDecl(decl) => {
                if node_kind != FamilyNodeKind::UseCase {
                    return Err(Diagnostic::error(format!(
                        "[E_FAMILY_MIXED] mixed diagram families are not supported: found usecase declaration in {} diagram",
                        family_kind_name(family_kind)
                    ))
                    .with_span(stmt.span));
                }
                nodes.push(FamilyNode {
                    kind: FamilyNodeKind::UseCase,
                    name: decl.name,
                    alias: decl.alias,
                    members: decl.members,
                });
            }
            StatementKind::FamilyRelation(rel) => relations.push(ModelFamilyRelation {
                from: rel.from,
                to: rel.to,
                arrow: rel.arrow,
                label: rel.label,
            }),
            StatementKind::Title(v) => title = Some(v),
            StatementKind::Header(v) => header = Some(v),
            StatementKind::Footer(v) => footer = Some(v),
            StatementKind::Caption(v) => caption = Some(v),
            StatementKind::Legend(v) => legend = Some(v),
            StatementKind::SkinParam { .. }
            | StatementKind::Theme(_)
            | StatementKind::Pragma(_)
            | StatementKind::Include(_)
            | StatementKind::Define { .. }
            | StatementKind::Undef(_) => {}
            StatementKind::Unknown(line) => {
                return Err(Diagnostic::error(format!(
                    "[E_PARSE_UNKNOWN] unsupported syntax: `{}`",
                    line
                ))
                .with_span(stmt.span));
            }
            _ => {
                return Err(Diagnostic::error(format!(
                    "[E_FAMILY_STUB_UNSUPPORTED] unsupported {} syntax in bootstrap slice",
                    family_kind_name(family_kind)
                ))
                .with_span(stmt.span));
            }
        }
    }

    Ok(FamilyDocument {
        kind: family_kind,
        nodes,
        relations,
        title,
        header,
        footer,
        caption,
        legend,
        warnings: Vec::new(),
    })
}

fn family_kind_name(kind: DiagramKind) -> &'static str {
    match kind {
        DiagramKind::Sequence => "sequence",
        DiagramKind::Class => "class",
        DiagramKind::Object => "object",
        DiagramKind::UseCase => "usecase",
        DiagramKind::MindMap => "mindmap",
        DiagramKind::Wbs => "wbs",
        DiagramKind::Gantt => "gantt",
        DiagramKind::Chronology => "chronology",
        DiagramKind::Component => "component",
        DiagramKind::Deployment => "deployment",
        DiagramKind::State => "state",
        DiagramKind::Activity => "activity",
        DiagramKind::Timing => "timing",
        DiagramKind::Unknown => "unknown",
    }
}

pub fn paginate(document: &SequenceDocument) -> Vec<SequencePage> {
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
        events: events.to_vec(),
        title,
        header: document.header.clone(),
        footer: document.footer.clone(),
        caption: document.caption.clone(),
        legend: document.legend.clone(),
        skinparams: document.skinparams.clone(),
        style: document.style.clone(),
        footbox_visible: document.footbox_visible,
        warnings: document.warnings.clone(),
    }
}

fn cleaned_title(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned)
}

pub fn normalize_with_options(
    document: Document,
    _options: &NormalizeOptions,
) -> Result<SequenceDocument, Diagnostic> {
    if document.kind != DiagramKind::Sequence {
        return Err(unsupported_family_diagnostic(document.kind));
    }

    let mut participants: Vec<Participant> = Vec::new();
    let mut participant_ix: BTreeMap<String, usize> = BTreeMap::new();
    let mut events = Vec::new();

    let mut title = None;
    let mut header = None;
    let mut footer = None;
    let mut caption = None;
    let mut legend = None;
    let mut skinparams = Vec::new();
    let mut footbox_visible = true;
    let mut style = SequenceStyle::default();
    let mut warnings: Vec<Diagnostic> = Vec::new();
    let mut alive_by_id: BTreeMap<String, bool> = BTreeMap::new();
    let mut activation_stack: Vec<ActivationFrame> = Vec::new();
    let mut group_stack: Vec<GroupFrame> = Vec::new();
    let mut last_message: Option<(String, String)> = None;
    let mut ignore_newpage = false;

    for stmt in document.statements {
        match stmt.kind {
            StatementKind::Participant(p) => {
                mark_group_content(&mut group_stack);
                let id = p.alias.unwrap_or_else(|| p.name.clone());
                let display = p.display.unwrap_or_else(|| p.name.clone());
                upsert_participant(
                    &mut participants,
                    &mut participant_ix,
                    id,
                    display,
                    map_role(p.role),
                    true,
                )
                .map_err(|e| Diagnostic::error(e).with_span(stmt.span))?;
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
                let directions = if parsed_arrow.bidirectional {
                    vec![
                        (m.from.clone(), m.to.clone()),
                        (m.to.clone(), m.from.clone()),
                    ]
                } else {
                    vec![(m.from.clone(), m.to.clone())]
                };

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
                        position: n.position,
                        target: n.target,
                        text: n.text,
                    },
                });
            }
            StatementKind::Group(g) => {
                if g.kind == "end" {
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
                } else if g.kind == "else" {
                    let Some(top) = group_stack.last_mut() else {
                        return Err(Diagnostic::error(
                            "[E_GROUP_ELSE_UNMATCHED] `else` without an open group block",
                        )
                        .with_span(stmt.span));
                    };
                    if !allows_else(top.kind.as_str()) {
                        return Err(Diagnostic::error(format!(
                            "[E_GROUP_ELSE_KIND] `else` is not valid inside `{}`",
                            top.kind
                        ))
                        .with_span(stmt.span));
                    }
                    if rejects_empty_group(top.kind.as_str()) && !top.branch_has_content {
                        return Err(Diagnostic::error(format!(
                            "[E_GROUP_EMPTY_BRANCH] `{}` block contains an empty branch before `else`",
                            top.kind
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
            StatementKind::Legend(v) => legend = Some(v),
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
                    warnings.push(
                        Diagnostic::warning(
                            "[W_PRAGMA_TEOZ_UNSUPPORTED] !pragma teoz is not supported yet; continuing with default sequence layout semantics"
                                .to_string(),
                        )
                        .with_span(stmt.span),
                    );
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
            StatementKind::Spacer => {
                mark_group_content(&mut group_stack);
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: SequenceEventKind::Spacer,
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
            StatementKind::ClassDecl(_)
            | StatementKind::ObjectDecl(_)
            | StatementKind::UseCaseDecl(_)
            | StatementKind::FamilyRelation(_)
            | StatementKind::GanttTaskDecl { .. }
            | StatementKind::GanttMilestoneDecl { .. }
            | StatementKind::GanttConstraint { .. }
            | StatementKind::ChronologyHappensOn { .. } => {
                return Err(Diagnostic::error(
                    "[E_FAMILY_MIXED] mixed diagram families are not supported in one document",
                )
                .with_span(stmt.span));
            }
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

    warnings.sort_by(|a, b| {
        let sa = a.span.map(|s| s.start).unwrap_or_default();
        let sb = b.span.map(|s| s.start).unwrap_or_default();
        (a.message.as_str(), sa).cmp(&(b.message.as_str(), sb))
    });

    Ok(SequenceDocument {
        participants,
        events,
        title,
        header,
        footer,
        caption,
        legend,
        skinparams,
        style,
        footbox_visible,
        warnings,
    })
}

fn unsupported_family_diagnostic(kind: DiagramKind) -> Diagnostic {
    let (code, family) = match kind {
        DiagramKind::Component => ("E_FAMILY_COMPONENT_UNSUPPORTED", "component"),
        DiagramKind::Deployment => ("E_FAMILY_DEPLOYMENT_UNSUPPORTED", "deployment"),
        DiagramKind::State => ("E_FAMILY_STATE_UNSUPPORTED", "state"),
        DiagramKind::Activity => ("E_FAMILY_ACTIVITY_UNSUPPORTED", "activity"),
        DiagramKind::Timing => ("E_FAMILY_TIMING_UNSUPPORTED", "timing"),
        DiagramKind::MindMap => ("E_FAMILY_MINDMAP_UNSUPPORTED", "mindmap"),
        DiagramKind::Wbs => ("E_FAMILY_WBS_UNSUPPORTED", "wbs"),
        DiagramKind::Gantt => ("E_FAMILY_GANTT_UNSUPPORTED", "gantt"),
        DiagramKind::Chronology => ("E_FAMILY_CHRONOLOGY_UNSUPPORTED", "chronology"),
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
    bidirectional: bool,
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
    let render_arrow = if bidirectional {
        if stripped.contains("--") {
            "-->".to_string()
        } else {
            "->".to_string()
        }
    } else {
        canonical_base
    };
    Some(ParsedMessageArrow {
        render_arrow,
        bidirectional,
        left_modifier,
        right_modifier,
    })
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
    if matches!(tokens.first(), Some(token) if token.eq_ignore_ascii_case("resume")) {
        resume = true;
        tokens.remove(0);
    }

    let mut idx = 0usize;
    let expected_numbers = if resume { 1 } else { 2 };
    while idx < tokens.len() && idx < expected_numbers && tokens[idx].parse::<u64>().is_ok() {
        idx += 1;
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
