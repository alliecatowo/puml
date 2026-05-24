use super::ast::Expr;
use crate::specialized::shared::escape_xml;

pub(super) struct Layout {
    pub(super) svg: String,
    pub(super) width: f64,
    pub(super) height: f64,
    ascent: f64, // distance from top to baseline
}

pub(super) const BASE_FONT: f64 = 20.0;
const CHAR_W_RATIO: f64 = 0.55; // approximate char width as fraction of font-size
const SUB_SCALE: f64 = 0.65;
const SUP_SCALE: f64 = 0.65;
const FRAC_PAD: f64 = 4.0;
const SQRT_LEAN: f64 = 8.0; // width of the radical foot
pub(super) const SVG_MATH_FONT_FAMILY: &str = "'Noto Sans Math','STIX Two Math','Cambria Math','Latin Modern Math','DejaVu Serif','Times New Roman',serif";

fn char_width(font_size: f64) -> f64 {
    font_size * CHAR_W_RATIO
}

fn layout_expr(expr: &Expr, font_size: f64) -> Layout {
    match expr {
        Expr::Literal(s) => {
            if s.is_empty() {
                return Layout {
                    svg: String::new(),
                    width: 0.0,
                    height: font_size,
                    ascent: font_size * 0.8,
                };
            }
            // Estimate width: each char is ~char_w
            let char_w = char_width(font_size);
            let width = s.chars().count() as f64 * char_w;
            let height = font_size * 1.2;
            let ascent = font_size * 0.8;
            let svg = format!(
                "<text x=\"0\" y=\"{}\" font-family=\"{}\" font-size=\"{}\" fill=\"#111\">{}</text>",
                ascent,
                SVG_MATH_FONT_FAMILY,
                font_size,
                escape_xml(s)
            );
            Layout {
                svg,
                width,
                height,
                ascent,
            }
        }
        Expr::Text(s) => {
            let char_w = char_width(font_size);
            let width = s.chars().count() as f64 * char_w;
            let height = font_size * 1.2;
            let ascent = font_size * 0.8;
            let svg = format!(
                "<text x=\"0\" y=\"{}\" font-family=\"{}\" font-size=\"{}\" fill=\"#111\">{}</text>",
                ascent,
                SVG_MATH_FONT_FAMILY,
                font_size,
                escape_xml(s)
            );
            Layout {
                svg,
                width,
                height,
                ascent,
            }
        }
        Expr::Greek(c) => {
            let char_w = char_width(font_size);
            let width = char_w * 1.2; // Greek chars slightly wider
            let height = font_size * 1.2;
            let ascent = font_size * 0.8;
            let svg = format!(
                "<text x=\"0\" y=\"{}\" font-family=\"{}\" font-size=\"{}\" fill=\"#111\">{}</text>",
                ascent,
                SVG_MATH_FONT_FAMILY,
                font_size,
                escape_xml(&c.to_string())
            );
            Layout {
                svg,
                width,
                height,
                ascent,
            }
        }
        Expr::Matrix { env, rows } => {
            let cell_pad_x = font_size * 0.45;
            let row_gap = font_size * 0.25;
            let layouts: Vec<Vec<Layout>> = rows
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|cell| layout_expr(cell, font_size * 0.9))
                        .collect()
                })
                .collect();
            let col_count = layouts.iter().map(|row| row.len()).max().unwrap_or(0);
            let mut col_widths = vec![font_size * 0.6; col_count];
            let mut row_heights = vec![font_size; layouts.len()];
            let mut row_ascents = vec![font_size * 0.75; layouts.len()];
            for (r, row) in layouts.iter().enumerate() {
                for (c, cell) in row.iter().enumerate() {
                    col_widths[c] = col_widths[c].max(cell.width);
                    row_heights[r] = row_heights[r].max(cell.height);
                    row_ascents[r] = row_ascents[r].max(cell.ascent);
                }
            }
            let body_w = col_widths.iter().sum::<f64>() + cell_pad_x * 2.0 * col_count as f64;
            let body_h =
                row_heights.iter().sum::<f64>() + row_gap * layouts.len().saturating_sub(1) as f64;
            let fence_w =
                if env == "matrix" || env == "smallmatrix" || env == "aligned" || env == "align" {
                    0.0
                } else {
                    font_size * 0.45
                };
            let total_w = body_w + fence_w * 2.0;
            let total_h = body_h.max(font_size);
            let ascent = total_h * 0.58;
            let mut svg = format!("<g data-math-env=\"{}\">", escape_xml(env));

            let mut y = 0.0;
            for (r, row) in layouts.iter().enumerate() {
                let mut x = fence_w;
                for (c, cell) in row.iter().enumerate() {
                    let cell_x = x + cell_pad_x + (col_widths[c] - cell.width) / 2.0;
                    let cell_y = y + row_ascents[r] - cell.ascent;
                    svg.push_str(&format!(
                        "<g transform=\"translate({},{})\">{}</g>",
                        cell_x, cell_y, cell.svg
                    ));
                    x += col_widths[c] + cell_pad_x * 2.0;
                }
                y += row_heights[r] + row_gap;
            }

            match env.as_str() {
                "pmatrix" => {
                    svg.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" font-family=\"{}\" font-size=\"{}\" fill=\"#111\" text-anchor=\"middle\">(</text><text x=\"{}\" y=\"{}\" font-family=\"{}\" font-size=\"{}\" fill=\"#111\" text-anchor=\"middle\">)</text>",
                        fence_w / 2.0, ascent, SVG_MATH_FONT_FAMILY, total_h * 1.15, total_w - fence_w / 2.0, ascent, SVG_MATH_FONT_FAMILY, total_h * 1.15
                    ));
                }
                "bmatrix" => {
                    svg.push_str(&format!(
                        "<path d=\"M {},0 L 0,0 L 0,{} L {},{}\" fill=\"none\" stroke=\"#333\" stroke-width=\"1.4\"/><path d=\"M {},0 L {},0 L {},{} L {},{}\" fill=\"none\" stroke=\"#333\" stroke-width=\"1.4\"/>",
                        fence_w, total_h, fence_w, total_h, total_w - fence_w, total_w, total_w, total_h, total_w - fence_w, total_h
                    ));
                }
                "Bmatrix" => {
                    svg.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" font-family=\"{}\" font-size=\"{}\" fill=\"#111\" text-anchor=\"middle\">{{</text><text x=\"{}\" y=\"{}\" font-family=\"{}\" font-size=\"{}\" fill=\"#111\" text-anchor=\"middle\">}}</text>",
                        fence_w / 2.0, ascent, SVG_MATH_FONT_FAMILY, total_h * 1.15, total_w - fence_w / 2.0, ascent, SVG_MATH_FONT_FAMILY, total_h * 1.15
                    ));
                }
                "vmatrix" | "Vmatrix" => {
                    let sw = if env == "Vmatrix" { 2.2 } else { 1.4 };
                    svg.push_str(&format!(
                        "<line x1=\"{}\" y1=\"0\" x2=\"{}\" y2=\"{}\" stroke=\"#333\" stroke-width=\"{}\"/><line x1=\"{}\" y1=\"0\" x2=\"{}\" y2=\"{}\" stroke=\"#333\" stroke-width=\"{}\"/>",
                        fence_w / 2.0, fence_w / 2.0, total_h, sw, total_w - fence_w / 2.0, total_w - fence_w / 2.0, total_h, sw
                    ));
                }
                _ => {}
            }
            svg.push_str("</g>");
            Layout {
                svg,
                width: total_w,
                height: total_h,
                ascent,
            }
        }
        Expr::Group(exprs) => layout_group(exprs, font_size),
        Expr::Sub(base, sub) => {
            let base_l = layout_expr(base, font_size);
            let sub_l = layout_expr(sub, font_size * SUB_SCALE);
            // sub goes below-right of base, shifted down by 0.3em
            let sub_shift = font_size * 0.3;
            let sub_x = base_l.width;
            let sub_y = base_l.ascent + sub_shift;
            let total_w = base_l.width + sub_l.width;
            let total_h = (sub_y + sub_l.height).max(base_l.height);
            let ascent = base_l.ascent;
            let svg = format!(
                "{}<g transform=\"translate({},{})\">{}</g>",
                base_l.svg,
                sub_x,
                sub_y - sub_l.ascent,
                sub_l.svg
            );
            Layout {
                svg,
                width: total_w,
                height: total_h,
                ascent,
            }
        }
        Expr::Sup(base, sup) => {
            let base_l = layout_expr(base, font_size);
            let sup_l = layout_expr(sup, font_size * SUP_SCALE);
            // sup goes above-right of base, shifted up by 0.5em
            let sup_shift = font_size * 0.5;
            let sup_x = base_l.width;
            let sup_y = base_l.ascent - sup_shift - sup_l.ascent;
            let _actual_sup_y = sup_y.min(0.0);
            let dy = if sup_y < 0.0 { -sup_y } else { 0.0 };
            let total_w = base_l.width + sup_l.width;
            let total_h = (base_l.height + dy).max(sup_l.height + dy);
            let ascent = base_l.ascent + dy;
            let svg =
                format!(
                "<g transform=\"translate(0,{})\">{}<g transform=\"translate({},{})\">{}</g></g>",
                dy, base_l.svg, sup_x, sup_y + dy, sup_l.svg
            );
            Layout {
                svg,
                width: total_w,
                height: total_h,
                ascent,
            }
        }
        Expr::SubSup(base, sub, sup) => {
            let base_l = layout_expr(base, font_size);
            let sub_l = layout_expr(sub, font_size * SUB_SCALE);
            let sup_l = layout_expr(sup, font_size * SUP_SCALE);
            let script_w = sub_l.width.max(sup_l.width);
            let sup_shift = font_size * 0.5;
            let sub_shift = font_size * 0.3;
            let sup_y = base_l.ascent - sup_shift - sup_l.ascent;
            let dy = if sup_y < 0.0 { -sup_y } else { 0.0 };
            let sub_y = base_l.ascent + sub_shift + dy;
            let total_w = base_l.width + script_w;
            let total_h = (sub_y + sub_l.height - sub_l.ascent).max(base_l.height + dy);
            let ascent = base_l.ascent + dy;
            let svg = format!(
                "<g transform=\"translate(0,{})\">{}<g transform=\"translate({},{})\">{}</g><g transform=\"translate({},{})\">{}</g></g>",
                dy,
                base_l.svg,
                base_l.width,
                sup_y + dy,
                sup_l.svg,
                base_l.width,
                sub_y - sub_l.ascent,
                sub_l.svg
            );
            Layout {
                svg,
                width: total_w,
                height: total_h,
                ascent,
            }
        }
        Expr::Frac(num, den) => {
            let num_l = layout_expr(num, font_size * 0.85);
            let den_l = layout_expr(den, font_size * 0.85);
            let inner_w = num_l.width.max(den_l.width) + FRAC_PAD * 2.0;
            // Line at the middle
            let line_y = num_l.height + FRAC_PAD;
            let total_h = num_l.height + FRAC_PAD + 2.0 + FRAC_PAD + den_l.height;
            let ascent = line_y + 1.0; // baseline at the fraction line
            let num_x = (inner_w - num_l.width) / 2.0;
            let den_x = (inner_w - den_l.width) / 2.0;
            let den_y = line_y + 2.0 + FRAC_PAD;
            let svg = format!(
                "<g transform=\"translate({},0)\">{}</g>\
                 <line x1=\"0\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#333\" stroke-width=\"1.2\"/>\
                 <g transform=\"translate({},{})\">{}</g>",
                num_x,
                num_l.svg,
                line_y + 1.0,
                inner_w,
                line_y + 1.0,
                den_x,
                den_y,
                den_l.svg
            );
            Layout {
                svg,
                width: inner_w,
                height: total_h,
                ascent,
            }
        }
        Expr::Binom(top, bottom) => {
            let top_l = layout_expr(top, font_size * 0.85);
            let bottom_l = layout_expr(bottom, font_size * 0.85);
            let pad = font_size * 0.35;
            let body_w = top_l.width.max(bottom_l.width) + pad * 2.0;
            let gap = font_size * 0.08;
            let body_h = top_l.height + gap + bottom_l.height;
            let fence_w = font_size * 0.35;
            let total_w = body_w + fence_w * 2.0;
            let total_h = body_h.max(font_size * 1.3);
            let ascent = total_h * 0.58;
            let top_x = fence_w + (body_w - top_l.width) / 2.0;
            let bottom_x = fence_w + (body_w - bottom_l.width) / 2.0;
            let bottom_y = top_l.height + gap;
            let svg = format!(
                "<g data-math-construct=\"binom\"><text x=\"{}\" y=\"{}\" font-family=\"{}\" font-size=\"{}\" fill=\"#111\" text-anchor=\"middle\">(</text><g transform=\"translate({},{})\">{}</g><g transform=\"translate({},{})\">{}</g><text x=\"{}\" y=\"{}\" font-family=\"{}\" font-size=\"{}\" fill=\"#111\" text-anchor=\"middle\">)</text></g>",
                fence_w / 2.0,
                ascent,
                SVG_MATH_FONT_FAMILY,
                total_h * 1.1,
                top_x,
                0.0,
                top_l.svg,
                bottom_x,
                bottom_y,
                bottom_l.svg,
                total_w - fence_w / 2.0,
                ascent,
                SVG_MATH_FONT_FAMILY,
                total_h * 1.1
            );
            Layout {
                svg,
                width: total_w,
                height: total_h,
                ascent,
            }
        }
        Expr::Sqrt(inner) => {
            let inner_l = layout_expr(inner, font_size);
            let pad = 4.0;
            let inner_x = SQRT_LEAN + pad;
            let inner_y = pad;
            let total_w = inner_x + inner_l.width + pad;
            let total_h = inner_l.height + pad * 2.0;
            let ascent = inner_l.ascent + pad;
            // Radical path: short foot then up to top then horizontal overline
            let foot_x = 0.0;
            let foot_y = total_h * 0.75;
            let corner_x = SQRT_LEAN * 0.5;
            let corner_y = total_h;
            let top_left_x = SQRT_LEAN;
            let top_left_y = inner_y;
            let overline_end_x = total_w - 1.0;
            let svg = format!(
                "<path d=\"M {},{} L {},{} L {},{} L {},{}\" fill=\"none\" stroke=\"#333\" stroke-width=\"1.5\"/>\
                 <g transform=\"translate({},{})\">{}</g>",
                foot_x, foot_y,
                corner_x, corner_y,
                top_left_x, top_left_y,
                overline_end_x, top_left_y,
                inner_x, inner_y,
                inner_l.svg
            );
            Layout {
                svg,
                width: total_w,
                height: total_h,
                ascent,
            }
        }
        Expr::Accent { kind, inner } => {
            let inner_l = layout_expr(inner, font_size);
            let top_pad = if kind == "underline" {
                2.0
            } else {
                font_size * 0.35
            };
            let bottom_pad = if kind == "underline" {
                font_size * 0.25
            } else {
                0.0
            };
            let width = inner_l.width.max(font_size * 0.7);
            let height = inner_l.height + top_pad + bottom_pad;
            let ascent = inner_l.ascent + top_pad;
            let inner_x = (width - inner_l.width) / 2.0;
            let mut svg = String::new();
            match kind.as_str() {
                "hat" => svg.push_str(&format!(
                    "<path d=\"M {},{} L {},{} L {},{}\" fill=\"none\" stroke=\"#333\" stroke-width=\"1.2\"/>",
                    width * 0.2,
                    top_pad,
                    width * 0.5,
                    1.0,
                    width * 0.8,
                    top_pad
                )),
                "vec" => svg.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#333\" stroke-width=\"1.2\" marker-end=\"url(#math-arrow)\"/>",
                    width * 0.15,
                    top_pad * 0.55,
                    width * 0.85,
                    top_pad * 0.55
                )),
                "dot" | "ddot" => {
                    svg.push_str(&format!(
                        "<circle cx=\"{}\" cy=\"{}\" r=\"1.5\" fill=\"#333\"/>",
                        width * 0.45,
                        top_pad * 0.45
                    ));
                    if kind == "ddot" {
                        svg.push_str(&format!(
                            "<circle cx=\"{}\" cy=\"{}\" r=\"1.5\" fill=\"#333\"/>",
                            width * 0.6,
                            top_pad * 0.45
                        ));
                    }
                }
                "underline" => svg.push_str(&format!(
                    "<line x1=\"0\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#333\" stroke-width=\"1.2\"/>",
                    inner_l.height + top_pad + 2.0,
                    width,
                    inner_l.height + top_pad + 2.0
                )),
                _ => svg.push_str(&format!(
                    "<line x1=\"0\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#333\" stroke-width=\"1.2\"/>",
                    top_pad * 0.45,
                    width,
                    top_pad * 0.45
                )),
            }
            svg.push_str(&format!(
                "<g transform=\"translate({},{})\">{}</g>",
                inner_x, top_pad, inner_l.svg
            ));
            Layout {
                svg,
                width,
                height,
                ascent,
            }
        }
        Expr::BigOp { op, sub, sup } => {
            let op_font = font_size * 1.6;
            let op_char = op.to_string();
            let op_char_w = char_width(op_font) * 1.4;
            let op_h = op_font * 1.2;
            let op_ascent = op_font * 0.8;

            let sub_l = layout_expr(sub, font_size * SUB_SCALE);
            let sup_l = layout_expr(sup, font_size * SUP_SCALE);

            let inner_w = op_char_w.max(sub_l.width).max(sup_l.width);

            // sup above operator, sub below
            let sup_y = 0.0;
            let op_y = sup_l.height + 2.0;
            let sub_y = op_y + op_h + 2.0;
            let total_h = sub_y + sub_l.height;
            let ascent = op_y + op_ascent;

            let op_x = (inner_w - op_char_w) / 2.0;
            let sup_x = (inner_w - sup_l.width) / 2.0;
            let sub_x = (inner_w - sub_l.width) / 2.0;

            let svg = format!(
                "<g transform=\"translate({},{})\">{}</g>\
                 <text x=\"{}\" y=\"{}\" font-family=\"{}\" font-size=\"{}\" fill=\"#111\" text-anchor=\"middle\">{}</text>\
                 <g transform=\"translate({},{})\">{}</g>",
                sup_x, sup_y, sup_l.svg,
                op_x + op_char_w / 2.0, op_y + op_ascent, SVG_MATH_FONT_FAMILY, op_font, escape_xml(&op_char),
                sub_x, sub_y, sub_l.svg
            );
            Layout {
                svg,
                width: inner_w,
                height: total_h,
                ascent,
            }
        }
    }
}

pub(super) fn layout_group(exprs: &[Expr], font_size: f64) -> Layout {
    if exprs.is_empty() {
        return Layout {
            svg: String::new(),
            width: 0.0,
            height: font_size * 1.2,
            ascent: font_size * 0.8,
        };
    }
    let layouts: Vec<Layout> = exprs.iter().map(|e| layout_expr(e, font_size)).collect();
    // Align all nodes by baseline
    let max_ascent = layouts.iter().map(|l| l.ascent).fold(0.0f64, f64::max);
    let max_below = layouts
        .iter()
        .map(|l| l.height - l.ascent)
        .fold(0.0f64, f64::max);
    let total_h = max_ascent + max_below;
    let mut x = 0.0f64;
    let mut svg = String::new();
    for l in &layouts {
        if l.width == 0.0 && l.svg.is_empty() {
            continue;
        }
        let dy = max_ascent - l.ascent;
        svg.push_str(&format!(
            "<g transform=\"translate({},{})\">{}</g>",
            x, dy, l.svg
        ));
        x += l.width;
    }
    // Add small gap between items
    let total_w = x;
    Layout {
        svg,
        width: total_w,
        height: total_h,
        ascent: max_ascent,
    }
}
