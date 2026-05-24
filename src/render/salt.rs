mod layout;
mod model;
mod parsing;
mod style;
mod text;
mod transform;
mod widgets;

use self::layout::{is_salt_separator_row, salt_row_layout};
use self::model::SaltCellRender;
use self::parsing::decode_salt_cell;
use self::style::SaltRenderStyle;
use self::text::{estimate_text_width, salt_text};
use self::transform::{transform_salt_row, SaltTransformState};
use self::widgets::{render_salt_cell_svg, SaltCellBox};
use super::*;

pub fn render_salt_svg(document: &FamilyDocument) -> String {
    const DEFAULT_CELL_H: i32 = 20;
    const CELL_PAD_X: i32 = 10;
    const MARGIN: i32 = 6;
    const MIN_CELL_W: i32 = 80;

    // Parse rows from the encoded node names.
    let mut rows: Vec<Vec<SaltCellRender>> = Vec::new();
    let mut salt_state = SaltTransformState::default();
    let mut style = SaltRenderStyle::default();
    for node in &document.nodes {
        if let Some(rest) = node.name.strip_prefix("SALT_ROW\x1f") {
            let cells: Vec<SaltCellRender> = rest.split('\x1e').map(decode_salt_cell).collect();
            if let Some(cells) = transform_salt_row(cells, &mut salt_state, &mut style) {
                rows.push(cells);
            }
        }
    }

    if rows.is_empty() {
        // Fallback: render a minimal empty wireframe
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"120\" height=\"60\" viewBox=\"0 0 120 60\"><rect width=\"120\" height=\"60\" fill=\"white\"/><text x=\"10\" y=\"30\" font-family=\"monospace\" font-size=\"11\" fill=\"#666\">[salt]</text></svg>".to_string();
    }

    // Compute number of columns from the max row width.
    let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(1);
    let table_like = rows.iter().flatten().any(SaltCellRender::is_table_like);

    // Per-row heights (support variable-height cells like open combos).
    let row_heights: Vec<i32> = rows
        .iter()
        .map(|row| {
            row.iter()
                .map(SaltCellRender::intrinsic_height)
                .max()
                .unwrap_or(DEFAULT_CELL_H)
        })
        .collect();

    // First pass: compute per-column minimum widths based on text content.
    let mut col_widths: Vec<i32> = vec![MIN_CELL_W; col_count];
    for row in &rows {
        for (col_idx, cell) in row.iter().enumerate() {
            let text_w = cell
                .intrinsic_width()
                .max(estimate_text_width(cell.text()) + CELL_PAD_X);
            if text_w > col_widths[col_idx] {
                col_widths[col_idx] = text_w;
            }
        }
    }

    let total_w = col_widths.iter().sum::<i32>() + MARGIN * 2;
    let total_h = row_heights.iter().sum::<i32>() + MARGIN * 2;

    // Header/footer/title heights.
    let header_h = document.header.as_deref().map(|_| 20i32).unwrap_or(0);
    let title_h = document.title.as_deref().map(|_| 22i32).unwrap_or(0);
    let footer_h = document.footer.as_deref().map(|_| 20i32).unwrap_or(0);
    let caption_h = document.caption.as_deref().map(|_| 18i32).unwrap_or(0);
    let legend_h = document.legend.as_deref().map(|_| 18i32).unwrap_or(0);
    let top_extra = header_h + title_h;
    let bottom_extra = footer_h + caption_h + legend_h;
    let svg_h = total_h + top_extra + bottom_extra;

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, svg_h, total_w, svg_h
    ));
    out.push_str(&format!(
        "<rect data-salt-style=\"canvas\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
        total_w, svg_h, style.canvas_fill
    ));

    let render_panel = table_like || style.panel_fill != SaltRenderStyle::default().panel_fill;
    if render_panel {
        out.push_str(&format!(
            "<rect data-salt-style=\"panel\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            MARGIN,
            MARGIN + top_extra,
            total_w - MARGIN * 2,
            total_h - MARGIN * 2,
            style.panel_fill,
            style.border_color
        ));
    }

    // Header (top of diagram, above title).
    if let Some(header) = &document.header {
        salt_text(
            &mut out,
            MARGIN,
            MARGIN + 14,
            &format!(
                "font-family=\"{}\" font-size=\"11\" fill=\"{}\"",
                style.font_family, style.muted_text_color
            ),
            header,
            &style.muted_text_color,
        );
    }

    // Title (below header, above content).
    if let Some(title) = &document.title {
        let ty = MARGIN + header_h + 16;
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"{}\" font-size=\"13\" font-weight=\"600\" fill=\"{}\" text-anchor=\"middle\">{}</text>",
            total_w / 2,
            ty,
            style.font_family,
            style.text_color,
            escape_text(title)
        ));
    }

    // Draw rows and cells.
    let mut current_y = MARGIN + top_extra;
    for (row_idx, cells) in rows.iter().enumerate() {
        let row_h = row_heights[row_idx];
        let row_y = current_y;
        current_y += row_h;

        if is_salt_separator_row(cells) {
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                MARGIN + 4,
                row_y + row_h / 2,
                total_w - MARGIN - 4,
                row_y + row_h / 2,
                style.border_color
            ));
            continue;
        }
        let mut col_x = MARGIN;
        let rendered_cells = salt_row_layout(cells, &col_widths, MIN_CELL_W);

        for cell in rendered_cells {
            render_salt_cell_svg(
                &mut out,
                cell.cell,
                SaltCellBox {
                    x: col_x,
                    y: row_y,
                    w: cell.width,
                    h: row_h,
                },
                cell.colspan,
                &style,
            );
            col_x += cell.width;
        }

        // Row separator line (skip the last row).
        if table_like && row_idx + 1 < rows.len() {
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
                MARGIN,
                current_y,
                total_w - MARGIN,
                current_y,
                style.grid_color
            ));
        }
    }

    // Column separator lines.
    if table_like {
        let mut col_x = MARGIN;
        for (col_idx, w) in col_widths.iter().enumerate() {
            col_x += w;
            if col_idx + 1 < col_count {
                out.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
                    col_x,
                    MARGIN + top_extra,
                    col_x,
                    MARGIN + top_extra + total_h - MARGIN * 2,
                    style.grid_color
                ));
            }
        }
    }

    // Footer (below content).
    let footer_y = MARGIN + top_extra + total_h;
    if let Some(footer) = &document.footer {
        salt_text(
            &mut out,
            MARGIN,
            footer_y + 14,
            &format!(
                "font-family=\"{}\" font-size=\"11\" fill=\"{}\"",
                style.font_family, style.muted_text_color
            ),
            footer,
            &style.muted_text_color,
        );
    }

    // Caption (below footer).
    if let Some(caption) = &document.caption {
        let cy = footer_y + footer_h + 14;
        salt_text(
            &mut out,
            total_w / 2,
            cy,
            &format!(
                "font-family=\"{}\" font-size=\"11\" fill=\"{}\" text-anchor=\"middle\"",
                style.font_family, style.muted_text_color
            ),
            caption,
            &style.muted_text_color,
        );
    }

    // Legend (below caption).
    if let Some(legend) = &document.legend {
        let ly = footer_y + footer_h + caption_h + 14;
        salt_text(
            &mut out,
            MARGIN,
            ly,
            &format!(
                "font-family=\"{}\" font-size=\"11\" font-style=\"italic\" fill=\"{}\"",
                style.font_family, style.muted_text_color
            ),
            legend,
            &style.muted_text_color,
        );
    }

    out.push_str("</svg>");
    out
}
