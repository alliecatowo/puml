use super::*;
/// Parse an inline `json $alias { ... }` or `yaml $alias { ... }` block.
/// Returns the projection statement and closing line index if found, else `None`.
/// Errors if a projection block is found but no matching closing `}` appears.
pub(crate) fn parse_json_projection_block(
    lines: &[(&str, Span)],
    start: usize,
    line: &str,
) -> Result<Option<(StatementKind, usize)>, Diagnostic> {
    // Match: `json|yaml` <whitespace> <identifier starting with optional $> `{`
    let lower = line.to_ascii_lowercase();
    let (keyword, is_yaml) = if lower.starts_with("json ") {
        ("json", false)
    } else if lower.starts_with("yaml ") {
        ("yaml", true)
    } else {
        return Ok(None);
    };
    let rest = line[keyword.len() + 1..].trim();
    if rest.is_empty() {
        return Ok(None);
    }

    // Parse alias (identifier, optionally starting with `$`)
    let (alias, after_alias) = {
        let mut end = 0;
        let chars: Vec<char> = rest.chars().collect();
        if chars.is_empty() {
            return Ok(None);
        }
        // Allow `$identifier` or plain `identifier`
        if chars[0] == '$' {
            end += 1;
        }
        while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
            end += 1;
        }
        if end == 0 || (end == 1 && rest.starts_with('$')) {
            return Ok(None);
        }
        let alias = rest[..end].to_string();
        let after = rest[end..].trim();
        (alias, after)
    };

    // Must be followed by `{`
    if !after_alias.starts_with('{') {
        return Ok(None);
    }

    // Accumulate body lines until the matching `}` (depth-tracked).
    let mut body_lines: Vec<&str> = Vec::new();
    // The opening `{` may have content after it on the same line.
    let inline_after_brace = after_alias[1..].trim();
    let mut depth: i32 = 1;

    // If everything is on one line: `json $alias { ... }`
    if !inline_after_brace.is_empty() {
        let mut in_quotes = false;
        let mut prev_escape = false;
        for (j, ch) in inline_after_brace.char_indices() {
            if in_quotes {
                if ch == '"' && !prev_escape {
                    in_quotes = false;
                }
                prev_escape = ch == '\\' && !prev_escape;
                continue;
            }
            match ch {
                '"' => in_quotes = true,
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        let body = dedent_projection_body(&[inline_after_brace[..j].trim()]);
                        let kind = if is_yaml {
                            StatementKind::YamlProjection { alias, body }
                        } else {
                            StatementKind::JsonProjection { alias, body }
                        };
                        return Ok(Some((kind, start)));
                    }
                }
                _ => {}
            }
            prev_escape = false;
        }
        // Depth > 0: content continues on next lines.
        body_lines.push(inline_after_brace);
    }

    // Continue scanning subsequent lines.
    let mut i = start + 1;
    while i < lines.len() {
        let (raw, _span) = lines[i];
        // Check for matching closing brace.
        let mut consumed_close = false;
        let mut close_pos = 0;
        let mut in_quotes = false;
        let mut prev_escape = false;
        for (pos, ch) in raw.char_indices() {
            if in_quotes {
                if ch == '"' && !prev_escape {
                    in_quotes = false;
                }
                prev_escape = ch == '\\' && !prev_escape;
                continue;
            }
            match ch {
                '"' => in_quotes = true,
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        consumed_close = true;
                        close_pos = pos;
                        break;
                    }
                }
                _ => {}
            }
            prev_escape = false;
        }
        if consumed_close {
            // Everything before the closing `}` is part of the body.
            let last_body = raw[..close_pos].trim_end();
            if !last_body.is_empty() {
                body_lines.push(last_body);
            }
            let body = dedent_projection_body(&body_lines);
            let kind = if is_yaml {
                StatementKind::YamlProjection { alias, body }
            } else {
                StatementKind::JsonProjection { alias, body }
            };
            return Ok(Some((kind, i)));
        }
        body_lines.push(raw.trim_end());
        i += 1;
    }

    // No closing brace found.
    Err(Diagnostic::error(format!(
        "[E_PROJECTION_UNCLOSED] `{keyword} {alias}` block has no matching closing `}}`"
    ))
    .with_span(lines[start].1))
}

pub(crate) fn dedent_projection_body(lines: &[&str]) -> String {
    let common_indent = lines
        .iter()
        .filter_map(|line| {
            if line.trim().is_empty() {
                None
            } else {
                Some(line.chars().take_while(|ch| *ch == ' ').count())
            }
        })
        .min()
        .unwrap_or(0);
    let prefix = " ".repeat(common_indent);

    lines
        .iter()
        .map(|line| line.strip_prefix(&prefix).unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Parse a single salt wireframe row line into a `SaltGridRow` statement.
/// A row is a `|`-delimited sequence of cell tokens.
/// Returns `None` if the line does not start with `|`.
pub(crate) fn parse_salt_grid_row(line: &str) -> Option<StatementKind> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    // `^combo^` or `^open^^item1^^item2^` patterns are whole-line combo/droplist widgets.
    let is_combo_line = trimmed.starts_with('^') && trimmed.ends_with('^') && trimmed.len() >= 3;
    // `[====  ]` progress bar — must only contain `=` and spaces inside `[...]`.
    let is_progress_bar =
        trimmed.starts_with('[') && trimmed.ends_with(']') && trimmed.len() >= 3 && {
            let inner = &trimmed[1..trimmed.len() - 1];
            !inner.is_empty() && inner.chars().all(|c| c == '=' || c == ' ') && inner.contains('=')
        };
    let whole_line_widget = trimmed == "}"
        || lower.starts_with("{*")
        || lower.starts_with("{/")
        || lower.starts_with("{s")
        || lower.starts_with("{t")
        || lower.starts_with("{+")
        || lower.starts_with("{#")
        || lower.starts_with("{!")
        || lower.starts_with("{^")
        || lower == "tree"
        || lower.starts_with("tree ")
        || lower == "menu"
        || lower.starts_with("menu ")
        || lower == "tab"
        || lower.starts_with("tab ")
        || lower == "tabs"
        || lower.starts_with("tabs ")
        || lower.starts_with("scroll")
        || lower.contains("scrollbar")
        || is_combo_line
        || is_progress_bar;
    if whole_line_widget {
        return Some(StatementKind::SaltGridRow {
            cells: vec![SaltCell::Label(trimmed.to_string())],
        });
    }
    if !trimmed.contains('|') {
        return None;
    }
    // Split on `|` and parse each cell token.
    let parts: Vec<&str> = trimmed.split('|').collect();
    let mut cells = Vec::new();
    for part in parts {
        let cell_text = part.trim();
        if cell_text.is_empty() {
            continue;
        }
        cells.push(parse_salt_cell(cell_text));
    }
    if cells.is_empty() {
        return None;
    }
    Some(StatementKind::SaltGridRow { cells })
}

/// Parse a single salt cell token into a `SaltCell` variant.
pub(crate) fn parse_salt_cell(text: &str) -> SaltCell {
    // `"placeholder"` → Input
    if text.starts_with('"') && text.ends_with('"') && text.len() >= 2 {
        let inner = &text[1..text.len() - 1];
        return SaltCell::Input(inner.to_string());
    }
    // `[X] label`, `[ ] label`, or compact `[] label` → Checkbox
    if text.starts_with("[X]") || text.starts_with("[x]") {
        let label = text[3..].trim().to_string();
        return SaltCell::CheckboxChecked(label);
    }
    if let Some(rest) = text.strip_prefix("[ ]") {
        return SaltCell::CheckboxUnchecked(rest.trim().to_string());
    }
    if let Some(rest) = text.strip_prefix("[]") {
        return SaltCell::CheckboxUnchecked(rest.trim().to_string());
    }
    // `(X) label`, `( ) label`, or compact `() label` → Radio
    if text.starts_with("(X)") || text.starts_with("(x)") {
        let label = text[3..].trim().to_string();
        return SaltCell::RadioOn(label);
    }
    if let Some(rest) = text.strip_prefix("( )") {
        return SaltCell::RadioOff(rest.trim().to_string());
    }
    if let Some(rest) = text.strip_prefix("()") {
        return SaltCell::RadioOff(rest.trim().to_string());
    }
    // `[====  ]` / `[========]` → progress bar (before button check to avoid clash)
    if text.starts_with('[') && text.ends_with(']') && text.len() >= 3 {
        let inner = &text[1..text.len() - 1];
        if !inner.is_empty() && inner.chars().all(|c| c == '=' || c == ' ') && inner.contains('=') {
            // Encode as a special label token; the renderer will decode it.
            return SaltCell::Label(text.to_string());
        }
    }
    // `[button text]` → Button
    if text.starts_with('[') && text.ends_with(']') && text.len() >= 2 {
        let inner = &text[1..text.len() - 1];
        return SaltCell::Button(inner.to_string());
    }
    // `^combo text^` → Combo
    if text.starts_with('^') && text.ends_with('^') && text.len() >= 2 {
        let inner = &text[1..text.len() - 1];
        return SaltCell::Combo(inner.to_string());
    }
    // Plain text → Label
    SaltCell::Label(text.to_string())
}
