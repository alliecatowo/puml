use crate::model::JsonProjection;
use crate::render::svg::escape_text;

#[derive(Clone)]
struct ProjectionTreeRow {
    depth: usize,
    label: String,
}

/// Extract deterministic display rows from a JSON/YAML projection body.
fn extract_projection_tree_rows(body: &str, format: &str) -> Vec<ProjectionTreeRow> {
    if format == "json" {
        if let Some(value) = parse_projection_json_value(body) {
            let mut rows = Vec::new();
            collect_projection_json_rows(None, &value, 0, &mut rows);
            if !rows.is_empty() {
                return rows;
            }
        }
    }
    if format == "yaml" {
        let rows = parse_projection_yaml_value(body)
            .map(|value| {
                let mut rows = Vec::new();
                collect_projection_yaml_rows(None, &value, 0, &mut rows);
                rows
            })
            .unwrap_or_else(|| {
                extract_yaml_kv_lines(body)
                    .into_iter()
                    .map(|label| ProjectionTreeRow { depth: 0, label })
                    .collect()
            });
        if !rows.is_empty() {
            return rows;
        }
    }
    extract_json_kv_lines(body)
        .into_iter()
        .map(|label| ProjectionTreeRow { depth: 0, label })
        .collect()
}

fn parse_projection_yaml_value(body: &str) -> Option<yaml_rust2::Yaml> {
    yaml_rust2::YamlLoader::load_from_str(body.trim())
        .ok()
        .and_then(|docs| {
            docs.into_iter()
                .find(|doc| !matches!(doc, yaml_rust2::Yaml::BadValue))
        })
}

fn parse_projection_json_value(body: &str) -> Option<serde_json::Value> {
    let trimmed = body.trim();
    serde_json::from_str::<serde_json::Value>(trimmed)
        .ok()
        .or_else(|| serde_json::from_str::<serde_json::Value>(&format!("{{{trimmed}}}")).ok())
}

pub(super) fn family_projection_extra_height(projections: &[JsonProjection]) -> i32 {
    if projections.is_empty() {
        return 0;
    }
    projections.iter().fold(12, |acc, proj| {
        let line_count = extract_projection_tree_rows(&proj.body, &proj.format)
            .len()
            .max(1) as i32;
        acc + 22 + 16 + (line_count * 16) + 20
    })
}

pub(super) fn render_family_projection_boxes(
    out: &mut String,
    projections: &[JsonProjection],
    x: i32,
    mut y: i32,
    width: i32,
) {
    for proj in projections {
        let projection_rows = extract_projection_tree_rows(&proj.body, &proj.format);
        let rows = if projection_rows.is_empty() {
            vec![ProjectionTreeRow {
                depth: 0,
                label: "(empty)".to_string(),
            }]
        } else {
            projection_rows
        };
        let header_h = 22;
        let line_h = 16;
        let row_indent = 18;
        let body_h = (rows.len() as i32) * line_h + 16;
        let height = header_h + body_h;
        out.push_str(&format!(
            "<g class=\"uml-projection\" data-uml-projection=\"{}\" data-uml-projection-format=\"{}\" data-uml-projection-lines=\"{}\">",
            escape_text(&proj.alias),
            escape_text(&proj.format),
            rows.len()
        ));
        out.push_str(&format!(
            "<rect x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" rx=\"5\" ry=\"5\" fill=\"#fffde7\" stroke=\"#f59e0b\" stroke-width=\"1.5\"/>"
        ));
        out.push_str(&format!(
            "<rect x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{header_h}\" rx=\"5\" ry=\"5\" fill=\"#fef08a\" stroke=\"#f59e0b\" stroke-width=\"1.5\"/>"
        ));
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" font-weight=\"600\" fill=\"#78350f\">{} ({})</text>",
            x + 8,
            y + 15,
            escape_text(&proj.alias),
            escape_text(&proj.format)
        ));
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#f59e0b\" stroke-width=\"1\"/>",
            x,
            y + header_h,
            x + width,
            y + header_h
        ));
        let row_ys: Vec<i32> = rows
            .iter()
            .enumerate()
            .map(|(idx, _)| y + header_h + 18 + (idx as i32 * line_h))
            .collect();
        for (idx, row) in rows.iter().enumerate() {
            let text_x = x + 16 + (row.depth as i32) * row_indent;
            let text_y = row_ys[idx];
            if row.depth > 0 {
                let parent_y = (0..idx)
                    .rev()
                    .find(|&parent_idx| rows[parent_idx].depth == row.depth - 1)
                    .map(|parent_idx| row_ys[parent_idx])
                    .unwrap_or(y + header_h + 18);
                let connector_x = x + 10 + ((row.depth as i32) - 1) * row_indent;
                out.push_str(&format!(
                    "<line class=\"uml-projection-connector\" data-uml-projection-connector=\"{}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#ca8a04\" stroke-width=\"1\"/>",
                    idx,
                    connector_x,
                    parent_y - 4,
                    connector_x,
                    text_y - 4
                ));
                out.push_str(&format!(
                    "<line class=\"uml-projection-connector\" data-uml-projection-connector=\"{}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#ca8a04\" stroke-width=\"1\"/>",
                    idx,
                    connector_x,
                    text_y - 4,
                    text_x - 6,
                    text_y - 4
                ));
            }
        }
        for (idx, row) in rows.iter().enumerate() {
            out.push_str(&format!(
                "<g class=\"uml-projection-row\" data-uml-projection-row=\"{}\" data-uml-projection-row-depth=\"{}\" data-uml-projection-row-label=\"{}\"><text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#1e293b\">{}</text></g>",
                idx,
                row.depth,
                escape_text(&row.label),
                x + 16 + (row.depth as i32) * row_indent,
                row_ys[idx],
                escape_text(&row.label)
            ));
        }
        out.push_str("</g>");
        y += height + 12;
    }
}

fn collect_projection_json_rows(
    label: Option<String>,
    value: &serde_json::Value,
    depth: usize,
    rows: &mut Vec<ProjectionTreeRow>,
) {
    match value {
        serde_json::Value::Object(obj) => {
            let child_depth = if let Some(label) = label {
                rows.push(ProjectionTreeRow { depth, label });
                depth + 1
            } else {
                depth
            };
            for (key, value) in obj {
                collect_projection_json_rows(Some(key.clone()), value, child_depth, rows);
            }
        }
        serde_json::Value::Array(items) => {
            let child_depth = if let Some(label) = label {
                rows.push(ProjectionTreeRow { depth, label });
                depth + 1
            } else {
                depth
            };
            for (idx, value) in items.iter().enumerate() {
                collect_projection_json_rows(Some(format!("[{idx}]")), value, child_depth, rows);
            }
        }
        serde_json::Value::String(s) => rows.push(ProjectionTreeRow {
            depth,
            label: match label {
                Some(label) => format!("{label}: {s}"),
                None => s.clone(),
            },
        }),
        serde_json::Value::Number(n) => rows.push(ProjectionTreeRow {
            depth,
            label: match label {
                Some(label) => format!("{label}: {n}"),
                None => n.to_string(),
            },
        }),
        serde_json::Value::Bool(b) => rows.push(ProjectionTreeRow {
            depth,
            label: match label {
                Some(label) => format!("{label}: {b}"),
                None => b.to_string(),
            },
        }),
        serde_json::Value::Null => rows.push(ProjectionTreeRow {
            depth,
            label: match label {
                Some(label) => format!("{label}: null"),
                None => "null".to_string(),
            },
        }),
    }
}

fn collect_projection_yaml_rows(
    label: Option<String>,
    value: &yaml_rust2::Yaml,
    depth: usize,
    rows: &mut Vec<ProjectionTreeRow>,
) {
    match value {
        yaml_rust2::Yaml::Hash(map) => {
            let child_depth = if let Some(label) = label {
                rows.push(ProjectionTreeRow { depth, label });
                depth + 1
            } else {
                depth
            };
            for (key, value) in map {
                collect_projection_yaml_rows(
                    Some(projection_yaml_label(key)),
                    value,
                    child_depth,
                    rows,
                );
            }
        }
        yaml_rust2::Yaml::Array(items) => {
            let child_depth = if let Some(label) = label {
                rows.push(ProjectionTreeRow { depth, label });
                depth + 1
            } else {
                depth
            };
            for (idx, value) in items.iter().enumerate() {
                collect_projection_yaml_rows(Some(format!("[{idx}]")), value, child_depth, rows);
            }
        }
        scalar => rows.push(ProjectionTreeRow {
            depth,
            label: match label {
                Some(label) => format!("{label}: {}", projection_yaml_label(scalar)),
                None => projection_yaml_label(scalar),
            },
        }),
    }
}

fn projection_yaml_label(value: &yaml_rust2::Yaml) -> String {
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

fn extract_yaml_kv_lines(body: &str) -> Vec<String> {
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

/// Extract `key: value` display lines from a JSON-ish body string.
/// Strips outer braces/brackets, parses simple string-keyed properties.
fn extract_json_kv_lines(body: &str) -> Vec<String> {
    let mut lines = Vec::new();
    // Simple line-by-line extraction: look for `"key": value` patterns.
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
        // Try to extract key: value from `"key": value` form.
        if let Some(kv) = parse_json_kv_display(trimmed) {
            lines.push(kv);
        } else if !trimmed.is_empty() {
            // Just push the trimmed line if we can't parse it as k/v.
            lines.push(trimmed.to_string());
        }
    }
    // If body is a flat single-line JSON, try splitting on commas.
    if lines.is_empty() && !body.trim().is_empty() {
        let flat = body
            .trim()
            .trim_start_matches('{')
            .trim_end_matches('}')
            .trim();
        for segment in flat.split(',') {
            let seg = segment.trim().trim_end_matches(',');
            if !seg.is_empty() {
                if let Some(kv) = parse_json_kv_display(seg) {
                    lines.push(kv);
                }
            }
        }
    }
    lines
}

/// Parse a single JSON key-value segment like `"name": "Alice"` → `name: Alice`.
fn parse_json_kv_display(segment: &str) -> Option<String> {
    // Expect: optional quote, key chars, optional quote, `:`, value
    let (key_part, val_part) = segment.split_once(':')?;
    let key = key_part.trim().trim_matches('"');
    let val = val_part.trim().trim_matches('"');
    if key.is_empty() {
        return None;
    }
    Some(format!("{key}: {val}"))
}

pub(super) fn class_projection_extra_height(projections: &[JsonProjection]) -> i32 {
    projections.iter().fold(0, |acc, proj| {
        let kv_count = extract_projection_tree_rows(&proj.body, &proj.format).len() as i32;
        acc + 22 + kv_count * 16 + 8 + 12
    })
}
