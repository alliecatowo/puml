fn parse_message(line: &str) -> Option<StatementKind> {
    let (line, parallel) = split_parallel_message_prefix(line);
    let (core, label) = split_message_label(line);
    let (lhs_raw, arrow, rhs_raw) = split_arrow(core)?;
    let mut style = parse_arrow_style(arrow);
    style.parallel = parallel;
    let parsed_arrow = parse_arrow(arrow)?;
    let (from_id_raw, from_modifier) = split_lifecycle_modifier(lhs_raw);
    let (to_id_raw, to_modifier) = split_lifecycle_modifier(rhs_raw);

    let from = if let Some(v) = normalize_virtual_endpoint(from_id_raw) {
        v
    } else {
        if looks_like_virtual_endpoint_syntax(from_id_raw) {
            return None;
        }
        clean_ident(from_id_raw)
    };
    let to = if let Some(v) = normalize_virtual_endpoint(to_id_raw) {
        v
    } else {
        if looks_like_virtual_endpoint_syntax(to_id_raw) {
            return None;
        }
        clean_ident(to_id_raw)
    };

    if from.is_empty() || to.is_empty() {
        return None;
    }

    let mut arrow_encoded = parsed_arrow.to_string();
    if let Some(modifier) = from_modifier {
        arrow_encoded.push_str("@L");
        arrow_encoded.push_str(modifier);
    }
    if let Some(modifier) = to_modifier {
        arrow_encoded.push_str("@R");
        arrow_encoded.push_str(modifier);
    }

    let from_virtual = ast_virtual_endpoint_from_id(&from, true);
    let to_virtual = ast_virtual_endpoint_from_id(&to, false);
    Some(StatementKind::Message(Message {
        from,
        to,
        arrow: arrow_encoded,
        label,
        style,
        from_virtual,
        to_virtual,
    }))
}

fn split_parallel_message_prefix(line: &str) -> (&str, bool) {
    let trimmed = line.trim_start();
    if let Some(rest) = trimmed.strip_prefix('&') {
        let rest = rest.trim_start();
        if !rest.is_empty() {
            return (rest, true);
        }
    }
    (line, false)
}

fn parse_arrow_style(arrow: &str) -> MessageStyle {
    let mut style = MessageStyle::default();
    if strip_sequence_arrow_brackets(arrow).contains('.') {
        style.dotted = true;
    }
    let mut chars = arrow.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '[' {
            continue;
        }
        let mut body = String::new();
        for inner in chars.by_ref() {
            if inner == ']' {
                break;
            }
            body.push(inner);
        }
        for token in body
            .split([',', ';'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let lower = token.to_ascii_lowercase();
            match lower.as_str() {
                "hidden" | "line.hidden" => style.hidden = true,
                "dashed" | "line.dashed" => style.dashed = true,
                "dotted" | "line.dotted" => style.dotted = true,
                "bold" | "thick" | "line.bold" | "line.thick" => style.thickness = Some(3),
                "thin" | "line.thin" => style.thickness = Some(1),
                _ if token.starts_with('#')
                    && matches!(token.len(), 4 | 5 | 7 | 9)
                    && token[1..].bytes().all(|b| b.is_ascii_hexdigit()) =>
                {
                    style.color = Some(format!("#{}", token[1..].to_ascii_lowercase()));
                }
                _ if token.starts_with('#')
                    && token[1..].bytes().all(|b| b.is_ascii_alphabetic()) =>
                {
                    style.color = Some(token[1..].to_ascii_lowercase());
                }
                _ if token.bytes().all(|b| b.is_ascii_alphabetic()) => {
                    style.color = Some(lower);
                }
                _ => {
                    if let Some(value) = lower
                        .strip_prefix("thickness=")
                        .or_else(|| lower.strip_prefix("thickness:"))
                        .or_else(|| lower.strip_prefix("thickness "))
                        .or_else(|| lower.strip_prefix("line.thickness="))
                        .or_else(|| lower.strip_prefix("line.thickness:"))
                        .or_else(|| lower.strip_prefix("line.thickness "))
                    {
                        if let Ok(n) = value.trim().parse::<u8>() {
                            style.thickness = Some(n.clamp(1, 8));
                        }
                    }
                }
            }
        }
    }
    style
}

fn ast_virtual_endpoint_from_id(id: &str, is_from: bool) -> Option<VirtualEndpoint> {
    let (side, kind) = match id {
        "[" => (VirtualEndpointSide::Left, VirtualEndpointKind::Plain),
        "]" => (VirtualEndpointSide::Right, VirtualEndpointKind::Plain),
        "[o" => (VirtualEndpointSide::Left, VirtualEndpointKind::Circle),
        "o]" => (VirtualEndpointSide::Right, VirtualEndpointKind::Circle),
        "[x" => (VirtualEndpointSide::Left, VirtualEndpointKind::Cross),
        "x]" => (VirtualEndpointSide::Right, VirtualEndpointKind::Cross),
        "[*]" => (
            if is_from {
                VirtualEndpointSide::Left
            } else {
                VirtualEndpointSide::Right
            },
            VirtualEndpointKind::Filled,
        ),
        // `?` short-arrow endpoints (feature 1.30): same side as `[`/`]` but
        // rendered as a stub from the diagram edge rather than a full endpoint.
        "?" => (
            if is_from {
                VirtualEndpointSide::Left
            } else {
                VirtualEndpointSide::Right
            },
            VirtualEndpointKind::Short,
        ),
        _ => return None,
    };
    Some(VirtualEndpoint { side, kind })
}

fn split_family_relation_label(line: &str) -> (&str, Option<String>) {
    if split_family_arrow(line).is_none() {
        return split_message_label(line);
    }
    if let Some(colon) = line.rfind(" :") {
        let suffix = line[colon + 2..].trim();
        if !suffix_has_family_relation_arrow(suffix) {
            let text = line[colon + 2..].trim();
            if !text.is_empty() {
                return (line[..colon].trim_end(), Some(text.to_string()));
            }
        }
    }
    let mut in_quote = false;
    let mut last_colon = None;
    for (idx, ch) in line.char_indices() {
        if ch == '"' {
            in_quote = !in_quote;
            continue;
        }
        if !in_quote
            && ch == ':'
            && !line[..idx].ends_with(':')
            && !line[idx + ch.len_utf8()..].starts_with(':')
        {
            last_colon = Some(idx);
        }
    }
    if let Some(colon) = last_colon {
        let prefix = line[..colon].trim_end();
        let suffix = line[colon + 1..].trim();
        if !suffix.is_empty()
            && !suffix_has_family_relation_arrow(suffix)
            && split_family_arrow(prefix).is_some()
        {
            return (prefix, Some(suffix.to_string()));
        }
    }
    (line.trim_end(), None)
}

fn suffix_has_family_relation_arrow(suffix: &str) -> bool {
    suffix.contains("--")
        || suffix.contains("..")
        || suffix.contains("->")
        || suffix.contains("<-")
        || suffix.contains("|>")
        || suffix.contains("<|")
}

fn split_message_label(line: &str) -> (&str, Option<String>) {
    if let Some(colon) = line.find(':') {
        let text = line[colon + 1..].trim();
        (
            line[..colon].trim_end(),
            Some(text.to_string()).filter(|s| !s.is_empty()),
        )
    } else {
        (line.trim_end(), None)
    }
}

fn split_arrow(core: &str) -> Option<(&str, &str, &str)> {
    fn is_arrow_char(c: char) -> bool {
        matches!(
            c,
            '-' | '.' | '<' | '>' | '[' | ']' | 'o' | 'x' | '/' | '\\'
        )
    }

    let mut run_start: Option<usize> = None;
    let mut in_bracket = false;
    let mut skip_until = 0usize;
    for (idx, ch) in core.char_indices() {
        if idx < skip_until {
            continue;
        }
        if let Some(start) = run_start {
            if in_bracket {
                if ch == ']' {
                    in_bracket = false;
                }
                continue;
            }
            if ch == '[' {
                in_bracket = true;
                continue;
            }
            if is_arrow_char(ch) {
                continue;
            }
            let candidate = &core[start..idx];
            if !candidate.contains('-')
                && !(candidate.contains('.')
                    && (candidate.contains('<') || candidate.contains('>')))
            {
                run_start = None;
                continue;
            }
            let lhs = core[..start].trim();
            let rhs = core[idx..].trim();
            if !lhs.is_empty() && !rhs.is_empty() {
                return Some((lhs, candidate.trim(), rhs));
            }
            run_start = None;
            continue;
        }
        if ch == '[' && core[..idx].trim().is_empty() {
            let mut skipped_open_endpoint = false;
            for endpoint in ["[o", "[x"] {
                if core[idx..].starts_with(endpoint)
                    && core[idx + endpoint.len()..]
                        .chars()
                        .next()
                        .is_some_and(char::is_whitespace)
                {
                    skip_until = idx + endpoint.len();
                    skipped_open_endpoint = true;
                    break;
                }
            }
            if skipped_open_endpoint {
                continue;
            }
            if let Some(close_rel) = core[idx..].find(']') {
                let bracket_body = &core[idx + ch.len_utf8()..idx + close_rel];
                if bracket_body.contains('-') {
                    continue;
                }
                let after_idx = idx + close_rel + 1;
                if core[after_idx..]
                    .chars()
                    .next()
                    .is_some_and(char::is_whitespace)
                {
                    skip_until = after_idx;
                    continue;
                }
            } else if core[idx + ch.len_utf8()..]
                .chars()
                .next()
                .is_some_and(char::is_whitespace)
            {
                continue;
            }
        }
        if is_arrow_char(ch) {
            if run_start.is_none() {
                run_start = Some(idx);
            }
            if ch == '[' {
                in_bracket = true;
            }
            continue;
        }
    }
    if let Some(start) = run_start {
        let candidate = &core[start..];
        if !candidate.contains('-')
            && !(candidate.contains('.') && (candidate.contains('<') || candidate.contains('>')))
        {
            return None;
        }
        let lhs = core[..start].trim();
        if lhs.is_empty() {
            return None;
        }
        return Some((lhs, candidate.trim(), ""));
    }
    None
}

fn parse_arrow(arrow: &str) -> Option<String> {
    const VALID_BASE_ARROWS: &[&str] = &[
        "->", "-->", "->>", "-->>", "<-", "<--", "<<-", "<<--", "<->", "<-->", "<<->>", "<<-->>",
    ];
    let arrow = strip_sequence_arrow_brackets(arrow);
    let mut squashed = String::with_capacity(arrow.len());
    let mut last_slash: Option<char> = None;
    let mut slash_run_len = 0usize;
    for ch in arrow.chars() {
        if matches!(ch, '/' | '\\') {
            if last_slash == Some(ch) {
                slash_run_len += 1;
            } else {
                last_slash = Some(ch);
                slash_run_len = 1;
            }
            if ch == '/' && slash_run_len > 1 {
                // Portable slash forms allow a single slash marker only.
                return None;
            }
            if slash_run_len == 1 {
                squashed.push(ch);
            }
            continue;
        }
        last_slash = None;
        slash_run_len = 0;
        squashed.push(ch);
    }

    let canonical = squashed.replace(['/', '\\'], "").replace('.', "-");
    if canonical.is_empty()
        || !canonical
            .chars()
            .all(|c| matches!(c, '-' | '<' | '>' | 'o' | 'x'))
        || !squashed
            .chars()
            .all(|c| matches!(c, '-' | '.' | '<' | '>' | 'o' | 'x' | '/' | '\\'))
    {
        return None;
    }
    let has_slash_marker = squashed.contains('/') || squashed.contains('\\');
    let has_dot_marker = squashed.contains('.');
    let expanded_marker = squashed.contains("-/") || squashed.contains("-\\");

    if has_slash_marker && matches!(canonical.as_str(), "-" | "--") {
        return Some(squashed);
    }

    if VALID_BASE_ARROWS.contains(&canonical.as_str()) {
        if has_dot_marker {
            return Some(canonical);
        }
        if has_slash_marker && !expanded_marker {
            return Some(canonical);
        }
        if expanded_marker
            && squashed.contains("-\\")
            && canonical == "-->>"
            && squashed.contains("->>")
        {
            return Some(squashed.replacen("->>", "-->>", 1));
        }
        return Some(squashed);
    }
    let with_left_trimmed = canonical
        .strip_prefix('o')
        .or_else(|| canonical.strip_prefix('x'))
        .unwrap_or(&canonical);
    let (core, right_marker_removed) = if let Some(stripped) = with_left_trimmed.strip_suffix('o') {
        (stripped, true)
    } else if let Some(stripped) = with_left_trimmed.strip_suffix('x') {
        (stripped, true)
    } else {
        (with_left_trimmed, false)
    };
    if core.is_empty() {
        return None;
    }
    if VALID_BASE_ARROWS.contains(&core) && (right_marker_removed || core != canonical) {
        if has_dot_marker {
            return Some(canonical);
        }
        if has_slash_marker && !expanded_marker {
            let mut out = core.to_string();
            if let Some(ch) = with_left_trimmed.chars().last() {
                if matches!(ch, 'o' | 'x') && right_marker_removed {
                    out.push(ch);
                }
            }
            return Some(out);
        }
        if expanded_marker && canonical.contains("-->>") && squashed.contains("->>") {
            return Some(squashed.replacen("->>", "-->>", 1));
        }
        return Some(squashed);
    }
    if let Some(stripped_core) = core.strip_prefix('-') {
        if VALID_BASE_ARROWS.contains(&stripped_core) && (right_marker_removed || core != canonical)
        {
            if has_dot_marker {
                return Some(canonical);
            }
            if has_slash_marker && !expanded_marker {
                let mut out = stripped_core.to_string();
                if let Some(ch) = with_left_trimmed.chars().last() {
                    if matches!(ch, 'o' | 'x') && right_marker_removed {
                        out.push(ch);
                    }
                }
                return Some(out);
            }
            if expanded_marker && canonical.contains("-->>") && squashed.contains("->>") {
                return Some(squashed.replacen("->>", "-->>", 1));
            }
            return Some(squashed);
        }
    }
    None
}

fn strip_sequence_arrow_brackets(arrow: &str) -> String {
    let mut out = String::with_capacity(arrow.len());
    let mut chars = arrow.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '[' {
            for next in chars.by_ref() {
                if next == ']' {
                    break;
                }
            }
            continue;
        }
        out.push(ch);
    }
    out
}

fn split_lifecycle_modifier(endpoint: &str) -> (&str, Option<&'static str>) {
    for suffix in ["++", "--", "**", "!!"] {
        if let Some(base) = endpoint.trim_end().strip_suffix(suffix) {
            return (base.trim_end(), Some(suffix));
        }
    }
    (endpoint, None)
}

fn normalize_virtual_endpoint(raw: &str) -> Option<String> {
    let t = raw.trim().trim_matches('"');
    let lower = t.to_ascii_lowercase();
    match lower.as_str() {
        "[*]" => Some("[*]".to_string()),
        "[" => Some("[".to_string()),
        "]" => Some("]".to_string()),
        "[o" | "o[" => Some("[o".to_string()),
        "o]" | "]o" => Some("o]".to_string()),
        "[x" | "x[" => Some("[x".to_string()),
        "x]" | "]x" => Some("x]".to_string()),
        // Short arrow endpoints: `?` on the from side = left edge short;
        // `?` on the to side = right edge short (feature 1.30 / 1.39.6-8).
        "?" => Some("?".to_string()),
        _ => None,
    }
}

fn looks_like_virtual_endpoint_syntax(raw: &str) -> bool {
    let t = raw.trim().trim_matches('"').to_ascii_lowercase();
    t.contains('[') || t.contains(']') || t == "?"
}

fn looks_like_arrow_syntax(line: &str) -> bool {
    if line.starts_with('!') || line.starts_with('@') {
        return false;
    }
    line.contains("->")
        || line.contains("-->")
        || line.contains("..>")
        || line.contains("<..")
        || line.contains("<-")
        || line.contains("<--")
        || line.contains("<->")
        || line.contains("<-->")
        || line.contains("->>")
        || line.contains("-->>")
        || line.contains("-x")
        || line.contains("x-")
        || line.contains("-o")
        || line.contains("o-")
}
