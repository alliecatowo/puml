use super::*;
use crate::model::FamilyRelation;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone)]
pub(super) struct TimingRange {
    pub(super) start: i64,
    pub(super) end: i64,
    pub(super) label: String,
    pub(super) fill_color: Option<String>,
}

pub(super) struct TimingModel<'a> {
    pub(super) signals: Vec<&'a FamilyNode>,
    pub(super) events: Vec<&'a FamilyNode>,
    pub(super) options: BTreeSet<String>,
    pub(super) hide_time_axis: bool,
    pub(super) manual_time_axis: bool,
    pub(super) compact_mode: bool,
    pub(super) time_labels: BTreeMap<i64, String>,
    pub(super) global_events: Vec<(i64, String)>,
    pub(super) ranges: Vec<TimingRange>,
    pub(super) time_vals: Vec<i64>,
    pub(super) t_min: i64,
    pub(super) t_max: i64,
    pub(super) t_span: i64,
}

impl<'a> TimingModel<'a> {
    pub(super) fn from_document(doc: &'a FamilyDocument) -> Self {
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
        let options: BTreeSet<String> = events
            .iter()
            .filter_map(|e| e.label.as_deref())
            .filter_map(|label| label.strip_prefix("__timing:").map(str::to_string))
            .collect();
        let hide_time_axis = options.contains("hide-time-axis");
        let manual_time_axis = options.contains("manual-time-axis");
        let compact_mode = options.contains("mode:compact");

        let mut time_labels = BTreeMap::<i64, String>::new();
        for event in &events {
            let Some(value) = timing_time_value(&event.name) else {
                continue;
            };
            time_labels
                .entry(value)
                .or_insert_with(|| format!("@{}", event.name.trim().trim_start_matches('@')));
        }

        let global_events = collect_global_events(&events);
        let ranges = collect_timing_ranges(&events);
        let mut time_vals = collect_time_values(&events, &ranges, &doc.relations);

        if time_vals.is_empty() {
            time_vals = vec![0, 10];
        }
        let t_min = time_vals[0];
        let t_max = time_vals[time_vals.len() - 1];
        let t_span = (t_max - t_min).max(1);

        Self {
            signals,
            events,
            options,
            hide_time_axis,
            manual_time_axis,
            compact_mode,
            time_labels,
            global_events,
            ranges,
            time_vals,
            t_min,
            t_max,
            t_span,
        }
    }
}

pub(super) struct TimingLayout {
    pub(super) left_pad: i32,
    pub(super) row_h: i32,
    pub(super) wave_top_pad: i32,
    pub(super) wave_h: i32,
    pub(super) axis_h: i32,
    pub(super) chart_w: i32,
    pub(super) width: i32,
    pub(super) height: i32,
    pub(super) axis_panel_w: i32,
    pub(super) content_x_max: i32,
    pub(super) axis_h_effective: i32,
    pub(super) axis_top: i32,
    pub(super) signals_top: i32,
    pub(super) n_signals: i32,
    pub(super) t_min: i64,
    pub(super) t_span: i64,
}

impl TimingLayout {
    pub(super) fn new(
        doc: &FamilyDocument,
        model: &TimingModel<'_>,
        _style: &crate::theme::TimingStyle,
    ) -> Self {
        let left_pad = 130;
        let tail_extra = 80;
        let max_label_half_w = 20;
        let right_gutter = 20;
        let row_h = if model.compact_mode { 48 } else { 64 };
        let wave_top_pad = 10;
        let wave_bot_pad = 10;
        let wave_h = row_h - wave_top_pad - wave_bot_pad;
        let axis_h = 48;
        let chart_w = timing_scaled_chart_width(&model.options, model.t_span).unwrap_or(760);
        let right_pad =
            (chart_w as f64 * 0.05) as i32 + tail_extra + max_label_half_w + right_gutter;
        let width = left_pad + chart_w + right_pad;
        let axis_panel_w = width - left_pad - max_label_half_w - right_gutter;
        let content_x_max = left_pad + axis_panel_w;

        let title_h = doc
            .title
            .as_deref()
            .map(|t| (t.lines().count() as i32) * 22 + 10)
            .unwrap_or(0)
            + 14;
        let header_h = doc
            .header
            .as_deref()
            .map(|h| h.lines().count() as i32 * 16 + 8)
            .unwrap_or(0);
        let footer_h = doc
            .footer
            .as_deref()
            .map(|f| f.lines().count() as i32 * 16 + 8)
            .unwrap_or(0);
        let caption_h = doc
            .caption
            .as_deref()
            .map(|c| c.lines().count() as i32 * 16 + 8)
            .unwrap_or(0);

        let n_signals = model.signals.len().max(1) as i32;
        let axis_h_effective = if model.hide_time_axis { 10 } else { axis_h };
        let height =
            header_h + title_h + axis_h_effective + n_signals * row_h + 32 + footer_h + caption_h;

        let mut ty = 14i32;
        if let Some(header) = &doc.header {
            ty += header.lines().count() as i32 * 16;
            ty += 4;
        }
        ty += 8;
        if let Some(title) = &doc.title {
            ty += title.lines().count() as i32 * 22;
        }
        ty += 14;
        let axis_top = ty + 4;
        let signals_top = axis_top + axis_h_effective;

        Self {
            left_pad,
            row_h,
            wave_top_pad,
            wave_h,
            axis_h,
            chart_w,
            width,
            height,
            axis_panel_w,
            content_x_max,
            axis_h_effective,
            axis_top,
            signals_top,
            n_signals,
            t_min: model.t_min,
            t_span: model.t_span,
        }
    }

    pub(super) fn time_to_x(&self, t: i64) -> i32 {
        self.left_pad + ((t - self.t_min) as f64 / self.t_span as f64 * self.chart_w as f64) as i32
    }

    pub(super) fn rows_h(&self) -> i32 {
        self.n_signals * self.row_h
    }

    /// Returns the time value at which all waveforms should end their last state block.
    ///
    /// Guarantees that the end position is at least `tail_extra` pixels past `t_max` on the
    /// canvas, so that the last state block (which starts at `t_max`) always has enough room
    /// to display its label without being clipped at the right edge.
    pub(super) fn waveform_end_t(&self) -> i64 {
        // Ensure the tail is at least `TAIL_EXTRA` canvas pixels past t_max.
        const TAIL_EXTRA_PX: i64 = 80;
        let tail_min_t = (TAIL_EXTRA_PX * self.t_span) / (self.chart_w as i64).max(1);
        // Also keep the 5% minimum for cases where chart_w is very large.
        let five_pct_t = (self.t_span as f64 * 0.05) as i64;
        self.t_min + self.t_span + five_pct_t.max(tail_min_t) + 1
    }
}

fn collect_global_events(events: &[&FamilyNode]) -> Vec<(i64, String)> {
    events
        .iter()
        .filter_map(|e| {
            if e.alias.is_some() {
                return None;
            }
            let t = timing_time_value(&e.name)?;
            let txt = e
                .label
                .clone()
                .or_else(|| e.members.first().map(|m| m.text.clone()))
                .unwrap_or_default();
            if txt.starts_with("__timing:") || parse_timing_range_note(&txt).is_some() {
                return None;
            }
            if txt.is_empty() {
                None
            } else {
                Some((t, txt))
            }
        })
        .collect()
}

fn collect_timing_ranges(events: &[&FamilyNode]) -> Vec<TimingRange> {
    events
        .iter()
        .filter_map(|e| {
            if e.alias.is_some() {
                return None;
            }
            let start = timing_time_value(&e.name)?;
            let txt = e
                .label
                .clone()
                .or_else(|| e.members.first().map(|m| m.text.clone()))
                .unwrap_or_default();
            let (end, label_raw) = parse_timing_range_note(&txt)?;
            let (label, fill_color) = if let Some((lbl, clr)) = label_raw.split_once('\x00') {
                (lbl.to_string(), Some(timing_svg_color(clr)))
            } else {
                (label_raw, None)
            };
            Some(TimingRange {
                start,
                end,
                label,
                fill_color,
            })
        })
        .collect()
}

fn collect_time_values(
    events: &[&FamilyNode],
    ranges: &[TimingRange],
    relations: &[FamilyRelation],
) -> Vec<i64> {
    let mut time_vals: Vec<i64> = events
        .iter()
        .filter_map(|e| timing_time_value(&e.name))
        .collect();
    time_vals.extend(ranges.iter().map(|range| range.end));
    for relation in relations {
        time_vals.extend(timing_relation_time(&relation.from));
        time_vals.extend(timing_relation_time(&relation.to));
    }
    time_vals.sort();
    time_vals.dedup();
    time_vals
}

pub(super) fn timing_control_i64(signal: &FamilyNode, key: &str) -> Option<i64> {
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

pub(super) fn timing_signal_is_analog(signal: &FamilyNode) -> bool {
    signal
        .members
        .iter()
        .any(|member| member.text == "__timing:analog")
}

pub(super) fn timing_svg_color(token: &str) -> String {
    let trimmed = token.trim();
    let Some(hex_or_name) = trimmed.strip_prefix('#') else {
        return trimmed.to_string();
    };
    let valid_hex_len = matches!(hex_or_name.len(), 3 | 6 | 8);
    if valid_hex_len && hex_or_name.bytes().all(|b| b.is_ascii_hexdigit()) {
        trimmed.to_string()
    } else {
        hex_or_name.to_string()
    }
}

pub(super) fn timing_relation_time(endpoint: &str) -> Option<i64> {
    endpoint
        .split_once('@')
        .and_then(|(_, time)| timing_time_value(time.trim()))
}

pub(super) fn timing_relation_endpoint(endpoint: &str) -> Option<(&str, i64)> {
    let (signal, time) = endpoint.split_once('@')?;
    Some((signal.trim(), timing_time_value(time.trim())?))
}

fn timing_scaled_chart_width(options: &BTreeSet<String>, t_span: i64) -> Option<i32> {
    let body = options
        .iter()
        .find_map(|option| option.strip_prefix("scale:"))?;
    let (units, pixels) = body.split_once(" as ")?;
    let units = units.trim().parse::<f64>().ok()?.abs().max(1.0);
    let pixels = pixels
        .split_whitespace()
        .next()?
        .trim()
        .parse::<f64>()
        .ok()?
        .abs()
        .max(1.0);
    Some(
        ((t_span as f64 / units) * pixels)
            .round()
            .clamp(240.0, 1600.0) as i32,
    )
}

pub(super) fn parse_timing_range_note(note: &str) -> Option<(i64, String)> {
    let rest = note.strip_prefix("range:")?;
    let (end, label) = rest.split_once(':').unwrap_or((rest, ""));
    let end = timing_time_value(end.trim().trim_start_matches('@'))?;
    let label = if label.trim().is_empty() {
        "range".to_string()
    } else {
        label.trim().to_string()
    };
    Some((end, label))
}

pub(super) fn timing_time_value(raw: &str) -> Option<i64> {
    let trimmed = raw.trim().trim_start_matches('@');
    if let Ok(value) = trimmed.parse::<i64>() {
        return Some(value);
    }
    parse_timing_hms(trimmed).or_else(|| parse_timing_date(trimmed))
}

fn parse_timing_hms(raw: &str) -> Option<i64> {
    let parts = raw
        .split(':')
        .map(str::trim)
        .map(|part| part.parse::<i64>().ok())
        .collect::<Option<Vec<_>>>()?;
    match parts.as_slice() {
        [hours, minutes, seconds] => Some(
            hours
                .saturating_mul(3600)
                .saturating_add(minutes.saturating_mul(60))
                .saturating_add(*seconds),
        ),
        _ => None,
    }
}

fn parse_timing_date(raw: &str) -> Option<i64> {
    let mut parts = raw.split('/').map(str::trim);
    let year = parts.next()?.parse::<i64>().ok()?;
    let month = parts.next()?.parse::<i64>().ok()?;
    let day = parts.next()?.parse::<i64>().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some(days_from_civil(year, month, day).saturating_mul(86_400))
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let mp = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}
