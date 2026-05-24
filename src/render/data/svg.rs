use super::*;

pub(super) fn render_structured_svg(
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
                let parent_index = (0..index).rev().find(|&j| rows[j].depth == node.depth - 1);
                let parent_y = parent_index.map(|j| node_ys[j]).unwrap_or(y);
                let connector_start_y = parent_index
                    .filter(|&j| rows[j].path.is_empty() && rows[j].key.is_empty())
                    .map(|_| row_top)
                    .unwrap_or(parent_y);
                let connector_x = table_x + 12 + ((node.depth as i32) - 1) * 18;
                let key_x = table_x + 8 + (node.depth as i32) * 18;
                out.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"{}/>",
                    connector_x,
                    connector_start_y,
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

pub(super) fn find_highlight<'a>(
    path: &[String],
    controls: &'a StructuredControls,
) -> Option<&'a HighlightSpec> {
    controls
        .highlights
        .iter()
        .find(|highlight| highlight.path == path)
}
