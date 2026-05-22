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

/// Parse the body of a `state X { ... }` block.
/// Returns (children, region_divider_indices, end_line_index).
fn parse_state_block(
    lines: &[(&str, Span)],
    start: usize,
    parent_state: &str,
) -> Result<(Vec<Statement>, Vec<usize>, usize), Diagnostic> {
    let mut children: Vec<Statement> = Vec::new();
    let mut region_dividers: Vec<usize> = Vec::new();
    let mut j = start + 1;

    while j < lines.len() {
        let (raw, span) = lines[j];
        let inner = raw.trim();

        // Closing brace — end of this block
        if inner == "}" {
            return Ok((children, region_dividers, j));
        }

        // Skip blank lines and comments
        if inner.is_empty() || inner.starts_with('\'') {
            j += 1;
            continue;
        }

        // `||` or `--` region divider (both are PlantUML concurrent-region separators)
        if inner == "||" || inner == "--" {
            region_dividers.push(children.len());
            j += 1;
            continue;
        }

        // History pseudo-states
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

        if let Some((kind, end_idx)) = parse_json_projection_block(lines, j, inner)? {
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

        if let Some((kind, end_idx)) = parse_multiline_note_block(lines, j, inner) {
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

        // Try to parse as a state statement (handles `state X { ... }` recursively)
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

        // Unknown brace block: skip over it by tracking depth manually
        if inner.ends_with('{') || inner == "{" {
            let mut depth = 1i32;
            j += 1;
            while j < lines.len() && depth > 0 {
                let (braw, _) = lines[j];
                let binner = braw.trim();
                if binner.ends_with('{') || binner == "{" {
                    depth += 1;
                }
                if binner == "}" {
                    depth -= 1;
                }
                j += 1;
            }
            continue;
        }

        // Unknown line inside block — store for normalizer
        children.push(Statement {
            span,
            kind: StatementKind::Unknown(inner.to_string()),
        });
        j += 1;
    }

    // Unclosed block — treat as if closed at EOF
    Ok((children, region_dividers, lines.len().saturating_sub(1)))
}

struct ParsedStateDeclHead {
    name_alias_part: String,
    description: Option<String>,
    stereotype: Option<String>,
    style: crate::ast::StateDeclStyle,
    has_block: bool,
}

fn parse_state_decl_head(rest: &str) -> ParsedStateDeclHead {
    let mut head = rest.trim().to_string();
    let has_block = head.ends_with('{');
    if has_block {
        head = head.trim_end_matches('{').trim().to_string();
    }

    let stereotype = extract_state_stereotype(&mut head);
    let style = extract_state_inline_style(&mut head);
    let description = split_state_description(&mut head);

    ParsedStateDeclHead {
        name_alias_part: head.trim().to_string(),
        description,
        stereotype,
        style,
        has_block,
    }
}

fn split_state_description(head: &mut String) -> Option<String> {
    let mut in_quotes = false;
    let mut prev = '\0';
    for (idx, ch) in head.char_indices() {
        if ch == '"' && prev != '\\' {
            in_quotes = !in_quotes;
        }
        if ch == ':' && !in_quotes {
            let description = head[idx + ch.len_utf8()..].trim().to_string();
            head.truncate(idx);
            return (!description.is_empty()).then_some(description);
        }
        prev = ch;
    }
    None
}

fn extract_state_stereotype(head: &mut String) -> Option<String> {
    let start = head.find("<<")?;
    let after = &head[start + 2..];
    let end_rel = after.find(">>")?;
    let end = start + 2 + end_rel + 2;
    let stereotype = head[start + 2..start + 2 + end_rel].trim().to_string();
    head.replace_range(start..end, " ");
    (!stereotype.is_empty()).then_some(stereotype)
}

fn extract_state_inline_style(head: &mut String) -> crate::ast::StateDeclStyle {
    let Some(style_start) = first_state_style_marker(head) else {
        return Default::default();
    };
    let style_part = head[style_start..].trim().to_string();
    head.truncate(style_start);
    parse_state_inline_style(&style_part)
}

fn first_state_style_marker(head: &str) -> Option<usize> {
    let mut in_quotes = false;
    let mut prev = '\0';
    for (idx, ch) in head.char_indices() {
        if ch == '"' && prev != '\\' {
            in_quotes = !in_quotes;
        }
        if ch == '#' && !in_quotes {
            return Some(idx);
        }
        prev = ch;
    }
    None
}

fn parse_state_inline_style(style_part: &str) -> crate::ast::StateDeclStyle {
    let mut style = crate::ast::StateDeclStyle::default();
    let compact = style_part.split_whitespace().collect::<String>();
    let mut tokens = Vec::new();
    let mut rest = compact.as_str();
    while let Some(stripped) = rest.strip_prefix('#') {
        let marker_len = if stripped.starts_with('#') || stripped.starts_with('[') {
            2.min(rest.len())
        } else {
            1
        };
        let body = &rest[marker_len..];
        let next = [body.find("##"), body.find("#[")]
            .into_iter()
            .flatten()
            .min();
        let (token, tail) = if let Some(next) = next {
            (&rest[..marker_len + next], &body[next..])
        } else {
            (rest, "")
        };
        tokens.push(token);
        rest = tail;
        if rest.is_empty() {
            break;
        }
    }
    for token in tokens {
        parse_state_style_token(token, &mut style);
    }
    style
}

fn parse_state_style_token(token: &str, style: &mut crate::ast::StateDeclStyle) {
    let token = token.trim().trim_end_matches(';');
    if token.is_empty() {
        return;
    }
    if let Some(rest) = token
        .strip_prefix("##")
        .or_else(|| token.strip_prefix('#').filter(|rest| rest.starts_with('[')))
    {
        let (modifiers, color) = parse_state_border_modifier(rest);
        apply_state_border_modifiers(modifiers, style);
        if !color.is_empty() {
            style.border_color = Some(normalize_state_color_token(color));
        }
        return;
    }
    let Some(rest) = token.strip_prefix('#') else {
        return;
    };
    if let Some(open) = rest.find('[') {
        if let Some(close_rel) = rest[open + 1..].find(']') {
            let fill = rest[..open].trim();
            let modifiers = &rest[open + 1..open + 1 + close_rel];
            let border = rest[open + 1 + close_rel + 1..].trim();
            if !fill.is_empty() {
                style.fill_color = Some(normalize_state_color_token(fill));
            }
            apply_state_border_modifiers(modifiers, style);
            if !border.is_empty() {
                style.border_color = Some(normalize_state_color_token(border));
            }
            return;
        }
    }
    if rest.contains(':') || rest.contains(';') {
        for part in rest.split(';') {
            let part = part.trim();
            if let Some(value) = part.strip_prefix("back:") {
                style.fill_color = Some(normalize_state_color_token(value));
            } else if let Some(value) = part.strip_prefix("line:") {
                style.border_color = Some(normalize_state_color_token(value));
            } else if let Some(value) = part.strip_prefix("text:") {
                style.text_color = Some(normalize_state_color_token(value));
            } else if part == "line.dashed" || part == "line.dotted" {
                style.border_dashed = true;
            } else if part == "line.bold" {
                style.border_thickness = Some(3);
            }
        }
    } else {
        style.fill_color = Some(normalize_state_color_token(rest));
    }
}

fn parse_state_border_modifier(rest: &str) -> (&str, &str) {
    if let Some(after_open) = rest.strip_prefix('[') {
        if let Some(end) = after_open.find(']') {
            let modifiers = &after_open[..end];
            let color = after_open[end + 1..].trim();
            return (modifiers, color);
        }
    }
    ("", rest)
}

fn apply_state_border_modifiers(modifiers: &str, style: &mut crate::ast::StateDeclStyle) {
    for modifier in modifiers.split(',').map(str::trim) {
        match modifier {
            "dashed" | "dotted" => style.border_dashed = true,
            "bold" => style.border_thickness = Some(3),
            _ => {}
        }
    }
}

fn normalize_state_color_token(token: &str) -> String {
    let raw = token.trim().trim_start_matches('#');
    let is_hex = matches!(raw.len(), 3 | 4 | 6 | 8) && raw.chars().all(|c| c.is_ascii_hexdigit());
    if is_hex {
        format!("#{raw}")
    } else if raw.contains('-') || raw.contains('|') {
        raw.split(['-', '|'])
            .next()
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .unwrap_or(raw)
            .to_string()
    } else {
        raw.to_string()
    }
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
