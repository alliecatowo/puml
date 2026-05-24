use super::state::SequenceNormalizeState;
use super::*;
use crate::model::{VirtualEndpoint, VirtualEndpointKind, VirtualEndpointSide};

#[derive(Debug, Clone)]
pub(super) struct ParsedMessageArrow {
    pub(super) render_arrow: String,
    /// True when the original arrow was left-facing; caller swaps endpoints.
    pub(super) reversed: bool,
    pub(super) left_modifier: Option<String>,
    pub(super) right_modifier: Option<String>,
}

pub(super) fn is_virtual_endpoint(id: &str) -> bool {
    matches!(id, "[*]" | "[" | "]" | "[o" | "o]" | "[x" | "x]" | "?")
}

pub(super) fn virtual_endpoint(id: &str, is_from: bool) -> Option<VirtualEndpoint> {
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
        "?" => (
            if is_from {
                VirtualEndpointSide::Left
            } else {
                VirtualEndpointSide::Right
            },
            VirtualEndpointKind::Short,
        ),
        _ => return None,
    };
    Some(VirtualEndpoint { side, kind })
}

pub(super) fn validate_virtual_endpoint_combination(
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
    let reversed = !bidirectional
        && (stripped.starts_with("<<-") || stripped.starts_with("<-"))
        && !stripped.contains('>');

    let render_arrow = if bidirectional {
        if stripped.contains("--") {
            "<-->".to_string()
        } else {
            "<->".to_string()
        }
    } else if reversed {
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

fn mirror_arrow(base: &str) -> String {
    let canonical = base.replace(['/', '\\'], "");
    let left_marker = canonical.chars().next().filter(|c| matches!(c, 'o' | 'x'));
    let right_marker = canonical.chars().last().filter(|c| matches!(c, 'o' | 'x'));
    let inner = canonical
        .strip_prefix(|c| matches!(c, 'o' | 'x'))
        .unwrap_or(&canonical);
    let inner = inner
        .strip_suffix(|c| matches!(c, 'o' | 'x'))
        .unwrap_or(inner);

    let mirrored_core = match inner {
        "<-" => "->",
        "<--" => "-->",
        "<<-" => "->>",
        "<<--" => "-->>",
        _ => return base.to_string(),
    };

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

impl SequenceNormalizeState {
    pub(super) fn handle_message(
        &mut self,
        span: crate::source::Span,
        m: crate::ast::Message,
    ) -> Result<(), Diagnostic> {
        groups::mark_group_content(&mut self.group_stack);
        let parsed_arrow = parse_message_arrow(&m.arrow).ok_or_else(|| {
            Diagnostic::error(format!(
                "[E_ARROW_INVALID] malformed sequence arrow syntax: `{}`",
                m.arrow
            ))
            .with_span(span)
        })?;
        if !is_virtual_endpoint(&m.from) {
            participants::ensure_implicit(
                &mut self.participants,
                &mut self.participant_ix,
                &m.from,
            );
        }
        if !is_virtual_endpoint(&m.to) {
            participants::ensure_implicit(&mut self.participants, &mut self.participant_ix, &m.to);
        }
        let (event_from, event_to) = if parsed_arrow.reversed {
            (m.to.clone(), m.from.clone())
        } else {
            (m.from.clone(), m.to.clone())
        };

        let from_virtual = virtual_endpoint(event_from.as_str(), true);
        let to_virtual = virtual_endpoint(event_to.as_str(), false);
        validate_virtual_endpoint_combination(
            span,
            &event_from,
            &event_to,
            from_virtual,
            to_virtual,
        )?;
        lifecycle::validate_and_touch_message_lifecycle(
            span,
            &event_from,
            &event_to,
            &mut self.participants,
            &mut self.participant_ix,
            &mut self.alive_by_id,
        )?;
        if !is_virtual_endpoint(&event_from) && !is_virtual_endpoint(&event_to) {
            self.last_message = Some((event_from.clone(), event_to.clone()));
        }
        self.events.push(SequenceEvent {
            span,
            kind: SequenceEventKind::Message {
                from: event_from.clone(),
                to: event_to.clone(),
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
        lifecycle::apply_lifecycle_shortcuts(
            span,
            &m.from,
            &m.to,
            &parsed_arrow,
            &mut self.participants,
            &mut self.participant_ix,
            &mut self.alive_by_id,
            &mut self.activation_stack,
            &mut self.events,
        )
    }
}
