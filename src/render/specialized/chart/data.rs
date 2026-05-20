use super::*;

pub(super) fn effective_chart_series(document: &ChartDocument) -> Vec<crate::model::ChartSeries> {
    if !document.series.is_empty() {
        return document.series.clone();
    }
    if document.data.is_empty() {
        return Vec::new();
    }
    vec![crate::model::ChartSeries {
        name: "Value".to_string(),
        values: document.data.iter().map(|p| p.value).collect(),
        color: None,
    }]
}

pub(super) fn effective_chart_categories(
    document: &ChartDocument,
    series: &[crate::model::ChartSeries],
) -> Vec<String> {
    if let Some(axis) = &document.h_axis {
        if !axis.categories.is_empty() {
            return axis.categories.clone();
        }
    }
    if !document.data.is_empty() {
        return document.data.iter().map(|p| p.label.clone()).collect();
    }
    let count = series.iter().map(|s| s.values.len()).max().unwrap_or(0);
    (1..=count).map(|idx| idx.to_string()).collect()
}

pub(super) fn effective_chart_points(
    document: &ChartDocument,
    series: &[crate::model::ChartSeries],
    categories: &[String],
) -> Vec<crate::model::ChartPoint> {
    if !document.data.is_empty() {
        return document.data.clone();
    }
    let Some(first_series) = series.first() else {
        return Vec::new();
    };
    first_series
        .values
        .iter()
        .enumerate()
        .map(|(idx, value)| crate::model::ChartPoint {
            label: categories
                .get(idx)
                .cloned()
                .unwrap_or_else(|| (idx + 1).to_string()),
            value: *value,
            color: first_series.color.clone(),
        })
        .collect()
}

pub(super) fn chart_value_range(
    document: &ChartDocument,
    series: &[crate::model::ChartSeries],
) -> (f64, f64) {
    let axis_min = document.v_axis.as_ref().and_then(|axis| axis.min);
    let axis_max = document.v_axis.as_ref().and_then(|axis| axis.max);
    let (computed_min, computed_max) = if document.stacked {
        let categories = series.iter().map(|s| s.values.len()).max().unwrap_or(0);
        let mut min_value = 0.0_f64;
        let mut max_value = 0.0_f64;
        for idx in 0..categories {
            let mut positive = 0.0_f64;
            let mut negative = 0.0_f64;
            for value in series
                .iter()
                .map(|s| s.values.get(idx).copied().unwrap_or(0.0))
            {
                if value >= 0.0 {
                    positive += value;
                } else {
                    negative += value;
                }
            }
            min_value = min_value.min(negative);
            max_value = max_value.max(positive);
        }
        (min_value, max_value)
    } else {
        let mut values = series
            .iter()
            .flat_map(|s| s.values.iter().copied())
            .peekable();
        if values.peek().is_none() {
            (0.0, 1.0)
        } else {
            values.fold((0.0_f64, 0.0_f64), |(min_value, max_value), value| {
                (min_value.min(value), max_value.max(value))
            })
        }
    };
    let min = axis_min.unwrap_or(computed_min.min(0.0));
    let max = axis_max
        .unwrap_or(computed_max.max(0.0).max(1.0))
        .max(min + 1.0);
    (min, max)
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

pub(super) fn chart_axis_color<'a>(
    axis: Option<&'a crate::model::ChartAxis>,
    fallback: &'a str,
) -> &'a str {
    axis.and_then(|axis| axis.color.as_deref())
        .unwrap_or(fallback)
}

pub(super) fn chart_axis_label_color<'a>(
    axis: Option<&'a crate::model::ChartAxis>,
    fallback: &'a str,
) -> &'a str {
    axis.and_then(|axis| axis.label_color.as_deref())
        .unwrap_or(fallback)
}

pub(super) fn chart_axis_grid_color<'a>(
    axis: Option<&'a crate::model::ChartAxis>,
    fallback: &'a str,
) -> &'a str {
    axis.and_then(|axis| axis.grid_color.as_deref())
        .unwrap_or(fallback)
}

pub(super) fn render_chart_axis_metadata(
    out: &mut String,
    name: &str,
    axis: Option<&crate::model::ChartAxis>,
) {
    let Some(axis) = axis else {
        return;
    };
    if let Some(color) = &axis.color {
        out.push_str(&format!(
            "<metadata data-chart-axis-{name}-color=\"{}\"/>",
            escape_text(color)
        ));
    }
    if let Some(color) = &axis.label_color {
        out.push_str(&format!(
            "<metadata data-chart-axis-{name}-text=\"{}\"/>",
            escape_text(color)
        ));
    }
    if let Some(color) = &axis.grid_color {
        out.push_str(&format!(
            "<metadata data-chart-axis-{name}-grid=\"{}\"/>",
            escape_text(color)
        ));
    }
}

pub(super) fn chart_series_color(
    document: &ChartDocument,
    series: &crate::model::ChartSeries,
    idx: usize,
    first_fallback: &str,
) -> String {
    series.color.clone().unwrap_or_else(|| {
        if let Some(color) = document.palette.get(idx) {
            return color.clone();
        }
        if idx == 0 {
            first_fallback.to_string()
        } else {
            CHART_PALETTE[idx % CHART_PALETTE.len()].to_string()
        }
    })
}

pub(super) fn chart_slice_color(
    document: &ChartDocument,
    point: &crate::model::ChartPoint,
    idx: usize,
    first_fallback: &str,
) -> String {
    point.color.clone().unwrap_or_else(|| {
        document.palette.get(idx).cloned().unwrap_or_else(|| {
            if idx == 0 {
                first_fallback.to_string()
            } else {
                CHART_PALETTE[idx % CHART_PALETTE.len()].to_string()
            }
        })
    })
}

pub(super) fn chart_legend_visible(
    document: &ChartDocument,
    series: &[crate::model::ChartSeries],
) -> bool {
    document.legend.visible
        || (!document.legend.explicit
            && (series.len() > 1
                || (document.subtype == ChartSubtype::Pie && document.data.len() > 1)))
}

pub(super) fn chart_legend_position(document: &ChartDocument) -> &'static str {
    match (document.legend.v_align, document.legend.h_align) {
        (crate::model::LegendVAlign::Top, crate::model::LegendHAlign::Left) => "top-left",
        (crate::model::LegendVAlign::Top, crate::model::LegendHAlign::Center) => "top",
        (crate::model::LegendVAlign::Top, crate::model::LegendHAlign::Right) => "right",
        (crate::model::LegendVAlign::Bottom, crate::model::LegendHAlign::Left) => "bottom-left",
        (crate::model::LegendVAlign::Bottom, crate::model::LegendHAlign::Center) => "bottom",
        (crate::model::LegendVAlign::Bottom, crate::model::LegendHAlign::Right) => "bottom-right",
    }
}

pub(super) fn chart_subtype_name(subtype: ChartSubtype) -> &'static str {
    match subtype {
        ChartSubtype::Bar => "bar",
        ChartSubtype::Line => "line",
        ChartSubtype::Pie => "pie",
        ChartSubtype::Area => "area",
        ChartSubtype::Scatter => "scatter",
    }
}

pub(super) fn format_chart_value(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{}", v as i64)
    } else {
        format!("{:.2}", v)
    }
}

pub(super) fn format_chart_percent(value: f64, total: f64) -> String {
    if total <= 0.0 {
        return "0%".to_string();
    }
    let pct = value.max(0.0) / total * 100.0;
    if (pct - pct.round()).abs() < 1e-9 {
        format!("{}%", pct as i64)
    } else {
        format!("{pct:.1}%")
    }
}

pub(super) fn chart_label_mode_name(mode: ChartLabelMode) -> &'static str {
    match mode {
        ChartLabelMode::Auto => "auto",
        ChartLabelMode::Inside => "inside",
        ChartLabelMode::Outside => "outside",
        ChartLabelMode::None => "none",
        ChartLabelMode::Value => "value",
        ChartLabelMode::Percent => "percent",
    }
}

pub(super) fn chart_pie_label_text(
    mode: ChartLabelMode,
    point: &crate::model::ChartPoint,
    value: f64,
    total: f64,
) -> String {
    match mode {
        ChartLabelMode::Value => format!("{} {}", point.label, format_chart_value(point.value)),
        ChartLabelMode::Percent => {
            format!("{} {}", point.label, format_chart_percent(value, total))
        }
        ChartLabelMode::None => String::new(),
        ChartLabelMode::Auto | ChartLabelMode::Inside | ChartLabelMode::Outside => {
            format!("{} {}", point.label, format_chart_percent(value, total))
        }
    }
}

pub(super) fn chart_legend_h_name(value: LegendHAlign) -> &'static str {
    match value {
        LegendHAlign::Left => "left",
        LegendHAlign::Center => "center",
        LegendHAlign::Right => "right",
    }
}

pub(super) fn chart_legend_v_name(value: LegendVAlign) -> &'static str {
    match value {
        LegendVAlign::Top => "top",
        LegendVAlign::Bottom => "bottom",
    }
}

pub(super) fn estimate_text_width(text: &str, font_size: i32) -> i32 {
    ((text.chars().count() as i32) * font_size * 3) / 5
}
