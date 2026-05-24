use super::*;

pub(super) struct GanttDetailsRenderContext<'a> {
    pub(super) out: &'a mut String,
    pub(super) document: &'a TimelineDocument,
    pub(super) row_index: &'a BTreeMap<String, i32>,
    pub(super) task_bounds: &'a BTreeMap<&'a str, (u32, u32)>,
    pub(super) milestone_day: &'a BTreeMap<&'a str, u32>,
    pub(super) task_count: i32,
    pub(super) margin_x: i32,
    pub(super) chart_left: i32,
    pub(super) chart_w: i32,
    pub(super) chart_right: i32,
    pub(super) chart_top: i32,
    pub(super) chart_h: i32,
    pub(super) header_h: i32,
    pub(super) bar_height: i32,
    pub(super) row_gap: i32,
    pub(super) milestone_anchor: u32,
    pub(super) min_day: u32,
    pub(super) bar_geom: &'a dyn Fn(&TimelineTask) -> (i32, i32),
    pub(super) day_to_x: &'a dyn Fn(u32) -> i32,
}

pub(super) fn write_gantt_milestones_links_and_annotations(
    ctx: &mut GanttDetailsRenderContext<'_>,
) {
    let out = &mut *ctx.out;
    let document = ctx.document;
    let row_index = ctx.row_index;
    let task_bounds = ctx.task_bounds;
    let milestone_day = ctx.milestone_day;
    let task_count = ctx.task_count;
    let margin_x = ctx.margin_x;
    let chart_left = ctx.chart_left;
    let chart_w = ctx.chart_w;
    let chart_right = ctx.chart_right;
    let chart_top = ctx.chart_top;
    let chart_h = ctx.chart_h;
    let header_h = ctx.header_h;
    let bar_height = ctx.bar_height;
    let row_gap = ctx.row_gap;
    let milestone_anchor = ctx.milestone_anchor;
    let min_day = ctx.min_day;
    let bar_geom = ctx.bar_geom;
    let day_to_x = ctx.day_to_x;
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
        let cx = cx.clamp(chart_left + r, chart_right - r);
        out.push_str(&format!(
            "<polygon class=\"gantt-milestone{crit}\" points=\"{x1},{y1} {x2},{y2} {x3},{y3} {x4},{y4}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
            crit = if milestone.is_critical { " gantt-critical" } else { "" },
            fill = if milestone.is_critical { "#fb7185" } else { "#facc15" },
            stroke = if milestone.is_critical { "#9f1239" } else { "#854d0e" },
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
            .and_then(|target| resolve_gantt_milestone_day(target, milestone_anchor, task_bounds))
            .map(day_to_x)
            .unwrap_or(chart_left);
        out.push_str(&format!(
            "<line class=\"gantt-separator\" data-gantt-separator=\"{sep}\" x1=\"{x}\" y1=\"{y1}\" x2=\"{x}\" y2=\"{y2}\" stroke=\"#7c3aed\" stroke-width=\"1.4\" stroke-dasharray=\"6 4\"/>",
            sep = escape_text(&separator.label),
            y1 = chart_top - header_h,
            y2 = chart_top + chart_h
        ));
        out.push_str(&format!(
            "<text class=\"gantt-separator-label\" x=\"{x}\" y=\"{y}\" font-family=\"monospace\" font-size=\"11\" fill=\"#5b21b6\">{label}</text>",
            x = (x + 6).min(chart_right - 80),
            label = escape_text(&separator.label)
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
            let from_task = document
                .tasks
                .iter()
                .find(|t| gantt_task_matches(t, &constraint.subject));
            let to_task = document
                .tasks
                .iter()
                .find(|t| gantt_task_matches(t, &normalized_target));
            let x2 = timeline_entity_x(
                from_task,
                document
                    .milestones
                    .iter()
                    .find(|milestone| milestone.name == constraint.subject),
                milestone_day,
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
                milestone_day,
                target_endpoint,
                &bar_geom,
                &day_to_x,
                chart_left + chart_w / 2,
            );
            let y1 = target_y;
            let y2 = subject_y;
            out.push_str(&format!(
                "<line class=\"gantt-dependency gantt-dependency-{kind}\" data-gantt-from=\"{from}\" data-gantt-to=\"{to}\" x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\" stroke=\"#64748b\" stroke-width=\"1.25\" stroke-dasharray=\"4 3\" marker-end=\"url(#gantt-arrow)\"/>",
                kind = escape_text(&constraint.kind),
                from = escape_text(&normalized_target),
                to = escape_text(&constraint.subject)
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
    for note in &document.notes {
        let text = note.text.lines().collect::<Vec<_>>();
        let h = 24 + (text.len().max(1) as i32 - 1) * 13;
        let target_label = note
            .target
            .as_deref()
            .map(|target| format!(" [{target}]"))
            .unwrap_or_default();
        out.push_str(&format!(
            "<rect class=\"gantt-note\" data-gantt-note-position=\"{}\" data-gantt-note-target=\"{}\" x=\"{x}\" y=\"{y}\" width=\"360\" height=\"{h}\" rx=\"3\" ry=\"3\" fill=\"#fff7ed\" stroke=\"#fdba74\" stroke-width=\"1\"/>",
            escape_text(&note.position),
            escape_text(note.target.as_deref().unwrap_or("")),
            x = margin_x,
            y = note_y
        ));
        for (line_idx, line) in text.iter().enumerate() {
            out.push_str(&format!(
                "<text class=\"gantt-note-text\" x=\"{x}\" y=\"{y}\" font-family=\"monospace\" font-size=\"11\" fill=\"#7c2d12\">{}{}</text>",
                escape_text(line),
                escape_text(if line_idx == 0 { &target_label } else { "" }),
                x = margin_x + 8,
                y = note_y + 16 + line_idx as i32 * 13
            ));
        }
        note_y += h + 8;
    }

    // Named-date markers: vertical dashed line + rotated label above the chart axis.
    for named in &document.named_dates {
        if named.day < min_day {
            continue;
        }
        let x = day_to_x(named.day);
        if x < chart_left || x > chart_right {
            continue;
        }
        out.push_str(&format!(
            "<line class=\"gantt-named-date\" data-gantt-date=\"{date}\" x1=\"{x}\" y1=\"{y1}\" x2=\"{x}\" y2=\"{y2}\" stroke=\"#b45309\" stroke-width=\"1.2\" stroke-dasharray=\"4 3\"/>",
            date = escape_text(&named.date),
            y1 = chart_top - header_h,
            y2 = chart_top + chart_h
        ));
        out.push_str(&format!(
            "<text class=\"gantt-named-date-label\" x=\"{lx}\" y=\"{ly}\" transform=\"rotate(-45,{lx},{ly})\" font-family=\"monospace\" font-size=\"10\" fill=\"#92400e\">{label}</text>",
            lx = x + 3,
            ly = chart_top - header_h + 2,
            label = escape_text(&named.label)
        ));
    }
}
