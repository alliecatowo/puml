use super::*;

pub fn render_chart_svg(document: &ChartDocument) -> String {
    let width = 780;
    let height = 420;
    let style = &document.style;
    let series = effective_chart_series(document);
    let categories = effective_chart_categories(document, &series);
    let type_name = chart_subtype_name(document.subtype);
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\" data-chart-type=\"{type_name}\" data-chart-horizontal=\"{}\" data-chart-stacked=\"{}\">",
        document.horizontal,
        document.stacked,
        w = width,
        h = height
    ));
    out.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>",
        escape_text(&style.background_color)
    ));
    out.push_str(&format!(
        "<metadata data-chart-style=\"{} {} {} {} {} {} {} {}\"/>",
        escape_text(&style.background_color),
        escape_text(&style.axis_color),
        escape_text(&style.grid_color),
        escape_text(&style.series_color),
        escape_text(&style.bar_color),
        escape_text(&style.line_color),
        escape_text(&style.pie_border_color),
        escape_text(&style.font_color)
    ));
    let mut y = 28;
    if let Some(title) = &document.title {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\">{}</text>",
            escape_text(title)
        ));
        y += 22;
    }
    // Suppress visible type-name label (#488) — it leaks into the axis title slot.
    // The type is already encoded in the SVG root attribute data-chart-type.
    out.push_str(&format!(
        "<metadata data-chart-subtype-label=\"{}\"/>",
        escape_text(type_name)
    ));
    if !document.palette.is_empty() {
        out.push_str(&format!(
            "<metadata data-chart-palette=\"{}\"/>",
            escape_text(&document.palette.join(" "))
        ));
    }
    if !series.is_empty() {
        let names = series
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>()
            .join("|");
        out.push_str(&format!(
            "<metadata data-chart-series=\"{}\"/>",
            escape_text(&names)
        ));
    }
    out.push_str(&format!(
        "<metadata data-chart-label-mode=\"{}\"/>",
        chart_label_mode_name(document.label_mode)
    ));
    let legend_visible = chart_legend_visible(document, &series);
    let legend_left = legend_visible && document.legend.h_align == crate::model::LegendHAlign::Left;
    let legend_right =
        legend_visible && document.legend.h_align == crate::model::LegendHAlign::Right;
    let legend_bottom =
        legend_visible && document.legend.v_align == crate::model::LegendVAlign::Bottom;
    let plot_top =
        y + if legend_visible && document.legend.v_align == crate::model::LegendVAlign::Top {
            54
        } else {
            16
        };
    let plot_bottom = height - if legend_bottom { 122 } else { 74 };
    let plot_left = if legend_left { 218 } else { 78 };
    let plot_right = width - if legend_right { 178 } else { 40 };
    let plot = ChartPlotArea {
        left: plot_left,
        top: plot_top,
        right: plot_right,
        bottom: plot_bottom,
    };
    match document.subtype {
        ChartSubtype::Bar if document.horizontal => {
            render_chart_horizontal_bars(&mut out, document, &series, &categories, plot, style)
        }
        ChartSubtype::Bar => {
            render_chart_bars(&mut out, document, &series, &categories, plot, style)
        }
        ChartSubtype::Line => {
            render_chart_line(&mut out, document, &series, &categories, plot, style)
        }
        ChartSubtype::Area => {
            render_chart_area(&mut out, document, &series, &categories, plot, style)
        }
        ChartSubtype::Scatter => {
            render_chart_scatter(&mut out, document, &series, &categories, plot, style)
        }
        ChartSubtype::Pie => {
            let points = effective_chart_points(document, &series, &categories);
            render_chart_pie(
                document,
                &mut out,
                &points,
                width / 2,
                (plot_top + plot_bottom) / 2,
                style,
            )
        }
    }
    render_chart_annotations(&mut out, document, plot);
    render_chart_caption(&mut out, document, width, height);
    render_chart_legend(&mut out, document, &series, plot);
    out.push_str("</svg>");
    out
}

const CHART_PALETTE: &[&str] = &[
    "#1d4ed8", "#16a34a", "#d97706", "#7c3aed", "#0891b2", "#dc2626", "#0f172a", "#facc15",
];

#[derive(Clone, Copy)]
struct ChartPlotArea {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

fn render_chart_bars(
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

fn render_chart_line(
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

fn render_chart_area(
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

fn render_chart_scatter(
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

fn render_chart_horizontal_bars(
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

fn render_chart_pie(
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

fn render_chart_axes(
    out: &mut String,
    document: &ChartDocument,
    categories: &[String],
    plot: ChartPlotArea,
    style: &crate::theme::ChartStyle,
) {
    let h_axis = document.h_axis.as_ref();
    let v_axis = document.v_axis.as_ref();
    let h_axis_color = chart_axis_color(h_axis, &style.axis_color);
    let v_axis_color = chart_axis_color(v_axis, &style.axis_color);
    let h_label_color = chart_axis_label_color(h_axis, &style.font_color);
    let v_label_color = chart_axis_label_color(v_axis, &style.font_color);
    let h_grid_color = chart_axis_grid_color(h_axis, &style.grid_color);
    let v_grid_color = chart_axis_grid_color(v_axis, &style.grid_color);
    out.push_str(&format!(
        "<line x1=\"{l}\" y1=\"{b}\" x2=\"{r}\" y2=\"{b}\" stroke=\"{}\" stroke-width=\"1\"/>",
        escape_text(h_axis_color),
        l = plot.left,
        r = plot.right,
        b = plot.bottom
    ));
    out.push_str(&format!(
        "<line x1=\"{l}\" y1=\"{t}\" x2=\"{l}\" y2=\"{b}\" stroke=\"{}\" stroke-width=\"1\"/>",
        escape_text(v_axis_color),
        l = plot.left,
        t = plot.top,
        b = plot.bottom
    ));
    let series = effective_chart_series(document);
    let (min_value, max_value) = chart_value_range(document, &series);
    let ticks = chart_axis_ticks(document, min_value, max_value);
    for value in ticks {
        let y = chart_y_for_value(value, min_value, max_value, plot);
        out.push_str(&format!(
            "<line class=\"chart-axis-grid chart-axis-grid-v\" x1=\"{l}\" y1=\"{y}\" x2=\"{r}\" y2=\"{y}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
            escape_text(v_grid_color),
            l = plot.left,
            r = plot.right
        ));
        out.push_str(&format!(
            "<text class=\"chart-axis-tick chart-axis-tick-v\" data-chart-axis-tick=\"{}\" x=\"{x}\" y=\"{ty}\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"10\" fill=\"{}\">{}</text>",
            format_chart_value(value),
            escape_text(v_label_color),
            format_chart_value(value),
            x = plot.left - 8,
            ty = y + 4
        ));
    }
    out.push_str(&format!(
        "<metadata data-chart-axis-v-range=\"{}..{}\"/>",
        format_chart_value(min_value),
        format_chart_value(max_value)
    ));
    render_chart_axis_metadata(out, "h", h_axis);
    render_chart_axis_metadata(out, "v", v_axis);
    if min_value <= 0.0 && max_value >= 0.0 {
        if document.horizontal {
            let x = chart_x_for_value(0.0, min_value, max_value, plot);
            out.push_str(&format!(
                "<line class=\"chart-zero-axis\" x1=\"{x}\" y1=\"{t}\" x2=\"{x}\" y2=\"{b}\" stroke=\"{}\" stroke-width=\"1.25\"/>",
                escape_text(v_axis_color),
                t = plot.top,
                b = plot.bottom
            ));
        } else {
            let y = chart_y_for_value(0.0, min_value, max_value, plot);
            out.push_str(&format!(
                "<line class=\"chart-zero-axis\" x1=\"{l}\" y1=\"{y}\" x2=\"{r}\" y2=\"{y}\" stroke=\"{}\" stroke-width=\"1.25\"/>",
                escape_text(v_axis_color),
                l = plot.left,
                r = plot.right
            ));
        }
    }
    if let Some(axis) = &document.h_axis {
        if let Some(label) = &axis.label {
            out.push_str(&format!(
                "<text x=\"{x}\" y=\"{y}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\" fill=\"{}\">{}</text>",
                escape_text(h_label_color),
                escape_text(label),
                x = (plot.left + plot.right) / 2,
                y = plot.bottom + 42
            ));
        }
    }
    if let Some(axis) = &document.v_axis {
        if let Some(label) = &axis.label {
            out.push_str(&format!(
                "<text x=\"18\" y=\"{y}\" transform=\"rotate(-90 18 {y})\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\" fill=\"{}\">{}</text>",
                escape_text(v_label_color),
                escape_text(label),
                y = (plot.top + plot.bottom) / 2
            ));
        }
    }
    if document.horizontal {
        return;
    }
    if categories.len() > 1 {
        let step =
            ((plot.right - plot.left) as f64) / ((categories.len() as i32 - 1).max(1) as f64);
        for idx in 0..categories.len() {
            let x = plot.left + ((idx as f64) * step) as i32;
            out.push_str(&format!(
                "<line class=\"chart-axis-grid chart-axis-grid-h\" x1=\"{x}\" y1=\"{t}\" x2=\"{x}\" y2=\"{b}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
                escape_text(h_grid_color),
                t = plot.top,
                b = plot.bottom
            ));
        }
    }
}

fn render_chart_legend(
    out: &mut String,
    document: &ChartDocument,
    series: &[crate::model::ChartSeries],
    plot: ChartPlotArea,
) {
    if !chart_legend_visible(document, series) {
        return;
    }
    let pie_points;
    let legend_items: Vec<ChartLegendItem<'_>> = if document.subtype == ChartSubtype::Pie {
        let categories = effective_chart_categories(document, series);
        pie_points = effective_chart_points(document, series, &categories);
        pie_points
            .iter()
            .enumerate()
            .map(|(idx, point)| ChartLegendItem {
                name: point.label.as_str(),
                color: chart_slice_color(document, point, idx, "#1d4ed8"),
            })
            .collect()
    } else {
        series
            .iter()
            .enumerate()
            .map(|(idx, item)| ChartLegendItem {
                name: item.name.as_str(),
                color: chart_series_color(document, item, idx, "#1d4ed8"),
            })
            .collect()
    };
    if legend_items.is_empty() {
        return;
    }
    let x = match document.legend.h_align {
        crate::model::LegendHAlign::Left => 24,
        crate::model::LegendHAlign::Center => ((plot.left + plot.right) / 2) - 66,
        crate::model::LegendHAlign::Right => plot.right + 20,
    };
    let y = match document.legend.v_align {
        crate::model::LegendVAlign::Top => (plot.top - 44).max(44),
        crate::model::LegendVAlign::Bottom => plot.bottom + 46,
    };
    let width = 132;
    let height = 18 + (legend_items.len() as i32) * 18;
    let background = document
        .legend
        .background_color
        .as_deref()
        .unwrap_or("#ffffff");
    let border = document.legend.border_color.as_deref().unwrap_or("#cbd5e1");
    let text_color = document.legend.text_color.as_deref().unwrap_or("#0f172a");
    out.push_str(&format!(
        "<g class=\"chart-legend\" data-chart-legend=\"{}\" data-chart-legend-h=\"{}\" data-chart-legend-v=\"{}\"><rect x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" rx=\"4\" fill=\"{}\" stroke=\"{}\"/>",
        chart_legend_position(document),
        chart_legend_h_name(document.legend.h_align),
        chart_legend_v_name(document.legend.v_align),
        escape_text(background),
        escape_text(border)
    ));
    for (idx, item) in legend_items.iter().enumerate() {
        let cy = y + 18 + (idx as i32) * 18;
        out.push_str(&format!(
            "<rect class=\"chart-legend-swatch\" x=\"{x1}\" y=\"{y1}\" width=\"10\" height=\"10\" fill=\"{}\"/><text class=\"chart-legend-label\" x=\"{tx}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
            escape_text(&item.color),
            escape_text(text_color),
            escape_text(item.name),
            x1 = x + 8,
            y1 = cy - 9,
            tx = x + 24,
            ty = cy
        ));
    }
    out.push_str("</g>");
}

struct ChartLegendItem<'a> {
    name: &'a str,
    color: String,
}

fn render_chart_annotations(out: &mut String, document: &ChartDocument, plot: ChartPlotArea) {
    if document.annotations.is_empty() {
        return;
    }
    let mut y = plot.top + 8;
    for annotation in &document.annotations {
        out.push_str(&format!(
            "<g data-chart-annotation=\"{}\"><rect x=\"{x}\" y=\"{y}\" width=\"190\" height=\"24\" rx=\"5\" ry=\"5\" fill=\"#fff7ed\" stroke=\"#f97316\" stroke-width=\"1\"/><text x=\"{tx}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"10\" fill=\"#7c2d12\">{}: {}</text></g>",
            escape_text(&annotation.target),
            escape_text(&annotation.target),
            escape_text(&annotation.text),
            x = plot.right - 196,
            y = y,
            tx = plot.right - 186,
            ty = y + 16
        ));
        y += 30;
    }
}

fn render_chart_caption(out: &mut String, document: &ChartDocument, width: i32, height: i32) {
    if let Some(caption) = &document.caption {
        out.push_str(&format!(
            "<text data-chart-caption=\"true\" x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#475569\">{}</text>",
            width / 2,
            height - 18,
            escape_text(caption)
        ));
    }
}

fn effective_chart_series(document: &ChartDocument) -> Vec<crate::model::ChartSeries> {
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

fn effective_chart_categories(
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

fn effective_chart_points(
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

fn chart_value_range(document: &ChartDocument, series: &[crate::model::ChartSeries]) -> (f64, f64) {
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

fn chart_y_for_value(value: f64, min_value: f64, max_value: f64, plot: ChartPlotArea) -> i32 {
    let value = value.clamp(min_value, max_value);
    let ratio = (value - min_value) / (max_value - min_value);
    plot.bottom - (ratio * ((plot.bottom - plot.top) as f64)) as i32
}

fn chart_x_for_value(value: f64, min_value: f64, max_value: f64, plot: ChartPlotArea) -> i32 {
    let value = value.clamp(min_value, max_value);
    let ratio = (value - min_value) / (max_value - min_value);
    plot.left + (ratio * ((plot.right - plot.left) as f64)) as i32
}

fn chart_axis_ticks(document: &ChartDocument, min_value: f64, max_value: f64) -> Vec<f64> {
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

fn chart_axis_color<'a>(axis: Option<&'a crate::model::ChartAxis>, fallback: &'a str) -> &'a str {
    axis.and_then(|axis| axis.color.as_deref())
        .unwrap_or(fallback)
}

fn chart_axis_label_color<'a>(
    axis: Option<&'a crate::model::ChartAxis>,
    fallback: &'a str,
) -> &'a str {
    axis.and_then(|axis| axis.label_color.as_deref())
        .unwrap_or(fallback)
}

fn chart_axis_grid_color<'a>(
    axis: Option<&'a crate::model::ChartAxis>,
    fallback: &'a str,
) -> &'a str {
    axis.and_then(|axis| axis.grid_color.as_deref())
        .unwrap_or(fallback)
}

fn render_chart_axis_metadata(
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

fn chart_series_color(
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

fn chart_slice_color(
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

fn chart_legend_visible(document: &ChartDocument, series: &[crate::model::ChartSeries]) -> bool {
    document.legend.visible
        || (!document.legend.explicit
            && (series.len() > 1
                || (document.subtype == ChartSubtype::Pie && document.data.len() > 1)))
}

fn chart_legend_position(document: &ChartDocument) -> &'static str {
    match (document.legend.v_align, document.legend.h_align) {
        (crate::model::LegendVAlign::Top, crate::model::LegendHAlign::Left) => "top-left",
        (crate::model::LegendVAlign::Top, crate::model::LegendHAlign::Center) => "top",
        (crate::model::LegendVAlign::Top, crate::model::LegendHAlign::Right) => "right",
        (crate::model::LegendVAlign::Bottom, crate::model::LegendHAlign::Left) => "bottom-left",
        (crate::model::LegendVAlign::Bottom, crate::model::LegendHAlign::Center) => "bottom",
        (crate::model::LegendVAlign::Bottom, crate::model::LegendHAlign::Right) => "bottom-right",
    }
}

fn chart_subtype_name(subtype: ChartSubtype) -> &'static str {
    match subtype {
        ChartSubtype::Bar => "bar",
        ChartSubtype::Line => "line",
        ChartSubtype::Pie => "pie",
        ChartSubtype::Area => "area",
        ChartSubtype::Scatter => "scatter",
    }
}

fn format_chart_value(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{}", v as i64)
    } else {
        format!("{:.2}", v)
    }
}

fn format_chart_percent(value: f64, total: f64) -> String {
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

fn chart_label_mode_name(mode: ChartLabelMode) -> &'static str {
    match mode {
        ChartLabelMode::Auto => "auto",
        ChartLabelMode::Inside => "inside",
        ChartLabelMode::Outside => "outside",
        ChartLabelMode::None => "none",
        ChartLabelMode::Value => "value",
        ChartLabelMode::Percent => "percent",
    }
}

fn chart_pie_label_text(
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

fn chart_legend_h_name(value: LegendHAlign) -> &'static str {
    match value {
        LegendHAlign::Left => "left",
        LegendHAlign::Center => "center",
        LegendHAlign::Right => "right",
    }
}

fn chart_legend_v_name(value: LegendVAlign) -> &'static str {
    match value {
        LegendVAlign::Top => "top",
        LegendVAlign::Bottom => "bottom",
    }
}
