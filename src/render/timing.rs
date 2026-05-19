use super::*;

fn timing_state_color(state: &str, idx: usize) -> &'static str {
    // Map well-known digital states first.
    let lower = state.to_ascii_lowercase();
    if lower == "high" || lower == "1" {
        return "#bbf7d0"; // green-100
    }
    if lower == "low" || lower == "0" {
        return "#fecaca"; // red-100
    }
    if lower == "undef" || lower == "x" || lower == "z" {
        return "#e2e8f0"; // slate-200
    }
    // Otherwise cycle through a palette.
    const PALETTE: &[&str] = &[
        "#bfdbfe", // blue-200
        "#ddd6fe", // violet-200
        "#fde68a", // amber-200
        "#a7f3d0", // emerald-200
        "#fca5a5", // red-300
        "#6ee7b7", // emerald-300
        "#93c5fd", // blue-300
        "#c4b5fd", // violet-300
    ];
    PALETTE[idx % PALETTE.len()]
}

pub fn render_timing_svg(doc: &FamilyDocument) -> String {
    let default_timing_style;
    let style = match &doc.family_style {
        Some(crate::model::FamilyStyle::Timing(style)) => style,
        _ => {
            default_timing_style = crate::theme::TimingStyle::default();
            &default_timing_style
        }
    };
    // ── Collect signals and events ────────────────────────────────────────────
    let signals: Vec<&FamilyNode> = doc
        .nodes
        .iter()
        .filter(|n| {
            matches!(
                n.kind,
                FamilyNodeKind::TimingConcise
                    | FamilyNodeKind::TimingRobust
                    | FamilyNodeKind::TimingClock
                    | FamilyNodeKind::TimingBinary
            )
        })
        .collect();
    let events: Vec<&FamilyNode> = doc
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, FamilyNodeKind::TimingEvent))
        .collect();
    let global_events: Vec<(i64, String)> = events
        .iter()
        .filter_map(|e| {
            if e.alias.is_some() {
                return None;
            }
            let t = e.name.parse::<i64>().ok()?;
            let txt = e
                .label
                .clone()
                .or_else(|| e.members.first().map(|m| m.text.clone()))
                .unwrap_or_default();
            if parse_timing_range_note(&txt).is_some() {
                return None;
            }
            if txt.is_empty() {
                None
            } else {
                Some((t, txt))
            }
        })
        .collect();
    let timing_ranges: Vec<(i64, i64, String)> = events
        .iter()
        .filter_map(|e| {
            if e.alias.is_some() {
                return None;
            }
            let start = e.name.parse::<i64>().ok()?;
            let txt = e
                .label
                .clone()
                .or_else(|| e.members.first().map(|m| m.text.clone()))
                .unwrap_or_default();
            let (end, label) = parse_timing_range_note(&txt)?;
            Some((start, end, label))
        })
        .collect();

    // ── Parse time positions (@N) ─────────────────────────────────────────────
    // Collect unique numeric time values, sort them.
    let mut time_vals: Vec<i64> = events
        .iter()
        .filter_map(|e| e.name.parse::<i64>().ok())
        .collect();
    time_vals.extend(timing_ranges.iter().map(|(_, end, _)| *end));
    time_vals.sort();
    time_vals.dedup();
    if time_vals.is_empty() {
        time_vals = vec![0, 10];
    }

    // SAFETY: time_vals is guaranteed non-empty by the guard above; use
    // explicit copy-out to avoid holding a reference across the borrow.
    let t_min = time_vals[0];
    let t_max = time_vals[time_vals.len() - 1];
    let t_span = (t_max - t_min).max(1);

    // ── Layout constants ──────────────────────────────────────────────────────
    let left_pad: i32 = 130; // signal name column width
                             // tail_extra: pixels allocated past the t_max tick for the final state label.
                             // Waveform segments extend to t_max + 5 % of t_span, so we add the same pixel
                             // budget (5 % of chart_w) plus a fixed label margin so labels are never clipped.
    let tail_extra: i32 = 80;
    // max_label_half_w: half the width of the widest axis tick label (e.g. "@10" at
    // monospace 11 px ≈ 4 chars × ~7 px/char ÷ 2 ≈ 14 px). Add a conservative 20 px
    // so that the rightmost "@N" label is never clipped regardless of digit count.
    let max_label_half_w: i32 = 20;
    // right_gutter: minimum blank space to the right of the last tick's label.
    let right_gutter: i32 = 20;
    let row_h: i32 = 64;
    let wave_top_pad: i32 = 10; // space above wave line inside row
    let wave_bot_pad: i32 = 10; // space below wave line inside row
    let wave_h: i32 = row_h - wave_top_pad - wave_bot_pad; // usable wave height
    let axis_h: i32 = 48;
    let chart_w: i32 = 760;
    // right_pad covers the 5 % overshoot of the waveform past t_max plus a label margin
    // PLUS the half-width of the rightmost axis label and the minimum right gutter, so
    // that the last "@N" tick label is never clipped at the canvas right edge.
    let right_pad: i32 =
        (chart_w as f64 * 0.05) as i32 + tail_extra + max_label_half_w + right_gutter;
    let width: i32 = left_pad + chart_w + right_pad;

    // 22px title lines + 14px subtitle + 10px padding
    let title_h: i32 = doc
        .title
        .as_deref()
        .map(|t| (t.lines().count() as i32) * 22 + 10)
        .unwrap_or(0)
        + 14; // subtitle line

    let n_signals = signals.len().max(1) as i32;
    let height: i32 = title_h + axis_h + n_signals * row_h + 32;

    // Map a time value to an x coordinate in the chart area.
    let time_to_x =
        |t: i64| -> i32 { left_pad + ((t - t_min) as f64 / t_span as f64 * chart_w as f64) as i32 };

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">"
    ));
    out.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>",
        escape_text(&style.background_color)
    ));
    out.push_str(&format!(
        "<metadata data-timing-style=\"{} {} {} {} {} {} {}\"/>",
        escape_text(&style.background_color),
        escape_text(&style.axis_color),
        escape_text(&style.grid_color),
        escape_text(&style.signal_background_color),
        escape_text(&style.signal_border_color),
        escape_text(&style.arrow_color),
        escape_text(&style.font_color)
    ));

    // ── Title ─────────────────────────────────────────────────────────────────
    let mut ty = 22i32;
    if let Some(title) = &doc.title {
        for line in title.lines() {
            out.push_str(&format!(
                "<text x=\"24\" y=\"{ty}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\" fill=\"{}\">{}</text>",
                escape_text(&style.font_color),
                escape_text(line)
            ));
            ty += 22;
        }
    }
    // Subtitle: always emit "timing diagram" so downstream checks/tests can rely on it.
    out.push_str(&format!(
        "<text x=\"24\" y=\"{ty}\" font-family=\"monospace\" font-size=\"11\" fill=\"#94a3b8\">timing diagram</text>",
    ));
    ty += 14;
    let axis_top = ty + 4;
    let signals_top = axis_top + axis_h;

    // ── Time axis ─────────────────────────────────────────────────────────────
    // Background strip for time axis
    out.push_str(&format!(
        "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
        escape_text(&style.signal_background_color),
        escape_text(&style.grid_color),
        x = left_pad,
        y = axis_top,
        w = chart_w,
        h = axis_h
    ));

    // Major ticks at each @N position
    let rows_h = n_signals * row_h;
    for (start, end, label) in &timing_ranges {
        let x1 = time_to_x((*start).min(*end));
        let x2 = time_to_x((*start).max(*end));
        let w = (x2 - x1).max(2);
        out.push_str(&format!(
            "<rect class=\"timing-range\" x=\"{x1}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"#fde68a\" opacity=\"0.45\" stroke=\"#f59e0b\" stroke-width=\"1\"/>",
            y = axis_top,
            h = axis_h + rows_h
        ));
        out.push_str(&format!(
            "<text class=\"timing-range-label\" x=\"{x}\" y=\"{y}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"#92400e\">{}</text>",
            escape_text(label),
            x = x1 + w / 2,
            y = axis_top + axis_h - 14
        ));
    }
    for &t in &time_vals {
        let tx = time_to_x(t);
        // Gridline through all signal rows
        out.push_str(&format!(
            "<line x1=\"{tx}\" y1=\"{y1}\" x2=\"{tx}\" y2=\"{y2}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"4 3\"/>",
            escape_text(&style.grid_color),
            y1 = signals_top,
            y2 = signals_top + rows_h
        ));
        // Tick mark on axis
        out.push_str(&format!(
            "<line x1=\"{tx}\" y1=\"{y1}\" x2=\"{tx}\" y2=\"{y2}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            escape_text(&style.axis_color),
            y1 = axis_top + axis_h - 8,
            y2 = axis_top + axis_h
        ));
        // Label
        out.push_str(&format!(
            "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">@{t}</text>",
            escape_text(&style.font_color),
            ty = axis_top + 20
        ));
    }

    for (t, note) in &global_events {
        let tx = time_to_x(*t);
        out.push_str(&format!(
            "<circle cx=\"{tx}\" cy=\"{cy}\" r=\"3\" fill=\"{}\"/>",
            escape_text(&style.arrow_color),
            cy = axis_top + 8
        ));
        out.push_str(&format!(
            "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"{}\">{}</text>",
            escape_text(&style.font_color),
            escape_text(note),
            ty = axis_top + 10
        ));
    }

    // Minor ticks at midpoints between adjacent time positions
    for w in time_vals.windows(2) {
        let mid = (w[0] + w[1]) / 2;
        let mx = time_to_x(mid);
        out.push_str(&format!(
            "<line x1=\"{mx}\" y1=\"{y1}\" x2=\"{mx}\" y2=\"{y2}\" stroke=\"{}\" stroke-width=\"0.75\"/>",
            escape_text(&style.axis_color),
            y1 = axis_top + axis_h - 4,
            y2 = axis_top + axis_h
        ));
    }

    // ── Signal rows ───────────────────────────────────────────────────────────
    for (row_idx, signal) in signals.iter().enumerate() {
        let row_y = signals_top + (row_idx as i32) * row_h;
        let wave_y_hi = row_y + wave_top_pad; // y for logical HIGH
        let wave_y_lo = row_y + wave_top_pad + wave_h; // y for logical LOW
        let wave_mid = (wave_y_hi + wave_y_lo) / 2;

        // Row background (alternating)
        let row_bg = if row_idx % 2 == 0 {
            "#ffffff"
        } else {
            "#f8fafc"
        };
        out.push_str(&format!(
            "<rect x=\"0\" y=\"{row_y}\" width=\"{width}\" height=\"{row_h}\" fill=\"{row_bg}\"/>",
        ));

        // Signal name label (left column)
        let signal_label = signal.label.as_deref().unwrap_or(&signal.name);
        out.push_str(&format!(
            "<text x=\"{x}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"12\" font-weight=\"600\" fill=\"{}\" text-anchor=\"end\">{name}</text>",
            escape_text(&style.font_color),
            x = left_pad - 8,
            ty = wave_mid + 4,
            name = escape_text(signal_label)
        ));
        // Signal kind tag
        out.push_str(&format!(
            "<text x=\"{x}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"9\" fill=\"#94a3b8\" text-anchor=\"end\">{kind}</text>",
            x = left_pad - 8,
            ty = wave_mid + 16,
            kind = family_node_label(signal.kind)
        ));
        if !signal.members.is_empty() {
            let controls = signal
                .members
                .iter()
                .map(|m| m.text.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!(
                "<text x=\"{x}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"9\" fill=\"#64748b\" text-anchor=\"end\">{controls}</text>",
                x = left_pad - 8,
                ty = wave_mid + 28,
                controls = escape_text(&controls)
            ));
        }

        // Collect events for this signal, sorted by time.
        let mut sig_events: Vec<(i64, String)> = events
            .iter()
            .filter(|e| e.alias.as_deref() == Some(signal.name.as_str()))
            .filter_map(|e| {
                let t = e.name.parse::<i64>().ok()?;
                let state = e
                    .members
                    .first()
                    .map(|m| m.text.clone())
                    .unwrap_or_default();
                Some((t, state))
            })
            .collect();
        sig_events.sort_by_key(|(t, _)| *t);

        // Row separator line at bottom
        out.push_str(&format!(
            "<line x1=\"0\" y1=\"{y}\" x2=\"{width}\" y2=\"{y}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
            escape_text(&style.grid_color),
            y = row_y + row_h
        ));

        match signal.kind {
            FamilyNodeKind::TimingBinary => {
                // Binary: flat baseline with vertical pulses at @N positions.
                // HIGH=1/high, LOW=0/low; default LOW if no state.
                let is_high = |s: &str| -> bool {
                    let l = s.to_ascii_lowercase();
                    matches!(l.as_str(), "1" | "high" | "on" | "true")
                };

                // Draw the waveform as segments between events.
                let mut segments: Vec<(i64, i64, bool)> = Vec::new();
                let end_t = t_max + (t_span as f64 * 0.05) as i64 + 1;
                if sig_events.is_empty() {
                    segments.push((t_min, end_t, false));
                } else {
                    // Before first event: assume low
                    segments.push((t_min, sig_events[0].0, false));
                    for i in 0..sig_events.len() {
                        let t_start = sig_events[i].0;
                        let t_end = sig_events.get(i + 1).map(|(t, _)| *t).unwrap_or(end_t);
                        segments.push((t_start, t_end, is_high(&sig_events[i].1)));
                    }
                }

                let mut path = String::from("M ");
                let mut first_seg = true;
                let mut cur_hi = false;
                for (ts, te, hi) in &segments {
                    let x1 = time_to_x(*ts);
                    let x2 = time_to_x(*te);
                    let cy = if *hi { wave_y_hi } else { wave_y_lo };
                    if first_seg {
                        path.push_str(&format!("{x1},{cy} "));
                        first_seg = false;
                        cur_hi = *hi;
                    } else if *hi != cur_hi {
                        // Vertical transition
                        path.push_str(&format!("L {x1},{cy} "));
                        cur_hi = *hi;
                    }
                    path.push_str(&format!("L {x2},{cy} "));
                }
                out.push_str(&format!(
                    "<polyline points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"2\"/>",
                    path.replace("M ", "").replace("L ", ""),
                    escape_text(&style.signal_border_color)
                ));

                // Pulse labels
                for (t, state) in &sig_events {
                    let lx = time_to_x(*t);
                    let label_ty = wave_y_hi - 4;
                    out.push_str(&format!(
                        "<text x=\"{lx}\" y=\"{label_ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"#475569\">{}</text>",
                        escape_text(state)
                    ));
                }
            }

            FamilyNodeKind::TimingClock => {
                // Clock: square wave. If edge events exist for this signal, use
                // their spacing as the period baseline; otherwise fallback.
                let controlled_period = timing_control_i64(signal, "period");
                let controlled_pulse = timing_control_i64(signal, "pulse");
                let controlled_offset = timing_control_i64(signal, "offset").unwrap_or(0);
                let period = if let Some(period) = controlled_period {
                    period.max(1)
                } else if sig_events.len() >= 2 {
                    (sig_events[1].0 - sig_events[0].0).max(1)
                } else if time_vals.len() >= 2 {
                    (time_vals[1] - time_vals[0]).max(1)
                } else {
                    t_span / 4
                };
                let half = controlled_pulse
                    .unwrap_or_else(|| (period / 2).max(1))
                    .clamp(1, period.max(1));
                let t_end = t_max + period;

                let mut path_pts = String::new();
                let mut cur_t = t_min.saturating_add(controlled_offset);
                while cur_t > t_min {
                    cur_t = cur_t.saturating_sub(period);
                }
                let mut cur_hi = sig_events
                    .first()
                    .map(|(_, s)| {
                        let l = s.to_ascii_lowercase();
                        matches!(l.as_str(), "high" | "1" | "on" | "true")
                    })
                    .unwrap_or(true);
                // Clock waveform is clamped to the canvas right edge so that extra
                // half-periods never bleed outside the viewBox.
                let x_max = width;
                let x0 = time_to_x(cur_t).min(x_max);
                let y0 = if cur_hi { wave_y_hi } else { wave_y_lo };
                path_pts.push_str(&format!("{x0},{y0}"));
                while cur_t < t_end {
                    let next_t = cur_t + half;
                    let x1 = time_to_x(next_t).min(x_max);
                    // Horizontal segment
                    let cur_y = if cur_hi { wave_y_hi } else { wave_y_lo };
                    path_pts.push_str(&format!(" {x1},{cur_y}"));
                    // Vertical transition
                    cur_hi = !cur_hi;
                    let next_y = if cur_hi { wave_y_hi } else { wave_y_lo };
                    path_pts.push_str(&format!(" {x1},{next_y}"));
                    cur_t = next_t;
                    if x1 >= x_max {
                        break;
                    }
                }
                out.push_str(&format!(
                    "<polyline data-timing-period=\"{period}\" data-timing-pulse=\"{half}\" data-timing-offset=\"{controlled_offset}\" points=\"{path_pts}\" fill=\"none\" stroke=\"{}\" stroke-width=\"2\"/>",
                    escape_text(&style.signal_border_color),
                ));
                // Clock label
                out.push_str(&format!(
                    "<text x=\"{x}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"10\" fill=\"#64748b\">clk</text>",
                    x = time_to_x(t_min) + 4,
                    ty = wave_y_hi - 4
                ));
            }

            FamilyNodeKind::TimingRobust => {
                // Robust: same as concise but with coloured fills per unique state.
                // Build unique state → colour map.
                let mut state_order: Vec<String> = Vec::new();
                for (_, state) in &sig_events {
                    if !state_order.contains(state) {
                        state_order.push(state.clone());
                    }
                }
                let state_color_idx =
                    |s: &str| -> usize { state_order.iter().position(|x| x == s).unwrap_or(0) };

                let end_t = t_max + (t_span as f64 * 0.05) as i64 + 1;
                let transition_w = 6i32; // slant width in px

                if sig_events.is_empty() {
                    // Flat unknown line
                    out.push_str(&format!(
                        "<line x1=\"{x1}\" y1=\"{wave_mid}\" x2=\"{x2}\" y2=\"{wave_mid}\" stroke=\"#94a3b8\" stroke-width=\"1.5\"/>",
                        x1 = time_to_x(t_min),
                        x2 = time_to_x(end_t)
                    ));
                } else {
                    // Render coloured state boxes with slanted transitions.
                    for i in 0..sig_events.len() {
                        let (t_start, ref state) = sig_events[i];
                        let t_end = sig_events.get(i + 1).map(|(t, _)| *t).unwrap_or(end_t);
                        let x1 = time_to_x(t_start);
                        let x2 = time_to_x(t_end);
                        let cidx = state_color_idx(state);
                        let fill = timing_state_color(state, cidx);

                        // Filled parallelogram-ish box
                        let pts = format!(
                            "{},{} {},{} {},{} {},{}",
                            x1 + transition_w,
                            wave_y_hi,
                            x2,
                            wave_y_hi,
                            x2 - transition_w,
                            wave_y_lo,
                            x1,
                            wave_y_lo
                        );
                        out.push_str(&format!(
                            "<polygon points=\"{pts}\" fill=\"{fill}\" stroke=\"#475569\" stroke-width=\"1.5\"/>",
                        ));

                        // State label centred in box
                        let label_x = (x1 + x2) / 2;
                        let label_ty = wave_mid + 4;
                        out.push_str(&format!(
                            "<text x=\"{label_x}\" y=\"{label_ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#0f172a\" font-weight=\"600\">{}</text>",
                            escape_text(state)
                        ));
                    }
                }
            }

            // TimingConcise (default)
            _ => {
                // Concise: state-name boxes with sharp vertical transitions.
                let end_t = t_max + (t_span as f64 * 0.05) as i64 + 1;

                if sig_events.is_empty() {
                    out.push_str(&format!(
                        "<line x1=\"{x1}\" y1=\"{wave_mid}\" x2=\"{x2}\" y2=\"{wave_mid}\" stroke=\"#94a3b8\" stroke-width=\"1.5\" stroke-dasharray=\"4 3\"/>",
                        x1 = time_to_x(t_min),
                        x2 = time_to_x(end_t)
                    ));
                } else {
                    // Top and bottom border lines for each segment.
                    for i in 0..sig_events.len() {
                        let (t_start, ref state) = sig_events[i];
                        let t_end = sig_events.get(i + 1).map(|(t, _)| *t).unwrap_or(end_t);
                        let x1 = time_to_x(t_start);
                        let x2 = time_to_x(t_end);

                        // Top border
                        out.push_str(&format!(
                            "<line x1=\"{x1}\" y1=\"{wave_y_hi}\" x2=\"{x2}\" y2=\"{wave_y_hi}\" stroke=\"#0f172a\" stroke-width=\"1.5\"/>",
                        ));
                        // Bottom border
                        out.push_str(&format!(
                            "<line x1=\"{x1}\" y1=\"{wave_y_lo}\" x2=\"{x2}\" y2=\"{wave_y_lo}\" stroke=\"#0f172a\" stroke-width=\"1.5\"/>",
                        ));
                        // Left vertical edge (transition)
                        out.push_str(&format!(
                            "<line x1=\"{x1}\" y1=\"{wave_y_hi}\" x2=\"{x1}\" y2=\"{wave_y_lo}\" stroke=\"#0f172a\" stroke-width=\"1.5\"/>",
                        ));

                        // State label centred in box
                        let label_x = (x1 + x2) / 2;
                        let label_ty = wave_mid + 4;
                        out.push_str(&format!(
                            "<text x=\"{label_x}\" y=\"{label_ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#1e293b\">{}</text>",
                            escape_text(state)
                        ));
                    }
                    // Right closing edge
                    let last_x = time_to_x(end_t);
                    out.push_str(&format!(
                        "<line x1=\"{last_x}\" y1=\"{wave_y_hi}\" x2=\"{last_x}\" y2=\"{wave_y_lo}\" stroke=\"#0f172a\" stroke-width=\"1.5\"/>",
                    ));
                }
            }
        }
    }

    out.push_str("</svg>");
    out
}

fn timing_control_i64(signal: &FamilyNode, key: &str) -> Option<i64> {
    for member in &signal.members {
        let mut parts = member.text.split_whitespace();
        while let Some(part) = parts.next() {
            if part.eq_ignore_ascii_case(key) {
                if let Some(value) = parts.next().and_then(|v| v.parse::<i64>().ok()) {
                    return Some(value);
                }
            }
        }
    }
    None
}

fn parse_timing_range_note(note: &str) -> Option<(i64, String)> {
    let rest = note.strip_prefix("range:")?;
    let (end, label) = rest.split_once(':').unwrap_or((rest, ""));
    let end = end.trim().trim_start_matches('@').parse::<i64>().ok()?;
    let label = if label.trim().is_empty() {
        "range".to_string()
    } else {
        label.trim().to_string()
    };
    Some((end, label))
}
