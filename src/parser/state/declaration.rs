pub(crate) struct ParsedStateDeclHead {
    pub(crate) name_alias_part: String,
    pub(crate) description: Option<String>,
    pub(crate) stereotype: Option<String>,
    pub(crate) style: crate::ast::StateDeclStyle,
    pub(crate) has_block: bool,
}

pub(crate) fn parse_state_decl_head(rest: &str) -> ParsedStateDeclHead {
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

pub(crate) fn split_state_description(head: &mut String) -> Option<String> {
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

pub(crate) fn extract_state_stereotype(head: &mut String) -> Option<String> {
    let start = head.find("<<")?;
    let after = &head[start + 2..];
    let end_rel = after.find(">>")?;
    let end = start + 2 + end_rel + 2;
    let stereotype = head[start + 2..start + 2 + end_rel].trim().to_string();
    head.replace_range(start..end, " ");
    (!stereotype.is_empty()).then_some(stereotype)
}

pub(crate) fn extract_state_inline_style(head: &mut String) -> crate::ast::StateDeclStyle {
    let Some(style_start) = first_state_style_marker(head) else {
        return Default::default();
    };
    let style_part = head[style_start..].trim().to_string();
    head.truncate(style_start);
    parse_state_inline_style(&style_part)
}

pub(crate) fn first_state_style_marker(head: &str) -> Option<usize> {
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

pub(crate) fn parse_state_inline_style(style_part: &str) -> crate::ast::StateDeclStyle {
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

pub(crate) fn parse_state_style_token(token: &str, style: &mut crate::ast::StateDeclStyle) {
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

pub(crate) fn parse_state_border_modifier(rest: &str) -> (&str, &str) {
    if let Some(after_open) = rest.strip_prefix('[') {
        if let Some(end) = after_open.find(']') {
            let modifiers = &after_open[..end];
            let color = after_open[end + 1..].trim();
            return (modifiers, color);
        }
    }
    ("", rest)
}

pub(crate) fn apply_state_border_modifiers(
    modifiers: &str,
    style: &mut crate::ast::StateDeclStyle,
) {
    for modifier in modifiers.split(',').map(str::trim) {
        match modifier {
            "dashed" | "dotted" => style.border_dashed = true,
            "bold" => style.border_thickness = Some(3),
            _ => {}
        }
    }
}

pub(crate) fn normalize_state_color_token(token: &str) -> String {
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
