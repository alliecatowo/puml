use super::*;

pub(super) fn render_chart_axes(
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

pub(super) fn render_chart_legend(
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

pub(super) struct ChartLegendItem<'a> {
    name: &'a str,
    color: String,
}

pub(super) fn render_chart_annotations(
    out: &mut String,
    document: &ChartDocument,
    plot: ChartPlotArea,
) {
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

pub(super) fn render_chart_caption(
    out: &mut String,
    document: &ChartDocument,
    width: i32,
    height: i32,
) {
    if let Some(caption) = &document.caption {
        out.push_str(&format!(
            "<text data-chart-caption=\"true\" x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#475569\">{}</text>",
            width / 2,
            height - 18,
            escape_text(caption)
        ));
    }
}
