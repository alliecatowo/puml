use super::*;

const BOARD_COLUMN_W: i32 = 220;
const BOARD_GAP: i32 = 18;
const BOARD_MARGIN: i32 = 24;
const BOARD_HEADER_H: i32 = 34;
const BOARD_CARD_H: i32 = 46;

pub fn render_board_svg(document: &BoardDocument) -> String {
    let columns = document.columns.len().max(1) as i32;
    let max_cards = document
        .columns
        .iter()
        .map(|column| column.cards.len())
        .max()
        .unwrap_or(0) as i32;
    let title_h = document.title.as_ref().map(|_| 30).unwrap_or(0);
    let width = BOARD_MARGIN * 2 + columns * BOARD_COLUMN_W + (columns - 1) * BOARD_GAP;
    let height =
        BOARD_MARGIN * 2 + title_h + BOARD_HEADER_H + 16 + max_cards.max(1) * (BOARD_CARD_H + 10);
    let mut out = svg_root(width, height);
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"#f8fafc\"/>");

    let mut y = BOARD_MARGIN;
    if let Some(title) = &document.title {
        out.push_str(&format!(
            "<text class=\"board-title\" x=\"{BOARD_MARGIN}\" y=\"{y}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"700\" fill=\"#0f172a\">{}</text>",
            escape_text(title)
        ));
        y += title_h;
    }

    for (idx, column) in document.columns.iter().enumerate() {
        let x = BOARD_MARGIN + idx as i32 * (BOARD_COLUMN_W + BOARD_GAP);
        let col_h = height - y - BOARD_MARGIN;
        out.push_str(&format!(
            "<g class=\"board-column\" data-board-column=\"{}\">",
            escape_text(&column.title)
        ));
        out.push_str(&format!(
            "<rect x=\"{x}\" y=\"{y}\" width=\"{BOARD_COLUMN_W}\" height=\"{col_h}\" rx=\"8\" ry=\"8\" fill=\"#e2e8f0\" stroke=\"#94a3b8\"/>"
        ));
        out.push_str(&format!(
            "<rect x=\"{x}\" y=\"{y}\" width=\"{BOARD_COLUMN_W}\" height=\"{BOARD_HEADER_H}\" rx=\"8\" ry=\"8\" fill=\"#334155\" stroke=\"#334155\"/>"
        ));
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"13\" font-weight=\"700\" fill=\"white\">{}</text>",
            x + 12,
            y + 22,
            escape_text(&column.title)
        ));
        let mut card_y = y + BOARD_HEADER_H + 12;
        for card in &column.cards {
            render_board_card(&mut out, x + 10, card_y, card);
            card_y += BOARD_CARD_H + 10;
        }
        out.push_str("</g>");
    }

    out.push_str("</svg>");
    out
}

fn render_board_card(out: &mut String, x: i32, y: i32, card: &BoardCard) {
    let indent = ((card.depth.saturating_sub(1) as i32) * 12).min(48);
    let card_x = x + indent;
    let card_w = BOARD_COLUMN_W - 20 - indent;
    let fill = match card.depth {
        1 => "#ffffff",
        2 => "#fefce8",
        3 => "#ecfdf5",
        _ => "#eef2ff",
    };
    out.push_str(&format!(
        "<g class=\"board-card\" data-board-depth=\"{}\">",
        card.depth
    ));
    out.push_str(&format!(
        "<rect x=\"{card_x}\" y=\"{y}\" width=\"{card_w}\" height=\"{BOARD_CARD_H}\" rx=\"6\" ry=\"6\" fill=\"{fill}\" stroke=\"#cbd5e1\"/>"
    ));
    out.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#0f172a\">{}</text>",
        card_x + 10,
        y + 20,
        escape_text(&truncate_label(&card.title, 24))
    ));
    if !card.tags.is_empty() {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" fill=\"#475569\">#{}</text>",
            card_x + 10,
            y + 36,
            escape_text(&card.tags.join(" #"))
        ));
    }
    out.push_str("</g>");
}

pub fn render_files_svg(document: &FilesDocument) -> String {
    let row_count = count_file_rows(&document.roots) + document.top_notes.len();
    let title_h = document.title.as_ref().map(|_| 30).unwrap_or(0);
    let width = 760;
    let height = 56 + title_h + (row_count.max(1) as i32 * 28);
    let mut out = svg_root(width, height);
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");

    let mut y = 28;
    if let Some(title) = &document.title {
        out.push_str(&format!(
            "<text class=\"files-title\" x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"700\" fill=\"#0f172a\">{}</text>",
            escape_text(title)
        ));
        y += 30;
    }
    for note in &document.top_notes {
        render_file_note(&mut out, 24, y - 14, note);
        y += 28;
    }
    for node in &document.roots {
        render_file_node(&mut out, node, 0, &mut y);
    }
    out.push_str("</svg>");
    out
}

fn render_file_node(out: &mut String, node: &FileTreeNode, depth: usize, y: &mut i32) {
    let x = 24 + depth as i32 * 24;
    let icon = if node.is_dir { "dir" } else { "file" };
    out.push_str(&format!(
        "<g class=\"files-entry\" data-files-path=\"{}\" data-files-kind=\"{icon}\">",
        escape_text(&node.path)
    ));
    if node.is_dir {
        out.push_str(&format!(
            "<path d=\"M{x} {} h9 l3 4 h14 v16 h-26 z\" fill=\"#dbeafe\" stroke=\"#2563eb\"/>",
            *y - 14
        ));
    } else {
        out.push_str(&format!(
            "<path d=\"M{x} {} h17 l7 7 v13 h-24 z\" fill=\"#f8fafc\" stroke=\"#64748b\"/>",
            *y - 14
        ));
    }
    out.push_str(&format!(
        "<text x=\"{}\" y=\"{y}\" font-family=\"monospace\" font-size=\"13\" fill=\"#0f172a\">{}</text>",
        x + 34,
        escape_text(&node.name)
    ));
    out.push_str("</g>");
    *y += 28;
    for note in &node.notes {
        render_file_note(out, x + 34, *y - 16, note);
        *y += 28;
    }
    for child in &node.children {
        render_file_node(out, child, depth + 1, y);
    }
}

fn render_file_note(out: &mut String, x: i32, y: i32, note: &str) {
    let text = note.replace('\n', " / ");
    out.push_str(&format!(
        "<g class=\"files-note\"><rect x=\"{x}\" y=\"{y}\" width=\"360\" height=\"22\" rx=\"5\" ry=\"5\" fill=\"#fff7ed\" stroke=\"#fdba74\"/><text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#7c2d12\">{}</text></g>",
        x + 8,
        y + 15,
        escape_text(&truncate_label(&text, 46))
    ));
}

fn count_file_rows(nodes: &[FileTreeNode]) -> usize {
    nodes
        .iter()
        .map(|node| 1 + node.notes.len() + count_file_rows(&node.children))
        .sum()
}

fn svg_root(width: i32, height: i32) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">"
    )
}

fn truncate_label(label: &str, max_chars: usize) -> String {
    let mut chars = label.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}
