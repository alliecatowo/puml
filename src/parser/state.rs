fn parse_chronology_baseline_statement(line: &str) -> Option<StatementKind> {
    let trimmed = line.trim();
    if let Some(kind) = parse_keyword(trimmed) {
        return Some(kind);
    }
    if let Some((subject, rest)) = parse_chronology_subject_and_tail(trimmed, "happens") {
        let (when, end, color, bracket) = parse_chronology_time_tail(rest, false)?;
        if subject.is_empty() || when.is_empty() {
            return None;
        }
        return Some(StatementKind::ChronologyHappensOn {
            subject,
            when,
            end,
            color,
            bracket,
        });
    }
    for (keyword, bracket) in [("era", false), ("span", false), ("bracket", true)] {
        if let Some((subject, rest)) = parse_chronology_leading_span(trimmed, keyword) {
            let (when, end, color, _) = parse_chronology_time_tail(rest, bracket)?;
            return Some(StatementKind::ChronologyHappensOn {
                subject,
                when,
                end,
                color,
                bracket,
            });
        }
    }
    for (verb, bracket) in [("spans", false), ("brackets", true)] {
        if let Some((subject, rest)) = parse_chronology_subject_and_tail(trimmed, verb) {
            let (when, end, color, _) = parse_chronology_time_tail(rest, bracket)?;
            return Some(StatementKind::ChronologyHappensOn {
                subject,
                when,
                end,
                color,
                bracket,
            });
        }
    }
    // Accept ISO `YYYY-MM-DD : Label` shorthand
    if let Some((lhs, rhs)) = trimmed.split_once(':') {
        let (when, end, color, bracket) = parse_chronology_time_tail(lhs.trim(), false)?;
        let subject = rhs.trim().trim_matches('"');
        if !when.is_empty()
            && !subject.is_empty()
            && when.chars().next().is_some_and(|c| c.is_ascii_digit())
        {
            return Some(StatementKind::ChronologyHappensOn {
                subject: subject.to_string(),
                when,
                end,
                color,
                bracket,
            });
        }
    }
    None
}

fn parse_chronology_subject_and_tail<'a>(
    line: &'a str,
    verb: &str,
) -> Option<(String, &'a str)> {
    if let Some((subject, rest)) = parse_bracket_subject(line) {
        let rest = rest.trim();
        if rest
            .get(..verb.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(verb))
        {
            let tail = rest[verb.len()..].trim();
            return Some((clean_chronology_subject(&subject), tail));
        }
        return None;
    }

    let lower = line.to_ascii_lowercase();
    let marker = format!(" {verb} ");
    let idx = lower.find(&marker)?;
    let subject = clean_chronology_subject(&line[..idx]);
    let tail = line[idx + marker.len()..].trim();
    (!subject.is_empty()).then_some((subject, tail))
}

fn parse_chronology_leading_span<'a>(
    line: &'a str,
    keyword: &str,
) -> Option<(String, &'a str)> {
    let rest = line
        .strip_prefix(keyword)
        .or_else(|| {
            line.get(..keyword.len())
                .filter(|head| head.eq_ignore_ascii_case(keyword))
                .map(|_| &line[keyword.len()..])
        })?
        .trim();
    let lower = rest.to_ascii_lowercase();
    let idx = lower
        .find(" from ")
        .or_else(|| lower.find(" on "))
        .or_else(|| lower.find(" at "))
        .or_else(|| lower.find(" between "))
        .or_else(|| lower.find(" "));
    let idx = idx?;
    let subject = clean_chronology_subject(&rest[..idx]);
    let tail = rest[idx..].trim();
    (!subject.is_empty()).then_some((subject, tail))
}

fn clean_chronology_subject(raw: &str) -> String {
    raw.trim()
        .trim_matches('"')
        .trim_matches('[')
        .trim_matches(']')
        .trim()
        .to_string()
}

fn parse_chronology_time_tail(
    raw: &str,
    default_bracket: bool,
) -> Option<(String, Option<String>, Option<String>, bool)> {
    let (tail, color) = split_chronology_color(raw);
    let mut bracket = default_bracket;
    let mut trimmed = tail.trim();
    let lower = trimmed.to_ascii_lowercase();
    for prefix in ["the ", "on ", "at ", "from ", "between "] {
        if lower.starts_with(prefix) {
            trimmed = trimmed[prefix.len()..].trim();
            break;
        }
    }
    if let Some(rest) = trimmed.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
        bracket = true;
        trimmed = rest.trim();
    }
    let lower = trimmed.to_ascii_lowercase();
    let (when, end) = if let Some(idx) = lower.find(" to ") {
        (
            trimmed[..idx].trim().to_string(),
            Some(trimmed[idx + " to ".len()..].trim().to_string()),
        )
    } else if let Some(idx) = lower.find(" through ") {
        (
            trimmed[..idx].trim().to_string(),
            Some(trimmed[idx + " through ".len()..].trim().to_string()),
        )
    } else if let Some(idx) = lower.find(" until ") {
        (
            trimmed[..idx].trim().to_string(),
            Some(trimmed[idx + " until ".len()..].trim().to_string()),
        )
    } else if let Some(idx) = lower.find(" and ") {
        (
            trimmed[..idx].trim().to_string(),
            Some(trimmed[idx + " and ".len()..].trim().to_string()),
        )
    } else {
        (trimmed.to_string(), None)
    };
    if when.is_empty() || end.as_deref().is_some_and(str::is_empty) {
        return None;
    }
    Some((when, end, color, bracket))
}

fn split_chronology_color(raw: &str) -> (&str, Option<String>) {
    let lower = raw.to_ascii_lowercase();
    for marker in [" is colored in ", " is coloured in ", " is colored ", " is coloured "] {
        if let Some(idx) = lower.rfind(marker) {
            let color = raw[idx + marker.len()..].trim();
            if !color.is_empty() {
                return (raw[..idx].trim(), Some(color.to_string()));
            }
        }
    }
    (raw, None)
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
