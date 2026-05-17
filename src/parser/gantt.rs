fn parse_gantt_baseline_statement(line: &str) -> Option<StatementKind> {
    let trimmed = line.trim();
    if let Some(kind) = parse_keyword(trimmed) {
        return Some(kind);
    }

    if let Some(scale) = parse_gantt_scale_directive(trimmed) {
        return Some(StatementKind::GanttConstraint {
            subject: "Project".to_string(),
            kind: "scale".to_string(),
            target: scale,
        });
    }
    if let Some(rest) = trimmed.strip_prefix("Project starts ") {
        let date = rest
            .trim()
            .strip_prefix("on ")
            .or_else(|| rest.trim().strip_prefix("the "))
            .unwrap_or_else(|| rest.trim())
            .trim();
        if is_iso_date_literal(date) {
            return Some(StatementKind::GanttConstraint {
                subject: "Project".to_string(),
                kind: "starts".to_string(),
                target: date.to_string(),
            });
        }
    }
    if let Some(rest) = trimmed.strip_prefix("Project ends ") {
        let date = rest
            .trim()
            .strip_prefix("on ")
            .or_else(|| rest.trim().strip_prefix("the "))
            .unwrap_or_else(|| rest.trim())
            .trim();
        if is_iso_date_literal(date) {
            return Some(StatementKind::GanttConstraint {
                subject: "Project".to_string(),
                kind: "ends".to_string(),
                target: date.to_string(),
            });
        }
    }
    if let Some((start_date, end_date)) = parse_gantt_closed_date_range(trimmed) {
        return Some(StatementKind::GanttCalendarClosedDateRange {
            start_date,
            end_date,
        });
    }
    if let Some((start_date, end_date)) = parse_gantt_open_date_range(trimmed) {
        return Some(StatementKind::GanttCalendarOpenDateRange {
            start_date,
            end_date,
        });
    }
    if let Some(day) = parse_gantt_closed_weekday(trimmed) {
        return Some(StatementKind::GanttCalendarClosed { day });
    }
    if let Some(day) = parse_gantt_open_weekday(trimmed) {
        return Some(StatementKind::GanttCalendarOpen { day });
    }
    if let Some(label) = parse_gantt_horizontal_separator(trimmed) {
        return Some(StatementKind::Separator(Some(label)));
    }
    if let Some((label, target)) = parse_gantt_vertical_separator(trimmed) {
        return Some(StatementKind::GanttConstraint {
            subject: format!("__separator::{label}"),
            kind: "separator".to_string(),
            target,
        });
    }
    let (subject, rest) = parse_bracket_subject(trimmed)?;
    if rest.is_empty() {
        return Some(StatementKind::GanttTaskDecl {
            name: subject,
            start_date: None,
            duration_days: None,
            depends_on: Vec::new(),
            resources: Vec::new(),
        });
    }
    let rest = rest.trim();
    let (rest_without_resources, resources) = extract_gantt_resources(rest);
    let rest = rest_without_resources.trim();
    if let Some(rest) = rest.strip_prefix(':') {
        if subject.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            let label = rest.trim();
            if !label.is_empty() {
                return Some(StatementKind::GanttMilestoneDecl {
                    name: label.to_string(),
                    happens_on: Some(subject),
                });
            }
        }
        return Some(StatementKind::GanttTaskDecl {
            name: rest.trim().to_string(),
            start_date: None,
            duration_days: None,
            depends_on: Vec::new(),
            resources,
        });
    }
    if let Some((start_date, duration_days)) = parse_gantt_start_and_duration(rest) {
        return Some(StatementKind::GanttTaskDecl {
            name: subject,
            start_date: Some(start_date),
            duration_days: Some(duration_days),
            depends_on: Vec::new(),
            resources,
        });
    }
    if let Some(duration_days) = parse_gantt_duration_clause(rest) {
        return Some(StatementKind::GanttTaskDecl {
            name: subject,
            start_date: None,
            duration_days: Some(duration_days),
            depends_on: Vec::new(),
            resources,
        });
    }
    if let Some(start_date) = parse_gantt_start_date_clause(rest) {
        return Some(StatementKind::GanttTaskDecl {
            name: subject,
            start_date: Some(start_date),
            duration_days: None,
            depends_on: Vec::new(),
            resources,
        });
    }
    if !resources.is_empty() {
        return Some(StatementKind::GanttTaskDecl {
            name: subject,
            start_date: None,
            duration_days: None,
            depends_on: Vec::new(),
            resources,
        });
    }
    let rest = rest.trim();
    if let Some(rest) = rest.strip_prefix(':') {
        return Some(StatementKind::GanttTaskDecl {
            name: rest.trim().to_string(),
            start_date: None,
            duration_days: None,
            depends_on: Vec::new(),
            resources: Vec::new(),
        });
    }
    let lower = rest.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "is critical" | "critical" | "is on critical path" | "is on the critical path"
    ) {
        return Some(StatementKind::GanttConstraint {
            subject,
            kind: "critical".to_string(),
            target: "true".to_string(),
        });
    }
    if let Some(target) = lower
        .strip_prefix("baseline ")
        .and_then(|_| rest.get("baseline ".len()..))
        .or_else(|| {
            lower
                .strip_prefix("has baseline ")
                .and_then(|_| rest.get("has baseline ".len()..))
        })
        .or_else(|| {
            lower
                .strip_prefix("planned ")
                .and_then(|_| rest.get("planned ".len()..))
        })
    {
        return Some(StatementKind::GanttConstraint {
            subject,
            kind: "baseline".to_string(),
            target: target.trim().to_string(),
        });
    }
    if lower.starts_with("happens") {
        return Some(StatementKind::GanttMilestoneDecl {
            name: subject,
            happens_on: parse_gantt_happens_target(rest),
        });
    }
    for kind in ["starts", "ends", "requires"] {
        if lower.starts_with(kind) {
            let target = rest[kind.len()..]
                .trim()
                .strip_prefix("at ")
                .unwrap_or_else(|| rest[kind.len()..].trim())
                .trim()
                .to_string();
            return Some(StatementKind::GanttConstraint {
                subject,
                kind: kind.to_string(),
                target,
            });
        }
    }
    None
}

fn parse_gantt_closed_weekday(line: &str) -> Option<String> {
    parse_gantt_weekday_status(line, "closed")
}

fn parse_gantt_open_weekday(line: &str) -> Option<String> {
    parse_gantt_weekday_status(line, "open")
        .or_else(|| parse_gantt_weekday_status(line, "opened"))
        .or_else(|| parse_gantt_weekday_status(line, "reopened"))
}

fn parse_gantt_weekday_status(line: &str, status: &str) -> Option<String> {
    let lower = line.trim().to_ascii_lowercase();
    let day = [
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
        "sunday",
    ]
    .into_iter()
    .find(|day| {
        lower == format!("{day} is {status}")
            || lower == format!("{day} are {status}")
            || lower == format!("{day}s are {status}")
    })?;
    Some(day.to_string())
}

fn parse_gantt_closed_date_range(line: &str) -> Option<(String, String)> {
    parse_gantt_date_range_status(line, &[" is closed", " are closed"])
}

fn parse_gantt_open_date_range(line: &str) -> Option<(String, String)> {
    parse_gantt_date_range_status(
        line,
        &[
            " is open",
            " are open",
            " is opened",
            " are opened",
            " is reopened",
            " are reopened",
        ],
    )
}

fn parse_gantt_date_range_status(line: &str, suffixes: &[&str]) -> Option<(String, String)> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    let suffix_len = suffixes
        .iter()
        .find(|suffix| lower.ends_with(**suffix))
        .map(|suffix| suffix.len())?;
    let range = trimmed[..trimmed.len().saturating_sub(suffix_len)].trim();
    let lower_range = lower[..lower.len().saturating_sub(suffix_len)].trim();
    let sep = " to ";
    let (start_date, end_date) = if let Some(idx) = lower_range.find(sep) {
        (range[..idx].trim(), range[idx + sep.len()..].trim())
    } else {
        (range, range)
    };
    if !is_iso_date_literal(start_date) || !is_iso_date_literal(end_date) {
        return None;
    }
    Some((start_date.to_string(), end_date.to_string()))
}

fn parse_gantt_scale_directive(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    let value = lower
        .strip_prefix("printscale ")
        .or_else(|| lower.strip_prefix("scale "))?
        .trim();
    let normalized = match value {
        "daily" | "day" | "days" => "daily",
        "weekly" | "week" | "weeks" => "weekly",
        "monthly" | "month" | "months" => "monthly",
        "quarterly" | "quarter" | "quarters" => "quarterly",
        "yearly" | "year" | "years" => "yearly",
        _ => return None,
    };
    Some(normalized.to_string())
}

fn parse_gantt_vertical_separator(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    let rest = lower
        .strip_prefix("separator just ")
        .and_then(|_| trimmed.get("Separator just ".len()..))
        .or_else(|| {
            lower
                .strip_prefix("separator ")
                .and_then(|_| trimmed.get("Separator ".len()..))
        })?
        .trim();
    if rest.is_empty() {
        return None;
    }
    Some(("Separator".to_string(), rest.to_string()))
}

fn parse_gantt_horizontal_separator(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let inner = trimmed.strip_prefix("--")?.strip_suffix("--")?.trim();
    Some(if inner.is_empty() {
        "Separator".to_string()
    } else {
        inner.to_string()
    })
}

fn parse_gantt_start_and_duration(rest: &str) -> Option<(String, u32)> {
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

fn parse_gantt_start_date_clause(rest: &str) -> Option<String> {
    let start_date = rest
        .trim()
        .strip_prefix("starts ")?
        .trim()
        .strip_prefix("at ")
        .unwrap_or_else(|| rest.trim().strip_prefix("starts ").unwrap().trim())
        .trim();
    if !is_iso_date_literal(start_date) {
        return None;
    }
    Some(start_date.to_string())
}

fn parse_gantt_duration_clause(rest: &str) -> Option<u32> {
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

fn extract_gantt_resources(rest: &str) -> (String, Vec<String>) {
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

fn parse_gantt_happens_target(rest: &str) -> Option<String> {
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
        Some(target.to_string())
    }
}

fn is_iso_date_literal(raw: &str) -> bool {
    let mut parts = raw.trim().split('-');
    let Some(y) = parts.next() else {
        return false;
    };
    let Some(m) = parts.next() else {
        return false;
    };
    let Some(d) = parts.next() else {
        return false;
    };
    if parts.next().is_some() {
        return false;
    }
    if y.len() != 4 || m.len() != 2 || d.len() != 2 {
        return false;
    }
    y.chars().all(|c| c.is_ascii_digit())
        && m.chars().all(|c| c.is_ascii_digit())
        && d.chars().all(|c| c.is_ascii_digit())
}

