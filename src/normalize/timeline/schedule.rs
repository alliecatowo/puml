use super::*;

pub(super) fn apply_gantt_absolute_constraints(
    tasks: &mut [TimelineTask],
    constraints: &[TimelineConstraint],
    anchor_day: u32,
) {
    for constraint in constraints {
        let kind = constraint.kind.to_ascii_lowercase();
        if !matches!(kind.as_str(), "starts" | "ends") {
            continue;
        }
        let Some(day) = resolve_gantt_absolute_day(&constraint.target, anchor_day) else {
            continue;
        };
        let Some(task) = tasks
            .iter_mut()
            .find(|task| gantt_task_matches(task, &constraint.subject))
        else {
            continue;
        };
        task.start_day = if kind == "ends" {
            day.saturating_sub(task.duration_days)
        } else {
            day
        };
    }
}

pub(super) fn apply_gantt_start_end_duration_constraints(
    tasks: &mut [TimelineTask],
    constraints: &[TimelineConstraint],
    anchor_day: u32,
) {
    for task in tasks {
        let mut start_day = None;
        let mut end_day = None;
        for constraint in constraints
            .iter()
            .filter(|constraint| gantt_task_matches(task, &constraint.subject))
        {
            if constraint.kind.eq_ignore_ascii_case("starts") {
                start_day = resolve_gantt_absolute_day(&constraint.target, anchor_day);
            } else if constraint.kind.eq_ignore_ascii_case("ends") {
                end_day = resolve_gantt_absolute_day(&constraint.target, anchor_day);
            }
        }
        let (Some(start_day), Some(end_day)) = (start_day, end_day) else {
            continue;
        };
        if end_day <= start_day {
            continue;
        }
        let span_days = end_day.saturating_sub(start_day).max(1);
        task.start_day = start_day;
        task.workload_days = span_days;
        task.duration_days = span_days;
    }
}

pub(super) fn apply_gantt_task_reference_constraints(
    tasks: &mut [TimelineTask],
    constraints: &[TimelineConstraint],
    closed_weekdays: &[String],
    closed_ranges: &[TimelineClosedRange],
    open_ranges: &[TimelineOpenRange],
    resource_off_ranges: &[TimelineResourceOffRange],
) {
    for _ in 0..tasks.len().max(1) {
        let mut changed = false;
        for constraint in constraints {
            let kind = constraint.kind.to_ascii_lowercase();
            if !matches!(kind.as_str(), "starts" | "ends" | "requires") {
                continue;
            }
            let Some(subject_idx) = tasks
                .iter()
                .position(|task| gantt_task_matches(task, &constraint.subject))
            else {
                continue;
            };
            let Some((target_name, endpoint)) = parse_gantt_task_reference(&constraint.target)
            else {
                continue;
            };
            let Some(target) = tasks
                .iter()
                .find(|task| gantt_task_matches(task, &target_name))
            else {
                continue;
            };
            let mut target_day = match endpoint {
                "start" => target.start_day,
                "end" => target.start_day.saturating_add(target.duration_days),
                _ => continue,
            };
            if let Some(offset) = parse_gantt_reference_day_offset(&constraint.target) {
                target_day = if offset.working_days {
                    add_gantt_working_days(
                        target_day,
                        offset.days,
                        closed_weekdays,
                        closed_ranges,
                        open_ranges,
                    )
                } else if offset.days.is_negative() {
                    target_day.saturating_sub(offset.days.unsigned_abs())
                } else {
                    target_day.saturating_add(offset.days as u32)
                };
            }
            let next_start = if kind == "ends" {
                target_day.saturating_sub(tasks[subject_idx].duration_days)
            } else if kind == "requires" {
                tasks[subject_idx].start_day.max(target_day)
            } else {
                target_day
            };
            if tasks[subject_idx].start_day != next_start {
                tasks[subject_idx].start_day = next_start;
                tasks[subject_idx].duration_days = timeline_scheduled_span_for_task(
                    &tasks[subject_idx],
                    closed_weekdays,
                    closed_ranges,
                    open_ranges,
                    resource_off_ranges,
                );
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
}

struct GanttReferenceOffset {
    days: i32,
    working_days: bool,
}

fn parse_gantt_reference_day_offset(target: &str) -> Option<GanttReferenceOffset> {
    let lower = target.to_ascii_lowercase();
    let (sign, marker, working_days) = if let Some(idx) = lower.find(" working days after ") {
        (1, idx, true)
    } else if let Some(idx) = lower.find(" working day after ") {
        (1, idx, true)
    } else if let Some(idx) = lower.find(" working days before ") {
        (-1, idx, true)
    } else if let Some(idx) = lower.find(" working day before ") {
        (-1, idx, true)
    } else if let Some(idx) = lower.find(" days after ") {
        (1, idx, false)
    } else if let Some(idx) = lower.find(" day after ") {
        (1, idx, false)
    } else if let Some(idx) = lower.find(" days before ") {
        (-1, idx, false)
    } else if let Some(idx) = lower.find(" day before ") {
        (-1, idx, false)
    } else {
        return None;
    };
    target[..marker]
        .split_whitespace()
        .last()
        .and_then(|n| n.parse::<i32>().ok())
        .map(|n| GanttReferenceOffset {
            days: n.saturating_mul(sign),
            working_days,
        })
}

pub(super) fn add_gantt_working_days(
    start_day: u32,
    offset: i32,
    closed_weekdays: &[String],
    closed_ranges: &[TimelineClosedRange],
    open_ranges: &[TimelineOpenRange],
) -> u32 {
    if offset == 0 {
        return start_day;
    }
    let mut day = start_day;
    let mut remaining = offset.unsigned_abs();
    let forward = offset > 0;
    while remaining > 0 {
        day = if forward {
            day.saturating_add(1)
        } else {
            day.saturating_sub(1)
        };
        if !is_gantt_closed_day(day, closed_weekdays, closed_ranges, open_ranges) {
            remaining -= 1;
        }
    }
    day
}

pub(super) fn parse_gantt_task_reference(target: &str) -> Option<(String, &'static str)> {
    let trimmed = target.trim();
    let name = extract_bracketed_name_from_target(trimmed)?;
    let lower = trimmed.to_ascii_lowercase();
    if lower.contains("'s end") || lower.contains(" end") {
        Some((name, "end"))
    } else if lower.contains("'s start") || lower.contains(" start") {
        Some((name, "start"))
    } else {
        Some((name, "end"))
    }
}

pub(super) fn extract_bracketed_name_from_target(target: &str) -> Option<String> {
    let start = target.find('[')?;
    let end = target[start + 1..].find(']')? + start + 1;
    let name = target[start + 1..end].trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

pub(super) fn mark_inferred_gantt_critical_path(
    tasks: &mut [TimelineTask],
    constraints: &[TimelineConstraint],
) {
    let Some(latest_end) = tasks
        .iter()
        .map(|task| task.start_day.saturating_add(task.duration_days))
        .max()
    else {
        return;
    };
    let mut stack: Vec<String> = tasks
        .iter()
        .filter(|task| task.start_day.saturating_add(task.duration_days) == latest_end)
        .map(|task| gantt_task_ref(&task.name, task.alias.as_deref()))
        .collect();
    while let Some(name) = stack.pop() {
        let Some(task) = tasks
            .iter_mut()
            .find(|task| gantt_task_matches(task, &name))
        else {
            continue;
        };
        if task.is_critical {
            continue;
        }
        task.is_critical = true;
        for dependency in constraints.iter().filter(|constraint| {
            constraint.subject == name && constraint.kind.eq_ignore_ascii_case("requires")
        }) {
            if let Some((target, _)) = parse_gantt_task_reference(&dependency.target) {
                stack.push(target);
            }
        }
    }
}
