use super::model::{TimingLayout, TimingModel};
use super::*;

pub(super) fn render_timing_axis(
    out: &mut String,
    model: &TimingModel<'_>,
    layout: &TimingLayout,
    style: &crate::theme::TimingStyle,
) {
    if !model.hide_time_axis {
        out.push_str(&format!(
            "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            escape_text(&style.signal_background_color),
            escape_text(&style.grid_color),
            x = layout.left_pad,
            y = layout.axis_top,
            w = layout.axis_panel_w,
            h = layout.axis_h
        ));
    }

    for range in &model.ranges {
        let x1 = layout.time_to_x(range.start.min(range.end));
        let x2 = layout.time_to_x(range.start.max(range.end));
        let w = (x2 - x1).max(2);
        let (fill, stroke, text_fill) = if let Some(clr) = range.fill_color.as_deref() {
            (clr, clr, "#0f172a")
        } else {
            ("#fde68a", "#f59e0b", "#92400e")
        };
        out.push_str(&format!(
            "<rect class=\"timing-range\" x=\"{x1}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"{fill}\" opacity=\"0.45\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
            y = layout.axis_top,
            h = layout.axis_h_effective + layout.rows_h()
        ));
        if !model.hide_time_axis {
            out.push_str(&format!(
                "<text class=\"timing-range-label\" x=\"{x}\" y=\"{y}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"{text_fill}\">{}</text>",
                escape_text(&range.label),
                x = x1 + w / 2,
                y = layout.axis_top + layout.axis_h - 14
            ));
        }
    }

    let mut manual_tick_values: Vec<i64> = model
        .events
        .iter()
        .filter(|e| e.alias.is_some())
        .filter_map(|e| super::model::timing_time_value(&e.name))
        .collect();
    manual_tick_values.sort();
    manual_tick_values.dedup();

    for &t in &model.time_vals {
        let tx = layout.time_to_x(t);
        out.push_str(&format!(
            "<line x1=\"{tx}\" y1=\"{y1}\" x2=\"{tx}\" y2=\"{y2}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"4 3\"/>",
            escape_text(&style.grid_color),
            y1 = layout.signals_top,
            y2 = layout.signals_top + layout.rows_h()
        ));
        if !model.hide_time_axis {
            out.push_str(&format!(
                "<line x1=\"{tx}\" y1=\"{y1}\" x2=\"{tx}\" y2=\"{y2}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                escape_text(&style.axis_color),
                y1 = layout.axis_top + layout.axis_h - 8,
                y2 = layout.axis_top + layout.axis_h
            ));
            if !model.manual_time_axis || manual_tick_values.contains(&t) {
                let tick_label = model
                    .time_labels
                    .get(&t)
                    .cloned()
                    .unwrap_or_else(|| format!("@{t}"));
                out.push_str(&format!(
                    "<text class=\"timing-tick\" x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
                    escape_text(&style.font_color),
                    escape_text(&tick_label),
                    ty = layout.axis_top + 20
                ));
            }
        }
    }

    for (t, note) in &model.global_events {
        let tx = layout.time_to_x(*t);
        out.push_str(&format!(
            "<circle cx=\"{tx}\" cy=\"{cy}\" r=\"3\" fill=\"{}\"/>",
            escape_text(&style.arrow_color),
            cy = layout.axis_top + 8
        ));
        out.push_str(&format!(
            "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"{}\">{}</text>",
            escape_text(&style.font_color),
            escape_text(note),
            ty = layout.axis_top + 10
        ));
    }

    if !model.hide_time_axis {
        for w in model.time_vals.windows(2) {
            let mid = (w[0] + w[1]) / 2;
            let mx = layout.time_to_x(mid);
            out.push_str(&format!(
                "<line x1=\"{mx}\" y1=\"{y1}\" x2=\"{mx}\" y2=\"{y2}\" stroke=\"{}\" stroke-width=\"0.75\"/>",
                escape_text(&style.axis_color),
                y1 = layout.axis_top + layout.axis_h - 4,
                y2 = layout.axis_top + layout.axis_h
            ));
        }
    }
}
