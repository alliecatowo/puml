fn parse_chronology_baseline_statement(line: &str) -> Option<StatementKind> {
    let trimmed = line.trim();
    if let Some(kind) = parse_keyword(trimmed) {
        return Some(kind);
    }
    let lower = trimmed.to_ascii_lowercase();
    let marker = " happens on ";
    if let Some(idx) = lower.find(marker) {
        let subject = trimmed[..idx].trim().trim_matches('"').to_string();
        let when = trimmed[idx + marker.len()..].trim().to_string();
        if subject.is_empty() || when.is_empty() {
            return None;
        }
        return Some(StatementKind::ChronologyHappensOn { subject, when });
    }
    // Accept ISO `YYYY-MM-DD : Label` shorthand
    if let Some((lhs, rhs)) = trimmed.split_once(':') {
        let when = lhs.trim();
        let subject = rhs.trim().trim_matches('"');
        if !when.is_empty()
            && !subject.is_empty()
            && when.chars().next().is_some_and(|c| c.is_ascii_digit())
        {
            return Some(StatementKind::ChronologyHappensOn {
                subject: subject.to_string(),
                when: when.to_string(),
            });
        }
    }
    None
}

/// Parse a state diagram statement from the current line.
/// Returns `Some((kind, end_index))` where `end_index` is the last consumed line.
fn parse_state_statement(
    lines: &[(&str, Span)],
    start: usize,
    line: &str,
) -> Result<Option<(StatementKind, usize)>, Diagnostic> {
    if let Some((kind, end_idx)) = parse_multiline_note_block(lines, start, line) {
        return Ok(Some((kind, end_idx)));
    }

    // Handle common keywords that are valid in any diagram
    if let Some(kind) = parse_keyword(line) {
        return Ok(Some((kind, start)));
    }

    // `[H]` or `[H*]` — history pseudo-states
    if line == "[H]" {
        return Ok(Some((StatementKind::StateHistory { deep: false }, start)));
    }
    if line == "[H*]" {
        return Ok(Some((StatementKind::StateHistory { deep: true }, start)));
    }

    for (keyword, stereotype) in [
        ("choice", "choice"),
        ("fork", "fork"),
        ("join", "join"),
        ("end", "end"),
    ] {
        // Only match if followed by whitespace (word boundary), not e.g. "fork1"
        let rest_opt = line.strip_prefix(keyword).and_then(|r| {
            if r.is_empty() || r.starts_with(char::is_whitespace) {
                Some(r.trim())
            } else {
                None
            }
        });
        if let Some(rest) = rest_opt {
            if rest.is_empty() {
                continue;
            }
            let (name_raw, alias) = if let Some((lhs, rhs)) = rest.split_once(" as ") {
                let alias = clean_ident(rhs.trim());
                (
                    clean_ident(lhs.trim()),
                    (!alias.is_empty()).then_some(alias),
                )
            } else {
                (clean_ident(rest), None)
            };
            if !name_raw.is_empty() {
                return Ok(Some((
                    StatementKind::StateDecl(StateDecl {
                        name: name_raw,
                        alias,
                        stereotype: Some(stereotype.to_string()),
                        style: Default::default(),
                        children: Vec::new(),
                        region_dividers: Vec::new(),
                    }),
                    start,
                )));
            }
        }
    }

    // `state Name` or `state Name <<stereotype>>` or `state Name { ... }`
    if line.starts_with("state ") {
        let rest = line.strip_prefix("state ").unwrap_or("").trim();
        if rest.is_empty() {
            return Ok(None);
        }

        let ParsedStateDeclHead {
            name_alias_part,
            description,
            stereotype,
            style,
            has_block,
        } = parse_state_decl_head(rest);

        // Extract alias
        let (name_raw, alias) = if let Some((lhs, rhs)) = name_alias_part.split_once(" as ") {
            let name = clean_ident(lhs.trim());
            let alias = clean_ident(rhs.trim());
            (name, if alias.is_empty() { None } else { Some(alias) })
        } else {
            (clean_ident(&name_alias_part), None)
        };

        if name_raw.is_empty() {
            return Ok(None);
        }

        if has_block {
            // Parse nested children until matching `}`
            let (children, region_dividers, end_idx) = parse_state_block(lines, start, &name_raw)?;
            let decl = StateDecl {
                name: name_raw,
                alias,
                stereotype,
                style,
                children,
                region_dividers,
            };
            return Ok(Some((StatementKind::StateDecl(decl), end_idx)));
        } else {
            let mut children = Vec::new();
            if let Some(description) = description {
                let action_state = name_raw.clone();
                children.push(Statement {
                    span: lines[start].1,
                    kind: StatementKind::StateInternalAction(StateInternalAction {
                        state: action_state,
                        kind: description,
                        action: String::new(),
                    }),
                });
            }
            let decl = StateDecl {
                name: name_raw,
                alias,
                stereotype,
                style,
                children,
                region_dividers: Vec::new(),
            };
            return Ok(Some((StatementKind::StateDecl(decl), start)));
        }
    }

    // Transition: `From --> To` or `From --> To : label`
    // Also handles `[*] --> X` and `X --> [*]`
    if let Some(transition) = parse_state_transition(line) {
        return Ok(Some((StatementKind::StateTransition(transition), start)));
    }

    // Internal action: `State : entry / action` or `State : exit / action` or `State : event / action`
    if let Some(action) = parse_state_internal_action(line) {
        return Ok(Some((StatementKind::StateInternalAction(action), start)));
    }

    Ok(None)
}

include!("state/block.rs");
include!("state/declaration.rs");
include!("state/transition.rs");

fn is_timeline_metadata_statement(kind: &StatementKind) -> bool {
    matches!(
        kind,
        StatementKind::Title(_)
            | StatementKind::Header(_)
            | StatementKind::Footer(_)
            | StatementKind::Caption(_)
            | StatementKind::Legend(_)
            | StatementKind::SkinParam { .. }
            | StatementKind::StyleParam { .. }
            | StatementKind::Theme(_)
            | StatementKind::Pragma(_)
    )
}
