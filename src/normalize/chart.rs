mod parse;

use parse::*;

use super::*;

pub(super) fn normalize_chart(document: Document) -> Result<ChartDocument, Diagnostic> {
    let (title, body) = collect_raw_body(&document);
    let mut caption = None;
    let mut subtype = ChartSubtype::Bar;
    let mut data = Vec::new();
    let mut h_axis: Option<ChartAxis> = None;
    let mut v_axis: Option<ChartAxis> = None;
    let mut series: Vec<ChartSeries> = Vec::new();
    let mut legend = ChartLegend::default();
    let mut legend_entries: Vec<ChartLegendEntry> = Vec::new();
    let mut palette = Vec::new();
    let mut annotations = Vec::new();
    let mut label_mode = crate::model::ChartLabelMode::Auto;
    let mut horizontal = false;
    let mut stacked = false;
    let mut style = ChartStyle::default();
    let mut monochrome_mode = None;
    let mut warnings: Vec<Diagnostic> = Vec::new();
    let mut first_non_empty = true;
    let mut in_legend_block = false;
    let mut rows: Vec<ChartDataRow> = Vec::new();
    for line in body {
        let line = line.trim();
        if line.is_empty() || line.starts_with('\'') {
            continue;
        }
        if in_legend_block {
            if let Some(entry) = parse_chart_legend_entry(line) {
                legend_entries.push(entry);
                continue;
            }
            in_legend_block = false;
        }
        if let Some(theme_name) = line.strip_prefix("!theme ") {
            style = chart_style_from_sequence_theme(
                &resolve_sequence_theme_preset(theme_name)
                    .map_err(Diagnostic::error)?
                    .style,
            );
            continue;
        }
        if line.to_ascii_lowercase().starts_with("skinparam ") {
            let rest = line[10..].trim();
            let mut parts = rest.splitn(2, char::is_whitespace);
            let key = parts.next().unwrap_or("").trim();
            let value = parts.next().unwrap_or("").trim();
            if key.eq_ignore_ascii_case("monochrome") {
                match classify_sequence_skinparam(key, value) {
                    SequenceSkinParamSupport::SupportedNoop => {}
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::Monochrome(mode),
                    ) => monochrome_mode = Some(mode),
                    _ => warnings.push(Diagnostic::warning(format!(
                        "[W_SKINPARAM_UNSUPPORTED_VALUE] unsupported value `{}` for skinparam `{}`",
                        value, key
                    ))),
                }
                continue;
            }
            if key.eq_ignore_ascii_case("handwritten") || key.eq_ignore_ascii_case("sepia") {
                match classify_sequence_skinparam(key, value) {
                    SequenceSkinParamSupport::SupportedNoop
                    | SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::Handwritten(_),
                    )
                    | SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::Sepia(_),
                    ) => {}
                    _ => warnings.push(Diagnostic::warning(format!(
                        "[W_SKINPARAM_UNSUPPORTED_VALUE] unsupported value `{}` for skinparam `{}`",
                        value, key
                    ))),
                }
                continue;
            }
            use crate::theme::ChartSkinParamValue;
            match classify_chart_skinparam(key, value) {
                SkinParamSupport::SupportedNoop => {}
                SkinParamSupport::SupportedWithValue(v) => match v {
                    ChartSkinParamValue::BackgroundColor(c) => style.background_color = c,
                    ChartSkinParamValue::AxisColor(c) => style.axis_color = c,
                    ChartSkinParamValue::GridColor(c) => style.grid_color = c,
                    ChartSkinParamValue::SeriesColor(c) => style.series_color = c,
                    ChartSkinParamValue::BarColor(c) => style.bar_color = c,
                    ChartSkinParamValue::LineColor(c) => style.line_color = c,
                    ChartSkinParamValue::PieBorderColor(c) => style.pie_border_color = c,
                    ChartSkinParamValue::FontColor(c) => style.font_color = c,
                },
                SkinParamSupport::UnsupportedKey => warnings.push(Diagnostic::warning(format!(
                    "[W_SKINPARAM_UNSUPPORTED] unsupported skinparam `{}`",
                    key
                ))),
                SkinParamSupport::UnsupportedValue => warnings.push(Diagnostic::warning(format!(
                    "[W_SKINPARAM_UNSUPPORTED_VALUE] unsupported value `{}` for skinparam `{}`",
                    value, key
                ))),
            }
            continue;
        }
        if first_non_empty {
            first_non_empty = false;
            match line.to_ascii_lowercase().as_str() {
                "bar" | "bars" | "bar chart" | "barchart" => {
                    subtype = ChartSubtype::Bar;
                    continue;
                }
                "line" | "lines" | "line chart" | "linechart" => {
                    subtype = ChartSubtype::Line;
                    continue;
                }
                "pie" | "pie chart" | "piechart" => {
                    subtype = ChartSubtype::Pie;
                    continue;
                }
                "area" | "area chart" | "areachart" => {
                    subtype = ChartSubtype::Area;
                    continue;
                }
                "scatter" | "scatter chart" | "scatterchart" => {
                    subtype = ChartSubtype::Scatter;
                    continue;
                }
                _ => {
                    // not a subtype keyword; fall through to data parsing.
                }
            }
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("caption ") {
            caption = Some(line[8..].trim().trim_matches('"').to_string());
            continue;
        }
        if lower.starts_with("palette ") {
            palette = parse_chart_palette(line);
            continue;
        }
        if let Some(annotation) = parse_chart_annotation(line) {
            annotations.push(annotation);
            continue;
        }
        if lower.starts_with("h-axis ") || lower == "h-axis" {
            h_axis = Some(parse_chart_axis(line, "h-axis"));
            continue;
        }
        if lower.starts_with("v-axis ") || lower == "v-axis" {
            v_axis = Some(parse_chart_axis(line, "v-axis"));
            continue;
        }
        if lower.starts_with("legend") {
            legend = parse_chart_legend(line);
            in_legend_block = true;
            continue;
        }
        if let Some(mode) = parse_chart_label_mode(line) {
            label_mode = mode;
            continue;
        }
        if lower == "horizontal" || lower == "horizontal true" || lower == "mode horizontal" {
            horizontal = true;
            continue;
        }
        if lower == "stacked" || lower == "stacked true" || lower == "mode stacked" {
            stacked = true;
            continue;
        }
        if lower == "horizontal stacked" || lower == "stacked horizontal" {
            horizontal = true;
            stacked = true;
            continue;
        }
        if let Some(parsed) = parse_chart_series(line) {
            subtype = parsed.0;
            series.push(parsed.1);
            continue;
        }
        match parse_chart_data_row(line) {
            Ok(row) => rows.push(row),
            Err(message) => {
                warnings.push(Diagnostic::warning(message));
            }
        }
    }
    if series.is_empty() && rows.iter().any(|row| row.values.len() > 1) {
        series = rows_to_chart_series(&rows, &legend_entries);
        let categories = rows.iter().map(|row| row.label.clone()).collect::<Vec<_>>();
        match &mut h_axis {
            Some(axis) if axis.categories.is_empty() => axis.categories = categories,
            None => {
                h_axis = Some(ChartAxis {
                    label: Some("Category".to_string()),
                    categories,
                    ..ChartAxis::default()
                });
            }
            _ => {}
        }
    } else if series.is_empty() {
        data = rows
            .into_iter()
            .filter_map(|row| {
                row.values.first().copied().map(|value| ChartPoint {
                    label: row.label,
                    value,
                    color: row.color,
                })
            })
            .collect();
    }
    // #545: Auto-generate a title when none is supplied in the source.
    let title = title.or_else(|| {
        Some(match subtype {
            ChartSubtype::Bar => "Bar Chart".to_string(),
            ChartSubtype::Line => "Line Chart".to_string(),
            ChartSubtype::Area => "Area Chart".to_string(),
            ChartSubtype::Pie => "Pie Chart".to_string(),
            ChartSubtype::Scatter => "Scatter Chart".to_string(),
        })
    });
    // #545: Auto-populate default axis labels when no h-axis / v-axis were
    // specified, so every chart has visible axis captions out of the box.
    if h_axis.is_none() && subtype != ChartSubtype::Pie {
        h_axis = Some(ChartAxis {
            label: Some("Category".to_string()),
            ..ChartAxis::default()
        });
    }
    if v_axis.is_none() && subtype != ChartSubtype::Pie {
        v_axis = Some(ChartAxis {
            label: Some("Value".to_string()),
            ..ChartAxis::default()
        });
    }
    if let Some(mode) = monochrome_mode {
        apply_monochrome_to_chart_style(&mut style, mode);
    }
    Ok(ChartDocument {
        title,
        caption,
        subtype,
        data,
        h_axis,
        v_axis,
        series,
        legend,
        palette,
        annotations,
        label_mode,
        horizontal,
        stacked,
        style,
        warnings,
    })
}
