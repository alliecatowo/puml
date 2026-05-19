use super::*;

pub(super) fn normalize_timeline_baseline(
    document: Document,
) -> Result<TimelineDocument, Diagnostic> {
    let mut tasks: Vec<TimelineTask> = Vec::new();
    let mut milestones = Vec::new();
    let mut separators = Vec::new();
    let mut constraints = Vec::new();
    let mut chronology_events = Vec::new();
    let mut closed_weekdays = Vec::new();
    let mut closed_ranges = Vec::new();
    let mut open_ranges = Vec::new();
    let mut scale = None;
    let mut title = None;
    let mut header = None;
    let mut footer = None;
    let mut caption = None;
    let mut legend = None;

    for stmt in document.statements {
        match stmt.kind {
            StatementKind::GanttTaskDecl {
                name,
                start_date,
                duration_days,
                resources,
                ..
            } => {
                let parsed_start_day = start_date.as_deref().and_then(parse_iso_date_day);
                // If a task with this name already exists, update it rather than
                // creating a duplicate.  PlantUML allows splitting the declaration
                // across multiple lines, e.g.:
                //   [Design]
                //   [Design] starts 2026-01-02
                if let Some(existing) = tasks.iter_mut().find(|t| t.name == name) {
                    if let Some(day) = parsed_start_day {
                        existing.start_day = day;
                    }
                    if let Some(d) = duration_days {
                        let workload = d.max(1);
                        let new_allocs = parse_timeline_resource_allocations(&resources);
                        existing.workload_days = workload;
                        existing.duration_days = resource_adjusted_work_days(workload, &new_allocs);
                        if !new_allocs.is_empty() {
                            existing.resource_allocations = new_allocs;
                        }
                    }
                    if !resources.is_empty() {
                        existing.resources = resources;
                    }
                } else {
                    // Default to 14 working days when no explicit duration is given, so task
                    // bars are visually readable on a date-axis gantt (#481).
                    let workload_days = duration_days.unwrap_or(14).max(1);
                    let resource_allocations = parse_timeline_resource_allocations(&resources);
                    let duration_days =
                        resource_adjusted_work_days(workload_days, &resource_allocations);
                    tasks.push(TimelineTask {
                        name,
                        start_day: parsed_start_day.unwrap_or(0),
                        workload_days,
                        duration_days,
                        resources,
                        resource_allocations,
                        baseline_start_day: None,
                        baseline_duration_days: None,
                        is_critical: false,
                    });
                }
            }
            StatementKind::GanttMilestoneDecl { name, happens_on } => {
                if let Some(target) = &happens_on {
                    constraints.push(TimelineConstraint {
                        subject: name.clone(),
                        kind: "happens".to_string(),
                        target: target.clone(),
                    });
                }
                milestones.push(TimelineMilestone {
                    name,
                    happens_on,
                    is_critical: false,
                })
            }
            StatementKind::GanttConstraint {
                subject,
                kind,
                target,
            } => {
                if subject.eq_ignore_ascii_case("Project") && kind.eq_ignore_ascii_case("scale") {
                    scale = Some(target);
                } else if kind.eq_ignore_ascii_case("separator") {
                    separators.push(TimelineSeparator {
                        label: subject
                            .strip_prefix("__separator::")
                            .unwrap_or(&subject)
                            .to_string(),
                        target: (!target.trim().is_empty()).then(|| target.trim().to_string()),
                    });
                } else {
                    constraints.push(TimelineConstraint {
                        subject,
                        kind,
                        target,
                    });
                }
            }
            StatementKind::GanttCalendarClosed { day } => {
                if !closed_weekdays.iter().any(|existing| existing == &day) {
                    closed_weekdays.push(day);
                }
            }
            StatementKind::GanttCalendarOpen { day } => {
                closed_weekdays.retain(|closed| closed != &day);
            }
            StatementKind::GanttCalendarClosedDateRange {
                start_date,
                end_date,
            } => {
                if let (Some(start_day), Some(end_day)) = (
                    parse_iso_date_day(&start_date),
                    parse_iso_date_day(&end_date),
                ) {
                    let (start_date, end_date, start_day, end_day) = if start_day <= end_day {
                        (start_date, end_date, start_day, end_day)
                    } else {
                        (end_date, start_date, end_day, start_day)
                    };
                    if !closed_ranges.iter().any(|existing: &TimelineClosedRange| {
                        existing.start_day == start_day && existing.end_day == end_day
                    }) {
                        closed_ranges.push(TimelineClosedRange {
                            start_date,
                            end_date,
                            start_day,
                            end_day,
                        });
                    }
                }
            }
            StatementKind::GanttCalendarOpenDateRange {
                start_date,
                end_date,
            } => {
                if let (Some(start_day), Some(end_day)) = (
                    parse_iso_date_day(&start_date),
                    parse_iso_date_day(&end_date),
                ) {
                    let (start_date, end_date, start_day, end_day) = if start_day <= end_day {
                        (start_date, end_date, start_day, end_day)
                    } else {
                        (end_date, start_date, end_day, start_day)
                    };
                    if !open_ranges.iter().any(|existing: &TimelineOpenRange| {
                        existing.start_day == start_day && existing.end_day == end_day
                    }) {
                        open_ranges.push(TimelineOpenRange {
                            start_date,
                            end_date,
                            start_day,
                            end_day,
                        });
                    }
                }
            }
            StatementKind::ChronologyHappensOn { subject, when } => {
                chronology_events.push(TimelineChronologyEvent { subject, when })
            }
            StatementKind::Title(v) => title = Some(v),
            StatementKind::Header(v) => header = Some(v),
            StatementKind::Footer(v) => footer = Some(v),
            StatementKind::Caption(v) => caption = Some(v),
            StatementKind::Legend(v) => {
                legend = Some(sequence::strip_legend_pos_prefix(&v));
            }
            StatementKind::Separator(label) => {
                separators.push(TimelineSeparator {
                    label: label.unwrap_or_else(|| "Separator".to_string()),
                    target: None,
                });
            }
            StatementKind::Scale(v) => {
                if document.kind == DiagramKind::Gantt {
                    scale = Some(v);
                }
            }
            StatementKind::SkinParam { .. }
            | StatementKind::Theme(_)
            | StatementKind::Pragma(_)
            | StatementKind::LegendPos(_)
            | StatementKind::SetOption { .. }
            | StatementKind::HideOption(_)
            // `<style>...</style>` blocks: silently accepted for compatibility.
            | StatementKind::StyleBlock { .. } => {}
            StatementKind::Unknown(line) => {
                return Err(Diagnostic::error(line).with_span(stmt.span));
            }
            _ => {
                let family = family::family_kind_name(document.kind);
                return Err(Diagnostic::error(format!(
                    "[E_TIMELINE_BASELINE_UNSUPPORTED] unsupported {family} syntax in baseline slice"
                ))
                .with_span(stmt.span));
            }
        }
    }

    let project_start = constraints
        .iter()
        .find(|c| {
            c.subject.eq_ignore_ascii_case("Project")
                && c.kind.eq_ignore_ascii_case("starts")
                && parse_iso_date_day(&c.target).is_some()
        })
        .map(|c| c.target.clone());
    let project_start_day = project_start.as_deref().and_then(parse_iso_date_day);
    let inferred_gantt_anchor_day =
        infer_gantt_anchor_day(project_start_day, &tasks, &milestones, &constraints);

    if document.kind == DiagramKind::Gantt && !tasks.is_empty() {
        let fallback_anchor = inferred_gantt_anchor_day.unwrap_or(0);
        let mut cursor = fallback_anchor;
        for task in &mut tasks {
            if task.start_day == 0 {
                task.start_day = cursor;
            }
            task.duration_days = timeline_scheduled_span_for_task(
                task,
                &closed_weekdays,
                &closed_ranges,
                &open_ranges,
            );
            let task_end = task.start_day.saturating_add(task.duration_days);
            if task_end > cursor {
                cursor = task_end;
            }
        }
        apply_gantt_task_metadata(&mut tasks, &mut milestones, &constraints);
        apply_gantt_task_reference_constraints(
            &mut tasks,
            &constraints,
            &closed_weekdays,
            &closed_ranges,
            &open_ranges,
        );
        for task in &mut tasks {
            task.duration_days = timeline_scheduled_span_for_task(
                task,
                &closed_weekdays,
                &closed_ranges,
                &open_ranges,
            );
        }
        if constraints.iter().any(|c| {
            c.subject.eq_ignore_ascii_case("Project")
                && (c.kind.eq_ignore_ascii_case("criticalPath")
                    || c.kind.eq_ignore_ascii_case("critical_path"))
        }) {
            mark_inferred_gantt_critical_path(&mut tasks, &constraints);
        }
    }

    Ok(TimelineDocument {
        kind: document.kind,
        tasks,
        milestones,
        separators,
        constraints,
        chronology_events,
        closed_weekdays,
        closed_ranges,
        open_ranges,
        scale,
        project_start,
        project_start_day,
        title,
        header,
        footer,
        caption,
        legend,
        warnings: Vec::new(),
    })
}

fn infer_gantt_anchor_day(
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

fn gantt_constraint_absolute_days(target: &str) -> Vec<u32> {
    let mut days = Vec::new();
    if let Some(day) = parse_iso_date_day(target) {
        days.push(day);
    }
    if let Some((start_day, _)) = parse_gantt_baseline_target(target) {
        days.push(start_day);
    }
    days
}

fn parse_iso_date_day(raw: &str) -> Option<u32> {
    let mut parts = raw.trim().split('-');
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

fn scheduled_gantt_span_days(
    start_day: u32,
    work_days: u32,
    closed_weekdays: &[String],
    closed_ranges: &[TimelineClosedRange],
    open_ranges: &[TimelineOpenRange],
) -> u32 {
    if closed_weekdays.is_empty() && closed_ranges.is_empty() {
        return work_days.max(1);
    }
    let mut day = start_day;
    let mut remaining = work_days.max(1);
    let mut span = 0u32;
    while remaining > 0 {
        if !is_gantt_closed_day(day, closed_weekdays, closed_ranges, open_ranges) {
            remaining -= 1;
        }
        day = day.saturating_add(1);
        span = span.saturating_add(1);
        if span > work_days.saturating_add(21) {
            break;
        }
    }
    span.max(1)
}

fn apply_gantt_task_reference_constraints(
    tasks: &mut [TimelineTask],
    constraints: &[TimelineConstraint],
    closed_weekdays: &[String],
    closed_ranges: &[TimelineClosedRange],
    open_ranges: &[TimelineOpenRange],
) {
    for _ in 0..tasks.len().max(1) {
        let mut changed = false;
        for constraint in constraints {
            let kind = constraint.kind.to_ascii_lowercase();
            if !matches!(kind.as_str(), "starts" | "ends" | "requires") {
                continue;
            }
            let Some(subject_idx) = tasks.iter().position(|t| t.name == constraint.subject) else {
                continue;
            };
            let Some((target_name, endpoint)) = parse_gantt_task_reference(&constraint.target)
            else {
                continue;
            };
            let Some(target) = tasks.iter().find(|t| t.name == target_name) else {
                continue;
            };
            let mut target_day = match endpoint {
                "start" => target.start_day,
                "end" => target.start_day.saturating_add(target.duration_days),
                _ => continue,
            };
            if let Some(offset) = parse_gantt_reference_day_offset(&constraint.target) {
                target_day = if offset.is_negative() {
                    target_day.saturating_sub(offset.unsigned_abs())
                } else {
                    target_day.saturating_add(offset as u32)
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
                );
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
}

fn parse_gantt_reference_day_offset(target: &str) -> Option<i32> {
    let lower = target.to_ascii_lowercase();
    let (sign, marker) = if let Some(idx) = lower.find(" days after ") {
        (1, idx)
    } else if let Some(idx) = lower.find(" day after ") {
        (1, idx)
    } else if let Some(idx) = lower.find(" days before ") {
        (-1, idx)
    } else if let Some(idx) = lower.find(" day before ") {
        (-1, idx)
    } else {
        return None;
    };
    target[..marker]
        .split_whitespace()
        .last()
        .and_then(|n| n.parse::<i32>().ok())
        .map(|n| n.saturating_mul(sign))
}

fn parse_gantt_task_reference(target: &str) -> Option<(String, &'static str)> {
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

fn extract_bracketed_name_from_target(target: &str) -> Option<String> {
    let start = target.find('[')?;
    let end = target[start + 1..].find(']')? + start + 1;
    let name = target[start + 1..end].trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn is_gantt_closed_day(
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

fn parse_timeline_resource_allocations(resources: &[String]) -> Vec<TimelineResourceAllocation> {
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

fn parse_load_percent(raw: &str) -> Option<u32> {
    let value = raw.trim().trim_end_matches('%').trim();
    value.parse::<u32>().ok().map(|n| n.clamp(1, 1000))
}

fn resource_adjusted_work_days(
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

fn timeline_scheduled_span_for_task(
    task: &TimelineTask,
    closed_weekdays: &[String],
    closed_ranges: &[TimelineClosedRange],
    open_ranges: &[TimelineOpenRange],
) -> u32 {
    scheduled_gantt_span_days(
        task.start_day,
        resource_adjusted_work_days(task.workload_days, &task.resource_allocations),
        closed_weekdays,
        closed_ranges,
        open_ranges,
    )
}

fn apply_gantt_task_metadata(
    tasks: &mut [TimelineTask],
    milestones: &mut [TimelineMilestone],
    constraints: &[TimelineConstraint],
) {
    for constraint in constraints {
        if constraint.kind.eq_ignore_ascii_case("critical") {
            if let Some(task) = tasks
                .iter_mut()
                .find(|task| task.name == constraint.subject)
            {
                task.is_critical = true;
            }
            if let Some(milestone) = milestones
                .iter_mut()
                .find(|milestone| milestone.name == constraint.subject)
            {
                milestone.is_critical = true;
            }
        }
        if constraint.kind.eq_ignore_ascii_case("baseline") {
            let Some((start_day, duration_days)) = parse_gantt_baseline_target(&constraint.target)
            else {
                continue;
            };
            if let Some(task) = tasks
                .iter_mut()
                .find(|task| task.name == constraint.subject)
            {
                task.baseline_start_day = Some(start_day);
                task.baseline_duration_days = Some(duration_days);
            }
        }
    }
}

fn parse_gantt_baseline_target(target: &str) -> Option<(u32, u32)> {
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

fn parse_timeline_duration_days(raw: &str) -> Option<u32> {
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

fn mark_inferred_gantt_critical_path(
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
        .map(|task| task.name.clone())
        .collect();
    while let Some(name) = stack.pop() {
        let Some(task) = tasks.iter_mut().find(|task| task.name == name) else {
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

fn is_gantt_closed_weekday(day: u32, closed_weekdays: &[String]) -> bool {
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
