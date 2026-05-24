use super::*;

pub(super) fn render_gantt_svg(document: &TimelineDocument) -> String {
    // Extra right padding so the last date-header label (≤10 chars × ~7px + gap) is
    // never clipped by the canvas edge (#485).  80px covers "YYYY-MM-DD" comfortably.
    let scale_options = parse_gantt_scale_render_options(&document.scale_options);
    let right_pad: i32 = 80;
    let margin_x: i32 = 32;
    let label_col_w: i32 = 160;
    let bar_height: i32 = 20;
    let row_gap: i32 = 14;
    let header_h: i32 = 28;
    let chart_left: i32 = margin_x + label_col_w + 12;
    let base_width: i32 = 800 + right_pad;
    let base_chart_right: i32 = base_width - margin_x - right_pad;
    let base_chart_w: i32 = base_chart_right - chart_left;
    let chart_w: i32 = ((base_chart_w as f32) * scale_options.zoom).round() as i32;
    let chart_right: i32 = chart_left + chart_w;
    let width: i32 = chart_right + margin_x + right_pad;

    let title_h = document
        .title
        .as_deref()
        .map(|t| 8 + (t.lines().count() as i32) * 22)
        .unwrap_or(0);
    let has_calendar_notes = !document.closed_weekdays.is_empty()
        || !document.closed_ranges.is_empty()
        || !document.open_ranges.is_empty()
        || !document.day_markers.is_empty()
        || !document.resource_off_ranges.is_empty();
    let calendar_h = if !has_calendar_notes { 0 } else { 18 };
    let scale_h = if document.scale.is_some() { 18 } else { 0 };

    let row_count =
        (document.tasks.len() + document.milestones.len() + document.separators.len()) as i32;
    let chart_top = 40 + title_h + calendar_h + scale_h + header_h;
    let chart_h = (row_count.max(1)) * (bar_height + row_gap) + 20;
    let annotation_h = if document.notes.is_empty() {
        0
    } else {
        (document.constraints.len() as i32) * 14 + (document.notes.len() as i32) * 48
    };
    let total_h = chart_top + chart_h + 50 + annotation_h;

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">",
        w = width,
        h = total_h
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    if let Some(scale) = &document.scale {
        out.push_str(&format!(
            "<metadata data-gantt-scale=\"{}\" data-gantt-scale-options=\"{}\" data-gantt-zoom=\"{}\" data-gantt-calendar-date=\"{}\" data-gantt-week-numbering-start=\"{}\"/>",
            escape_text(scale),
            escape_text(&document.scale_options.join("; ")),
            format_gantt_zoom(scale_options.zoom),
            scale_options.calendar_date,
            scale_options
                .week_numbering_start
                .map(|value| value.to_string())
                .unwrap_or_default()
        ));
    }
    if let (Some(start), Some(end)) = (&document.print_start, &document.print_end) {
        out.push_str(&format!(
            "<metadata data-gantt-print-start=\"{}\" data-gantt-print-end=\"{}\"/>",
            escape_text(start),
            escape_text(end)
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
        "<metadata data-gantt-resource-count=\"{resource_count}\" data-gantt-separator-count=\"{}\" data-gantt-hide-footbox=\"{}\" data-gantt-hide-resource-names=\"{}\" data-gantt-hide-resource-footbox=\"{}\"/>",
        document.separators.len(),
        document.hide_footbox,
        document.hide_resource_names,
        document.hide_resource_footbox
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
        labels.extend(document.day_markers.iter().filter_map(|marker| {
            marker.label.as_ref().map(|label| {
                if marker.start_date == marker.end_date {
                    format!("{} named {label}", marker.start_date)
                } else {
                    format!("{} to {} named {label}", marker.start_date, marker.end_date)
                }
            })
        }));
        labels.extend(document.resource_off_ranges.iter().map(|range| {
            if range.start_date == range.end_date {
                format!("{} off {}", range.resource, range.start_date)
            } else {
                format!(
                    "{} off {} to {}",
                    range.resource, range.start_date, range.end_date
                )
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
        let options = if document.scale_options.is_empty() {
            String::new()
        } else {
            format!(" ({})", document.scale_options.join("; "))
        };
        out.push_str(&format!(
            "<text class=\"gantt-scale\" x=\"{x}\" y=\"{y}\" font-family=\"monospace\" font-size=\"11\" fill=\"#334155\">Scale: {scale}{options}</text>",
            x = margin_x,
            y = 42 + title_h + calendar_h,
            scale = escape_text(scale),
            options = escape_text(&options)
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
        row_index.insert(gantt_task_key(task), row_counter);
        row_index.entry(task.name.clone()).or_insert(row_counter);
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
                gantt_task_key_ref(t),
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

    let content_min_day = document
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
    let mut visual_anchor_days: Vec<u32> = document
        .tasks
        .iter()
        .map(|task| task.start_day)
        .chain(milestone_day.values().copied())
        .chain(project_end_day)
        .collect();
    visual_anchor_days.sort_unstable();
    visual_anchor_days.dedup();
    let visual_task_bounds: std::collections::BTreeMap<&str, (u32, u32)> = document
        .tasks
        .iter()
        .map(|task| {
            let default_end = task.start_day.saturating_add(task.duration_days.max(1));
            let visual_end = if should_expand_gantt_task_visual_span(task) {
                visual_anchor_days
                    .iter()
                    .copied()
                    .find(|day| *day > task.start_day)
                    .map(|day| day.max(default_end))
                    .unwrap_or(default_end)
            } else {
                default_end
            };
            (gantt_task_key_ref(task), (task.start_day, visual_end))
        })
        .collect();
    let content_max_day_exclusive = document
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
        .unwrap_or(content_min_day.saturating_add(1));
    let min_day = document.print_start_day.unwrap_or(content_min_day);
    let max_day_exclusive = document
        .print_end_day
        .map(|day| day.saturating_add(1))
        .unwrap_or(content_max_day_exclusive)
        .max(min_day.saturating_add(1));
    let total_days = max_day_exclusive.saturating_sub(min_day).max(1);
    let date_axis = document.project_start_day.is_some()
        || document.print_start_day.is_some()
        || content_min_day > 366;
    let tick_offsets = gantt_tick_offsets_for_width(total_days, document.scale.as_deref(), chart_w);

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
        // Clamp label start so it never extends past the canvas right edge (#485).
        // A "YYYY-MM-DD" label is ~70 px wide; shift left if the tick is near the right.
        let label_w = 70_i32;
        let tx = (x + 6).min(width - label_w);
        out.push_str(&format!(
            "<text class=\"gantt-scale-tick\" data-gantt-tick-day=\"{day}\" x=\"{tx}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"11\" fill=\"#475569\">{label}</text>",
            day = escape_text(&format_gantt_axis_label(
                min_day.saturating_add(day_offset),
                min_day,
                true
            )),
            ty = chart_top - 10,
            label = escape_text(&format_gantt_scale_axis_label(
                min_day.saturating_add(day_offset),
                min_day,
                date_axis,
                document.scale.as_deref(),
                &scale_options
            ))
        ));
    }

    let clamp_span_to_chart = |x: i32, width: i32| -> (i32, i32) {
        let clamped_width = width.max(1).min(chart_w.max(1));
        let max_x = chart_right - clamped_width;
        (x.clamp(chart_left, max_x), clamped_width)
    };
    let bar_geom = |task: &TimelineTask| -> (i32, i32) {
        let (start_day, end_day) = visual_task_bounds
            .get(gantt_task_key_ref(task))
            .copied()
            .unwrap_or((
                task.start_day,
                task.start_day.saturating_add(task.duration_days.max(1)),
            ));
        if end_day <= min_day || start_day >= max_day_exclusive {
            return (chart_left, 0);
        }
        let start_day = start_day.max(min_day);
        let end_day = end_day.min(max_day_exclusive);
        let start_offset = start_day.saturating_sub(min_day);
        let bx = chart_left + ((chart_w as u32 * start_offset) / total_days) as i32;
        let span_days = end_day.saturating_sub(start_day).max(1);
        let bw = (((chart_w as u32) * span_days) / total_days).max(8) as i32;
        clamp_span_to_chart(bx, bw)
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
            "<rect class=\"gantt-open-range\" data-gantt-open=\"{open}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"#dcfce7\" opacity=\"0.62\"/>",
            open = escape_text(&format!(
                "{} to {}",
                format_gantt_axis_label(start, min_day, true),
                format_gantt_axis_label(end.saturating_sub(1), min_day, true)
            )),
            y = chart_top,
            h = chart_h
        ));
    }
    for marker in &document.day_markers {
        if marker.end_day < min_day || marker.start_day > max_day_exclusive {
            continue;
        }
        let start = marker.start_day.max(min_day);
        let end = marker.end_day.saturating_add(1).min(max_day_exclusive);
        let x = day_to_x(start);
        let w = (day_to_x(end) - x).max(2);
        if let Some(color) = &marker.color {
            out.push_str(&format!(
                "<rect class=\"gantt-day-marker\" data-gantt-day-marker=\"{}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"{}\" opacity=\"0.48\"/>",
                escape_text(&marker.label.clone().unwrap_or_else(|| format!(
                    "{} to {}",
                    marker.start_date, marker.end_date
                ))),
                escape_text(color),
                y = chart_top,
                h = chart_h
            ));
        }
        if let Some(label) = &marker.label {
            out.push_str(&format!(
                "<text class=\"gantt-day-marker-label\" x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" fill=\"#334155\">{}</text>",
                (x + 4).min(chart_right - 100),
                chart_top + 12,
                escape_text(label)
            ));
        }
    }
    if !document.closed_weekdays.is_empty() {
        let mut day = min_day;
        while day < max_day_exclusive {
            if is_gantt_closed_weekday_number(day, &document.closed_weekdays)
                && !document
                    .open_ranges
                    .iter()
                    .any(|range| (range.start_day..=range.end_day).contains(&day))
            {
                let x = day_to_x(day);
                let w = (day_to_x(day.saturating_add(1)) - x).max(2);
                out.push_str(&format!(
                    "<rect class=\"gantt-closed-weekday\" data-gantt-day=\"{day}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"#f8fafc\" opacity=\"0.82\"/>",
                    day = escape_text(&format_gantt_axis_label(day, min_day, date_axis)),
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
    write_gantt_task_rows(&mut GanttRowsRenderContext {
        out: &mut out,
        document,
        ordered_tasks: &ordered_tasks,
        has_resource_lanes,
        margin_x,
        chart_left,
        chart_w,
        chart_right,
        chart_top,
        bar_height,
        row_gap,
        min_day,
        max_day_exclusive,
        total_days,
        date_axis,
        bar_geom: &bar_geom,
        clamp_span_to_chart: &clamp_span_to_chart,
        day_to_x: &day_to_x,
    });

    write_gantt_milestones_links_and_annotations(&mut GanttDetailsRenderContext {
        out: &mut out,
        document,
        row_index: &row_index,
        task_bounds: &task_bounds,
        milestone_day: &milestone_day,
        task_count,
        margin_x,
        chart_left,
        chart_w,
        chart_right,
        chart_top,
        chart_h,
        header_h,
        bar_height,
        row_gap,
        milestone_anchor,
        min_day,
        bar_geom: &bar_geom,
        day_to_x: &day_to_x,
    });

    out.push_str("</svg>");
    out
}
