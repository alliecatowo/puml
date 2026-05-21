fn parse_gantt_baseline_statement(line: &str) -> Option<StatementKind> {
    let trimmed = line.trim();
    if let Some(kind) = parse_keyword(trimmed) {
        if matches!(&kind, StatementKind::Note(note) if note.text.is_empty()) {
            return None;
        }
        return Some(kind);
    }
    let lower_trimmed = trimmed.to_ascii_lowercase();
    if matches!(
        lower_trimmed.as_str(),
        "hide resources names" | "hide resource names" | "hide resources footbox" | "hide resource footbox"
    ) {
        return Some(StatementKind::HideOption(
            lower_trimmed.strip_prefix("hide ").unwrap_or("").to_string(),
        ));
    }

    if let Some((scale, options)) = parse_gantt_scale_directive(trimmed) {
        return Some(StatementKind::GanttConstraint {
            subject: "Project".to_string(),
            kind: "scale".to_string(),
            target: if options.is_empty() {
                scale
            } else {
                format!("{scale};{}", options.join(";"))
            },
        });
    }
    if let Some((resource, start_date, end_date)) = parse_gantt_resource_off_range(trimmed) {
        return Some(StatementKind::GanttConstraint {
            subject: format!("__resource::{resource}"),
            kind: "resource_off".to_string(),
            target: if start_date == end_date {
                start_date
            } else {
                format!("{start_date} to {end_date}")
            },
        });
    }
    if let Some((start_date, end_date, color)) = parse_gantt_day_color(trimmed) {
        return Some(StatementKind::GanttConstraint {
            subject: format!("__day::{start_date}::{end_date}"),
            kind: "day_color".to_string(),
            target: color,
        });
    }
    if let Some((start_date, end_date, label)) = parse_gantt_day_name(trimmed) {
        return Some(StatementKind::GanttConstraint {
            subject: format!("__day::{start_date}::{end_date}"),
            kind: "day_name".to_string(),
            target: label,
        });
    }
    if let Some(rest) = trimmed.strip_prefix("Project starts ") {
        if let Some(date) = parse_gantt_date_literal(rest) {
            return Some(StatementKind::GanttConstraint {
                subject: "Project".to_string(),
                kind: "starts".to_string(),
                target: date,
            });
        }
    }
    if let Some(rest) = trimmed.strip_prefix("Project ends ") {
        if let Some(date) = parse_gantt_date_literal(rest) {
            return Some(StatementKind::GanttConstraint {
                subject: "Project".to_string(),
                kind: "ends".to_string(),
                target: date,
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
    if let Some(kind) = parse_gantt_named_date(trimmed) {
        return Some(kind);
    }
    if let Some(compound) = parse_gantt_then_statement(trimmed) {
        return Some(compound);
    }
    if let Some((from, to, style)) = parse_gantt_arrow_dependency(trimmed) {
        return Some(StatementKind::GanttConstraint {
            subject: to,
            kind: "requires".to_string(),
            target: if let Some(style) = style {
                format!("[{from}] with {style}")
            } else {
                format!("[{from}]")
            },
        });
    }

    let (subject, rest) = parse_bracket_subject(trimmed)?;
    let (subject, alias, rest) = parse_gantt_alias(subject, rest);
    let subject_key = alias.clone().unwrap_or_else(|| subject.clone());
    if rest.is_empty() {
        return Some(StatementKind::GanttTaskDecl {
            name: subject,
            alias,
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
            alias: None,
            start_date: None,
            duration_days: None,
            depends_on: Vec::new(),
            resources,
        });
    }
    if is_gantt_compound_clause(rest) {
        return Some(StatementKind::GanttCompound {
            name: subject,
            alias,
            resources,
            clauses: rest.to_string(),
            after_previous: false,
        });
    }
    if let Some((start_date, duration_days)) = parse_gantt_start_and_duration(rest) {
        return Some(StatementKind::GanttTaskDecl {
            name: subject,
            alias,
            start_date: Some(start_date),
            duration_days: Some(duration_days),
            depends_on: Vec::new(),
            resources,
        });
    }
    if let Some(duration_days) = parse_gantt_duration_clause(rest) {
        return Some(StatementKind::GanttTaskDecl {
            name: subject,
            alias,
            start_date: None,
            duration_days: Some(duration_days),
            depends_on: Vec::new(),
            resources,
        });
    }
    if let Some(start_date) = parse_gantt_start_date_clause(rest) {
        return Some(StatementKind::GanttTaskDecl {
            name: subject,
            alias,
            start_date: Some(start_date),
            duration_days: None,
            depends_on: Vec::new(),
            resources,
        });
    }
    if !resources.is_empty() {
        return Some(StatementKind::GanttTaskDecl {
            name: subject,
            alias,
            start_date: None,
            duration_days: None,
            depends_on: Vec::new(),
            resources,
        });
    }
    let lower = rest.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "is critical" | "critical" | "is on critical path" | "is on the critical path"
    ) {
        return Some(StatementKind::GanttConstraint {
            subject: subject_key,
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
            subject: subject_key,
            kind: "baseline".to_string(),
            target: target.trim().to_string(),
        });
    }
    if lower.starts_with("happens") {
        return Some(StatementKind::GanttMilestoneDecl {
            name: subject_key,
            happens_on: parse_gantt_happens_target(rest),
        });
    }
    if let Some(color) = parse_gantt_task_color(rest) {
        return Some(StatementKind::GanttConstraint {
            subject: subject_key,
            kind: "color".to_string(),
            target: color,
        });
    }
    if let Some(percent) = parse_gantt_completion(rest) {
        return Some(StatementKind::GanttConstraint {
            subject: subject_key,
            kind: "completion".to_string(),
            target: percent.to_string(),
        });
    }
    if lower == "is deleted" || lower == "deleted" {
        return Some(StatementKind::GanttConstraint {
            subject: subject_key,
            kind: "deleted".to_string(),
            target: "true".to_string(),
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
                subject: subject_key,
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

fn parse_gantt_alias(subject: String, rest: &str) -> (String, Option<String>, &str) {
    let rest = rest.trim();
    let Some(after_as) = rest.strip_prefix("as ") else {
        return (subject, None, rest);
    };
    let Some((alias, remaining)) = parse_bracket_subject(after_as.trim()) else {
        return (subject, None, rest);
    };
    (subject, Some(alias), remaining.trim())
}

fn parse_gantt_then_statement(line: &str) -> Option<StatementKind> {
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

fn parse_gantt_arrow_dependency(line: &str) -> Option<(String, String, Option<String>)> {
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

fn is_gantt_compound_clause(rest: &str) -> bool {
    let lower = rest.to_ascii_lowercase();
    lower.contains(" and ")
        || lower.starts_with("is colored in ")
        || lower.contains("% complete")
        || lower.contains("% completed")
        || lower == "is deleted"
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
    Some((
        parse_gantt_date_literal(start_date)?,
        parse_gantt_date_literal(end_date)?,
    ))
}

fn parse_gantt_scale_directive(line: &str) -> Option<(String, Vec<String>)> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    let value = lower
        .strip_prefix("printscale ")
        .or_else(|| lower.strip_prefix("projectscale "))
        .or_else(|| lower.strip_prefix("ganttscale "))
        .or_else(|| lower.strip_prefix("scale "))?
        .trim();
    let mut parts = value.split_whitespace();
    let unit = parts.next()?;
    let normalized = match unit {
        "daily" | "day" | "days" => "daily",
        "weekly" | "week" | "weeks" => "weekly",
        "monthly" | "month" | "months" => "monthly",
        "quarterly" | "quarter" | "quarters" => "quarterly",
        "yearly" | "year" | "years" => "yearly",
        _ => return None,
    };
    let trailing = parts.collect::<Vec<_>>().join(" ");
    let options = (!trailing.is_empty()).then_some(trailing).into_iter().collect();
    Some((normalized.to_string(), options))
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
    // Strip the mandatory "starts " prefix (returns None if absent).
    let after_starts = rest.trim().strip_prefix("starts ")?.trim();
    // Accept both "starts at <date>" and "starts <date>" forms.
    let start_date = after_starts
        .strip_prefix("at ")
        .unwrap_or(after_starts)
        .trim();
    parse_gantt_date_literal(start_date).or_else(|| parse_gantt_relative_day(start_date))
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

fn parse_gantt_task_color(rest: &str) -> Option<String> {
    let lower = rest.to_ascii_lowercase();
    lower
        .strip_prefix("is colored in ")
        .and_then(|_| rest.get("is colored in ".len()..))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn parse_gantt_completion(rest: &str) -> Option<u32> {
    let lower = rest.to_ascii_lowercase();
    let percent_idx = lower.find('%')?;
    let value = lower[..percent_idx].split_whitespace().last()?;
    let suffix = lower[percent_idx + 1..].trim();
    if suffix == "complete" || suffix == "completed" {
        return value.parse::<u32>().ok().map(|value| value.min(100));
    }
    None
}

fn parse_gantt_day_color(line: &str) -> Option<(String, String, String)> {
    let (range, color) = split_gantt_date_range_suffix(line, &[" is colored in ", " are colored in "])?;
    let (start_date, end_date) = parse_gantt_date_range(range)?;
    Some((start_date, end_date, color.to_string()))
}

fn parse_gantt_day_name(line: &str) -> Option<(String, String, String)> {
    let (range, label) = split_gantt_date_range_suffix(line, &[" is named ", " are named "])?;
    let (start_date, end_date) = parse_gantt_date_range(range)?;
    Some((
        start_date,
        end_date,
        label.trim()
            .strip_prefix('[')
            .and_then(|value| value.strip_suffix(']'))
            .unwrap_or(label.trim())
            .to_string(),
    ))
}

fn parse_gantt_resource_off_range(line: &str) -> Option<(String, String, String)> {
    let (resource, rest) = line.trim().strip_prefix('{')?.split_once('}')?;
    let rest = rest.trim();
    let range = rest
        .strip_prefix("is off on ")
        .or_else(|| rest.strip_prefix("are off on "))?
        .trim();
    let (start_date, end_date) = parse_gantt_date_range(range)?;
    Some((resource.trim().to_string(), start_date, end_date))
}

fn split_gantt_date_range_suffix<'a>(line: &'a str, markers: &[&str]) -> Option<(&'a str, &'a str)> {
    let lower = line.to_ascii_lowercase();
    for marker in markers {
        if let Some(idx) = lower.find(marker) {
            return Some((line[..idx].trim(), line[idx + marker.len()..].trim()));
        }
    }
    None
}

fn parse_gantt_date_range(range: &str) -> Option<(String, String)> {
    let lower = range.to_ascii_lowercase();
    let (start, end) = if let Some(idx) = lower.find(" to ") {
        (&range[..idx], &range[idx + " to ".len()..])
    } else {
        (range, range)
    };
    Some((parse_gantt_date_literal(start)?, parse_gantt_date_literal(end)?))
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
        parse_gantt_date_literal(target).or_else(|| Some(target.to_string()))
    }
}

fn parse_gantt_date_literal(raw: &str) -> Option<String> {
    let trimmed = raw
        .trim()
        .strip_prefix("on ")
        .or_else(|| raw.trim().strip_prefix("the "))
        .unwrap_or_else(|| raw.trim())
        .trim();
    if let Some(relative) = parse_gantt_relative_day(trimmed) {
        return Some(relative);
    }
    if is_iso_date_literal(trimmed) {
        return Some(trimmed.replace('/', "-"));
    }
    parse_gantt_verbal_date(trimmed)
}

fn parse_gantt_relative_day(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    let rest = trimmed
        .strip_prefix("D+")
        .or_else(|| trimmed.strip_prefix("d+"))?;
    rest.trim()
        .parse::<u32>()
        .ok()
        .map(|value| format!("D+{value}"))
}

fn parse_gantt_verbal_date(raw: &str) -> Option<String> {
    let cleaned = raw
        .replace(',', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let lower = cleaned.to_ascii_lowercase();
    let lower = lower
        .strip_prefix("the ")
        .unwrap_or(&lower)
        .trim()
        .to_string();
    let parts = lower.split_whitespace().collect::<Vec<_>>();
    if parts.len() == 4 && parts[1] == "of" {
        let day = parse_ordinal_day(parts[0])?;
        let month = parse_month_name(parts[2])?;
        let year = parts[3].parse::<u32>().ok()?;
        return Some(format!("{year:04}-{month:02}-{day:02}"));
    }
    if parts.len() == 3 {
        if let Some(day) = parse_ordinal_day(parts[0]) {
            let month = parse_month_name(parts[1])?;
            let year = parts[2].parse::<u32>().ok()?;
            return Some(format!("{year:04}-{month:02}-{day:02}"));
        }
        if let Some(month) = parse_month_name(parts[0]) {
            let day = parse_ordinal_day(parts[1])?;
            let year = parts[2].parse::<u32>().ok()?;
            return Some(format!("{year:04}-{month:02}-{day:02}"));
        }
    }
    None
}

fn parse_ordinal_day(raw: &str) -> Option<u32> {
    let digits = raw
        .trim()
        .trim_end_matches("st")
        .trim_end_matches("nd")
        .trim_end_matches("rd")
        .trim_end_matches("th");
    let day = digits.parse::<u32>().ok()?;
    (1..=31).contains(&day).then_some(day)
}

fn parse_month_name(raw: &str) -> Option<u32> {
    match raw.to_ascii_lowercase().as_str() {
        "jan" | "january" => Some(1),
        "feb" | "february" => Some(2),
        "mar" | "march" => Some(3),
        "apr" | "april" => Some(4),
        "may" => Some(5),
        "jun" | "june" => Some(6),
        "jul" | "july" => Some(7),
        "aug" | "august" => Some(8),
        "sep" | "sept" | "september" => Some(9),
        "oct" | "october" => Some(10),
        "nov" | "november" => Some(11),
        "dec" | "december" => Some(12),
        _ => None,
    }
}

fn is_iso_date_literal(raw: &str) -> bool {
    let normalized = raw.trim().replace('/', "-");
    let mut parts = normalized.split('-');
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

fn parse_gantt_named_date(line: &str) -> Option<StatementKind> {
    let lower = line.to_ascii_lowercase();
    let is_named_idx = lower.find(" is named [")?;
    let date = line[..is_named_idx].trim();
    if !is_iso_date_literal(date) {
        return None;
    }
    let after_bracket = is_named_idx + " is named [".len();
    let close = line[after_bracket..].find(']')?;
    let label = line[after_bracket..after_bracket + close].trim().to_string();
    if label.is_empty() {
        return None;
    }
    Some(StatementKind::GanttNamedDate {
        date: date.to_string(),
        label,
    })
}
