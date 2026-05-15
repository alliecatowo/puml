use std::collections::BTreeMap;

use crate::ast::{DiagramKind, Document, ParticipantRole as AstRole, StatementKind};
use crate::diagnostic::Diagnostic;
use crate::model::{
    Participant, ParticipantRole, SequenceDocument, SequenceEvent, SequenceEventKind,
};

#[derive(Debug, Clone, Default)]
pub struct NormalizeOptions {
    pub include_root: Option<std::path::PathBuf>,
}

pub fn normalize(document: Document) -> Result<SequenceDocument, Diagnostic> {
    normalize_with_options(document, &NormalizeOptions::default())
}

pub fn normalize_with_options(
    document: Document,
    _options: &NormalizeOptions,
) -> Result<SequenceDocument, Diagnostic> {
    if document.kind != DiagramKind::Sequence {
        return Err(Diagnostic::error(
            "puml currently renders sequence diagrams only",
        ));
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
    let mut warnings: Vec<Diagnostic> = Vec::new();
    let mut alive_by_id: BTreeMap<String, bool> = BTreeMap::new();
    let mut activation_stack: Vec<ActivationFrame> = Vec::new();
    let mut last_message: Option<(String, String)> = None;

    for stmt in document.statements {
        match stmt.kind {
            StatementKind::Participant(p) => {
                let id = p.alias.unwrap_or_else(|| p.name.clone());
                let display = p.display.unwrap_or_else(|| p.name.clone());
                upsert_participant(
                    &mut participants,
                    &mut participant_ix,
                    id,
                    display,
                    map_role(p.role),
                    true,
                );
            }
            StatementKind::Message(m) => {
                let from_virtual = is_virtual_endpoint(&m.from);
                let to_virtual = is_virtual_endpoint(&m.to);
                if !from_virtual {
                    ensure_implicit(&mut participants, &mut participant_ix, &m.from);
                }
                if !to_virtual {
                    ensure_implicit(&mut participants, &mut participant_ix, &m.to);
                }
                if !from_virtual && !is_alive(&alive_by_id, &m.from) {
                    return Err(Diagnostic::error(format!(
                        "[E_LIFECYCLE_DESTROYED_SENDER] message sender `{}` is destroyed",
                        m.from
                    ))
                    .with_span(stmt.span));
                }
                if !to_virtual && !is_alive(&alive_by_id, &m.to) {
                    return Err(Diagnostic::error(format!(
                        "[E_LIFECYCLE_DESTROYED_TARGET] message target `{}` is destroyed (recreate it before sending messages to it)",
                        m.to
                    ))
                    .with_span(stmt.span));
                }
                if !from_virtual {
                    alive_by_id.insert(m.from.clone(), true);
                }
                if !to_virtual {
                    alive_by_id.insert(m.to.clone(), true);
                }
                if !from_virtual && !to_virtual {
                    last_message = Some((m.from.clone(), m.to.clone()));
                }
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: SequenceEventKind::Message {
                        from: m.from,
                        to: m.to,
                        arrow: m.arrow,
                        label: m.label,
                    },
                });
            }
            StatementKind::Note(n) => events.push(SequenceEvent {
                span: stmt.span,
                kind: SequenceEventKind::Note {
                    position: n.position,
                    target: n.target,
                    text: n.text,
                },
            }),
            StatementKind::Group(g) => {
                if g.kind == "end" {
                    events.push(SequenceEvent {
                        span: stmt.span,
                        kind: SequenceEventKind::GroupEnd,
                    });
                } else {
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
                skinparams.push((key.clone(), value.clone()));
                if !is_supported_skinparam(&key) {
                    warnings.push(
                        Diagnostic::warning(format!(
                            "[W_SKINPARAM_UNSUPPORTED] unsupported skinparam `{}`",
                            key
                        ))
                        .with_span(stmt.span),
                    );
                }
            }
            StatementKind::Theme(name) => {
                warnings.push(
                    Diagnostic::warning(format!(
                        "[W_THEME_UNSUPPORTED] !theme is not supported yet{}",
                        if name.is_empty() {
                            "".to_string()
                        } else {
                            format!(" (`{}`)", name)
                        }
                    ))
                    .with_span(stmt.span),
                );
            }
            StatementKind::Footbox(v) => footbox_visible = v,
            StatementKind::Delay(v) => events.push(SequenceEvent {
                span: stmt.span,
                kind: SequenceEventKind::Delay(v),
            }),
            StatementKind::Divider(v) => events.push(SequenceEvent {
                span: stmt.span,
                kind: SequenceEventKind::Divider(v),
            }),
            StatementKind::Spacer => events.push(SequenceEvent {
                span: stmt.span,
                kind: SequenceEventKind::Spacer,
            }),
            StatementKind::NewPage(v) => events.push(SequenceEvent {
                span: stmt.span,
                kind: SequenceEventKind::NewPage(v),
            }),
            StatementKind::Autonumber(v) => events.push(SequenceEvent {
                span: stmt.span,
                kind: SequenceEventKind::Autonumber(v),
            }),
            StatementKind::Activate(id) => {
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
            StatementKind::Return(v) => events.push(SequenceEvent {
                span: stmt.span,
                kind: infer_return_event(v, &mut activation_stack),
            }),
            StatementKind::Include(_) | StatementKind::Define { .. } | StatementKind::Undef(_) => {
                // Preprocessor directives should be expanded before normalization.
            }
            StatementKind::Unknown(_) => {}
        }
    }

    if !warnings.is_empty() {
        warnings.sort_by(|a, b| {
            let sa = a.span.map(|s| s.start).unwrap_or_default();
            let sb = b.span.map(|s| s.start).unwrap_or_default();
            (a.message.as_str(), sa).cmp(&(b.message.as_str(), sb))
        });
        return Err(warnings.remove(0));
    }

    Ok(SequenceDocument {
        participants,
        events,
        title,
        header,
        footer,
        caption,
        legend,
        skinparams,
        footbox_visible,
    })
}

fn is_alive(alive_by_id: &BTreeMap<String, bool>, id: &str) -> bool {
    alive_by_id.get(id).copied().unwrap_or(true)
}

#[derive(Debug, Clone)]
struct ActivationFrame {
    participant: String,
    caller: Option<String>,
}

fn infer_return_event(
    label: Option<String>,
    activation_stack: &mut Vec<ActivationFrame>,
) -> SequenceEventKind {
    if let Some(frame) = activation_stack.pop() {
        if let Some(caller) = frame.caller {
            return SequenceEventKind::Return {
                label,
                from: Some(frame.participant),
                to: Some(caller),
            };
        }
        return SequenceEventKind::Return {
            label,
            from: None,
            to: None,
        };
    }
    SequenceEventKind::Return {
        label,
        from: None,
        to: None,
    }
}

fn is_supported_skinparam(key: &str) -> bool {
    matches!(key.to_ascii_lowercase().as_str(), "maxmessagesize")
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
) {
    if let Some(ix) = index.get(&id).copied() {
        participants[ix].display = display;
        participants[ix].role = role;
        participants[ix].explicit = explicit;
        return;
    }

    let pos = participants.len();
    participants.push(Participant {
        id: id.clone(),
        display,
        role,
        explicit,
    });
    index.insert(id, pos);
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
    }
}

fn is_virtual_endpoint(id: &str) -> bool {
    id == "[*]"
}
