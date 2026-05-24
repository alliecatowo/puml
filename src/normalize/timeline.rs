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
    let mut day_markers = Vec::new();
    let mut resource_off_ranges = Vec::new();
    let mut named_dates: Vec<TimelineNamedDate> = Vec::new();
    let mut scale = None;
    let mut scale_options = Vec::new();
    let mut print_start = None;
    let mut print_end = None;
    let mut print_start_day = None;
    let mut print_end_day = None;
    let mut title = None;
    let mut header = None;
    let mut footer = None;
    let mut caption = None;
    let mut legend = None;
    let mut notes = Vec::new();
    let mut hide_footbox = false;
    let mut hide_resource_names = false;
    let mut hide_resource_footbox = false;
    let mut last_task_ref: Option<String> = None;

    for stmt in document.statements {
        match stmt.kind {
            StatementKind::GanttTaskDecl {
                name,
                alias,
                start_date,
                duration_days,
                resources,
                ..
            } => {
                let task_ref = gantt_task_ref(&name, alias.as_deref());
                upsert_gantt_task(
                    &mut tasks,
                    name,
                    alias,
                    start_date.as_deref(),
                    duration_days,
                    resources,
                );
                if let Some(start_date) = start_date {
                    if parse_iso_date_day(&start_date).is_none() {
                        constraints.push(TimelineConstraint {
                            subject: task_ref.clone(),
                            kind: "starts".to_string(),
                            target: start_date,
                        });
                    }
                }
                last_task_ref = Some(task_ref);
            }
            StatementKind::GanttCompound {
                name,
                alias,
                resources,
                clauses,
                after_previous,
            } => {
                let task_ref = gantt_task_ref(&name, alias.as_deref());
                upsert_gantt_task(&mut tasks, name, alias, None, None, resources.clone());
                if after_previous {
                    if let Some(previous) = &last_task_ref {
                        constraints.push(TimelineConstraint {
                            subject: task_ref.clone(),
                            kind: "starts".to_string(),
                            target: format!("[{previous}]'s end"),
                        });
                    }
                }
                apply_gantt_compound_clauses(
                    &mut tasks,
                    &mut constraints,
                    &task_ref,
                    &clauses,
                    &resources,
                );
                last_task_ref = Some(task_ref);
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
                    let (value, options) = split_gantt_scale_target(&target);
                    scale = Some(value);
                    scale_options.extend(options);
                } else if subject.eq_ignore_ascii_case("Project")
                    && kind.eq_ignore_ascii_case("print_between")
                {
                    if let Some((start_date, end_date)) = parse_gantt_target_range(&target) {
                        if let (Some(start_day), Some(end_day)) = (
                            parse_iso_date_day(&start_date),
                            parse_iso_date_day(&end_date),
                        ) {
                            let (start_date, end_date, start_day, end_day) = if start_day <= end_day
                            {
                                (start_date, end_date, start_day, end_day)
                            } else {
                                (end_date, start_date, end_day, start_day)
                            };
                            print_start = Some(start_date);
                            print_end = Some(end_date);
                            print_start_day = Some(start_day);
                            print_end_day = Some(end_day);
                        }
                    }
                } else if kind.eq_ignore_ascii_case("separator") {
                    separators.push(TimelineSeparator {
                        label: subject
                            .strip_prefix("__separator::")
                            .unwrap_or(&subject)
                            .to_string(),
                        target: (!target.trim().is_empty()).then(|| target.trim().to_string()),
                    });
                } else if kind.eq_ignore_ascii_case("day_color")
                    || kind.eq_ignore_ascii_case("day_name")
                {
                    upsert_gantt_day_marker(&mut day_markers, &subject, &kind, &target);
                } else if kind.eq_ignore_ascii_case("resource_off") {
                    if let Some(range) = parse_gantt_resource_off_constraint(&subject, &target) {
                        resource_off_ranges.push(range);
                    }
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
            StatementKind::GanttNamedDate { date, label } => {
                if let Some(day) = parse_iso_date_day(&date) {
                    if !named_dates.iter().any(|nd| nd.day == day) {
                        named_dates.push(TimelineNamedDate { date, label, day });
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
            StatementKind::Note(note) => {
                notes.push(TimelineNote {
                    target: note.target.or_else(|| last_task_ref.clone()),
                    position: note.position,
                    text: note.text,
                });
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
            StatementKind::Footbox(show) => {
                hide_footbox = !show;
            }
            StatementKind::SkinParam { .. }
            | StatementKind::Theme(_)
            | StatementKind::Pragma(_)
            | StatementKind::LegendPos(_)
            | StatementKind::SetOption { .. } => {}
            StatementKind::HideOption(option) => match option.as_str() {
                "resources names" | "resource names" => hide_resource_names = true,
                "resources footbox" | "resource footbox" => hide_resource_footbox = true,
                _ => {}
            },
            StatementKind::Unknown(line)
            | StatementKind::UnsupportedSyntax(line)
            | StatementKind::DeferredRaw(line)
            | StatementKind::CommentLowered(line)
            | StatementKind::MalformedSyntax(line) => {
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
                &resource_off_ranges,
            );
            let task_end = task.start_day.saturating_add(task.duration_days);
            if task_end > cursor {
                cursor = task_end;
            }
        }
        apply_gantt_task_metadata(&mut tasks, &mut milestones, &constraints);
        apply_gantt_absolute_constraints(&mut tasks, &constraints, fallback_anchor);
        apply_gantt_start_end_duration_constraints(&mut tasks, &constraints, fallback_anchor);
        for task in &mut tasks {
            task.duration_days = timeline_scheduled_span_for_task(
                task,
                &closed_weekdays,
                &closed_ranges,
                &open_ranges,
                &resource_off_ranges,
            );
        }
        apply_gantt_task_reference_constraints(
            &mut tasks,
            &constraints,
            &closed_weekdays,
            &closed_ranges,
            &open_ranges,
            &resource_off_ranges,
        );
        for task in &mut tasks {
            task.duration_days = timeline_scheduled_span_for_task(
                task,
                &closed_weekdays,
                &closed_ranges,
                &open_ranges,
                &resource_off_ranges,
            );
        }
        apply_gantt_start_end_duration_constraints(&mut tasks, &constraints, fallback_anchor);
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
        day_markers,
        resource_off_ranges,
        named_dates,
        scale,
        scale_options,
        print_start,
        print_end,
        print_start_day,
        print_end_day,
        project_start,
        project_start_day,
        title,
        header,
        footer,
        caption,
        legend,
        notes,
        hide_footbox,
        hide_resource_names,
        hide_resource_footbox,
        warnings: Vec::new(),
    })
}

fn gantt_task_ref(name: &str, alias: Option<&str>) -> String {
    alias.unwrap_or(name).to_string()
}

fn gantt_task_matches(task: &TimelineTask, reference: &str) -> bool {
    task.alias.as_deref() == Some(reference) || task.name == reference
}

fn upsert_gantt_task(
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

fn apply_gantt_compound_clauses(
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

fn parse_gantt_completion_percent(clause: &str) -> Option<u32> {
    let lower = clause.to_ascii_lowercase();
    let percent_idx = lower.find('%')?;
    let value = lower[..percent_idx].split_whitespace().last()?;
    let suffix = lower[percent_idx + 1..].trim();
    matches!(suffix, "complete" | "completed")
        .then(|| value.parse::<u32>().ok().map(|n| n.min(100)))
        .flatten()
}

fn parse_gantt_link_target(clause: &str) -> Option<String> {
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

fn parse_gantt_pause_clause(clause: &str) -> Option<String> {
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

fn split_gantt_scale_target(target: &str) -> (String, Vec<String>) {
    let mut parts = target.split(';');
    let scale = parts.next().unwrap_or(target).trim().to_string();
    let options = parts
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect();
    (scale, options)
}

fn upsert_gantt_day_marker(
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

fn parse_gantt_day_marker_subject(subject: &str) -> Option<(String, String)> {
    let rest = subject.strip_prefix("__day::")?;
    let mut parts = rest.split("::");
    Some((parts.next()?.to_string(), parts.next()?.to_string()))
}

fn parse_gantt_resource_off_constraint(
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

fn parse_gantt_target_range(target: &str) -> Option<(String, String)> {
    let lower = target.to_ascii_lowercase();
    if let Some(idx) = lower.find(" to ") {
        return Some((
            target[..idx].trim().to_string(),
            target[idx + " to ".len()..].trim().to_string(),
        ));
    }
    Some((target.trim().to_string(), target.trim().to_string()))
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

fn parse_relative_day(raw: &str) -> Option<u32> {
    let trimmed = raw.trim();
    let rest = trimmed
        .strip_prefix("D+")
        .or_else(|| trimmed.strip_prefix("d+"))?;
    rest.trim().parse::<u32>().ok()
}

fn resolve_gantt_absolute_day(target: &str, anchor_day: u32) -> Option<u32> {
    parse_iso_date_day(target)
        .or_else(|| parse_relative_day(target).map(|day| anchor_day.saturating_add(day)))
}

fn apply_gantt_absolute_constraints(
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

fn apply_gantt_start_end_duration_constraints(
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

fn scheduled_gantt_span_days_for_task(
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

fn apply_gantt_task_reference_constraints(
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

fn add_gantt_working_days(
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

fn is_gantt_non_working_day_for_task(
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

fn is_gantt_task_paused_day(task: &TimelineTask, day: u32) -> bool {
    task.pause_weekdays
        .iter()
        .any(|weekday| is_gantt_weekday(day, weekday))
        || task
            .pause_ranges
            .iter()
            .any(|range| (range.start_day..=range.end_day).contains(&day))
}

fn is_gantt_resource_off_day(
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

fn parse_gantt_pause_weekday(raw: &str) -> Option<&'static str> {
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

fn parse_gantt_task_pause_range(target: &str) -> Option<TimelineTaskPauseRange> {
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

fn apply_gantt_task_metadata(
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

fn parse_gantt_color_pair(raw: &str) -> (Option<String>, Option<String>) {
    let (fill, stroke) = raw.split_once('/').unwrap_or((raw, ""));
    (
        (!fill.trim().is_empty()).then(|| fill.trim().to_string()),
        (!stroke.trim().is_empty()).then(|| stroke.trim().to_string()),
    )
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

fn is_gantt_weekday(day: u32, weekday: &str) -> bool {
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
