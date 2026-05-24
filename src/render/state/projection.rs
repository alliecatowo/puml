use super::*;

pub(super) struct StateProjectionRow {
    pub(super) depth: usize,
    pub(super) label: String,
}

pub(super) fn state_projection_format(node: &StateNode) -> &str {
    match node.stereotype.as_deref() {
        Some("yaml") => "yaml",
        _ => "json",
    }
}

pub(super) fn state_projection_layout(node: &StateNode) -> (String, Vec<StateProjectionRow>) {
    let display = node.display.as_deref().unwrap_or(&node.name);
    let mut lines = display.lines();
    let alias = lines
        .next()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .unwrap_or(&node.name)
        .to_string();
    let body = lines.collect::<Vec<_>>().join("\n");
    let mut rows = extract_state_projection_rows(&body, state_projection_format(node));
    if rows.is_empty() {
        rows.push(StateProjectionRow {
            depth: 0,
            label: "(empty)".to_string(),
        });
    }
    (alias, rows)
}

pub(super) fn extract_state_projection_rows(body: &str, format: &str) -> Vec<StateProjectionRow> {
    if format == "json" {
        if let Some(value) = parse_state_projection_json_value(body) {
            let mut rows = Vec::new();
            collect_state_projection_json_rows(None, &value, 0, &mut rows);
            if !rows.is_empty() {
                return rows;
            }
        }
    }
    if format == "yaml" {
        let rows = parse_state_projection_yaml_value(body)
            .map(|value| {
                let mut rows = Vec::new();
                collect_state_projection_yaml_rows(None, &value, 0, &mut rows);
                rows
            })
            .unwrap_or_else(|| {
                extract_state_yaml_kv_lines(body)
                    .into_iter()
                    .map(|label| StateProjectionRow { depth: 0, label })
                    .collect()
            });
        if !rows.is_empty() {
            return rows;
        }
    }
    extract_state_json_kv_lines(body)
        .into_iter()
        .map(|label| StateProjectionRow { depth: 0, label })
        .collect()
}

pub(super) fn parse_state_projection_json_value(body: &str) -> Option<serde_json::Value> {
    let trimmed = body.trim();
    serde_json::from_str::<serde_json::Value>(trimmed)
        .ok()
        .or_else(|| serde_json::from_str::<serde_json::Value>(&format!("{{{trimmed}}}")).ok())
}

pub(super) fn parse_state_projection_yaml_value(body: &str) -> Option<yaml_rust2::Yaml> {
    yaml_rust2::YamlLoader::load_from_str(body.trim())
        .ok()
        .and_then(|docs| {
            docs.into_iter()
                .find(|doc| !matches!(doc, yaml_rust2::Yaml::BadValue))
        })
}

pub(super) fn collect_state_projection_json_rows(
    label: Option<String>,
    value: &serde_json::Value,
    depth: usize,
    rows: &mut Vec<StateProjectionRow>,
) {
    match value {
        serde_json::Value::Object(obj) => {
            let child_depth = if let Some(label) = label {
                rows.push(StateProjectionRow { depth, label });
                depth + 1
            } else {
                depth
            };
            for (key, value) in obj {
                collect_state_projection_json_rows(Some(key.clone()), value, child_depth, rows);
            }
        }
        serde_json::Value::Array(items) => {
            let child_depth = if let Some(label) = label {
                rows.push(StateProjectionRow { depth, label });
                depth + 1
            } else {
                depth
            };
            for (idx, value) in items.iter().enumerate() {
                collect_state_projection_json_rows(
                    Some(format!("[{idx}]")),
                    value,
                    child_depth,
                    rows,
                );
            }
        }
        serde_json::Value::String(s) => rows.push(StateProjectionRow {
            depth,
            label: match label {
                Some(label) => format!("{label}: {s}"),
                None => s.clone(),
            },
        }),
        serde_json::Value::Number(n) => rows.push(StateProjectionRow {
            depth,
            label: match label {
                Some(label) => format!("{label}: {n}"),
                None => n.to_string(),
            },
        }),
        serde_json::Value::Bool(b) => rows.push(StateProjectionRow {
            depth,
            label: match label {
                Some(label) => format!("{label}: {b}"),
                None => b.to_string(),
            },
        }),
        serde_json::Value::Null => rows.push(StateProjectionRow {
            depth,
            label: match label {
                Some(label) => format!("{label}: null"),
                None => "null".to_string(),
            },
        }),
    }
}

pub(super) fn collect_state_projection_yaml_rows(
    label: Option<String>,
    value: &yaml_rust2::Yaml,
    depth: usize,
    rows: &mut Vec<StateProjectionRow>,
) {
    match value {
        yaml_rust2::Yaml::Hash(map) => {
            let child_depth = if let Some(label) = label {
                rows.push(StateProjectionRow { depth, label });
                depth + 1
            } else {
                depth
            };
            for (key, value) in map {
                collect_state_projection_yaml_rows(
                    Some(state_projection_yaml_label(key)),
                    value,
                    child_depth,
                    rows,
                );
            }
        }
        yaml_rust2::Yaml::Array(items) => {
            let child_depth = if let Some(label) = label {
                rows.push(StateProjectionRow { depth, label });
                depth + 1
            } else {
                depth
            };
            for (idx, value) in items.iter().enumerate() {
                collect_state_projection_yaml_rows(
                    Some(format!("[{idx}]")),
                    value,
                    child_depth,
                    rows,
                );
            }
        }
        scalar => rows.push(StateProjectionRow {
            depth,
            label: match label {
                Some(label) => format!("{label}: {}", state_projection_yaml_label(scalar)),
                None => state_projection_yaml_label(scalar),
            },
        }),
    }
}

pub(super) fn state_projection_yaml_label(value: &yaml_rust2::Yaml) -> String {
    match value {
        yaml_rust2::Yaml::Real(s) | yaml_rust2::Yaml::String(s) => s.clone(),
        yaml_rust2::Yaml::Integer(n) => n.to_string(),
        yaml_rust2::Yaml::Boolean(b) => b.to_string(),
        yaml_rust2::Yaml::Alias(id) => format!("*{id}"),
        yaml_rust2::Yaml::Null => "null".to_string(),
        yaml_rust2::Yaml::BadValue => "(invalid)".to_string(),
        yaml_rust2::Yaml::Array(_) => "[...]".to_string(),
        yaml_rust2::Yaml::Hash(_) => "{...}".to_string(),
    }
}

pub(super) fn extract_state_yaml_kv_lines(body: &str) -> Vec<String> {
    let mut path: Vec<String> = Vec::new();
    let mut lines = Vec::new();
    for raw in body.lines() {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = raw.chars().take_while(|c| *c == ' ').count() / 2;
        path.truncate(indent);
        let item = trimmed.strip_prefix("- ").unwrap_or(trimmed);
        let Some((key, value)) = item.split_once(':') else {
            continue;
        };
        let key = key.trim().trim_matches('"').trim_matches('\'').to_string();
        let value = value.trim().trim_matches('"').trim_matches('\'');
        if value.is_empty() {
            path.push(key);
        } else {
            let mut full = path.clone();
            full.push(key);
            lines.push(format!("{}: {}", full.join("."), value));
        }
    }
    lines
}

pub(super) fn extract_state_json_kv_lines(body: &str) -> Vec<String> {
    let mut lines = Vec::new();
    for raw in body.lines() {
        let trimmed = raw.trim().trim_end_matches(',');
        if trimmed.is_empty()
            || trimmed == "{"
            || trimmed == "}"
            || trimmed == "["
            || trimmed == "]"
        {
            continue;
        }
        if let Some(kv) = parse_state_json_kv_display(trimmed) {
            lines.push(kv);
        } else {
            lines.push(trimmed.to_string());
        }
    }
    if lines.is_empty() && !body.trim().is_empty() {
        let flat = body
            .trim()
            .trim_start_matches('{')
            .trim_end_matches('}')
            .trim();
        for segment in flat.split(',') {
            let segment = segment.trim().trim_end_matches(',');
            if let Some(kv) = parse_state_json_kv_display(segment) {
                lines.push(kv);
            }
        }
    }
    lines
}

pub(super) fn parse_state_json_kv_display(segment: &str) -> Option<String> {
    let (key_part, value_part) = segment.split_once(':')?;
    let key = key_part.trim().trim_matches('"');
    let value = value_part.trim().trim_matches('"');
    (!key.is_empty()).then(|| format!("{key}: {value}"))
}

pub(super) fn render_state_json_projection(
    out: &mut String,
    node: &StateNode,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    state_style: &crate::theme::StateStyle,
) {
    let format = state_projection_format(node);
    let (alias, rows) = state_projection_layout(node);
    let fill = node
        .style
        .fill_color
        .as_deref()
        .map(escape_text)
        .unwrap_or_else(|| match format {
            "yaml" => "#fff7ed".to_string(),
            _ => "#eef6ff".to_string(),
        });
    let border = node
        .style
        .border_color
        .as_deref()
        .map(escape_text)
        .unwrap_or_else(|| match format {
            "yaml" => "#f59e0b".to_string(),
            _ => state_node_border(node, state_style),
        });
    let header_fill = match format {
        "yaml" => "#fed7aa",
        _ => "#bfdbfe",
    };
    let header_text = match format {
        "yaml" => "#7c2d12",
        _ => "#1e3a8a",
    };
    out.push_str(&format!(
        "<g class=\"state-json-projection\" data-state-json=\"true\" data-state-projection-format=\"{}\" data-state-projection-alias=\"{}\" data-state-projection-lines=\"{}\">",
        escape_text(format),
        escape_text(&alias),
        rows.len()
    ));
    out.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" ry=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"{} />",
        x,
        y,
        w,
        h,
        fill,
        border,
        state_node_border_dash(node)
    ));
    out.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"22\" rx=\"6\" ry=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"{} />",
        x,
        y,
        w,
        header_fill,
        border,
        state_node_border_dash(node)
    ));
    out.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
        x,
        y + 22,
        x + w,
        y + 22,
        border
    ));
    out.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" font-weight=\"600\" fill=\"{}\">{} ({})</text>",
        x + STATE_NOTE_PAD_X,
        y + 15,
        header_text,
        escape_text(&alias),
        escape_text(format)
    ));

    let body_top = y + 22;
    let row_indent = 16;
    let row_ys: Vec<i32> = rows
        .iter()
        .enumerate()
        .map(|(idx, _)| body_top + 18 + idx as i32 * 16)
        .collect();
    for (idx, row) in rows.iter().enumerate() {
        let text_x = x + STATE_NOTE_PAD_X + (row.depth as i32) * row_indent;
        let text_y = row_ys[idx];
        if row.depth > 0 {
            let parent_y = (0..idx)
                .rev()
                .find(|&parent_idx| rows[parent_idx].depth == row.depth - 1)
                .map(|parent_idx| row_ys[parent_idx])
                .unwrap_or(body_top + 18);
            let connector_x = x + STATE_NOTE_PAD_X + ((row.depth as i32) - 1) * row_indent + 7;
            out.push_str(&format!(
                "<line class=\"state-projection-connector\" data-state-projection-connector=\"{}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                idx,
                connector_x,
                parent_y - 4,
                connector_x,
                text_y - 4,
                border
            ));
            out.push_str(&format!(
                "<line class=\"state-projection-connector\" data-state-projection-connector=\"{}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                idx,
                connector_x,
                text_y - 4,
                text_x - 4,
                text_y - 4,
                border
            ));
        }
        out.push_str(&format!(
            "<g class=\"state-projection-row\" data-state-projection-row=\"{}\" data-state-projection-row-depth=\"{}\" data-state-projection-row-label=\"{}\"><text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text></g>",
            idx,
            row.depth,
            escape_text(&row.label),
            text_x,
            text_y,
            state_node_text(node, state_style),
            escape_text(&row.label)
        ));
    }
    out.push_str("</g>");
}
