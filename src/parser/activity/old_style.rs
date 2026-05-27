use super::*;
pub(crate) fn looks_like_old_activity_flow(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("(*)")
        || trimmed.starts_with("-->")
        || (trimmed.contains("-->") && trimmed.contains("(*)"))
}

pub(crate) fn parse_activity_old_style_flow(line: &str) -> Option<Vec<StatementKind>> {
    let trimmed = line.trim();
    let arrow_idx = trimmed.find("-->")?;
    let lhs = trimmed[..arrow_idx].trim();
    let rhs = trimmed[arrow_idx + 3..].trim();
    let mut steps = Vec::new();

    if lhs == "(*)" {
        steps.push(activity_step_statement(ActivityStepKind::Start, None));
    }

    if let Some(target) = parse_old_activity_arrow_target(rhs) {
        if target == "(*)" {
            steps.push(activity_step_statement(ActivityStepKind::Stop, None));
        } else if target.eq_ignore_ascii_case("detach") {
            steps.push(activity_step_statement(
                ActivityStepKind::Detach,
                Some("detach".to_string()),
            ));
        } else {
            steps.push(activity_step_statement(
                ActivityStepKind::Action,
                Some(target),
            ));
        }
    }

    (!steps.is_empty()).then_some(steps)
}

pub(crate) fn parse_old_activity_arrow_target(rhs: &str) -> Option<String> {
    let mut rest = rhs.trim();
    if let Some(after_label) = rest.strip_prefix('[') {
        let close = after_label.find(']')?;
        rest = after_label[close + 1..].trim();
    }
    if let Some(after_dir) = rest.strip_prefix("right of ") {
        rest = after_dir.trim();
    } else if let Some(after_dir) = rest.strip_prefix("left of ") {
        rest = after_dir.trim();
    } else if let Some(after_dir) = rest.strip_prefix("up of ") {
        rest = after_dir.trim();
    } else if let Some(after_dir) = rest.strip_prefix("down of ") {
        rest = after_dir.trim();
    }

    if rest == "(*)" {
        return Some(rest.to_string());
    }

    if let Some(label) = parse_quoted_activity_label(rest) {
        return Some(label);
    }

    let label = rest.trim_end_matches(';').trim();
    (!label.is_empty()).then(|| label.to_string())
}

pub(crate) fn parse_quoted_activity_label(input: &str) -> Option<String> {
    let input = input.trim();
    let rest = input.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}
