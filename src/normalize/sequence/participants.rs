use super::state::SequenceNormalizeState;
use super::*;

pub(super) fn ensure_implicit(
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

pub(super) fn upsert_participant(
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

pub(super) fn map_role(role: AstRole) -> ParticipantRole {
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

pub(super) fn parse_participant_group_label(raw: Option<&str>) -> (Option<String>, Option<String>) {
    let Some(raw) = raw.map(str::trim).filter(|raw| !raw.is_empty()) else {
        return (None, None);
    };

    let mut label = raw;
    let mut color = None;
    if let Some(last) = raw.split_whitespace().last() {
        if let Some(parsed) = parse_sequence_box_color(last) {
            color = Some(parsed);
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

fn parse_sequence_box_color(token: &str) -> Option<String> {
    let value = token.strip_prefix('#')?;
    if value.is_empty() {
        return None;
    }

    let is_hex =
        matches!(value.len(), 3 | 4 | 6 | 8) && value.bytes().all(|b| b.is_ascii_hexdigit());
    if is_hex {
        return Some(format!("#{}", value.to_ascii_lowercase()));
    }

    if value.bytes().all(|b| b.is_ascii_alphabetic()) {
        return Some(value.to_ascii_lowercase());
    }

    None
}

impl SequenceNormalizeState {
    pub(super) fn handle_participant(
        &mut self,
        span: crate::source::Span,
        p: crate::ast::ParticipantDecl,
    ) -> Result<(), Diagnostic> {
        groups::mark_group_content(&mut self.group_stack);
        let id = p.alias.unwrap_or_else(|| p.name.clone());
        let display = p.display.unwrap_or_else(|| p.name.clone());
        if let Some(order) = p.order {
            self.participant_order.insert(id.clone(), order);
        }
        upsert_participant(
            &mut self.participants,
            &mut self.participant_ix,
            id.clone(),
            display,
            map_role(p.role),
            true,
        )
        .map_err(|e| Diagnostic::error(e).with_span(span))?;
        for group in &mut self.participant_group_stack {
            if !group.participant_ids.iter().any(|member| member == &id) {
                group.participant_ids.push(id.clone());
            }
        }
        Ok(())
    }

    pub(super) fn apply_hide_unlinked(&mut self, hidden_participants: &mut Vec<String>) {
        let mut referenced: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for ev in &self.events {
            match &ev.kind {
                SequenceEventKind::Message { from, to, .. } => {
                    if !messages::is_virtual_endpoint(from) {
                        referenced.insert(from.clone());
                    }
                    if !messages::is_virtual_endpoint(to) {
                        referenced.insert(to.clone());
                    }
                }
                SequenceEventKind::Note {
                    target: Some(t), ..
                } => {
                    for part in t.split(',') {
                        let id = part.trim();
                        if !id.is_empty() && !messages::is_virtual_endpoint(id) {
                            referenced.insert(id.to_string());
                        }
                    }
                }
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
        self.participants.retain(|p| {
            let keep = !p.explicit || referenced.contains(&p.id);
            if !keep {
                hidden_participants.push(p.id.clone());
            }
            keep
        });
        if !hidden_participants.is_empty() {
            self.rebuild_participant_index();
            self.participant_groups = self
                .participant_groups
                .drain(..)
                .filter_map(|mut group| {
                    group
                        .participant_ids
                        .retain(|id| !hidden_participants.contains(id));
                    (!group.participant_ids.is_empty()).then_some(group)
                })
                .collect();
        }
    }

    pub(super) fn apply_participant_order(&mut self) {
        if self.participant_order.is_empty() {
            return;
        }
        let original_ix: BTreeMap<String, usize> = self
            .participants
            .iter()
            .enumerate()
            .map(|(idx, p)| (p.id.clone(), idx))
            .collect();
        self.participants.sort_by(|a, b| {
            let a_key = (
                self.participant_order
                    .get(&a.id)
                    .copied()
                    .unwrap_or(i32::MAX),
                original_ix.get(&a.id).copied().unwrap_or(usize::MAX),
            );
            let b_key = (
                self.participant_order
                    .get(&b.id)
                    .copied()
                    .unwrap_or(i32::MAX),
                original_ix.get(&b.id).copied().unwrap_or(usize::MAX),
            );
            a_key.cmp(&b_key)
        });
        self.rebuild_participant_index();
    }

    pub(super) fn rebuild_participant_index(&mut self) {
        self.participant_ix.clear();
        for (idx, p) in self.participants.iter().enumerate() {
            self.participant_ix.insert(p.id.clone(), idx);
        }
    }
}
