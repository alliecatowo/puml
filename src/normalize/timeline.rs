use super::common::{self, CommonDirectives, LegendTextMode, RawSyntaxContext};
use super::*;

mod calendar;
mod dates;
mod schedule;
mod tasks;

use calendar::*;
use dates::*;
use schedule::*;
use tasks::*;

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
    let mut common = CommonDirectives::default();
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
            StatementKind::ChronologyHappensOn {
                subject,
                when,
                end,
                color,
                bracket,
            } => chronology_events.push(TimelineChronologyEvent {
                subject,
                when,
                end,
                color,
                bracket,
            }),
            StatementKind::Title(v) => common.title(v),
            StatementKind::Header(v) => common.raw_header(v),
            StatementKind::Footer(v) => common.raw_footer(v),
            StatementKind::Caption(v) => common.caption(v),
            StatementKind::Legend(v) => common.legend(v, LegendTextMode::StripPackedPosition),
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
            | StatementKind::StyleParam { .. }
            | StatementKind::Theme(_)
            | StatementKind::Pragma(_)
            | StatementKind::AllowMixing
            | StatementKind::LegendPos(_)
            | StatementKind::SetOption { .. } => {}
            StatementKind::HideOption(option) => match option.as_str() {
                "resources names" | "resource names" => hide_resource_names = true,
                "resources footbox" | "resource footbox" => hide_resource_footbox = true,
                _ => {}
            },
            kind if kind.raw_syntax().is_some() => {
                let raw = kind.raw_syntax().expect("raw syntax guard");
                return Err(common::raw_syntax_diagnostic(
                    raw,
                    stmt.span,
                    RawSyntaxContext::Timeline(document.kind),
                ));
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
        title: common.title,
        header: common.header,
        footer: common.footer,
        caption: common.caption,
        legend: common.legend,
        notes,
        hide_footbox,
        hide_resource_names,
        hide_resource_footbox,
        warnings: Vec::new(),
    })
}
