use super::model::{Connector, DitaaGrid, DitaaOptions, Shape, ShapeKind};
use crate::specialized::shared::{escape_xml, svg_header, svg_white_bg};

pub(super) fn emit_svg(
    grid: &DitaaGrid,
    title: Option<&str>,
    options: &DitaaOptions,
    shapes: &[Shape],
    connectors: &[Connector],
) -> String {
    let mut out = String::new();
    out.push_str(&svg_header(grid.svg_w, grid.svg_h));
    emit_background(&mut out, options);
    emit_defs(&mut out, options);
    emit_title(&mut out, grid, title);
    emit_shapes(&mut out, grid, options, shapes);
    emit_connectors(&mut out, connectors);
    emit_junctions(&mut out, grid, shapes);
    emit_unclaimed_text(&mut out, grid, shapes);
    out.push_str("</svg>");
    out
}

fn emit_background(out: &mut String, options: &DitaaOptions) {
    if options.transparent {
        return;
    }
    if let Some(background) = &options.background {
        out.push_str(&format!(
            "<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>",
            escape_xml(background)
        ));
    } else {
        out.push_str(svg_white_bg());
    }
}

fn emit_defs(out: &mut String, options: &DitaaOptions) {
    out.push_str(
        "<defs>\
         <marker id=\"da\" markerWidth=\"8\" markerHeight=\"6\" refX=\"6\" refY=\"3\" orient=\"auto\">\
         <path d=\"M0,0 L0,6 L8,3 z\" fill=\"#444\"/></marker>\
         <marker id=\"dah\" markerWidth=\"8\" markerHeight=\"6\" refX=\"2\" refY=\"3\" orient=\"auto\">\
         <path d=\"M8,0 L8,6 L0,3 z\" fill=\"#444\"/></marker>\
         </defs>",
    );
    if options.shadow {
        out.push_str("<defs><filter id=\"ditaa-shadow\" x=\"-20%\" y=\"-20%\" width=\"140%\" height=\"140%\"><feDropShadow dx=\"2\" dy=\"2\" stdDeviation=\"1.5\" flood-color=\"#00000033\"/></filter></defs>");
    }
}

fn emit_title(out: &mut String, grid: &DitaaGrid, title: Option<&str>) {
    if let Some(t) = title {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"14\" font-weight=\"600\" text-anchor=\"middle\" fill=\"#222\">{}</text>",
            grid.svg_w / 2,
            grid.margin + 16,
            escape_xml(t)
        ));
    }
}

fn emit_shapes(out: &mut String, grid: &DitaaGrid, options: &DitaaOptions, shapes: &[Shape]) {
    for shape in shapes {
        let rx = grid.x_for_col(shape.c1);
        let ry = grid.y_for_row(shape.r1);
        let rw = (shape.c2 - shape.c1) as i32 * grid.cell_w;
        let rh = (shape.r2 - shape.r1) as i32 * grid.cell_h;
        let stroke = if shape.dashed {
            "stroke-dasharray=\"6,3\""
        } else {
            ""
        };
        let filter = if options.shadow {
            "filter=\"url(#ditaa-shadow)\""
        } else {
            ""
        };
        let shape_attr = shape.kind.attr();

        match shape.kind {
            ShapeKind::Rect => out.push_str(&format!(
                "<rect data-ditaa-shape=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"#3344aa\" stroke-width=\"1.5\" {} {}/>",
                shape_attr, rx, ry, rw, rh, shape.fill, stroke, filter
            )),
            ShapeKind::RoundedRect => out.push_str(&format!(
                "<rect data-ditaa-shape=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"12\" ry=\"12\" fill=\"{}\" stroke=\"#3344aa\" stroke-width=\"1.5\" {} {}/>",
                shape_attr, rx, ry, rw, rh, shape.fill, stroke, filter
            )),
            ShapeKind::Document => {
                emit_document(out, shape, shape_attr, ShapeBounds { rx, ry, rw, rh }, stroke)
            }
            ShapeKind::Cylinder => {
                emit_cylinder(out, shape, shape_attr, ShapeBounds { rx, ry, rw, rh }, stroke)
            }
            ShapeKind::Diamond => out.push_str(&format!(
                "<polygon data-ditaa-shape=\"{}\" points=\"{},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"#3344aa\" stroke-width=\"1.5\" {}/>",
                shape_attr, rx + rw / 2, ry, rx + rw, ry + rh / 2, rx + rw / 2, ry + rh, rx, ry + rh / 2, shape.fill, stroke
            )),
            ShapeKind::Io => emit_slanted(
                out,
                shape,
                shape_attr,
                ShapeBounds { rx, ry, rw, rh },
                stroke,
                filter,
                false,
            ),
            ShapeKind::ManualOperation | ShapeKind::Trapezoid => {
                emit_slanted(
                    out,
                    shape,
                    shape_attr,
                    ShapeBounds { rx, ry, rw, rh },
                    stroke,
                    filter,
                    true,
                )
            }
            ShapeKind::Ellipse => out.push_str(&format!(
                "<ellipse data-ditaa-shape=\"{}\" cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"#3344aa\" stroke-width=\"1.5\" {} {}/>",
                shape_attr, rx + rw / 2, ry + rh / 2, (rw / 2).max(8), (rh / 2).max(8), shape.fill, stroke, filter
            )),
        }

        for (row_idx, text) in &shape.text_lines {
            let tx = rx + rw / 2;
            let ty = grid.y_for_row(*row_idx) + grid.cell_h - 3;
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" text-anchor=\"middle\" fill=\"#111\">{}</text>",
                tx,
                ty,
                escape_xml(text)
            ));
        }
    }
}

fn emit_document(
    out: &mut String,
    shape: &Shape,
    shape_attr: &str,
    bounds: ShapeBounds,
    stroke: &str,
) {
    let ShapeBounds { rx, ry, rw, rh } = bounds;
    let cx = rx + rw / 2;
    let bot_y = ry + rh;
    out.push_str(&format!(
        "<path data-ditaa-shape=\"{}\" d=\"M {},{} L {},{} L {},{} Q {},{} {},{} Q {},{} {},{}  Z\" fill=\"{}\" stroke=\"#3344aa\" stroke-width=\"1.5\" {}/>",
        shape_attr,
        rx,
        ry,
        rx + rw,
        ry,
        rx + rw,
        bot_y - 8,
        cx + rw / 4,
        bot_y + 6,
        cx,
        bot_y - 4,
        cx - rw / 4,
        bot_y - 14,
        rx,
        bot_y - 8,
        shape.fill,
        stroke
    ));
}

fn emit_cylinder(
    out: &mut String,
    shape: &Shape,
    shape_attr: &str,
    bounds: ShapeBounds,
    stroke: &str,
) {
    let ShapeBounds { rx, ry, rw, rh } = bounds;
    let cx = rx + rw / 2;
    let ell_ry = 6i32;
    out.push_str(&format!(
        "<g data-ditaa-shape=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"#3344aa\" stroke-width=\"1.5\" {}/>\
         <ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"#3344aa\" stroke-width=\"1.5\"/>\
         <ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"none\" stroke=\"#3344aa\" stroke-width=\"1\"/></g>",
        shape_attr,
        rx,
        ry + ell_ry,
        rw,
        rh - ell_ry,
        shape.fill,
        stroke,
        cx,
        ry + ell_ry,
        rw / 2,
        ell_ry,
        shape.fill,
        cx,
        ry + rh,
        rw / 2,
        ell_ry
    ));
}

#[derive(Clone, Copy)]
struct ShapeBounds {
    rx: i32,
    ry: i32,
    rw: i32,
    rh: i32,
}

fn emit_slanted(
    out: &mut String,
    shape: &Shape,
    shape_attr: &str,
    bounds: ShapeBounds,
    stroke: &str,
    filter: &str,
    symmetric: bool,
) {
    let ShapeBounds { rx, ry, rw, rh } = bounds;
    let slant = (rw / 6).max(8);
    let points = if symmetric {
        format!(
            "{},{} {},{} {},{} {},{}",
            rx,
            ry,
            rx + rw,
            ry,
            rx + rw - slant,
            ry + rh,
            rx + slant,
            ry + rh
        )
    } else {
        format!(
            "{},{} {},{} {},{} {},{}",
            rx + slant,
            ry,
            rx + rw,
            ry,
            rx + rw - slant,
            ry + rh,
            rx,
            ry + rh
        )
    };
    out.push_str(&format!(
        "<polygon data-ditaa-shape=\"{}\" points=\"{}\" fill=\"{}\" stroke=\"#3344aa\" stroke-width=\"1.5\" {} {}/>",
        shape_attr, points, shape.fill, stroke, filter
    ));
}

fn emit_connectors(out: &mut String, connectors: &[Connector]) {
    for conn in connectors {
        let dash = if conn.dashed {
            " stroke-dasharray=\"6,3\""
        } else {
            ""
        };
        let marker_end = if conn.has_head_end {
            " marker-end=\"url(#da)\""
        } else {
            ""
        };
        let marker_start = if conn.has_head_start {
            " marker-start=\"url(#dah)\""
        } else {
            ""
        };
        out.push_str(&format!(
            "<line class=\"ditaa-connector\" data-ditaa-arrow-start=\"{}\" data-ditaa-arrow-end=\"{}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#444\" stroke-width=\"1.5\"{}{}{}/>",
            conn.has_head_start, conn.has_head_end, conn.x1, conn.y1, conn.x2, conn.y2, dash, marker_end, marker_start
        ));
    }
}

fn emit_junctions(out: &mut String, grid: &DitaaGrid, shapes: &[Shape]) {
    for (row_idx, row) in grid.lines.iter().enumerate() {
        for (c, &ch) in row.iter().enumerate() {
            if ch != '+' || is_in_shape(shapes, row_idx, c) {
                continue;
            }
            let horizontal = matches!(grid.get(row_idx, c.saturating_sub(1)), '-' | '=')
                || matches!(grid.get(row_idx, c + 1), '-' | '=' | '>');
            let vertical = matches!(grid.get(row_idx.saturating_sub(1), c), '|' | ':' | '^')
                || matches!(grid.get(row_idx + 1, c), '|' | ':' | 'v');
            if horizontal && vertical {
                let x = grid.x_for_col(c) + grid.cell_w / 2;
                let y = grid.y_for_row(row_idx) + grid.cell_h / 2;
                out.push_str(&format!(
                    "<circle class=\"ditaa-junction\" data-ditaa-junction=\"true\" cx=\"{}\" cy=\"{}\" r=\"3\" fill=\"#444\"/>",
                    x, y
                ));
            }
        }
    }
}

fn emit_unclaimed_text(out: &mut String, grid: &DitaaGrid, shapes: &[Shape]) {
    for (row_idx, row) in grid.lines.iter().enumerate() {
        let ty = grid.y_for_row(row_idx) + grid.cell_h - 3;
        let mut run_start_col: Option<usize> = None;
        let mut run_text = String::new();

        for (c, &ch) in row.iter().enumerate() {
            let is_structural = is_in_shape(shapes, row_idx, c)
                || matches!(
                    ch,
                    '+' | '-'
                        | '|'
                        | '='
                        | ':'
                        | '>'
                        | '<'
                        | 'v'
                        | '^'
                        | '~'
                        | '('
                        | ')'
                        | '/'
                        | '\\'
                );
            if is_structural {
                flush_text_run(out, grid, ty, &mut run_start_col, &mut run_text);
                continue;
            }
            if ch == ' ' {
                if run_start_col.is_some() {
                    run_text.push(' ');
                }
                continue;
            }
            if run_start_col.is_none() {
                run_start_col = Some(c);
            }
            run_text.push(ch);
        }
        flush_text_run(out, grid, ty, &mut run_start_col, &mut run_text);
    }
}

fn flush_text_run(
    out: &mut String,
    grid: &DitaaGrid,
    ty: i32,
    run_start_col: &mut Option<usize>,
    run_text: &mut String,
) {
    if let Some(sc) = run_start_col.take() {
        let run_trimmed = run_text.trim_end().to_string();
        if !run_trimmed.is_empty() {
            let tx = grid.x_for_col(sc);
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#222\" xml:space=\"preserve\">{}</text>",
                tx,
                ty,
                escape_xml(&run_trimmed)
            ));
        }
        run_text.clear();
    }
}

fn is_in_shape(shapes: &[Shape], row_idx: usize, c: usize) -> bool {
    shapes
        .iter()
        .any(|s| row_idx >= s.r1 && row_idx <= s.r2 && c >= s.c1 && c <= s.c2)
}
