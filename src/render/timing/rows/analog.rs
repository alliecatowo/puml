use super::*;

pub(super) struct TimingAnalogRender<'a> {
    pub(super) signal: &'a FamilyNode,
    pub(super) sig_events: &'a [(i64, String)],
    pub(super) t_min: i64,
    pub(super) t_max: i64,
    /// Pre-computed waveform end time from `TimingLayout::waveform_end_t()`.
    pub(super) waveform_end_t: i64,
    pub(super) wave_y_hi: i32,
    pub(super) wave_y_lo: i32,
    pub(super) wave_mid: i32,
    pub(super) time_to_x: &'a dyn Fn(i64) -> i32,
    pub(super) style: &'a crate::theme::TimingStyle,
}

pub(super) fn render_timing_analog_signal(out: &mut String, ctx: TimingAnalogRender<'_>) {
    let mut min_max = ctx.signal.members.iter().find_map(|member| {
        let rest = member.text.strip_prefix("__timing:analog_between ")?;
        let mut parts = rest.split_whitespace();
        Some((
            parts.next()?.parse::<f64>().ok()?,
            parts.next()?.parse::<f64>().ok()?,
        ))
    });
    if min_max.is_none() {
        let values: Vec<f64> = ctx
            .sig_events
            .iter()
            .filter_map(|(_, state)| timing_state_display(state).parse::<f64>().ok())
            .collect();
        if let (Some(min), Some(max)) = (
            values.iter().copied().reduce(f64::min),
            values.iter().copied().reduce(f64::max),
        ) {
            min_max = Some((0.0_f64.min(min), max.max(1.0)));
        }
    }
    let (min_v, max_v) = min_max.unwrap_or((0.0, 1.0));
    let span = (max_v - min_v).abs().max(1.0);
    let y_for = |value: f64| -> i32 {
        let ratio = ((value - min_v) / span).clamp(0.0, 1.0);
        ctx.wave_y_lo - (ratio * (ctx.wave_y_lo - ctx.wave_y_hi) as f64).round() as i32
    };
    out.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#cbd5e1\" stroke-width=\"1\"/>",
        (ctx.time_to_x)(ctx.t_min),
        ctx.wave_y_lo,
        (ctx.time_to_x)(ctx.waveform_end_t),
        ctx.wave_y_lo
    ));
    if ctx.sig_events.is_empty() {
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#94a3b8\" stroke-width=\"1.5\"/>",
            (ctx.time_to_x)(ctx.t_min),
            ctx.wave_mid,
            (ctx.time_to_x)(ctx.t_max),
            ctx.wave_mid
        ));
        return;
    }
    let points = ctx
        .sig_events
        .iter()
        .filter_map(|(time, state)| {
            let value = timing_state_display(state).parse::<f64>().ok()?;
            Some(format!("{},{}", (ctx.time_to_x)(*time), y_for(value)))
        })
        .collect::<Vec<_>>()
        .join(" ");
    out.push_str(&format!(
        "<polyline class=\"timing-analog\" points=\"{points}\" fill=\"none\" stroke=\"{}\" stroke-width=\"2\"/>",
        escape_text(&ctx.style.signal_border_color)
    ));
    for (time, state) in ctx.sig_events {
        let display = timing_state_display(state);
        let Ok(value) = display.parse::<f64>() else {
            continue;
        };
        let x = (ctx.time_to_x)(*time);
        let y = y_for(value);
        // Extract per-event color if present (e.g. `3.3 #palegreen`).
        let event_fill = timing_state_style(state).fill;
        let point_fill = event_fill
            .as_deref()
            .unwrap_or(ctx.style.signal_border_color.as_str());
        out.push_str(&format!(
            "<circle class=\"timing-analog-point\" cx=\"{x}\" cy=\"{y}\" r=\"3\" fill=\"{}\"/>",
            escape_text(point_fill)
        ));
        // If per-event color is present, draw a small fill rect below the point to
        // visually indicate the event color.
        if let Some(ref fill) = event_fill {
            let rect_h = (ctx.wave_y_lo - y).max(0);
            if rect_h > 0 {
                out.push_str(&format!(
                    "<rect class=\"timing-analog-fill\" x=\"{}\" y=\"{y}\" width=\"6\" height=\"{rect_h}\" fill=\"{}\" opacity=\"0.5\"/>",
                    x - 3,
                    escape_text(fill)
                ));
            }
        }
        out.push_str(&format!(
            "<text x=\"{x}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"#475569\">{}</text>",
            y - 6,
            escape_text(&display)
        ));
    }
}
