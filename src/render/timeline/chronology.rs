use super::*;

struct ChronologyRenderEvent<'a> {
    event: &'a TimelineChronologyEvent,
    start_day: Option<u32>,
    end_day: Option<u32>,
}

pub(super) fn render_chronology_svg(document: &TimelineDocument) -> String {
    let width: i32 = 900;
    let margin_x: i32 = 36;
    let axis_x: i32 = 220;
    let card_x: i32 = 285;
    let card_w: i32 = width - card_x - margin_x;
    let row_gap: i32 = 58;

    let title_h = document
        .title
        .as_deref()
        .map(|t| 8 + (t.lines().count() as i32) * 22)
        .unwrap_or(0);
    let axis_top = 72 + title_h;

    let mut events: Vec<ChronologyRenderEvent<'_>> = document
        .chronology_events
        .iter()
        .map(|event| ChronologyRenderEvent {
            start_day: parse_iso_date_day_number(&event.when),
            end_day: event.end.as_deref().and_then(parse_iso_date_day_number),
            event,
        })
        .collect();
    events.sort_by(|a, b| {
        (
            a.start_day.unwrap_or(u32::MAX),
            a.end_day.unwrap_or(a.start_day.unwrap_or(u32::MAX)),
            a.event.subject.as_str(),
        )
            .cmp(&(
                b.start_day.unwrap_or(u32::MAX),
                b.end_day.unwrap_or(b.start_day.unwrap_or(u32::MAX)),
                b.event.subject.as_str(),
            ))
    });

    let dated_days = events
        .iter()
        .flat_map(|entry| [entry.start_day, entry.end_day])
        .flatten()
        .collect::<Vec<_>>();
    let min_day = dated_days.iter().copied().min();
    let max_day = dated_days.iter().copied().max();
    let total_rows = events.len().max(1) as i32;
    let axis_h = (total_rows * row_gap).max(300);
    let total_h = axis_top + axis_h + 72;
    let axis_bottom = axis_top + axis_h;

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\" data-chronology-renderer=\"vertical-axis\">",
        w = width,
        h = total_h
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"#f8fafc\"/>");
    out.push_str(&format!(
        "<rect x=\"{x}\" y=\"0\" width=\"{w}\" height=\"100%\" fill=\"#ffffff\"/>",
        x = axis_x - 34,
        w = width - axis_x + 34
    ));

    if let Some(title) = &document.title {
        let mut ty = 30;
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
            "<text x=\"{x}\" y=\"32\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\" fill=\"#0f172a\">Chronology</text>",
            x = margin_x
        ));
    }

    out.push_str("<defs><filter id=\"chronology-card-shadow\" x=\"-4%\" y=\"-20%\" width=\"108%\" height=\"140%\"><feDropShadow dx=\"0\" dy=\"1\" stdDeviation=\"1.2\" flood-color=\"#0f172a\" flood-opacity=\"0.12\"/></filter></defs>");
    out.push_str(&format!(
        "<line class=\"chronology-axis\" x1=\"{x}\" y1=\"{y1}\" x2=\"{x}\" y2=\"{y2}\" stroke=\"#334155\" stroke-width=\"3\" stroke-linecap=\"round\"/>",
        x = axis_x,
        y1 = axis_top,
        y2 = axis_bottom
    ));
    if let (Some(min_day), Some(max_day)) = (min_day, max_day) {
        if let Some(start) = day_number_to_iso(min_day) {
            out.push_str(&format!(
                "<text x=\"{x}\" y=\"{y}\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"11\" fill=\"#64748b\">{txt}</text>",
                x = axis_x - 70,
                y = axis_top + 4,
                txt = escape_text(&start)
            ));
        }
        if min_day != max_day {
            if let Some(end) = day_number_to_iso(max_day) {
                out.push_str(&format!(
                    "<text x=\"{x}\" y=\"{y}\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"11\" fill=\"#64748b\">{txt}</text>",
                    x = axis_x - 70,
                    y = axis_bottom + 4,
                    txt = escape_text(&end)
                ));
            }
        }
    }

    for entry in &events {
        if let (Some(start), Some(end)) = (entry.start_day, entry.end_day) {
            let y1 = chronology_y_for_day(start.min(end), min_day, max_day, axis_top, axis_h);
            let y2 = chronology_y_for_day(start.max(end), min_day, max_day, axis_top, axis_h);
            let color = entry.event.color.as_deref().unwrap_or("#38bdf8");
            if entry.event.bracket {
                out.push_str(&format!(
                    "<path class=\"chronology-bracket\" d=\"M{x1},{y1} h22 M{x1},{y2} h22 M{x1},{y1} V{y2}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"3\" stroke-linecap=\"round\" data-chronology-label=\"{label}\"/>",
                    x1 = axis_x - 42,
                    y1 = y1,
                    y2 = y2,
                    color = escape_text(color),
                    label = escape_text(&entry.event.subject)
                ));
            } else {
                out.push_str(&format!(
                    "<rect class=\"chronology-era\" x=\"{x}\" y=\"{y}\" width=\"28\" height=\"{h}\" rx=\"8\" ry=\"8\" fill=\"{color}\" fill-opacity=\"0.28\" stroke=\"{color}\" stroke-opacity=\"0.75\" data-chronology-label=\"{label}\"/>",
                    x = axis_x - 14,
                    y = y1,
                    h = (y2 - y1).max(12),
                    color = escape_text(color),
                    label = escape_text(&entry.event.subject)
                ));
            }
        }
    }

    let mut last_label_y = axis_top - row_gap;
    for (idx, entry) in events.iter().enumerate() {
        let start_y = entry
            .start_day
            .map(|day| chronology_y_for_day(day, min_day, max_day, axis_top, axis_h))
            .unwrap_or(axis_top + idx as i32 * row_gap + row_gap / 2);
        let label_y = start_y.max(last_label_y + row_gap);
        last_label_y = label_y;
        let card_y = label_y - 22;
        let dot_color = entry.event.color.as_deref().unwrap_or("#2563eb");
        let date_label = chronology_date_label(entry.event);
        out.push_str(&format!(
            "<line class=\"chronology-connector\" x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\" stroke=\"#cbd5e1\" stroke-width=\"1.5\"/>",
            x1 = axis_x,
            y1 = start_y,
            x2 = card_x - 14,
            y2 = label_y
        ));
        out.push_str(&format!(
            "<circle class=\"chronology-marker\" cx=\"{cx}\" cy=\"{cy}\" r=\"7\" fill=\"{color}\" stroke=\"#0f172a\" stroke-width=\"1.5\"/>",
            cx = axis_x,
            cy = start_y,
            color = escape_text(dot_color)
        ));
        out.push_str(&format!(
            "<rect class=\"chronology-event-card\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"44\" rx=\"6\" ry=\"6\" fill=\"#ffffff\" stroke=\"#cbd5e1\" stroke-width=\"1\" filter=\"url(#chronology-card-shadow)\" data-chronology-index=\"{idx}\"/>",
            x = card_x,
            y = card_y,
            w = card_w
        ));
        out.push_str(&format!(
            "<rect class=\"chronology-event-accent\" x=\"{x}\" y=\"{y}\" width=\"5\" height=\"44\" rx=\"2\" ry=\"2\" fill=\"{color}\"/>",
            x = card_x,
            y = card_y,
            color = escape_text(dot_color)
        ));
        out.push_str(&format!(
            "<text x=\"{x}\" y=\"{y}\" font-family=\"monospace\" font-size=\"11\" fill=\"#64748b\">{txt}</text>",
            x = card_x + 16,
            y = label_y - 4,
            txt = escape_text(&date_label)
        ));
        out.push_str(&format!(
            "<text x=\"{x}\" y=\"{y}\" font-family=\"monospace\" font-size=\"14\" font-weight=\"600\" fill=\"#0f172a\">{txt}</text>",
            x = card_x + 16,
            y = label_y + 14,
            txt = escape_text(&entry.event.subject)
        ));
    }

    out.push_str("</svg>");
    out
}

fn chronology_y_for_day(
    day: u32,
    min_day: Option<u32>,
    max_day: Option<u32>,
    axis_top: i32,
    axis_h: i32,
) -> i32 {
    let (Some(min_day), Some(max_day)) = (min_day, max_day) else {
        return axis_top;
    };
    if min_day == max_day {
        return axis_top + axis_h / 2;
    }
    let span = u64::from(max_day - min_day);
    let offset = u64::from(day.saturating_sub(min_day));
    axis_top + ((offset * axis_h as u64) / span) as i32
}

fn chronology_date_label(event: &TimelineChronologyEvent) -> String {
    event
        .end
        .as_deref()
        .map(|end| format!("{} to {end}", event.when))
        .unwrap_or_else(|| event.when.clone())
}
