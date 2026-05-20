// ─── Family 1: @startmath ─────────────────────────────────────────────────────
//
// Real LaTeX expression tree with layout engine.

mod ast;
mod commands;
mod layout;
mod parser;
mod tokenizer;

use self::layout::{layout_group, BASE_FONT, SVG_MATH_FONT_FAMILY};
use self::parser::parse_math_expr;
use super::shared::{escape_xml, strip_block, svg_header, svg_white_bg};
use crate::diagnostic::Diagnostic;

/// Render a math diagram directly from its body and optional title, without
/// reconstructing the `@startmath...@endmath` wrapper. Called from the model
/// render path (`render::specialized::math`) so the LSP and CLI share the same
/// rendering logic without the two-hop reconstruct round-trip.
pub(crate) fn render_math_from_parts(
    body: &str,
    title: Option<&str>,
) -> Result<String, Diagnostic> {
    let expr_text = body.trim();
    if expr_text.is_empty() {
        return Err(Diagnostic::error("[E_MATH_EMPTY] @startmath body is empty"));
    }

    // Parse and layout
    let exprs = parse_math_expr(expr_text);
    let layout = layout_group(&exprs, BASE_FONT);

    let title_h = if title.is_some() { 28i32 } else { 0 };
    let margin = 30i32;
    let w = (layout.width as i32 + margin * 2).max(200);
    let h = layout.height as i32 + margin * 2 + title_h;

    let mut out = String::new();
    out.push_str(&svg_header(w, h));
    out.push_str(svg_white_bg());
    out.push_str(
        "<defs><marker id=\"math-arrow\" markerWidth=\"7\" markerHeight=\"5\" refX=\"6\" refY=\"2.5\" orient=\"auto\"><path d=\"M0,0 L0,5 L7,2.5 z\" fill=\"#333\"/></marker></defs>",
    );

    if let Some(t) = title {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"22\" font-family=\"{}\" font-size=\"14\" font-weight=\"600\" text-anchor=\"middle\" fill=\"#333\">{}</text>",
            w / 2,
            SVG_MATH_FONT_FAMILY,
            escape_xml(t)
        ));
    }

    // Expression background box
    let ex = (w as f64 - layout.width) / 2.0;
    let ey = title_h + margin;
    out.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" fill=\"#f9f9f9\" stroke=\"#ddd\" stroke-width=\"1\"/>",
        (ex - 10.0) as i32, ey - 10, (layout.width + 20.0) as i32, (layout.height + 20.0) as i32
    ));

    out.push_str(&format!(
        "<g transform=\"translate({},{})\">{}</g>",
        ex as i32, ey, layout.svg
    ));

    out.push_str("</svg>");
    Ok(out)
}

pub(super) fn render_math(source: &str) -> Result<String, Diagnostic> {
    let (body, title) = strip_block(source, "@startmath", "@endmath");
    render_math_from_parts(body, title.as_deref())
}

pub(super) fn render_latex(source: &str) -> Result<String, Diagnostic> {
    let (body, title) = strip_block(source, "@startlatex", "@endlatex");
    let mut normalized = String::from("@startmath");
    if let Some(t) = title {
        normalized.push(' ');
        normalized.push('"');
        normalized.push_str(&t.replace('"', "\\\""));
        normalized.push('"');
    }
    normalized.push('\n');
    normalized.push_str(body);
    normalized.push('\n');
    normalized.push_str("@endmath\n");
    render_math(&normalized)
}
