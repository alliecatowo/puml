use super::*;

pub(super) fn upsert_gantt_day_marker(
    markers: &mut Vec<TimelineDayMarker>,
    subject: &str,
    kind: &str,
    target: &str,
) {
    let Some((start_date, end_date)) = parse_gantt_day_marker_subject(subject) else {
        return;
    };
    let Some(start_day) = parse_iso_date_day(&start_date) else {
        return;
    };
    let Some(end_day) = parse_iso_date_day(&end_date) else {
        return;
    };
    let (start_date, end_date, start_day, end_day) = if start_day <= end_day {
        (start_date, end_date, start_day, end_day)
    } else {
        (end_date, start_date, end_day, start_day)
    };
    if let Some(existing) = markers
        .iter_mut()
        .find(|marker| marker.start_day == start_day && marker.end_day == end_day)
    {
        if kind.eq_ignore_ascii_case("day_color") {
            existing.color = Some(target.to_string());
        } else {
            existing.label = Some(target.to_string());
        }
        return;
    }
    markers.push(TimelineDayMarker {
        start_date,
        end_date,
        start_day,
        end_day,
        label: kind
            .eq_ignore_ascii_case("day_name")
            .then(|| target.to_string()),
        color: kind
            .eq_ignore_ascii_case("day_color")
            .then(|| target.to_string()),
    });
}

pub(super) fn parse_gantt_day_marker_subject(subject: &str) -> Option<(String, String)> {
    let rest = subject.strip_prefix("__day::")?;
    let mut parts = rest.split("::");
    Some((parts.next()?.to_string(), parts.next()?.to_string()))
}

pub(super) fn parse_gantt_resource_off_constraint(
    subject: &str,
    target: &str,
) -> Option<TimelineResourceOffRange> {
    let resource = subject.strip_prefix("__resource::")?.to_string();
    let (start_date, end_date) = parse_gantt_target_range(target)?;
    let start_day = parse_iso_date_day(&start_date)?;
    let end_day = parse_iso_date_day(&end_date)?;
    let (start_date, end_date, start_day, end_day) = if start_day <= end_day {
        (start_date, end_date, start_day, end_day)
    } else {
        (end_date, start_date, end_day, start_day)
    };
    Some(TimelineResourceOffRange {
        resource,
        start_date,
        end_date,
        start_day,
        end_day,
    })
}

pub(super) fn scheduled_gantt_span_days_for_task(
    task: &TimelineTask,
    start_day: u32,
    work_days: u32,
    closed_weekdays: &[String],
    closed_ranges: &[TimelineClosedRange],
    open_ranges: &[TimelineOpenRange],
    resource_off_ranges: &[TimelineResourceOffRange],
) -> u32 {
    let mut day = start_day;
    let mut remaining = work_days.max(1);
    let mut span = 0u32;
    while remaining > 0 {
        if !is_gantt_non_working_day_for_task(
            task,
            day,
            closed_weekdays,
            closed_ranges,
            open_ranges,
            resource_off_ranges,
        ) {
            remaining -= 1;
        }
        day = day.saturating_add(1);
        span = span.saturating_add(1);
        if span > work_days.saturating_add(120) {
            break;
        }
    }
    span.max(1)
}

pub(super) fn is_gantt_closed_day(
    day: u32,
    closed_weekdays: &[String],
    closed_ranges: &[TimelineClosedRange],
    open_ranges: &[TimelineOpenRange],
) -> bool {
    if open_ranges
        .iter()
        .any(|range| (range.start_day..=range.end_day).contains(&day))
    {
        return false;
    }
    is_gantt_closed_weekday(day, closed_weekdays)
        || closed_ranges
            .iter()
            .any(|range| (range.start_day..=range.end_day).contains(&day))
}

pub(super) fn is_gantt_non_working_day_for_task(
    task: &TimelineTask,
    day: u32,
    closed_weekdays: &[String],
    closed_ranges: &[TimelineClosedRange],
    open_ranges: &[TimelineOpenRange],
    resource_off_ranges: &[TimelineResourceOffRange],
) -> bool {
    is_gantt_closed_day(day, closed_weekdays, closed_ranges, open_ranges)
        || is_gantt_task_paused_day(task, day)
        || is_gantt_resource_off_day(task, day, resource_off_ranges)
}

pub(super) fn is_gantt_task_paused_day(task: &TimelineTask, day: u32) -> bool {
    task.pause_weekdays
        .iter()
        .any(|weekday| is_gantt_weekday(day, weekday))
        || task
            .pause_ranges
            .iter()
            .any(|range| (range.start_day..=range.end_day).contains(&day))
}

pub(super) fn is_gantt_resource_off_day(
    task: &TimelineTask,
    day: u32,
    resource_off_ranges: &[TimelineResourceOffRange],
) -> bool {
    if task.resource_allocations.is_empty() {
        return false;
    }
    resource_off_ranges.iter().any(|range| {
        (range.start_day..=range.end_day).contains(&day)
            && task
                .resource_allocations
                .iter()
                .any(|allocation| allocation.name == range.resource)
    })
}

pub(super) fn parse_gantt_pause_weekday(raw: &str) -> Option<&'static str> {
    match raw.trim().to_ascii_lowercase().trim_end_matches('s') {
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

pub(super) fn parse_gantt_task_pause_range(target: &str) -> Option<TimelineTaskPauseRange> {
    let (start_date, end_date) = parse_gantt_target_range(target)?;
    let start_day = parse_iso_date_day(&start_date)?;
    let end_day = parse_iso_date_day(&end_date)?;
    let (start_date, end_date, start_day, end_day) = if start_day <= end_day {
        (start_date, end_date, start_day, end_day)
    } else {
        (end_date, start_date, end_day, start_day)
    };
    Some(TimelineTaskPauseRange {
        start_date,
        end_date,
        start_day,
        end_day,
    })
}

pub(super) fn parse_timeline_resource_allocations(
    resources: &[String],
) -> Vec<TimelineResourceAllocation> {
    resources
        .iter()
        .map(|resource| {
            let trimmed = resource.trim();
            let (name, load_percent) = if let Some((name, load)) = trimmed.rsplit_once(':') {
                (name.trim(), parse_load_percent(load))
            } else {
                (trimmed, None)
            };
            TimelineResourceAllocation {
                name: if name.is_empty() {
                    trimmed.to_string()
                } else {
                    name.to_string()
                },
                load_percent,
            }
        })
        .collect()
}

pub(super) fn parse_load_percent(raw: &str) -> Option<u32> {
    let value = raw.trim().trim_end_matches('%').trim();
    value.parse::<u32>().ok().map(|n| n.clamp(1, 1000))
}

pub(super) fn resource_adjusted_work_days(
    workload_days: u32,
    allocations: &[TimelineResourceAllocation],
) -> u32 {
    let workload_days = workload_days.max(1);
    if allocations.is_empty() {
        return workload_days;
    }
    let total_load: u32 = allocations
        .iter()
        .map(|allocation| allocation.load_percent.unwrap_or(100).max(1))
        .sum();
    if total_load >= 100 {
        workload_days
    } else {
        workload_days
            .saturating_mul(100)
            .div_ceil(total_load)
            .max(1)
    }
}

pub(super) fn timeline_scheduled_span_for_task(
    task: &TimelineTask,
    closed_weekdays: &[String],
    closed_ranges: &[TimelineClosedRange],
    open_ranges: &[TimelineOpenRange],
    resource_off_ranges: &[TimelineResourceOffRange],
) -> u32 {
    scheduled_gantt_span_days_for_task(
        task,
        task.start_day,
        resource_adjusted_work_days(task.workload_days, &task.resource_allocations),
        closed_weekdays,
        closed_ranges,
        open_ranges,
        resource_off_ranges,
    )
}

pub(super) fn is_gantt_closed_weekday(day: u32, closed_weekdays: &[String]) -> bool {
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

pub(super) fn is_gantt_weekday(day: u32, weekday: &str) -> bool {
    let actual = match (day + 3) % 7 {
        0 => "monday",
        1 => "tuesday",
        2 => "wednesday",
        3 => "thursday",
        4 => "friday",
        5 => "saturday",
        _ => "sunday",
    };
    actual == weekday
}
