use super::svg::escape_text;
use crate::model::{FamilyDocument, FamilyNodeKind};

// Color palette — kept as separate constants to avoid embedding bare `#NNN`
// inside format!() argument strings, which the compiler would misparse.
const BG_FILL: &str = "#ffffff";
const TEXT_FILL: &str = "#222222";
const FOLDER_FILL: &str = "#f5c542";
const FILE_FILL: &str = "#7ab3e0";
const FOLDER_STROKE: &str = "#c8a000";
const FILE_STROKE: &str = "#3d7ab5";
const EMPTY_COLOR: &str = "#888888";

/// Render a `@startfiles` / `@endfiles` file tree diagram as SVG.
///
/// Layout: left-anchored tree. Each node is indented by `depth * INDENT_PX`.
/// Folders get a folder-tab icon; files get a document icon.
pub fn render_files_svg(doc: &FamilyDocument) -> String {
    const INDENT_PX: i32 = 20;
    const ROW_H: i32 = 26;
    const ICON_W: i32 = 18;
    const MARGIN_LEFT: i32 = 16;
    const MARGIN_TOP: i32 = 16;
    const MARGIN_BOTTOM: i32 = 16;
    const FONT_SIZE: i32 = 13;

    let nodes = &doc.nodes;
    if nodes.is_empty() {
        let mut out = String::new();
        out.push_str(r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="60">"#);
        out.push_str(&format!(
            r#"<rect width="200" height="60" fill="{bg}"/>"#,
            bg = BG_FILL
        ));
        out.push_str(&format!(
            r#"<text x="100" y="34" text-anchor="middle" font-family="monospace" font-size="13" fill="{c}">empty files</text>"#,
            c = EMPTY_COLOR
        ));
        out.push_str("</svg>");
        return out;
    }

    // Compute the max text width to size the diagram
    let max_chars = nodes
        .iter()
        .map(|n| n.depth as i32 * (INDENT_PX / 8) + n.name.chars().count() as i32)
        .max()
        .unwrap_or(20);
    let total_w = MARGIN_LEFT + ICON_W + 6 + max_chars * 8 + MARGIN_LEFT + 20;
    let total_h = MARGIN_TOP + nodes.len() as i32 * ROW_H + MARGIN_BOTTOM;

    let mut svg = String::new();
    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}">"#,
        w = total_w.max(200),
        h = total_h,
    ));
    svg.push_str(&format!(
        r#"<rect width="{w}" height="{h}" fill="{bg}"/>"#,
        w = total_w.max(200),
        h = total_h,
        bg = BG_FILL,
    ));

    // Title
    if let Some(title) = &doc.title {
        svg.push_str(&format!(
            r#"<text x="{x}" y="{y}" text-anchor="middle" font-family="monospace" font-size="14" font-weight="bold" fill="{fill}">{t}</text>"#,
            x = total_w / 2,
            y = 12,
            fill = TEXT_FILL,
            t = escape_text(title),
        ));
    }

    for (i, node) in nodes.iter().enumerate() {
        let x = MARGIN_LEFT + node.depth as i32 * INDENT_PX;
        let y = MARGIN_TOP + i as i32 * ROW_H;
        let cy = y + ROW_H / 2;

        let is_folder = node.kind == FamilyNodeKind::FilesFolder;

        if is_folder {
            // Folder icon: tab on top + body below
            let tab_w = ICON_W / 2;
            let tab_h = 4;
            let body_h = ROW_H - 8;
            svg.push_str(&format!(
                r#"<rect x="{x}" y="{ty}" width="{tw}" height="{th}" rx="1" fill="{fill}" stroke="{stroke}" stroke-width="1"/>"#,
                x = x,
                ty = cy - body_h / 2 - tab_h,
                tw = tab_w,
                th = tab_h,
                fill = FOLDER_FILL,
                stroke = FOLDER_STROKE,
            ));
            svg.push_str(&format!(
                r#"<rect x="{x}" y="{by}" width="{w}" height="{h}" rx="2" fill="{fill}" stroke="{stroke}" stroke-width="1"/>"#,
                x = x,
                by = cy - body_h / 2,
                w = ICON_W,
                h = body_h,
                fill = FOLDER_FILL,
                stroke = FOLDER_STROKE,
            ));
        } else {
            // File icon: rectangle with folded corner
            let fw = ICON_W - 2;
            let fh = ROW_H - 6;
            let fold = 5;
            svg.push_str(&format!(
                r#"<polygon points="{x1},{y1} {x2},{y2} {x3},{y3} {x4},{y4} {x5},{y5}" fill="{fill}" stroke="{stroke}" stroke-width="1"/>"#,
                x1 = x,
                y1 = cy - fh / 2,
                x2 = x + fw - fold,
                y2 = cy - fh / 2,
                x3 = x + fw,
                y3 = cy - fh / 2 + fold,
                x4 = x + fw,
                y4 = cy + fh / 2,
                x5 = x,
                y5 = cy + fh / 2,
                fill = FILE_FILL,
                stroke = FILE_STROKE,
            ));
            // Folded corner
            svg.push_str(&format!(
                r#"<polyline points="{x1},{y1} {x2},{y2} {x3},{y3}" fill="none" stroke="{stroke}" stroke-width="1"/>"#,
                x1 = x + fw - fold,
                y1 = cy - fh / 2,
                x2 = x + fw - fold,
                y2 = cy - fh / 2 + fold,
                x3 = x + fw,
                y3 = cy - fh / 2 + fold,
                stroke = FILE_STROKE,
            ));
        }

        // Label
        svg.push_str(&format!(
            r#"<text x="{tx}" y="{ty}" dominant-baseline="middle" font-family="monospace" font-size="{fs}" fill="{fill}">{name}</text>"#,
            tx = x + ICON_W + 6,
            ty = cy,
            fs = FONT_SIZE,
            fill = TEXT_FILL,
            name = escape_text(&node.name),
        ));
    }

    svg.push_str("</svg>");
    svg
}
