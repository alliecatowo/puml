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

pub(super) fn gantt_task_key(task: &TimelineTask) -> String {
    task.alias.clone().unwrap_or_else(|| task.name.clone())
}

pub(super) fn gantt_task_key_ref(task: &TimelineTask) -> &str {
    task.alias.as_deref().unwrap_or(&task.name)
}

pub(super) fn gantt_task_matches(task: &TimelineTask, reference: &str) -> bool {
    task.alias.as_deref() == Some(reference) || task.name == reference
}

pub(super) struct GanttTaskPauseRender {
    pub(super) day: u32,
    pub(super) label: String,
    pub(super) class_name: &'static str,
    pub(super) fill: &'static str,
}

pub(super) fn gantt_task_pause_segments(
    task: &TimelineTask,
    resource_off_ranges: &[TimelineResourceOffRange],
    min_day: u32,
    max_day_exclusive: u32,
) -> Vec<GanttTaskPauseRender> {
    let start = task.start_day.max(min_day);
    let end = task
        .start_day
        .saturating_add(task.duration_days.max(1))
        .min(max_day_exclusive);
    let mut segments = Vec::new();
    let mut day = start;
    while day < end {
        if let Some(label) = gantt_task_pause_label(task, day) {
            segments.push(GanttTaskPauseRender {
                day,
                label,
                class_name: "gantt-task-pause",
                fill: "#f97316",
            });
        } else if let Some(label) = gantt_resource_off_label(task, day, resource_off_ranges) {
            segments.push(GanttTaskPauseRender {
                day,
                label,
                class_name: "gantt-resource-off",
                fill: "#ef4444",
            });
        }
        day = day.saturating_add(1);
    }
    segments
}

pub(super) fn gantt_task_pause_label(task: &TimelineTask, day: u32) -> Option<String> {
    if task
        .pause_weekdays
        .iter()
        .any(|weekday| is_gantt_weekday_number(day, weekday))
    {
        return Some(format!(
            "{} paused",
            format_gantt_axis_label(day, day, true)
        ));
    }
    task.pause_ranges
        .iter()
        .find(|range| (range.start_day..=range.end_day).contains(&day))
        .map(|range| {
            if range.start_date == range.end_date {
                format!("{} paused", range.start_date)
            } else {
                format!("{} to {} paused", range.start_date, range.end_date)
            }
        })
}

pub(super) fn gantt_resource_off_label(
    task: &TimelineTask,
    day: u32,
    resource_off_ranges: &[TimelineResourceOffRange],
) -> Option<String> {
    resource_off_ranges
        .iter()
        .find(|range| {
            (range.start_day..=range.end_day).contains(&day)
                && task
                    .resource_allocations
                    .iter()
                    .any(|allocation| allocation.name == range.resource)
        })
        .map(|range| {
            if range.start_date == range.end_date {
                format!("{} off {}", range.resource, range.start_date)
            } else {
                format!(
                    "{} off {} to {}",
                    range.resource, range.start_date, range.end_date
                )
            }
        })
}

pub(super) fn is_gantt_weekday_number(day: u32, expected: &str) -> bool {
    let weekday = match (day + 3) % 7 {
        0 => "monday",
        1 => "tuesday",
        2 => "wednesday",
        3 => "thursday",
        4 => "friday",
        5 => "saturday",
        _ => "sunday",
    };
    weekday == expected
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
