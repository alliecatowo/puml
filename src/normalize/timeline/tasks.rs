use super::*;

pub(super) fn gantt_task_ref(name: &str, alias: Option<&str>) -> String {
    alias.unwrap_or(name).to_string()
}

pub(super) fn gantt_task_matches(task: &TimelineTask, reference: &str) -> bool {
    task.alias.as_deref() == Some(reference) || task.name == reference
}

pub(super) fn upsert_gantt_task(
    tasks: &mut Vec<TimelineTask>,
    name: String,
    alias: Option<String>,
    start_date: Option<&str>,
    duration_days: Option<u32>,
    resources: Vec<String>,
) {
    let parsed_start_day = start_date.and_then(parse_iso_date_day);
    let task_ref = gantt_task_ref(&name, alias.as_deref());
    if let Some(existing) = tasks
        .iter_mut()
        .find(|task| gantt_task_matches(task, &task_ref))
    {
        if existing.alias.is_none() {
            existing.alias = alias;
        }
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
        return;
    }

    let workload_days = duration_days.unwrap_or(14).max(1);
    let resource_allocations = parse_timeline_resource_allocations(&resources);
    let duration_days = resource_adjusted_work_days(workload_days, &resource_allocations);
    tasks.push(TimelineTask {
        name,
        alias,
        start_day: parsed_start_day.unwrap_or(0),
        workload_days,
        duration_days,
        resources,
        resource_allocations,
        baseline_start_day: None,
        baseline_duration_days: None,
        is_critical: false,
        fill_color: None,
        stroke_color: None,
        completion_percent: None,
        hyperlink: None,
        is_deleted: false,
        pause_weekdays: Vec::new(),
        pause_ranges: Vec::new(),
    });
}

pub(super) fn apply_gantt_compound_clauses(
    tasks: &mut [TimelineTask],
    constraints: &mut Vec<TimelineConstraint>,
    subject: &str,
    clauses: &str,
    resources: &[String],
) {
    let lower_clauses = clauses.to_ascii_lowercase();
    if let Some(target) = lower_clauses
        .strip_prefix("baseline ")
        .and_then(|_| clauses.get("baseline ".len()..))
        .or_else(|| {
            lower_clauses
                .strip_prefix("has baseline ")
                .and_then(|_| clauses.get("has baseline ".len()..))
        })
        .or_else(|| {
            lower_clauses
                .strip_prefix("planned ")
                .and_then(|_| clauses.get("planned ".len()..))
        })
    {
        constraints.push(TimelineConstraint {
            subject: subject.to_string(),
            kind: "baseline".to_string(),
            target: target.trim().to_string(),
        });
        return;
    }
    let normalized = clauses.replace(" and ", "\n");
    for clause in normalized
        .lines()
        .map(str::trim)
        .filter(|clause| !clause.is_empty())
    {
        let lower = clause.to_ascii_lowercase();
        if let Some(days) = parse_timeline_duration_days(clause) {
            if lower.starts_with("lasts ") || lower.starts_with("requires ") {
                if let Some(task) = tasks
                    .iter_mut()
                    .find(|task| gantt_task_matches(task, subject))
                {
                    task.workload_days = days.max(1);
                    let allocations = if resources.is_empty() {
                        task.resource_allocations.clone()
                    } else {
                        parse_timeline_resource_allocations(resources)
                    };
                    task.duration_days = resource_adjusted_work_days(days, &allocations);
                    if !allocations.is_empty() {
                        task.resource_allocations = allocations;
                    }
                }
                continue;
            }
        }
        if let Some(color) = lower
            .strip_prefix("is colored in ")
            .and_then(|_| clause.get("is colored in ".len()..))
        {
            constraints.push(TimelineConstraint {
                subject: subject.to_string(),
                kind: "color".to_string(),
                target: color.trim().to_string(),
            });
            continue;
        }
        if let Some(percent) = parse_gantt_completion_percent(clause) {
            constraints.push(TimelineConstraint {
                subject: subject.to_string(),
                kind: "completion".to_string(),
                target: percent.to_string(),
            });
            continue;
        }
        if lower == "is deleted" || lower == "deleted" {
            constraints.push(TimelineConstraint {
                subject: subject.to_string(),
                kind: "deleted".to_string(),
                target: "true".to_string(),
            });
            continue;
        }
        if let Some(target) = parse_gantt_pause_clause(clause) {
            constraints.push(TimelineConstraint {
                subject: subject.to_string(),
                kind: "pause".to_string(),
                target,
            });
            continue;
        }
        if let Some(url) = parse_gantt_link_target(clause) {
            constraints.push(TimelineConstraint {
                subject: subject.to_string(),
                kind: "link".to_string(),
                target: url,
            });
            continue;
        }
        if let Some(target) = lower
            .strip_prefix("baseline ")
            .and_then(|_| clause.get("baseline ".len()..))
            .or_else(|| {
                lower
                    .strip_prefix("has baseline ")
                    .and_then(|_| clause.get("has baseline ".len()..))
            })
            .or_else(|| {
                lower
                    .strip_prefix("planned ")
                    .and_then(|_| clause.get("planned ".len()..))
            })
        {
            constraints.push(TimelineConstraint {
                subject: subject.to_string(),
                kind: "baseline".to_string(),
                target: target.trim().to_string(),
            });
            continue;
        }
        for kind in ["starts", "ends", "requires"] {
            if lower.starts_with(kind) {
                let target = clause[kind.len()..]
                    .trim()
                    .strip_prefix("at ")
                    .unwrap_or_else(|| clause[kind.len()..].trim())
                    .trim()
                    .to_string();
                constraints.push(TimelineConstraint {
                    subject: subject.to_string(),
                    kind: kind.to_string(),
                    target,
                });
                break;
            }
        }
    }
}

pub(super) fn parse_gantt_completion_percent(clause: &str) -> Option<u32> {
    let lower = clause.to_ascii_lowercase();
    let percent_idx = lower.find('%')?;
    let value = lower[..percent_idx].split_whitespace().last()?;
    let suffix = lower[percent_idx + 1..].trim();
    matches!(suffix, "complete" | "completed")
        .then(|| value.parse::<u32>().ok().map(|n| n.min(100)))
        .flatten()
}

pub(super) fn parse_gantt_link_target(clause: &str) -> Option<String> {
    let trimmed = clause.trim();
    let lower = trimmed.to_ascii_lowercase();
    let target = lower
        .strip_prefix("links to ")
        .and_then(|_| trimmed.get("links to ".len()..))?
        .trim();
    let inner = target.strip_prefix("[[")?.strip_suffix("]]")?.trim();
    let url = inner.split_whitespace().next().unwrap_or(inner).trim();
    (!url.is_empty()).then(|| url.to_string())
}

pub(super) fn parse_gantt_pause_clause(clause: &str) -> Option<String> {
    let trimmed = clause.trim();
    let lower = trimmed.to_ascii_lowercase();
    lower
        .strip_prefix("pauses on ")
        .and_then(|_| trimmed.get("pauses on ".len()..))
        .or_else(|| {
            lower
                .strip_prefix("pause on ")
                .and_then(|_| trimmed.get("pause on ".len()..))
        })
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub(super) fn split_gantt_scale_target(target: &str) -> (String, Vec<String>) {
    let mut parts = target.split(';');
    let scale = parts.next().unwrap_or(target).trim().to_string();
    let options = parts
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect();
    (scale, options)
}

pub(super) fn apply_gantt_task_metadata(
    tasks: &mut [TimelineTask],
    milestones: &mut [TimelineMilestone],
    constraints: &[TimelineConstraint],
) {
    for constraint in constraints {
        if constraint.kind.eq_ignore_ascii_case("critical") {
            if let Some(task) = tasks
                .iter_mut()
                .find(|task| gantt_task_matches(task, &constraint.subject))
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
                .find(|task| gantt_task_matches(task, &constraint.subject))
            {
                task.baseline_start_day = Some(start_day);
                task.baseline_duration_days = Some(duration_days);
            }
        }
        if constraint.kind.eq_ignore_ascii_case("color") {
            if let Some(task) = tasks
                .iter_mut()
                .find(|task| gantt_task_matches(task, &constraint.subject))
            {
                let (fill, stroke) = parse_gantt_color_pair(&constraint.target);
                task.fill_color = fill;
                task.stroke_color = stroke;
            }
        }
        if constraint.kind.eq_ignore_ascii_case("completion") {
            if let Some(task) = tasks
                .iter_mut()
                .find(|task| gantt_task_matches(task, &constraint.subject))
            {
                task.completion_percent = constraint.target.parse::<u32>().ok().map(|n| n.min(100));
            }
        }
        if constraint.kind.eq_ignore_ascii_case("link") {
            if let Some(task) = tasks
                .iter_mut()
                .find(|task| gantt_task_matches(task, &constraint.subject))
            {
                task.hyperlink = Some(constraint.target.clone());
            }
        }
        if constraint.kind.eq_ignore_ascii_case("deleted") {
            if let Some(task) = tasks
                .iter_mut()
                .find(|task| gantt_task_matches(task, &constraint.subject))
            {
                task.is_deleted = true;
            }
        }
        if constraint.kind.eq_ignore_ascii_case("pause") {
            let Some(task) = tasks
                .iter_mut()
                .find(|task| gantt_task_matches(task, &constraint.subject))
            else {
                continue;
            };
            if let Some(weekday) = parse_gantt_pause_weekday(&constraint.target) {
                if !task
                    .pause_weekdays
                    .iter()
                    .any(|existing| existing == weekday)
                {
                    task.pause_weekdays.push(weekday.to_string());
                }
                continue;
            }
            if let Some(range) = parse_gantt_task_pause_range(&constraint.target) {
                if !task.pause_ranges.iter().any(|existing| {
                    existing.start_day == range.start_day && existing.end_day == range.end_day
                }) {
                    task.pause_ranges.push(range);
                }
            }
        }
    }
}

pub(super) fn parse_gantt_color_pair(raw: &str) -> (Option<String>, Option<String>) {
    let (fill, stroke) = raw.split_once('/').unwrap_or((raw, ""));
    (
        (!fill.trim().is_empty()).then(|| fill.trim().to_string()),
        (!stroke.trim().is_empty()).then(|| stroke.trim().to_string()),
    )
}
