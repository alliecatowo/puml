use super::parts::*;
use super::*;

pub(super) fn render_chart_bars(
    out: &mut String,
    document: &ChartDocument,
    series: &[crate::model::ChartSeries],
    categories: &[String],
    plot: ChartPlotArea,
    style: &crate::theme::ChartStyle,
) {
    render_chart_axes(out, document, categories, plot, style);
    if series.is_empty() || categories.is_empty() {
        return;
    }
    let (min_value, max_value) = chart_value_range(document, series);
    let count = categories.len() as i32;
    let avail = (plot.right - plot.left).max(20);
    let band = (avail / count).max(10);
    let group_count = if document.stacked {
        1
    } else {
        series.len().max(1) as i32
    };
    let bar_w = ((band - 8) / group_count).max(4);
    for (cat_idx, category) in categories.iter().enumerate() {
        let band_x = plot.left + (cat_idx as i32) * band;
        let mut stack_pos = 0.0_f64;
        let mut stack_neg = 0.0_f64;
        for (series_idx, item) in series.iter().enumerate() {
            let value = item.values.get(cat_idx).copied().unwrap_or(0.0);
            let bx = band_x
                + 4
                + if document.stacked {
                    0
                } else {
                    (series_idx as i32) * bar_w
                };
            let (from, to) = if document.stacked {
                if value >= 0.0 {
                    let from = stack_pos;
                    stack_pos += value;
                    (from, stack_pos)
                } else {
                    let from = stack_neg;
                    stack_neg += value;
                    (from, stack_neg)
                }
            } else {
                (0.0, value)
            };
            let y1 = chart_y_for_value(from, min_value, max_value, plot);
            let y2 = chart_y_for_value(to, min_value, max_value, plot);
            let by = y1.min(y2);
            let bh = (y1 - y2).abs().max(1);
            let color = chart_series_color(document, item, series_idx, style.bar_color.as_str());
            out.push_str(&format!(
                "<rect x=\"{bx}\" y=\"{by}\" width=\"{bw}\" height=\"{bh}\" fill=\"{color}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
                escape_text(&style.axis_color),
                bx = bx,
                by = by,
                bw = bar_w,
                bh = bh,
                color = escape_text(&color)
            ));
            out.push_str(&format!(
                "<text class=\"chart-value-label\" data-chart-label-mode=\"{}\" x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"#334155\">{}</text>",
                chart_label_mode_name(document.label_mode),
                format_chart_value(value),
                tx = bx + bar_w / 2,
                ty = if value >= 0.0 { by - 4 } else { by + bh + 12 }
            ));
        }
        out.push_str(&format!(
            "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
            escape_text(&style.font_color),
            escape_text(category),
            tx = band_x + band / 2,
            ty = plot.bottom + 16
        ));
    }
}

pub(super) fn render_chart_line(
    out: &mut String,
    document: &ChartDocument,
    series: &[crate::model::ChartSeries],
    categories: &[String],
    plot: ChartPlotArea,
    style: &crate::theme::ChartStyle,
) {
    render_chart_axes(out, document, categories, plot, style);
    if series.is_empty() || categories.is_empty() {
        return;
    }
    let (min_value, max_value) = chart_value_range(document, series);
    let count = categories.len() as i32;
    let step = ((plot.right - plot.left) as f64) / ((count.max(2) - 1) as f64).max(1.0);
    for (series_idx, item) in series.iter().enumerate() {
        let color = chart_series_color(document, item, series_idx, style.line_color.as_str());
        let mut points = String::new();
        for (idx, category) in categories.iter().enumerate() {
            let value = item.values.get(idx).copied().unwrap_or(0.0);
            let px = plot.left + ((idx as f64) * step) as i32;
            let py = chart_y_for_value(value, min_value, max_value, plot);
            if !points.is_empty() {
                points.push(' ');
            }
            points.push_str(&format!("{px},{py}"));
            out.push_str(&format!(
                "<circle class=\"chart-point\" data-chart-value=\"{}\" cx=\"{px}\" cy=\"{py}\" r=\"3\" fill=\"{}\"/>",
                format_chart_value(value),
                escape_text(&color)
            ));
            if !matches!(document.label_mode, ChartLabelMode::None) {
                // Offset label above the point with enough clearance to avoid
                // overlapping with category tick labels (#491).
                out.push_str(&format!(
                    "<text class=\"chart-value-label\" data-chart-label-mode=\"{}\" x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"#334155\">{}</text>",
                    chart_label_mode_name(document.label_mode),
                    format_chart_value(value),
                    tx = px + 14, // shift right to avoid tick label below
                    ty = py - 16  // raise by 16 px to clear axis tick labels
                ));
            }
            if series_idx == 0 {
                out.push_str(&format!(
                    "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
                    escape_text(&style.font_color),
                    escape_text(category),
                    tx = px,
                    ty = plot.bottom + 16
                ));
            }
        }
        out.push_str(&format!(
            "<polyline points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            points,
            escape_text(&color)
        ));
    }
}

pub(super) fn render_chart_area(
    out: &mut String,
    document: &ChartDocument,
    series: &[crate::model::ChartSeries],
    categories: &[String],
    plot: ChartPlotArea,
    style: &crate::theme::ChartStyle,
) {
    render_chart_axes(out, document, categories, plot, style);
    if series.is_empty() || categories.is_empty() {
        return;
    }
    let (min_value, max_value) = chart_value_range(document, series);
    let count = categories.len() as i32;
    let step = ((plot.right - plot.left) as f64) / ((count.max(2) - 1) as f64).max(1.0);
    for (series_idx, item) in series.iter().enumerate() {
        let color = chart_series_color(document, item, series_idx, style.line_color.as_str());
        let mut line_points = String::new();
        let mut fill_points = format!("{},{} ", plot.left, plot.bottom);
        for (idx, category) in categories.iter().enumerate() {
            let value = item.values.get(idx).copied().unwrap_or(0.0);
            let px = plot.left + ((idx as f64) * step) as i32;
            let py = chart_y_for_value(value, min_value, max_value, plot);
            if !line_points.is_empty() {
                line_points.push(' ');
            }
            line_points.push_str(&format!("{px},{py}"));
            fill_points.push_str(&format!("{px},{py} "));
            if series_idx == 0 {
                out.push_str(&format!(
                    "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
                    escape_text(&style.font_color),
                    escape_text(category),
                    tx = px,
                    ty = plot.bottom + 16
                ));
            }
        }
        fill_points.push_str(&format!("{},{}", plot.right, plot.bottom));
        // Filled area polygon with 20% opacity
        out.push_str(&format!(
            "<polygon points=\"{}\" fill=\"{}\" fill-opacity=\"0.2\" stroke=\"none\"/>",
            fill_points,
            escape_text(&color)
        ));
        // Line on top
        out.push_str(&format!(
            "<polyline points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            line_points,
            escape_text(&color)
        ));
        // Dots
        for (idx, _category) in categories.iter().enumerate() {
            let value = item.values.get(idx).copied().unwrap_or(0.0);
            let px = plot.left + ((idx as f64) * step) as i32;
            let py = chart_y_for_value(value, min_value, max_value, plot);
            out.push_str(&format!(
                "<circle class=\"chart-point\" data-chart-value=\"{}\" cx=\"{px}\" cy=\"{py}\" r=\"3\" fill=\"{}\"/>",
                format_chart_value(value),
                escape_text(&color)
            ));
        }
    }
}

pub(super) fn render_chart_scatter(
    out: &mut String,
    document: &ChartDocument,
    series: &[crate::model::ChartSeries],
    categories: &[String],
    plot: ChartPlotArea,
    style: &crate::theme::ChartStyle,
) {
    render_chart_axes(out, document, categories, plot, style);
    if series.is_empty() || categories.is_empty() {
        return;
    }
    let (min_value, max_value) = chart_value_range(document, series);
    let count = categories.len() as i32;
    let step = ((plot.right - plot.left) as f64) / ((count.max(2) - 1) as f64).max(1.0);
    for (series_idx, item) in series.iter().enumerate() {
        let color = chart_series_color(document, item, series_idx, style.series_color.as_str());
        for (idx, category) in categories.iter().enumerate() {
            let value = item.values.get(idx).copied().unwrap_or(0.0);
            let px = plot.left + ((idx as f64) * step) as i32;
            let py = chart_y_for_value(value, min_value, max_value, plot);
            out.push_str(&format!(
                "<circle class=\"chart-point\" data-chart-value=\"{}\" cx=\"{px}\" cy=\"{py}\" r=\"4\" fill=\"{}\" stroke=\"white\" stroke-width=\"1.5\"/>",
                format_chart_value(value),
                escape_text(&color)
            ));
            if series_idx == 0 {
                out.push_str(&format!(
                    "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
                    escape_text(&style.font_color),
                    escape_text(category),
                    tx = px,
                    ty = plot.bottom + 16
                ));
            }
        }
    }
}

pub(super) fn render_chart_horizontal_bars(
    out: &mut String,
    document: &ChartDocument,
    series: &[crate::model::ChartSeries],
    categories: &[String],
    plot: ChartPlotArea,
    style: &crate::theme::ChartStyle,
) {
    render_chart_axes(out, document, categories, plot, style);
    if series.is_empty() || categories.is_empty() {
        return;
    }
    let (min_value, max_value) = chart_value_range(document, series);
    let count = categories.len() as i32;
    let avail = (plot.bottom - plot.top).max(20);
    let band = (avail / count).max(10);
    let group_count = if document.stacked {
        1
    } else {
        series.len().max(1) as i32
    };
    let bar_h = ((band - 8) / group_count).max(4);
    for (cat_idx, category) in categories.iter().enumerate() {
        let band_y = plot.top + (cat_idx as i32) * band;
        let mut stack_pos = 0.0_f64;
        let mut stack_neg = 0.0_f64;
        for (series_idx, item) in series.iter().enumerate() {
            let value = item.values.get(cat_idx).copied().unwrap_or(0.0);
            let (from, to) = if document.stacked {
                if value >= 0.0 {
                    let from = stack_pos;
                    stack_pos += value;
                    (from, stack_pos)
                } else {
                    let from = stack_neg;
                    stack_neg += value;
                    (from, stack_neg)
                }
            } else {
                (0.0, value)
            };
            let x1 = chart_x_for_value(from, min_value, max_value, plot);
            let x2 = chart_x_for_value(to, min_value, max_value, plot);
            let bx = x1.min(x2);
            let bw = (x1 - x2).abs().max(1);
            let by = band_y
                + 4
                + if document.stacked {
                    0
                } else {
                    (series_idx as i32) * bar_h
                };
            let color = chart_series_color(document, item, series_idx, style.bar_color.as_str());
            out.push_str(&format!(
                "<rect x=\"{bx}\" y=\"{by}\" width=\"{bw}\" height=\"{bh}\" fill=\"{color}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
                escape_text(&style.axis_color),
                bx = bx,
                by = by,
                bw = bw,
                bh = bar_h,
                color = escape_text(&color)
            ));
        }
        out.push_str(&format!(
            "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
            escape_text(&style.font_color),
            escape_text(category),
            tx = plot.left - 8,
            ty = band_y + band / 2 + 4
        ));
    }
}

pub(super) fn render_chart_pie(
    document: &ChartDocument,
    out: &mut String,
    data: &[crate::model::ChartPoint],
    cx: i32,
    cy: i32,
    style: &crate::theme::ChartStyle,
) {
    let radius = 120_i32;
    let total: f64 = data.iter().map(|p| p.value.max(0.0)).sum();
    if total <= 0.0 {
        out.push_str(&format!(
            "<circle cx=\"{cx}\" cy=\"{cy}\" r=\"{r}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            style.grid_color,
            style.pie_border_color,
            cx = cx,
            cy = cy,
            r = radius
        ));
        return;
    }
    let mut acc = 0.0_f64;
    // Deterministic angle accumulation using f64.
    for (idx, point) in data.iter().enumerate() {
        let v = point.value.max(0.0);
        let start = acc / total * std::f64::consts::TAU;
        acc += v;
        let end = acc / total * std::f64::consts::TAU;
        let x1 = cx as f64 + (radius as f64) * start.cos();
        let y1 = cy as f64 + (radius as f64) * start.sin();
        let x2 = cx as f64 + (radius as f64) * end.cos();
        let y2 = cy as f64 + (radius as f64) * end.sin();
        let large = if (end - start) > std::f64::consts::PI {
            1
        } else {
            0
        };
        let color = chart_slice_color(document, point, idx, style.series_color.as_str());
        out.push_str(&format!(
            "<path class=\"chart-pie-slice\" data-chart-slice=\"{}\" data-chart-value=\"{}\" data-chart-percent=\"{}\" d=\"M {cx} {cy} L {x1:.2} {y1:.2} A {r} {r} 0 {large} 1 {x2:.2} {y2:.2} Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
            escape_text(&point.label),
            format_chart_value(point.value),
            format_chart_percent(v, total),
            escape_text(&color),
            escape_text(&style.pie_border_color),
            cx = cx,
            cy = cy,
            r = radius,
            x1 = x1,
            y1 = y1,
            x2 = x2,
            y2 = y2,
            large = large
        ));
        let mid = (start + end) / 2.0;
        if !matches!(document.label_mode, ChartLabelMode::None) {
            let label_radius = if matches!(document.label_mode, ChartLabelMode::Outside) {
                1.23
            } else {
                0.6
            };
            let lx = cx as f64 + ((radius as f64) * label_radius) * mid.cos();
            let ly = cy as f64 + ((radius as f64) * label_radius) * mid.sin();
            let label_text = chart_pie_label_text(document.label_mode, point, v, total);
            if matches!(document.label_mode, ChartLabelMode::Outside) {
                let c1x = cx as f64 + ((radius as f64) * 0.82) * mid.cos();
                let c1y = cy as f64 + ((radius as f64) * 0.82) * mid.sin();
                let c2x = cx as f64 + ((radius as f64) * 1.08) * mid.cos();
                let c2y = cy as f64 + ((radius as f64) * 1.08) * mid.sin();
                out.push_str(&format!(
                    "<line class=\"chart-pie-callout\" data-chart-slice-callout=\"{}\" x1=\"{c1x:.0}\" y1=\"{c1y:.0}\" x2=\"{c2x:.0}\" y2=\"{c2y:.0}\" stroke=\"{}\" stroke-width=\"0.75\"/>",
                    escape_text(&point.label),
                    escape_text(&style.axis_color),
                ));
            }
            out.push_str(&format!(
                "<text class=\"chart-pie-label\" data-chart-label-mode=\"{}\" data-chart-slice-label=\"{}\" x=\"{lx:.0}\" y=\"{ly:.0}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
                chart_label_mode_name(document.label_mode),
                escape_text(&point.label),
                escape_text(&style.font_color),
                escape_text(&label_text),
                lx = lx,
                ly = ly
            ));
        }
    }
}
