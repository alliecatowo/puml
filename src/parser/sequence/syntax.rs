use super::*;

pub(super) fn split_family_relation_label(line: &str) -> (&str, Option<String>) {
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
        if !in_quote && ch == ':' {
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

pub(super) fn split_message_label(line: &str) -> (&str, Option<String>) {
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

pub(super) fn split_arrow(core: &str) -> Option<(&str, &str, &str)> {
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

pub(super) fn parse_arrow(arrow: &str) -> Option<String> {
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

pub(super) fn strip_sequence_arrow_brackets(arrow: &str) -> String {
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

pub(super) fn split_lifecycle_modifier(endpoint: &str) -> (&str, Option<&'static str>) {
    for suffix in ["++", "--", "**", "!!"] {
        if let Some(base) = endpoint.trim_end().strip_suffix(suffix) {
            return (base.trim_end(), Some(suffix));
        }
    }
    (endpoint, None)
}

pub(super) fn normalize_virtual_endpoint(raw: &str) -> Option<String> {
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
        _ => None,
    }
}

pub(super) fn looks_like_virtual_endpoint_syntax(raw: &str) -> bool {
    let t = raw.trim().trim_matches('"').to_ascii_lowercase();
    t.contains('[') || t.contains(']')
}

pub(super) fn looks_like_arrow_syntax(line: &str) -> bool {
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

pub(super) fn is_sequence_keyword(kind: &StatementKind) -> bool {
    matches!(
        kind,
        StatementKind::Group(_)
            | StatementKind::Footbox(_)
            | StatementKind::Delay(_)
            | StatementKind::Divider(_)
            | StatementKind::Separator(_)
            | StatementKind::Spacer(_)
            | StatementKind::NewPage(_)
            | StatementKind::IgnoreNewPage
            | StatementKind::Autonumber(_)
            | StatementKind::Activate(_)
            | StatementKind::Deactivate(_)
            | StatementKind::Destroy(_)
            | StatementKind::Create(_)
            | StatementKind::Return(_)
    )
}

pub(super) fn note_block_continues(lines: &[(&str, Span)], idx: usize, line: &str) -> bool {
    let lower = line.trim().to_ascii_lowercase();
    if !(lower.starts_with("note ") || lower.starts_with("hnote ") || lower.starts_with("rnote ")) {
        return false;
    }
    for (candidate, _) in lines.iter().skip(idx + 1) {
        let trimmed = candidate.trim();
        if trimmed.eq_ignore_ascii_case("end note")
            || trimmed.eq_ignore_ascii_case("endnote")
            || trimmed.eq_ignore_ascii_case("endhnote")
            || trimmed.eq_ignore_ascii_case("endrnote")
        {
            return true;
        }
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('@') {
            return false;
        }
    }
    !line.contains(':')
}

pub(super) fn text_block_continues(lines: &[(&str, Span)], idx: usize, line: &str) -> bool {
    let lower = line.trim().to_ascii_lowercase();
    let keyword = ["title", "header", "footer", "caption", "legend"]
        .into_iter()
        .find(|keyword| lower.starts_with(&format!("{keyword} ")));
    let Some(keyword) = keyword else {
        return false;
    };
    let end_marker = format!("end {keyword}");
    for (candidate, _) in lines.iter().skip(idx + 1) {
        let trimmed = candidate.trim();
        if trimmed.eq_ignore_ascii_case(&end_marker) {
            return true;
        }
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('@') {
            return false;
        }
    }
    false
}

pub(super) fn is_family_common_keyword(kind: &StatementKind) -> bool {
    matches!(
        kind,
        StatementKind::Note(_)
            | StatementKind::Title(_)
            | StatementKind::Caption(_)
            | StatementKind::Header(_)
            | StatementKind::Footer(_)
            | StatementKind::Legend(_)
            | StatementKind::LegendPos(_)
            | StatementKind::SkinParam { .. }
            | StatementKind::Theme(_)
            | StatementKind::Scale(_)
            | StatementKind::SetOption { .. }
            | StatementKind::HideOption(_)
            | StatementKind::Pragma(_)
    )
}
