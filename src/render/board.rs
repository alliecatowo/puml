use super::svg::escape_text;
use crate::model::{FamilyDocument, FamilyNodeKind};

// Palette constants — defined here so they're never embedded inside format!()
// calls where the leading `#` could be misread as a format specifier.
const BG_FILL: &str = "#f0f4f8";
const COL_FILL: &str = "#dde4ee";
const HEADER_FILL: &str = "#4a90d9";
const HEADER_TEXT: &str = "#ffffff";
const CARD_FILL: &str = "#ffffff";
const CARD_BORDER: &str = "#cccccc";
const CARD_TEXT: &str = "#333333";
const EMPTY_TEXT: &str = "#888888";

/// Render a `@startboard` / `@endboard` Kanban diagram as SVG.
///
/// Layout: columns arranged horizontally. Each column has a header banner
/// and a stack of cards below it. Depth-0 nodes are column headers;
/// depth-1+ nodes are cards nested in the nearest preceding column.
pub fn render_board_svg(doc: &FamilyDocument) -> String {
    // Layout constants
    const COL_W: i32 = 200;
    const COL_GAP: i32 = 20;
    const CARD_H: i32 = 40;
    const CARD_GAP: i32 = 8;
    const HEADER_H: i32 = 36;
    const MARGIN: i32 = 20;
    const CARD_RADIUS: i32 = 6;
    const HEADER_RADIUS: i32 = 8;

    // Collect columns and their cards
    struct Column {
        name: String,
        cards: Vec<String>,
    }

    let mut columns: Vec<Column> = Vec::new();
    for node in &doc.nodes {
        match node.kind {
            FamilyNodeKind::BoardColumn => {
                columns.push(Column {
                    name: node.name.clone(),
                    cards: Vec::new(),
                });
            }
            FamilyNodeKind::BoardCard => {
                if let Some(col) = columns.last_mut() {
                    col.cards.push(node.name.clone());
                }
            }
            _ => {}
        }
    }

    if columns.is_empty() {
        let mut out = String::new();
        out.push_str(r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="80">"#);
        out.push_str(&format!(
            r#"<rect width="200" height="80" fill="{bg}"/>"#,
            bg = BG_FILL
        ));
        out.push_str(&format!(
            r#"<text x="100" y="44" text-anchor="middle" font-family="sans-serif" font-size="14" fill="{c}">empty board</text>"#,
            c = EMPTY_TEXT
        ));
        out.push_str("</svg>");
        return out;
    }

    // Compute column heights
    let max_cards = columns.iter().map(|c| c.cards.len()).max().unwrap_or(0);
    let col_h = HEADER_H + (CARD_H + CARD_GAP) * (max_cards as i32).max(1) + CARD_GAP;

    let n_cols = columns.len() as i32;
    let total_w = MARGIN * 2 + n_cols * COL_W + (n_cols - 1) * COL_GAP;
    let total_h = MARGIN * 2 + col_h;

    let mut svg = String::new();
    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}">"#,
        w = total_w,
        h = total_h,
    ));
    svg.push_str(&format!(
        r#"<rect width="{w}" height="{h}" fill="{bg}"/>"#,
        w = total_w,
        h = total_h,
        bg = BG_FILL,
    ));

    // Title
    if let Some(title) = &doc.title {
        let title_color = CARD_TEXT;
        svg.push_str(&format!(
            r#"<text x="{x}" y="16" text-anchor="middle" font-family="sans-serif" font-size="14" font-weight="bold" fill="{c}">{t}</text>"#,
            x = total_w / 2,
            c = title_color,
            t = escape_text(title),
        ));
    }

    for (ci, col) in columns.iter().enumerate() {
        let cx = MARGIN + ci as i32 * (COL_W + COL_GAP);
        let cy = MARGIN;

        // Column background
        svg.push_str(&format!(
            r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}" fill="{fill}"/>"#,
            x = cx,
            y = cy,
            w = COL_W,
            h = col_h,
            r = HEADER_RADIUS,
            fill = COL_FILL,
        ));

        // Column header banner
        svg.push_str(&format!(
            r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}" fill="{fill}"/>"#,
            x = cx,
            y = cy,
            w = COL_W,
            h = HEADER_H,
            r = HEADER_RADIUS,
            fill = HEADER_FILL,
        ));
        // Square off the bottom corners of the header
        svg.push_str(&format!(
            r#"<rect x="{x}" y="{y}" width="{w}" height="{rh}" fill="{fill}"/>"#,
            x = cx,
            y = cy + HEADER_H - HEADER_RADIUS,
            w = COL_W,
            rh = HEADER_RADIUS,
            fill = HEADER_FILL,
        ));

        svg.push_str(&format!(
            r#"<text x="{x}" y="{y}" text-anchor="middle" dominant-baseline="middle" font-family="sans-serif" font-size="13" font-weight="bold" fill="{fill}">{name}</text>"#,
            x = cx + COL_W / 2,
            y = cy + HEADER_H / 2,
            fill = HEADER_TEXT,
            name = escape_text(&col.name),
        ));

        // Cards
        for (i, card) in col.cards.iter().enumerate() {
            let card_x = cx + CARD_GAP;
            let card_y = cy + HEADER_H + CARD_GAP + i as i32 * (CARD_H + CARD_GAP);
            let card_w = COL_W - CARD_GAP * 2;

            svg.push_str(&format!(
                r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}" fill="{fill}" stroke="{stroke}" stroke-width="1"/>"#,
                x = card_x,
                y = card_y,
                w = card_w,
                h = CARD_H,
                r = CARD_RADIUS,
                fill = CARD_FILL,
                stroke = CARD_BORDER,
            ));

            svg.push_str(&format!(
                r#"<text x="{x}" y="{y}" text-anchor="middle" dominant-baseline="middle" font-family="sans-serif" font-size="12" fill="{fill}">{name}</text>"#,
                x = card_x + card_w / 2,
                y = card_y + CARD_H / 2,
                fill = CARD_TEXT,
                name = escape_text(card),
            ));
        }
    }

    svg.push_str("</svg>");
    svg
}
