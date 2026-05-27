use super::*;
pub(crate) fn parse_gantt_closed_weekday(line: &str) -> Option<String> {
    parse_gantt_weekday_status(line, "closed")
}

pub(crate) fn parse_gantt_open_weekday(line: &str) -> Option<String> {
    parse_gantt_weekday_status(line, "open")
        .or_else(|| parse_gantt_weekday_status(line, "opened"))
        .or_else(|| parse_gantt_weekday_status(line, "reopened"))
}

pub(crate) fn parse_gantt_weekday_status(line: &str, status: &str) -> Option<String> {
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
pub(crate) fn parse_gantt_closed_date_range(line: &str) -> Option<(String, String)> {
    parse_gantt_date_range_status(line, &[" is closed", " are closed"])
}

pub(crate) fn parse_gantt_open_date_range(line: &str) -> Option<(String, String)> {
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

pub(crate) fn parse_gantt_date_range_status(
    line: &str,
    suffixes: &[&str],
) -> Option<(String, String)> {
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

pub(crate) fn parse_gantt_scale_directive(line: &str) -> Option<(String, Vec<String>)> {
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
    let options = (!trailing.is_empty())
        .then_some(trailing)
        .into_iter()
        .collect();
    Some((normalized.to_string(), options))
}

pub(crate) fn parse_gantt_print_between(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    let rest = lower
        .strip_prefix("print between ")
        .and_then(|_| trimmed.get("print between ".len()..))?
        .trim();
    let lower_rest = rest.to_ascii_lowercase();
    let sep = " and ";
    let idx = lower_rest.find(sep)?;
    let start = rest[..idx].trim();
    let end = rest[idx + sep.len()..].trim();
    Some((
        parse_gantt_date_literal(start)?,
        parse_gantt_date_literal(end)?,
    ))
}

pub(crate) fn parse_gantt_vertical_separator(line: &str) -> Option<(String, String)> {
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

pub(crate) fn parse_gantt_horizontal_separator(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let inner = trimmed.strip_prefix("--")?.strip_suffix("--")?.trim();
    Some(if inner.is_empty() {
        "Separator".to_string()
    } else {
        inner.to_string()
    })
}
pub(crate) fn parse_gantt_day_color(line: &str) -> Option<(String, String, String)> {
    let (range, color) =
        split_gantt_date_range_suffix(line, &[" is colored in ", " are colored in "])?;
    let (start_date, end_date) = parse_gantt_date_range(range)?;
    Some((start_date, end_date, color.to_string()))
}

pub(crate) fn parse_gantt_day_name(line: &str) -> Option<(String, String, String)> {
    let (range, label) = split_gantt_date_range_suffix(line, &[" is named ", " are named "])?;
    let (start_date, end_date) = parse_gantt_date_range(range)?;
    Some((
        start_date,
        end_date,
        label
            .trim()
            .strip_prefix('[')
            .and_then(|value| value.strip_suffix(']'))
            .unwrap_or(label.trim())
            .to_string(),
    ))
}

pub(crate) fn parse_gantt_resource_off_range(line: &str) -> Option<(String, String, String)> {
    let (resource, rest) = line.trim().strip_prefix('{')?.split_once('}')?;
    let rest = rest.trim();
    let range = rest
        .strip_prefix("is off on ")
        .or_else(|| rest.strip_prefix("are off on "))?
        .trim();
    let (start_date, end_date) = parse_gantt_date_range(range)?;
    Some((resource.trim().to_string(), start_date, end_date))
}

pub(crate) fn split_gantt_date_range_suffix<'a>(
    line: &'a str,
    markers: &[&str],
) -> Option<(&'a str, &'a str)> {
    let lower = line.to_ascii_lowercase();
    for marker in markers {
        if let Some(idx) = lower.find(marker) {
            return Some((line[..idx].trim(), line[idx + marker.len()..].trim()));
        }
    }
    None
}

pub(crate) fn parse_gantt_date_range(range: &str) -> Option<(String, String)> {
    let lower = range.to_ascii_lowercase();
    let (start, end) = if let Some(idx) = lower.find(" to ") {
        (&range[..idx], &range[idx + " to ".len()..])
    } else {
        (range, range)
    };
    Some((
        parse_gantt_date_literal(start)?,
        parse_gantt_date_literal(end)?,
    ))
}
pub(crate) fn parse_gantt_date_literal(raw: &str) -> Option<String> {
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

pub(crate) fn parse_gantt_relative_day(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    let rest = trimmed
        .strip_prefix("D+")
        .or_else(|| trimmed.strip_prefix("d+"))?;
    rest.trim()
        .parse::<u32>()
        .ok()
        .map(|value| format!("D+{value}"))
}

pub(crate) fn parse_gantt_verbal_date(raw: &str) -> Option<String> {
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

pub(crate) fn parse_ordinal_day(raw: &str) -> Option<u32> {
    let digits = raw
        .trim()
        .trim_end_matches("st")
        .trim_end_matches("nd")
        .trim_end_matches("rd")
        .trim_end_matches("th");
    let day = digits.parse::<u32>().ok()?;
    (1..=31).contains(&day).then_some(day)
}

pub(crate) fn parse_month_name(raw: &str) -> Option<u32> {
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

pub(crate) fn parse_gantt_named_date(line: &str) -> Option<StatementKind> {
    let lower = line.to_ascii_lowercase();
    let is_named_idx = lower.find(" is named [")?;
    let date = line[..is_named_idx].trim();
    if !is_iso_date_literal(date) {
        return None;
    }
    let after_bracket = is_named_idx + " is named [".len();
    let close = line[after_bracket..].find(']')?;
    let label = line[after_bracket..after_bracket + close]
        .trim()
        .to_string();
    if label.is_empty() {
        return None;
    }
    Some(StatementKind::GanttNamedDate {
        date: date.to_string(),
        label,
    })
}

pub(crate) fn is_iso_date_literal(raw: &str) -> bool {
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
