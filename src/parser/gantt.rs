fn parse_gantt_baseline_statement(line: &str) -> Option<StatementKind> {
    let trimmed = line.trim();
    if let Some(kind) = parse_keyword(trimmed) {
        if matches!(&kind, StatementKind::Note(note) if note.text.is_empty()) {
            return None;
        }
        return Some(kind);
    }
    let lower_trimmed = trimmed.to_ascii_lowercase();
    if matches!(
        lower_trimmed.as_str(),
        "hide resources names" | "hide resource names" | "hide resources footbox" | "hide resource footbox"
    ) {
        return Some(StatementKind::HideOption(
            lower_trimmed.strip_prefix("hide ").unwrap_or("").to_string(),
        ));
    }

    if let Some((start_date, end_date)) = parse_gantt_print_between(trimmed) {
        return Some(StatementKind::GanttConstraint {
            subject: "Project".to_string(),
            kind: "print_between".to_string(),
            target: format!("{start_date} to {end_date}"),
        });
    }
    if let Some((scale, options)) = parse_gantt_scale_directive(trimmed) {
        return Some(StatementKind::GanttConstraint {
            subject: "Project".to_string(),
            kind: "scale".to_string(),
            target: if options.is_empty() {
                scale
            } else {
                format!("{scale};{}", options.join(";"))
            },
        });
    }
    if let Some((resource, start_date, end_date)) = parse_gantt_resource_off_range(trimmed) {
        return Some(StatementKind::GanttConstraint {
            subject: format!("__resource::{resource}"),
            kind: "resource_off".to_string(),
            target: if start_date == end_date {
                start_date
            } else {
                format!("{start_date} to {end_date}")
            },
        });
    }
    if let Some((start_date, end_date, color)) = parse_gantt_day_color(trimmed) {
        return Some(StatementKind::GanttConstraint {
            subject: format!("__day::{start_date}::{end_date}"),
            kind: "day_color".to_string(),
            target: color,
        });
    }
    if let Some(kind) = parse_gantt_named_date(trimmed) {
        return Some(kind);
    }
    if let Some((start_date, end_date, label)) = parse_gantt_day_name(trimmed) {
        return Some(StatementKind::GanttConstraint {
            subject: format!("__day::{start_date}::{end_date}"),
            kind: "day_name".to_string(),
            target: label,
        });
    }
    if let Some(rest) = trimmed.strip_prefix("Project starts ") {
        if let Some(date) = parse_gantt_date_literal(rest) {
            return Some(StatementKind::GanttConstraint {
                subject: "Project".to_string(),
                kind: "starts".to_string(),
                target: date,
            });
        }
    }
    if let Some(rest) = trimmed.strip_prefix("Project ends ") {
        if let Some(date) = parse_gantt_date_literal(rest) {
            return Some(StatementKind::GanttConstraint {
                subject: "Project".to_string(),
                kind: "ends".to_string(),
                target: date,
            });
        }
    }
    if let Some((start_date, end_date)) = parse_gantt_closed_date_range(trimmed) {
        return Some(StatementKind::GanttCalendarClosedDateRange {
            start_date,
            end_date,
        });
    }
    if let Some((start_date, end_date)) = parse_gantt_open_date_range(trimmed) {
        return Some(StatementKind::GanttCalendarOpenDateRange {
            start_date,
            end_date,
        });
    }
    if let Some(day) = parse_gantt_closed_weekday(trimmed) {
        return Some(StatementKind::GanttCalendarClosed { day });
    }
    if let Some(day) = parse_gantt_open_weekday(trimmed) {
        return Some(StatementKind::GanttCalendarOpen { day });
    }
    if let Some(label) = parse_gantt_horizontal_separator(trimmed) {
        return Some(StatementKind::Separator(Some(label)));
    }
    if let Some((label, target)) = parse_gantt_vertical_separator(trimmed) {
        return Some(StatementKind::GanttConstraint {
            subject: format!("__separator::{label}"),
            kind: "separator".to_string(),
            target,
        });
    }
    if let Some(compound) = parse_gantt_then_statement(trimmed) {
        return Some(compound);
    }
    if let Some((from, to, style)) = parse_gantt_arrow_dependency(trimmed) {
        return Some(StatementKind::GanttConstraint {
            subject: to,
            kind: "requires".to_string(),
            target: if let Some(style) = style {
                format!("[{from}] with {style}")
            } else {
                format!("[{from}]")
            },
        });
    }

    let (subject, rest) = parse_bracket_subject(trimmed)?;
    let (subject, alias, rest) = parse_gantt_alias(subject, rest);
    let subject_key = alias.clone().unwrap_or_else(|| subject.clone());
    if rest.is_empty() {
        return Some(StatementKind::GanttTaskDecl {
            name: subject,
            alias,
            start_date: None,
            duration_days: None,
            depends_on: Vec::new(),
            resources: Vec::new(),
        });
    }

    let rest = rest.trim();
    let (rest_without_resources, resources) = extract_gantt_resources(rest);
    let rest = rest_without_resources.trim();
    if let Some(rest) = rest.strip_prefix(':') {
        if subject.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            let label = rest.trim();
            if !label.is_empty() {
                return Some(StatementKind::GanttMilestoneDecl {
                    name: label.to_string(),
                    happens_on: Some(subject),
                });
            }
        }
        return Some(StatementKind::GanttTaskDecl {
            name: rest.trim().to_string(),
            alias: None,
            start_date: None,
            duration_days: None,
            depends_on: Vec::new(),
            resources,
        });
    }
    if is_gantt_compound_clause(rest) {
        return Some(StatementKind::GanttCompound {
            name: subject,
            alias,
            resources,
            clauses: rest.to_string(),
            after_previous: false,
        });
    }
    if let Some((start_date, duration_days)) = parse_gantt_start_and_duration(rest) {
        return Some(StatementKind::GanttTaskDecl {
            name: subject,
            alias,
            start_date: Some(start_date),
            duration_days: Some(duration_days),
            depends_on: Vec::new(),
            resources,
        });
    }
    if let Some(duration_days) = parse_gantt_duration_clause(rest) {
        return Some(StatementKind::GanttTaskDecl {
            name: subject,
            alias,
            start_date: None,
            duration_days: Some(duration_days),
            depends_on: Vec::new(),
            resources,
        });
    }
    if let Some(start_date) = parse_gantt_start_date_clause(rest) {
        return Some(StatementKind::GanttTaskDecl {
            name: subject,
            alias,
            start_date: Some(start_date),
            duration_days: None,
            depends_on: Vec::new(),
            resources,
        });
    }
    if !resources.is_empty() {
        return Some(StatementKind::GanttTaskDecl {
            name: subject,
            alias,
            start_date: None,
            duration_days: None,
            depends_on: Vec::new(),
            resources,
        });
    }
    let lower = rest.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "is critical" | "critical" | "is on critical path" | "is on the critical path"
    ) {
        return Some(StatementKind::GanttConstraint {
            subject: subject_key,
            kind: "critical".to_string(),
            target: "true".to_string(),
        });
    }
    if let Some(target) = lower
        .strip_prefix("baseline ")
        .and_then(|_| rest.get("baseline ".len()..))
        .or_else(|| {
            lower
                .strip_prefix("has baseline ")
                .and_then(|_| rest.get("has baseline ".len()..))
        })
        .or_else(|| {
            lower
                .strip_prefix("planned ")
                .and_then(|_| rest.get("planned ".len()..))
        })
    {
        return Some(StatementKind::GanttConstraint {
            subject: subject_key,
            kind: "baseline".to_string(),
            target: target.trim().to_string(),
        });
    }
    if lower.starts_with("happens") {
        return Some(StatementKind::GanttMilestoneDecl {
            name: subject_key,
            happens_on: parse_gantt_happens_target(rest),
        });
    }
    if let Some(color) = parse_gantt_task_color(rest) {
        return Some(StatementKind::GanttConstraint {
            subject: subject_key,
            kind: "color".to_string(),
            target: color,
        });
    }
    if let Some(percent) = parse_gantt_completion(rest) {
        return Some(StatementKind::GanttConstraint {
            subject: subject_key,
            kind: "completion".to_string(),
            target: percent.to_string(),
        });
    }
    if let Some(url) = parse_gantt_link_target(rest) {
        return Some(StatementKind::GanttConstraint {
            subject: subject_key,
            kind: "link".to_string(),
            target: url,
        });
    }
    if lower == "is deleted" || lower == "deleted" {
        return Some(StatementKind::GanttConstraint {
            subject: subject_key,
            kind: "deleted".to_string(),
            target: "true".to_string(),
        });
    }
    if let Some(target) = parse_gantt_pause_target(rest) {
        return Some(StatementKind::GanttConstraint {
            subject: subject_key,
            kind: "pause".to_string(),
            target,
        });
    }
    for kind in ["starts", "ends", "requires"] {
        if lower.starts_with(kind) {
            let target = rest[kind.len()..]
                .trim()
                .strip_prefix("at ")
                .unwrap_or_else(|| rest[kind.len()..].trim())
                .trim()
                .to_string();
            return Some(StatementKind::GanttConstraint {
                subject: subject_key,
                kind: kind.to_string(),
                target,
            });
        }
    }
    None
}
