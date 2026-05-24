use super::*;

pub(super) fn render_chronology_svg(document: &TimelineDocument) -> String {
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
