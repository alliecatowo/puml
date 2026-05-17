fn parse_activity_step(line: &str) -> Option<StatementKind> {
    let trimmed = line.trim();
    if let Some(label) = parse_activity_swimlane(trimmed) {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::PartitionStart,
            label: Some(label),
        }));
    }
    if let Some(label) = parse_activity_colored_action(trimmed) {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Action,
            label: Some(label),
        }));
    }
    // `:action;` or `:action` form
    if let Some(rest) = trimmed.strip_prefix(':') {
        let body = rest.trim_end_matches(';').trim();
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Action,
            label: if body.is_empty() {
                None
            } else {
                Some(body.to_string())
            },
        }));
    }
    if trimmed == "start" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Start,
            label: None,
        }));
    }
    if trimmed == "stop" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Stop,
            label: None,
        }));
    }
    if trimmed == "end" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::End,
            label: None,
        }));
    }
    if trimmed == "else" || trimmed.starts_with("else ") || trimmed.starts_with("else(") {
        let label = if trimmed == "else" {
            None
        } else {
            extract_paren_label(trimmed.trim_start_matches("else").trim())
        };
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Else,
            label,
        }));
    }
    if trimmed == "endif" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::EndIf,
            label: None,
        }));
    }
    if trimmed == "fork" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Fork,
            label: None,
        }));
    }
    if trimmed == "fork again" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::ForkAgain,
            label: None,
        }));
    }
    if trimmed == "end fork" || trimmed == "endfork" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::EndFork,
            label: None,
        }));
    }
    if trimmed == "split" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Fork,
            label: Some("split".to_string()),
        }));
    }
    if trimmed == "split again" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::ForkAgain,
            label: Some("split again".to_string()),
        }));
    }
    if trimmed == "end split" || trimmed == "endsplit" || trimmed == "end merge" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::EndFork,
            label: Some("end split".to_string()),
        }));
    }
    if trimmed == "endwhile" || trimmed == "end while" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::EndWhile,
            label: None,
        }));
    }
    if trimmed == "repeat" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::RepeatStart,
            label: None,
        }));
    }
    if let Some(rest) = trimmed.strip_prefix("if ") {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::IfStart,
            label: Some(parse_activity_if_label(rest.trim())),
        }));
    }
    if let Some(rest) = trimmed.strip_prefix("switch ") {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::IfStart,
            label: Some(format!(
                "switch {}",
                extract_paren_label(rest.trim()).unwrap_or_else(|| rest.trim().to_string())
            )),
        }));
    }
    if let Some(rest) = trimmed.strip_prefix("case ") {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Else,
            label: extract_paren_label(rest.trim()).or_else(|| Some(rest.trim().to_string())),
        }));
    }
    if trimmed == "endswitch" || trimmed == "end switch" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::EndIf,
            label: Some("endswitch".to_string()),
        }));
    }
    if let Some(rest) = trimmed.strip_prefix("elseif ") {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Else,
            label: Some(format!("elseif {}", parse_activity_if_label(rest.trim()))),
        }));
    }
    if let Some(label) = parse_activity_note_step(trimmed) {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Action,
            label: Some(label),
        }));
    }
    if let Some(rest) = trimmed.strip_prefix("while ") {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::WhileStart,
            label: Some(parse_activity_condition_with_branches(rest.trim())),
        }));
    }
    if let Some(rest) = trimmed.strip_prefix("repeat while") {
        let r = rest.trim();
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::RepeatWhile,
            label: (!r.is_empty()).then(|| parse_activity_condition_with_branches(r)),
        }));
    }
    if trimmed == "end repeat" || trimmed == "endrepeat" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::EndWhile,
            label: Some("end repeat".to_string()),
        }));
    }
    if let Some(rest) = trimmed.strip_prefix("partition ") {
        let label = rest.trim().trim_end_matches('{').trim();
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::PartitionStart,
            label: Some(label.to_string()),
        }));
    }
    if trimmed == "}" {
        // Treat lone `}` inside activity as partition close.
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::PartitionEnd,
            label: None,
        }));
    }
    if let Some(rest) = trimmed.strip_prefix("label ") {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Action,
            label: Some(format!("label {}", rest.trim())),
        }));
    }
    if let Some(rest) = trimmed.strip_prefix("goto ") {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Action,
            label: Some(format!("goto {}", rest.trim())),
        }));
    }
    if let Some(rest) = trimmed.strip_prefix("backward") {
        let label = rest
            .trim()
            .trim_start_matches(':')
            .trim_end_matches(';')
            .trim();
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Action,
            label: Some(if label.is_empty() {
                "backward".to_string()
            } else {
                format!("backward {label}")
            }),
        }));
    }
    if trimmed == "kill" || trimmed == "detach" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Stop,
            label: Some(trimmed.to_string()),
        }));
    }
    if trimmed == "break" || trimmed == "continue" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Action,
            label: Some(trimmed.to_string()),
        }));
    }
    None
}

fn looks_like_old_activity_flow(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("(*)")
        || trimmed.starts_with("-->")
        || (trimmed.contains("-->") && trimmed.contains("(*)"))
}

fn parse_activity_old_style_flow(line: &str) -> Option<Vec<StatementKind>> {
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
        } else {
            steps.push(activity_step_statement(
                ActivityStepKind::Action,
                Some(target),
            ));
        }
    }

    (!steps.is_empty()).then_some(steps)
}

fn parse_old_activity_arrow_target(rhs: &str) -> Option<String> {
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

fn parse_quoted_activity_label(input: &str) -> Option<String> {
    let input = input.trim();
    let rest = input.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn parse_activity_swimlane(line: &str) -> Option<String> {
    if !line.starts_with('|') || !line.ends_with('|') {
        return None;
    }
    let parts: Vec<&str> = line
        .trim_matches('|')
        .split('|')
        .map(str::trim)
        .filter(|part| !part.is_empty() && !part.starts_with('#'))
        .collect();
    parts.last().map(|part| (*part).to_string())
}

fn parse_activity_colored_action(line: &str) -> Option<String> {
    let rest = line.strip_prefix('#')?;
    let (_color, body) = rest.split_once(':')?;
    let label = body.trim().trim_end_matches(';').trim();
    (!label.is_empty()).then(|| label.to_string())
}

fn activity_step_statement(kind: ActivityStepKind, label: Option<String>) -> StatementKind {
    StatementKind::ActivityStep(ActivityStep { kind, label })
}

fn parse_activity_note_step(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    let prefixes = [
        "floating note left",
        "floating note right",
        "floating note",
        "note left",
        "note right",
        "note top",
        "note bottom",
    ];
    let prefix = prefixes.iter().find(|prefix| lower.starts_with(**prefix))?;
    let rest = line[prefix.len()..]
        .trim()
        .trim_start_matches(':')
        .trim_end_matches(';')
        .trim();
    let display_prefix = prefix.replace("floating ", "");
    Some(if rest.is_empty() {
        display_prefix
    } else {
        format!("{display_prefix}: {rest}")
    })
}

fn parse_activity_if_label(input: &str) -> String {
    let lower = input.to_ascii_lowercase();
    if let Some(idx) = lower.find(" then ") {
        let condition_raw = input[..idx].trim();
        let then_raw = input[idx + " then ".len()..].trim();
        let condition = parse_activity_condition_with_branches(condition_raw);
        if let Some(branch) = extract_paren_label(then_raw) {
            if !branch.is_empty() {
                return format!("{condition} / {branch}");
            }
        }
        return condition;
    }
    let body = input.trim_end_matches("then").trim();
    extract_paren_label(body).unwrap_or_else(|| body.to_string())
}

fn parse_activity_condition_with_branches(input: &str) -> String {
    let trimmed = input.trim();
    let condition = extract_first_paren_label(trimmed).unwrap_or_else(|| {
        trimmed
            .split_once(" is ")
            .map(|(lhs, _)| lhs.trim())
            .unwrap_or(trimmed)
            .trim_end_matches("then")
            .trim()
            .to_string()
    });
    let mut parts = vec![condition];
    for marker in [" is ", " then ", " not "] {
        if let Some((_, tail)) = trimmed.split_once(marker) {
            if let Some(value) = extract_first_paren_label(tail) {
                if !value.is_empty() {
                    parts.push(value);
                }
            }
        }
    }
    parts.join(" / ")
}

fn extract_first_paren_label(input: &str) -> Option<String> {
    let s = input.trim();
    let open = s.find('(')?;
    let close = s[open + 1..].find(')')? + open + 1;
    if close <= open {
        return None;
    }
    Some(s[open + 1..close].trim().to_string())
}

fn extract_paren_label(input: &str) -> Option<String> {
    let s = input.trim();
    let open = s.find('(')?;
    let close = s.rfind(')')?;
    if close <= open {
        return None;
    }
    Some(s[open + 1..close].trim().to_string())
}
