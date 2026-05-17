fn parse_timing_decl(line: &str) -> Option<StatementKind> {
    let trimmed = line.trim();
    let kinds: &[(&str, TimingDeclKind)] = &[
        ("concise", TimingDeclKind::Concise),
        ("robust", TimingDeclKind::Robust),
        ("clock", TimingDeclKind::Clock),
        ("binary", TimingDeclKind::Binary),
    ];
    for (kw, kind) in kinds.iter().copied() {
        if let Some(rest) = trimmed.strip_prefix(kw) {
            if !rest.starts_with(char::is_whitespace) {
                continue;
            }
            let rest = rest.trim();
            if rest.is_empty() {
                return None;
            }
            let (label, name_raw) = if rest.starts_with('"') {
                let stripped = rest.strip_prefix('"')?;
                let end = stripped.find('"')?;
                let rem = stripped[end + 1..].trim();
                let name = rem.strip_prefix("as ").map(str::trim).unwrap_or(rem).trim();
                (Some(stripped[..end].to_string()), name)
            } else if let Some((lhs, rhs)) = rest.split_once(" as ") {
                (Some(lhs.trim().to_string()), rhs.trim())
            } else {
                (None, rest)
            };
            let (name_raw, controls) = split_timing_decl_controls(name_raw);
            let name = clean_ident(&name_raw);
            if name.is_empty() {
                return None;
            }
            return Some(StatementKind::TimingDecl {
                kind,
                name,
                label,
                controls,
            });
        }
    }
    None
}

fn split_timing_decl_controls(input: &str) -> (String, Vec<String>) {
    let trimmed = input.trim();
    let lower = trimmed.to_ascii_lowercase();
    if let Some(idx) = lower.find(" with ") {
        let name = trimmed[..idx].trim().to_string();
        let controls = trimmed[idx + " with ".len()..]
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        return (name, controls);
    }
    (trimmed.to_string(), Vec::new())
}

fn parse_timing_event(line: &str) -> Option<StatementKind> {
    let trimmed = line.trim();
    if let Some((start, end, label)) = parse_timing_highlight(trimmed) {
        return Some(StatementKind::TimingEvent {
            time: start,
            signal: None,
            state: None,
            note: Some(format!("range:{end}:{label}")),
        });
    }
    // `@<time>` standalone, or `<signal> is <state>` or `@<time> <signal> is <state>`
    if let Some(rest) = trimmed.strip_prefix('@') {
        let rest = rest.trim();
        if rest.is_empty() {
            return Some(StatementKind::TimingEvent {
                time: String::new(),
                signal: None,
                state: None,
                note: None,
            });
        }
        // split at first whitespace
        let (time, after) = rest
            .split_once(char::is_whitespace)
            .map(|(a, b)| (a.trim().to_string(), b.trim()))
            .unwrap_or_else(|| (rest.to_string(), ""));
        if after.is_empty() {
            return Some(StatementKind::TimingEvent {
                time,
                signal: None,
                state: None,
                note: None,
            });
        }
        if let Some((end, label)) = parse_timing_range_after_time(after) {
            return Some(StatementKind::TimingEvent {
                time,
                signal: None,
                state: None,
                note: Some(format!("range:{end}:{label}")),
            });
        }
        // after may contain "signal is state"
        if let Some((sig, state)) = split_is(after) {
            return Some(StatementKind::TimingEvent {
                time,
                signal: Some(sig),
                state: Some(normalize_timing_state_literal(&state)),
                note: None,
            });
        }
        return Some(StatementKind::TimingEvent {
            time,
            signal: None,
            state: None,
            note: Some(after.to_string()),
        });
    }
    if let Some((time, state)) = parse_timing_oriented_state(trimmed) {
        return Some(StatementKind::TimingEvent {
            time,
            signal: None,
            state: Some(normalize_timing_state_literal(&state)),
            note: None,
        });
    }
    if let Some((sig, state)) = split_is(trimmed) {
        return Some(StatementKind::TimingEvent {
            time: String::new(),
            signal: Some(sig),
            state: Some(normalize_timing_state_literal(&state)),
            note: None,
        });
    }
    None
}

fn parse_timing_oriented_state(line: &str) -> Option<(String, String)> {
    let (time, state) = split_is(line)?;
    if time.trim().is_empty()
        || !time
            .trim()
            .chars()
            .next()
            .is_some_and(|c| c == '+' || c == '-' || c.is_ascii_digit() || c == ':')
    {
        return None;
    }
    Some((time.trim().to_string(), state.trim().to_string()))
}

fn normalize_timing_state_literal(state: &str) -> String {
    let trimmed = state.trim().trim_matches('"').trim();
    let body = trimmed
        .strip_prefix('{')
        .and_then(|v| v.strip_suffix('}'))
        .unwrap_or(trimmed)
        .trim();
    match body.to_ascii_lowercase().as_str() {
        "up" | "hi" | "high" | "on" | "true" => "high".to_string(),
        "down" | "lo" | "low" | "off" | "false" => "low".to_string(),
        _ => body.to_string(),
    }
}

fn parse_timing_range_after_time(after: &str) -> Option<(String, String)> {
    let rest = after.trim().strip_prefix("<->")?.trim();
    let rest = rest.strip_prefix('@').unwrap_or(rest).trim();
    let (end, label) = rest
        .split_once(':')
        .map(|(e, l)| (e.trim(), l.trim()))
        .unwrap_or((rest, ""));
    if end.is_empty() {
        return None;
    }
    Some((end.to_string(), label.trim_matches('"').to_string()))
}

fn parse_timing_highlight(line: &str) -> Option<(String, String, String)> {
    let rest = line.strip_prefix("highlight ")?.trim();
    let lower = rest.to_ascii_lowercase();
    let idx = lower.find(" to ")?;
    let start = rest[..idx].trim().trim_start_matches('@');
    let after = rest[idx + " to ".len()..].trim();
    let (end_part, label) = after
        .split_once(':')
        .map(|(e, l)| (e.trim(), l.trim()))
        .unwrap_or((after, ""));
    let end = end_part
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_start_matches('@');
    if start.is_empty() || end.is_empty() {
        return None;
    }
    let label = if label.is_empty() {
        "highlight".to_string()
    } else {
        label.trim_matches('"').to_string()
    };
    Some((start.to_string(), end.to_string(), label))
}

fn split_is(s: &str) -> Option<(String, String)> {
    let needle = " is ";
    let idx = s.find(needle)?;
    let lhs = s[..idx].trim();
    let rhs = s[idx + needle.len()..]
        .trim()
        .trim_matches('"')
        .trim_matches('{')
        .trim_matches('}');
    if lhs.is_empty() || rhs.is_empty() {
        return None;
    }
    Some((lhs.to_string(), rhs.to_string()))
}
