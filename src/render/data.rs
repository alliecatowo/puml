use std::collections::BTreeMap;

use super::*;

#[derive(Clone, Copy)]
enum DataFamily {
    Json,
    Yaml,
}

impl DataFamily {
    fn projection(self) -> &'static str {
        match self {
            DataFamily::Json => "json",
            DataFamily::Yaml => "yaml",
        }
    }

    fn title(self) -> &'static str {
        match self {
            DataFamily::Json => "JSON",
            DataFamily::Yaml => "YAML",
        }
    }

    fn connector_color(self) -> &'static str {
        match self {
            DataFamily::Json => "#94a3b8",
            DataFamily::Yaml => "#ca8a04",
        }
    }

    fn connector_dash(self) -> &'static str {
        match self {
            DataFamily::Json => "",
            DataFamily::Yaml => " stroke-dasharray=\"2 2\"",
        }
    }
}

#[derive(Clone)]
struct RenderRow {
    depth: usize,
    label: String,
    key: String,
    value: Option<String>,
    path: Vec<String>,
}

#[derive(Clone)]
struct HighlightSpec {
    path: Vec<String>,
    class_name: Option<String>,
}

#[derive(Clone)]
struct RowStyle {
    fill: String,
    stroke: String,
    font_color: String,
    font_style: Option<String>,
    font_weight: Option<String>,
}

impl RowStyle {
    fn json_node() -> Self {
        Self {
            fill: "#f1f5f9".to_string(),
            stroke: "#94a3b8".to_string(),
            font_color: "#0f172a".to_string(),
            font_style: None,
            font_weight: None,
        }
    }

    fn yaml_node() -> Self {
        Self {
            fill: "#fef9c3".to_string(),
            stroke: "#ca8a04".to_string(),
            font_color: "#0f172a".to_string(),
            font_style: None,
            font_weight: None,
        }
    }

    fn highlight() -> Self {
        Self {
            fill: "#facc15".to_string(),
            stroke: "#d97706".to_string(),
            font_color: "#111827".to_string(),
            font_style: None,
            font_weight: Some("700".to_string()),
        }
    }

    fn merge_patch(&self, patch: Option<&StylePatch>) -> Self {
        let Some(patch) = patch else {
            return self.clone();
        };
        Self {
            fill: patch.fill.clone().unwrap_or_else(|| self.fill.clone()),
            stroke: patch.stroke.clone().unwrap_or_else(|| self.stroke.clone()),
            font_color: patch
                .font_color
                .clone()
                .unwrap_or_else(|| self.font_color.clone()),
            font_style: patch.font_style.clone().or_else(|| self.font_style.clone()),
            font_weight: patch
                .font_weight
                .clone()
                .or_else(|| self.font_weight.clone()),
        }
    }
}

#[derive(Clone, Default)]
struct StylePatch {
    fill: Option<String>,
    stroke: Option<String>,
    font_color: Option<String>,
    font_style: Option<String>,
    font_weight: Option<String>,
}

#[derive(Default)]
struct StructuredControls {
    payload: String,
    highlights: Vec<HighlightSpec>,
    default_highlight: StylePatch,
    class_styles: BTreeMap<String, StylePatch>,
}

pub fn render_json_svg(document: &JsonDocument) -> String {
    let controls = parse_structured_controls(&document.raw, DataFamily::Json);
    let rows = json_render_rows(&controls.payload).unwrap_or_else(|| {
        document
            .nodes
            .iter()
            .map(|node| RenderRow {
                depth: node.depth,
                label: node.label.clone(),
                key: node.label.clone(),
                value: None,
                path: Vec::new(),
            })
            .collect()
    });
    render_structured_svg(
        document.title.as_deref(),
        DataFamily::Json,
        &rows,
        &controls,
    )
}

pub fn render_yaml_svg(document: &YamlDocument) -> String {
    let controls = parse_structured_controls(&document.raw, DataFamily::Yaml);
    let rows = yaml_render_rows(&controls.payload).unwrap_or_else(|| {
        document
            .nodes
            .iter()
            .map(|node| RenderRow {
                depth: node.depth,
                label: node.label.clone(),
                key: node.label.clone(),
                value: None,
                path: Vec::new(),
            })
            .collect()
    });
    render_structured_svg(
        document.title.as_deref(),
        DataFamily::Yaml,
        &rows,
        &controls,
    )
}

fn render_structured_svg(
    title: Option<&str>,
    family: DataFamily,
    rows: &[RenderRow],
    controls: &StructuredControls,
) -> String {
    let width = 760;
    let row_height = 24;
    let table_x = 24;
    let table_width = width - 48;
    let key_col_width = 236;
    let height = 82 + (rows.len().max(1) as i32) * row_height;
    let max_depth = rows.iter().map(|node| node.depth).max().unwrap_or(0);
    let projection = family.projection();
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\" data-projection=\"{}\" data-{}-node-count=\"{}\" data-{}-max-depth=\"{}\">",
        width,
        height,
        width,
        height,
        projection,
        projection,
        rows.len(),
        projection,
        max_depth
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    render_relation_marker_defs(&mut out, "#475569");
    let mut y = 28;
    out.push_str(&format!(
        "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\">{}</text>",
        y,
        escape_text(title.unwrap_or(family.title()))
    ));
    y += 28;
    if rows.is_empty() {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">(empty)</text>",
            y
        ));
    } else {
        let table_y = y - 16;
        let node_ys: Vec<i32> = rows
            .iter()
            .enumerate()
            .map(|(i, _)| y + (i as i32) * row_height)
            .collect();

        let normal_style = match family {
            DataFamily::Json => RowStyle::json_node(),
            DataFamily::Yaml => RowStyle::yaml_node(),
        };
        for (index, node) in rows.iter().enumerate() {
            let ny = node_ys[index];
            let row_top = ny - 16;
            let highlight = find_highlight(&node.path, controls);
            let (row_style, highlight_class) = match highlight {
                Some(spec) => {
                    let patch = spec
                        .class_name
                        .as_ref()
                        .and_then(|name| controls.class_styles.get(name))
                        .or(Some(&controls.default_highlight));
                    (
                        RowStyle::highlight().merge_patch(patch),
                        spec.class_name.as_deref(),
                    )
                }
                None => (normal_style.clone(), None),
            };
            let highlight_attr = if highlight.is_some() { "true" } else { "false" };
            let class_attr = highlight_class
                .map(|class_name| {
                    format!(
                        " data-{}-highlight-class=\"{}\"",
                        projection,
                        escape_text(class_name)
                    )
                })
                .unwrap_or_default();
            out.push_str(&format!(
                "<g class=\"data-tree-node {}-node {}-depth-{}{}\" data-projection=\"{}\" data-{}-index=\"{}\" data-{}-depth=\"{}\" data-{}-path=\"{}\" data-{}-highlight=\"{}\" data-{}-label=\"{}\"{}>",
                projection,
                projection,
                node.depth,
                if highlight.is_some() { " data-highlight" } else { "" },
                projection,
                projection,
                index,
                projection,
                node.depth,
                projection,
                escape_text(&path_attr(&node.path)),
                projection,
                highlight_attr,
                projection,
                escape_text(&node.label),
                class_attr
            ));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"none\"/>",
                table_x,
                row_top,
                table_width,
                row_height,
                escape_text(&row_style.fill),
            ));
            if index > 0 {
                out.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\" opacity=\"0.75\"/>",
                    table_x,
                    row_top,
                    table_x + table_width,
                    row_top,
                    escape_text(&normal_style.stroke)
                ));
            }
            out.push_str(&format!(
                "<line class=\"data-table-separator\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\" opacity=\"0.85\"/>",
                table_x + key_col_width,
                row_top,
                table_x + key_col_width,
                row_top + row_height,
                escape_text(&normal_style.stroke)
            ));
            if node.depth > 0 {
                let parent_y = (0..index)
                    .rev()
                    .find(|&j| rows[j].depth == node.depth - 1)
                    .map(|j| node_ys[j])
                    .unwrap_or(y);
                let connector_x = table_x + 12 + ((node.depth as i32) - 1) * 18;
                let key_x = table_x + 8 + (node.depth as i32) * 18;
                out.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"{}/>",
                    connector_x,
                    parent_y,
                    connector_x,
                    ny,
                    family.connector_color(),
                    family.connector_dash()
                ));
                out.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"{}/>",
                    connector_x,
                    ny,
                    key_x - 4,
                    ny,
                    family.connector_color(),
                    family.connector_dash()
                ));
            }
            let mut text_attrs = "font-family=\"monospace\" font-size=\"12\"".to_string();
            if let Some(font_style) = &row_style.font_style {
                text_attrs.push_str(&format!(" font-style=\"{}\"", escape_text(font_style)));
            }
            if let Some(font_weight) = &row_style.font_weight {
                text_attrs.push_str(&format!(" font-weight=\"{}\"", escape_text(font_weight)));
            }
            let key_x = table_x + 8 + (node.depth as i32) * 18;
            if !node.key.is_empty() {
                out.push_str(&creole_text(
                    key_x,
                    ny + 4,
                    &text_attrs,
                    &node.key,
                    &row_style.font_color,
                ));
            }
            if let Some(value) = &node.value {
                out.push_str(&creole_text(
                    table_x + key_col_width + 8,
                    ny + 4,
                    &text_attrs,
                    value,
                    &row_style.font_color,
                ));
            }
            out.push_str("</g>");
        }
        out.push_str(&format!(
            "<rect class=\"data-table-frame {}-table\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" ry=\"3\" fill=\"none\" stroke=\"{}\" stroke-width=\"1\"/>",
            projection,
            table_x,
            table_y,
            table_width,
            rows.len() as i32 * row_height,
            escape_text(&normal_style.stroke)
        ));
    }
    out.push_str("</svg>");
    out
}

fn find_highlight<'a>(
    path: &[String],
    controls: &'a StructuredControls,
) -> Option<&'a HighlightSpec> {
    controls
        .highlights
        .iter()
        .find(|highlight| highlight.path == path)
}

fn parse_structured_controls(raw: &str, family: DataFamily) -> StructuredControls {
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

fn parse_highlight(line: &str) -> Option<HighlightSpec> {
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

fn parse_style_lines(style_source: &str, family: DataFamily, controls: &mut StructuredControls) {
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

fn apply_style_property(
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

fn apply_patch_property(patch: &mut StylePatch, key: &str, value: &str) {
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

fn json_render_rows(raw: &str) -> Option<Vec<RenderRow>> {
    let value = serde_json::from_str::<serde_json::Value>(raw.trim()).ok()?;
    let mut rows = Vec::new();
    flatten_json_render_value(&value, None, Vec::new(), 0, &mut rows);
    Some(rows)
}

fn flatten_json_render_value(
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

fn structured_render_row(
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

fn json_string_label(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| format!("\"{value}\""))
}

fn yaml_render_rows(raw: &str) -> Option<Vec<RenderRow>> {
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

fn flatten_yaml_render_value(
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

fn path_attr(path: &[String]) -> String {
    if path.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", path.join("/"))
    }
}
