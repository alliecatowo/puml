use super::*;

pub fn render_timeline_stub_svg(document: &TimelineDocument) -> String {
    render_timeline_svg(document)
}

/// Render Gantt/Chronology timelines as real SVGs:
///   - Gantt: horizontal task bars on a date axis, milestone diamonds,
///     dashed arrows for `requires`/start/etc. constraints between bars.
///   - Chronology: vertical timeline with event bullets along a date axis.
pub fn render_timeline_svg(document: &TimelineDocument) -> String {
    match document.kind {
        DiagramKind::Chronology => render_chronology_svg(document),
        _ => render_gantt_svg(document),
    }
}

fn render_gantt_svg(document: &TimelineDocument) -> String {
    let width: i32 = 800;
    let margin_x: i32 = 32;
    let label_col_w: i32 = 160;
    let bar_height: i32 = 20;
    let row_gap: i32 = 14;
    let header_h: i32 = 28;
    let chart_left: i32 = margin_x + label_col_w + 12;
    let chart_right: i32 = width - margin_x;
    let chart_w: i32 = chart_right - chart_left;

    let title_h = document
        .title
        .as_deref()
        .map(|t| 8 + (t.lines().count() as i32) * 22)
        .unwrap_or(0);
    let has_calendar_notes = !document.closed_weekdays.is_empty()
        || !document.closed_ranges.is_empty()
        || !document.open_ranges.is_empty();
    let calendar_h = if !has_calendar_notes { 0 } else { 18 };
    let scale_h = if document.scale.is_some() { 18 } else { 0 };

    let row_count =
        (document.tasks.len() + document.milestones.len() + document.separators.len()) as i32;
    let chart_top = 40 + title_h + calendar_h + scale_h + header_h;
    let chart_h = (row_count.max(1)) * (bar_height + row_gap) + 20;
    let total_h = chart_top + chart_h + 40;

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">",
        w = width,
        h = total_h
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    if let Some(scale) = &document.scale {
        out.push_str(&format!(
            "<metadata data-gantt-scale=\"{}\"/>",
            escape_text(scale)
        ));
    }
    let resource_count = document
        .tasks
        .iter()
        .flat_map(|task| {
            task.resource_allocations
                .iter()
                .map(|allocation| allocation.name.as_str())
        })
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    out.push_str(&format!(
        "<metadata data-gantt-resource-count=\"{resource_count}\" data-gantt-separator-count=\"{}\"/>",
        document.separators.len()
    ));

    // Title
    if let Some(title) = &document.title {
        let mut ty = 28;
        for line in title.lines() {
            out.push_str(&format!(
                "<text x=\"{x}\" y=\"{y}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\" fill=\"#0f172a\">{txt}</text>",
                x = margin_x,
                y = ty,
                txt = escape_text(line)
            ));
            ty += 22;
        }
    } else {
        out.push_str(&format!(
            "<text x=\"{x}\" y=\"28\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\" fill=\"#0f172a\">Gantt</text>",
            x = margin_x
        ));
    }
    if has_calendar_notes {
        let mut labels = Vec::new();
        if !document.closed_weekdays.is_empty() {
            labels.push(
                document
                    .closed_weekdays
                    .iter()
                    .map(|day| title_case_ascii(day))
                    .collect::<Vec<_>>()
                    .join(", "),
            );
        }
        labels.extend(document.closed_ranges.iter().map(|range| {
            if range.start_date == range.end_date {
                range.start_date.clone()
            } else {
                format!("{} to {}", range.start_date, range.end_date)
            }
        }));
        let mut label = if labels.is_empty() {
            String::new()
        } else {
            format!("closed {}", labels.join("; "))
        };
        if !document.open_ranges.is_empty() {
            let open_label = document
                .open_ranges
                .iter()
                .map(|range| {
                    if range.start_date == range.end_date {
                        range.start_date.clone()
                    } else {
                        format!("{} to {}", range.start_date, range.end_date)
                    }
                })
                .collect::<Vec<_>>()
                .join("; ");
            if label.is_empty() {
                label = format!("open {open_label}");
            } else {
                label.push_str(&format!("; open {open_label}"));
            }
        }
        out.push_str(&format!(
            "<text class=\"gantt-calendar\" x=\"{x}\" y=\"{y}\" font-family=\"monospace\" font-size=\"11\" fill=\"#92400e\">Calendar: {label}</text>",
            x = margin_x,
            y = 42 + title_h,
            label = escape_text(&label)
        ));
    }
    if let Some(scale) = &document.scale {
        out.push_str(&format!(
            "<text class=\"gantt-scale\" x=\"{x}\" y=\"{y}\" font-family=\"monospace\" font-size=\"11\" fill=\"#334155\">Scale: {scale}</text>",
            x = margin_x,
            y = 42 + title_h + calendar_h,
            scale = escape_text(scale)
        ));
    }

    let has_resource_lanes = document.tasks.iter().any(|t| !t.resources.is_empty());
    let mut ordered_tasks: Vec<&TimelineTask> = document.tasks.iter().collect();
    if has_resource_lanes {
        ordered_tasks.sort_by(|a, b| {
            resource_lane_label(a)
                .cmp(&resource_lane_label(b))
                .then_with(|| a.name.cmp(&b.name))
        });
    }

    // Build row index for tasks + milestones
    let mut row_index: std::collections::BTreeMap<String, i32> = std::collections::BTreeMap::new();
    let mut row_counter: i32 = 0;
    for task in &ordered_tasks {
        row_index.insert(task.name.clone(), row_counter);
        row_counter += 1;
    }
    let task_count = document.tasks.len() as i32;
    for milestone in &document.milestones {
        row_index.insert(milestone.name.clone(), row_counter);
        row_counter += 1;
    }
    for separator in &document.separators {
        row_index.insert(format!("__separator::{}", separator.label), row_counter);
        row_counter += 1;
    }

    let task_bounds: std::collections::BTreeMap<&str, (u32, u32)> = document
        .tasks
        .iter()
        .map(|t| {
            (
                t.name.as_str(),
                (
                    t.start_day,
                    t.start_day.saturating_add(t.duration_days.max(1)),
                ),
            )
        })
        .collect();
    let preliminary_min_day = document
        .project_start_day
        .into_iter()
        .chain(document.tasks.iter().map(|t| t.start_day))
        .min()
        .unwrap_or(0);
    let milestone_anchor = document.project_start_day.unwrap_or(preliminary_min_day);
    let mut milestone_day: std::collections::BTreeMap<&str, u32> =
        std::collections::BTreeMap::new();
    for ms in &document.milestones {
        if let Some(day) = ms
            .happens_on
            .as_deref()
            .and_then(|target| resolve_gantt_milestone_day(target, milestone_anchor, &task_bounds))
        {
            milestone_day.insert(ms.name.as_str(), day);
            continue;
        }
        for c in &document.constraints {
            if c.subject != ms.name {
                continue;
            }
            if let Some(day) =
                resolve_gantt_milestone_day(&c.target, milestone_anchor, &task_bounds)
            {
                milestone_day.insert(ms.name.as_str(), day);
                break;
            }
        }
    }

    let min_day = document
        .project_start_day
        .into_iter()
        .chain(document.tasks.iter().map(|t| t.start_day))
        .chain(milestone_day.values().copied())
        .min()
        .unwrap_or(0);
    let project_end_day = document
        .constraints
        .iter()
        .find(|c| {
            c.subject.eq_ignore_ascii_case("Project")
                && c.kind.eq_ignore_ascii_case("ends")
                && parse_iso_date_day_number(&c.target).is_some()
        })
        .and_then(|c| parse_iso_date_day_number(&c.target));
    let max_day_exclusive = document
        .project_start_day
        .map(|d| d.saturating_add(1))
        .into_iter()
        .chain(project_end_day.map(|d| d.saturating_add(1)))
        .chain(
            document
                .tasks
                .iter()
                .map(|t| t.start_day.saturating_add(t.duration_days.max(1))),
        )
        .chain(milestone_day.values().map(|d| d.saturating_add(1)))
        .chain(document.separators.iter().filter_map(|separator| {
            separator
                .target
                .as_deref()
                .and_then(|target| {
                    resolve_gantt_milestone_day(target, milestone_anchor, &task_bounds)
                })
                .map(|day| day.saturating_add(1))
        }))
        .max()
        .unwrap_or(min_day.saturating_add(1));
    let total_days = max_day_exclusive.saturating_sub(min_day).max(1);
    let date_axis = document.project_start_day.is_some() || min_day > 366;
    let tick_offsets = gantt_tick_offsets(total_days, document.scale.as_deref());

    // Axis header bar
    out.push_str(&format!(
        "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"#f1f5f9\" stroke=\"#cbd5e1\" stroke-width=\"1\"/>",
        x = chart_left,
        y = chart_top - header_h,
        w = chart_w,
        h = header_h
    ));
    for day_offset in tick_offsets {
        let x = chart_left + ((chart_w as u32 * day_offset) / total_days) as i32;
        out.push_str(&format!(
            "<line x1=\"{x}\" y1=\"{y1}\" x2=\"{x}\" y2=\"{y2}\" stroke=\"#e2e8f0\" stroke-width=\"1\"/>",
            y1 = chart_top - header_h,
            y2 = chart_top + chart_h
        ));
        out.push_str(&format!(
            "<text class=\"gantt-scale-tick\" data-gantt-tick-day=\"{day}\" x=\"{tx}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"11\" fill=\"#475569\">{label}</text>",
            day = escape_text(&format_gantt_axis_label(
                min_day.saturating_add(day_offset),
                min_day,
                true
            )),
            tx = x + 6,
            ty = chart_top - 10,
            label = escape_text(&format_gantt_scale_axis_label(
                min_day.saturating_add(day_offset),
                min_day,
                date_axis,
                document.scale.as_deref()
            ))
        ));
    }

    let bar_geom = |task: &TimelineTask| -> (i32, i32) {
        let start_offset = task.start_day.saturating_sub(min_day);
        let bx = chart_left + ((chart_w as u32 * start_offset) / total_days) as i32;
        let bw = (((chart_w as u32) * task.duration_days.max(1)) / total_days).max(8) as i32;
        (bx, bw)
    };
    let day_to_x = |day: u32| -> i32 {
        let start_offset = day.saturating_sub(min_day);
        chart_left + ((chart_w as u32 * start_offset) / total_days) as i32
    };
    for range in &document.closed_ranges {
        if range.end_day < min_day || range.start_day > max_day_exclusive {
            continue;
        }
        let start = range.start_day.max(min_day);
        let end = range.end_day.saturating_add(1).min(max_day_exclusive);
        let x = day_to_x(start);
        let w = (day_to_x(end) - x).max(2);
        out.push_str(&format!(
            "<rect class=\"gantt-closed-range\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"#fef3c7\" opacity=\"0.7\"/>",
            y = chart_top,
            h = chart_h
        ));
    }
    for range in &document.open_ranges {
        if range.end_day < min_day || range.start_day > max_day_exclusive {
            continue;
        }
        let start = range.start_day.max(min_day);
        let end = range.end_day.saturating_add(1).min(max_day_exclusive);
        let x = day_to_x(start);
        let w = (day_to_x(end) - x).max(2);
        out.push_str(&format!(
            "<rect class=\"gantt-open-range\" data-gantt-open=\"{}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"#dcfce7\" opacity=\"0.62\"/>",
            escape_text(&format!(
                "{} to {}",
                format_gantt_axis_label(start, min_day, true),
                format_gantt_axis_label(end.saturating_sub(1), min_day, true)
            )),
            y = chart_top,
            h = chart_h
        ));
    }
    if !document.closed_weekdays.is_empty() {
        let mut day = min_day;
        while day < max_day_exclusive {
            if is_gantt_closed_weekday_number(day, &document.closed_weekdays) {
                let x = day_to_x(day);
                let w = (day_to_x(day.saturating_add(1)) - x).max(2);
                out.push_str(&format!(
                    "<rect class=\"gantt-closed-weekday\" data-gantt-day=\"{}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"#f8fafc\" opacity=\"0.82\"/>",
                    escape_text(&format_gantt_axis_label(day, min_day, date_axis)),
                    y = chart_top,
                    h = chart_h
                ));
            }
            day = day.saturating_add(1);
        }
    }
    if let Some(day) = project_end_day {
        if (min_day..=max_day_exclusive).contains(&day) {
            let x = day_to_x(day);
            out.push_str(&format!(
                "<line class=\"gantt-project-end\" x1=\"{x}\" y1=\"{y1}\" x2=\"{x}\" y2=\"{y2}\" stroke=\"#dc2626\" stroke-width=\"1.5\" stroke-dasharray=\"5 3\"/>",
                y1 = chart_top - header_h,
                y2 = chart_top + chart_h
            ));
            out.push_str(&format!(
                "<text class=\"gantt-project-end-label\" x=\"{x}\" y=\"{y}\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"10\" fill=\"#991b1b\">Project ends {label}</text>",
                x = x - 4,
                y = chart_top - header_h - 4,
                label = escape_text(&format_gantt_axis_label(day, min_day, true))
            ));
        }
    }
    if has_resource_lanes {
        let mut lane_start = 0usize;
        while lane_start < ordered_tasks.len() {
            let lane = resource_lane_label(ordered_tasks[lane_start]);
            let mut lane_end = lane_start + 1;
            while lane_end < ordered_tasks.len()
                && resource_lane_label(ordered_tasks[lane_end]) == lane
            {
                lane_end += 1;
            }
            let y = chart_top + lane_start as i32 * (bar_height + row_gap);
            let h = (lane_end - lane_start) as i32 * (bar_height + row_gap);
            out.push_str(&format!(
                "<rect class=\"resource-lane\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"#eff6ff\" stroke=\"#bfdbfe\" stroke-width=\"1\" opacity=\"0.72\"/>",
                x = chart_left,
                w = chart_w
            ));
            out.push_str(&format!(
                "<text x=\"{x}\" y=\"{y}\" font-family=\"monospace\" font-size=\"11\" fill=\"#1d4ed8\">{label}</text>",
                x = chart_left + 6,
                y = y + 14,
                label = escape_text(&lane)
            ));
            lane_start = lane_end;
        }
    }

    // Render tasks as horizontal bars
    for (i, task) in ordered_tasks.iter().enumerate() {
        let row = i as i32;
        let y = chart_top + row * (bar_height + row_gap) + row_gap / 2;
        // Label
        out.push_str(&format!(
            "<text x=\"{x}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"12\" fill=\"#0f172a\">{txt}</text>",
            x = margin_x,
            ty = y + bar_height - 6,
            txt = escape_text(&task.name)
        ));
        let (bx, bw) = bar_geom(task);
        if let (Some(base_start), Some(base_duration)) =
            (task.baseline_start_day, task.baseline_duration_days)
        {
            let base_offset = base_start.saturating_sub(min_day);
            let base_x = chart_left + ((chart_w as u32 * base_offset) / total_days) as i32;
            let base_w = (((chart_w as u32) * base_duration.max(1)) / total_days).max(8) as i32;
            out.push_str(&format!(
                "<rect class=\"gantt-baseline\" data-gantt-baseline-start=\"{}\" data-gantt-baseline-duration=\"{}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"4\" rx=\"2\" ry=\"2\" fill=\"#64748b\" opacity=\"0.88\"/>",
                escape_text(&format_gantt_axis_label(base_start, min_day, true)),
                base_duration,
                x = base_x,
                y = y + bar_height + 3,
                w = base_w
            ));
        }
        let resource_load = format_resource_load_metadata(task);
        let critical_class = if task.is_critical {
            " gantt-critical"
        } else {
            ""
        };
        let fill = if task.is_critical {
            "#ef4444"
        } else {
            "#3b82f6"
        };
        let stroke = if task.is_critical {
            "#991b1b"
        } else {
            "#1e40af"
        };
        out.push_str(&format!(
            "<rect class=\"gantt-task{critical_class}\" data-gantt-start=\"{}\" data-gantt-workload=\"{}\" data-gantt-duration=\"{}\" data-gantt-resources=\"{}\" data-gantt-load=\"{}\" x=\"{bx}\" y=\"{y}\" width=\"{bw}\" height=\"{bh}\" rx=\"3\" ry=\"3\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
            escape_text(&format_gantt_axis_label(task.start_day, min_day, date_axis)),
            task.workload_days,
            task.duration_days,
            escape_text(&task.resources.join(", ")),
            escape_text(&resource_load),
            bh = bar_height
        ));
        if !task.resources.is_empty() {
            let resource_label = task.resources.join(", ");
            let pill_w = ((resource_label.len() as i32) * 7 + 14).min((bw - 6).max(0));
            if pill_w > 26 {
                out.push_str(&format!(
                    "<rect class=\"gantt-resource-pill\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"14\" rx=\"7\" ry=\"7\" fill=\"#dbeafe\" stroke=\"#93c5fd\" stroke-width=\"1\"/>",
                    x = bx + 4,
                    y = y + 3,
                    w = pill_w
                ));
                out.push_str(&format!(
                    "<text class=\"gantt-resource\" data-gantt-load=\"{load}\" x=\"{x}\" y=\"{y}\" font-family=\"monospace\" font-size=\"9\" fill=\"#1e40af\">{txt}</text>",
                    load = escape_text(&resource_load),
                    x = bx + 10,
                    y = y + 14,
                    txt = escape_text(&resource_label)
                ));
            }
            out.push_str(&format!(
                "<text x=\"{x}\" y=\"{y}\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"10\" fill=\"#1e40af\">{txt}</text>",
                x = chart_right - 6,
                y = y + bar_height - 6,
                txt = escape_text(&resource_label)
            ));
        }
    }

    // Render milestones as diamonds
    for (i, milestone) in document.milestones.iter().enumerate() {
        let row = task_count + i as i32;
        let y = chart_top + row * (bar_height + row_gap) + row_gap / 2;
        let cy = y + bar_height / 2;
        out.push_str(&format!(
            "<text x=\"{x}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"12\" fill=\"#0f172a\">{txt}</text>",
            x = margin_x,
            ty = y + bar_height - 6,
            txt = escape_text(&milestone.name)
        ));
        let cx = milestone_day
            .get(milestone.name.as_str())
            .map(|d| day_to_x(*d))
            .unwrap_or(chart_left + chart_w / 2);
        let r = (bar_height / 2) - 2;
        out.push_str(&format!(
            "<polygon class=\"gantt-milestone{}\" points=\"{x1},{y1} {x2},{y2} {x3},{y3} {x4},{y4}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            if milestone.is_critical { " gantt-critical" } else { "" },
            if milestone.is_critical { "#fb7185" } else { "#facc15" },
            if milestone.is_critical { "#9f1239" } else { "#854d0e" },
            x1 = cx,
            y1 = cy - r,
            x2 = cx + r,
            y2 = cy,
            x3 = cx,
            y3 = cy + r,
            x4 = cx - r,
            y4 = cy
        ));
    }

    for (i, separator) in document.separators.iter().enumerate() {
        let row = task_count + document.milestones.len() as i32 + i as i32;
        let y = chart_top + row * (bar_height + row_gap) + row_gap / 2 + bar_height / 2;
        let x = separator
            .target
            .as_deref()
            .and_then(|target| resolve_gantt_milestone_day(target, milestone_anchor, &task_bounds))
            .map(day_to_x)
            .unwrap_or(chart_left);
        out.push_str(&format!(
            "<line class=\"gantt-separator\" data-gantt-separator=\"{}\" x1=\"{x}\" y1=\"{}\" x2=\"{x}\" y2=\"{}\" stroke=\"#7c3aed\" stroke-width=\"1.4\" stroke-dasharray=\"6 4\"/>",
            escape_text(&separator.label),
            chart_top - header_h,
            chart_top + chart_h
        ));
        out.push_str(&format!(
            "<text class=\"gantt-separator-label\" x=\"{}\" y=\"{y}\" font-family=\"monospace\" font-size=\"11\" fill=\"#5b21b6\">{}</text>",
            (x + 6).min(chart_right - 80),
            escape_text(&separator.label)
        ));
    }

    // Render constraints as arrows between rows
    for constraint in &document.constraints {
        // Only draw if both endpoints exist
        let Some(&from_row) = row_index.get(&constraint.subject) else {
            // Render textual annotation when target row is missing
            continue;
        };
        // Some constraints are "starts <date>" with target being a date string, not a row.
        // We render row-to-row arrows for `requires`-style constraints.
        // The parser includes the keyword in `target` (e.g. "requires [Design]");
        // try to extract a bracketed target name.
        let Some((normalized_target, target_endpoint)) =
            parse_gantt_render_reference(&constraint.target)
        else {
            continue;
        };
        let to_row = row_index.get(&normalized_target).copied();
        if let Some(to_row) = to_row {
            let subject_endpoint = match constraint.kind.to_ascii_lowercase().as_str() {
                "ends" => "end",
                _ => "start",
            };
            let subject_y =
                chart_top + from_row * (bar_height + row_gap) + row_gap / 2 + bar_height / 2;
            let target_y =
                chart_top + to_row * (bar_height + row_gap) + row_gap / 2 + bar_height / 2;
            let from_task = document.tasks.iter().find(|t| t.name == constraint.subject);
            let to_task = document.tasks.iter().find(|t| t.name == normalized_target);
            let x2 = timeline_entity_x(
                from_task,
                document
                    .milestones
                    .iter()
                    .find(|milestone| milestone.name == constraint.subject),
                &milestone_day,
                subject_endpoint,
                &bar_geom,
                &day_to_x,
                chart_left,
            );
            let x1 = timeline_entity_x(
                to_task,
                document
                    .milestones
                    .iter()
                    .find(|milestone| milestone.name == normalized_target),
                &milestone_day,
                target_endpoint,
                &bar_geom,
                &day_to_x,
                chart_left + chart_w / 2,
            );
            let y1 = target_y;
            let y2 = subject_y;
            out.push_str(&format!(
                "<line class=\"gantt-dependency gantt-dependency-{}\" data-gantt-from=\"{}\" data-gantt-to=\"{}\" x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\" stroke=\"#64748b\" stroke-width=\"1.25\" stroke-dasharray=\"4 3\" marker-end=\"url(#gantt-arrow)\"/>",
                escape_text(&constraint.kind),
                escape_text(&normalized_target),
                escape_text(&constraint.subject)
            ));
        }
    }

    // Constraint arrow marker def
    out.push_str("<defs>");
    out.push_str(
        "<marker id=\"gantt-arrow\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" \
         markerWidth=\"8\" markerHeight=\"8\" orient=\"auto-start-reverse\">\
         <path d=\"M0,0 L10,5 L0,10 z\" fill=\"#64748b\"/>\
         </marker>",
    );
    out.push_str("</defs>");

    // Render textual constraint annotations beneath chart (start/requires with date strings)
    let mut note_y = chart_top + chart_h + 10;
    for constraint in &document.constraints {
        if row_index.contains_key(&constraint.target)
            || extract_bracketed_name(&constraint.target)
                .as_deref()
                .is_some_and(|target| row_index.contains_key(target))
        {
            continue;
        }
        out.push_str(&format!(
            "<text x=\"{x}\" y=\"{y}\" font-family=\"monospace\" font-size=\"11\" fill=\"#475569\">{s} {k} {t}</text>",
            x = margin_x,
            y = note_y,
            s = escape_text(&constraint.subject),
            k = escape_text(&constraint.kind),
            t = escape_text(&constraint.target)
        ));
        note_y += 14;
    }

    out.push_str("</svg>");
    out
}

fn extract_bracketed_name(target: &str) -> Option<String> {
    let start = target.find('[')?;
    let end = target.rfind(']')?;
    if end <= start + 1 {
        return None;
    }
    Some(target[start + 1..end].trim().to_string())
}

fn resource_lane_label(task: &TimelineTask) -> String {
    if task.resources.is_empty() {
        "Unassigned".to_string()
    } else {
        task.resources.join(", ")
    }
}

fn format_resource_load_metadata(task: &TimelineTask) -> String {
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

fn title_case_ascii(raw: &str) -> String {
    let mut chars = raw.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut out = String::new();
    out.push(first.to_ascii_uppercase());
    out.push_str(chars.as_str());
    out
}

fn parse_relative_day(raw: &str) -> Option<u32> {
    let t = raw.trim();
    let rest = t.strip_prefix("D+").or_else(|| t.strip_prefix("d+"))?;
    rest.trim().parse::<u32>().ok()
}

fn resolve_gantt_milestone_day(
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

fn parse_gantt_render_reference(target: &str) -> Option<(String, &'static str)> {
    let name = extract_bracketed_name(target)?;
    let lower = target.to_ascii_lowercase();
    let endpoint = if lower.contains("'s start") || lower.contains(" start") {
        "start"
    } else {
        "end"
    };
    Some((name, endpoint))
}

fn timeline_entity_x(
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

fn gantt_tick_offsets(total_days: u32, scale: Option<&str>) -> Vec<u32> {
    let step = match scale {
        Some("weekly") => 7,
        Some("monthly") => 30,
        Some("quarterly") => 90,
        Some("yearly") => 365,
        _ => 1,
    };
    let mut offsets = Vec::new();
    let mut offset = 0u32;
    while offset < total_days {
        offsets.push(offset);
        offset = offset.saturating_add(step);
        if offsets.len() >= 8 && offset < total_days {
            let remaining = total_days.saturating_sub(offset).max(1);
            offset = offset.saturating_add(remaining.div_ceil(8));
        }
    }
    if offsets.last().copied() != Some(total_days) {
        offsets.push(total_days);
    }
    offsets
}

fn format_gantt_axis_label(day: u32, min_day: u32, date_axis: bool) -> String {
    if date_axis {
        day_number_to_iso(day).unwrap_or_else(|| format!("D+{}", day.saturating_sub(min_day)))
    } else {
        format!("D+{}", day.saturating_sub(min_day))
    }
}

fn format_gantt_scale_axis_label(
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

fn format_month_label(year_month: &str) -> String {
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

fn format_quarter_label(iso: &str) -> Option<String> {
    let year = iso.get(0..4)?;
    let month = iso.get(5..7)?.parse::<u32>().ok()?;
    let quarter = month.saturating_sub(1) / 3 + 1;
    Some(format!("Q{quarter} {year}"))
}

fn is_gantt_closed_weekday_number(day: u32, closed_weekdays: &[String]) -> bool {
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

fn parse_iso_date_tuple(raw: &str) -> Option<(i32, i32, i32)> {
    let mut parts = raw.trim().split('-');
    let y = parts.next()?.parse::<i32>().ok()?;
    let m = parts.next()?.parse::<i32>().ok()?;
    let d = parts.next()?.parse::<i32>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((y, m, d))
}

fn parse_iso_date_day_number(raw: &str) -> Option<u32> {
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

fn day_number_to_iso(day: u32) -> Option<String> {
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

fn render_chronology_svg(document: &TimelineDocument) -> String {
    let width: i32 = 760;
    let margin_x: i32 = 32;
    let line_x: i32 = margin_x + 60;
    let event_gap: i32 = 56;
    let top_pad: i32 = 60;

    let title_h = document
        .title
        .as_deref()
        .map(|t| 8 + (t.lines().count() as i32) * 22)
        .unwrap_or(0);

    let total_events = document.chronology_events.len() as i32;
    let total_h = top_pad + title_h + total_events.max(1) * event_gap + 60;

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">",
        w = width,
        h = total_h
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");

    // Title
    let mut header_bottom = 28 + title_h;
    if let Some(title) = &document.title {
        let mut ty = 28;
        for line in title.lines() {
            out.push_str(&format!(
                "<text x=\"{x}\" y=\"{y}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\" fill=\"#0f172a\">{txt}</text>",
                x = margin_x,
                y = ty,
                txt = escape_text(line)
            ));
            ty += 22;
        }
    } else {
        out.push_str(&format!(
            "<text x=\"{x}\" y=\"28\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\" fill=\"#0f172a\">Chronology</text>",
            x = margin_x
        ));
        header_bottom = 36;
    }

    // Vertical line
    let line_top = header_bottom + 20;
    let line_bottom = line_top + total_events.max(1) * event_gap;
    out.push_str(&format!(
        "<line x1=\"{x}\" y1=\"{y1}\" x2=\"{x}\" y2=\"{y2}\" stroke=\"#94a3b8\" stroke-width=\"2\"/>",
        x = line_x,
        y1 = line_top,
        y2 = line_bottom
    ));

    // Events (sorted by ISO date when parsable)
    let mut events: Vec<&TimelineChronologyEvent> = document.chronology_events.iter().collect();
    events.sort_by_key(|e| parse_iso_date_tuple(&e.when).unwrap_or((i32::MAX, i32::MAX, i32::MAX)));
    for (i, event) in events.iter().enumerate() {
        let cy = line_top + (i as i32) * event_gap + event_gap / 2;
        let card_y = cy - 16;
        let card_x = line_x + 12;
        out.push_str(&format!(
            "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"28\" rx=\"4\" ry=\"4\" fill=\"{bg}\" stroke=\"#cbd5e1\" stroke-width=\"1\"/>",
            x = card_x,
            y = card_y,
            w = width - card_x - margin_x,
            bg = if i % 2 == 0 { "#ffffff" } else { "#f8fafc" }
        ));
        // Bullet circle
        out.push_str(&format!(
            "<circle cx=\"{cx}\" cy=\"{cy}\" r=\"6\" fill=\"#3b82f6\" stroke=\"#1e40af\" stroke-width=\"1.5\"/>",
            cx = line_x
        ));
        // Date on left
        out.push_str(&format!(
            "<text x=\"{x}\" y=\"{y}\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"11\" fill=\"#475569\">{txt}</text>",
            x = line_x - 14,
            y = cy + 4,
            txt = escape_text(&event.when)
        ));
        // Subject on right
        out.push_str(&format!(
            "<text x=\"{x}\" y=\"{y}\" font-family=\"monospace\" font-size=\"13\" fill=\"#0f172a\">{txt}</text>",
            x = line_x + 20,
            y = cy + 4,
            txt = escape_text(&event.subject)
        ));
    }

    out.push_str("</svg>");
    out
}
