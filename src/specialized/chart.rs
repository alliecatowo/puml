// ─── Family 3: @startchart ────────────────────────────────────────────────────

use super::shared::{escape_xml, strip_block, svg_header, svg_white_bg};
use crate::diagnostic::Diagnostic;
use crate::theme::resolve_sequence_theme_preset;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChartType {
    Bar,
    Line,
    Area,
    Scatter,
    Pie,
    Column, // same as bar but explicit
}

#[derive(Debug, Clone)]
struct ChartData {
    label: String,
    value: f64,
}

#[derive(Debug, Clone)]
struct ChartAnnotation {
    target: String,
    text: String,
}

#[derive(Debug, Clone)]
struct ChartRenderOptions {
    background: String,
    axis_color: Option<String>,
    palette: Vec<String>,
    annotations: Vec<ChartAnnotation>,
    caption: Option<String>,
}

impl Default for ChartRenderOptions {
    fn default() -> Self {
        Self {
            background: "white".to_string(),
            axis_color: None,
            palette: Vec::new(),
            annotations: Vec::new(),
            caption: None,
        }
    }
}

pub(super) fn render_chart(source: &str) -> Result<String, Diagnostic> {
    // Parse @startchart <type> header
    let first_line = source.lines().next().unwrap_or("").trim();
    let chart_type_str = first_line
        .to_ascii_lowercase()
        .strip_prefix("@startchart")
        .unwrap_or("")
        .trim()
        .to_string();
    let mut chart_type = match chart_type_str.split_whitespace().next().unwrap_or("") {
        "line" => ChartType::Line,
        "area" => ChartType::Area,
        "scatter" => ChartType::Scatter,
        "pie" => ChartType::Pie,
        "column" => ChartType::Column,
        _ => ChartType::Bar, // "bar" is default
    };

    let (body, _) = strip_block(source, "@startchart", "@endchart");
    let mut title: Option<String> = None;
    let mut data: Vec<ChartData> = Vec::new();
    let mut options = ChartRenderOptions::default();

    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        match lower.as_str() {
            "bar" | "bars" | "bar chart" | "barchart" => {
                chart_type = ChartType::Bar;
                continue;
            }
            "line" | "lines" | "line chart" | "linechart" => {
                chart_type = ChartType::Line;
                continue;
            }
            "area" | "area chart" | "areachart" => {
                chart_type = ChartType::Area;
                continue;
            }
            "scatter" | "scatter chart" | "scatterchart" => {
                chart_type = ChartType::Scatter;
                continue;
            }
            "pie" | "pie chart" | "piechart" => {
                chart_type = ChartType::Pie;
                continue;
            }
            "column" | "column chart" | "columnchart" => {
                chart_type = ChartType::Column;
                continue;
            }
            _ => {}
        }
        if lower.starts_with("title ") {
            title = Some(line[6..].trim().to_string());
            continue;
        }
        if lower.starts_with("caption ") {
            options.caption = Some(line[8..].trim().to_string());
            continue;
        }
        if let Some(theme_name) = line.strip_prefix("!theme ") {
            let preset = resolve_sequence_theme_preset(theme_name).map_err(Diagnostic::error)?;
            options.background = preset
                .style
                .background_color
                .unwrap_or_else(|| "white".to_string());
            options.axis_color = Some(preset.style.arrow_color);
            options.palette = vec![
                preset.style.participant_border_color,
                preset.style.lifeline_border_color,
                preset.style.group_border_color,
            ];
            continue;
        }
        if parse_chart_annotation(line, &mut options.annotations) {
            continue;
        }
        if parse_chart_style(line, &mut options) {
            continue;
        }
        // Parse `"label" : value`, `label : value`, or `"label" value` (legacy)
        let sep_pos = line.rfind(':').or_else(|| {
            // Fall back to splitting on the last whitespace if no colon
            line.rfind(char::is_whitespace)
        });
        if let Some(pos) = sep_pos {
            let label_part = line[..pos]
                .trim_end_matches(':')
                .trim()
                .trim_matches('"')
                .to_string();
            let val_part = line[pos + 1..].trim();
            if let Ok(val) = val_part.parse::<f64>() {
                if !label_part.is_empty() {
                    data.push(ChartData {
                        label: label_part,
                        value: val,
                    });
                }
            }
        }
    }

    if data.is_empty() {
        return Err(Diagnostic::error(
            "[E_CHART_EMPTY] @startchart contains no data rows",
        ));
    }

    let svg = match chart_type {
        ChartType::Bar | ChartType::Column => render_bar_chart(&data, &title, false),
        ChartType::Line => render_line_chart(&data, &title),
        ChartType::Area => render_area_chart(&data, &title),
        ChartType::Scatter => render_scatter_chart(&data, &title),
        ChartType::Pie => render_pie_chart(&data, &title),
    }?;
    let svg = add_chart_type_metadata(svg, chart_type);
    Ok(apply_chart_render_options(svg, &options))
}

fn add_chart_type_metadata(svg: String, chart_type: ChartType) -> String {
    let name = match chart_type {
        ChartType::Bar => "bar",
        ChartType::Line => "line",
        ChartType::Area => "area",
        ChartType::Scatter => "scatter",
        ChartType::Pie => "pie",
        ChartType::Column => "column",
    };
    svg.replace(
        "</svg>",
        &format!("<metadata data-chart-type=\"{name}\"/></svg>"),
    )
}

const CHART_COLORS: &[&str] = &[
    "#4e79a7", "#f28e2b", "#e15759", "#76b7b2", "#59a14f", "#edc948", "#b07aa1", "#ff9da7",
    "#9c755f", "#bab0ac",
];

fn bar_color(idx: usize) -> &'static str {
    CHART_COLORS[idx % CHART_COLORS.len()]
}

fn parse_chart_annotation(line: &str, annotations: &mut Vec<ChartAnnotation>) -> bool {
    let lower = line.to_ascii_lowercase();
    if let Some(rest) = lower
        .strip_prefix("annotation ")
        .or_else(|| lower.strip_prefix("annotate "))
    {
        let source_rest = &line[line.len() - rest.len()..];
        if let Some((target, text)) = source_rest.split_once(':') {
            annotations.push(ChartAnnotation {
                target: target.trim().trim_matches('"').to_string(),
                text: text.trim().trim_matches('"').to_string(),
            });
            return true;
        }
    }
    if let Some(rest) = lower.strip_prefix("note at ") {
        let source_rest = &line[line.len() - rest.len()..];
        if let Some((target, text)) = source_rest.split_once(':') {
            annotations.push(ChartAnnotation {
                target: target.trim().trim_matches('"').to_string(),
                text: text.trim().trim_matches('"').to_string(),
            });
            return true;
        }
    }
    if let Some(rest) = lower.strip_prefix("note ") {
        let source_rest = &line[line.len() - rest.len()..];
        if let Some((text, target)) = source_rest.split_once(" at ") {
            annotations.push(ChartAnnotation {
                target: target.trim().trim_matches('"').to_string(),
                text: text.trim().trim_matches('"').to_string(),
            });
            return true;
        }
    }
    false
}

fn parse_chart_style(line: &str, options: &mut ChartRenderOptions) -> bool {
    let lower = line.to_ascii_lowercase();
    let parts: Vec<&str> = line.split_whitespace().collect();
    if lower.starts_with("skinparam ") && lower.contains("backgroundcolor") {
        if let Some(color) = parts.last().filter(|part| part.starts_with('#')) {
            options.background = (*color).to_string();
        }
        return true;
    }
    if lower.starts_with("skinparam ") && lower.contains("axiscolor") {
        if let Some(color) = parts.last().filter(|part| part.starts_with('#')) {
            options.axis_color = Some((*color).to_string());
        }
        return true;
    }
    if lower.starts_with("skinparam ") && lower.contains("chart") {
        if let Some(color) = parts.last().filter(|part| part.starts_with('#')) {
            options.palette.push((*color).to_string());
        }
        return true;
    }
    if lower.starts_with("palette ") {
        options.palette = parts
            .iter()
            .skip(1)
            .filter(|part| part.starts_with('#'))
            .map(|part| (*part).to_string())
            .collect();
        return true;
    }
    false
}

fn apply_chart_render_options(mut svg: String, options: &ChartRenderOptions) -> String {
    if options.background != "white" {
        svg = svg.replacen(
            "fill=\"white\"/>",
            &format!("fill=\"{}\"/>", escape_xml(&options.background)),
            1,
        );
    }
    if let Some(axis_color) = &options.axis_color {
        svg = svg.replace(
            "stroke=\"#888\"",
            &format!("stroke=\"{}\"", escape_xml(axis_color)),
        );
    }
    let mut additions = String::new();
    if !options.palette.is_empty() {
        additions.push_str(&format!(
            "<metadata data-chart-palette=\"{}\"/>",
            escape_xml(&options.palette.join(" "))
        ));
    }
    let mut y = 34;
    for ann in &options.annotations {
        additions.push_str(&format!(
            "<g data-chart-annotation=\"{}\"><rect x=\"560\" y=\"{}\" width=\"190\" height=\"24\" rx=\"5\" ry=\"5\" fill=\"#fff7ed\" stroke=\"#f97316\" stroke-width=\"1\"/><text x=\"570\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" fill=\"#7c2d12\">{}: {}</text></g>",
            escape_xml(&ann.target),
            y,
            y + 16,
            escape_xml(&ann.target),
            escape_xml(&ann.text)
        ));
        y += 30;
    }
    if let Some(caption) = &options.caption {
        additions.push_str(&format!(
            "<text data-chart-caption=\"true\" x=\"50%\" y=\"96%\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#475569\">{}</text>",
            escape_xml(caption)
        ));
    }
    if additions.is_empty() {
        svg
    } else {
        svg.replace("</svg>", &format!("{additions}</svg>"))
    }
}

fn render_bar_chart(
    data: &[ChartData],
    title: &Option<String>,
    _vertical: bool,
) -> Result<String, Diagnostic> {
    let margin_left = 50i32;
    let margin_right = 20i32;
    let margin_top = if title.is_some() { 48 } else { 20 };
    let margin_bottom = 40i32;
    let chart_w = (data.len() as i32) * 60 + margin_left + margin_right;
    let chart_h = 300i32;
    let plot_w = chart_w - margin_left - margin_right;
    let plot_h = chart_h - margin_top - margin_bottom;

    let max_val = data.iter().map(|d| d.value).fold(0.0f64, f64::max);
    if max_val == 0.0 {
        return Err(Diagnostic::error(
            "[E_CHART_ZERO] chart has all-zero values",
        ));
    }

    let bar_w = (plot_w / data.len() as i32 - 8).max(8);
    let x_step = plot_w / data.len() as i32;

    let mut out = String::new();
    out.push_str(&svg_header(chart_w, chart_h));
    out.push_str(svg_white_bg());

    // Title
    if let Some(t) = title {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"24\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\" text-anchor=\"middle\" fill=\"#222\">{}</text>",
            chart_w / 2, escape_xml(t)
        ));
    }

    // Axes
    let ax = margin_left;
    let ay_top = margin_top;
    let ay_bot = chart_h - margin_bottom;
    let ax_right = chart_w - margin_right;
    out.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#888\" stroke-width=\"1.5\"/>",
        ax, ay_top, ax, ay_bot
    ));
    out.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#888\" stroke-width=\"1.5\"/>",
        ax, ay_bot, ax_right, ay_bot
    ));

    // Y-axis labels
    for i in 0..=4 {
        let val = max_val * (i as f64) / 4.0;
        let y_pos = ay_bot - (plot_h * i / 4);
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#ddd\" stroke-width=\"1\"/>",
            ax, y_pos, ax_right, y_pos
        ));
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" fill=\"#666\" text-anchor=\"end\">{:.0}</text>",
            ax - 4, y_pos + 4, val
        ));
    }

    // Bars
    for (idx, d) in data.iter().enumerate() {
        let bx = ax + idx as i32 * x_step + (x_step - bar_w) / 2;
        let bar_h = ((d.value / max_val) * plot_h as f64) as i32;
        let by = ay_bot - bar_h;
        let color = bar_color(idx);
        out.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" rx=\"2\"/>",
            bx, by, bar_w, bar_h, color
        ));
        // Value label on top
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" text-anchor=\"middle\" fill=\"{}\">{:.0}</text>",
            bx + bar_w / 2, by - 3, color, d.value
        ));
        // X-axis label
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" text-anchor=\"middle\" fill=\"#444\">{}</text>",
            bx + bar_w / 2, ay_bot + 14, escape_xml(&d.label)
        ));
    }

    out.push_str("</svg>");
    Ok(out)
}

fn render_line_chart(data: &[ChartData], title: &Option<String>) -> Result<String, Diagnostic> {
    let margin_left = 50i32;
    let margin_right = 20i32;
    let margin_top = if title.is_some() { 48 } else { 20 };
    let margin_bottom = 40i32;
    let chart_w = (data.len() as i32) * 60 + margin_left + margin_right;
    let chart_h = 300i32;
    let plot_w = chart_w - margin_left - margin_right;
    let plot_h = chart_h - margin_top - margin_bottom;

    let max_val = data.iter().map(|d| d.value).fold(0.0f64, f64::max);
    if max_val == 0.0 {
        return Err(Diagnostic::error(
            "[E_CHART_ZERO] chart has all-zero values",
        ));
    }

    let x_step = plot_w / (data.len() as i32 - 1).max(1);
    let ax = margin_left;
    let ay_top = margin_top;
    let ay_bot = chart_h - margin_bottom;
    let ax_right = chart_w - margin_right;

    let mut out = String::new();
    out.push_str(&svg_header(chart_w, chart_h));
    out.push_str(svg_white_bg());

    if let Some(t) = title {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"24\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\" text-anchor=\"middle\" fill=\"#222\">{}</text>",
            chart_w / 2, escape_xml(t)
        ));
    }

    // Axes
    out.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#888\" stroke-width=\"1.5\"/>",
        ax, ay_top, ax, ay_bot
    ));
    out.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#888\" stroke-width=\"1.5\"/>",
        ax, ay_bot, ax_right, ay_bot
    ));

    // Y-grid
    for i in 0..=4 {
        let val = max_val * (i as f64) / 4.0;
        let y_pos = ay_bot - (plot_h * i / 4);
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#eee\" stroke-width=\"1\"/>",
            ax, y_pos, ax_right, y_pos
        ));
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" fill=\"#666\" text-anchor=\"end\">{:.0}</text>",
            ax - 4, y_pos + 4, val
        ));
    }

    // Compute point coords
    let points: Vec<(i32, i32)> = data
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let px = ax + i as i32 * x_step;
            let py = ay_bot - ((d.value / max_val) * plot_h as f64) as i32;
            (px, py)
        })
        .collect();

    // Polyline
    if points.len() >= 2 {
        let pts_str: String = points
            .iter()
            .map(|(x, y)| format!("{},{}", x, y))
            .collect::<Vec<_>>()
            .join(" ");
        out.push_str(&format!(
            "<polyline points=\"{}\" fill=\"none\" stroke=\"#4e79a7\" stroke-width=\"2\"/>",
            pts_str
        ));
    }

    // Points and labels
    for (i, (px, py)) in points.iter().enumerate() {
        out.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"4\" fill=\"#4e79a7\" stroke=\"white\" stroke-width=\"1.5\"/>",
            px, py
        ));
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" text-anchor=\"middle\" fill=\"#444\">{}</text>",
            px, ay_bot + 14, escape_xml(&data[i].label)
        ));
    }

    out.push_str("</svg>");
    Ok(out)
}

fn render_area_chart(data: &[ChartData], title: &Option<String>) -> Result<String, Diagnostic> {
    let margin_left = 50i32;
    let margin_right = 20i32;
    let margin_top = if title.is_some() { 48 } else { 20 };
    let margin_bottom = 40i32;
    let chart_w = (data.len() as i32) * 60 + margin_left + margin_right;
    let chart_h = 300i32;
    let plot_w = chart_w - margin_left - margin_right;
    let plot_h = chart_h - margin_top - margin_bottom;

    let max_val = data.iter().map(|d| d.value).fold(0.0f64, f64::max);
    if max_val == 0.0 {
        return Err(Diagnostic::error(
            "[E_CHART_ZERO] chart has all-zero values",
        ));
    }

    let x_step = plot_w / (data.len() as i32 - 1).max(1);
    let ax = margin_left;
    let ay_top = margin_top;
    let ay_bot = chart_h - margin_bottom;
    let ax_right = chart_w - margin_right;

    let mut out = String::new();
    out.push_str(&svg_header(chart_w, chart_h));
    out.push_str(svg_white_bg());
    if let Some(t) = title {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"24\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\" text-anchor=\"middle\" fill=\"#222\">{}</text>",
            chart_w / 2, escape_xml(t)
        ));
    }

    out.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#888\" stroke-width=\"1.5\"/>",
        ax, ay_top, ax, ay_bot
    ));
    out.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#888\" stroke-width=\"1.5\"/>",
        ax, ay_bot, ax_right, ay_bot
    ));

    let points: Vec<(i32, i32)> = data
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let px = ax + i as i32 * x_step;
            let py = ay_bot - ((d.value / max_val) * plot_h as f64) as i32;
            (px, py)
        })
        .collect();

    if points.len() >= 2 {
        let mut area_points = format!("{},{} ", points[0].0, ay_bot);
        area_points.push_str(
            &points
                .iter()
                .map(|(x, y)| format!("{},{}", x, y))
                .collect::<Vec<_>>()
                .join(" "),
        );
        area_points.push_str(&format!(" {},{}", points[points.len() - 1].0, ay_bot));
        out.push_str(&format!(
            "<polygon points=\"{}\" fill=\"#4e79a733\" stroke=\"none\"/>",
            area_points
        ));
        out.push_str(&format!(
            "<polyline points=\"{}\" fill=\"none\" stroke=\"#4e79a7\" stroke-width=\"2\"/>",
            points
                .iter()
                .map(|(x, y)| format!("{},{}", x, y))
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }

    for (i, (px, py)) in points.iter().enumerate() {
        out.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"3.5\" fill=\"#4e79a7\" stroke=\"white\" stroke-width=\"1.2\"/>",
            px, py
        ));
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" text-anchor=\"middle\" fill=\"#444\">{}</text>",
            px, ay_bot + 14, escape_xml(&data[i].label)
        ));
    }

    out.push_str("</svg>");
    Ok(out)
}

fn render_scatter_chart(data: &[ChartData], title: &Option<String>) -> Result<String, Diagnostic> {
    let margin_left = 50i32;
    let margin_right = 20i32;
    let margin_top = if title.is_some() { 48 } else { 20 };
    let margin_bottom = 40i32;
    let chart_w = (data.len() as i32) * 60 + margin_left + margin_right;
    let chart_h = 300i32;
    let plot_w = chart_w - margin_left - margin_right;
    let plot_h = chart_h - margin_top - margin_bottom;

    let max_val = data.iter().map(|d| d.value).fold(0.0f64, f64::max);
    if max_val == 0.0 {
        return Err(Diagnostic::error(
            "[E_CHART_ZERO] chart has all-zero values",
        ));
    }

    let x_step = plot_w / (data.len() as i32 - 1).max(1);
    let ax = margin_left;
    let ay_top = margin_top;
    let ay_bot = chart_h - margin_bottom;
    let ax_right = chart_w - margin_right;

    let mut out = String::new();
    out.push_str(&svg_header(chart_w, chart_h));
    out.push_str(svg_white_bg());
    if let Some(t) = title {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"24\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\" text-anchor=\"middle\" fill=\"#222\">{}</text>",
            chart_w / 2, escape_xml(t)
        ));
    }
    out.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#888\" stroke-width=\"1.5\"/>",
        ax, ay_top, ax, ay_bot
    ));
    out.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#888\" stroke-width=\"1.5\"/>",
        ax, ay_bot, ax_right, ay_bot
    ));
    for (i, d) in data.iter().enumerate() {
        let px = ax + i as i32 * x_step;
        let py = ay_bot - ((d.value / max_val) * plot_h as f64) as i32;
        let color = bar_color(i);
        out.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"{}\" fill-opacity=\"0.85\" stroke=\"white\" stroke-width=\"1\"/>",
            px, py, color
        ));
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" text-anchor=\"middle\" fill=\"#444\">{}</text>",
            px, ay_bot + 14, escape_xml(&d.label)
        ));
    }
    out.push_str("</svg>");
    Ok(out)
}

fn render_pie_chart(data: &[ChartData], title: &Option<String>) -> Result<String, Diagnostic> {
    let total: f64 = data.iter().map(|d| d.value).sum();
    if total == 0.0 {
        return Err(Diagnostic::error(
            "[E_CHART_ZERO] pie chart has all-zero values",
        ));
    }

    let legend_w = 180i32;
    let diagram_size = 300i32;
    let chart_w = diagram_size + legend_w + 20;
    let chart_h = diagram_size + 40;
    let cx = diagram_size / 2;
    let cy = diagram_size / 2 + 30;
    let r = (diagram_size / 2 - 20).min(110);

    let mut out = String::new();
    out.push_str(&svg_header(chart_w, chart_h));
    out.push_str(svg_white_bg());

    if let Some(t) = title {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"20\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\" text-anchor=\"middle\" fill=\"#222\">{}</text>",
            diagram_size / 2, escape_xml(t)
        ));
    }

    let mut start_angle: f64 = -90.0_f64.to_radians(); // start from top

    for (idx, d) in data.iter().enumerate() {
        let sweep = (d.value / total) * 2.0 * std::f64::consts::PI;
        let end_angle = start_angle + sweep;
        let large_arc = if sweep > std::f64::consts::PI { 1 } else { 0 };

        let x1 = cx as f64 + r as f64 * start_angle.cos();
        let y1 = cy as f64 + r as f64 * start_angle.sin();
        let x2 = cx as f64 + r as f64 * end_angle.cos();
        let y2 = cy as f64 + r as f64 * end_angle.sin();

        let color = CHART_COLORS[idx % CHART_COLORS.len()];
        out.push_str(&format!(
            "<path d=\"M {} {} L {} {} A {} {} 0 {} 1 {} {} Z\" fill=\"{}\" stroke=\"white\" stroke-width=\"1.5\"/>",
            cx, cy,
            x1 as i32, y1 as i32,
            r, r, large_arc,
            x2 as i32, y2 as i32,
            color
        ));

        // Mid-angle for label positioning
        let mid_angle = start_angle + sweep / 2.0;
        let label_r = r as f64 * 0.65;
        let lx = cx as f64 + label_r * mid_angle.cos();
        let ly = cy as f64 + label_r * mid_angle.sin();
        let pct = (d.value / total * 100.0) as i32;
        if pct >= 5 {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" text-anchor=\"middle\" dominant-baseline=\"middle\" fill=\"white\" font-weight=\"600\">{}%</text>",
                lx as i32, ly as i32, pct
            ));
        }

        start_angle = end_angle;
    }

    // Legend
    let legend_x = diagram_size + 10;
    let mut legend_y = 40;
    out.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" font-weight=\"600\" fill=\"#333\">Legend</text>",
        legend_x, legend_y
    ));
    legend_y += 18;
    for (idx, d) in data.iter().enumerate() {
        let color = CHART_COLORS[idx % CHART_COLORS.len()];
        out.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"12\" height=\"12\" fill=\"{}\"/>",
            legend_x,
            legend_y - 10,
            color
        ));
        let label = format!("{} ({:.0})", d.label, d.value);
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#333\">{}</text>",
            legend_x + 18, legend_y, escape_xml(&label)
        ));
        legend_y += 18;
    }

    out.push_str("</svg>");
    Ok(out)
}
