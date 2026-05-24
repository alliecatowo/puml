use super::*;

pub fn render_chart_svg(document: &ChartDocument) -> String {
    let title_lines: Vec<&str> = document
        .title
        .as_deref()
        .map(|title| title.lines().collect())
        .unwrap_or_default();
    let title_px = title_lines
        .iter()
        .map(|line| estimate_text_width(line, 16))
        .max()
        .unwrap_or(0);
    let width = 780.max(title_px + 80);
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
    if !title_lines.is_empty() {
        for line in &title_lines {
            out.push_str(&format!(
                "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\">{}</text>",
                escape_text(line)
            ));
            y += 22;
        }
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

mod layout;
mod model;
mod parts;
mod svg;

use layout::*;
use model::*;
use parts::*;
use svg::*;
