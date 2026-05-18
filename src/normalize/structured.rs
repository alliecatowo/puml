use super::*;

pub(super) fn normalize_json_document(document: Document) -> Result<JsonDocument, Diagnostic> {
    let (raw, title) = collect_raw_block(&document);
    let nodes = match serde_json::from_str::<serde_json::Value>(raw.trim()) {
        Ok(value) => {
            let mut out = Vec::new();
            flatten_json_value(&value, None, 0, &mut out);
            out
        }
        Err(_) => raw
            .lines()
            .map(|line| JsonTreeNode {
                depth: 0,
                label: line.trim_end().to_string(),
            })
            .collect(),
    };
    Ok(JsonDocument {
        raw,
        nodes,
        title,
        warnings: Vec::new(),
    })
}

fn flatten_json_value(
    value: &serde_json::Value,
    label: Option<&str>,
    depth: usize,
    out: &mut Vec<JsonTreeNode>,
) {
    use serde_json::Value;
    match value {
        Value::Object(map) => {
            let header = label
                .map(|l| format!("{l}: {{...}}"))
                .unwrap_or_else(|| "{...}".to_string());
            out.push(JsonTreeNode {
                depth,
                label: header,
            });
            for (k, v) in map {
                flatten_json_value(v, Some(k), depth + 1, out);
            }
        }
        Value::Array(items) => {
            let header = label
                .map(|l| format!("{l}: [...]"))
                .unwrap_or_else(|| "[...]".to_string());
            out.push(JsonTreeNode {
                depth,
                label: header,
            });
            for (i, v) in items.iter().enumerate() {
                let key = format!("[{i}]");
                flatten_json_value(v, Some(&key), depth + 1, out);
            }
        }
        Value::String(s) => out.push(JsonTreeNode {
            depth,
            label: label
                .map(|l| format!("{l}: \"{s}\""))
                .unwrap_or_else(|| format!("\"{s}\"")),
        }),
        Value::Number(n) => out.push(JsonTreeNode {
            depth,
            label: label
                .map(|l| format!("{l}: {n}"))
                .unwrap_or_else(|| n.to_string()),
        }),
        Value::Bool(b) => out.push(JsonTreeNode {
            depth,
            label: label
                .map(|l| format!("{l}: {b}"))
                .unwrap_or_else(|| b.to_string()),
        }),
        Value::Null => out.push(JsonTreeNode {
            depth,
            label: label
                .map(|l| format!("{l}: null"))
                .unwrap_or_else(|| "null".to_string()),
        }),
    }
}

pub(super) fn normalize_yaml_document(document: Document) -> Result<YamlDocument, Diagnostic> {
    let (raw, title) = collect_raw_block(&document);
    let nodes = match yaml_rust2::YamlLoader::load_from_str(raw.trim()) {
        Ok(docs) => {
            let mut out = Vec::new();
            for doc in docs
                .iter()
                .filter(|doc| !matches!(doc, yaml_rust2::Yaml::BadValue))
            {
                flatten_yaml_value(doc, None, 0, &mut out);
            }
            out
        }
        Err(_) => flatten_yaml_by_indent(&raw),
    };
    Ok(YamlDocument {
        raw,
        nodes,
        title,
        warnings: Vec::new(),
    })
}

fn flatten_yaml_value(
    value: &yaml_rust2::Yaml,
    label: Option<String>,
    depth: usize,
    out: &mut Vec<YamlTreeNode>,
) {
    match value {
        yaml_rust2::Yaml::Hash(map) => {
            out.push(YamlTreeNode {
                depth,
                label: label
                    .map(|l| format!("{l}: {{...}}"))
                    .unwrap_or_else(|| "{...}".to_string()),
            });
            for (key, value) in map {
                flatten_yaml_value(value, Some(yaml_key_label(key)), depth + 1, out);
            }
        }
        yaml_rust2::Yaml::Array(items) => {
            out.push(YamlTreeNode {
                depth,
                label: label
                    .map(|l| format!("{l}: [...]"))
                    .unwrap_or_else(|| "[...]".to_string()),
            });
            for (idx, value) in items.iter().enumerate() {
                flatten_yaml_value(value, Some(format!("[{idx}]")), depth + 1, out);
            }
        }
        scalar => out.push(YamlTreeNode {
            depth,
            label: match label {
                Some(label) => format!("{label}: {}", yaml_scalar_label(scalar)),
                None => yaml_scalar_label(scalar),
            },
        }),
    }
}

fn yaml_key_label(value: &yaml_rust2::Yaml) -> String {
    match value {
        yaml_rust2::Yaml::String(s) => s.clone(),
        scalar => yaml_scalar_label(scalar),
    }
}

fn yaml_scalar_label(value: &yaml_rust2::Yaml) -> String {
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

fn flatten_yaml_by_indent(raw: &str) -> Vec<YamlTreeNode> {
    let mut nodes = Vec::new();
    for line in raw.lines() {
        // Strip trailing whitespace; skip fully blank lines and comment-only lines.
        let trimmed_end = line.trim_end();
        if trimmed_end.is_empty() {
            continue;
        }
        let indent = trimmed_end.len() - trimmed_end.trim_start().len();
        let depth = indent / 2;
        let content = trimmed_end.trim_start();
        if content.starts_with('#') {
            continue;
        }
        nodes.push(YamlTreeNode {
            depth,
            label: content.to_string(),
        });
    }
    nodes
}
