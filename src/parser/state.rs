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
        if let Some(rest) = line.strip_prefix(keyword).map(str::trim) {
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

        // Extract optional stereotype `<<...>>`
        let (name_part, stereotype) = if let Some(idx) = rest.find("<<") {
            let name = rest[..idx].trim();
            let after = &rest[idx + 2..];
            let stereo = after.find(">>").map(|end| after[..end].trim().to_string());
            (name, stereo)
        } else {
            (rest, None)
        };

        // Check if there's a block
        let (name_alias_part, has_block) = if name_part.ends_with('{') {
            (name_part.trim_end_matches('{').trim(), true)
        } else {
            (name_part, false)
        };

        // Extract alias
        let (name_raw, alias) = if let Some((lhs, rhs)) = name_alias_part.split_once(" as ") {
            let name = clean_ident(lhs.trim());
            let alias = clean_ident(rhs.trim());
            (name, if alias.is_empty() { None } else { Some(alias) })
        } else {
            (clean_ident(name_alias_part), None)
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
                children,
                region_dividers,
            };
            return Ok(Some((StatementKind::StateDecl(decl), end_idx)));
        } else {
            let decl = StateDecl {
                name: name_raw,
                alias,
                stereotype,
                children: Vec::new(),
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

/// Parse the body of a `state X { ... }` block.
/// Returns (children, region_divider_indices, end_line_index).
fn parse_state_block(
    lines: &[(&str, Span)],
    start: usize,
    parent_state: &str,
) -> Result<(Vec<Statement>, Vec<usize>, usize), Diagnostic> {
    let mut children: Vec<Statement> = Vec::new();
    let mut region_dividers: Vec<usize> = Vec::new();
    let mut depth = 1i32;
    let mut j = start + 1;

    while j < lines.len() {
        let (raw, span) = lines[j];
        let inner = raw.trim();

        if inner.ends_with('{') || inner == "{" {
            depth += 1;
        }
        if inner == "}" {
            depth -= 1;
            if depth == 0 {
                return Ok((children, region_dividers, j));
            }
        }

        // `||` region divider
        if inner == "||" && depth == 1 {
            region_dividers.push(children.len());
            j += 1;
            continue;
        }

        // Recurse for nested state declarations inside a block
        if depth == 1 {
            if inner.is_empty() || inner.starts_with('\'') {
                j += 1;
                continue;
            }
            if inner == "[H]" {
                children.push(Statement {
                    span,
                    kind: StatementKind::StateHistory { deep: false },
                });
                j += 1;
                continue;
            }
            if inner == "[H*]" {
                children.push(Statement {
                    span,
                    kind: StatementKind::StateHistory { deep: true },
                });
                j += 1;
                continue;
            }
            if let Some((kind, end_idx)) = parse_state_statement(lines, j, inner)? {
                children.push(Statement {
                    span: if end_idx > j {
                        Span::new(span.start, lines[end_idx].1.end)
                    } else {
                        span
                    },
                    kind,
                });
                j = end_idx + 1;
                continue;
            }
            if let Some(transition) = parse_state_transition(inner) {
                children.push(Statement {
                    span,
                    kind: StatementKind::StateTransition(transition),
                });
                j += 1;
                continue;
            }
            if let Some(action) = parse_state_internal_action(inner) {
                children.push(Statement {
                    span,
                    kind: StatementKind::StateInternalAction(action),
                });
                j += 1;
                continue;
            }
            if let Some(action) = parse_state_bare_internal_action(parent_state, inner) {
                children.push(Statement {
                    span,
                    kind: StatementKind::StateInternalAction(action),
                });
                j += 1;
                continue;
            }
            if let Some(kind) = parse_keyword(inner) {
                children.push(Statement { span, kind });
                j += 1;
                continue;
            }
            // Unknown line inside block — store for normalizer
            children.push(Statement {
                span,
                kind: StatementKind::Unknown(inner.to_string()),
            });
        }
        j += 1;
    }

    // Unclosed block — treat as if closed at EOF
    Ok((children, region_dividers, lines.len().saturating_sub(1)))
}

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

fn is_timeline_metadata_statement(kind: &StatementKind) -> bool {
    matches!(
        kind,
        StatementKind::Title(_)
            | StatementKind::Header(_)
            | StatementKind::Footer(_)
            | StatementKind::Caption(_)
            | StatementKind::Legend(_)
            | StatementKind::SkinParam { .. }
            | StatementKind::Theme(_)
            | StatementKind::Pragma(_)
    )
}
