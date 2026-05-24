use super::state::SequenceNormalizeState;
use super::*;

#[derive(Debug, Clone)]
pub(super) struct GroupFrame {
    pub(super) kind: String,
    pub(super) span: crate::source::Span,
    pub(super) branch_has_content: bool,
}

impl GroupFrame {
    pub(super) fn new(kind: String, span: crate::source::Span) -> Self {
        Self {
            kind,
            span,
            branch_has_content: false,
        }
    }
}

pub(super) fn mark_group_content(group_stack: &mut [GroupFrame]) {
    for frame in group_stack {
        frame.branch_has_content = true;
    }
}

fn allows_else(kind: &str) -> bool {
    matches!(kind, "alt" | "par" | "critical")
}

/// Returns `true` if `separator` is valid inside `group_kind`.
pub(super) fn allows_branch_separator(group_kind: &str, separator: &str) -> bool {
    match separator {
        "also" => matches!(group_kind, "par"),
        _ => allows_else(group_kind),
    }
}

pub(super) fn rejects_empty_group(kind: &str) -> bool {
    matches!(kind, "alt" | "par" | "critical")
}

impl SequenceNormalizeState {
    pub(super) fn handle_group(
        &mut self,
        span: crate::source::Span,
        g: crate::ast::Group,
    ) -> Result<(), Diagnostic> {
        if g.kind.eq_ignore_ascii_case("box") {
            let (label, color) = participants::parse_participant_group_label(g.label.as_deref());
            self.participant_group_stack.push(SequenceParticipantGroup {
                label,
                color,
                participant_ids: Vec::new(),
            });
            return Ok(());
        }
        if g.kind == "end" {
            return self.handle_group_end(span, g.label);
        }
        if g.kind == "else" || g.kind == "also" {
            return self.handle_group_branch(span, g.kind, g.label);
        }
        mark_group_content(&mut self.group_stack);
        if g.kind != "ref" {
            self.group_stack.push(GroupFrame::new(g.kind.clone(), span));
        } else {
            self.ensure_ref_participants(g.label.as_deref());
        }
        self.events.push(SequenceEvent {
            span,
            kind: SequenceEventKind::GroupStart {
                kind: g.kind,
                label: g.label,
            },
        });
        Ok(())
    }

    fn handle_group_end(
        &mut self,
        span: crate::source::Span,
        label: Option<String>,
    ) -> Result<(), Diagnostic> {
        if label.as_deref() == Some("box") {
            let Some(group) = self.participant_group_stack.pop() else {
                return Err(Diagnostic::error(
                    "[E_BOX_END_UNMATCHED] `end box` without an open box block",
                )
                .with_span(span));
            };
            if !group.participant_ids.is_empty() {
                self.participant_groups.push(group);
            }
            return Ok(());
        }
        let Some(open) = self.group_stack.pop() else {
            return Err(Diagnostic::error(
                "[E_GROUP_END_UNMATCHED] `end` without an open group block",
            )
            .with_span(span));
        };
        if let Some(expected) = label.as_deref() {
            if expected != open.kind {
                return Err(Diagnostic::error(format!(
                    "[E_GROUP_END_KIND] `end {}` does not match open `{}` block",
                    expected, open.kind
                ))
                .with_span(span));
            }
        }
        if rejects_empty_group(open.kind.as_str()) && !open.branch_has_content {
            return Err(Diagnostic::error(format!(
                "[E_GROUP_EMPTY] `{}` block must not be empty",
                open.kind
            ))
            .with_span(span));
        }
        self.events.push(SequenceEvent {
            span,
            kind: SequenceEventKind::GroupEnd,
        });
        if open.kind == "par" {
            self.events.push(SequenceEvent {
                span,
                kind: SequenceEventKind::Spacer(Some(1)),
            });
        }
        Ok(())
    }

    fn handle_group_branch(
        &mut self,
        span: crate::source::Span,
        kind: String,
        label: Option<String>,
    ) -> Result<(), Diagnostic> {
        let Some(top) = self.group_stack.last_mut() else {
            return Err(Diagnostic::error(format!(
                "[E_GROUP_ELSE_UNMATCHED] `{kind}` without an open group block",
            ))
            .with_span(span));
        };
        if !allows_branch_separator(top.kind.as_str(), kind.as_str()) {
            return Err(Diagnostic::error(format!(
                "[E_GROUP_ELSE_KIND] `{}` is not valid inside `{}`",
                kind, top.kind
            ))
            .with_span(span));
        }
        if rejects_empty_group(top.kind.as_str()) && !top.branch_has_content {
            return Err(Diagnostic::error(format!(
                "[E_GROUP_EMPTY_BRANCH] `{}` block contains an empty branch before `{}`",
                top.kind, kind
            ))
            .with_span(span));
        }
        top.branch_has_content = false;
        self.events.push(SequenceEvent {
            span,
            kind: SequenceEventKind::GroupStart { kind, label },
        });
        Ok(())
    }

    fn ensure_ref_participants(&mut self, label: Option<&str>) {
        let Some(lbl) = label else {
            return;
        };
        let first_line = lbl.lines().next().unwrap_or("");
        let Some(over_spec) = first_line.strip_prefix("over ") else {
            return;
        };
        for id in over_spec
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            participants::ensure_implicit(&mut self.participants, &mut self.participant_ix, id);
        }
    }
}
