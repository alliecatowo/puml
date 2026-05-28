use super::*;
pub(crate) fn parse_activity_step(line: &str) -> Option<StatementKind> {
    let trimmed = line.trim();
    if let Some(label) = parse_activity_swimlane(trimmed) {
        return Some(activity_step_statement(
            ActivityStepKind::PartitionStart,
            Some(label),
        ));
    }
    if let Some(label) = parse_activity_arrow_directive(trimmed) {
        return Some(activity_step_statement(
            ActivityStepKind::Arrow,
            Some(label),
        ));
    }
    if let Some((label, color)) = parse_activity_colored_connector(trimmed) {
        return Some(activity_step_statement(
            ActivityStepKind::Connector,
            Some(activity_style_label(label, color.as_deref())),
        ));
    }
    if let Some(label) = parse_activity_connector(trimmed) {
        return Some(activity_step_statement(
            ActivityStepKind::Connector,
            Some(label),
        ));
    }
    if let Some((label, color)) = parse_activity_colored_action(trimmed) {
        return Some(activity_step_statement(
            ActivityStepKind::Action,
            Some(activity_style_label(label, color.as_deref())),
        ));
    }
    // `:action;` or `:action` form (including SDL terminator variants: |, <, >, /, \, ], })
    if let Some(rest) = trimmed.strip_prefix(':') {
        let (body, sdl_shape) = parse_activity_action_terminator(rest);
        let label = if let Some(shape) = sdl_shape {
            if body.is_empty() {
                None
            } else {
                Some(format!("\x1fsdl:{shape}\x1f{body}"))
            }
        } else if body.is_empty() {
            None
        } else {
            Some(body.to_string())
        };
        return Some(activity_step_statement(ActivityStepKind::Action, label));
    }
    if trimmed == "start" {
        return Some(activity_step_statement(ActivityStepKind::Start, None));
    }
    if trimmed == "stop" {
        return Some(activity_step_statement(ActivityStepKind::Stop, None));
    }
    if trimmed == "end" {
        return Some(activity_step_statement(ActivityStepKind::End, None));
    }
    if trimmed == "else" || trimmed.starts_with("else ") || trimmed.starts_with("else(") {
        let label = if trimmed == "else" {
            None
        } else {
            extract_paren_label(trimmed.trim_start_matches("else").trim())
        };
        return Some(activity_step_statement(ActivityStepKind::Else, label));
    }
    if trimmed == "endif" {
        return Some(activity_step_statement(ActivityStepKind::EndIf, None));
    }
    if trimmed == "fork" {
        return Some(activity_step_statement(ActivityStepKind::Fork, None));
    }
    if trimmed == "fork again" {
        return Some(activity_step_statement(ActivityStepKind::ForkAgain, None));
    }
    if trimmed == "end fork" || trimmed == "endfork" {
        return Some(activity_step_statement(ActivityStepKind::EndFork, None));
    }
    if trimmed == "split" {
        return Some(activity_step_statement(
            ActivityStepKind::Fork,
            Some("split".to_string()),
        ));
    }
    if trimmed == "split again" {
        return Some(activity_step_statement(
            ActivityStepKind::ForkAgain,
            Some("split again".to_string()),
        ));
    }
    if trimmed == "end split" || trimmed == "endsplit" || trimmed == "end merge" {
        return Some(activity_step_statement(
            ActivityStepKind::EndFork,
            Some("end split".to_string()),
        ));
    }
    if let Some(exit_label) = parse_activity_endwhile(trimmed) {
        return Some(activity_step_statement(
            ActivityStepKind::EndWhile,
            exit_label,
        ));
    }
    if trimmed == "repeat" {
        return Some(activity_step_statement(ActivityStepKind::RepeatStart, None));
    }
    if let Some(rest) = trimmed.strip_prefix("if ") {
        return Some(activity_step_statement(
            ActivityStepKind::IfStart,
            Some(parse_activity_if_label(rest.trim())),
        ));
    }
    if let Some(rest) = trimmed.strip_prefix("switch ") {
        return Some(activity_step_statement(
            ActivityStepKind::IfStart,
            Some(format!(
                "switch {}",
                extract_paren_label(rest.trim()).unwrap_or_else(|| rest.trim().to_string())
            )),
        ));
    }
    if let Some(rest) = trimmed.strip_prefix("case ") {
        return Some(activity_step_statement(
            ActivityStepKind::Else,
            extract_paren_label(rest.trim()).or_else(|| Some(rest.trim().to_string())),
        ));
    }
    if trimmed == "endswitch" || trimmed == "end switch" {
        return Some(activity_step_statement(
            ActivityStepKind::EndIf,
            Some("endswitch".to_string()),
        ));
    }
    if let Some(rest) = trimmed.strip_prefix("elseif ") {
        return Some(activity_step_statement(
            ActivityStepKind::Else,
            Some(format!("elseif {}", parse_activity_if_label(rest.trim()))),
        ));
    }
    if let Some(label) = parse_activity_note_step(trimmed) {
        return Some(activity_step_statement(ActivityStepKind::Note, Some(label)));
    }
    if let Some(rest) = trimmed.strip_prefix("while ") {
        return Some(activity_step_statement(
            ActivityStepKind::WhileStart,
            Some(parse_activity_condition_with_branches(rest.trim())),
        ));
    }
    if let Some(rest) = trimmed.strip_prefix("repeat while") {
        let r = rest.trim();
        return Some(activity_step_statement(
            ActivityStepKind::RepeatWhile,
            (!r.is_empty()).then(|| parse_activity_condition_with_branches(r)),
        ));
    }
    if trimmed == "end repeat" || trimmed == "endrepeat" {
        return Some(activity_step_statement(
            ActivityStepKind::EndWhile,
            Some("end repeat".to_string()),
        ));
    }
    if let Some((label, color)) = parse_activity_partition_like(trimmed) {
        return Some(activity_step_statement(
            ActivityStepKind::PartitionStart,
            Some(activity_style_label(label, color.as_deref())),
        ));
    }
    if trimmed == "}" || trimmed == "end group" {
        // Treat lone `}` inside activity as partition close.
        return Some(activity_step_statement(
            ActivityStepKind::PartitionEnd,
            None,
        ));
    }
    if let Some(rest) = trimmed.strip_prefix("label ") {
        return Some(activity_step_statement(
            ActivityStepKind::Action,
            Some(format!("label {}", rest.trim())),
        ));
    }
    if let Some(rest) = trimmed.strip_prefix("goto ") {
        return Some(activity_step_statement(
            ActivityStepKind::Action,
            Some(format!("goto {}", rest.trim())),
        ));
    }
    if let Some(rest) = trimmed.strip_prefix("backward") {
        let label = rest
            .trim()
            .trim_start_matches(':')
            .trim_end_matches(';')
            .trim();
        return Some(activity_step_statement(
            ActivityStepKind::Action,
            Some(if label.is_empty() {
                "backward".to_string()
            } else {
                format!("backward {label}")
            }),
        ));
    }
    if trimmed == "kill" {
        return Some(activity_step_statement(
            ActivityStepKind::Kill,
            Some(trimmed.to_string()),
        ));
    }
    if trimmed == "detach" {
        return Some(activity_step_statement(
            ActivityStepKind::Detach,
            Some(trimmed.to_string()),
        ));
    }
    if trimmed == "break" || trimmed == "continue" {
        return Some(activity_step_statement(
            ActivityStepKind::Action,
            Some(trimmed.to_string()),
        ));
    }
    None
}

pub(crate) mod conditions;
pub(crate) mod notes;
pub(crate) mod old_style;
pub(crate) mod style;

pub(crate) use conditions::*;
pub(crate) use notes::*;
pub(crate) use old_style::*;
pub(crate) use style::*;
