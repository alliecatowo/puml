use super::*;

pub(super) fn extract_bracketed_name(target: &str) -> Option<String> {
    let start = target.find('[')?;
    let end = target.rfind(']')?;
    if end <= start + 1 {
        return None;
    }
    Some(target[start + 1..end].trim().to_string())
}

pub(super) fn resource_lane_label(task: &TimelineTask) -> String {
    if task.resources.is_empty() {
        "Unassigned".to_string()
    } else {
        task.resources.join(", ")
    }
}

pub(super) fn should_expand_gantt_task_visual_span(task: &TimelineTask) -> bool {
    task.workload_days == 14
        && task.duration_days == 14
        && task.baseline_start_day.is_none()
        && task.baseline_duration_days.is_none()
}

pub(super) fn format_resource_load_metadata(task: &TimelineTask) -> String {
    if task.resource_allocations.is_empty() {
        return String::new();
    }
    task.resource_allocations
        .iter()
        .map(|allocation| match allocation.load_percent {
            Some(load) => format!("{}:{load}%", allocation.name),
            None => allocation.name.clone(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn title_case_ascii(raw: &str) -> String {
    let mut chars = raw.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut out = String::new();
    out.push(first.to_ascii_uppercase());
    out.push_str(chars.as_str());
    out
}

pub(super) fn parse_relative_day(raw: &str) -> Option<u32> {
    let t = raw.trim();
    let rest = t.strip_prefix("D+").or_else(|| t.strip_prefix("d+"))?;
    rest.trim().parse::<u32>().ok()
}

pub(super) fn resolve_gantt_milestone_day(
    target: &str,
    anchor_day: u32,
    task_bounds: &std::collections::BTreeMap<&str, (u32, u32)>,
) -> Option<u32> {
    if let Some((task_name, endpoint)) = parse_gantt_render_reference(target) {
        if let Some((start, end)) = task_bounds.get(task_name.as_str()) {
            return Some(if endpoint == "start" { *start } else { *end });
        }
    }
    if let Some(day) = parse_relative_day(target) {
        return Some(anchor_day.saturating_add(day));
    }
    parse_iso_date_day_number(target)
}

pub(super) fn parse_gantt_render_reference(target: &str) -> Option<(String, &'static str)> {
    let name = extract_bracketed_name(target)?;
    let lower = target.to_ascii_lowercase();
    let endpoint = if lower.contains("'s start") || lower.contains(" start") {
        "start"
    } else {
        "end"
    };
    Some((name, endpoint))
}

pub(super) fn timeline_entity_x(
    task: Option<&TimelineTask>,
    milestone: Option<&TimelineMilestone>,
    milestone_day: &std::collections::BTreeMap<&str, u32>,
    endpoint: &str,
    bar_geom: &impl Fn(&TimelineTask) -> (i32, i32),
    day_to_x: &impl Fn(u32) -> i32,
    fallback: i32,
) -> i32 {
    if let Some(task) = task {
        let (x, w) = bar_geom(task);
        return if endpoint == "start" { x } else { x + w };
    }
    if let Some(milestone) = milestone {
        if let Some(day) = milestone_day.get(milestone.name.as_str()) {
            return day_to_x(*day);
        }
    }
    fallback
}

/// Compute tick offsets for the gantt date-header axis.
///
/// `chart_w` is the pixel width of the chart area. Each date label needs
/// roughly 72 px (10 chars × ~7 px each, plus a few px gap). We pick the
/// smallest stride from `[1, 2, 3, 7, 14, 30, 90, 180, 365]` that keeps
/// the number of ticks ≤ `chart_w / label_px`, then fall back to the
/// explicit `scale` override.
///
/// The final tick (last day) is only appended when it is at least
/// `LABEL_PX` pixels away from the preceding tick; otherwise the two labels
/// smear together into an unreadable blob (#426).
pub(super) fn gantt_tick_offsets_for_width(
    total_days: u32,
    scale: Option<&str>,
    chart_w: i32,
) -> Vec<u32> {
    // Approximate pixel width of a single date label (YYYY-MM-DD + small gap)
    const LABEL_PX: i32 = 72;

    let step = match scale {
        Some("weekly") => 7,
        Some("monthly") => 30,
        Some("quarterly") => 90,
        Some("yearly") => 365,
        // No explicit scale: auto-select the smallest stride that avoids overlap
        _ => {
            let max_ticks = (chart_w / LABEL_PX).max(2) as u32;
            let candidates = [1u32, 2, 3, 7, 14, 30, 90, 180, 365];
            candidates
                .into_iter()
                .find(|&s| total_days.div_ceil(s) <= max_ticks)
                .unwrap_or(365)
        }
    };
    let mut offsets = Vec::new();
    let mut offset = 0u32;
    while offset < total_days {
        offsets.push(offset);
        offset = offset.saturating_add(step);
    }
    let last_day_offset = total_days.saturating_sub(1);
    if offsets.last().copied() != Some(last_day_offset) {
        // Only append the last-day tick when it won't collide with the
        // preceding tick.  Collision threshold = one full label width in
        // pixels, converted back to days (#426).
        let min_gap_days = (LABEL_PX as u32)
            .saturating_mul(total_days)
            .div_ceil(chart_w.max(1) as u32);
        let prev = offsets.last().copied().unwrap_or(0);
        if last_day_offset.saturating_sub(prev) >= min_gap_days {
            offsets.push(last_day_offset);
        }
    }
    offsets
}

pub(super) fn format_gantt_axis_label(day: u32, min_day: u32, date_axis: bool) -> String {
    if date_axis {
        day_number_to_iso(day).unwrap_or_else(|| format!("D+{}", day.saturating_sub(min_day)))
    } else {
        format!("D+{}", day.saturating_sub(min_day))
    }
}

pub(super) fn format_gantt_scale_axis_label(
    day: u32,
    min_day: u32,
    date_axis: bool,
    scale: Option<&str>,
) -> String {
    if !date_axis {
        return format_gantt_axis_label(day, min_day, false);
    }
    let Some(iso) = day_number_to_iso(day) else {
        return format_gantt_axis_label(day, min_day, true);
    };
    match scale {
        Some("weekly") => format!("Wk {iso}"),
        Some("monthly") => iso
            .get(0..7)
            .map(format_month_label)
            .unwrap_or_else(|| iso.clone()),
        Some("quarterly") => format_quarter_label(&iso).unwrap_or_else(|| iso.clone()),
        Some("yearly") => iso.get(0..4).unwrap_or(&iso).to_string(),
        _ => iso,
    }
}

pub(super) fn format_month_label(year_month: &str) -> String {
    let Some((year, month)) = year_month.split_once('-') else {
        return year_month.to_string();
    };
    let month = match month {
        "01" => "Jan",
        "02" => "Feb",
        "03" => "Mar",
        "04" => "Apr",
        "05" => "May",
        "06" => "Jun",
        "07" => "Jul",
        "08" => "Aug",
        "09" => "Sep",
        "10" => "Oct",
        "11" => "Nov",
        "12" => "Dec",
        _ => return year_month.to_string(),
    };
    format!("{month} {year}")
}

pub(super) fn format_quarter_label(iso: &str) -> Option<String> {
    let year = iso.get(0..4)?;
    let month = iso.get(5..7)?.parse::<u32>().ok()?;
    let quarter = month.saturating_sub(1) / 3 + 1;
    Some(format!("Q{quarter} {year}"))
}

pub(super) fn is_gantt_closed_weekday_number(day: u32, closed_weekdays: &[String]) -> bool {
    let weekday = match (day + 3) % 7 {
        0 => "monday",
        1 => "tuesday",
        2 => "wednesday",
        3 => "thursday",
        4 => "friday",
        5 => "saturday",
        _ => "sunday",
    };
    closed_weekdays.iter().any(|closed| closed == weekday)
}

pub(super) fn parse_iso_date_tuple(raw: &str) -> Option<(i32, i32, i32)> {
    let mut parts = raw.trim().split('-');
    let y = parts.next()?.parse::<i32>().ok()?;
    let m = parts.next()?.parse::<i32>().ok()?;
    let d = parts.next()?.parse::<i32>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((y, m, d))
}

pub(super) fn parse_iso_date_day_number(raw: &str) -> Option<u32> {
    let (y, m, d) = parse_iso_date_tuple(raw)?;
    if y < 0 || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let y = i64::from(y);
    let m = i64::from(m);
    let d = i64::from(d);
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

pub(super) fn day_number_to_iso(day: u32) -> Option<String> {
    let z = i64::from(day) + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    y += if m <= 2 { 1 } else { 0 };
    Some(format!("{y:04}-{m:02}-{d:02}"))
}
