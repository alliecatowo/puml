use super::*;

#[derive(Clone)]
pub(super) struct ProjectionTreeRow {
    depth: usize,
    label: String,
}

/// Extract deterministic display rows from a JSON/YAML projection body.
pub(super) fn extract_projection_tree_rows(body: &str, format: &str) -> Vec<ProjectionTreeRow> {
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

pub(super) fn parse_projection_yaml_value(body: &str) -> Option<yaml_rust2::Yaml> {
    yaml_rust2::YamlLoader::load_from_str(body.trim())
        .ok()
        .and_then(|docs| {
            docs.into_iter()
                .find(|doc| !matches!(doc, yaml_rust2::Yaml::BadValue))
        })
}

pub(super) fn parse_projection_json_value(body: &str) -> Option<serde_json::Value> {
    let trimmed = body.trim();
    serde_json::from_str::<serde_json::Value>(trimmed)
        .ok()
        .or_else(|| serde_json::from_str::<serde_json::Value>(&format!("{{{trimmed}}}")).ok())
}

pub(super) fn family_projection_extra_height(projections: &[crate::model::JsonProjection]) -> i32 {
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
    projections: &[crate::model::JsonProjection],
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

pub(super) fn collect_projection_json_rows(
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

pub(super) fn collect_projection_yaml_rows(
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

pub(super) fn projection_yaml_label(value: &yaml_rust2::Yaml) -> String {
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

pub(super) fn extract_yaml_kv_lines(body: &str) -> Vec<String> {
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
pub(super) fn extract_json_kv_lines(body: &str) -> Vec<String> {
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
pub(super) fn parse_json_kv_display(segment: &str) -> Option<String> {
    // Expect: optional quote, key chars, optional quote, `:`, value
    let (key_part, val_part) = segment.split_once(':')?;
    let key = key_part.trim().trim_matches('"');
    let val = val_part.trim().trim_matches('"');
    if key.is_empty() {
        return None;
    }
    Some(format!("{key}: {val}"))
}

pub fn render_family_tree_svg(document: &FamilyDocument) -> String {
    const MARGIN: i32 = 24;
    const CHAR_WIDTH: i32 = 7;
    const NODE_FONT_SIZE: i32 = 12;
    const NODE_MIN_WIDTH: i32 = 220;
    const NODE_MAX_WIDTH: i32 = 360;
    const NODE_PADDING_X: i32 = 12;
    const NODE_PADDING_Y: i32 = 12;
    const MIN_SPACING_X: i32 = 80;
    const MIN_SPACING_Y: i32 = 48;
    const MAX_LINE_CHARS: usize = 24;

    let mut out = String::new();
    let title_lines = document
        .title
        .as_deref()
        .map(|v| v.lines().collect::<Vec<_>>())
        .unwrap_or_default();

    let hide_empty_members = document.hide_options.contains("empty members")
        || document.hide_options.contains("empty methods")
        || document.hide_options.contains("empty fields");
    let hide_circle = document.hide_options.contains("circle");
    let hide_stereotype = document.hide_options.contains("stereotype");

    let mut layouts = Vec::with_capacity(document.nodes.len());
    for node in &document.nodes {
        let raw_label = node.alias.as_ref().map_or_else(
            || node.name.clone(),
            |alias| format!("{} as {}", node.name, alias),
        );
        let lines = wrap_text(raw_label, MAX_LINE_CHARS, document.text_overflow_policy);
        let width_chars = lines
            .iter()
            .map(|line| line.chars().count() as i32)
            .max()
            .unwrap_or(1);
        let width =
            (width_chars * CHAR_WIDTH + (NODE_PADDING_X * 2)).clamp(NODE_MIN_WIDTH, NODE_MAX_WIDTH);
        let member_count = if hide_empty_members && node.members.is_empty() {
            0
        } else {
            node.members.len() as i32
        };
        let height = (lines.len() as i32 * 18) + (NODE_PADDING_Y * 2) + (member_count * 16);
        layouts.push(NodeLayout {
            label_lines: lines,
            width,
            height,
            x: 0,
            y: 0,
        });
    }

    let mut levels = Vec::<Vec<usize>>::new();
    let mut max_depth = 0usize;
    for (idx, node) in document.nodes.iter().enumerate() {
        let depth = node.depth;
        if depth > max_depth {
            max_depth = depth;
        }
        if levels.len() <= depth {
            levels.resize_with(depth + 1, Vec::new);
        }
        levels[depth].push(idx);
    }

    let mut depth_slot = vec![0usize; document.nodes.len()];
    for level_nodes in &levels {
        for (slot, idx) in level_nodes.iter().copied().enumerate() {
            depth_slot[idx] = slot;
        }
    }

    let max_node_width = layouts
        .iter()
        .map(|layout| layout.width)
        .max()
        .unwrap_or(NODE_MIN_WIDTH);
    let max_node_height = layouts
        .iter()
        .map(|layout| layout.height)
        .max()
        .unwrap_or(58);

    let x_step = max_node_width + MIN_SPACING_X;
    let y_step = max_node_height + MIN_SPACING_Y;

    let mut y_offsets = vec![0i32; levels.len()];
    for i in 1..levels.len() {
        let prev = y_offsets[i - 1] + y_step;
        y_offsets[i] = prev;
    }

    let vertical = matches!(
        document.orientation,
        FamilyOrientation::TopToBottom | FamilyOrientation::BottomToTop
    );

    let mut height_offset = MARGIN;
    if !title_lines.is_empty() {
        height_offset += (title_lines.len() as i32) * 24;
        height_offset += 12;
    }
    // Extra space for groups
    height_offset += (document.groups.len() as i32) * 48;

    for (depth, level_nodes) in levels.iter().enumerate() {
        for &node_idx in level_nodes {
            let slot = depth_slot[node_idx] as i32;
            let display_depth = match document.orientation {
                FamilyOrientation::TopToBottom => depth,
                FamilyOrientation::BottomToTop => max_depth.saturating_sub(depth),
                FamilyOrientation::LeftToRight => depth,
                FamilyOrientation::RightToLeft => max_depth.saturating_sub(depth),
            };

            if vertical {
                layouts[node_idx].x = MARGIN + (slot * x_step);
                layouts[node_idx].y = height_offset + (display_depth as i32 * y_step);
            } else {
                layouts[node_idx].x = MARGIN + (display_depth as i32 * x_step);
                layouts[node_idx].y = MARGIN + (slot * y_step);
            }
        }
    }

    let mut max_x = MARGIN;
    let mut max_y = height_offset;
    for layout in &layouts {
        max_x = max_x.max(layout.x + layout.width);
        max_y = max_y.max(layout.y + layout.height);
    }
    if !title_lines.is_empty() {
        max_y = max_y.max(height_offset);
    }

    let width = (max_x + MARGIN).max(760);
    let height = (max_y + MARGIN).max(180);

    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        width, height, width, height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");

    let mut y_cursor = MARGIN;
    if !title_lines.is_empty() {
        for line in &title_lines {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\">{}</text>",
                MARGIN,
                y_cursor,
                escape_text(line)
            ));
            y_cursor += 24;
        }
        y_cursor += 12;
    }
    // Render groups (together/package/namespace) as labeled frames before class boxes
    for group in &document.groups {
        let group_label = match group.label.as_deref() {
            // `rectangle` is a visual-boundary keyword; show just the label (fix #553)
            Some(lbl) if group.kind == "rectangle" => lbl.to_string(),
            Some(lbl) => format!("{} {}", group.kind, lbl),
            None => group.kind.clone(),
        };
        let member_list = group.member_ids.join(", ");
        out.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"200\" height=\"40\" rx=\"6\" ry=\"6\" fill=\"#f0f4ff\" stroke=\"#6366f1\" stroke-width=\"1.5\"/>",
            MARGIN,
            y_cursor
        ));
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" font-weight=\"600\" fill=\"#4338ca\">{}</text>",
            MARGIN + 8,
            y_cursor + 14,
            escape_text(&group_label)
        ));
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" fill=\"#6366f1\">{}</text>",
            MARGIN + 8,
            y_cursor + 28,
            escape_text(&member_list)
        ));
        y_cursor += 48;
    }

    for (idx, layout) in layouts.iter().enumerate() {
        out.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" ry=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            layout.x,
            layout.y,
            layout.width,
            layout.height,
            document.style.participant_background_color,
            document.style.participant_border_color
        ));

        let node = &document.nodes[idx];
        // Render label lines (name, alias)
        for (line_idx, line) in layout.label_lines.iter().enumerate() {
            let tx = if !hide_circle && node.kind == crate::model::FamilyNodeKind::Class {
                layout.x + NODE_PADDING_X + 16
            } else {
                layout.x + NODE_PADDING_X
            };
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"{}\" fill=\"#0f172a\">{}</text>",
                tx,
                layout.y + NODE_PADDING_Y + (line_idx as i32 * 18),
                NODE_FONT_SIZE,
                escape_text(line)
            ));
        }
        // Class circle icon
        if !hide_circle && node.kind == crate::model::FamilyNodeKind::Class {
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"6\" fill=\"none\" stroke=\"#64748b\" stroke-width=\"1\"/>",
                layout.x + NODE_PADDING_X + 8,
                layout.y + NODE_PADDING_Y + 6
            ));
        }
        // Render members with visibility markers + modifier styling
        let show_members = !hide_empty_members || !node.members.is_empty();
        if show_members {
            // Detect abstract/interface nodes so members can be rendered italic (fix #767)
            let node_is_abstract = node
                .members
                .first()
                .and_then(|m| builtin_type_stereotype_label(&m.text))
                .map(|lbl| lbl == "\u{ab}abstract\u{bb}" || lbl == "\u{ab}interface\u{bb}")
                .unwrap_or(false);
            let member_y_base =
                layout.y + NODE_PADDING_Y + (layout.label_lines.len() as i32 * 18) + 4;
            for (midx, member) in node.members.iter().enumerate() {
                let my = member_y_base + (midx as i32 * 16);
                let (symbol, color, member_text) = parse_visibility_member(&member.text);
                if let Some(sym) = symbol {
                    out.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
                        layout.x + NODE_PADDING_X,
                        my,
                        color,
                        escape_text(sym)
                    ));
                }
                let (base_style, clean_text) = parse_member_modifiers(member_text);
                let mut extra_style = String::from(base_style);
                match &member.modifier {
                    Some(MemberModifier::Abstract) | Some(MemberModifier::Field) => {
                        if !extra_style.contains("font-style") {
                            extra_style.push_str(" font-style=\"italic\"");
                        }
                    }
                    Some(MemberModifier::Static) => {
                        if !extra_style.contains("text-decoration") {
                            extra_style.push_str(" text-decoration=\"underline\"");
                        }
                    }
                    Some(MemberModifier::Method) | None => {
                        // Interface members are implicitly abstract — render in italic (fix #767)
                        if node_is_abstract && !extra_style.contains("font-style") {
                            extra_style.push_str(" font-style=\"italic\"");
                        }
                    }
                }
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#334155\"{}>{}</text>",
                    layout.x + NODE_PADDING_X + 12,
                    my,
                    extra_style,
                    escape_text(clean_text)
                ));
            }
        }
    }
    let _ = hide_stereotype; // used in branch version; suppress warning

    for relation in &document.relations {
        let from_idx = document
            .nodes
            .iter()
            .position(|node| node.name == relation.from)
            .or_else(|| {
                document
                    .nodes
                    .iter()
                    .position(|node| node.alias.as_deref() == Some(relation.from.as_str()))
            });
        let to_idx = document
            .nodes
            .iter()
            .position(|node| node.name == relation.to)
            .or_else(|| {
                document
                    .nodes
                    .iter()
                    .position(|node| node.alias.as_deref() == Some(relation.to.as_str()))
            });

        if let (Some(from), Some(to)) = (from_idx, to_idx) {
            let from_layout = &layouts[from];
            let to_layout = &layouts[to];
            let (x1, y1, x2, y2) = match document.orientation {
                FamilyOrientation::TopToBottom => (
                    from_layout.x + from_layout.width / 2,
                    from_layout.y + from_layout.height,
                    to_layout.x + to_layout.width / 2,
                    to_layout.y,
                ),
                FamilyOrientation::BottomToTop => (
                    from_layout.x + from_layout.width / 2,
                    from_layout.y,
                    to_layout.x + to_layout.width / 2,
                    to_layout.y + to_layout.height,
                ),
                FamilyOrientation::LeftToRight => (
                    from_layout.x + from_layout.width,
                    from_layout.y + from_layout.height / 2,
                    to_layout.x,
                    to_layout.y + to_layout.height / 2,
                ),
                FamilyOrientation::RightToLeft => (
                    from_layout.x,
                    from_layout.y + from_layout.height / 2,
                    to_layout.x + to_layout.width,
                    to_layout.y + to_layout.height / 2,
                ),
            };

            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                x1, y1, x2, y2, document.style.arrow_color
            ));
            render_tree_arrow(&mut out, x1, y1, x2, y2, &document.style.arrow_color);

            if let Some(label) = &relation.label {
                let label = usecase_dependency_label(Some(label)).unwrap_or(label);
                let label_lines = wrap_text(label.to_string(), 18, document.text_overflow_policy);
                let label_x = ((x1 + x2) / 2).max(4);
                let label_y = ((y1 + y2) / 2).min(height - 8);
                for (line_idx, line) in label_lines.iter().enumerate() {
                    out.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\">{}</text>",
                        label_x,
                        label_y + (line_idx as i32 * 12),
                        escape_text(line)
                    ));
                }
            }
        }
    }

    let relation_count = if document.relations.is_empty() {
        "relationships: 0".to_string()
    } else {
        format!("relationships: {}", document.relations.len())
    };
    out.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#475569\">{}</text>",
        MARGIN,
        height - 12,
        relation_count
    ));

    out.push_str("</svg>");
    out
}
