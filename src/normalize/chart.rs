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
    let mut palette = Vec::new();
    let mut annotations = Vec::new();
    let mut label_mode = crate::model::ChartLabelMode::Auto;
    let mut horizontal = false;
    let mut stacked = false;
    let mut style = ChartStyle::default();
    let mut monochrome_mode = None;
    let mut warnings: Vec<Diagnostic> = Vec::new();
    let mut first_non_empty = true;
    for line in body {
        let line = line.trim();
        if line.is_empty() || line.starts_with('\'') {
            continue;
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
            if key.eq_ignore_ascii_case("handwritten") {
                match classify_sequence_skinparam(key, value) {
                    SequenceSkinParamSupport::SupportedNoop
                    | SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::Handwritten(_),
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
        // Parse data point: "Label" value  OR  Label value
        let (label, rest) = if let Some(stripped) = line.strip_prefix('"') {
            if let Some(end) = stripped.find('"') {
                (
                    stripped[..end].to_string(),
                    stripped[end + 1..].trim().trim_start_matches(':').trim(),
                )
            } else {
                warnings.push(Diagnostic::warning(format!(
                    "[W_CHART_UNQUOTED] unterminated quoted label on line `{line}`"
                )));
                (stripped.to_string(), "")
            }
        } else if let Some((head, tail)) = line.split_once(':') {
            (head.trim().trim_matches('"').to_string(), tail.trim())
        } else {
            let mut parts = line.splitn(2, char::is_whitespace);
            let head = parts.next().unwrap_or("");
            let tail = parts.next().unwrap_or("").trim();
            (head.to_string(), tail)
        };
        let value_str = rest.split_whitespace().next().unwrap_or("");
        match value_str.parse::<f64>() {
            Ok(v) => data.push(ChartPoint {
                label,
                value: v,
                color: parse_chart_point_color(rest),
            }),
            Err(_) => warnings.push(Diagnostic::warning(format!(
                "[W_CHART_NUMERIC] could not parse numeric value `{value_str}`"
            ))),
        }
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

fn parse_chart_label_mode(line: &str) -> Option<crate::model::ChartLabelMode> {
    let lower = line.trim().to_ascii_lowercase();
    let rest = lower
        .strip_prefix("labels ")
        .or_else(|| lower.strip_prefix("label "))
        .or_else(|| lower.strip_prefix("show labels "))
        .or_else(|| lower.strip_prefix("show label "))?
        .trim();
    match rest {
        "inside" | "in" | "inner" => Some(crate::model::ChartLabelMode::Inside),
        "outside" | "out" | "outer" | "callout" | "callouts" => {
            Some(crate::model::ChartLabelMode::Outside)
        }
        "off" | "none" | "false" | "hidden" => Some(crate::model::ChartLabelMode::None),
        "value" | "values" => Some(crate::model::ChartLabelMode::Value),
        "percent" | "percentage" | "percentages" => Some(crate::model::ChartLabelMode::Percent),
        "auto" | "on" | "true" => Some(crate::model::ChartLabelMode::Auto),
        _ => None,
    }
}

fn parse_chart_axis(line: &str, prefix: &str) -> ChartAxis {
    let mut rest = line[prefix.len()..].trim();
    let mut axis = ChartAxis::default();
    parse_chart_axis_style(&mut axis, rest);
    if let Some((label, after)) = parse_optional_quoted_prefix(rest) {
        axis.label = Some(label);
        rest = after;
    }
    if let Some(start) = rest.find('[') {
        if let Some(end_rel) = rest[start + 1..].find(']') {
            let end = start + 1 + end_rel;
            axis.categories = parse_chart_array_labels(&rest[start + 1..end]);
            rest = rest[end + 1..].trim();
        }
    }
    if let Some((left, right)) = rest.split_once("-->") {
        axis.min = last_numeric_token(left);
        axis.max = first_numeric_token(right);
        axis.tick_step = parse_chart_tick_step(right);
    } else if axis.label.is_none() && !rest.is_empty() && !rest.starts_with('[') {
        axis.label = Some(rest.trim().trim_matches('"').to_string());
    } else {
        axis.tick_step = parse_chart_tick_step(rest);
    }
    axis
}

fn parse_chart_axis_style(axis: &mut ChartAxis, input: &str) {
    let mut tokens = input
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|token| !token.is_empty())
        .peekable();
    while let Some(token) = tokens.next() {
        let key = token
            .trim_matches(|c: char| c == '[' || c == ']' || c == '"' || c == ':')
            .to_ascii_lowercase();
        let target = match key.as_str() {
            "color" | "colour" | "axiscolor" | "linecolor" => Some("axis"),
            "text" | "textcolor" | "fontcolor" | "labelcolor" => Some("label"),
            "grid" | "gridcolor" => Some("grid"),
            _ => None,
        };
        if let Some(target) = target {
            if let Some(value) = tokens.next().and_then(normalize_chart_color) {
                match target {
                    "axis" => axis.color = Some(value),
                    "label" => axis.label_color = Some(value),
                    "grid" => axis.grid_color = Some(value),
                    _ => {}
                }
            }
        }
    }
}

fn parse_chart_tick_step(input: &str) -> Option<f64> {
    let mut tokens = input
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|token| !token.is_empty())
        .peekable();
    while let Some(token) = tokens.next() {
        if matches!(
            token.to_ascii_lowercase().as_str(),
            "step" | "tick" | "ticks" | "by"
        ) {
            if let Some(value) = tokens.next().and_then(|v| v.parse::<f64>().ok()) {
                if value > 0.0 {
                    return Some(value);
                }
            }
        }
    }
    None
}

fn parse_chart_palette(line: &str) -> Vec<String> {
    line.split_whitespace()
        .skip(1)
        .filter_map(normalize_chart_color)
        .collect()
}

fn normalize_chart_color(token: &str) -> Option<String> {
    if token.starts_with('#') {
        Some(token.to_string())
    } else {
        crate::theme::css3_color_to_hex(token).map(ToString::to_string)
    }
}

fn parse_chart_annotation(line: &str) -> Option<ChartAnnotation> {
    let lower = line.to_ascii_lowercase();
    let rest = lower
        .strip_prefix("annotation ")
        .or_else(|| lower.strip_prefix("annotate "))
        .or_else(|| lower.strip_prefix("note at "))
        .map(|suffix| &line[line.len() - suffix.len()..]);
    if let Some(rest) = rest {
        let (target, text) = rest.split_once(':')?;
        return Some(ChartAnnotation {
            target: target.trim().trim_matches('"').to_string(),
            text: text.trim().trim_matches('"').to_string(),
        });
    }
    if let Some(rest) = lower.strip_prefix("note ") {
        let source_rest = &line[line.len() - rest.len()..];
        let (text, target) = source_rest.split_once(" at ")?;
        return Some(ChartAnnotation {
            target: target.trim().trim_matches('"').to_string(),
            text: text.trim().trim_matches('"').to_string(),
        });
    }
    None
}

fn parse_chart_legend(line: &str) -> ChartLegend {
    let mut legend = ChartLegend {
        visible: true,
        explicit: true,
        ..ChartLegend::default()
    };
    let rest = line[6..].trim().to_ascii_lowercase();
    if rest == "off" || rest == "false" || rest == "none" {
        legend.visible = false;
        return legend;
    }
    let mut tokens = rest
        .split_whitespace()
        .filter(|token| {
            !matches!(
                *token,
                "at" | "position" | "pos" | "inside" | "outside" | "legend"
            )
        })
        .peekable();
    while let Some(token) = tokens.next() {
        match token {
            "left" => legend.h_align = LegendHAlign::Left,
            "center" | "centre" => legend.h_align = LegendHAlign::Center,
            "right" => legend.h_align = LegendHAlign::Right,
            "top" => legend.v_align = LegendVAlign::Top,
            "bottom" => legend.v_align = LegendVAlign::Bottom,
            "background" | "backgroundcolor" | "back" | "bg" | "fill" => {
                legend.background_color = tokens.next().and_then(normalize_chart_color);
            }
            "border" | "bordercolor" | "line" | "stroke" => {
                legend.border_color = tokens.next().and_then(normalize_chart_color);
            }
            "text" | "textcolor" | "font" | "fontcolor" | "color" | "colour" => {
                legend.text_color = tokens.next().and_then(normalize_chart_color);
            }
            _ => {}
        }
    }
    legend
}

fn parse_chart_series(line: &str) -> Option<(ChartSubtype, ChartSeries)> {
    let lower = line.to_ascii_lowercase();
    let (subtype, rest) = if lower.starts_with("bar ") {
        (ChartSubtype::Bar, line[3..].trim())
    } else if lower.starts_with("line ") {
        (ChartSubtype::Line, line[4..].trim())
    } else if lower.starts_with("pie ") {
        (ChartSubtype::Pie, line[3..].trim())
    } else {
        return None;
    };
    let (name, rest) = if let Some((label, after)) = parse_optional_quoted_prefix(rest) {
        (label, after)
    } else {
        let mut parts = rest.splitn(2, char::is_whitespace);
        (
            parts.next().unwrap_or("Series").trim().to_string(),
            parts.next().unwrap_or("").trim(),
        )
    };
    let start = rest.find('[')?;
    let end = rest[start + 1..].find(']')? + start + 1;
    let values = parse_chart_number_array(&rest[start + 1..end]);
    if values.is_empty() {
        return None;
    }
    let color = rest[end + 1..]
        .split_whitespace()
        .find(|token| token.starts_with('#') || crate::theme::css3_color_to_hex(token).is_some())
        .map(|token| {
            crate::theme::css3_color_to_hex(token)
                .unwrap_or(token)
                .to_string()
        });
    Some((
        subtype,
        ChartSeries {
            name,
            values,
            color,
        },
    ))
}

fn parse_chart_point_color(rest: &str) -> Option<String> {
    rest.split_whitespace()
        .skip(1)
        .find_map(normalize_chart_color)
}

fn parse_optional_quoted_prefix(input: &str) -> Option<(String, &str)> {
    let stripped = input.strip_prefix('"')?;
    let end = stripped.find('"')?;
    Some((stripped[..end].to_string(), stripped[end + 1..].trim()))
}

fn parse_chart_array_labels(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|v| v.trim().trim_matches('"').to_string())
        .filter(|v| !v.is_empty())
        .collect()
}

fn parse_chart_number_array(input: &str) -> Vec<f64> {
    input
        .split(',')
        .filter_map(|v| v.trim().parse::<f64>().ok())
        .collect()
}

fn first_numeric_token(input: &str) -> Option<f64> {
    input
        .split_whitespace()
        .find_map(|token| token.trim_matches('"').parse::<f64>().ok())
}

fn last_numeric_token(input: &str) -> Option<f64> {
    input
        .split_whitespace()
        .filter_map(|token| token.trim_matches('"').parse::<f64>().ok())
        .next_back()
}
