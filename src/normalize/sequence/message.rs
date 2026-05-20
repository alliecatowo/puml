use super::participants::{ensure_implicit, is_alive, is_virtual_endpoint};
use super::*;

#[derive(Debug, Clone)]
pub(super) struct ActivationFrame {
    pub(super) participant: String,
    pub(super) caller: Option<String>,
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

#[derive(Debug, Clone)]
pub(super) struct ParsedMessageArrow {
    pub(super) render_arrow: String,
    /// True when the original arrow was left-facing (e.g. `<-`).  The caller
    /// must swap from/to before storing the event so that `from` always
    /// represents the semantic sender and x1 is the sender's centre.
    pub(super) reversed: bool,
    pub(super) left_modifier: Option<String>,
    pub(super) right_modifier: Option<String>,
}

pub(super) fn parse_message_arrow(raw: &str) -> Option<ParsedMessageArrow> {
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

pub(super) fn validate_and_touch_message_lifecycle(
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
