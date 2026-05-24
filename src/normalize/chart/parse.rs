use super::*;

pub(super) fn parse_chart_label_mode(line: &str) -> Option<crate::model::ChartLabelMode> {
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

pub(super) fn parse_chart_axis(line: &str, prefix: &str) -> ChartAxis {
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

pub(super) fn parse_chart_axis_style(axis: &mut ChartAxis, input: &str) {
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

pub(super) fn parse_chart_tick_step(input: &str) -> Option<f64> {
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

pub(super) fn parse_chart_palette(line: &str) -> Vec<String> {
    line.split_whitespace()
        .skip(1)
        .filter_map(normalize_chart_color)
        .collect()
}

pub(super) fn normalize_chart_color(token: &str) -> Option<String> {
    if token.starts_with('#') {
        Some(token.to_string())
    } else {
        crate::theme::color::css3_color_to_hex(token).map(ToString::to_string)
    }
}

pub(super) fn parse_chart_annotation(line: &str) -> Option<ChartAnnotation> {
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

pub(super) fn parse_chart_legend(line: &str) -> ChartLegend {
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

#[derive(Debug, Clone)]
pub(super) struct ChartLegendEntry {
    pub(super) name: String,
    pub(super) color: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct ChartDataRow {
    pub(super) label: String,
    pub(super) values: Vec<f64>,
    pub(super) color: Option<String>,
}

pub(super) fn parse_chart_legend_entry(line: &str) -> Option<ChartLegendEntry> {
    let (name, rest) = parse_chart_label_and_rest(line).ok()?;
    let color = rest
        .split(|c: char| c.is_whitespace() || c == ':' || c == ',')
        .find_map(|token| normalize_chart_color(token.trim_matches('"')));
    if name.is_empty() || (color.is_none() && !line.contains(':')) {
        return None;
    }
    Some(ChartLegendEntry { name, color })
}

pub(super) fn parse_chart_data_row(line: &str) -> Result<ChartDataRow, String> {
    let (label, rest) = parse_chart_label_and_rest(line)?;
    let values = rest
        .split(|c: char| c.is_whitespace() || c == ':' || c == ',')
        .filter(|token| !token.is_empty())
        .filter_map(|token| token.parse::<f64>().ok())
        .collect::<Vec<_>>();
    if values.is_empty() {
        let value_str = rest.split_whitespace().next().unwrap_or("");
        return Err(format!(
            "[W_CHART_NUMERIC] could not parse numeric value `{value_str}`"
        ));
    }
    Ok(ChartDataRow {
        label,
        values,
        color: parse_chart_point_color(rest),
    })
}

pub(super) fn parse_chart_label_and_rest(line: &str) -> Result<(String, &str), String> {
    if let Some(stripped) = line.strip_prefix('"') {
        if let Some(end) = stripped.find('"') {
            return Ok((
                stripped[..end].to_string(),
                stripped[end + 1..].trim().trim_start_matches(':').trim(),
            ));
        }
        return Err(format!(
            "[W_CHART_UNQUOTED] unterminated quoted label on line `{line}`"
        ));
    }
    if let Some((head, tail)) = line.split_once(':') {
        return Ok((head.trim().trim_matches('"').to_string(), tail.trim()));
    }
    let mut parts = line.splitn(2, char::is_whitespace);
    let head = parts.next().unwrap_or("");
    let tail = parts.next().unwrap_or("").trim();
    Ok((head.to_string(), tail))
}

pub(super) fn rows_to_chart_series(
    rows: &[ChartDataRow],
    legend_entries: &[ChartLegendEntry],
) -> Vec<ChartSeries> {
    let count = rows.iter().map(|row| row.values.len()).max().unwrap_or(0);
    (0..count)
        .map(|idx| {
            let legend_entry = legend_entries.get(idx);
            ChartSeries {
                name: legend_entry
                    .map(|entry| entry.name.clone())
                    .unwrap_or_else(|| format!("Series {}", idx + 1)),
                values: rows
                    .iter()
                    .map(|row| row.values.get(idx).copied().unwrap_or(0.0))
                    .collect(),
                color: legend_entry.and_then(|entry| entry.color.clone()),
            }
        })
        .collect()
}

pub(super) fn parse_chart_series(line: &str) -> Option<(ChartSubtype, ChartSeries)> {
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
        .find(|token| {
            token.starts_with('#') || crate::theme::color::css3_color_to_hex(token).is_some()
        })
        .map(|token| {
            crate::theme::color::resolve_css3_color_or_original(token).unwrap_or_default()
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

pub(super) fn parse_chart_point_color(rest: &str) -> Option<String> {
    rest.split_whitespace()
        .skip(1)
        .find_map(normalize_chart_color)
}

pub(super) fn parse_optional_quoted_prefix(input: &str) -> Option<(String, &str)> {
    let stripped = input.strip_prefix('"')?;
    let end = stripped.find('"')?;
    Some((stripped[..end].to_string(), stripped[end + 1..].trim()))
}

pub(super) fn parse_chart_array_labels(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|v| v.trim().trim_matches('"').to_string())
        .filter(|v| !v.is_empty())
        .collect()
}

pub(super) fn parse_chart_number_array(input: &str) -> Vec<f64> {
    input
        .split(',')
        .filter_map(|v| v.trim().parse::<f64>().ok())
        .collect()
}

pub(super) fn first_numeric_token(input: &str) -> Option<f64> {
    input
        .split_whitespace()
        .find_map(|token| token.trim_matches('"').parse::<f64>().ok())
}

pub(super) fn last_numeric_token(input: &str) -> Option<f64> {
    input
        .split_whitespace()
        .filter_map(|token| token.trim_matches('"').parse::<f64>().ok())
        .next_back()
}
