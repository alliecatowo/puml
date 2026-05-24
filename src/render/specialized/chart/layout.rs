use super::*;
use crate::render::text_metrics::proportional_monospace_width;

#[derive(Clone, Copy)]
pub(super) struct ChartPlotArea {
    pub(super) left: i32,
    pub(super) top: i32,
    pub(super) right: i32,
    pub(super) bottom: i32,
}

pub(super) fn chart_y_for_value(
    value: f64,
    min_value: f64,
    max_value: f64,
    plot: ChartPlotArea,
) -> i32 {
    let value = value.clamp(min_value, max_value);
    let ratio = (value - min_value) / (max_value - min_value);
    plot.bottom - (ratio * ((plot.bottom - plot.top) as f64)) as i32
}

pub(super) fn chart_x_for_value(
    value: f64,
    min_value: f64,
    max_value: f64,
    plot: ChartPlotArea,
) -> i32 {
    let value = value.clamp(min_value, max_value);
    let ratio = (value - min_value) / (max_value - min_value);
    plot.left + (ratio * ((plot.right - plot.left) as f64)) as i32
}

pub(super) fn chart_axis_ticks(
    document: &ChartDocument,
    min_value: f64,
    max_value: f64,
) -> Vec<f64> {
    let Some(step) = document
        .v_axis
        .as_ref()
        .and_then(|axis| axis.tick_step)
        .filter(|step| *step > 0.0)
    else {
        return (0..=4)
            .map(|tick| min_value + ((max_value - min_value) * (tick as f64) / 4.0))
            .collect();
    };
    let mut ticks = Vec::new();
    let mut value = (min_value / step).ceil() * step;
    while value <= max_value + 1e-9 && ticks.len() < 64 {
        ticks.push(value);
        value += step;
    }
    if ticks
        .first()
        .is_none_or(|first| (*first - min_value).abs() > 1e-9)
    {
        ticks.insert(0, min_value);
    }
    if ticks
        .last()
        .is_none_or(|last| (*last - max_value).abs() > 1e-9)
    {
        ticks.push(max_value);
    }
    ticks
}

pub(super) fn estimate_text_width(text: &str, font_size: i32) -> i32 {
    proportional_monospace_width(text, font_size)
}
