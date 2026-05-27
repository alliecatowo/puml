use super::*;
/// Parse the body of a `state X { ... }` block.
/// Returns (children, region_divider_indices, end_line_index).
pub(crate) fn parse_state_block(
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
            kind: StatementKind::UnsupportedSyntax(inner.to_string()),
        });
        j += 1;
    }

    // Unclosed block — treat as if closed at EOF
    Ok((children, region_dividers, lines.len().saturating_sub(1)))
}
