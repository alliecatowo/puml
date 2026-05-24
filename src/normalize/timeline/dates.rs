use super::*;

pub(super) fn parse_gantt_target_range(target: &str) -> Option<(String, String)> {
    let lower = target.to_ascii_lowercase();
    if let Some(idx) = lower.find(" to ") {
        return Some((
            target[..idx].trim().to_string(),
            target[idx + " to ".len()..].trim().to_string(),
        ));
    }
    Some((target.trim().to_string(), target.trim().to_string()))
}

pub(super) fn infer_gantt_anchor_day(
    project_start_day: Option<u32>,
    tasks: &[TimelineTask],
    milestones: &[TimelineMilestone],
    constraints: &[TimelineConstraint],
) -> Option<u32> {
    let task_starts = tasks
        .iter()
        .filter(|task| task.start_day > 0)
        .map(|task| task.start_day);
    let milestone_dates = milestones
        .iter()
        .filter_map(|milestone| milestone.happens_on.as_deref())
        .filter_map(parse_iso_date_day);
    let constraint_dates = constraints
        .iter()
        .flat_map(|constraint| gantt_constraint_absolute_days(&constraint.target));

    project_start_day
        .into_iter()
        .chain(task_starts)
        .chain(milestone_dates)
        .chain(constraint_dates)
        .min()
}

pub(super) fn gantt_constraint_absolute_days(target: &str) -> Vec<u32> {
    let mut days = Vec::new();
    if let Some(day) = parse_iso_date_day(target) {
        days.push(day);
    }
    if let Some((start_day, _)) = parse_gantt_baseline_target(target) {
        days.push(start_day);
    }
    days
}

pub(super) fn parse_iso_date_day(raw: &str) -> Option<u32> {
    let normalized = raw.trim().replace('/', "-");
    let mut parts = normalized.split('-');
    let y = parts.next()?.parse::<i64>().ok()?;
    let m = parts.next()?.parse::<i64>().ok()?;
    let d = parts.next()?.parse::<i64>().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) || y < 0 {
        return None;
    }
    let y_adj = y - if m <= 2 { 1 } else { 0 };
    let era = if y_adj >= 0 { y_adj } else { y_adj - 399 } / 400;
    let yoe = y_adj - era * 400;
    let mp = m + if m > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    if days < 0 {
        return None;
    }
    u32::try_from(days).ok()
}

pub(super) fn parse_relative_day(raw: &str) -> Option<u32> {
    let trimmed = raw.trim();
    let rest = trimmed
        .strip_prefix("D+")
        .or_else(|| trimmed.strip_prefix("d+"))?;
    rest.trim().parse::<u32>().ok()
}

pub(super) fn resolve_gantt_absolute_day(target: &str, anchor_day: u32) -> Option<u32> {
    parse_iso_date_day(target)
        .or_else(|| parse_relative_day(target).map(|day| anchor_day.saturating_add(day)))
}

pub(super) fn parse_gantt_baseline_target(target: &str) -> Option<(u32, u32)> {
    let trimmed = target.trim();
    let lower = trimmed.to_ascii_lowercase();
    if let Some((start, end)) = lower.find(" to ").and_then(|idx| {
        let start = trimmed[..idx].trim();
        let end = trimmed[idx + " to ".len()..].trim();
        Some((parse_iso_date_day(start)?, parse_iso_date_day(end)?))
    }) {
        return Some((start, end.saturating_sub(start).saturating_add(1).max(1)));
    }
    let (idx, marker_len) = lower
        .find(" and lasts ")
        .map(|idx| (idx, " and lasts ".len()))
        .or_else(|| {
            lower
                .find(" and requires ")
                .map(|idx| (idx, " and requires ".len()))
        })?;
    let start_clause = trimmed[..idx]
        .trim()
        .strip_prefix("starts ")
        .map(str::trim)
        .unwrap_or(trimmed[..idx].trim());
    let start_day = parse_iso_date_day(start_clause.strip_prefix("at ").unwrap_or(start_clause))?;
    let duration_days = parse_timeline_duration_days(&trimmed[idx + marker_len..])?;
    Some((start_day, duration_days))
}

pub(super) fn parse_timeline_duration_days(raw: &str) -> Option<u32> {
    let clause = raw
        .trim()
        .strip_prefix("lasts ")
        .or_else(|| raw.trim().strip_prefix("requires "))
        .map(str::trim)
        .unwrap_or(raw.trim());
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
            _ => return None,
        };
        total = total.saturating_add(days);
    }
    (total > 0).then_some(total)
}
