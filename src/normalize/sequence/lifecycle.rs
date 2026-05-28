use super::state::SequenceNormalizeState;
use super::*;
use crate::normalize::sequence::messages::ParsedMessageArrow;

#[derive(Debug, Clone)]
pub(super) struct ActivationFrame {
    pub(super) participant: String,
    pub(super) caller: Option<String>,
}

pub(super) fn is_alive(alive_by_id: &BTreeMap<String, bool>, id: &str) -> bool {
    alive_by_id.get(id).copied().unwrap_or(true)
}

pub(super) fn infer_return_event(
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

pub(super) fn validate_and_touch_message_lifecycle(
    span: crate::source::Span,
    from: &str,
    to: &str,
    participants: &mut Vec<Participant>,
    participant_ix: &mut BTreeMap<String, usize>,
    alive_by_id: &mut BTreeMap<String, bool>,
) -> Result<(), Diagnostic> {
    let from_virtual = messages::is_virtual_endpoint(from);
    let to_virtual = messages::is_virtual_endpoint(to);
    if !from_virtual {
        participants::ensure_implicit(participants, participant_ix, from);
    }
    if !to_virtual {
        participants::ensure_implicit(participants, participant_ix, to);
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
pub(super) fn apply_lifecycle_shortcuts(
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
    if messages::is_virtual_endpoint(id) {
        return Err(Diagnostic::error(format!(
            "[E_LIFECYCLE_SHORTCUT_VIRTUAL] cannot apply lifecycle shortcut `{}` to virtual endpoint",
            token
        ))
        .with_span(span));
    }
    participants::ensure_implicit(participants, participant_ix, id);
    match token {
        "++" => activate_shortcut(span, id, caller, alive_by_id, activation_stack, events),
        "--" => deactivate_shortcut(span, id, alive_by_id, activation_stack, events),
        "**" => {
            alive_by_id.insert(id.to_string(), true);
            events.push(SequenceEvent {
                span,
                kind: SequenceEventKind::Create(id.to_string()),
            });
            Ok(())
        }
        "!!" => destroy_shortcut(span, id, alive_by_id, activation_stack, events),
        _ => Err(Diagnostic::error(format!(
            "[E_LIFECYCLE_SHORTCUT_INVALID] unknown lifecycle shortcut `{}`",
            token
        ))
        .with_span(span)),
    }
}

fn activate_shortcut(
    span: crate::source::Span,
    id: &str,
    caller: Option<String>,
    alive_by_id: &mut BTreeMap<String, bool>,
    activation_stack: &mut Vec<ActivationFrame>,
    events: &mut Vec<SequenceEvent>,
) -> Result<(), Diagnostic> {
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
    Ok(())
}

fn deactivate_shortcut(
    span: crate::source::Span,
    id: &str,
    alive_by_id: &mut BTreeMap<String, bool>,
    activation_stack: &mut Vec<ActivationFrame>,
    events: &mut Vec<SequenceEvent>,
) -> Result<(), Diagnostic> {
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
    Ok(())
}

fn destroy_shortcut(
    span: crate::source::Span,
    id: &str,
    alive_by_id: &mut BTreeMap<String, bool>,
    activation_stack: &[ActivationFrame],
    events: &mut Vec<SequenceEvent>,
) -> Result<(), Diagnostic> {
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
    Ok(())
}

fn shortcut_caller(active: &str, other: &str) -> Option<String> {
    if messages::is_virtual_endpoint(active) || messages::is_virtual_endpoint(other) {
        None
    } else {
        Some(other.to_string())
    }
}

impl SequenceNormalizeState {
    pub(super) fn handle_activate(
        &mut self,
        span: crate::source::Span,
        id: String,
    ) -> Result<(), Diagnostic> {
        groups::mark_group_content(&mut self.group_stack);
        participants::ensure_implicit(&mut self.participants, &mut self.participant_ix, &id);
        if !is_alive(&self.alive_by_id, &id) {
            return Err(Diagnostic::error(format!(
                "[E_LIFECYCLE_ACTIVATE_DESTROYED] cannot activate destroyed participant `{}`",
                id
            ))
            .with_span(span));
        }
        self.alive_by_id.insert(id.clone(), true);
        let caller = match &self.last_message {
            Some((from, to)) if to == &id => Some(from.clone()),
            _ => self.activation_stack.last().map(|f| f.participant.clone()),
        };
        self.activation_stack.push(ActivationFrame {
            participant: id.clone(),
            caller,
        });
        self.events.push(SequenceEvent {
            span,
            kind: SequenceEventKind::Activate(id),
        });
        Ok(())
    }

    pub(super) fn handle_deactivate(
        &mut self,
        span: crate::source::Span,
        id: String,
    ) -> Result<(), Diagnostic> {
        groups::mark_group_content(&mut self.group_stack);
        participants::ensure_implicit(&mut self.participants, &mut self.participant_ix, &id);
        if !is_alive(&self.alive_by_id, &id) {
            return Err(Diagnostic::error(format!(
                "[E_LIFECYCLE_DEACTIVATE_DESTROYED] cannot deactivate destroyed participant `{}`",
                id
            ))
            .with_span(span));
        }
        self.alive_by_id.insert(id.clone(), true);
        match self.activation_stack.last() {
            Some(frame) if frame.participant == id => {
                self.activation_stack.pop();
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
        self.events.push(SequenceEvent {
            span,
            kind: SequenceEventKind::Deactivate(id),
        });
        Ok(())
    }

    pub(super) fn handle_destroy(
        &mut self,
        span: crate::source::Span,
        id: String,
    ) -> Result<(), Diagnostic> {
        groups::mark_group_content(&mut self.group_stack);
        participants::ensure_implicit(&mut self.participants, &mut self.participant_ix, &id);
        if !is_alive(&self.alive_by_id, &id) {
            return Err(Diagnostic::error(format!(
                "[E_LIFECYCLE_DESTROY_TWICE] participant `{}` is already destroyed",
                id
            ))
            .with_span(span));
        }
        if self.activation_stack.iter().any(|f| f.participant == id) {
            return Err(Diagnostic::error(format!(
                "[E_LIFECYCLE_DESTROY_ACTIVE] cannot destroy active participant `{}`",
                id
            ))
            .with_span(span));
        }
        self.alive_by_id.insert(id.clone(), false);
        self.events.push(SequenceEvent {
            span,
            kind: SequenceEventKind::Destroy(id),
        });
        Ok(())
    }

    pub(super) fn handle_create(
        &mut self,
        span: crate::source::Span,
        id: String,
    ) -> Result<(), Diagnostic> {
        groups::mark_group_content(&mut self.group_stack);
        participants::ensure_implicit(&mut self.participants, &mut self.participant_ix, &id);
        if self.alive_by_id.get(&id).copied() == Some(true) {
            return Err(Diagnostic::error(format!(
                "[E_LIFECYCLE_CREATE_EXISTING] participant `{}` already exists; destroy before create",
                id
            ))
            .with_span(span));
        }
        self.alive_by_id.insert(id.clone(), true);
        // Mark this participant as mid-flow created so the layout can suppress
        // the header box at the top and render it at the creation row instead.
        self.created_participants.insert(id.clone());
        self.events.push(SequenceEvent {
            span,
            kind: SequenceEventKind::Create(id),
        });
        Ok(())
    }

    pub(super) fn handle_return(
        &mut self,
        span: crate::source::Span,
        label: Option<String>,
    ) -> Result<(), Diagnostic> {
        groups::mark_group_content(&mut self.group_stack);
        self.events.push(SequenceEvent {
            span,
            kind: infer_return_event(span, label, &mut self.activation_stack, &self.last_message)?,
        });
        Ok(())
    }
}
