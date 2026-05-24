/// Parse `From --> To` or `From --> To : label`
fn parse_state_transition(line: &str) -> Option<StateTransition> {
    let (core, label) = split_message_label(line);
    let (from_raw, arrow, relation_style, to_raw) = split_family_arrow_styled(core)?;

    if !arrow.contains('>') || from_raw.is_empty() || to_raw.is_empty() {
        return None;
    }

    Some(StateTransition {
        from: clean_bracketed_ident(from_raw),
        to: clean_bracketed_ident(to_raw),
        label,
        line_color: relation_style.line_color,
        dashed: relation_style.dashed,
        hidden: relation_style.hidden,
        thickness: relation_style.thickness,
        direction: relation_style.direction,
    })
}

/// Parse `State : entry / action` or `State : exit / action` or `State : event / action`
fn parse_state_internal_action(line: &str) -> Option<StateInternalAction> {
    let (state_part, rest) = line.split_once(':')?;
    let state = state_part.trim();
    if state.is_empty() || state.contains("-->") {
        return None;
    }
    // Rest should have form `kind / action` or `kind`
    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }
    let (kind, action) = if let Some((k, a)) = rest.split_once('/') {
        (k.trim().to_string(), a.trim().to_string())
    } else {
        (rest.to_string(), String::new())
    };
    if kind.is_empty() {
        return None;
    }
    Some(StateInternalAction {
        state: state.to_string(),
        kind,
        action,
    })
}

fn parse_state_bare_internal_action(parent_state: &str, line: &str) -> Option<StateInternalAction> {
    let trimmed = line.trim().trim_end_matches(';').trim();
    if trimmed.is_empty() || trimmed.contains("-->") || trimmed.starts_with("state ") {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    let known_prefix = ["entry", "exit", "do"]
        .into_iter()
        .any(|prefix| lower == prefix || lower.starts_with(&format!("{prefix} /")));
    if !known_prefix {
        return None;
    }
    let (kind, action) = if let Some((k, a)) = trimmed.split_once('/') {
        (k.trim().to_string(), a.trim().to_string())
    } else {
        (trimmed.to_string(), String::new())
    };
    Some(StateInternalAction {
        state: parent_state.to_string(),
        kind,
        action,
    })
}
