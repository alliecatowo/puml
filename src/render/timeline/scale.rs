use super::*;

#[derive(Clone, Copy)]
pub(super) struct GanttScaleRenderOptions {
    pub(super) zoom: f32,
    pub(super) calendar_date: bool,
    pub(super) week_numbering_start: Option<i32>,
}

pub(super) fn parse_gantt_scale_render_options(options: &[String]) -> GanttScaleRenderOptions {
    let mut parsed = GanttScaleRenderOptions {
        zoom: 1.0,
        calendar_date: false,
        week_numbering_start: None,
    };
    for option in options {
        let lower = option.to_ascii_lowercase();
        if lower.contains("with calendar date") {
            parsed.calendar_date = true;
        }
        let tokens = lower.split_whitespace().collect::<Vec<_>>();
        for (idx, token) in tokens.iter().enumerate() {
            if *token == "zoom" {
                if let Some(value) = tokens.get(idx + 1).and_then(|raw| raw.parse::<f32>().ok()) {
                    if value.is_finite() && value > 0.0 {
                        parsed.zoom = value.clamp(0.25, 4.0);
                    }
                }
            }
            if *token == "numbering" && tokens.get(idx + 1) == Some(&"from") {
                if let Some(value) = tokens.get(idx + 2).and_then(|raw| raw.parse::<i32>().ok()) {
                    parsed.week_numbering_start = Some(value);
                }
            }
        }
    }
    parsed
}

pub(super) fn format_gantt_zoom(zoom: f32) -> String {
    let rounded = (zoom * 100.0).round() / 100.0;
    if (rounded.fract()).abs() < f32::EPSILON {
        format!("{}", rounded as i32)
    } else {
        format!("{rounded:.2}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

/// Compute tick offsets for the gantt date-header axis.
///
/// `chart_w` is the pixel width of the chart area. Each date label needs
/// roughly 72 px (10 chars x ~7 px each, plus a few px gap). We pick the
/// smallest stride from `[1, 2, 3, 7, 14, 30, 90, 180, 365]` that keeps
/// the number of ticks <= `chart_w / label_px`, then fall back to the
/// explicit `scale` override.
///
/// The final tick (last day) is only appended when it is at least
/// `LABEL_PX` pixels away from the preceding tick; otherwise the two labels
/// smear together into an unreadable blob (#426).
pub(super) fn gantt_tick_offsets_for_width(
    total_days: u32,
    scale: Option<&str>,
    chart_w: i32,
) -> Vec<u32> {
    // Approximate pixel width of a single date label (YYYY-MM-DD + small gap)
    const LABEL_PX: i32 = 72;

    let step = match scale {
        Some("weekly") => 7,
        Some("monthly") => 30,
        Some("quarterly") => 90,
        Some("yearly") => 365,
        // No explicit scale: auto-select the smallest stride that avoids overlap
        _ => {
            let max_ticks = (chart_w / LABEL_PX).max(2) as u32;
            let candidates = [1u32, 2, 3, 7, 14, 30, 90, 180, 365];
            candidates
                .into_iter()
                .find(|&s| total_days.div_ceil(s) <= max_ticks)
                .unwrap_or(365)
        }
    };
    let mut offsets = Vec::new();
    let mut offset = 0u32;
    while offset < total_days {
        offsets.push(offset);
        offset = offset.saturating_add(step);
    }
    let last_day_offset = total_days.saturating_sub(1);
    if offsets.last().copied() != Some(last_day_offset) {
        // Only append the last-day tick when it won't collide with the
        // preceding tick.  Collision threshold = one full label width in
        // pixels, converted back to days (#426).
        let min_gap_days = (LABEL_PX as u32)
            .saturating_mul(total_days)
            .div_ceil(chart_w.max(1) as u32);
        let prev = offsets.last().copied().unwrap_or(0);
        if last_day_offset.saturating_sub(prev) >= min_gap_days {
            offsets.push(last_day_offset);
        }
    }
    offsets
}

pub(super) fn format_gantt_axis_label(day: u32, min_day: u32, date_axis: bool) -> String {
    if date_axis {
        day_number_to_iso(day).unwrap_or_else(|| format!("D+{}", day.saturating_sub(min_day)))
    } else {
        format!("D+{}", day.saturating_sub(min_day))
    }
}

pub(super) fn format_gantt_scale_axis_label(
    day: u32,
    min_day: u32,
    date_axis: bool,
    scale: Option<&str>,
    options: &GanttScaleRenderOptions,
) -> String {
    if !date_axis {
        return format_gantt_axis_label(day, min_day, false);
    }
    let Some(iso) = day_number_to_iso(day) else {
        return format_gantt_axis_label(day, min_day, true);
    };
    match scale {
        Some("weekly") => {
            if let Some(start) = options.week_numbering_start {
                let week = start + (day.saturating_sub(min_day) / 7) as i32;
                format!("Week {week}")
            } else if options.calendar_date {
                iso
            } else {
                format!("Wk {iso}")
            }
        }
        Some("monthly") => iso
            .get(0..7)
            .map(format_month_label)
            .unwrap_or_else(|| iso.clone()),
        Some("quarterly") => format_quarter_label(&iso).unwrap_or_else(|| iso.clone()),
        Some("yearly") => iso.get(0..4).unwrap_or(&iso).to_string(),
        _ => iso,
    }
}

pub(super) fn format_month_label(year_month: &str) -> String {
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

pub(super) fn format_quarter_label(iso: &str) -> Option<String> {
    let year = iso.get(0..4)?;
    let month = iso.get(5..7)?.parse::<u32>().ok()?;
    let quarter = month.saturating_sub(1) / 3 + 1;
    Some(format!("Q{quarter} {year}"))
}

pub(super) fn is_gantt_closed_weekday_number(day: u32, closed_weekdays: &[String]) -> bool {
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
