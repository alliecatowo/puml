use super::*;
pub(crate) fn parse_gantt_alias(subject: String, rest: &str) -> (String, Option<String>, &str) {
    let rest = rest.trim();
    let Some(after_as) = rest.strip_prefix("as ") else {
        return (subject, None, rest);
    };
    let Some((alias, remaining)) = parse_bracket_subject(after_as.trim()) else {
        return (subject, None, rest);
    };
    (subject, Some(alias), remaining.trim())
}

pub(crate) fn parse_gantt_then_statement(line: &str) -> Option<StatementKind> {
    let rest = line
        .strip_prefix("then ")
        .or_else(|| line.strip_prefix("Then "))?
        .trim();
    let (name, tail) = parse_bracket_subject(rest)?;
    let (name, alias, tail) = parse_gantt_alias(name, tail);
    let (clauses, resources) = extract_gantt_resources(tail.trim());
    Some(StatementKind::GanttCompound {
        name,
        alias,
        resources,
        clauses: clauses.trim().to_string(),
        after_previous: true,
    })
}

pub(crate) fn parse_gantt_arrow_dependency(line: &str) -> Option<(String, String, Option<String>)> {
    let (from, rest) = parse_bracket_subject(line)?;
    let rest = rest.trim();
    let arrow_idx = rest.find("->").or_else(|| rest.find("-->"))?;
    let after_arrow = rest[arrow_idx..]
        .trim_start_matches('-')
        .trim_start_matches('>')
        .trim();
    let (to, tail) = parse_bracket_subject(after_arrow)?;
    let style = tail
        .trim()
        .strip_prefix("with ")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    Some((from, to, style))
}

pub(crate) fn is_gantt_compound_clause(rest: &str) -> bool {
    let lower = rest.to_ascii_lowercase();
    lower.contains(" and ")
        || lower.starts_with("is colored in ")
        || lower.starts_with("links to ")
        || lower.contains("% complete")
        || lower.contains("% completed")
        || lower == "is deleted"
}
pub(crate) fn parse_gantt_start_and_duration(rest: &str) -> Option<(String, u32)> {
    let lower = rest.to_ascii_lowercase();
    let (idx, marker_len) = lower
        .find(" and lasts ")
        .map(|idx| (idx, " and lasts ".len()))
        .or_else(|| {
            lower
                .find(" and requires ")
                .map(|idx| (idx, " and requires ".len()))
        })?;
    let start_clause = rest[..idx].trim();
    let duration_clause = rest[idx + marker_len..].trim();
    let start_date = parse_gantt_start_date_clause(start_clause)?;
    Some((start_date, parse_gantt_duration_clause(duration_clause)?))
}

pub(crate) fn parse_gantt_start_date_clause(rest: &str) -> Option<String> {
    // Strip the mandatory "starts " prefix (returns None if absent).
    let after_starts = rest.trim().strip_prefix("starts ")?.trim();
    // Accept both "starts at <date>" and "starts <date>" forms.
    let start_date = after_starts
        .strip_prefix("at ")
        .unwrap_or(after_starts)
        .trim();
    parse_gantt_date_literal(start_date).or_else(|| parse_gantt_relative_day(start_date))
}

pub(crate) fn parse_gantt_duration_clause(rest: &str) -> Option<u32> {
    let trimmed = rest.trim();
    let clause = trimmed
        .strip_prefix("lasts ")
        .or_else(|| trimmed.strip_prefix("requires "))
        .map(str::trim)
        .unwrap_or(trimmed);
    let mut total = 0u32;
    let mut parts = clause.split_whitespace().peekable();
    while parts.peek().is_some() {
        if parts.peek().copied() == Some("and") {
            parts.next();
            continue;
        }
        let n = parts.next()?.parse::<u32>().ok()?;
        let unit = parts.next()?.to_ascii_lowercase();
        let days = match unit.as_str() {
            "day" | "days" => n,
            "week" | "weeks" => n.saturating_mul(7),
            "month" | "months" => n.saturating_mul(30),
            _ => return None,
        };
        total = total.saturating_add(days);
    }
    if total == 0 {
        None
    } else {
        Some(total)
    }
}

pub(crate) fn parse_gantt_task_color(rest: &str) -> Option<String> {
    let lower = rest.to_ascii_lowercase();
    lower
        .strip_prefix("is colored in ")
        .and_then(|_| rest.get("is colored in ".len()..))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub(crate) fn parse_gantt_completion(rest: &str) -> Option<u32> {
    let lower = rest.to_ascii_lowercase();
    let percent_idx = lower.find('%')?;
    let value = lower[..percent_idx].split_whitespace().last()?;
    let suffix = lower[percent_idx + 1..].trim();
    if suffix == "complete" || suffix == "completed" {
        return value.parse::<u32>().ok().map(|value| value.min(100));
    }
    None
}

pub(crate) fn parse_gantt_pause_target(rest: &str) -> Option<String> {
    let trimmed = rest.trim();
    let lower = trimmed.to_ascii_lowercase();
    let target = lower
        .strip_prefix("pauses on ")
        .and_then(|_| trimmed.get("pauses on ".len()..))
        .or_else(|| {
            lower
                .strip_prefix("pause on ")
                .and_then(|_| trimmed.get("pause on ".len()..))
        })?
        .trim();
    if target.is_empty() {
        return None;
    }
    if let Some((start_date, end_date)) = parse_gantt_date_range(target) {
        return Some(if start_date == end_date {
            start_date
        } else {
            format!("{start_date} to {end_date}")
        });
    }
    parse_gantt_weekday_name(target).map(str::to_string)
}

pub(crate) fn parse_gantt_weekday_name(raw: &str) -> Option<&'static str> {
    let lower = raw.trim().to_ascii_lowercase();
    match lower.trim_end_matches('s') {
        "monday" => Some("monday"),
        "tuesday" => Some("tuesday"),
        "wednesday" => Some("wednesday"),
        "thursday" => Some("thursday"),
        "friday" => Some("friday"),
        "saturday" => Some("saturday"),
        "sunday" => Some("sunday"),
        _ => None,
    }
}

pub(crate) fn parse_gantt_link_target(rest: &str) -> Option<String> {
    let trimmed = rest.trim();
    let lower = trimmed.to_ascii_lowercase();
    let target = lower
        .strip_prefix("links to ")
        .and_then(|_| trimmed.get("links to ".len()..))?
        .trim();
    let inner = target.strip_prefix("[[")?.strip_suffix("]]")?.trim();
    let url = inner.split_whitespace().next().unwrap_or(inner).trim();
    (!url.is_empty()).then(|| url.to_string())
}
pub(crate) fn extract_gantt_resources(rest: &str) -> (String, Vec<String>) {
    let lower = rest.to_ascii_lowercase();
    let Some(on_idx) = lower
        .find(" on {")
        .or_else(|| lower.strip_prefix("on {").map(|_| 0))
    else {
        return (rest.to_string(), Vec::new());
    };
    let mut cursor = if on_idx == 0 {
        "on ".len()
    } else {
        on_idx + " on ".len()
    };
    let mut resources = Vec::new();
    while cursor < rest.len() {
        let skipped = rest[cursor..].len() - rest[cursor..].trim_start().len();
        cursor += skipped;
        if !rest[cursor..].starts_with('{') {
            break;
        }
        let Some(end_rel) = rest[cursor + 1..].find('}') else {
            break;
        };
        let end = cursor + 1 + end_rel;
        let resource = rest[cursor + 1..end].trim();
        for resource in resource
            .split(',')
            .map(str::trim)
            .filter(|resource| !resource.is_empty())
        {
            resources.push(resource.to_string());
        }
        cursor = end + 1;
    }
    if resources.is_empty() {
        return (rest.to_string(), Vec::new());
    }
    let prefix = rest[..on_idx].trim_end();
    let suffix = rest[cursor..]
        .trim_start()
        .strip_prefix("and ")
        .unwrap_or_else(|| rest[cursor..].trim_start())
        .trim_start();
    let cleaned = if prefix.is_empty() {
        suffix.to_string()
    } else if suffix.is_empty() {
        prefix.to_string()
    } else {
        format!("{prefix} {suffix}")
    };
    (cleaned, resources)
}

pub(crate) fn parse_gantt_happens_target(rest: &str) -> Option<String> {
    let lower = rest.to_ascii_lowercase();
    let target = lower
        .strip_prefix("happens on ")
        .and_then(|_| rest.get("happens on ".len()..))
        .or_else(|| {
            lower
                .strip_prefix("happens at ")
                .and_then(|_| rest.get("happens at ".len()..))
        })?
        .trim();
    if target.is_empty() {
        None
    } else {
        parse_gantt_date_literal(target).or_else(|| Some(target.to_string()))
    }
}
