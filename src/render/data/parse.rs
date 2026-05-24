use super::*;
pub(super) fn parse_structured_controls(raw: &str, family: DataFamily) -> StructuredControls {
    let mut controls = StructuredControls::default();
    let mut payload = Vec::new();
    let mut style_lines = Vec::new();
    let mut in_style = false;
    for line in raw.lines() {
        let trimmed = line.trim_start();
        let lower = trimmed.to_ascii_lowercase();
        if in_style {
            if lower.contains("</style>") {
                in_style = false;
                if let Some(before_end) = line.split("</style>").next() {
                    style_lines.push(before_end.to_string());
                }
            } else {
                style_lines.push(line.to_string());
            }
            continue;
        }
        if lower.starts_with("<style") {
            in_style = !lower.contains("</style>");
            if let Some(after_start) = line.split('>').nth(1) {
                let before_end = after_start.split("</style>").next().unwrap_or(after_start);
                if !before_end.trim().is_empty() {
                    style_lines.push(before_end.to_string());
                }
            }
            continue;
        }
        if trimmed.starts_with("#highlight") {
            if let Some(highlight) = parse_highlight(trimmed) {
                controls.highlights.push(highlight);
            }
            continue;
        }
        payload.push(line);
    }
    controls.payload = payload.join("\n");
    parse_style_lines(&style_lines.join("\n"), family, &mut controls);
    controls
}

pub(super) fn parse_highlight(line: &str) -> Option<HighlightSpec> {
    let mut rest = line.strip_prefix("#highlight")?.trim();
    let mut class_name = None;
    if let Some(start) = rest.find("<<") {
        if let Some(end) = rest[start + 2..].find(">>") {
            class_name = Some(rest[start + 2..start + 2 + end].trim().to_string());
        }
        rest = rest[..start].trim();
    }

    let mut path = Vec::new();
    let mut chars = rest.chars().peekable();
    while let Some(ch) = chars.peek().copied() {
        match ch {
            '"' => {
                chars.next();
                let mut segment = String::new();
                let mut escaped = false;
                for next in chars.by_ref() {
                    if escaped {
                        segment.push(next);
                        escaped = false;
                    } else if next == '\\' {
                        escaped = true;
                    } else if next == '"' {
                        break;
                    } else {
                        segment.push(next);
                    }
                }
                path.push(segment);
            }
            '/' | ' ' | '\t' => {
                chars.next();
            }
            _ => {
                let mut segment = String::new();
                while let Some(next) = chars.peek().copied() {
                    if next == '/' || next.is_whitespace() {
                        break;
                    }
                    segment.push(next);
                    chars.next();
                }
                if !segment.is_empty() {
                    path.push(segment);
                }
            }
        }
    }

    if path.is_empty() {
        None
    } else {
        Some(HighlightSpec { path, class_name })
    }
}

pub(super) fn parse_style_lines(
    style_source: &str,
    family: DataFamily,
    controls: &mut StructuredControls,
) {
    let prepared = style_source.replace('{', "\n{\n").replace('}', "\n}\n");
    let mut stack: Vec<String> = Vec::new();
    let mut pending_selector: Option<String> = None;
    for raw in prepared.lines() {
        let line = raw.trim().trim_end_matches(';').trim();
        if line.is_empty() {
            continue;
        }
        if line == "{" {
            if let Some(selector) = pending_selector.take() {
                stack.push(selector);
            }
            continue;
        }
        if line == "}" {
            stack.pop();
            continue;
        }
        if apply_style_property(line, &stack, family, controls) {
            continue;
        }
        pending_selector = Some(line.to_string());
    }
}

pub(super) fn apply_style_property(
    line: &str,
    stack: &[String],
    family: DataFamily,
    controls: &mut StructuredControls,
) -> bool {
    let mut parts = line.splitn(2, char::is_whitespace);
    let Some(raw_key) = parts.next() else {
        return false;
    };
    let key = raw_key.trim_end_matches(':').to_ascii_lowercase();
    if !matches!(
        key.as_str(),
        "backgroundcolor" | "fontcolor" | "fontstyle" | "linecolor"
    ) {
        return false;
    }
    let value = parts
        .next()
        .unwrap_or_default()
        .trim()
        .trim_start_matches(':')
        .trim()
        .trim_end_matches(';')
        .trim();
    if value.is_empty() {
        return true;
    }

    let class_selector = stack
        .iter()
        .rev()
        .find_map(|selector| selector.strip_prefix('.'));
    if let Some(class_name) = class_selector {
        let patch = controls
            .class_styles
            .entry(class_name.to_string())
            .or_default();
        apply_patch_property(patch, &key, value);
        return true;
    }

    let family_selector = match family {
        DataFamily::Json => "jsondiagram",
        DataFamily::Yaml => "yamldiagram",
    };
    let in_family = stack
        .iter()
        .any(|selector| selector.eq_ignore_ascii_case(family_selector));
    let in_highlight = stack
        .iter()
        .any(|selector| selector.eq_ignore_ascii_case("highlight"));
    if in_highlight && (in_family || !stack.is_empty()) {
        apply_patch_property(&mut controls.default_highlight, &key, value);
    }
    true
}

pub(super) fn apply_patch_property(patch: &mut StylePatch, key: &str, value: &str) {
    match key {
        "backgroundcolor" => patch.fill = Some(value.to_string()),
        "linecolor" => patch.stroke = Some(value.to_string()),
        "fontcolor" => patch.font_color = Some(value.to_string()),
        "fontstyle" => {
            let lower = value.to_ascii_lowercase();
            patch.font_style = lower.contains("italic").then(|| "italic".to_string());
            patch.font_weight = lower.contains("bold").then(|| "700".to_string());
        }
        _ => {}
    }
}

pub(super) fn json_render_rows(raw: &str) -> Option<Vec<RenderRow>> {
    let value = serde_json::from_str::<serde_json::Value>(raw.trim()).ok()?;
    let mut rows = Vec::new();
    flatten_json_render_value(&value, None, Vec::new(), 0, &mut rows);
    Some(rows)
}

pub(super) fn flatten_json_render_value(
    value: &serde_json::Value,
    label: Option<&str>,
    path: Vec<String>,
    depth: usize,
    out: &mut Vec<RenderRow>,
) {
    use serde_json::Value;
    match value {
        Value::Object(map) => {
            let compact = if map.is_empty() { "{}" } else { "{...}" };
            let row_label = label
                .map(|l| format!("{l}: {compact}"))
                .unwrap_or_else(|| compact.to_string());
            out.push(structured_render_row(
                depth,
                row_label,
                label,
                compact,
                path.clone(),
            ));
            for (key, child) in map {
                let mut child_path = path.clone();
                child_path.push(key.clone());
                flatten_json_render_value(child, Some(key), child_path, depth + 1, out);
            }
        }
        Value::Array(items) => {
            let compact = if items.is_empty() { "[]" } else { "[...]" };
            let row_label = label
                .map(|l| format!("{l}: {compact}"))
                .unwrap_or_else(|| compact.to_string());
            out.push(structured_render_row(
                depth,
                row_label,
                label,
                compact,
                path.clone(),
            ));
            for (idx, child) in items.iter().enumerate() {
                let display = format!("[{idx}]");
                let mut child_path = path.clone();
                child_path.push(idx.to_string());
                flatten_json_render_value(child, Some(&display), child_path, depth + 1, out);
            }
        }
        Value::String(s) => {
            let value = json_string_label(s);
            let row_label = label
                .map(|l| format!("{l}: {value}"))
                .unwrap_or_else(|| value.clone());
            out.push(structured_render_row(depth, row_label, label, &value, path));
        }
        Value::Number(n) => {
            let value = n.to_string();
            let row_label = label
                .map(|l| format!("{l}: {value}"))
                .unwrap_or_else(|| value.clone());
            out.push(structured_render_row(depth, row_label, label, &value, path));
        }
        Value::Bool(b) => {
            let value = b.to_string();
            let row_label = label
                .map(|l| format!("{l}: {value}"))
                .unwrap_or_else(|| value.clone());
            out.push(structured_render_row(depth, row_label, label, &value, path));
        }
        Value::Null => {
            let value = "null";
            let row_label = label
                .map(|l| format!("{l}: {value}"))
                .unwrap_or_else(|| value.to_string());
            out.push(structured_render_row(depth, row_label, label, value, path));
        }
    }
}

pub(super) fn structured_render_row(
    depth: usize,
    label: String,
    key: Option<&str>,
    value: &str,
    path: Vec<String>,
) -> RenderRow {
    RenderRow {
        depth,
        label,
        key: key.unwrap_or_default().to_string(),
        value: Some(value.to_string()),
        path,
    }
}

pub(super) fn json_string_label(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| format!("\"{value}\""))
}

pub(super) fn yaml_render_rows(raw: &str) -> Option<Vec<RenderRow>> {
    let docs = yaml_rust2::YamlLoader::load_from_str(raw.trim()).ok()?;
    let mut rows = Vec::new();
    for doc in docs
        .iter()
        .filter(|doc| !matches!(doc, yaml_rust2::Yaml::BadValue))
    {
        flatten_yaml_render_value(doc, None, Vec::new(), 0, &mut rows);
    }
    Some(rows)
}

pub(super) fn flatten_yaml_render_value(
    value: &yaml_rust2::Yaml,
    label: Option<String>,
    path: Vec<String>,
    depth: usize,
    out: &mut Vec<RenderRow>,
) {
    match value {
        yaml_rust2::Yaml::Hash(map) => {
            let compact = if map.is_empty() { "{}" } else { "{...}" };
            let row_label = label
                .as_deref()
                .map(|l| format!("{l}: {compact}"))
                .unwrap_or_else(|| compact.to_string());
            out.push(structured_render_row(
                depth,
                row_label,
                label.as_deref(),
                compact,
                path.clone(),
            ));
            for (key, value) in map {
                let key_label = yaml_key_label(key);
                let mut child_path = path.clone();
                child_path.push(key_label.clone());
                flatten_yaml_render_value(value, Some(key_label), child_path, depth + 1, out);
            }
        }
        yaml_rust2::Yaml::Array(items) => {
            let compact = if items.is_empty() { "[]" } else { "[...]" };
            let row_label = label
                .as_deref()
                .map(|l| format!("{l}: {compact}"))
                .unwrap_or_else(|| compact.to_string());
            out.push(structured_render_row(
                depth,
                row_label,
                label.as_deref(),
                compact,
                path.clone(),
            ));
            for (idx, value) in items.iter().enumerate() {
                let mut child_path = path.clone();
                child_path.push(idx.to_string());
                flatten_yaml_render_value(
                    value,
                    Some(format!("[{idx}]")),
                    child_path,
                    depth + 1,
                    out,
                );
            }
        }
        scalar => {
            let value = yaml_scalar_label(scalar);
            let row_label = match label.as_deref() {
                Some(label) => format!("{label}: {value}"),
                None => value.clone(),
            };
            out.push(structured_render_row(
                depth,
                row_label,
                label.as_deref(),
                &value,
                path,
            ));
        }
    }
}

pub(super) fn yaml_key_label(value: &yaml_rust2::Yaml) -> String {
    match value {
        yaml_rust2::Yaml::String(s) => s.clone(),
        scalar => yaml_scalar_label(scalar),
    }
}

pub(super) fn yaml_scalar_label(value: &yaml_rust2::Yaml) -> String {
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

pub(super) fn path_attr(path: &[String]) -> String {
    if path.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", path.join("/"))
    }
}
