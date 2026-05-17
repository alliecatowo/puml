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
    Ok(YamlDocument {
        raw,
        nodes,
        title,
        warnings: Vec::new(),
    })
}
