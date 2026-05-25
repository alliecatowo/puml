use super::text::TextOutputMode;
use super::text_output::{finish_text, optional_label, push_meta, text_value};
use crate::model::{BoardDocument, FileTreeNode, FilesDocument, WireDocument, WireEndpoint};

pub(super) fn render_board_text(doc: &BoardDocument, mode: TextOutputMode) -> String {
    let mut lines = Vec::new();
    lines.push("board".to_string());
    push_meta(&mut lines, "title", doc.title.as_deref(), mode);
    for column in &doc.columns {
        lines.push(format!("column {}", text_value(&column.title, mode)));
        for card in &column.cards {
            let tags = if card.tags.is_empty() {
                String::new()
            } else {
                format!(" #{}", card.tags.join(" #"))
            };
            lines.push(format!(
                "{}{}{}",
                spaces(card.depth),
                text_value(&card.title, mode),
                tags
            ));
        }
    }
    finish_text(lines)
}

pub(super) fn render_files_text(doc: &FilesDocument, mode: TextOutputMode) -> String {
    let mut lines = Vec::new();
    lines.push("files".to_string());
    push_meta(&mut lines, "title", doc.title.as_deref(), mode);
    for note in &doc.top_notes {
        lines.push(format!("note {}", text_value(note, mode)));
    }
    for node in &doc.roots {
        push_file_text_node(&mut lines, node, 0, mode);
    }
    finish_text(lines)
}

fn push_file_text_node(
    lines: &mut Vec<String>,
    node: &FileTreeNode,
    depth: usize,
    mode: TextOutputMode,
) {
    let kind = if node.is_dir { "dir" } else { "file" };
    lines.push(format!(
        "{}{} {}",
        spaces(depth + 1),
        kind,
        text_value(&node.name, mode)
    ));
    for note in &node.notes {
        lines.push(format!(
            "{}note {}",
            spaces(depth + 2),
            text_value(note, mode)
        ));
    }
    for child in &node.children {
        push_file_text_node(lines, child, depth + 1, mode);
    }
}

pub(super) fn render_wire_text(doc: &WireDocument, mode: TextOutputMode) -> String {
    let mut lines = Vec::new();
    lines.push("wire".to_string());
    push_meta(&mut lines, "title", doc.title.as_deref(), mode);
    lines.push(format!("components ({})", doc.components.len()));
    for component in &doc.components {
        lines.push(format!(
            "  {} [{:.0}x{:.0}] at {:.0},{:.0}",
            text_value(&component.label, mode),
            component.width,
            component.height,
            component.x,
            component.y
        ));
        for port in &component.ports {
            lines.push(format!(
                "    {} {}",
                port.side.as_str(),
                text_value(&port.label, mode)
            ));
        }
    }
    lines.push(format!("links ({})", doc.links.len()));
    for link in &doc.links {
        let arrow = if link.directed { "-->" } else { "--" };
        let label = optional_label(link.label.as_deref(), mode);
        lines.push(format!(
            "  {} {arrow} {}{label}",
            wire_endpoint_text(&link.from, mode),
            wire_endpoint_text(&link.to, mode)
        ));
    }
    finish_text(lines)
}

fn wire_endpoint_text(endpoint: &WireEndpoint, mode: TextOutputMode) -> String {
    match &endpoint.port {
        Some(port) => format!(
            "{}.{}",
            text_value(&endpoint.component, mode),
            text_value(port, mode)
        ),
        None => text_value(&endpoint.component, mode),
    }
}

fn spaces(indent: usize) -> String {
    "  ".repeat(indent)
}
