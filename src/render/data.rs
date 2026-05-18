use super::*;

pub fn render_json_svg(document: &JsonDocument) -> String {
    let width = 760;
    let height = 80 + (document.nodes.len().max(1) as i32) * 22;
    let max_depth = document
        .nodes
        .iter()
        .map(|node| node.depth)
        .max()
        .unwrap_or(0);
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\" data-projection=\"json\" data-json-node-count=\"{}\" data-json-max-depth=\"{}\">",
        width,
        height,
        width,
        height,
        document.nodes.len(),
        max_depth
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    render_relation_marker_defs(&mut out, "#475569");
    let mut y = 28;
    out.push_str(&format!(
        "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\">{}</text>",
        y,
        escape_text(document.title.as_deref().unwrap_or("JSON"))
    ));
    y += 28;
    if document.nodes.is_empty() {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">(empty)</text>",
            y
        ));
    } else {
        for (index, node) in document.nodes.iter().enumerate() {
            let x = 24 + (node.depth as i32) * 18;
            out.push_str(&format!(
                "<g class=\"data-tree-node json-node json-depth-{}\" data-projection=\"json\" data-json-index=\"{}\" data-json-depth=\"{}\" data-json-label=\"{}\">",
                node.depth,
                index,
                node.depth,
                escape_text(&node.label)
            ));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"18\" rx=\"3\" ry=\"3\" fill=\"#f1f5f9\" stroke=\"#94a3b8\" stroke-width=\"1\"/>",
                x,
                y - 12,
                (width - 48 - (node.depth as i32) * 18).max(80)
            ));
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#0f172a\">{}</text>",
                x + 6,
                y + 2,
                escape_text(&node.label)
            ));
            out.push_str("</g>");
            y += 22;
        }
    }
    out.push_str("</svg>");
    out
}

// ─── State diagram renderer ──────────────────────────────────────────────────

pub fn render_yaml_svg(document: &YamlDocument) -> String {
    let width = 760;
    let height = 80 + (document.nodes.len().max(1) as i32) * 22;
    let max_depth = document
        .nodes
        .iter()
        .map(|node| node.depth)
        .max()
        .unwrap_or(0);
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\" data-projection=\"yaml\" data-yaml-node-count=\"{}\" data-yaml-max-depth=\"{}\">",
        width,
        height,
        width,
        height,
        document.nodes.len(),
        max_depth
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    render_relation_marker_defs(&mut out, "#475569");
    let mut y = 28;
    out.push_str(&format!(
        "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\">{}</text>",
        y,
        escape_text(document.title.as_deref().unwrap_or("YAML"))
    ));
    y += 28;
    if document.nodes.is_empty() {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">(empty)</text>",
            y
        ));
    } else {
        for (index, node) in document.nodes.iter().enumerate() {
            let x = 24 + (node.depth as i32) * 18;
            out.push_str(&format!(
                "<g class=\"data-tree-node yaml-node yaml-depth-{}\" data-projection=\"yaml\" data-yaml-index=\"{}\" data-yaml-depth=\"{}\" data-yaml-label=\"{}\">",
                node.depth,
                index,
                node.depth,
                escape_text(&node.label)
            ));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"18\" rx=\"3\" ry=\"3\" fill=\"#fef9c3\" stroke=\"#ca8a04\" stroke-width=\"1\"/>",
                x,
                y - 12,
                (width - 48 - (node.depth as i32) * 18).max(80)
            ));
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#0f172a\">{}</text>",
                x + 6,
                y + 2,
                escape_text(&node.label)
            ));
            out.push_str("</g>");
            y += 22;
        }
    }
    out.push_str("</svg>");
    out
}
