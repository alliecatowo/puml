use super::model::{timing_control_i64, timing_signal_is_analog, TimingLayout, TimingModel};
use super::*;
mod analog;
use analog::{render_timing_analog_signal, TimingAnalogRender};
use std::collections::BTreeMap;

pub(super) fn signal_row_midpoints(
    signals: &[&FamilyNode],
    layout: &TimingLayout,
) -> BTreeMap<String, i32> {
    let mut signal_row_mid = BTreeMap::new();
    for (idx, signal) in signals.iter().enumerate() {
        let row_y = layout.signals_top + (idx as i32) * layout.row_h;
        let y = row_y + layout.row_h / 2;
        signal_row_mid.insert(signal.name.to_ascii_lowercase(), y);
        if let Some(alias) = signal.alias.as_deref() {
            signal_row_mid.insert(alias.to_ascii_lowercase(), y);
        }
        if let Some(label) = signal.label.as_deref() {
            signal_row_mid.insert(label.to_ascii_lowercase(), y);
        }
    }
    signal_row_mid
}

pub(super) fn render_timing_rows(
    out: &mut String,
    model: &TimingModel<'_>,
    layout: &TimingLayout,
    style: &crate::theme::TimingStyle,
) {
    for (row_idx, signal) in model.signals.iter().enumerate() {
        let row_y = layout.signals_top + (row_idx as i32) * layout.row_h;
        let wave_y_hi = row_y + layout.wave_top_pad;
        let wave_y_lo = row_y + layout.wave_top_pad + layout.wave_h;
        let wave_mid = (wave_y_hi + wave_y_lo) / 2;

        render_row_background(out, row_idx, row_y, layout, style);
        render_signal_labels(out, signal, wave_mid, layout, style);

        let sig_events = collect_signal_events(signal, model);
        if timing_signal_is_analog(signal) {
            render_timing_analog_signal(
                out,
                TimingAnalogRender {
                    signal,
                    sig_events: &sig_events,
                    t_min: model.t_min,
                    t_max: model.t_max,
                    waveform_end_t: layout.waveform_end_t(),
                    wave_y_hi,
                    wave_y_lo,
                    wave_mid,
                    time_to_x: &|time| layout.time_to_x(time),
                    style,
                },
            );
            continue;
        }

        let ctx = RowRender {
            sig_events: &sig_events,
            model,
            layout,
            wave_y_hi,
            wave_y_lo,
            wave_mid,
            style,
        };
        match signal.kind {
            FamilyNodeKind::TimingBinary => render_binary_signal(out, ctx),
            FamilyNodeKind::TimingClock => render_clock_signal(out, signal, ctx),
            FamilyNodeKind::TimingRobust => render_robust_signal(out, signal, ctx),
            _ => render_concise_signal(out, ctx),
        }
    }
}

fn render_row_background(
    out: &mut String,
    row_idx: usize,
    row_y: i32,
    layout: &TimingLayout,
    style: &crate::theme::TimingStyle,
) {
    let row_bg = if row_idx.is_multiple_of(2) {
        "#ffffff"
    } else {
        "#f8fafc"
    };
    out.push_str(&format!(
        "<rect x=\"0\" y=\"{row_y}\" width=\"{width}\" height=\"{row_h}\" fill=\"{row_bg}\"/>",
        width = layout.width,
        row_h = layout.row_h
    ));
    out.push_str(&format!(
        "<line x1=\"0\" y1=\"{y}\" x2=\"{width}\" y2=\"{y}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
        escape_text(&style.grid_color),
        y = row_y + layout.row_h,
        width = layout.width
    ));
}

fn render_signal_labels(
    out: &mut String,
    signal: &FamilyNode,
    wave_mid: i32,
    layout: &TimingLayout,
    style: &crate::theme::TimingStyle,
) {
    let signal_label = signal.label.as_deref().unwrap_or(&signal.name);
    out.push_str(&format!(
        "<text x=\"{x}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"12\" font-weight=\"600\" fill=\"{}\" text-anchor=\"end\">{name}</text>",
        escape_text(&style.font_color),
        x = layout.left_pad - 8,
        ty = wave_mid + 4,
        name = escape_text(signal_label)
    ));
    // Kind-tag suppression (#1372): PlantUML does not emit "concise", "robust",
    // "binary", or "clock" sub-labels under lane names.  Only the lane name
    // itself is shown.  We suppress the kind sub-label entirely to match parity.
    if !signal.members.is_empty() {
        let controls = signal
            .members
            .iter()
            .map(|m| m.text.as_str())
            .filter(|text| !text.starts_with("__timing:"))
            .collect::<Vec<_>>()
            .join(", ");
        if !controls.is_empty() {
            out.push_str(&format!(
                "<text x=\"{x}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"9\" fill=\"#64748b\" text-anchor=\"end\">{controls}</text>",
                x = layout.left_pad - 8,
                ty = wave_mid + 28,
                controls = escape_text(&controls)
            ));
        }
    }
}

fn collect_signal_events(signal: &FamilyNode, model: &TimingModel<'_>) -> Vec<(i64, String)> {
    let mut sig_events: Vec<(i64, String)> = model
        .events
        .iter()
        .filter(|e| e.alias.as_deref() == Some(signal.name.as_str()))
        .filter_map(|e| {
            let t = if e.name.is_empty() {
                model.t_min
            } else {
                super::model::timing_time_value(&e.name)?
            };
            let state = e
                .members
                .first()
                .map(|m| m.text.clone())
                .unwrap_or_default();
            Some((t, state))
        })
        .collect();
    sig_events.sort_by_key(|(t, _)| *t);
    sig_events
}

struct RowRender<'a> {
    sig_events: &'a [(i64, String)],
    model: &'a TimingModel<'a>,
    layout: &'a TimingLayout,
    wave_y_hi: i32,
    wave_y_lo: i32,
    wave_mid: i32,
    style: &'a crate::theme::TimingStyle,
}

fn render_binary_signal(out: &mut String, ctx: RowRender<'_>) {
    let is_high = |s: &str| -> bool {
        let l = timing_state_display(s).to_ascii_lowercase();
        matches!(l.as_str(), "1" | "high" | "on" | "true")
    };
    let mut segments: Vec<(i64, i64, bool)> = Vec::new();
    let end_t = waveform_end_t(ctx.layout);
    if ctx.sig_events.is_empty() {
        segments.push((ctx.model.t_min, end_t, false));
    } else {
        segments.push((ctx.model.t_min, ctx.sig_events[0].0, false));
        for i in 0..ctx.sig_events.len() {
            let t_start = ctx.sig_events[i].0;
            let t_end = ctx.sig_events.get(i + 1).map(|(t, _)| *t).unwrap_or(end_t);
            segments.push((t_start, t_end, is_high(&ctx.sig_events[i].1)));
        }
    }

    let mut path = String::from("M ");
    let mut first_seg = true;
    let mut cur_hi = false;
    for (ts, te, hi) in &segments {
        let x1 = ctx.layout.time_to_x(*ts);
        let x2 = ctx.layout.time_to_x(*te).min(ctx.layout.content_x_max);
        let cy = if *hi { ctx.wave_y_hi } else { ctx.wave_y_lo };
        if first_seg {
            path.push_str(&format!("{x1},{cy} "));
            first_seg = false;
            cur_hi = *hi;
        } else if *hi != cur_hi {
            path.push_str(&format!("L {x1},{cy} "));
            cur_hi = *hi;
        }
        path.push_str(&format!("L {x2},{cy} "));
    }
    out.push_str(&format!(
        "<polyline points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"2\"/>",
        path.replace("M ", "").replace("L ", ""),
        escape_text(&ctx.style.signal_border_color)
    ));

    for (t, state) in ctx.sig_events {
        if timing_state_hidden(state) {
            continue;
        }
        let lx = ctx.layout.time_to_x(*t);
        let label_ty = ctx.wave_y_hi - 4;
        out.push_str(&format!(
            "<text x=\"{lx}\" y=\"{label_ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"#475569\">{}</text>",
            escape_text(&timing_state_display(state))
        ));
    }
}

fn render_clock_signal(out: &mut String, signal: &FamilyNode, ctx: RowRender<'_>) {
    let clock_swing = (ctx.layout.wave_h / 4).max(8);
    let clock_y_hi = ctx.wave_mid - clock_swing;
    let clock_y_lo = ctx.wave_mid + clock_swing;
    let controlled_period = timing_control_i64(signal, "period");
    let controlled_pulse = timing_control_i64(signal, "pulse");
    let controlled_offset = timing_control_i64(signal, "offset").unwrap_or(0);
    let period = if let Some(period) = controlled_period {
        period.max(1)
    } else if ctx.sig_events.len() >= 2 {
        (ctx.sig_events[1].0 - ctx.sig_events[0].0).max(1)
    } else if ctx.model.time_vals.len() >= 2 {
        (ctx.model.time_vals[1] - ctx.model.time_vals[0]).max(1)
    } else {
        ctx.model.t_span / 4
    };
    let half = controlled_pulse
        .unwrap_or_else(|| (period / 2).max(1))
        .clamp(1, period.max(1));
    let t_end = ctx.model.t_max + period;

    let mut path_pts = String::new();
    let mut cur_t = ctx.model.t_min.saturating_add(controlled_offset);
    while cur_t > ctx.model.t_min {
        cur_t = cur_t.saturating_sub(period);
    }
    let mut cur_hi = ctx
        .sig_events
        .first()
        .map(|(_, s)| {
            let l = timing_state_display(s).to_ascii_lowercase();
            matches!(l.as_str(), "high" | "1" | "on" | "true")
        })
        .unwrap_or(true);
    let x_max = ctx.layout.content_x_max;
    let x0 = ctx.layout.time_to_x(cur_t).min(x_max);
    let y0 = if cur_hi { clock_y_hi } else { clock_y_lo };
    path_pts.push_str(&format!("{x0},{y0}"));
    while cur_t < t_end {
        let next_t = cur_t + half;
        let x1 = ctx.layout.time_to_x(next_t).min(x_max);
        let cur_y = if cur_hi { clock_y_hi } else { clock_y_lo };
        path_pts.push_str(&format!(" {x1},{cur_y}"));
        cur_hi = !cur_hi;
        let next_y = if cur_hi { clock_y_hi } else { clock_y_lo };
        path_pts.push_str(&format!(" {x1},{next_y}"));
        cur_t = next_t;
        if x1 >= x_max {
            break;
        }
    }
    out.push_str(&format!(
        "<polyline data-timing-period=\"{period}\" data-timing-pulse=\"{half}\" data-timing-offset=\"{controlled_offset}\" points=\"{path_pts}\" fill=\"none\" stroke=\"{}\" stroke-width=\"2\"/>",
        escape_text(&ctx.style.signal_border_color),
    ));
    out.push_str(&format!(
        "<text x=\"{x}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"10\" fill=\"#64748b\">clk</text>",
        x = ctx.layout.time_to_x(ctx.model.t_min) + 4,
        ty = clock_y_hi - 4
    ));
}

fn render_robust_signal(out: &mut String, signal: &FamilyNode, ctx: RowRender<'_>) {
    let declared_order: Vec<String> = signal
        .members
        .iter()
        .find_map(|m| m.text.strip_prefix("__timing:order:"))
        .map(|order| {
            order
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let mut state_order: Vec<String> = declared_order.clone();
    for (_, state) in ctx.sig_events {
        let display = timing_state_display(state);
        if !state_order.contains(&display) {
            state_order.push(display);
        }
    }
    let state_color_idx =
        |s: &str| -> usize { state_order.iter().position(|x| x == s).unwrap_or(0) };

    let end_t = waveform_end_t(ctx.layout);
    let transition_w = 6i32;

    if ctx.sig_events.is_empty() {
        out.push_str(&format!(
            "<line x1=\"{x1}\" y1=\"{wave_mid}\" x2=\"{x2}\" y2=\"{wave_mid}\" stroke=\"#94a3b8\" stroke-width=\"1.5\"/>",
            x1 = ctx.layout.time_to_x(ctx.model.t_min),
            x2 = ctx.layout.time_to_x(end_t).min(ctx.layout.content_x_max),
            wave_mid = ctx.wave_mid
        ));
        return;
    }

    for i in 0..ctx.sig_events.len() {
        let (t_start, ref state) = ctx.sig_events[i];
        let t_end = ctx.sig_events.get(i + 1).map(|(t, _)| *t).unwrap_or(end_t);
        let x1 = ctx.layout.time_to_x(t_start);
        let x2 = ctx.layout.time_to_x(t_end).min(ctx.layout.content_x_max);
        if timing_state_hidden(state) {
            render_hidden_state(out, x1, x2, ctx.wave_mid);
            continue;
        }
        let display = timing_state_display(state);
        let cidx = state_color_idx(&display);
        let state_style = timing_state_style(state);
        let fill = state_style
            .fill
            .as_deref()
            .unwrap_or_else(|| timing_state_color(&display, cidx));
        let stroke = state_style.line.as_deref().unwrap_or("#475569");
        let pts = format!(
            "{},{} {},{} {},{} {},{}",
            x1 + transition_w,
            ctx.wave_y_hi,
            x2,
            ctx.wave_y_hi,
            x2 - transition_w,
            ctx.wave_y_lo,
            x1,
            ctx.wave_y_lo
        );
        out.push_str(&format!(
            "<polygon points=\"{pts}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            escape_text(fill),
            escape_text(stroke)
        ));
        render_state_label(
            out,
            (x1 + x2) / 2,
            ctx.wave_mid + 4,
            &display,
            "#0f172a",
            true,
            x2 - x1,
        );
    }
}

fn render_concise_signal(out: &mut String, ctx: RowRender<'_>) {
    let end_t = waveform_end_t(ctx.layout);
    if ctx.sig_events.is_empty() {
        out.push_str(&format!(
            "<line x1=\"{x1}\" y1=\"{wave_mid}\" x2=\"{x2}\" y2=\"{wave_mid}\" stroke=\"#94a3b8\" stroke-width=\"1.5\" stroke-dasharray=\"4 3\"/>",
            x1 = ctx.layout.time_to_x(ctx.model.t_min),
            x2 = ctx.layout.time_to_x(end_t).min(ctx.layout.content_x_max),
            wave_mid = ctx.wave_mid
        ));
        return;
    }

    for i in 0..ctx.sig_events.len() {
        let (t_start, ref state) = ctx.sig_events[i];
        let t_end = ctx.sig_events.get(i + 1).map(|(t, _)| *t).unwrap_or(end_t);
        let x1 = ctx.layout.time_to_x(t_start);
        let x2 = ctx.layout.time_to_x(t_end).min(ctx.layout.content_x_max);
        if timing_state_hidden(state) {
            render_hidden_state(out, x1, x2, ctx.wave_mid);
            continue;
        }
        let display = timing_state_display(state);
        let state_style = timing_state_style(state);
        let stroke = state_style.line.as_deref().unwrap_or("#0f172a");
        if let Some(fill) = state_style.fill.as_deref() {
            out.push_str(&format!(
                "<rect class=\"timing-state-fill\" x=\"{x1}\" y=\"{wave_y_hi}\" width=\"{}\" height=\"{}\" fill=\"{}\" opacity=\"0.5\"/>",
                (x2 - x1).max(1),
                ctx.wave_y_lo - ctx.wave_y_hi,
                escape_text(fill),
                wave_y_hi = ctx.wave_y_hi
            ));
        }
        out.push_str(&format!(
            "<line x1=\"{x1}\" y1=\"{wave_y_hi}\" x2=\"{x2}\" y2=\"{wave_y_hi}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            escape_text(stroke),
            wave_y_hi = ctx.wave_y_hi
        ));
        out.push_str(&format!(
            "<line x1=\"{x1}\" y1=\"{wave_y_lo}\" x2=\"{x2}\" y2=\"{wave_y_lo}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            escape_text(stroke),
            wave_y_lo = ctx.wave_y_lo
        ));
        out.push_str(&format!(
            "<line x1=\"{x1}\" y1=\"{wave_y_hi}\" x2=\"{x1}\" y2=\"{wave_y_lo}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            escape_text(stroke),
            wave_y_hi = ctx.wave_y_hi,
            wave_y_lo = ctx.wave_y_lo
        ));
        render_state_label(
            out,
            (x1 + x2) / 2,
            ctx.wave_mid + 4,
            &display,
            "#1e293b",
            false,
            x2 - x1,
        );
    }
    let last_x = ctx.layout.time_to_x(end_t).min(ctx.layout.content_x_max);
    out.push_str(&format!(
        "<line x1=\"{last_x}\" y1=\"{wave_y_hi}\" x2=\"{last_x}\" y2=\"{wave_y_lo}\" stroke=\"#0f172a\" stroke-width=\"1.5\"/>",
        wave_y_hi = ctx.wave_y_hi,
        wave_y_lo = ctx.wave_y_lo
    ));
}

fn render_hidden_state(out: &mut String, x1: i32, x2: i32, wave_mid: i32) {
    out.push_str(&format!(
        "<line class=\"timing-hidden-state\" x1=\"{x1}\" y1=\"{wave_mid}\" x2=\"{x2}\" y2=\"{wave_mid}\" stroke=\"#cbd5e1\" stroke-width=\"1.2\" stroke-dasharray=\"5 4\"/>",
    ));
}

fn render_state_label(
    out: &mut String,
    label_x: i32,
    label_ty: i32,
    display: &str,
    fill: &str,
    bold: bool,
    cell_w: i32,
) {
    // #1524: squish label to fit cell width so adjacent labels never collide.
    // Estimate text width at 11px monospace (~6.5 px/char).  If the label
    // would overflow the cell, use SVG textLength to compress it into the
    // available space.  Narrow cells (≤ 10 px) carry the full label without
    // compression — they are tail extensions that have no visible neighbour.
    let char_w_px = 6.5_f32;
    let label_w = (display.chars().count() as f32 * char_w_px).ceil() as i32;
    let available = (cell_w - 8).max(0);
    let weight = if bold { " font-weight=\"600\"" } else { "" };
    if available > 10 && label_w > available {
        let squeezed = available.max(16);
        out.push_str(&format!(
            "<text x=\"{label_x}\" y=\"{label_ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{fill}\"{weight} textLength=\"{squeezed}\" lengthAdjust=\"spacingAndGlyphs\">{}</text>",
            escape_text(display)
        ));
    } else {
        out.push_str(&format!(
            "<text x=\"{label_x}\" y=\"{label_ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{fill}\"{weight}>{}</text>",
            escape_text(display)
        ));
    }
}

fn waveform_end_t(layout: &TimingLayout) -> i64 {
    layout.waveform_end_t()
}

fn timing_state_color(state: &str, idx: usize) -> &'static str {
    let lower = state.to_ascii_lowercase();
    if lower == "high" || lower == "1" {
        return "#bbf7d0";
    }
    if lower == "low" || lower == "0" {
        return "#fecaca";
    }
    if lower == "undef" || lower == "x" || lower == "z" {
        return "#e2e8f0";
    }
    const PALETTE: &[&str] = &[
        "#bfdbfe", "#ddd6fe", "#fde68a", "#a7f3d0", "#fca5a5", "#6ee7b7", "#93c5fd", "#c4b5fd",
    ];
    PALETTE[idx % PALETTE.len()]
}

#[derive(Default)]
struct TimingStateStyle {
    fill: Option<String>,
    line: Option<String>,
}

fn timing_state_style(state: &str) -> TimingStateStyle {
    let Some((_, style)) = state.split_once(" #") else {
        return TimingStateStyle::default();
    };
    let mut parsed = TimingStateStyle::default();
    let style = format!("#{style}");
    for token in style
        .split(';')
        .map(str::trim)
        .filter(|token| !token.is_empty())
    {
        if let Some(line) = token.strip_prefix("line:") {
            parsed.line = Some(super::model::timing_svg_color(line));
        } else if let Some(line) = token.strip_prefix("line.") {
            if matches!(line, "dashed" | "dotted" | "bold") {
                continue;
            }
        } else if token.starts_with('#') {
            parsed.fill = Some(super::model::timing_svg_color(token));
        }
    }
    parsed
}

fn timing_state_display(state: &str) -> String {
    state
        .split_once(" #")
        .map(|(display, _)| display)
        .unwrap_or(state)
        .trim()
        .to_string()
}

fn timing_state_hidden(state: &str) -> bool {
    matches!(
        timing_state_display(state).to_ascii_lowercase().as_str(),
        "-" | "hidden"
    )
}
