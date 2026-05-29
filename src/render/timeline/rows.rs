use super::*;

pub(super) struct GanttRowsRenderContext<'a> {
    pub(super) out: &'a mut String,
    pub(super) document: &'a TimelineDocument,
    pub(super) ordered_tasks: &'a [&'a TimelineTask],
    pub(super) has_resource_lanes: bool,
    pub(super) margin_x: i32,
    pub(super) chart_left: i32,
    pub(super) chart_w: i32,
    pub(super) chart_right: i32,
    pub(super) chart_top: i32,
    pub(super) bar_height: i32,
    pub(super) row_gap: i32,
    pub(super) min_day: u32,
    pub(super) max_day_exclusive: u32,
    pub(super) total_days: u32,
    pub(super) date_axis: bool,
    pub(super) bar_geom: &'a dyn Fn(&TimelineTask) -> (i32, i32),
    pub(super) clamp_span_to_chart: &'a dyn Fn(i32, i32) -> (i32, i32),
    pub(super) day_to_x: &'a dyn Fn(u32) -> i32,
}

pub(super) fn write_gantt_task_rows(ctx: &mut GanttRowsRenderContext<'_>) {
    let out = &mut *ctx.out;
    let document = ctx.document;
    let ordered_tasks = ctx.ordered_tasks;
    let has_resource_lanes = ctx.has_resource_lanes;
    let margin_x = ctx.margin_x;
    let chart_left = ctx.chart_left;
    let chart_w = ctx.chart_w;
    let chart_right = ctx.chart_right;
    let chart_top = ctx.chart_top;
    let bar_height = ctx.bar_height;
    let row_gap = ctx.row_gap;
    let min_day = ctx.min_day;
    let max_day_exclusive = ctx.max_day_exclusive;
    let total_days = ctx.total_days;
    let date_axis = ctx.date_axis;
    let bar_geom = ctx.bar_geom;
    let clamp_span_to_chart = ctx.clamp_span_to_chart;
    let day_to_x = ctx.day_to_x;
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
        if let (Some(base_start), Some(base_duration)) =
            (task.baseline_start_day, task.baseline_duration_days)
        {
            let base_end = base_start.saturating_add(base_duration.max(1));
            if base_end > min_day && base_start < max_day_exclusive {
                let base_start = base_start.max(min_day);
                let base_end = base_end.min(max_day_exclusive);
                let base_offset = base_start.saturating_sub(min_day);
                let base_x = chart_left + ((chart_w as u32 * base_offset) / total_days) as i32;
                let base_w = (((chart_w as u32) * base_end.saturating_sub(base_start).max(1))
                    / total_days)
                    .max(8) as i32;
                let (base_x, base_w) = clamp_span_to_chart(base_x, base_w);
                out.push_str(&format!(
                    "<rect class=\"gantt-baseline\" data-gantt-baseline-start=\"{start}\" data-gantt-baseline-duration=\"{dur}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"4\" rx=\"2\" ry=\"2\" fill=\"#64748b\" opacity=\"0.88\"/>",
                    start = escape_text(&format_gantt_axis_label(base_start, min_day, true)),
                    dur = base_duration,
                    x = base_x,
                    y = y + bar_height + 3,
                    w = base_w
                ));
            }
        }
        let (bx, bw) = bar_geom(task);
        if bw == 0 {
            continue;
        }
        let resource_load = format_resource_load_metadata(task);
        let critical_class = if task.is_critical {
            " gantt-critical"
        } else {
            ""
        };
        let fill = if let Some(fill) = &task.fill_color {
            fill.as_str()
        } else if task.is_critical {
            "#ef4444"
        } else {
            "#3b82f6"
        };
        let stroke = if let Some(stroke) = &task.stroke_color {
            stroke.as_str()
        } else if task.is_critical {
            "#991b1b"
        } else {
            "#1e40af"
        };
        let deleted_attrs = if task.is_deleted {
            " opacity=\"0.42\" stroke-dasharray=\"4 3\""
        } else {
            ""
        };
        if let Some(href) = &task.hyperlink {
            out.push_str(&format!(
                "<a class=\"gantt-task-link\" xlink:href=\"{}\" xmlns:xlink=\"http://www.w3.org/1999/xlink\">",
                escape_text(href)
            ));
        }
        let link_attr = task
            .hyperlink
            .as_deref()
            .map(|href| format!(" data-gantt-link=\"{}\"", escape_text(href)))
            .unwrap_or_default();
        out.push_str(&format!(
            "<rect class=\"gantt-task{critical_class}\" data-gantt-start=\"{start}\" data-gantt-workload=\"{wl}\" data-gantt-duration=\"{dur}\" data-gantt-resources=\"{res}\" data-gantt-load=\"{load}\" data-gantt-completion=\"{completion}\"{link_attr} data-gantt-deleted=\"{deleted}\" x=\"{bx}\" y=\"{y}\" width=\"{bw}\" height=\"{bh}\" rx=\"3\" ry=\"3\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1\"{deleted_attrs}/>",
            start = escape_text(&format_gantt_axis_label(task.start_day, min_day, date_axis)),
            wl = task.workload_days,
            dur = task.duration_days,
            res = escape_text(&task.resources.join(", ")),
            load = escape_text(&resource_load),
            completion = task.completion_percent.unwrap_or(0),
            deleted = task.is_deleted,
            bh = bar_height
        ));
        for pause in gantt_task_pause_segments(
            task,
            &document.resource_off_ranges,
            min_day,
            max_day_exclusive,
        ) {
            let x = day_to_x(pause.day);
            let w = (day_to_x(pause.day.saturating_add(1)) - x).max(2);
            out.push_str(&format!(
                "<rect class=\"{class}\" data-gantt-pause=\"{label}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"{fill}\" opacity=\"0.55\"/>",
                class = pause.class_name,
                label = escape_text(&pause.label),
                y = y + 1,
                h = bar_height - 2,
                fill = pause.fill
            ));
        }
        if let Some(percent) = task.completion_percent {
            let complete_w = (bw * percent.min(100) as i32) / 100;
            if complete_w > 0 {
                out.push_str(&format!(
                    "<rect class=\"gantt-task-completion\" x=\"{bx}\" y=\"{y}\" width=\"{complete_w}\" height=\"{bh}\" rx=\"3\" ry=\"3\" fill=\"#0f172a\" opacity=\"0.22\"/>",
                    bh = bar_height
                ));
            }
        }
        if task.is_deleted {
            out.push_str(&format!(
                "<line class=\"gantt-task-deleted\" x1=\"{bx}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#7f1d1d\" stroke-width=\"1.5\"/>",
                y + bar_height / 2,
                bx + bw,
                y + bar_height / 2
            ));
        }
        if task.hyperlink.is_some() {
            out.push_str("</a>");
        }
        if !document.hide_resource_names && !task.resources.is_empty() {
            let resource_label = task.resources.join(", ");
            // Use the shared text-width helper; resource labels are ASCII in practice.
            let pill_w =
                (crate::render_core::text_metrics::estimate_text_width_default(&resource_label)
                    + 14)
                    .min((bw - 6).max(0));
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
}
