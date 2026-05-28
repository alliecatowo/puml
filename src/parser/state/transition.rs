use super::*;
/// Parse `From --> To` or `From --> To : label`
///
/// Also handles tail-form inline style on the target name, e.g.:
///   `From --> To #red : event`
///   `From --> To #line:blue;line.bold : event`
/// This mirrors PlantUML's inline-style syntax (spec §3.36) and avoids
/// `To #red` being incorrectly parsed as the node name.
pub(crate) fn parse_state_transition(line: &str) -> Option<StateTransition> {
    // Pre-strip tail-form inline relation style before label splitting so that
    // `To #line:color` inside the label-separator colon does not confuse the
    // label splitter, and so the `#...` token does not leak into the node name.
    let (preprocessed, pre_style) = pre_strip_inline_relation_style(line);
    let line_ref = preprocessed.as_str();

    let (core, label) = split_message_label(line_ref);
    let (from_raw, arrow, mut relation_style, to_raw) = split_family_arrow_styled(core)?;

    if !arrow.contains('>') || from_raw.is_empty() || to_raw.is_empty() {
        return None;
    }

    // Merge pre-stripped style (colour/dash/etc.) into the arrow-bracket style.
    if let Some(pre) = pre_style {
        if pre.line_color.is_some() && relation_style.line_color.is_none() {
            relation_style.line_color = pre.line_color;
        }
        if pre.dashed {
            relation_style.dashed = true;
        }
        if pre.hidden {
            relation_style.hidden = true;
        }
        if pre.thickness.is_some() && relation_style.thickness.is_none() {
            relation_style.thickness = pre.thickness;
        }
    }

    // Also apply any remaining tail-form style still attached to the RHS token
    // (e.g. when the label was absent and `pre_strip` left a residual `#color`
    // after the target identifier — this can happen with bare `A --> B #red`).
    let to_clean = {
        let mut s = relation_style.clone();
        let cleaned = parse_rhs_inline_relation_style(to_raw, &mut s);
        relation_style = s;
        cleaned
    };

    Some(StateTransition {
        from: clean_bracketed_ident(from_raw),
        to: clean_bracketed_ident(&to_clean),
        label,
        line_color: relation_style.line_color,
        dashed: relation_style.dashed,
        hidden: relation_style.hidden,
        thickness: relation_style.thickness,
        direction: relation_style.direction,
    })
}

/// Parse `State : entry / action` or `State : exit / action` or `State : event / action`
pub(crate) fn parse_state_internal_action(line: &str) -> Option<StateInternalAction> {
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

pub(crate) fn parse_state_bare_internal_action(
    parent_state: &str,
    line: &str,
) -> Option<StateInternalAction> {
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
